use std::{marker::PhantomData, sync::Arc};

use crate::state_machine::{StateMachine, SurfDeserialize, SurfSerialize};

/// A surf program is defined as a graph of state machines, generic over the structure of data that is transmitted
/// by the target bittide network and the maximum degree that the nodes support.
pub struct SurfProgram<WordType, const DEGREE: usize> {
    // TODO: we want to be able to simulate, but also need to be able to compile easily.
    // I think we can make the trait as complex / arcane as we need, as long as the body of the next() function is untouched
    // For compilation the proc macro should gather those and at the end put it all in a lib.rs
    // -> focus on getting sim working
    state_machines: Vec<Arc<SurfNode<WordType, DEGREE>>>,
}

/// A node is a full definition of what will run on a node. Hence it should have an amount of inputs
/// and outputs that correspond to the node, and those outputs are of the type that the physical system
/// transmits words in.
/// TODO: convenience functions that upgrade a state machine to a SurfNode using surfserde and a user provided allocation
pub struct SurfNode<WordType, const DEGREE: usize> {
    program: Box<dyn StateMachine<[WordType; DEGREE], [WordType; DEGREE]>>,
    // TODO: all auxiliary data necessary for compilation can be stored here
    // e.g. which links are active
}
