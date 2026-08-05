//! Nail source-code utilities: syntax highlighting and live transpilation.
//!
//! `highlight_html` runs the real Nail lexer over a source string and emits
//! HTML with one `<span class="tok-*">` per token, so highlighting can never
//! drift from the language. `transpile_to_rust` runs the full compiler
//! pipeline (lex → parse → check → transpile) and returns the generated Rust.

use crate::checker::checker;
use crate::lexer::{collect_lexer_errors, lexer, Token, TokenType};
use crate::parser::parse;
use crate::embedded::{self, Piece};
use crate::transpiler::Transpiler;

/// Escaping lives in the string module, where anyone building a page will
/// look for it; the highlighter here is just its first caller.
use super::string::{escape_html as escape, push_escaped_html as push_escaped};

fn token_class(token_type: &TokenType) -> Option<&'static str> {
    match token_type {
        TokenType::Comment(_) => Some("tok-com"),
        TokenType::StringLiteral { .. } => Some("tok-str"),
        TokenType::Integer(_) | TokenType::Float(_) => Some("tok-num"),
        TokenType::BooleanLiteral(_) => Some("tok-kw"),
        TokenType::IfDeclaration
        | TokenType::ElseDeclaration
        | TokenType::ForDeclaration
        | TokenType::MapDeclaration
        | TokenType::FilterDeclaration
        | TokenType::ReduceDeclaration
        | TokenType::ScanDeclaration
        | TokenType::EachDeclaration
        | TokenType::FindDeclaration
        | TokenType::AllDeclaration
        | TokenType::AnyDeclaration
        | TokenType::WhileDeclaration
        | TokenType::LoopKeyword
        | TokenType::InKeyword
        | TokenType::FromKeyword
        | TokenType::WhenKeyword
        | TokenType::BreakKeyword
        | TokenType::ContinueKeyword
        | TokenType::MaxKeyword
        | TokenType::StepKeyword
        | TokenType::Return
        | TokenType::Yield
        | TokenType::InsertKeyword
        | TokenType::InsertSafeKeyword => Some("tok-kw"),
        TokenType::ParallelStart
        | TokenType::ParallelEnd
        | TokenType::ConcurrentStart
        | TokenType::ConcurrentEnd
        | TokenType::SpawnKeyword => Some("tok-cb"),
        TokenType::FunctionName(_) => Some("tok-fn"),
        TokenType::TypeDeclaration(_) | TokenType::FunctionReturnTypeDeclaration(_) | TokenType::EnumVariant(_) => Some("tok-ty"),
        TokenType::Operator(_) | TokenType::Assignment | TokenType::ArrowAssignment | TokenType::Range | TokenType::RangeInclusive | TokenType::Dot => Some("tok-op"),
        TokenType::LexerError(_) => Some("tok-err"),
        _ => None,
    }
}

/// Distinguishes tokens the highlighter treats specially: colons followed by
/// an adjacent identifier form a type annotation (`evens:a:i`), which the
/// lexer leaves as plain identifiers because types there are context-dependent.
#[derive(Clone, PartialEq)]
enum SpanKind {
    Styled(String),
    Colon,
    Ident,
    Plain,
}

type SpanEntry = (usize, usize, usize, usize, SpanKind);

fn flatten(tokens: &[Token], out: &mut Vec<SpanEntry>) {
    for token in tokens {
        let span = &token.code_span;
        match &token.token_type {
            TokenType::FunctionSignature(inner) => {
                // The signature's own span starts at the `f` keyword, which has
                // no token of its own; style that single character as a keyword.
                out.push((span.start_line, span.start_column, span.start_line, span.start_column + 1, SpanKind::Styled("tok-kw".to_string())));
                flatten(inner, out);
            }
            TokenType::EndOfFile => {}
            TokenType::Colon => out.push((span.start_line, span.start_column, span.end_line, span.end_column, SpanKind::Colon)),
            TokenType::Identifier(_) => out.push((span.start_line, span.start_column, span.end_line, span.end_column, SpanKind::Ident)),
            token_type => {
                let kind = match token_class(token_type) {
                    // A tagged string keeps the plain string class and gains one
                    // naming its language, so a page can style html`...` apart
                    // from the rest without losing the base styling.
                    Some(class_name) => SpanKind::Styled(match token_type {
                        TokenType::StringLiteral { tag: Some(tag), .. } => format!("{} tok-str-{}", class_name, tag),
                        _ => class_name.to_string(),
                    }),
                    None => SpanKind::Plain,
                };
                out.push((span.start_line, span.start_column, span.end_line, span.end_column, kind));
            }
        }
    }
}

