//! Utility functions for clocking on the minsync PCB, both local to the rp2040 and with the Si5351

use fugit::HertzU32;
use hal::{
    clocks::{ClockError, ClockSource, ClocksManager},
    gpio::{
        bank0::{Gpio14, Gpio15, Gpio25},
        Function, FunctionClock, FunctionI2c, Pin, PullNone, PullType, PullUp,
    },
    pac::{self, PLL_SYS},
    pll::{self, setup_pll_blocking, Locked, PLLConfig, PhaseLockedLoop},
    rosc::RingOscillator,
    Clock, I2C,
};
use log::info;
use si5351::{Si5351, Si5351Device};

use crate::SI5351_CRYSTAL_FREQ;

pub fn configure_gpout3_sys(clocks: &pac::CLOCKS, divider: u32) {
    clocks.clk_gpout3_div().write(|w| {
        w.frac().variant(0);
        w.int().variant(divider)
    });

    clocks.clk_gpout3_ctrl().write(|w| {
        w.auxsrc().clk_sys();
        w.enable().set_bit()
    });
}

pub fn configure_gpout3_pll(clocks: &pac::CLOCKS, divider: u32) {
    clocks.clk_gpout3_div().write(|w| {
        w.frac().variant(0);
        w.int().variant(divider)
    });

    clocks.clk_gpout3_ctrl().write(|w| {
        w.auxsrc().clksrc_pll_sys(); // Debug PLL directly instead of the sysclk
        w.enable().set_bit()
    });
}

/// Sets up gpout 3 to route the system clock out, which is often used for debugging,
/// and sets up the ring oscillator as the system clock. This is nice to have in binaries
/// so they have something to run off should the Si5351 be unconfigured.
/// ```
/// minsync::clocks::minimal_clock_setup(pac.CLOCKS, pac.ROSC, pins.gpout3)
/// ```
pub fn minimal_clock_setup<F, P>(
    clocks: pac::CLOCKS,
    rosc: pac::ROSC,
    gpout3: Pin<Gpio25, F, P>,
) -> Result<ClocksManager, ClockError>
where
    F: Function,
    P: PullType,
{
    configure_gpout3_sys(&clocks, 1000);

    let _: Pin<Gpio25, FunctionClock, PullNone> = gpout3.reconfigure();

    let mut clocks = ClocksManager::new(clocks);
    let rosc = RingOscillator::new(rosc);
    let rosc = rosc.initialize();

    clocks
        .system_clock
        .configure_clock(&rosc, rosc.get_freq())?;

    Ok(clocks)
}

pub type SiI2C = I2C<
    pac::I2C1,
    (
        Pin<Gpio14, FunctionI2c, PullUp>,
        Pin<Gpio15, FunctionI2c, PullUp>,
    ),
>;

const CRYSTAL_FREQ: HertzU32 = HertzU32::from_raw(12_000_000u32);

/// Sets up SI as 12MHz to function as a default crystal for the rp2040
/// ```
/// minsync::clocks::setup_si_as_crystal(minsync::si_i2c!(pac, pins, clocks, 1.kHz()));
/// ```
pub fn setup_si_as_crystal(i2c: SiI2C) -> Result<Si5351Device<SiI2C>, si5351::Error> {
    let mut si_clock = Si5351Device::new(i2c, false, SI5351_CRYSTAL_FREQ);

    let status = si_clock.read_device_status()?.bits();
    info!("Created SI device. {:?}", status);

    si_clock.init(si5351::CrystalLoad::_8)?;
    si_clock.set_frequency(
        si5351::PLL::A,
        si5351::ClockOutput::Clk0,
        CRYSTAL_FREQ.to_Hz(),
    )?;

    info!("Configured SI.");

    Ok(si_clock)
}

pub const SYS_PLL_CONFIG: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1000),
    refdiv: 1,
    post_div1: 5,
    post_div2: 3,
};

/// Panics on failure.
/// ```
/// minsync::clocks::setup_pll_and_sysclk(&mut clocks, pac.PLL_SYS, &mut pac.XOSC, &mut pac.RESETS);
/// ```
pub fn setup_pll_and_sysclk(
    clocks: &mut ClocksManager,
    pll_sys: pac::PLL_SYS,
    xosc_device: &mut pac::XOSC,
    resets: &mut pac::RESETS,
) {
    let pll = setup_pll(clocks, pll_sys, xosc_device, resets).expect("Failed to set up PLL");
    setup_sysclk(clocks, pll).expect("Failed to setup sysclk");
}

pub fn setup_pll(
    clocks: &mut ClocksManager,
    pll_sys: pac::PLL_SYS,
    xosc_device: &mut pac::XOSC,
    resets: &mut pac::RESETS,
) -> Result<PhaseLockedLoop<Locked, PLL_SYS>, pll::Error> {
    xosc_device.ctrl().write(|w| w.enable().disable());

    setup_pll_blocking(pll_sys, CRYSTAL_FREQ, SYS_PLL_CONFIG, clocks, resets)
}

pub fn setup_sysclk(
    clocks: &mut ClocksManager,
    locked_pll_sys: PhaseLockedLoop<Locked, PLL_SYS>,
) -> Result<(), ClockError> {
    clocks
        .system_clock
        .configure_clock(&locked_pll_sys, locked_pll_sys.get_freq())
}

#[macro_export]
macro_rules! si_i2c {
    ($pac:expr, $pins:expr, $clocks:expr, $freq:expr) => {
        minsync::hal::I2C::i2c1(
            $pac.I2C1,
            $pins.si_sda.reconfigure(),
            $pins.si_scl.reconfigure(),
            $freq,
            &mut $pac.RESETS,
            &$clocks.system_clock,
        )
    };
}
