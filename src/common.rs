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
            other => other.to_string(),
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
        let mut out = String::new();
        out.push_str(&format!("error: {}\n", self.message));

        let line_no = self.code_span.start_line;
        let source_line = if line_no >= 1 { source.lines().nth(line_no - 1) } else { None };

        match source_line {
            Some(text) => {
                out.push_str(&format!("  --> {}:{}:{}\n", filename, line_no, self.code_span.start_column));
                let gutter = line_no.to_string().len().max(2);
                out.push_str(&format!("{:>width$} |\n", "", width = gutter));
                out.push_str(&format!("{:>width$} | {}\n", line_no, text, width = gutter));

                let col = self.code_span.start_column.max(1);
                let underline_len = if self.code_span.end_line == line_no && self.code_span.end_column > self.code_span.start_column {
                    self.code_span.end_column - self.code_span.start_column
                } else {
                    1
                };
                // Pad to the caret start honoring tabs so the underline stays aligned
                let pad: String = text.chars().take(col - 1).map(|c| if c == '\t' { '\t' } else { ' ' }).collect();
                out.push_str(&format!("{:>width$} | {}{}\n", "", pad, "^".repeat(underline_len), width = gutter));
            }
            None => {
                out.push_str(&format!("  --> {}\n", filename));
            }
        }

        if let Some(help) = &self.help {
            out.push_str(&format!("help: {}\n", help));
        }
        out
    }
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