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
use embedded_hal::blocking::i2c::{Write, WriteRead};
use embedded_hal::digital::v2::OutputPin;
use fugit::{HertzU32, RateExtU32};
use panic_probe as _;
use rp2040_hal::gpio::{bank0::Gpio25, FunctionClock, Pin, PullNone};
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

    pac.CLOCKS.clk_gpout3_div().write(|w| {
        w.frac().variant(0);
        w.int().variant(1000)
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

    let si_frequency = 12.MHz();

    info!("Hoi!");

    // TODO: test patched minsync board
    // TODO: test LED on PWM hardware / investigate other resistor
    // TODO: test power rings

    let mut i2c = I2C::i2c1(
        pac.I2C1,
        pins.si_sda.reconfigure(),
        pins.si_scl.reconfigure(),
        10.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    let mut buffer: [u8; 1] = [0];
    let si5351_i2c_address: u8 = 0x60;
    let register_address = 0;
    // // let result = i2c.write_read(si5351_i2c_address, &[register_address], &mut buffer);
    let result = i2c.write(si5351_i2c_address, &[register_address]);

    info!("result {:?}", result);

    loop {}

    // let mut si_clock = Si5351Device::new(i2c, false, minsync::SI5351_CRYSTAL_FREQ);

    // info!("Created SI device.");

    // let test = si_clock.init(si5351::CrystalLoad::_8);

    // info!("tried init SI: {:?}", test.is_err());

    // match test {
    //     Ok(()) => {}
    //     Err(si5351::Error::CommunicationError) => info!("SI comm error"),
    //     Err(si5351::Error::InvalidParameter) => info!("SI invalid param"),
    // }

    // si_clock
    //     .set_frequency(
    //         si5351::PLL::A,
    //         si5351::ClockOutput::Clk0,
    //         si_frequency.to_Hz(),
    //     )
    //     .expect("Cannot set frequency");

    // info!("Configured SI.");

    let gpout3: Pin<Gpio25, FunctionClock, PullNone> = pins.gpout3.reconfigure();

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

    clocks
        .gpio_output3_clock
        .configure_clock(&locked_pll_sys, locked_pll_sys.get_freq())
        .expect("Couldn't setup gpout3");

    info!("GPOUT 3 is setup");

    loop {}

    clocks
        .system_clock
        .configure_clock(&locked_pll_sys, si_frequency)
        .expect("Couldn't set system clock to PLL.");

    let mut led_pin = pins.led_or_si_clk1.into_push_pull_output();

    let mut flashes = 0;

    loop {
        // led_pin.set_high().unwrap();
        // led_pin.set_low().unwrap();

        // flashes += 1;

        // if flashes == 5 {
        //     si_clock
        //         .set_frequency(
        //             si5351::PLL::A,
        //             si5351::ClockOutput::Clk0,
        //             si_frequency.to_Hz() / 2,
        //         )
        //         .expect("Couldn't set frequency");
        // }

        // if flashes == 10 {
        //     si_clock
        //         .set_frequency(
        //             si5351::PLL::A,
        //             si5351::ClockOutput::Clk0,
        //             si_frequency.to_Hz(),
        //         )
        //         .expect("Couldn't set frequency");

        //     flashes = 0;
        // }
    }
}
