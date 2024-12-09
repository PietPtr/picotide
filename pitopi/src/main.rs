#![no_std]
#![no_main]

#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use core::{cell::RefCell, ops::RangeInclusive, u32};

use crate::pac::interrupt;
use cortex_m::asm;
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
            bank0::{Gpio18, Gpio19, Gpio20},
            FunctionPio0, FunctionSio, FunctionSioInput, Interrupt, Pin, PullDown, PullNone,
            SioOutput,
        },
        pio::{PIOBuilder, PIOExt, PinDir},
        pll::{setup_pll_blocking, PLLConfig},
        sio::Sio,
        xosc::setup_xosc_blocking,
    },
    pac::{self, PPB},
};

pub const EXTERNAL_XTAL_FREQ_HZ: HertzU32 = HertzU32::from_raw(12_000_000u32);

pub const SYS_PLL_CONFIG_100MHZ: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1600),
    refdiv: 1,
    post_div1: 6,
    post_div2: 2,
};

/// Measures frequency of the sysclk by dividing a clock down via PIO, exposing that to pin 10,
/// and measuring the time between interrupts on pin 20. Connect pin 10 to 20 with a jumper wire to make it work.
/// Can be improved by using the internal frequency counter in the clocking module of the rp2040.
/// Pin 11 exposes the system clock at half rate through a PIO program.
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

    let pitopi_tx_data = pins.gpio20.into_function::<FunctionPio0>();
    let pitopi_tx_clk = pins.gpio21.into_function::<FunctionPio0>();

    let (mut pio, sm0, sm1, _, _) = pac.PIO0.split(&mut pac.RESETS);

    // Set up TX
    let pitopi_tx_program = pio_file!("src/programs.pio", select_program("pitopi_tx")).program;
    let toggle_pin = pio.install(&pitopi_tx_program).unwrap();

    let (mut sm0, _rx0, mut tx0) = PIOBuilder::from_program(toggle_pin)
        .out_pins(pitopi_tx_data.id().num, 1)
        .side_set_pin_base(pitopi_tx_clk.id().num)
        .clock_divisor_fixed_point(1, 0)
        .build(sm0);

    sm0.set_pindirs([
        (pitopi_tx_data.id().num, PinDir::Output),
        (pitopi_tx_clk.id().num, PinDir::Output),
    ]);
    let sm0 = sm0.start();

    let pitopi_rx_data: Pin<Gpio18, FunctionPio0, PullDown> =
        pins.gpio18.into_function::<FunctionPio0>();
    let pitopi_rx_clk: Pin<Gpio19, FunctionPio0, PullDown> =
        pins.gpio19.into_function::<FunctionPio0>();

    let pitopi_rx_program = pio_file!("src/programs.pio", select_program("pitopi_rx")).program;
    let pitopi_rx_program = pio.install(&pitopi_rx_program).unwrap();

    info!(
        "rx wrap target {}, offset {}",
        pitopi_rx_program.wrap_target(),
        pitopi_rx_program.offset()
    );

    let (mut sm1, mut rx1, _tx1) = PIOBuilder::from_program(pitopi_rx_program)
        .in_pin_base(pitopi_rx_data.id().num)
        .clock_divisor_fixed_point(1, 0)
        .build(sm1);

    sm1.set_pindirs([
        (pitopi_rx_data.id().num, PinDir::Input),
        (pitopi_rx_clk.id().num, PinDir::Input),
    ]);

    info!("Start.");

    let sm1 = sm1.start();

    // tx0.write(0xaaaa_aaaa);
    // tx0.write(0x0000_0000);
    // tx0.write(0x0000_0000);
    // tx0.write(0xaaaa_aaaa);
    // tx0.write(0xffff_ffff);
    // tx0.write(0xffff_ffff);
    // tx0.write(0x0000_0000);

    // let mut read_data = Vec::<_, 128>::new();

    // for i in 0..128 {
    //     let data = rx1.read();
    //     if let Some(data) = data {
    //         read_data.push((i, data)).unwrap();
    //     }
    // }

    // for (i, data) in read_data {
    //     info!(
    //         "\n       ................................\ndata = {:b} 0x{:x} ({})",
    //         data, data, i
    //     );
    // }

    // let mut addresses = [0; 128];

    // for i in 0..addresses.len() {
    //     addresses[i] = sm1.instruction_address();
    // }

    // info!("{}", addresses);

    info!("{:?}", rx1.read());

    let mut i: u32 = 1337;

    loop {
        // tx0.write(0xa000_0000);
        tx0.write(i);
        for _ in 0..1000000 {
            asm::nop();
        }
        if let Some(data) = rx1.read() {
            // let data = data.reverse_bits();
            info!("0x{:x} == 0x{:x} {}", i, data, i == data);
        }
        for _ in 0..1000000 {
            asm::nop();
        }

        i = i.overflowing_mul(1337).0;
    }
}
