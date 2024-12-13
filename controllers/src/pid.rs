use core::ops::{Add, AddAssign, Mul, Sub};

pub struct PidSettings<F> {
    pub kp: F,
    pub ki: F,
    pub kd: F,
}

pub struct PidControl<F> {
    k: PidSettings<F>,
    previous_error: F,
    integral: F,
}

impl<F> PidControl<F>
where
    F: Add<Output = F> + Sub<Output = F> + Mul<Output = F> + AddAssign + Default + Copy,
{
    pub fn new(k: PidSettings<F>) -> Self {
        Self {
            k,
            previous_error: F::default(),
            integral: F::default(),
        }
    }

    pub fn run(&mut self, setpoint: F, measurement: F) -> F {
        let error = setpoint - measurement;

        self.integral += error;
        let derivative = error - self.previous_error;

        let output = self.k.kp * error + self.k.ki * self.integral + self.k.kd * derivative;

        self.previous_error = error;

        output
    }
}
