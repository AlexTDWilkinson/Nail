//! The Markdown tokenizer.
//!
//! Markdown is marks around prose, so this scans for the marks - headings,
//! bullets, quotes, emphasis, links, code - and leaves everything between them
//! as text. Code spans are the one thing that changes how the text after them
//! reads, so an unclosed one is carried to the next line.

use super::{flush, Piece};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    Text,
    InCode,
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
    let mut line_start = true;

    while index < chars.len() {
        let ch = chars[index];

        // A run of backticks opens a code span and the matching run closes it.
        // Inside a Nail string each one is written `\``, so the backslash in
        // front of it belongs to the fence, not to the text.
        if ch == '`' || (ch == '\\' && chars.get(index + 1) == Some(&'`')) {
            let mut fence = String::new();
            while let Some(&next) = chars.get(index) {
                if next == '`' || (next == '\\' && chars.get(index + 1) == Some(&'`')) {
                    fence.push(next);
                    index += 1;
                } else {
                    break;
                }
            }
            match *state {
                State::Text => {
                    flush(&mut run, Piece::Text, &mut emit);
                    emit(&fence, Piece::Bracket);
                    *state = State::InCode;
                }
                State::InCode => {
                    flush(&mut run, Piece::Value, &mut emit);
                    emit(&fence, Piece::Bracket);
                    *state = State::Text;
                }
            }
            line_start = false;
            continue;
        }

        if *state == State::InCode {
            run.push(ch);
            index += 1;
            continue;
        }

        if ch == '\n' {
            flush(&mut run, Piece::Text, &mut emit);
            emit("\n", Piece::Text);
            index += 1;
            line_start = true;
        } else if line_start && ch.is_whitespace() {
            run.push(ch);
            index += 1;
        } else if line_start && ch == '#' {
            // A heading: the hashes mark it, and the rest of the line is what
            // the heading says.
            flush(&mut run, Piece::Text, &mut emit);
            let mut hashes = String::new();
            while chars.get(index) == Some(&'#') {
                hashes.push('#');
                index += 1;
            }
            emit(&hashes, Piece::Keyword);
            let end = line_end(&chars, index);
            let heading: String = chars[index..end].iter().collect();
            if !heading.is_empty() {
                emit(&heading, Piece::Element);
                index = end;
            }
            line_start = false;
        } else if line_start && ch == '>' {
            // A quote: the marks in front, stacked when nested, and then the
            // quoted line. Quoted lines read as comments, the way mail readers
            // and diff viewers have always painted somebody else's words, and
            // it keeps them apart from the prose around them, which is already
            // in the string color.
            flush(&mut run, Piece::Text, &mut emit);
            let mut marks = String::new();
            while let Some(&next) = chars.get(index) {
                if next == '>' || (next == ' ' && chars.get(index + 1) == Some(&'>')) {
                    marks.push(next);
                    index += 1;
                } else {
                    break;
                }
            }
            emit(&marks, Piece::Bracket);
            let end = line_end(&chars, index);
            if end > index {
                let quoted: String = chars[index..end].iter().collect();
                emit(&quoted, Piece::Comment);
                index = end;
            }
            line_start = false;
        } else if line_start && matches!(ch, '-' | '*' | '_') && rule_end(&chars, index).is_some() {
            // A horizontal rule, which is also what the fences around
            // frontmatter look like. Checked before bullets, because `* * *`
            // reads as a rule and not as a list.
            flush(&mut run, Piece::Text, &mut emit);
            let end = rule_end(&chars, index).unwrap_or(index);
            let rule: String = chars[index..end].iter().collect();
            emit(&rule, Piece::Operator);
            index = end;
            line_start = false;
        } else if ch == '|' {
            // A table pipe, wherever in the row it falls. A cell of dashes and
            // colons after it is the delimiter row under a header, so it reads
            // as marks rather than as prose.
            flush(&mut run, Piece::Text, &mut emit);
            emit("|", Piece::Bracket);
            index += 1;
            let mut cell_end = index;
            while let Some(&next) = chars.get(cell_end) {
                if next == '|' || next == '\n' {
                    break;
                }
                cell_end += 1;
            }
            let cell: String = chars[index..cell_end].iter().collect();
            if cell.contains('-') && cell.chars().all(|inside| matches!(inside, '-' | ':' | ' ')) {
                emit(&cell, Piece::Operator);
                index = cell_end;
            }
            line_start = false;
        } else if line_start && matches!(ch, '-' | '*' | '+') && chars.get(index + 1).map(|next| next.is_whitespace()).unwrap_or(true) {
            flush(&mut run, Piece::Text, &mut emit);
            emit(&ch.to_string(), Piece::Bracket);
            index += 1;
            line_start = false;
        } else if line_start && ch.is_ascii_digit() && ordered_marker(&chars, index).is_some() {
            flush(&mut run, Piece::Text, &mut emit);
            let end = ordered_marker(&chars, index).unwrap_or(index);
            let marker: String = chars[index..end].iter().collect();
            emit(&marker, Piece::Bracket);
            index = end;
            line_start = false;
        } else if ch == '*' || ch == '_' || ch == '~' {
            // Emphasis marks, however many are stacked up.
            flush(&mut run, Piece::Text, &mut emit);
            let mut marks = String::new();
            while let Some(&next) = chars.get(index) {
                if next == ch {
                    marks.push(next);
                    index += 1;
                } else {
                    break;
                }
            }
            emit(&marks, Piece::Operator);
            line_start = false;
        } else if ch == '[' || ch == ']' || ch == '(' || ch == ')' || (ch == '!' && chars.get(index + 1) == Some(&'[')) {
            // The pieces of a link. What sits inside the parentheses is the
            // destination, which reads as a value.
            flush(&mut run, Piece::Text, &mut emit);
            emit(&ch.to_string(), Piece::Bracket);
            index += 1;
            if ch == '(' {
                let mut target = String::new();
                while let Some(&next) = chars.get(index) {
                    if next == ')' || next == '\n' {
                        break;
                    }
                    target.push(next);
                    index += 1;
                }
                if !target.is_empty() {
                    emit(&target, Piece::Value);
                }
            }
            line_start = false;
        } else {
            run.push(ch);
            index += 1;
            line_start = false;
        }
    }

    let leftover = match *state {
        State::InCode => Piece::Value,
        State::Text => Piece::Text,
    };
    flush(&mut run, leftover, &mut emit);
}

