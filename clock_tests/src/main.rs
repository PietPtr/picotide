#![no_std]
#![no_main]

#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use core::{cell::RefCell, u32};

use crate::pac::interrupt;
use critical_section::Mutex;
use defmt::{error, info, warn};
use defmt_rtt as _;
use embedded_hal::digital::v2::{InputPin, OutputPin, StatefulOutputPin, ToggleableOutputPin};
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
            bank0::{Gpio19, Gpio20, Gpio25},
            FunctionPio0, FunctionSioInput, FunctionSioOutput, Interrupt, Pin, PullDown, PullNone,
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

    // let pll_sys = setup_pll_blocking(
    //     pac.PLL_SYS,
    //     xosc.operating_frequency(),
    //     SYS_PLL_CONFIG_100MHZ,
    //     &mut clocks,
    //     &mut pac.RESETS,
    // )
    // .unwrap();

    // let fbdiv = pll_sys.device.fbdiv_int.read().fbdiv_int().bits();
    info!(
        "fbdiv_int={}",
        pac.PLL_SYS.fbdiv_int.read().fbdiv_int().bits()
    );
    pac.PLL_SYS
        .fbdiv_int
        .write(|w| unsafe { w.fbdiv_int().bits(101) });
    info!(
        "fbdiv_int={}",
        pac.PLL_SYS.fbdiv_int.read().fbdiv_int().bits()
    );

    // let start_freq = HertzU32::MHz(100);

    // clocks
    //     .system_clock
    //     .configure_clock(&pll_sys, pll_sys.get_freq())
    //     .unwrap();

    // info!(
    //     "Configured system clock at frequency: {:?}MHz",
    //     start_freq.to_Hz() as f32 / 1e6
    // );

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

    // clocks
    //     .system_clock
    //     .configure_clock(&pll_sys, start_freq + HertzU32::MHz(10))
    //     .unwrap();

    // info!(
    //     "Freq should now be: {}MHz",
    //     clocks.system_clock.get_freq().to_Hz() as f32 / 1e6
    // );
    let test_pin: Pin<gpio::bank0::Gpio19, gpio::FunctionSio<gpio::SioInput>, PullDown> =
        pins.gpio19.into_pull_down_input();

    let mut drive_pin = pins.gpio18.into_push_pull_output();
    drive_pin.set_high().unwrap();

    let mut i = 0;

    let mut fbdivs = [100, 105, 100, 95, 90, 80].iter().cycle();

    loop {
        for _ in 0..100000000 {
            pac.PLL_SYS.fbdiv_int.read().fbdiv_int().bits();
        }

        let &new_fbdiv = fbdivs.next().unwrap();
        info!("Set new feedback divider: {}", new_fbdiv);

        pac.PLL_SYS
            .fbdiv_int
            .write(|w| unsafe { w.fbdiv_int().bits(new_fbdiv) });
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

        info!("syst diff={}", 0xffffff - syst_value);

        clk_senser.clear_interrupt(Interrupt::EdgeHigh);
    }
}
