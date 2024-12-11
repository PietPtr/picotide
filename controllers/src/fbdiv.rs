use core::ops::RangeInclusive;

use rp_pico::pac::PLL_SYS;

use crate::controller::FrequencyController;

/// Frequency controller that affects frequency through the fbdiv_int register on the rpi pico.
/// This register must be of a value between 16 and 320, and a lower value results in a higher
/// system clock frequency.
pub struct FbdivController {
    pll_sys: PLL_SYS,
    scaler: i16,
}

impl FbdivController {
    pub fn new(pll_sys: PLL_SYS, scaler: i16) -> Self {
        Self { pll_sys, scaler }
    }

    pub fn read_fbdiv(&self) -> u16 {
        self.pll_sys.fbdiv_int.read().fbdiv_int().bits()
    }

    pub fn write_fbdiv(&mut self, new_fbdiv: u16) {
        let new_fbdiv = if !FBDIV_RANGE.contains(&new_fbdiv) {
            new_fbdiv.clamp(FBDIV_RANGE.min().unwrap(), FBDIV_RANGE.max().unwrap())
        } else {
            new_fbdiv
        };

        self.pll_sys
            .fbdiv_int
            .write(|w| unsafe { w.fbdiv_int().bits(new_fbdiv) });
    }
}

const FBDIV_RANGE: RangeInclusive<u16> = 16..=320;

impl<const N: usize, const B: usize> FrequencyController<N, B> for FbdivController {
    fn run(&mut self, buffer_levels: &[usize]) {
        assert_eq!(buffer_levels.len(), N); // TODO: compile time?
        let half_full = (N * B) / 2;
        let total_level: usize = buffer_levels.iter().sum();

        let adjust = if total_level > half_full { 1 } else { -1 };

        let new_fbdiv = (self.read_fbdiv() as i32 + adjust) as u16;

        self.write_fbdiv(new_fbdiv);
    }
}
