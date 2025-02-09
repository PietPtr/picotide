use surf_proc::state_machine;

state_machine! {
    Name = MonteCarlo;

    struct Configuration {
        // The amount of iterations to run before publishing the sum
        start_iterations: u32,
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
