use core::ops::RangeInclusive;

use fixed::types::I16F16;
use heapless::Vec;
use rp_pico::pac::PLL_SYS;

use crate::{
    controller::FrequencyController,
    pid::{PidControl, PidSettings},
};

/// Frequency controller that affects frequency through the fbdiv_int register on the rpi pico.
/// This register must be of a value between 16 and 320, and a lower value results in a higher
/// system clock frequency.
pub struct FbdivController {
    pll_sys: PLL_SYS,
    fbdiv_internal: I16F16,
    pid: PidControl,

    /// For debugging the first few runs of the controller
    debug: Vec<(usize, I16F16), 256>,
    i: u32,
}

impl FbdivController {
    pub fn new(pll_sys: PLL_SYS, pid_settings: PidSettings) -> Self {
        let initial_fbdiv = pll_sys.fbdiv_int.read().fbdiv_int().bits();

        Self {
            pll_sys,
            fbdiv_internal: I16F16::from_num(initial_fbdiv),
            pid: PidControl::new(pid_settings),
            debug: Vec::new(),
            i: 0,
        }
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

// const FBDIV_RANGE: RangeInclusive<u16> = 16..=320;
const FBDIV_RANGE: RangeInclusive<u16> = 90..=110;

impl<const N: usize, const B: usize> FrequencyController<N, B> for FbdivController {
    fn run(&mut self, buffer_levels: &[usize]) {
        self.i += 1;
        assert_eq!(buffer_levels.len(), N); // TODO: compile time?
        let half_full = (N * B) / 2;
        let total_level: usize = buffer_levels.iter().sum();

        if self.i % 32768 == 0 {
            defmt::info!(
                "{} buffer_levels={}->{} fbdiv={}",
                self.i,
                buffer_levels,
                half_full,
                self.fbdiv_internal.to_num::<f32>(),
            );
        }

        let adjust = self
            .pid
            .run(I16F16::from_num(half_full), I16F16::from_num(total_level));

        self.fbdiv_internal = self.fbdiv_internal.saturating_add(adjust);

        if !self.debug.is_full() {
            self.debug.push((total_level, self.fbdiv_internal)).unwrap();
        }

        if self.i == 1000 {
            defmt::info!(
                "fbdiv controller debug:\n{:?}",
                self.debug
                    .iter()
                    .map(|(buffer_level, internal_fbdiv)| (
                        buffer_level,
                        internal_fbdiv.to_num::<f32>()
                    ))
                    .collect::<Vec<_, 256>>()
            );
        }

        // self.write_fbdiv(self.fbdiv_internal.round().int().to_bits() as u16);
    }
}
