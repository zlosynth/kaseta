pub enum Quantization {
    Six,
    Eight,
    Both,
}

impl From<(bool, bool)> for Quantization {
    fn from(source: (bool, bool)) -> Self {
        match source {
            (true, true) => Self::Both,
            (true, false) => Self::Six,
            (false, true) => Self::Eight,
            _ => panic!(),
        }
    }
}

// Each beat divided into 1/6 or 1/8 notes.
//
// LCM(6, 8) = 24
//
// 1/8:  |||---|||---|||---|||---
// 1/6:  ||||----||||----||||----
// both: ||--  ||  --||--  ||  -- (8)
//       ||  --  ||  --  ||  --   (6)
pub fn quantize(x: f32, quantization: Quantization) -> f32 {
    match quantization {
        Quantization::Six => {
            const STEP: f32 = 1.0 / 6.0;
            const HALF: f32 = STEP / 2.0;
            if x < STEP - HALF {
                0.0 * STEP
            } else if x < 2.0 * STEP - HALF {
                1.0 * STEP
            } else if x < 3.0 * STEP - HALF {
                2.0 * STEP
            } else if x < 4.0 * STEP - HALF {
                3.0 * STEP
            } else if x < 5.0 * STEP - HALF {
                4.0 * STEP
            } else {
                5.0 * STEP
            }
        }
        Quantization::Eight => {
            const STEP: f32 = 1.0 / 8.0;
            const HALF: f32 = STEP / 2.0;
            if x < STEP - HALF {
                0.0 * STEP
            } else if x < 2.0 * STEP - HALF {
                1.0 * STEP
            } else if x < 3.0 * STEP - HALF {
                2.0 * STEP
            } else if x < 4.0 * STEP - HALF {
                3.0 * STEP
            } else if x < 5.0 * STEP - HALF {
                4.0 * STEP
            } else if x < 6.0 * STEP - HALF {
                5.0 * STEP
            } else if x < 7.0 * STEP - HALF {
                6.0 * STEP
            } else {
                7.0 * STEP
            }
        }
        Quantization::Both => {
            const STEP: f32 = 1.0 / 12.0;
            const HALF: f32 = STEP / 2.0;
            const EIGHTH: f32 = 1.0 / 8.0;
            const SIXTH: f32 = 1.0 / 6.0;
            if x < STEP - HALF {
                0.0
            } else if x < 2.0 * STEP - HALF {
                1.0 * EIGHTH
            } else if x < 3.0 * STEP - HALF {
                1.0 * SIXTH
            } else if x < 4.0 * STEP - HALF {
                2.0 * EIGHTH
            } else if x < 5.0 * STEP - HALF {
                2.0 * SIXTH
            } else if x < 6.0 * STEP - HALF {
                3.0 * EIGHTH
            } else if x < 7.0 * STEP - HALF {
                1.0 / 2.0
            } else if x < 8.0 * STEP - HALF {
                5.0 * EIGHTH
            } else if x < 9.0 * STEP - HALF {
                4.0 * SIXTH
            } else if x < 10.0 * STEP - HALF {
                6.0 * EIGHTH
            } else if x < 11.0 * STEP - HALF {
                5.0 * SIXTH
            } else {
                7.0 * EIGHTH
            }
        }
    }
}
