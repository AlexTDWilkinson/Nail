//! Tokenizing the inside of a tagged string.
//!
//! A string literal carrying a language tag - html`<p>hi</p>`, css`.hero {}`,
//! js`const x = 1;` - holds another language, and Nail's highlighters want to
//! color it. The scanning lives here, one module per language, so there is a
//! single implementation of what an element name or a CSS property is and the
//! callers only decide how to paint the pieces it hands back.

pub mod css;
pub mod generic;
pub mod markdown;
pub mod markup;
pub mod toml;
pub mod yaml;

/// The kinds of thing an embedded language is made of. One vocabulary covers
/// all of them, so a caller picks a color per piece once and every language
/// added later lands in a palette it already has.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Piece {
    /// Brackets and separators: `<`, `>`, `{`, `;`, `,`.
    Bracket,
    /// The thing being described: an element name in markup, a selector in CSS.
    Element,
    /// A name hanging off it: an attribute in markup, a property in CSS, a
    /// member after a `.` in a script.
    Attribute,
    /// A function name, in a script or in a CSS value like `rgb(...)`.
    Function,
    /// A reserved word: `const` in a script, `@media` or `!important` in CSS.
    Keyword,
    /// `=`, `:`, and the arithmetic and comparison operators.
    Operator,
    /// A quoted string, quotes included.
    Value,
    /// A number, including a CSS unit or hex color.
    Number,
    /// A comment, its delimiters included.
    Comment,
    /// Everything else: the text between tags, whitespace, plain words.
    Text,
}

/// How far into a language's syntax a line ended. A tag whose attributes run
/// onto the next line, a CSS block, a `/* ... */` - all of them stay open
/// across the break, so this travels with the string from one line to the next.
#[derive(Clone, PartialEq, Debug)]
pub enum State {
    Markup(markup::State),
    Css(css::State),
    Generic(generic::State),
    Yaml(yaml::State),
    Toml(toml::State),
    Markdown(markdown::State),
}

/// The state a string tagged `tag` starts in, or `None` for a tag no tokenizer
/// here knows. An unknown tag degrades to a plain string rather than being
/// mangled by the wrong tokenizer, so an unlisted language is never worse off
/// than it was before this module existed.
pub fn state_for_tag(tag: &str) -> Option<State> {
    return match tag {
        "html" | "htm" | "xhtml" | "svg" | "xml" | "rss" | "atom" => Some(State::Markup(markup::start())),
        "css" | "scss" | "sass" | "less" => Some(State::Css(css::start())),
        "yaml" | "yml" => Some(State::Yaml(yaml::start())),
        "toml" | "ini" | "cfg" | "conf" | "properties" => Some(State::Toml(toml::start())),
        "md" | "markdown" => Some(State::Markdown(markdown::start())),
        _ => dialect_for_tag(tag).map(|dialect| State::Generic(generic::start(dialect))),
    };
}

/// The tags handled by the one table-driven scanner: everything built out of
/// words, numbers, strings and comments.
fn dialect_for_tag(tag: &str) -> Option<generic::Dialect> {
    return match tag {
        "js" | "javascript" | "mjs" | "cjs" | "jsx" | "ts" | "typescript" | "tsx" | "json" | "jsonc" => Some(generic::Dialect::Script),
        "sql" | "postgres" | "postgresql" | "mysql" | "sqlite" => Some(generic::Dialect::Sql),
        "py" | "python" => Some(generic::Dialect::Python),
        "rb" | "ruby" => Some(generic::Dialect::Ruby),
        "sh" | "bash" | "zsh" | "shell" | "dockerfile" => Some(generic::Dialect::Shell),
        "rs" | "rust" => Some(generic::Dialect::Rust),
        "go" | "golang" => Some(generic::Dialect::Go),
        "java" => Some(generic::Dialect::Java),
        "cs" | "csharp" => Some(generic::Dialect::CSharp),
        "c" | "h" | "cpp" | "cc" | "hpp" | "cxx" => Some(generic::Dialect::C),
        "php" => Some(generic::Dialect::Php),
        "swift" => Some(generic::Dialect::Swift),
        "kt" | "kotlin" => Some(generic::Dialect::Kotlin),
        "lua" => Some(generic::Dialect::Lua),
        "graphql" | "gql" => Some(generic::Dialect::GraphQl),
        "wgsl" => Some(generic::Dialect::Wgsl),
        _ => None,
    };
}

