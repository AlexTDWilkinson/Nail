//! The tokenizer for angle-bracket languages: html, svg, xml.

use super::{flush, Piece};

/// How far into markup a line ended. A tag whose attributes run onto the next
/// line has to stay open across the break.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    Text,
    InTag,
    InQuote(char),
    InComment,
}

pub fn start() -> State {
    return State::Text;
}

/// Walks `body` from `state`, handing every run to `emit`. See
/// [`super::tokenize`] for how callers drive this across lines.
pub fn tokenize(body: &str, state: &mut State, mut emit: impl FnMut(&str, Piece)) {
    let chars: Vec<char> = body.chars().collect();
    let mut index = 0;
    let mut run = String::new();
    let mut run_piece = match state {
        State::Text => Piece::Text,
        State::InTag => Piece::Attribute,
        State::InQuote(_) => Piece::Value,
        State::InComment => Piece::Comment,
    };

    while index < chars.len() {
        let ch = chars[index];
        match *state {
            State::Text => {
                if ch == '<' {
                    flush(&mut run, run_piece, &mut emit);
                    if chars[index..].starts_with(&['<', '!', '-', '-']) {
                        run.push_str("<!--");
                        index += 4;
                        *state = State::InComment;
                        run_piece = Piece::Comment;
                        continue;
                    }
                    let mut bracket = String::from("<");
                    index += 1;
                    if chars.get(index) == Some(&'/') {
                        bracket.push('/');
                        index += 1;
                    }
                    emit(&bracket, Piece::Bracket);

                    let mut name = String::new();
                    while let Some(&next) = chars.get(index) {
                        if next.is_ascii_alphanumeric() || next == '-' || next == '!' || next == '_' || next == ':' {
                            name.push(next);
                            index += 1;
                        } else {
                            break;
                        }
                    }
                    if !name.is_empty() {
                        emit(&name, Piece::Element);
                    }
                    *state = State::InTag;
                    run_piece = Piece::Attribute;
                } else {
                    run.push(ch);
                    index += 1;
                }
            }
            State::InTag => {
                if ch == '>' {
                    flush(&mut run, run_piece, &mut emit);
                    emit(">", Piece::Bracket);
                    *state = State::Text;
                    run_piece = Piece::Text;
                    index += 1;
                } else if ch == '"' || ch == '\'' {
                    flush(&mut run, run_piece, &mut emit);
                    run.push(ch);
                    *state = State::InQuote(ch);
                    run_piece = Piece::Value;
                    index += 1;
                } else if ch == '=' || ch == '/' {
                    flush(&mut run, run_piece, &mut emit);
                    emit(&ch.to_string(), Piece::Operator);
                    run_piece = Piece::Attribute;
                    index += 1;
                } else if ch.is_whitespace() {
                    // Whitespace separates attributes; keeping it out of the
                    // attribute run stops a leading space from being colored
                    // as part of the name.
                    flush(&mut run, run_piece, &mut emit);
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
                    run_piece = Piece::Attribute;
                } else {
                    run.push(ch);
                    index += 1;
                }
            }
            State::InQuote(quote) => {
                run.push(ch);
                index += 1;
                if ch == quote {
                    flush(&mut run, Piece::Value, &mut emit);
                    *state = State::InTag;
                    run_piece = Piece::Attribute;
                }
            }
            State::InComment => {
                run.push(ch);
                index += 1;
                if run.ends_with("-->") {
                    flush(&mut run, Piece::Comment, &mut emit);
                    *state = State::Text;
                    run_piece = Piece::Text;
                }
            }
        }
    }

    flush(&mut run, run_piece, &mut emit);
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
    fn an_element_and_its_attributes_come_back_apart() {
        let out = pieces(r#"<section class="hero">Nail</section>"#);
        assert!(out.contains(&("section".to_string(), Piece::Element)));
        assert!(out.contains(&("class".to_string(), Piece::Attribute)));
        assert!(out.contains(&("\"hero\"".to_string(), Piece::Value)));
        assert!(out.contains(&("Nail".to_string(), Piece::Text)));
        assert!(out.contains(&("</".to_string(), Piece::Bracket)));
    }

    #[test]
    fn every_character_survives_the_round_trip() {
        // Highlighting may never drop or invent text.
        let source = r##"  <a href="#x" data-n='1'>link</a> tail <br/>"##;
        let rebuilt: String = pieces(source).into_iter().map(|(text, _)| text).collect();
        assert_eq!(rebuilt, source);
    }

    #[test]
    fn a_tag_left_open_carries_its_state_to_the_next_line() {
        let mut state = start();
        advance("<svg viewBox=\"0 0 20 20\"", &mut state);
        assert_eq!(state, State::InTag);

        let mut out = Vec::new();
        tokenize("    stroke=\"currentColor\">", &mut state, |text, piece| out.push((text.to_string(), piece)));
        assert!(out.contains(&("stroke".to_string(), Piece::Attribute)), "got {:?}", out);
        assert_eq!(state, State::Text);
    }

    #[test]
    fn a_quote_left_open_carries_its_state_too() {
        let mut state = start();
        advance("<p title=\"unfinished", &mut state);
        assert_eq!(state, State::InQuote('"'));
    }

    #[test]
    fn a_comment_is_one_piece_and_survives_a_line_break() {
        let out = pieces("<p><!-- aside --></p>");
        assert!(out.contains(&("<!-- aside -->".to_string(), Piece::Comment)), "got {:?}", out);

        let mut state = start();
        advance("<!-- opened here", &mut state);
        assert_eq!(state, State::InComment);
        advance(" and closed here -->", &mut state);
        assert_eq!(state, State::Text);
    }
}
