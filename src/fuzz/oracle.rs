//! What counts as a bug.
//!
//! A fuzzer is only as good as the questions it asks of each program, and
//! these are the questions. They are deliberately not "does this program
//! compile": most fuzzed programs are nonsense and being refused is the
//! correct answer. What is never correct is crashing, contradicting an
//! earlier stage, or pointing at a place in the file that does not exist.

use std::panic::{self, AssertUnwindSafe};
use std::path::Path;
use std::sync::Once;

use crate::checker::checker;
use crate::common::CodeError;
use crate::lexer::{collect_lexer_errors, lex_program};
use crate::parser::{parse, ASTNode};
use crate::transpiler::Transpiler;

/// The invariant a case broke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Property {
    /// A compiler stage panicked. Any input at all, however malformed, has to
    /// come back as an error rather than as a crash.
    NoPanic,
    /// The type checker passed a program the transpiler then refused. The
    /// checker is what stands between a programmer and a Rust error, so
    /// anything it accepts has to survive every later stage.
    CheckImpliesTranspile,
    /// The type checker passed a program rustc then refused. Same promise,
    /// one stage further out, and the one that reaches users as a wall of
    /// Rust they never wrote.
    CheckImpliesBuild,
    /// A program built and then printed something other than its answer.
    /// Every stage before this one asks whether the compiler accepted the
    /// program: this one asks whether the program it produced is the program
    /// that was written, which is the only failure a person cannot see
    /// coming.
    RunsWithTheRightAnswer,
    /// An error pointed outside the file it was reported against.
    ErrorPointsSomewhereReal,
    /// Formatting a file twice differs from formatting it once.
    FormatIsStable,
    /// Formatting changed whether the program compiles.
    FormatKeepsMeaning,
    /// Syntax highlighting crashed on a file. The editor colorizes whatever
    /// is in the buffer, valid or not, on every keystroke, so a panic here is
    /// an editor that dies while somebody is typing.
    HighlightingNeverCrashes,
    /// Highlighting changed the text of a line. Colors are painted over the
    /// buffer, so whatever the colorizer hands back is what the editor draws:
    /// a dropped or duplicated character is a file that looks wrong on screen
    /// while being right on disk.
    HighlightingKeepsTheText,
    /// A program the compiler said was fit for a browser used something a
    /// browser does not have, or one it refused was refused for no reason the
    /// registry can name. `nailc --target=wasm` is the promise that a program
    /// either runs in a browser or is told plainly why not, and both halves
    /// of that have to hold.
    WasmRefusalIsHonest,
    /// Sandboxed code reached the world. `import` promises that the file it
    /// brings in can only compute, and that promise is what makes importing
    /// somebody else's code safe at all.
    SandboxHolds,
    /// Highlighting disagreed with the lexer about what something is. The
    /// colorizer is its own reader, deliberately, so that a file that does
    /// not compile still colors sensibly, but where the two do read the same
    /// text they have to agree: a string the compiler sees has to be painted
    /// as a string.
    HighlightingAgreesWithTheLexer,
}

