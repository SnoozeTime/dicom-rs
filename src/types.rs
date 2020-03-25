//! Types specific to Dicom.
use crate::error::*;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use chrono::NaiveDate;
use std::fmt::{self, Display};
use std::io::Cursor;
use nom::number::Endianness;
use std::convert::TryFrom;

use crate::{Tag, ValueRepresentation};
use crate::parser::sq::Item;
use crate::img::DicomImage;

/// Represent a DICOM file
#[derive(Debug)]
pub struct DicomObject<'buf> {
    /// All the tags that were parsed from .dcm file
    pub elements: Vec<DataElement<'buf>>,
    /// Transfer syntax extracted from x0002
    pub transfer_syntax: TransferSyntax,

    pub image: Option<DicomImage>,
}

impl<'buf> DicomObject<'buf> {
    pub fn new(elements: Vec<DataElement<'buf>>, transfer_syntax: TransferSyntax) -> Self {
        Self {
            elements,
            transfer_syntax,
            image: None,
        }
    }

    pub fn append(&mut self, mut elements: Vec<DataElement<'buf>>) {
        self.elements.append(&mut elements);
    }

    pub fn elements(&self) -> &Vec<DataElement> {
        &self.elements
    }

    pub fn get_element(&self, tag: Tag) -> Option<&DataElement> {
        self.elements.iter().find(|el| el.tag == tag)
    }

    pub fn get<T: FromDicomValue + 'static>(&self, tag: Tag) -> T {
        match self.try_get(tag) {
            Ok(v) => v,
            Err(e) => panic!(
                "Cannot get value {:?} for tag {:?} = {}",
                std::any::TypeId::of::<T>(),
                tag,
                e
            ),
        }
    }

    pub fn try_get<T: FromDicomValue>(&self, tag: Tag) -> DicomResult<T> {
        match self.get_element(tag) {
            Some(ref el) => FromDicomValue::from_element(el, &self.transfer_syntax),
            None => Err(DicomError::NoSuchTag(tag)),
        }
    }
}

/// Data elements are the basic unit of a DICOM object.
///
/// They are made of:
/// - a Tag that indicates what the element is referring to
/// - an optional ValueRepresentation that gives information about the type of the data.
/// - a buffer that represents something. When value representation is known, the library will be
///   able to parse automatically the value to the correct type. Otherwise, it has to be known by
///   the user.
#[derive(Debug)]
pub struct DataElement<'buf> {
    pub tag: Tag,
    pub vr: Option<ValueRepresentation>,
    pub length: u32,
    pub data: Value<'buf>,
}

#[derive(Debug)]
pub enum Value<'a> {
    Buf(&'a [u8]),
    Sequence(Vec<Item<'a>>)
}

/// Transfer syntax defines the endianness and the presence of value representation.
/// It is necessary during parsing. The transfer syntax is defined in the tag (0x0002,0x010) which
/// is at the beginning of the file
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TransferSyntax {
    endianness: Endianness,
    is_vr_explicit: bool,
    pub compression_scheme: Option<CompressionScheme>,
}

impl TransferSyntax {
    pub fn with_compression_scheme(scheme: CompressionScheme) -> Self {
        Self {
            endianness: Endianness::Little,
            is_vr_explicit: true,
            compression_scheme: Some(scheme),
        }
    }

    pub fn little_endian_explicit() -> Self {
        Self {
            endianness: Endianness::Little,
            is_vr_explicit: true,
            compression_scheme: None,
        }
    }

    pub fn big_endian_explicit() -> Self {
        Self {
            endianness: Endianness::Big,
            is_vr_explicit: true,
            compression_scheme: None,
        }
    }

    pub fn little_endian_implicit() -> Self {
        Self {
            endianness: Endianness::Little,
            is_vr_explicit: false,
            compression_scheme: None,
        }
    }

    /// Return the endianness in which the dicom data was encoded.
    pub fn endianness(&self) -> Endianness {
        self.endianness
    }

    /// Return true if the value representation is explicit in data elements
    pub fn is_vr_explicit(&self) -> bool {
        self.is_vr_explicit
    }
}

impl TryFrom<&Value<'_>> for TransferSyntax {
    type Error = DicomError;

    fn try_from(v: &Value) -> Result<Self, Self::Error> {
        if let Value::Buf(bytes) = v {
            let value = std::str::from_utf8(bytes)?;
            // If a Value Field containing one or more UIDs is an odd number of bytes in length, the Value Field shall be padded with a single trailing NULL (00H) character to ensure that the Value Field is an even number of bytes in length. See Section 9 and Annex B for a complete specification and examples
            // No comment
            match value {
                "1.2.840.10008.1.2.2\u{0}" => Ok(TransferSyntax::big_endian_explicit()),
                "1.2.840.10008.1.2.1\u{0}" => Ok(TransferSyntax::little_endian_explicit()),
                "1.2.840.10008.1.2\u{0}" => Ok(TransferSyntax::little_endian_implicit()),
                "1.2.840.10008.1.2.4.90" => Ok(TransferSyntax::with_compression_scheme(
                    CompressionScheme::Jpeg2000Lossless,
                )),
                _ => Err(DicomError::TransferSyntaxNotSupported(String::from(value))),
            }
        } else {
            Err(DicomError::ConvertTypeExpectBuf("TransferSyntax".to_string()))
        }
    }
}

