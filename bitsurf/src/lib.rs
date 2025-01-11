#![no_std]
use core::ops::Mul;

pub trait Component
where
    Self: Sized,
{
    type Input;
    type Output;
    type Intermediate;

    fn work(&self) -> usize;
    fn compute(&mut self, input: Self::Input) -> Self::Output;
    fn split(&self) -> Option<(impl Component, impl Component)>;
}

pub enum Never {}
impl Component for Never {
    type Input = ();
    type Output = ();
    type Intermediate = ();

    fn work(&self) -> usize {
        0
    }

    fn compute(&mut self, input: Self::Input) -> Self::Output {
        panic!("This should never happen")
    }

    fn split(&self) -> Option<(impl Component, impl Component)> {
        Option::<(Self, Self)>::None
    }
}

pub struct Multiply<T>(T);

impl<T> Component for Multiply<T>
where
    T: Mul<T, Output = T> + Clone + Copy,
{
    type Input = T;
    type Output = T;
    type Intermediate = ();

    fn work(&self) -> usize {
        1
    }

    fn compute(&mut self, input: T) -> T {
        input * self.0
    }

    fn split(&self) -> Option<(impl Component, impl Component)> {
        Option::<(Never, Never)>::None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_multiply() {
        let mut multiplier = Multiply(5);
        assert_eq!(multiplier.compute(3), 15);
        assert_eq!(multiplier.work(), 1);
        assert!(multiplier.split().is_none());
    }
}
