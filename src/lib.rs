mod error;
mod img;
mod tag;
pub mod types;
mod vr;
pub mod parser;

/*
    Crate exports.
*/
pub use img::DicomImage;
pub use error::{DicomError, DicomResult};
pub use parser::obj::Parser;
pub use tag::Tag;
pub use vr::ValueRepresentation;
pub use types::{TransferSyntax, DicomObject};