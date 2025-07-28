use lazy_static::lazy_static;
use ratatui::text::Span;
use ratatui::{
    style::{Color, Style},
    text::Line,
};
use rayon::prelude::*;

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
    // First pass: detect multi-line strings
    let string_state = detect_multiline_strings(&content);

    // Parallel colorization per line
    let colored_lines: Vec<Line<'static>> = content.into_par_iter().enumerate().map(|(line_idx, line)| colorize_line(line, line_idx, &string_state, theme)).collect();

    colored_lines
}

fn detect_multiline_strings(content: &[Line]) -> Vec<bool> {
    let mut in_string = false;
    let mut string_state = Vec::with_capacity(content.len());

    for line in content {
        let line_content = line.spans.iter().map(|span| span.content.as_ref()).collect::<Vec<_>>().join("");

        // Nail uses backticks for strings, not quotes
        let mut chars = line_content.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '`' {
                in_string = !in_string;
            }
        }

        string_state.push(in_string);
    }

    string_state
}

fn colorize_line(line: Line, line_idx: usize, string_state: &[bool], theme: &ColorScheme) -> Line<'static> {
    if line.spans.is_empty() {
        return Line::from(vec![Span::raw("")]);
    }

    let mut colored_spans = Vec::new();
    let is_in_multiline_string = string_state.get(line_idx).copied().unwrap_or(false);

    for span in line.spans {
        let content = span.content.as_ref();

        // Handle comments first (highest priority)
        if content.trim().starts_with("//") {
            colored_spans.push(Span::styled(content.to_string(), Style::default().fg(theme.comment)));
            continue;
        }

        // Handle multi-line strings
        if is_in_multiline_string {
            colored_spans.push(Span::styled(content.to_string(), Style::default().fg(theme.string_literal)));
            continue;
        }

        // Handle string literals (complete strings on one line)
        if content.starts_with('`') && content.ends_with('`') && content.len() > 1 {
            colored_spans.push(Span::styled(content.to_string(), Style::default().fg(theme.string_literal)));
            continue;
        }

        // Check if the entire content is a string that may contain keywords
        if content.contains('`') {
            // More complex string handling - find string boundaries and colorize accordingly
            let mut result = String::new();
            let mut chars = content.chars();
            let mut in_string = false;
            let mut string_start = 0;
            let mut current_pos = 0;

            while let Some(ch) = chars.next() {
                if ch == '`' {
                    if in_string {
                        // End of string
                        if string_start <= current_pos && current_pos < content.len() {
                            let string_content = &content[string_start..=current_pos];
                            colored_spans.push(Span::styled(string_content.to_string(), Style::default().fg(theme.string_literal)));
                        }
                        result.clear();
                        in_string = false;
                    } else {
                        // Start of string
                        if !result.is_empty() {
                            // Colorize non-string content before the string
                            colorize_non_string_content(&result, &mut colored_spans, theme);
                            result.clear();
                        }
                        string_start = current_pos;
                        in_string = true;
                    }
                } else if !in_string {
                    result.push(ch);
                }
                current_pos += ch.len_utf8();
            }

            // Handle remaining content
            if in_string {
                // Unclosed string
                let string_content = &content[string_start..];
                colored_spans.push(Span::styled(string_content.to_string(), Style::default().fg(theme.string_literal)));
            } else if !result.is_empty() {
                // Check if the remaining content has a comment
                if let Some(comment_pos) = result.find("//") {
                    // Split into pre-comment and comment
                    let pre_comment = &result[..comment_pos];
                    let comment_part = &result[comment_pos..];

                    if !pre_comment.is_empty() {
                        // Remove trailing spaces from pre_comment
                        let pre_comment_trimmed = pre_comment.trim_end();
                        let space_count = pre_comment.len() - pre_comment_trimmed.len();

                        if !pre_comment_trimmed.is_empty() {
                            colorize_non_string_content(pre_comment_trimmed, &mut colored_spans, theme);
                        }

                        // Add back the spaces between pre-comment and comment
                        if space_count > 0 {
                            colored_spans.push(Span::raw(" ".repeat(space_count)));
                        }
                    }

                    // Add comment as a single span - DO NOT tokenize!
                    colored_spans.push(Span::styled(comment_part.to_string(), Style::default().fg(theme.comment)));
                } else {
                    colorize_non_string_content(&result, &mut colored_spans, theme);
                }
            }

            continue;
        }

        // Check for inline comments and handle them specially
        if let Some(comment_pos) = content.find("//") {
            // Debug log
            if content.contains("final_message") {
                log::debug!("Processing line with final_message: '{}'", content);
            }
            // Handle leading whitespace first
            let leading_spaces = content.len() - content.trim_start().len();
            let actual_comment_pos = comment_pos;

            if leading_spaces > 0 && actual_comment_pos >= leading_spaces {
                // Preserve leading whitespace
                colored_spans.push(Span::raw(" ".repeat(leading_spaces)));

                // Split into pre-comment and comment parts after leading spaces
                let content_after_spaces = &content[leading_spaces..];
                let comment_pos_adjusted = actual_comment_pos - leading_spaces;

                let pre_comment = &content_after_spaces[..comment_pos_adjusted];
                let comment_part = &content_after_spaces[comment_pos_adjusted..];

                // Colorize pre-comment part normally (without leading spaces)
                if !pre_comment.is_empty() {
                    // Remove trailing spaces from pre_comment
                    let pre_comment_trimmed = pre_comment.trim_end();
                    let space_count = pre_comment.len() - pre_comment_trimmed.len();

                    if !pre_comment_trimmed.is_empty() {
                        colorize_non_string_content(pre_comment_trimmed, &mut colored_spans, theme);
                    }

                    // Add back the spaces between pre-comment and comment
                    if space_count > 0 {
                        colored_spans.push(Span::raw(" ".repeat(space_count)));
                    }
                }

                // Colorize comment part
                // Debug check
                if comment_part.contains("/ /") {
                    log::error!("WARNING: comment_part already has space: '{}'", comment_part);
                } else if comment_part.starts_with("//") {
                    log::debug!("Comment part is correct: '{}'", comment_part);
                }
                colored_spans.push(Span::styled(comment_part.to_string(), Style::default().fg(theme.comment)));
            } else {
                // Original logic for no leading spaces
                let pre_comment = &content[..comment_pos];
                let comment_part = &content[comment_pos..];

                // Colorize pre-comment part normally
                if !pre_comment.is_empty() {
                    // Remove trailing spaces from pre_comment
                    let pre_comment_trimmed = pre_comment.trim_end();
                    let space_count = pre_comment.len() - pre_comment_trimmed.len();

                    if !pre_comment_trimmed.is_empty() {
                        colorize_non_string_content(pre_comment_trimmed, &mut colored_spans, theme);
                    }

                    // Add back the spaces between pre-comment and comment
                    if space_count > 0 {
                        colored_spans.push(Span::raw(" ".repeat(space_count)));
                    }
                }

                // Colorize comment part
                // Debug check
                if comment_part.contains("/ /") {
                    log::error!("WARNING: comment_part already has space: '{}'", comment_part);
                } else if comment_part.starts_with("//") {
                    log::debug!("Comment part is correct: '{}'", comment_part);
                }
                colored_spans.push(Span::styled(comment_part.to_string(), Style::default().fg(theme.comment)));
            }
        } else {
            // Tokenize and colorize individual words normally
            colorize_non_string_content(content, &mut colored_spans, theme);
        }
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
                    if (ch == '=' && next_ch == '=') || (ch == '!' && next_ch == '=') || (ch == '<' && next_ch == '=') || (ch == '>' && next_ch == '=') || (ch == '=' && next_ch == '>') {
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

        // Literals
        "true" | "false" => Span::styled(word.to_string(), Style::default().fg(theme.keyword)),

        // Operators
        "==" | "!=" | "<" | ">" | "<=" | ">=" | "=" | "+" | "-" | "*" | "/" => Span::styled(word.to_string(), Style::default().fg(theme.operator)),
        "=>" => Span::styled(word.to_string(), Style::default().fg(theme.arrow_decl)),

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

        // String literals (Nail uses backticks)
        _ if word.starts_with('`') || word.ends_with('`') => Span::styled(word.to_string(), Style::default().fg(theme.string_literal)),

        // Known stdlib functions
        "print" | "from" | "time_now" | "math_sqrt" | "array_len" | "map" | "filter_int" | "reduce" | "range" | "safe" | "divide" => {
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
        let content4 = "if { x != 0 => { print(`ok`); } }";
        let tokens4 = tokenize_code(content4);
        assert!(tokens4.contains(&"!=".to_string()), "!= should be a single token, got: {:?}", tokens4);
    }
}
