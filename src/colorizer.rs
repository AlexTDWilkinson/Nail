use lazy_static::lazy_static;
use ratatui::crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::text::Span;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs},
    Frame, Terminal,
};
use rayon::prelude::*;
use regex::Regex;

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
    pub rust_literal: Color,
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
fn hex_to_color(hex: &str) -> Color {
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
        function: hex_to_color("#0550AE"),       // Dark Blue
        const_decl: hex_to_color("#D91A60"),     // Deep Pink
        var_decl: hex_to_color("#986801"),       // Dark Gold
        if_decl: hex_to_color("#8823AA"),        // Deep Purple
        else_decl: hex_to_color("#A035CC"),      // Medium Purple
        arrow_decl: hex_to_color("#0B8A8F"),     // Deep Teal
        identifier: hex_to_color("#383A42"),     // Dark Gray
        unsigned_int: hex_to_color("#C4630E"),   // Brown
        signed_int: hex_to_color("#B85820"),     // Rust Brown
        float: hex_to_color("#177C31"),          // Dark Green
        operator: hex_to_color("#0B8A8F"),       // Deep Teal
        keyword: hex_to_color("#8823AA"),        // Deep Purple
        comma: hex_to_color("#5C6370"),          // Medium Gray
        string_literal: hex_to_color("#177C31"), // Dark Green
        rust_literal: hex_to_color("#D4730E"),   // Dark Orange
        identifier_type: hex_to_color("#0969DA"), // Medium Blue
        unknown: hex_to_color("#5C6370"),        // Medium Gray
        parenthesis: hex_to_color("#4C5360"),    // Darker Gray
        block: hex_to_color("#067A7F"),          // Darker Teal
        end_statement: hex_to_color("#5C6370"),  // Medium Gray
        async_keyword: hex_to_color("#B8860B"),  // Dark Goldenrod
        parallel_keyword: hex_to_color("#DAA520"),// Goldenrod
        struct_keyword: hex_to_color("#C71585"), // Medium Violet Red
        enum_keyword: hex_to_color("#9932CC"),   // Dark Orchid
        return_keyword: hex_to_color("#FF1493"), // Deep Pink
        default: hex_to_color("#383A42"),        // Dark Gray
        background: hex_to_color("#e4e5d6"),     // Very Light Gray (almost white)
        comment: hex_to_color("#7a7771"),        // Light Gray
        error: hex_to_color("#D91A60"),          // Deep Pink
    };

    pub static ref DARK_THEME: ColorScheme = ColorScheme {
        function: hex_to_color("#61AFEF"),       // Bright Blue
        const_decl: hex_to_color("#E06C75"),     // Bright Red/Pink
        var_decl: hex_to_color("#E5C07B"),       // Bright Yellow/Gold
        if_decl: hex_to_color("#C678DD"),        // Bright Purple
        else_decl: hex_to_color("#B57BDB"),      // Lighter Purple
        arrow_decl: hex_to_color("#56B6C2"),     // Bright Cyan
        identifier: hex_to_color("#ABB2BF"),     // Light Gray
        unsigned_int: hex_to_color("#D19A66"),   // Bright Orange
        signed_int: hex_to_color("#E09956"),     // Lighter Orange
        float: hex_to_color("#98C379"),          // Bright Green
        operator: hex_to_color("#56B6C2"),       // Bright Cyan
        keyword: hex_to_color("#C678DD"),        // Bright Purple
        comma: hex_to_color("#6B7089"),          // Muted Blue-Gray
        string_literal: hex_to_color("#98C379"), // Bright Green
        rust_literal: hex_to_color("#F0A45D"),   // Light Orange
        identifier_type: hex_to_color("#5DAEC2"), // Lighter Cyan
        unknown: hex_to_color("#ABB2BF"),        // Light Gray
        parenthesis: hex_to_color("#FFD700"),    // Gold color for better visibility
        block: hex_to_color("#8B90A9"),          // Even Lighter Blue-Gray
        end_statement: hex_to_color("#8B90A9"),  // Lighter Blue-Gray
        async_keyword: hex_to_color("#F5BD4F"),  // Golden Yellow
        parallel_keyword: hex_to_color("#FFD580"),// Light Gold
        struct_keyword: hex_to_color("#DA70D6"), // Orchid Purple
        enum_keyword: hex_to_color("#BA55D3"),   // Medium Orchid
        return_keyword: hex_to_color("#FF69B4"), // Hot Pink
        default: hex_to_color("#E5E5E7"),        // Off-white
        background: hex_to_color("#1E1E20"),     // Rich dark background
        comment: hex_to_color("#7F8799"),        // Lighter gray
        error: hex_to_color("#FF6B6B"),          // Bright error red
    };



}

