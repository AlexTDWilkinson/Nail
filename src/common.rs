use std::fmt;

pub const GLOBAL_SCOPE: usize = 0;
pub const ERROR_SCOPE: usize = usize::MAX;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NailDataTypeDescriptor {
    Int,
    Float,
    String,
    Boolean,
    Array(Box<NailDataTypeDescriptor>), // Generic array type for all arrays
    Struct(String),
    Enum(String),
    Void,
    Never, // For functions that never return (like panic, todo)
    Error,
    OneOf(Vec<NailDataTypeDescriptor>),
    Fn(Vec<NailDataTypeDescriptor>, Box<NailDataTypeDescriptor>),
    Result(Box<NailDataTypeDescriptor>),                               // For types like i!e, f!e, s!e
    HashMap(Box<NailDataTypeDescriptor>, Box<NailDataTypeDescriptor>), // For types like h<s,s>
    Any,                                                               // Any type accepts any concrete type
    FailedToResolve,                                                   // Only used internally during type resolution
    TypeVar(String, Vec<NailDataTypeDescriptor>),                      // Type variable in stdlib signatures (e.g. T); resolved by unification at call sites. An empty bounds list accepts any type; a non-empty list restricts the variable to those types (e.g. T: i|f)
}

impl fmt::Display for NailDataTypeDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NailDataTypeDescriptor::Int => write!(f, "i"),
            NailDataTypeDescriptor::Float => write!(f, "f"),
            NailDataTypeDescriptor::String => write!(f, "s"),
            NailDataTypeDescriptor::Boolean => write!(f, "b"),
            NailDataTypeDescriptor::Array(inner) => write!(f, "a:{}", inner),
            NailDataTypeDescriptor::Struct(name) => write!(f, "{}", name),
            NailDataTypeDescriptor::Enum(name) => write!(f, "{}", name),
            NailDataTypeDescriptor::Void => write!(f, "v"),
            NailDataTypeDescriptor::Never => write!(f, "!"),
            NailDataTypeDescriptor::Error => write!(f, "e"),
            NailDataTypeDescriptor::Result(inner) => write!(f, "{}!e", inner),
            NailDataTypeDescriptor::TypeVar(name, bounds) => {
                write!(f, "{}", name)?;
                if !bounds.is_empty() {
                    write!(f, ": ")?;
                    for (i, b) in bounds.iter().enumerate() {
                        if i > 0 {
                            write!(f, "|")?;
                        }
                        write!(f, "{}", b)?;
                    }
                }
                Ok(())
            }
            NailDataTypeDescriptor::HashMap(key, value) => write!(f, "h<{},{}>", key, value),
            NailDataTypeDescriptor::OneOf(types) => {
                write!(f, "OneOf<")?;
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, ">")
            }
            NailDataTypeDescriptor::Fn(params, ret) => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, "):{}", ret)
            }
            NailDataTypeDescriptor::Any => write!(f, "Any"),
            NailDataTypeDescriptor::FailedToResolve => write!(f, "FailedToResolve"),
        }
    }
}

impl NailDataTypeDescriptor {
    /// Human-friendly name used in error messages: the plain-English word
    /// followed by the Nail type syntax, e.g. "an integer (i)".
    pub fn describe(&self) -> String {
        match self {
            NailDataTypeDescriptor::Int => "an integer (i)".to_string(),
            NailDataTypeDescriptor::Float => "a float (f)".to_string(),
            NailDataTypeDescriptor::String => "a string (s)".to_string(),
            NailDataTypeDescriptor::Boolean => "a boolean (b)".to_string(),
            NailDataTypeDescriptor::Void => "void (v)".to_string(),
            NailDataTypeDescriptor::Error => "an error (e)".to_string(),
            NailDataTypeDescriptor::Result(inner) => format!("a result ({}!e) that may contain an error", inner),
            NailDataTypeDescriptor::Array(_) => format!("an array ({})", self),
            NailDataTypeDescriptor::HashMap(_, _) => format!("a hashmap ({})", self),
            NailDataTypeDescriptor::Struct(name) => format!("the struct '{}'", name),
            NailDataTypeDescriptor::Enum(name) => format!("the enum '{}'", name),
            NailDataTypeDescriptor::Fn(_, _) => format!("a function ({})", self),
            NailDataTypeDescriptor::TypeVar(_, bounds) if !bounds.is_empty() => bounds.iter().map(|b| b.describe()).collect::<Vec<_>>().join(" or "),
            NailDataTypeDescriptor::TypeVar(_, _) => "any type".to_string(),
            NailDataTypeDescriptor::Any => "any type".to_string(),
            NailDataTypeDescriptor::Never => "a value that never returns (!)".to_string(),
            NailDataTypeDescriptor::OneOf(types) if !types.is_empty() => types.iter().map(|t| t.describe()).collect::<Vec<_>>().join(" or "),
            // Internal placeholders for types the checker could not work out
            // (e.g. because of an earlier error); never show their debug names.
            NailDataTypeDescriptor::OneOf(_) | NailDataTypeDescriptor::FailedToResolve => "a value whose type could not be determined".to_string(),
        }
    }

