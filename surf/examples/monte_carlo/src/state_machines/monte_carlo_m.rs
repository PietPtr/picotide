use surf_proc::state_machine;

// pub struct MonteCarlo<'a, const INPUT_COUNT: usize, const OUTPUT_COUNT: usize> {
//     state: MonteCarloState,
//     configuration: &'a MonteCarloConfiguration,
// }
// pub struct MonteCarloConfiguration {
//     pub start_iterations: u32,
// }
// pub enum MonteCarloState {
//     Delay { delay: u32 },
//     Start,
//     GenerateAndCompute { iterations: u32, sum: i32 },
// }
// pub struct MonteCarloInput {}

// pub struct MonteCarloOutput {
//     pub sum: Option<i32>,
// }
// impl<'a> MonteCarlo<'a, 0, 1> {
//     pub fn new(initial_state: MonteCarloState, configuration: &'a MonteCarloConfiguration) -> Self {
//         Self {
//             state: initial_state,
//             configuration,
//         }
//     }
// }
// impl<'a> surf_lang::state_machine::StateMachine<MonteCarloInput, MonteCarloOutput, 0, 1>
//     for MonteCarlo<'a, 0, 1>
// {
//     fn next(&mut self, _: MonteCarloInput) -> MonteCarloOutput {
//         match self.state {
//             MonteCarloState::Delay { delay } => {
//                 self.state = if delay == 0 {
//                     MonteCarloState::Start
//                 } else {
//                     MonteCarloState::Delay { delay: delay - 1 }
//                 };
//                 MonteCarloOutput { sum: None }
//             }
//             MonteCarloState::Start => {
//                 self.state = MonteCarloState::GenerateAndCompute {
//                     iterations: self.configuration.start_iterations,
//                     sum: 0,
//                 };
//                 MonteCarloOutput { sum: None }
//             }
//             MonteCarloState::GenerateAndCompute { iterations, sum } => {
//                 const ROSC_RANDOM_BIT_ADDRESS: *const u32 = (0x40060000 + 0x1c) as *const u32;
//                 let mut random_num = 0u32;
//                 for _ in 0..16 {
//                     random_num |= unsafe { ROSC_RANDOM_BIT_ADDRESS.read() };
//                     random_num <<= 1;
//                 }
//                 let (new_state, output) = if iterations == 0 {
//                     (MonteCarloState::Start, MonteCarloOutput { sum: Some(sum) })
//                 } else {
//                     (
//                         MonteCarloState::GenerateAndCompute {
//                             iterations: iterations - 1,
//                             sum: sum + random_num as i32,
//                         },
//                         MonteCarloOutput { sum: None },
//                     )
//                 };
//                 self.state = new_state;
//                 output
//             }
//         }
//     }

//     fn materialize<WordType, SM>(&self) -> SM
//     where
//         SM: surf_lang::state_machine::StateMachine<[WordType; 0], [WordType; 1], 1, 1>,
//     {
//         todo!()
//         // TODO: all we really need here is a new next() method which first deserializes the input, then serializes the output, and puts it in the correct link
//         // TODO: i think this depends on better proc macros which support tuples as implicitly defining inputs and outputs (i.e. need type aliases on the ins/outputs)
//         // TODO: first solidify linear state machine composition
//     }
// }

state_machine! {
    Name = MonteCarlo;

    struct Configuration {
        // The amount of iterations to run before publishing the sum
        pub start_iterations: u32,
    }

    enum State {
        Delay { delay: u32 },
        Start,
        GenerateAndCompute { iterations: u32, sum: i32 },
    }

    struct Input {}

    struct Output {
        pub sum: Option<i32>
    }

    impl {
        Delay { delay } => {
            self.state = if delay == 0 {
                State::Start
            } else {
                State::Delay { delay: delay - 1 }
            };

            Output { sum: None }
        }
        Start => {
            self.state = State::GenerateAndCompute {
                iterations: self.configuration.start_iterations,
                sum: 0,
            };

            Output { sum: None}
        }
        GenerateAndCompute { iterations, sum } => {
            const ROSC_RANDOM_BIT_ADDRESS: *const u32 = (0x40060000 + 0x1c) as *const u32;

            // generate random number
            let mut random_num = 0u32;
            for _ in 0..16 {
                random_num |= unsafe { ROSC_RANDOM_BIT_ADDRESS.read() };
                random_num <<= 1;
            }

            let (new_state, output) = if iterations == 0 {
                (State::Start, Output { sum: Some(sum) })
            } else {
                (
                    State::GenerateAndCompute {
                        iterations: iterations - 1,
                        sum: sum + random_num as i32,
                    },
                    Output { sum: None },
                )
            };

            self.state = new_state;

            output
        }
    }
}
