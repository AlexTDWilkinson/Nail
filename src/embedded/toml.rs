//! The TOML tokenizer, which also serves INI files.
//!
//! Both are the same shape: `[section]` headers, `key = value` lines, and `#`
//! comments. INI's `;` comments are the only difference worth carrying, and
//! both marks are accepted here rather than splitting the two apart.

use super::{flush, Piece};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    Code,
    Quote(char),
}

pub fn start() -> State {
    return State::Code;
}

/// Walks `body` from `state`, handing every run to `emit`. See
/// [`super::tokenize`] for how callers drive this across lines.
pub fn tokenize(body: &str, state: &mut State, mut emit: impl FnMut(&str, Piece)) {
    let chars: Vec<char> = body.chars().collect();
    let mut index = 0;
    let mut run = String::new();
    // Everything before the `=` on a line is the key; everything after is the
    // value, and the two color differently.
    let mut in_value = false;

    while index < chars.len() {
        let ch = chars[index];
        match *state {
            State::Quote(quote) => {
                run.push(ch);
                index += 1;
                if ch == quote {
                    flush(&mut run, Piece::Value, &mut emit);
                    *state = State::Code;
                }
            }
            State::Code => {
                if ch == '\n' {
                    emit("\n", Piece::Text);
                    index += 1;
                    in_value = false;
                } else if ch.is_whitespace() {
                    let mut spacing = String::new();
                    while let Some(&next) = chars.get(index) {
                        if next.is_whitespace() && next != '\n' {
                            spacing.push(next);
                            index += 1;
                        } else {
                            break;
                        }
                    }
                    emit(&spacing, Piece::Text);
                } else if ch == '#' || ch == ';' {
                    let end = chars[index..].iter().position(|next| *next == '\n').map(|at| index + at).unwrap_or(chars.len());
                    let comment: String = chars[index..end].iter().collect();
                    emit(&comment, Piece::Comment);
                    index = end;
                } else if ch == '=' {
                    emit("=", Piece::Operator);
                    index += 1;
                    in_value = true;
                } else if ch == '"' || ch == '\'' {
                    run.push(ch);
                    index += 1;
                    *state = State::Quote(ch);
                } else if matches!(ch, '[' | ']' | '{' | '}' | ',') {
                    emit(&ch.to_string(), Piece::Bracket);
                    index += 1;
                } else {
                    let mut word = String::new();
                    while let Some(&next) = chars.get(index) {
                        if next.is_whitespace() || matches!(next, '[' | ']' | '{' | '}' | ',' | '=' | '#' | ';' | '"' | '\'') {
                            break;
                        }
                        word.push(next);
                        index += 1;
                    }
                    emit(&word, word_piece(&word, in_value));
                }
            }
        }
    }

    let leftover = match *state {
        State::Quote(_) => Piece::Value,
        State::Code => Piece::Text,
    };
    flush(&mut run, leftover, &mut emit);
}

/// A word is a key until the `=`, and a value after it. A section name sits
/// between brackets with no `=` on the line, which leaves it a key - and a
/// section is close enough to one to share its color.
fn word_piece(word: &str, in_value: bool) -> Piece {
    if !in_value {
        return Piece::Attribute;
    }
    if matches!(word, "true" | "false") {
        return Piece::Keyword;
    }
    if is_number(word) {
        return Piece::Number;
    }
    return Piece::Text;
}

/// Whether a value is a number: an integer, a float, or one of TOML's dates,
/// which start with a year.
fn is_number(word: &str) -> bool {
    let digits = word.trim_start_matches(['-', '+']);
    return digits.chars().next().map(|first| first.is_ascii_digit()).unwrap_or(false);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pieces(body: &str) -> Vec<(String, Piece)> {
        let mut state = start();
        let mut out = Vec::new();
        tokenize(body, &mut state, |text, piece| out.push((text.to_string(), piece)));
        return out;
    }

    #[test]
    fn a_key_and_its_value_come_back_apart() {
        let out = pieces("name = \"nail\"\nport = 8080\ndebug = true");
        assert!(out.contains(&("name".to_string(), Piece::Attribute)), "got {:?}", out);
        assert!(out.contains(&("\"nail\"".to_string(), Piece::Value)), "got {:?}", out);
        assert!(out.contains(&("8080".to_string(), Piece::Number)), "got {:?}", out);
        assert!(out.contains(&("true".to_string(), Piece::Keyword)), "got {:?}", out);
        assert!(out.contains(&("=".to_string(), Piece::Operator)));
    }

    #[test]
    fn a_section_header_is_bracketed() {
        let out = pieces("[package]\nname = \"nail\"");
        assert!(out.contains(&("[".to_string(), Piece::Bracket)), "got {:?}", out);
        assert!(out.contains(&("package".to_string(), Piece::Attribute)), "got {:?}", out);
    }

    #[test]
    fn a_date_reads_as_a_number_and_a_comment_ends_at_its_line() {
        let out = pieces("released = 1979-05-27 # a while ago\nname = \"x\"");
        assert!(out.contains(&("1979-05-27".to_string(), Piece::Number)), "got {:?}", out);
        assert!(out.contains(&("# a while ago".to_string(), Piece::Comment)), "got {:?}", out);
        assert!(out.contains(&("name".to_string(), Piece::Attribute)), "the next line is a key again, got {:?}", out);
    }

    #[test]
    fn an_ini_semicolon_comment_is_a_comment_too() {
        let out = pieces("; legacy\nkey = 1");
        assert!(out.contains(&("; legacy".to_string(), Piece::Comment)), "got {:?}", out);
    }

    #[test]
    fn every_character_survives_the_round_trip() {
        let source = "# config\n[package]\nname = \"nail\"\nfeatures = [\"a\", \"b\"]\n";
        let rebuilt: String = pieces(source).into_iter().map(|(text, _)| text).collect();
        assert_eq!(rebuilt, source);
    }
}
