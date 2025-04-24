use controllers::{
    controller::FrequencyController,
    pid::PidSettings,
    si5351::{Si5351, Si5351Controller},
};
use fixed::types::I16F16;

pub struct SimulatedLink<
    const CYCLES_PER_TRANSFER: usize,
    const BUFFER_SIZE: usize,
    C: SimulatedClockGenerator,
> {
    left_buffer_level: f64,
    left_clock_generator: C,
    right_buffer_level: f64,
    right_clock_generator: C,
}

pub enum SimulatedLinkError {
    BufferFull,
    BufferEmpty,
}

impl<const CYCLES_PER_TRANSFER: usize, const BUFFER_SIZE: usize, C: SimulatedClockGenerator>
    SimulatedLink<CYCLES_PER_TRANSFER, BUFFER_SIZE, C>
{
    /// Updates the internal state based on the frequency and returns the buffer levels
    /// on both sides of the link.
    /// TODO: ignores inflight items, all transfers are currently instantaneous
    pub fn update(&mut self, dt: f64) -> Result<(usize, usize), SimulatedLinkError> {
        let left_freq = self.left_clock_generator.frequency();
        let right_freq = self.right_clock_generator.frequency();

        // determine how many cycles have passed in dt seconds
        let left_cycles = dt / (1. / left_freq);
        let right_cycles = dt / (1. / right_freq);

        // determine how many items have therefore left both buffers, and therefore joined both buffers
        let left_items_left = left_cycles / CYCLES_PER_TRANSFER as f64;
        let right_items_left = right_cycles / CYCLES_PER_TRANSFER as f64;

        // that's the new buffer size, round down, clamp to maximum / minimum
        self.left_buffer_level += right_items_left;
        self.right_buffer_level += left_items_left;

        if self.left_buffer_level > BUFFER_SIZE as f64
            || self.right_buffer_level > BUFFER_SIZE as f64
        {
            Err(SimulatedLinkError::BufferFull)
        } else if self.left_buffer_level <= 0. || self.right_buffer_level <= 0. {
            Err(SimulatedLinkError::BufferEmpty)
        } else {
            Ok((
                (self.left_buffer_level as usize).clamp(0, BUFFER_SIZE),
                (self.right_buffer_level as usize).clamp(0, BUFFER_SIZE),
            ))
        }
    }
}

pub trait SimulatedClockGenerator {
    fn frequency(&mut self) -> f64;
}

pub struct MockedSi5351 {}

impl Si5351 for MockedSi5351 {
    type Error = ();

    fn set_pll_frac(&mut self, frac: u32) -> Result<(), Self::Error> {
        log::info!("set pll frac {}", frac);
        Ok(())
    }
}

impl SimulatedClockGenerator for MockedSi5351 {
    fn frequency(&mut self) -> f64 {
        implement this
        todo!()
    }
}

#[test]
fn test_si5351_controller() {
    use tracing_subscriber::EnvFilter;

    let log_level =
        EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("error,controllers=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .without_time()
        .init();

    const KP: I16F16 = I16F16::unwrapped_from_str("0.001");
    const KD: I16F16 = I16F16::unwrapped_from_str("0.0001");
    const KI: I16F16 = I16F16::unwrapped_from_str("0.00002");

    log::info!("Kp {:?} \nKd {:?} \nKi {:?}", KP, KD, KI,);

    let si = MockedSi5351 {};

    let mut frequency_controller = Si5351Controller::new(
        si,
        4,
        PidSettings {
            kp: KP,
            ki: KD,
            kd: KI,
        },
    );

    for _ in 0..100 {
        <Si5351Controller<MockedSi5351> as FrequencyController<4>>::run(
            &mut frequency_controller,
            &[27, 32, 32, 32],
        )
        .ok()
        .unwrap();
    }
}