    /// The same type with every type variable's bounds dropped. A signature
    /// prints `T` at each position the variable appears and says what T
    /// accepts once at the end, because repeating the bounds inline read as
    /// `value:T: i|f|s|b`, two colons deep before the reader reached the type.
    pub fn without_type_var_bounds(&self) -> NailDataTypeDescriptor {
        match self {
            NailDataTypeDescriptor::TypeVar(name, _) => NailDataTypeDescriptor::TypeVar(name.clone(), Vec::new()),
            NailDataTypeDescriptor::Array(inner) => NailDataTypeDescriptor::Array(Box::new(inner.without_type_var_bounds())),
            NailDataTypeDescriptor::HashMap(key, value) => NailDataTypeDescriptor::HashMap(Box::new(key.without_type_var_bounds()), Box::new(value.without_type_var_bounds())),
            NailDataTypeDescriptor::Result(inner) => NailDataTypeDescriptor::Result(Box::new(inner.without_type_var_bounds())),
            NailDataTypeDescriptor::Fn(parameters, return_type) => NailDataTypeDescriptor::Fn(
                parameters.iter().map(|parameter| parameter.without_type_var_bounds()).collect(),
                Box::new(return_type.without_type_var_bounds()),
            ),
            NailDataTypeDescriptor::OneOf(types) => NailDataTypeDescriptor::OneOf(types.iter().map(|one| one.without_type_var_bounds()).collect()),
            other => other.clone(),
        }
    }

    /// Every type variable in this type that restricts what it accepts, in the
    /// order it appears, as (name, the types it accepts). A signature uses
    /// this to explain its variables after printing them.
    pub fn bounded_type_vars(&self) -> Vec<(String, Vec<NailDataTypeDescriptor>)> {
        match self {
            NailDataTypeDescriptor::TypeVar(name, bounds) if !bounds.is_empty() => vec![(name.clone(), bounds.clone())],
            NailDataTypeDescriptor::Array(inner) | NailDataTypeDescriptor::Result(inner) => inner.bounded_type_vars(),
            NailDataTypeDescriptor::HashMap(key, value) => {
                let mut found = key.bounded_type_vars();
                found.extend(value.bounded_type_vars());
                found
            }
            NailDataTypeDescriptor::Fn(parameters, return_type) => {
                let mut found: Vec<(String, Vec<NailDataTypeDescriptor>)> = parameters.iter().flat_map(|parameter| parameter.bounded_type_vars()).collect();
                found.extend(return_type.bounded_type_vars());
                found
            }
            NailDataTypeDescriptor::OneOf(types) => types.iter().flat_map(|one| one.bounded_type_vars()).collect(),
            _ => Vec::new(),
        }
    }

    /// True when this type is an internal placeholder meaning "the checker could
    /// not work out the type", usually because an earlier error (undefined
    /// variable, bad call, ...) was already reported. Follow-up errors about
    /// such values should be suppressed rather than shown as noise.
    pub fn is_unresolved(&self) -> bool {
        match self {
            NailDataTypeDescriptor::FailedToResolve => true,
            NailDataTypeDescriptor::OneOf(types) => types.is_empty(),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodeError {
    pub message: String,
    pub code_span: CodeSpan,
    /// Optional actionable fix suggestion, rendered as a `help:` line under the snippet.
    pub help: Option<String>,
}

// Writing into a String cannot actually fail; this conversion exists so `write!`
// can be used with `?` inside functions that report real CodeErrors.
impl From<fmt::Error> for CodeError {
    fn from(_: fmt::Error) -> Self {
        CodeError { help: None, message: "internal transpiler error: output formatting failed".to_string(), code_span: CodeSpan::default() }
    }
}

impl fmt::Display for CodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error at line {}, column {}: {}",
               self.code_span.start_line,
               self.code_span.start_column,
               self.message)
    }
}

impl CodeError {
    /// Render this error Rust-style: the message, a file:line:column pointer, the
    /// offending source line with a caret underline, and an optional help line.
    /// Lines and columns in CodeSpan are 1-based; a 0 line means the span is
    /// unknown and the snippet is omitted.
    pub fn render(&self, filename: &str, source: &str) -> String {
        self.render_with_map(&SourceMap::single(filename, source))
    }

