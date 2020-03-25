//! Code to extract an ImageBuffer from a DICOM file.
//!
//! DCM files can have multiple images embedded, with different interlacing, color format and
//! so on. This should take care of it and return an ImageBuffer from the
//! image crate, which can then be used to save the image to a file.
//!
use image::{ImageBuffer, Luma};

use crate::error::DicomResult;
use std::fmt;
use std::path::Path;
use std::fs::File;
use std::io::Write;

// for some reason image does not export this type...
pub(crate) type Gray16Image = ImageBuffer<Luma<u16>, Vec<u16>>;
pub enum DicomImage {
    Grayscale16 {
        image: Gray16Image,
    },
    Grayscale8 {
        image: image::GrayImage,
    },
    Jpeg2000 {
        image: Vec<u8>,
    }
}

impl fmt::Debug for DicomImage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DicomImage::Grayscale16 { .. } => write!(f, "DicomImage::Grayscale16"),
            DicomImage::Grayscale8 { .. } => write!(f, "DicomImage::Grayscale8"),
            DicomImage::Jpeg2000 { .. } => write!(f, "DicomImage::Jpeg2000"),
        }
    }
}

impl DicomImage {
    pub fn save<P: AsRef<Path>>(&self, path: P) -> DicomResult<()> {
        match *self {
            DicomImage::Grayscale16 { ref image  } => image.save(path).map_err(|e| e.into()),
            DicomImage::Grayscale8 { ref image } => image.save(path).map_err(|e| e.into()),
            DicomImage::Jpeg2000 { ref image } => {
                let mut file = File::create(path)?;
                file.write_all(&image).map_err(|e| e.into())
            },
        }
    }

    pub fn thumbnail(&self, width: u32, height: u32) -> DicomImage {
        match *self {
            DicomImage::Grayscale16 {
                ref image,
            } => DicomImage::Grayscale16 {
                image: image::imageops::thumbnail(image, width, height),
            },
            DicomImage::Grayscale8 {
                ref image,
            } => DicomImage::Grayscale8 {
                image: image::imageops::thumbnail(image, width, height),
            },
            _ => unimplemented!()
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        match *self {
            DicomImage::Grayscale16 { image: ref img} => img.dimensions(),
            DicomImage::Grayscale8 { image: ref img } => img.dimensions(),
            _ => unimplemented!()
        }
    }
}
