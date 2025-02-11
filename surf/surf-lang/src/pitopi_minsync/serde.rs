// pub str

use bitvec::prelude::*;

use crate::state_machine::{SurfDeserialize, SurfSerialize};

/// The pitopi protocol reserves 3 bits for control on data words, so we can send 29 bits per transaction
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PitopiData(u32);

pub const PITOPI_DATA_WIDTH: usize = 29;

impl SurfSerialize<PitopiData> for u32 {
    fn serialize(&self) -> Option<PitopiData> {
        if *self > 0x1fff_ffff {
            None
        } else {
            Some(PitopiData(*self))
        }
    }
}

impl SurfDeserialize<PitopiData> for u32 {
    fn deserialize(word: PitopiData) -> Option<Self> {
        if word.0 > 0x1fff_ffff {
            None
        } else {
            Some(word.0 & 0x1fff_ffff)
        }
    }
}

/// Uses 0x1fff_ffff as sentinel value for None
impl<T: SurfSerialize<PitopiData>> SurfSerialize<PitopiData> for Option<T> {
    fn serialize(&self) -> Option<PitopiData> {
        match self {
            Some(t) => {
                let serialized = t.serialize()?;
                if serialized.0 >= 0x1fff_ffff {
                    None
                } else {
                    Some(serialized)
                }
            }
            None => Some(PitopiData(0x1fff_ffff)),
        }
    }
}

impl<T: SurfDeserialize<PitopiData>> SurfDeserialize<PitopiData> for Option<T> {
    fn deserialize(word: PitopiData) -> Option<Self>
    where
        Self: std::marker::Sized,
    {
        if word.0 == 0x1fff_ffff {
            Some(None)
        } else {
            let deserialized = T::deserialize(word)?;
            Some(Some(deserialized))
        }
    }
}
