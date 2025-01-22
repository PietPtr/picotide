#![no_std]

pub extern crate rp2040_hal as hal;

#[cfg(feature = "rt")]
extern crate cortex_m_rt;

#[cfg(feature = "rt")]
pub use hal::entry;

#[cfg(feature = "boot2")]
#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

pub use hal::pac;

hal::bsp_pins!(
    Gpio0 { name: north_rx2 },
    Gpio1 { name: north_rx1 },
    Gpio2 { name: north_rx0 },
    Gpio3 { name: north_tx0 },
    Gpio4 { name: north_tx1 },
    Gpio5 { name: north_tx2 },
    Gpio6 { name: east_rx2 },
    Gpio7 { name: east_rx1 },
    Gpio8 { name: east_rx0 },
    Gpio9 { name: east_tx0 },
    Gpio10 { name: east_tx1 },
    Gpio11 { name: east_tx2 },
    Gpio12 { name: south_rx2 },
    Gpio13 { name: south_rx1 },
    Gpio14 { name: south_rx0 },
    Gpio15 { name: south_tx0 },
    Gpio16 { name: south_tx1 },
    Gpio17 { name: south_tx2 },
    Gpio18 { name: west_rx2 },
    Gpio19 { name: west_rx1 },
    Gpio20 {
        name: led_or_si_clk1
    },
    Gpio21 { name: west_rx0 },
    Gpio22 { name: west_tx2 },
    Gpio23 { name: west_tx1 },
    Gpio24 { name: west_tx0 },
    Gpio25 { name: gpout3 },
    Gpio26 { name: si_sda },
    Gpio27 { name: si_scl },
    Gpio28 { name: oled_sda },
    Gpio29 { name: oled_scl },
);

/// This board has an Si5351 configurable clock module connected to XIN of the RP2040.
/// Instead there is a crystal connectod to the Si5351 module.
pub const SI5351_CRYSTAL_FREQ: u32 = 25_000_000;
