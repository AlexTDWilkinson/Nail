//! Terminal module stdlib registry entries.

use super::*;

/// The colouring functions all have the same shape - text in, styled text out,
/// one TERM_Color - and all need the enum imported, so they are built from one
/// description rather than eight copies of the same struct literal.
fn color_fn(rust_path: &str, description: &'static str, example: &'static str) -> StdlibFunction {
    return StdlibFunction {
        rust_path: rust_path.to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("TERM_Color", "nail::std_lib::term")],
        module: StdlibModule::Term,
        parameters: vec![nail_param!(text: s), StdlibParameter { name: "color".to_string(), param_type: NailDataTypeDescriptor::Enum("TERM_Color".to_string()), pass_by_reference: false }],
        return_type: nail_type!(s),
        diverging: false,
        description,
        example,
    };
}

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Term:
        "term_bold" => "std_lib::term::bold", (text: s) -> s,
            "Returns the text wrapped in the escape codes that make a terminal show it bold.",
            "heading:s = term_bold(`Results`);";
        "term_dim" => "std_lib::term::dim", (text: s) -> s,
            "Returns the text wrapped in the escape codes that make a terminal show it faintly.",
            "hint:s = term_dim(`press q to quit`);";
        "term_italic" => "std_lib::term::italic", (text: s) -> s,
            "Returns the text wrapped in the escape codes that make a terminal show it italic.",
            "quote:s = term_italic(`as written`);";
        "term_underline" => "std_lib::term::underline", (text: s) -> s,
            "Returns the text wrapped in the escape codes that make a terminal underline it.",
            "link_text:s = term_underline(`documentation`);";
        "term_inverse" => "std_lib::term::inverse", (text: s) -> s,
            "Returns the text with foreground and background swapped, which is what a selected row looks like.",
            "selected:s = term_inverse(row);";
        "term_strip_styles" => "std_lib::term::strip_styles", (text: s) -> s,
            "Removes every escape sequence, leaving the text as it will be read. Use it before writing coloured output anywhere that is not a terminal.",
            "plain:s = term_strip_styles(colored);";
        "term_display_width" => "std_lib::term::display_width", (text: s) -> i,
            "Returns how wide the text is once printed, counting what a person sees rather than the characters in the string.",
            "columns:i = term_display_width(cell);";
        "term_is_tty" => "std_lib::term::is_tty", () -> b,
            "Returns whether standard output is a terminal rather than a file or a pipe. False means do not colour and do not draw progress.",
            "interactive:b = term_is_tty();";
        "term_width" [Crossterm] => "std_lib::term::width", () -> i,
            "Returns how many columns the terminal has, or 80 when there is no terminal to ask.",
            "columns:i = term_width();";
        "term_height" [Crossterm] => "std_lib::term::height", () -> i,
            "Returns how many rows the terminal has, or 24 when there is no terminal to ask.",
            "rows:i = term_height();";
        "term_hyperlink" => "std_lib::term::hyperlink", (text: s, url: s) -> s,
            "Returns a clickable link where the terminal supports them, and the plain text where it does not.",
            "link:s = term_hyperlink(`Nail`, `https://nail-lang.org`);";
        "term_progress_bar" => "std_lib::term::progress_bar", (share: f, width: i) -> (s!e),
            "Returns a progress bar of the given width filled to the given share from 0.0 to 1.0.",
            "bar:s = danger(term_progress_bar(0.42, 30));";
        "term_table" => "std_lib::term::table", (headers: [s], rows: [[s]]) -> (s!e),
            "Returns a plain-text table with aligned columns. Errors if a row has a different number of cells than there are headers.",
            "rendered:s = danger(term_table(headers, rows));";
    }

    m.insert("term_paint", color_fn("std_lib::term::paint", "Returns the text in the given colour.", "warning:s = term_paint(`careful`, TERM_Color::Yellow);"));
    m.insert("term_background", color_fn("std_lib::term::background", "Returns the text on the given background colour.", "banner:s = term_background(`FAIL`, TERM_Color::Red);"));
}
