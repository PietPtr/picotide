#![no_std]
#![no_main]

// #[link_section = ".boot2"]
// #[no_mangle]
// #[used]
// pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use core::cell::RefCell;
use core::fmt::Write;
use core::sync::atomic::{self, AtomicU32};

use bittide::bittide::{BittideChannelControlDebugInfo, BittideChannelControlError, BittideFifo};
use bittide_impls::chips::rp2040::Rp2040Links;
use controllers::pid::PidSettings;
use controllers::si5351::Si5351Controller;
use cortex_m::asm;
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
use embedded_hal::digital::v2::{InputPin, ToggleableOutputPin};
use fixed::types::I16F16;
use fugit::{HertzU32, RateExtU32};
use heapless::{String, Vec};
use minsync::display::{draw_key_integral, draw_key_value, DEFAULT_TEXT_STYLE};
use minsync::hal::gpio::{self, FunctionPio0, FunctionPio1};
use minsync::hal::pio::PIOExt;
use minsync::si_i2c;
use panic_probe as _;
use pitopi::{LinkConfig, Pitopi};

use minsync::hal;
use minsync::hal::pac;
use minsync::{entry, hal::Watchdog};

mod generated_constants;

pub const EXTERNAL_XTAL_FREQ_HZ: HertzU32 = HertzU32::from_raw(12_000_000u32);

/// The divisor of how many CPU cycles should pass before a new word is sent to all neigboring nodes.
// pub const CLOCKS_PER_SYNC_WORD: u32 = 4096;
pub const CLOCKS_PER_SYNC_WORD: u32 = 4096 * 256;

