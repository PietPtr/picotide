#![no_std]
#![no_main]

#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use core::ptr::addr_of;

#[allow(unused_imports)]
use defmt::{error, info, warn};
use defmt_rtt as _;
use panic_probe as _;
use rp_pico::pac;
use rp_pico::{
    entry,
    hal::{
        gpio::{self},
        sio::Sio,
    },
};

#[no_mangle]
#[used]
#[link_section = ".device_info"]
static mut NODE_IDENTIFIER: u32 = 0xdeadbeef;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let sio = Sio::new(pac.SIO);

    let pins = gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    pins.gpio25
        .into_push_pull_output_in_state(gpio::PinState::High);

    #[allow(clippy::empty_loop)]
    loop {
        let address = unsafe { addr_of!(NODE_IDENTIFIER) };

        info!("{}", address);

        unsafe {
            NODE_IDENTIFIER += 1;
        }
    }
}
