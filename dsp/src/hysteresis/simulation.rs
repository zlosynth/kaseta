//! This module contains basic building blocks for hysteresis simulation.
//!
//! Kudos to Jatin Chowdhury:
//!
//! * <https://jatinchowdhury18.medium.com/complex-nonlinearities-episode-3-hysteresis-fdeb2cd3e3f6>
//! * <https://dafx2019.bcu.ac.uk/papers/DAFx2019_paper_3.pdf>
//! * <https://ccrma.stanford.edu/~jatin/papers/Complex_NLs.pdf>
//! * <https://github.com/jatinchowdhury18/audio_dspy>

use libm::{fabsf as fabs, sqrtf as sqrt};

/// Time domain differentiation using the trapezoidal rule.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Differentiator {
    /// Period between samples
    t: f32,
    /// Previous sample
    x_n1: f32,
    /// Time derivative of previous sample
    x_d_n1: f32,
}

impl Differentiator {
    pub fn new(fs: f32) -> Self {
        Self {
            t: 1.0 / fs,
            x_n1: 0.0,
            x_d_n1: 0.0,
        }
    }

    pub fn differentiate(&mut self, x: f32) -> f32 {
        const D_ALPHA: f32 = 0.75;
        let x_d = (((1.0 + D_ALPHA) / self.t) * (x - self.x_n1)) - D_ALPHA * self.x_d_n1;
        self.x_n1 = x;
        self.x_d_n1 = x_d;
        x_d
    }
}

/// Approximation of tanh.
fn tanh(x: f32) -> f32 {
    let x2 = x * x;
    x / (1.0 + (x2 / (3.0 + (x2 / 5.0 + (x2 / 7.0)))))
}

/// Langevin function: coth(x) - (1/x)
fn langevin(x: f32) -> f32 {
    if fabs(x) > 0.001 {
        1.0 / tanh(x) - 1.0 / x
    } else {
        x / 3.0
    }
}

/// Derivative of the Langevin function: (1/x^2) - coth(x)^2 + 1
fn langevin_deriv(x: f32) -> f32 {
    if fabs(x) > 0.001 {
        1.0 / (x * x) - (1.0 / (tanh(x) * tanh(x))) + 1.0
    } else {
        1.0 / 3.0
    }
}

/// Applying hysteresis on input signal.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Simulation {
    /// Drive level
    drive: f32,
    /// Saturation level
    saturation: f32,
    /// Width level
    width: f32,

    differentiator: Differentiator,
    /// Period between samples
    t: f32,
    /// Magnetisation saturation
    m_s: f32,
    /// Anhysteric magnetisation shape
    a: f32,
    /// Initial susceptibilities
    c: f32,

    /// Previous magnetisation
    m_n1: f32,
    /// Previous magnetic field
    h_n1: f32,
    /// Time derivative of the previous magnetic field
    h_d_n1: f32,
}

impl Simulation {
    /// Hysteresis loop width / coercivity
    const K: f32 = 0.47875;

    /// Mean field parameter.
    const ALPHA: f32 = 1.6e-3;

    #[must_use]
    pub fn new(fs: f32) -> Self {
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
        hysteresis.set_drive(0.0);
        hysteresis.set_saturation(0.0);
        hysteresis.set_width(0.0);
        hysteresis
    }

    pub fn set_drive(&mut self, drive: f32) {
        self.drive = drive;
        self.a = self.m_s / (0.01 + 6.0 * drive);
    }

    pub fn set_saturation(&mut self, saturation: f32) {
        self.saturation = saturation;
        self.m_s = 0.5 + 1.5 * (1.0 - saturation);
        self.set_drive(self.drive);
    }

