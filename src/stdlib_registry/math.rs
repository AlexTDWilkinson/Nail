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
        "math_divide" => "std_lib::math::divide", (numerator: (T: i|f), denominator: (T: i|f)) -> (T!e),
            "Divides two numbers, returning an error on division by zero.",
            "ratio:f = danger(math_divide(10.0, 4.0));";
        "math_gcd" => "std_lib::math::gcd", (first: i, second: i) -> i,
            "Returns the greatest common divisor of two integers.",
            "common:i = math_gcd(12, 18);";
        "math_lcm" => "std_lib::math::lcm", (first: i, second: i) -> i,
            "Returns the least common multiple of two integers.",
            "multiple:i = math_lcm(4, 6);";
        "math_factorial" => "std_lib::math::factorial", (value: i) -> (i!e),
            "Returns value! as an integer. Errors for negative input or results that overflow.",
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
            "Returns the arcsine in radians. Errors if the input is outside -1.0..1.0.",
            "angle:f = danger(math_asin(0.5));";
        "math_acos" => "std_lib::math::acos", (value: f) -> (f!e),
            "Returns the arccosine in radians. Errors if the input is outside -1.0..1.0.",
            "angle:f = danger(math_acos(0.5));";
        "math_atan" => "std_lib::math::atan", (value: f) -> f,
            "Returns the arctangent in radians.",
            "angle:f = math_atan(1.0);";
        "math_log" => "std_lib::math::log", (value: f) -> (f!e),
            "Returns the natural logarithm. Errors if the input is not positive.",
            "ln:f = danger(math_log(2.718));";
        "math_log10" => "std_lib::math::log10", (value: f) -> (f!e),
            "Returns the base-10 logarithm. Errors if the input is not positive.",
            "digits:f = danger(math_log10(1000.0));";
        "math_log2" => "std_lib::math::log2", (value: f) -> (f!e),
            "Returns the base-2 logarithm. Errors if the input is not positive.",
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
        "math_sign" => "std_lib::math::sign", (value: (T: i|f)) -> i,
            "Returns -1, 0 or 1 according to whether the value is negative, zero or positive.",
            "direction:i = math_sign(-4.2);";
        "math_atan2" => "std_lib::math::atan2", (y: f, x: f) -> f,
            "Returns the angle from the positive x axis to the point (x, y), from -pi to pi. Use this rather than math_atan for angles - a plain arc tangent cannot tell the quadrants apart.",
            "angle:f = math_atan2(dy, dx);";
        "math_hypot" => "std_lib::math::hypot", (x: f, y: f) -> f,
            "Returns the distance from the origin to (x, y), computed without squaring the inputs first so very large distances stay exact.",
            "distance:f = math_hypot(dx, dy);";
        "math_cbrt" => "std_lib::math::cbrt", (value: f) -> f,
            "Returns the cube root, which is defined for negative numbers too - unlike raising to the power of one third.",
            "side:f = math_cbrt(27.0);";
        "math_trunc" => "std_lib::math::trunc", (value: f) -> f,
            "Throws away the fractional part towards zero, so -2.7 becomes -2.0 where math_floor would give -3.0.",
            "whole:f = math_trunc(-2.7);";
        "math_fract" => "std_lib::math::fract", (value: f) -> f,
            "Returns just the fractional part, keeping the sign.",
            "part:f = math_fract(2.75);";
        "math_to_degrees" => "std_lib::math::to_degrees", (radians: f) -> f,
            "Writes an angle given in radians as degrees.",
            "degrees:f = math_to_degrees(angle);";
        "math_to_radians" => "std_lib::math::to_radians", (degrees: f) -> f,
            "Writes an angle given in degrees as radians, which is what every function here that takes an angle expects.",
            "angle:f = math_to_radians(90.0);";
        "math_sinh" => "std_lib::math::sinh", (value: f) -> f,
            "Returns the hyperbolic sine.",
            "value:f = math_sinh(1.0);";
        "math_cosh" => "std_lib::math::cosh", (value: f) -> f,
            "Returns the hyperbolic cosine.",
            "value:f = math_cosh(1.0);";
        "math_tanh" => "std_lib::math::tanh", (value: f) -> f,
            "Returns the hyperbolic tangent.",
            "value:f = math_tanh(1.0);";
        "math_modulo" => "std_lib::math::modulo", (value: f, divisor: f) -> (f!e),
            "Returns the remainder with the sign of the divisor, so -1 modulo 12 is 11. What clock arithmetic and wrapping an index round an array need, and what % does not give.",
            "wrapped:f = danger(math_modulo(-1.0, 12.0));";
        "math_log_base" => "std_lib::math::log_base", (value: f, base: f) -> (f!e),
            "Returns the logarithm in a base of your choosing. Errors on a value at or below zero or an unusable base.",
            "power:f = danger(math_log_base(81.0, 3.0));";
        "math_is_nan" => "std_lib::math::is_nan", (value: f) -> b,
            "Returns whether this is the not-a-number value. It is the one value not equal to itself, so == cannot be used to ask.",
            "broken:b = math_is_nan(computed);";
        "math_is_infinite" => "std_lib::math::is_infinite", (value: f) -> b,
            "Returns whether this is positive or negative infinity.",
            "overflowed:b = math_is_infinite(computed);";
        "math_is_finite" => "std_lib::math::is_finite", (value: f) -> b,
            "Returns whether this is an ordinary number - neither infinite nor not-a-number. The check to make before trusting a computed value.",
            "usable:b = math_is_finite(computed);";
        "math_round_to" => "std_lib::math::round_to", (value: f, decimals: i) -> (f!e),
            "Rounds to a fixed number of decimal places (0 to 12), halves away from zero the way people round on paper.",
            "price:f = danger(math_round_to(2.34567, 2));";
        "math_percent_change" => "std_lib::math::percent_change", (old: f, new: f) -> (f!e),
            "Returns how much a value grew or shrank as a percentage of where it started. Errors when the old value is zero.",
            "growth:f = danger(math_percent_change(50.0, 75.0));";
        "math_percent_of" => "std_lib::math::percent_of", (part: f, whole: f) -> (f!e),
            "Returns what percentage the part is of the whole. Errors when the whole is zero.",
            "share:f = danger(math_percent_of(30.0, 120.0));";
        "math_nth_root" => "std_lib::math::nth_root", (value: f, degree: i) -> (f!e),
            "Returns the nth root. An odd root of a negative number is negative, an even root of one is an error.",
            "side:f = danger(math_nth_root(32.0, 5));";
        "math_combinations" => "std_lib::math::combinations", (n: i, k: i) -> (i!e),
            "Returns how many ways to choose k things from n when order does not matter. Choosing more than there are gives 0, negatives and overflow are errors.",
            "hands:i = danger(math_combinations(52, 5));";
        "math_permutations" => "std_lib::math::permutations", (n: i, k: i) -> (i!e),
            "Returns how many ways to arrange k things drawn from n when order matters. Drawing more than there are gives 0, negatives and overflow are errors.",
            "orders:i = danger(math_permutations(10, 3));";
        "math_smoothstep" => "std_lib::math::smoothstep", (edge_low: f, edge_high: f, value: f) -> (f!e),
            "Eases from 0 at the low edge to 1 at the high edge along a smooth S-curve, holding at 0 and 1 outside them. The edges must differ.",
            "fade:f = danger(math_smoothstep(0.0, 1.0, progress));";
        "math_compound_growth" => "std_lib::math::compound_growth", (principal: f, rate_per_period: f, periods: i) -> (f!e),
            "Returns what a starting amount becomes after growing by a fixed rate for a number of periods. Errors on negative periods.",
            "balance:f = danger(math_compound_growth(1000.0, 0.05, 10));";
        "math_log1p" => "std_lib::math::log1p", (value: f) -> (f!e),
            "Returns ln(1 + x), computed accurately for x very close to zero where adding 1 first would lose it. Errors at or below -1.",
            "rate:f = danger(math_log1p(0.000001));";
        "math_expm1" => "std_lib::math::expm1", (value: f) -> f,
            "Returns e^x - 1, computed accurately for x very close to zero where subtracting 1 afterwards would cancel the precision away.",
            "growth:f = math_expm1(0.000001);";
        "math_copysign" => "std_lib::math::copysign", (magnitude: f, sign_source: f) -> f,
            "Returns the first number wearing the sign of the second.",
            "signed:f = math_copysign(3.0, -1.5);";
        "math_sum_of_digits" => "std_lib::math::sum_of_digits", (value: i) -> i,
            "Returns the decimal digits of a number added together, ignoring its sign.",
            "checksum:i = math_sum_of_digits(1234);";
        "math_digit_count" => "std_lib::math::digit_count", (value: i) -> i,
            "Returns how many decimal digits a number has, ignoring its sign. 0 has one digit.",
            "width:i = math_digit_count(-1234);";
        "math_is_perfect_square" => "std_lib::math::is_perfect_square", (value: i) -> b,
            "Returns whether the integer is some integer multiplied by itself. No negative number is.",
            "square:b = math_is_perfect_square(49);";
        "math_fibonacci" => "std_lib::math::fibonacci", (position: i) -> (i!e),
            "Returns the Fibonacci number at a position, counting from fibonacci(0) = 0. Position 92 is the last that fits in a 64-bit integer.",
            "grown:i = danger(math_fibonacci(10));";
        "math_triangular" => "std_lib::math::triangular", (n: i) -> (i!e),
            "Returns the nth triangular number 1 + 2 + ... + n. Errors on negative input or overflow.",
            "stacked:i = danger(math_triangular(100));";
        "math_wrap" => "std_lib::math::wrap", (value: f, low: f, high: f) -> (f!e),
            "Folds a value into the range from low up to but not including high, the way an angle of 370 degrees is really 10. The low edge must be below the high.",
            "heading:f = danger(math_wrap(370.0, 0.0, 360.0));";
    }
}
