use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::path::Path;

use mp4::{Mp4Box, Result, WriteBox};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: mp4dump <filename>");
        std::process::exit(1);
    }

    if let Err(err) = dump(&args[1]) {
        let _ = writeln!(io::stderr(), "{}", err);
    }
}

fn dump<P: AsRef<Path>>(filename: &P) -> Result<()> {
    let f = File::open(filename)?;

    let size = f.metadata()?.len();
    let reader = BufReader::new(f);
    let mp4: mp4::Mp4Reader<BufReader<File>> = mp4::Mp4Reader::read_header(reader, size)?;

    let mut writer = File::create("video.mp4")?;
    let container = mp4.container();
    for moof in container.moofs.iter() {
        let c = moof.write_box(&mut writer)?;
        println!("size: {}", c);
    }
    if let Some(mdat) = &container.mdat {
        let c = mdat.write_box(&mut writer)?;
        println!("size: {}", c);
    }

    Ok(())
}
