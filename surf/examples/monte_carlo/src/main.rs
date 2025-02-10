use std::sync::Arc;

use state_machines::monte_carlo_m::{MonteCarlo, MonteCarloConfiguration, MonteCarloState};
use surf_lang::StateMachine;

mod state_machines;

fn main() {
    // TODO: compose these state machines and design composition library,
    // TODO: make all the network definitions
    // TODO: code for checking if the composition fits on the network

    let monte_carlo = MonteCarlo::init(
        MonteCarloState::Delay { delay: 1 },
        &MonteCarloConfiguration {
            start_iterations: 16,
        },
    );
}

/// A surf program is defined as a graph of state machines.
pub struct SurfProgram {
    // TODO: we want to be able to simulate, but also need to be able to compile easily.
    // I think we can make the trait as complex / arcane as we need, as long as the body of the next() function is untouched
    // For compilation the proc macro should gather those and at the end put it all in a lib.rs
    // -> focus on getting sim working
    state_machines: Vec<Arc<dyn StateMachine>>,
}

pub struct SurfNode<'a, SM, I, O, S, C>
where
    SM: StateMachine<'a, I, O, State = S, Configuration = C>,
{
    program: SM,
    _marker: &'a std::marker::PhantomData<(I, O, S, C)>,
}
