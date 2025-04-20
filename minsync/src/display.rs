//! Helper functions to setup the display and common debugging tools

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, StyledDrawable},
    text::{Baseline, Text},
};
use hal::pac;
use hal::{
    gpio::{
        bank0::{Gpio12, Gpio13},
        FunctionI2c, Pin, PullUp,
    },
    I2C,
};
use heapless::String;
use ssd1306::{
    mode::{BufferedGraphicsMode, DisplayConfig},
    prelude::{DisplayRotation, I2CInterface},
    size::DisplaySize128x32,
    I2CDisplayInterface, Ssd1306,
};

type DisplayI2C = I2C<
    pac::I2C0,
    (
        Pin<Gpio12, FunctionI2c, PullUp>,
        Pin<Gpio13, FunctionI2c, PullUp>,
    ),
>;

pub fn setup(
    i2c_display: DisplayI2C,
) -> Result<
    Ssd1306<I2CInterface<DisplayI2C>, DisplaySize128x32, BufferedGraphicsMode<DisplaySize128x32>>,
    display_interface::DisplayError,
> {
    let interface = I2CDisplayInterface::new(i2c_display);
    let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init()?;

    Ok(display)
}

pub const DEFAULT_TEXT_STYLE: MonoTextStyle<'_, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_6X10)
    .text_color(BinaryColor::On)
    .build();

pub const ON_STYLE: &PrimitiveStyle<BinaryColor> = &PrimitiveStyle::with_fill(BinaryColor::On);

#[macro_export]
macro_rules! display_i2c {
    ($pac:expr, $pins:expr, $clocks:expr, $freq:expr) => {
        minsync::hal::I2C::i2c0(
            $pac.I2C0,
            $pins.oled_sda.reconfigure(),
            $pins.oled_scl.reconfigure(),
            $freq,
            &mut $pac.RESETS,
            &$clocks.system_clock,
        )
    };
}

pub fn draw_key_integral<D, I>(
    display: &mut D,
    line: i32,
    key: &str,
    value: I,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
    I: itoa::Integer,
{
    let mut buffer = itoa::Buffer::new();
    let i_as_str = buffer.format(value);
    draw_key_value(display, line, key, i_as_str)
}

pub fn draw_key_value<D>(display: &mut D, line: i32, key: &str, value: &str) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let mut key_value: String<22> = String::new();

    key_value.push_str(key).ok();
    key_value.push_str(": ").ok();
    key_value.push_str(value).ok();

    let mut text =
        Text::with_baseline(&key_value, Point::zero(), DEFAULT_TEXT_STYLE, Baseline::Top);

    text.position.y = text.bounding_box().size.height as i32 * line;

    Rectangle::new(text.position, text.bounding_box().size)
        .draw_styled(&PrimitiveStyle::with_fill(BinaryColor::Off), display)?;

    text.draw(display)?;

    Ok(())
}
