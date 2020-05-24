use dicom::Tag;
use std::fs::File;
use std::io::Read;
use std::time::Instant;

fn main() {
    pretty_env_logger::init();
    let filename = std::env::args().nth(1).unwrap();
    let parse_image = std::env::args().nth(2).unwrap();
    let save_to = std::env::args().nth(3).unwrap();
    let parse_image: bool = parse_image.parse().unwrap();
    let mut file = File::open(filename).unwrap();
    let mut content = vec![];
    file.read_to_end(&mut content).unwrap();

    let now = Instant::now();
    let mut parser = dicom::parser::obj::Parser::new(parse_image);
    let res = parser.parse_object(&content);
    let time_to_parse = Instant::now() - now;
    let now = Instant::now();

    let obj = res.unwrap();
    //println!("{:?}", obj);

    println!("Bits allocated {:?}", obj.get::<u16>(Tag::x0028x0100));
    println!("Bits stored {:?}", obj.get::<u16>(Tag::x0028x0101));
    println!("Window Center {:?}", obj.get::<String>(Tag::x0028x1050));
    println!("Window Width {:?}", obj.get::<String>(Tag::x0028x1051));

    if let Some(img) = obj.image {
        img.save(save_to).unwrap();
    }
    let time_to_save = Instant::now() - now;
    //println!("{:?}", res);

    println!("Time to parse = {:?}", time_to_parse);
    println!("Time to save = {:?}", time_to_save);
}
