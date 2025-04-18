#![no_std]
#![no_main]

#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

#[allow(unused_imports)]
use defmt::{error, info, warn};
use defmt_rtt as _;
use embedded_graphics::mono_font::ascii::FONT_6X13;
use embedded_graphics::mono_font::{MonoTextStyle, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{Dimensions, DrawTarget, Point};
use embedded_graphics::primitives::{PrimitiveStyle, PrimitiveStyleBuilder, StyledDrawable};
use embedded_graphics::text::{Baseline, Text};
use embedded_graphics::Drawable;
use fugit::{HertzU32, RateExtU32};
use panic_probe as _;
use si5351::{Si5351, Si5351Device};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

use minsync::{
    entry,
    hal::{
        self,
        clocks::{ClockSource, ClocksManager},
        pac,
        pll::PLLConfig,
        rosc::RingOscillator,
        Clock, Watchdog, I2C,
    },
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

    let i2c_display = I2C::i2c0(
        pac.I2C0,
        pins.oled_sda.reconfigure(),
        pins.oled_scl.reconfigure(),
        350.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    let interface = I2CDisplayInterface::new(i2c_display);
    let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X13)
        .text_color(BinaryColor::On)
        .build();

    Text::with_baseline("Hello world!", Point::zero(), text_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();

    display.flush().unwrap();

    info!("Initialized display");

    // Show that the SI5351 also still works when the display is in use
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

    // Draw some texts
    let mut text1 = ScrollingText::new("minsync", Point::new(0, 0), text_style, 3);
    let mut text2 = ScrollingText::new("minsync", Point::new(50, 9), text_style, 1);
    let mut text3 = ScrollingText::new("minsync", Point::new(100, 18), text_style, 2);
    let rect_style = PrimitiveStyleBuilder::new()
        .fill_color(BinaryColor::Off)
        .build();

    loop {
        text1.clear(&mut display, &rect_style);
        text2.clear(&mut display, &rect_style);
        text3.clear(&mut display, &rect_style);

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

    fn clear<D: DrawTarget<Color = BinaryColor>>(
        &self,
        display: &mut D,
        style: &PrimitiveStyle<BinaryColor>,
    ) {
        self.text
            .bounding_box()
            .draw_styled(style, display)
            .ok()
            .unwrap();
    }

    fn draw<D: DrawTarget<Color = BinaryColor>>(&self, display: &mut D) {
        self.text.draw(display).ok().unwrap();
    }
}