/// Sometime DCM files contain the image as JPG...
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CompressionScheme {
    Jpeg2000Lossless,
}

/// Trait to convert a series of bytes to the correct type.
///
/// ```rust
/// use dicom::types::FromDicomValue;
/// use dicom::element::{Value, DataElement};
/// use dicom::{Tag, TransferSyntax};
/// let content = vec![0x00, 0x01];
/// let element = DataElement {
///     data: Value::Buf(&content),
///     vr: None,
///     length: 2,
///     tag: Tag::UNKNOWN(0,0)
/// };
/// let transfer_syntax = TransferSyntax::little_endian_implicit();
/// let value_u16: u16 = FromDicomValue::from_element(&element, &transfer_syntax).unwrap();
/// ```
pub trait FromDicomValue: Sized {
    /// Parse the Dicom Type from the bytes
    fn from_element(el: &DataElement, transfer_syntax: &TransferSyntax) -> DicomResult<Self>;
}

impl FromDicomValue for u16 {
    fn from_element(
        el: &DataElement,
        transfer_syntax: &TransferSyntax,
    ) -> Result<Self, DicomError> {
        if let Value::Buf(data) = el.data {
            let mut rdr = Cursor::new(data);
            let repr = if let Endianness::Little = transfer_syntax.endianness() {
                rdr.read_u16::<LittleEndian>()?
            } else {
                rdr.read_u16::<BigEndian>()?
            };
            Ok(repr)
        } else {
            Err(DicomError::ConvertTypeExpectBuf("u16".to_string()))
        }
    }
}

impl FromDicomValue for String {
    fn from_element(
        el: &DataElement,
        _transfer_syntax: &TransferSyntax,
    ) -> Result<Self, DicomError> {
        if let Value::Buf(data) = el.data {
            let v = std::str::from_utf8(data)?;
            Ok(v.to_string())
        } else {
            Err(DicomError::ConvertTypeExpectBuf("String".to_string()))
        }
    }
}

/// The same DICOM type :) When the VR is known, this will give the correct type.
#[derive(Debug)]
pub enum DicomType {
    Str(Vec<String>),
    UnsignedInt(Vec<u16>),
    Date(Vec<NaiveDate>),
    PersonName(Vec<String>),
    Age(Vec<Age>),
    SignedLong(Vec<i32>),
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum AgeFormat {
    Day,
    Week,
    Month,
    Year,
}

impl AgeFormat {
    pub fn parse_from_str(repr: &str) -> DicomResult<Self> {
        match repr {
            "D" => Ok(AgeFormat::Day),
            "W" => Ok(AgeFormat::Week),
            "M" => Ok(AgeFormat::Month),
            "Y" => Ok(AgeFormat::Year),
            _ => Err(DicomError::ParseAS(format!(
                "Unknown age format = {}",
                repr
            ))),
        }
    }
}

impl Display for AgeFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AgeFormat::Day => write!(f, "D"),
            AgeFormat::Week => write!(f, "W"),
            AgeFormat::Month => write!(f, "M"),
            AgeFormat::Year => write!(f, "Y"),
        }
    }
}

