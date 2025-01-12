use std::{error::Error, path::Path, process::Command, time::Duration};

use clap::Parser;
use probe_rs::{
    flashing::{download_file_with_options, DownloadOptions, FlashProgress, Format},
    probe::list::Lister,
    Permissions,
};

#[derive(Debug, Parser)]
struct Arguments {
    binary_name: String,
    #[arg(value_delimiter = ',')]
    pins: Vec<u8>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Arguments::parse();

    let lister = Lister::new();
    let probes = lister.list_all();
    println!("Probes:\n{probes:?}");

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

        let probe = probes.first().expect("No probes found").open().unwrap();
        let mut session = probe.attach("rp2040", Permissions::default()).unwrap();

        let mut options = DownloadOptions::default();
        options.progress = Some(FlashProgress::new(|e| println!("{e:?}")));

        download_file_with_options(&mut session, &path, Format::Elf, options).unwrap();

        session
            .core(0)?
            .reset_and_halt(Duration::from_millis(100))?;
        session.core(0)?.run()?;
    }

    Ok(())
}
