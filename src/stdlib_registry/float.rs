//! Float module stdlib registry entries. Float arithmetic (abs, sqrt, round,
//! etc.) lives in the Math module - this module only converts to floats.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Float:
        "float_from" => "std_lib::float::from", (value: (T: i|f|s|b)) -> (f!e),
            "Converts a value (string, int, etc.) to a float. Errors if it cannot be parsed.",
            "price:f = danger(float_from(`19.99`));";
        "float_approx_equal" => "std_lib::float::approx_equal", (first: f, second: f, tolerance: f) -> b,
            "Returns whether two floats are within a tolerance of each other. This is how floats should be compared - 0.1 + 0.2 is not equal to 0.3, so == on computed floats is nearly always a bug.",
            "same:b = float_approx_equal(0.1 + 0.2, 0.3, 0.000000001);";
        "float_is_whole" => "std_lib::float::is_whole", (value: f) -> b,
            "Returns whether the float holds a whole number exactly. Neither infinity nor not-a-number counts.",
            "exact:b = float_is_whole(3.0);";
    }
}
