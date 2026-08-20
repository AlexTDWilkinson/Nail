//! Bending real programs out of shape.
//!
//! Byte-level noise is close to useless against this compiler: four hundred
//! randomly corrupted corpus files produced not one crash, because a stream
//! of junk bytes stops at the lexer. What does reach the deeper stages is
//! damage that still looks like Nail, so every mutation here works in units
//! the language has: lines, names, types, numbers and blocks.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use super::corpus::Corpus;

/// Types written in Nail, for the mutation that swaps a declaration's type
/// out from under its value. Mismatches are the checker's whole job, so the
/// interesting part is the paths the checker takes to report them.
const TYPES: &[&str] = &["i", "f", "s", "b", "v", "e", "a:i", "a:s", "a:f", "a:b", "a:a:i", "h<s,i>", "h<s,s>", "i!e", "s!e", "f!e"];

/// Numbers worth writing where a number already was. The two at the ends are
/// the edges of what an i can hold, and the ones past them are what a Rust
/// literal cannot.
const NUMBERS: &[&str] = &["0", "1", "-1", "9223372036854775807", "-9223372036854775808", "9223372036854775808", "99999999999999999999999", "0.0", "-0.0", "1.7976931348623157", "18446744073709551616"];

/// Pieces of syntax to drop into a line, chosen because each one changes the
/// shape of the program rather than only its text.
const FRAGMENTS: &[&str] = &["{", "}", "(", ")", "[", "]", ";", ":", ",", "->", "!", "r ", "y ", "f ", "if {", "else -> {", "danger(", "safe(", "expect(", "p", "/p", "c", "/c", "`", "e(`x`)", "..", "..=", "&&", "||", "=="];

/// Build the case for one seed: a corpus file with a handful of edits.
pub fn case(seed: u64, corpus: &Corpus) -> (String, String) {
    let mut rng = StdRng::seed_from_u64(seed);
    let entry = corpus.pick(rng.gen());
    let mut lines = entry.lines.clone();

    let edits = rng.gen_range(1..=4);
    for _ in 0..edits {
        apply(&mut rng, &mut lines, corpus);
        if lines.is_empty() {
            break;
        }
    }

    let mut source = lines.join("\n");
    source.push('\n');
    (source, format!("mutate {}", entry.path.display()))
}

