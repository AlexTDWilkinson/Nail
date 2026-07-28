//! Nail source-code utilities: syntax highlighting and live transpilation.
//!
//! `highlight_html` runs the real Nail lexer over a source string and emits
//! HTML with one `<span class="tok-*">` per token, so highlighting can never
//! drift from the language. `transpile_to_rust` runs the full compiler
//! pipeline (lex → parse → check → transpile) and returns the generated Rust.

use crate::checker::checker;
use crate::lexer::{collect_lexer_errors, lexer, Token, TokenType};
use crate::parser::parse;
use crate::transpiler::Transpiler;

fn push_escaped(ch: char, out: &mut String) {
    match ch {
        '&' => out.push_str("&amp;"),
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        _ => out.push(ch),
    }
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    for ch in text.chars() {
        push_escaped(ch, &mut out);
    }
    out
}

fn token_class(token_type: &TokenType) -> Option<&'static str> {
    match token_type {
        TokenType::Comment(_) => Some("tok-com"),
        TokenType::StringLiteral(_) => Some("tok-str"),
        TokenType::Integer(_) | TokenType::Float(_) => Some("tok-num"),
        TokenType::BooleanLiteral(_) => Some("tok-kw"),
        TokenType::IfDeclaration
        | TokenType::ElseDeclaration
        | TokenType::ForDeclaration
        | TokenType::MapDeclaration
        | TokenType::FilterDeclaration
        | TokenType::ReduceDeclaration
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
        | TokenType::InsertKeyword => Some("tok-kw"),
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
#[derive(Clone, Copy, PartialEq)]
enum SpanKind {
    Styled(&'static str),
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
                out.push((span.start_line, span.start_column, span.start_line, span.start_column + 1, SpanKind::Styled("tok-kw")));
                flatten(inner, out);
            }
            TokenType::EndOfFile => {}
            TokenType::Colon => out.push((span.start_line, span.start_column, span.end_line, span.end_column, SpanKind::Colon)),
            TokenType::Identifier(_) => out.push((span.start_line, span.start_column, span.end_line, span.end_column, SpanKind::Ident)),
            token_type => {
                let kind = match token_class(token_type) {
                    Some(class_name) => SpanKind::Styled(class_name),
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
    let mut classes: Vec<Option<&'static str>> = spans
        .iter()
        .map(|entry| match entry.4 {
            SpanKind::Styled(class_name) => Some(class_name),
            _ => None,
        })
        .collect();
    for index in 0..spans.len().saturating_sub(1) {
        let (_, _, end_line, end_column, kind) = spans[index];
        let (next_start_line, next_start_column, _, _, next_kind) = spans[index + 1];
        if kind == SpanKind::Colon && next_kind == SpanKind::Ident && next_start_line == end_line && next_start_column == end_column {
            classes[index] = Some("tok-ty");
            classes[index + 1] = Some("tok-ty");
        }
    }

    let lines: Vec<Vec<char>> = source.split('\n').map(|line| line.chars().collect()).collect();
    let mut out = String::with_capacity(source.len() * 2);
    let mut cur = (1usize, 1usize);

    for (index, &(start_line, start_column, end_line, end_column, _)) in spans.iter().enumerate() {
        if end_line == 0 || (start_line, start_column) < cur {
            continue;
        }
        advance_to(&lines, &mut cur, (start_line, start_column), &mut out, true);
        match classes[index] {
            Some(class_name) => {
                out.push_str("<span class=\"");
                out.push_str(class_name);
                out.push_str("\">");
                advance_to(&lines, &mut cur, (end_line, end_column), &mut out, false);
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
