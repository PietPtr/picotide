#![no_std]
#![no_main]

#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use core::cell::RefCell;

use controllers::pid::PidSettings;
use controllers::si5351::Si5351Controller;
use cortex_m::asm;
use cortex_m_rt::exception;
use critical_section::Mutex;
#[allow(unused_imports)]
use defmt::{error, info, warn};
use defmt_rtt as _;
use fixed::types::I16F16;
use fugit::HertzU32;
use fugit::RateExtU32;
use panic_probe as _;
use pitopi::Pitopi;
use rp_pico::hal::gpin::GpIn0;
use rp_pico::hal::gpio::bank0::Gpio21;
use rp_pico::hal::gpio::FunctionClock;
use rp_pico::hal::gpio::Pin;
use rp_pico::hal::gpio::PullNone;
use rp_pico::hal::rosc::RingOscillator;
use rp_pico::hal::I2C;
use rp_pico::pac;
use rp_pico::{
    entry,
    hal::{
        clocks::{Clock, ClockSource, ClocksManager},
        gpio::{self, FunctionPio0, FunctionPio1},
        pio::PIOExt,
        pll::PLLConfig,
        sio::Sio,
    },
};

use bittide::bittide::BittideFifo;
use bittide_impls::chips::rp2040::Rp2040Links;
use si5351::Si5351;
use si5351::Si5351Device;

pub const EXTERNAL_XTAL_FREQ_HZ: HertzU32 = HertzU32::from_raw(12_000_000u32);

pub const SYS_PLL_CONFIG_100MHZ: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1200),
    refdiv: 1,
    post_div1: 6,
    post_div2: 3,
};