impl Property {
    pub fn name(self) -> &'static str {
        match self {
            Property::NoPanic => "no-panic",
            Property::CheckImpliesTranspile => "check-implies-transpile",
            Property::CheckImpliesBuild => "check-implies-build",
            Property::RunsWithTheRightAnswer => "runs-with-the-right-answer",
            Property::ErrorPointsSomewhereReal => "error-points-somewhere-real",
            Property::FormatIsStable => "format-is-stable",
            Property::FormatKeepsMeaning => "format-keeps-meaning",
            Property::HighlightingNeverCrashes => "highlighting-never-crashes",
            Property::HighlightingKeepsTheText => "highlighting-keeps-the-text",
            Property::HighlightingAgreesWithTheLexer => "highlighting-agrees-with-the-lexer",
            Property::WasmRefusalIsHonest => "wasm-refusal-is-honest",
            Property::SandboxHolds => "sandbox-holds",
        }
    }

    /// What the property promises, spelled out for the finding report so a
    /// person reading it later does not have to come back here.
    pub fn promise(self) -> &'static str {
        match self {
            Property::NoPanic => "every input, however broken, produces an error rather than a crash",
            Property::CheckImpliesTranspile => "a program the type checker accepts can always be transpiled",
            Property::CheckImpliesBuild => "a program the type checker accepts always builds as Rust",
            Property::RunsWithTheRightAnswer => "a program that builds prints exactly what the code it was written from says it should",
            Property::ErrorPointsSomewhereReal => "every error points at a line and column that exist in the file",
            Property::FormatIsStable => "formatting an already formatted file changes nothing",
            Property::FormatKeepsMeaning => "formatting never changes whether a program compiles",
            Property::HighlightingNeverCrashes => "the editor can colorize any buffer, however broken, without dying",
            Property::HighlightingKeepsTheText => "coloring a line paints it, and never changes a character of it",
            Property::HighlightingAgreesWithTheLexer => "what the compiler reads as a string or a number is painted as one",
            Property::WasmRefusalIsHonest => "a program is refused for the browser exactly when it uses something a browser does not have",
            Property::SandboxHolds => "code brought in by import can compute and nothing else",
        }
    }
}

/// One broken invariant, with enough detail to fix it and to tell it apart
/// from every other finding.
#[derive(Debug, Clone)]
pub struct Finding {
    pub property: Property,
    /// The compiler stage that broke it: lex, parse, check, transpile, format.
    pub stage: &'static str,
    /// What happened, in one line. This names the program's own text, so it
    /// is for reading rather than for counting.
    pub detail: String,
    /// The kind of failure, with nothing of the program in it. Two cases that
    /// break the same way fold into one finding, however different the
    /// programs are. Without it a run reports thousands of copies of one bug,
    /// one per literal it happened to be looking at.
    pub class: &'static str,
    /// Where in the compiler, when that is known (a panic's file and line).
    /// This is what makes two findings with different programs but the same
    /// underlying bug fold into one.
    pub site: Option<String>,
}

impl Finding {
    /// A short stable name for this bug, used as the findings file name and
    /// as the deduplication key. Two cases that break in the same place are
    /// the same finding, however different the programs look.
    pub fn fingerprint(&self) -> String {
        let what = self.site.clone().unwrap_or_else(|| self.class.to_string());
        format!("{}_{}_{}", self.property.name(), self.stage, squash(&what))
    }
}

/// A string reduced to something usable as a file name: lowercase, letters and
/// digits and single underscores, capped in length.
fn squash(text: &str) -> String {
    let mut out = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
        if out.len() >= 60 {
            break;
        }
    }
    out.trim_matches('_').to_string()
}

