use crate::tag::Tag;
use crate::ValueRepresentation;
use std::convert::From;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DicomError {

    #[error("Error while parsing = {0}")]
    ParseError(String),

    #[error("Cannot read header")]
    CannotReadHeader,

    #[error(transparent)]
    IoError(#[from] io::Error),

    #[error("Could not parse tag")]
    UnknownTag,

    #[error("Cannot convert VR CS to string = {0}")]
    ParseCS(std::str::Utf8Error),

    #[error("Cannot convert to {0}: expect Buf but got sequence")]
    ConvertTypeExpectBuf(String),

    #[error("Cannot convert VR DA to timestamp = {0}")]
    ParseDA(chrono::format::ParseError),

    #[error("Cannot parse AS to Age = {0}")]
    ParseAS(String),

    #[error(transparent)]
    ParseIS(#[from] std::num::ParseIntError),

    #[error("Cannot save to PNG, image format is not supported")]
    ImageFormatNotSupported,

    #[error(transparent)]
    ImageError(#[from] image::ImageError),

    #[error("Missing Tag: {0}")]
    MissingTag(Tag),

    #[error("Transfer syntax is not supported: {0}")]
    TransferSyntaxNotSupported(String),

    #[error("Cannot get value from ValueRepresentation: {0}")]
    VrValueNotImplementated(ValueRepresentation),

    #[error("No tag {0:?} in Dicom object. Did you forget to parse it?")]
    NoSuchTag(Tag),

    #[error("First group should be 0x0002 but got {0:?} instead")]
    ExpectedGroup2(Tag),
}

impl<E> From<nom::Err<E>> for DicomError where E: std::fmt::Debug {
    fn from(err: nom::Err<E>) -> Self {
        DicomError::ParseError(format!("{}", err))
    }
}

impl From<std::str::Utf8Error> for DicomError {
    fn from(err: std::str::Utf8Error) -> Self {
        DicomError::ParseCS(err)
    }
}

impl From<chrono::format::ParseError> for DicomError {
    fn from(err: chrono::format::ParseError) -> Self {
        DicomError::ParseDA(err)
    }
}

pub type DicomResult<T> = Result<T, DicomError>;
