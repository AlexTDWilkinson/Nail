use crate::common::{CodeSpan, SourceFile, SourceMap};
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::fs;
use std::collections::{HashMap, HashSet};

//  static the alphabet in lower and uppercase and 0-9

static ALPHABET_AND_NUMBERS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
static ALPHABET_LOWERCASE_AND_NUMBERS: &str = "abcdefghijklmnopqrstuvwxyz0123456789";
static ALPHABET_UPPERCASE_AND_NUMBERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
static ALPHABET_LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
static ALPHABET: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
static ALPHABET_UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDeclarationData {
    pub name: String,
    pub fields: Vec<StructDeclarationDataField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDeclarationDataField {
    pub name: String,
    pub data_type: NailDataTypeDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Maybe<NailDataType: 'static> {
    Ok(&'static NailDataType), // Statically known data
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NailStruct {
    name: String,
    fields: Vec<(String, NailDataType)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDeclarationData {
    pub name: String,
    pub variants: Vec<Token>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariantData {
    pub name: String,
    pub variant: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NailDataType {
    Int,
    Float,
    String,
    Boolean,
    Array(Vec<NailDataType>), // Can hold other NailDataType values
    Error(String),
    EnumDeclaration(EnumDeclarationData),
    StructDeclaration(StructDeclarationData),
    Maybe(Maybe<NailDataType>), // This can hold a reference to a static NailDataType
    Void,
}

// The core type-system descriptor lives in `common` (shared by parser, checker,
// transpiler, and the stdlib registry); re-exported here for compatibility.
pub use crate::common::NailDataTypeDescriptor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub token_type: TokenType,
    pub code_span: CodeSpan,
}

// Nail is deterministic syntax wise, so we can take advantage of that
// and give our lexer_inner a lot of insight into the syntax of the language
// by having it lex entire declarations at a time, rather than just
// individual tokens. This likely makes both the lexer_inner and parser simpler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    ArrayOpen,
    ArrayClose,
    FunctionReturnTypeDeclaration(NailDataTypeDescriptor),
    FunctionName(String),
    StructDeclaration(StructDeclarationData), // For struct declarations
    EnumDeclaration(EnumDeclarationData), // For enum data
    StructFieldAccess(String, String),
    EnumVariant(EnumVariantData),            // For enum variant name
    Comment(String),                         // For comments
    FunctionSignature(Vec<Token>),           // For function declarations ie "fn"
    Dot,                                     // For dot operator (.)
    IfDeclaration,                           // For if keyword
    ImportKeyword,                           // For import keyword (sandboxed file inclusion)
    ImportDangerousKeyword,                  // For import_dangerous keyword (unsandboxed file inclusion)
    SandboxStart,                             // Marks where tokens spliced by import begin
    SandboxEnd,                               // Marks where tokens spliced by import end
    ElseDeclaration,                         // For else keyword
    ParallelStart,                           // For p keyword
    ParallelEnd,                             // For /p keyword
    ConcurrentStart,                         // For c keyword
    ConcurrentEnd,                           // For /c keyword
    MapDeclaration,                          // For map keyword
    FilterDeclaration,                       // For filter keyword
    ReduceDeclaration,                       // For reduce keyword
    ScanDeclaration,                         // For scan keyword
    EachDeclaration,                         // For each keyword
    FindDeclaration,                         // For find keyword
    AllDeclaration,                          // For all keyword
    AnyDeclaration,                          // For any keyword
    ForeverKeyword,                             // For loop keyword (infinite loops)
    InKeyword,                               // For in keyword
    FromKeyword,                             // For from keyword (initial accumulator)
    Assignment,                              // For assignment ie =
    ArrowAssignment,                         // For arrow assignment ie ->
    Identifier(String),                      // For variable names, etc.
    Float(String),                           // For float numbers
    Integer(String),                         // For integer numbers
    BooleanLiteral(bool),                    // For boolean literals (true/false)
    Operator(Operation),                     // For operators like +, -, *, /
    Comma,                                   // For commas
    Colon,                                   // For colons
    // For string literals. `tag` is the optional language marker written in
    // front of the opening backtick (html`<p>hi</p>`), carried purely so
    // highlighters can render embedded languages. The lexer is agnostic about
    // which tags exist; anything that looks like an identifier is accepted and
    // it is up to each highlighter to recognize the ones it knows.
    StringLiteral { value: String, tag: Option<String> },
    TypeDeclaration(NailDataTypeDescriptor), // For explicit type declarations
    ParenthesisOpen,                         // For parenthesis open
    ParenthesisClose,                        // For parenthesis close
    BlockOpen,                               // For block start
    BlockClose,                              // For block end
    EndStatementOrExpression,                // For end of statement or expression
    LexerError(String),                      // For lexer_inner errors
    Return,                                  // For return keyword
    Yield,                                   // For yield keyword
    EndOfFile,                               // For end of file
}

impl TokenType {
    /// Plain-language name for a token, used in error messages instead of the
    /// internal enum variant name.
    pub fn describe(&self) -> String {
        match self {
            TokenType::BlockOpen => "'{'".to_string(),
            TokenType::BlockClose => "'}'".to_string(),
            TokenType::ParenthesisOpen => "'('".to_string(),
            TokenType::ParenthesisClose => "')'".to_string(),
            TokenType::ArrayOpen => "'['".to_string(),
            TokenType::ArrayClose => "']'".to_string(),
            TokenType::Comma => "','".to_string(),
            TokenType::EndStatementOrExpression => "';'".to_string(),
            TokenType::Assignment => "'='".to_string(),
            TokenType::ArrowAssignment => "'->'".to_string(),
            TokenType::Dot => "'.'".to_string(),
            TokenType::EndOfFile => "the end of the file".to_string(),
            TokenType::Identifier(name) => format!("the name '{}'", name),
            TokenType::Colon => "':'".to_string(),
            TokenType::Integer(value) => format!("the number '{}'", value),
            TokenType::Float(value) => format!("the number '{}'", value),
            TokenType::BooleanLiteral(value) => format!("the boolean '{}'", value),
            TokenType::StringLiteral { tag: None, .. } => "a string literal".to_string(),
            TokenType::StringLiteral { tag: Some(tag), .. } => format!("a {} string literal", tag),
            TokenType::Operator(op) => format!("the operator '{}'", op),
            TokenType::TypeDeclaration(data_type) => format!("the type ':{}'", data_type),
            TokenType::FunctionReturnTypeDeclaration(data_type) => format!("the return type ':{}'", data_type),
            TokenType::FunctionName(name) => format!("the function name '{}'", name),
            TokenType::FunctionSignature(_) => "a function declaration".to_string(),
            TokenType::StructDeclaration(_) => "a struct declaration".to_string(),
            TokenType::EnumDeclaration(_) => "an enum declaration".to_string(),
            TokenType::StructFieldAccess(object, field) => format!("the field access '{}.{}'", object, field),
            TokenType::EnumVariant(_) => "an enum variant".to_string(),
            TokenType::Comment(_) => "a comment".to_string(),
            TokenType::IfDeclaration => "the 'if' keyword".to_string(),
            TokenType::ElseDeclaration => "the 'else' keyword".to_string(),
            TokenType::ImportKeyword => "the 'import' keyword".to_string(),
            TokenType::ImportDangerousKeyword => "the 'import_dangerous' keyword".to_string(),
            TokenType::SandboxStart => "the start of a sandboxed import inclusion".to_string(),
            TokenType::SandboxEnd => "the end of a sandboxed import inclusion".to_string(),
            TokenType::ParallelStart => "the 'p' (parallel) keyword".to_string(),
            TokenType::ParallelEnd => "the '/p' (end parallel) keyword".to_string(),
            TokenType::ConcurrentStart => "the 'c' (concurrent) keyword".to_string(),
            TokenType::ConcurrentEnd => "the '/c' (end concurrent) keyword".to_string(),
            TokenType::MapDeclaration => "the 'map' keyword".to_string(),
            TokenType::FilterDeclaration => "the 'filter' keyword".to_string(),
            TokenType::ReduceDeclaration => "the 'reduce' keyword".to_string(),
            TokenType::ScanDeclaration => "the 'scan' keyword".to_string(),
            TokenType::EachDeclaration => "the 'each' keyword".to_string(),
            TokenType::FindDeclaration => "the 'find' keyword".to_string(),
            TokenType::AllDeclaration => "the 'all' keyword".to_string(),
            TokenType::AnyDeclaration => "the 'any' keyword".to_string(),
            TokenType::ForeverKeyword => "the 'forever' keyword".to_string(),
            TokenType::InKeyword => "the 'in' keyword".to_string(),
            TokenType::FromKeyword => "the 'from' keyword".to_string(),
            TokenType::Return => "the 'r' (return) keyword".to_string(),
            TokenType::Yield => "the 'y' (yield) keyword".to_string(),
            TokenType::LexerError(_) => "an invalid token".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Operation {
    Add, // "+"
    Sub, // "-"
    Mul, // "*"
    Div, // "/"
    Mod, // "%"
    Eq,  // "=="
    Ne,  // "!="
    Lt,  // "<"
    Lte, // "<="
    Gt,  // ">"
    Gte, // ">="
    // unary operations
    And, // "&&"
    Or,  // "||"
    Not, // "!"
    Neg, // "-"
}

impl Operation {
    pub fn precedence(&self) -> u8 {
        match self {
            Operation::Or => 0,
            Operation::And => 1,
            Operation::Eq | Operation::Ne => 2,
            Operation::Lt | Operation::Lte | Operation::Gt | Operation::Gte => 3,
            Operation::Add | Operation::Sub => 4,
            Operation::Mul | Operation::Div | Operation::Mod => 5,
            Operation::Not | Operation::Neg => 6, // Highest precedence for unary operators
        }
    }

    pub fn is_unary(&self) -> bool {
        matches!(self, Operation::Not | Operation::Neg)
    }
}

impl Display for Operation {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Operation::Add => write!(f, "+"),
            Operation::Sub => write!(f, "-"),
            Operation::Mul => write!(f, "*"),
            Operation::Div => write!(f, "/"),
            Operation::Mod => write!(f, "%"),
            Operation::Eq => write!(f, "=="),
            Operation::Ne => write!(f, "!="),
            Operation::Lt => write!(f, "<"),
            Operation::Lte => write!(f, "<="),
            Operation::Gt => write!(f, ">"),
            Operation::Gte => write!(f, ">="),
            Operation::Not => write!(f, "!"),
            Operation::Neg => write!(f, "-"),
            Operation::And => write!(f, "&&"),
            Operation::Or => write!(f, "||"),
        }
    }
}

fn advance(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> Option<char> {
    // this is so strings and comments do not mess up the line and column count
    if let Some(c) = chars.next() {
        if c == '\n' {
            state.line += 1;
            state.column = 1;
        } else {
            state.column += 1;
        }
        Some(c)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexerOutput {
    pub token_type: TokenType,
    pub start_line: usize,
    pub end_line: usize,
    pub start_column: usize,
    pub end_column: usize,
}

pub struct LexerState {
    pub line: usize,
    pub column: usize,
}

/// All lexer errors in a token stream, in source order. The lexer embeds
/// errors as LexerError tokens rather than failing; callers use this to
/// report every lex error eagerly instead of hitting them one at a time
/// during parsing.
pub fn collect_lexer_errors(tokens: &[Token]) -> Vec<crate::common::CodeError> {
    let mut errors = Vec::new();
    for token in tokens {
        match &token.token_type {
            TokenType::LexerError(message) => errors.push(crate::common::CodeError { help: None, message: message.clone(), code_span: token.code_span.clone() }),
            // Compound tokens carry nested token streams whose errors must surface too
            TokenType::FunctionSignature(inner) => errors.extend(collect_lexer_errors(inner)),
            _ => {}
        }
    }
    errors
}

/// What the lexer does when it meets import() or import_dangerous(). Compiling a
/// program splices the included file in. A tool that displays one file must
/// not, since the spliced tokens carry the other file's line and column
/// numbers and would paint the wrong characters.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ImportMode {
    Expand,
    Keep,
}

/// A lexed program: the token stream with every import spliced in, plus the
/// source map that says which file each program line belongs to. Imported
/// files are numbered into their own line ranges past the end of the entry
/// file, so no two files' spans can collide and every error can name the
/// file it is actually in.
pub struct LexedProgram {
    pub tokens: Vec<Token>,
    pub source_map: SourceMap,
}

/// Everything one lex of a program accumulates across imports. The stack
/// catches true cycles; the spliced set makes imports idempotent, so a diamond
/// (two files importing the same third) splices it once instead of failing on
/// duplicate definitions.
struct ImportCtx {
    /// Files currently being spliced, importer-first. Importing one again is a cycle.
    stack: HashSet<PathBuf>,
    /// Files already spliced once, with the capability they were spliced under
    /// (true = sandboxed import()). Importing one again splices nothing, and
    /// importing it under the other capability is an error.
    spliced: HashMap<PathBuf, bool>,
    /// The next free program line, so every imported file gets its own range.
    next_base: usize,
    map: SourceMap,
}

pub fn lexer(input: &str) -> Vec<Token> {
    lex_program(input, None).tokens
}

pub fn lexer_with_context(input: &str, current_file: Option<&Path>) -> Vec<Token> {
    lex_program(input, current_file).tokens
}

/// Lex a whole program starting from its entry file, keeping the source map.
/// This is the entry point for anything that reports errors: the map is what
/// turns a program line back into a file and that file's own line number.
pub fn lex_program(input: &str, entry_path: Option<&Path>) -> LexedProgram {
    let entry_lines = input.lines().count();
    let display_path = entry_path.map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| "<buffer>".to_string());
    let mut ctx = ImportCtx {
        stack: HashSet::new(),
        spliced: HashMap::new(),
        next_base: entry_lines,
        map: SourceMap {
            files: vec![SourceFile { path: display_path, base: 0, lines: entry_lines, content: input.to_string(), imported_at: 0 }],
        },
    };
    // The entry is on the stack for the whole lex, so a file importing its
    // own entry is reported as the cycle it is.
    if let Some(path) = entry_path {
        if let Ok(canonical) = path.canonicalize() {
            ctx.stack.insert(canonical.clone());
            ctx.spliced.insert(canonical, false);
        }
    }
    let mut state = LexerState { line: 1, column: 1 };
    let tokens = lexer_inner(input, &mut state, entry_path, &mut ctx, ImportMode::Expand, false);
    LexedProgram { tokens, source_map: ctx.map }
}

/// Lex one file's own text: import lines stay ordinary tokens, so every span
/// belongs to the text that was handed in. For highlighting and formatting.
pub fn lexer_without_imports(input: &str) -> Vec<Token> {
    let mut state = LexerState { line: 1, column: 1 };
    let mut ctx = ImportCtx { stack: HashSet::new(), spliced: HashMap::new(), next_base: 0, map: SourceMap::default() };
    lexer_inner(input, &mut state, None, &mut ctx, ImportMode::Keep, false)
}

fn lexer_inner(input: &str, state: &mut LexerState, current_file: Option<&Path>, ctx: &mut ImportCtx, import_mode: ImportMode, in_sandbox: bool) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();

    // The version version line and any shebang above it are addressed to the launcher, not
    // to the language, so they never become tokens. Skipping them here rather
    // than in each caller means an imported file may carry one too. The line
    // count moves with the skip, so every span that follows still names the
    // line it came from.
    let (input, header_lines) = crate::version_line::strip_header(input);
    state.line += header_lines as usize;

    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            _ if c.is_whitespace() => {
                if c == '\n' {
                    state.line += 1;
                    state.column = 1;
                } else {
                    state.column += 1;
                }
                chars.next();
            }
            _ if is_parallel_end(&mut chars) => {
                let lexer_output = lex_parallel_end(&mut chars, state);
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
            }
            _ if is_concurrent_end(&mut chars) => {
                let lexer_output = lex_concurrent_end(&mut chars, state);
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
            }
            _ if is_comment(&mut chars) => {
                lex_comment(&mut chars, state);
            }

            _ if is_function_signature(&mut chars) => {
                let lexer_output = lex_function_signature(&mut chars, state);
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
            }

            '[' => {
                let start_line = state.line;
                let start_column = state.column;
                chars.next(); // consume '['
                state.column += 1;
                tokens.push(Token { token_type: TokenType::ArrayOpen, code_span: CodeSpan { start_line, end_line: state.line, start_column, end_column: state.column } });
            }
            ']' => {
                let start_line = state.line;
                let start_column = state.column;
                chars.next(); // consume ']'
                state.column += 1;
                tokens.push(Token { token_type: TokenType::ArrayClose, code_span: CodeSpan { start_line, end_line: state.line, start_column, end_column: state.column } });
            }
            '`' => {
                let lexer_output: LexerOutput = lex_string_literal(&mut chars, state);
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
            }

            _ if is_tagged_string_literal(&chars) => {
                let lexer_output: LexerOutput = lex_tagged_string_literal(&mut chars, state);
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
            }


            _ if is_enum_declaration(&mut chars) => {
                let lexer_output: LexerOutput = lex_enum_delcaration(&mut chars, state);
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
            }

            _ if is_struct_declaration(&mut chars) => {
                let lexer_output: LexerOutput = lex_struct_declaration(&mut chars, state);
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
            }

            // Struct instantiation now handled by parser - struct names are just identifiers

            _ if is_enum_variant(&mut chars) => {
                let lexer_output = lex_enum_variant(&mut chars, state);
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
            }

            _ if is_identifier_or_keyword(c) => {
                let lexer_output = lex_identifier_or_keyword(&mut chars, state);
                
                // Check if this is an import or import_dangerous keyword followed
                // by a file path. Both splice the included file's tokens in place.
                // import additionally wraps the splice in SandboxStart and
                // SandboxEnd markers so later stages know the code is untrusted.
                if import_mode == ImportMode::Expand && (lexer_output.token_type == TokenType::ImportKeyword || lexer_output.token_type == TokenType::ImportDangerousKeyword) {
                    let sandboxed = lexer_output.token_type == TokenType::ImportKeyword;
                    // Inside a sandboxed import the whole subtree has to stay
                    // sandboxed, or the sandbox would prove nothing.
                    let forbidden = in_sandbox && lexer_output.token_type == TokenType::ImportDangerousKeyword;
                    // Skip whitespace
                    while let Some(&c) = chars.peek() {
                        if c.is_whitespace() {
                            if c == '\n' {
                                state.line += 1;
                                state.column = 1;
                            } else {
                                state.column += 1;
                            }
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    
                    // Check for opening parenthesis
                    if chars.peek() == Some(&'(') {
                        chars.next();
                        state.column += 1;
                        
                        // Skip whitespace
                        while let Some(&c) = chars.peek() {
                            if c.is_whitespace() {
                                if c == '\n' {
                                    state.line += 1;
                                    state.column = 1;
                                } else {
                                    state.column += 1;
                                }
                                chars.next();
                            } else {
                                break;
                            }
                        }
                        
                        // Check for string literal
                        if chars.peek() == Some(&'`') {
                            let string_output = lex_string_literal(&mut chars, state);
                            
                            if let TokenType::StringLiteral { value: filepath, .. } = string_output.token_type {
                                // Skip whitespace
                                while let Some(&c) = chars.peek() {
                                    if c.is_whitespace() {
                                        if c == '\n' {
                                            state.line += 1;
                                            state.column = 1;
                                        } else {
                                            state.column += 1;
                                        }
                                        chars.next();
                                    } else {
                                        break;
                                    }
                                }
                                
                                // Check for closing parenthesis
                                if chars.peek() == Some(&')') {
                                    chars.next();
                                    state.column += 1;
                                    
                                    // Now handle the file inclusion
                                    if forbidden {
                                        tokens.push(Token {
                                            token_type: TokenType::LexerError("import_dangerous is not allowed inside a sandboxed import: a file brought in with import() can only use import() itself".to_string()),
                                            code_span: CodeSpan {
                                                start_line: lexer_output.start_line,
                                                end_line: state.line,
                                                start_column: lexer_output.start_column,
                                                end_column: state.column
                                            },
                                        });
                                    } else {
                                    match handle_import(&filepath, current_file, ctx, state, sandboxed || in_sandbox) {
                                        Ok(Some(inserted_tokens)) => {
                                            // An import_dangerous nested inside an imported
                                            // file already lands between the outer markers,
                                            // so the whole subtree is sandboxed either way.
                                            if sandboxed {
                                                tokens.push(Token {
                                                    token_type: TokenType::SandboxStart,
                                                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                                                });
                                            }
                                            tokens.extend(inserted_tokens);
                                            if sandboxed {
                                                tokens.push(Token {
                                                    token_type: TokenType::SandboxEnd,
                                                    code_span: CodeSpan { start_line: state.line, end_line: state.line, start_column: state.column, end_column: state.column },
                                                });
                                            }
                                        }
                                        // Already spliced by an earlier import: an import
                                        // is idempotent, so there is nothing to insert.
                                        Ok(None) => {}
                                        Err(error) => {
                                            tokens.push(Token {
                                                token_type: TokenType::LexerError(format!("Import error: {}", error)),
                                                code_span: CodeSpan {
                                                    start_line: lexer_output.start_line,
                                                    end_line: state.line,
                                                    start_column: lexer_output.start_column,
                                                    end_column: state.column
                                                },
                                            });
                                        }
                                    }
                                    }
                                } else {
                                    // Pointed at the import itself rather than
                                    // at wherever reading stopped: an import
                                    // whose ')' is missing runs to the end of
                                    // the file, and an error reported there is
                                    // an error on a line that does not exist.
                                    tokens.push(Token {
                                        token_type: TokenType::LexerError("This import is missing the ')' that closes it".to_string()),
                                        code_span: CodeSpan {
                                            start_line: lexer_output.start_line,
                                            end_line: lexer_output.end_line,
                                            start_column: lexer_output.start_column,
                                            end_column: lexer_output.end_column,
                                        },
                                    });
                                }
                            } else {
                                tokens.push(Token {
                                    token_type: string_output.token_type,
                                    code_span: CodeSpan { start_line: string_output.start_line, end_line: string_output.end_line, start_column: string_output.start_column, end_column: string_output.end_column },
                                });
                            }
                        } else {
                            // Not an import statement, push the keyword token
                            tokens.push(Token {
                                token_type: lexer_output.token_type,
                                code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                            });
                            // Push the open paren
                            tokens.push(Token {
                                token_type: TokenType::ParenthesisOpen,
                                code_span: CodeSpan { start_line: state.line, end_line: state.line, start_column: state.column - 1, end_column: state.column },
                            });
                        }
                    } else {
                        // Not followed by '(', just push the keyword token
                        tokens.push(Token {
                            token_type: lexer_output.token_type,
                            code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                        });
                    }
                } else {
                    tokens.push(Token {
                        token_type: lexer_output.token_type,
                        code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                    });
                }
            }
            _ if is_number(&mut chars) => {
                let lexer_output: LexerOutput = lex_number(&mut chars, state);
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
            }
            _ if is_double_character_token(&mut chars) => {
                let lexer_output: LexerOutput = lex_double_character_token(&mut chars, state);
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
            }
            _ if is_single_character_token(&mut chars) => {
                let lexer_output: LexerOutput = lex_single_character_token(&mut chars, state);
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
            }
            _ => {
                tokens.push(Token {
                    token_type: TokenType::LexerError(format!("Unrecognized character: {}", c)),
                    code_span: CodeSpan { start_line: state.line, end_line: state.line, start_column: state.column, end_column: state.column },
                });
                chars.next();
            }
        }
    }

    tokens
}

pub fn is_in_alphabet(c: char) -> bool {
    ALPHABET.contains(c)
}

pub fn is_in_alphabet_lowercase(c: char) -> bool {
    ALPHABET_LOWERCASE.contains(c)
}

pub fn is_in_alphabet_or_number(c: char) -> bool {
    ALPHABET_AND_NUMBERS.contains(c)
}

pub fn is_in_alphabet_lowercase_or_number(c: char) -> bool {
    ALPHABET_LOWERCASE_AND_NUMBERS.contains(c)
}

pub fn is_in_alphabet_upppercase_or_number(c: char) -> bool {
    ALPHABET_UPPERCASE_AND_NUMBERS.contains(c)
}

pub fn is_alphabet_uppercase(c: char) -> bool {
    ALPHABET_UPPERCASE.contains(c)
}


// fn lex_array(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
//     let start_line = state.line;
//     let start_column = state.column;
//
//     let mut elements = Vec::new();
//
//     advance(chars, state); // Consume '['
//
//     loop {
//         // Skip whitespace
//         while let Some(&c) = chars.peek() {
//             if c.is_whitespace() {
//                 advance(chars, state);
//             } else {
//                 break;
//             }
//         }
//
//         // Check for array end
//         if chars.peek() == Some(&']') {
//             advance(chars, state); // Consume ']'
//             break;
//         }
//
//         // Lex array element
//         if is_array(chars) {
//             let nested_array = lex_array(chars, state);
//             elements.push(Token {
//                 token_type: nested_array.token_type,
//                 code_span: CodeSpan { start_line: nested_array.start_line, end_line: nested_array.end_line, start_column: nested_array.start_column, end_column: nested_array.end_column },
//             });
//         } else {
//             let element = lex_value(chars, state);
//             elements.push(Token {
//                 token_type: element.token_type,
//                 code_span: CodeSpan { start_line: element.start_line, end_line: element.end_line, start_column: element.start_column, end_column: element.end_column },
//             });
//         }
//
//         // Skip whitespace
//         while let Some(&c) = chars.peek() {
//             if c.is_whitespace() {
//                 advance(chars, state);
//             } else {
//                 break;
//             }
//         }
//
//         // Check for comma or array end
//         match chars.peek() {
//             Some(&',') => {
//                 advance(chars, state); // Consume ','
//             }
//             Some(&']') => continue, // Will be handled at the start of the loop
//             _ => return LexerOutput { token_type: TokenType::LexerError("Expected ',' or ']' in array".to_string()), start_line, start_column, end_line: state.line, end_column: state.column },
//         }
//     }
//
//     LexerOutput { token_type: TokenType::Array(elements), start_line, start_column, end_line: state.line, end_column: state.column }
// }

fn is_function_signature(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    let mut lookahead = chars.clone();
    // Check if 'f' followed by whitespace and then an identifier (function name)
    if lookahead.next() == Some('f') {
        // Must have at least one whitespace
        if !matches!(lookahead.peek(), Some(&c) if c.is_whitespace()) {
            return false;
        }
        // Skip whitespace
        while matches!(lookahead.peek(), Some(&c) if c.is_whitespace()) {
            lookahead.next();
        }
        // Check if followed by an identifier (letter or underscore)
        matches!(lookahead.peek(), Some(&c) if c.is_alphabetic() || c == '_')
    } else {
        false
    }
}

fn lex_function_signature(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;

    let mut tokens: Vec<Token> = vec![];

    advance(chars, state); // skip 'f'

    // eat whitespace

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            advance(chars, state);
        } else {
            break;
        }
    }

    // get the name of the function
    let function_name = lex_identifier_or_keyword(chars, state);

    tokens.push(Token {
        token_type: match function_name.token_type {
            TokenType::Identifier(s) => TokenType::FunctionName(s),
            // Reserved words and other keywords already lex to a specific error
            // or token; surface that instead of a generic complaint
            TokenType::LexerError(message) => TokenType::LexerError(message),
            other => TokenType::LexerError(format!("Expected a function name here, but found {}", other.describe())),
        },
        code_span: CodeSpan { start_line: function_name.start_line, end_line: function_name.end_line, start_column: function_name.start_column, end_column: function_name.end_column },
    });

    // Parse parameters
    if chars.peek() == Some(&'(') {
        advance(chars, state); // skip '('

        // Parse parameter(s)
        while let Some(&c) = chars.peek() {
            if c == ')' {
                break;
            }
            // A parameter that consumes nothing would spin here forever, so
            // report the offending character instead of hanging the compiler.
            let position_before = (state.line, state.column);
            let param_name = lex_identifier_only(chars, state);
            tokens.push(Token {
                token_type: param_name.token_type,
                code_span: CodeSpan { start_line: param_name.start_line, end_line: param_name.end_line, start_column: param_name.start_column, end_column: param_name.end_column },
            });

            // Parse parameter type
            if chars.peek() == Some(&':') {
                let param_type = lex_type_system_type(chars, state);
                tokens.push(Token {
                    token_type: param_type.token_type,
                    code_span: CodeSpan { start_line: param_type.start_line, end_line: param_type.end_line, start_column: param_type.start_column, end_column: param_type.end_column },
                });
            }

            // get next params repeatedly
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    advance(chars, state);
                } else {
                    break;
                }
            }

            // Consume comma
            if chars.peek() == Some(&',') {
                tokens.push(Token { token_type: TokenType::Comma, code_span: CodeSpan { start_line: state.line, end_line: state.line, start_column: state.column, end_column: state.column + 1 } });
                advance(chars, state);
            }

            // eat whitespace
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    advance(chars, state);
                } else {
                    break;
                }
            }

            if (state.line, state.column) == position_before {
                let found = chars.peek().copied().unwrap_or(' ');
                // Discard the rest of the signature: without this the outer
                // lexer resumes mid-parameter-list and reports a second error
                // for the same mistake.
                while let Some(&c) = chars.peek() {
                    advance(chars, state);
                    if c == ')' {
                        break;
                    }
                }
                return LexerOutput {
                    token_type: TokenType::LexerError(format!("Expected a parameter name or ')' here, but found '{}'", found)),
                    start_line,
                    start_column,
                    end_line: state.line,
                    end_column: state.column,
                };
            }
        }

        // Consume closing parenthesis
        if chars.peek() == Some(&')') {
            advance(chars, state);
        }
    }

    // Parse the function's return type if present
    if chars.peek() == Some(&':') {
        let return_type = lex_type_system_type(chars, state);
        if let TokenType::TypeDeclaration(t) = return_type.token_type {
            tokens.push(Token {
                token_type: TokenType::FunctionReturnTypeDeclaration(t),
                code_span: CodeSpan { start_line: return_type.start_line, end_line: return_type.end_line, start_column: return_type.start_column, end_column: return_type.end_column },
            });
        } else {
            return LexerOutput {
                token_type: TokenType::LexerError("Expected type declaration for function return type".to_string()),
                start_line: state.line,
                start_column: state.column,
                end_line: state.line,
                end_column: state.column,
            };
        }
    }

    LexerOutput { token_type: TokenType::FunctionSignature(tokens), start_line, start_column, end_line: state.line, end_column: state.column }
}

