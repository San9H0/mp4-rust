use std::collections::HashMap;
use std::io::{Read, Seek};
use std::time::Duration;

use mdat::MdatBox;

use crate::meta::MetaBox;
use crate::*;

#[derive(Debug)]
pub struct Mp4Reader<R> {
    reader: R,
    pub mp4: Mp4Container,
    // pub ftyp: FtypBox,
    // pub moov: MoovBox,
    // pub moofs: Vec<MoofBox>,
    // pub emsgs: Vec<EmsgBox>,
    // tracks: HashMap<u32, Mp4Track>,
    // size: u64,
}

impl<R: Read + Seek> Mp4Reader<R> {
    pub fn read_header(mut reader: R, size: u64) -> Result<Self> {
        let start = reader.stream_position()?;

        let mut ftyp = None;
        let mut moov = None;
        let mut mdat = None;
        let mut moofs = Vec::new();
        let mut moof_offsets = Vec::new();
        let mut emsgs = Vec::new();

        let mut current = start;
        while current < size {
            // Get box header.
            let header = BoxHeader::read(&mut reader)?;
            let BoxHeader { name, size: s } = header;
            if s > size {
                return Err(Error::InvalidData(
                    "file contains a box with a larger size than it",
                ));
            }

            // Break if size zero BoxHeader, which can result in dead-loop.
            if s == 0 {
                break;
            }

            println!("name: {:?}, size: {}", name, s);

            // Match and parse the atom boxes.
            match name {
                BoxType::FtypBox => {
                    ftyp = Some(FtypBox::read_box(&mut reader, s)?);
                }
                BoxType::FreeBox => {
                    skip_box(&mut reader, s)?;
                }
                BoxType::MdatBox => {
                    println!("mdat box");
                    mdat = Some(MdatBox::read_box(&mut reader, s)?);
                }
                BoxType::MoovBox => {
                    moov = Some(MoovBox::read_box(&mut reader, s)?);
                }
                BoxType::MoofBox => {
                    let moof_offset = reader.stream_position()? - 8;
                    let moof = MoofBox::read_box(&mut reader, s)?;
                    moofs.push(moof);
                    moof_offsets.push(moof_offset);
                }
                BoxType::EmsgBox => {
                    let emsg = EmsgBox::read_box(&mut reader, s)?;
                    emsgs.push(emsg);
                }
                _ => {
                    // XXX warn!()
                    skip_box(&mut reader, s)?;
                }
            }
            current = reader.stream_position()?;
        }

        // let size = current - start;
        // let mut tracks = if let Some(ref moov) = moov {
        //     if moov.traks.iter().any(|trak| trak.tkhd.track_id == 0) {
        //         return Err(Error::InvalidData("illegal track id 0"));
        //     }
        //     moov.traks
        //         .iter()
        //         .map(|trak| (trak.tkhd.track_id, Mp4Track::from(trak)))
        //         .collect()
        // } else {
        //     HashMap::new()
        // };

        // // Update tracks if any fragmented (moof) boxes are found.f
        // if !moofs.is_empty() {
        //     for (moof, moof_offset) in moofs.iter().zip(moof_offsets) {
        //         for traf in moof.trafs.iter() {
        //             let track_id = traf.tfhd.track_id;
        //             if let Some(track) = tracks.get_mut(&track_id) {
        //                 let mut default_sample_duration = 0;
        //                 if let Some(ref moov) = moov {
        //                     if let Some(ref mvex) = &moov.mvex {
        //                         for trex in mvex.trexs.iter() {
        //                             if trex.track_id == track_id {
        //                                 default_sample_duration = trex.default_sample_duration;
        //                                 break;
        //                             }
        //                         }
        //                     }
        //                 }

        //                 track.default_sample_duration = default_sample_duration;
        //                 track.moof_offsets.push(moof_offset);
        //                 track.trafs.push(traf.clone())
        //             } else {
        //                 println!("4 track_id: {}", track_id);
        //                 return Err(Error::TrakNotFound(track_id));
        //             }
        //         }
        //     }
        // }

        Ok(Mp4Reader {
            reader,
            mp4: Mp4Container {
                ftyp,
                moov,
                moofs,
                emsgs,
                mdat,
            },
            // ftyp: ftyp.unwrap(),
            // moov: moov.unwrap(),
            // moofs,
            // emsgs,
            // size,
            // tracks,
        })
    }

