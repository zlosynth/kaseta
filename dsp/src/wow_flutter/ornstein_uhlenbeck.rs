//! Ornstein-Uhlenbeck process, modeling brownian motion.
//!
//! Based on <https://github.com/mhampton/ZetaCarinaeModules>.

use libm::sqrtf as sqrt;

use crate::random::Random;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OrnsteinUhlenbeck {
    value: f32,
    sample_interval: f32,
    sqrt_delta: f32,
    pub noise: f32,
    pub spring: f32,
}

impl OrnsteinUhlenbeck {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            value: 0.0,
            sample_interval: 1.0 / sample_rate,
            sqrt_delta: 1.0 / sqrt(sample_rate),
            noise: 0.0,
            spring: 1.0,
        }
    }

    pub fn pop(&mut self, random: &mut impl Random) -> f32 {
        self.value += self.spring * (-self.value) * self.sample_interval;
        self.value += self.noise * random.normal() * self.sqrt_delta;
        self.value
    }
}
