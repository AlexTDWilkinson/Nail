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

    simple_fns! { m, Process:
        "process_run" [Tokio] => "std_lib::process::run", (command: s, arguments: [s]) -> (s!e),
            "Runs an external command and returns its stdout; errors if the command fails.",
            "output:s = danger(process_run(`ls`, [`-la`]));";
        "spawn" [Tokio] => "std_lib::process::spawn", () -> v,
            "Runs a block concurrently in the background (used via the spawn keyword).",
            "spawn { print(`background work`); }";
    }
}
