use serde::{de::Error, Deserializer};
use serde_derive::Deserialize;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct CsvTag {
    #[serde(deserialize_with = "from_hex")]
    group: u32,

    #[serde(deserialize_with = "from_hex")]
    element: u32,
    multiplicity: i8,
    name: String,
    description: String,
}

fn from_hex<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = serde::Deserialize::deserialize(deserializer)?;
    // do better hex decoding than this
    u32::from_str_radix(&s[2..], 16).map_err(D::Error::custom)
}

impl CsvTag {
    fn to_macro_line(self) -> String {
        format!(
            "(x{:04X}x{:04X}, {:#04X}, {:#04X}, {}, \"{}\", \"{}\")",
            self.group,
            self.element,
            self.group,
            self.element,
            self.multiplicity,
            self.name,
            self.description
        )
    }
}

fn main() {
    let csv_file = std::env::var("DCM_TAG_FILE").unwrap_or("tags/tags.csv".to_string());
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("tags.rs");

    let mut rdr = csv::Reader::from_path(csv_file.clone()).unwrap();

    let mut macro_str = "tags! {".to_string();

    let lines: Vec<String> = rdr
        .deserialize()
        .into_iter()
        .map(|row| {
            let row: CsvTag = row.unwrap();
            row.to_macro_line()
        })
        .collect();
    macro_str.push_str(&lines.join(","));
    macro_str.push_str("}");

    fs::write(dest_path, macro_str).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", csv_file);
}
