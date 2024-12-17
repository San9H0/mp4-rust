use serde::Serialize;
use std::io::{Read, Seek, Write};

use crate::mp4box::*;
use crate::mp4box::{mfhd::MfhdBox, traf::TrafBox};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct MdatBox {
    data: Vec<u8>,
}

impl MdatBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::MdatBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + self.data.len() as u64
    }
}

impl Mp4Box for MdatBox {
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
        let s = format!("data={}", self.data.len());
        Ok(s)
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for MdatBox {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        println!("mdat start:{}, size: {}", start, size);
        let mut data = vec![0; (size - HEADER_SIZE) as usize];
        reader.read_exact(&mut data)?;

        skip_bytes_to(reader, start + size)?;

        Ok(MdatBox { data })
    }
}

impl<W: Write> WriteBox<&mut W> for MdatBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();

        let mut written = 0;
        written += BoxHeader::new(self.box_type(), size).write(writer)?;
        writer.write_all(&self.data)?;
        written += self.data.len() as u64;

        if size != written {
            return Err(Error::InvalidData("mdat box size mismatch"));
        }
        Ok(written)
    }
}
