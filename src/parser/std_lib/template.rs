//! Filling values into text.
//!
//! A page built by concatenating strings is a page where every single value has
//! to be escaped by hand, and the one that gets forgotten is the security hole.
//! So this fills values into a template and escapes them on the way in, and
//! getting raw markup out takes saying so.
//!
//! The syntax is the small subset of Mustache that earns its keep:
//!
//!   `{{name}}`                    the value, escaped for HTML
//!   `{{{name}}}`                  the value with nothing escaped
//!   `{{#if name}}...{{/if}}`      the part between, when the value is set
//!   `{{#if name}}...{{else}}...{{/if}}`
//!   `{{#unless name}}...{{/unless}}`
//!   `{{! anything }}`             a comment, which renders as nothing
//!
//! There is no loop tag, because a loop belongs in the program: Nail already
//! maps over an array better than a template language can, and
//! `template_render_rows` renders one template once per row. There is no
//! expression syntax either - a template that can compute is a program in the
//! wrong file.
//!
//! A name the values do not hold is an error rather than an empty string. A
//! missing value in a page is a bug every time, and it is much cheaper to find
//! when the render says which name was missing than when someone notices the
//! page has a blank in it. When a gap really is acceptable,
//! `template_render_or` fills a missing name with a chosen fallback instead.

use crate::parser::std_lib::string::push_escaped_html;
use dashmap::DashMap;

/// What a template is made of, once read.
enum Node {
    Literal(String),
    /// A value to fill in, escaped unless the template asked for it raw.
    Value {
        name: String,
        escape: bool,
    },
    /// `{{#if}}` or `{{#unless}}`, with the two branches. `unless` is not a
    /// separate node because it is an `if` with the branches the other way
    /// round - which is exactly how it is read below.
    Conditional {
        name: String,
        when_set: Vec<Node>,
        when_not_set: Vec<Node>,
    },
}

/// Whether a value counts as set. Nail's hashmaps hold strings, so a boolean
/// arriving here has already been spelled as one - and `false` reading as true
/// because it is a non-empty string is the kind of thing that ships.
fn is_set(value: &str) -> bool {
    let trimmed = value.trim();
    return !(trimmed.is_empty() || trimmed.eq_ignore_ascii_case("false") || trimmed == "0");
}

/// One tag, as found between `{{` and `}}`.
enum Tag {
    Value { name: String, escape: bool },
    IfOpen { name: String, negated: bool },
    Else,
    IfClose,
    UnlessClose,
    Comment,
}

/// Reads the inside of a tag. The braces are already off; what is left says
/// which kind of tag it is.
fn read_tag(inside: &str, raw: bool) -> Result<Tag, String> {
    let trimmed = inside.trim();
    if trimmed.is_empty() {
        return Err("a tag with nothing in it".to_string());
    }
    if let Some(rest) = trimmed.strip_prefix('!') {
        let _ = rest;
        return Ok(Tag::Comment);
    }
    if trimmed == "else" {
        return Ok(Tag::Else);
    }
    if trimmed == "/if" {
        return Ok(Tag::IfClose);
    }
    if trimmed == "/unless" {
        return Ok(Tag::UnlessClose);
    }
    if let Some(name) = trimmed.strip_prefix("#if ") {
        return Ok(Tag::IfOpen { name: name.trim().to_string(), negated: false });
    }
    if let Some(name) = trimmed.strip_prefix("#unless ") {
        return Ok(Tag::IfOpen { name: name.trim().to_string(), negated: true });
    }
    if trimmed.starts_with('#') || trimmed.starts_with('/') {
        return Err(format!("`{}` is not a tag this understands - the tags are #if, #unless, else, /if and /unless", trimmed));
    }
    if trimmed.contains(char::is_whitespace) {
        return Err(format!("`{}` is not a name - a value tag holds one name and nothing else", trimmed));
    }
    return Ok(Tag::Value { name: trimmed.to_string(), escape: !raw });
}

