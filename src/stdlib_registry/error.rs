//! Error module stdlib registry entries - result handling functions.
//! Diverging panic/todo live in the Panic module (panic.rs).

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Error:
        "safe" => "std_lib::error::safe", (value: (T!e), handler: (fn(e) -> T)) -> T,
            "Unwraps a result, calling the error handler to produce a fallback value on failure.",
            "count:i = safe(int_from(input), zero_when_unreadable);";
        "danger" => "std_lib::error::danger", (value: (T!e)) -> T,
            "Unwraps a result, crashing the program if it is an error. Intended as a temporary escape hatch.",
            "content:s = danger(fs_read(`config.txt`));";
        "expect" => "std_lib::error::expect", (value: (T!e)) -> T,
            "Unwraps a result, crashing on error. Like danger, but signals the failure is considered impossible.",
            "config:s = expect(fs_read(`config.txt`));";
        "error_message" => "std_lib::error::message", (err: e) -> s,
            "The text inside an error value, for handlers that want to show it or wrap it.",
            "f fallback(err:e):s { r error_message(err); }";
    }
}
