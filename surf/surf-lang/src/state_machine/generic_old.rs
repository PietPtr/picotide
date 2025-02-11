use std::{marker::PhantomData, usize};

use super::{StateMachine, SurfDeserialize, SurfSerialize};

/// Generic over any state machine whose I/O implements Serialize/Deserialize, and generates a state machine
/// that serde's its I/O to the given WordType.
/// It panics on failed serde's.
pub struct SerdeStateMachine<WordType, SM, I, O>
where
    I: SurfDeserialize<WordType>,
    O: SurfSerialize<WordType>,
    SM: StateMachine<I, O>,
{
    state_machine: SM,
    _marker: PhantomData<(I, O, WordType)>,
}

impl<WordType, SM, I, O> SerdeStateMachine<WordType, SM, I, O>
where
    I: SurfDeserialize<WordType>,
    O: SurfSerialize<WordType>,
    SM: StateMachine<I, O>,
{
    pub fn new(state_machine: SM) -> Self {
        Self {
            state_machine,
            _marker: PhantomData,
        }
    }
}

// TODO: WordType capitalisation...
// TODO: get the load bearing proc macro involved, how can it know of this state machine? (and other generic state machines)
impl<WordType, SM, I, O> StateMachine<WordType, WordType> for SerdeStateMachine<WordType, SM, I, O>
where
    I: SurfDeserialize<WordType>,
    O: SurfSerialize<WordType>,
    SM: StateMachine<I, O>,
{
    fn next(&mut self, input: WordType) -> WordType {
        let deserialized = I::deserialize(input).expect("Cannot deserialize.");

        let output = self.state_machine.next(deserialized);

        output.serialize().expect("Cannot serialize.")
    }

    fn materialize<WordTypeM, const INPUT_COUNT: usize, const OUTPUT_COUNT: usize, SM2>(
        &self,
    ) -> SM2
    where
        SM2: StateMachine<[WordTypeM; INPUT_COUNT], [WordTypeM; OUTPUT_COUNT]>,
    {
        todo!()
    }
}

// TODO: HUUUUGE types
// pub struct FourLinksInput<WordType, TNorth, TEast, TSouth, TWest>
// where
//     TNorth: SurfDeserialize<WordType>,
//     TEast: SurfDeserialize<WordType>,
//     TSouth: SurfDeserialize<WordType>,
//     TWest: SurfDeserialize<WordType>,
// {
//     north: TNorth,
//     east: TEast,
//     south: TSouth,
//     west: TWest,
//     _marker: PhantomData<WordType>,
// }

// pub struct FourLinksOutput<WordType, TNorth, TEast, TSouth, TWest>
// where
//     TNorth: SurfSerialize<WordType>,
//     TEast: SurfSerialize<WordType>,
//     TSouth: SurfSerialize<WordType>,
//     TWest: SurfSerialize<WordType>,
// {
//     north: TNorth,
//     east: TEast,
//     south: TSouth,
//     west: TWest,
//     _marker: PhantomData<WordType>,
// }

// pub struct FourLinkStateMachine<
//     WordType,
//     SM,
//     InTNorth,
//     InTEast,
//     InTSouth,
//     InTWest,
//     OutTNorth,
//     OutTEast,
//     OutTSouth,
//     OutTWest,
// > where
//     InTNorth: SurfDeserialize<WordType>,
//     InTEast: SurfDeserialize<WordType>,
//     InTSouth: SurfDeserialize<WordType>,
//     InTWest: SurfDeserialize<WordType>,
//     OutTNorth: SurfSerialize<WordType>,
//     OutTEast: SurfSerialize<WordType>,
//     OutTSouth: SurfSerialize<WordType>,
//     OutTWest: SurfSerialize<WordType>,
//     SM: StateMachine<
//         FourLinksInput<WordType, InTNorth, InTEast, InTSouth, InTWest>,
//         FourLinksOutput<WordType, OutTNorth, OutTEast, OutTSouth, OutTWest>,
//     >,
// {
//     state_machine: SM,
//     _marker: PhantomData<(
//         WordType,
//         InTNorth,
//         InTEast,
//         InTSouth,
//         InTWest,
//         OutTNorth,
//         OutTEast,
//         OutTSouth,
//         OutTWest,
//     )>,
// }

