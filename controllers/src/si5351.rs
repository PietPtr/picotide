use defmt::info;
use embedded_hal::blocking::i2c;
use fixed::types::I16F16;
use rp_pico::hal::I2C;
use rp_pico::pac::{self, I2C1};
use si5351::{Si5351, Si5351Device, PLL};

use crate::{
    controller::FrequencyController,
    pid::{PidControl, PidSettings},
};

pub struct Si5351Controller<I2C> {
    degree: usize,
    si: Si5351Device<I2C>,
    pid: PidControl,
    divider: i32,
    debug: Si5351Debug,
}

impl<I2C, E> Si5351Controller<I2C>
where
    I2C: i2c::WriteRead<Error = E> + i2c::Write<Error = E>,
{
    pub fn new(si: Si5351Device<I2C>, degree: usize, settings: PidSettings) -> Self {
        Self {
            degree,
            si,
            pid: PidControl::new(settings),
            divider: PLL_FRAC_MAX / 2,
            debug: Default::default(),
        }
    }
}

impl<SDA, SCL> Si5351Controller<I2C<I2C1, (SDA, SCL)>> {
    fn set_pll_frac(&mut self, frac: u32) -> Result<(), Si5351Error> {
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

        self.debug.frac = frac;

        self.si
            .setup_pll(PLL::A, 35, frac, 0xfffff)
            .map_err(|_| Si5351Error::SetupPllI2cError)
    }
}

const PLL_FRAC_MAX: i32 = 0xf_ffff;
const PLL_FRAC_MIN: i32 = 0x0_0000;

pub enum Si5351Error {
    SetupPllI2cError,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Si5351Debug {
    pub frac: u32,
    pub adjust: i32,
}

impl<SDA, SCL, const B: usize> FrequencyController<B> for Si5351Controller<I2C<I2C1, (SDA, SCL)>> {
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
            .to_bits();

        self.debug.adjust = adjust;

        self.divider = self.divider.saturating_sub(adjust);

        let frac_offset_from_center = self.divider >> 12; // Keep 20 msb's
        let frac =
            (PLL_FRAC_MAX / 2 + frac_offset_from_center).clamp(PLL_FRAC_MIN, PLL_FRAC_MAX) as u32;

        // TODO: takes ~1ms to apply (depending on I2C speed), might be too long?
        self.set_pll_frac(frac)?; // TODO: way more safety in these type conversions?

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

#[test]
fn control_playground() {
    let degree = 4;
    let b = 64;
    let mut pid = PidControl::new(PidSettings {
        kp: I16F16::from_num(0.0001),
        ki: I16F16::from_num(0.0),
        kd: I16F16::from_num(0.0),
    });
    let mut divider = PLL_FRAC_MAX / 2;

    let mut control = |buffer_levels: &[usize]| {
        assert!(buffer_levels.len() >= degree);
        let half_full = (degree * b) / 2;
        let total_level: usize = buffer_levels.iter().sum();

        let adjust = pid
            .run(I16F16::from_num(half_full), I16F16::from_num(total_level))
            .to_bits();

        dbg!(adjust);

        divider = divider.saturating_add(adjust);

        let frac = (PLL_FRAC_MAX / 2 + divider).clamp(PLL_FRAC_MIN, PLL_FRAC_MAX) as u32;

        // (divider.to_bits() as u32) & 0x7ffff
        dbg!(frac);
        dbg!(divider);
        // dbg!(divider.to_bits() & 0x7ffff);

        frac & 0x7ffff
    };

    let mut i = 0;
    loop {
        let frac = control(&[33, 32, 32, 32]);

        i += 1;

        if i == 100 {
            break;
        }
        // println!("{frac:22b} {frac}");
        if frac == 0 {
            panic!("done")
        }
        if frac == 0b11111111111111111111 {
            panic!("done")
        }
    }
}