    /// Same rendering, but for a program assembled out of imports: the span's
    /// line is looked up in the source map, so the pointer names the file the
    /// error is actually in, at that file's own line number.
    pub fn render_with_map(&self, map: &SourceMap) -> String {
        let mut out = String::new();
        out.push_str(&format!("error: {}\n", self.message));

        match map.resolve(self.code_span.start_line) {
            Some((file, line_no)) if file.content.lines().nth(line_no - 1).is_some() => {
                let text = file.content.lines().nth(line_no - 1).unwrap();
                out.push_str(&format!("  --> {}:{}:{}\n", file.path, line_no, self.code_span.start_column));
                let gutter = line_no.to_string().len().max(2);
                out.push_str(&format!("{:>width$} |\n", "", width = gutter));
                out.push_str(&format!("{:>width$} | {}\n", line_no, text, width = gutter));

                let col = self.code_span.start_column.max(1);
                let underline_len = if self.code_span.end_line == self.code_span.start_line && self.code_span.end_column > self.code_span.start_column {
                    self.code_span.end_column - self.code_span.start_column
                } else {
                    1
                };
                // Pad to the caret start honoring tabs so the underline stays aligned
                let pad: String = text.chars().take(col - 1).map(|c| if c == '\t' { '\t' } else { ' ' }).collect();
                out.push_str(&format!("{:>width$} | {}{}\n", "", pad, "^".repeat(underline_len), width = gutter));
            }
            _ => {
                out.push_str(&format!("  --> {}\n", map.entry().path));
            }
        }

        if let Some(help) = &self.help {
            out.push_str(&format!("help: {}\n", help));
        }
        out
    }
}

/// One diagnostic as a machine-readable record. `nailc --json` prints these
/// so a tool driving the compiler can read positions and fixes as data
/// instead of parsing the human rendering. Positions are 1-based and name
/// the file the error is actually in, the same resolution the human
/// rendering does. Fields whose value is unknown are omitted rather than
/// filled with a placeholder.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiagnosticRecord {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

impl DiagnosticRecord {
    pub fn from_error(error: &CodeError, map: &SourceMap) -> DiagnosticRecord {
        match map.resolve(error.code_span.start_line) {
            Some((file, line)) => {
                // The end position only means something when it lands in the
                // same file as the start, which it always does today. Spans
                // are never zero-width, so a same-line end column equal to
                // the start would be a lie worth omitting too.
                let end_line = map.resolve(error.code_span.end_line).filter(|(end_file, _)| end_file.path == file.path).map(|(_, resolved)| resolved);
                DiagnosticRecord {
                    message: error.message.clone(),
                    file: Some(file.path.clone()),
                    line: Some(line),
                    column: Some(error.code_span.start_column),
                    end_line,
                    end_column: end_line.map(|_| error.code_span.end_column),
                    help: error.help.clone(),
                }
            }
            None => DiagnosticRecord {
                message: error.message.clone(),
                file: Some(map.entry().path.clone()),
                line: None,
                column: None,
                end_line: None,
                end_column: None,
                help: error.help.clone(),
            },
        }
    }
}

