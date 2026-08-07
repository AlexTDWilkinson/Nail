//! A total order for the values Nail sorts.
//!
//! Rust splits ordering in two. `Ord` is a total order, where any two values
//! can be put in sequence, and `PartialOrd` is not, because floats hold NaN and
//! NaN is not less than, equal to or greater than anything at all. A sort needs
//! a total order, so a stdlib written against either one alone gets a defect:
//!
//!   - `Ord` refuses floats outright. `array_sort` was written that way, so
//!     sorting an array of floats type checked in Nail and then failed to
//!     compile in Rust, which is the worst place for an error to surface.
//!   - `PartialOrd` accepts floats and then has nowhere to put a NaN. The usual
//!     patch is `partial_cmp(..).unwrap_or(Ordering::Equal)`, which says a NaN
//!     equals every number it meets. That breaks transitivity, and a sort given
//!     an intransitive comparison returns an arbitrary order with no complaint.
//!     The array comes back shuffled and nothing says why.
//!
//! So sorting asks for this instead. Every type Nail can sort implements it,
//! floats included, using `f64::total_cmp` - the IEEE 754 total order, where
//! NaN sits at the ends rather than nowhere. Sorting therefore cannot fail, and
//! none of the sort functions need to return a result to say so.

use std::cmp::Ordering;

/// How two values of the same type are put in order. Total: every pair has an
/// answer, and the answer is consistent, which is all a sort requires.
pub trait NailOrd {
    fn nail_cmp(&self, other: &Self) -> Ordering;
}

/// The types with a total order of their own just use it.
macro_rules! total_order_from_ord {
    ($($type:ty),*) => {
        $(impl NailOrd for $type {
            fn nail_cmp(&self, other: &Self) -> Ordering {
                return self.cmp(other);
            }
        })*
    };
}

total_order_from_ord!(i64, i32, u8, u32, usize, bool, char, String);

impl NailOrd for f64 {
    /// `total_cmp` is IEEE 754's own total order: -NaN below every number,
    /// +NaN above every number, and -0.0 just under 0.0. Any two floats have
    /// an answer, so a NaN in the array lands at one end instead of scrambling
    /// the rest of it.
    fn nail_cmp(&self, other: &Self) -> Ordering {
        return self.total_cmp(other);
    }
}

impl NailOrd for f32 {
    fn nail_cmp(&self, other: &Self) -> Ordering {
        return self.total_cmp(other);
    }
}

impl NailOrd for &str {
    fn nail_cmp(&self, other: &Self) -> Ordering {
        return self.cmp(other);
    }
}

/// Arrays compare the way words do: first element first, and a prefix comes
/// before what extends it.
impl<T: NailOrd> NailOrd for Vec<T> {
    fn nail_cmp(&self, other: &Self) -> Ordering {
        for (mine, theirs) in self.iter().zip(other.iter()) {
            let answer = mine.nail_cmp(theirs);
            if answer != Ordering::Equal {
                return answer;
            }
        }
        return self.len().cmp(&other.len());
    }
}

impl<T: NailOrd> NailOrd for &T {
    fn nail_cmp(&self, other: &Self) -> Ordering {
        return (*self).nail_cmp(*other);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_text_order_as_they_always_did() {
        assert_eq!(3i64.nail_cmp(&5), Ordering::Less);
        assert_eq!(1.5f64.nail_cmp(&1.25), Ordering::Greater);
        assert_eq!("apple".to_string().nail_cmp(&"banana".to_string()), Ordering::Less);
        assert_eq!(false.nail_cmp(&true), Ordering::Less);
    }

    #[test]
    fn a_nan_has_a_place_instead_of_equalling_everything() {
        let nan = f64::NAN;
        // The old `partial_cmp(..).unwrap_or(Equal)` said all three of these
        // were Equal, which is what let a NaN scramble a sort.
        assert_eq!(nan.nail_cmp(&1.0), Ordering::Greater);
        assert_eq!(1.0f64.nail_cmp(&nan), Ordering::Less);
        assert_eq!(nan.nail_cmp(&nan), Ordering::Equal);
    }

    #[test]
    fn sorting_with_a_nan_keeps_every_other_value_in_order() {
        let mut values = vec![3.0, f64::NAN, 1.0, 2.0];
        values.sort_by(|left, right| left.nail_cmp(right));
        let without_nan: Vec<f64> = values.iter().copied().filter(|value| !value.is_nan()).collect();
        assert_eq!(without_nan, vec![1.0, 2.0, 3.0]);
        assert!(values.last().expect("four values").is_nan(), "a positive NaN sorts above every number");
    }

    #[test]
    fn arrays_order_like_words() {
        assert_eq!(vec![1i64, 2].nail_cmp(&vec![1, 3]), Ordering::Less);
        assert_eq!(vec![1i64, 2].nail_cmp(&vec![1, 2, 0]), Ordering::Less);
        assert_eq!(vec![1i64, 2].nail_cmp(&vec![1, 2]), Ordering::Equal);
    }
}
