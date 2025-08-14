use crate::common::CodeSpan;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::hash::Hash;

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
}

impl std::fmt::Display for NailDataTypeDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    ElseDeclaration,                         // For else keyword
    ParallelStart,                           // For p keyword
    ParallelEnd,                             // For /p keyword
    ForDeclaration,                          // For for keyword
    MapDeclaration,                          // For map keyword
    FilterDeclaration,                       // For filter keyword
    ReduceDeclaration,                       // For reduce keyword
    EachDeclaration,                         // For each keyword
    FindDeclaration,                         // For find keyword
    AllDeclaration,                          // For all keyword
    AnyDeclaration,                          // For any keyword
    WhileDeclaration,                        // For while keyword
    LoopKeyword,                             // For loop keyword (infinite loops)
    SpawnKeyword,                            // For spawn keyword (background tasks)
    InKeyword,                               // For in keyword
    FromKeyword,                             // For from keyword (initial accumulator)
    WhenKeyword,                             // For when keyword (filtering)
    BreakKeyword,                            // For break keyword
    ContinueKeyword,                         // For continue keyword
    MaxKeyword,                              // For max keyword (while bounds)
    StepKeyword,                             // For step keyword (for loops)
    Range,                                   // For .. operator
    RangeInclusive,                          // For ..= operator
    Assignment,                              // For assignment ie =
    ArrowAssignment,                         // For arrow assignment ie =>
    Identifier(String),                      // For variable names, etc.
    Float(String),                           // For float numbers
    Integer(String),                         // For integer numbers
    BooleanLiteral(bool),                    // For boolean literals (true/false)
    Operator(Operation),                     // For operators like +, -, *, /
    Comma,                                   // For commas
    Colon,                                   // For colons
    StringLiteral(String),                   // For string literals
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

pub fn lexer(input: &str) -> Vec<Token> {
    let mut state = LexerState { line: 1, column: 1 };
    lexer_inner(input, &mut state)
}

fn lexer_inner(input: &str, state: &mut LexerState) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
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
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
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
            _ => TokenType::LexerError("Expected function name".to_string()),
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
    let mut lookahead = chars.clone();
    if let Some(first) = lookahead.next() {
        if first == '/' {
            if let Some(second) = lookahead.next() {
                return second == 'p';
            }
        }
    }
    false
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

