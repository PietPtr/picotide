#![no_std]
#![no_main]

#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use core::{cell::RefCell, u32};

use crate::pac::interrupt;
use critical_section::Mutex;
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
        gpio::{
            self,
            bank0::{Gpio20, Gpio25},
            FunctionPio0, FunctionSioOutput, Interrupt, Pin, PullNone,
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
    vco_freq: HertzU32::MHz(1200),
    refdiv: 1,
    post_div1: 6,
    post_div2: 2,
};

const SYST_RVR: u32 = 0xffffff;

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

    let toggle_pin_program =
        pio_file!("src/programs.pio", select_program("toggle_pin_slow")).program;
    let toggle_pin = pio.install(&toggle_pin_program).unwrap();

    let (mut sm0, _rx0, _tx0) = PIOBuilder::from_program(toggle_pin)
        .set_pins(exposed_clock_pin.id().num, 1)
        .clock_divisor_fixed_point(0, 0)
        .build(sm0);

    sm0.set_pindirs([(exposed_clock_pin.id().num, PinDir::Output)]);
    sm0.start();

    info!("Start.");

    // pac.PPB.syst_csr.write(|w| w.clksource().set_bit());
    // pac.PPB.syst_csr.write(|w| w.enable().set_bit());
    // TODO: Make sure it is NOT set to sysclk
    pac.PPB.syst_csr.write(|w| unsafe { w.bits(0x5) });
    pac.PPB.syst_rvr.write(|w| unsafe { w.bits(SYST_RVR) });

    info!(
        "\nclksource={} ({})\nenabled={}\ntickint={}\nrvr={:#x}",
        if pac.PPB.syst_csr.read().clksource().bit() {
            "processor"
        } else {
            "refclock"
        },
        pac.PPB.syst_csr.read().clksource().bit(),
        pac.PPB.syst_csr.read().enable().bit_is_set(),
        pac.PPB.syst_csr.read().tickint().bit(),
        pac.PPB.syst_rvr.read().bits(),
    );

    let senser_pin = pins.gpio20.reconfigure();
    senser_pin.set_interrupt_enabled(Interrupt::EdgeHigh, true);

    critical_section::with(|cs| {
        GLOBAL_PINS.borrow(cs).replace(Some(senser_pin));
        GLOBAL_PPB.borrow(cs).replace(Some(pac.PPB));
    });

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

type SenserPin = Pin<Gpio20, FunctionSioOutput, PullNone>;

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

    info!("IO_IRQ_BANK0 fired.");

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

        info!("syst_value={}", syst_value);

        clk_senser.clear_interrupt(Interrupt::EdgeHigh);
    }
}
