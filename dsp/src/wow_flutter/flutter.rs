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
    phase: f32,
    amount: usize,
    pops: [Option<Pop>; 3],
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Pop {
    intensity: f32,
    slowdown: f32,
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
        let amount_rng = (rand - (1.0 - self.chance)) / self.chance;
        self.pops = if amount_rng < 0.0 {
            None
        } else {
            let amount = random_amount_of_clicks(amount_rng);

            let mut pops = [None, None, None];

            for pop in pops.iter_mut().take(amount) {
                let intensity = random_intensity(random.normal());
                let slowdown = random_slowdown(random.normal());
                *pop = Some(Pop {
                    intensity,
                    slowdown,
                });
            }

            Some(Pops {
                phase: 0.0,
                amount,
                pops,
            })
        }
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

fn random_amount_of_clicks(normal: f32) -> usize {
    if normal < 0.7 {
        1
    } else if normal < 0.9 {
        2
    } else {
        3
    }
}

fn random_intensity(normal: f32) -> f32 {
    const START: f32 = 0.2;
    const MIDDLE: f32 = 0.4;
    const STOP: f32 = 1.0;
    if normal < 0.9 {
        START + (normal / 0.9) * (MIDDLE - START)
    } else {
        MIDDLE + ((normal - 0.9) / 0.1) * (STOP - MIDDLE)
    }
}

fn random_slowdown(normal: f32) -> f32 {
    0.01 + normal * 0.99
}

impl Pops {
    /// Simulate flutter friction.
    ///
    /// Sinusoidal slowdown until it stops. Then after it overcomes the static friction
    /// it launches with linear speed.
    fn friction(&self) -> f32 {
        let phase = self.phase.fract();

        let intensity = self.pops[(self.phase * 0.96) as usize]
            .as_ref()
            .unwrap()
            .intensity;

        let breaking = phase < 0.5;
        let offset = if breaking {
            (trigonometry::cos(phase + 0.5) + 1.0) / 2.0
        } else {
            let catching_up_phase = (phase - 0.5) * 2.0;
            1.0 - catching_up_phase
        };

        offset * intensity
    }

    /// Increase phase based on the current position.
    ///
    /// While breaking, it would apply slowdown coefficient. It would go in
    /// full speed while catching back up.
    fn tick(mut self, sample_rate: f32) -> Option<Self> {
        let breaking = self.phase.fract() < 0.5;
        let slowdown_coefficient = if breaking {
            self.pops[(self.phase * 0.95) as usize]
                .as_ref()
                .unwrap()
                .slowdown
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
            phase: 0.0,
            amount: 3,
            pops: [
                Some(Pop {
                    slowdown: 0.25,
                    intensity: 1.0,
                }),
                Some(Pop {
                    slowdown: 0.5,
                    intensity: 0.3,
                }),
                Some(Pop {
                    slowdown: 1.0,
                    intensity: 0.8,
                }),
            ],
        };
        let mut last_friction = 0.0;
        loop {
            let new_friction = pops.friction();
            let eq = relative_eq!(new_friction, last_friction, epsilon = 0.04);
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