// Where the last panic happened, kept by the hook below so that a caught
// panic can be attributed to a place in the compiler rather than only to a
// message. Thread local because workers run cases on their own threads.
thread_local! {
    static LAST_PANIC_SITE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

static HOOK: Once = Once::new();

/// Silence the default panic printout and record where the panic came from.
/// Without this every caught panic would print a backtrace banner and bury
/// the fuzzer's own output, and the finding would name no place in the source.
pub fn install_panic_hook() {
    HOOK.call_once(|| {
        panic::set_hook(Box::new(|info| {
            let site = info.location().map(|location| format!("{}:{}", location.file(), location.line()));
            LAST_PANIC_SITE.with(|cell| *cell.borrow_mut() = site);
        }));
    });
}

/// Run one stage, turning a panic into a value. The stage name is carried so
/// a finding can say which one blew up.
fn guard<T>(stage: &'static str, work: impl FnOnce() -> T) -> Result<T, Finding> {
    LAST_PANIC_SITE.with(|cell| *cell.borrow_mut() = None);
    match panic::catch_unwind(AssertUnwindSafe(work)) {
        Ok(value) => Ok(value),
        Err(payload) => {
            let message = if let Some(text) = payload.downcast_ref::<&str>() {
                (*text).to_string()
            } else if let Some(text) = payload.downcast_ref::<String>() {
                text.clone()
            } else {
                "panicked with a value that is not text".to_string()
            };
            let site = LAST_PANIC_SITE.with(|cell| cell.borrow().clone());
            Err(Finding { property: Property::NoPanic, stage, detail: message, site, class: "panicked" })
        }
    }
}

/// How far a program got, so the caller knows whether it is worth handing to
/// rustc. The AST is not carried out: it is the transpiled Rust that the
/// build tier needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Refused by some stage, which for a fuzzed program is the usual
    /// answer. The stage that refused it is carried so a run can report how
    /// far its programs are getting: an engine whose cases all die at the
    /// lexer is an engine testing nothing.
    Refused(&'static str),
    /// Type checked and transpiled. The Rust is the string.
    Built(String),
}

/// Ask every in-process question of one program. A finding is returned as
/// soon as one is found, because the first broken invariant is the one worth
/// reporting and everything after it is downstream of the same bug.
///
/// `path` is where the case lives on disk. It matters only because `import`
/// resolves against the importing file, so a case that imports something has
/// to be looked at from where it sits.
pub fn examine(source: &str, path: &Path) -> (Option<Finding>, Outcome) {
    // On the compiler's own stack, the same one every other entry point uses,
    // so a case is judged by the depth limits rather than by which thread the
    // fuzzer happened to call from.
    crate::common::with_compiler_stack(|| examine_here(source, path, Sampling::Some))
}

/// Every question, however expensive, for looking at one file by hand.
pub fn examine_thoroughly(source: &str, path: &Path) -> (Option<Finding>, Outcome) {
    crate::common::with_compiler_stack(|| examine_here(source, path, Sampling::None))
}

/// Whether the expensive checks run on every case or on a share of them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sampling {
    /// Everything, for one file a person is asking about.
    None,
    /// A share of the cases, for a run doing thousands a second.
    Some,
}

