#![no_std]
#![no_main]

// #[link_section = ".boot2"]
// #[no_mangle]
// #[used]
// pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use core::cell::RefCell;
use core::fmt::Write;
use core::sync::atomic::{self, AtomicI32, AtomicU32};

use bittide::bittide::{BittideChannelControlDebugInfo, BittideChannelControlError};
use bittide_impls::boards::minsync_v02::{MinsyncPins, MinsyncV02};
use controllers::pid::PidSettings;
use controllers::si5351::{Si5351Controller, Si5351Debug};
use cortex_m_rt::exception;
use critical_section::Mutex;
use debugging::debuggers::graph::{GraphDebugger, GraphDebuggerSettings};
use debugging::debuggers::text::TextDebugger;
use debugging::BittideControlDebugger;
#[allow(unused_imports)]
use defmt::{error, info, warn};
use defmt_rtt as _;
use embedded_graphics::prelude::DrawTarget;
use embedded_graphics::primitives::{PrimitiveStyle, StyledDrawable};
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{Point, Size},
    primitives::Rectangle,
    text::{Baseline, Text},
    Drawable,
};
use embedded_hal::digital::v2::ToggleableOutputPin;
use fixed::types::I16F16;
use fugit::{HertzU32, RateExtU32};
use heapless::String;
use minsync::display::{draw_key_integral, draw_key_value, DEFAULT_TEXT_STYLE};
use minsync::si_i2c;
use panic_probe as _;

use minsync::hal;
use minsync::hal::pac;
use minsync::{entry, hal::Watchdog};

mod generated_constants;

pub const EXTERNAL_XTAL_FREQ_HZ: HertzU32 = HertzU32::from_raw(12_000_000u32);

/// The divisor of how many CPU cycles should pass before a new word is sent to all neigboring nodes.
// pub const CLOCKS_PER_SYNC_WORD: u32 = 4096;
pub const CLOCKS_PER_SYNC_WORD: u32 = 700_000;

#[entry]
fn main_pitopi_test() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut core = pac::CorePeripherals::take().unwrap();
    let sio = hal::Sio::new(pac.SIO);

    let watchdog = Watchdog::new(pac.WATCHDOG);
    watchdog.disable();

    let pins: MinsyncPins = minsync::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    )
    .into();

    let mut clocks = minsync::clocks::minimal_clock_setup(pac.CLOCKS, pac.ROSC, pins.rest.gpout3)
        .expect("Failed to do minimal clock setup.");
    let si_clock = minsync::clocks::setup_si_as_crystal(si_i2c!(pac, pins.rest, clocks, 1.kHz()))
        .expect("Failed to setup Si5351");
    minsync::clocks::setup_pll_and_sysclk(&mut clocks, pac.PLL_SYS, &mut pac.XOSC, &mut pac.RESETS);
    // minsync::clocks::setup_pll(&mut clocks, pac.PLL_SYS, &mut pac.XOSC, &mut pac.RESETS).unwrap();

    let mut display =
        minsync::display::setup(minsync::display_i2c!(pac, pins.rest, clocks, 100.kHz()))
            .expect("Couldn't set up display.");

    draw_key_value(&mut display, 0, "Name", generated_constants::NAME).unwrap();
    draw_key_integral(
        &mut display,
        1,
        "should send",
        generated_constants::SHOULD_SEND as u32,
    )
    .unwrap();

    display.flush().unwrap();

    let link_mask = [
        generated_constants::LINK_NORTH,
        generated_constants::LINK_EAST,
        generated_constants::LINK_SOUTH,
        generated_constants::LINK_WEST,
    ];

    const KP: I16F16 = I16F16::unwrapped_from_str("0.001");
    const KD: I16F16 = I16F16::unwrapped_from_str("0.0001");
    const KI: I16F16 = I16F16::unwrapped_from_str("0.00001");

    info!(
        "Kp {} \nKd {} \nKi {}",
        KP.to_bits(),
        KD.to_bits(),
        KI.to_bits()
    );

    let frequency_controller = Si5351Controller::new(
        si_clock,
        4,
        PidSettings {
            kp: KP,
            ki: KD,
            kd: KI,
        },
    );

    let bittide_controller = MinsyncV02::setup(
        link_mask,
        frequency_controller,
        pins.link,
        pac.PIO0,
        pac.PIO1,
        &mut pac.RESETS,
        sio.fifo,
    );

    critical_section::with(|cs| {
        GLOBAL_CONTROL.borrow(cs).replace(Some(bittide_controller));
    });

    bittide_impls::chips::rp2040::setup_interrupt(CLOCKS_PER_SYNC_WORD, &mut core.SYST);

    #[allow(unused_variables, unused_mut)]
    let mut led = pins.rest.led_or_si_clk1.into_push_pull_output();

    loop {
        // led.toggle().unwrap();

        DEBUG.draw(&mut display, Point::new(0, 9)).ok();
        display.flush().ok();
    }
}

static GLOBAL_CONTROL: Mutex<RefCell<Option<bittide_impls::boards::minsync_v02::Control>>> =
    Mutex::new(RefCell::new(None));

pub static DEBUG: GraphDebugger = GraphDebugger::new(GraphDebuggerSettings {
    buffer_size: bittide_impls::boards::minsync_v02::BUFFER_SIZE,
});

#[exception]
fn SysTick() {
    critical_section::with(|cs| {
        let mut refcell = GLOBAL_CONTROL
            .borrow(cs)
            .try_borrow_mut()
            .expect("Control algorithm cannot keep up, already borrowed");
        let mut control = refcell.take().expect("control not initialized.");

        // safe because we're only going to be reading systick
        let core = unsafe { pac::CorePeripherals::steal() };
        let start = core.SYST.cvr.read();
        let result = control.interrupt();
        let end = core.SYST.cvr.read();

        // info!("bittide algo took {} cycles", start - end);

        DEBUG.update(control.debug(), result);

        // TODO: visualize freq stabilizer

        *refcell = Some(control);
    });
}