fn is_comment(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    let mut lookahead = chars.clone();
    lookahead.next() == Some('/') && lookahead.next() == Some('/')
}

fn is_parallel_end(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    closes_a_block(chars, 'p')
}

fn is_concurrent_end(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    closes_a_block(chars, 'c')
}

/// Whether what comes next is `/p` or `/c`, the tokens that close a parallel
/// or a concurrent block, rather than a division sign in front of a name.
///
/// The letter has to end the token. Reading `/p` greedily meant that
/// `total_value /price_value`, division written the way most languages let
/// you write it, became the end of a parallel block followed by a variable
/// called `rice_value`, and the error that came out named neither of them.
fn closes_a_block(chars: &std::iter::Peekable<std::str::Chars>, letter: char) -> bool {
    let mut lookahead = chars.clone();
    if lookahead.next() != Some('/') {
        return false;
    }
    if lookahead.next() != Some(letter) {
        return false;
    }
    !lookahead.next().map_or(false, |next| is_in_alphabet_or_number(next) || next == '_')
}

fn lex_parallel_end(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;

    // Consume "/p"
    chars.next(); // consume '/'
    state.column += 1;
    chars.next(); // consume 'p'
    state.column += 1;

    LexerOutput { token_type: TokenType::ParallelEnd, start_line, start_column, end_line: state.line, end_column: state.column }
}

