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
    divider: I16F16,
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
        }
    }
}

impl<SDA, SCL> Si5351Controller<I2C<I2C1, (SDA, SCL)>> {
    /// TODO: This blocks and is too slow
    /// """non""" blocking operation idea:
    /// - every iteration of this control loop, put one (or FIFO size, or however many fit) byte into the tx fifo of the I2C block.
    /// - or just use DMA?
    ///
    /// or
    ///
    /// the config needs an address + 9 bytes, which is less than the 16 bytes that the i2c TX fifo seems to have, so this call _should_
    /// be non-blocking already, however, run() is certainly called more often than it takes to transmit those 83 bits over a ≤400kHz line.
    /// So we can monitor the fill level of the tx fifo here to check if we're ready to send.
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
        let frac = frac & 0x7ffff; // TODO: verify. error?
        self.si
            .setup_pll(PLL::A, 35, frac, 0xfffff)
            .map_err(|_| Si5351Error::SetupPllI2cError)
    }
}

const PLL_FRAC_MAX: I16F16 = I16F16::from_bits(0xfffff);
const PLL_FRAC_MIN: I16F16 = I16F16::from_bits(0x00000);

pub enum Si5351Error {
    SetupPllI2cError,
}

impl<SDA, SCL, const B: usize> FrequencyController<B> for Si5351Controller<I2C<I2C1, (SDA, SCL)>> {
    type Error = Si5351Error;

    fn run(&mut self, buffer_levels: &[usize]) -> Result<(), Self::Error> {
        //TODO: remove some common code between controllers
        assert!(buffer_levels.len() >= self.degree); // TODO: return Err? type level thing?
        let half_full = (self.degree * B) / 2;
        let total_level: usize = buffer_levels.iter().sum();

        let adjust = self
            .pid
            .run(I16F16::from_num(half_full), I16F16::from_num(total_level));

        self.divider = (self.divider + adjust).clamp(PLL_FRAC_MIN, PLL_FRAC_MAX);

        // TODO: takes ~1ms to apply (depending on I2C speed), might be too long?
        self.set_pll_frac(self.divider.to_bits() as u32)?; // TODO: way more safety in these type conversions?

        Ok(())
    }

    fn set_degree(&mut self, new_degree: usize) {
        self.degree = new_degree
    }
}
