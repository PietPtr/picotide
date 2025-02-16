use fir::fir::{Fir, FirConfiguration, FirInput, FirOutput, FirState};
use surf_lang::{
    pitopi_minsync::links::LinkAssociation,
    state_machine::{
        generic::{
            Flatten3, Flatten4, LinearComposition, ParallelComposition, Swap, UnitStateMachine,
        },
        StateMachine,
    },
};

fn main() {
    let config = FirConfiguration::from_float(0.5);
    let first_fir = ParallelComposition::new(
        ParallelComposition::new(
            Fir::new(FirState::Register(0), &config),
            UnitStateMachine::<()>::new(),
        ),
        ParallelComposition::new(UnitStateMachine::<()>::new(), UnitStateMachine::<()>::new()),
    );

    let switch_layer_one = ParallelComposition::new(
        Swap::new(),
        ParallelComposition::new(UnitStateMachine::<()>::new(), UnitStateMachine::<()>::new()),
    );

    let switch_layer_two = LinearComposition::new(
        Flatten4::new(),
        LinearComposition::new(
            ParallelComposition::new(
                LinearComposition::new(
                    Flatten3::new(),
                    LinearComposition::new(
                        ParallelComposition::new(UnitStateMachine::<()>::new(), Swap::new()),
                        Flatten3::new(),
                    ),
                ),
                UnitStateMachine::<()>::new(),
            ),
            Flatten4::new(),
        ),
    );

    let routed = LinearComposition::new(
        LinearComposition::new(first_fir, switch_layer_one),
        Flatten4::new(),
    );
    let routed = LinearComposition::new(routed, switch_layer_two);
    let mut routed = LinearComposition::new(Flatten4::new(), routed);

    let input = (
        FirInput {
            sample: 250,
            sum: 0,
        },
        (),
        (),
        (),
    );
    let output = routed.transition(input.clone());
    let output = routed.transition(input.clone());
    let output = routed.transition(input.clone());
    let output = routed.transition(input.clone());
    let output = routed.transition(input.clone());
    dbg!(output.north(), output.east(), output.south(), output.west());
}

fn all_in_one<I, O, SM: StateMachine<I, O>>() {
    let config = FirConfiguration::from_float(0.5);
    LinearComposition::new(
        Flatten4::new(),
        LinearComposition::new(
            LinearComposition::new(
                LinearComposition::new(
                    ParallelComposition::new(
                        ParallelComposition::new(
                            Fir::new(FirState::Register(0), &config),
                            UnitStateMachine::<()>::new(),
                        ),
                        ParallelComposition::new(
                            UnitStateMachine::<()>::new(),
                            UnitStateMachine::<()>::new(),
                        ),
                    ),
                    ParallelComposition::new(
                        Swap::<FirOutput, ()>::new(),
                        ParallelComposition::new(
                            UnitStateMachine::<()>::new(),
                            UnitStateMachine::<()>::new(),
                        ),
                    ),
                ),
                Flatten4::new(),
            ),
            LinearComposition::new(
                Flatten4::new(),
                LinearComposition::new(
                    ParallelComposition::new(
                        LinearComposition::new(
                            Flatten3::new(),
                            LinearComposition::new(
                                ParallelComposition::new(
                                    UnitStateMachine::<()>::new(),
                                    Swap::<FirOutput, ()>::new(),
                                ),
                                Flatten3::new(),
                            ),
                        ),
                        UnitStateMachine::<()>::new(),
                    ),
                    Flatten4::new(),
                ),
            ),
        ),
    );
}