fn lex_concurrent_end(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;

    // Consume "/c"
    chars.next(); // consume '/'
    state.column += 1;
    chars.next(); // consume 'c'
    state.column += 1;

    LexerOutput { token_type: TokenType::ConcurrentEnd, start_line, start_column, end_line: state.line, end_column: state.column }
}

fn lex_comment(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;

    // Consume the two forward slashes
    advance(chars, state);
    advance(chars, state);

    let mut comment = String::new();

    // Consume the rest of the line
    while let Some(&c) = chars.peek() {
        if c == '\n' {
            break;
        }
        comment.push(c);
        advance(chars, state); // Consume the newline
    }

    LexerOutput { token_type: TokenType::Comment(comment), start_line, start_column, end_line: state.line, end_column: state.column }
}

fn is_single_character_token(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    let mut lookahead = chars.clone();

    match lookahead.next() {
        Some(c) => match c {
            '(' | ')' | ';' | '{' | '}' | ',' | '.' | ':' | '!' | '+' | '-' | '*' | '/' | '%' | '=' | '<' | '>' => {
                // Check if it's followed by a space, or by something it's allowed to be beside or end of input
                match lookahead.next() {
                    Some(next_char) => {
                        next_char.is_whitespace()
                            || is_in_alphabet_or_number(next_char)
                            || next_char == ';'
                            || next_char == ','
                            || next_char == '.'
                            || next_char == '('
                            || next_char == ')'
                            || next_char == '{'
                            || next_char == '}'
                            || next_char == ':'
                            || next_char == '!'
                            || next_char == '+'
                            || next_char == '-'
                            || next_char == '*'
                            || next_char == '/'
                            || next_char == '`'
                            || next_char == '['
                            || next_char == ']'
                            // Nested hashmap types close with '>>' (h<s,h<s,s>>).
                            // Nail has no shift operators, so a '>' or '<' beside
                            // another is always type syntax.
                            || next_char == '>'
                            || next_char == '<'
                            || next_char == '\n'
                    }
                    None => true, // End of input is fine
                }
            }
            _ => false,
        },
        None => false,
    }
}

