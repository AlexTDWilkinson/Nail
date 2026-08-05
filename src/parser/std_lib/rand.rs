//! Random numbers for simulations, sampling and shuffles.
//!
//! Two halves that must not be confused with each other. `rand_int`,
//! `rand_float` and friends draw from a generator seeded by the operating
//! system: good enough for a dice roll, a jitter delay or picking a random
//! greeting, and never good enough for a session id. Anything an attacker must
//! not be able to guess belongs to `crypto_random_hex`.
//!
//! The `rand_seeded_*` half takes the seed as an argument, so the same seed
//! always produces the same answer. That is what makes a test that samples
//! data reproducible, and what lets a generated world be regenerated from a
//! short number instead of stored.

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

/// Shared bounds check, so every range function fails the same way.
fn check_range<T: std::fmt::Display + PartialOrd>(function: &str, min: T, max: T) -> Result<(), String> {
    if min > max {
        return Err(format!("{}: the low bound {} is above the high bound {}", function, min, max));
    }
    return Ok(());
}

/// A whole number from min to max, both ends included.
pub fn int(min: i64, max: i64) -> Result<i64, String> {
    check_range("rand_int", min, max)?;
    return Ok(rand::thread_rng().gen_range(min..=max));
}

/// A fraction from 0.0 up to but not including 1.0.
pub fn float() -> f64 {
    return rand::thread_rng().gen::<f64>();
}

/// A fraction from min up to but not including max.
pub fn float_range(min: f64, max: f64) -> Result<f64, String> {
    check_range("rand_float_range", min, max)?;
    if min == max {
        return Ok(min);
    }
    return Ok(rand::thread_rng().gen_range(min..max));
}

/// True or false with even odds.
pub fn boolean() -> bool {
    return rand::thread_rng().gen::<bool>();
}

/// True with the given probability, so `rand_chance(0.1)` is true one time in
/// ten. A probability outside 0.0 to 1.0 is an error rather than a silent
/// always or never.
pub fn chance(probability: f64) -> Result<bool, String> {
    if !(0.0..=1.0).contains(&probability) {
        return Err(format!("rand_chance: {} is not a probability between 0.0 and 1.0", probability));
    }
    return Ok(rand::thread_rng().gen::<f64>() < probability);
}

/// One element of the array, chosen evenly. Errors on an empty array, because
/// there is no element to return and inventing one would be worse.
pub fn pick<T: Clone>(items: &Vec<T>) -> Result<T, String> {
    if items.is_empty() {
        return Err("rand_pick: the array is empty, so there is nothing to pick".to_string());
    }
    let index = rand::thread_rng().gen_range(0..items.len());
    return Ok(items[index].clone());
}

/// A given number of elements drawn without replacement, in random order.
/// Asking for more than the array holds is an error rather than a short answer.
pub fn sample<T: Clone>(items: &Vec<T>, count: i64) -> Result<Vec<T>, String> {
    if count < 0 {
        return Err(format!("rand_sample: asked for {} elements, which is not a count", count));
    }
    if count as usize > items.len() {
        return Err(format!("rand_sample: asked for {} elements from an array of {}", count, items.len()));
    }
    let mut copy = items.clone();
    copy.shuffle(&mut rand::thread_rng());
    copy.truncate(count as usize);
    return Ok(copy);
}

/// Shared Box-Muller draw, so the seeded and unseeded halves cannot drift
/// apart. Two uniform draws become one normally distributed value.
fn normal_from<R: Rng>(rng: &mut R, mean: f64, stddev: f64) -> f64 {
    // Shifting the first draw into (0.0, 1.0] keeps the logarithm finite.
    let first = 1.0 - rng.gen::<f64>();
    let second = rng.gen::<f64>();
    let standard = (-2.0 * first.ln()).sqrt() * (std::f64::consts::TAU * second).cos();
    return mean + stddev * standard;
}

/// A value from a normal distribution with the given mean and standard
/// deviation - the bell curve that measurement noise, heights and load-test
/// jitter follow.
pub fn normal(mean: f64, stddev: f64) -> f64 {
    return normal_from(&mut rand::thread_rng(), mean, stddev);
}

