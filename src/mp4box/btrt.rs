use serde::Serialize;
use std::io::{Read, Seek, Write};

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Default, Eq, Serialize)]
pub struct BtrtBox {
    pub buffer_size_db: u32,
    pub max_bitrate: u32,
    pub avg_bitrate: u32,
}

impl BtrtBox {
    pub fn new(max_bitrate: u32, avg_bitrate: u32) -> Self {
        Self {
            buffer_size_db: 0,
            max_bitrate,
            avg_bitrate,
        }
    }

    pub fn get_type(&self) -> BoxType {
        BoxType::BtrtBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + 12
    }
}

impl Mp4Box for BtrtBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = String::new();
        Ok(s)
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for BtrtBox {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        let buffer_size_db = reader.read_u32::<BigEndian>()?;
        let max_bitrate = reader.read_u32::<BigEndian>()?;
        let avg_bitrate = reader.read_u32::<BigEndian>()?;

        skip_bytes_to(reader, start + size)?;

        Ok(BtrtBox {
            buffer_size_db,
            max_bitrate,
            avg_bitrate,
        })
    }
}

impl<W: Write> WriteBox<&mut W> for BtrtBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        let mut written = 0;
        written += BoxHeader::new(self.box_type(), size).write(writer)?;

        writer.write_u32::<BigEndian>(self.buffer_size_db)?;
        written += 4;
        writer.write_u32::<BigEndian>(self.max_bitrate)?;
        written += 4;
        writer.write_u32::<BigEndian>(self.avg_bitrate)?;
        written += 4;

        if written != size {
            return Err(Error::InvalidData("btrt box size mismatch"));
        }

        Ok(written)
    }
}
