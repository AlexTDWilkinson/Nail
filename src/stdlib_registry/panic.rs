//! Panic module stdlib registry entries - diverging functions that abort the
//! program. Implementations live in src/parser/std_lib/panic.rs.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("panic", StdlibFunction {
        rust_path: "std_lib::panic::panic".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Panic,
        parameters: vec![nail_param!(message: s)],
        return_type: nail_type!(never),
        diverging: true,
        description: "Prints the message to stderr and aborts the program immediately. Never returns.",
        example: "panic(`unreachable state`);",
    });

    m.insert("todo", StdlibFunction {
        rust_path: "std_lib::panic::todo".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Panic,
        parameters: vec![nail_param!(message: s)],
        return_type: nail_type!(never),
        diverging: true,
        description: "Marks unfinished code: prints the message to stderr and aborts. Never returns.",
        example: "todo(`implement checkout flow`);",
    });
}
