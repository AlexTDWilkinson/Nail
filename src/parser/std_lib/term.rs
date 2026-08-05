//! The terminal a command-line program is talking to: colour, emphasis, size,
//! and the two or three drawings worth having built in.
//!
//! Everything that adds colour returns a string rather than printing it, so a
//! coloured value can be built up, joined, put in a table cell and printed
//! once. The escape codes are ANSI SGR sequences, understood by every terminal
//! written this century and by the Windows console since 2016.
//!
//! Colour a program cannot see the point of is worse than none: when output is
//! piped to a file, the escape codes become noise in the file. Ask
//! `term_is_tty` before colouring, or strip it back out with
//! `term_strip_styles`.

use serde::{Deserialize, Serialize};
use std::io::IsTerminal;

/// The sixteen colours every terminal agrees on. Anything more exact is a
/// guess about someone else's colour scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TERM_Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl TERM_Color {
    /// The SGR number for this colour as a foreground.
    fn foreground_code(&self) -> u8 {
        match self {
            TERM_Color::Black => 30,
            TERM_Color::Red => 31,
            TERM_Color::Green => 32,
            TERM_Color::Yellow => 33,
            TERM_Color::Blue => 34,
            TERM_Color::Magenta => 35,
            TERM_Color::Cyan => 36,
            TERM_Color::White => 37,
            TERM_Color::BrightBlack => 90,
            TERM_Color::BrightRed => 91,
            TERM_Color::BrightGreen => 92,
            TERM_Color::BrightYellow => 93,
            TERM_Color::BrightBlue => 94,
            TERM_Color::BrightMagenta => 95,
            TERM_Color::BrightCyan => 96,
            TERM_Color::BrightWhite => 97,
        }
    }
}

/// Wraps text in an SGR code and the reset that ends it.
fn wrap(code: String, text: &str) -> String {
    return format!("\u{1b}[{}m{}\u{1b}[0m", code, text);
}

/// Text in a colour.
pub fn paint(text: String, color: TERM_Color) -> String {
    return wrap(color.foreground_code().to_string(), &text);
}

/// Text on a coloured background. The background codes are the foreground
/// ones plus ten, which is the one piece of arithmetic in the ANSI standard.
pub fn background(text: String, color: TERM_Color) -> String {
    return wrap((color.foreground_code() + 10).to_string(), &text);
}

pub fn bold(text: String) -> String {
    return wrap("1".to_string(), &text);
}

pub fn dim(text: String) -> String {
    return wrap("2".to_string(), &text);
}

pub fn italic(text: String) -> String {
    return wrap("3".to_string(), &text);
}

pub fn underline(text: String) -> String {
    return wrap("4".to_string(), &text);
}

/// Swaps foreground and background. What a selected row looks like.
pub fn inverse(text: String) -> String {
    return wrap("7".to_string(), &text);
}

