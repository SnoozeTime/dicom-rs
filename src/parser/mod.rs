//! All the functions to parse the DICOM.

use crate::{Tag, ValueRepresentation};
use nom::bytes::streaming::take;
use nom::character::streaming::one_of;
use nom::number::streaming::{be_u16, be_u32, le_u16, le_u32};
use nom::number::Endianness;
use nom::IResult;

mod element;
pub mod obj;
pub(crate) mod image;
pub mod sq;

/// Normal value of a data element is just a number of bytes.
fn parse_data(buf: &[u8], length: u32) -> IResult<&[u8], &[u8]> {
    take(length)(buf)
}

/// A tag is made of two u16: the group and the element.
///
/// some tags are known from the standard and added to the library.
fn parse_tag(buf: &[u8], endian: Endianness) -> IResult<&[u8], Tag> {
    let (rest, group) = parse_u16(buf, endian)?;
    let (rest, element) = parse_u16(rest, endian)?;
    Ok((rest, Tag::from_values(group, element)))
}

/// Parse a 4 bytes unsigned integer according to the endianness
fn parse_u32(buf: &[u8], endian: Endianness) -> IResult<&[u8], u32> {
    match endian {
        Endianness::Little => le_u32(buf),
        Endianness::Big => be_u32(buf),
    }
}

/// Parse a 2 bytes unsigned integer according to the endianness
fn parse_u16(buf: &[u8], endian: Endianness) -> IResult<&[u8], u16> {
    match endian {
        Endianness::Little => le_u16(buf),
        Endianness::Big => be_u16(buf),
    }
}

/// Value Representation is encoded as two characters (ascii).
fn parse_vr(buf: &[u8]) -> IResult<&[u8], ValueRepresentation> {
    let (rest, first_char) = one_of(VR_CHARS)(buf)?;
    let (rest, second_char) = one_of(VR_CHARS)(rest)?;
    Ok((
        rest,
        ValueRepresentation::from_chars(first_char, second_char),
    ))
}

/// Depending on whether there is a VR, the length is parsed differently:
/// - No VR => 4 bytes
/// - VR => normal case, 2 bytes,
///         special case, 2 bytes padding + 4 bytes of length.
fn parse_length<'buf>(
    buf: &'buf [u8],
    vr: &Option<ValueRepresentation>,
    endian: Endianness,
) -> IResult<&'buf [u8], u32> {
    match vr {
        Some(vr) => {
            if vr.has_special_length() {
                // in some VR cases, there is some padding before the actual length...
                let (buf, _padding) = parse_u16(buf, endian)?;
                parse_u32(buf, endian)
            } else {
                let (buf, length) = parse_u16(buf, endian)?;
                Ok((buf, length as u32))
            }
        }
        None => {
            // If no VR, length is 4 bytes.
            parse_u32(buf, endian)
        }
    }
}

const VR_CHARS: &str = "qwertyuiopasdfghjklzxcvbnmQWERTYUIOPASDFGHJKLZXCVBNM";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_known_vr() {
        let vr_str = "UL234".as_bytes();
        let res = parse_vr(vr_str);
        assert!(res.is_ok());

        let (rest, vr) = res.unwrap();
        assert_eq!(ValueRepresentation::UL, vr);
        assert_eq!(rest.len(), 3);
    }

    #[test]
    pub fn test_unknown_vr() {
        let vr_str = "ul123".as_bytes();
        let res = parse_vr(vr_str);
        assert!(res.is_ok());

        let (rest, vr) = res.unwrap();
        assert_eq!(ValueRepresentation::UNKNOWN("ul".to_string()), vr);
        assert_eq!(rest.len(), 3);
    }

    #[test]
    pub fn test_error() {
        let vr_str = "a1";
        let res = parse_vr(vr_str.as_bytes());
        assert!(res.is_err());
    }

    #[test]
    pub fn test_u16() {
        let nb = vec![0xE0, 0x12];
        let (_, parsed) = parse_u16(&nb, Endianness::Big).unwrap();
        assert_eq!(parsed, 0xE012);
        let (_, parsed) = parse_u16(&nb, Endianness::Little).unwrap();
        assert_eq!(parsed, 0x12E0);
    }

    #[test]
    pub fn test_u32() {
        let nb = vec![0xE0, 0x12, 0x01, 0x22];
        let (_, parsed) = parse_u32(&nb, Endianness::Big).unwrap();
        assert_eq!(parsed, 0xE0120122);
        let (_, parsed) = parse_u32(&nb, Endianness::Little).unwrap();
        assert_eq!(parsed, 0x220112E0);
    }

    #[test]
    pub fn parse_known_tag() {
        // x0028x0103
        let bytes = vec![0, 0x28, 0x01, 0x03];
        let (_, tag) = parse_tag(&bytes, Endianness::Big).unwrap();
        assert_eq!(tag, Tag::x0028x0103);
    }

    #[test]
    pub fn parse_unknown_tag() {
        let bytes = vec![0, 0, 0x01, 0x03];
        let (_, tag) = parse_tag(&bytes, Endianness::Big).unwrap();
        assert_eq!(tag, Tag::UNKNOWN(0, 0x0103));
    }

    #[test]
    pub fn parse_length_novr() {
        let bytes = vec![0x00, 0x10, 0x00, 0x03];
        let (_, length) = parse_length(&bytes, &None, Endianness::Big).unwrap();
        assert_eq!(length, 0x100003);
    }

    #[test]
    pub fn parse_length_normalvr() {
        let bytes = vec![0x00, 0x10, 0x00, 0x03];
        let (_, length) =
            parse_length(&bytes, &Some(ValueRepresentation::UL), Endianness::Big).unwrap();
        assert_eq!(length, 0x10);
    }

    #[test]
    pub fn parse_length_special_vr() {
        let bytes = vec![0x00, 0x10, 0x00, 0x03, 0x02, 0x02];
        let (_, length) =
            parse_length(&bytes, &Some(ValueRepresentation::UV), Endianness::Big).unwrap();
        assert_eq!(length, 0x030202);
    }

    #[test]
    pub fn test_parse_data() {
        let bytes = vec![0x00, 0x10, 0x00, 0x03, 0x02, 0x02];
        let (rest, value) = parse_data(&bytes, 2).unwrap();
        assert_eq!(rest, &[0x00, 0x03, 0x02, 0x02]);
        assert_eq!(value, &[0x00, 0x10]);
    }
}