/// The whole report `nailc --json` prints for a failed compile: which stage
/// refused and every diagnostic it produced, as one line of JSON.
pub fn diagnostics_json(stage: &str, errors: &[CodeError], map: &SourceMap) -> String {
    let records: Vec<DiagnosticRecord> = errors.iter().map(|error| DiagnosticRecord::from_error(error, map)).collect();
    serde_json::json!({
        "status": "error",
        "stage": stage,
        "errors": records,
    })
    .to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CodeSpan {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

impl Default for CodeSpan {
    fn default() -> Self {
        CodeSpan {
            start_line: 0,
            start_column: 0,
            end_line: 0,
            end_column: 0,
        }
    }
}

/// One file of a lexed program. Spans carry no file of their own; instead
/// every file owns a range of the program's line numbers, `base + 1 ..= base +
/// lines`, and a span's line is mapped back through the SourceMap to the file
/// it came from. The entry file has base 0, so for a program with no imports
/// the numbers are the file's own and nothing changes.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceFile {
    /// The path a human should see: the entry file as it was named on the
    /// command line, an imported file as its import path resolved.
    pub path: String,
    /// This file's lines n are program lines base + n.
    pub base: usize,
    /// How many lines the file has, version line included.
    pub lines: usize,
    /// The file's full text, for error snippets.
    pub content: String,
    /// Program line of the import statement that spliced this file in.
    /// 0 for the entry file.
    pub imported_at: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SourceMap {
    pub files: Vec<SourceFile>,
}

impl SourceMap {
    /// A map for a bare single file, so single-file tools and the old
    /// render() path keep working unchanged.
    pub fn single(path: &str, content: &str) -> SourceMap {
        SourceMap {
            files: vec![SourceFile { path: path.to_string(), base: 0, lines: content.lines().count(), content: content.to_string(), imported_at: 0 }],
        }
    }

    pub fn entry(&self) -> &SourceFile {
        &self.files[0]
    }

    /// The file owning this program line, and the 1-based line inside it.
    pub fn resolve(&self, program_line: usize) -> Option<(&SourceFile, usize)> {
        if program_line == 0 {
            return None;
        }
        self.files.iter().find(|f| program_line > f.base && program_line <= f.base + f.lines).map(|f| (f, program_line - f.base))
    }

    /// True when this program line belongs to the entry file itself.
    pub fn is_entry_line(&self, program_line: usize) -> bool {
        matches!(self.resolve(program_line), Some((file, _)) if file.base == 0)
    }

    /// Walk import statements outward until the line lands in the entry file:
    /// the line a single-file view should pin a foreign error to.
    pub fn anchor_in_entry(&self, mut program_line: usize) -> usize {
        for _ in 0..=self.files.len() {
            match self.resolve(program_line) {
                Some((file, real)) if file.base == 0 => return real,
                Some((file, _)) => program_line = file.imported_at,
                None => return 1,
            }
        }
        1
    }
}

/// How much stack the compiler runs on.
///
/// Reading a program is recursive at every stage, and a stack frame in an
/// unoptimized build of the transpiler is large, so an expression a few dozen
/// levels deep can use more stack than a thread is given by default. A thread
/// created without asking gets eight megabytes on the main thread and two on
/// any other, and the compiler runs on both: `cargo test` runs it on a test
/// thread, and the editor runs it on a worker. This is the one number that
/// makes all of those the same, and it is far larger than the deepest program
/// the parser will accept (see MAX_AST_DEPTH).
pub const COMPILER_STACK_BYTES: usize = 64 * 1024 * 1024;

/// Run the compiler on a stack of its own.
///
/// Every entry into the pipeline goes through here, so that how much stack is
/// available never depends on who called. Depth limits in the parser decide
/// what a program may do, and this decides that those limits mean the same
/// thing everywhere.
#[cfg(not(target_arch = "wasm32"))]
pub fn with_compiler_stack<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    // Already on one: run here. This is what lets a caller that compiles
    // thousands of programs in a row (the fuzzer) pay for the thread once
    // instead of once per program, while a caller that compiles one program
    // still gets the stack without having to know about any of this.
    if ALREADY_ON_ONE.with(|flag| flag.get()) {
        return work();
    }
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("nail-compiler".to_string())
            .stack_size(COMPILER_STACK_BYTES)
            .spawn_scoped(scope, || {
                ALREADY_ON_ONE.with(|flag| flag.set(true));
                work()
            })
            .expect("the compiler thread starts")
            .join()
            // A panic inside the compiler is resumed on the calling thread, so
            // callers that catch panics (the fuzzer) still see them, and
            // callers that do not still get the usual crash.
            .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
    })
}

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static ALREADY_ON_ONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A browser has one thread and no way to ask for a bigger stack, so there
/// the compiler simply runs where it was called.
#[cfg(target_arch = "wasm32")]
pub fn with_compiler_stack<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    work()
}

#[cfg(test)]
mod source_map_tests {
    use super::*;

    fn map() -> SourceMap {
        SourceMap {
            files: vec![
                SourceFile { path: "entry.nail".into(), base: 0, lines: 10, content: String::new(), imported_at: 0 },
                SourceFile { path: "a.nail".into(), base: 10, lines: 5, content: String::new(), imported_at: 3 },
                // b is imported from line 12, which is line 2 of a
                SourceFile { path: "b.nail".into(), base: 15, lines: 5, content: String::new(), imported_at: 12 },
            ],
        }
    }

    #[test]
    fn resolves_lines_to_owning_files() {
        let m = map();
        assert_eq!(m.resolve(0), None);
        assert_eq!(m.resolve(1).map(|(f, l)| (f.path.as_str(), l)), Some(("entry.nail", 1)));
        assert_eq!(m.resolve(10).map(|(f, l)| (f.path.as_str(), l)), Some(("entry.nail", 10)));
        assert_eq!(m.resolve(11).map(|(f, l)| (f.path.as_str(), l)), Some(("a.nail", 1)));
        assert_eq!(m.resolve(15).map(|(f, l)| (f.path.as_str(), l)), Some(("a.nail", 5)));
        assert_eq!(m.resolve(16).map(|(f, l)| (f.path.as_str(), l)), Some(("b.nail", 1)));
        assert_eq!(m.resolve(21), None);
    }

    #[test]
    fn anchors_foreign_lines_at_their_import_statement() {
        let m = map();
        assert_eq!(m.anchor_in_entry(7), 7);
        assert_eq!(m.anchor_in_entry(12), 3);
        // b came in through a, so the anchor walks a's own import too
        assert_eq!(m.anchor_in_entry(17), 3);
        assert_eq!(m.anchor_in_entry(999), 1);
    }
}