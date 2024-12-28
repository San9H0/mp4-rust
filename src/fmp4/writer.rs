use std::io::{Cursor, Seek, Write};

use bytes::BytesMut;

use crate::{
    Avc1Box, BoxHeader, BoxType, Error, FixedPointU8, FtypBox, HdlrBox, Hev1Box, MediaConfig,
    MoofBox, MoovBox, Mp4Config, Mp4Sample, Mp4aBox, MvexBox, OpusBox, SmhdBox, StcoBox, TfdtBox,
    TfhdBox, TrackConfig, TrackType, TrafBox, TrakBox, TrexBox, TrunBox, Tx3gBox, VmhdBox, Vp09Box,
    WriteBox, HEADER_SIZE,
};

pub struct Fmp4Writer {
    cursor: Cursor<Vec<u8>>,

    next_track_id: u32,
    leading_track_id: u32,
    tracks: Vec<Fmp4TrackWriter>,

    fragment_sequence: u32,
    ftyp: FtypBox,
    moov: MoovBox,
}

impl Fmp4Writer {
    pub fn new(config: &Mp4Config) -> Result<Self, Error> {
        let ftyp = FtypBox {
            major_brand: config.major_brand,
            minor_version: config.minor_version,
            compatible_brands: config.compatible_brands.clone(),
        };

        let mut moov = MoovBox::default();
        moov.mvhd.timescale = config.timescale;
        Ok(Self {
            cursor: Cursor::new(Vec::<u8>::new()),
            next_track_id: 1,
            tracks: Vec::new(),
            fragment_sequence: 1,
            leading_track_id: 0,
            ftyp,
            moov,
        })
    }

    pub fn add_track(&mut self, config: &TrackConfig) -> Result<(), Error> {
        let track_id = self.tracks.len() as u32 + 1;
        let mut track = Fmp4TrackWriter::new(track_id, config)?;
        if self.leading_track_id == 0 || config.track_type == TrackType::Video {
            self.leading_track_id = track_id;
        }
        self.next_track_id += 1;
        self.moov.mvhd.next_track_id = self.next_track_id;

        // trak box
        let trak = track.trak_box();
        self.moov.traks.push(trak);

        // trex box
        if self.moov.mvex.is_none() {
            self.moov.mvex = Some(MvexBox::default());
        }
        let trex = track.trex_box();
        self.moov.mvex.as_mut().unwrap().trexs.push(trex);

        self.tracks.push(track);
        Ok(())
    }

    pub fn get_cursor(&mut self) -> &mut Cursor<Vec<u8>> {
        &mut self.cursor
    }

