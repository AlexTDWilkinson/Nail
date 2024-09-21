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
        const_decl: hex_to_color("#555170"),       // Deep Pink
        var_decl: hex_to_color("#555170"),       // Deep Pink
        if_decl: hex_to_color("#8823AA"),        // Deep Purple
        else_decl: hex_to_color("#8823AA"),      // Deep Purple (same as if)
        arrow_decl: hex_to_color("#0B8A8F"),     // Deep Teal
        identifier: hex_to_color("#383A42"),     // Dark Gray
        unsigned_int: hex_to_color("#C4630E"),   // Brown
        signed_int: hex_to_color("#C4630E"),     // Brown (same as unsigned)
        float: hex_to_color("#177C31"),          // Dark Green
        operator: hex_to_color("#0B8A8F"),       // Deep Teal
        keyword: hex_to_color("#8823AA"),        // Deep Purple
        comma: hex_to_color("#5C6370"),          // Medium Gray
        string_literal: hex_to_color("#177C31"), // Dark Green
        rust_literal: hex_to_color("#C4630E"),   // Brown
        identifier_type: hex_to_color("#0550AE"), // Dark Blue
        unknown: hex_to_color("#5C6370"),        // Medium Gray
        parenthesis: hex_to_color("#5C6370"),    // Medium Gray
        block: hex_to_color("#0B8A8F"),          // Deep Teal
        end_statement: hex_to_color("#5C6370"),  // Medium Gray
        async_keyword: hex_to_color("#C4630E"),  // Brown
        parallel_keyword: hex_to_color("#C4630E"),// Brown
        struct_keyword: hex_to_color("#D91A60"), // Deep Pink
        enum_keyword: hex_to_color("#D91A60"),   // Deep Pink
        return_keyword: hex_to_color("#8823AA"), // Deep Purple
        default: hex_to_color("#383A42"),        // Dark Gray
        background: hex_to_color("#e4e5d6"),     // Very Light Gray (almost white)
        comment: hex_to_color("#7a7771"),        // Light Gray
        error: hex_to_color("#D91A60"),          // Deep Pink
    };

    pub static ref DARK_THEME: ColorScheme = ColorScheme {
        function: hex_to_color("#61AFEF"),       // Light Blue
        const_decl: hex_to_color("#E06C75"),       // Soft Red
        var_decl: hex_to_color("#E06C75"),       // Soft Red
        if_decl: hex_to_color("#C678DD"),        // Light Purple
        else_decl: hex_to_color("#C678DD"),      // Light Purple (same as if)
        arrow_decl: hex_to_color("#56B6C2"),     // Light Cyan
        identifier: hex_to_color("#ABB2BF"),     // Light Gray
        unsigned_int: hex_to_color("#D19A66"),   // Light Orange
        signed_int: hex_to_color("#D19A66"),     // Light Orange (same as unsigned)
        float: hex_to_color("#98C379"),          // Light Green
        operator: hex_to_color("#e3d1da"),       // Soft Red
        keyword: hex_to_color("#C678DD"),        // Light Purple
        comma: hex_to_color("#ABB2BF"),          // Light Gray
        string_literal: hex_to_color("#ccc689"), // Light Green
        rust_literal: hex_to_color("#E5C07B"),   // Light Yellow
        identifier_type: hex_to_color("#61AFEF"), // Light Blue
                    unknown: hex_to_color("#ABB2BF"),        // Light Gray
        parenthesis: hex_to_color("#ABB2BF"),    // Light Gray
        block: hex_to_color("#56B6C2"),          // Light Cyan
        end_statement: hex_to_color("#ABB2BF"),  // Light Gray
        async_keyword: hex_to_color("#E5C07B"),  // Light Yellow
        parallel_keyword: hex_to_color("#E5C07B"),// Light Yellow
        struct_keyword: hex_to_color("#E06C75"), // Soft Red
        enum_keyword: hex_to_color("#E06C75"),   // Soft Red
        return_keyword: hex_to_color("#C678DD"), // Light Purple
        default: hex_to_color("#ABB2BF"),        // Light Gray
        background: hex_to_color("#18181a"),     // Dark background (slightly lighter than pure black)
        comment: hex_to_color("#8282a0"),        // Medium Gray
        error: hex_to_color("#E06C75"),          // Soft Red
    };



}

use std::sync::Mutex;

