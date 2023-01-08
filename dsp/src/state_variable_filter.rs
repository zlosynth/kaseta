//! State variable filter can be used as low/high/band pass or band reject.

#[allow(unused_imports)]
use micromath::F32Ext as _;

use core::f32::consts::PI;

/// Yields filtered signal.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct StateVariableFilter {
    sample_rate: u32,
    f: f32,
    q: f32,
    delay_1: f32,
    delay_2: f32,
}

impl StateVariableFilter {
    pub fn new(sample_rate: u32) -> Self {
        let mut filter = Self {
            sample_rate,
            f: 0.0,
            q: 0.0,
            delay_1: 0.0,
            delay_2: 0.0,
        };
        filter.set_q_factor(0.7);
        filter.set_frequency(0.0);
        filter
    }

    pub fn set_frequency(&mut self, frequency: f32) -> &mut Self {
        self.f = 2.0 * f32::sin((PI * frequency) / self.sample_rate as f32);
        self
    }

    pub fn set_q_factor(&mut self, q_factor: f32) -> &mut Self {
        self.q = 1.0 / f32::max(q_factor, 0.5);
        self
    }

    // https://www.earlevel.com/main/2003/03/02/the-digital-state-variable-filter/
    //
    //             +----------------------------------------------------------+
    //             |                                                          |
    //             +-->[high pass]      +-->[band pass]                    [sum 4]-->[band reject]
    //             |                    |                                     |
    // -->[sum 1]--+--[mul f]--[sum 2]--+->[delay 1]--+--[mul f]--[sum 3]--+--+----+-->[low pass]
    //    - A  A -                A                   |              A     |       |
    //      |   \                 |                   |              |  [delay 2]  |
    //      |    \                +-------------------+              |     |       |
    //      |     \                                   |              +-----+       |
    //      |      \---[mut q]------------------------+                            |
    //      |                                                                      |
    //      +----------------------------------------------------------------------+
    //
    pub fn tick(&mut self, value: f32) -> Signal {
        let mut signal = Signal::default();

        let sum_3 = self.delay_1 * self.f + self.delay_2;
        let sum_1 = value - sum_3 - self.delay_1 * self.q;
        let sum_2 = sum_1 * self.f + self.delay_1;

        signal.low_pass = sum_3;
        signal.high_pass = sum_1;
        signal.band_pass = sum_2;
        signal.band_pass = sum_2;
        signal.band_reject = {
            #[allow(clippy::let_and_return)]
            let sum_4 = sum_1 + sum_3;
            sum_4
        };

        self.delay_1 = sum_2;
        self.delay_2 = sum_3;

        signal
    }
}

/// Filtered signal.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Default)]
pub struct Signal {
    pub low_pass: f32,
    pub high_pass: f32,
    pub band_pass: f32,
    pub band_reject: f32,
}
