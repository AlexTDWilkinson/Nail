// Generic math functions for Nail
use std::cmp::{Ord, Ordering};
use std::ops::Neg;

// Generic min function - returns minimum of two values
pub async fn min<T>(a: T, b: T) -> T
where
    T: PartialOrd,
{
    if a <= b {
        a
    } else {
        b
    }
}

// Generic max function - returns maximum of two values
pub async fn max<T>(a: T, b: T) -> T
where
    T: PartialOrd,
{
    if a >= b {
        a
    } else {
        b
    }
}

// Generic clamp function - clamps value between min and max
pub async fn clamp<T>(value: T, min_val: T, max_val: T) -> T
where
    T: Ord,
{
    if value < min_val {
        min_val
    } else if value > max_val {
        max_val
    } else {
        value
    }
}

// Generic sign function - returns -1, 0, or 1
pub async fn sign<T>(value: T) -> i64
where
    T: Ord + Default,
{
    let zero = T::default();
    match value.cmp(&zero) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

pub async fn abs<T>(value: T) -> T
where
    T: Ord + Default + Neg<Output = T>,
{
    let zero = T::default();
    if value < zero {
        -value
    } else {
        value
    }
}
