#![no_std]
#![no_main]

use cortex_m::asm;
#[allow(unused_imports)]
use defmt::{error, info, warn};
use defmt_rtt as _;
use fugit::{HertzU32, RateExtU32};
use panic_probe as _;
use rp2040_hal::clocks::ClocksManager;
use rp2040_hal::Watchdog;
use si5351::Si5351;

use minsync::hal::{self, pac};
use minsync::{entry, hal::pll::PLLConfig};

pub const SYS_PLL_CONFIG_100MHZ: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1000),
    refdiv: 1,
    post_div1: 5,
    post_div2: 2,
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

    let clocks = minsync::clocks::minimal_clock_setup(pac.CLOCKS, pac.ROSC, pins.gpout3)
        .expect("Failed to do basic clock setup.");

    let clocks = clocks.free();
    minsync::clocks::configure_gpout3_pll(&clocks, 1_000);
    let clocks = ClocksManager::new(clocks);

    let si_frequency: fugit::Rate<u32, 1, 1> = 12.MHz();

    let mut si_clock =
        minsync::clocks::setup_si_as_crystal(minsync::si_i2c!(pac, pins, clocks, 1.kHz()))
            .expect("Failed to setup Si5351");

    // Change the frequency safely with set_frequency. This interrupts the clock signal for a bit.
    // See the smooth_adjust for how to adjust the frequency without interupting the clock.
    loop {
        for freq_offset in 1..100 {
            let freq = si_frequency.to_Hz() + freq_offset * 10_000;
            info!("Setting freq to {}MHz", freq as f32 / 1_000_000.);

            si_clock
                .set_frequency(si5351::PLL::A, si5351::ClockOutput::Clk2, freq)
                .expect("Cannot set frequency");

            for _ in 0..1_000_000 {
                asm::nop();
            }
        }
    }
}
