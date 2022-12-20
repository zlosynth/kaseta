#[allow(unused_imports)]
use micromath::F32Ext as _;

use crate::random::Random;
use crate::trigonometry;

const BASE_FREQUENCY: f32 = 6.0;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub depth: f32,
    // Assuming 48 kHz sample rate and buffers of 32 samples,
    // the dice should be thrown
    //   48000 / 32 = 1500 times a second.
    // With chance of X, successful throw is one in
    //   1 / X.
    // Chance to trigger pops within one second is X * 1500.
    pub chance: f32,
}

// TODO: Based on intensity, start with pulsing, then enable fully.
// TODO: On the very end it would start getting deeper
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Flutter {
    sample_rate: f32,
    depth: f32,
    chance: f32,
    pops: Option<Pops>,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Pops {
    amount: u8,
    phase: f32,
    slowdowns: [f32; 3],
    // TODO: Intensity
}

impl Flutter {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: sample_rate as f32,
            depth: 0.0,
            chance: 0.0,
            pops: None,
        }
    }

    pub fn roll_dice(&mut self, random: &mut impl Random) {
        if self.pops.is_some() {
            return;
        }

        let rand = random.normal();
        let amount = (rand - (1.0 - self.chance)) / self.chance;
        self.pops = if amount < 0.0 {
            None
        } else if amount < 0.7 {
            Some(Pops::with_amount(1))
        } else if amount < 0.9 {
            Some(Pops::with_amount(2))
        } else {
            Some(Pops::with_amount(3))
        };
    }

    pub fn pop(&mut self) -> f32 {
        let (x, pops) = if let Some(pops) = self.pops.take() {
            let x = pops.friction();
            let maybe_pops = pops.tick(self.sample_rate);
            (x * self.depth, maybe_pops)
        } else {
            (0.0, None)
        };

        self.pops = pops;

        x
    }

    pub fn set_attributes(&mut self, attributes: &Attributes) {
        self.depth = attributes.depth;
        self.chance = attributes.chance;
    }
}

impl Pops {
    fn with_amount(amount: u8) -> Self {
        Pops {
            amount,
            phase: 0.0,
            slowdowns: [0.25, 0.5, 0.4],
        }
    }

    /// Simulate flutter friction.
    ///
    /// Sinusoidal slowdown until it stops. Then after it overcomes the static friction
    /// it launches with linear speed.
    fn friction(&self) -> f32 {
        let phase = self.phase.fract();
        let breaking = phase < 0.5;
        if breaking {
            (trigonometry::cos(phase + 0.5) + 1.0) / 2.0
        } else {
            let catching_up_phase = (phase - 0.5) * 2.0;
            1.0 - catching_up_phase
        }
    }

    /// Increase phase based on the current position.
    ///
    /// While breaking, it would apply slowdown coefficient. It would go in
    /// full speed while catching back up.
    fn tick(mut self, sample_rate: f32) -> Option<Self> {
        let breaking = self.phase.fract() < 0.5;
        let slowdown_coefficient = if breaking {
            self.slowdowns[self.phase as usize]
        } else {
            1.0
        };
        self.phase += BASE_FREQUENCY / sample_rate * slowdown_coefficient;

        if self.phase > self.amount as f32 {
            None
        } else {
            Some(self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pops_friction_is_continuous() {
        let mut pops = Pops {
            amount: 3,
            phase: 0.0,
            slowdowns: [0.25, 0.5, 1.0],
        };
        let mut last_friction = 0.0;
        loop {
            let new_friction = pops.friction();
            let eq = relative_eq!(new_friction, last_friction, epsilon = 0.01);
            assert!(
                eq,
                "new_friction({new_friction}) != last_friction({last_friction}) for phase({})",
                pops.phase
            );
            if let Some(new_pops) = pops.tick(48_000.0) {
                last_friction = new_friction;
                pops = new_pops;
            } else {
                break;
            }
        }
    }
}