/// One element of the array, chosen with probability proportional to its
/// weight, so a weight of 2.0 is picked twice as often as a weight of 1.0 and
/// a weight of zero is never picked. The arrays must be the same length, every
/// weight nonnegative, and at least one weight positive.
pub fn weighted_pick<T: Clone>(options: &Vec<T>, weights: &Vec<f64>) -> Result<T, String> {
    if options.is_empty() {
        return Err("rand_weighted_pick: the array is empty, so there is nothing to pick".to_string());
    }
    if options.len() != weights.len() {
        return Err(format!("rand_weighted_pick: {} options against {} weights, and every option needs exactly one", options.len(), weights.len()));
    }
    for weight in weights {
        if !weight.is_finite() || *weight < 0.0 {
            return Err(format!("rand_weighted_pick: the weight {} is not a nonnegative number", weight));
        }
    }
    let total: f64 = weights.iter().sum();
    if total <= 0.0 {
        return Err("rand_weighted_pick: every weight is zero, so nothing can be picked".to_string());
    }

    let target = rand::thread_rng().gen::<f64>() * total;
    let mut cumulative = 0.0;
    for (option, weight) in options.iter().zip(weights) {
        cumulative += weight;
        if target < cumulative {
            return Ok(option.clone());
        }
    }
    // Floating-point summation can leave the target a hair past the last
    // positive weight; that weight's option is the right answer.
    let last_positive = options.iter().zip(weights).rev().find(|(_, weight)| **weight > 0.0).expect("at least one weight is positive");
    return Ok(last_positive.0.clone());
}

/// The same whole number every time for a given seed.
pub fn seeded_int(seed: i64, min: i64, max: i64) -> Result<i64, String> {
    check_range("rand_seeded_int", min, max)?;
    return Ok(StdRng::seed_from_u64(seed as u64).gen_range(min..=max));
}

/// The same fraction every time for a given seed, from 0.0 up to 1.0.
pub fn seeded_float(seed: i64) -> f64 {
    return StdRng::seed_from_u64(seed as u64).gen::<f64>();
}

/// The same normally distributed value every time for a given seed, with the
/// given mean and standard deviation.
pub fn seeded_normal(seed: i64, mean: f64, stddev: f64) -> f64 {
    return normal_from(&mut StdRng::seed_from_u64(seed as u64), mean, stddev);
}

