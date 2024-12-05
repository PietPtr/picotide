#![no_std]
#![no_main]

#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use core::u32;

use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::v2::OutputPin;
use fugit::HertzU32;
use panic_probe as _;
use pio::Program;
use pio_proc::pio_file;
use rp_pico::{
    entry,
    hal::{
        clocks::{Clock, ClockSource, ClocksManager},
        gpio::{self, FunctionPio0},
        pio::{PIOBuilder, PIOExt, PinDir},
        pll::{setup_pll_blocking, PLLConfig},
        sio::Sio,
        xosc::setup_xosc_blocking,
    },
    pac,
};

pub const EXTERNAL_XTAL_FREQ_HZ: HertzU32 = HertzU32::from_raw(12_000_000u32);

pub const SYS_PLL_CONFIG_100MHZ: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1200),
    refdiv: 1,
    post_div1: 6,
    post_div2: 2,
};

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let sio = Sio::new(pac.SIO);

    let mut clocks = ClocksManager::new(pac.CLOCKS);

    let xosc = setup_xosc_blocking(pac.XOSC, EXTERNAL_XTAL_FREQ_HZ).unwrap();

    let pll_sys = setup_pll_blocking(
        pac.PLL_SYS,
        xosc.operating_frequency(),
        SYS_PLL_CONFIG_100MHZ,
        &mut clocks,
        &mut pac.RESETS,
    )
    .unwrap();

    let start_freq = HertzU32::MHz(60);

    clocks
        .system_clock
        .configure_clock(&pll_sys, start_freq)
        .unwrap();

    info!(
        "Configured system clock at frequency: {:?}MHz",
        start_freq.to_Hz() as f32 / 1e6
    );

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let exposed_clock_pin = pins.gpio10.into_function::<FunctionPio0>();

    let (mut pio, sm0, sm1, sm2, sm3) = pac.PIO0.split(&mut pac.RESETS);

    let toggle_pin_program = pio_file!("src/programs.pio", select_program("toggle_pin")).program;
    let toggle_pin = pio.install(&toggle_pin_program).unwrap();

    let (mut sm0, _rx0, _tx0) = PIOBuilder::from_program(toggle_pin)
        .set_pins(exposed_clock_pin.id().num, 1)
        .clock_divisor_fixed_point(2, 0)
        .build(sm0);

    sm0.set_pindirs([(exposed_clock_pin.id().num, PinDir::Output)]);
    sm0.start();

    let mut led_pin = pins.gpio25.into_push_pull_output();

    info!("Start.");

    // pac.PPB.syst_csr.write(|w| w.clksource().set_bit());
    // pac.PPB.syst_csr.write(|w| w.enable().set_bit());
    // pac.PPB.syst_csr.write(|w| unsafe { w.bits(0x5) });
    // pac.PPB.syst_rvr.write(|w| unsafe { w.bits(0xffffff) });

    // info!(
    //     "\nclksource={} ({})\nenabled={}\ntickint={}\nrvr={:#x}",
    //     if pac.PPB.syst_csr.read().clksource().bit() {
    //         "processor"
    //     } else {
    //         "refclock"
    //     },
    //     pac.PPB.syst_csr.read().clksource().bit(),
    //     pac.PPB.syst_csr.read().enable().bit_is_set(),
    //     pac.PPB.syst_csr.read().tickint().bit(),
    //     pac.PPB.syst_rvr.read().bits(),
    // );

    // let points = [
    //     pac.PPB.syst_cvr.read().current().bits(),
    //     pac.PPB.syst_cvr.read().current().bits(),
    //     pac.PPB.syst_cvr.read().current().bits(),
    //     pac.PPB.syst_cvr.read().current().bits(),
    //     pac.PPB.syst_cvr.read().current().bits(),
    //     pac.PPB.syst_cvr.read().current().bits(),
    //     pac.PPB.syst_cvr.read().current().bits(),
    //     pac.PPB.syst_cvr.read().current().bits(),
    // ];

    // pac.PPB
    //     .syst_cvr
    //     .write(|w| unsafe { w.current().bits(0xffffff) });

    // let after_reset = pac.PPB.syst_cvr.read().current().bits();

    // info!("{:x}\n{:x}", points, after_reset);

    // delay.delay_ms(1000);
    // info!("Setting clock higher.");

    // clocks
    //     .system_clock
    //     .configure_clock(&pll_sys, start_freq + HertzU32::MHz(10))
    //     .unwrap();

    // info!(
    //     "Freq should now be: {}MHz",
    //     clocks.system_clock.get_freq().to_Hz() as f32 / 1e6
    // );
    loop {

        // info!("Setting clock lower.");

        // clocks
        //     .system_clock
        //     .configure_clock(&pll_sys, start_freq - HertzU32::MHz(1))
        //     .unwrap();
    }
}
