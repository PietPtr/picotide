use std::sync::Arc;

use state_machines::monte_carlo_m::{
    MonteCarlo, MonteCarloConfiguration, MonteCarloInput, MonteCarloState,
};
use surf_lang::pitopi_minsync::links::LinkAssociation;
use surf_lang::state_machine::{
    generic::{ParallelComposition, UnitStateMachine},
    StateMachine,
};

mod state_machines;

fn main() {
    // TODO: compose these state machines and design composition library,
    // TODO: make all the network definitions
    // TODO: code for checking if the composition fits on the network

    let monte_carlo = |delay| {
        MonteCarlo::new(
            MonteCarloState::Delay { delay },
            &MonteCarloConfiguration {
                start_iterations: 16,
            },
        )
    };

    let mut node_compatible = ParallelComposition::new(
        ParallelComposition::new(monte_carlo(0), monte_carlo(1)),
        ParallelComposition::new(monte_carlo(2), monte_carlo(3)),
    );

    let input = (
        (MonteCarloInput {}, MonteCarloInput {}),
        (MonteCarloInput {}, MonteCarloInput {}),
    );

    for _ in 0..20 {
        let out = node_compatible.next(input.clone());
        dbg!(out.north(), out.east(), out.south(), out.west());
    }
}