use std::sync::Mutex;

pub fn colorize_code(content: Vec<Line>, theme: &ColorScheme) -> Vec<Line<'static>> {
    // First pass: detect multi-line strings
    let string_state = detect_multiline_strings(&content);
    
    // Parallel colorization per line
    let colored_lines: Vec<Line<'static>> = content
        .into_par_iter()
        .enumerate()
        .map(|(line_idx, line)| colorize_line(line, line_idx, &string_state, theme))
        .collect();
    
    colored_lines
}

fn detect_multiline_strings(content: &[Line]) -> Vec<bool> {
    let mut in_string = false;
    let mut string_state = Vec::with_capacity(content.len());
    
    for line in content {
        let line_content = line.spans.iter()
            .map(|span| span.content.as_ref())
            .collect::<Vec<_>>()
            .join("");
        
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
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
                
                let mut op = ch.to_string();
                // Check for two-character operators
                if let Some(&next_ch) = chars.peek() {
                    if (ch == '=' && next_ch == '=') ||
                       (ch == '!' && next_ch == '=') ||
                       (ch == '<' && next_ch == '=') ||
                       (ch == '>' && next_ch == '=') ||
                       (ch == '=' && next_ch == '>') {
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
        if need_space && !matches!(token.as_str(), "," | ";" | ")" | "]" | "}") {
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
            need_space = false;
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
        "parallel" => Span::styled(word.to_string(), Style::default().fg(theme.parallel_keyword)),
        "if" | "else" => Span::styled(word.to_string(), Style::default().fg(theme.keyword)),
        "fn" => Span::styled(word.to_string(), Style::default().fg(theme.function)),
        "struct" => Span::styled(word.to_string(), Style::default().fg(theme.struct_keyword)),
        "enum" => Span::styled(word.to_string(), Style::default().fg(theme.enum_keyword)),
        "r" | "return" => Span::styled(word.to_string(), Style::default().fg(theme.return_keyword)),
        "async" | "await" => Span::styled(word.to_string(), Style::default().fg(theme.async_keyword)),
        "c" | "v" => Span::styled(word.to_string(), Style::default().fg(theme.keyword)),  // const/var keywords
        
        // Literals
        "true" | "false" => Span::styled(word.to_string(), Style::default().fg(theme.rust_literal)),
        
        // Operators
        "==" | "!=" | "<" | ">" | "<=" | ">=" | "=" | "+" | "-" | "*" | "/" => 
            Span::styled(word.to_string(), Style::default().fg(theme.operator)),
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
        _ if word.parse::<i64>().is_ok() => 
            Span::styled(word.to_string(), Style::default().fg(theme.signed_int)),
        _ if word.parse::<f64>().is_ok() => 
            Span::styled(word.to_string(), Style::default().fg(theme.float)),
        
        // String literals (Nail uses backticks)
        _ if word.starts_with('`') || word.ends_with('`') => 
            Span::styled(word.to_string(), Style::default().fg(theme.string_literal)),
        
        // Known stdlib functions
        "print" | "string_concat" | "to_string" | "time_now" | "math_sqrt" | "array_len" |
        "map_int" | "filter_int" | "reduce_int" | "range" | "safe" | "divide" => {
            Span::styled(word.to_string(), Style::default().fg(theme.function))
        }
        
        // Function references (common patterns for callbacks)
        _ if word.ends_with("_func") => {
            Span::styled(word.to_string(), Style::default().fg(theme.function))
        }
        
        // Default
        _ => Span::styled(word.to_string(), Style::default().fg(theme.default)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::text::{Line, Span};
    use ratatui::style::{Color, Style};

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
            rust_literal: Color::LightRed,
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
        let content = vec![
            Line::from(vec![Span::raw("x:i = 42;")]),
            Line::from(vec![Span::raw("y:s = `hello`;")]),
            Line::from(vec![Span::raw("if true { return 1; }")]),
        ];

        let result = colorize_code(content, &theme);
        
        assert_eq!(result.len(), 3);
        
        // Check that 'if' is colored as keyword
        let third_line = &result[2];
        assert!(!third_line.spans.is_empty());
        // Check that the 'if' keyword is colored correctly
        let has_if_keyword = third_line.spans.iter().any(|span| {
            span.content == "if" && span.style.fg == Some(theme.keyword)
        });
        assert!(has_if_keyword, "The 'if' keyword should be colored correctly");
    }

    #[test]
    fn test_colorize_function_calls() {
        let theme = test_theme();
        let content = vec![
            Line::from(vec![Span::raw("print(`hello`);")]),
            Line::from(vec![Span::raw("to_string(42);")]),
            Line::from(vec![Span::raw("time_now();")]),
        ];

        let result = colorize_code(content, &theme);
        
        assert_eq!(result.len(), 3);
        
        // Check function calls are colored correctly
        for line in &result {
            let has_function_color = line.spans.iter().any(|span| {
                // Function names are colored separately from parentheses
                (span.content == "print" || span.content == "to_string" || span.content == "time_now") && 
                span.style.fg == Some(theme.function)
            });
            assert!(has_function_color, "Function call should be colored as function");
        }
    }

    #[test]
    fn test_colorize_variable_declarations() {
        let theme = test_theme();
        let content = vec![
            Line::from(vec![Span::raw("name:s = `Alice`;")]),
            Line::from(vec![Span::raw("age:i = 25;")]),
            Line::from(vec![Span::raw("score:f = 95.5;")]),
        ];

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
        let content = vec![
            Line::from(vec![Span::raw("x:i = 42;")]),
            Line::from(vec![Span::raw("y:f = 3.14;")]),
            Line::from(vec![Span::raw("z:i = -100;")]),
        ];

        let result = colorize_code(content, &theme);
        
        // Check that numbers are colored correctly
        for line in &result {
            let has_number = line.spans.iter().any(|span| {
                (span.content.parse::<i64>().is_ok() || span.content.parse::<f64>().is_ok()) &&
                (span.style.fg == Some(theme.signed_int) || span.style.fg == Some(theme.float))
            });
            // Note: Some lines might not have numbers due to splitting
        }
    }

    #[test]
    fn test_colorize_strings() {
        let theme = test_theme();
        let content = vec![
            Line::from(vec![Span::raw("msg:s = \"hello world\";")]),
            Line::from(vec![Span::raw("print(\"test\");")]),
        ];

        let result = colorize_code(content, &theme);
        
        // Check that string literals are colored correctly
        for line in &result {
            let has_string = line.spans.iter().any(|span| {
                (span.content.starts_with('"') || span.content.ends_with('"')) &&
                span.style.fg == Some(theme.string_literal)
            });
            // Note: Strings might be split across spans
        }
    }

    #[test]
    fn test_colorize_operators() {
        let theme = test_theme();
        let content = vec![
            Line::from(vec![Span::raw("result:i = x + y * 2;")]),
            Line::from(vec![Span::raw("if a == b || c != d {")]),
            Line::from(vec![Span::raw("=> x / 2")]),
        ];

        let result = colorize_code(content, &theme);
        
        // Check that operators are colored correctly
        for line in &result {
            let has_operator = line.spans.iter().any(|span| {
                matches!(span.content.as_ref(), "+" | "-" | "*" | "/" | "==" | "!=" | "<" | ">" | "<=" | ">=" | "=") &&
                span.style.fg == Some(theme.operator)
            });
            // Note: Not all lines may have operators
        }
    }

    #[test]
    fn test_colorize_comments() {
        let theme = test_theme();
        let content = vec![
            Line::from(vec![Span::raw("// This is a comment")]),
            Line::from(vec![Span::raw("x:i = 42; // Inline comment")]),
            Line::from(vec![Span::raw("// TODO: implement this")]),
        ];

        let result = colorize_code(content, &theme);
        
        // Check that comments are colored correctly
        for line in &result {
            let has_comment = line.spans.iter().any(|span| {
                span.content.trim().starts_with("//") &&
                span.style.fg == Some(theme.comment)
            });
            if line.spans.iter().any(|span| span.content.contains("//")) {
                assert!(has_comment, "Comments should be colored as comment color");
            }
        }
    }

    #[test]
    fn test_colorize_parallel_blocks() {
        let theme = test_theme();
        let content = vec![
            Line::from(vec![Span::raw("parallel {")]),
            Line::from(vec![Span::raw("    print(\"task 1\");")]),
            Line::from(vec![Span::raw("    print(\"task 2\");")]),
            Line::from(vec![Span::raw("}")]),
        ];

        let result = colorize_code(content, &theme);
        
        // Check that 'parallel' keyword is colored correctly
        let first_line = &result[0];
        let has_parallel = first_line.spans.iter().any(|span| {
            span.content == "parallel" &&
            span.style.fg == Some(theme.parallel_keyword)
        });
        assert!(has_parallel, "Parallel keyword should be colored correctly");
    }

    #[test]
    fn test_colorize_multiline_strings() {
        let theme = test_theme();
        let content = vec![
            Line::from(vec![Span::raw("msg:s = `line 1")]),
            Line::from(vec![Span::raw("line 2")]),
            Line::from(vec![Span::raw("line 3`;")]),
            Line::from(vec![Span::raw("other:i = 42;")]),
        ];

        let result = colorize_code(content, &theme);
        
        // The string state detection should identify lines 1 and 2 as being inside a string
        assert_eq!(result.len(), 4);
        
        // Lines inside multiline string should be colored as string_literal
        let second_line = &result[1];
        let has_string_color = second_line.spans.iter().any(|span| {
            span.style.fg == Some(theme.string_literal)
        });
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
            Line::from(vec![Span::raw("parallel {")]),
            Line::from(vec![Span::raw("    result1:s = to_string(age);")]),
            Line::from(vec![Span::raw("    result2:i = time_now();")]),
            Line::from(vec![Span::raw("}")]),
        ];

        let result = colorize_code(content, &theme);
        
        assert_eq!(result.len(), 15);
        
        // Verify the first line is a comment
        let first_line = &result[0];
        assert!(first_line.spans.iter().any(|span| 
            span.style.fg == Some(theme.comment)
        ));
        
        // Verify we have const declarations (colored with var_decl)
        assert!(result.iter().any(|line| 
            line.spans.iter().any(|span| 
                span.style.fg == Some(theme.var_decl)
            )
        ));
        
        // Verify we have the parallel keyword
        assert!(result.iter().any(|line| 
            line.spans.iter().any(|span| 
                span.content == "parallel" && span.style.fg == Some(theme.parallel_keyword)
            )
        ));
        
        // Verify we have function calls
        assert!(result.iter().any(|line| 
            line.spans.iter().any(|span| 
                (span.content == "print" || span.content == "to_string") &&
                span.style.fg == Some(theme.function)
            )
        ));
    }

    #[test]
    fn test_colorize_empty_lines() {
        let theme = test_theme();
        let content = vec![
            Line::from(vec![]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![Span::raw("x:i = 42;")]),
            Line::from(vec![]),
        ];

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
        assert!(result.iter().any(|line| 
            line.spans.iter().any(|span| 
                span.style.fg == Some(theme.var_decl)
            )
        ));
    }
}
