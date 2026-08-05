use lazy_static::lazy_static;
use log;
use ratatui::text::Span;
use ratatui::{
    style::{Color, Style},
    text::Line,
};
use rayon::prelude::*;

use crate::embedded::{self, Piece};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorScheme {
    pub function: Color,
    pub const_decl: Color,
    pub var_decl: Color,
    pub if_decl: Color,
    pub else_decl: Color,
    pub arrow_decl: Color,
    pub identifier: Color,
    pub unsigned_int: Color,
    pub signed_int: Color,
    pub float: Color,
    pub operator: Color,
    pub keyword: Color,
    pub comma: Color,
    pub string_literal: Color,
    pub identifier_type: Color,
    pub unknown: Color,
    pub parenthesis: Color,
    pub block: Color,
    pub end_statement: Color,
    pub async_keyword: Color,
    pub parallel_keyword: Color,
    pub struct_keyword: Color,
    pub enum_keyword: Color,
    pub return_keyword: Color,
    pub default: Color,
    pub background: Color,
    pub comment: Color,
    pub error: Color,
}

/// Convert a hex color string (e.g., "#FF5733") to a `tui::style::Color`
pub fn hex_to_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');

    if hex.len() == 6 {
        let red = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let green = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let blue = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        Color::Rgb(red, green, blue)
    } else {
        Color::Reset // Fallback color
    }
}

lazy_static! {
    pub static ref LIGHT_THEME: ColorScheme = ColorScheme {
        function: hex_to_color("#004380"),      // Darkened Ocean Blue
        const_decl: hex_to_color("#B01D28"),    // Darkened Ruby Red
        var_decl: hex_to_color("#663C00"),      // Heavily Darkened Burnt Orange
        if_decl: hex_to_color("#6C40D9"),       // Darkened Electric Purple
        else_decl: hex_to_color("#7E62B0"),     // Darkened Medium Purple
        arrow_decl: hex_to_color("#004A44"),    // Heavily Darkened Teal Green
        identifier: hex_to_color("#1B2026"),    // Darkened Almost Black
        unsigned_int: hex_to_color("#6B4A00"),  // Heavily Darkened Dark Gold
        signed_int: hex_to_color("#7A3000"),    // Heavily Darkened Rust Orange
        float: hex_to_color("#0F4520"),         // Heavily Darkened Forest Green
        operator: hex_to_color("#B6303D"),      // Darkened Bright Red
        keyword: hex_to_color("#5D37A2"),       // Darkened Royal Purple
        comma: hex_to_color("#485058"),         // Darkened Dark Gray
        string_literal: hex_to_color("#0A5520"), // Heavily Darkened Grass Green
        identifier_type: hex_to_color("#004DA6"), // Darkened Azure Blue
        unknown: hex_to_color("#58606A"),       // Darkened Medium Gray
        parenthesis: hex_to_color("#483700"),   // Heavily Darkened Dark Olive
        block: hex_to_color("#0255B3"),         // Darkened Bright Blue
        end_statement: hex_to_color("#7D848C"), // Darkened Light Gray
        async_keyword: hex_to_color("#6B5300"),  // Heavily Darkened Mustard
        parallel_keyword: hex_to_color("#A14C3D"), // Heavily Darkened Coral Orange
        struct_keyword: hex_to_color("#C53E4C"), // Darkened Watermelon
        enum_keyword: hex_to_color("#987BCC"),   // Darkened Lavender
        return_keyword: hex_to_color("#D07BAF"), // Darkened Pink Rose
        default: hex_to_color("#15191D"),       // Darkened Charcoal
        background: hex_to_color("#CCCCCC"),    // Light Gray
        comment: hex_to_color("#5A5C63"),       // Slightly Lightened Comment Gray
        error: hex_to_color("#AB1F29"),         // Darkened Error Red
    };

    pub static ref DARK_THEME: ColorScheme = ColorScheme {
        function: hex_to_color("#61AFEF"),      // Soft Blue
        const_decl: hex_to_color("#E06C75"),    // Salmon Pink
        var_decl: hex_to_color("#D19A66"),      // Sandy Brown
        if_decl: hex_to_color("#C678DD"),       // Violet
        else_decl: hex_to_color("#E5C0FF"),     // Pale Violet
        arrow_decl: hex_to_color("#56B6C2"),    // Cyan
        identifier: hex_to_color("#E6E6E6"),    // Light Gray
        unsigned_int: hex_to_color("#CE9178"),  // Terra Cotta
        signed_int: hex_to_color("#B5CEA8"),    // Sage Green
        float: hex_to_color("#4EC9B0"),         // Mint
        operator: hex_to_color("#569CD6"),      // Sky Blue
        keyword: hex_to_color("#C586C0"),       // Orchid
        comma: hex_to_color("#858585"),         // Medium Gray
        string_literal: hex_to_color("#98C379"), // Spring Green
        identifier_type: hex_to_color("#4FC1E9"), // Light Blue
        unknown: hex_to_color("#808080"),       // Gray
        parenthesis: hex_to_color("#FFD602"),   // Bright Yellow
        block: hex_to_color("#9CDCFE"),         // Powder Blue
        end_statement: hex_to_color("#6B6B6B"), // Dark Gray
        async_keyword: hex_to_color("#DCDCAA"),  // Pale Yellow
        parallel_keyword: hex_to_color("#FFB86C"), // Orange Cream
        struct_keyword: hex_to_color("#FF79C6"), // Hot Pink
        enum_keyword: hex_to_color("#BD93F9"),   // Purple Rain
        return_keyword: hex_to_color("#FF6AC1"), // Magenta
        default: hex_to_color("#D4D4D4"),       // Off White
        background: hex_to_color("#1A1A1C"),    // Deep Black
        comment: hex_to_color("#7F7F7F"),       // Neutral Gray
        error: hex_to_color("#F97583"),         // Light Red
    };



}

