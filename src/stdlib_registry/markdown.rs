//! Markdown module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Markdown:
        "markdown_to_html" [Pulldown] => "std_lib::markdown::to_html", (markdown: s) -> s,
            "Converts a Markdown string to HTML.",
            "html:s = markdown_to_html(`# Title`);";
        "markdown_to_html_with_options" [Pulldown] => "std_lib::markdown::to_html_with_options", (markdown: s, enable_tables: b, enable_footnotes: b, enable_strikethrough: b) -> s,
            "Converts Markdown to HTML with tables, footnotes, and strikethrough toggled individually.",
            "html:s = markdown_to_html_with_options(doc, true, false, true);";
    }
}
