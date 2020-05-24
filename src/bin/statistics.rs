use std::fs;
use std::fs::{File};
use std::path::Path;
use nom::lib::std::fmt::{Formatter, Error};
use std::io::Read;
use dicom::Tag;
use std::any::Any;

struct Results {
    number_of_frames: i32,
    bits_allocated: u16,
    bits_stored: u16,
    window_center: String,
    window_width: String,
}

impl std::fmt::Display for Results {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{},{},{},{},{}", self.number_of_frames, self.bits_allocated, self.bits_stored, self.window_center, self.window_width)
    }
}

fn get_results<P: AsRef<Path>>(path: P) -> Result<Results, String> {
    let mut file = File::open(path).unwrap();
    let mut content = vec![];
    file.read_to_end(&mut content).unwrap();

    let mut parser = dicom::parser::obj::Parser::new(false);
    let res = parser.parse_object(&content);
    let obj = res.map_err(|e| format!("{}", e))?;

    let number_of_frames = obj.try_get::<i32>(Tag::x0028x0008).unwrap_or(1);
    let bits_allocated = obj.try_get::<u16>(Tag::x0028x0100).map_err(|e| format!("{}", e))?;
    let bits_stored = obj.try_get::<u16>(Tag::x0028x0101).map_err(|e| format!("{}", e))?;
    let window_center = obj.try_get::<String>(Tag::x0028x1050).map_err(|e| format!("{}", e))?;
    let window_width = obj.try_get::<String>(Tag::x0028x1051).map_err(|e| format!("{}", e))?;

    Ok(Results {
        window_width, window_center,
        bits_stored,
        bits_allocated, number_of_frames
    })
}

fn main() {
    let dir_name = std::env::args().nth(1).unwrap();
    println!("Will parse folder {}", dir_name);

    let mut results = vec![];
    let mut errors = vec![];
    for file in fs::read_dir(dir_name).unwrap() {
        if let Ok(entry) = file {
            if let Ok(ft) = entry.file_type() {
                if ft.is_file() && !entry.path().ends_with("zip") {
                    match get_results(entry.path()) {
                        Ok(r) => results.push(r),
                        Err(e) => errors.push(e)
                    }
                }
            }
        }
    }

    println!("RESULTS\nNumber of frames,Bits allocated, bits stored, window center, window width");
    for r in results {
        println!("{}", r);
    }

    println!("Errors:\n{:?}", errors);
}