// impl<
//         WordType,
//         GSM,
//         InTNorth,
//         InTEast,
//         InTSouth,
//         InTWest,
//         OutTNorth,
//         OutTEast,
//         OutTSouth,
//         OutTWest,
//     >
//     FourLinkStateMachine<
//         WordType,
//         GSM,
//         InTNorth,
//         InTEast,
//         InTSouth,
//         InTWest,
//         OutTNorth,
//         OutTEast,
//         OutTSouth,
//         OutTWest,
//     >
// where
//     InTNorth: SurfDeserialize<WordType>,
//     InTEast: SurfDeserialize<WordType>,
//     InTSouth: SurfDeserialize<WordType>,
//     InTWest: SurfDeserialize<WordType>,
//     OutTNorth: SurfSerialize<WordType>,
//     OutTEast: SurfSerialize<WordType>,
//     OutTSouth: SurfSerialize<WordType>,
//     OutTWest: SurfSerialize<WordType>,
//     GSM: StateMachine<
//         FourLinksInput<WordType, InTNorth, InTEast, InTSouth, InTWest>,
//         FourLinksOutput<WordType, OutTNorth, OutTEast, OutTSouth, OutTWest>,
//     >,
// {
//     pub fn new<SM, I, O>(state_machine: SM) -> Self
//     where
//     I:
//         SM: StateMachine<I, O>,
//     {
//         todo!()
//     }
// }

// TODO: very fancy, doesn't work vvv

pub struct ManyToManySerdeStateMachine<
    WordType,
    SM,
    I,
    O,
    const INPUT_COUNT: usize,
    const OUTPUT_COUNT: usize,
> where
    I: DeserializeMultipleInput<WordType, INPUT_COUNT> + SurfDeserialize<WordType>,
    O: SerializeMultipleOutput<WordType, OUTPUT_COUNT> + SurfSerialize<WordType>,
    SM: StateMachine<I, O>,
{
    state_machine: SM,
    _marker: PhantomData<(I, O, WordType)>,
}

impl<WordType, SM, I, O, const INPUT_COUNT: usize, const OUTPUT_COUNT: usize>
    ManyToManySerdeStateMachine<WordType, SM, I, O, INPUT_COUNT, OUTPUT_COUNT>
where
    I: DeserializeMultipleInput<WordType, INPUT_COUNT> + SurfDeserialize<WordType>,
    O: SerializeMultipleOutput<WordType, OUTPUT_COUNT> + SurfSerialize<WordType>,
    SM: StateMachine<I, O>,
{
    pub fn new(state_machine: SM) -> Self {
        Self {
            state_machine,
            _marker: PhantomData,
        }
    }
}

// impl<WordType, SM, I, O, const INPUT_COUNT: usize, const OUTPUT_COUNT: usize>
//     StateMachine<[WordType; INPUT_COUNT], [WordType; OUTPUT_COUNT]>
//     for ManyToManySerdeStateMachine<WordType, SM, I, O, INPUT_COUNT, OUTPUT_COUNT>
// where
//     I: DeserializeMultipleInput<WordType, INPUT_COUNT> + SurfDeserialize<WordType>,
//     O: SerializeMultipleOutput<WordType, OUTPUT_COUNT> + SurfSerialize<WordType>,
//     SM: StateMachine<I, O>,
// {
//     fn next(&mut self, input: [WordType; INPUT_COUNT]) -> [WordType; OUTPUT_COUNT] {
//         let deserialized = I::input(index, words)
//     }
// }

pub trait DeserializeMultipleInput<WordType, const INPUT_COUNT: usize> {
    type EnumeratedType;
    fn input(index: usize, words: [WordType; INPUT_COUNT]) -> Option<Self::EnumeratedType>;
}

pub trait SerializeMultipleOutput<WordType, const OUTPUT_COUNT: usize> {
    fn output(self, index: usize) -> WordType;
}

pub enum TwoTupleAsEnum<T1, T2> {
    First(T1),
    Second(T2),
}

impl<WordType, T1, T2> DeserializeMultipleInput<WordType, 2> for (T1, T2)
where
    WordType: Clone,
    T1: SurfDeserialize<WordType>,
    T2: SurfDeserialize<WordType>,
{
    type EnumeratedType = TwoTupleAsEnum<T1, T2>;
    fn input(index: usize, words: [WordType; 2]) -> Option<Self::EnumeratedType> {
        match index {
            0 => Some(TwoTupleAsEnum::First(T1::deserialize(words[0].clone())?)),
            1 => Some(TwoTupleAsEnum::Second(T2::deserialize(words[1].clone())?)),
            _ => panic!("No index {index} in 2-tuple"),
        }
    }
}