/// The same reordering every time for a given seed.
pub fn seeded_shuffle<T: Clone>(seed: i64, items: Vec<T>) -> Vec<T> {
    let mut copy = items;
    copy.shuffle(&mut StdRng::seed_from_u64(seed as u64));
    return copy;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_stays_inside_its_bounds() {
        for _ in 0..200 {
            let value = int(3, 5).expect("a valid range");
            assert!((3..=5).contains(&value), "{} is outside 3..=5", value);
        }
    }

    #[test]
    fn int_allows_a_single_value_range() {
        assert_eq!(int(7, 7).expect("a valid range"), 7);
    }

    #[test]
    fn int_rejects_a_backwards_range() {
        assert!(int(5, 3).unwrap_err().contains("above the high bound"));
    }

    #[test]
    fn float_range_stays_inside_its_bounds() {
        for _ in 0..200 {
            let value = float_range(-1.5, 2.5).expect("a valid range");
            assert!((-1.5..2.5).contains(&value), "{} is outside -1.5..2.5", value);
        }
    }

    #[test]
    fn chance_of_zero_is_never_and_one_is_always() {
        for _ in 0..100 {
            assert!(!chance(0.0).expect("a probability"));
            assert!(chance(1.0).expect("a probability"));
        }
    }

    #[test]
    fn chance_rejects_a_value_that_is_not_a_probability() {
        assert!(chance(1.5).unwrap_err().contains("not a probability"));
    }

    #[test]
    fn pick_returns_an_element_of_the_array() {
        let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        for _ in 0..50 {
            assert!(items.contains(&pick(&items).expect("a non-empty array")));
        }
    }

    #[test]
    fn pick_rejects_an_empty_array() {
        let empty: Vec<i64> = vec![];
        assert!(pick(&empty).unwrap_err().contains("empty"));
    }

    #[test]
    fn sample_returns_distinct_elements() {
        let items = vec![1, 2, 3, 4, 5];
        let drawn = sample(&items, 3).expect("fewer than the array holds");
        assert_eq!(drawn.len(), 3);
        let mut sorted = drawn.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 3, "sample repeated an element");
    }

    #[test]
    fn sample_rejects_asking_for_more_than_there_is() {
        assert!(sample(&vec![1, 2], 3).unwrap_err().contains("from an array of 2"));
    }

    #[test]
    fn the_same_seed_gives_the_same_answers() {
        assert_eq!(seeded_int(42, 0, 1000).expect("a valid range"), seeded_int(42, 0, 1000).expect("a valid range"));
        assert_eq!(seeded_float(42), seeded_float(42));
        let items = vec![1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(seeded_shuffle(42, items.clone()), seeded_shuffle(42, items));
    }

    #[test]
    fn different_seeds_give_different_answers() {
        let items: Vec<i64> = (0..64).collect();
        assert_ne!(seeded_shuffle(1, items.clone()), seeded_shuffle(2, items));
    }

    /// A wide statistical net: with ten thousand draws the sample mean sits
    /// within a few hundredths of the true mean, so a tolerance of 0.2 only
    /// fails when the distribution itself is wrong.
    #[test]
    fn normal_draws_center_on_the_mean_with_the_right_spread() {
        let draws: Vec<f64> = (0..10_000).map(|_| normal(5.0, 2.0)).collect();
        let mean = draws.iter().sum::<f64>() / draws.len() as f64;
        assert!((mean - 5.0).abs() < 0.2, "sample mean {} is too far from 5.0", mean);
        let variance = draws.iter().map(|value| (value - mean).powi(2)).sum::<f64>() / draws.len() as f64;
        let stddev = variance.sqrt();
        assert!((stddev - 2.0).abs() < 0.2, "sample stddev {} is too far from 2.0", stddev);
    }

    #[test]
    fn the_same_seed_gives_the_same_normal_value() {
        assert_eq!(seeded_normal(42, 0.0, 1.0), seeded_normal(42, 0.0, 1.0));
        assert_ne!(seeded_normal(1, 0.0, 1.0), seeded_normal(2, 0.0, 1.0));
    }

    /// The seeded draw is mean + stddev * z for a z fixed by the seed, so
    /// shifting and scaling it is exact, not merely close.
    #[test]
    fn a_seeded_normal_scales_exactly_with_its_mean_and_stddev() {
        let standard = seeded_normal(7, 0.0, 1.0);
        assert_eq!(seeded_normal(7, 10.0, 2.0), 10.0 + 2.0 * standard);
    }

    #[test]
    fn a_weighted_pick_never_lands_on_a_zero_weight() {
        let options = vec!["common".to_string(), "impossible".to_string(), "rare".to_string()];
        let weights = vec![10.0, 0.0, 1.0];
        for _ in 0..500 {
            let picked = weighted_pick(&options, &weights).expect("valid weights");
            assert_ne!(picked, "impossible", "a zero weight was picked");
        }
    }

    #[test]
    fn a_single_positive_weight_is_always_picked() {
        let options = vec!["a".to_string(), "b".to_string()];
        for _ in 0..50 {
            assert_eq!(weighted_pick(&options, &vec![0.0, 3.0]).expect("valid weights"), "b");
        }
    }

    #[test]
    fn weighted_pick_rejects_bad_weights() {
        let options = vec!["a".to_string(), "b".to_string()];
        assert!(weighted_pick(&options, &vec![1.0]).unwrap_err().contains("2 options against 1 weights"));
        assert!(weighted_pick(&options, &vec![1.0, -0.5]).unwrap_err().contains("not a nonnegative number"));
        assert!(weighted_pick(&options, &vec![0.0, 0.0]).unwrap_err().contains("every weight is zero"));
        let empty: Vec<String> = vec![];
        assert!(weighted_pick(&empty, &vec![]).unwrap_err().contains("empty"));
    }
}
