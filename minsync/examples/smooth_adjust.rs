#![no_std]
#![no_main]

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
    minsync::clocks::configure_gpout3_pll(&clocks, 50_000);
    let mut clocks = ClocksManager::new(clocks);

    let mut si_clock =
        minsync::clocks::setup_si_as_crystal(minsync::si_i2c!(pac, pins, clocks, 1.kHz()))
            .expect("Failed to setup Si5351");

    minsync::clocks::setup_pll(&mut clocks, pac.PLL_SYS, &mut pac.XOSC, &mut pac.RESETS)
        .expect("Failed to setup PLL");

    info!("PLL should be locked to the SI now.");

    // Sweep through different fractional values without interrupting the clock
    loop {
        for frac in (0..0x7ffff).step_by(100) {
            si_clock
                .setup_pll(si5351::PLL::A, 35, frac, 0xfffff)
                .expect("Cannot setup PLL");
        }
    }
}
