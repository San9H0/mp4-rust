use std::io::Write;

use mp4::{FourCC, FtypBox, Mp4Box, Mp4Writer, WriteBox};

fn main() {
    let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
    let ftyp_box = mp4::FtypBox {
        major_brand: FourCC::from(*b"mp42"),
        minor_version: 1,
        compatible_brands: vec![
            FourCC::from(*b"mp41"),
            FourCC::from(*b"mp42"),
            FourCC::from(*b"isom"),
            FourCC::from(*b"hlsf"),
        ],
    };
    ftyp_box.write_box(&mut cursor).unwrap();

    // let moov = mp4::MoovBox {
    //     mvhd: mp4::MvhdBox {
    //         ..Default::default()
    //     },
    //     meta: None,
    //     mvex: None,
    //     traks: vec![mp4::TrakBox {
    //         tkhd: mp4::TkhdBox {
    //             track_id: 1,
    //             ..Default::default()
    //         },
    //         mdia: mp4::MdiaBox {
    //             mdhd: mp4::MdhdBox {
    //                 timescale: 48000,
    //                 language: "und".to_string(),
    //                 ..Default::default()
    //             },
    //             hdlr: mp4::HdlrBox {
    //                 handler_type: FourCC::from(*b"soun"),
    //                 name: "SoundHandler".to_string(),
    //                 ..Default::default()
    //             },
    //             minf: mp4::MinfBox {
    //                 smhd: Some(mp4::SmhdBox {
    //                     ..Default::default()
    //                 }),
    //                 dinf: mp4::DinfBox {
    //                     ..Default::default()
    //                 },
    //                 stbl: mp4::StblBox {
    //                     stsd: mp4::StsdBox { entry_count: 1 },
    //                     ..Default::default()
    //                 },
    //             },
    //         },
    //         edts: None,
    //         meta: None,
    //     }],
    //     udta: None,
    // };
    // moov.write_box(&mut cursor).unwrap();

    let data = cursor.into_inner();

    let path = std::path::Path::new("output.mp4");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .unwrap();

    file.write_all(&data).unwrap();
}
