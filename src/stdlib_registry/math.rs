//! Math module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Math:
        "math_abs" => "std_lib::math::abs", (value: f) -> f,
            "Returns the absolute value of a float.",
            "positive:f = math_abs(-4.2);";
        "math_sqrt" => "std_lib::math::sqrt", (value: f) -> f,
            "Returns the square root of a float.",
            "root:f = math_sqrt(16.0);";
        "math_pow" => "std_lib::math::pow", (base: f, exponent: f) -> f,
            "Raises base to the power of exponent.",
            "cube:f = math_pow(2.0, 3.0);";
        "math_round" => "std_lib::math::round", (value: f) -> f,
            "Rounds to the nearest whole number, returned as a float.",
            "whole:f = math_round(2.6);";
        "math_round_to_int" => "std_lib::math::round_to_int", (value: f) -> i,
            "Rounds to the nearest whole number, returned as an integer.",
            "whole:i = math_round_to_int(2.6);";
        "math_floor" => "std_lib::math::floor", (value: f) -> f,
            "Rounds down to the nearest whole number.",
            "lower:f = math_floor(2.9);";
        "math_ceil" => "std_lib::math::ceil", (value: f) -> f,
            "Rounds up to the nearest whole number.",
            "upper:f = math_ceil(2.1);";
        "math_min" => "std_lib::math::min", (first: f, second: f) -> f,
            "Returns the smaller of two floats.",
            "smaller:f = math_min(1.0, 2.0);";
        "math_max" => "std_lib::math::max", (first: f, second: f) -> f,
            "Returns the larger of two floats.",
            "larger:f = math_max(1.0, 2.0);";
        "math_random" [Rand] => "std_lib::math::random", () -> f,
            "Returns a random float between 0.0 (inclusive) and 1.0 (exclusive).",
            "roll:f = math_random();";
        "math_divide" => "std_lib::math::divide", (numerator: any, denominator: any) -> (any!e),
            "Divides two numbers, returning an error on division by zero.",
            "ratio:f = danger(math_divide(10.0, 4.0));";
        "math_gcd" => "std_lib::math::gcd", (first: i, second: i) -> i,
            "Returns the greatest common divisor of two integers.",
            "common:i = math_gcd(12, 18);";
        "math_lcm" => "std_lib::math::lcm", (first: i, second: i) -> i,
            "Returns the least common multiple of two integers.",
            "multiple:i = math_lcm(4, 6);";
        "math_factorial" => "std_lib::math::factorial", (value: i) -> (i!e),
            "Returns value! as an integer; errors for negative input or results that overflow.",
            "result:i = danger(math_factorial(5));";
        "math_is_prime" => "std_lib::math::is_prime", (value: i) -> b,
            "Returns true if the integer is a prime number.",
            "prime:b = math_is_prime(7);";
        "math_sigmoid" => "std_lib::math::sigmoid", (value: f) -> f,
            "Returns the logistic sigmoid 1 / (1 + e^-x).",
            "squashed:f = math_sigmoid(0.5);";
        "math_lerp" => "std_lib::math::lerp", (start: f, end: f, t: f) -> f,
            "Linearly interpolates between start and end by t (clamped to 0.0..1.0).",
            "mid:f = math_lerp(0.0, 10.0, 0.5);";
        "math_sin" => "std_lib::math::sin", (radians: f) -> f,
            "Returns the sine of an angle in radians.",
            "wave:f = math_sin(3.14159);";
        "math_cos" => "std_lib::math::cos", (radians: f) -> f,
            "Returns the cosine of an angle in radians.",
            "wave:f = math_cos(0.0);";
        "math_tan" => "std_lib::math::tan", (radians: f) -> f,
            "Returns the tangent of an angle in radians.",
            "slope:f = math_tan(0.785);";
        "math_asin" => "std_lib::math::asin", (value: f) -> (f!e),
            "Returns the arcsine in radians; errors if the input is outside -1.0..1.0.",
            "angle:f = danger(math_asin(0.5));";
        "math_acos" => "std_lib::math::acos", (value: f) -> (f!e),
            "Returns the arccosine in radians; errors if the input is outside -1.0..1.0.",
            "angle:f = danger(math_acos(0.5));";
        "math_atan" => "std_lib::math::atan", (value: f) -> f,
            "Returns the arctangent in radians.",
            "angle:f = math_atan(1.0);";
        "math_log" => "std_lib::math::log", (value: f) -> (f!e),
            "Returns the natural logarithm; errors if the input is not positive.",
            "ln:f = danger(math_log(2.718));";
        "math_log10" => "std_lib::math::log10", (value: f) -> (f!e),
            "Returns the base-10 logarithm; errors if the input is not positive.",
            "digits:f = danger(math_log10(1000.0));";
        "math_log2" => "std_lib::math::log2", (value: f) -> (f!e),
            "Returns the base-2 logarithm; errors if the input is not positive.",
            "bits:f = danger(math_log2(8.0));";
        "math_clamp" => "std_lib::math::clamp", (value: f, min: f, max: f) -> f,
            "Restricts a value to the range min..max.",
            "bounded:f = math_clamp(15.0, 0.0, 10.0);";
        "math_exp" => "std_lib::math::exp", (value: f) -> f,
            "Returns e raised to the given power.",
            "growth:f = math_exp(1.0);";
        "math_pi" => "std_lib::math::pi", () -> f,
            "Returns the constant pi (3.14159...).",
            "circumference:f = 2.0 * math_pi() * radius;";
        "math_e" => "std_lib::math::e", () -> f,
            "Returns Euler's number e (2.71828...).",
            "base:f = math_e();";
    }
}