/// The divisor of how many CPU cycles should pass before a new word is sent to all neigboring nodes.
pub const CLOCKS_PER_SYNC_WORD: u32 = 4096;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut core = pac::CorePeripherals::take().unwrap();
    let sio = Sio::new(pac.SIO);

    let mut clocks = ClocksManager::new(pac.CLOCKS);

    let pins = gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Run of Ring oscillatior (should be the default, unless other software has made changes to clocking, so set on startup)
    let rosc = RingOscillator::new(pac.ROSC);
    let rosc = rosc.initialize();

    clocks
        .system_clock
        .configure_clock(&rosc, rosc.get_freq())
        .unwrap();

    info!("Set clock to rosc {:?}MHz", rosc.get_freq().to_MHz());

    info!("Configuring Si5351");

    pins.gpio25
        .into_push_pull_output_in_state(gpio::PinState::High);

    let i2c = I2C::i2c1(
        pac.I2C1,
        pins.gpio26.reconfigure(),
        pins.gpio27.reconfigure(),
        100.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    let mut si_clock = Si5351Device::new(i2c, false, 25_000_000);
    si_clock
        .init(si5351::CrystalLoad::_10)
        .expect("Cannot init clock.");

    si_clock
        .set_frequency(si5351::PLL::A, si5351::ClockOutput::Clk0, 13_000_000)
        .expect("Cannot set frequency");

    {
        let _gpout0_pin: Pin<Gpio21, FunctionClock, PullNone> = pins.gpio21.reconfigure();

        if let Err(e) = clocks
            .gpio_output0_clock
            .configure_clock(&clocks.system_clock, clocks.system_clock.get_freq())
        {
            warn!("Unable to route system clock to GPIO 21: {:?}", e);
        }
    };

    // Set up clock to run from gpio input from Si5351 clock
    let gpin0_pin = pins.gpio20.reconfigure();
    let gpin0: GpIn0 = GpIn0::new(gpin0_pin, EXTERNAL_XTAL_FREQ_HZ);

    clocks
        .system_clock
        .configure_clock(&gpin0, gpin0.get_freq())
        .unwrap();

    let (rx_pio, rx_sm0, rx_sm1, rx_sm2, rx_sm3) = pac.PIO0.split(&mut pac.RESETS);
    let (tx_pio, tx_sm0, tx_sm1, tx_sm2, tx_sm3) = pac.PIO1.split(&mut pac.RESETS);

    let rx0_data = pins.gpio3.into_function::<FunctionPio0>().into_dyn_pin();
    let rx0_clk = pins.gpio4.into_function::<FunctionPio0>().into_dyn_pin();
    let rx0_word = pins.gpio5.into_function::<FunctionPio0>().into_dyn_pin();

    let rx1_data = pins.gpio9.into_function::<FunctionPio0>().into_dyn_pin();
    let rx1_clk = pins.gpio10.into_function::<FunctionPio0>().into_dyn_pin();
    let rx1_word = pins.gpio11.into_function::<FunctionPio0>().into_dyn_pin();

    let rx2_data = pins.gpio15.into_function::<FunctionPio0>().into_dyn_pin();
    let rx2_clk = pins.gpio16.into_function::<FunctionPio0>().into_dyn_pin();
    let rx2_word = pins.gpio17.into_function::<FunctionPio0>().into_dyn_pin();

    let rx3_data = pins.gpio23.into_function::<FunctionPio0>().into_dyn_pin();
    let rx3_clk = pins.gpio24.into_function::<FunctionPio0>().into_dyn_pin();
    let rx3_word = pins.gpio28.into_function::<FunctionPio0>().into_dyn_pin();

    let tx0_data = pins.gpio0.into_function::<FunctionPio1>().into_dyn_pin();
    let tx0_clk = pins.gpio1.into_function::<FunctionPio1>().into_dyn_pin();
    let tx0_word = pins.gpio2.into_function::<FunctionPio1>().into_dyn_pin();

    let tx1_data = pins.gpio6.into_function::<FunctionPio1>().into_dyn_pin();
    let tx1_clk = pins.gpio7.into_function::<FunctionPio1>().into_dyn_pin();
    let tx1_word = pins.gpio8.into_function::<FunctionPio1>().into_dyn_pin();

    let tx2_data = pins.gpio12.into_function::<FunctionPio1>().into_dyn_pin();
    let tx2_clk = pins.gpio13.into_function::<FunctionPio1>().into_dyn_pin();
    let tx2_word = pins.gpio14.into_function::<FunctionPio1>().into_dyn_pin();

    let tx3_data = pins.gpio18.into_function::<FunctionPio1>().into_dyn_pin();
    let tx3_clk = pins.gpio19.into_function::<FunctionPio1>().into_dyn_pin();
    let tx3_word = pins.gpio22.into_function::<FunctionPio1>().into_dyn_pin();

    let mut pitopi = Pitopi::new(rx_pio, tx_pio);

    pitopi.install_programs();

    let (_, rx0, _, tx0) = pitopi
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

    let tide_controller = bittide_impls::boards::pico1_and_si5351::Control::new(
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
        bittide_impls::chips::rp2040::SioFifo(sio_fifo),
        tide_fifos,
    );

    critical_section::with(|cs| {
        GLOBAL_CONTROL.borrow(cs).replace(Some(tide_controller));
    });

    info!("Start.");
    bittide_impls::chips::rp2040::setup_interrupt(CLOCKS_PER_SYNC_WORD, &mut core.SYST);

    #[allow(clippy::empty_loop)]
    loop {
        for _ in 0..150000 {
            asm::nop();
        }
    }
}

static GLOBAL_CONTROL: Mutex<RefCell<Option<bittide_impls::boards::pico1_and_si5351::Control>>> =
    Mutex::new(RefCell::new(None));

#[exception]
fn SysTick() {
    static mut CONTROL: Option<bittide_impls::boards::pico1_and_si5351::Control> = None;

    if CONTROL.is_none() {
        critical_section::with(|cs| {
            let _ = CONTROL.insert(GLOBAL_CONTROL.borrow(cs).take().unwrap());
        });
    }

    if let Some(control) = CONTROL {
        control.interrupt();
    }
}