fn lex_single_character_token(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;

    let c = advance(chars, state).expect("This should be the operator");

    let token_type = match c {
        '(' => TokenType::ParenthesisOpen,
        ')' => TokenType::ParenthesisClose,
        ';' => TokenType::EndStatementOrExpression,
        '{' => TokenType::BlockOpen,
        '}' => TokenType::BlockClose,
        ',' => TokenType::Comma,
        '.' => TokenType::Dot,
        ':' => TokenType::Colon,
        '=' => TokenType::Assignment,
        '!' => TokenType::Operator(Operation::Not),
        '+' => TokenType::Operator(Operation::Add),
        '-' => TokenType::Operator(Operation::Sub),
        '*' => TokenType::Operator(Operation::Mul),
        '/' => TokenType::Operator(Operation::Div),
        '%' => TokenType::Operator(Operation::Mod),
        '<' => TokenType::Operator(Operation::Lt),
        '>' => TokenType::Operator(Operation::Gt),
        _ => panic!("Unrecognized operator: {}", c),
    };

    LexerOutput { token_type, start_line, start_column, end_line: state.line, end_column: state.column }
}

// A keyword only opens a declaration when it stands as a whole word, so an
// identifier that merely starts with one - `structure_of`, `enumerate` - stays
// an identifier.
fn starts_with_keyword(chars: &std::iter::Peekable<std::str::Chars>, keyword: &str) -> bool {
    let mut lookahead = chars.clone();
    for expected in keyword.chars() {
        if lookahead.next() != Some(expected) {
            return false;
        }
    }
    match lookahead.next() {
        Some(next) => !(is_in_alphabet_or_number(next) || next == '_'),
        None => true,
    }
}

fn is_struct_declaration(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    return starts_with_keyword(chars, "struct");
}

fn lex_struct_declaration(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;

    // Skip 'struct'
    advance(chars, state); // Skip 's'
    advance(chars, state); // Skip 't'
    advance(chars, state); // Skip 'r'
    advance(chars, state); // Skip 'u'
    advance(chars, state); // Skip 'c'
    advance(chars, state); // Skip 't'

    // Skip whitespace
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            advance(chars, state);
        } else {
            break;
        }
    }

    // Parse struct name
    let mut struct_name = String::new();
    while let Some(&c) = chars.peek() {
        if is_in_alphabet_or_number(c) || c == '_' {
            struct_name.push(c);
            advance(chars, state);
        } else {
            break;
        }
    }

    // Skip whitespace
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            advance(chars, state);
        } else {
            break;
        }
    }

    // A struct is a type, and a type without a name cannot be written down
    // anywhere else, so there is nothing it could be for. Leaving it unnamed
    // reached rustc as `struct  {`.
    if struct_name.is_empty() {
        return LexerOutput { token_type: TokenType::LexerError("This struct has no name, and a type has to have one to be used".to_string()), start_line, start_column, end_line: state.line, end_column: state.column };
    }

    // Check for opening brace
    if chars.peek() != Some(&'{') {
        return LexerOutput { token_type: TokenType::LexerError("Expected '{' after struct name".to_string()), start_line, start_column, end_line: state.line, end_column: state.column };
    }
    advance(chars, state); // consume '{'

    // Parse struct fields
    let mut fields = Vec::new();
    loop {
        // Skip whitespace
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                advance(chars, state);
            } else {
                break;
            }
        }

        // Check for closing brace
        if chars.peek() == Some(&'}') {
            advance(chars, state);
            break;
        }

        // Parse field name
        let mut field_name = String::new();
        while let Some(&c) = chars.peek() {
            if is_in_alphabet_or_number(c) || c == '_' {
                field_name.push(c);
                advance(chars, state);
            } else {
                break;
            }
        }

        // Validate field name
        if let Some(error) = validate_identifier_name(&field_name) {
            return LexerOutput { token_type: TokenType::LexerError(error), start_line, start_column, end_line: state.line, end_column: state.column };
        }

        // Parse field type
        let field_type = lex_type_system_type(chars, state);

        fields.push(StructDeclarationDataField {
            name: field_name,
            data_type: match field_type {
                LexerOutput { token_type: TokenType::TypeDeclaration(t), .. } => t,
                _ => {
                    return LexerOutput {
                        token_type: TokenType::LexerError("Expected type declaration after field name in struct".to_string()),
                        start_line,
                        start_column,
                        end_line: state.line,
                        end_column: state.column,
                    }
                }
            },
        });

        // Skip whitespace
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                advance(chars, state);
            } else {
                break;
            }
        }

        // Check for comma or closing brace
        if chars.peek() == Some(&',') {
            advance(chars, state);
        } else if chars.peek() != Some(&'}') {
            return LexerOutput { token_type: TokenType::LexerError("Expected ',' or '}' after field type".to_string()), start_line, start_column, end_line: state.line, end_column: state.column };
        }
    }

    LexerOutput { token_type: TokenType::StructDeclaration(StructDeclarationData { name: struct_name, fields }), start_line, start_column, end_line: state.line, end_column: state.column }
}

fn is_enum_declaration(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    return starts_with_keyword(chars, "enum");
}

