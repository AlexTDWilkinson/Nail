//! Args module stdlib registry entries - parsed command-line arguments.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Args:
        "args_get" => "std_lib::args::get", (index: i) -> (s!e),
            "Returns the positional command-line argument at the index; errors if out of range.",
            "first:s = danger(args_get(0));";
        "args_flag" => "std_lib::args::flag", (name: s) -> b,
            "Returns true if a flag like --verbose is present.",
            "verbose:b = args_flag(`verbose`);";
        "args_value" => "std_lib::args::value", (name: s) -> (s!e),
            "Returns the value of a flag like --name=value; errors if the flag is missing.",
            "output:s = danger(args_value(`output`));";
        "args_count" => "std_lib::args::count", () -> i,
            "Returns the number of command-line arguments.",
            "total:i = args_count();";
    }
}
