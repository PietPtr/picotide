#![no_std]
#![no_main]

// #[link_section = ".boot2"]
// #[no_mangle]
// #[used]
// pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use cortex_m::asm;
#[allow(unused_imports)]
use defmt::{error, info, warn};
use defmt_rtt as _;
use embedded_graphics::mono_font::ascii::FONT_6X9;
use embedded_graphics::prelude::DrawTarget;
use embedded_graphics::primitives::{PrimitiveStyle, StyledDrawable};
use embedded_graphics::{
    mono_font::MonoTextStyleBuilder,
    pixelcolor::BinaryColor,
    prelude::{Dimensions, Point, Size},
    primitives::Rectangle,
    text::{Baseline, Text},
    Drawable,
};
use embedded_hal::digital::v2::OutputPin;
use fugit::{HertzU32, RateExtU32};
use heapless::String;
use minsync::display::{draw_key_integral, draw_key_value};
use minsync::si_i2c;
use panic_probe as _;
use si5351::Si5351;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

use minsync::hal;
use minsync::hal::pac;
use minsync::{
    entry,
    hal::{pll::PLLConfig, Watchdog, I2C},
};

mod generated_constants;

pub const EXTERNAL_XTAL_FREQ_HZ: HertzU32 = HertzU32::from_raw(12_000_000u32);

pub const SYS_PLL_CONFIG_100MHZ: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1000),
    refdiv: 1,
    post_div1: 5,
    post_div2: 1,
};

/// The divisor of how many CPU cycles should pass before a new word is sent to all neigboring nodes.
pub const CLOCKS_PER_SYNC_WORD: u32 = 4096;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let _core = pac::CorePeripherals::take().unwrap();
    let sio = hal::Sio::new(pac.SIO);

    let watchdog = Watchdog::new(pac.WATCHDOG);
    watchdog.disable();

    let pins = minsync::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut clocks = minsync::clocks::minimal_clock_setup(pac.CLOCKS, pac.ROSC, pins.gpout3)
        .expect("Failed to do minimal clock setup.");
    let mut si_clock = minsync::clocks::setup_si_as_crystal(si_i2c!(pac, pins, clocks, 1.kHz()))
        .expect("Failed to setup Si5351");
    minsync::clocks::setup_pll_and_sysclk(&mut clocks, pac.PLL_SYS, &mut pac.XOSC, &mut pac.RESETS);

    let mut display = minsync::display::setup(minsync::display_i2c!(pac, pins, clocks, 300.kHz()))
        .expect("Couldn't set up display.");

    draw_key_value(&mut display, 0, "Name", generated_constants::NAME).unwrap();
    draw_key_integral(&mut display, 1, "SI frac", generated_constants::SI_FRAC).unwrap();

    display.flush().unwrap();

    let mut led = pins.led_or_si_clk1.into_push_pull_output();

    loop {
        for frac in (-generated_constants::SI_FRAC..generated_constants::SI_FRAC)
            .chain((-generated_constants::SI_FRAC..generated_constants::SI_FRAC).rev())
        {
            for _ in 0..5_000_000 {
                asm::nop();
            }

            led.set_high().unwrap();

            for _ in 0..1000 {
                asm::nop();
            }

            led.set_low().unwrap();

            let frac = (0x3ffff + frac) as u32;

            si_clock
                .setup_pll(si5351::PLL::A, 35, frac, 0xfffff)
                .expect("Cannot setup PLL");

            draw_key_integral(&mut display, 2, "frac", frac).unwrap();
            display.flush().unwrap();
        }
    }
}

// TODO: find out if drawing can be safely stalled by bittide controllers
