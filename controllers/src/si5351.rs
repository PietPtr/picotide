use embedded_hal::blocking::i2c;
use fixed::types::I16F16;
use rp_pico::pac::{self};
use si5351::{Si5351Device, PLL};

use crate::{
    controller::FrequencyController,
    pid::{PidControl, PidSettings},
};

// Put all hardware specific things to set a frac in an impl for this so we can mock the whole thing.
pub trait Si5351 {
    type Error;
    fn set_pll_frac(&mut self, frac: u32) -> Result<(), Self::Error>;
}

impl<I2C, E> Si5351 for Si5351Device<I2C>
where
    I2C: i2c::WriteRead<Error = E> + i2c::Write<Error = E>,
{
    type Error = si5351::Error;

    fn set_pll_frac(&mut self, frac: u32) -> Result<(), Self::Error> {
        // unsafe and a hack, can we do the ownership in a better way? only works if the SI is connected on I2C1 now...
        unsafe {
            let pac = pac::Peripherals::steal();
            let fill_level = pac.I2C1.ic_txflr().read().txflr().bits();

            // Cannot run I2C non-blocking
            if fill_level > 0 {
                return Ok(());
            }
        }

        let frac = frac & 0xfffff; // TODO: verify. error?

        si5351::Si5351::setup_pll(self, PLL::A, 35, frac, 0xfffff)
    }
}

pub struct Si5351Controller<SI> {
    degree: usize,
    si: SI,
    pid: PidControl,
    divider: i32,
    debug: Si5351Debug,
}

impl<SI> Si5351Controller<SI>
where
    SI: Si5351,
{
    pub fn new(si: SI, degree: usize, settings: PidSettings) -> Self {
        Self {
            degree,
            si,
            pid: PidControl::new(settings),
            divider: PLL_FRAC_MAX / 2,
            debug: Default::default(),
        }
    }
}

const PLL_FRAC_MAX: i32 = 0xf_ffff;
const PLL_FRAC_MIN: i32 = 0x0_0000;

pub enum Si5351Error {
    SetupPllI2cError,
    PidError,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Si5351Debug {
    pub frac: u32,
    pub adjust: i32,
}

impl<SI: Si5351, const B: usize> FrequencyController<B> for Si5351Controller<SI> {
    type Error = Si5351Error;
    type Debug = Si5351Debug;

    fn run(&mut self, buffer_levels: &[usize]) -> Result<(), Self::Error> {
        //TODO: remove some common code between controllers
        assert!(buffer_levels.len() >= self.degree); // TODO: return Err? type level thing?
        let half_full = (self.degree * B) / 2;
        let total_level: usize = buffer_levels.iter().sum();

        let adjust = self
            .pid
            .run(I16F16::from_num(half_full), I16F16::from_num(total_level))
            .ok_or(Si5351Error::PidError)?
            .to_bits();

        self.debug.adjust = adjust;

        self.divider = self.divider.saturating_sub(adjust);

        let frac_offset_from_center = self.divider >> 12; // Keep 20 msb's
        let frac =
            (PLL_FRAC_MAX / 2 + frac_offset_from_center).clamp(PLL_FRAC_MIN, PLL_FRAC_MAX) as u32;

        // TODO: takes ~1ms to apply (depending on I2C speed), might be too long?
        self.si
            .set_pll_frac(frac)
            .map_err(|_| Si5351Error::SetupPllI2cError)?; // TODO: way more safety in these type conversions?

        Ok(())
    }

    // TODO: obsolete after link masks
    fn set_degree(&mut self, new_degree: usize) {
        self.degree = new_degree
    }

    fn debug(&self) -> Self::Debug {
        self.debug
    }
}
