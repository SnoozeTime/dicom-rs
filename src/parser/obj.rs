use super::element::{parse_dataelement};
use crate::types::DataElement;
use crate::parser::{parse_tag, image::parse_image};
use crate::{Tag, TransferSyntax, DicomObject, DicomError};
use log::debug;
use nom::bytes::streaming::{tag, take};
use nom::combinator::peek;
use nom::number::Endianness;
use nom::IResult;
use std::convert::TryFrom;

/// Header is just 132 bytes of padding + the value DICM.
fn parse_header(buf: &[u8]) -> IResult<&[u8], ()> {
    let (buf, _) = take(128usize)(buf)?;
    let (buf, _) = tag("DICM")(buf)?;
    Ok((buf, ()))
}

enum ParserState {
    /// Need to parse the header.
    Header,
    /// Need to parse group2
    Group2,
    Content,
    Images,
    Finished,
}

/// High-level parser for dicom objects.
///
/// ```rust,no_run
/// // read the file
/// use dicom::types::PersonName;
/// use std::fs::File;
/// use std::io::Read;
/// use dicom::DicomResult;
/// use dicom::parser::obj::Parser;
/// use dicom::Tag;
///
/// let mut file = File::open("somefile.dcm").unwrap();
/// let mut content = vec![];
/// file.read_to_end(&mut content).unwrap();
///
/// // Parse the dicom.
/// let mut parser = Parser::default();
/// let res = parser.parse_object(&content);
///
/// if let Ok((_, dcm)) = res {
///     // dcm contains the Dicom object. Its lifetime is bound to the content vec.
///
///     // Save the image data.
///     if let Some(ref img) = dcm.image {
///         img.save("somewhere.png").unwrap();
///     }
///
///     // extract some tags (need to import `FromDicomValue` trait)
///     let name: PersonName = dcm.get(Tag::x0010x0010); // panic if cannot convert or find.
///     let name: DicomResult<PersonName> = dcm.try_get(Tag::x0010x0010); // panic if cannot convert or find.
/// }
/// ```
pub struct Parser {
    parse_image: bool,
}

impl Default for Parser {
    fn default() -> Self {
        Self {
            parse_image: true,
        }
    }
}

impl Parser {

    /// Create a new parser. if `parse_image` is true, the images will be parsed and returned in the
    /// `DicomObject`. Otherwise, only the tags that are before the image data tag will be parsed.
    pub fn new(parse_image: bool) -> Self {
        Self {
            parse_image
        }
    }

    /// Parse the DICOM object.
    ///
    /// Will return a `DicomObject` which has the same lifetime as the input slice.
    pub fn parse_object<'buf>(&mut self, buf: &'buf [u8]) -> Result<DicomObject<'buf>, DicomError> {

        let mut state = ParserState::Header;
        debug!("Start parsing object");
        let mut current_buf = buf;
        let mut obj: Option<DicomObject> = None;

        loop {
            let (next_state, next_buf) = match state {
                ParserState::Header => {
                    debug!("Parse header");
                    let (buf, _) = parse_header(current_buf)?;
                    (ParserState::Group2, buf)
                }
                ParserState::Group2 => {
                    debug!("Parse group 2");
                    let (buf, (transfer_syntax, elements)) = parse_group2(current_buf)?;
                    debug!("Transfer syntax is {:?}", transfer_syntax);
                    obj = Some(DicomObject::new(elements, transfer_syntax));
                    (ParserState::Content, buf)
                }
                ParserState::Content => {
                    debug!("Parse content");
                    let obj = obj.as_mut().unwrap();
                    let (buf, elements) = parse_content(current_buf, obj.transfer_syntax)?;
                    obj.append(elements);
                    (ParserState::Images, buf)
                }
                ParserState::Images => {

                    if self.parse_image {
                        let obj = obj.as_mut().unwrap();
                        let rows: u16 = obj.try_get(Tag::x0028x0010).unwrap();
                        let cols: u16 = obj.try_get(Tag::x0028x0011).unwrap();
                        // TODO image with colors
                        let _samples_per_pixel: u16 = obj.try_get(Tag::x0028x0002).unwrap();
                        let _nb_of_frames: Result<u16, _> = obj.try_get(Tag::x0028x0008);
                        let _representation: Result<String, _> = obj.try_get(Tag::x0028x0004);

                        let bits_stored: u16 = obj.try_get(Tag::x0028x0101).unwrap();
                        let bits_allocated: u16 = obj.try_get(Tag::x0028x0100).unwrap();

                        let (buf, image) = parse_image(current_buf, obj.transfer_syntax, rows, cols, bits_allocated, bits_stored)?;
                        obj.image = Some(image);
                        (ParserState::Finished, buf)
                    } else {
                        (ParserState::Finished, buf)
                    }
                },
                ParserState::Finished => break,
            };

            state = next_state;
            current_buf = next_buf;
        }

        Ok(obj.unwrap())
    }
}

fn parse_group2(buf: &[u8]) -> IResult<&[u8], (TransferSyntax, Vec<DataElement>)> {
    let mut ts = None;

    let mut current_buf = buf;
    let mut group2_elements = vec![];
    loop {
        // Will stop if next tag is not for the second group.
        let (buf, next_tag) = peek(|i| parse_tag(i, Endianness::Little))(current_buf)?;
        if next_tag.get_group() != 2 {
            debug!("Next tag is {:?}, stop group2 parsing", next_tag);
            break;
        }

        let (buf, data_element) =
            parse_dataelement(buf, TransferSyntax::little_endian_explicit())?;
        if data_element.tag == Tag::x0002x0010 {
            ts = Some(TransferSyntax::try_from(&data_element.data).unwrap());
        }

        group2_elements.push(data_element);
        current_buf = buf;
    }

    Ok((
        current_buf,
        (
            ts.expect("There should be the transfer syntax in group 2."),
            group2_elements,
        ),
    ))
}

fn parse_content(buf: &[u8], transfer_syntax: TransferSyntax) -> IResult<&[u8], Vec<DataElement>> {
    let mut current_buf = buf;
    let mut elements = vec![];

    let endian = transfer_syntax.endianness();

    loop {
        // Will stop if next tag is for images.
        let (buf, next_tag) = peek(|i| parse_tag(i, endian))(current_buf)?;
        if next_tag == Tag::x7FE0x0010 {
            break;
        }

        let (buf, data_element) = parse_dataelement(buf, transfer_syntax)?;
        elements.push(data_element);
        current_buf = buf;
    }

    Ok((current_buf, elements))
}