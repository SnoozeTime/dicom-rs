//! SQ: Sequence of items, has its own special section in the DICOM specification.
//! It is a way to encode a sequence of multiple items... See http://dicom.nema.org/dicom/2013/output/chtml/part05/sect_7.5.html
//! For Implicit VR, the length is defined as usual.
//! For explicit VR, the length is undefined and the SQ ends with a Sequence delimitation item.
//!
//! One item can contain multiple data elements.
//!
//! There are three special SQ related Data Elements that are not ruled by the VR encoding rules
//! conveyed by the Transfer Syntax. They shall be encoded as Implicit VR.
//! These special Data Elements are Item (FFFE,E000), Item Delimitation Item (FFFE,E00D),
//! and Sequence Delimitation Item (FFFE,E0DD). However, the Data Set within the Value Field of
//! the Data Element Item (FFFE,E000) shall be encoded according to the rules conveyed by the Transfer Syntax.

use crate::types::DataElement;
use crate::TransferSyntax;
use nom::IResult;
use crate::parser::{parse_tag, parse_length};
use crate::Tag;
use nom::combinator::peek;
use nom::number::Endianness;
use log::debug;
use crate::parser::element::parse_dataelement;

/// An item is a list of data elements.
#[derive(Debug)]
pub struct Item<'buf> {
    pub elements: Vec<DataElement<'buf>>,
}

/// A sequence is a list of items. Special sequence elements are always using little endian implicit (no VR)
/// A sequence with undefined length is finished by the special element xFFFExE0DD.
///
/// The buffer here only contains the data part of the SQ data element (the rest has already been
/// parsed).
///
/// TODO Length defined.
pub(crate) fn parse_seq(buf: &[u8], _length: u32, transfer_syntax: TransferSyntax) -> IResult<&[u8], Vec<Item>> {

    let mut current = buf;
    let mut items = vec![];
    'parse_loop: loop {
        let (_, next_tag) = peek(|i| parse_tag(i, Endianness::Little))(current)?;
        match next_tag {
            Tag::xFFFExE000 => {
                // Item !
                let (buf, item) = parse_item(current, transfer_syntax)?;
                current = buf;
                items.push(item);
            },
            Tag::xFFFExE0DD => {
                // Sequence delimitation !
                let (buf, _) = parse_dataelement(current, TransferSyntax::little_endian_implicit())?;
                current = buf;
                break 'parse_loop;
            },
            _ => panic!("Unexpected tag {:?}", next_tag),
        }
    }

    Ok((current, items))
}

/// An Item is just a sequence of data elements. The Item starts with tag xFFFExE000. It has no
/// VR but it can have a length. If length is `std::u32::MAX`, then the Item will finish by the
/// Item delimitation tag xFFFExE00D
///
/// | TAG | LENGTH | DATA |
/// | 4   | 4      \ n    |
///
pub(crate) fn parse_item(buf: &[u8], transfer_syntax: TransferSyntax) -> IResult<&[u8], Item> {

    let (buf, tag) = parse_tag(buf, transfer_syntax.endianness())?;
    // FIXME error handling.
    assert_eq!(Tag::xFFFExE000, tag);
    let (buf, length) = parse_length(buf, &None, transfer_syntax.endianness())?;

    let is_len_undefined = length == std::u32::MAX;

    // will parse the content of an item. An item contains a buf of data elements.
    let mut current = buf;
    let mut remaining_len = length as usize;

    let mut elements = vec![];

    'parse_loop: loop {

        // Stop condition.
        if is_len_undefined {
            // Expect to have a Item delimitation element
            let (_, next_tag) = peek(|i| parse_tag(i, Endianness::Little))(current)?;
            if next_tag == Tag::xFFFExE00D {
                debug!("Found Item delimitation tag");
                let (buf, _) = parse_dataelement(current, TransferSyntax::little_endian_implicit())?;
                current = buf;
                break 'parse_loop;
            }
        } else if remaining_len == 0 {
            break 'parse_loop;
        }


        let length_before = current.len();
        let (buf, data_element) = parse_dataelement(current, transfer_syntax)?;
        let parsed_len = length_before - buf.len();
        remaining_len -= parsed_len;
        elements.push(data_element);

        current = buf;
    }

    Ok((current, Item { elements }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_item_implicitlength() {
        let data: Vec<u8> = vec![
            0xFE, 0xFF, 0x00, 0xE0, // item start, always little endian
            0xFF, 0xFF, 0xFF, 0xFF, // undefined length.
            0x08, 0x00, 0x00, 0x00, 0x55, 0x4c, 0x04, 0x00, 0x30, 0x00, 0x00, 0x00, 0x08, 0x00,
            0x00, 0x01, 0x53, 0x48, 0x08, 0x00, 0x54, 0x2d, 0x31, 0x31, 0x35, 0x30, 0x33, 0x20,
            0x08, 0x00, 0x02, 0x01, 0x53, 0x48, 0x04, 0x00, 0x53, 0x4e, 0x4d, 0x33, 0x08, 0x00,
            0x04, 0x01, 0x4c, 0x4f, 0x0c, 0x00, 0x4c, 0x75, 0x6d, 0x62, 0x61, 0x72, 0x20, 0x73,
            0x70, 0x69, 0x6e, 0x65, // content
            0xFE, 0xFF, 0x0D, 0xE0, 0x00, 0x00, 0x00, 0x00, // item delimitation tag
        ];

        let res = parse_item(
            &data,
            TransferSyntax::little_endian_explicit(),
        );

        assert!(res.is_ok());
        let (_, item)  = res.unwrap();
        println!("{:?}", item);
        assert!(false);
    }
}