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
        "markdown_front_matter" [DashMap] => "std_lib::markdown::front_matter", (document: s) -> (h s s),
            "The key: value lines between a pair of --- fences at the top of a document, as a hashmap. A document with no front matter gives an empty one.",
            "meta:h<s,s> = markdown_front_matter(post);";
        "markdown_without_front_matter" => "std_lib::markdown::without_front_matter", (document: s) -> s,
            "The document without its front matter, which is the part to render.",
            "body:s = markdown_without_front_matter(post);";
    }
}
