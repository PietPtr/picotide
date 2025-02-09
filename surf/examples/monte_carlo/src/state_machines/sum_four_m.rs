use surf_proc::state_machine;

use super::aggregrate_m::AggregrateOutput;

state_machine! {
    Name = SumFour;

    struct Configuration {}

    enum State {
        Unit
    }

    struct Input {
        inps: (
            Option<AggregrateOutput>,
            Option<AggregrateOutput>,
            Option<AggregrateOutput>,
            Option<AggregrateOutput>,
        )
    }

    struct Output {
        sum: i32
    }

    impl {
        Unit => {
            let (one, two, three, four) = input.inps;

            let mut sum = 0;

            if let Some(s) = one {
                sum += s.accumulated_error;
            }
            if let Some(s) = two {
                sum += s.accumulated_error;
            }
            if let Some(s) = three {
                sum += s.accumulated_error;
            }
            if let Some(s) = four {
                sum += s.accumulated_error;
            }

            SumFourOutput { sum }
        }
    }
}
