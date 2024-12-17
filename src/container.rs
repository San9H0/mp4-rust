use std::{fs::File, io::BufReader};

use mdat::MdatBox;

use crate::*;
use crate::{EmsgBox, FtypBox, MoofBox, MoovBox, Mp4Reader};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mp4Container {
    pub ftyp: Option<FtypBox>,
    pub moov: Option<MoovBox>,
    pub moofs: Vec<MoofBox>,
    pub emsgs: Vec<EmsgBox>,
    pub mdat: Option<MdatBox>,
}

impl Mp4Container {
    // pub fn new(reader: BufReader<File>) -> Result<Self> {
    //     let mp4 = Mp4Reader::read_header(reader, size)?;
    //     Ok(mp4)
    // }
    pub fn is_fragmented(&self) -> bool {
        !self.moofs.is_empty()
    }
}