    pub fn set_width(&mut self, width: f32) {
        self.width = width;
        self.c = sqrt(1.0 - width) - 0.01;
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
    fn dmdt(&self, m: f32, h: f32, h_d: f32) -> f32 {
        let q = (h + Self::ALPHA * m) / self.a;
        let m_diff = self.m_s * langevin(q) - m;

        let delta_s = if h_d > 0.0 { 1.0 } else { -1.0 };

        let delta_m = if f32::is_sign_positive(delta_s) == f32::is_sign_positive(m_diff) {
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

    // /// Compute hysteresis function with Runge-Kutta 4th order.
    // ///
    // /// # Parameters
    // ///
    // /// * `m_n1`: Previous magnetisation
    // /// * `h`: Magnetic field
    // /// * `h_n1`: Previous magnetic field
    // /// * `h_d`: Magnetic field derivative
    // /// * `h_d_n1`: Previous magnetic field derivative
    // ///
    // /// # Returns
    // ///
    // /// Current magnetisation
    // fn rk4(&self, m_n1: f32, h: f32, h_n1: f32, h_d: f32, h_d_n1: f32) -> f32 {
    //     let k1 = self.t * self.dmdt(m_n1, h_n1, h_d_n1);
    //     let k2 = self.t * self.dmdt(m_n1 + k1 / 2.0, (h + h_n1) / 2.0, (h_d + h_d_n1) / 2.0);
    //     let k3 = self.t * self.dmdt(m_n1 + k2 / 2.0, (h + h_n1) / 2.0, (h_d + h_d_n1) / 2.0);
    //     let k4 = self.t * self.dmdt(m_n1 + k3, h, h_d);
    //     m_n1 + (k1 / 6.0) + (k2 / 3.0) + (k3 / 3.0) + (k4 / 6.0)
    // }

    /// Compute hysteresis function with Runge-Kutta 2nd order.
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
    fn rk2(&self, m_n1: f32, h: f32, h_n1: f32, h_d: f32, h_d_n1: f32) -> f32 {
        let k1 = self.t * self.dmdt(m_n1, h_n1, h_d_n1);
        let k2 = self.t * self.dmdt(m_n1 + k1 / 2.0, (h + h_n1) / 2.0, (h_d + h_d_n1) / 2.0);
        m_n1 + k2
    }

    #[must_use]
    pub fn process(&mut self, h: f32) -> f32 {
        let (h_d, m) = {
            let h_d = self.differentiator.differentiate(h);
            let m = self.rk2(self.m_n1, h, self.h_n1, h_d, self.h_d_n1);

            const UPPER_LIMIT: f32 = 20.0;
            if (-UPPER_LIMIT..=UPPER_LIMIT).contains(&m) {
                (h_d, m)
            } else {
                (0.0, 0.0)
            }
        };

        self.m_n1 = m;
        self.h_n1 = h;
        self.h_d_n1 = h_d;

        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::Vec;

    #[test]
    fn given_hysteresis_when_given_simple_sine_it_adds_odd_harmonics() {
        use sirena::signal::{self, SignalTake};
        use sirena::spectral_analysis::SpectralAnalysis;

        const FS: f32 = 1024.0;
        const FREQ: f32 = 32.0;
        const SAMPLES: usize = 1024;

        let mut buffer: [f32; SAMPLES] = signal::sine(FS, FREQ)
            .take(SAMPLES)
            .collect::<Vec<_, SAMPLES>>()
            .as_slice()
            .try_into()
            .unwrap();

        const DRIVE: f32 = 0.5;
        const SATURATION: f32 = 0.5;
        const WIDTH: f32 = 0.5;
        let mut hysteresis = Simulation::new(FS);
        hysteresis.set_drive(DRIVE);
        hysteresis.set_saturation(SATURATION);
        hysteresis.set_width(WIDTH);

        for x in buffer.iter_mut() {
            *x = hysteresis.process(*x);
        }

        let analysis = SpectralAnalysis::analyze(&buffer, FS as u32);
        let harmonic_1 = analysis.magnitude(FREQ);
        let harmonic_2 = analysis.magnitude(FREQ * 2.0);
        let harmonic_3 = analysis.magnitude(FREQ * 3.0);
        let harmonic_4 = analysis.magnitude(FREQ * 4.0);
        let harmonic_5 = analysis.magnitude(FREQ * 5.0);
        let harmonic_6 = analysis.magnitude(FREQ * 6.0);
        let harmonic_7 = analysis.magnitude(FREQ * 7.0);
        let harmonic_8 = analysis.magnitude(FREQ * 8.0);
        let harmonic_9 = analysis.magnitude(FREQ * 9.0);

        assert!(harmonic_1 > harmonic_3);
        assert!(harmonic_3 > harmonic_5);
        assert!(harmonic_5 > harmonic_7);
        assert!(harmonic_7 > harmonic_9);

        assert!(harmonic_2 < harmonic_9);
        assert!(harmonic_4 < harmonic_9);
        assert!(harmonic_6 < harmonic_9);
        assert!(harmonic_8 < harmonic_9);
    }

    #[test]
    fn when_input_is_above_nyquist_given_hysteresis_when_given_noise_it_remains_stable() {
        const PRE_AMP: f32 = 20.0;
        const FS: f32 = 1024.0;
        const DRIVE: f32 = 1.0;
        const SATURATION: f32 = 1.0;
        const WIDTH: f32 = 0.0;
        let mut hysteresis = Simulation::new(FS);
        hysteresis.set_drive(DRIVE);
        hysteresis.set_saturation(SATURATION);
        hysteresis.set_width(WIDTH);

        use rand::Rng;
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let input = rng.gen_range(-PRE_AMP..PRE_AMP);
            let output = hysteresis.process(input);
            assert!(
                output > -1000.0 && output < 1000.0,
                "Hysteresis output is unstable: {output}"
            );
        }
    }
}
