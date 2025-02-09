pub enum MonteCarloState {
    Delay { delay: u32 },
    Start,
    GenerateAndCompute { iterations: u32, sum: i32 },
}

/// Computes the sum of some amount of randomly distributed numbers
pub struct MonteCarlo<'a> {
    state: MonteCarloState,
    configuration: &'a MonteCarloConfiguration,
}

pub struct MonteCarloSum {
    pub sum: i32,
}

pub type MonteCarloOutput = Option<MonteCarloSum>;

pub struct MonteCarloConfiguration {
    start_iterations: u32,
}

impl<'a> surf_lang::StateMachine<'a, (), MonteCarloOutput> for MonteCarlo<'a> {
    type State = MonteCarloState;
    type Configuration = MonteCarloConfiguration;

    // should always be the same anyway, can be done by macro?
    fn init(state: Self::State, configuration: &'a Self::Configuration) -> Self {
        Self {
            state,
            configuration,
        }
    }

    fn next(&mut self, _: ()) -> MonteCarloOutput {
        match self.state {
            MonteCarloState::Delay { delay } => {
                self.state = if delay == 0 {
                    MonteCarloState::Start
                } else {
                    MonteCarloState::Delay { delay: delay - 1 }
                };

                None
            }
            MonteCarloState::Start => {
                self.state = MonteCarloState::GenerateAndCompute {
                    iterations: self.configuration.start_iterations,
                    sum: 0,
                };
                None
            }
            MonteCarloState::GenerateAndCompute { iterations, sum } => {
                const ROSC_RANDOM_BIT_ADDRESS: *const u32 = (0x40060000 + 0x1c) as *const u32;

                // generate random number
                let mut random_num = 0u32;
                for _ in 0..16 {
                    random_num |= unsafe { ROSC_RANDOM_BIT_ADDRESS.read() };
                    random_num <<= 1;
                }

                let (new_state, output) = if iterations == 0 {
                    (MonteCarloState::Start, Some(MonteCarloSum { sum }))
                } else {
                    (
                        MonteCarloState::GenerateAndCompute {
                            iterations: iterations - 1,
                            sum: sum + random_num as i32,
                        },
                        None,
                    )
                };

                self.state = new_state;

                output
            }
        }
    }
}
