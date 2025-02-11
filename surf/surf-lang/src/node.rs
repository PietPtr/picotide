//TODO: not really a very strong trait, and will fttb only have one impl
pub trait SurfNode {
    type WordType;
    type Input;
    type Output;

    fn cycle(&mut self, input: Self::Input) -> Self::Output;
}
