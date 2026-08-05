//! Random module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Rand:
        "rand_int" [Rand] => "std_lib::rand::int", (min: i, max: i) -> (i!e),
            "Returns a random whole number from min to max, both ends included; errors if min is above max.",
            "roll:i = danger(rand_int(1, 6));";
        "rand_float" [Rand] => "std_lib::rand::float", () -> f,
            "Returns a random fraction from 0.0 up to but not including 1.0.",
            "chance:f = rand_float();";
        "rand_float_range" [Rand] => "std_lib::rand::float_range", (min: f, max: f) -> (f!e),
            "Returns a random fraction from min up to but not including max; errors if min is above max.",
            "jitter:f = danger(rand_float_range(0.5, 1.5));";
        "rand_bool" [Rand] => "std_lib::rand::boolean", () -> b,
            "Returns true or false with even odds.",
            "heads:b = rand_bool();";
        "rand_chance" [Rand] => "std_lib::rand::chance", (probability: f) -> (b!e),
            "Returns true with the given probability from 0.0 to 1.0; errors on anything outside that.",
            "rare:b = danger(rand_chance(0.05));";
        "rand_pick" [Rand] => "std_lib::rand::pick", (items: (&[T])) -> (T!e),
            "Returns one element of the array, chosen evenly; errors if the array is empty.",
            "winner:s = danger(rand_pick(names));";
        "rand_sample" [Rand] => "std_lib::rand::sample", (items: (&[T]), count: i) -> ([T]!e),
            "Returns the given number of elements drawn without replacement, in random order; errors if the array is smaller than that.",
            "three:a:s = danger(rand_sample(names, 3));";
        "rand_seeded_int" [Rand] => "std_lib::rand::seeded_int", (seed: i, min: i, max: i) -> (i!e),
            "Returns the same whole number every time for a given seed, from min to max inclusive. Use it when a random result has to be reproducible.",
            "roll:i = danger(rand_seeded_int(42, 1, 6));";
        "rand_seeded_float" [Rand] => "std_lib::rand::seeded_float", (seed: i) -> f,
            "Returns the same fraction every time for a given seed, from 0.0 up to 1.0.",
            "value:f = rand_seeded_float(42);";
        "rand_seeded_shuffle" [Rand] => "std_lib::rand::seeded_shuffle", (seed: i, items: [T]) -> [T],
            "Returns the array in the same shuffled order every time for a given seed.",
            "deck:a:i = rand_seeded_shuffle(42, cards);";
    }
}
