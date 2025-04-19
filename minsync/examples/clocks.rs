//! Sets up the Si5351 as clock source for the PLL, sets up the PLL, and then runs the
//! RP2040 of that PLL and continuously changes between two clock frequencies while still running.

#![no_std]
#![no_main]

use cortex_m::asm;
#[allow(unused_imports)]
use defmt::{error, info, warn};
use defmt_rtt as _;
use embedded_hal::digital::v2::OutputPin;
use fugit::{HertzU32, RateExtU32};
use panic_probe as _;
use si5351::Si5351;

use minsync::hal::{self, pac};
use minsync::{
    entry,
    hal::{pll::PLLConfig, Watchdog},
};

pub const SYS_PLL_CONFIG_100MHZ: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1000),
    refdiv: 1,
    post_div1: 6,
    post_div2: 6,
};

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
        .expect("Failed to do basic clock setup.");

    let mut si_clock =
        minsync::clocks::setup_si_as_crystal(minsync::si_i2c!(pac, pins, clocks, 1.kHz()))
            .expect("Failed to setup Si5351");

    // Test the clk2 debug endpoint
    si_clock
        .set_frequency(si5351::PLL::A, si5351::ClockOutput::Clk2, 10_000_000u32)
        .expect("Cannot set frequency");

    minsync::clocks::setup_pll_and_sysclk(&mut clocks, pac.PLL_SYS, &mut pac.XOSC, &mut pac.RESETS);

    let mut led_pin = pins.led_or_si_clk1.into_push_pull_output();

    let mut flashes = 0;

    loop {
        led_pin.set_high().unwrap();

        for _ in 0..30_000 {
            asm::nop();
        }

        led_pin.set_low().unwrap();

        flashes += 1;

        if flashes == 100 {
            si_clock
                .setup_pll(si5351::PLL::A, 35, 0, 0xfffff)
                .expect("Cannot setup PLL");
        }

        if flashes == 200 {
            si_clock
                .setup_pll(si5351::PLL::A, 35, 0x7ffff, 0xfffff)
                .expect("Cannot setup PLL");

            flashes = 0;
        }

        // for _ in 0..1_000_000 {
        //     asm::nop();
        // }
    }
}
