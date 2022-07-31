//! This module contains basic building blocks for hysteresis simulation.
//!
//! Kudos to Jatin Chowdhury:
//!
//! * <https://jatinchowdhury18.medium.com/complex-nonlinearities-episode-3-hysteresis-fdeb2cd3e3f6>
//! * <https://dafx2019.bcu.ac.uk/papers/DAFx2019_paper_3.pdf>
//! * <https://ccrma.stanford.edu/~jatin/papers/Complex_NLs.pdf>
//! * <https://github.com/jatinchowdhury18/audio_dspy>

use libm::{fabs, sqrt, tanh};

/// Time domain differentiation using the trapezoidal rule.
#[derive(Debug)]
struct Differentiator {
    /// Period between samples
    t: f64,
    /// Previous sample
    x_n1: f64,
    /// Time derivative of previous sample
    x_d_n1: f64,
}

impl Differentiator {
    pub fn new(fs: f64) -> Self {
        Self {
            t: 1.0 / fs,
            x_n1: 0.0,
            x_d_n1: 0.0,
        }
    }

    pub fn differentiate(&mut self, x: f64) -> f64 {
        const D_ALPHA: f64 = 0.75;
        let x_d = (((1.0 + D_ALPHA) / self.t) * (x - self.x_n1)) - D_ALPHA * self.x_d_n1;
        self.x_n1 = x;
        self.x_d_n1 = x_d;
        x_d
    }
}

/// Langevin function: coth(x) - (1/x)
fn langevin(x: f64) -> f64 {
    if fabs(x) > 0.001 {
        1.0 / tanh(x) - 1.0 / x
    } else {
        x / 3.0
    }
}

/// Derivative of the Langevin function: (1/x^2) - coth(x)^2 + 1
fn langevin_deriv(x: f64) -> f64 {
    if fabs(x) > 0.001 {
        1.0 / (x * x) - (1.0 / (tanh(x) * tanh(x))) + 1.0
    } else {
        1.0 / 3.0
    }
}

/// Applying hysteresis on input signal.
#[derive(Debug)]
pub struct Hysteresis {
    /// Drive level
    drive: f32,
    /// Saturation level
    saturation: f32,
    /// Width level
    width: f32,

    differentiator: Differentiator,
    /// Period between samples
    t: f64,
    /// Magnetisation saturation
    m_s: f64,
    /// Anhysteric magnetisation shape
    a: f64,
    /// Initial susceptibilities
    c: f64,

    /// Previous magnetisation
    m_n1: f64,
    /// Previous magnetic field
    h_n1: f64,
    /// Time derivative of the previous magnetic field
    h_d_n1: f64,
}

impl Hysteresis {
    /// Hysteresis loop width / coercivity
    const K: f64 = 0.47875;

    /// Mean field parameter.
    const ALPHA: f64 = 1.6e-3;

    pub fn new(fs: f32, drive: f32, saturation: f32, width: f32) -> Self {
        let fs = fs as f64;
        let mut hysteresis = Self {
            drive: 0.0,
            saturation: 0.0,
            width: 0.0,

            differentiator: Differentiator::new(fs),
            t: 1.0 / fs,
            m_s: 0.0,
            a: 0.0,
            c: 0.0,

            m_n1: 0.0,
            h_n1: 0.0,
            h_d_n1: 0.0,
        };
        hysteresis.set_drive(drive);
        hysteresis.set_saturation(saturation);
        hysteresis.set_width(width);
        hysteresis
    }

    pub fn set_drive(&mut self, drive: f32) {
        self.drive = drive;
        self.a = self.m_s / (0.01 + 6.0 * drive as f64);
    }

    pub fn drive(&self) -> f32 {
        self.drive
    }

    pub fn set_saturation(&mut self, saturation: f32) {
        self.saturation = saturation;
        self.m_s = 0.5 + 1.5 * (1.0 - saturation as f64);
        self.set_drive(self.drive);
    }

    pub fn saturation(&self) -> f32 {
        self.saturation
    }

    pub fn set_width(&mut self, width: f32) {
        self.width = width;
        self.c = sqrt(1.0 - width as f64) - 0.01;
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    /// Jiles-Atherton differential equation.
    ///
    /// # Parameters
    ///
    /// * `m`: Magnetisation
    /// * `h`: Magnetic field
    /// * `h_d`: Time derivative of magnetic field
    ///
    /// # Returns
    ///
    /// Derivative of magnetisation w.r.t time
    fn dmdt(&self, m: f64, h: f64, h_d: f64) -> f64 {
        let q = (h + Self::ALPHA * m) / self.a;
        let m_diff = self.m_s * langevin(q) - m;

        let delta_s = if h_d > 0.0 { 1.0 } else { -1.0 };

        let delta_m = if f64::is_sign_positive(delta_s) == f64::is_sign_positive(m_diff) {
            1.0
        } else {
            0.0
        };

        let l_prime = langevin_deriv(q);

        let c_diff = 1.0 - self.c;
        let t1_numerator = c_diff * delta_m * m_diff;
        let t1_denominator = c_diff * delta_s * Self::K - Self::ALPHA * m_diff;
        let t1 = (t1_numerator / t1_denominator) * h_d;

        let t2 = self.c * (self.m_s / self.a) * h_d * l_prime;

        let numerator = t1 + t2;
        let denominator = 1.0 - self.c * Self::ALPHA * (self.m_s / self.a) * l_prime;

        numerator / denominator
    }

    /// Compute hysteresis function with Runge-Kutta 4th order.
    ///
    /// # Parameters
    ///
    /// * `m_n1`: Previous magnetisation
    /// * `h`: Magnetic field
    /// * `h_n1`: Previous magnetic field
    /// * `h_d`: Magnetic field derivative
    /// * `h_d_n1`: Previous magnetic field derivative
    ///
    /// # Returns
    ///
    /// Current magnetisation
    fn rk4(&self, m_n1: f64, h: f64, h_n1: f64, h_d: f64, h_d_n1: f64) -> f64 {
        let k1 = self.t * self.dmdt(m_n1, h_n1, h_d_n1);
        let k2 = self.t * self.dmdt(m_n1 + k1 / 2.0, (h + h_n1) / 2.0, (h_d + h_d_n1) / 2.0);
        let k3 = self.t * self.dmdt(m_n1 + k2 / 2.0, (h + h_n1) / 2.0, (h_d + h_d_n1) / 2.0);
        let k4 = self.t * self.dmdt(m_n1 + k3, h, h_d);
        m_n1 + (k1 / 6.0) + (k2 / 3.0) + (k3 / 3.0) + (k4 / 6.0)
    }

    pub fn process(&mut self, h: f32) -> f32 {
        let h = h as f64;
        let (h_d, m) = {
            let h_d = self.differentiator.differentiate(h);
            let m = self.rk4(self.m_n1, h, self.h_n1, h_d, self.h_d_n1);

            const UPPER_LIMIT: f64 = 20.0;
            if m > UPPER_LIMIT {
                (0.0, 0.0)
            } else {
                (h_d, m)
            }
        };

        self.m_n1 = m;
        self.h_n1 = h;
        self.h_d_n1 = h_d;
        m as f32
    }
}