    pub fn read_fragment_header<FR: Read + Seek>(
        &self,
        mut reader: FR,
        size: u64,
    ) -> Result<Mp4Reader<FR>> {
        let start = reader.stream_position()?;

        let mut mdat = None;
        let mut moofs = Vec::new();
        let mut moof_offsets = Vec::new();

        let mut current = start;
        while current < size {
            // Get box header.
            let header = BoxHeader::read(&mut reader)?;
            let BoxHeader { name, size: s } = header;
            if s > size {
                return Err(Error::InvalidData(
                    "file contains a box with a larger size than it",
                ));
            }

            // Break if size zero BoxHeader, which can result in dead-loop.
            if s == 0 {
                break;
            }

            // Match and parse the atom boxes.
            match name {
                BoxType::MdatBox => {
                    mdat = Some(MdatBox::read_box(&mut reader, s)?);
                }
                BoxType::MoofBox => {
                    let moof_offset = reader.stream_position()? - 8;
                    let moof = MoofBox::read_box(&mut reader, s)?;
                    moofs.push(moof);
                    moof_offsets.push(moof_offset);
                }
                _ => {
                    // XXX warn!()
                    skip_box(&mut reader, s)?;
                }
            }
            current = reader.stream_position()?;
        }

        // if moofs.is_empty() {
        //     return Err(Error::BoxNotFound(BoxType::MoofBox));
        // }

        // let size = current - start;

        // let mut tracks = HashMap::new();
        // if let Some(ref moov) = self.mp4.moov {
        //     tracks = self
        //         .mp4
        //         .moov
        //         .as_ref()
        //         .unwrap()
        //         .traks
        //         .iter()
        //         .map(|trak| (trak.tkhd.track_id, Mp4Track::from(trak)))
        //         .collect();
        // }

        // for (moof, moof_offset) in moofs.iter().zip(moof_offsets) {
        //     for traf in moof.trafs.iter() {
        //         let track_id = traf.tfhd.track_id;
        //         if let Some(track) = tracks.get_mut(&track_id) {
        //             let mut default_sample_duration = 0;
        //             if let Some(ref mvex) = &self.mp4.moov.as_ref().unwrap().mvex {
        //                 for trex in mvex.trexs.iter() {
        //                     if trex.track_id == track_id {
        //                         default_sample_duration = trex.default_sample_duration;
        //                         break;
        //                     }
        //                 }
        //             }

        //             track.default_sample_duration = default_sample_duration;
        //             track.moof_offsets.push(moof_offset);
        //             track.trafs.push(traf.clone())
        //         } else {
        //             println!("5 track_id: {}", track_id);
        //             return Err(Error::TrakNotFound(track_id));
        //         }
        //     }
        // }

        Ok(Mp4Reader {
            reader,
            mp4: Mp4Container {
                ftyp: self.mp4.ftyp.clone(),
                moov: self.mp4.moov.clone(),
                moofs,
                emsgs: Vec::new(),
                mdat,
            },
            // tracks,
            // size,
        })
    }

    // pub fn size(&self) -> u64 {
    //     self.size
    // }

    pub fn container(&self) -> &Mp4Container {
        &self.mp4
    }

    // pub fn tracks(&self) -> &HashMap<u32, Mp4Track> {
    //     &self.tracks
    // }

    // pub fn sample_count(&self, track_id: u32) -> Result<u32> {
    //     if let Some(track) = self.tracks.get(&track_id) {
    //         Ok(track.sample_count())
    //     } else {
    //         println!("1 track_id: {}", track_id);
    //         Err(Error::TrakNotFound(track_id))
    //     }
    // }

    // pub fn read_sample(&mut self, track_id: u32, sample_id: u32) -> Result<Option<Mp4Sample>> {
    //     if let Some(track) = self.tracks.get(&track_id) {
    //         track.read_sample(&mut self.reader, sample_id)
    //     } else {
    //         println!("2 track_id: {}", track_id);
    //         Err(Error::TrakNotFound(track_id))
    //     }
    // }

    // pub fn sample_offset(&mut self, track_id: u32, sample_id: u32) -> Result<u64> {
    //     if let Some(track) = self.tracks.get(&track_id) {
    //         track.sample_offset(sample_id)
    //     } else {
    //         println!("3 track_id: {}", track_id);
    //         Err(Error::TrakNotFound(track_id))
    //     }
    // }
}

impl<R> Mp4Reader<R> {
    pub fn metadata(&self) -> impl Metadata<'_> {
        self.mp4
            .moov
            .as_ref()
            .unwrap()
            .udta
            .as_ref()
            .and_then(|udta| {
                udta.meta.as_ref().and_then(|meta| match meta {
                    MetaBox::Mdir { ilst } => ilst.as_ref(),
                    _ => None,
                })
            })
    }
}
