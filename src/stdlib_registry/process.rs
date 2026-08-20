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
        return_type: nail_type!(v),
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
        example: "options:PROCESS_Options = process_default_options();\nran:PROCESS_Result = danger(process_run_with(`psql`, [`-f`, `-`], options));",
    });

    simple_fns! { m, Process:
        "process_run" [Tokio] => "std_lib::process::run", (command: s, arguments: [s]) -> (s!e),
            "Runs an external command and returns its stdout. Errors if the command fails.",
            "output:s = danger(process_run(`ls`, [`-la`]));";
        "process_which" [Tokio] => "std_lib::process::which", (name: s) -> (s!e),
            "Returns where a command would be found on PATH, the way which answers it. Errors if there is no such program. What to check before offering a feature that shells out.",
            "git:s = danger(process_which(`git`));";
        "process_wait_for_interrupt" [Tokio] => "std_lib::process::wait_for_interrupt", () -> (v!e),
            "Waits until the program is asked to stop - Ctrl-C, or the TERM signal a service manager sends - and returns when it is. Put it after starting everything, and shut down cleanly afterwards instead of being killed mid-request.",
            "danger(process_wait_for_interrupt());";
        "process_open_browser" [Tokio] => "std_lib::process::open_browser", (url: s) -> (v!e),
            "Opens a URL in the person's browser through the desktop's own opener. For local tools that want to show the page they just made.",
            "danger(process_open_browser(`http://localhost:8080`));";
    }

    let handle_parameter = || StdlibParameter { name: "process".to_string(), param_type: NailDataTypeDescriptor::Struct("PROCESS_Handle".to_string()), pass_by_reference: true };
    let handle_import = || vec![("PROCESS_Handle", "nail::std_lib::process")];
    let streaming_deps = || vec![CrateDependency::Tokio, CrateDependency::DashMap, CrateDependency::Uuid, CrateDependency::Serde];

    m.insert("process_spawn", StdlibFunction {
        rust_path: "std_lib::process::spawn_process".to_string(),
        crate_deps: streaming_deps(),
        struct_derives: vec![],
        custom_type_imports: handle_import(),
        module: StdlibModule::Process,
        parameters: vec![nail_param!(command: s), nail_param!(arguments: [s])],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("PROCESS_Handle".to_string()))),
        diverging: false,
        description: "Starts a program and keeps it running - what process_run cannot do, because it collects everything at the end. Output streams out through process_next_line. process_wait collects the exit code.",
        example: "input_path:s = `clip.mov`;\noutput_path:s = `clip.mp4`;\nffmpeg:PROCESS_Handle = danger(process_spawn(`ffmpeg`, [`-i`, input_path, output_path]));",
    });

    m.insert("process_next_line", StdlibFunction {
        rust_path: "std_lib::process::next_line".to_string(),
        crate_deps: streaming_deps(),
        struct_derives: vec![],
        custom_type_imports: handle_import(),
        module: StdlibModule::Process,
        parameters: vec![handle_parameter()],
        return_type: nail_type!((s!e)),
        diverging: false,
        description: "The next line the process printed, stdout and stderr together in arrival order. Waits for one if none is ready. An error means the output is over. The shape of a tail loop is: ask for lines with safe(), stop on the error.",
        example: "ffmpeg:PROCESS_Handle = danger(process_spawn(`ffmpeg`, [`-i`, `clip.mov`, `clip.mp4`]));\nline:s = danger(process_next_line(ffmpeg));",
    });

    m.insert("process_write_stdin", StdlibFunction {
        rust_path: "std_lib::process::write_stdin".to_string(),
        crate_deps: streaming_deps(),
        struct_derives: vec![],
        custom_type_imports: handle_import(),
        module: StdlibModule::Process,
        parameters: vec![handle_parameter(), nail_param!(text: s)],
        return_type: nail_type!((v!e)),
        diverging: false,
        description: "Writes text to the process's stdin, exactly as given - add a newline yourself when the program reads lines.",
        example: "repl:PROCESS_Handle = danger(process_spawn(`python3`, [`-i`]));\ndanger(process_write_stdin(repl, `help\\n`));",
    });

    m.insert("process_close_stdin", StdlibFunction {
        rust_path: "std_lib::process::close_stdin".to_string(),
        crate_deps: streaming_deps(),
        struct_derives: vec![],
        custom_type_imports: handle_import(),
        module: StdlibModule::Process,
        parameters: vec![handle_parameter()],
        return_type: nail_type!((v!e)),
        diverging: false,
        description: "Closes the process's stdin - the end-of-input many programs wait for before finishing.",
        example: "sort:PROCESS_Handle = danger(process_spawn(`sort`, []));\ndanger(process_write_stdin(sort, `b\\na\\n`));\ndanger(process_close_stdin(sort));",
    });

    m.insert("process_is_running", StdlibFunction {
        rust_path: "std_lib::process::is_running".to_string(),
        crate_deps: streaming_deps(),
        struct_derives: vec![],
        custom_type_imports: handle_import(),
        module: StdlibModule::Process,
        parameters: vec![handle_parameter()],
        return_type: nail_type!((b!e)),
        diverging: false,
        description: "Whether the process is still going.",
        example: "ffmpeg:PROCESS_Handle = danger(process_spawn(`ffmpeg`, [`-i`, `clip.mov`, `clip.mp4`]));\nalive:b = danger(process_is_running(ffmpeg));",
    });

    m.insert("process_wait", StdlibFunction {
        rust_path: "std_lib::process::wait_process".to_string(),
        crate_deps: streaming_deps(),
        struct_derives: vec![],
        custom_type_imports: handle_import(),
        module: StdlibModule::Process,
        parameters: vec![handle_parameter()],
        return_type: nail_type!((i!e)),
        diverging: false,
        description: "Waits for the process to end and returns its exit code. Read the lines you want first - waiting forgets the handle, and any unread output with it.",
        example: "ffmpeg:PROCESS_Handle = danger(process_spawn(`ffmpeg`, [`-i`, `clip.mov`, `clip.mp4`]));\ncode:i = danger(process_wait(ffmpeg));",
    });

    m.insert("process_kill", StdlibFunction {
        rust_path: "std_lib::process::kill_process".to_string(),
        crate_deps: streaming_deps(),
        struct_derives: vec![],
        custom_type_imports: handle_import(),
        module: StdlibModule::Process,
        parameters: vec![handle_parameter()],
        return_type: nail_type!((v!e)),
        diverging: false,
        description: "Stops the process now and forgets its handle.",
        example: "stuck_job:PROCESS_Handle = danger(process_spawn(`sleep`, [`3600`]));\ndanger(process_kill(stuck_job));",
    });
}
