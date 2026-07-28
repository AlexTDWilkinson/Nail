//! Environment module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Env:
        "env_get" => "std_lib::env::get", (key: s) -> (s!e),
            "Returns the value of an environment variable; errors if it is not set.",
            "home:s = danger(env_get(`HOME`));";
        "env_set" => "std_lib::env::set", (key: s, value: s) -> (v!e),
            "Sets an environment variable for the current process.",
            "danger(env_set(`MODE`, `production`));";
        "env_args" => "std_lib::env::args", () -> [s],
            "Returns all command-line arguments, including the program name.",
            "arguments:a:s = env_args();";
    }
}