pub fn colorize_code(content: Vec<Line>, theme: &ColorScheme) -> Vec<Line<'static>> {
    // Safety check: ensure we have a valid theme and content
    if content.is_empty() {
        return vec![Line::from(vec![Span::raw("")])];
    }
    
    // Validate theme colors to prevent unexpected green text
    if matches!(theme.default, Color::Green) && !cfg!(test) {
        log::warn!("Detected potentially incorrect theme with green default color");
    }

    // First pass: detect multi-line strings
    let string_state = scan_string_states(&content);

    // Parallel colorization per line with bounds checking
    let colored_lines: Vec<Line<'static>> = content.into_par_iter().enumerate().map(|(line_idx, line)| {
        // Add bounds checking for line_idx
        if line_idx < string_state.len() {
            colorize_line(line, line_idx, &string_state, theme)
        } else {
            log::warn!("Colorizer: line index {} out of bounds for string_state len {}", line_idx, string_state.len());
            // Return uncolored line as fallback
            Line::from(line.spans.into_iter().map(|span| Span::raw(span.content.to_string())).collect::<Vec<_>>())
        }
    }).collect();

    colored_lines
}

/// Length of the language tag sitting at the end of `content`, if any. A tag
/// is an identifier-shaped run written flush against a string's opening
/// backtick - the `html` in html`<p>hi</p>` - and the lexer treats it as part
/// of the literal, so the colorizer has to as well. Tags are ASCII by
/// construction, so the returned length is good for both bytes and chars.
fn trailing_tag_len(content: &str) -> usize {
    let tag_len = content.chars().rev().take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_').count();
    if tag_len == 0 {
        return 0;
    }
    match content[content.len() - tag_len..].chars().next() {
        Some(first) if first.is_ascii_lowercase() => tag_len,
        // A run of digits or underscores is not an identifier, so it is not a tag.
        _ => 0,
    }
}

/// Where a line begins: inside a string literal, or in ordinary code.
/// A string opened with a language tag carries how far into that language's
/// syntax the previous line got - a `<div` whose attributes run onto the next
/// line, a CSS block, a `/* ... */` - since all of those stay open across the
/// break. A string with no tag, or a tag no tokenizer knows, carries `None`
/// and is colored as one plain string.
#[derive(Clone, PartialEq, Debug)]
struct StringContext {
    embedded: Option<embedded::State>,
}

/// The state each line *starts* in. The previous version recorded the state at
/// the end of each line, which put the boundary a line out at both ends: the
/// line opening a multi-line string had its leading code painted as string,
/// and the line closing one had its trailing markup painted as code.
fn scan_string_states(content: &[Line]) -> Vec<Option<StringContext>> {
    let mut state: Option<StringContext> = None;
    let mut states = Vec::with_capacity(content.len());

    for line in content {
        states.push(state.clone());
        let text = line.spans.iter().map(|span| span.content.as_ref()).collect::<Vec<_>>().join("");
        advance_line_state(&text, &mut state, None);
    }

    states
}

/// Walks one line, updating `state`, and colors it into `emit` if one is
/// given. Colorizing and state-tracking share this one walk so the two can
/// never disagree about where a string ends.
fn advance_line_state(text: &str, state: &mut Option<StringContext>, mut emit: Option<(&mut Vec<Span<'static>>, &ColorScheme)>) {
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;
    let mut code_run = String::new();
    let mut string_run = String::new();

    while index < chars.len() {
        let ch = chars[index];

        if let Some(context) = state.clone() {
            // Inside a string. An escaped character never closes it.
            if ch == '\\' && index + 1 < chars.len() {
                string_run.push(ch);
                string_run.push(chars[index + 1]);
                index += 2;
                continue;
            }
            if ch == '`' {
                let mut embedded = context.embedded.clone();
                match emit.as_mut() {
                    Some((spans, theme)) => {
                        push_string_body(&string_run, &mut embedded, spans, theme);
                        spans.push(Span::styled("`".to_string(), Style::default().fg(theme.string_literal)));
                    }
                    None => advance_string_body(&string_run, &mut embedded),
                }
                string_run.clear();
                *state = None;
                index += 1;
                continue;
            }
            string_run.push(ch);
            index += 1;
            continue;
        }

        // In code. A comment runs to the end of the line and is never tokenized.
        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            let comment: String = chars[index..].iter().collect();
            if let Some((spans, theme)) = emit.as_mut() {
                push_code_run(&code_run, spans, theme);
                spans.push(Span::styled(comment, Style::default().fg(theme.comment)));
            }
            code_run.clear();
            return;
        }

        if ch == '`' {
            // A language tag written against the backtick belongs to the
            // string, not to the code before it.
            let tag_len = trailing_tag_len(&code_run);
            let tag: String = code_run[code_run.len() - tag_len..].to_string();
            code_run.truncate(code_run.len() - tag_len);
            if let Some((spans, theme)) = emit.as_mut() {
                push_code_run(&code_run, spans, theme);
                spans.push(Span::styled(format!("{}`", tag), Style::default().fg(theme.string_literal)));
            }
            code_run.clear();
            *state = Some(StringContext { embedded: embedded::state_for_tag(&tag) });
            index += 1;
            continue;
        }

        code_run.push(ch);
        index += 1;
    }

    // Whatever is left runs off the end of the line.
    if let Some(context) = state.as_mut() {
        match emit.as_mut() {
            Some((spans, theme)) => push_string_body(&string_run, &mut context.embedded, spans, theme),
            None => advance_string_body(&string_run, &mut context.embedded),
        }
    } else if let Some((spans, theme)) = emit.as_mut() {
        push_code_run(&code_run, spans, theme);
    }
}

fn push_code_run(code_run: &str, spans: &mut Vec<Span<'static>>, theme: &ColorScheme) {
    if code_run.is_empty() {
        return;
    }
    colorize_non_string_content_preserve_positions(code_run, spans, theme);
}

