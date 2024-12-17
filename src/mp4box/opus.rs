use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use serde::Serialize;
use std::io::{Read, Seek, Write};

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpusBox {
    pub data_reference_index: u16,
    pub channelcount: u16,
    pub samplesize: u16,

    #[serde(with = "value_u32")]
    pub samplerate: FixedPointU16,
    pub dops: Option<OpusSpecificBox>,
}

impl Default for OpusBox {
    fn default() -> Self {
        Self {
            data_reference_index: 0,
            channelcount: 2,
            samplesize: 16,
            samplerate: FixedPointU16::new(48000),
            dops: Some(OpusSpecificBox::default()),
        }
    }
}

impl OpusBox {
    // pub fn new(config: &AacConfig) -> Self {
    //     Self {
    //         data_reference_index: 1,
    //         channelcount: config.chan_conf as u16,
    //         samplesize: 16,
    //         samplerate: FixedPointU16::new(config.freq_index.freq() as u16),
    //         esds: Some(EsdsBox::new(config)),
    //         dops: None,
    //     }
    // }

    pub fn new() -> Self {
        Self {
            data_reference_index: 1,
            channelcount: 0,
            samplesize: 16,
            samplerate: FixedPointU16::new(48000),
            dops: Some(OpusSpecificBox::default()),
        }
    }

    pub fn get_type(&self) -> BoxType {
        BoxType::OpusBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + 8 + 20;
        if let Some(ref dops) = self.dops {
            size += dops.box_size();
        }
        size
    }
}

impl Mp4Box for OpusBox {
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
        let s = format!(
            "channel_count={} sample_size={} sample_rate={}",
            self.channelcount,
            self.samplesize,
            self.samplerate.value()
        );
        Ok(s)
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for OpusBox {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        reader.read_u32::<BigEndian>()?; // reserved
        reader.read_u16::<BigEndian>()?; // reserved
        let data_reference_index = reader.read_u16::<BigEndian>()?;

        let version = reader.read_u16::<BigEndian>()?;
        reader.read_u16::<BigEndian>()?; // reserved
        reader.read_u32::<BigEndian>()?; // reserved
        let channelcount = reader.read_u16::<BigEndian>()?;
        let samplesize = reader.read_u16::<BigEndian>()?;
        reader.read_u32::<BigEndian>()?; // pre-defined, reserved
        let samplerate = FixedPointU16::new_raw(reader.read_u32::<BigEndian>()?);

        if version == 1 {
            // Skip QTFF
            reader.read_u64::<BigEndian>()?;
            reader.read_u64::<BigEndian>()?;
        }

        // Find esds in mp4a or wave
        let mut dops = None;
        let end = start + size;
        loop {
            let current = reader.stream_position()?;
            if current >= end {
                break;
            }
            let header = BoxHeader::read(reader)?;
            let BoxHeader { name, size: s } = header;
            if s > size {
                return Err(Error::InvalidData(
                    "mp4a box contains a box with a larger size than it",
                ));
            }
            if name == BoxType::OpusSpecificBox {
                dops = Some(OpusSpecificBox::read_box(reader, s)?);
                break;
            } else {
                // Skip boxes
                let skip_to = current + s;
                skip_bytes_to(reader, skip_to)?;
            }
        }

        skip_bytes_to(reader, end)?;

        Ok(OpusBox {
            data_reference_index,
            channelcount,
            samplesize,
            samplerate,
            dops,
        })
    }
}

impl<W: Write> WriteBox<&mut W> for OpusBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;

        writer.write_u32::<BigEndian>(0)?; // reserved
        writer.write_u16::<BigEndian>(0)?; // reserved
        writer.write_u16::<BigEndian>(self.data_reference_index)?;

        writer.write_u64::<BigEndian>(0)?; // reserved
        writer.write_u16::<BigEndian>(self.channelcount)?;
        writer.write_u16::<BigEndian>(self.samplesize)?;
        writer.write_u32::<BigEndian>(0)?; // reserved
        writer.write_u32::<BigEndian>(self.samplerate.raw_value())?;

        if let Some(ref dops) = self.dops {
            dops.write_box(writer)?;
        }

        Ok(size)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
struct ChannelMappingTable {
    pub stream_count: u8,
    pub coupled_count: u8,
    pub channel_mapping: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct OpusSpecificBox {
    pub version: u8,
    pub output_channel_count: u8,
    pub pre_skip: u16,
    pub input_sample_rate: u32,
    pub outupt_gain: i16,
    pub channel_mapping_family: u8,
    pub channel_mapping_table: Option<ChannelMappingTable>,
}

impl Mp4Box for OpusSpecificBox {
    fn box_type(&self) -> BoxType {
        BoxType::DopsBox
    }

    fn box_size(&self) -> u64 {
        if let Some(cmt) = &self.channel_mapping_table {
            return HEADER_SIZE + 11 + 2 + cmt.channel_mapping.len() as u64;
        };
        return HEADER_SIZE + 11;
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        Ok(String::new())
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for OpusSpecificBox {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let version = reader.read_u8()?;
        if version != 0 {
            return Err(Error::InvalidData("opus specific box version is not 0"));
        }

        let output_channel_count = reader.read_u8()?;
        let pre_skip = reader.read_u16::<BigEndian>()?;
        let input_sample_rate = reader.read_u32::<BigEndian>()?;
        let outupt_gain = reader.read_i16::<BigEndian>()?;
        let channel_mapping_family = reader.read_u8()?;

        let channel_mapping_table = if channel_mapping_family == 0 {
            None
        } else {
            let stream_count = reader.read_u8()?;
            let coupled_count = reader.read_u8()?;
            let channel_mapping = read_buf(reader, output_channel_count.into())?;

            Some(ChannelMappingTable {
                stream_count,
                coupled_count,
                channel_mapping,
            })
        };

        Ok(OpusSpecificBox {
            version,
            output_channel_count,
            pre_skip,
            input_sample_rate,
            outupt_gain,
            channel_mapping_family,
            channel_mapping_table,
        })
    }
}

/// Read size bytes into a Vector or return error.
fn read_buf<T: Read>(src: &mut T, size: u64) -> Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(size as usize);
    let bytes_read = src.take(size).read_to_end(&mut buf)?;

    if bytes_read as u64 != size {
        return Err(Error::ReadBytesFailed(size));
    }

    Ok(buf)
}

impl<W: Write> WriteBox<&mut W> for OpusSpecificBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;

        writer.write_u8(self.version)?;
        writer.write_u8(self.output_channel_count)?;
        writer.write_u16::<BigEndian>(self.pre_skip)?;
        writer.write_u32::<BigEndian>(self.input_sample_rate)?;
        writer.write_i16::<BigEndian>(self.outupt_gain)?;
        writer.write_u8(self.channel_mapping_family)?;

        if let Some(cmt) = &self.channel_mapping_table {
            writer.write_u8(cmt.stream_count)?;
            writer.write_u8(cmt.coupled_count)?;
            writer.write_all(&cmt.channel_mapping)?;
        }

        Ok(size)
    }
}
