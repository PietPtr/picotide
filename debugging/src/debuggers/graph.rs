use core::fmt::Write;
use core::sync::atomic::{self, AtomicI32, AtomicU32};

use bittide::bittide::{BittideChannelControlDebugInfo, BittideChannelControlError};
use controllers::si5351::Si5351Debug;
use defmt::info;
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, Point, Size},
    primitives::{PrimitiveStyle, Rectangle, StyledDrawable},
    text::{Baseline, Text},
    Drawable,
};
use heapless::String;
use minsync::display::{DEFAULT_TEXT_STYLE, ON_STYLE};

use crate::BittideControlDebugger;

#[derive(Debug, Default)]
pub struct GraphDebugger {
    settings: GraphDebuggerSettings,
    buffer_levels: [AtomicU32; 4],
    error: AtomicU32,
    rx_sync_message_counter: AtomicU32,
    rx_comm_message_counter: AtomicU32,
    pll_frac: AtomicU32,
    pid_adjust: AtomicI32,
}

#[derive(Debug, Default)]
pub struct GraphDebuggerSettings {
    pub buffer_size: usize,
}

impl GraphDebugger {
    pub const fn new(settings: GraphDebuggerSettings) -> Self {
        Self {
            settings,
            buffer_levels: [
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

impl BittideControlDebugger<Si5351Debug> for GraphDebugger {
    fn update(
        &self,
        debug_info: &BittideChannelControlDebugInfo<Si5351Debug>,
        result: Result<(), BittideChannelControlError>,
    ) {
        for (level, atomic) in debug_info
            .buffer_levels
            .iter()
            .zip(self.buffer_levels.iter())
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

    fn draw<D>(&self, display: &mut D, _position: Point) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        display.clear(BinaryColor::Off)?;

        Rectangle::new(
            Point::new(0, display.bounding_box().size.height as i32 / 2),
            Size::new(1, 1),
        )
        .draw_styled(ON_STYLE, display)?;

        Rectangle::new(
            Point::new(
                2 * self.buffer_levels.len() as i32,
                display.bounding_box().size.height as i32 / 2,
            ),
            Size::new(1, 1),
        )
        .draw_styled(ON_STYLE, display)?;

        for (x, buffer_level) in self.buffer_levels.iter().enumerate() {
            let buffer_level = buffer_level.load(atomic::Ordering::Relaxed) as i32;

            // Assumes buffer sizes are an integer multiple of 32
            let scale = (self.settings.buffer_size / 32) as i32;
            let length = buffer_level / scale;

            Rectangle::new(
                Point::new(
                    1 + (x as i32) * 2,
                    display.bounding_box().size.height as i32 - length,
                ),
                Size::new(1, length as u32),
            )
            .draw_styled(ON_STYLE, display)?;
        }

        // TODO: graph animating PLL frac
        Ok(())
    }
}