#[entry]
fn main_pitopi_test() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut core = pac::CorePeripherals::take().unwrap();
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
    let si_clock = minsync::clocks::setup_si_as_crystal(si_i2c!(pac, pins, clocks, 1.kHz()))
        .expect("Failed to setup Si5351");
    minsync::clocks::setup_pll_and_sysclk(&mut clocks, pac.PLL_SYS, &mut pac.XOSC, &mut pac.RESETS);
    // minsync::clocks::setup_pll(&mut clocks, pac.PLL_SYS, &mut pac.XOSC, &mut pac.RESETS).unwrap();

    let mut display = minsync::display::setup(minsync::display_i2c!(pac, pins, clocks, 100.kHz()))
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

    let (rx_pio, rx_sm0, rx_sm1, rx_sm2, rx_sm3) = pac.PIO0.split(&mut pac.RESETS);
    let (tx_pio, tx_sm0, tx_sm1, tx_sm2, tx_sm3) = pac.PIO1.split(&mut pac.RESETS);

    let rx0_data = pins.north_3.into_function::<FunctionPio0>().into_dyn_pin();
    let rx0_word = pins.north_4.into_function::<FunctionPio0>().into_dyn_pin();
    let rx0_clk = pins.north_5.into_function::<FunctionPio0>().into_dyn_pin();

    let rx1_data = pins.east_9.into_function::<FunctionPio0>().into_dyn_pin();
    let rx1_word = pins.east_10.into_function::<FunctionPio0>().into_dyn_pin();
    let rx1_clk = pins.east_11.into_function::<FunctionPio0>().into_dyn_pin();

    let rx2_data = pins.south_19.into_function::<FunctionPio0>().into_dyn_pin();
    let rx2_word = pins.south_21.into_function::<FunctionPio0>().into_dyn_pin();
    let rx2_clk = pins.south_22.into_function::<FunctionPio0>().into_dyn_pin();

    let rx3_data = pins.west_27.into_function::<FunctionPio0>().into_dyn_pin();
    let rx3_word = pins.west_28.into_function::<FunctionPio0>().into_dyn_pin();
    let rx3_clk = pins.west_29.into_function::<FunctionPio0>().into_dyn_pin();

    let tx0_data = pins.north_2.into_function::<FunctionPio1>().into_dyn_pin();
    let tx0_word = pins.north_1.into_function::<FunctionPio1>().into_dyn_pin();
    let tx0_clk = pins.north_0.into_function::<FunctionPio1>().into_dyn_pin();

    let tx1_data = pins.east_8.into_function::<FunctionPio1>().into_dyn_pin();
    let tx1_word = pins.east_7.into_function::<FunctionPio1>().into_dyn_pin();
    let tx1_clk = pins.east_6.into_function::<FunctionPio1>().into_dyn_pin();

    let tx2_data = pins.south_18.into_function::<FunctionPio1>().into_dyn_pin();
    let tx2_word = pins.south_17.into_function::<FunctionPio1>().into_dyn_pin();
    let tx2_clk = pins.south_16.into_function::<FunctionPio1>().into_dyn_pin();

    let tx3_data = pins.west_26.into_function::<FunctionPio1>().into_dyn_pin();
    let tx3_word = pins.west_24.into_function::<FunctionPio1>().into_dyn_pin();
    let tx3_clk = pins.west_23.into_function::<FunctionPio1>().into_dyn_pin();

    let mut pitopi = Pitopi::new(rx_pio, tx_pio);

    pitopi.install_programs();

    let north_link_config = LinkConfig {
        rx_program: pitopi::RxProgram::Consecutive,
        tx_program: pitopi::TxProgram::SidesetWC,
    };

    let (_, mut rx0, _, mut tx0) = pitopi
        .setup_link(
            north_link_config,
            rx_sm0,
            rx0_data,
            rx0_clk,
            rx0_word,
            tx_sm0,
            tx0_data,
            tx0_clk,
            tx0_word,
        )
        .unwrap();

    let east_link_config = LinkConfig {
        rx_program: pitopi::RxProgram::Consecutive,
        tx_program: pitopi::TxProgram::SidesetWC,
    };

    let (_, mut rx1, _, mut tx1) = pitopi
        .setup_link(
            east_link_config,
            rx_sm1,
            rx1_data,
            rx1_clk,
            rx1_word,
            tx_sm1,
            tx1_data,
            tx1_clk,
            tx1_word,
        )
        .unwrap();

    let south_link_config = LinkConfig {
        rx_program: pitopi::RxProgram::P023,
        tx_program: pitopi::TxProgram::SidesetWC,
    };

    let (_, mut rx2, _, mut tx2) = pitopi
        .setup_link(
            south_link_config,
            rx_sm2,
            rx2_data,
            rx2_clk,
            rx2_word,
            tx_sm2,
            tx2_data,
            tx2_clk,
            tx2_word,
        )
        .unwrap();

    let west_link_config = LinkConfig {
        rx_program: pitopi::RxProgram::Consecutive,
        tx_program: pitopi::TxProgram::SidesetWC,
    };

    let (_, mut rx3, _, mut tx3) = pitopi
        .setup_link(
            west_link_config,
            rx_sm3,
            rx3_data,
            rx3_clk,
            rx3_word,
            tx_sm3,
            tx3_data,
            tx3_clk,
            tx3_word,
        )
        .unwrap();

    let mut led = pins
        .led_or_si_clk1
        .into_push_pull_output_in_state(gpio::PinState::High);

    let mut i = 0;
    loop {
        #[allow(clippy::identity_op)]
        tx0.write(i & 0b1111_1111 | 0 << 8);
        tx1.write(i & 0b1111_1111 | 1 << 8);
        tx2.write(i & 0b1111_1111 | 2 << 8);
        tx3.write(i & 0b1111_1111 | 3 << 8);
        i += 1;

        for _ in 0..1000000 {
            if let Some(v) = rx0.read() {
                info!("[0] [{}] {:#?}", v >> 8, v & 0b1111_1111);
            }
            if let Some(v) = rx1.read() {
                info!("[1] [{}] {:#?}", v >> 8, v & 0b1111_1111);
            }
            if let Some(v) = rx2.read() {
                info!("[2] [{}] {:#?}", v >> 8, v & 0b1111_1111);
            }
            if let Some(v) = rx3.read() {
                info!("[3] [{}] {:#?}", v >> 8, v & 0b1111_1111);
            }
        }
        led.toggle().ok();
    }
}

