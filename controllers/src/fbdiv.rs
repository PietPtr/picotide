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
    degree: usize,
    pll_sys: PLL_SYS,
    fbdiv_internal: I16F16,
    pid: PidControl,

    /// For debugging the first few runs of the controller
    debug: Vec<(usize, I16F16), 256>,
    i: u32,
}

impl FbdivController {
    pub fn new(degree: usize, pll_sys: PLL_SYS, pid_settings: PidSettings) -> Self {
        let initial_fbdiv = pll_sys.fbdiv_int().read().fbdiv_int().bits();

        Self {
            degree,
            pll_sys,
            fbdiv_internal: I16F16::from_num(initial_fbdiv),
            pid: PidControl::new(pid_settings),
            debug: Vec::new(),
            i: 0,
        }
    }

    pub fn read_fbdiv(&self) -> u16 {
        self.pll_sys.fbdiv_int().read().fbdiv_int().bits()
    }

    pub fn write_fbdiv(&mut self, new_fbdiv: i32) {
        let new_fbdiv = if !FBDIV_RANGE.contains(&new_fbdiv) {
            new_fbdiv.clamp(FBDIV_RANGE.min().unwrap(), FBDIV_RANGE.max().unwrap())
        } else {
            new_fbdiv
        } as u16;

        self.pll_sys
            .fbdiv_int()
            .write(|w| unsafe { w.fbdiv_int().bits(new_fbdiv) });
    }
}

// const FBDIV_RANGE: RangeInclusive<u16> = 16..=320;
const FBDIV_RANGE: RangeInclusive<i32> = 97..=103;

impl<const B: usize> FrequencyController<B> for FbdivController {
    type Error = ();

    fn run(&mut self, buffer_levels: &[usize]) -> Result<(), Self::Error> {
        self.i += 1;
        assert!(buffer_levels.len() >= self.degree); // TODO: return Err?
        let half_full = (self.degree * B) / 2;
        let total_level: usize = buffer_levels.iter().sum();

        if self.i % 32768 == 0 {
            defmt::info!(
                "{} buffer_levels={}->{} fbdiv={} (={})",
                self.i,
                buffer_levels,
                half_full,
                self.fbdiv_internal.to_num::<f32>(),
                self.pll_sys.fbdiv_int().read().fbdiv_int().bits(),
            );

            // for (i, (dbg, _)) in self.debug.iter().enumerate() {
            //     defmt::info!("{} = {}", i, dbg);
            // }
        }

        let adjust = self
            .pid
            .run(I16F16::from_num(half_full), I16F16::from_num(total_level));

        self.fbdiv_internal = self.fbdiv_internal.saturating_sub(adjust);

        self.fbdiv_internal = self.fbdiv_internal.clamp(
            I16F16::from_num(FBDIV_RANGE.min().unwrap()),
            I16F16::from_num(FBDIV_RANGE.max().unwrap()),
        );

        if self.fbdiv_internal == I16F16::from_num(FBDIV_RANGE.min().unwrap()) {
            self.fbdiv_internal = I16F16::from_num(100);
        }

        if !self.debug.is_full() {
            self.debug.push((total_level, self.fbdiv_internal)).unwrap();
        }

        let fbdiv_int = self.fbdiv_internal.round().int().to_bits() >> 16;

        self.write_fbdiv(fbdiv_int);

        Ok(())
    }

    fn set_degree(&mut self, new_degree: usize) {
        self.degree = new_degree
    }
}
