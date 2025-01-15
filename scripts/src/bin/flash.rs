use std::{
    error::Error,
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use clap::Parser;
use probe_rs::{
    flashing::{download_file_with_options, DownloadOptions, FlashProgress, Format},
    probe::{list::Lister, Probe},
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

        // TODO: use a thing more similar to probe-rs main becasue this fails often.
        let mut timeout = 1;
        'flash_loop: while flash(&path).is_err() {
            eprintln!("Flash failed, retrying ({timeout})");
            timeout += 1;

            if timeout > 3 {
                eprintln!("Max retries for flashing exceeded.");
                break 'flash_loop;
            }

            scripts::usb::usb(pin).unwrap();
        }
    }

    Ok(())
}

fn flash(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let lister = Lister::new();
    let probes = lister.list_all();
    let probe = probes.first().expect("No probes found").open()?;
    let mut session = probe.attach("rp2040", Permissions::default())?;

    let mut options = DownloadOptions::default();
    options.progress = Some(FlashProgress::new(|e| println!("{e:?}")));

    download_file_with_options(&mut session, path, Format::Elf, options)?;

    session
        .core(0)?
        .reset_and_halt(Duration::from_millis(100))?;
    session.core(0)?.run()?;

    Ok(())
}
