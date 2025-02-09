pub trait StateMachine<'a, I, O> {
    type State;
    type Configuration;

    fn init(state: Self::State, configuration: &'a Self::Configuration) -> Self;
    fn next(&mut self, input: I) -> O;
}
