use super::aggregrate::AggregrateOutput;

/// Accepts up to four inputs, and sums them
pub struct SumFour {
    state: SumFourState,
}

pub enum SumFourState {
    Unit,
}

pub struct SumFourOutput {
    sum: i32,
}

impl<'a>
    surf_lang::StateMachine<
        'a,
        (
            Option<AggregrateOutput>,
            Option<AggregrateOutput>,
            Option<AggregrateOutput>,
            Option<AggregrateOutput>,
        ),
        SumFourOutput,
    > for SumFour
{
    type State = SumFourState;

    type Configuration = ();

    fn init(state: Self::State, (): &'a Self::Configuration) -> Self {
        Self { state }
    }

    fn next(
        &mut self,
        input: (
            Option<AggregrateOutput>,
            Option<AggregrateOutput>,
            Option<AggregrateOutput>,
            Option<AggregrateOutput>,
        ),
    ) -> SumFourOutput {
        match self.state {
            SumFourState::Unit => {
                let (one, two, three, four) = input;

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
}