/// Reads a template into nodes. Called on itself for the inside of an `if`,
/// which is what makes conditionals nest.
///
/// `stop_at` says which closing tag ends this run - `None` at the top level,
/// where a closing tag is a mistake.
fn read_nodes(template: &str, position: &mut usize, stop_at: Option<&str>) -> Result<(Vec<Node>, Option<String>), String> {
    let bytes = template.as_bytes();
    let mut nodes: Vec<Node> = Vec::new();
    let mut literal = String::new();

    while *position < bytes.len() {
        // Everything up to the next `{{` is literal text.
        let next_open = template[*position..].find("{{").map(|offset| *position + offset);
        let open = match next_open {
            Some(open) => open,
            None => {
                literal.push_str(&template[*position..]);
                *position = bytes.len();
                break;
            }
        };
        literal.push_str(&template[*position..open]);

        // Three braces means the value goes in unescaped.
        let raw = template[open..].starts_with("{{{");
        let opening_length = if raw { 3 } else { 2 };
        let closing = if raw { "}}}" } else { "}}" };
        let inside_start = open + opening_length;
        let close = match template[inside_start..].find(closing) {
            Some(offset) => inside_start + offset,
            None => return Err(format!("a tag opened at character {} is never closed with `{}`", open, closing)),
        };
        let tag = read_tag(&template[inside_start..close], raw)?;
        *position = close + closing.len();

        match tag {
            Tag::Comment => {}
            Tag::Value { name, escape } => {
                if !literal.is_empty() {
                    nodes.push(Node::Literal(std::mem::take(&mut literal)));
                }
                nodes.push(Node::Value { name, escape });
            }
            Tag::IfOpen { name, negated } => {
                if !literal.is_empty() {
                    nodes.push(Node::Literal(std::mem::take(&mut literal)));
                }
                let closing_tag = if negated { "/unless" } else { "/if" };
                let (first_branch, ended_with) = read_nodes(template, position, Some(closing_tag))?;
                let mut when_set = first_branch;
                let mut when_not_set: Vec<Node> = Vec::new();
                // An `else` ends the first branch without ending the tag, so the
                // second branch is read the same way.
                if ended_with.as_deref() == Some("else") {
                    let (second_branch, closed_with) = read_nodes(template, position, Some(closing_tag))?;
                    if closed_with.as_deref() != Some(closing_tag) {
                        return Err(format!("a `{{{{{} {}}}}}` is never closed with `{{{{{}}}}}`", if negated { "#unless" } else { "#if" }, name, closing_tag));
                    }
                    when_not_set = second_branch;
                } else if ended_with.as_deref() != Some(closing_tag) {
                    return Err(format!("a `{{{{{} {}}}}}` is never closed with `{{{{{}}}}}`", if negated { "#unless" } else { "#if" }, name, closing_tag));
                }
                if negated {
                    std::mem::swap(&mut when_set, &mut when_not_set);
                }
                nodes.push(Node::Conditional { name, when_set, when_not_set });
            }
            Tag::Else => {
                if stop_at.is_none() {
                    return Err("an `{{else}}` with nothing open before it - an else belongs inside an `{{#if}}` or `{{#unless}}`".to_string());
                }
                if !literal.is_empty() {
                    nodes.push(Node::Literal(literal));
                }
                return Ok((nodes, Some("else".to_string())));
            }
            Tag::IfClose | Tag::UnlessClose => {
                let closing_tag = if matches!(tag, Tag::IfClose) { "/if" } else { "/unless" };
                match stop_at {
                    Some(expected) if expected == closing_tag => {
                        if !literal.is_empty() {
                            nodes.push(Node::Literal(literal));
                        }
                        return Ok((nodes, Some(closing_tag.to_string())));
                    }
                    Some(expected) => return Err(format!("a `{{{{{}}}}}` closes a tag that was opened with the other kind - this one wants `{{{{{}}}}}`", closing_tag, expected)),
                    None => return Err(format!("a `{{{{{}}}}}` with nothing open before it", closing_tag)),
                }
            }
        }
    }

    if !literal.is_empty() {
        nodes.push(Node::Literal(literal));
    }
    return Ok((nodes, None));
}