fn apply(rng: &mut StdRng, lines: &mut Vec<String>, corpus: &Corpus) {
    if lines.is_empty() {
        return;
    }
    match rng.gen_range(0..11) {
        // Delete a line. Half of every syntax error in the world.
        0 => {
            let index = rng.gen_range(0..lines.len());
            lines.remove(index);
        }
        // Repeat a line, which redeclares names and duplicates declarations.
        1 => {
            let index = rng.gen_range(0..lines.len());
            let line = lines[index].clone();
            lines.insert(index, line);
        }
        // Move a line somewhere it does not belong, which is how uses end up
        // before their declarations.
        2 => {
            let from = rng.gen_range(0..lines.len());
            let to = rng.gen_range(0..lines.len());
            let line = lines.remove(from);
            lines.insert(to.min(lines.len()), line);
        }
        // Graft a run of lines out of a different program, so two unrelated
        // scopes have to be reconciled by whatever the checker does next.
        3 => {
            let donor = corpus.pick(rng.gen());
            if donor.lines.is_empty() {
                return;
            }
            let start = rng.gen_range(0..donor.lines.len());
            let length = rng.gen_range(1..=8).min(donor.lines.len() - start);
            let at = rng.gen_range(0..=lines.len());
            for (offset, line) in donor.lines[start..start + length].iter().enumerate() {
                lines.insert((at + offset).min(lines.len()), line.clone());
            }
        }
        // Drop a fragment of syntax into a line, mid-token as often as not.
        4 => {
            let index = rng.gen_range(0..lines.len());
            let fragment = FRAGMENTS[rng.gen_range(0..FRAGMENTS.len())];
            let line = &lines[index];
            let at = char_boundary(line, rng.gen_range(0..=line.chars().count()));
            let mut mutated = line.clone();
            mutated.insert_str(at, fragment);
            lines[index] = mutated;
        }
        // Cut a run of characters out of a line.
        5 => {
            let index = rng.gen_range(0..lines.len());
            let line = lines[index].clone();
            if line.is_empty() {
                return;
            }
            let count = line.chars().count();
            let start = rng.gen_range(0..count);
            let end = (start + rng.gen_range(1..=6)).min(count);
            let (start, end) = (char_boundary(&line, start), char_boundary(&line, end));
            let mut mutated = line;
            mutated.replace_range(start..end, "");
            lines[index] = mutated;
        }
        // Swap a declared type for another one, leaving the value alone.
        6 => {
            let index = rng.gen_range(0..lines.len());
            let replacement = TYPES[rng.gen_range(0..TYPES.len())];
            if let Some(position) = lines[index].find(':') {
                let after = &lines[index][position + 1..];
                let end = after.find(|character: char| !character.is_alphanumeric() && character != ':' && character != '<' && character != '>' && character != ',' && character != '!').unwrap_or(after.len());
                let mut mutated = lines[index].clone();
                mutated.replace_range(position + 1..position + 1 + end, replacement);
                lines[index] = mutated;
            }
        }
        // Swap a number for one at an edge of what its type can hold.
        7 => {
            let index = rng.gen_range(0..lines.len());
            let line = lines[index].clone();
            let Some(start) = line.find(|character: char| character.is_ascii_digit()) else { return };
            let end = line[start..].find(|character: char| !character.is_ascii_digit() && character != '.').map(|offset| start + offset).unwrap_or(line.len());
            let mut mutated = line;
            mutated.replace_range(start..end, NUMBERS[rng.gen_range(0..NUMBERS.len())]);
            lines[index] = mutated;
        }
        // Rename one use of an identifier to another name in the same file,
        // which is how a program ends up using the wrong variable of the
        // wrong type rather than an undefined one.
        8 => {
            let names = identifiers(lines);
            if names.len() < 2 {
                return;
            }
            let index = rng.gen_range(0..lines.len());
            let from = &names[rng.gen_range(0..names.len())];
            let to = &names[rng.gen_range(0..names.len())];
            lines[index] = lines[index].replacen(from.as_str(), to.as_str(), 1);
        }
        // Nest a block inside itself, a few levels at a time. Deep nesting is
        // where recursion in the compiler used to run the stack out.
        9 => {
            let index = rng.gen_range(0..lines.len());
            let depth = rng.gen_range(2..=40);
            let line = lines[index].clone();
            let nested = format!("{}{}{}", "if { true -> { ".repeat(depth), line, " }, else -> { } }".repeat(depth));
            lines[index] = nested;
        }
        // Duplicate a whole span of the file, which grows programs the way
        // real ones grow and keeps declarations paired with their uses.
        _ => {
            let start = rng.gen_range(0..lines.len());
            let length = rng.gen_range(1..=12).min(lines.len() - start);
            let block: Vec<String> = lines[start..start + length].to_vec();
            let at = rng.gen_range(0..=lines.len());
            for (offset, line) in block.into_iter().enumerate() {
                lines.insert((at + offset).min(lines.len()), line);
            }
        }
    }
}

/// The names a file uses, deduplicated, for the rename mutation. Anything
/// that reads like a Nail identifier counts: keywords included, because
/// renaming a use into a keyword is exactly the kind of edit a person makes
/// by accident.
fn identifiers(lines: &[String]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for line in lines {
        let mut current = String::new();
        for character in line.chars() {
            if character.is_alphanumeric() || character == '_' {
                current.push(character);
            } else {
                if current.len() >= 2 && !current.chars().next().unwrap().is_ascii_digit() && !names.contains(&current) {
                    names.push(current.clone());
                }
                current.clear();
            }
        }
        if current.len() >= 2 && !current.chars().next().unwrap().is_ascii_digit() && !names.contains(&current) {
            names.push(current);
        }
    }
    names
}

/// The byte offset of a character position, so inserting into a line never
/// splits a multi-byte character. Nail files are full of them: every string
/// is written between backticks and plenty of them hold text that is not
/// ASCII.
fn char_boundary(line: &str, character_index: usize) -> usize {
    line.char_indices().nth(character_index).map(|(offset, _)| offset).unwrap_or(line.len())
}