fn lex_enum_delcaration(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;

    // Skip 'enum'
    for _ in 0..4 {
        advance(chars, state);
    }

    // Skip whitespace
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            advance(chars, state);
        } else {
            break;
        }
    }

    // Parse enum name
    let mut enum_name = String::new();
    while let Some(&c) = chars.peek() {
        if is_in_alphabet_or_number(c) || c == '_' {
            enum_name.push(c);
            advance(chars, state);
        } else {
            break;
        }
    }

    // Skip whitespace
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            advance(chars, state);
        } else {
            break;
        }
    }

    // Same as a struct: a type with no name cannot be named anywhere else.
    if enum_name.is_empty() {
        return LexerOutput { token_type: TokenType::LexerError("This enum has no name, and a type has to have one to be used".to_string()), start_line, start_column, end_line: state.line, end_column: state.column };
    }

    // Check for opening brace
    if chars.peek() != Some(&'{') {
        return LexerOutput { token_type: TokenType::LexerError("Expected '{' after enum name".to_string()), start_line, start_column, end_line: state.line, end_column: state.column };
    }
    advance(chars, state); // consume '{'

    // Parse enum variants
    let mut variants = Vec::new();
    loop {
        // Skip whitespace
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                advance(chars, state);
            } else {
                break;
            }
        }

        // Check for closing brace
        if chars.peek() == Some(&'}') {
            advance(chars, state);
            break;
        }

        // Parse variant name
        let mut variant_name = String::new();
        let variant_start_column = state.column;
        while let Some(&c) = chars.peek() {
            if is_in_alphabet_or_number(c) || c == '_' {
                variant_name.push(c);
                advance(chars, state);
            } else {
                break;
            }
        }

        // Validate variant name
        if let Some(error) = validate_identifier_name(&variant_name) {
            return LexerOutput { token_type: TokenType::LexerError(error), start_line, start_column, end_line: state.line, end_column: state.column };
        }

        variants.push(Token {
            token_type: TokenType::EnumVariant(EnumVariantData { name: enum_name.clone(), variant: variant_name }),
            code_span: CodeSpan { start_line: state.line, end_line: state.line, start_column: variant_start_column, end_column: state.column },
        });

        // Skip whitespace
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                advance(chars, state);
            } else {
                break;
            }
        }

        // Check for comma or closing brace
        if chars.peek() == Some(&',') {
            advance(chars, state);
        } else if chars.peek() != Some(&'}') {
            return LexerOutput { token_type: TokenType::LexerError("Expected ',' or '}' after enum variant".to_string()), start_line, start_column, end_line: state.line, end_column: state.column };
        }
    }

    LexerOutput { token_type: TokenType::EnumDeclaration(EnumDeclarationData { name: enum_name, variants }), start_line, start_column, end_line: state.line, end_column: state.column }
}

fn is_identifier_or_keyword(c: char) -> bool {
    // ensure first character is not digit.
    if c.is_digit(10) {
        return false;
    }
    is_in_alphabet_or_number(c) || c == '_'
}

fn validate_identifier_name(identifier: &str) -> Option<String> {
    // Every single-letter identifier is refused, except 'e' (the error
    // constructor, as in r e(`message`)) and the type letters.
    let valid_single_letters = ["e", "i", "f", "s", "b", "v", "a", "h"];
    if identifier.len() == 1 && identifier.chars().all(|c| c.is_alphabetic()) && !valid_single_letters.contains(&identifier) {
        Some("Variable name too short. Must use descriptive names.".to_string())
    } else {
        None
    }
}

fn lex_identifier_or_keyword(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;
    let mut identifier = String::new();

    while let Some(&c) = chars.peek() {
        if is_in_alphabet_or_number(c) || c == '_' {
            identifier.push(c);
            advance(chars, state);
        } else {
            break;
        }
    }

    let token_type = match identifier.as_str() {
        "r" => {
            // 'r' is only Return if followed by whitespace
            let mut lookahead = chars.clone();
            if let Some(&next_char) = lookahead.peek() {
                if next_char.is_whitespace() {
                    TokenType::Return
                } else {
                    // Validate before treating as identifier
                    if let Some(error) = validate_identifier_name(&identifier) {
                        TokenType::LexerError(error)
                    } else {
                        TokenType::Identifier(identifier)
                    }
                }
            } else {
                if let Some(error) = validate_identifier_name(&identifier) {
                    TokenType::LexerError(error)
                } else {
                    TokenType::Identifier(identifier)
                }
            }
        }
        "y" => {
            // 'y' is only Yield if followed by whitespace
            let mut lookahead = chars.clone();
            if let Some(&next_char) = lookahead.peek() {
                if next_char.is_whitespace() {
                    TokenType::Yield
                } else {
                    // Validate before treating as identifier
                    if let Some(error) = validate_identifier_name(&identifier) {
                        TokenType::LexerError(error)
                    } else {
                        TokenType::Identifier(identifier)
                    }
                }
            } else {
                if let Some(error) = validate_identifier_name(&identifier) {
                    TokenType::LexerError(error)
                } else {
                    TokenType::Identifier(identifier)
                }
            }
        }
        "if" => TokenType::IfDeclaration,
        "else" => TokenType::ElseDeclaration,
        "true" => TokenType::BooleanLiteral(true),
        "false" => TokenType::BooleanLiteral(false),

        // rust keywords
        "main" => TokenType::LexerError("'main' is a reserved keyword and cannot be used as an identifier".to_string()),
        "self" => TokenType::LexerError("'self' is a reserved keyword and cannot be used as an identifier".to_string()),
        "super" => TokenType::LexerError("'super' is a reserved keyword and cannot be used as an identifier".to_string()),
        "crate" => TokenType::LexerError("'crate' is a reserved keyword and cannot be used as an identifier".to_string()),
        "mod" => TokenType::LexerError("'mod' is a reserved keyword and cannot be used as an identifier".to_string()),
        "pub" => TokenType::LexerError("'pub' is a reserved keyword and cannot be used as an identifier".to_string()),
        "use" => TokenType::LexerError("'use' is a reserved keyword and cannot be used as an identifier".to_string()),
        "fn" => TokenType::LexerError("'fn' is a reserved keyword and cannot be used as an identifier".to_string()),
        "let" => TokenType::LexerError("'let' is a reserved keyword and cannot be used as an identifier".to_string()),
        "import" => TokenType::ImportKeyword,
        "import_dangerous" => TokenType::ImportDangerousKeyword,
        "mut" => TokenType::LexerError("'mut' is a reserved keyword and cannot be used as an identifier".to_string()),
        "const" => TokenType::LexerError("'const' is a reserved keyword and cannot be used as an identifier".to_string()),
        "static" => TokenType::LexerError("'static' is a reserved keyword and cannot be used as an identifier".to_string()),
        "struct" => TokenType::LexerError("'struct' is a reserved keyword and cannot be used as an identifier".to_string()),
        "enum" => TokenType::LexerError("'enum' is a reserved keyword and cannot be used as an identifier".to_string()),
        "trait" => TokenType::LexerError("'trait' is a reserved keyword and cannot be used as an identifier".to_string()),
        "impl" => TokenType::LexerError("'impl' is a reserved keyword and cannot be used as an identifier".to_string()),
        "type" => TokenType::LexerError("'type' is a reserved keyword and cannot be used as an identifier".to_string()),
        "where" => TokenType::LexerError("'where' is a reserved keyword and cannot be used as an identifier".to_string()),
        "dyn" => TokenType::LexerError("'dyn' is a reserved keyword and cannot be used as an identifier".to_string()),
        "async" => TokenType::LexerError("'async' is a reserved keyword and cannot be used as an identifier".to_string()),
        "await" => TokenType::LexerError("'await' is a reserved keyword and cannot be used as an identifier".to_string()),
        "move" => TokenType::LexerError("'move' is a reserved keyword and cannot be used as an identifier".to_string()),
        "match" => TokenType::LexerError("'match' is a reserved keyword and cannot be used as an identifier".to_string()),
        "forever" => TokenType::ForeverKeyword,
        "loop" => TokenType::LexerError("Nail has no 'loop': a block that runs until the program ends is written forever { }, and a collection is walked with 'each'".to_string()),
        "spawn" => TokenType::LexerError("Nail has no 'spawn': nothing runs behind the program's back. Run things at once with c ... /c, which ends when all of them have ended".to_string()),
        "while" => TokenType::LexerError("Nail has no 'while' loop: walk a collection with 'each', accumulate with 'reduce', repeat until something changes with a function that calls itself, and run until the program ends with 'forever'".to_string()),
        "for" => TokenType::LexerError("Nail has no 'for' loop: walk a collection with 'each', and build one with 'map', 'filter', 'reduce' or 'scan'".to_string()),
        "map" => TokenType::MapDeclaration,
        "filter" => TokenType::FilterDeclaration,
        "reduce" => TokenType::ReduceDeclaration,
        "scan" => TokenType::ScanDeclaration,
        "each" => TokenType::EachDeclaration,
        "find" => TokenType::FindDeclaration,
        "all" => TokenType::AllDeclaration,
        "any" => TokenType::AnyDeclaration,
        "in" => TokenType::InKeyword,
        "from" => TokenType::FromKeyword,
        "break" => TokenType::LexerError("Nail has no 'break': a 'forever' block runs until the program ends, so walk a collection with 'each', leave a forever block inside a function with 'r', or repeat until something changes with a function that calls itself".to_string()),
        "continue" => TokenType::LexerError("Nail has no 'continue': choose the elements with 'filter' before the loop instead of skipping them inside it".to_string()),
        "return" => TokenType::LexerError("'return' is a reserved keyword and cannot be used as an identifier".to_string()),
        "yield" => TokenType::LexerError("'yield' is a reserved keyword and cannot be used as an identifier".to_string()),
        "ref" => TokenType::LexerError("'ref' is a reserved keyword and cannot be used as an identifier".to_string()),
        "as" => TokenType::LexerError("'as' is a reserved keyword and cannot be used as an identifier".to_string()),
        "extern" => TokenType::LexerError("'extern' is a reserved keyword and cannot be used as an identifier".to_string()),
        "box" => TokenType::LexerError("'box' is a reserved keyword and cannot be used as an identifier".to_string()),
        "unsafe" => TokenType::LexerError("'unsafe' is a reserved keyword and cannot be used as an identifier".to_string()),
        // end rust keywords
        "p" => {
            // 'p' is only ParallelStart if it's at the beginning of a line
            // and followed by whitespace and then a statement
            let mut lookahead = chars.clone();
            
            // Guard: No next character means end of input - treat as identifier
            let Some(&next_char) = lookahead.peek() else {
                let token_type = match validate_identifier_name(&identifier) {
                    Some(error) => TokenType::LexerError(error),
                    None => TokenType::Identifier(identifier),
                };
                return LexerOutput { token_type, start_line, start_column, end_line: state.line, end_column: state.column };
            };
            
            // Guard: If not followed by whitespace, it's an identifier
            if !next_char.is_whitespace() {
                let token_type = match validate_identifier_name(&identifier) {
                    Some(error) => TokenType::LexerError(error),
                    None => TokenType::Identifier(identifier),
                };
                return LexerOutput { token_type, start_line, start_column, end_line: state.line, end_column: state.column };
            }
            
            // Skip whitespace
            while let Some(&c) = lookahead.peek() {
                if !c.is_whitespace() {
                    break;
                }
                lookahead.next();
            }
            
            // Check what follows the whitespace
            let next_after_whitespace = lookahead.peek().copied();
            lookahead.next();
            let second_after_whitespace = lookahead.peek().copied();
            match next_after_whitespace {
                None => {
                    // End of input after whitespace - treat as identifier
                    match validate_identifier_name(&identifier) {
                        Some(error) => TokenType::LexerError(error),
                        None => TokenType::Identifier(identifier),
                    }
                }
                Some(c) => {
                    // A '/' can open a comment ('p  // note'), which is a
                    // statement context, or be division ('p / 2'), which is not.
                    // It can also close the block this very keyword opened, as
                    // in an empty 'p /p', which is a statement context too: an
                    // empty parallel block used to be read as a variable named
                    // p and reported as a name that is too short.
                    let starts_comment = c == '/' && second_after_whitespace == Some('/');
                    let closes_the_block = c == '/' && matches!(second_after_whitespace, Some('p') | Some('c'));
                    // These characters indicate 'p' is being used as a variable/identifier
                    let is_identifier_context = !starts_comment
                        && !closes_the_block
                        && matches!(c,
                            '.' | ':' | '(' | '+' | '-' | '*' | '/' | '=' |
                            '<' | '>' | ';' | ',' | ')' | ']' | '}' | '|'
                        );

                    if is_identifier_context {
                        match validate_identifier_name(&identifier) {
                            Some(error) => TokenType::LexerError(error),
                            None => TokenType::Identifier(identifier),
                        }
                    } else {
                        // It's a parallel block start
                        TokenType::ParallelStart
                    }
                }
            }
        }
        "c" => {
            // 'c' is only ConcurrentStart if it's at the beginning of a line
            // and followed by whitespace and then a statement
            let mut lookahead = chars.clone();
            
            // Guard: No next character means end of input - treat as identifier
            let Some(&next_char) = lookahead.peek() else {
                let token_type = match validate_identifier_name(&identifier) {
                    Some(error) => TokenType::LexerError(error),
                    None => TokenType::Identifier(identifier),
                };
                return LexerOutput { token_type, start_line, start_column, end_line: state.line, end_column: state.column };
            };
            
            // Guard: If not followed by whitespace, it's an identifier
            if !next_char.is_whitespace() {
                let token_type = match validate_identifier_name(&identifier) {
                    Some(error) => TokenType::LexerError(error),
                    None => TokenType::Identifier(identifier),
                };
                return LexerOutput { token_type, start_line, start_column, end_line: state.line, end_column: state.column };
            }
            
            // Skip whitespace
            while let Some(&c) = lookahead.peek() {
                if !c.is_whitespace() {
                    break;
                }
                lookahead.next();
            }
            
            // Check what follows the whitespace
            let next_after_whitespace = lookahead.peek().copied();
            lookahead.next();
            let second_after_whitespace = lookahead.peek().copied();
            match next_after_whitespace {
                None => {
                    // End of input after whitespace - treat as identifier
                    match validate_identifier_name(&identifier) {
                        Some(error) => TokenType::LexerError(error),
                        None => TokenType::Identifier(identifier),
                    }
                }
                Some(c) => {
                    // A '/' can open a comment ('c  // note'), which is a
                    // statement context, or be division ('c / 2'), which is not
                    let starts_comment = c == '/' && second_after_whitespace == Some('/');
                    // The same rule as above: `/p` and `/c` close a block rather than divide.
                    let closes_the_block = c == '/' && matches!(second_after_whitespace, Some('p') | Some('c'));
                    // These characters indicate 'c' is being used as a variable/identifier
                    let is_identifier_context = !starts_comment
                        && !closes_the_block
                        && matches!(c,
                            '.' | ':' | '(' | '+' | '-' | '*' | '/' | '=' |
                            '<' | '>' | ';' | ',' | ')' | ']' | '}' | '|'
                        );

                    if is_identifier_context {
                        match validate_identifier_name(&identifier) {
                            Some(error) => TokenType::LexerError(error),
                            None => TokenType::Identifier(identifier),
                        }
                    } else {
                        // It's a concurrent block start
                        TokenType::ConcurrentStart
                    }
                }
            }
        }
        "/p" => TokenType::ParallelEnd,
        "/c" => TokenType::ConcurrentEnd,
        _ => {
            // Validate before treating as identifier
            if let Some(error) = validate_identifier_name(&identifier) {
                TokenType::LexerError(error)
            } else {
                TokenType::Identifier(identifier)
            }
        }
    };

    LexerOutput { token_type, start_line, start_column, end_line: state.line, end_column: state.column }
}

