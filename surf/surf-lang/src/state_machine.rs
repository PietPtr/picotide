pub mod generic;

// pub trait StateMachine<I, O, const INPUT_COUNT: usize, const OUTPUT_COUNT: usize> {
pub trait StateMachine<I, O> {
    fn transition(&mut self, input: I) -> O;
    // fn materialize<WordType, SM>(&self) -> SM
    // where
    //     SM: StateMachine<[WordType; INPUT_COUNT], [WordType; OUTPUT_COUNT], 1, 1>;
}

pub trait SurfSerialize<WordType> {
    fn serialize(&self) -> Option<WordType>;
}

pub trait SurfDeserialize<WordType> {
    fn deserialize(word: WordType) -> Option<Self>
    where
        Self: std::marker::Sized;
}
