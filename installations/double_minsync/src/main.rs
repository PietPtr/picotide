#![no_std]
#![no_main]

// #[link_section = ".boot2"]
// #[no_mangle]
// #[used]
// pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use cortex_m::asm;
#[allow(unused_imports)]
use defmt::{error, info, warn};
use defmt_rtt as _;
use embedded_graphics::mono_font::ascii::FONT_6X9;
use embedded_graphics::prelude::DrawTarget;
use embedded_graphics::primitives::{PrimitiveStyle, StyledDrawable};
use embedded_graphics::{
    mono_font::MonoTextStyleBuilder,
    pixelcolor::BinaryColor,
    prelude::{Dimensions, Point, Size},
    primitives::Rectangle,
    text::{Baseline, Text},
    Drawable,
};
use embedded_hal::digital::v2::OutputPin;
use fugit::{HertzU32, RateExtU32};
use heapless::String;
use minsync::si_i2c;
use panic_probe as _;
use si5351::Si5351;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

use minsync::hal;
use minsync::hal::pac;
use minsync::{
    entry,
    hal::{pll::PLLConfig, Watchdog, I2C},
};

mod generated_constants;

pub const EXTERNAL_XTAL_FREQ_HZ: HertzU32 = HertzU32::from_raw(12_000_000u32);

pub const SYS_PLL_CONFIG_100MHZ: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1000),
    refdiv: 1,
    post_div1: 5,
    post_div2: 1,
};

/// The divisor of how many CPU cycles should pass before a new word is sent to all neigboring nodes.
pub const CLOCKS_PER_SYNC_WORD: u32 = 4096;

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

    let mut clocks = minsync::clocks::minimal_clock_setup(pac.CLOCKS, pac.ROSC, pins.gpout3)
        .expect("Failed to do minimal clock setup.");

    let i2c_display = I2C::i2c0(
        pac.I2C0,
        pins.oled_sda.reconfigure(),
        pins.oled_scl.reconfigure(),
        100.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    let interface = I2CDisplayInterface::new(i2c_display);
    let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    // TODO: clear display function
    Rectangle::new(Point::zero(), Size::new(128, 32))
        .draw_styled(&PrimitiveStyle::with_fill(BinaryColor::Off), &mut display)
        .unwrap();

    draw_key_value(&mut display, 0, "Name", generated_constants::NAME).unwrap();

    // let mut buffer = itoa::Buffer::new();
    // let node_id_str = buffer.format(generated_constants::NODE_ID);
    // draw_key_value(&mut display, 1, "Node ID", node_id_str).unwrap();

    let mut buffer = itoa::Buffer::new();
    let si_frac_str = buffer.format(generated_constants::SI_FRAC);
    draw_key_value(&mut display, 1, "SI Frac", si_frac_str).unwrap();

    display.flush().unwrap();

    info!("Initialized display");

    let mut led = pins.led_or_si_clk1.into_push_pull_output();

    let mut si_clock = minsync::clocks::setup_si_as_crystal(si_i2c!(pac, pins, clocks, 1.kHz()))
        .expect("Failed to setup Si5351");
    minsync::clocks::setup_pll_and_sysclk(&mut clocks, pac.PLL_SYS, &mut pac.XOSC, &mut pac.RESETS);

    loop {
        for frac in (-generated_constants::SI_FRAC..generated_constants::SI_FRAC)
            .chain((-generated_constants::SI_FRAC..generated_constants::SI_FRAC).rev())
        {
            for _ in 0..5_000_000 {
                asm::nop();
            }

            led.set_high().unwrap();

            for _ in 0..1000 {
                asm::nop();
            }

            led.set_low().unwrap();

            let frac = (0x3ffff + frac) as u32;

            si_clock
                .setup_pll(si5351::PLL::A, 35, frac, 0xfffff)
                .expect("Cannot setup PLL");

            Rectangle::new(Point::new(0, 18), Size::new(128, 32))
                .draw_styled(&PrimitiveStyle::with_fill(BinaryColor::Off), &mut display)
                .unwrap();

            let mut buffer = itoa::Buffer::new();
            let tellertje_str = buffer.format(frac);
            draw_key_value(&mut display, 2, "frac", tellertje_str).unwrap();
            display.flush().unwrap();
        }
    }
}

// TODO: find out if drawing can be safely stalled by bittide controllers
// TODO: need some common graphics crate for minsync with tools like this
fn draw_key_value<D>(display: &mut D, line: i32, key: &str, value: &str) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    // Draw some texts
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X9)
        .text_color(BinaryColor::On)
        .build();

    let mut key_value: String<22> = String::new();

    key_value.push_str(key).ok();
    key_value.push_str(": ").ok();
    key_value.push_str(value).ok();

    let mut text = Text::with_baseline(&key_value, Point::zero(), text_style, Baseline::Top);
    text.position.y = text.bounding_box().size.height as i32 * line;
    text.draw(display)?;

    Ok(())
}
