use bittide::bittide::BittideChannelControl;
use controllers::fbdiv::FbdivController;

use crate::chips::rp2040::Rp2040Links;

pub type Control =
    BittideChannelControl<FbdivController, 256, Rp2040Links, 4, crate::chips::rp2040::SioFifo>;