/// Age formatted according to DCM protocol. It's always
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Age {
    pub age: u8,
    pub format: AgeFormat,
}

impl Display for Age {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:03}{}", self.age, self.format)
    }
}

impl Age {
    pub fn parse_from_str(repr: &str) -> DicomResult<Age> {
        if repr.len() != 4 {
            return Err(DicomError::ParseAS(format!(
                "The length of the Age String should be 4 (got {})",
                repr.len()
            )));
        }

        let age: u8 = repr[0..3]
            .parse()
            .map_err(|e| DicomError::ParseAS(format!("Cannot get integer = {:?}", e)))?;
        let format = AgeFormat::parse_from_str(&repr[3..])?;

        Ok(Age { age, format })
    }
}

impl FromDicomValue for Age {
    fn from_element(
        el: &DataElement,
        _transfer_syntax: &TransferSyntax,
    ) -> Result<Self, DicomError> {
        if let Value::Buf(data) = el.data {
            let repr = std::str::from_utf8(data)?;
            let v = Age::parse_from_str(repr)?;
            Ok(v)
        } else {
            Err(DicomError::ConvertTypeExpectBuf("Age".to_string()))
        }
    }
}

impl FromDicomValue for NaiveDate {
    fn from_element(
        el: &DataElement,
        _transfer_syntax: &TransferSyntax,
    ) -> Result<Self, DicomError> {
        if let Value::Buf(data) = el.data {
            let repr = std::str::from_utf8(data)?;
            let dt = NaiveDate::parse_from_str(repr, "%Y%m%d")?;
            Ok(dt)
        } else {
            Err(DicomError::ConvertTypeExpectBuf("NaiveDate".to_string()))
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PersonName(pub Vec<String>);

impl FromDicomValue for PersonName {
    fn from_element(
        el: &DataElement,
        _transfer_syntax: &TransferSyntax,
    ) -> Result<Self, DicomError> {
        if let Value::Buf(data) = el.data {
            let v = std::str::from_utf8(data)?
                .to_string()
                .split('^')
                .map(|s| s.to_owned())
                .collect::<Vec<_>>();
            Ok(PersonName(v))
        } else {
            Err(DicomError::ConvertTypeExpectBuf("PersonName".to_string()))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tag::Tag;
    #[test]
    fn parse_years() {
        let repr = "014Y";
        let age = Age::parse_from_str(repr);
        assert!(age.is_ok());
        let age = age.unwrap();
        assert_eq!(14, age.age);
        assert_eq!(AgeFormat::Year, age.format);
    }

    #[test]
    fn parse_months() {
        let repr = "114M";
        let age = Age::parse_from_str(repr);
        assert!(age.is_ok());
        let age = age.unwrap();
        assert_eq!(114, age.age);
        assert_eq!(AgeFormat::Month, age.format);
    }

    #[test]
    fn parse_days() {
        let repr = "010D";
        let age = Age::parse_from_str(repr);
        assert!(age.is_ok());
        let age = age.unwrap();
        assert_eq!(10, age.age);
        assert_eq!(AgeFormat::Day, age.format);
    }

    #[test]
    fn parse_weeks() {
        let repr = "004W";
        let age = Age::parse_from_str(repr);
        assert!(age.is_ok());
        let age = age.unwrap();
        assert_eq!(4, age.age);
        assert_eq!(AgeFormat::Week, age.format);
    }

    #[test]
    fn parse_wrong_length() {
        let repr = "004W11";
        let age = Age::parse_from_str(repr);
        assert!(age.is_err());
        let err = age.err().unwrap();
        assert_eq!(
            "Cannot parse AS to Age = The length of the Age String should be 4 (got 6)",
            format!("{}", err).as_str()
        );

        let repr = "4W";
        let age = Age::parse_from_str(repr);
        assert!(age.is_err());
        let err = age.err().unwrap();
        assert_eq!(
            "Cannot parse AS to Age = The length of the Age String should be 4 (got 2)",
            format!("{}", err).as_str()
        );
    }

    #[test]
    fn parse_wrong_uint() {
        let repr = "0-4W";
        let age = Age::parse_from_str(repr);
        assert!(age.is_err());
        let err = age.err().unwrap();
        assert_eq!(
            "Cannot parse AS to Age = Cannot get integer = ParseIntError { kind: InvalidDigit }",
            format!("{}", err).as_str()
        );
    }

    #[test]
    fn parse_wrong_fmt() {
        let repr = "000V";
        let age = Age::parse_from_str(repr);
        assert!(age.is_err());
        let err = age.err().unwrap();
        assert_eq!(
            "Cannot parse AS to Age = Unknown age format = V",
            format!("{}", err).as_str()
        );
    }

    #[test]
    fn format_age() {
        assert_eq!(
            "245W",
            &format!(
                "{}",
                Age {
                    age: 245,
                    format: AgeFormat::Week
                }
            )
        );

        assert_eq!(
            "025Y",
            &format!(
                "{}",
                Age {
                    age: 25,
                    format: AgeFormat::Year
                }
            )
        );

        assert_eq!(
            "001D",
            &format!(
                "{}",
                Age {
                    age: 1,
                    format: AgeFormat::Day
                }
            )
        );

        assert_eq!(
            "020M",
            &format!(
                "{}",
                Age {
                    age: 20,
                    format: AgeFormat::Month
                }
            )
        );
    }

    #[test]
    fn from_el_u16() {
        let bytes: Vec<u8> = vec![8,0];
        let el = DataElement {
            tag: Tag::x0002x0010,
            length: 0,
            data: Value::Buf(&bytes),
            vr: None,
        };
        let v: Result<u16, _> =
            FromDicomValue::from_element(&el, &TransferSyntax::little_endian_implicit());
        assert!(v.is_ok());
        assert_eq!(8, v.unwrap());
    }

    #[test]
    fn from_el_age() {
        let age = Age {
            age: 5,
            format: AgeFormat::Year,
        };
        let age_bytes = age.to_string();
        let el = DataElement {
            tag: Tag::x0002x0010,
            length: 0,
            data: Value::Buf(age_bytes.as_bytes()),
            vr: None,
        };

        let v: Result<Age, _> =
            FromDicomValue::from_element(&el, &TransferSyntax::little_endian_implicit());
        assert!(v.is_ok());
        assert_eq!(age, v.unwrap());
    }

    #[test]
    fn from_el_date() {
        let date = NaiveDate::from_ymd(2020, 2, 3);
        let date_bytes = String::from("20200203");
        let el = DataElement {
            tag: Tag::x0002x0010,
            length: 0,
            data: Value::Buf(date_bytes.as_bytes()),
            vr: None,
        };

        let v: Result<NaiveDate, _> =
            FromDicomValue::from_element(&el, &TransferSyntax::little_endian_implicit());
        assert!(v.is_ok());
        assert_eq!(date, v.unwrap());
    }

    #[test]
    fn from_el_name() {
        let expected = PersonName(vec!["BENOIT".to_owned(), "EUDIER".to_owned()]);
        let name_bytes = String::from("BENOIT^EUDIER");
        let el = DataElement {
            tag: Tag::x0002x0010,
            length: 0,
            data: Value::Buf(name_bytes.as_bytes()),
            vr: None,
        };

        let v: Result<PersonName, _> =
            FromDicomValue::from_element(&el, &TransferSyntax::little_endian_implicit());
        assert!(v.is_ok());
        assert_eq!(expected, v.unwrap());
    }
}
