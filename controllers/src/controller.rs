/// Trait for frequency Controllers generic over:
/// B: the elastic buffer size.
/// The controller should have access to the resources that control frequency.
pub trait FrequencyController<const B: usize> {
    /// Run the frequency control algorithm. This is called at a set interval (every N cycles)
    /// Therefore, run must always take fewer than N cycles.
    fn run(&mut self, buffer_levels: &[usize]);
    /// Change the amount of neighboring nodes that the controller should assume.
    fn set_degree(&mut self, new_degree: usize);
}