/// Copies characters from `cur` (inclusive) up to `target` (exclusive), both
/// 1-indexed (line, column) positions, HTML-escaping as it goes. When
/// `style_comments` is set, `//` runs are wrapped in comment spans — the lexer
/// consumes comments without emitting tokens, so between-token gaps are the
/// only place they can appear.
fn advance_to(lines: &[Vec<char>], cur: &mut (usize, usize), target: (usize, usize), out: &mut String, style_comments: bool) {
    let mut in_comment = false;
    while *cur < target {
        let (line, col) = *cur;
        if line > lines.len() {
            break;
        }
        let line_chars = &lines[line - 1];
        if col > line_chars.len() {
            if in_comment {
                out.push_str("</span>");
                in_comment = false;
            }
            if line < lines.len() {
                out.push('\n');
            }
            *cur = (line + 1, 1);
        } else {
            if style_comments && !in_comment && line_chars[col - 1] == '/' && col < line_chars.len() && line_chars[col] == '/' {
                out.push_str("<span class=\"tok-com\">");
                in_comment = true;
            }
            push_escaped(line_chars[col - 1], out);
            *cur = (line, col + 1);
        }
    }
    if in_comment {
        out.push_str("</span>");
    }
}

/// Copies the raw characters of a span, advancing `cur` exactly as
/// `advance_to` does but without escaping them - the markup scanner needs the
/// text as written, and escaping happens per piece afterwards.
fn collect_raw(lines: &[Vec<char>], cur: &mut (usize, usize), target: (usize, usize)) -> String {
    let mut raw = String::new();
    while *cur < target {
        let (line, col) = *cur;
        if line > lines.len() {
            break;
        }
        let line_chars = &lines[line - 1];
        if col > line_chars.len() {
            if line < lines.len() {
                raw.push('\n');
            }
            *cur = (line + 1, 1);
        } else {
            raw.push(line_chars[col - 1]);
            *cur = (line, col + 1);
        }
    }
    raw
}

/// The tokenizer a span's contents want, if the class list names a language
/// this build knows - html, css, js and friends. Read back off the class
/// rather than threaded separately, so the two can never disagree.
fn embedded_state_of(class: &Option<String>) -> Option<embedded::State> {
    let class = class.as_ref()?;
    let tag = class.split_whitespace().find_map(|part| part.strip_prefix("tok-str-"))?;
    return embedded::state_for_tag(tag);
}

/// Splits a tagged string span into the `tag` and backtick that open it, the
/// body in the other language, and the backtick that closes it. A string the
/// lexer never saw closed has no closing backtick, and is split all the same.
fn split_delimiters(raw: &str) -> (&str, &str, &str) {
    let opening = match raw.find('`') {
        Some(at) => at + 1,
        None => return (raw, "", ""),
    };
    let closing = if raw.len() > opening && raw.ends_with('`') { raw.len() - 1 } else { raw.len() };
    return (&raw[..opening], &raw[opening..closing], &raw[closing..]);
}

/// The class each piece of an embedded language is wrapped in. The names are
/// shared across languages - a CSS selector and an HTML element are the same
/// kind of thing to a page's stylesheet - and plain text gets none, so it keeps
/// the string color of the span it sits inside.
fn embedded_class(piece: Piece) -> Option<&'static str> {
    return match piece {
        Piece::Bracket => Some("tok-md-punct"),
        Piece::Element => Some("tok-md-el"),
        Piece::Attribute => Some("tok-md-attr"),
        Piece::Function => Some("tok-md-fn"),
        Piece::Keyword => Some("tok-md-kw"),
        Piece::Operator => Some("tok-md-op"),
        Piece::Value => Some("tok-md-val"),
        Piece::Number => Some("tok-md-num"),
        Piece::Comment => Some("tok-md-com"),
        Piece::Text => None,
    };
}

