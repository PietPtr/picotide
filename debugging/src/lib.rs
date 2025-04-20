#![no_std]

use bittide::bittide::{BittideChannelControlDebugInfo, BittideChannelControlError};
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, Point},
};

pub mod debuggers;

pub trait BittideControlDebugger<F> {
    fn update(
        &self,
        debug_info: &BittideChannelControlDebugInfo<F>,
        result: Result<(), BittideChannelControlError>,
    );

    fn draw<D>(&self, display: &mut D, position: Point) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = BinaryColor>;
}