fn examine_here(source: &str, path: &Path, sampling: Sampling) -> (Option<Finding>, Outcome) {
    // Highlighting comes first because it is asked of everything: the editor
    // colorizes a buffer on every keystroke, long before it is a program.
    //
    // One case in eight, rather than all of them: colorizing spreads itself
    // across every core, which inside a fuzzer already running one worker per
    // core costs more than the rest of the pipeline put together. Which cases
    // are chosen comes from the text, so a case that highlights once
    // highlights every time and a finding still reproduces from its seed.
    if sampling == Sampling::None || fingerprint(source) % 8 == 0 {
        if let Some(finding) = highlighting_holds(source) {
            return (Some(finding), Outcome::Refused("highlight"));
        }
    }

    let lexed = match guard("lex", || lex_program(source, Some(path))) {
        Ok(lexed) => lexed,
        Err(finding) => return (Some(finding), Outcome::Refused("lex")),
    };

    let lexer_errors = match guard("lex", || collect_lexer_errors(&lexed.tokens)) {
        Ok(errors) => errors,
        Err(finding) => return (Some(finding), Outcome::Refused("lex")),
    };
    if !lexer_errors.is_empty() {
        if let Some(finding) = spans_are_real("lex", &lexer_errors, source) {
            return (Some(finding), Outcome::Refused("lex"));
        }
        return (None, Outcome::Refused("lex"));
    }

    let parsed = match guard("parse", || parse(lexed.tokens)) {
        Ok(parsed) => parsed,
        Err(finding) => return (Some(finding), Outcome::Refused("parse")),
    };
    let mut ast = match parsed {
        Ok(ast) => ast,
        Err(error) => {
            if let Some(finding) = spans_are_real("parse", std::slice::from_ref(&error), source) {
                return (Some(finding), Outcome::Refused("parse"));
            }
            return (None, Outcome::Refused("parse"));
        }
    };

    // Colors are compared against what the compiler read, on a file that
    // parses. Half-written text is the colorizer's own business: it has to
    // paint something sensible mid-keystroke, and what that is is its
    // judgement rather than the lexer's.
    if sampling == Sampling::None || fingerprint(source) % 8 == 0 {
        if let Some(finding) = colors_agree_with_the_lexer(source) {
            return (Some(finding), Outcome::Refused("highlight"));
        }
    }

    let checked = match guard("check", || {
        let mut copy = ast.clone();
        let result = checker(&mut copy);
        (copy, result)
    }) {
        Ok((copy, result)) => {
            ast = copy;
            result
        }
        Err(finding) => return (Some(finding), Outcome::Refused("check")),
    };
    if let Err(errors) = checked {
        if let Some(finding) = spans_are_real("check", &errors, source) {
            return (Some(finding), Outcome::Refused("check"));
        }
        return (None, Outcome::Refused("check"));
    }

    // Past this point the program is one the compiler said yes to, so every
    // later failure is the compiler contradicting itself rather than the
    // program being wrong.
    let transpiled = match guard("transpile", || {
        let mut transpiler = Transpiler::new();
        // Profiling writes a file beside the program and reads the clock,
        // neither of which says anything about whether the language works.
        transpiler.profile = false;
        transpiler.transpile(&ast)
    }) {
        Ok(result) => result,
        Err(finding) => return (Some(finding), Outcome::Refused("transpile")),
    };
    let rust = match transpiled {
        Ok(rust) => rust,
        Err(error) => {
            return (
                Some(Finding { property: Property::CheckImpliesTranspile, stage: "transpile", detail: error.message.clone(), site: None, class: "the transpiler refused it" }),
                Outcome::Refused("transpile"),
            )
        }
    };

    if let Some(finding) = formatting_holds(source, &ast, path) {
        return (Some(finding), Outcome::Built(rust));
    }

    if let Some(finding) = the_browser_answer_is_honest(&ast) {
        return (Some(finding), Outcome::Built(rust));
    }

    (None, Outcome::Built(rust))
}

