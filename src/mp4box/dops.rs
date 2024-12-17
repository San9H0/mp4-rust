use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use serde::Serialize;
use std::io::{Read, Seek, Write};

use crate::mp4box::*;

// #[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
// pub struct DopsBox {
//     pub version: u8,
//     pub output_channel_count: u8,
//     pub pre_skip: u16,
//     pub input_sample_rate: u32,
//     pub output_gain: u16,
//     pub channel_mapping_family: u8,
//     pub stream_count: u8,
//     pub couped_count: u8,
//     pub channel_mapping: Vec<u8>,
// }

// impl DopsBox {
//     pub fn new() -> Self {
//         Self {
//             version: 0,
//             output_channel_count: 0,
//             pre_skip: 0,
//             input_sample_rate: 0,
//             output_gain: 0,
//             channel_mapping_family: 0,
//             stream_count: 0,
//             couped_count: 0,
//             channel_mapping: vec![],
//         }
//     }
// }

// impl Mp4Box for DopsBox {
//     fn box_type(&self) -> BoxType {
//         BoxType::EsdsBox
//     }

//     fn box_size(&self) -> u64 {
//         HEADER_SIZE + HEADER_EXT_SIZE + 1
//         // + size_of_length(ESDescriptor::desc_size()) as u64
//         // + ESDescriptor::desc_size() as u64
//     }

//     fn to_json(&self) -> Result<String> {
//         Ok(serde_json::to_string(&self).unwrap())
//     }

//     fn summary(&self) -> Result<String> {
//         Ok(String::new())
//     }
// }

// impl<R: Read + Seek> ReadBox<&mut R> for DopsBox {
//     fn read_box(reader: &mut R, size: u64) -> Result<Self> {
//         let start = box_start(reader)?;

//         let (version, flags) = read_box_header_ext(reader)?;

//         let mut es_desc = None;

//         let mut current = reader.stream_position()?;
//         let end = start + size;
//         while current < end {
//             let (desc_tag, desc_size) = read_desc(reader)?;
//             match desc_tag {
//                 0x03 => {
//                     es_desc = Some(ESDescriptor::read_desc(reader, desc_size)?);
//                 }
//                 _ => break,
//             }
//             current = reader.stream_position()?;
//         }

//         if es_desc.is_none() {
//             return Err(Error::InvalidData("ESDescriptor not found"));
//         }

//         skip_bytes_to(reader, start + size)?;

//         Ok(EsdsBox {
//             version,
//             flags,
//             es_desc: es_desc.unwrap(),
//         })
//     }
// }

// impl<W: Write> WriteBox<&mut W> for DopsBox {
//     fn write_box(&self, writer: &mut W) -> Result<u64> {
//         let size = self.box_size();
//         BoxHeader::new(self.box_type(), size).write(writer)?;

//         write_box_header_ext(writer, self.version, self.flags)?;

//         self.es_desc.write_desc(writer)?;

//         Ok(size)
//     }
// }
