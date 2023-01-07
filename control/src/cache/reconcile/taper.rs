#[allow(unused_imports)]
use micromath::F32Ext;

const LOG: [f32; 22] = [
    0.0,
    0.005,
    0.019_996_643,
    0.040_958_643,
    0.062_983_93,
    0.086_186_11,
    0.110_698_28,
    0.136_677_15,
    0.164_309_44,
    0.19382,
    0.225_483,
    0.259_637_3,
    0.296_708_64,
    0.337_242_2,
    0.381_951_87,
    0.431_798_22,
    0.488_116_62,
    0.552_841_96,
    0.628_932_1,
    0.721_246_36,
    0.838_632,
    1.0,
];

pub fn log(position: f32) -> f32 {
    if position < 0.0 {
        return 0.0;
    } else if position > 1.0 {
        return 1.0;
    }

    let array_position = position * (LOG.len() - 1) as f32;
    let index_a = array_position as usize;
    let index_b = (array_position as usize + 1).min(LOG.len() - 1);
    let remainder = array_position.fract();

    let value = LOG[index_a];
    let delta_to_next = LOG[index_b] - LOG[index_a];

    value + delta_to_next * remainder
}

#[allow(unused)]
pub fn reverse_log(position: f32) -> f32 {
    1.0 - log(1.0 - position)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_taper_below_zero() {
        assert_relative_eq!(log(-1.0), 0.0);
    }

    #[test]
    fn log_taper_above_one() {
        assert_relative_eq!(log(2.0), 1.0);
    }

    #[test]
    fn log_taper_within_limits() {
        assert_relative_eq!(log(0.0), 0.0);
        assert_relative_eq!(log(0.025), 0.002_625_000_2);
        assert_relative_eq!(log(0.05), 0.005_749_833_3);
        assert_relative_eq!(log(0.1), 0.022_092_845);
        assert_relative_eq!(log(0.3), 0.118_491_95);
        assert_relative_eq!(log(0.5), 0.242_560_15);
        assert_relative_eq!(log(0.7), 0.416_844_3);
        assert_relative_eq!(log(0.9), 0.712_014_9);
        assert_relative_eq!(log(1.0), 1.0);
    }

    #[test]
    fn reverse_log_taper_below_zero() {
        assert_relative_eq!(reverse_log(-1.0), 0.0);
    }

    #[test]
    fn reverse_log_taper_above_one() {
        assert_relative_eq!(reverse_log(2.0), 1.0);
    }

    #[test]
    fn reverse_log_taper_within_limits() {
        assert_relative_eq!(reverse_log(0.0), 0.0);
        assert_relative_eq!(reverse_log(0.1), 0.287_985_1);
        assert_relative_eq!(reverse_log(0.3), 0.583_155_7);
        assert_relative_eq!(reverse_log(0.5), 0.757_439_85);
        assert_relative_eq!(reverse_log(0.7), 0.881_508_05);
        assert_relative_eq!(reverse_log(0.9), 0.977_907_1);
        assert_relative_eq!(reverse_log(0.95), 0.994_250_2);
        assert_relative_eq!(reverse_log(0.975), 0.997_375);
        assert_relative_eq!(reverse_log(1.0), 1.0);
    }
}