/// Where an ordered-list marker ends - the `1.` or `2)` at the start of a line
/// - or `None` if the digits here are just a number.
fn ordered_marker(chars: &[char], from: usize) -> Option<usize> {
    let mut index = from;
    while chars.get(index).map(|next| next.is_ascii_digit()).unwrap_or(false) {
        index += 1;
    }
    if index == from {
        return None;
    }
    match chars.get(index) {
        Some('.') | Some(')') if chars.get(index + 1).map(|next| next.is_whitespace()).unwrap_or(true) => Some(index + 1),
        _ => None,
    }
}

/// Where a horizontal rule ends - a line of three or more `-`, `*` or `_`,
/// spaces allowed between them - or `None` if this line is something else.
fn rule_end(chars: &[char], from: usize) -> Option<usize> {
    let mark = *chars.get(from)?;
    let mut count = 0;
    let mut index = from;
    while let Some(&next) = chars.get(index) {
        if next == mark {
            count += 1;
            index += 1;
        } else if next == ' ' {
            index += 1;
        } else if next == '\n' {
            break;
        } else {
            return None;
        }
    }
    return if count >= 3 { Some(index) } else { None };
}

fn line_end(chars: &[char], from: usize) -> usize {
    return chars[from..].iter().position(|next| *next == '\n').map(|at| from + at).unwrap_or(chars.len());
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
    fn a_heading_is_marked_and_named() {
        let out = pieces("## Getting started\ntext");
        assert!(out.contains(&("##".to_string(), Piece::Keyword)), "got {:?}", out);
        assert!(out.contains(&(" Getting started".to_string(), Piece::Element)), "got {:?}", out);
    }

    #[test]
    fn bullets_quotes_and_numbers_start_their_lines() {
        let out = pieces("- one\n2. two\n> quoted");
        assert!(out.contains(&("-".to_string(), Piece::Bracket)), "got {:?}", out);
        assert!(out.contains(&("2.".to_string(), Piece::Bracket)), "got {:?}", out);
        assert!(out.contains(&(">".to_string(), Piece::Bracket)), "got {:?}", out);
    }

    #[test]
    fn a_quoted_line_is_marked_and_reads_as_a_comment() {
        let out = pieces("> Would you keep playing guitar if nobody could ever hear it?\n");
        assert!(out.contains(&(">".to_string(), Piece::Bracket)), "got {:?}", out);
        assert!(
            out.contains(&(" Would you keep playing guitar if nobody could ever hear it?".to_string(), Piece::Comment)),
            "got {:?}",
            out
        );

        let nested = pieces("> > deeper");
        assert!(nested.contains(&("> >".to_string(), Piece::Bracket)), "got {:?}", nested);
        assert!(nested.contains(&(" deeper".to_string(), Piece::Comment)), "got {:?}", nested);
    }

    #[test]
    fn a_horizontal_rule_is_one_operator_and_a_bullet_is_still_a_bullet() {
        let out = pieces("---\n");
        assert!(out.contains(&("---".to_string(), Piece::Operator)), "got {:?}", out);

        let spaced = pieces("* * *\n");
        assert!(spaced.contains(&("* * *".to_string(), Piece::Operator)), "got {:?}", spaced);

        let bullet = pieces("- one\n");
        assert!(bullet.contains(&("-".to_string(), Piece::Bracket)), "got {:?}", bullet);
        assert!(bullet.contains(&(" one".to_string(), Piece::Text)), "got {:?}", bullet);
    }

    #[test]
    fn every_pipe_in_a_table_row_is_marked() {
        let out = pieces("| name | age |\n");
        let pipes = out.iter().filter(|(text, piece)| text == "|" && *piece == Piece::Bracket).count();
        assert_eq!(pipes, 3, "got {:?}", out);
    }

    #[test]
    fn a_delimiter_row_reads_as_marks_and_a_cell_of_prose_does_not() {
        let out = pieces("| :--- | ---: |\n");
        assert!(out.contains(&(" :--- ".to_string(), Piece::Operator)), "got {:?}", out);
        assert!(out.contains(&(" ---: ".to_string(), Piece::Operator)), "got {:?}", out);

        let row = pieces("| 19.3 million | 8.1% |\n");
        assert!(row.iter().any(|(text, piece)| text.contains("million") && *piece == Piece::Text), "got {:?}", row);
    }

    #[test]
    fn emphasis_marks_are_operators_and_a_link_target_is_a_value() {
        let out = pieces("see **this** and [the docs](https://nail.dev)");
        assert!(out.contains(&("**".to_string(), Piece::Operator)), "got {:?}", out);
        assert!(out.contains(&("https://nail.dev".to_string(), Piece::Value)), "got {:?}", out);
    }

    #[test]
    fn a_code_span_written_the_nail_way_holds_until_it_closes() {
        let out = pieces(r#"run \`nailc\` now"#);
        assert!(out.iter().any(|(text, piece)| text == "nailc" && *piece == Piece::Value), "got {:?}", out);

        let mut state = start();
        tokenize(r#"a fence: \`\`\`"#, &mut state, |_, _| {});
        assert_eq!(state, State::InCode, "an unclosed fence carries to the next line");
    }

    #[test]
    fn every_character_survives_the_round_trip() {
        let source = "# Title\n\nSome *emphasis* and a [link](https://nail.dev).\n\n> a quote\n\n---\n\n| a | b |\n\n- one\n- two\n";
        let rebuilt: String = pieces(source).into_iter().map(|(text, _)| text).collect();
        assert_eq!(rebuilt, source);
    }
}
