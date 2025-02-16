use crate::state_machine::StateMachine;

pub fn test_run_statemachine<I, O, SM>(state_machine: &mut SM, inputs: Vec<I>) -> Vec<O>
where
    SM: StateMachine<I, O>,
{
    let mut outputs = Vec::with_capacity(inputs.len());

    for input in inputs {
        outputs.push(state_machine.transition(input));
    }

    outputs
}
