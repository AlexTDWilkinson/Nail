//! The CSS tokenizer.
//!
//! CSS is three contexts wearing one syntax: the selector before a `{`, the
//! property names inside it, and the values after each `:`. The same word means
//! a different thing in each - `color` is a property in one and a keyword-ish
//! value in another - so the scanner tracks which one it is in, and how many
//! braces deep it is, since `@media` wraps whole rules in another block.

use super::{flush, Piece};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Selector,
    Block,
    Value,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Inside {
    Code,
    Quote(char),
    Comment,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct State {
    mode: Mode,
    inside: Inside,
    depth: u16,
}

pub fn start() -> State {
    return State { mode: Mode::Selector, inside: Inside::Code, depth: 0 };
}

/// Walks `body` from `state`, handing every run to `emit`. See
/// [`super::tokenize`] for how callers drive this across lines.
pub fn tokenize(body: &str, state: &mut State, mut emit: impl FnMut(&str, Piece)) {
    let chars: Vec<char> = body.chars().collect();
    let mut index = 0;
    let mut run = String::new();

    while index < chars.len() {
        let ch = chars[index];
        match state.inside {
            Inside::Comment => {
                run.push(ch);
                index += 1;
                if run.ends_with("*/") {
                    flush(&mut run, Piece::Comment, &mut emit);
                    state.inside = Inside::Code;
                }
            }
            Inside::Quote(quote) => {
                run.push(ch);
                index += 1;
                if ch == quote {
                    flush(&mut run, Piece::Value, &mut emit);
                    state.inside = Inside::Code;
                }
            }
            Inside::Code => {
                let word = word_piece(state.mode, &run);
                if ch == '/' && chars.get(index + 1) == Some(&'*') {
                    flush(&mut run, word, &mut emit);
                    run.push_str("/*");
                    index += 2;
                    state.inside = Inside::Comment;
                } else if ch == '"' || ch == '\'' {
                    flush(&mut run, word, &mut emit);
                    run.push(ch);
                    index += 1;
                    state.inside = Inside::Quote(ch);
                } else if ch.is_whitespace() {
                    // Whitespace ends a word without belonging to it, so an
                    // indented property is not colored from the margin in.
                    flush(&mut run, word, &mut emit);
                    let mut spacing = String::new();
                    while let Some(&next) = chars.get(index) {
                        if next.is_whitespace() {
                            spacing.push(next);
                            index += 1;
                        } else {
                            break;
                        }
                    }
                    emit(&spacing, Piece::Text);
                } else if ch == '{' {
                    flush(&mut run, word, &mut emit);
                    emit("{", Piece::Bracket);
                    index += 1;
                    state.depth += 1;
                    state.mode = Mode::Block;
                } else if ch == '}' {
                    flush(&mut run, word, &mut emit);
                    emit("}", Piece::Bracket);
                    index += 1;
                    state.depth = state.depth.saturating_sub(1);
                    state.mode = if state.depth == 0 { Mode::Selector } else { Mode::Block };
                } else if ch == ';' {
                    flush(&mut run, word, &mut emit);
                    emit(";", Piece::Bracket);
                    index += 1;
                    state.mode = if state.depth == 0 { Mode::Selector } else { Mode::Block };
                } else if ch == ':' && state.mode == Mode::Block {
                    // Only inside a block does a colon separate a declaration;
                    // in a selector it introduces a pseudo-class, `a:hover`.
                    flush(&mut run, word, &mut emit);
                    emit(":", Piece::Operator);
                    index += 1;
                    state.mode = Mode::Value;
                } else if ch == ',' {
                    flush(&mut run, word, &mut emit);
                    emit(",", Piece::Bracket);
                    index += 1;
                } else if ch == '(' {
                    // In a value, the word against the parenthesis named a
                    // function - `rgb(`, `var(`, `translateX(`. In a selector
                    // it is part of the selector itself, `a:not(.x)`.
                    flush(&mut run, if state.mode == Mode::Value { Piece::Function } else { word }, &mut emit);
                    emit("(", Piece::Bracket);
                    index += 1;
                } else if ch == ')' {
                    flush(&mut run, word, &mut emit);
                    emit(")", Piece::Bracket);
                    index += 1;
                } else {
                    run.push(ch);
                    index += 1;
                }
            }
        }
    }

    let leftover = match state.inside {
        Inside::Comment => Piece::Comment,
        Inside::Quote(_) => Piece::Value,
        Inside::Code => word_piece(state.mode, &run),
    };
    flush(&mut run, leftover, &mut emit);
}

/// What a bare word means, given where in a rule it was found.
fn word_piece(mode: Mode, word: &str) -> Piece {
    if word.starts_with('@') {
        return Piece::Keyword;
    }
    return match mode {
        Mode::Selector => Piece::Element,
        Mode::Block => Piece::Attribute,
        // `!important` is the one keyword that shows up in a value.
        Mode::Value if word.starts_with('!') => Piece::Keyword,
        Mode::Value if is_number(word) => Piece::Number,
        Mode::Value => Piece::Text,
    };
}

/// Whether a value word is a number: a length like `1.5rem`, a percentage, or
/// a hex color, all of which read better in the number color than as text.
fn is_number(word: &str) -> bool {
    let digits = word.trim_start_matches(['-', '+', '.']);
    return word.starts_with('#') || digits.chars().next().map(|first| first.is_ascii_digit()).unwrap_or(false);
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

    fn advance(body: &str, state: &mut State) {
        tokenize(body, state, |_, _| {});
    }

    #[test]
    fn a_rule_comes_back_as_selector_property_and_value() {
        let out = pieces(".hero > h1 { font-size: 1.5rem; }");
        assert!(out.contains(&(".hero".to_string(), Piece::Element)), "got {:?}", out);
        assert!(out.contains(&("font-size".to_string(), Piece::Attribute)), "got {:?}", out);
        assert!(out.contains(&("1.5rem".to_string(), Piece::Number)), "got {:?}", out);
        assert!(out.contains(&("{".to_string(), Piece::Bracket)));
    }

    #[test]
    fn a_pseudo_class_stays_part_of_the_selector() {
        let out = pieces("a:hover { color: red; }");
        assert!(out.contains(&("a:hover".to_string(), Piece::Element)), "got {:?}", out);
        assert!(out.contains(&("red".to_string(), Piece::Text)), "got {:?}", out);
    }

    #[test]
    fn an_at_rule_is_a_keyword_and_its_inner_rules_close_back_to_the_top() {
        let mut state = start();
        let mut out = Vec::new();
        tokenize("@media (min-width: 40rem) { .hero { color: #fff; } }", &mut state, |text, piece| out.push((text.to_string(), piece)));
        assert!(out.contains(&("@media".to_string(), Piece::Keyword)), "got {:?}", out);
        assert!(out.contains(&("#fff".to_string(), Piece::Number)), "got {:?}", out);
        assert_eq!(state, start(), "both blocks closed, so the scanner is back where it started");
    }

    #[test]
    fn a_function_in_a_value_is_named() {
        let out = pieces("p { color: rgb(1, 2, 3); }");
        assert!(out.contains(&("rgb".to_string(), Piece::Function)), "got {:?}", out);
    }

    #[test]
    fn important_is_a_keyword() {
        let out = pieces("p { color: red !important; }");
        assert!(out.contains(&("!important".to_string(), Piece::Keyword)), "got {:?}", out);
    }

    #[test]
    fn a_block_and_a_comment_carry_their_state_to_the_next_line() {
        let mut state = start();
        advance(".hero {", &mut state);
        let mut out = Vec::new();
        tokenize("    color: red;", &mut state, |text, piece| out.push((text.to_string(), piece)));
        assert!(out.contains(&("color".to_string(), Piece::Attribute)), "got {:?}", out);

        let mut state = start();
        advance("/* opened here", &mut state);
        assert_eq!(state.inside, Inside::Comment);
        let mut out = Vec::new();
        tokenize("still a comment */ .hero {", &mut state, |text, piece| out.push((text.to_string(), piece)));
        assert!(out.contains(&("still a comment */".to_string(), Piece::Comment)), "got {:?}", out);
        assert!(out.contains(&(".hero".to_string(), Piece::Element)), "got {:?}", out);
    }
}
