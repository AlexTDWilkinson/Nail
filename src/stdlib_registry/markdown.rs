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
        "markdown_to_text" [Pulldown] => "std_lib::markdown::to_text", (markdown: s) -> s,
            "Strips a Markdown document to plain text: headings, bold and links keep their text, code blocks keep their code, list items keep their lines.",
            "plain:s = markdown_to_text(doc);";
        "markdown_links" [Pulldown] => "std_lib::markdown::links", (markdown: s) -> [s],
            "Every link destination in the document, in the order the links appear.",
            "urls:a:s = markdown_links(doc);";
        "markdown_headings" [Pulldown] => "std_lib::markdown::headings", (markdown: s) -> [s],
            "Every heading's text in the document, in order, whatever its level.",
            "titles:a:s = markdown_headings(doc);";
        "markdown_toc" [Pulldown] => "std_lib::markdown::toc", (markdown: s) -> s,
            "A Markdown bullet list of the document's headings, indented two spaces per level below the top, each linking to its GitHub-style anchor.",
            "contents:s = markdown_toc(doc);";
        "markdown_word_count" [Pulldown] => "std_lib::markdown::word_count", (markdown: s) -> i,
            "How many words the document's plain text holds, with the formatting not counted.",
            "words:i = markdown_word_count(doc);";
        "markdown_front_matter" [DashMap] => "std_lib::markdown::front_matter", (document: s) -> (h s s),
            "The key: value lines between a pair of --- fences at the top of a document, as a hashmap. A document with no front matter gives an empty one.",
            "meta:h<s,s> = markdown_front_matter(post);";
        "markdown_without_front_matter" => "std_lib::markdown::without_front_matter", (document: s) -> s,
            "The document without its front matter, which is the part to render.",
            "body:s = markdown_without_front_matter(post);";
    }
}
