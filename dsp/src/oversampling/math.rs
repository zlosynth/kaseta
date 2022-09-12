pub fn upper_power_of_two(mut n: usize) -> usize {
    if n == 0 {
        return 0;
    }

    n -= 1;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    n += 1;
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_as_expected() {
        assert_eq!(upper_power_of_two(0), 0);
        assert_eq!(upper_power_of_two(1), 1);
        assert_eq!(upper_power_of_two(3), 4);
        assert_eq!(upper_power_of_two(800), 1024);
    }
}