/// One value into the output, escaped unless the tag asked for it raw.
fn push_value(value: &str, escape: bool, out: &mut String) {
    if escape {
        for character in value.chars() {
            push_escaped_html(character, out);
        }
    } else {
        out.push_str(value);
    }
}

/// Renders read nodes against the values. `fallback` is what a missing value
/// becomes, and `None` makes a missing value the error described at the top of
/// this file.
fn render_nodes(nodes: &[Node], values: &DashMap<String, String>, fallback: Option<&str>, out: &mut String) -> Result<(), String> {
    for node in nodes {
        match node {
            Node::Literal(text) => out.push_str(text),
            Node::Value { name, escape } => match values.get(name) {
                Some(found) => push_value(found.value(), *escape, out),
                None => match fallback {
                    Some(text) => push_value(text, *escape, out),
                    None => return Err(format!("the template uses `{}`, which the values do not have", name)),
                },
            },
            Node::Conditional { name, when_set, when_not_set } => {
                // A name a conditional asks about may simply be absent - that is
                // what "not set" means - so this one does not insist on it.
                let taken = values.get(name).map(|found| is_set(found.value())).unwrap_or(false);
                let branch = if taken { when_set } else { when_not_set };
                render_nodes(branch, values, fallback, out)?;
            }
        }
    }
    return Ok(());
}

/// Fills the values into the template. Values go in HTML-escaped, so a name, a
/// comment or a search term echoed back cannot become part of the markup.
pub fn render(template: String, values: DashMap<String, String>) -> Result<String, String> {
    let mut position = 0;
    let (nodes, ended_with) = read_nodes(&template, &mut position, None).map_err(|detail| format!("template_render: {}", detail))?;
    if let Some(unexpected) = ended_with {
        return Err(format!("template_render: a `{{{{{}}}}}` with nothing open before it", unexpected));
    }
    let mut out = String::with_capacity(template.len() + 64);
    render_nodes(&nodes, &values, None, &mut out).map_err(|detail| format!("template_render: {}", detail))?;
    return Ok(out);
}

/// Fills the values into the template like render, except that a name the
/// values do not have becomes the fallback text instead of an error. For the
/// page where a gap really is acceptable, such as a draft shown before every
/// value exists. The fallback goes through the same escaping as a value. A
/// template that cannot be read comes back unchanged, since nothing in it
/// could be filled.
pub fn render_or(template: String, values: DashMap<String, String>, fallback: String) -> String {
    let mut position = 0;
    let (nodes, ended_with) = match read_nodes(&template, &mut position, None) {
        Ok(read) => read,
        Err(_) => return template,
    };
    if ended_with.is_some() {
        return template;
    }
    let mut out = String::with_capacity(template.len() + 64);
    // With a fallback in hand a missing value cannot fail the render, so this
    // error path exists only to keep the impossible case harmless.
    if render_nodes(&nodes, &values, Some(&fallback), &mut out).is_err() {
        return template;
    }
    return out;
}

/// Renders the same template once for each set of values and joins the results,
/// which is how a table body or a list of cards is built. The template is read
/// once however many rows there are.
pub fn render_rows(template: String, rows: Vec<DashMap<String, String>>) -> Result<String, String> {
    let mut position = 0;
    let (nodes, ended_with) = read_nodes(&template, &mut position, None).map_err(|detail| format!("template_render_rows: {}", detail))?;
    if let Some(unexpected) = ended_with {
        return Err(format!("template_render_rows: a `{{{{{}}}}}` with nothing open before it", unexpected));
    }
    let mut out = String::with_capacity(template.len() * rows.len().max(1));
    for (index, row) in rows.iter().enumerate() {
        render_nodes(&nodes, row, None, &mut out).map_err(|detail| format!("template_render_rows: row {}: {}", index + 1, detail))?;
    }
    return Ok(out);
}