    pub fn write_header<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        self.ftyp.write_box(writer)?;
        self.moov.write_box(writer)?;
        Ok(())
    }

    pub fn write_end<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        if self.leading_track_id == 0 {
            return Err(Error::TrakNotFound(self.leading_track_id));
        }

        let mut moof: MoofBox = MoofBox::default();
        moof.mfhd.sequence_number = self.fragment_sequence;
        self.fragment_sequence += 1;
        let mut parts = vec![];
        for track in self.tracks.iter_mut() {
            let mut part = track.fragment_part();

            let mut traf = part.traf_box();
            if let Some(ref mut trun) = traf.trun {
                trun.data_offset = Some(0); // set after mdat
                trun.flags |= TrunBox::FLAG_DATA_OFFSET
            }

            parts.push(part);

            moof.trafs.push(traf);
        }

        // calculate mdat_pos
        let mut mdat_offset = moof.get_size() + 8;
        let mut mdat_size = 0;
        for (i, payload) in parts.iter_mut().enumerate() {
            let traf = moof.trafs.get_mut(i).unwrap();
            if let Some(ref mut trun) = traf.trun {
                trun.data_offset = Some(mdat_offset as i32); // set after mdat
            }
            let payload = payload.payload();
            mdat_offset += payload.len() as u64;
            mdat_size += payload.len() as u64;
        }

        moof.write_box(writer)?;
        if parts.len() > 0 {
            BoxHeader::new(BoxType::MdatBox, HEADER_SIZE + mdat_size as u64).write(writer)?;
        }
        for part in parts.iter_mut() {
            let payload = part.payload();
            writer.write_all(&payload)?;
        }

        Ok(())
    }

    pub fn write_sample(&mut self, track_id: u32, sample: &Mp4Sample) -> Result<(), Error> {
        if track_id == 0 || track_id >= self.next_track_id {
            return Err(Error::TrakNotFound(track_id));
        }
        let Some(track) = self.tracks.get_mut(track_id as usize - 1) else {
            return Err(Error::TrakNotFound(track_id));
        };

        track.write_sample(sample)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Fmp4TrackWriter {
    track_type: TrackType,
    timescale: u32,

    trak: TrakBox,
    trex: TrexBox,
    traf: TrafBox,

    prev_track_sample_count: u32,
    prev_track_duration: u32,

    track_sample_count: u32,
    track_duration: u32,
    track_buffers: BytesMut,

    default_sample_size: u32,
    default_sample_duration: u32,
}

impl Fmp4TrackWriter {
    pub fn new(track_id: u32, config: &TrackConfig) -> Result<Self, Error> {
        let mut trak = TrakBox::default();
        let mut trex = TrexBox::default();
        let mut traf = TrafBox::default();
        trex.track_id = track_id;
        trak.tkhd.track_id = track_id;
        traf.tfhd.track_id = track_id;
        if config.track_type == TrackType::Audio {
            trak.tkhd.alternate_group = 1; // for audio, alternate_group is 1
            trak.tkhd.volume = FixedPointU8::new(1);
        }
        trak.mdia.mdhd.timescale = config.timescale;
        trak.mdia.mdhd.language = config.language.to_owned();
        trak.mdia.hdlr = HdlrBox::new(
            config.track_type.into(),
            config.track_type.to_handle_name().to_string(),
        );
        trak.mdia.minf.stbl.stco = Some(StcoBox::default());
        match config.media_conf {
            MediaConfig::AvcConfig(ref avc_config) => {
                trak.tkhd.set_width(avc_config.width);
                trak.tkhd.set_height(avc_config.height);

                let vmhd = VmhdBox::default();
                trak.mdia.minf.vmhd = Some(vmhd);

                let avc1 = Avc1Box::new(avc_config);
                trak.mdia.minf.stbl.stsd.avc1 = Some(avc1);
            }
            MediaConfig::HevcConfig(ref hevc_config) => {
                trak.tkhd.set_width(hevc_config.width);
                trak.tkhd.set_height(hevc_config.height);

                let vmhd = VmhdBox::default();
                trak.mdia.minf.vmhd = Some(vmhd);

                let hev1 = Hev1Box::new(hevc_config);
                trak.mdia.minf.stbl.stsd.hev1 = Some(hev1);
            }
            MediaConfig::Vp9Config(ref config) => {
                trak.tkhd.set_width(config.width);
                trak.tkhd.set_height(config.height);

                trak.mdia.minf.stbl.stsd.vp09 = Some(Vp09Box::new(config));
            }
            MediaConfig::AacConfig(ref aac_config) => {
                let smhd = SmhdBox::default();
                trak.mdia.minf.smhd = Some(smhd);

                let mp4a = Mp4aBox::new(aac_config);
                trak.mdia.minf.stbl.stsd.mp4a = Some(mp4a);
            }
            MediaConfig::TtxtConfig(ref _ttxt_config) => {
                let tx3g = Tx3gBox::default();
                trak.mdia.minf.stbl.stsd.tx3g = Some(tx3g);
            }
            MediaConfig::OpusConfig(ref _opus_config) => {
                let smhd = SmhdBox::default();
                trak.mdia.minf.smhd = Some(smhd);

                let opus = OpusBox::default();
                trak.mdia.minf.stbl.stsd.opus = Some(opus);
            }
        }

        Ok(Self {
            track_type: config.track_type,
            timescale: config.timescale,
            trak,
            trex,
            traf,
            track_buffers: BytesMut::new(),
            prev_track_sample_count: 0,
            prev_track_duration: 0,
            track_sample_count: 0,
            track_duration: 0,
            default_sample_size: 0,
            default_sample_duration: 0,
        })
    }

    fn track_type(&self) -> TrackType {
        self.track_type
    }

    // fn get_sum_of_playtime_ms(&self) -> u32 {
    //     (self.track_duration * 1000) / self.timescale
    // }

    fn trak_box(&mut self) -> TrakBox {
        self.trak.clone()
    }

    fn trex_box(&mut self) -> TrexBox {
        self.trex.clone()
    }

    fn fragment_part(&mut self) -> Fmp4Part {
        let part = Fmp4Part {
            track_type: self.track_type(),
            traf: self.traf.clone(),
            chunk_buffer: std::mem::replace(&mut self.track_buffers, BytesMut::new()),
            track_duration: self.prev_track_duration,
            track_sample_count: self.track_sample_count,
            default_sample_size: self.default_sample_size,
            default_sample_duration: self.default_sample_duration,
        };

        self.traf.trun = None;
        self.prev_track_sample_count = self.track_sample_count;
        self.prev_track_duration = self.track_duration;

        part
    }

    pub fn write_sample(&mut self, sample: &Mp4Sample) -> Result<(), Error> {
        if self.track_buffers.len() == 0 {
            self.default_sample_size = sample.bytes.len() as u32;
            self.default_sample_duration = sample.duration;
        }

        self.track_buffers.extend_from_slice(&sample.bytes);
        self.track_sample_count += 1;
        self.track_duration += sample.duration;

        self.update_samples(sample);

        Ok(())
    }

    fn update_samples(&mut self, sample: &Mp4Sample) {
        if self.traf.trun.is_none() {
            self.traf.trun = Some(TrunBox::default());
        }
        let Some(ref mut trun) = self.traf.trun else {
            return;
        };
        if self.default_sample_duration != sample.duration {
            trun.flags |= TrunBox::FLAG_SAMPLE_DURATION;
        }
        if self.default_sample_size != sample.bytes.len() as u32 {
            trun.flags |= TrunBox::FLAG_SAMPLE_SIZE;
        }
        trun.sample_durations.push(sample.duration);
        trun.sample_sizes.push(sample.bytes.len() as u32);
        trun.sample_cts.push(sample.start_time as u32);
        trun.sample_count += 1;
    }
}

pub struct Fmp4Part {
    track_type: TrackType,
    traf: TrafBox,

    chunk_buffer: BytesMut,

    track_duration: u32,
    track_sample_count: u32,

    default_sample_size: u32,
    default_sample_duration: u32,
}

impl Fmp4Part {
    fn traf_box(&mut self) -> TrafBox {
        self.traf.tfhd.set_base_data_offset(0);
        self.traf
            .tfhd
            .set_default_sample_size(self.default_sample_size);
        self.traf
            .tfhd
            .set_default_sample_duration(self.default_sample_duration);
        // self.traf.tfhd.set_default_sample_flags(0x200_000); // todo check
        self.traf.tfdt = Some(TfdtBox::new(self.track_duration as u64));

        self.traf.clone()
    }

    fn write_mdat<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        BoxHeader::new(BoxType::MdatBox, HEADER_SIZE).write(writer)?;
        writer.write_all(&self.chunk_buffer)?;

        Ok(())
    }

    fn payload(&mut self) -> &BytesMut {
        return &self.chunk_buffer;
    }
}