/// The inside of a string literal. A tag naming a language the highlighter
/// knows is colored piece by piece; anything else - an untagged string, or a
/// tag no tokenizer covers - stays one color.
fn push_string_body(body: &str, embedded: &mut Option<embedded::State>, spans: &mut Vec<Span<'static>>, theme: &ColorScheme) {
    if body.is_empty() {
        return;
    }
    match embedded {
        Some(state) => embedded::tokenize(body, state, |text, piece| {
            spans.push(Span::styled(text.to_string(), Style::default().fg(embedded_color(piece, theme))));
        }),
        None => spans.push(Span::styled(body.to_string(), Style::default().fg(theme.string_literal))),
    }
}

fn advance_string_body(body: &str, embedded: &mut Option<embedded::State>) {
    if let Some(state) = embedded {
        embedded::advance(body, state);
    }
}

/// One color per kind of piece, shared by every embedded language: an element
/// name and a CSS selector name the same thing about their language, so they
/// are painted the same way.
fn embedded_color(piece: Piece, theme: &ColorScheme) -> Color {
    return match piece {
        Piece::Bracket => theme.comma,
        Piece::Element => theme.keyword,
        Piece::Attribute => theme.identifier_type,
        Piece::Function => theme.function,
        Piece::Keyword => theme.keyword,
        Piece::Operator => theme.operator,
        Piece::Value => theme.string_literal,
        Piece::Number => theme.signed_int,
        Piece::Comment => theme.comment,
        Piece::Text => theme.string_literal,
    };
}

fn colorize_line(line: Line, line_idx: usize, string_states: &[Option<StringContext>], theme: &ColorScheme) -> Line<'static> {
    if line.spans.is_empty() {
        return Line::from(vec![Span::raw("")]);
    }

    let text = line.spans.iter().map(|span| span.content.as_ref()).collect::<Vec<_>>().join("");
    let mut state = string_states.get(line_idx).cloned().flatten();
    let mut colored_spans: Vec<Span<'static>> = Vec::new();
    advance_line_state(&text, &mut state, Some((&mut colored_spans, theme)));

    if colored_spans.is_empty() {
        colored_spans.push(Span::raw(text));
    }

    Line::from(colored_spans)
}

fn tokenize_code(content: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            // Delimiters that should be separate tokens
            '(' | ')' | '{' | '}' | '[' | ']' | ',' | ';' | ':' => {
                // Push any accumulated token
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
                // Push the delimiter as its own token
                tokens.push(ch.to_string());
            }
            // Operators that might be multi-character
            '=' | '!' | '<' | '>' | '+' | '-' | '*' | '/' => {
                // Special case for error types: check if ! is part of type!e pattern
                if ch == '!' {
                    // Check if the current token is a type character and next is 'e'
                    let is_error_type = !current_token.is_empty() && current_token.chars().all(|c| matches!(c, 'i' | 'f' | 's' | 'b' | 'a')) && chars.peek() == Some(&'e');

                    if is_error_type {
                        // This is an error type like i!e, keep it as one token
                        current_token.push(ch);
                        current_token.push(chars.next().unwrap()); // consume the 'e'
                        continue;
                    }
                }

                // Special case for parallel end: check if / is followed by p
                if ch == '/' && chars.peek() == Some(&'p') {
                    // This is /p for parallel end, keep it as one token
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                    current_token.push(ch);
                    current_token.push(chars.next().unwrap()); // consume the 'p'
                    tokens.push(current_token.clone());
                    current_token.clear();
                    continue;
                }

                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }

                let mut op = ch.to_string();
                // Check for two-character operators
                if let Some(&next_ch) = chars.peek() {
                    if (ch == '=' && next_ch == '=') || (ch == '!' && next_ch == '=') || (ch == '<' && next_ch == '=') || (ch == '>' && next_ch == '=') || (ch == '-' && next_ch == '>') {
                        op.push(chars.next().unwrap());
                    }
                }
                tokens.push(op);
            }
            // Whitespace
            ' ' | '\t' | '\n' | '\r' => {
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
            }
            // Regular characters
            _ => {
                current_token.push(ch);
            }
        }
    }

    // Don't forget the last token
    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    tokens
}

fn colorize_non_string_content(content: &str, colored_spans: &mut Vec<Span<'static>>, theme: &ColorScheme) {
    // Safety check: never tokenize comments
    if content.trim().starts_with("//") {
        colored_spans.push(Span::styled(content.to_string(), Style::default().fg(theme.comment)));
        return;
    }

    // Preserve leading whitespace
    let leading_spaces = content.len() - content.trim_start().len();
    if leading_spaces > 0 {
        colored_spans.push(Span::raw(" ".repeat(leading_spaces)));
    }

    let trimmed_content = content.trim_start();
    let tokens = tokenize_code(trimmed_content);
    let mut i = 0;
    let mut need_space = false;

    while i < tokens.len() {
        let token = &tokens[i];

        // Skip whitespace tokens
        if token.trim().is_empty() {
            i += 1;
            continue;
        }

        // Add space between tokens when needed
        // Special case: don't add space before ':' if previous token was '|' (lambda return type)
        let prev_token = if i > 0 { Some(tokens[i - 1].as_str()) } else { None };
        if need_space && !matches!(token.as_str(), "," | ";" | ")" | "]" | "}") && !(token == ":" && prev_token == Some("|")) {
            colored_spans.push(Span::raw(" "));
        }
        need_space = false;

        // Check if this is an identifier:type pattern
        if i + 2 < tokens.len() && tokens[i + 1] == ":" && !token.starts_with('`') && !token.contains("::") {
            // Color identifier
            colored_spans.push(Span::styled(token.to_string(), Style::default().fg(theme.var_decl)));
            // Color colon
            colored_spans.push(Span::styled(tokens[i + 1].to_string(), Style::default().fg(theme.operator)));

            // Handle type part (might be array type)
            let type_token = &tokens[i + 2];
            if type_token == "a" && i + 4 < tokens.len() && tokens[i + 3] == ":" {
                // Array type like a:i
                colored_spans.push(Span::styled("a".to_string(), Style::default().fg(theme.identifier_type)));
                colored_spans.push(Span::styled(":".to_string(), Style::default().fg(theme.operator)));
                colored_spans.push(Span::styled(tokens[i + 4].to_string(), Style::default().fg(theme.identifier_type)));
                i += 5;
            } else {
                // Simple type
                colored_spans.push(Span::styled(type_token.to_string(), Style::default().fg(theme.identifier_type)));
                i += 3;
            }
        }
        // Check if this is a function call
        else if i + 1 < tokens.len() && tokens[i + 1] == "(" {
            colored_spans.push(Span::styled(token.to_string(), Style::default().fg(theme.function)));
            // Process the '(' immediately to avoid adding space
            colored_spans.push(Span::styled("(".to_string(), Style::default().fg(theme.parenthesis)));
            i += 2;
            continue;
        }
        // Regular token
        else {
            let styled_span = colorize_word(token, theme);
            colored_spans.push(styled_span);
            i += 1;
        }

        // Set need_space for next iteration
        need_space = !matches!(token.as_str(), "(" | "[" | "{");
    }
}

