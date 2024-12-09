#![no_std]
#![no_main]

#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use core::{cell::RefCell, ops::RangeInclusive, u32};

use crate::pac::interrupt;
use cortex_m::{asm, peripheral::syst::SystClkSource};
use cortex_m_rt::exception;
use critical_section::Mutex;
#[allow(unused_imports)]
use defmt::{error, info, warn};
use defmt_rtt as _;
use embedded_hal::digital::v2::OutputPin;
use fugit::HertzU32;
use heapless::Vec;
use panic_probe as _;
use pio_proc::pio_file;
use rp_pico::{
    entry,
    hal::{
        clocks::{Clock, ClockSource, ClocksManager},
        gpio::{
            self,
            bank0::{Gpio16, Gpio17, Gpio18, Gpio19, Gpio20},
            FunctionPio0, FunctionPio1, FunctionSio, FunctionSioInput, Interrupt, Pin, PullDown,
            PullNone, SioOutput,
        },
        pio::{PIOBuilder, PIOExt, PinDir, Rx, Tx, ValidStateMachine, SM0, SM1, SM2, SM3},
        pll::{setup_pll_blocking, PLLConfig},
        sio::Sio,
        xosc::setup_xosc_blocking,
    },
    pac::{self, CLOCKS, PIO0, PIO1, PPB},
};

pub const EXTERNAL_XTAL_FREQ_HZ: HertzU32 = HertzU32::from_raw(12_000_000u32);

pub const SYS_PLL_CONFIG_100MHZ: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1600),
    refdiv: 1,
    post_div1: 6,
    post_div2: 2,
};

/// The divisor of how many CPU cycles should pass before a new word is sent to all neigboring nodes.
pub const CLOCKS_PER_SYNC_WORD: u32 = 1024;

// TODO: instantiate 4 RX and 4 TXs
// TODO: test between two seperate pico's instead of loopback.
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

    clocks
        .system_clock
        .configure_clock(&pll_sys, pll_sys.get_freq())
        .unwrap();

    info!(
        "Configured system clock at frequency: {:?}MHz",
        pll_sys.get_freq().to_Hz() as f32 / 1e6
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

    let (mut tx_sm0, _rx0, mut tx0) = PIOBuilder::from_program(unsafe { tx_program.share() })
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

    let (mut tx_sm1, _rx1, mut tx1) = PIOBuilder::from_program(unsafe { tx_program.share() })
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

    let (mut tx_sm2, _rx2, mut tx2) = PIOBuilder::from_program(unsafe { tx_program.share() })
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

    let (mut tx_sm3, _rx3, mut tx3) = PIOBuilder::from_program(unsafe { tx_program.share() })
        .out_pins(tx3_data.id().num, 1)
        .side_set_pin_base(tx3_clk.id().num)
        .clock_divisor_fixed_point(4, 0)
        .build(tx_sm3);

    tx_sm3.set_pindirs([
        (tx3_data.id().num, PinDir::Output),
        (tx3_clk.id().num, PinDir::Output),
        (tx3_word.id().num, PinDir::Output),
    ]);

    tx_sm3.start();

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

    let (mut rx_sm0, mut rx0, _tx0) = PIOBuilder::from_program(unsafe { rx_program.share() })
        .in_pin_base(rx0_data.id().num)
        .clock_divisor_fixed_point(1, 0)
        .build(rx_sm0);

    rx_sm0.set_pindirs([
        (rx0_data.id().num, PinDir::Input),
        (rx0_clk.id().num, PinDir::Input),
        (rx0_word.id().num, PinDir::Input),
    ]);

    rx_sm0.start();

    let (mut rx_sm1, mut rx1, _tx1) = PIOBuilder::from_program(unsafe { rx_program.share() })
        .in_pin_base(rx1_data.id().num)
        .clock_divisor_fixed_point(1, 0)
        .build(rx_sm1);

    rx_sm1.set_pindirs([
        (rx1_data.id().num, PinDir::Input),
        (rx1_clk.id().num, PinDir::Input),
        (rx1_word.id().num, PinDir::Input),
    ]);

    rx_sm1.start();

    let (mut rx_sm2, mut rx2, _tx2) = PIOBuilder::from_program(unsafe { rx_program.share() })
        .in_pin_base(rx2_data.id().num)
        .clock_divisor_fixed_point(1, 0)
        .build(rx_sm2);

    rx_sm2.set_pindirs([
        (rx2_data.id().num, PinDir::Input),
        (rx2_clk.id().num, PinDir::Input),
        (rx2_word.id().num, PinDir::Input),
    ]);

    rx_sm2.start();

    let (mut rx_sm3, mut rx3, _tx3) = PIOBuilder::from_program(unsafe { rx_program.share() })
        .in_pin_base(rx3_data.id().num)
        .clock_divisor_fixed_point(1, 0)
        .build(rx_sm3);

    rx_sm3.set_pindirs([
        (rx3_data.id().num, PinDir::Input),
        (rx3_clk.id().num, PinDir::Input),
        (rx3_word.id().num, PinDir::Input),
    ]);

    rx_sm3.start();

    info!("Start.");

    let mut systick = core.SYST;
    systick.set_reload(CLOCKS_PER_SYNC_WORD - 1);
    systick.clear_current();
    systick.enable_counter();
    systick.set_clock_source(SystClkSource::Core);
    // systick.enable_interrupt();

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

    let mut i: u32 = 1337;

    loop {
        tx0.write(i);
        tx1.write(i);
        tx2.write(i);
        tx3.write(i);

        for _ in 0..4500 {
            asm::nop();
        }

        if let Some(data) = rx0.read() {
            info!("0x{:x}", data);
        }

        // if let Some(data) = rx1.read() {
        //     if i != data {
        //         warn!("0x{:x} == 0x{:x} {}", i, data, i == data);
        //     }
        // }

        i = i.overflowing_mul(1337).0;
    }
}

#[exception]
fn SysTick() {
    info!("systick expired;");
}

struct TideChannelControl<F, const N: usize> {
    frequency_controller: F,
    rxs: Rxs,
    txs: Txs,
}

struct Txs {
    tx0: Tx<(PIO1, SM0)>,
    tx1: Tx<(PIO1, SM1)>,
    tx2: Tx<(PIO1, SM2)>,
    tx3: Tx<(PIO1, SM3)>,
}

struct Rxs {
    rx0: Rx<(PIO0, SM0)>,
    rx1: Rx<(PIO0, SM1)>,
    rx2: Rx<(PIO0, SM2)>,
    rx3: Rx<(PIO0, SM3)>,
}
