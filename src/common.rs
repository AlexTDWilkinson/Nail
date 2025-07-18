use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct CodeError {
    pub message: String,
    pub code_span: CodeSpan,
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