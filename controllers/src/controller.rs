/// Trait for frequency Controllers generic over:
/// N: the amount of nodes connected to the controller
/// B: the elastic buffer size.
/// The controller should have access to the resources that control frequency.
pub trait FrequencyController<const N: usize, const B: usize> {
    fn run(&mut self, buffer_levels: &[usize; N]);
}
