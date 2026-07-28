//! Time module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Time:
        "time_now" => "std_lib::time::now", () -> i,
            "Returns the current Unix timestamp in seconds.",
            "now:i = time_now();";
        "time_sleep" [Tokio] => "std_lib::time::sleep", (seconds: f) -> v,
            "Pauses the current task for the given number of seconds.",
            "time_sleep(0.5);";
        "time_add_seconds" => "std_lib::time::add_seconds", (timestamp: i, seconds: i) -> i,
            "Returns the timestamp shifted by the given number of seconds (negative to subtract).",
            "later:i = time_add_seconds(now, 3600);";
        "time_diff" => "std_lib::time::diff", (timestamp1: i, timestamp2: i) -> i,
            "Returns the absolute difference between two timestamps in seconds.",
            "elapsed:i = time_diff(finish, start);";
        "time_now_millis" => "std_lib::time::now_millis", () -> i,
            "Returns the current Unix timestamp in milliseconds.",
            "now_ms:i = time_now_millis();";
    }

    // time_format / time_parse take the TimeFormat enum, which needs a custom
    // type import, so they use the full struct form.
    m.insert("time_format", StdlibFunction {
        rust_path: "std_lib::time::format".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("TimeFormat", "nail::std_lib::time")],
        module: StdlibModule::Time,
        parameters: vec![
            nail_param!(timestamp: i),
            StdlibParameter { name: "format".to_string(), param_type: NailDataTypeDescriptor::Enum("TimeFormat".to_string()), pass_by_reference: false },
        ],
        return_type: nail_type!(s),
        diverging: false,
        description: "Formats a Unix timestamp as a string using the given TimeFormat.",
        example: "text:s = time_format(now, TimeFormat::Unix);",
    });

    m.insert("time_parse", StdlibFunction {
        rust_path: "std_lib::time::parse".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("TimeFormat", "nail::std_lib::time")],
        module: StdlibModule::Time,
        parameters: vec![
            nail_param!(time_str: s),
            StdlibParameter { name: "format".to_string(), param_type: NailDataTypeDescriptor::Enum("TimeFormat".to_string()), pass_by_reference: false },
        ],
        return_type: nail_type!((i!e)),
        diverging: false,
        description: "Parses a time string in the given TimeFormat into a Unix timestamp; errors on invalid input.",
        example: "stamp:i = danger(time_parse(`1700000000`, TimeFormat::Unix));",
    });
}