/*
fn main_full() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut core = pac::CorePeripherals::take().unwrap();
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
    let si_clock = minsync::clocks::setup_si_as_crystal(si_i2c!(pac, pins, clocks, 1.kHz()))
        .expect("Failed to setup Si5351");
    minsync::clocks::setup_pll_and_sysclk(&mut clocks, pac.PLL_SYS, &mut pac.XOSC, &mut pac.RESETS);
    // minsync::clocks::setup_pll(&mut clocks, pac.PLL_SYS, &mut pac.XOSC, &mut pac.RESETS).unwrap();

    let mut display = minsync::display::setup(minsync::display_i2c!(pac, pins, clocks, 100.kHz()))
        .expect("Couldn't set up display.");

    draw_key_value(&mut display, 0, "Name", generated_constants::NAME).unwrap();
    draw_key_integral(&mut display, 1, "SI frac", generated_constants::SI_FRAC).unwrap();

    display.flush().unwrap();

    // -- Bittide setup -- TODO: make common in BSP

    {
        let (rx_pio, rx_sm0, rx_sm1, rx_sm2, rx_sm3) = pac.PIO0.split(&mut pac.RESETS);
        let (tx_pio, tx_sm0, tx_sm1, tx_sm2, tx_sm3) = pac.PIO1.split(&mut pac.RESETS);

        let rx0_data = pins
            .north_rx0
            .into_function::<FunctionPio0>()
            .into_dyn_pin();
        let rx0_clk = pins
            .north_rx1
            .into_function::<FunctionPio0>()
            .into_dyn_pin();
        let rx0_word = pins
            .north_rx2
            .into_function::<FunctionPio0>()
            .into_dyn_pin();

        let rx1_data = pins.east_rx0.into_function::<FunctionPio0>().into_dyn_pin();
        let rx1_clk = pins.east_rx1.into_function::<FunctionPio0>().into_dyn_pin();
        let rx1_word = pins.east_rx2.into_function::<FunctionPio0>().into_dyn_pin();

        let rx2_data = pins
            .south_rx0
            .into_function::<FunctionPio0>()
            .into_dyn_pin();
        let rx2_clk = pins
            .south_rx1
            .into_function::<FunctionPio0>()
            .into_dyn_pin();
        let rx2_word = pins
            .south_rx2
            .into_function::<FunctionPio0>()
            .into_dyn_pin();

        let rx3_data = pins.west_rx0.into_function::<FunctionPio0>().into_dyn_pin();
        let rx3_clk = pins.west_rx1.into_function::<FunctionPio0>().into_dyn_pin();
        let rx3_word = pins.west_rx2.into_function::<FunctionPio0>().into_dyn_pin();

        let tx0_data = pins
            .north_tx0
            .into_function::<FunctionPio1>()
            .into_dyn_pin();
        let tx0_clk = pins
            .north_tx1
            .into_function::<FunctionPio1>()
            .into_dyn_pin();
        let tx0_word = pins
            .north_tx2
            .into_function::<FunctionPio1>()
            .into_dyn_pin();

        let tx1_data = pins.east_tx0.into_function::<FunctionPio1>().into_dyn_pin();
        let tx1_clk = pins.east_tx1.into_function::<FunctionPio1>().into_dyn_pin();
        let tx1_word = pins.east_tx2.into_function::<FunctionPio1>().into_dyn_pin();

        let tx2_data = pins
            .south_tx0
            .into_function::<FunctionPio1>()
            .into_dyn_pin();
        let tx2_clk = pins
            .south_tx1
            .into_function::<FunctionPio1>()
            .into_dyn_pin();
        let tx2_word = pins
            .south_tx2
            .into_function::<FunctionPio1>()
            .into_dyn_pin();

        let tx3_data = pins.west_tx0.into_function::<FunctionPio1>().into_dyn_pin();
        let tx3_clk = pins.west_tx1.into_function::<FunctionPio1>().into_dyn_pin();
        let tx3_word = pins.west_tx2.into_function::<FunctionPio1>().into_dyn_pin();

        let mut pitopi = Pitopi::new(rx_pio, tx_pio);

        pitopi.install_programs();

        let (rx0_sm, mut rx0, _, tx0) = pitopi
            .setup_link(
                rx_sm0, rx0_data, rx0_clk, rx0_word, tx_sm0, tx0_data, tx0_clk, tx0_word,
            )
            .unwrap();

        let (_, rx1, _, tx1) = pitopi
            .setup_link(
                rx_sm1, rx1_data, rx1_clk, rx1_word, tx_sm1, tx1_data, tx1_clk, tx1_word,
            )
            .unwrap();

        let (_, rx2, _, tx2) = pitopi
            .setup_link(
                rx_sm2, rx2_data, rx2_clk, rx2_word, tx_sm2, tx2_data, tx2_clk, tx2_word,
            )
            .unwrap();

        let (_, rx3, _, tx3) = pitopi
            .setup_link(
                rx_sm3, rx3_data, rx3_clk, rx3_word, tx_sm3, tx3_data, tx3_clk, tx3_word,
            )
            .unwrap();

        let sio_fifo = sio.fifo;

        let tide_fifos = [
            BittideFifo::new(),
            BittideFifo::new(),
            BittideFifo::new(),
            BittideFifo::new(),
        ];

        let link_mask = [
            generated_constants::LINK_NORTH,
            generated_constants::LINK_EAST,
            generated_constants::LINK_SOUTH,
            generated_constants::LINK_WEST,
        ];

        info!("{:?}", link_mask);
        use bittide::bittide::Links;

        // let mut links = Rp2040Links::new(rx0, rx1, rx2, rx3, tx0, tx1, tx2, tx3);
        if !generated_constants::SHOULD_SEND {
            info!("Reading rx0");

            loop {
                if let Some(v) = rx0.read() {
                    info!("{:#?}", v);
                }

                let mut addresses = Vec::<_, 100>::new();

                while !addresses.is_full() {
                    addresses.push(rx0_sm.instruction_address()).unwrap();
                }

                info!("{:?}", addresses);
            }
        }

        let tide_controller = bittide_impls::boards::minsync_v02::Control::new(
            Si5351Controller::new(
                si_clock,
                4,
                PidSettings {
                    kp: I16F16::from_num(0.01),
                    ki: I16F16::from_num(0.00000001),
                    kd: I16F16::from_num(0.01),
                },
            ),
            Rp2040Links::new(rx0, rx1, rx2, rx3, tx0, tx1, tx2, tx3),
            link_mask,
            bittide_impls::chips::rp2040::SioFifo(sio_fifo),
            tide_fifos,
        );

        critical_section::with(|cs| {
            GLOBAL_CONTROL.borrow(cs).replace(Some(tide_controller));
        });

        bittide_impls::chips::rp2040::setup_interrupt(CLOCKS_PER_SYNC_WORD, &mut core.SYST);
    }

    let mut led = pins
        .led_or_si_clk1
        .into_push_pull_output_in_state(gpio::PinState::High);

    info!("halloooooooooo");

    #[allow(clippy::empty_loop)]
    loop {
        led.toggle().ok();

        DEBUG.draw(&mut display, Point::new(0, 9)).ok();
        display.flush().ok();
    }
}
*/

static GLOBAL_CONTROL: Mutex<RefCell<Option<bittide_impls::boards::minsync_v02::Control>>> =
    Mutex::new(RefCell::new(None));

pub static DEBUG: BufferDebugger = BufferDebugger::new();

// TODO: find out if drawing can be safely stalled by bittide controllers

#[exception]
fn SysTick() {
    critical_section::with(|cs| {
        let mut refcell = GLOBAL_CONTROL
            .borrow(cs)
            .try_borrow_mut()
            .expect("Control algorithm cannot keep up, already borrowed");
        let mut control = refcell.take().expect("control not initialized.");

        let result = control.interrupt();

        DEBUG.update(control.debug(), result);

        *refcell = Some(control);
    });
}

pub trait OledDebugger {
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

        let mut error_str = String::<22>::new();

        match error {
            Ok(()) => {
                error_str.push_str("Ok()").ok();
            }
            Err(err) => {
                write!(&mut error_str, "{:?}", err).ok();
            }
        };

        Text::with_baseline(
            &error_str,
            position + Point::new(0, 9),
            DEFAULT_TEXT_STYLE,
            Baseline::Top,
        )
        .draw(display)?;

        Ok(())
    }
}

impl OledDebugger for BufferDebugger {
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
    }
}
