use rusb::{Context, DeviceHandle, UsbContext};

pub fn usb(swdio_pin: u8) -> Result<(), String> {
    const VID: u16 = 0x2e8a;
    const PID: u16 = 0x000c;

    println!("Setting to GPIO={}", swdio_pin);

    let context = Context::new().expect("Failed to initialize libusb");
    let handle: DeviceHandle<Context> = match context.open_device_with_vid_pid(VID, PID) {
        Some(h) => h,
        None => {
            return Err("Device not found".to_string());
        }
    };

    let out_endpoint = 0x04;
    let in_endpoint = 0x85;

    let data = vec![0x02, 0x03, swdio_pin];
    handle
        .write_bulk(out_endpoint, &data, std::time::Duration::from_secs(1))
        .map_err(|err| format!("Write error: {err:?}"))?;

    let mut response = vec![0; 3];
    handle
        .read_bulk(
            in_endpoint,
            &mut response,
            std::time::Duration::from_secs(1),
        )
        .map_err(|err| format!("Read error: {err:?}"))?;

    let &actual_pin = response.get(2).unwrap();

    if actual_pin != swdio_pin {
        eprintln!("Failed to set pin, expected {swdio_pin}, but probe is set to {actual_pin}.")
    }

    Ok(())
}
