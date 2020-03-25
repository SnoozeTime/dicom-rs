use nom::number::Endianness;
use nom::IResult;
use image::{ImageBuffer, GrayImage, Luma};
use crate::img::{DicomImage, Gray16Image};
use crate::parser::{parse_u16, parse_tag, parse_vr, parse_length};
use crate::{Tag, TransferSyntax, types::CompressionScheme};
use nom::combinator::cond;
use log::debug;

pub(crate) fn parse_image(buf: &[u8], transfer_syntax: TransferSyntax, rows: u16, columns: u16, bits_allocated: u16, bits_stored: u16) -> IResult<&[u8], DicomImage>{
    // First need to consume the tag, vr and length.
    debug!("Parse image: Rows {} Cols {}, Bits (allocated: {}/Stored {})", rows, columns, bits_allocated, bits_stored);
    let (buf, tag) = parse_tag(buf, transfer_syntax.endianness())?;
    assert!(tag == Tag::x7FE0x0010);
    let (buf, vr) = cond(transfer_syntax.is_vr_explicit(), parse_vr)(buf)?;
    let (buf, _) = parse_length(buf, &vr, transfer_syntax.endianness())?;

    if let Some(CompressionScheme::Jpeg2000Lossless) = transfer_syntax.compression_scheme {
        debug!("Image is in JPEG2000 format.");
        return Ok((&[], DicomImage::Jpeg2000 { image: buf.to_vec() }))
    }

    debug!("Will parse {} bytes", columns as u32 * rows as u32 * bits_allocated as u32 /2);
    debug!("Remaining length of buffer = {}", buf.len());
    // Depending on bits allocated, we need to read either 8 or 16 bytes.
    match bits_allocated {
        8 => {
            //assert_eq!(rows as u32 *columns as u32 , length);
            let (rest, image) = parse_img_u8(buf, rows, columns)?;
            Ok((rest, DicomImage::Grayscale8 { image }))
        }
        16 => {
            //assert_eq!(rows as u32 *columns as u32, length/2);
            let (rest, image) = parse_img_u16(buf, transfer_syntax.endianness(), rows, columns, bits_allocated, bits_stored)?;
            Ok((rest, DicomImage::Grayscale16 { image }))
        }
        _ => panic!("Bits allocated not supported yet = {}", bits_allocated)
    }
}

fn parse_img_u8(buf: &[u8], rows: u16, columns: u16) -> IResult<&[u8], GrayImage> {
    let mut img = ImageBuffer::new(columns as u32, rows as u32);
    let mut current_buf = buf;
    for y in 0..rows {
        for x in 0..columns {
            let (rest, grey_value) = nom::number::complete::be_u8(current_buf)?;
            let pixel = img.get_pixel_mut(x as u32, y as u32);
            *pixel = Luma([grey_value]);
            current_buf = rest;
        }
    }
    Ok((current_buf, img))
}

fn parse_img_u16(buf: &[u8], endian: Endianness, rows: u16, columns: u16, bits_allocated: u16, bits_stored: u16) -> IResult<&[u8], Gray16Image> {
    let mut img = ImageBuffer::new(columns as u32, rows as u32);
    let mut current_buf = buf;

    for y in 0..rows {
        for x in 0..columns {
            let (rest, grey_value) = parse_u16(current_buf, endian)?;

            let pixel = img.get_pixel_mut(x as u32, y as u32);
            if bits_stored != 16 {
                let diff = bits_allocated - bits_stored;
                let mut mask = 0u16;
                for _ in 0..diff {
                    mask = (mask << 1) | 0b1;
                }
                let mask = mask << bits_stored;

                let left: u16 = grey_value << diff;
                let left = left | (left & mask) >> bits_stored;
                *pixel = Luma([left]);
            } else {
                *pixel = Luma([grey_value]);
            }

            current_buf = rest;
        }
    }

    Ok((current_buf, img))
}
//
//fn parse_imgbuf_u8<T>(reader: &mut T, rows: u16, columns: u16) -> DicomResult<GrayImage>
//where
//    T: Read,
//{
//    let mut img = ImageBuffer::new(columns as u32, rows as u32);
//    for y in 0..rows {
//        for x in 0..columns {
//            let grey_value: u8 = reader.read_u8()?;
//            let pixel = img.get_pixel_mut(x as u32, y as u32);
//            *pixel = Luma([grey_value]);
//        }
//    }
//
//    Ok(img)
//}
//fn parse_imgbuf<T>(
//    reader: &mut T,
//    endianness: Endian,
//    rows: u16,
//    columns: u16,
//    bits_allocated: u16,
//    bits_stored: u16,
//) -> DicomResult<Gray16Image>
//where
//    T: Read,
//{
//    let mut img = ImageBuffer::new(columns as u32, rows as u32);
//    for y in 0..rows {
//        for x in 0..columns {
//            let grey_value = if let Endian::LE = endianness {
//                reader.read_u16::<LittleEndian>()?
//            } else {
//                reader.read_u16::<BigEndian>()?
//            };
//
//            let pixel = img.get_pixel_mut(x as u32, y as u32);
//            if bits_stored != 16 {
//                let diff = bits_allocated - bits_stored;
//                let mut mask = 0u16;
//                for _ in 0..diff {
//                    mask = (mask << 1) | 0b1;
//                }
//                let mask = mask << bits_stored;
//
//                let left: u16 = grey_value << diff;
//                let left = left | (left & mask) >> bits_stored;
//                *pixel = Luma([left]);
//            } else {
//                *pixel = Luma([grey_value]);
//            }
//        }
//    }
//
//    Ok(img)
//}

