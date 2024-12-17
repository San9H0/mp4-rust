use std::future;
use std::io::{Seek, Write};

use opus::OpusBox;

use crate::Mp4Writer;
use crate::*;

pub enum Track {
    Video(VideoTrack),
    Audio(AudioTrack),
}

impl Track {
    // pub fn new(codec: Codec) -> Self {
    // match codec {
    //     Codec::H264(h264) => Self::Video(VideoTrack::new(h264)),
    //     Codec::Opus(opus) => Self::Audio(AudioTrack::new(opus)),
    // }
    // }

    pub fn get_trak_box(&self) -> TrakBox {
        match self {
            Track::Video(video) => video.get_trak_box(),
            Track::Audio(audio) => audio.get_trak_box(),
        }
    }
}

pub struct VideoTrack {
    pub id: u32,
    pub max_bitrate: u32,
    pub avg_bitrate: u32,

    pub width: u16,
    pub height: u16,

    pub codec: Codec,
}

impl VideoTrack {
    pub fn timescale(&self) -> u32 {
        90000
    }

    pub fn get_trak_box(&self) -> TrakBox {
        TrakBox {
            tkhd: TkhdBox {
                width: FixedPointU16::new(self.width),
                height: FixedPointU16::new(self.height),
                ..Default::default()
            },
            mdia: MdiaBox {
                mdhd: MdhdBox {
                    timescale: self.timescale(),
                    language: "und".to_string(),
                    ..Default::default()
                },
                hdlr: HdlrBox {
                    handler_type: str::parse::<FourCC>("vide").unwrap(),
                    name: String::from("VideoHandler"),
                    ..Default::default()
                },
                minf: MinfBox {
                    vmhd: Some(VmhdBox {
                        flags: 1,
                        ..Default::default()
                    }),
                    smhd: None,
                    dinf: DinfBox::default(),
                    stbl: self.codec.get_box(),
                },
            },
            ..Default::default()
        }
    }
}

pub struct AudioTrack {
    pub id: u32,
    pub max_bitrate: u32,
    pub avg_bitrate: u32,

    pub volume: u8,

    pub codec: Codec,
}

impl AudioTrack {
    pub fn timescale(&self) -> u32 {
        48000
    }

    pub fn get_trak_box(&self) -> TrakBox {
        TrakBox {
            tkhd: TkhdBox {
                volume: FixedPointU8::new(self.volume),
                alternate_group: 1,
                ..Default::default()
            },
            mdia: MdiaBox {
                mdhd: MdhdBox {
                    timescale: self.timescale(),
                    language: "und".to_string(),
                    ..Default::default()
                },
                hdlr: HdlrBox {
                    handler_type: str::parse::<FourCC>("soun").unwrap(),
                    name: String::from("SoundHandler"),
                    ..Default::default()
                },
                minf: MinfBox {
                    vmhd: None,
                    smhd: Some(SmhdBox::default()),
                    dinf: DinfBox::default(),
                    stbl: self.codec.get_box(),
                },
            },
            ..Default::default()
        }
    }
}

pub struct H264 {
    pub config: AvcConfig,
}

impl H264 {
    pub fn new(config: &AvcConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub fn get_box(&self) -> StblBox {
        StblBox {
            stsd: StsdBox {
                entry_count: 1,
                avc1: Some(Avc1Box {
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub struct Opus {}

impl Opus {
    pub fn get_box(&self) -> StblBox {
        StblBox {
            stsd: StsdBox {
                entry_count: 1,
                opus: Some(OpusBox {
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub enum Codec {
    H264(H264),
    Opus(Opus),
}

impl Codec {
    pub fn get_box(&self) -> StblBox {
        match self {
            Codec::H264(h264) => h264.get_box(),
            Codec::Opus(opus) => opus.get_box(),
        }
    }
}

pub struct Muxer {
    pub major_brand: FourCC,
    pub minor_version: u32,
    pub compatible_brands: Vec<FourCC>,

    pub tracks: Vec<Track>,
}

impl Muxer {
    pub fn new(major_brand: FourCC, minor_version: u32, compatible_brands: Vec<FourCC>) -> Self {
        Self {
            major_brand,
            minor_version,
            compatible_brands,
            tracks: Vec::new(),
        }
    }

    pub fn push_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn write_init<T>(&mut self, mut writer: T) -> Result<()>
    where
        T: Write + Seek,
    {
        let ftyp = FtypBox {
            major_brand: FourCC::from(self.major_brand),
            minor_version: self.minor_version,
            compatible_brands: self.compatible_brands.clone(),
        };
        ftyp.write_box(&mut writer)?;

        let mut traks = Vec::new();
        for track in self.tracks.iter() {
            traks.push(track.get_trak_box());
        }

        let mut moov = MoovBox {
            mvhd: MvhdBox {
                timescale: 1000,
                rate: FixedPointU16::new(1),
                volume: FixedPointU8::new(1),
                ..Default::default()
            },
            traks,
            // mvex: Some(MvexBox {
            //     mehd: None,
            //     trexs: vec![
            //         TrexBox {
            //             version: 0,
            //             flags: 0,
            //             track_id: 1,
            //             default_sample_description_index: 0,
            //             default_sample_duration: 0,
            //             default_sample_size: 0,
            //             default_sample_flags: 0,
            //         },
            //         TrexBox {
            //             version: 0,
            //             flags: 0,
            //             track_id: 2,
            //             default_sample_description_index: 0,
            //             default_sample_duration: 0,
            //             default_sample_size: 0,
            //             default_sample_flags: 0,
            //         },
            //     ],
            // }),
            ..Default::default()
        };
        println!("moov: {:?}", moov);
        moov.write_box(&mut writer)?;

        // for track in self.tracks.iter_mut() {
        //     moov.traks.push(track.write_end(&mut self.writer)?);
        // }

        // TODO largesize
        // let mdat_pos = writer.stream_position()?;
        // BoxHeader::new(BoxType::MdatBox, HEADER_SIZE).write(&mut writer)?;
        // BoxHeader::new(BoxType::WideBox, HEADER_SIZE).write(&mut writer)?;

        // let tracks = Vec::new();
        // let timescale = config.timescale;
        // let duration = 0;
        // Ok(Self {
        //     writer,
        //     tracks,
        //     mdat_pos,
        //     timescale,
        //     duration,
        // })
        Ok(())
    }
}
