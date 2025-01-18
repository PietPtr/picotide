use std::{error::Error, path::Path};

use clap::Parser;

#[derive(Debug, Parser)]
struct Arguments {
    binary_name: String,
    #[arg(value_delimiter = ',')]
    pins: Vec<u8>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Arguments::parse();

    let path = Path::new("target")
        .join("thumbv6m-none-eabi")
        .join("release")
        .join(&args.binary_name);

    if !path.exists() {
        eprintln!("{path:?}: no such file or directory");
        return Err("bin not found".into());
    }

    for pin in args.pins {
        scripts::usb::usb(pin).unwrap();
        scripts::flash::flash_with_retries(pin, &path)?;
    }

    Ok(())
}
