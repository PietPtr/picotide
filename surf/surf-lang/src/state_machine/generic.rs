use std::marker::PhantomData;

use super::StateMachine;

#[derive(Debug, Default)]
pub struct UnitStateMachine<T> {
    _marker: PhantomData<T>,
}

impl<T> UnitStateMachine<T> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData {},
        }
    }
}

impl<I> StateMachine<I, I> for UnitStateMachine<I> {
    fn next(&mut self, input: I) -> I {
        input
    }
}

pub struct LinearComposition<SM1, I, B, SM2, O>
where
    SM1: StateMachine<I, B>,
    SM2: StateMachine<B, O>,
{
    left: SM1,
    right: SM2,
    _marker: PhantomData<(I, B, O)>,
}

impl<SM1, I, B, SM2, O> LinearComposition<SM1, I, B, SM2, O>
where
    SM1: StateMachine<I, B>,
    SM2: StateMachine<B, O>,
{
    pub fn new(left: SM1, right: SM2) -> Self {
        Self {
            left,
            right,
            _marker: PhantomData {},
        }
    }
}

impl<SM1, I, B, SM2, O> StateMachine<I, O> for LinearComposition<SM1, I, B, SM2, O>
where
    SM1: StateMachine<I, B>,
    SM2: StateMachine<B, O>,
{
    fn next(&mut self, input: I) -> O {
        let between = self.left.next(input);
        self.right.next(between)
    }
}

pub struct ParallelComposition<SM1, I1, O1, SM2, I2, O2>
where
    SM1: StateMachine<I1, O1>,
    SM2: StateMachine<I2, O2>,
{
    upper: SM1,
    lower: SM2,
    _marker: PhantomData<(I1, O1, I2, O2)>,
}

impl<SM1, I1, O1, SM2, I2, O2> ParallelComposition<SM1, I1, O1, SM2, I2, O2>
where
    SM1: StateMachine<I1, O1>,
    SM2: StateMachine<I2, O2>,
{
    pub fn new(upper: SM1, lower: SM2) -> Self {
        Self {
            upper,
            lower,
            _marker: PhantomData {},
        }
    }
}

impl<SM1, I1, O1, SM2, I2, O2> StateMachine<(I1, I2), (O1, O2)>
    for ParallelComposition<SM1, I1, O1, SM2, I2, O2>
where
    SM1: StateMachine<I1, O1>,
    SM2: StateMachine<I2, O2>,
{
    fn next(&mut self, input: (I1, I2)) -> (O1, O2) {
        let o1 = self.upper.next(input.0);
        let o2 = self.lower.next(input.1);
        (o1, o2)
    }
}

#[derive(Debug, Default)]
pub struct Swap<I1, I2> {
    _marker: PhantomData<(I1, I2)>,
}

impl<I1, I2> Swap<I1, I2> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<I1, I2> StateMachine<(I1, I2), (I2, I1)> for Swap<I1, I2> {
    fn next(&mut self, (i1, i2): (I1, I2)) -> (I2, I1) {
        (i2, i1)
    }
}

#[derive(Debug, Default)]
pub struct Flatten3<I1, I2, I3> {
    _marker: PhantomData<(I1, I2, I3)>,
}

impl<I1, I2, I3> Flatten3<I1, I2, I3> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<I1, I2, I3> StateMachine<((I1, I2), I3), (I1, I2, I3)> for Flatten3<I1, I2, I3> {
    fn next(&mut self, ((i1, i2), i3): ((I1, I2), I3)) -> (I1, I2, I3) {
        (i1, i2, i3)
    }
}

impl<I1, I2, I3> StateMachine<(I1, (I2, I3)), (I1, I2, I3)> for Flatten3<I1, I2, I3> {
    fn next(&mut self, (i1, (i2, i3)): (I1, (I2, I3))) -> (I1, I2, I3) {
        (i1, i2, i3)
    }
}
