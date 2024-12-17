#![no_std]
#![no_main]

#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use core::{cell::RefCell, u32};

use controllers::pid::PidSettings;
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::exception;
use critical_section::Mutex;
#[allow(unused_imports)]
use defmt::{error, info, warn};
use defmt_rtt as _;
use fixed::types::I16F16;
use fugit::HertzU32;
use panic_probe as _;
use pio_proc::pio_file;
use rp_pico::pac;
use rp_pico::{
    entry,
    hal::{
        clocks::{Clock, ClockSource, ClocksManager},
        gpio::{self, FunctionPio0, FunctionPio1},
        pio::{PIOBuilder, PIOExt, PinDir},
        pll::{setup_pll_blocking, PLLConfig},
        sio::Sio,
        xosc::setup_xosc_blocking,
    },
};

use controllers::fbdiv::FbdivController;
use tide::tide::{Rxs, TideChannelControl, TideFifo, Txs};

pub const EXTERNAL_XTAL_FREQ_HZ: HertzU32 = HertzU32::from_raw(12_000_000u32);

pub const SYS_PLL_CONFIG_100MHZ: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1200),
    refdiv: 1,
    post_div1: 6,
    post_div2: 2,
};

/// The divisor of how many CPU cycles should pass before a new word is sent to all neigboring nodes.
pub const CLOCKS_PER_SYNC_WORD: u32 = 4096;

