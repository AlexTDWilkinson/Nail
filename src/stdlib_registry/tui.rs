//! Terminal interface module stdlib registry entries.
//!
//! `tui_run` is the only stdlib function that calls back into the program
//! twice - once for `view` and once for `update` - and the only one whose
//! callbacks are written in terms of a type the registry cannot name. Both
//! signatures use the type variable `T`, which `tui_run`'s own argument binds
//! to whatever struct the program uses for its state (see HANDLER_CALLBACKS).

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("tui_run", StdlibFunction {
        rust_path: "std_lib::tui::run".to_string(),
        crate_deps: vec![CrateDependency::Crossterm, CrateDependency::Tokio],
        struct_derives: vec![],
        custom_type_imports: vec![("TUI_Screen", "nail::std_lib::tui"), ("TUI_Line", "nail::std_lib::tui"), ("TUI_Event", "nail::std_lib::tui"), ("TERM_Color", "nail::std_lib::term")],
        module: StdlibModule::Tui,
        parameters: vec![nail_param!(initial: T)],
        return_type: nail_type!((T!e)),
        diverging: false,
        description: "Runs a full-screen terminal program until its view reports quit, and returns the state it finished with. The program supplies two functions - view(state) and update(state, event) - and this owns raw mode, input, redrawing, resizing and putting the terminal back, including when the program panics.",
        example: "struct App { count:i }\n\nf view(state:App):TUI_Screen {\n    total:TUI_Line = tui_line(string_concat([`count: `, string_from(state.count)]));\n    r TUI_Screen { title = `Counter`, lines = [total], status = `q quits`, quit = false };\n}\n\nf update(state:App, event:TUI_Event):App {\n    r App { count = state.count + 1 };\n}\n\nfinal_state:App = danger(tui_run(App { count = 0 }));",
    });

    m.insert("tui_line", StdlibFunction {
        rust_path: "std_lib::tui::line".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("TUI_Line", "nail::std_lib::tui"), ("TERM_Color", "nail::std_lib::term")],
        module: StdlibModule::Tui,
        parameters: vec![nail_param!(text: s)],
        return_type: NailDataTypeDescriptor::Struct("TUI_Line".to_string()),
        diverging: false,
        description: "A plain line of the screen, in the terminal's own colour.",
        example: "row:TUI_Line = tui_line(`hello`);",
    });

    m.insert("tui_styled", StdlibFunction {
        rust_path: "std_lib::tui::styled".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("TUI_Line", "nail::std_lib::tui"), ("TERM_Color", "nail::std_lib::term")],
        module: StdlibModule::Tui,
        parameters: vec![
            nail_param!(text: s),
            StdlibParameter { name: "color".to_string(), param_type: NailDataTypeDescriptor::Enum("TERM_Color".to_string()), pass_by_reference: false },
            nail_param!(bold: b),
            nail_param!(selected: b),
        ],
        return_type: NailDataTypeDescriptor::Struct("TUI_Line".to_string()),
        diverging: false,
        description: "A line with everything about its appearance said explicitly. A selected line is drawn with the foreground and background swapped, which is what a chosen row in a list looks like.",
        example: "row:TUI_Line = tui_styled(`hello`, TERM_Color::Cyan, true, false);",
    });
}
