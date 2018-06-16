extern crate srszip;
use std::path::Path;

use std::io;
use std::io::{Read, Write};

fn main() {
    std::process::exit(real_main());
}

fn real_main() -> i32 {
    let mut args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} ZIPFILE FILE ...", args[0]);
        return 1;
    }

    args.remove(0);
    let zipfile = args.remove(0);

    match zip(&zipfile, &args) {
        Ok(_) => println!("Success"),
        Err(e) => {
            println!("Error: {:?}", e);
            return 1;
        }
    }

    0
}

fn add_file_and_directory<W: Write + io::Seek>(
    zip: &mut srszip::archive::ZipWriter<W>,
    path: &str,
) -> srszip::archive::ZipResult<()> {
    let mut dirs = Vec::new();
    let mut sub_path = Path::new(path);

    while let Some(x) = sub_path.parent() {
        sub_path = x;
        if x.to_str() == Some("") {
            continue;
        }
        dirs.push(sub_path);
    }

    dirs.reverse();

    for dir in &dirs {
        try!(zip.add_directory(dir.to_str().unwrap().to_owned() + "/"));
    }

    let mut file = std::fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    try!(zip.add_file(path, buffer.as_ref()));

    Ok(())
}

fn zip(zipfile: &str, paths: &Vec<std::string::String>) -> srszip::archive::ZipResult<()> {
    let zipfile = std::fs::File::create(&zipfile).unwrap();

    let mut zip = srszip::archive::ZipWriter::new(zipfile);

    for path in paths {
        try!(add_file_and_directory(&mut zip, &path));
    }

    Ok(())
}