/// Removes every escape sequence, leaving the text as it will be read. Use it
/// when coloured output has to go somewhere that is not a terminal - a log
/// file, a test comparison, an HTTP response.
pub fn strip_styles(text: String) -> String {
    let mut out = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if character != '\u{1b}' {
            out.push(character);
            continue;
        }
        match characters.peek() {
            // A control sequence - colour, cursor movement - runs to its final
            // byte, which is a letter.
            Some('[') => {
                characters.next();
                while let Some(inside) = characters.next() {
                    if inside.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            // An operating system command - a hyperlink, a window title - runs
            // to a bell character or to an escape followed by a backslash.
            Some(']') => {
                characters.next();
                while let Some(inside) = characters.next() {
                    if inside == '\u{7}' {
                        break;
                    }
                    if inside == '\u{1b}' {
                        characters.next();
                        break;
                    }
                }
            }
            // A two-character escape: drop the character that follows.
            Some(_) => {
                characters.next();
            }
            None => {}
        }
    }
    return out;
}

/// How wide the text is once printed - the length a person sees, not the
/// number of characters in the string. Escape sequences take no width.
pub fn display_width(text: String) -> i64 {
    return strip_styles(text).chars().count() as i64;
}

/// Whether standard output is a terminal rather than a file or a pipe. False
/// means: do not colour, do not draw progress bars, do not clear the screen.
pub fn is_tty() -> bool {
    return std::io::stdout().is_terminal();
}

/// How many columns the terminal has, or 80 when there is no terminal to ask.
pub fn width() -> i64 {
    return crossterm::terminal::size().map(|(columns, _)| columns as i64).unwrap_or(80);
}

/// How many rows the terminal has, or 24 when there is no terminal to ask.
pub fn height() -> i64 {
    return crossterm::terminal::size().map(|(_, rows)| rows as i64).unwrap_or(24);
}

/// A clickable link, where the terminal supports them, and plain text where it
/// does not - the escape sequence hides the URL either way, so a terminal that
/// ignores it shows the text alone.
pub fn hyperlink(text: String, url: String) -> String {
    return format!("\u{1b}]8;;{}\u{7}{}\u{1b}]8;;\u{7}", url, text);
}

/// A progress bar of a given width, filled to the given share from 0.0 to 1.0.
/// Returns the bar as a string, so it can be printed on its own line or put
/// beside a label.
pub fn progress_bar(share: f64, width: i64) -> Result<String, String> {
    if !(0.0..=1.0).contains(&share) {
        return Err(format!("term_progress_bar: {} is not a share between 0.0 and 1.0", share));
    }
    if width < 1 {
        return Err(format!("term_progress_bar: a bar {} characters wide cannot be drawn", width));
    }

    let filled = (share * width as f64).round() as i64;
    let mut bar = String::from("[");
    for column in 0..width {
        bar.push(if column < filled { '#' } else { '.' });
    }
    bar.push(']');
    return Ok(bar);
}

/// A plain-text table with aligned columns. Every row must have as many cells
/// as there are headers, because a ragged table is a bug in the data and
/// papering over it hides that.
pub fn table(headers: Vec<String>, rows: Vec<Vec<String>>) -> Result<String, String> {
    if headers.is_empty() {
        return Err("term_table: a table needs at least one column".to_string());
    }

    for (index, row) in rows.iter().enumerate() {
        if row.len() != headers.len() {
            return Err(format!("term_table: row {} has {} cells but there are {} columns", index, row.len(), headers.len()));
        }
    }

    // Column widths are measured on what a person sees, so a coloured cell
    // does not throw the alignment out by the length of its escape codes.
    let mut widths: Vec<usize> = headers.iter().map(|header| strip_styles(header.clone()).chars().count()).collect();
    for row in rows.iter() {
        for (column, cell) in row.iter().enumerate() {
            let cell_width = strip_styles(cell.clone()).chars().count();
            if cell_width > widths[column] {
                widths[column] = cell_width;
            }
        }
    }

    /// Pads a cell to a column width, counting visible characters.
    fn pad(cell: &str, width: usize) -> String {
        let visible = strip_styles(cell.to_string()).chars().count();
        let mut out = cell.to_string();
        for _ in visible..width {
            out.push(' ');
        }
        return out;
    }

    let mut out = String::new();
    let header_line: Vec<String> = headers.iter().enumerate().map(|(column, header)| pad(header, widths[column])).collect();
    out.push_str(&header_line.join("  "));
    out.push('\n');

    let rule: Vec<String> = widths.iter().map(|width| "-".repeat(*width)).collect();
    out.push_str(&rule.join("  "));

    for row in rows.iter() {
        out.push('\n');
        let line: Vec<String> = row.iter().enumerate().map(|(column, cell)| pad(cell, widths[column])).collect();
        out.push_str(line.join("  ").trim_end());
    }

    return Ok(out);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_wraps_the_text_and_resets_after_it() {
        let painted = paint("hi".to_string(), TERM_Color::Red);
        assert_eq!(painted, "\u{1b}[31mhi\u{1b}[0m");
        assert_eq!(background("hi".to_string(), TERM_Color::Red), "\u{1b}[41mhi\u{1b}[0m");
    }

    #[test]
    fn every_colour_has_a_distinct_code() {
        let colors = [
            TERM_Color::Black,
            TERM_Color::Red,
            TERM_Color::Green,
            TERM_Color::Yellow,
            TERM_Color::Blue,
            TERM_Color::Magenta,
            TERM_Color::Cyan,
            TERM_Color::White,
            TERM_Color::BrightBlack,
            TERM_Color::BrightRed,
            TERM_Color::BrightGreen,
            TERM_Color::BrightYellow,
            TERM_Color::BrightBlue,
            TERM_Color::BrightMagenta,
            TERM_Color::BrightCyan,
            TERM_Color::BrightWhite,
        ];
        let mut codes: Vec<u8> = colors.iter().map(|color| color.foreground_code()).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), colors.len());
    }

    #[test]
    fn styles_stack_and_strip_back_to_the_original() {
        let styled = bold(paint(underline("hi".to_string()), TERM_Color::Cyan));
        assert!(styled.contains("hi"));
        assert_eq!(strip_styles(styled), "hi");
    }

    #[test]
    fn stripping_leaves_text_without_escapes_alone() {
        assert_eq!(strip_styles("plain text".to_string()), "plain text");
    }

    #[test]
    fn display_width_counts_what_is_seen() {
        assert_eq!(display_width(paint("hello".to_string(), TERM_Color::Green)), 5);
        assert_eq!(display_width("hello".to_string()), 5);
    }

    #[test]
    fn a_progress_bar_fills_in_proportion() {
        assert_eq!(progress_bar(0.0, 4).expect("a share"), "[....]");
        assert_eq!(progress_bar(0.5, 4).expect("a share"), "[##..]");
        assert_eq!(progress_bar(1.0, 4).expect("a share"), "[####]");
    }

    #[test]
    fn a_progress_bar_rejects_nonsense() {
        assert!(progress_bar(2.0, 4).unwrap_err().contains("not a share"));
        assert!(progress_bar(0.5, 0).unwrap_err().contains("cannot be drawn"));
    }

    #[test]
    fn a_table_aligns_its_columns() {
        let rendered = table(
            vec!["name".to_string(), "count".to_string()],
            vec![vec!["apples".to_string(), "3".to_string()], vec!["figs".to_string(), "12".to_string()]],
        )
        .expect("matching rows");
        let lines: Vec<&str> = rendered.split('\n').collect();
        assert_eq!(lines[0], "name    count");
        assert_eq!(lines[1], "------  -----");
        assert_eq!(lines[2], "apples  3");
        assert_eq!(lines[3], "figs    12");
    }

    #[test]
    fn a_table_measures_coloured_cells_by_what_is_seen() {
        let rendered = table(vec!["name".to_string()], vec![vec![paint("ok".to_string(), TERM_Color::Green)]]).expect("matching rows");
        let lines: Vec<&str> = rendered.split('\n').collect();
        assert_eq!(lines[1], "----");
    }

    #[test]
    fn a_ragged_table_is_an_error() {
        let failure = table(vec!["a".to_string(), "b".to_string()], vec![vec!["only one".to_string()]]).unwrap_err();
        assert!(failure.contains("row 0 has 1 cells but there are 2 columns"));
    }

    #[test]
    fn a_table_needs_a_column() {
        assert!(table(vec![], vec![]).unwrap_err().contains("at least one column"));
    }

    #[test]
    fn a_hyperlink_carries_both_the_text_and_the_url() {
        let link = hyperlink("Nail".to_string(), "https://example.com".to_string());
        assert!(link.contains("Nail"));
        assert!(link.contains("https://example.com"));
        // A terminal that does not do hyperlinks shows the text and nothing else.
        assert_eq!(strip_styles(link), "Nail");
    }
}
