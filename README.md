# Parse DCM files and extract images

## Supported
- DCM file with one frame: PNG 8 and 16 bits
- Tag extraction

## How to use

```rust
use std::fs;
use std::io::Read;
use dicom::Parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read all the DCM to vec. For now buffer read is not supported.
    let mut f = fs::open("file.dcm")?;
    let mut content = vec![];
    f.read_to_end(&mut content)?;
   
    // If true, will also parse the image.
    let parser = Parser::new(true);

    parser.parse
    Ok(())
}
```