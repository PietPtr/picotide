# surf

This draft document outlines ideas on for an approach for efficient engineering for a bittide system.

## Goals

Surf should be a strict subset of Rust that can be trivially compiled to a bittide system. It is:

1) A library of (proc-)macros to construct and compose valid bittide-compatible programs

2) A compile-time check for compatibility with the targeted Bittide topology

3) Tools to rapidly aid in developing efficient programs

## Subgoal 1)

surf consists strictly of state-machines and compositions of state machines. A state machine is defined in the Moore sense and can be modelled as a function (state, input) -> (state, output), or more usual in a language like rust, an instance of a struct with a `next(&mut self, input: I) -> O`, generic over input and output types `I` and `O`.

Each state machine has a very strict implementation of this next function (enforced at compile time). It must define, for each current state, what the next state is, and how to compute output from just the current state and the input. Since this computation will be scheduled on a bittide node, and we want to take advantage of the invariants a synchronized bittide network provides, this computation _must take at most a known, constant, amount of cycles_. Specifically, if the network such that all nodes transmit every 4096 cycles, it is most efficient if each branch of every statemachine takes about 4096 (or a multiple of 4096) cycles to execute. For general purpose Rust code it is impossible to test whether it executes in constant time in constant time, so further restrictions are necessary.

A composition of state machines can be compiled to a bittide network since a node can be assigned to a (composition of) state machines. The input of that state machine will be (an aggregration of) the inputs that the node has access to, and with the output the node will communicate to its neighbors.

It should be possible to use the full Rust feature set to compose state machines. This is because state machines contain the logic that actually runs, but composing state machines into a network that feeds ones inputs to anothers outputs is essentially a compile-time task. By allowing regular Rust for this task, it should be possible to build abstractions on top of state machines, making it easier to build complex programs while maintaining confidence in their correctness. At some point it may become clear that some elementary operations (like destructuring outputs to send them to different inputs) will be necessary. It may be possible to encode that as a single state state machine. 

### Enforcing Constant Execution Time.

The subset of usuable rust in state machine branches must be chosen such that each element takes up a known amount of cycles. The following code structures take _exactly_ the same amount of cycles, no matter the input data (which is a stronger requirement than stated):

1) Arithmetic operations that the target architecture supplies hardware for.

2) `if {} else {}` blocks where each subblock takes the same amount of cycles and the condition runs in a constant amount of cycles.

3) `if {} else if {}` else blocks where all conditions take the same amount of cycles and _each_ condition runs in the same amount of cycles

4) A bounded for loop without break or continue statements. The loop should iterate over a structure that has a length that is known at compile time, e.g. bounded ranges or arrays.

5) Iterators over structures with a known size at compile time (this is essentially the same case as 4.)

6) Function calls to functions that take a constant amount of cycles, since these can be inlined.

Using _exactly_ the same amount of cycles for a given state every single time makes the whole program run more efficiently, since once the state machine produces an output, that output will have to wait for a transmit on the bittide network before the state machine is allowed to advance.

However, it should be allowed by surf to write programs that use _up to_ a known amount of cycles. This will waste some computation time, as for each branch that is possible to take there is a probability that the core will idle uselessly. However, sometimes this will aid in programming ergonomics.

1) Arithmetic operations still take a constant known amount of time.

2) `if {} else {}` statements can take a different amount of time per branch, take the max.

3) `if {} else if {} else {}` statements can take a different amount of time per branch and condition. The condition time should be summed and added to the current if to find the total execution time of that if, then the max should be taken.

4) A bounded loop with a break may exist, then the worst case run time is if the break never triggers. Similar with continue.

5) Same as for loops, though iterators usually don't use control flow.

6) Same.

## Subgoal 2)

### Compilation Products

In the specific RP2040 based architecture, where core0 runs the bittide network logic, and core1 runs the user logic, the compilation product of surf will be Arm0+ assembly code that can be flashed to RAM, where core1 can start execution. This code will be _different_ for _each_ node. An efficient way to flash this code has to exist, and may use the running bittide network.

The amount of cycles that it takes to send a bittide word may be configurable, in that case, it can be a compile product as well, where instead of the programmer taking into account a fixed number and optimizing their program for that, the compiler will produce the value by taking the max of all state machine branches in the program. If changing this value messes too much with the control stability of the network this is not feasible.

### Assigning Compositions of State Machines to a Bittide Topology

There's an algebra to the graph of state machines that'll be produced by a surf program, specifically when for each state machine and composition of state machines the execution time is known, and this algebra can be used to simplify the graph. There exist many paths of simplification and there should be a way to quantify how good a fit the current graph is for the given bittide topology. This sounds a lot like FPGA logic synthesis and its mapping to LUTs.

## Subgoal 3)

Since it matters a lot for program throughput how many cycles each state of a state machine takes, tools should exist that evaluate that. As shown in [Enforcing Constant Time Execution](#enforcing-constant-execution-time), it is possible to define a subset of Rust for which an upper bound of the amount of cycles taken is computable. An expansion of Rust analyzer might be feasible which shows runtime statistics of each branch in a statemachine definition on save.


## Implementation

First implement an unsafe version of the language. This should include:

1) Ergonomics to make creating and arranging state machines easy. This can be provided by a proc macro that takes care of the boiler plate and serves as a platform for checks on state machine branches later on.

2) Compilation of the given arrangements to a set of binaries for Arm0+. The arrangement methods of before will result in code text that can be compiled by the rust compiler. The code text will include a library with _all_ state machine definitions, and a set of `main.rs` files (one for each node) instantiating and invoking the state machines as defined in the arrangement.

3) Simulation of the arrangements. An arrangement of state machines should result in regular x86 compilable rust code as well (as long as no peripherals of the RP2040 are used, but a sim/compatibility layer could be engineered)


## The FPGA / Digital Hardware Analogy