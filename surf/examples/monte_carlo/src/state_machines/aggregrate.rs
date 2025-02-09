use super::monte_carlo::MonteCarloOutput;

pub struct Aggregrate<'a> {
    state: AggregrateState,
    configuration: &'a AggregrateConfiguration,
}

pub enum AggregrateState {
    AccumulatedError { error: i32 },
}

pub struct AggregrateConfiguration {
    expected_midpoint: i32,
}

pub struct AggregrateOutput {
    pub accumulated_error: i32,
}

impl<'a> surf_lang::StateMachine<'a, MonteCarloOutput, AggregrateOutput> for Aggregrate<'a> {
    type State = AggregrateState;
    type Configuration = AggregrateConfiguration;

    fn init(state: Self::State, configuration: &'a Self::Configuration) -> Self {
        Self {
            state,
            configuration,
        }
    }

    fn next(&mut self, input: MonteCarloOutput) -> AggregrateOutput {
        match self.state {
            AggregrateState::AccumulatedError { error } => {
                let new_error = if let Some(sum) = input {
                    self.configuration.expected_midpoint - sum.sum
                } else {
                    0
                };

                AggregrateOutput {
                    accumulated_error: error + new_error,
                }
            }
        }
    }
}
