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

/// The same whole number every time for a given seed.
pub fn seeded_int(seed: i64, min: i64, max: i64) -> Result<i64, String> {
    check_range("rand_seeded_int", min, max)?;
    return Ok(StdRng::seed_from_u64(seed as u64).gen_range(min..=max));
}

/// The same fraction every time for a given seed, from 0.0 up to 1.0.
pub fn seeded_float(seed: i64) -> f64 {
    return StdRng::seed_from_u64(seed as u64).gen::<f64>();
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
}
