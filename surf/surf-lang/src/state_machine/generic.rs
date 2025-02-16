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
    fn transition(&mut self, input: I) -> I {
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
    fn transition(&mut self, input: I) -> O {
        let between = self.left.transition(input);
        self.right.transition(between)
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
    fn transition(&mut self, input: (I1, I2)) -> (O1, O2) {
        let o1 = self.upper.transition(input.0);
        let o2 = self.lower.transition(input.1);
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
    fn transition(&mut self, (i1, i2): (I1, I2)) -> (I2, I1) {
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
    fn transition(&mut self, ((i1, i2), i3): ((I1, I2), I3)) -> (I1, I2, I3) {
        (i1, i2, i3)
    }
}

impl<I1, I2, I3> StateMachine<(I1, I2, I3), ((I1, I2), I3)> for Flatten3<I1, I2, I3> {
    fn transition(&mut self, (i1, i2, i3): (I1, I2, I3)) -> ((I1, I2), I3) {
        ((i1, i2), i3)
    }
}

impl<I1, I2, I3> StateMachine<(I1, (I2, I3)), (I1, I2, I3)> for Flatten3<I1, I2, I3> {
    fn transition(&mut self, (i1, (i2, i3)): (I1, (I2, I3))) -> (I1, I2, I3) {
        (i1, i2, i3)
    }
}

impl<I1, I2, I3> StateMachine<(I1, I2, I3), (I1, (I2, I3))> for Flatten3<I1, I2, I3> {
    fn transition(&mut self, (i1, i2, i3): (I1, I2, I3)) -> (I1, (I2, I3)) {
        (i1, (i2, i3))
    }
}

// TODO: srictly speaking not all the impls are necessary if you make the right compositions with flatten3
#[derive(Debug, Default)]
pub struct Flatten4<I1, I2, I3, I4> {
    _marker: PhantomData<(I1, I2, I3, I4)>,
}

impl<I1, I2, I3, I4> Flatten4<I1, I2, I3, I4> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<I1, I2, I3, I4> StateMachine<(((I1, I2), I3), I4), (I1, I2, I3, I4)>
    for Flatten4<I1, I2, I3, I4>
{
    fn transition(&mut self, (((i1, i2), i3), i4): (((I1, I2), I3), I4)) -> (I1, I2, I3, I4) {
        (i1, i2, i3, i4)
    }
}

impl<I1, I2, I3, I4> StateMachine<(I1, (I2, I3), I4), (I1, I2, I3, I4)>
    for Flatten4<I1, I2, I3, I4>
{
    fn transition(&mut self, (i1, (i2, i3), i4): (I1, (I2, I3), I4)) -> (I1, I2, I3, I4) {
        (i1, i2, i3, i4)
    }
}

impl<I1, I2, I3, I4> StateMachine<((I1, I2), (I3, I4)), (I1, I2, I3, I4)>
    for Flatten4<I1, I2, I3, I4>
{
    fn transition(&mut self, ((i1, i2), (i3, i4)): ((I1, I2), (I3, I4))) -> (I1, I2, I3, I4) {
        (i1, i2, i3, i4)
    }
}

impl<I1, I2, I3, I4> StateMachine<(I1, (I2, (I3, I4))), (I1, I2, I3, I4)>
    for Flatten4<I1, I2, I3, I4>
{
    fn transition(&mut self, (i1, (i2, (i3, i4))): (I1, (I2, (I3, I4)))) -> (I1, I2, I3, I4) {
        (i1, i2, i3, i4)
    }
}

impl<I1, I2, I3, I4> StateMachine<(I1, (I2, I3, I4)), (I1, I2, I3, I4)>
    for Flatten4<I1, I2, I3, I4>
{
    fn transition(&mut self, (i1, (i2, i3, i4)): (I1, (I2, I3, I4))) -> (I1, I2, I3, I4) {
        (i1, i2, i3, i4)
    }
}

impl<I1, I2, I3, I4> StateMachine<((I1, I2, I3), I4), (I1, I2, I3, I4)>
    for Flatten4<I1, I2, I3, I4>
{
    fn transition(&mut self, ((i1, i2, i3), i4): ((I1, I2, I3), I4)) -> (I1, I2, I3, I4) {
        (i1, i2, i3, i4)
    }
}

impl<I1, I2, I3, I4> StateMachine<(I1, I2, I3, I4), (((I1, I2), I3), I4)>
    for Flatten4<I1, I2, I3, I4>
{
    fn transition(&mut self, (i1, i2, i3, i4): (I1, I2, I3, I4)) -> (((I1, I2), I3), I4) {
        (((i1, i2), i3), i4)
    }
}

impl<I1, I2, I3, I4> StateMachine<(I1, I2, I3, I4), (I1, (I2, I3), I4)>
    for Flatten4<I1, I2, I3, I4>
{
    fn transition(&mut self, (i1, i2, i3, i4): (I1, I2, I3, I4)) -> (I1, (I2, I3), I4) {
        (i1, (i2, i3), i4)
    }
}

impl<I1, I2, I3, I4> StateMachine<(I1, I2, I3, I4), ((I1, I2), (I3, I4))>
    for Flatten4<I1, I2, I3, I4>
{
    fn transition(&mut self, (i1, i2, i3, i4): (I1, I2, I3, I4)) -> ((I1, I2), (I3, I4)) {
        ((i1, i2), (i3, i4))
    }
}

impl<I1, I2, I3, I4> StateMachine<(I1, I2, I3, I4), (I1, (I2, (I3, I4)))>
    for Flatten4<I1, I2, I3, I4>
{
    fn transition(&mut self, (i1, i2, i3, i4): (I1, I2, I3, I4)) -> (I1, (I2, (I3, I4))) {
        (i1, (i2, (i3, i4)))
    }
}

impl<I1, I2, I3, I4> StateMachine<(I1, I2, I3, I4), (I1, (I2, I3, I4))>
    for Flatten4<I1, I2, I3, I4>
{
    fn transition(&mut self, (i1, i2, i3, i4): (I1, I2, I3, I4)) -> (I1, (I2, I3, I4)) {
        (i1, (i2, i3, i4))
    }
}

impl<I1, I2, I3, I4> StateMachine<(I1, I2, I3, I4), ((I1, I2, I3), I4)>
    for Flatten4<I1, I2, I3, I4>
{
    fn transition(&mut self, (i1, i2, i3, i4): (I1, I2, I3, I4)) -> ((I1, I2, I3), I4) {
        ((i1, i2, i3), i4)
    }
}