fn lex_identifier_only(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;
    let mut identifier = String::new();
    while let Some(&c) = chars.peek() {
        if is_in_alphabet_or_number(c) || c == '_' {
            identifier.push(c);
            advance(chars, state);
        } else {
            break;
        }
    }
    // Always treat as identifier, never as keyword
    let token_type = if let Some(error) = validate_identifier_name(&identifier) { TokenType::LexerError(error) } else { TokenType::Identifier(identifier) };

    LexerOutput { token_type, start_line, start_column, end_line: state.line, end_column: state.column }
}

fn is_type_system_type(c: char) -> bool {
    c == ':'
}

fn lex_type_system_type(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;

    advance(chars, state); // skip ':'
    let mut type_name = String::new();

    // Parse the type name
    while let Some(&c) = chars.peek() {
        if is_in_alphabet_or_number(c) || c == '_' || c == ':' {
            type_name.push(c);
            advance(chars, state);
        } else {
            break;
        }
    }

    // Hashmap types carry their key and value in angle brackets (h<s,s>), and
    // the value can itself be a hashmap, so track depth rather than stopping at
    // the first '>'. Appending to type_name lets parse_type handle the whole
    // spelling in one place instead of a second, divergent parser here.
    if chars.peek() == Some(&'<') {
        let mut depth = 0usize;
        while let Some(&c) = chars.peek() {
            // None of these can appear inside a type, so an unclosed '<' is
            // reported against the type alone rather than swallowing the rest
            // of the program into the message.
            if c == '\n' || c == ';' || c == '{' || c == ')' {
                break;
            }
            if c == '<' {
                depth += 1;
            } else if c == '>' {
                depth -= 1;
            }
            type_name.push(c);
            advance(chars, state);
            if depth == 0 {
                break;
            }
        }

        if depth != 0 {
            return LexerOutput {
                token_type: TokenType::LexerError(format!("Expected '>' to close the type ':{}'", type_name)),
                start_line,
                start_column,
                end_line: state.line,
                end_column: state.column,
            };
        }

        return match parse_type(&type_name) {
            Ok(type_desc) => LexerOutput { token_type: TokenType::TypeDeclaration(type_desc), start_line, start_column, end_line: state.line, end_column: state.column },
            Err(e) => LexerOutput { token_type: TokenType::LexerError(e), start_line, start_column, end_line: state.line, end_column: state.column },
        };
    }

    // Check for result type (base_type!e)
    if chars.peek() == Some(&'!') {
        advance(chars, state); // skip '!'

        // Expect 'e' for error type
        if chars.peek() == Some(&'e') {
            advance(chars, state); // skip 'e'

            // Parse the base type first - could be a struct or enum name
            let base_type = if type_name.chars().next().map_or(false, |c| c.is_uppercase()) {
                // It's likely a struct or enum name
                NailDataTypeDescriptor::Struct(type_name.clone())
            } else {
                // Try to parse as a primitive type
                match parse_type(&type_name) {
                    Ok(base_type) => base_type,
                    Err(_) => {
                        // If it's not a primitive type, assume it's a struct
                        NailDataTypeDescriptor::Struct(type_name.clone())
                    }
                }
            };

            // Create a Result type wrapping the base type
            LexerOutput { token_type: TokenType::TypeDeclaration(NailDataTypeDescriptor::Result(Box::new(base_type))), start_line, start_column, end_line: state.line, end_column: state.column }
        } else {
            LexerOutput { token_type: TokenType::LexerError("Expected 'e' after '!' in result type".to_string()), start_line, start_column, end_line: state.line, end_column: state.column }
        }
    } else {
        // Handle other types
        // Special case: empty type name should not be an error, just skip
        if type_name.is_empty() {
            return LexerOutput { token_type: TokenType::LexerError("Empty type name".to_string()), start_line, start_column, end_line: state.line, end_column: state.column };
        }
        match parse_type(&type_name) {
            Ok(type_desc) => LexerOutput { token_type: TokenType::TypeDeclaration(type_desc), start_line, start_column, end_line: state.line, end_column: state.column },
            Err(e) => LexerOutput { token_type: TokenType::LexerError(e), start_line, start_column, end_line: state.line, end_column: state.column },
        }
    }
}

/// How deeply one type may nest. `a:a:i` is two levels, `h<s,a:i>` is two.
/// Reading a type is recursive, and recursion is stack, so a type written
/// thousands of levels deep has to become an error rather than an abort. Real
/// types nest two or three levels, so this is far past anything written on
/// purpose and still cheap to prove.
pub const MAX_TYPE_DEPTH: usize = 32;

