use std::env;
use std::fs::File;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: simple <filename>");
        std::process::exit(1);
    }

    let filename = &args[1];
    let f = File::open(filename).unwrap();
    let mp4 = mp4::read_mp4(f).unwrap();
    let container = mp4.container();
    let ftyp = container.ftyp.as_ref().unwrap();

    println!("Major Brand: {}", ftyp.major_brand());

    for track in mp4.tracks().values() {
        println!(
            "Track: #{}({}) {} {}",
            track.track_id(),
            track.language(),
            track.track_type().unwrap(),
            track.box_type().unwrap(),
        );
    }
}
