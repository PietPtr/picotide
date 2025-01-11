use bittide::bittide::BittideChannelControl;
use controllers::si5351::Si5351Controller;
use rp_pico::{
    hal::{
        gpio::{
            bank0::{Gpio26, Gpio27},
            FunctionI2c, Pin, PullUp,
        },
        I2C,
    },
    pac::I2C1,
};

use crate::chips::rp2040::Rp2040Links;

pub type Control = BittideChannelControl<
    Si5351Controller<
        I2C<
            I2C1,
            (
                Pin<Gpio26, FunctionI2c, PullUp>,
                Pin<Gpio27, FunctionI2c, PullUp>,
            ),
        >,
    >,
    64,
    Rp2040Links,
    4,
    crate::chips::rp2040::SioFifo,
>;