fn parse_type(t: &str) -> Result<NailDataTypeDescriptor, String> {
    parse_type_at_depth(t, 0)
}

fn parse_type_at_depth(t: &str, depth: usize) -> Result<NailDataTypeDescriptor, String> {
    if depth > MAX_TYPE_DEPTH {
        return Err(format!("This type nests more than {} levels deep, which is deeper than any type the compiler will read", MAX_TYPE_DEPTH));
    }
    match t {
        "i" => Ok(NailDataTypeDescriptor::Int),
        "f" => Ok(NailDataTypeDescriptor::Float),
        "s" => Ok(NailDataTypeDescriptor::String),
        "b" => Ok(NailDataTypeDescriptor::Boolean),
        "v" => Ok(NailDataTypeDescriptor::Void),
        "a:i" => Ok(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int))),
        "a:f" => Ok(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Float))),
        "a:s" => Ok(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String))),
        "a:b" => Ok(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Boolean))),
        "e" => Ok(NailDataTypeDescriptor::Error),
        t if t.starts_with("struct:") => {
            let struct_name = t.strip_prefix("struct:").unwrap_or("").to_string();
            Ok(NailDataTypeDescriptor::Struct(struct_name))
        }
        t if t.starts_with("enum:") => {
            let enum_name = t.strip_prefix("enum:").unwrap_or("").to_string();
            Ok(NailDataTypeDescriptor::Enum(enum_name))
        }
        t if t.starts_with("a:struct:") => {
            let struct_name = t.strip_prefix("a:struct:").unwrap_or("").to_string();
            Ok(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct(struct_name))))
        }
        t if t.starts_with("a:enum:") => {
            let enum_name = t.strip_prefix("a:enum:").unwrap_or("").to_string();
            Ok(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Enum(enum_name))))
        }
        t if t.starts_with("h<") && t.ends_with(">") => {
            // Handle hashmap types like h<s,s>, h<i,s>, etc.
            let inner = t.strip_prefix("h<").unwrap().strip_suffix(">").unwrap();
            if let Some(comma_pos) = inner.find(',') {
                let key_type_str = inner[..comma_pos].trim();
                let value_type_str = inner[comma_pos + 1..].trim();

                let key_type = parse_type_at_depth(key_type_str, depth + 1)?;
                let value_type = parse_type_at_depth(value_type_str, depth + 1)?;

                Ok(NailDataTypeDescriptor::HashMap(Box::new(key_type), Box::new(value_type)))
            } else {
                Err(format!("Invalid hashmap type syntax: {}", t))
            }
        }
        t if t.starts_with("a:") => {
            // Handle array of custom types like a:Point, and nested arrays
            // like a:a:i by recursing on the element type
            let type_name = t.strip_prefix("a:").unwrap_or("").to_string();
            // Assume it's a struct array if it starts with uppercase
            if type_name.chars().next().map_or(false, |c| c.is_uppercase()) {
                Ok(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct(type_name))))
            } else {
                let element_type = parse_type_at_depth(&type_name, depth + 1)?;
                Ok(NailDataTypeDescriptor::Array(Box::new(element_type)))
            }
        }
        // If it starts with uppercase, assume it's a custom type (struct or enum)
        t if t.chars().next().map_or(false, |c| c.is_uppercase()) => Ok(NailDataTypeDescriptor::Struct(t.to_string())),
        _ => Err(format!("FailedToResolve type: {}", t)),
    }
}

fn is_number(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    if let Some(&c) = chars.peek() {
        c.is_digit(10)
            || (c == '-' && {
                let mut lookahead = chars.clone();
                lookahead.next(); // Skip the '-'
                lookahead.peek().map_or(false, |&next_char| next_char.is_digit(10))
            })
    } else {
        false
    }
}

fn lex_number(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;
    let mut number = String::new();
    let mut is_float = false; // To keep track if the number contains a decimal point

    // Handle leading negative sign
    if let Some(&'-') = chars.peek() {
        // Peek ahead to see if the next character is a digit (to handle cases like "-2")
        let mut lookahead = chars.clone();
        lookahead.next(); // Skip the '-'
        if lookahead.peek().map_or(false, |&c| c.is_digit(10)) {
            number.push('-');
            advance(chars, state); // Consume the '-'
        }
    }

    while let Some(&c) = chars.peek() {
        if c.is_digit(10) {
            number.push(c);
            advance(chars, state);
        } else if c == '.' {
            // A '.' belongs to the number only when a digit follows it. That
            // keeps `1..5` two tokens rather than one impossible number, and
            // it is what makes a second decimal point an error here rather
            // than a Rust error later: `1.2.0` used to lex as one float, type
            // check, and reach rustc as `1.2.0f64`.
            let mut lookahead = chars.clone();
            lookahead.next();
            let next = lookahead.peek().copied();
            // Two dots are a range, and the number ends before them.
            if next == Some('.') {
                break;
            }
            if !next.map_or(false, |next| next.is_digit(10)) {
                advance(chars, state);
                return LexerOutput {
                    token_type: TokenType::LexerError(format!("'{}.' has a decimal point with no digits after it, and a number needs at least one", number)),
                    start_line,
                    start_column,
                    end_line: state.line,
                    end_column: state.column,
                };
            }
            if is_float {
                advance(chars, state);
                return LexerOutput {
                    token_type: TokenType::LexerError(format!("'{}.' has more than one decimal point, and a number has at most one", number)),
                    start_line,
                    start_column,
                    end_line: state.line,
                    end_column: state.column,
                };
            }
            is_float = true;
            number.push(c);
            advance(chars, state);
        } else {
            break;
        }
    }

    let token_type = if is_float {
        TokenType::Float(number) // Return as float if a decimal point is found
    } else {
        TokenType::Integer(number) // Otherwise, return as integer
    };

    LexerOutput { token_type, start_line, start_column, end_line: state.line, end_column: state.column }
}

// A tagged string is an identifier written flush against the opening backtick,
// as in html`<p>hi</p>`. Nail has no other syntax where an identifier abuts a
// backtick, so the shape is unambiguous. The tag has no meaning to the
// compiler - the string lexes and transpiles exactly as an untagged one - it
// only tells a highlighter which language the contents are written in.
fn is_tagged_string_literal(chars: &std::iter::Peekable<std::str::Chars>) -> bool {
    let mut lookahead = chars.clone();

    match lookahead.next() {
        Some(c) if is_in_alphabet_lowercase(c) => {}
        _ => return false,
    }

    while let Some(&c) = lookahead.peek() {
        if is_in_alphabet_or_number(c) || c == '_' {
            lookahead.next();
        } else {
            break;
        }
    }

    return lookahead.peek() == Some(&'`');
}

fn lex_tagged_string_literal(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;

    let mut tag = String::new();
    while let Some(&c) = chars.peek() {
        if c == '`' {
            break;
        }
        tag.push(c);
        advance(chars, state);
    }

    let string_output = lex_string_literal(chars, state);

    // The string span starts at the tag, not at the backtick, so the whole
    // literal highlights as one unit.
    let token_type = match string_output.token_type {
        TokenType::StringLiteral { value, .. } => TokenType::StringLiteral { value, tag: Some(tag) },
        other => other,
    };

    return LexerOutput { token_type, start_line, start_column, end_line: string_output.end_line, end_column: string_output.end_column };
}

fn lex_string_literal(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;
    advance(chars, state); // Skip opening quote
    let mut string_literal = String::new();
    while let Some(c) = advance(chars, state) {
        if c == '\\' {
            // Handle escaped characters
            if let Some(next_c) = advance(chars, state) {
                match next_c {
                    '`' => string_literal.push('`'),   // Escaped backtick
                    'n' => string_literal.push('\n'),  // Newline
                    't' => string_literal.push('\t'),  // Tab
                    'r' => string_literal.push('\r'),  // Carriage return
                    '\\' => string_literal.push('\\'), // Escaped backslash
                    _ => {
                        // For unrecognized escape sequences, include both characters
                        string_literal.push('\\');
                        string_literal.push(next_c);
                    }
                }
            } else {
                // Backslash at end of string is an error
                return LexerOutput {
                    token_type: TokenType::LexerError("Unterminated escape sequence in string literal".to_string()),
                    start_line,
                    start_column,
                    end_line: state.line,
                    end_column: state.column,
                };
            }
        } else if c == '`' {
            return LexerOutput { token_type: TokenType::StringLiteral { value: string_literal, tag: None }, start_line, start_column, end_line: state.line, end_column: state.column };
        } else {
            string_literal.push(c);
        }
    }

    LexerOutput { token_type: TokenType::LexerError("Unterminated string literal".to_string()), start_line, start_column, end_line: state.line, end_column: state.column }
}

fn is_enum_variant(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    let mut lookahead = chars.clone();

    // Check if it starts with a capital letter (enum name)
    if let Some(c) = lookahead.next() {
        if is_alphabet_uppercase(c) {
            // Look for "::" after the enum name
            while let Some(c) = lookahead.next() {
                if c == ':' {
                    if lookahead.next() == Some(':') {
                        // Now look for the variant name (should start with a capital letter)
                        while let Some(c) = lookahead.next() {
                            if is_alphabet_uppercase(c) {
                                return true;
                            } else if !c.is_whitespace() {
                                return false;
                            }
                        }
                    }
                    return false;
                } else if !c.is_alphanumeric() && c != '_' {
                    return false;
                }
            }
        }
    }
    false
}