/// Walks `body` from `state`, handing every run to `emit` with the kind of
/// thing it is. `body` is one line's worth at most: callers keep `state` and
/// pass the next line in with it.
pub fn tokenize(body: &str, state: &mut State, emit: impl FnMut(&str, Piece)) {
    match state {
        State::Markup(inner) => markup::tokenize(body, inner, emit),
        State::Css(inner) => css::tokenize(body, inner, emit),
        State::Generic(inner) => generic::tokenize(body, inner, emit),
        State::Yaml(inner) => yaml::tokenize(body, inner, emit),
        State::Toml(inner) => toml::tokenize(body, inner, emit),
        State::Markdown(inner) => markdown::tokenize(body, inner, emit),
    }
}

/// Runs `body` through the scanner for its effect on `state` alone, for a
/// caller that only needs to know where the next line starts.
pub fn advance(body: &str, state: &mut State) {
    tokenize(body, state, |_, _| {});
}

/// Hands `run` to `emit` as one piece and empties it. Every scanner here
/// builds up a run of characters and flushes it when the next thing starts.
pub(crate) fn flush(run: &mut String, piece: Piece, emit: &mut impl FnMut(&str, Piece)) {
    if !run.is_empty() {
        emit(run, piece);
        run.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pieces(tag: &str, body: &str) -> Vec<(String, Piece)> {
        let mut state = state_for_tag(tag).expect("the tag should be known");
        let mut out = Vec::new();
        tokenize(body, &mut state, |text, piece| out.push((text.to_string(), piece)));
        return out;
    }

    #[test]
    fn each_known_tag_picks_its_own_tokenizer() {
        assert!(matches!(state_for_tag("html"), Some(State::Markup(_))));
        assert!(matches!(state_for_tag("css"), Some(State::Css(_))));
        assert!(matches!(state_for_tag("ts"), Some(State::Generic(_))));
        assert!(matches!(state_for_tag("sql"), Some(State::Generic(_))));
        assert!(matches!(state_for_tag("wgsl"), Some(State::Generic(_))));
        assert!(matches!(state_for_tag("yaml"), Some(State::Yaml(_))));
        assert!(matches!(state_for_tag("toml"), Some(State::Toml(_))));
        assert!(matches!(state_for_tag("md"), Some(State::Markdown(_))));
        assert!(state_for_tag("cobol").is_none());
        assert!(state_for_tag("").is_none());
    }

    #[test]
    fn every_language_hands_back_every_character_it_was_given() {
        // Highlighting may never drop or invent text, whatever the language.
        let sources = [
            ("html", r##"  <a href="#x">link</a> tail <br/>"##),
            ("css", ".hero > p, a:hover { color: #fff; /* note */ }"),
            ("js", "const total = items.map(x => x * 2); // done"),
            ("json", r#"{"name": "nail", "ok": true, "n": 3}"#),
            ("sql", "-- everyone\nSELECT name, id FROM users WHERE id > 3;"),
            ("py", "def run(n):  # start\n    return [x * 2 for x in range(n)]"),
            ("rs", "fn name<'a>(text: &'a str) -> usize { return text.len(); }"),
            ("sh", "if [ -n \"$HOME\" ]; then\n  echo \"hi\" # note\nfi"),
            ("go", "func main() {\n\tfmt.Println(\"hi\")\n}"),
            ("yaml", "# config\nname: nail\nhosts:\n  - one\n  - two   # trailing\n"),
            ("toml", "# config\n[package]\nname = \"nail\"\nedition = 2024\n"),
            ("md", "# Title\n\nSome *emphasis* and a [link](https://nail.dev).\n\n- one\n"),
            ("graphql", "query Posts($first: Int) {\n  posts(first: $first) { id title }\n}"),
            ("wgsl", "@group(0) @binding(0) var<uniform> scene: SceneUniform;\n// glow\nfn tint() -> vec4<f32> { return vec4<f32>(1.0); }"),
        ];
        for (tag, source) in sources {
            let rebuilt: String = pieces(tag, source).into_iter().map(|(text, _)| text).collect();
            assert_eq!(rebuilt, source, "{} lost or invented text", tag);
        }
    }
}
