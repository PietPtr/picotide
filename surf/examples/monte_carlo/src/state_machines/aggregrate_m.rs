use surf_proc::state_machine;

use super::monte_carlo_m::MonteCarloOutput;

state_machine! {
    Name = Aggregrate;

    struct Configuration {
        expected_midpoint: i32
    }

    enum State {
        AccumulatedError { error: i32 }
    }

    struct Input {
        monte_carlo_sum: MonteCarloOutput
    }

    struct Output {
        pub accumulated_error: i32,
    }

    impl {
        AccumulatedError { error } => {
            let new_error = if let Some(sum) = input.monte_carlo_sum.sum {
                self.configuration.expected_midpoint - sum
            } else {
                0
            };

            Output {
                accumulated_error: error + new_error,
            }
        }
    }
}
