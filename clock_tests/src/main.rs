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
use panic_probe as _;
use pio_proc::pio_file;
use rp_pico::{
    entry,
    hal::{
        clocks::{Clock, ClockSource, ClocksManager},
        gpio::{
            self,
            bank0::{Gpio19, Gpio20},
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

const SYST_RVR: u32 = 0xffffff;

fn interpolate_frequency(value: u32) -> f32 {
    let freq_100mhz = 1e8;
    let value_100mhz = 3697.5; // Emperical

    let constant = freq_100mhz * value_100mhz;

    constant / value as f32
}

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

    let pll = pll_sys.free();

    info!(
        "current feedback divider: {}",
        pll.fbdiv_int.read().fbdiv_int().bits()
    );

    let pins = gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let exposed_slow_clock_pin = pins.gpio10.into_function::<FunctionPio0>();
    let exposed_fast_clock_pin = pins.gpio11.into_function::<FunctionPio0>();

    let (mut pio, sm0, sm1, _, _) = pac.PIO0.split(&mut pac.RESETS);

    // Set up slow sys clk exposing pin
    let toggle_pin_program =
        pio_file!("src/programs.pio", select_program("toggle_pin_slow")).program;
    let toggle_pin = pio.install(&toggle_pin_program).unwrap();

    let (mut sm0, _rx0, _tx0) = PIOBuilder::from_program(toggle_pin)
        .set_pins(exposed_slow_clock_pin.id().num, 1)
        .clock_divisor_fixed_point(0, 0)
        .build(sm0);

    sm0.set_pindirs([(exposed_slow_clock_pin.id().num, PinDir::Output)]);
    sm0.start();

    // Set up fast sys clk exposing pin
    let toggle_pin_program = pio_file!("src/programs.pio", select_program("toggle_pin")).program;
    let toggle_pin = pio.install(&toggle_pin_program).unwrap();

    let (mut sm1, _rx0, _tx0) = PIOBuilder::from_program(toggle_pin)
        .set_pins(exposed_fast_clock_pin.id().num, 1)
        .clock_divisor_fixed_point(5, 0)
        .build(sm1);

    sm1.set_pindirs([(exposed_fast_clock_pin.id().num, PinDir::Output)]);
    sm1.start();

    info!("Start.");

    pac.PPB.syst_csr.write(|w| unsafe { w.bits(0b001) });
    pac.PPB.syst_rvr.write(|w| unsafe { w.bits(SYST_RVR) });

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

    pac.PPB
        .syst_cvr
        .write(|w| unsafe { w.current().bits(0xfff) });

    let senser_pin: SenserPin = pins.gpio20.into_pull_down_input();
    senser_pin.set_interrupt_enabled(Interrupt::EdgeHigh, true);

    critical_section::with(|cs| {
        GLOBAL_PINS.borrow(cs).replace(Some(senser_pin));
        GLOBAL_PPB.borrow(cs).replace(Some(pac.PPB));
    });

    unsafe {
        pac::NVIC::unmask(pac::Interrupt::IO_IRQ_BANK0);
    }

    // Pin to attach a probe to to trigger on fbdiv changes
    let mut test_pin: Pin<Gpio19, FunctionSio<SioOutput>, PullNone> = pins.gpio19.reconfigure();
    test_pin.set_low().unwrap();

    let mut drive_pin = pins.gpio18.into_push_pull_output();
    drive_pin.set_high().unwrap();

    const MIN_FBDIV: u16 = 50;
    const MAX_FBDIV: u16 = 150;
    const FBDIV_RANGE: RangeInclusive<u16> = MIN_FBDIV..=MAX_FBDIV;
    let mut fbdivs = FBDIV_RANGE.chain(FBDIV_RANGE.rev()).cycle();

    loop {
        for _ in 0..1000000 {
            asm::nop();
        }

        let new_fbdiv = fbdivs.next().unwrap();
        info!("Set new feedback divider: {}", new_fbdiv);

        pll.fbdiv_int
            .write(|w| unsafe { w.fbdiv_int().bits(new_fbdiv) });

        test_pin.set_high().unwrap();
        for _ in 0..50 {
            asm::nop();
        }
        test_pin.set_low().unwrap();
    }
}

type SenserPin = Pin<Gpio20, FunctionSioInput, PullDown>;

static GLOBAL_PINS: Mutex<RefCell<Option<SenserPin>>> = Mutex::new(RefCell::new(None));
static GLOBAL_PPB: Mutex<RefCell<Option<PPB>>> = Mutex::new(RefCell::new(None));

#[interrupt]
#[allow(non_snake_case)]
fn IO_IRQ_BANK0() {
    static mut CLK_SENSER: Option<SenserPin> = None;
    static mut PPB: Option<PPB> = None;

    if CLK_SENSER.is_none() {
        critical_section::with(|cs| {
            let _ = CLK_SENSER.insert(GLOBAL_PINS.borrow(cs).take().unwrap());
        });
    }

    if PPB.is_none() {
        critical_section::with(|cs| {
            let _ = PPB.insert(GLOBAL_PPB.borrow(cs).take().unwrap());
        })
    }

    let Some(clk_senser) = CLK_SENSER.as_mut() else {
        info!("No clk senser pin available");
        return;
    };
    let Some(ppb) = PPB.as_mut() else {
        info!("PPB unavailable in interrupt");
        return;
    };

    if clk_senser.interrupt_status(Interrupt::EdgeHigh) {
        // Diference between rvr and read value is the time between clocks
        let syst_value = ppb.syst_cvr.read().current().bits();

        // reset syst_cvr counter to immediately start counting the next clock
        ppb.syst_cvr
            .write(|w| unsafe { w.current().bits(0xffffff) });

        let ticks_taken = 0xffffff - syst_value;
        info!(
            "syst diff={} (~{}MHz)",
            ticks_taken,
            interpolate_frequency(ticks_taken) as f32 / 1e6
        );

        clk_senser.clear_interrupt(Interrupt::EdgeHigh);
    }
}