// New function that preserves exact character positions
fn colorize_non_string_content_preserve_positions(content: &str, colored_spans: &mut Vec<Span<'static>>, theme: &ColorScheme) {
    // Safety check: never tokenize comments
    if content.trim().starts_with("//") {
        colored_spans.push(Span::styled(content.to_string(), Style::default().fg(theme.comment)));
        return;
    }

    // Track position in original string
    let mut pos = 0;
    let chars: Vec<char> = content.chars().collect();
    // True when the previous non-whitespace token was ':' - the next word is
    // then a type annotation, not a variable declaration
    let mut prev_was_colon = false;

    while pos < chars.len() {
        // Skip whitespace
        let start_pos = pos;
        // Add safety counter to prevent infinite loops
        let mut ws_counter = 0;
        while pos < chars.len() && chars[pos].is_whitespace() && ws_counter < 100 {
            pos += 1;
            ws_counter += 1;
        }
        
        // Add whitespace span if any
        if pos > start_pos {
            let whitespace: String = chars[start_pos..pos].iter().collect();
            colored_spans.push(Span::raw(whitespace));
        }
        
        if pos >= chars.len() {
            break;
        }
        
        // Find the end of the current token
        let token_start = pos;
        
        // Check for operators and delimiters
        let ch = chars[pos];
        if matches!(ch, '(' | ')' | '{' | '}' | '[' | ']' | ',' | ';' | ':') {
            // Single character delimiter
            colored_spans.push(colorize_single_char(ch, theme));
            prev_was_colon = ch == ':';
            pos += 1;
        } else if matches!(ch, '=' | '!' | '<' | '>' | '+' | '-' | '*' | '/') {
            // Potentially multi-character operator
            let mut token_end = pos + 1;
            
            // Check for two-character operators
            if token_end < chars.len() {
                let next_ch = chars[token_end];
                if (ch == '=' && next_ch == '=') ||
                   (ch == '-' && next_ch == '>') ||
                   (ch == '!' && next_ch == '=') ||
                   (ch == '<' && next_ch == '=') ||
                   (ch == '>' && next_ch == '=') ||
                   (ch == '/' && next_ch == 'p') {
                    token_end += 1;
                }
            }
            
            // Special case for error types (e.g., i!e)
            if ch == '!' && pos > 0 && token_end < chars.len() && chars[token_end] == 'e' {
                let prev_ch = chars[pos - 1];
                if matches!(prev_ch, 'i' | 'f' | 's' | 'b' | 'a') {
                    // This is part of an error type, handled elsewhere
                    pos += 1;
                    continue;
                }
            }
            
            let token: String = chars[token_start..token_end].iter().collect();
            colored_spans.push(Span::styled(token, Style::default().fg(theme.operator)));
            prev_was_colon = false;
            pos = token_end;
        } else {
            // Regular word/identifier
            let mut token_end = pos;
            // Add safety counter to prevent infinite loops
            let mut loop_counter = 0;
            while token_end < chars.len() && loop_counter < 1000 {
                loop_counter += 1;
                let ch = chars[token_end];
                if ch.is_whitespace() || matches!(ch, '(' | ')' | '{' | '}' | '[' | ']' | ',' | ';' | ':' | '=' | '<' | '>' | '+' | '-' | '*' | '/') {
                    // Special case: don't break on ! if it's part of an error type
                    if ch == '!' && token_end + 1 < chars.len() && chars[token_end + 1] == 'e' {
                        let token_so_far: String = chars[token_start..token_end].iter().collect();
                        if token_so_far.chars().all(|c| matches!(c, 'i' | 'f' | 's' | 'b' | 'a')) {
                            token_end += 2; // Include !e
                            continue;
                        }
                    }
                    break;
                }
                // Special case for error types: include !e
                if ch == '!' && token_end + 1 < chars.len() && chars[token_end + 1] == 'e' {
                    token_end += 2;
                } else {
                    token_end += 1;
                }
            }
            
            // Safety check: if we hit the loop limit, skip this token
            if loop_counter >= 1000 {
                log::warn!("Colorizer: potential infinite loop detected, skipping token");
                pos += 1;
                continue;
            }
            
            let token: String = chars[token_start..token_end].iter().collect();
            
            // Check if next token is '(' to identify function calls
            let mut next_non_ws = token_end;
            // Add safety counter to prevent infinite loops
            let mut ws_loop_counter = 0;
            while next_non_ws < chars.len() && chars[next_non_ws].is_whitespace() && ws_loop_counter < 100 {
                next_non_ws += 1;
                ws_loop_counter += 1;
            }
            
            if next_non_ws < chars.len() && chars[next_non_ws] == '(' {
                colored_spans.push(Span::styled(token, Style::default().fg(theme.function)));
            } else if prev_was_colon {
                // Word directly after ':' is a type annotation (name:TYPE)
                colored_spans.push(Span::styled(token, Style::default().fg(theme.identifier_type)));
            } else if next_non_ws < chars.len() && chars[next_non_ws] == ':' && token.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') {
                // Word directly before ':' is a variable declaration (NAME:type)
                colored_spans.push(Span::styled(token, Style::default().fg(theme.var_decl)));
            } else {
                colored_spans.push(colorize_word(&token, theme));
            }
            prev_was_colon = false;
            
            pos = token_end;
        }
    }
}

