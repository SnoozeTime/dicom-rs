use super::{parse_data, parse_length, parse_tag, parse_vr, sq::parse_seq};
use crate::types::{TransferSyntax, Value, DataElement};
use log::trace;
use nom::combinator::cond;
use nom::IResult;

pub(crate) fn parse_dataelement(
    buf: &[u8],
    transfer_syntax: TransferSyntax,
) -> IResult<&[u8], DataElement> {
    // If no transfer syntax, we expect group 2. For the group 2, the Little endian, explicit VR is used.
    let endian = transfer_syntax.endianness();
    let (buf, tag) = parse_tag(buf, endian)?;
    trace!("TAG = {:?}", tag);
    let (buf, vr) = cond(transfer_syntax.is_vr_explicit(), parse_vr)(buf)?;
    trace!("VR = {:?}", vr);
    let (buf, length) = parse_length(buf, &vr, endian)?;
    trace!("LENGTH = {:?}", length);

    let (buf, data) = parse_element_data(buf, length, transfer_syntax)?;
    trace!("DATA = {:?}", data);

    Ok((
        buf,
        DataElement {
            tag,
            vr,
            length,
            data,
        },
    ))
}

fn parse_element_data(buf: &[u8], length: u32, transfer_syntax: TransferSyntax) -> IResult<&[u8], Value> {
    if length == std::u32::MAX {
        let (buf, items) = parse_seq(buf, length, transfer_syntax)?;
        Ok((buf, Value::Sequence(items)))
    } else {
        let (buf, data) = parse_data(buf, length)?;
        Ok((buf, Value::Buf(data)))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{ValueRepresentation, Tag};

    #[test]
    fn parse_lee_dataelement() {
        //x0010x0010
        let vr = "CS".as_bytes();
        let name = "benoit".as_bytes();
        let mut data = vec![
            0x10, 0x00, 0x10, 0x00, // patient name
            vr[0], vr[1], // CS code string
            0x06, 0x00, // length is two bytes for CS
        ];
        data.extend_from_slice(name);

        let data_element = parse_dataelement(&data, TransferSyntax::little_endian_explicit());
        assert!(data_element.is_ok());
        let (_, data_element) = data_element.unwrap();

        assert_eq!(Tag::x0010x0010, data_element.tag);
        assert_eq!(data_element.length, 6);
        if let Value::Buf(data) = data_element.data {
            assert_eq!(std::str::from_utf8(data).unwrap(), "benoit");
        } else {
            assert!(false);
        }
        assert_eq!(ValueRepresentation::CS, *data_element.vr.as_ref().unwrap());
    }

    #[test]
    fn parse_lei_dataelement() {
        //x0010x0010
        let name = "benoit".as_bytes();
        let mut data = vec![
            0x10, 0x00, 0x10, 0x00, // patient name
            0x06, 0x00, 0x00, 0x00, // length is four bytes when no VR
        ];
        data.extend_from_slice(name);

        let data_element = parse_dataelement(&data, TransferSyntax::little_endian_implicit());
        assert!(data_element.is_ok());
        let (_, data_element) = data_element.unwrap();
        assert_eq!(Tag::x0010x0010, data_element.tag);
        assert_eq!(data_element.length, 6);
        if let Value::Buf(data) = data_element.data {
            assert_eq!(std::str::from_utf8(data).unwrap(), "benoit");
        } else {
            assert!(false);
        }
        assert!(data_element.vr.is_none());
    }

    #[test]
    fn parse_bee_dataelement() {
        //x0010x0010
        let vr = "CS".as_bytes();
        let name = "benoit".as_bytes();
        let mut data = vec![
            0x00, 0x10, 0x00, 0x10, // patient name
            vr[0], vr[1], // CS code string
            0x00, 0x06, // length is two bytes for CS
        ];
        data.extend_from_slice(name);

        let data_element = parse_dataelement(&data, TransferSyntax::big_endian_explicit());
        assert!(data_element.is_ok());
        let (_, data_element) = data_element.unwrap();
        assert_eq!(Tag::x0010x0010, data_element.tag);
        assert_eq!(data_element.length, 6);
        if let Value::Buf(data) = data_element.data {
            assert_eq!(std::str::from_utf8(data).unwrap(), "benoit");
        } else {
            assert!(false);
        }
        assert_eq!(ValueRepresentation::CS, *data_element.vr.as_ref().unwrap());
    }
}
