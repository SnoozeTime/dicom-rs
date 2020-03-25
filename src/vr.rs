//! Value representation defines what type is the data in the data element.
//! VR defined in the DICOM standard are created from the macro `vr!`.

use crate::{DicomResult};
use std::fmt;
use std::io::Read;

macro_rules! vr {
    ( $(( $name:ident, $repr:expr, $desc:expr, $special_length:expr)),+) => {

        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Eq, PartialEq)]
        pub enum ValueRepresentation {
            $($name,)+
            UNKNOWN(String),
        }


        impl ValueRepresentation {

            pub fn from_chars(first: char, second: char) -> Self {
                // TODO Change that to not do string allocation...
                let vr_str = format!("{}{}", first, second);
                match vr_str.as_str() {
                    $(
                        $repr => ValueRepresentation::$name,
                    )+
                    _ => ValueRepresentation::UNKNOWN(vr_str)
                }
            }

            /// Will parse the value representation from a `Read` trait
            pub fn parse<T>(reader: &mut T) -> DicomResult<Self>
            where
                T: Read,
            {
               let mut buf = [0; 2];
               reader.read_exact(&mut buf)?;

               std::str::from_utf8(&buf)
                   .map(|vr| match vr {
                        $(
                            $repr => ValueRepresentation::$name,
                        )+
                       _ => ValueRepresentation::UNKNOWN(String::from(vr)),
                   }).map_err(|e| e.into())
            }

            pub fn has_special_length(&self) -> bool {
                match self {
                    $(
                        ValueRepresentation::$name => {
                            $special_length
                        }
                    )+
                    ValueRepresentation::UNKNOWN(_) => false,
                }
            }
        }

        impl fmt::Display for ValueRepresentation {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match *self {
                    $(ValueRepresentation::$name => write!(f, "{}", $desc),)+
                    ValueRepresentation::UNKNOWN(ref x) => write!(f, "Unknown VR({})", x),
                }
            }
        }
    }

}

vr! {
    (UL, "UL", "Unsigned Long", false),
    (CS, "CS", "Code String", false),
    (AG, "AG", "Age String", false),
    (DA, "DA", "Date", false),
    (DS, "DS", "Decimal String", false),
    (DT, "DT", "Date Time", false),
    (SH, "SH", "Short String", false),
    (ST, "ST", "Short Text", false),
    (US, "US", "Unsigned Short", false),
    (UI, "UI", "Unique Identifier", false),
    (LO, "LO", "Long String", false),
    (PN, "PN", "Person Name", false),
    (AS, "AS", "Age String", false),
    (SL, "SL", "Signed Long", false),

    // Special length parsing
    (OB, "OB", "Other byte", true),
    (OD, "OD", "Other double", true),
    (OF, "OF", "Other float", true),
    (OL, "OL", "Other long", true),
    (OV, "OV", "Other 64-bits very long", true),
    (OW, "OW", "Other word", true),
    (SQ, "SQ", "Sequence of items", true),
    (SV, "SV", "Signed 64-bits very long", true),
    (UC, "UC", "Unlimited characters", true),
    (UR, "UR", "URI or URL", true),
    (UT, "UT", "Unlimited text", true),
    (UN, "UN", "Unknown", true),
    (UV, "UV", "Unsigned 64-bits very long", true)
}