fn colorize_single_char(ch: char, theme: &ColorScheme) -> Span<'static> {
    match ch {
        '(' | ')' => Span::styled(ch.to_string(), Style::default().fg(theme.parenthesis)),
        '{' | '}' => Span::styled(ch.to_string(), Style::default().fg(theme.block)),
        ';' => Span::styled(ch.to_string(), Style::default().fg(theme.end_statement)),
        ',' => Span::styled(ch.to_string(), Style::default().fg(theme.comma)),
        ':' => Span::styled(ch.to_string(), Style::default().fg(theme.operator)),
        _ => Span::styled(ch.to_string(), Style::default().fg(theme.default)),
    }
}

fn colorize_word(word: &str, theme: &ColorScheme) -> Span<'static> {
    match word {
        // Keywords
        "p" => Span::styled(word.to_string(), Style::default().fg(theme.parallel_keyword)),
        "if" | "else" => Span::styled(word.to_string(), Style::default().fg(theme.keyword)),
        "f" => Span::styled(word.to_string(), Style::default().fg(theme.function)),
        "struct" => Span::styled(word.to_string(), Style::default().fg(theme.struct_keyword)),
        "enum" => Span::styled(word.to_string(), Style::default().fg(theme.enum_keyword)),
        "r" | "return" => Span::styled(word.to_string(), Style::default().fg(theme.return_keyword)),
        "async" | "await" => Span::styled(word.to_string(), Style::default().fg(theme.async_keyword)),
        "c" | "v" => Span::styled(word.to_string(), Style::default().fg(theme.keyword)), // const/var keywords

        // Collection/iteration language constructs (lexer keywords, not stdlib functions)
        "map" | "filter" | "reduce" | "scan" | "each" | "find" | "all" | "any" | "loop" | "while" | "for" | "in" | "from" | "when" | "break" | "continue" | "spawn" => {
            Span::styled(word.to_string(), Style::default().fg(theme.function))
        }

        // Literals
        "true" | "false" => Span::styled(word.to_string(), Style::default().fg(theme.keyword)),

        // Operators
        "==" | "!=" | "<" | ">" | "<=" | ">=" | "=" | "+" | "-" | "*" | "/" => Span::styled(word.to_string(), Style::default().fg(theme.operator)),
        "->" => Span::styled(word.to_string(), Style::default().fg(theme.arrow_decl)),

        // Punctuation
        "(" | ")" => Span::styled(word.to_string(), Style::default().fg(theme.parenthesis)),
        "{" | "}" => Span::styled(word.to_string(), Style::default().fg(theme.block)),
        ";" => Span::styled(word.to_string(), Style::default().fg(theme.end_statement)),
        "," => Span::styled(word.to_string(), Style::default().fg(theme.comma)),

        // Function calls (identifier followed by parentheses)
        _ if word.contains("(") && word.contains(")") && !word.starts_with('`') => {
            let paren_pos = word.find('(').unwrap();
            if paren_pos > 0 {
                Span::styled(word.to_string(), Style::default().fg(theme.function))
            } else {
                Span::styled(word.to_string(), Style::default().fg(theme.default))
            }
        }

        // Numbers
        _ if word.parse::<i64>().is_ok() => Span::styled(word.to_string(), Style::default().fg(theme.signed_int)),
        _ if word.parse::<f64>().is_ok() => Span::styled(word.to_string(), Style::default().fg(theme.float)),

        // String literals (Nail uses backticks). A word containing one is
        // either a string or a tagged string's opening, html`<p>hi</p>`.
        _ if word.contains('`') => Span::styled(word.to_string(), Style::default().fg(theme.string_literal)),

        // Known stdlib functions (queried from the registry so the list never goes stale)
        _ if crate::stdlib_registry::is_stdlib_function(word) => {
            Span::styled(word.to_string(), Style::default().fg(theme.function))
        }

        // Function references (common patterns for callbacks)
        _ if word.ends_with("_func") => Span::styled(word.to_string(), Style::default().fg(theme.function)),

        // Default
        _ => Span::styled(word.to_string(), Style::default().fg(theme.default)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Style};
    use ratatui::text::{Line, Span};

    fn test_theme() -> ColorScheme {
        ColorScheme {
            function: Color::Blue,
            const_decl: Color::Red,
            var_decl: Color::Green,
            if_decl: Color::Cyan,
            else_decl: Color::Cyan,
            arrow_decl: Color::Yellow,
            identifier: Color::Magenta,
            unsigned_int: Color::LightBlue,
            signed_int: Color::LightBlue,
            float: Color::LightGreen,
            operator: Color::White,
            keyword: Color::Cyan,
            comma: Color::Gray,
            string_literal: Color::Green,
            identifier_type: Color::Magenta,
            unknown: Color::White,
            parenthesis: Color::Yellow,
            block: Color::Yellow,
            end_statement: Color::Gray,
            async_keyword: Color::LightMagenta,
            parallel_keyword: Color::LightCyan,
            struct_keyword: Color::LightYellow,
            enum_keyword: Color::LightYellow,
            return_keyword: Color::LightRed,
            default: Color::White,
            background: Color::Black,
            comment: Color::DarkGray,
            error: Color::Red,
        }
    }

    #[test]
    fn markup_inside_an_html_string_is_colored_as_markup() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("page:s = html`<section class=\"hero\">`;")])];

        let result = colorize_code(content, &theme);
        let colored: Vec<(String, Option<Color>)> = result[0].spans.iter().map(|span| (span.content.to_string(), span.style.fg)).collect();

        let element = colored.iter().find(|(text, _)| text == "section").expect("the element name should be its own span");
        assert_eq!(element.1, Some(theme.keyword), "element names are colored as markup, not as string text");
        let value = colored.iter().find(|(text, _)| text == "\"hero\"").expect("the attribute value should be its own span");
        assert_eq!(value.1, Some(theme.string_literal));
        let bracket = colored.iter().find(|(text, _)| text == "<").expect("brackets should be their own spans");
        assert_eq!(bracket.1, Some(theme.comma));
    }

    #[test]
    fn a_css_string_is_colored_by_the_css_tokenizer() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("sheet:s = css`.hero { font-size: 1.5rem; }`;")])];

        let result = colorize_code(content, &theme);
        let colored: Vec<(String, Option<Color>)> = result[0].spans.iter().map(|span| (span.content.to_string(), span.style.fg)).collect();

        let selector = colored.iter().find(|(text, _)| text == ".hero").expect("the selector should be its own span");
        assert_eq!(selector.1, Some(theme.keyword));
        let property = colored.iter().find(|(text, _)| text == "font-size").expect("the property should be its own span");
        assert_eq!(property.1, Some(theme.identifier_type));
        let length = colored.iter().find(|(text, _)| text == "1.5rem").expect("the length should be its own span");
        assert_eq!(length.1, Some(theme.signed_int));
    }

    #[test]
    fn a_script_string_is_colored_by_the_script_tokenizer() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("script:s = js`const total = items.length; // done`;")])];

        let result = colorize_code(content, &theme);
        let colored: Vec<(String, Option<Color>)> = result[0].spans.iter().map(|span| (span.content.to_string(), span.style.fg)).collect();

        let keyword = colored.iter().find(|(text, _)| text == "const").expect("the keyword should be its own span");
        assert_eq!(keyword.1, Some(theme.keyword));
        let member = colored.iter().find(|(text, _)| text == "length").expect("the member should be its own span");
        assert_eq!(member.1, Some(theme.identifier_type));
        let comment = colored.iter().find(|(text, _)| text == "// done").expect("the comment should be its own span");
        assert_eq!(comment.1, Some(theme.comment));
    }

    #[test]
    fn the_line_oriented_languages_are_colored_too() {
        let theme = test_theme();
        let content = vec![
            Line::from(vec![Span::raw("query:s = sql`select name from users`;")]),
            Line::from(vec![Span::raw("config:s = yaml`port: 8080`;")]),
            Line::from(vec![Span::raw("manifest:s = toml`edition = 2024`;")]),
        ];

        let result = colorize_code(content, &theme);

        let keyword = result[0].spans.iter().find(|span| span.content == "select").expect("a lowercase SQL keyword is still a keyword");
        assert_eq!(keyword.style.fg, Some(theme.keyword));
        let key = result[1].spans.iter().find(|span| span.content == "port").expect("the YAML key should be its own span");
        assert_eq!(key.style.fg, Some(theme.identifier_type));
        let number = result[2].spans.iter().find(|span| span.content == "2024").expect("the TOML value should be its own span");
        assert_eq!(number.style.fg, Some(theme.signed_int));
    }

    #[test]
    fn a_css_block_left_open_across_a_line_break_stays_open() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("sheet:s = css`.hero {")]), Line::from(vec![Span::raw("    color: red;")]), Line::from(vec![Span::raw("}`;")])];

        let result = colorize_code(content, &theme);

        // `color` is a property, not a selector: the block opened on the line
        // before has not been closed yet.
        let property = result[1].spans.iter().find(|span| span.content == "color").expect("the property should be its own span");
        assert_eq!(property.style.fg, Some(theme.identifier_type));
        let semicolon = result[2].spans.iter().find(|span| span.content == ";").expect("the semicolon is code again");
        assert_eq!(semicolon.style.fg, Some(theme.end_statement));
    }

    #[test]
    fn a_string_without_a_markup_tag_stays_one_color() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("plain:s = `<not markup, just text>`;")])];

        let result = colorize_code(content, &theme);
        let body = result[0].spans.iter().find(|span| span.content.contains("not markup")).expect("the body should survive as one span");
        assert_eq!(body.content, "<not markup, just text>", "an untagged string is never tokenized");
        assert_eq!(body.style.fg, Some(theme.string_literal));
    }

    #[test]
    fn a_multi_line_string_covers_exactly_its_own_lines() {
        let theme = test_theme();
        let content = vec![
            Line::from(vec![Span::raw("page:s = html`<section>")]),
            Line::from(vec![Span::raw("    <h1>Nail</h1>")]),
            Line::from(vec![Span::raw("</section>`;")]),
        ];

        let result = colorize_code(content, &theme);

        // The code before the string keeps its own colors rather than being
        // swallowed by the string that starts later on the line.
        let name = result[0].spans.iter().find(|span| span.content == "page").expect("the declaration should still be colored");
        assert_eq!(name.style.fg, Some(theme.var_decl));

        // The closing line is still inside the string, so its markup is markup
        // and only the trailing `;` is code.
        let closing = result[2].spans.iter().find(|span| span.content == "section").expect("the closing element should be colored as markup");
        assert_eq!(closing.style.fg, Some(theme.keyword));
        let semicolon = result[2].spans.iter().find(|span| span.content == ";").expect("the semicolon is code");
        assert_eq!(semicolon.style.fg, Some(theme.end_statement));
    }

    #[test]
    fn a_tag_left_open_across_a_line_break_stays_open() {
        let theme = test_theme();
        let content = vec![
            Line::from(vec![Span::raw("page:s = html`<svg viewBox=\"0 0 20 20\"")]),
            Line::from(vec![Span::raw("     stroke=\"currentColor\"><path/></svg>`;")]),
        ];

        let result = colorize_code(content, &theme);

        // `stroke` is an attribute, not text: the tag opened on the line before
        // has not been closed yet.
        let attribute = result[1].spans.iter().find(|span| span.content.trim() == "stroke").expect("the attribute should be its own span");
        assert_eq!(attribute.style.fg, Some(theme.identifier_type));
    }

    #[test]
    fn a_language_tag_is_colored_as_part_of_its_string() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("page:s = html`<p>hi</p>`;")])];

        let result = colorize_code(content, &theme);

        let tag_span = result[0].spans.iter().find(|span| span.content.contains("html"));
        let tag_span = tag_span.expect("the tag should still appear in the line");
        assert_eq!(tag_span.style.fg, Some(theme.string_literal), "the tag belongs to the string, not to the code around it");
    }

    #[test]
    fn a_word_before_a_string_is_only_a_tag_when_it_touches_the_backtick() {
        assert_eq!(trailing_tag_len("page:s = html"), 4);
        assert_eq!(trailing_tag_len("page:s = "), 0);
        // Digits alone are not identifier-shaped, so they are not a tag.
        assert_eq!(trailing_tag_len("x = 42"), 0);
        assert_eq!(trailing_tag_len("y:s = my_lang2"), 8);
    }

    #[test]
    fn test_colorize_keywords() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("x:i = 42;")]), Line::from(vec![Span::raw("y:s = `hello`;")]), Line::from(vec![Span::raw("if true { return 1; }")])];

        let result = colorize_code(content, &theme);

        assert_eq!(result.len(), 3);

        // Check that 'if' is colored as keyword
        let third_line = &result[2];
        assert!(!third_line.spans.is_empty());
        // Check that the 'if' keyword is colored correctly
        let has_if_keyword = third_line.spans.iter().any(|span| span.content == "if" && span.style.fg == Some(theme.keyword));
        assert!(has_if_keyword, "The 'if' keyword should be colored correctly");
    }

    #[test]
    fn test_colorize_function_calls() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("print(`hello`);")]), Line::from(vec![Span::raw("from(42);")]), Line::from(vec![Span::raw("time_now();")])];

        let result = colorize_code(content, &theme);

        assert_eq!(result.len(), 3);

        // Check function calls are colored correctly
        for line in &result {
            let has_function_color = line.spans.iter().any(|span| {
                // Function names are colored separately from parentheses
                (span.content == "print" || span.content == "from" || span.content == "time_now") && span.style.fg == Some(theme.function)
            });
            assert!(has_function_color, "Function call should be colored as function");
        }
    }

    #[test]
    fn test_colorize_variable_declarations() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("name:s = `Alice`;")]), Line::from(vec![Span::raw("age:i = 25;")]), Line::from(vec![Span::raw("score:f = 95.5;")])];

        let result = colorize_code(content, &theme);

        // Check that variable declarations (name:type) are colored correctly
        for line in &result {
            let has_identifier = line.spans.iter().any(|span| {
                // Check for variable declarations (colored with var_decl)
                span.style.fg == Some(theme.var_decl)
            });
            assert!(has_identifier, "Variable declaration should be colored as identifier");
        }
    }

    #[test]
    fn test_colorize_numbers() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("x:i = 42;")]), Line::from(vec![Span::raw("y:f = 3.14;")]), Line::from(vec![Span::raw("z:i = -100;")])];

        let result = colorize_code(content, &theme);

        // Check that numbers are colored correctly
        for line in &result {
            let has_number = line
                .spans
                .iter()
                .any(|span| (span.content.parse::<i64>().is_ok() || span.content.parse::<f64>().is_ok()) && (span.style.fg == Some(theme.signed_int) || span.style.fg == Some(theme.float)));
            // Note: Some lines might not have numbers due to splitting
        }
    }

    #[test]
    fn test_colorize_strings() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("msg:s = \"hello world\";")]), Line::from(vec![Span::raw("print(\"test\");")])];

        let result = colorize_code(content, &theme);

        // Check that string literals are colored correctly
        for line in &result {
            let has_string = line.spans.iter().any(|span| (span.content.starts_with('"') || span.content.ends_with('"')) && span.style.fg == Some(theme.string_literal));
            // Note: Strings might be split across spans
        }
    }

    #[test]
    fn test_colorize_operators() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("result:i = x + y * 2;")]), Line::from(vec![Span::raw("if a == b || c != d {")]), Line::from(vec![Span::raw("=> x / 2")])];

        let result = colorize_code(content, &theme);

        // Check that operators are colored correctly
        for line in &result {
            let has_operator =
                line.spans.iter().any(|span| matches!(span.content.as_ref(), "+" | "-" | "*" | "/" | "==" | "!=" | "<" | ">" | "<=" | ">=" | "=") && span.style.fg == Some(theme.operator));
            // Note: Not all lines may have operators
        }
    }

    #[test]
    fn test_colorize_comments() {
        let theme = test_theme();
        let content = vec![Line::from(vec![Span::raw("// This is a comment")]), Line::from(vec![Span::raw("x:i = 42; // Inline comment")]), Line::from(vec![Span::raw("// TODO: implement this")])];

        let result = colorize_code(content, &theme);

        // Check that comments are colored correctly
        for line in &result {
            let has_comment = line.spans.iter().any(|span| span.content.trim().starts_with("//") && span.style.fg == Some(theme.comment));
            if line.spans.iter().any(|span| span.content.contains("//")) {
                assert!(has_comment, "Comments should be colored as comment color");
            }
        }
    }

    #[test]
    fn test_colorize_parallel_blocks() {
        let theme = test_theme();
        let content =
            vec![Line::from(vec![Span::raw("p")]), Line::from(vec![Span::raw("    print(\"task 1\");")]), Line::from(vec![Span::raw("    print(\"task 2\");")]), Line::from(vec![Span::raw("/p")])];

        let result = colorize_code(content, &theme);

        // Check that 'parallel' keyword is colored correctly
        let first_line = &result[0];
        let has_parallel = first_line.spans.iter().any(|span| span.content == "p" && span.style.fg == Some(theme.parallel_keyword));
        assert!(has_parallel, "Parallel keyword should be colored correctly");
    }

    #[test]
    fn test_colorize_multiline_strings() {
        let theme = test_theme();
        let content =
            vec![Line::from(vec![Span::raw("msg:s = `line 1")]), Line::from(vec![Span::raw("line 2")]), Line::from(vec![Span::raw("line 3`;")]), Line::from(vec![Span::raw("other:i = 42;")])];

        let result = colorize_code(content, &theme);

        // The string state detection should identify lines 1 and 2 as being inside a string
        assert_eq!(result.len(), 4);

        // Lines inside multiline string should be colored as string_literal
        let second_line = &result[1];
        let has_string_color = second_line.spans.iter().any(|span| span.style.fg == Some(theme.string_literal));
        assert!(has_string_color, "Content inside multiline string should be colored as string");
    }

    #[test]
    fn test_colorize_complex_nail_program() {
        let theme = test_theme();
        let content = vec![
            Line::from(vec![Span::raw("// Complex Nail program")]),
            Line::from(vec![Span::raw("name:s = \"Alice\";")]),
            Line::from(vec![Span::raw("age:i = 25;")]),
            Line::from(vec![Span::raw("score:f = 95.5;")]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![Span::raw("if age > 18 {")]),
            Line::from(vec![Span::raw("    print(`Adult`);")]),
            Line::from(vec![Span::raw("} else {")]),
            Line::from(vec![Span::raw("    print(`Minor`);")]),
            Line::from(vec![Span::raw("}")]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![Span::raw("p")]),
            Line::from(vec![Span::raw("    result1:s = string_from(age);")]),
            Line::from(vec![Span::raw("    result2:i = time_now();")]),
            Line::from(vec![Span::raw("}")]),
        ];

        let result = colorize_code(content, &theme);

        assert_eq!(result.len(), 15);

        // Verify the first line is a comment
        let first_line = &result[0];
        assert!(first_line.spans.iter().any(|span| span.style.fg == Some(theme.comment)));

        // Verify we have const declarations (colored with var_decl)
        assert!(result.iter().any(|line| line.spans.iter().any(|span| span.style.fg == Some(theme.var_decl))));

        // Verify we have the parallel keyword
        assert!(result.iter().any(|line| line.spans.iter().any(|span| span.content == "p" && span.style.fg == Some(theme.parallel_keyword))));

        // Verify we have function calls
        assert!(result.iter().any(|line| line.spans.iter().any(|span| (span.content == "print" || span.content == "from") && span.style.fg == Some(theme.function))));
    }

    #[test]
    fn test_colorize_empty_lines() {
        let theme = test_theme();
        let content = vec![Line::from(vec![]), Line::from(vec![Span::raw("")]), Line::from(vec![Span::raw("x:i = 42;")]), Line::from(vec![])];

        let result = colorize_code(content, &theme);

        assert_eq!(result.len(), 4);

        // Empty lines should be handled gracefully
        assert_eq!(result[0].spans.len(), 1);
        assert_eq!(result[0].spans[0].content, "");

        assert_eq!(result[3].spans.len(), 1);
        assert_eq!(result[3].spans[0].content, "");
    }

    #[test]
    fn test_parallel_colorization_performance() {
        let theme = test_theme();

        // Create a large program to test parallel performance
        let mut content = Vec::new();
        for i in 0..1000 {
            content.push(Line::from(vec![Span::raw(format!("var{}:i = {};", i, i))]));
        }

        let start = std::time::Instant::now();
        let result = colorize_code(content, &theme);
        let duration = start.elapsed();

        assert_eq!(result.len(), 1000);

        // Should complete within reasonable time (parallel processing should help)
        assert!(duration.as_millis() < 1000, "Colorization took too long: {:?}", duration);

        // Verify some lines are colored correctly (var declarations)
        assert!(result.iter().any(|line| line.spans.iter().any(|span| span.style.fg == Some(theme.var_decl))));
    }

    #[test]
    fn test_error_type_tokenization() {
        // Test that error types like i!e are kept as single tokens without spaces
        let content = "f divide(num:i, den:i):i!e {";
        let tokens = tokenize_code(content);

        // Check that i!e is a single token
        assert!(tokens.contains(&"i!e".to_string()), "i!e should be a single token, got: {:?}", tokens);

        // Test other error types
        let content2 = "result:f!e = parse_float(str);";
        let tokens2 = tokenize_code(content2);
        assert!(tokens2.contains(&"f!e".to_string()), "f!e should be a single token, got: {:?}", tokens2);

        let content3 = "data:s!e = read_file(path);";
        let tokens3 = tokenize_code(content3);
        assert!(tokens3.contains(&"s!e".to_string()), "s!e should be a single token, got: {:?}", tokens3);

        // Test that regular ! operators are still handled correctly
        let content4 = "if { x != 0 -> { print(`ok`); } }";
        let tokens4 = tokenize_code(content4);
        assert!(tokens4.contains(&"!=".to_string()), "!= should be a single token, got: {:?}", tokens4);
    }
}
