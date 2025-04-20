use core::fmt::Write;
use core::sync::atomic::{self, AtomicI32, AtomicU32};

use bittide::bittide::{BittideChannelControlDebugInfo, BittideChannelControlError};
use controllers::si5351::Si5351Debug;
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, Point, Size},
    primitives::{PrimitiveStyle, Rectangle, StyledDrawable},
    text::{Baseline, Text},
    Drawable,
};
use heapless::String;
use minsync::display::DEFAULT_TEXT_STYLE;

use crate::BittideControlDebugger;

#[derive(Debug, Default)]
pub struct TextDebugger {
    buffer_levels_a: [AtomicU32; 4],
    error: AtomicU32,
    rx_sync_message_counter: AtomicU32,
    rx_comm_message_counter: AtomicU32,
    pll_frac: AtomicU32,
    pid_adjust: AtomicI32,
}

impl TextDebugger {
    pub const fn new() -> Self {
        Self {
            buffer_levels_a: [
                AtomicU32::new(0),
                AtomicU32::new(1),
                AtomicU32::new(2),
                AtomicU32::new(3),
            ],
            error: AtomicU32::new(0),
            rx_sync_message_counter: AtomicU32::new(0),
            rx_comm_message_counter: AtomicU32::new(0),
            pll_frac: AtomicU32::new(0),
            pid_adjust: AtomicI32::new(0),
        }
    }
}

impl BittideControlDebugger<Si5351Debug> for TextDebugger {
    fn update(
        &self,
        debug_info: &BittideChannelControlDebugInfo<Si5351Debug>,
        result: Result<(), BittideChannelControlError>,
    ) {
        for (level, atomic) in debug_info
            .buffer_levels
            .iter()
            .zip(self.buffer_levels_a.iter())
        {
            atomic.store(*level, atomic::Ordering::Relaxed);
        }

        self.error.store(
            BittideChannelControlError::encode(result),
            atomic::Ordering::Relaxed,
        );

        self.rx_comm_message_counter.store(
            debug_info.rx_comm_message_counter,
            atomic::Ordering::Relaxed,
        );

        self.rx_sync_message_counter.store(
            debug_info.rx_sync_message_counter,
            atomic::Ordering::Relaxed,
        );

        self.pll_frac.store(
            debug_info.frequency_controller_debug.frac,
            atomic::Ordering::Relaxed,
        );

        self.pid_adjust.store(
            debug_info.frequency_controller_debug.adjust,
            atomic::Ordering::Relaxed,
        );
    }

    fn draw<D>(&self, display: &mut D, position: Point) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        let size = Size::new(128, 22);

        Rectangle::new(position, size)
            .draw_styled(&PrimitiveStyle::with_fill(BinaryColor::Off), display)?;

        let mut buffer_texts = String::<22>::new();

        let cardinals = ["N", "E", "S", "W"];

        for (buffer_level, cardinal) in self.buffer_levels_a.iter().zip(cardinals.iter()) {
            let mut buffer = itoa::Buffer::new();
            let i_as_str = buffer.format(buffer_level.load(atomic::Ordering::Relaxed));
            buffer_texts.push_str(cardinal).ok();
            buffer_texts.push_str(i_as_str).ok();
            buffer_texts.push(' ').ok();
        }

        Text::with_baseline(&buffer_texts, position, DEFAULT_TEXT_STYLE, Baseline::Top)
            .draw(display)?;

        let error = BittideChannelControlError::decode(self.error.load(atomic::Ordering::Relaxed));

        let mut line_two = String::<50>::new();

        let mut buffer = itoa::Buffer::new();
        let first_num = buffer.format(self.pll_frac.load(atomic::Ordering::Relaxed));

        let mut buffer = itoa::Buffer::new();
        let second_num = buffer.format(self.pid_adjust.load(atomic::Ordering::Relaxed));

        line_two.push_str(first_num).ok();
        line_two.push_str(" ").ok();
        line_two.push_str(second_num).ok();
        line_two.push_str(" ").ok();

        match error {
            Ok(()) => {
                line_two.push_str("Ok()").ok();
            }
            Err(err) => {
                write!(&mut line_two, "{:?}", err).ok();
            }
        };

        Text::with_baseline(
            &line_two,
            position + Point::new(0, 9),
            DEFAULT_TEXT_STYLE,
            Baseline::Top,
        )
        .draw(display)?;

        Ok(())
    }
}
