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
    TypeVar(String),                                                   // Type variable in stdlib signatures (e.g. T); resolved by unification at call sites
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
            NailDataTypeDescriptor::TypeVar(name) => write!(f, "{}", name),
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

#[derive(Debug, Clone, PartialEq)]
pub struct CodeError {
    pub message: String,
    pub code_span: CodeSpan,
}

// Writing into a String cannot actually fail; this conversion exists so `write!`
// can be used with `?` inside functions that report real CodeErrors.
impl From<fmt::Error> for CodeError {
    fn from(_: fmt::Error) -> Self {
        CodeError { message: "internal transpiler error: output formatting failed".to_string(), code_span: CodeSpan::default() }
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

#[derive(Debug, Clone, PartialEq, Eq)]
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