pub fn colorize_code(content: &str, theme: &ColorScheme) -> Vec<Line<'static>> {
    let result = Mutex::new(Vec::new());

    content.lines().for_each(|line| {
        let mut line_spans = Vec::new();

        if line.trim().is_empty() {
            // For empty lines, just add an empty Line
            result.lock().unwrap().push(Line::from(vec![Span::raw("")]));
        } else {
            for (index, word) in line.split(" ").enumerate() {
                let styled_span = match word {
                    _ if line.trim().starts_with("//") => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.comment),
                    )],

                    // do some like "if word matches regex, etc"
                    // do this for tough ones to get like print(message:s):s
                    // print
                    // _ if word.matches("^[a-zA-Z_][a-zA-Z0-9_]*$").count() > 0 => {
                    //     vec![Span::styled(
                    //         word.to_string(),
                    //         Style::default().fg(theme.identifier),
                    //     )]
                    // }

                    // if starts with ^[ and ends with ]^ color it and everything between
                    "R{" => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.block),
                    )],
                    "c" => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.const_decl),
                    )],
                    "v" => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.const_decl),
                    )],
                    "true" | "false" => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.rust_literal),
                    )],
                    "async" | "await" => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.async_keyword),
                    )],
                    "par" => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.parallel_keyword),
                    )],
                    "struct" => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.struct_keyword),
                    )],
                    "enum" => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.enum_keyword),
                    )],
                    "return" => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.return_keyword),
                    )],
                    "fn" => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.function),
                    )],
                    "if" | "else" => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.keyword),
                    )],
                    "=>" => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.arrow_decl),
                    )],
                    "==" | "!=" | "<" | ">" | "<=" | ">=" | "=" | "+" | "-" | "*" | "/" => {
                        vec![Span::styled(
                            word.to_string(),
                            Style::default().fg(theme.operator),
                        )]
                    }
                    "(" | ")" => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.parenthesis),
                    )],
                    ";" => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.end_statement),
                    )],
                    "," => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.comma),
                    )],

                    // lone } at end of line is a block close
                    "{" | "}" => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.block),
                    )],

                    //   print(hello_world);
                    _ if word.contains("(") && word.contains(")") => {
                        let mut split = word.split("(");
                        // color the first word as a function identifier
                        let first = split.next().unwrap_or_default();
                        let second = split.next().unwrap_or_default();
                        if !first.is_empty() && !second.is_empty() {
                            vec![
                                Span::styled(
                                    first.to_string(),
                                    Style::default().fg(theme.function),
                                ),
                                Span::styled(
                                    "(".to_string(),
                                    Style::default().fg(theme.parenthesis),
                                ),
                                Span::styled(
                                    second.to_string(),
                                    Style::default().fg(theme.parenthesis),
                                ),
                            ]
                        } else {
                            vec![Span::styled(
                                word.to_string(),
                                Style::default().fg(theme.string_literal),
                            )]
                        }
                    }
                    // if a line does not start with c or fn it is a string literal
                    _ if !line.starts_with("fn") && !line.starts_with("c") => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.string_literal),
                    )],

                    _ if index > 0 && line.split(" ").nth(index - 1).unwrap() == "c" => {
                        if word.contains(":") {
                            let mut split = word.split(":");
                            let first = split.next().unwrap_or_default();
                            let second = split.next().unwrap_or_default();
                            if !first.is_empty() && !second.is_empty() {
                                vec![
                                    Span::styled(
                                        first.to_string(),
                                        Style::default().fg(theme.identifier),
                                    ),
                                    Span::styled(
                                        ":".to_string(),
                                        Style::default().fg(theme.operator),
                                    ),
                                    Span::styled(
                                        second.to_string(),
                                        Style::default().fg(theme.identifier_type),
                                    ),
                                ]
                            } else {
                                vec![Span::styled(
                                    word.to_string(),
                                    Style::default().fg(theme.string_literal),
                                )]
                            }
                        } else {
                            vec![Span::styled(
                                word.to_string(),
                                Style::default().fg(theme.string_literal),
                            )]
                        }
                    }
                    _ if word.contains(":") => {
                        let mut split = word.split(":");
                        let first = split.next().unwrap_or_default();
                        let second = split.next().unwrap_or_default();
                        if !first.is_empty() && !second.is_empty() {
                            vec![
                                Span::styled(
                                    first.to_string(),
                                    Style::default().fg(theme.identifier),
                                ),
                                Span::styled(":".to_string(), Style::default().fg(theme.operator)),
                                Span::styled(
                                    second.to_string(),
                                    Style::default().fg(theme.identifier_type),
                                ),
                            ]
                        } else {
                            vec![Span::styled(
                                word.to_string(),
                                Style::default().fg(theme.string_literal),
                            )]
                        }
                    }

                    _ if word.parse::<f64>().is_ok() => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.signed_int),
                    )],
                    _ => vec![Span::styled(
                        word.to_string(),
                        Style::default().fg(theme.string_literal),
                    )],
                };
                line_spans.extend(styled_span);

                // Add a space after each word (except the last one)
                if index < line.split(" ").count() - 1 {
                    line_spans.push(Span::raw(" "));
                }
            }

            // Add the line to the result
            result.lock().unwrap().push(Line::from(line_spans));
        }
    });

    result.into_inner().expect("Failed to unwrap result")
}