// TODO: "waste" 1 state machine on exposing the system clock for debugging?

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let sio = Sio::new(pac.SIO);

    let mut clocks = ClocksManager::new(pac.CLOCKS);

    let xosc = setup_xosc_blocking(pac.XOSC, EXTERNAL_XTAL_FREQ_HZ).unwrap();

    let locked_pll_sys = setup_pll_blocking(
        pac.PLL_SYS,
        xosc.operating_frequency(),
        SYS_PLL_CONFIG_100MHZ,
        &mut clocks,
        &mut pac.RESETS,
    )
    .unwrap();

    clocks
        .system_clock
        .configure_clock(&locked_pll_sys, locked_pll_sys.get_freq())
        .unwrap();

    let freq = locked_pll_sys.get_freq().to_Hz() as f32 / 1e6;
    let pll_sys = locked_pll_sys.free();

    info!(
        "Configured system clock at frequency: {:?}MHz (fbdiv={})",
        freq,
        pll_sys.fbdiv_int.read().fbdiv_int().bits()
    );

    let pins = gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let (mut tx_pio, tx_sm0, tx_sm1, tx_sm2, tx_sm3) = pac.PIO1.split(&mut pac.RESETS);
    let (mut rx_pio, rx_sm0, rx_sm1, rx_sm2, rx_sm3) = pac.PIO0.split(&mut pac.RESETS);

    // Set up 4 TX channels
    let tx0_data = pins.gpio0.into_function::<FunctionPio1>();
    let tx0_clk = pins.gpio1.into_function::<FunctionPio1>();
    let tx0_word = pins.gpio2.into_function::<FunctionPio1>();

    let tx1_data = pins.gpio6.into_function::<FunctionPio1>();
    let tx1_clk = pins.gpio7.into_function::<FunctionPio1>();
    let tx1_word = pins.gpio8.into_function::<FunctionPio1>();

    let tx2_data = pins.gpio12.into_function::<FunctionPio1>();
    let tx2_clk = pins.gpio13.into_function::<FunctionPio1>();
    let tx2_word = pins.gpio14.into_function::<FunctionPio1>();

    let tx3_data = pins.gpio18.into_function::<FunctionPio1>();
    let tx3_clk = pins.gpio19.into_function::<FunctionPio1>();
    let tx3_word = pins.gpio20.into_function::<FunctionPio1>();

    let pitopi_tx_program = pio_file!("src/programs.pio", select_program("pitopi_tx")).program;
    let tx_program = tx_pio.install(&pitopi_tx_program).unwrap();

    let (mut tx_sm0, _rx0, tx0) = PIOBuilder::from_program(unsafe { tx_program.share() })
        .out_pins(tx0_data.id().num, 1)
        .side_set_pin_base(tx0_clk.id().num)
        .clock_divisor_fixed_point(4, 0)
        .build(tx_sm0);

    tx_sm0.set_pindirs([
        (tx0_data.id().num, PinDir::Output),
        (tx0_clk.id().num, PinDir::Output),
        (tx0_word.id().num, PinDir::Output),
    ]);

    tx_sm0.start();

    let (mut tx_sm1, _rx1, tx1) = PIOBuilder::from_program(unsafe { tx_program.share() })
        .out_pins(tx1_data.id().num, 1)
        .side_set_pin_base(tx1_clk.id().num)
        .clock_divisor_fixed_point(4, 0)
        .build(tx_sm1);

    tx_sm1.set_pindirs([
        (tx1_data.id().num, PinDir::Output),
        (tx1_clk.id().num, PinDir::Output),
        (tx1_word.id().num, PinDir::Output),
    ]);

    tx_sm1.start();

    let (mut tx_sm2, _rx2, tx2) = PIOBuilder::from_program(unsafe { tx_program.share() })
        .out_pins(tx2_data.id().num, 1)
        .side_set_pin_base(tx2_clk.id().num)
        .clock_divisor_fixed_point(4, 0)
        .build(tx_sm2);

    tx_sm2.set_pindirs([
        (tx2_data.id().num, PinDir::Output),
        (tx2_clk.id().num, PinDir::Output),
        (tx2_word.id().num, PinDir::Output),
    ]);

    tx_sm2.start();

    // TODO: feature?
    // let (mut tx_sm3, _rx3, tx3) = PIOBuilder::from_program(unsafe { tx_program.share() })
    //     .out_pins(tx3_data.id().num, 1)
    //     .side_set_pin_base(tx3_clk.id().num)
    //     .clock_divisor_fixed_point(4, 0)
    //     .build(tx_sm3);

    // tx_sm3.set_pindirs([
    //     (tx3_data.id().num, PinDir::Output),
    //     (tx3_clk.id().num, PinDir::Output),
    //     (tx3_word.id().num, PinDir::Output),
    // ]);

    // tx_sm3.start();

    let program = pio_file!("src/programs.pio", select_program("toggle_pin")).program;
    let toggle_pin_program = tx_pio.install(&program).unwrap();

    // --- make compile time debugging feature ---

    let clk_pin = tx3_clk;

    info!("pin num {}", clk_pin.id().num);

    let (mut tx_sm3, _rx3, tx3) = PIOBuilder::from_program(toggle_pin_program)
        .set_pins(clk_pin.id().num, 1)
        .clock_divisor_fixed_point((CLOCKS_PER_SYNC_WORD / 2) as u16, 0) // TODO: unsafe cast
        .build(tx_sm3);

    tx_sm3.set_pindirs([(clk_pin.id().num, PinDir::Output)]);

    tx_sm3.start();

    // --- until here ---

    let rx0_data = pins.gpio3.into_function::<FunctionPio0>();
    let rx0_clk = pins.gpio4.into_function::<FunctionPio0>();
    let rx0_word = pins.gpio5.into_function::<FunctionPio0>();

    let rx1_data = pins.gpio9.into_function::<FunctionPio0>();
    let rx1_clk = pins.gpio10.into_function::<FunctionPio0>();
    let rx1_word = pins.gpio11.into_function::<FunctionPio0>();

    let rx2_data = pins.gpio15.into_function::<FunctionPio0>();
    let rx2_clk = pins.gpio16.into_function::<FunctionPio0>();
    let rx2_word = pins.gpio17.into_function::<FunctionPio0>();

    let rx3_data = pins.gpio21.into_function::<FunctionPio0>();
    let rx3_clk = pins.gpio22.into_function::<FunctionPio0>();
    let rx3_word = pins.gpio23.into_function::<FunctionPio0>();

    let pitopi_rx_program = pio_file!("src/programs.pio", select_program("pitopi_rx")).program;
    let rx_program = rx_pio.install(&pitopi_rx_program).unwrap();

    let (mut rx_sm0, rx0, _tx0) = PIOBuilder::from_program(unsafe { rx_program.share() })
        .in_pin_base(rx0_data.id().num)
        .clock_divisor_fixed_point(1, 0)
        .build(rx_sm0);

    rx_sm0.set_pindirs([
        (rx0_data.id().num, PinDir::Input),
        (rx0_clk.id().num, PinDir::Input),
        (rx0_word.id().num, PinDir::Input),
    ]);

    let (mut rx_sm1, rx1, _tx1) = PIOBuilder::from_program(unsafe { rx_program.share() })
        .in_pin_base(rx1_data.id().num)
        .clock_divisor_fixed_point(1, 0)
        .build(rx_sm1);

    rx_sm1.set_pindirs([
        (rx1_data.id().num, PinDir::Input),
        (rx1_clk.id().num, PinDir::Input),
        (rx1_word.id().num, PinDir::Input),
    ]);

    let (mut rx_sm2, rx2, _tx2) = PIOBuilder::from_program(unsafe { rx_program.share() })
        .in_pin_base(rx2_data.id().num)
        .clock_divisor_fixed_point(1, 0)
        .build(rx_sm2);

    rx_sm2.set_pindirs([
        (rx2_data.id().num, PinDir::Input),
        (rx2_clk.id().num, PinDir::Input),
        (rx2_word.id().num, PinDir::Input),
    ]);

    let (mut rx_sm3, rx3, _tx3) = PIOBuilder::from_program(unsafe { rx_program.share() })
        .in_pin_base(rx3_data.id().num)
        .clock_divisor_fixed_point(1, 0)
        .build(rx_sm3);

    rx_sm3.set_pindirs([
        (rx3_data.id().num, PinDir::Input),
        (rx3_clk.id().num, PinDir::Input),
        (rx3_word.id().num, PinDir::Input),
    ]);

    let sio_fifo = sio.fifo;

    let tide_fifos = [
        TideFifo::new(),
        TideFifo::new(),
        TideFifo::new(),
        TideFifo::new(),
    ];

    let tide_controller = Control::new(
        FbdivController::new(
            4,
            pll_sys,
            PidSettings {
                kp: I16F16::from_num(0.01),
                ki: I16F16::from_num(0.00000001),
                kd: I16F16::from_num(0.01),
            },
        ),
        Rxs::new(rx0, rx1, rx2, rx3),
        Txs::new(tx0, tx1, tx2, tx3),
        sio_fifo,
        tide_fifos,
    );

    critical_section::with(|cs| {
        GLOBAL_CONTROL.borrow(cs).replace(Some(tide_controller));
    });

    let mut systick = core.SYST;
    systick.set_reload(CLOCKS_PER_SYNC_WORD - 1);
    systick.clear_current();
    systick.enable_counter();
    systick.set_clock_source(SystClkSource::Core);

    info!(
        "\nclksource={} ({})\nenabled={}\ntickint={}\nrvr={:#x}\nsyst_calib: noref={} skew={} tenms={:x}",
        if pac.PPB.syst_csr.read().clksource().bit() {
            "processor"
        } else {
            "refclock"
        },
        pac.PPB.syst_csr.read().clksource().bit(),
        pac.PPB.syst_csr.read().enable().bit_is_set(),
        pac.PPB.syst_csr.read().tickint().bit(),
        pac.PPB.syst_rvr.read().bits(),
        pac.PPB.syst_calib.read().noref().bit(),
        pac.PPB.syst_calib.read().skew().bit(),
        pac.PPB.syst_calib.read().tenms().bits(),
    );

    info!("Start.");

    systick.enable_interrupt();
    rx_sm0.start();
    rx_sm1.start();
    rx_sm2.start();
    rx_sm3.start();

    #[allow(clippy::empty_loop)]
    loop {
        // info!("sm3 intsr {}", tx_sm3.instruction_address());
    }
}

type Control = TideChannelControl<FbdivController, 64>;

static GLOBAL_CONTROL: Mutex<RefCell<Option<Control>>> = Mutex::new(RefCell::new(None));

#[exception]
fn SysTick() {
    static mut CONTROL: Option<Control> = None;

    if CONTROL.is_none() {
        critical_section::with(|cs| {
            let _ = CONTROL.insert(GLOBAL_CONTROL.borrow(cs).take().unwrap());
        });
    }

    if let Some(control) = CONTROL {
        control.interrupt();
    }
}
