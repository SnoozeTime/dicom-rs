//! Tag are represented by two 2bytes unsigned integer: gggg,eeee where gggg is the group and
//! eeee is the element.
//!
//! You can use the macro `tags!` to add a known tag to the crate. If a tag is parsed and is not
//! defined by the macro, the tag will be `Tag::UNKNOWN(u16, u16)`
use std::fmt;
use log::trace;
macro_rules! tags {
    ($( ($name:ident, $_0:expr, $_1:expr, $multiplicity:expr, $repr:expr, $kw:expr)),+) => {

        #[allow(non_camel_case_types)]
        #[derive(Eq, PartialEq, Copy, Clone, Hash)]
        pub enum Tag {
            $( $name ),+
                ,
                UNKNOWN(u16, u16),
        }

        impl Tag {
            pub fn from_values(group: u16, element: u16) -> Tag {
                trace!("Tag from values: {} and {}", group, element);
                match (group, element) {
                    $(
                        ($_0, $_1) => {
                        trace!("Got {}", Tag::$name);
                        Tag::$name
                        }
                        )+
                        _ => Tag::UNKNOWN(group, element),
                }
            }

            pub fn get_keyword(&self) -> &str {
                match *self {
                    $(Tag::$name => $kw,)+
                        _ => "Unknown",
                }
            }

            #[allow(unreachable_patterns)]
            pub fn lookup_by_kw(kw: &str) -> Option<Tag> {

                match kw {
                    $($kw => Some(Tag::$name),)+
                    _ => None,
                }
            }

            /// Return the group for the given tag.
            pub fn get_group(&self) -> u16 {
                match *self {
                    $(Tag::$name => $_0,)+
                    Tag::UNKNOWN(group, _) => group,
                }
            }

            pub fn multiplicity(&self) -> usize {
                match *self {
                    $(Tag::$name => $multiplicity,)+
                    Tag::UNKNOWN(_, _) => 0,
                }
            }
        }

        impl fmt::Debug for Tag {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match *self {
                    $(
                        Tag::$name => write!(f, "{}: {}", stringify!($name), $repr),
                        )+
                        Tag::UNKNOWN(b0, b1) => write!(f, "Unkown tag: x{:x}, x{:x}", b0, b1),
                }
            }
        }

        impl fmt::Display for Tag {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match *self {
                    $(
                        Tag::$name => write!(f, "{}", $repr),
                        )+
                        Tag::UNKNOWN(b0, b1) => write!(f, "Unkown tag: x{:x}, x{:x}", b0, b1),
                }
            }
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/tags.rs"));