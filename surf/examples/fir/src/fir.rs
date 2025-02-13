use surf_proc::state_machine;

type Sample = i32;
type Sum = i32;

state_machine! {
    Name = Fir;

    struct Configuration {
        // The constant to multiply the signal with at this stage
        constant: i32,
        // The constant is a fixed point number, this field denotes the fractional bits
        fractional_bits: u8,
    }

    enum State {
        Register(Sample)
    }

    struct Input {
        pub sample: Sample,
        pub sum: Sum,
    }

    struct Output {
        pub sample: Sample,
        pub sum: Sum,
    }

    impl {
        Register(sample) => {
            let result = sample as i64 * self.configuration.constant as i64;
            let result = (result >> self.configuration.fractional_bits) as i32;
            let sum = input.sum + result;

            self.state = State::Register(input.sample);

            Output {
                sample,
                sum,
            }
        }
    }
}

impl FirConfiguration {
    pub fn to_float(&self) -> f32 {
        (self.constant >> self.fractional_bits) as f32
            + (self.constant & ((1 << self.fractional_bits) - 1)) as f32
                / (1 << self.fractional_bits) as f32
    }

    pub fn from_float(constant: f32) -> Self {
        let fractional_bits = 16;
        let scaled = (constant * (1 << fractional_bits) as f32).round() as i32;
        Self {
            constant: scaled,
            fractional_bits,
        }
    }
}

#[test]
fn test_fir() {
    use surf_lang::test::runner::test_run_statemachine;

    let config = FirConfiguration {
        constant: 0b1,
        fractional_bits: 2,
    };
    let mut fir = Fir::new(FirState::Register(0), &config);

    println!("constant={}", config.to_float());
    assert_eq!(config.to_float(), 0.25);

    let inputs = vec![
        FirInput {
            sample: 1000,
            sum: 0,
        },
        FirInput {
            sample: 500,
            sum: 100,
        },
        FirInput {
            sample: -500,
            sum: 200,
        },
        FirInput {
            sample: -1000,
            sum: 400,
        },
        FirInput {
            sample: -1000,
            sum: 400,
        },
    ];

    let outputs = test_run_statemachine(&mut fir, inputs);

    dbg!(outputs);
    // TODO: once derives work, impl partialeq and add assert
}