fn lex_enum_variant(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;

    let mut full_name = String::new();
    while let Some(&c) = chars.peek() {
        if c == ':' {
            full_name.push(c);
            advance(chars, state);
            if chars.peek() == Some(&':') {
                full_name.push(':');
                advance(chars, state);
            } else {
                break;
            }
        } else if is_in_alphabet_or_number(c) || c == '_' {
            // Digits belong to the name: a variant like ISO8601 is one word.
            full_name.push(c);
            advance(chars, state);
        } else {
            break;
        }
    }

    let parts: Vec<&str> = full_name.split("::").collect();
    if parts.len() == 2 {
        // Validate variant name
        if let Some(error) = validate_identifier_name(parts[1]) {
            return LexerOutput { token_type: TokenType::LexerError(error), start_line, start_column, end_line: state.line, end_column: state.column };
        }

        LexerOutput {
            token_type: TokenType::EnumVariant(EnumVariantData { name: parts[0].to_string(), variant: parts[1].to_string() }),
            start_line,
            start_column,
            end_line: state.line,
            end_column: state.column,
        }
    } else {
        LexerOutput { token_type: TokenType::LexerError(format!("Invalid enum variant syntax: {}", full_name)), start_line, start_column, end_line: state.line, end_column: state.column }
    }
}

/// Splice an imported file into the program. Ok(Some(tokens)) is a first
/// import, Ok(None) means the file is already part of the program and an
/// import is idempotent. A file importing a file that is still being spliced
/// is a cycle, and a file reached through both import() and
/// import_dangerous() has no one capability, so both are errors.
fn handle_import(
    filepath: &str,
    current_file: Option<&Path>,
    ctx: &mut ImportCtx,
    state: &LexerState,
    sandboxed: bool,
) -> Result<Option<Vec<Token>>, String> {
    // Resolve the path relative to the current file
    let resolved_path = if let Some(current) = current_file {
        if let Some(parent) = current.parent() {
            parent.join(filepath)
        } else {
            PathBuf::from(filepath)
        }
    } else {
        PathBuf::from(filepath)
    };

    // Get canonical path to handle symlinks and relative paths
    let canonical_path = resolved_path.canonicalize()
        .map_err(|e| format!("Cannot resolve path '{}': {}", filepath, e))?;

    // A file that is still in the middle of being spliced cannot be imported
    // again underneath itself
    if ctx.stack.contains(&canonical_path) {
        return Err(format!("Circular import detected: '{}'", filepath));
    }

    if let Some(&was_sandboxed) = ctx.spliced.get(&canonical_path) {
        if was_sandboxed != sandboxed {
            return Err(format!(
                "'{}' is imported both with import() and import_dangerous(). One file has one capability, so pick one form for it everywhere",
                filepath
            ));
        }
        return Ok(None);
    }

    // Read the file
    let content = fs::read_to_string(&canonical_path)
        .map_err(|e| format!("Cannot read file '{}': {}", filepath, e))?;

    // Give the file its own range of program lines, just past everything
    // lexed so far, and remember which import brought it in
    let lines = content.lines().count();
    let base = ctx.next_base;
    ctx.next_base += lines;
    // Show the file the way the user would name it: relative to the working
    // directory when it is under it, the resolved path otherwise
    let display_path = std::env::current_dir()
        .ok()
        .and_then(|cwd| canonical_path.strip_prefix(&cwd).ok().map(|p| p.to_string_lossy().into_owned()))
        .unwrap_or_else(|| resolved_path.to_string_lossy().into_owned());
    ctx.map.files.push(SourceFile {
        path: display_path,
        base,
        lines,
        content: content.clone(),
        imported_at: state.line,
    });

    ctx.stack.insert(canonical_path.clone());
    ctx.spliced.insert(canonical_path.clone(), sandboxed);

    // Lex the included file with its own context
    let mut sub_state = LexerState { line: base + 1, column: 1 };
    let tokens = lexer_inner(&content, &mut sub_state, Some(&canonical_path), ctx, ImportMode::Expand, sandboxed);

    ctx.stack.remove(&canonical_path);

    Ok(Some(tokens))
}

fn is_double_character_token(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    let mut lookahead = chars.clone();

    match lookahead.next() {
        Some('=') => match lookahead.peek() {
            Some('=') => true,
            _ => false,
        },
        Some('-') => match lookahead.peek() {
            Some('>') => true,
            _ => false,
        },
        Some('<') => match lookahead.peek() {
            Some('=') => true,
            _ => false,
        },
        Some('>') => match lookahead.peek() {
            Some('=') => true,
            _ => false,
        },
        Some('!') => match lookahead.peek() {
            Some('=') => true,
            _ => false,
        },
        Some('&') => match lookahead.peek() {
            Some('&') => true,
            _ => false,
        },
        Some('|') => match lookahead.peek() {
            Some('|') => true,
            _ => false,
        },
        Some('.') => match lookahead.peek() {
            Some('.') => true,
            _ => false,
        },
        _ => false,
    }
}

fn lex_double_character_token(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;

    let operator = advance(chars, state).expect("This should be the operator");

    let token_type = match operator {
        '=' => match advance(chars, state) {
            Some('=') => TokenType::Operator(Operation::Eq),
            _ => panic!("Unrecognized operator: {}", operator),
        },
        '-' => match advance(chars, state) {
            Some('>') => TokenType::ArrowAssignment,
            _ => panic!("Unrecognized operator: {}", operator),
        },
        '<' => match advance(chars, state) {
            Some('=') => TokenType::Operator(Operation::Lte),
            _ => panic!("Unrecognized operator: {}", operator),
        },
        '>' => match advance(chars, state) {
            Some('=') => TokenType::Operator(Operation::Gte),
            _ => panic!("Unrecognized operator: {}", operator),
        },
        '!' => match advance(chars, state) {
            Some('=') => TokenType::Operator(Operation::Ne),
            _ => panic!("Unrecognized operator: {}", operator),
        },
        '&' => match advance(chars, state) {
            Some('&') => TokenType::Operator(Operation::And),
            _ => panic!("Unrecognized operator: {}", operator),
        },
        '|' => match advance(chars, state) {
            Some('|') => TokenType::Operator(Operation::Or),
            _ => panic!("Unrecognized operator: {}", operator),
        },
        '.' => match advance(chars, state) {
            Some('.') => {
                if chars.peek() == Some(&'=') {
                    advance(chars, state);
                }
                TokenType::LexerError("Nail has no range syntax: array_range(start, end) and array_range_inclusive(start, end) build the array".to_string())
            },
            _ => panic!("Unrecognized operator: {}", operator),
        },
        _ => panic!("Unrecognized operator: {}", operator),
    };

    LexerOutput { token_type, start_line, start_column, end_line: state.line, end_column: state.column }
}

fn lex_value(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    // lex_value handles the lexing of individual, non-nested values:
    // - String literals (e.g., "hello")
    // - Numbers (integers and floats)
    // - Identifiers and keywords
    //
    // It does NOT handle:
    // - Complex expressions or operations
    // - Nested structures (e.g., arrays within arrays, structs within structs)
    // - Operators
    // - Parenthesized expressions
    // - Type annotations (e.g., :i, :s)
    //
    // This function is primarily used for lexing elements within arrays
    // and struct instantiations, where only values (not expressions) are allowed.
    if let Some(&c) = chars.peek() {
        let lexer_output: LexerOutput = match c {
            // Arrays are now handled by the parser, not the lexer
            '`' => lex_string_literal(chars, state),
            _ if is_tagged_string_literal(chars) => lex_tagged_string_literal(chars, state),
            _ if is_number(chars) => lex_number(chars, state),
            // Struct instantiation now handled by parser
            _ if is_enum_variant(chars) => lex_enum_variant(chars, state),
            _ if is_identifier_or_keyword(c) => lex_identifier_or_keyword(chars, state),
            _ => LexerOutput {
                token_type: TokenType::LexerError(format!("Unrecognized character in expression: {}", c)),
                start_line: state.line,
                end_line: state.line,
                start_column: state.column,
                end_column: state.column,
            },
        };
        lexer_output
    } else {
        LexerOutput { token_type: TokenType::LexerError("Unexpected end of input".to_string()), start_line: state.line, end_line: state.line, start_column: state.column, end_column: state.column }
    }
}


#[cfg(test)]
mod tagged_string_tests {
    use super::*;

    fn string_tokens(source: &str) -> Vec<(String, Option<String>)> {
        lexer(source)
            .into_iter()
            .filter_map(|token| match token.token_type {
                TokenType::StringLiteral { value, tag } => Some((value, tag)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_tag_against_the_backtick_is_kept_with_the_string() {
        let tokens = string_tokens("page:s = html`<p>hi</p>`;");
        assert_eq!(tokens, vec![("<p>hi</p>".to_string(), Some("html".to_string()))]);
    }

    #[test]
    fn any_identifier_shaped_tag_lexes_since_the_lexer_knows_no_languages() {
        let tokens = string_tokens("a:s = css`p{}`;\nb:s = sql`SELECT 1`;\nc:s = my_lang2`x`;");
        let tags: Vec<Option<String>> = tokens.into_iter().map(|(_, tag)| tag).collect();
        assert_eq!(tags, vec![Some("css".to_string()), Some("sql".to_string()), Some("my_lang2".to_string())]);
    }

    #[test]
    fn an_untagged_string_carries_no_tag() {
        let tokens = string_tokens("plain:s = `hello`;");
        assert_eq!(tokens, vec![("hello".to_string(), None)]);
    }

    #[test]
    fn a_tag_must_touch_the_backtick_to_count() {
        // With a space between them these are two separate tokens: an
        // identifier and an ordinary string.
        let tokens = string_tokens("html = `hello`;");
        assert_eq!(tokens, vec![("hello".to_string(), None)]);
    }

    #[test]
    fn a_tagged_string_escapes_exactly_like_an_untagged_one() {
        let tagged = string_tokens("a:s = html`a \\` b\\nc`;");
        let untagged = string_tokens("a:s = `a \\` b\\nc`;");
        assert_eq!(tagged[0].0, untagged[0].0);
        assert_eq!(tagged[0].0, "a ` b\nc");
    }

    #[test]
    fn the_span_of_a_tagged_string_starts_at_the_tag() {
        let tokens = lexer("page:s = html`<p>hi</p>`;");
        let string_token = tokens.iter().find(|token| matches!(token.token_type, TokenType::StringLiteral { .. })).expect("a string literal");
        // `page:s = ` is nine characters, so the tag starts at column ten.
        assert_eq!(string_token.code_span.start_column, 10);
    }
}
