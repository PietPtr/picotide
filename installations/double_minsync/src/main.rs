#![no_std]
#![no_main]

// #[link_section = ".boot2"]
// #[no_mangle]
// #[used]
// pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use core::cell::RefCell;
use core::fmt::Write;
use core::sync::atomic::{self, AtomicU32};

use bittide::bittide::{BittideChannelControlDebugInfo, BittideChannelControlError};
use bittide_impls::boards::minsync_v02::{MinsyncPins, MinsyncV02};
use controllers::pid::PidSettings;
use controllers::si5351::Si5351Controller;
use cortex_m_rt::exception;
use critical_section::Mutex;
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

    let frequency_controller = Si5351Controller::new(
        si_clock,
        4,
        PidSettings {
            kp: I16F16::from_num(0.01),
            ki: I16F16::from_num(0.00000001),
            kd: I16F16::from_num(0.01),
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

    let mut led = pins.rest.led_or_si_clk1.into_push_pull_output();

    #[allow(clippy::empty_loop)]
    loop {
        led.toggle().unwrap();

        DEBUG.draw(&mut display, Point::new(0, 9)).ok();
        display.flush().ok();
    }
}

static GLOBAL_CONTROL: Mutex<RefCell<Option<bittide_impls::boards::minsync_v02::Control>>> =
    Mutex::new(RefCell::new(None));

pub static DEBUG: BufferDebugger = BufferDebugger::new();

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

        info!("bittide algo took {} cycles", start - end);

        DEBUG.update(control.debug(), result);

        // TODO: visualize freq stabilizer

        *refcell = Some(control);
    });
}

// TODO: move to debugger crate
pub trait BittideControlDebugger {
    fn update(
        &self,
        debug_info: &BittideChannelControlDebugInfo,
        result: Result<(), BittideChannelControlError>,
    );
}

#[derive(Debug, Default)]
pub struct BufferDebugger {
    buffer_levels_a: [AtomicU32; 4],
    error: AtomicU32,
    rx_sync_message_counter: AtomicU32,
    rx_comm_message_counter: AtomicU32,
}

impl BufferDebugger {
    pub const fn new() -> Self {
        Self {
            buffer_levels_a: [
                AtomicU32::new(0),
                AtomicU32::new(1),
                AtomicU32::new(2),
                AtomicU32::new(3),
            ],
            error: AtomicU32::new(0),
            rx_sync_message_counter: AtomicU32::new(0),
            rx_comm_message_counter: AtomicU32::new(0),
        }
    }

    pub fn draw<D>(&self, display: &mut D, position: Point) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        let size = Size::new(128, 22);

        Rectangle::new(position, size)
            .draw_styled(&PrimitiveStyle::with_fill(BinaryColor::Off), display)?;

        let mut buffer_texts = String::<22>::new();

        let cardinals = ["N", "E", "S", "W"];

        for (buffer_level, cardinal) in self.buffer_levels_a.iter().zip(cardinals.iter()) {
            let mut buffer = itoa::Buffer::new();
            let i_as_str = buffer.format(buffer_level.load(atomic::Ordering::Relaxed));
            buffer_texts.push_str(cardinal).ok();
            buffer_texts.push_str(i_as_str).ok();
            buffer_texts.push(' ').ok();
        }

        Text::with_baseline(&buffer_texts, position, DEFAULT_TEXT_STYLE, Baseline::Top)
            .draw(display)?;

        let error = BittideChannelControlError::decode(self.error.load(atomic::Ordering::Relaxed));

        let mut line_two = String::<22>::new();

        let mut buffer = itoa::Buffer::new();
        let comm_str = buffer.format(self.rx_comm_message_counter.load(atomic::Ordering::Relaxed));

        let mut buffer = itoa::Buffer::new();
        let sync_str = buffer.format(self.rx_sync_message_counter.load(atomic::Ordering::Relaxed));

        line_two.push_str(comm_str).ok();
        line_two.push_str(" ").ok();
        line_two.push_str(sync_str).ok();
        line_two.push_str(" ").ok();

        match error {
            Ok(()) => {
                line_two.push_str("Ok()").ok();
            }
            Err(err) => {
                write!(&mut line_two, "{:?}", err).ok();
            }
        };

        Text::with_baseline(
            &line_two,
            position + Point::new(0, 9),
            DEFAULT_TEXT_STYLE,
            Baseline::Top,
        )
        .draw(display)?;

        Ok(())
    }
}

impl BittideControlDebugger for BufferDebugger {
    fn update(
        &self,
        debug_info: &BittideChannelControlDebugInfo,
        result: Result<(), BittideChannelControlError>,
    ) {
        for (level, atomic) in debug_info
            .buffer_levels
            .iter()
            .zip(self.buffer_levels_a.iter())
        {
            atomic.store(*level, atomic::Ordering::Relaxed);
        }

        self.error.store(
            BittideChannelControlError::encode(result),
            atomic::Ordering::Relaxed,
        );

        self.rx_comm_message_counter.store(
            debug_info.rx_comm_message_counter,
            atomic::Ordering::Relaxed,
        );

        self.rx_sync_message_counter.store(
            debug_info.rx_sync_message_counter,
            atomic::Ordering::Relaxed,
        );
    }
}
