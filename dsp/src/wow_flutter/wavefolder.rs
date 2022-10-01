pub fn fold(x: f32, min: f32, max: f32) -> f32 {
    let diff = max - min;
    if x < min {
        min + f32::min(min - x, diff)
    } else if x <= max {
        x
    } else {
        max - f32::min(x - max, diff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_value_within_range_it_leaves_it() {
        assert_relative_eq!(fold(0.1, -1.0, 1.0), 0.1);
    }

    #[test]
    fn given_value_close_below_min_it_wraps_it() {
        assert_relative_eq!(fold(-1.1, -1.0, 1.0), -0.9);
    }

    #[test]
    fn given_value_close_above_max_it_wraps_it() {
        assert_relative_eq!(fold(1.1, -1.0, 1.0), 0.9);
    }

    #[test]
    fn given_value_far_below_min_it_wraps_it_no_higher_than_to_the_max() {
        assert_relative_eq!(fold(-3.0, -1.0, 1.0), 1.0);
    }

    #[test]
    fn given_value_far_above_max_it_wraps_it_no_lower_than_to_the_min() {
        assert_relative_eq!(fold(3.0, -1.0, 1.0), -1.0);
    }
}
