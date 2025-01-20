use std::{error::Error, path::PathBuf, time::Duration};

use probe_rs::{
    flashing::{download_file_with_options, DownloadOptions, FlashProgress, Format},
    probe::list::Lister,
    Permissions,
};

use crate::usb;

pub fn flash_with_retries(pin: u8, path: &PathBuf) -> Result<(), Box<dyn Error>> {
    // TODO: use a thing more similar to probe-rs main becasue this fails often.

    let mut timeout = 1;
    'flash_loop: while flash(pin, path).is_err() {
        eprintln!("Flash failed, retrying ({timeout})");
        timeout += 1;

        if timeout > 3 {
            eprintln!("Max retries for flashing exceeded.");
            break 'flash_loop;
        }
    }

    Ok(())
}

pub fn flash(pin: u8, path: &PathBuf) -> Result<(), Box<dyn Error>> {
    usb::usb(pin).unwrap();

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
