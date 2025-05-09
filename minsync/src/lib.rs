#![no_std]

pub mod clocks;
pub mod display;

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
    Gpio0 { name: north_0 },
    Gpio1 { name: north_1 },
    Gpio2 { name: north_2 },
    Gpio3 { name: north_3 },
    Gpio4 { name: north_4 },
    Gpio5 { name: north_5 },
    Gpio6 { name: east_6 },
    Gpio7 { name: east_7 },
    Gpio8 { name: east_8 },
    Gpio9 { name: east_9 },
    Gpio10 { name: east_10 },
    Gpio11 { name: east_11 },
    Gpio12 { name: oled_sda },
    Gpio13 { name: oled_scl },
    Gpio14 { name: si_sda },
    Gpio15 { name: si_scl },
    Gpio16 { name: south_16 },
    Gpio17 { name: south_17 },
    Gpio18 { name: south_18 },
    Gpio19 { name: south_19 },
    Gpio20 {
        name: led_or_si_clk1
    },
    Gpio21 { name: south_21 },
    Gpio22 { name: south_22 },
    Gpio23 { name: west_23 },
    Gpio24 { name: west_24 },
    Gpio25 { name: gpout3 },
    Gpio26 { name: west_26 },
    Gpio27 { name: west_27 },
    Gpio28 { name: west_28 },
    Gpio29 { name: west_29 },
);

/// This board has an Si5351 configurable clock module connected to XIN of the RP2040.
/// Instead there is a crystal connectod to the Si5351 module.
pub const SI5351_CRYSTAL_FREQ: u32 = 25_000_000;
