use fixed::types::I16F16;

pub struct PidSettings {
    pub kp: I16F16,
    pub ki: I16F16,
    pub kd: I16F16,
}

pub struct PidControl {
    k: PidSettings,
    previous_error: I16F16,
    integral: I16F16,
}

impl PidControl {
    pub fn new(k: PidSettings) -> Self {
        Self {
            k,
            previous_error: I16F16::default(),
            integral: I16F16::default(),
        }
    }

    pub fn run(&mut self, setpoint: I16F16, measurement: I16F16) -> Option<I16F16> {
        let error = setpoint.checked_sub(measurement)?;

        self.integral = self.integral.saturating_add(error);
        let derivative = error.saturating_sub(self.previous_error);

        let proportional = self.k.kp.saturating_mul(error);
        let integral = self.k.ki.saturating_mul(self.integral);
        let derivative = self.k.kd.saturating_mul(derivative);

        let output = proportional
            .saturating_add(integral)
            .saturating_add(derivative);

        self.previous_error = error;

        Some(output)
    }
}
