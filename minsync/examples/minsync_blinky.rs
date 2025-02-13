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
use embedded_graphics::mono_font::ascii::FONT_4X6;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::Point;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use embedded_hal::blocking::i2c::{Write, WriteRead};
use embedded_hal::digital::v2::{OutputPin, ToggleableOutputPin};
use embedded_hal::Pwm;
use embedded_hal::PwmPin;
use fugit::{HertzU32, RateExtU32};
use panic_probe as _;
use rp2040_hal::gpio::{bank0::Gpio25, FunctionClock, Pin, PullNone};
use rp2040_hal::Watchdog;
use sh1106::{prelude::*, Builder};
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

    let watchdog = Watchdog::new(pac.WATCHDOG);
    watchdog.disable();

    pac.CLOCKS.clk_gpout3_div().write(|w| {
        w.frac().variant(0);
        w.int().variant(1000)
    });

    pac.CLOCKS.clk_gpout3_ctrl().write(|w| {
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

    // let clocks = clocks.free();
    // let sysdiv_int = clocks.clk_sys_div().read().int().bits();
    // let sysdiv_frac = clocks.clk_sys_div().read().frac().bits();
    // clocks.clk_sys_div().write(|w| w.int().variant(2));

    let pins = minsync::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let _: Pin<Gpio25, FunctionClock, PullNone> = pins.gpout3.reconfigure();
    // let mut gpout_test = pins.gpout3.into_push_pull_output();

    // let si_frequency = 12.MHz();

    info!("Hoi!");
    // info!("sysdiv {} {}", sysdiv_int, sysdiv_frac);

    let mut i2c = I2C::i2c1(
        pac.I2C1,
        pins.si_sda.reconfigure(),
        pins.si_scl.reconfigure(),
        100.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    let mut buffer: [u8; 1] = [0];
    let si5351_i2c_address: u8 = 0x60;
    let register_address = 0;
    // let result = i2c.write_read(si5351_i2c_address, &[register_address], &mut buffer);
    // let result = i2c.write(si5351_i2c_address, &[register_address]);
    // info!("result {:?} {:?}", result, buffer);

    // TODO: move to its own display example
    // info!("initting display");

    // let i2c_display = I2C::i2c0(
    //     pac.I2C0,
    //     pins.oled_sda.reconfigure(),
    //     pins.oled_scl.reconfigure(),
    //     100.kHz(),
    //     &mut pac.RESETS,
    //     &clocks.system_clock,
    // );

    // let mut display: GraphicsMode<_> = Builder::new().connect_i2c(i2c_display).into();
    // display.init().unwrap();
    // let style = MonoTextStyle::new(&FONT_4X6, BinaryColor::On);
    // Text::new("Hello!", Point::new(0, 10), style)
    //     .draw(&mut display)
    //     .unwrap();
    // display.flush().unwrap();

    let mut pwm_slices = hal::pwm::Slices::new(pac.PWM, &mut pac.RESETS);
    let pwm = &mut pwm_slices.pwm2;
    pwm.set_ph_correct();
    pwm.enable();
    let channel = &mut pwm.channel_a;
    channel.set_duty(5000);
    channel.output_to(pins.led_or_si_clk1);

    let mut i = 0;

    let mut si_clock = Si5351Device::new(i2c, false, minsync::SI5351_CRYSTAL_FREQ);

    let status = si_clock.read_device_status().unwrap().bits();

    info!("Created SI device. {:?}", status);

    let test = si_clock.init(si5351::CrystalLoad::_8);

    info!("tried init SI: {:?}", test.is_err());

    // TODO: fork library and add clearer error messages
    match test {
        Ok(()) => {}
        Err(si5351::Error::CommunicationError) => info!("SI comm error"),
        Err(si5351::Error::InvalidParameter) => info!("SI invalid param"),
    }

    // si_clock
    //     .set_frequency(
    //         si5351::PLL::A,
    //         si5351::ClockOutput::Clk0,
    //         si_frequency.to_Hz(),
    //     )
    //     .expect("Cannot set frequency");

    // info!("Configured SI.");

    // Disable the XOSC circuit since we're passing in a stable CMOS clock from the Si5351
    // pac.XOSC.ctrl().write(|w| w.enable().disable());

    // let locked_pll_sys = setup_pll_blocking(
    //     pac.PLL_SYS,
    //     si_frequency,
    //     SYS_PLL_CONFIG_100MHZ,
    //     &mut clocks,
    //     &mut pac.RESETS,
    // )
    // .expect("Couldn't lock PLL");

    // clocks
    //     .gpio_output3_clock
    //     .configure_clock(&locked_pll_sys, locked_pll_sys.get_freq())
    //     .expect("Couldn't setup gpout3");

    // info!("GPOUT 3 is setup");

    // loop {}

    // clocks
    //     .system_clock
    //     .configure_clock(&locked_pll_sys, si_frequency)
    //     .expect("Couldn't set system clock to PLL.");

    // let mut led_pin = pins.led_or_si_clk1.into_push_pull_output();

    // let mut flashes = 0;

    // loop {
    //     // led_pin.set_high().unwrap();
    //     // led_pin.set_low().unwrap();

    //     // flashes += 1;

    //     // if flashes == 5 {
    //     //     si_clock
    //     //         .set_frequency(
    //     //             si5351::PLL::A,
    //     //             si5351::ClockOutput::Clk0,
    //     //             si_frequency.to_Hz() / 2,
    //     //         )
    //     //         .expect("Couldn't set frequency");
    //     // }

    //     // if flashes == 10 {
    //     //     si_clock
    //     //         .set_frequency(
    //     //             si5351::PLL::A,
    //     //             si5351::ClockOutput::Clk0,
    //     //             si_frequency.to_Hz(),
    //     //         )
    //     //         .expect("Couldn't set frequency");

    //     //     flashes = 0;
    //     // }
    // }

    loop {}
}
