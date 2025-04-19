#![no_std]
#![no_main]

#[allow(unused_imports)]
use defmt::{error, info, warn};
use defmt_rtt as _;
use embedded_graphics::{
    mono_font::MonoTextStyle,
    pixelcolor::BinaryColor,
    prelude::{Dimensions, DrawTarget, Point},
    text::{Baseline, Text},
    Drawable,
};
use fugit::{HertzU32, RateExtU32};
use panic_probe as _;
use si5351::Si5351;

use minsync::{
    display::DEFAULT_TEXT_STYLE,
    entry,
    hal::{self, pac, pll::PLLConfig, Watchdog},
};

pub const SYS_PLL_CONFIG_100MHZ: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1000),
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

    let pins = minsync::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let clocks = minsync::clocks::minimal_clock_setup(pac.CLOCKS, pac.ROSC, pins.gpout3)
        .expect("Failed to do basic clock set up.");

    let mut display = minsync::display::setup(minsync::display_i2c!(pac, pins, clocks, 300.kHz()))
        .expect("Couldn't set up display.");

    // Show that the SI5351 also still works when the display is in use
    let mut si_clock =
        minsync::clocks::setup_si_as_crystal(minsync::si_i2c!(pac, pins, clocks, 1.kHz()))
            .expect("Failed to setup Si5351");

    si_clock
        .set_frequency(si5351::PLL::A, si5351::ClockOutput::Clk2, 10_000_000)
        .expect("Cannot set frequency");

    // Draw some texts
    let mut text1 = ScrollingText::new("minsync", Point::new(0, 0), DEFAULT_TEXT_STYLE, 3);
    let mut text2 = ScrollingText::new("minsync", Point::new(0, 9), DEFAULT_TEXT_STYLE, 1);
    let mut text3 = ScrollingText::new("minsync", Point::new(0, 18), DEFAULT_TEXT_STYLE, 2);

    loop {
        display.clear(BinaryColor::Off).unwrap();

        text1.update();
        text2.update();
        text3.update();

        text1.draw(&mut display);
        text2.draw(&mut display);
        text3.draw(&mut display);

        display.flush().unwrap();
    }
}

struct ScrollingText<'a> {
    text: Text<'a, MonoTextStyle<'a, BinaryColor>>,
    speed: i32,
}

impl<'a> ScrollingText<'a> {
    fn new(
        content: &'a str,
        position: Point,
        style: MonoTextStyle<'a, BinaryColor>,
        speed: i32,
    ) -> Self {
        Self {
            text: Text::with_baseline(content, position, style, Baseline::Top),
            speed,
        }
    }

    fn update(&mut self) {
        self.text.position.x += self.speed;
        if self.text.position.x > 128 {
            self.text.position.x = -(self.text.bounding_box().size.width as i32);
        }
    }

    fn draw<D: DrawTarget<Color = BinaryColor>>(&self, display: &mut D) {
        self.text.draw(display).ok().unwrap();
    }
}
