//! Code module stdlib registry entries: Nail source highlighting and transpilation.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Code:
        "code_highlight_html" => "std_lib::code::highlight_html", (source: s) -> s,
            "Highlights Nail source as HTML using the real Nail lexer; wraps tokens in <span class=\"tok-*\"> elements for use inside <pre>.",
            "html:s = code_highlight_html(nail_source);";
        "code_transpile_to_rust" => "std_lib::code::transpile_to_rust", (source: s) -> (s!e),
            "Runs the full Nail compiler pipeline (lex, parse, type check, transpile) on a source string and returns the generated Rust code.",
            "rust_code:s = danger(code_transpile_to_rust(nail_source));";
        "code_escape_html" => "std_lib::code::escape_html", (text: s) -> s,
            "Escapes &, < and > so arbitrary text can be embedded safely in HTML.",
            "safe_text:s = code_escape_html(user_input);";
    }
}