fn is_struct_declaration(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    let mut lookahead = chars.clone();
    lookahead.next() == Some('s') && lookahead.next() == Some('t') && lookahead.next() == Some('r') && lookahead.next() == Some('u') && lookahead.next() == Some('c') && lookahead.next() == Some('t')
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
    let mut lookahead = chars.clone();
    lookahead.next() == Some('e') && lookahead.next() == Some('n') && lookahead.next() == Some('u') && lookahead.next() == Some('m')
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
    // Check if identifier is a single letter - all single letter identifiers are forbidden
    // EXCEPT 'e' which is used for error returns (e.g., r e("error message"))
    // AND type annotations: i, f, s, b, v, a, h
    // AND common struct field names: x, y, z, w (for coordinates/vectors)
    let valid_single_letters = ["e", "i", "f", "s", "b", "v", "a", "h", "x", "y", "z", "w"];
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
        "loop" => TokenType::LoopKeyword,
        "spawn" => TokenType::SpawnKeyword,
        "while" => TokenType::WhileDeclaration,
        "for" => TokenType::ForDeclaration,
        "map" => TokenType::MapDeclaration,
        "filter" => TokenType::FilterDeclaration,
        "reduce" => TokenType::ReduceDeclaration,
        "each" => TokenType::EachDeclaration,
        "find" => TokenType::FindDeclaration,
        "all" => TokenType::AllDeclaration,
        "any" => TokenType::AnyDeclaration,
        "in" => TokenType::InKeyword,
        "from" => TokenType::FromKeyword,
        "when" => TokenType::WhenKeyword,
        "break" => TokenType::BreakKeyword,
        "continue" => TokenType::ContinueKeyword,
        "max" => TokenType::MaxKeyword,
        "step" => TokenType::StepKeyword,
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
            if let Some(&next_char) = lookahead.peek() {
                // If followed by '.' or ':' or any non-whitespace character, it's an identifier
                if !next_char.is_whitespace() {
                    if let Some(error) = validate_identifier_name(&identifier) {
                        TokenType::LexerError(error)
                    } else {
                        TokenType::Identifier(identifier)
                    }
                } else {
                    // Skip whitespace
                    while let Some(&c) = lookahead.peek() {
                        if c.is_whitespace() {
                            lookahead.next();
                        } else {
                            break;
                        }
                    }
                    // Check what follows the whitespace
                    if let Some(&c) = lookahead.peek() {
                        // These characters indicate 'p' is being used as a variable/identifier
                        if c == '.'
                            || c == ':'
                            || c == '('
                            || c == '+'
                            || c == '-'
                            || c == '*'
                            || c == '/'
                            || c == '='
                            || c == '<'
                            || c == '>'
                            || c == ';'
                            || c == ','
                            || c == ')'
                            || c == ']'
                            || c == '}'
                            || c == '|'
                        {
                            if let Some(error) = validate_identifier_name(&identifier) {
                                TokenType::LexerError(error)
                            } else {
                                TokenType::Identifier(identifier)
                            }
                        } else {
                            // It's likely a parallel block start
                            TokenType::ParallelStart
                        }
                    } else {
                        // End of input after 'p', treat as identifier
                        if let Some(error) = validate_identifier_name(&identifier) {
                            TokenType::LexerError(error)
                        } else {
                            TokenType::Identifier(identifier)
                        }
                    }
                }
            } else {
                // End of input, treat as identifier
                if let Some(error) = validate_identifier_name(&identifier) {
                    TokenType::LexerError(error)
                } else {
                    TokenType::Identifier(identifier)
                }
            }
        }
        "/p" => TokenType::ParallelEnd,
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

    // Check for hashmap type (h(key_type, value_type))
    if type_name == "h" && chars.peek() == Some(&'(') {
        advance(chars, state); // skip '('

        // Parse key type
        let mut key_type_name = String::new();

        while let Some(&c) = chars.peek() {
            if c == ',' || c == ')' {
                break;
            }
            if is_in_alphabet_or_number(c) || c == '_' {
                key_type_name.push(c);
                advance(chars, state);
            } else {
                break;
            }
        }

        // Skip whitespace
        while chars.peek() == Some(&' ') {
            advance(chars, state);
        }

        // Expect comma
        if chars.peek() != Some(&',') {
            return LexerOutput { token_type: TokenType::LexerError("Expected ',' in hashmap type".to_string()), start_line, start_column, end_line: state.line, end_column: state.column };
        }
        advance(chars, state); // skip ','

        // Skip whitespace
        while chars.peek() == Some(&' ') {
            advance(chars, state);
        }

        // Parse value type
        let mut value_type_name = String::new();
        while let Some(&c) = chars.peek() {
            if c == ')' {
                break;
            }
            if is_in_alphabet_or_number(c) || c == '_' {
                value_type_name.push(c);
                advance(chars, state);
            } else {
                break;
            }
        }

        // Expect closing parenthesis
        if chars.peek() != Some(&')') {
            return LexerOutput { token_type: TokenType::LexerError("Expected ')' in hashmap type".to_string()), start_line, start_column, end_line: state.line, end_column: state.column };
        }
        advance(chars, state); // skip ')'

        // Parse the key and value types
        let key_type = match parse_type(&key_type_name) {
            Ok(t) => t,
            Err(_) => {
                // If it's not a primitive type, assume it's a struct
                NailDataTypeDescriptor::Struct(key_type_name)
            }
        };

        let value_type = match parse_type(&value_type_name) {
            Ok(t) => t,
            Err(_) => {
                // If it's not a primitive type, assume it's a struct
                NailDataTypeDescriptor::Struct(value_type_name)
            }
        };

        return LexerOutput {
            token_type: TokenType::TypeDeclaration(NailDataTypeDescriptor::HashMap(Box::new(key_type), Box::new(value_type))),
            start_line,
            start_column,
            end_line: state.line,
            end_column: state.column,
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
    } else if type_name == "oneof" {
        // Handle 'oneof' type
        if chars.peek() == Some(&'(') {
            advance(chars, state); // advance past the '('

            let mut types_in_oneof = Vec::new();
            let mut any_string = String::new();

            while let Some(&c) = chars.peek() {
                if c == ')' {
                    advance(chars, state);
                    break;
                }
                any_string.push(c);
                advance(chars, state);
            }

            let types = any_string.split("|").collect::<Vec<&str>>();

            for t in types {
                match parse_type(t) {
                    Ok(type_desc) => {
                        types_in_oneof.push(type_desc);
                    }
                    Err(e) => return LexerOutput { token_type: TokenType::LexerError(e), start_line, start_column, end_line: state.line, end_column: state.column },
                }
            }

            LexerOutput { token_type: TokenType::TypeDeclaration(NailDataTypeDescriptor::OneOf(types_in_oneof)), start_line, start_column, end_line: state.line, end_column: state.column }
        } else {
            LexerOutput { token_type: TokenType::LexerError("Expected '(' after 'oneof'".to_string()), start_line, start_column, end_line: state.line, end_column: state.column }
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

fn parse_type(t: &str) -> Result<NailDataTypeDescriptor, String> {
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

                let key_type = parse_type(key_type_str)?;
                let value_type = parse_type(value_type_str)?;

                Ok(NailDataTypeDescriptor::HashMap(Box::new(key_type), Box::new(value_type)))
            } else {
                Err(format!("Invalid hashmap type syntax: {}", t))
            }
        }
        t if t.starts_with("a:") => {
            // Handle array of custom types like a:Point
            let type_name = t.strip_prefix("a:").unwrap_or("").to_string();
            // Assume it's a struct array if it starts with uppercase
            if type_name.chars().next().map_or(false, |c| c.is_uppercase()) {
                Ok(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct(type_name))))
            } else {
                Err(format!("FailedToResolve array type: {}", t))
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
            // If there's a decimal point, mark it as a float
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
            return LexerOutput { token_type: TokenType::StringLiteral(string_literal), start_line, start_column, end_line: state.line, end_column: state.column };
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
        } else if is_in_alphabet(c) || c == '_' {
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

fn is_double_character_token(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    let mut lookahead = chars.clone();

    match lookahead.next() {
        Some('=') => match lookahead.peek() {
            Some('=') => true,
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
                // Check for ..= (inclusive range)
                if chars.peek() == Some(&'=') {
                    advance(chars, state);
                    TokenType::RangeInclusive
                } else {
                    TokenType::Range
                }
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