/// Highlights Nail source as HTML using the real Nail lexer. Every token is
/// wrapped in a `<span class="tok-*">`; text is HTML-escaped. Intended for
/// embedding inside a `<pre>` element.
pub fn highlight_html(source: String) -> String {
    let tokens = lexer(&source);
    let mut spans: Vec<SpanEntry> = Vec::new();
    flatten(&tokens, &mut spans);
    spans.sort_by_key(|entry| (entry.0, entry.1));

    // A colon with an identifier attached directly after it is a type
    // annotation (`x:i`, `evens:a:i`) — style both as types.
    let mut classes: Vec<Option<String>> = spans
        .iter()
        .map(|entry| match &entry.4 {
            SpanKind::Styled(class_name) => Some(class_name.clone()),
            _ => None,
        })
        .collect();
    for index in 0..spans.len().saturating_sub(1) {
        let (_, _, end_line, end_column, ref kind) = spans[index];
        let (next_start_line, next_start_column, _, _, ref next_kind) = spans[index + 1];
        if *kind == SpanKind::Colon && *next_kind == SpanKind::Ident && next_start_line == end_line && next_start_column == end_column {
            classes[index] = Some("tok-ty".to_string());
            classes[index + 1] = Some("tok-ty".to_string());
        }
    }

    // Which spans hold another language, so their contents get scanned rather
    // than painted as one block of string.
    let embedded_states: Vec<Option<embedded::State>> = classes.iter().map(embedded_state_of).collect();

    let lines: Vec<Vec<char>> = source.split('\n').map(|line| line.chars().collect()).collect();
    let mut out = String::with_capacity(source.len() * 2);
    let mut cur = (1usize, 1usize);

    for (index, &(start_line, start_column, end_line, end_column, _)) in spans.iter().enumerate() {
        if end_line == 0 || (start_line, start_column) < cur {
            continue;
        }
        advance_to(&lines, &mut cur, (start_line, start_column), &mut out, true);
        match &classes[index] {
            Some(class_name) => {
                out.push_str("<span class=\"");
                out.push_str(class_name);
                out.push_str("\">");
                match &embedded_states[index] {
                    Some(start_state) => {
                        // The span holds another language: scan it and wrap
                        // each piece, so the markup, CSS or script reads as
                        // itself instead of as one flat string.
                        let raw = collect_raw(&lines, &mut cur, (end_line, end_column));
                        // The span runs from the language tag through the
                        // closing backtick; only what sits between the
                        // backticks is the other language, so the delimiters go
                        // out plain rather than through the scanner - a `css`
                        // tag glued to the first selector would otherwise be
                        // colored as part of it.
                        let (opening, body, closing) = split_delimiters(&raw);
                        for character in opening.chars() {
                            push_escaped(character, &mut out);
                        }
                        let mut state = start_state.clone();
                        embedded::tokenize(body, &mut state, |text, piece| match embedded_class(piece) {
                            Some(piece_class) => {
                                out.push_str("<span class=\"");
                                out.push_str(piece_class);
                                out.push_str("\">");
                                for character in text.chars() {
                                    push_escaped(character, &mut out);
                                }
                                out.push_str("</span>");
                            }
                            None => {
                                for character in text.chars() {
                                    push_escaped(character, &mut out);
                                }
                            }
                        });
                        for character in closing.chars() {
                            push_escaped(character, &mut out);
                        }
                    }
                    None => advance_to(&lines, &mut cur, (end_line, end_column), &mut out, false),
                }
                out.push_str("</span>");
            }
            None => {
                advance_to(&lines, &mut cur, (end_line, end_column), &mut out, false);
            }
        }
    }

    let last_line = lines.len();
    let last_column = lines.last().map(|line| line.len() + 1).unwrap_or(1);
    advance_to(&lines, &mut cur, (last_line, last_column), &mut out, true);
    out
}

/// Runs the full Nail compiler pipeline on a source string and returns the
/// generated Rust code, or the first error encountered at any stage.
pub fn transpile_to_rust(source: String) -> Result<String, String> {
    let tokens = lexer(&source);
    let lex_errors = collect_lexer_errors(&tokens);
    if let Some(first) = lex_errors.first() {
        return Err(format!("code_transpile_to_rust: lexer error in the source: {}", first.message));
    }
    let mut ast = parse(tokens).map_err(|error| format!("code_transpile_to_rust: parse error in the source: {}", error.message))?;
    checker(&mut ast).map_err(|errors| match errors.first() {
        Some(error) => format!("code_transpile_to_rust: type error in the source: {}", error.message),
        None => "code_transpile_to_rust: type checking of the source failed".to_string(),
    })?;
    let mut transpiler = Transpiler::new();
    transpiler.transpile(&ast).map_err(|error| format!("code_transpile_to_rust: transpile error in the source: {}", error.message))
}

