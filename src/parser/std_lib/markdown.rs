use pulldown_cmark::{Parser, Options, html};

pub fn to_html(markdown: String) -> String {
    let parser = Parser::new(&markdown);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

pub fn to_html_with_options(markdown: String, enable_tables: bool, enable_footnotes: bool, enable_strikethrough: bool) -> String {
    let mut options = Options::empty();
    if enable_tables {
        options.insert(Options::ENABLE_TABLES);
    }
    if enable_footnotes {
        options.insert(Options::ENABLE_FOOTNOTES);
    }
    if enable_strikethrough {
        options.insert(Options::ENABLE_STRIKETHROUGH);
    }
    
    let parser = Parser::new_ext(&markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}
/// The front matter of a document: the `key: value` lines between a pair of
/// `---` fences at the very top, which is how every static site generator
/// carries a post's title and date alongside its text.
///
/// The values are read as text and nothing else - no nested structures, no
/// lists, no types. That is not YAML, and it is not trying to be: front matter
/// that needs more shape than this wants to be a TOML or JSON file beside the
/// document. A document with no front matter gives an empty hashmap rather than
/// an error, since most documents have none.
pub fn front_matter(document: String) -> dashmap::DashMap<String, String> {
    let values = dashmap::DashMap::new();
    let Some(block) = front_matter_block(&document) else {
        return values;
    };

    for line in block.lines() {
        let trimmed = line.trim();
        // A blank line or a comment inside the block is skipped rather than
        // being an error, because both are ordinary in a hand-edited file.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            // Quotes around a value are how a title with a colon in it is
            // written, so they are taken off rather than kept.
            let cleaned = value.trim().trim_matches('"').trim_matches('\'').to_string();
            values.insert(key.trim().to_string(), cleaned);
        }
    }
    return values;
}

/// The document without its front matter, which is the part to render. A
/// document with no front matter comes back unchanged.
pub fn without_front_matter(document: String) -> String {
    return match front_matter_block(&document) {
        Some(block) => {
            // Past the block, its fences, and the newline after the closing one.
            let after = document.find(&block).map(|start| start + block.len()).unwrap_or(0);
            let rest = &document[after..];
            let rest = rest.trim_start_matches(|character| character == '-' || character == '\r');
            rest.strip_prefix('\n').unwrap_or(rest).to_string()
        }
        None => document,
    };
}

/// The text between the opening and closing fences, if the document opens with
/// one. The fence must be the first thing in the document - a `---` further down
/// is a horizontal rule, and treating it as front matter would eat the text
/// above it.
fn front_matter_block(document: &str) -> Option<String> {
    let after_opening = document.strip_prefix("---\n").or_else(|| document.strip_prefix("---\r\n"))?;
    let end = after_opening.find("\n---")?;
    return Some(after_opening[..end + 1].to_string());
}

#[cfg(test)]
mod front_matter_tests {
    use super::*;

    const POST: &str = "---\ntitle: A Post\ndate: 2026-08-04\ndraft: false\n---\n# Heading\n\nThe text of the post.\n";

    #[test]
    fn the_values_between_the_fences_are_read() {
        let values = front_matter(POST.to_string());
        assert_eq!(values.get("title").expect("a title").value(), "A Post");
        assert_eq!(values.get("date").expect("a date").value(), "2026-08-04");
        assert_eq!(values.get("draft").expect("a flag").value(), "false");
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn the_body_is_what_is_left() {
        assert_eq!(without_front_matter(POST.to_string()), "# Heading\n\nThe text of the post.\n");
    }

    #[test]
    fn a_document_with_no_front_matter_is_left_alone() {
        let plain = "# Heading\n\nJust text.\n".to_string();
        assert_eq!(without_front_matter(plain.clone()), plain);
        assert!(front_matter(plain).is_empty());
    }

    /// A rule partway down a document is a rule, not a fence.
    #[test]
    fn a_horizontal_rule_further_down_is_not_front_matter() {
        let with_rule = "# Heading\n\n---\n\nBelow the rule.\n".to_string();
        assert_eq!(without_front_matter(with_rule.clone()), with_rule);
        assert!(front_matter(with_rule).is_empty());
    }

    #[test]
    fn quotes_around_a_value_are_taken_off() {
        let quoted = "---\ntitle: \"A Post: With a Colon\"\nauthor: 'Alex'\n---\nbody\n".to_string();
        let values = front_matter(quoted);
        assert_eq!(values.get("title").expect("a title").value(), "A Post: With a Colon");
        assert_eq!(values.get("author").expect("an author").value(), "Alex");
    }

    #[test]
    fn blank_lines_and_comments_in_the_block_are_skipped() {
        let messy = "---\n# which post this is\ntitle: A Post\n\ndate: 2026-08-04\n---\nbody\n".to_string();
        let values = front_matter(messy);
        assert_eq!(values.len(), 2);
        assert_eq!(values.get("title").expect("a title").value(), "A Post");
    }

    #[test]
    fn front_matter_and_body_together_are_the_whole_document() {
        let values = front_matter(POST.to_string());
        assert!(!values.is_empty());
        let body = without_front_matter(POST.to_string());
        assert!(!body.contains("title:"), "got: {}", body);
        assert!(body.starts_with("# Heading"));
    }
}