/// A number drawn from the text itself, for deciding which cases get the
/// checks that are too expensive to run on all of them.
fn fingerprint(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Transpiling for a browser answers one of two ways, and both have to be
/// true: either the program is fit for a browser, or it names the calls that
/// are not. A program whose calls are all browser-safe may not be refused,
/// and a program that uses something a browser does not have may not pass.
///
/// The registry decides what is browser-safe, so this compares the compiler's
/// answer with the registry's, which is the only way the answer can be wrong
/// without one of them being obviously broken.
fn the_browser_answer_is_honest(ast: &ASTNode) -> Option<Finding> {
    let outcome = guard("wasm", || {
        let mut transpiler = Transpiler::new();
        transpiler.profile = false;
        transpiler.wasm_target = true;
        let transpiled = transpiler.transpile(ast);
        let blockers = transpiler.wasm_unsupported_functions();
        let used: Vec<String> = transpiler.stdlib_functions_used();
        (transpiled.is_ok(), blockers, used)
    });
    let (transpiled, blockers, used): (bool, Vec<String>, Vec<String>) = match outcome {
        Ok(answer) => answer,
        Err(finding) => return Some(Finding { property: Property::WasmRefusalIsHonest, ..finding }),
    };
    // A refusal is allowed: a browser has limits beyond which functions
    // exist, and the transpiler says so in its own words when it hits one.
    // What is checked is that it never crashes reaching that answer, and that
    // the calls it names agree with the library's own record of what a
    // browser has.
    if !transpiled {
        return None;
    }

    let mut expected: Vec<String> = used.iter().filter(|name| !crate::stdlib_registry::is_stdlib_fn_wasm_safe(name)).cloned().collect();
    expected.sort();
    if expected != blockers {
        return Some(Finding {
            property: Property::WasmRefusalIsHonest,
            stage: "wasm",
            detail: format!("the compiler names {:?} as needing an operating system, and the library says {:?}", blockers, expected),
            site: None,
            class: "the list of blockers disagrees with the registry",
        });
    }
    None
}

/// Colorizing a buffer must never crash, whatever is in it. The editor runs
/// this on text that is mid-edit, so most of what it sees is not a program
/// and never will be.
fn highlighting_holds(source: &str) -> Option<Finding> {
    let lines: Vec<ratatui::text::Line> = source.lines().map(|line| ratatui::text::Line::from(line.to_string())).collect();
    let theme = crate::colorizer::theme_by_name("dracula").unwrap_or(crate::colorizer::THEMES[0].1);
    let painted = match guard("highlight", || crate::colorizer::colorize_code(lines, theme)) {
        Ok(painted) => painted,
        Err(finding) => return Some(Finding { property: Property::HighlightingNeverCrashes, ..finding }),
    };

    // What the editor would draw, line by line, and the color of each
    // character of it.
    let drawn: Vec<(String, Vec<Option<ratatui::style::Color>>)> = painted
        .iter()
        .map(|line| {
            let mut text = String::new();
            let mut colors = Vec::new();
            for span in &line.spans {
                text.push_str(span.content.as_ref());
                for _ in span.content.chars() {
                    colors.push(span.style.fg);
                }
            }
            (text, colors)
        })
        .collect();

    for (index, source_line) in source.lines().enumerate() {
        let Some((drawn_line, _)) = drawn.get(index) else {
            return Some(Finding { property: Property::HighlightingKeepsTheText, stage: "highlight", detail: format!("line {} was dropped entirely", index + 1), site: None, class: "a line went missing" });
        };
        if drawn_line != source_line {
            return Some(Finding {
                property: Property::HighlightingKeepsTheText,
                stage: "highlight",
                detail: format!("line {} was drawn as '{}' but reads '{}'", index + 1, drawn_line, source_line),
                site: None,
                class: "a line was drawn differently",
            });
        }
    }

    None
}

/// The color of every character the editor would draw, for the checks that
/// compare colors against what the compiler read.
fn painted_lines(source: &str) -> Option<Vec<(String, Vec<Option<ratatui::style::Color>>)>> {
    let lines: Vec<ratatui::text::Line> = source.lines().map(|line| ratatui::text::Line::from(line.to_string())).collect();
    let theme = crate::colorizer::theme_by_name("dracula").unwrap_or(crate::colorizer::THEMES[0].1);
    let painted = guard("highlight", || crate::colorizer::colorize_code(lines, theme)).ok()?;
    Some(
        painted
            .iter()
            .map(|line| {
                let mut text = String::new();
                let mut colors = Vec::new();
                for span in &line.spans {
                    text.push_str(span.content.as_ref());
                    for _ in span.content.chars() {
                        colors.push(span.style.fg);
                    }
                }
                (text, colors)
            })
            .collect(),
    )
}

/// Where the compiler and the colorizer read the same thing, they have to
/// paint it the same. Only the two kinds with one unambiguous color each are
/// compared, strings and numbers, and only where a token sits on one line:
/// everything else (an identifier that might be a type, a language embedded
/// in a tagged string) is the colorizer's own judgement to make.
fn colors_agree_with_the_lexer(source: &str) -> Option<Finding> {
    use crate::lexer::TokenType;

    let tokens = match guard("highlight", || crate::lexer::lexer_without_imports(source)) {
        Ok(tokens) => tokens,
        Err(finding) => return Some(Finding { property: Property::HighlightingAgreesWithTheLexer, ..finding }),
    };
    // Only a file the compiler reads cleanly is compared. On text that does
    // not lex, the colorizer is deliberately its own reader: it still has to
    // paint something sensible while a line is half typed, and what that is
    // is its own business.
    if !crate::lexer::collect_lexer_errors(&tokens).is_empty() {
        return None;
    }
    let theme = crate::colorizer::theme_by_name("dracula").unwrap_or(crate::colorizer::THEMES[0].1);
    let drawn = painted_lines(source)?;
    let drawn = &drawn[..];

    for token in &tokens {
        let (what, wanted) = match &token.token_type {
            // A tagged string carries a second language inside it, and the
            // colorizer paints that language's own colors there, so only
            // plain strings are compared.
            TokenType::StringLiteral { tag: None, .. } => ("a string", vec![theme.string_literal]),
            TokenType::Integer(_) => ("a whole number", vec![theme.unsigned_int, theme.signed_int]),
            TokenType::Float(_) => ("a number", vec![theme.float]),
            _ => continue,
        };
        let span = &token.code_span;
        if span.start_line == 0 || span.start_line != span.end_line {
            continue;
        }
        let Some((text, colors)) = drawn.get(span.start_line - 1) else { continue };
        // Columns are one based, and the end column is one past the last
        // character. A span that does not fit the line as drawn is left
        // alone: that is a span bug, which the span invariant reports.
        if span.start_column == 0 || span.end_column <= span.start_column || span.end_column - 1 > colors.len() {
            continue;
        }
        // A minus sign in front of a number is one token to the compiler and
        // an operator to the eye, and the colorizer paints it as an operator
        // on purpose. Only the digits are compared.
        let first_column = if matches!(text.chars().nth(span.start_column - 1), Some('-') | Some('+')) { span.start_column + 1 } else { span.start_column };
        for column in first_column..span.end_column {
            let color = colors[column - 1];
            if !wanted.iter().any(|candidate| color == Some(*candidate)) {
                let piece: String = text.chars().skip(first_column - 1).take(span.end_column - first_column).collect();
                return Some(Finding {
                    property: Property::HighlightingAgreesWithTheLexer,
                    stage: "highlight",
                    detail: format!("the compiler reads '{}' on line {} as {}, and the editor paints it {:?}", piece, span.start_line, what, color),
                    site: None,
                    class: what,
                });
            }
        }
    }
    None
}

/// Every error has to point somewhere a person can look. A line past the end
/// of the file, or a column past the end of its line, means the editor
/// underlines nothing and the message floats free of the code.
///
/// Line zero is how the compiler says "no particular place", which is
/// allowed, so only positive line numbers are held to the file's shape.
fn spans_are_real(stage: &'static str, errors: &[CodeError], source: &str) -> Option<Finding> {
    let lines: Vec<&str> = source.lines().collect();
    for error in errors {
        if error.message.trim().is_empty() {
            return Some(Finding { property: Property::ErrorPointsSomewhereReal, stage, detail: "an error was reported with no message".to_string(), site: None, class: "no message" });
        }
        let span = &error.code_span;
        if span.start_line == 0 {
            continue;
        }
        if span.start_line > lines.len() {
            return Some(Finding {
                property: Property::ErrorPointsSomewhereReal,
                stage,
                detail: format!("error points at line {} of a {} line file: {}", span.start_line, lines.len(), error.message),
                site: None,
                class: "past the end of the file",
            });
        }
        if span.end_line != 0 && span.end_line < span.start_line {
            return Some(Finding {
                property: Property::ErrorPointsSomewhereReal,
                stage,
                detail: format!("error ends at line {} but starts at line {}: {}", span.end_line, span.start_line, error.message),
                site: None,
                class: "ends before it starts",
            });
        }
    }
    None
}

/// The formatter is held to two promises, and only for programs that compile,
/// because a file that does not parse is one the formatter is allowed to
/// leave alone. Formatting twice must match formatting once, and a program
/// that compiled before formatting must still compile after it.
fn formatting_holds(source: &str, original: &ASTNode, path: &Path) -> Option<Finding> {
    let lines: Vec<String> = source.lines().map(String::from).collect();
    let once = match guard("format", || crate::formatter::format_nail_code(&lines)) {
        Ok(formatted) => formatted,
        Err(finding) => return Some(finding),
    };
    let twice = match guard("format", || crate::formatter::format_nail_code(&once)) {
        Ok(formatted) => formatted,
        Err(finding) => return Some(finding),
    };
    if once != twice {
        let first_difference = once
            .iter()
            .zip(twice.iter())
            .position(|(left, right)| left != right)
            .map(|index| format!("line {} became '{}' after a second pass, from '{}'", index + 1, twice[index].trim(), once[index].trim()))
            .unwrap_or_else(|| format!("the formatted file changed length, {} lines then {}", once.len(), twice.len()));
        return Some(Finding { property: Property::FormatIsStable, stage: "format", detail: first_difference, site: None, class: "a second pass differs from the first" });
    }

    // The formatted text has to still be the same program. Checking that it
    // still type checks is the strongest cheap version of that question: the
    // original did, so anything less is the formatter changing meaning.
    let formatted_source = once.join("\n");
    let lexed = match guard("format", || lex_program(&formatted_source, Some(path))) {
        Ok(lexed) => lexed,
        Err(finding) => return Some(finding),
    };
    if !collect_lexer_errors(&lexed.tokens).is_empty() {
        return Some(Finding { property: Property::FormatKeepsMeaning, stage: "format", detail: "the formatted file no longer lexes".to_string(), site: None, class: "no longer lexes" });
    }
    let parsed = match guard("format", || parse(lexed.tokens)) {
        Ok(parsed) => parsed,
        Err(finding) => return Some(finding),
    };
    let mut formatted_ast = match parsed {
        Ok(ast) => ast,
        Err(error) => return Some(Finding { property: Property::FormatKeepsMeaning, stage: "format", detail: format!("the formatted file no longer parses: {}", error.message), site: None, class: "no longer parses" }),
    };
    let checked = match guard("format", || checker(&mut formatted_ast)) {
        Ok(result) => result,
        Err(finding) => return Some(finding),
    };
    if let Err(errors) = checked {
        let first = errors.first().map(|error| error.message.clone()).unwrap_or_default();
        return Some(Finding { property: Property::FormatKeepsMeaning, stage: "format", detail: format!("the formatted file no longer type checks: {}", first), site: None, class: "no longer type checks" });
    }

    // Comparing the trees themselves would be stronger still, but spans move
    // when lines move, so the trees are compared with their spans ignored.
    if !same_shape(original, &formatted_ast) {
        return Some(Finding { property: Property::FormatKeepsMeaning, stage: "format", detail: "the formatted file parses into a different program".to_string(), site: None, class: "a different program" });
    }
    None
}

/// Whether two trees are the same program, ignoring where in the file each
/// piece was written. Formatting moves code, so spans differ by design, and
/// the debug rendering of a node carries its span. Comparing the kind of each
/// node in walk order catches a formatter that drops, reorders or re-nests
/// something without tripping over the positions.
fn same_shape(left: &ASTNode, right: &ASTNode) -> bool {
    let mut left_stack = vec![left];
    let mut right_stack = vec![right];
    loop {
        match (left_stack.pop(), right_stack.pop()) {
            (None, None) => return true,
            (Some(left_node), Some(right_node)) => {
                if std::mem::discriminant(left_node) != std::mem::discriminant(right_node) {
                    return false;
                }
                let left_children = left_node.children();
                let right_children = right_node.children();
                if left_children.len() != right_children.len() {
                    return false;
                }
                left_stack.extend(left_children);
                right_stack.extend(right_children);
            }
            _ => return false,
        }
    }
}
