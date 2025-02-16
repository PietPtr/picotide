use std::marker::PhantomData;

use crate::{
    node::SurfNode,
    state_machine::{StateMachine, SurfDeserialize, SurfSerialize},
};

use super::{links::LinkAssociation, serde::PitopiData};

pub struct MinsyncNode<
    SM,
    I,
    O,
    InNorthT,
    InEastT,
    InSouthT,
    InWestT,
    OutNorthT,
    OutEastT,
    OutSouthT,
    OutWestT,
> where
    InNorthT: SurfDeserialize<PitopiData>,
    InEastT: SurfDeserialize<PitopiData>,
    InSouthT: SurfDeserialize<PitopiData>,
    InWestT: SurfDeserialize<PitopiData>,
    OutNorthT: SurfSerialize<PitopiData>,
    OutEastT: SurfSerialize<PitopiData>,
    OutSouthT: SurfSerialize<PitopiData>,
    OutWestT: SurfSerialize<PitopiData>,
    I: LinkAssociation<InNorthT, InEastT, InSouthT, InWestT>,
    O: LinkAssociation<OutNorthT, OutEastT, OutSouthT, OutWestT>,
    SM: StateMachine<I, O>,
{
    state_machine: SM,
    #[allow(clippy::type_complexity)]
    _marker: PhantomData<(
        I,
        O,
        InNorthT,
        InEastT,
        InSouthT,
        InWestT,
        OutNorthT,
        OutEastT,
        OutSouthT,
        OutWestT,
    )>,
}

impl<SM, I, O, InNorthT, InEastT, InSouthT, InWestT, OutNorthT, OutEastT, OutSouthT, OutWestT>
    MinsyncNode<
        SM,
        I,
        O,
        InNorthT,
        InEastT,
        InSouthT,
        InWestT,
        OutNorthT,
        OutEastT,
        OutSouthT,
        OutWestT,
    >
where
    InNorthT: SurfDeserialize<PitopiData>,
    InEastT: SurfDeserialize<PitopiData>,
    InSouthT: SurfDeserialize<PitopiData>,
    InWestT: SurfDeserialize<PitopiData>,
    OutNorthT: SurfSerialize<PitopiData>,
    OutEastT: SurfSerialize<PitopiData>,
    OutSouthT: SurfSerialize<PitopiData>,
    OutWestT: SurfSerialize<PitopiData>,
    I: LinkAssociation<InNorthT, InEastT, InSouthT, InWestT>,
    O: LinkAssociation<OutNorthT, OutEastT, OutSouthT, OutWestT>,
    SM: StateMachine<I, O>,
{
    pub fn new(state_machine: SM) -> Self {
        Self {
            state_machine,
            _marker: PhantomData {},
        }
    }
}

impl<SM, I, O, InNorthT, InEastT, InSouthT, InWestT, OutNorthT, OutEastT, OutSouthT, OutWestT>
    SurfNode
    for MinsyncNode<
        SM,
        I,
        O,
        InNorthT,
        InEastT,
        InSouthT,
        InWestT,
        OutNorthT,
        OutEastT,
        OutSouthT,
        OutWestT,
    >
where
    InNorthT: SurfDeserialize<PitopiData>,
    InEastT: SurfDeserialize<PitopiData>,
    InSouthT: SurfDeserialize<PitopiData>,
    InWestT: SurfDeserialize<PitopiData>,
    OutNorthT: SurfSerialize<PitopiData>,
    OutEastT: SurfSerialize<PitopiData>,
    OutSouthT: SurfSerialize<PitopiData>,
    OutWestT: SurfSerialize<PitopiData>,
    I: LinkAssociation<InNorthT, InEastT, InSouthT, InWestT>,
    O: LinkAssociation<OutNorthT, OutEastT, OutSouthT, OutWestT>,
    SM: StateMachine<I, O>,
{
    type WordType = PitopiData;

    type Input = [Self::WordType; 4];
    type Output = [Self::WordType; 4];

    fn cycle(&mut self, input: Self::Input) -> Self::Output {
        // TODO: this code should be running on the rp2040, so the proc macro should see it at some point!
        // TODO: find some way to handle the unwraps
        let north_input = InNorthT::deserialize(input[0]).unwrap();
        let east_input = InEastT::deserialize(input[1]).unwrap();
        let south_input = InSouthT::deserialize(input[2]).unwrap();
        let west_input = InWestT::deserialize(input[3]).unwrap();

        let output = self.state_machine.transition(I::construct(
            north_input,
            east_input,
            south_input,
            west_input,
        ));

        [
            output.north().serialize().unwrap(),
            output.east().serialize().unwrap(),
            output.south().serialize().unwrap(),
            output.west().serialize().unwrap(),
        ]
    }
}
