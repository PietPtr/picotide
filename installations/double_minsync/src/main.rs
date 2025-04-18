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
use embedded_graphics::mono_font::ascii::FONT_6X9;
use embedded_graphics::prelude::DrawTarget;
use embedded_graphics::primitives::{PrimitiveStyle, StyledDrawable};
use embedded_graphics::{
    mono_font::{
        ascii::{FONT_6X10, FONT_6X13},
        MonoTextStyleBuilder,
    },
    pixelcolor::BinaryColor,
    prelude::{Dimensions, Point, Size},
    primitives::Rectangle,
    text::{Baseline, Text},
    Drawable,
};
use embedded_hal::digital::v2::OutputPin;
use fugit::{HertzU32, RateExtU32};
use heapless::String;
use minsync::hal::pll::setup_pll_blocking;
use panic_probe as _;
use si5351::{Si5351, Si5351Device};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

use minsync::{
    entry,
    hal::{
        self,
        clocks::{ClockSource, ClocksManager},
        gpio::{bank0::Gpio25, FunctionClock, Pin, PullNone},
        pac,
        pll::PLLConfig,
        rosc::RingOscillator,
        Clock, Watchdog, I2C,
    },
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

    pac.CLOCKS.clk_gpout3_div().write(|w| {
        w.frac().variant(0);
        w.int().variant(50000)
    });

    pac.CLOCKS.clk_gpout3_ctrl().write(|w| {
        w.auxsrc().clksrc_pll_sys();
        // w.auxsrc().clk_sys();
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

    let mut tellertje = 0;

    // ------------- clocking code ---------------

    let si_frequency: fugit::Rate<u32, 1, 1> = 12.MHz();

    // TODO: should also move to something common for minsync, reuse that in examples too
    let i2c = I2C::i2c1(
        pac.I2C1,
        pins.si_sda.reconfigure(),
        pins.si_scl.reconfigure(),
        1.kHz(), // TODO: this is fickle, and its not very clear when si doesn't update
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

    pac.XOSC.ctrl().write(|w| w.enable().disable());

    info!("System clock now on SI clock.");

    // -------------

    loop {
        for frac in (0..generated_constants::SI_FRAC).chain((0..generated_constants::SI_FRAC).rev())
        {
            for _ in 0..10_000 {
                asm::nop();
            }

            si_clock
                .setup_pll(si5351::PLL::A, 35, (frac as u32) * 10, 0xfffff)
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

    #[allow(clippy::empty_loop)]
    loop {
        for _ in 0..1_000_000 {
            asm::nop();
        }

        led.set_high().unwrap();

        for _ in 0..10_000 {
            asm::nop();
        }

        led.set_low().unwrap();

        Rectangle::new(Point::new(0, 18), Size::new(128, 32))
            .draw_styled(&PrimitiveStyle::with_fill(BinaryColor::Off), &mut display)
            .unwrap();

        let mut buffer = itoa::Buffer::new();
        let tellertje_str = buffer.format(tellertje);
        draw_key_value(&mut display, 2, "tellertje", tellertje_str).unwrap();

        display.flush().unwrap();

        tellertje += 1;
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
