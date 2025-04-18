#![no_std]
#![no_main]

#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use cortex_m::asm;
#[allow(unused_imports)]
use defmt::{error, info, warn};
use defmt_rtt as _;
use embedded_hal::digital::v2::OutputPin;
use fugit::{HertzU32, RateExtU32};
use panic_probe as _;
use rp2040_hal::gpio::{bank0::Gpio25, FunctionClock, Pin, PullNone};
use rp2040_hal::Watchdog;
use si5351::{Si5351, Si5351Device};

use minsync::{
    entry,
    hal::{
        self,
        clocks::{ClockSource, ClocksManager},
        pac,
        pll::{setup_pll_blocking, PLLConfig},
        rosc::RingOscillator,
        Clock, I2C,
    },
};

pub const SYS_PLL_CONFIG_100MHZ: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1000),
    refdiv: 1,
    post_div1: 6,
    post_div2: 6,
};

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let _core = pac::CorePeripherals::take().unwrap();
    let sio = hal::Sio::new(pac.SIO);

    let watchdog = Watchdog::new(pac.WATCHDOG);
    watchdog.disable();

    pac.CLOCKS.clk_gpout3_div().write(|w| {
        w.frac().variant(0);
        w.int().variant(1000)
    });

    pac.CLOCKS.clk_gpout3_ctrl().write(|w| {
        // w.auxsrc().clksrc_pll_sys();
        w.auxsrc().clk_sys();
        w.enable().set_bit()
    });

    let mut clocks = ClocksManager::new(pac.CLOCKS);
    let rosc = RingOscillator::new(pac.ROSC);
    let rosc = rosc.initialize();

    clocks
        .system_clock
        .configure_clock(&rosc, rosc.get_freq())
        .unwrap();

    let pins = minsync::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let _: Pin<Gpio25, FunctionClock, PullNone> = pins.gpout3.reconfigure();

    let si_frequency: fugit::Rate<u32, 1, 1> = 12.MHz();

    let i2c = I2C::i2c1(
        pac.I2C1,
        pins.si_sda.reconfigure(),
        pins.si_scl.reconfigure(),
        100.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    let mut si_clock = Si5351Device::new(i2c, false, minsync::SI5351_CRYSTAL_FREQ);

    let status = si_clock.read_device_status().unwrap().bits();

    info!("Created SI device. {:?}", status);

    si_clock
        .init(si5351::CrystalLoad::_8)
        .expect("Failed to init SI5351");

    si_clock
        .set_frequency(
            si5351::PLL::A,
            si5351::ClockOutput::Clk2,
            si_frequency.to_Hz(),
        )
        .expect("Cannot set frequency");

    si_clock
        .set_frequency(
            si5351::PLL::A,
            si5351::ClockOutput::Clk0,
            si_frequency.to_Hz(),
        )
        .expect("Cannot set frequency");

    info!("Configured SI.");

    // Disable the XOSC circuit since we're passing in a stable CMOS clock from the Si5351
    pac.XOSC.ctrl().write(|w| w.enable().disable());

    let locked_pll_sys = setup_pll_blocking(
        pac.PLL_SYS,
        si_frequency,
        SYS_PLL_CONFIG_100MHZ,
        &mut clocks,
        &mut pac.RESETS,
    )
    .expect("Couldn't lock PLL");

    info!(
        "PLL should be locked to the SI now ({}MHz)",
        locked_pll_sys.get_freq().to_MHz()
    );

    clocks
        .system_clock
        .configure_clock(&locked_pll_sys, locked_pll_sys.get_freq())
        .expect("Couldn't set system clock to PLL.");

    info!("System clock now on SI clock.");

    let mut led_pin = pins.led_or_si_clk1.into_push_pull_output();

    let mut flashes = 0;

    loop {
        led_pin.set_high().unwrap();

        for _ in 0..30_000 {
            asm::nop();
        }

        led_pin.set_low().unwrap();

        flashes += 1;

        if flashes == 100 {
            si_clock
                .setup_pll(si5351::PLL::A, 35, 0, 0xfffff)
                .expect("Cannot setup PLL");
        }

        if flashes == 200 {
            si_clock
                .setup_pll(si5351::PLL::A, 35, 0x7ffff, 0xfffff)
                .expect("Cannot setup PLL");

            flashes = 0;
        }

        // for _ in 0..1_000_000 {
        //     asm::nop();
        // }
    }
}
