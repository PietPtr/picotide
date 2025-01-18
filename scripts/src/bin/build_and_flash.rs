use std::{
    env,
    error::Error,
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
};

use clap::Parser;

#[derive(Debug, Parser)]
struct Arguments {
    binary_name: String,
    #[arg(value_delimiter = ',')]
    pins: Vec<u8>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Arguments::parse();

    // for each pin in args.pins,
    // using current binary index,
    // set env var for binary index
    // build given crate in installation,
    // and flash it

    for (binary_index, &pin) in args.pins.iter().enumerate() {
        env::set_var("BINARY_INDEX", format!("{}", binary_index));

        println!(
            "Compiling {} with binary index {}",
            &args.binary_name, binary_index
        );

        let mut process = Command::new("cargo")
            .arg("build")
            .arg("-p")
            .arg(&args.binary_name)
            .arg("--config")
            .arg(format!(
                "installations/{}/.cargo/config.toml",
                args.binary_name
            ))
            .spawn()
            .unwrap();

        process.wait().unwrap();

        let path = Path::new("target")
            .join("thumbv6m-none-eabi")
            .join("release")
            .join(&args.binary_name);

        scripts::flash::flash(&path)?;
    }

    Ok(())
}
