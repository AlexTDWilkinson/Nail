//! Log module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Log:
        "log_debug" => "std_lib::log::debug", (message: s) -> v,
            "Writes a message to standard error at the Debug level, hidden unless the level is lowered to Debug.",
            "log_debug(`cache miss`);";
        "log_info" => "std_lib::log::info", (message: s) -> v,
            "Writes a message to standard error at the Info level, the default threshold.",
            "log_info(`server listening on 8080`);";
        "log_warn" => "std_lib::log::warn", (message: s) -> v,
            "Writes a message to standard error at the Warn level.",
            "log_warn(`retrying after a timeout`);";
        "log_error" => "std_lib::log::error", (message: s) -> v,
            "Writes a message to standard error at the Error level.",
            "log_error(`could not reach the database`);";
        "log_set_json" => "std_lib::log::set_json", (enabled: b) -> v,
            "Switches log lines between a human-readable form and one JSON object per line, for the rest of the run.",
            "log_set_json(true);";
        "log_set_file" => "std_lib::log::set_file", (path: s) -> (v!e),
            "Sends log lines to a file instead of standard error, for the rest of the run. The file is added to rather than replaced. Errors if the file cannot be opened, which is better found here than by losing lines later.",
            "danger(log_set_file(`/var/log/orders.log`));";
    }

    // log_set_level and log_with_fields take the LOG_Level enum, which needs a
    // custom type import, so they use the full struct form.
    m.insert("log_set_level", StdlibFunction {
        rust_path: "std_lib::log::set_level".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("LOG_Level", "nail::std_lib::log")],
        module: StdlibModule::Log,
        parameters: vec![StdlibParameter { name: "level".to_string(), param_type: NailDataTypeDescriptor::Enum("LOG_Level".to_string()), pass_by_reference: false }],
        return_type: nail_type!(v),
        diverging: false,
        description: "Hides every message below this level for the rest of the run. Info by default.",
        example: "log_set_level(LOG_Level::Debug);",
    });

    m.insert("log_with_fields", StdlibFunction {
        rust_path: "std_lib::log::with_fields".to_string(),
        crate_deps: vec![CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("LOG_Level", "nail::std_lib::log")],
        module: StdlibModule::Log,
        parameters: vec![
            StdlibParameter { name: "level".to_string(), param_type: NailDataTypeDescriptor::Enum("LOG_Level".to_string()), pass_by_reference: false },
            nail_param!(message: s),
            nail_param!(fields: (&(h s s))),
        ],
        return_type: nail_type!(v),
        diverging: false,
        description: "Writes a message with named values beside it, which is what makes a log line searchable rather than just readable.",
        example: "log_with_fields(LOG_Level::Info, `request served`, fields);",
    });
}
