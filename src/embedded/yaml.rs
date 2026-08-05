//! The YAML tokenizer.
//!
//! YAML is read a line at a time: what comes before the first `: ` on a line is
//! a key, what comes after it is a value, and the same word means a different
//! thing in each place. So the scanner works line by line, which also means the
//! only thing it has to carry across a line break is an unterminated quote.

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
    // Before the key of the current line, so `- name: nail` still finds its key
    // after the dash.
    let mut before_key = true;

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
                    before_key = true;
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
                } else if ch == '#' {
                    let end = line_end(&chars, index);
                    let comment: String = chars[index..end].iter().collect();
                    emit(&comment, Piece::Comment);
                    index = end;
                } else if before_key && (starts_with(&chars, index, "---") || starts_with(&chars, index, "...")) {
                    let marker: String = chars[index..index + 3].iter().collect();
                    emit(&marker, Piece::Bracket);
                    index += 3;
                } else if before_key && ch == '-' && chars.get(index + 1).map(|next| next.is_whitespace()).unwrap_or(true) {
                    // A sequence dash leaves the line still looking for a key:
                    // `- name: nail` has one.
                    emit("-", Piece::Bracket);
                    index += 1;
                } else if before_key {
                    match key_end(&chars, index) {
                        Some(end) => {
                            let key: String = chars[index..end].iter().collect();
                            emit(&key, Piece::Attribute);
                            emit(":", Piece::Operator);
                            index = end + 1;
                        }
                        // No `: ` on this line, so what is here is a value - an
                        // item in a sequence, or the continuation of one.
                        None => {}
                    }
                    before_key = false;
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
                        if next.is_whitespace() || matches!(next, '[' | ']' | '{' | '}' | ',' | '#') {
                            break;
                        }
                        word.push(next);
                        index += 1;
                    }
                    emit(&word, value_piece(&word));
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

/// What a bare value is worth coloring as.
fn value_piece(word: &str) -> Piece {
    if matches!(word, "true" | "false" | "null" | "~" | "yes" | "no" | "on" | "off" | "True" | "False" | "Null") {
        return Piece::Keyword;
    }
    if is_number(word) {
        return Piece::Number;
    }
    return Piece::Text;
}

fn is_number(word: &str) -> bool {
    let digits = word.trim_start_matches(['-', '+', '.']);
    return digits.chars().next().map(|first| first.is_ascii_digit()).unwrap_or(false);
}

/// Where this line's key ends, if it has one: the first colon followed by
/// whitespace or the end of the line. A colon inside a value - a URL, a time -
/// has something else after it, so it does not end a key.
fn key_end(chars: &[char], from: usize) -> Option<usize> {
    let mut index = from;
    while index < chars.len() && chars[index] != '\n' {
        if chars[index] == '#' {
            return None;
        }
        if chars[index] == ':' && chars.get(index + 1).map(|next| next.is_whitespace()).unwrap_or(true) {
            return Some(index);
        }
        index += 1;
    }
    return None;
}

fn line_end(chars: &[char], from: usize) -> usize {
    return chars[from..].iter().position(|next| *next == '\n').map(|at| from + at).unwrap_or(chars.len());
}

fn starts_with(chars: &[char], index: usize, marker: &str) -> bool {
    return chars[index..].iter().zip(marker.chars()).filter(|(here, there)| *here == there).count() == marker.chars().count();
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
    fn a_mapping_comes_back_as_key_and_value() {
        let out = pieces("name: nail\nport: 8080\ndebug: true");
        assert!(out.contains(&("name".to_string(), Piece::Attribute)), "got {:?}", out);
        assert!(out.contains(&("nail".to_string(), Piece::Text)), "got {:?}", out);
        assert!(out.contains(&("8080".to_string(), Piece::Number)), "got {:?}", out);
        assert!(out.contains(&("true".to_string(), Piece::Keyword)), "got {:?}", out);
    }

    #[test]
    fn a_sequence_item_is_a_value_and_a_dash_still_allows_a_key() {
        let out = pieces("hosts:\n  - one\n  - name: two");
        assert!(out.contains(&("one".to_string(), Piece::Text)), "a bare item is a value, got {:?}", out);
        assert!(out.contains(&("name".to_string(), Piece::Attribute)), "a dash still leaves room for a key, got {:?}", out);
        assert!(out.contains(&("-".to_string(), Piece::Bracket)), "got {:?}", out);
    }

    #[test]
    fn a_colon_inside_a_value_does_not_start_one() {
        let out = pieces("url: https://nail.dev/docs");
        assert!(out.contains(&("url".to_string(), Piece::Attribute)), "got {:?}", out);
        assert!(out.contains(&("https://nail.dev/docs".to_string(), Piece::Text)), "the URL is one value, got {:?}", out);
    }

    #[test]
    fn a_comment_runs_to_the_end_of_its_line() {
        let out = pieces("port: 80 # the default\nname: nail");
        assert!(out.contains(&("# the default".to_string(), Piece::Comment)), "got {:?}", out);
        assert!(out.contains(&("name".to_string(), Piece::Attribute)), "the next line is a mapping again, got {:?}", out);
    }

    #[test]
    fn a_quoted_value_is_one_piece_and_carries_across_a_line_break() {
        let out = pieces("greeting: \"hello there\"");
        assert!(out.contains(&("\"hello there\"".to_string(), Piece::Value)), "got {:?}", out);

        let mut state = start();
        tokenize("greeting: \"unfinished", &mut state, |_, _| {});
        assert_eq!(state, State::Quote('"'));
    }

    #[test]
    fn every_character_survives_the_round_trip() {
        let source = "# config\nname: nail\nhosts:\n  - one\n  - two   # trailing\nflags: [a, b]\n";
        let rebuilt: String = pieces(source).into_iter().map(|(text, _)| text).collect();
        assert_eq!(rebuilt, source);
    }
}
