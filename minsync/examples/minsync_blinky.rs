#![no_std]
#![no_main]

use cortex_m::asm;
use embedded_hal::digital::v2::OutputPin;
use fugit::{HertzU32, RateExtU32};
use panic_probe as _;
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
    vco_freq: HertzU32::MHz(1200),
    refdiv: 1,
    post_div1: 5,
    post_div2: 2,
};

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let _core = pac::CorePeripherals::take().unwrap();
    let sio = hal::Sio::new(pac.SIO);

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

    let si_frequency = 12.MHz();

    let i2c = I2C::i2c1(
        pac.I2C1,
        pins.si_sda.reconfigure(),
        pins.si_scl.reconfigure(),
        100.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    let mut si_clock = Si5351Device::new(i2c, false, minsync::SI5351_CRYSTAL_FREQ);
    si_clock
        .init(si5351::CrystalLoad::_8)
        .expect("Cannot init clock.");

    si_clock
        .set_frequency(
            si5351::PLL::A,
            si5351::ClockOutput::Clk0,
            si_frequency.to_Hz(),
        )
        .expect("Cannot set frequency");

    // Disable the XOSC circuit since we're passing in a stable CMOS clock from the Si5351
    pac.XOSC.ctrl().write(|w| w.enable().disable());

    let locked_pll_sys = setup_pll_blocking(
        pac.PLL_SYS,
        si_frequency,
        SYS_PLL_CONFIG_100MHZ,
        &mut clocks,
        &mut pac.RESETS,
    )
    .unwrap();

    clocks
        .system_clock
        .configure_clock(&locked_pll_sys, si_frequency)
        .expect("Couldn't set system clock to PLL.");

    let mut led_pin = pins.led_or_si_clk1.into_push_pull_output();

    let mut flashes = 0;

    loop {
        led_pin.set_high().unwrap();
        for _ in 0..10000000 {
            asm::nop();
        }
        led_pin.set_low().unwrap();
        for _ in 0..10000000 {
            asm::nop();
        }

        flashes += 1;

        if flashes == 5 {
            si_clock
                .set_frequency(
                    si5351::PLL::A,
                    si5351::ClockOutput::Clk0,
                    si_frequency.to_Hz() / 2,
                )
                .expect("Couldn't set frequency");
        }

        if flashes == 10 {
            si_clock
                .set_frequency(
                    si5351::PLL::A,
                    si5351::ClockOutput::Clk0,
                    si_frequency.to_Hz(),
                )
                .expect("Couldn't set frequency");

            flashes = 0;
        }
    }
}