/// Whether the template mentions the named placeholder, in a value tag or as
/// the name a conditional asks about. A template that cannot be read mentions
/// nothing, so a broken one reports false.
pub fn has(template: String, name: String) -> bool {
    let mut position = 0;
    let nodes = match read_nodes(&template, &mut position, None) {
        Ok((nodes, _)) => nodes,
        Err(_) => return false,
    };
    let mut found: Vec<String> = Vec::new();
    collect_names(&nodes, &mut found);
    return found.contains(&name);
}

/// The names a template asks for, so a program can check it holds them before
/// rendering - what a test does with a template it did not write.
pub fn names_used(template: String) -> Result<Vec<String>, String> {
    let mut position = 0;
    let (nodes, _) = read_nodes(&template, &mut position, None).map_err(|detail| format!("template_names_used: {}", detail))?;
    let mut found: Vec<String> = Vec::new();
    collect_names(&nodes, &mut found);
    found.dedup();
    return Ok(found);
}

fn collect_names(nodes: &[Node], found: &mut Vec<String>) {
    for node in nodes {
        match node {
            Node::Literal(_) => {}
            Node::Value { name, .. } => {
                if !found.contains(name) {
                    found.push(name.clone());
                }
            }
            Node::Conditional { name, when_set, when_not_set } => {
                if !found.contains(name) {
                    found.push(name.clone());
                }
                collect_names(when_set, found);
                collect_names(when_not_set, found);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&str, &str)]) -> DashMap<String, String> {
        let map = DashMap::new();
        for (key, value) in pairs {
            map.insert(key.to_string(), value.to_string());
        }
        return map;
    }

    #[test]
    fn a_value_is_filled_in() {
        let out = render("Hello, {{name}}!".to_string(), values(&[("name", "Alex")])).expect("a fillable template");
        assert_eq!(out, "Hello, Alex!");
    }

    #[test]
    fn a_template_with_no_tags_comes_back_as_itself() {
        assert_eq!(render("just text".to_string(), values(&[])).expect("nothing to fill"), "just text");
        assert_eq!(render(String::new(), values(&[])).expect("nothing at all"), "");
    }

    /// The reason to use a template rather than concatenation: this escaping is
    /// not something anyone has to remember.
    #[test]
    fn values_are_escaped_on_the_way_in() {
        let out = render("<p>{{comment}}</p>".to_string(), values(&[("comment", "<script>alert('x')</script>")])).expect("a fillable template");
        assert!(!out.contains("<script>"), "got: {}", out);
        assert_eq!(out, "<p>&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;</p>");
    }

    #[test]
    fn three_braces_put_the_value_in_as_markup() {
        let out = render("<main>{{{body}}}</main>".to_string(), values(&[("body", "<p>already markup</p>")])).expect("a fillable template");
        assert_eq!(out, "<main><p>already markup</p></main>");
    }

    #[test]
    fn a_name_the_values_do_not_have_is_an_error_naming_it() {
        let failure = render("Hello, {{name}}!".to_string(), values(&[("other", "x")])).unwrap_err();
        assert!(failure.contains("`name`"), "got: {}", failure);
        assert!(failure.contains("the values do not have"), "got: {}", failure);
    }

    #[test]
    fn a_conditional_keeps_or_drops_the_part_between() {
        let template = "{{#if admin}}<a href=\"/admin\">Admin</a>{{/if}}done".to_string();
        assert_eq!(render(template.clone(), values(&[("admin", "true")])).expect("fillable"), "<a href=\"/admin\">Admin</a>done");
        assert_eq!(render(template.clone(), values(&[("admin", "false")])).expect("fillable"), "done");
        assert_eq!(render(template.clone(), values(&[("admin", "")])).expect("fillable"), "done");
        // A name a conditional asks about may be absent entirely.
        assert_eq!(render(template, values(&[])).expect("fillable"), "done");
    }

    #[test]
    fn a_value_spelled_false_or_zero_is_not_set() {
        let template = "{{#if flag}}yes{{else}}no{{/if}}".to_string();
        assert_eq!(render(template.clone(), values(&[("flag", "false")])).expect("fillable"), "no");
        assert_eq!(render(template.clone(), values(&[("flag", "FALSE")])).expect("fillable"), "no");
        assert_eq!(render(template.clone(), values(&[("flag", "0")])).expect("fillable"), "no");
        assert_eq!(render(template.clone(), values(&[("flag", "  ")])).expect("fillable"), "no");
        assert_eq!(render(template, values(&[("flag", "1")])).expect("fillable"), "yes");
    }

    #[test]
    fn an_else_branch_is_taken_when_the_value_is_not_set() {
        let template = "{{#if name}}Hello, {{name}}{{else}}Hello, stranger{{/if}}".to_string();
        assert_eq!(render(template.clone(), values(&[("name", "Alex")])).expect("fillable"), "Hello, Alex");
        assert_eq!(render(template, values(&[("name", "")])).expect("fillable"), "Hello, stranger");
    }

    #[test]
    fn unless_is_an_if_the_other_way_round() {
        let template = "{{#unless logged_in}}<a href=\"/login\">Log in</a>{{/unless}}".to_string();
        assert_eq!(render(template.clone(), values(&[("logged_in", "")])).expect("fillable"), "<a href=\"/login\">Log in</a>");
        assert_eq!(render(template, values(&[("logged_in", "true")])).expect("fillable"), "");
    }

    #[test]
    fn unless_takes_an_else_too() {
        let template = "{{#unless empty}}has items{{else}}nothing here{{/unless}}".to_string();
        assert_eq!(render(template.clone(), values(&[("empty", "")])).expect("fillable"), "has items");
        assert_eq!(render(template, values(&[("empty", "yes")])).expect("fillable"), "nothing here");
    }

    #[test]
    fn conditionals_nest() {
        let template = "{{#if outer}}[{{#if inner}}both{{else}}outer only{{/if}}]{{/if}}".to_string();
        assert_eq!(render(template.clone(), values(&[("outer", "1"), ("inner", "1")])).expect("fillable"), "[both]");
        assert_eq!(render(template.clone(), values(&[("outer", "1"), ("inner", "")])).expect("fillable"), "[outer only]");
        assert_eq!(render(template, values(&[("outer", ""), ("inner", "1")])).expect("fillable"), "");
    }

    /// A value inside a branch that is not taken is never looked up, so a
    /// template can guard its own values.
    #[test]
    fn a_branch_not_taken_needs_none_of_its_values() {
        let template = "{{#if signed_in}}Hello, {{user_name}}{{/if}}".to_string();
        assert_eq!(render(template, values(&[("signed_in", "")])).expect("fillable"), "");
    }

    #[test]
    fn a_comment_renders_as_nothing() {
        assert_eq!(render("a{{! this is a note }}b".to_string(), values(&[])).expect("fillable"), "ab");
    }

    #[test]
    fn a_tag_that_is_never_closed_says_so() {
        assert!(render("Hello, {{name".to_string(), values(&[("name", "x")])).unwrap_err().contains("never closed"));
        assert!(render("{{#if a}}text".to_string(), values(&[("a", "1")])).unwrap_err().contains("never closed"));
    }

    #[test]
    fn a_closing_tag_with_nothing_open_says_so() {
        assert!(render("text{{/if}}".to_string(), values(&[])).unwrap_err().contains("nothing open"));
        assert!(render("text{{else}}more".to_string(), values(&[])).unwrap_err().contains("nothing open"));
    }

    #[test]
    fn the_two_kinds_of_conditional_cannot_close_each_other() {
        let failure = render("{{#if a}}text{{/unless}}".to_string(), values(&[("a", "1")])).unwrap_err();
        assert!(failure.contains("the other kind"), "got: {}", failure);
    }

    #[test]
    fn a_tag_that_is_not_a_tag_says_what_the_tags_are() {
        let failure = render("{{#each items}}x{{/each}}".to_string(), values(&[])).unwrap_err();
        assert!(failure.contains("#if, #unless, else, /if and /unless"), "got: {}", failure);
        assert!(render("{{two names}}".to_string(), values(&[])).unwrap_err().contains("holds one name"));
        assert!(render("{{}}".to_string(), values(&[])).unwrap_err().contains("nothing in it"));
    }

    #[test]
    fn rows_render_the_same_template_once_each() {
        let template = "<tr><td>{{name}}</td><td>{{score}}</td></tr>".to_string();
        let rows = vec![values(&[("name", "Alex"), ("score", "10")]), values(&[("name", "Sam"), ("score", "8")])];
        let out = render_rows(template, rows).expect("fillable rows");
        assert_eq!(out, "<tr><td>Alex</td><td>10</td></tr><tr><td>Sam</td><td>8</td></tr>");
    }

    #[test]
    fn no_rows_render_nothing() {
        assert_eq!(render_rows("<tr>{{name}}</tr>".to_string(), vec![]).expect("no rows"), "");
    }

    #[test]
    fn a_row_missing_a_value_says_which_row() {
        let rows = vec![values(&[("name", "Alex")]), values(&[])];
        let failure = render_rows("{{name}}".to_string(), rows).unwrap_err();
        assert!(failure.contains("row 2"), "got: {}", failure);
    }

    #[test]
    fn the_names_a_template_uses_can_be_listed() {
        let template = "{{title}} {{#if admin}}{{admin_name}}{{else}}{{guest_name}}{{/if}} {{title}}".to_string();
        let names = names_used(template).expect("a readable template");
        assert_eq!(names, vec!["title".to_string(), "admin".to_string(), "admin_name".to_string(), "guest_name".to_string()]);
    }

    #[test]
    fn listing_names_of_a_broken_template_is_an_error() {
        assert!(names_used("{{unclosed".to_string()).is_err());
    }

    #[test]
    fn has_reports_the_names_a_template_mentions() {
        assert!(has("Hello, {{name}}!".to_string(), "name".to_string()));
        assert!(has("{{#if admin}}x{{/if}}".to_string(), "admin".to_string()));
        assert!(has("{{#if admin}}{{admin_name}}{{/if}}".to_string(), "admin_name".to_string()));
        assert!(has("<main>{{{body}}}</main>".to_string(), "body".to_string()));
        assert!(!has("Hello, {{name}}!".to_string(), "other".to_string()));
        assert!(!has("plain text".to_string(), "name".to_string()));
    }

    #[test]
    fn a_broken_template_mentions_nothing() {
        assert!(!has("{{unclosed".to_string(), "unclosed".to_string()));
    }

    #[test]
    fn render_or_fills_a_missing_name_with_the_fallback() {
        let out = render_or("Hello, {{name}}!".to_string(), values(&[]), "stranger".to_string());
        assert_eq!(out, "Hello, stranger!");
    }

    #[test]
    fn render_or_still_prefers_the_value_it_has() {
        let out = render_or("Hello, {{name}}!".to_string(), values(&[("name", "Alex")]), "stranger".to_string());
        assert_eq!(out, "Hello, Alex!");
    }

    #[test]
    fn render_or_escapes_the_fallback_the_same_as_a_value() {
        let escaped = render_or("<p>{{name}}</p>".to_string(), values(&[]), "<anon>".to_string());
        assert_eq!(escaped, "<p>&lt;anon&gt;</p>");
        let raw = render_or("<main>{{{body}}}</main>".to_string(), values(&[]), "<p>x</p>".to_string());
        assert_eq!(raw, "<main><p>x</p></main>");
    }

    #[test]
    fn render_or_leaves_conditionals_to_their_own_rules() {
        let template = "{{#if admin}}yes{{else}}no{{/if}}".to_string();
        assert_eq!(render_or(template, values(&[]), "FALLBACK".to_string()), "no");
    }

    #[test]
    fn render_or_gives_a_broken_template_back_unchanged() {
        assert_eq!(render_or("Hello, {{name".to_string(), values(&[]), "x".to_string()), "Hello, {{name");
        assert_eq!(render_or("text{{/if}}".to_string(), values(&[]), "x".to_string()), "text{{/if}}");
    }
}
