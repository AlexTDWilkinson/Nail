//! Args module stdlib registry entries - parsed command-line arguments.
//!
//! Two ways in. The reading functions take one argument at a time and need no
//! description. args_parse takes the program's description of its whole
//! command line, an array of ARGS_Option, and returns it read and checked in
//! one go - and args_help_text prints that same description, so a program
//! cannot accept a flag its help page never mentions.

use super::*;

/// The array-of-ARGS_Option parameter both describing functions take.
fn options_parameter() -> StdlibParameter {
    return StdlibParameter { name: "options".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct("ARGS_Option".to_string()))), pass_by_reference: true };
}

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Args:
        "args_get" => "std_lib::args::get", (index: i) -> (s!e),
            "Returns the positional command-line argument at the index. Errors if out of range.",
            "first:s = danger(args_get(0));";
        "args_flag" => "std_lib::args::flag", (name: s) -> b,
            "Returns true if a flag like --verbose is present.",
            "verbose:b = args_flag(`verbose`);";
        "args_value" => "std_lib::args::value", (name: s) -> (s!e),
            "Returns the value of a flag like --name=value. Errors if the flag is missing.",
            "output:s = danger(args_value(`output`));";
        "args_value_or" => "std_lib::args::value_or", (name: s, fallback: s) -> s,
            "Returns the value of a flag, or the fallback when it was not passed.",
            "output:s = args_value_or(`output`, `report.txt`);";
        "args_value_int" => "std_lib::args::value_int", (name: s) -> (i!e),
            "Returns the value of a flag read as a whole number. A missing flag and a value that is not a number are different errors.",
            "retries:i = danger(args_value_int(`retries`));";
        "args_value_float" => "std_lib::args::value_float", (name: s) -> (f!e),
            "Returns the value of a flag read as a fraction.",
            "ratio:f = danger(args_value_float(`ratio`));";
        "args_count" => "std_lib::args::count", () -> i,
            "Returns the number of command-line arguments.",
            "total:i = args_count();";
        "args_wants_help" => "std_lib::args::wants_help", () -> b,
            "Returns whether the program was asked for help with --help or -h. Check this first and print args_help_text.",
            "asked:b = args_wants_help();";
    }

    // The two functions that take the program's description of its whole
    // command line use the full struct form.
    m.insert("args_parse", StdlibFunction {
        rust_path: "std_lib::args::parse".to_string(),
        crate_deps: vec![CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("ARGS_Option", "nail::std_lib::args"), ("ARGS_Parsed", "nail::std_lib::args")],
        module: StdlibModule::Args,
        parameters: vec![options_parameter()],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("ARGS_Parsed".to_string()))),
        diverging: false,
        description: "Reads and checks the whole command line against the program's description of it, and returns it as data: the subcommand, the positional arguments, the values and the flags. Errors on an unknown flag, a missing value, a value given to a flag that takes none, or a missing required option.",
        example: "options:a:ARGS_Option = [\n    ARGS_Option { name = `output`, short = `o`, description = `where to write the result`, takes_value = true, required = false },\n    ARGS_Option { name = `quiet`, short = `q`, description = `say less while working`, takes_value = false, required = false },\n];\nparsed:ARGS_Parsed = danger(args_parse(options));",
    });

    m.insert("args_help_text", StdlibFunction {
        rust_path: "std_lib::args::help_text".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("ARGS_Option", "nail::std_lib::args")],
        module: StdlibModule::Args,
        parameters: vec![nail_param!(program: s), nail_param!(description: s), options_parameter()],
        return_type: nail_type!(s),
        diverging: false,
        description: "Builds the --help page from the program's own description of its options, so the page cannot drift from what the program accepts.",
        example: "options:a:ARGS_Option = [\n    ARGS_Option { name = `output`, short = `o`, description = `where to write the result`, takes_value = true, required = false },\n    ARGS_Option { name = `quiet`, short = `q`, description = `say less while working`, takes_value = false, required = false },\n];\nprint(args_help_text(`mytool`, `Does a thing.`, options));",
    });
}