/// Escapes text for safe embedding in HTML (`&`, `<`, `>`).
pub fn escape_html(text: String) -> String {
    escape(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_inside_a_tagged_string_is_wrapped_piece_by_piece() {
        let html = highlight_html("page:s = html`<section class=\"hero\">Nail</section>`;".to_string());
        assert!(html.contains("<span class=\"tok-md-el\">section</span>"), "expected element spans, got: {}", html);
        assert!(html.contains("<span class=\"tok-md-attr\">class</span>"), "expected attribute spans, got: {}", html);
        assert!(html.contains("<span class=\"tok-md-val\">&quot;hero&quot;</span>"), "expected an escaped value span, got: {}", html);
    }

    #[test]
    fn css_and_script_strings_are_wrapped_by_their_own_tokenizers() {
        let css = highlight_html("sheet:s = css`.hero { color: #fff; }`;".to_string());
        assert!(css.contains("<span class=\"tok-md-el\">.hero</span>"), "expected a selector span, got: {}", css);
        assert!(css.contains("<span class=\"tok-md-attr\">color</span>"), "expected a property span, got: {}", css);
        assert!(css.contains("<span class=\"tok-md-num\">#fff</span>"), "expected a colour span, got: {}", css);

        let script = highlight_html("script:s = ts`const total:i = items.length;`;".to_string());
        assert!(script.contains("<span class=\"tok-md-kw\">const</span>"), "expected a keyword span, got: {}", script);
        assert!(script.contains("<span class=\"tok-md-attr\">length</span>"), "expected a member span, got: {}", script);
    }

    #[test]
    fn a_tag_a_highlighter_does_not_know_is_left_whole() {
        let html = highlight_html("program:s = cobol`DISPLAY 'hi'.`;".to_string());
        assert!(html.contains("tok-str-cobol"), "the language should still be named, got: {}", html);
        assert!(!html.contains("tok-md-"), "an unknown language is never tokenized, got: {}", html);
    }

    #[test]
    fn highlighting_markup_never_drops_or_invents_text() {
        // Stripping the tags back out has to give the source again, or the
        // page is showing something the file does not say.
        let source = "page:s = html`<a href=\"#x\">link</a>`;\nplain:s = `<not markup>`;";
        let html = highlight_html(source.to_string());
        let mut stripped = String::new();
        let mut in_tag = false;
        for character in html.chars() {
            match character {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => stripped.push(character),
                _ => {}
            }
        }
        let unescaped = stripped.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&amp;", "&");
        assert_eq!(unescaped, source);
    }

    #[test]
    fn a_tagged_string_keeps_the_string_class_and_names_its_language() {
        let html = highlight_html("page:s = html`<p>hi</p>`;".to_string());
        assert!(html.contains("<span class=\"tok-str tok-str-html\">"), "expected a tagged string span, got: {}", html);
        // The tag is part of the highlighted span, not left outside it.
        assert!(html.contains("tok-str-html\">html`"), "expected the tag inside the span, got: {}", html);
    }

    #[test]
    fn an_untagged_string_keeps_the_plain_string_class() {
        let html = highlight_html("plain:s = `hello`;".to_string());
        assert!(html.contains("<span class=\"tok-str\">"), "expected a plain string span, got: {}", html);
        assert!(!html.contains("tok-str-"), "expected no language class, got: {}", html);
    }

    #[test]
    fn the_contents_of_a_tagged_string_are_still_escaped() {
        // The brackets are their own spans now that markup is tokenized, but
        // every one of them is still escaped.
        let html = highlight_html("page:s = html`<p>hi</p>`;".to_string());
        assert!(html.contains("&lt;"), "expected escaped brackets, got: {}", html);
        assert!(html.contains("&lt;/"), "expected an escaped closing bracket, got: {}", html);
        assert!(!html.contains("<p>"), "no raw markup may reach the page, got: {}", html);
    }
}
