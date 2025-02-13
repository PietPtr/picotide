use std::rc::Rc;

use fir::fir::{Fir, FirConfiguration, FirOutput, FirState};
use surf_lang::state_machine::{
    generic::{LinearComposition, ParallelComposition, Swap, UnitStateMachine},
    StateMachine,
};

fn main() {
    let first_fir = ParallelComposition::new(
        ParallelComposition::new(
            Fir::new(FirState::Register(0), &FirConfiguration::from_float(0.5)),
            UnitStateMachine::<()>::new(),
        ),
        ParallelComposition::new(UnitStateMachine::<()>::new(), UnitStateMachine::<()>::new()),
    );

    let switch_layer_one = ParallelComposition::new(
        Swap::<FirOutput, ()>::new(),
        ParallelComposition::new(UnitStateMachine::<()>::new(), UnitStateMachine::<()>::new()),
    );

    let switch_layer_two = ParallelComposition::new(
        ParallelComposition::new(UnitStateMachine::<()>::new(), Swap::<FirOutput, ()>::new()),
        UnitStateMachine::<()>::new(),
    );

    let test = switch_layer_two.next(input);

    let routed = LinearComposition::new(first_fir, switch_layer_one);
    // let routed = LinearComposition::new(routed, switch_layer_two);

    let input = todo!();
    let output = routed.next(input);
}
