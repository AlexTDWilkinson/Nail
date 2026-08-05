//! Process module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("process_exit", StdlibFunction {
        rust_path: "std_lib::process::exit".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Process,
        parameters: vec![nail_param!(code: i)],
        return_type: nail_type!(never),
        diverging: true,
        description: "Terminates the program immediately with the given exit code. Never returns.",
        example: "process_exit(1);",
    });

    m.insert("process_default_options", StdlibFunction {
        rust_path: "std_lib::process::default_options".to_string(),
        crate_deps: vec![CrateDependency::Tokio, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("PROCESS_Options", "nail::std_lib::process")],
        module: StdlibModule::Process,
        parameters: vec![],
        return_type: NailDataTypeDescriptor::Struct("PROCESS_Options".to_string()),
        diverging: false,
        description: "The default options for running a command: here, with nothing added, no input, and no time limit. Nail has no default field values, so this saves spelling out every field of PROCESS_Options.",
        example: "options:PROCESS_Options = process_default_options();",
    });

    m.insert("process_run_result", StdlibFunction {
        rust_path: "std_lib::process::run_result".to_string(),
        crate_deps: vec![CrateDependency::Tokio, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("PROCESS_Result", "nail::std_lib::process")],
        module: StdlibModule::Process,
        parameters: vec![nail_param!(command: s), nail_param!(arguments: [s])],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("PROCESS_Result".to_string()))),
        diverging: false,
        description: "Runs a command and returns everything about how it went: both its output streams and its exit code. A command that fails is not an error here - the exit code is the answer. Errors only when the command could not be started at all.",
        example: "ran:PROCESS_Result = danger(process_run_result(`git`, [`status`]));",
    });

    m.insert("process_run_with", StdlibFunction {
        rust_path: "std_lib::process::run_with".to_string(),
        crate_deps: vec![CrateDependency::Tokio, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("PROCESS_Options", "nail::std_lib::process"), ("PROCESS_Result", "nail::std_lib::process")],
        module: StdlibModule::Process,
        parameters: vec![
            nail_param!(command: s),
            nail_param!(arguments: [s]),
            StdlibParameter { name: "options".to_string(), param_type: NailDataTypeDescriptor::Struct("PROCESS_Options".to_string()), pass_by_reference: false },
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("PROCESS_Result".to_string()))),
        diverging: false,
        description: "Runs a command in another directory, with extra environment variables, with text on its standard input, or with a time limit - whichever of those the options set. A command that runs out of time is killed.",
        example: "ran:PROCESS_Result = danger(process_run_with(`psql`, [`-f`, `-`], options));",
    });

    simple_fns! { m, Process:
        "process_run" [Tokio] => "std_lib::process::run", (command: s, arguments: [s]) -> (s!e),
            "Runs an external command and returns its stdout; errors if the command fails.",
            "output:s = danger(process_run(`ls`, [`-la`]));";
        "process_which" [Tokio] => "std_lib::process::which", (name: s) -> (s!e),
            "Returns where a command would be found on PATH, the way which answers it; errors if there is no such program. What to check before offering a feature that shells out.",
            "git:s = danger(process_which(`git`));";
        "process_wait_for_interrupt" [Tokio] => "std_lib::process::wait_for_interrupt", () -> (v!e),
            "Waits until the program is asked to stop - Ctrl-C, or the TERM signal a service manager sends - and returns when it is. Put it after starting everything, and shut down cleanly afterwards instead of being killed mid-request.",
            "danger(process_wait_for_interrupt());";
        "spawn" [Tokio] => "std_lib::process::spawn", () -> v,
            "Runs a block concurrently in the background (used via the spawn keyword).",
            "spawn { print(`background work`); }";
    }
}
