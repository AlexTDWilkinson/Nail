use crate::statics_for_tests::*;
use dashmap::DashMap;
use lazy_static::lazy_static;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::hash::Hash;
use std::hash::Hasher;

//  static the alphabet in lower and uppercase and 0-9

static ALPHABET_AND_NUMBERS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
static ALPHABET_LOWERCASE_AND_NUMBERS: &str = "abcdefghijklmnopqrstuvwxyz0123456789";
static ALPHABET_UPPERCASE_AND_NUMBERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
static ALPHABET_LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
static ALPHABET: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
static ALPHABET_UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
static NUMBERS: &str = "0123456789";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDeclarationData {
    pub name: String,
    pub fields: Vec<StructDeclarationDataField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructInstantiationData {
    pub name: String,
    pub fields: Vec<StructInstantiationDataField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructInstantiationDataField {
    pub name: String,
    pub value: Token,
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
    ArrayInt,
    ArrayFloat,
    ArrayString,
    ArrayBoolean,
    ArrayStruct(String),
    ArrayEnum(String),
    Struct(String),
    Enum(String),
    Void,
    Error,
    Any(Vec<NailDataTypeDescriptor>),
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
    Array(Vec<Token>),
    LambdaSignature(Vec<Token>),
    LambdaReturnTypeDeclaration(NailDataTypeDescriptor),
    FunctionReturnTypeDeclaration(NailDataTypeDescriptor),
    FunctionName(String),
    StructDeclaration(StructDeclarationData), // For struct declarations
    StructInstantiation(StructInstantiationData),
    EnumDeclaration(EnumDeclarationData), // For enum data
    StructFieldAccess(String, String),
    EnumVariant(EnumVariantData),            // For enum variant name
    Comment(String),                         // For comments
    FunctionSignature(Vec<Token>),           // For function declarations ie "fn"
    ConstDeclaration,                        // For const declarations ie "c"
    VariableDeclaration,                     // For variable declarations ie "v"
    IfDeclaration,                           // For if keyword
    ElseDeclaration,                         // For else keyword
    Assignment,                              // For assignment ie =
    ArrowAssignment,                         // For arrow assignment ie =>
    Identifier(String),                      // For variable names, etc.
    Float(String),                           // For float numbers
    Integer(String),                         // For integer numbers
    Operator(Operation),                     // For operators like +, -, *, /
    Comma,                                   // For commas
    StringLiteral(String),                   // For string literals
    TypeDeclaration(NailDataTypeDescriptor), // For explicit type declarations
    ParenthesisOpen,                         // For parenthesis open
    ParenthesisClose,                        // For parenthesis close
    BlockOpen,                               // For block start
    BlockClose,                              // For block end
    EndStatementOrExpression,                // For end of statement or expression
    RustEscape(Vec<Token>),                  // For rust code container
    RustLiteral(String),                     // For a piece of literal rust code
    RustNailInsert(Vec<Token>),              // For a nail insert in rust code
    LexerError(String),                      // For lexer_inner errors
    Return,                                  // For return keyword
    EndOfFile,                               // For end of file
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Operation {
    Add, // "+"
    Sub, // "-"
    Mul, // "*"
    Div, // "/"
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
            Operation::Mul | Operation::Div => 5,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeSpan {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
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
            _ if is_rust_literal(&mut chars) => {
                let lexer_output: LexerOutput = lex_rust_escape(&mut chars, state);
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

            _ if is_lambda(&mut chars) => {
                let lexer_output = lex_lambda(&mut chars, state);
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
            }

            _ if is_array(&mut chars) => {
                let lexer_output = lex_array(&mut chars, state);
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
            }
            '`' => {
                let lexer_output: LexerOutput = lex_string_literal(&mut chars, state);
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
            }

            _ if is_type_system_type(c) => {
                let lexer_output: LexerOutput = lex_type_system_type(&mut chars, state);
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

            _ if is_struct_instantiation(&mut chars) => {
                let lexer_output: LexerOutput = lex_struct_instantiation(&mut chars, state);
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
            }

            _ if is_enum_variant(&mut chars) => {
                let lexer_output = lex_enum_variant(&mut chars, state);
                tokens.push(Token {
                    token_type: lexer_output.token_type,
                    code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                });
            }

            _ if is_identifier_or_keyword(c) => {
                let mut lookahead = chars.clone();
                let mut word = String::new();
                while let Some(&c) = lookahead.peek() {
                    if is_in_alphabet_or_number(c) || c == '_' {
                        word.push(c);
                        lookahead.next();
                    } else {
                        break;
                    }
                }
                if lookahead.peek() == Some(&'.') {
                    let lexer_output = lex_struct_field_access(&mut chars, state);
                    tokens.push(Token {
                        token_type: lexer_output.token_type,
                        code_span: CodeSpan { start_line: lexer_output.start_line, end_line: lexer_output.end_line, start_column: lexer_output.start_column, end_column: lexer_output.end_column },
                    });
                } else {
                    let lexer_output = lex_identifier_or_keyword(&mut chars, state);
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

fn is_struct_instantiation(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    let mut lookahead = chars.clone();

    // Check if it starts with a capital letter
    if let Some(c) = lookahead.next() {
        if c.is_ascii_uppercase() {
            // Consume the rest of the identifier
            while let Some(c) = lookahead.next() {
                if !is_in_alphabet_or_number(c) && c != '_' {
                    break;
                }
            }

            // Look for opening brace, ignoring whitespace
            while let Some(c) = lookahead.next() {
                if c.is_whitespace() {
                    continue;
                } else if c == '{' {
                    return true;
                } else {
                    return false;
                }
            }
        }
    }

    false
}

fn lex_struct_instantiation(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;

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

    // Expect opening brace
    if chars.peek() != Some(&'{') {
        return LexerOutput { token_type: TokenType::LexerError("Expected '{' in struct instantiation".to_string()), start_line, start_column, end_line: state.line, end_column: state.column };
    }
    advance(chars, state);

    let mut fields: Vec<StructInstantiationDataField> = Vec::new();
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

        let mut field_name = String::new();
        while let Some(&c) = chars.peek() {
            if is_in_alphabet_or_number(c) || c == '_' {
                field_name.push(c);
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

        // Expect colon
        if chars.peek() != Some(&':') {
            return LexerOutput {
                token_type: TokenType::LexerError("Expected ':' after field name in struct instantiation".to_string()),
                start_line,
                start_column,
                end_line: state.line,
                end_column: state.column,
            };
        }
        advance(chars, state);

        // Skip whitespace
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                advance(chars, state);
            } else {
                break;
            }
        }

        // Parse field value
        let value = lex_value(chars, state);
        fields.push(StructInstantiationDataField {
            name: field_name,
            value: Token {
                token_type: value.token_type,
                code_span: CodeSpan { start_line: value.start_line, end_line: value.end_line, start_column: value.start_column, end_column: value.end_column },
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

        // Expect comma or closing brace
        match chars.peek() {
            Some(&',') => {
                advance(chars, state);
            }
            Some(&'}') => {
                // We'll handle this at the start of the loop
            }
            _ => {
                return LexerOutput {
                    token_type: TokenType::LexerError("Expected ',' or '}' after field value in struct instantiation".to_string()),
                    start_line,
                    start_column,
                    end_line: state.line,
                    end_column: state.column,
                };
            }
        }
    }

    LexerOutput { token_type: TokenType::StructInstantiation(StructInstantiationData { name: struct_name, fields }), start_line, start_column, end_line: state.line, end_column: state.column }
}

fn lex_array(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;

    let mut elements = Vec::new();

    advance(chars, state); // Consume '['

    loop {
        // Skip whitespace
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                advance(chars, state);
            } else {
                break;
            }
        }

        // Check for array end
        if chars.peek() == Some(&']') {
            advance(chars, state); // Consume ']'
            break;
        }

        // Lex array element
        if is_array(chars) {
            let nested_array = lex_array(chars, state);
            elements.push(Token {
                token_type: nested_array.token_type,
                code_span: CodeSpan { start_line: nested_array.start_line, end_line: nested_array.end_line, start_column: nested_array.start_column, end_column: nested_array.end_column },
            });
        } else {
            let element = lex_value(chars, state);
            elements.push(Token {
                token_type: element.token_type,
                code_span: CodeSpan { start_line: element.start_line, end_line: element.end_line, start_column: element.start_column, end_column: element.end_column },
            });
        }

        // Skip whitespace
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                advance(chars, state);
            } else {
                break;
            }
        }

        // Check for comma or array end
        match chars.peek() {
            Some(&',') => {
                advance(chars, state); // Consume ','
            }
            Some(&']') => continue, // Will be handled at the start of the loop
            _ => return LexerOutput { token_type: TokenType::LexerError("Expected ',' or ']' in array".to_string()), start_line, start_column, end_line: state.line, end_column: state.column },
        }
    }

    LexerOutput { token_type: TokenType::Array(elements), start_line, start_column, end_line: state.line, end_column: state.column }
}

fn is_function_signature(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    let mut lookahead = chars.clone();
    lookahead.next() == Some('f') && lookahead.next() == Some('n')
}

fn lex_function_signature(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;

    let mut tokens: Vec<Token> = vec![];

    advance(chars, state); // skip 'f'
    advance(chars, state); // skip 'n'

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
            let param_name = lex_identifier_or_keyword(chars, state);
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

fn is_function_call(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    let mut lookahead = chars.clone();

    // Consume valid function name characters (letters, numbers, or underscore)
    while let Some(c) = lookahead.next() {
        if is_in_alphabet_or_number(c) || c == '_' {
            continue;
        } else if c == '(' {
            return true;
        } else {
            return false;
        }
    }

    false
}

// Helper function to skip whitespace
fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) {
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            advance(chars, state);
        } else {
            break;
        }
    }
}

fn is_lambda(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    chars.peek() == Some(&'|')
}
fn lex_lambda(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;
    let mut tokens = Vec::new();

    // Consume opening '|'
    advance(chars, state);

    // Parse parameters
    loop {
        // Eat whitespace
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                advance(chars, state);
            } else {
                break;
            }
        }

        // Check if we've reached the end of parameters
        if chars.peek() == Some(&'|') {
            break;
        }

        // Parse parameter name
        let param_name = lex_identifier_or_keyword(chars, state);
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

        // Eat whitespace
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                advance(chars, state);
            } else {
                break;
            }
        }

        // Check for comma or end of parameters
        if chars.peek() == Some(&',') {
            tokens.push(Token { token_type: TokenType::Comma, code_span: CodeSpan { start_line: state.line, end_line: state.line, start_column: state.column, end_column: state.column + 1 } });
            advance(chars, state);
        } else if chars.peek() == Some(&'|') {
            break;
        } else {
            return LexerOutput {
                token_type: TokenType::LexerError("Expected ',' or '|' after lambda parameter".to_string()),
                start_line,
                start_column,
                end_line: state.line,
                end_column: state.column,
            };
        }
    }

    // Consume closing '|'
    if chars.peek() == Some(&'|') {
        advance(chars, state);
    } else {
        return LexerOutput { token_type: TokenType::LexerError("Expected '|' at end of lambda parameters".to_string()), start_line, start_column, end_line: state.line, end_column: state.column };
    }

    // Parse return type if present
    if chars.peek() == Some(&':') {
        let return_type = lex_type_system_type(chars, state);
        if let TokenType::TypeDeclaration(t) = return_type.token_type {
            tokens.push(Token {
                token_type: TokenType::LambdaReturnTypeDeclaration(t),
                code_span: CodeSpan { start_line: return_type.start_line, end_line: return_type.end_line, start_column: return_type.start_column, end_column: return_type.end_column },
            });
        } else {
            return LexerOutput {
                token_type: TokenType::LexerError("Expected type declaration for lambda return type".to_string()),
                start_line: state.line,
                start_column: state.column,
                end_line: state.line,
                end_column: state.column,
            };
        }
    }

    LexerOutput { token_type: TokenType::LambdaSignature(tokens), start_line, start_column, end_line: state.line, end_column: state.column }
}

fn is_array(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    chars.peek() == Some(&'[')
}

fn is_comment(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    let mut lookahead = chars.clone();
    lookahead.next() == Some('/') && lookahead.next() == Some('/')
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
            '(' | ')' | ';' | '{' | '}' | ',' | '!' | '+' | '-' | '*' | '/' | '=' | '<' | '>' => {
                // Check if it's followed by a space, or by something it's allowed to be beside or end of input
                match lookahead.next() {
                    Some(next_char) => {
                        next_char.is_whitespace()
                            || is_in_alphabet_or_number(next_char)
                            || next_char == ';'
                            || next_char == ','
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
        '=' => TokenType::Assignment,
        '!' => TokenType::Operator(Operation::Ne),
        '+' => TokenType::Operator(Operation::Add),
        '-' => TokenType::Operator(Operation::Sub),
        '*' => TokenType::Operator(Operation::Mul),
        '/' => TokenType::Operator(Operation::Div),
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
        "c" => TokenType::ConstDeclaration,
        "v" => TokenType::VariableDeclaration,
        "r" => TokenType::Return,
        "if" => TokenType::IfDeclaration,
        "else" => TokenType::ElseDeclaration,
        _ => TokenType::Identifier(identifier),
    };

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

    if type_name == "any" {
        // Handle 'any' type
        if chars.peek() == Some(&'(') {
            advance(chars, state); // advance past the '('

            let mut types_in_any = Vec::new();
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
                        types_in_any.push(type_desc);
                    }
                    Err(e) => return LexerOutput { token_type: TokenType::LexerError(e), start_line, start_column, end_line: state.line, end_column: state.column },
                }
            }

            LexerOutput { token_type: TokenType::TypeDeclaration(NailDataTypeDescriptor::Any(types_in_any)), start_line, start_column, end_line: state.line, end_column: state.column }
        } else {
            LexerOutput { token_type: TokenType::LexerError("Expected '(' after 'any'".to_string()), start_line, start_column, end_line: state.line, end_column: state.column }
        }
    } else {
        // Handle other types
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
        "a:i" => Ok(NailDataTypeDescriptor::ArrayInt),
        "a:f" => Ok(NailDataTypeDescriptor::ArrayFloat),
        "a:s" => Ok(NailDataTypeDescriptor::ArrayString),
        "a:b" => Ok(NailDataTypeDescriptor::ArrayBoolean),
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
            Ok(NailDataTypeDescriptor::ArrayStruct(struct_name))
        }
        t if t.starts_with("a:enum:") => {
            let enum_name = t.strip_prefix("a:enum:").unwrap_or("").to_string();
            Ok(NailDataTypeDescriptor::ArrayEnum(enum_name))
        }
        _ => Err(format!("Unknown type: {}", t)),
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
        if c == '`' {
            return LexerOutput { token_type: TokenType::StringLiteral(string_literal), start_line, start_column, end_line: state.line, end_column: state.column };
        }
        string_literal.push(c);
    }

    LexerOutput { token_type: TokenType::LexerError("Unterminated string literal".to_string()), start_line, start_column, end_line: state.line, end_column: state.column }
}

fn is_rust_literal(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    let mut lookahead = chars.clone();
    if lookahead.next() == Some('R') && lookahead.next() == Some('{') {
        return true;
    }
    false
}

// this is an escape hatch to allow for rust code to be embedded in nail code for the standard library, or mega power users I guess
fn lex_rust_escape(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;
    // Skip 'R{'
    advance(chars, state);
    advance(chars, state);
    let mut content = Vec::new();
    let mut rust_literal = String::new();
    let mut brace_count = 1;

    while let Some(&c) = chars.peek() {
        match c {
            '^' if chars.clone().nth(1) == Some('[') => {
                if !rust_literal.is_empty() {
                    let token = Token {
                        token_type: TokenType::RustLiteral(rust_literal),
                        code_span: CodeSpan { start_line: start_line, end_line: state.line, start_column: start_column, end_column: state.column },
                    };
                    content.push(token);
                    rust_literal = String::new();
                }
                // Handle NAIL injection
                let nail_start_line = state.line;
                let nail_start_column = state.column;
                advance(chars, state); // Skip '^'
                advance(chars, state); // Skip '['
                let mut nail_content = String::new();
                while let Some(&c) = chars.peek() {
                    if c == ']' && chars.clone().nth(1) == Some('^') {
                        advance(chars, state); // Skip ']'
                        advance(chars, state); // Skip '^'
                        break;
                    }
                    nail_content.push(c);
                    advance(chars, state);
                    if chars.peek().is_none() {
                        return LexerOutput { token_type: TokenType::LexerError("Unterminated NAIL injection".to_string()), start_line, start_column, end_line: state.line, end_column: state.column };
                    }
                }
                // Recursively lex the NAIL content
                // This is why we seperate lexer_inner, so we can call it recursively w/out losing the line/cols.
                let nail_tokens = lexer_inner(&nail_content, state);
                let token = Token {
                    token_type: TokenType::RustNailInsert(nail_tokens),
                    code_span: CodeSpan { start_line: nail_start_line, end_line: state.line, start_column: nail_start_column, end_column: state.column },
                };
                content.push(token);
            }
            '{' => {
                brace_count += 1;
                rust_literal.push(c);
                advance(chars, state);
            }
            '}' => {
                brace_count -= 1;
                if brace_count == 0 {
                    advance(chars, state);
                    break;
                }
                rust_literal.push(c);
                advance(chars, state);
            }
            _ => {
                if chars.peek().is_none() {
                    return LexerOutput { token_type: TokenType::LexerError("Unterminated rust escape".to_string()), start_line, start_column, end_line: state.line, end_column: state.column };
                }
                rust_literal.push(c);
                advance(chars, state);
            }
        }
    }

    if !rust_literal.is_empty() {
        let token =
            Token { token_type: TokenType::RustLiteral(rust_literal), code_span: CodeSpan { start_line: start_line, end_line: state.line, start_column: start_column, end_column: state.column } };
        content.push(token);
    }

    LexerOutput { token_type: TokenType::RustEscape(content), start_line, end_line: state.line, start_column, end_column: state.column }
}

fn lex_struct_field_access(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) -> LexerOutput {
    let start_line = state.line;
    let start_column = state.column;

    let mut struct_name = String::new();
    while let Some(&c) = chars.peek() {
        if c == '.' {
            advance(chars, state);
            break;
        }
        struct_name.push(c);
        advance(chars, state);
    }

    let mut field_name = String::new();
    while let Some(&c) = chars.peek() {
        if is_in_alphabet_or_number(c) || c == '_' {
            field_name.push(c);
            advance(chars, state);
        } else {
            break;
        }
    }

    LexerOutput { token_type: TokenType::StructFieldAccess(struct_name, field_name), start_line, start_column, end_line: state.line, end_column: state.column }
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
            _ if is_array(chars) => lex_array(chars, state),
            '`' => lex_string_literal(chars, state),
            _ if is_number(chars) => lex_number(chars, state),
            _ if is_struct_instantiation(chars) => lex_struct_instantiation(chars, state),
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
mod tests {
    use super::*;
    use crate::lexer::TokenType::*;

    #[test]
    fn test_if_statement() {
        let input = "if { a > 5 => {} };";
        let result = lexer(input);
        println!("RESULT: {:#?}", result);
        assert_eq!(
            result,
            vec![
                Token { token_type: IfDeclaration, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 3 } },
                Token { token_type: BlockOpen, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 4, end_column: 5 } },
                Token { token_type: Identifier("a".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 6, end_column: 7 } },
                Token { token_type: Operator(Operation::Gt), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 8, end_column: 9 } },
                Token { token_type: Integer("5".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 10, end_column: 11 } },
                Token { token_type: ArrowAssignment, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 12, end_column: 14 } },
                Token { token_type: BlockOpen, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 15, end_column: 16 } },
                Token { token_type: BlockClose, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 16, end_column: 17 } },
                Token { token_type: BlockClose, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 18, end_column: 19 } },
                Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 19, end_column: 20 } },
            ]
        );
    }

    #[test]
    fn test_if_else_statement() {
        let input = "if { a > 5 => {}, else => {} };";
        let result = lexer(input);
        println!("RESULT: {:#?}", result);
        assert_eq!(
            result,
            vec![
                Token { token_type: IfDeclaration, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 3 } },
                Token { token_type: BlockOpen, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 4, end_column: 5 } },
                Token { token_type: Identifier("a".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 6, end_column: 7 } },
                Token { token_type: Operator(Operation::Gt), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 8, end_column: 9 } },
                Token { token_type: Integer("5".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 10, end_column: 11 } },
                Token { token_type: ArrowAssignment, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 12, end_column: 14 } },
                Token { token_type: BlockOpen, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 15, end_column: 16 } },
                Token { token_type: BlockClose, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 16, end_column: 17 } },
                Token { token_type: Comma, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 17, end_column: 18 } },
                Token { token_type: ElseDeclaration, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 19, end_column: 23 } },
                Token { token_type: ArrowAssignment, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 24, end_column: 26 } },
                Token { token_type: BlockOpen, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 27, end_column: 28 } },
                Token { token_type: BlockClose, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 28, end_column: 29 } },
                Token { token_type: BlockClose, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 30, end_column: 31 } },
                Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 31, end_column: 32 } }
            ]
        );
    }

    #[test]
    fn test_function_call() {
        let input = "fun(param);";
        let result = lexer(input);
        println!("RESULT: {:#?}", result);
        assert_eq!(
            result,
            vec![
                Token { token_type: Identifier("fun".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 4 } },
                Token { token_type: ParenthesisOpen, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 4, end_column: 5 } },
                Token { token_type: Identifier("param".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 5, end_column: 10 } },
                Token { token_type: ParenthesisClose, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 10, end_column: 11 } },
                Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 11, end_column: 12 } }
            ]
        );
    }

    #[test]
    fn test_function_nested_call() {
        let input = "fun(times(param));";
        let result = lexer(input);
        println!("RESULT: {:#?}", result);
        assert_eq!(
            result,
            vec![
                Token { token_type: Identifier("fun".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 4 } },
                Token { token_type: ParenthesisOpen, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 4, end_column: 5 } },
                Token { token_type: Identifier("times".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 5, end_column: 10 } },
                Token { token_type: ParenthesisOpen, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 10, end_column: 11 } },
                Token { token_type: Identifier("param".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 11, end_column: 16 } },
                Token { token_type: ParenthesisClose, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 16, end_column: 17 } },
                Token { token_type: ParenthesisClose, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 17, end_column: 18 } },
                Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 18, end_column: 19 } }
            ]
        );
    }

    #[test]
    fn test_function_declaration() {
        let input = "fn fun(param:i):i { v x:i = 5; r x; }";
        let result = lexer(input);
        println!("RESULT: {:#?}", result);
        assert_eq!(
            result,
            vec![
                Token {
                    token_type: FunctionSignature(vec![
                        Token { token_type: FunctionName("fun".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 4, end_column: 7 } },
                        Token { token_type: Identifier("param".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 8, end_column: 13 } },
                        Token { token_type: TypeDeclaration(NailDataTypeDescriptor::Int), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 13, end_column: 15 } },
                        Token { token_type: FunctionReturnTypeDeclaration(NailDataTypeDescriptor::Int), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 16, end_column: 18 } }
                    ]),
                    code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 18 }
                },
                Token { token_type: BlockOpen, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 19, end_column: 20 } },
                Token { token_type: VariableDeclaration, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 21, end_column: 22 } },
                Token { token_type: Identifier("x".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 23, end_column: 24 } },
                Token { token_type: TypeDeclaration(NailDataTypeDescriptor::Int), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 24, end_column: 26 } },
                Token { token_type: Assignment, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 27, end_column: 28 } },
                Token { token_type: Integer("5".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 29, end_column: 30 } },
                Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 30, end_column: 31 } },
                Token { token_type: Return, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 32, end_column: 33 } },
                Token { token_type: Identifier("x".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 34, end_column: 35 } },
                Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 35, end_column: 36 } },
                Token { token_type: BlockClose, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 37, end_column: 38 } }
            ]
        );
    }

    #[test]
    fn test_function_declaration_multiple_params() {
        let input = r#"fn random(x:i, y:f):s { v result:s = `test`; r result; }"#;
        let result = lexer(input);
        println!("RESULT: {:#?}", result);
        assert_eq!(
            result,
            vec![
                Token {
                    token_type: FunctionSignature(vec![
                        Token { token_type: FunctionName("random".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 4, end_column: 10 } },
                        Token { token_type: Identifier("x".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 11, end_column: 12 } },
                        Token { token_type: TypeDeclaration(NailDataTypeDescriptor::Int), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 12, end_column: 14 } },
                        Token { token_type: Comma, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 14, end_column: 15 } },
                        Token { token_type: Identifier("y".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 16, end_column: 17 } },
                        Token { token_type: TypeDeclaration(NailDataTypeDescriptor::Float), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 17, end_column: 19 } },
                        Token { token_type: FunctionReturnTypeDeclaration(NailDataTypeDescriptor::String), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 20, end_column: 22 } }
                    ]),
                    code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 22 }
                },
                Token { token_type: BlockOpen, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 23, end_column: 24 } },
                Token { token_type: VariableDeclaration, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 25, end_column: 26 } },
                Token { token_type: Identifier("result".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 27, end_column: 33 } },
                Token { token_type: TypeDeclaration(NailDataTypeDescriptor::String), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 33, end_column: 35 } },
                Token { token_type: Assignment, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 36, end_column: 37 } },
                Token { token_type: StringLiteral("test".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 38, end_column: 44 } },
                Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 44, end_column: 45 } },
                Token { token_type: Return, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 46, end_column: 47 } },
                Token { token_type: Identifier("result".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 48, end_column: 54 } },
                Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 54, end_column: 55 } },
                Token { token_type: BlockClose, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 56, end_column: 57 } }
            ]
        );
    }

    #[test]
    fn test_lambda() {
        let input = "| x:i |:i { v result:i = x + 1; r result; }";
        let result = lexer(input);
        println!("RESULT: {:#?}", result);
        assert_eq!(
            result,
            vec![
                Token {
                    token_type: LambdaSignature(vec![
                        Token { token_type: Identifier("x".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 3, end_column: 4 } },
                        Token { token_type: TypeDeclaration(NailDataTypeDescriptor::Int), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 4, end_column: 6 } },
                        Token { token_type: LambdaReturnTypeDeclaration(NailDataTypeDescriptor::Int), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 8, end_column: 10 } }
                    ]),
                    code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 10 }
                },
                Token { token_type: BlockOpen, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 11, end_column: 12 } },
                Token { token_type: VariableDeclaration, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 13, end_column: 14 } },
                Token { token_type: Identifier("result".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 15, end_column: 21 } },
                Token { token_type: TypeDeclaration(NailDataTypeDescriptor::Int), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 21, end_column: 23 } },
                Token { token_type: Assignment, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 24, end_column: 25 } },
                Token { token_type: Identifier("x".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 26, end_column: 27 } },
                Token { token_type: Operator(Operation::Add), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 28, end_column: 29 } },
                Token { token_type: Integer("1".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 30, end_column: 31 } },
                Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 31, end_column: 32 } },
                Token { token_type: Return, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 33, end_column: 34 } },
                Token { token_type: Identifier("result".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 35, end_column: 41 } },
                Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 41, end_column: 42 } },
                Token { token_type: BlockClose, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 43, end_column: 44 } }
            ]
        );
    }

    #[test]
    fn test_lambda_multi_param() {
        let input = "| x:i, y:f |:i { v result:i = x + 1; r result; }";
        let result = lexer(input);
        println!("RESULT: {:#?}", result);
        assert_eq!(
            result,
            vec![
                Token {
                    token_type: LambdaSignature(vec![
                        Token { token_type: Identifier("x".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 3, end_column: 4 } },
                        Token { token_type: TypeDeclaration(NailDataTypeDescriptor::Int), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 4, end_column: 6 } },
                        Token { token_type: Comma, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 6, end_column: 7 } },
                        Token { token_type: Identifier("y".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 8, end_column: 9 } },
                        Token { token_type: TypeDeclaration(NailDataTypeDescriptor::Float), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 9, end_column: 11 } },
                        Token { token_type: LambdaReturnTypeDeclaration(NailDataTypeDescriptor::Int), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 13, end_column: 15 } }
                    ]),
                    code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 15 }
                },
                Token { token_type: BlockOpen, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 16, end_column: 17 } },
                Token { token_type: VariableDeclaration, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 18, end_column: 19 } },
                Token { token_type: Identifier("result".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 20, end_column: 26 } },
                Token { token_type: TypeDeclaration(NailDataTypeDescriptor::Int), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 26, end_column: 28 } },
                Token { token_type: Assignment, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 29, end_column: 30 } },
                Token { token_type: Identifier("x".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 31, end_column: 32 } },
                Token { token_type: Operator(Operation::Add), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 33, end_column: 34 } },
                Token { token_type: Integer("1".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 35, end_column: 36 } },
                Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 36, end_column: 37 } },
                Token { token_type: Return, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 38, end_column: 39 } },
                Token { token_type: Identifier("result".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 40, end_column: 46 } },
                Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 46, end_column: 47 } },
                Token { token_type: BlockClose, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 48, end_column: 49 } }
            ]
        );
    }

    #[test]
    fn test_array_declaration_lexing() {
        let result = lexer("v test_array:a:i = [1, 2, 3];");
        println!("RESULT: {:#?}", result);
        assert!(result.eq(&[
            Token { token_type: VariableDeclaration, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 2 } },
            Token { token_type: Identifier("test_array".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 3, end_column: 13 } },
            Token { token_type: TypeDeclaration(NailDataTypeDescriptor::ArrayInt), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 13, end_column: 17 } },
            Token { token_type: Assignment, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 18, end_column: 19 } },
            Token {
                token_type: Array(vec![
                    Token { token_type: Integer("1".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 21, end_column: 22 } },
                    Token { token_type: Integer("2".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 24, end_column: 25 } },
                    Token { token_type: Integer("3".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 27, end_column: 28 } },
                ]),
                code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 20, end_column: 29 }
            },
            Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 29, end_column: 30 } }
        ]));
    }

    #[test]
    fn test_array_of_point_structs() {
        let input = r#"
struct Point { x:i, y:i }
v points:a:struct:Point = [Point { x: 1, y: 2 }, Point { x: 3, y: 4 }];
"#;
        let result = lexer(input);
        println!("RESULT: {:#?}", result);

        assert_eq!(
            result,
            [
                Token {
                    token_type: StructDeclaration(StructDeclarationData {
                        name: "Point".to_string(),
                        fields: vec![
                            StructDeclarationDataField { name: "x".to_string(), data_type: NailDataTypeDescriptor::Int },
                            StructDeclarationDataField { name: "y".to_string(), data_type: NailDataTypeDescriptor::Int }
                        ]
                    }),
                    code_span: CodeSpan { start_line: 2, end_line: 2, start_column: 1, end_column: 26 }
                },
                Token { token_type: VariableDeclaration, code_span: CodeSpan { start_line: 3, end_line: 3, start_column: 1, end_column: 2 } },
                Token { token_type: Identifier("points".to_string()), code_span: CodeSpan { start_line: 3, end_line: 3, start_column: 3, end_column: 9 } },
                Token { token_type: TypeDeclaration(NailDataTypeDescriptor::ArrayStruct("Point".to_string())), code_span: CodeSpan { start_line: 3, end_line: 3, start_column: 9, end_column: 24 } },
                Token { token_type: Assignment, code_span: CodeSpan { start_line: 3, end_line: 3, start_column: 25, end_column: 26 } },
                Token {
                    token_type: Array(vec![
                        Token {
                            token_type: StructInstantiation(StructInstantiationData {
                                name: "Point".to_string(),
                                fields: vec![
                                    StructInstantiationDataField {
                                        name: "x".to_string(),
                                        value: Token { token_type: Integer("1".to_string()), code_span: CodeSpan { start_line: 3, end_line: 3, start_column: 39, end_column: 40 } }
                                    },
                                    StructInstantiationDataField {
                                        name: "y".to_string(),
                                        value: Token { token_type: Integer("2".to_string()), code_span: CodeSpan { start_line: 3, end_line: 3, start_column: 45, end_column: 46 } }
                                    }
                                ]
                            }),
                            code_span: CodeSpan { start_line: 3, end_line: 3, start_column: 28, end_column: 48 }
                        },
                        Token {
                            token_type: StructInstantiation(StructInstantiationData {
                                name: "Point".to_string(),
                                fields: vec![
                                    StructInstantiationDataField {
                                        name: "x".to_string(),
                                        value: Token { token_type: Integer("3".to_string()), code_span: CodeSpan { start_line: 3, end_line: 3, start_column: 61, end_column: 62 } }
                                    },
                                    StructInstantiationDataField {
                                        name: "y".to_string(),
                                        value: Token { token_type: Integer("4".to_string()), code_span: CodeSpan { start_line: 3, end_line: 3, start_column: 67, end_column: 68 } }
                                    }
                                ]
                            }),
                            code_span: CodeSpan { start_line: 3, end_line: 3, start_column: 50, end_column: 70 }
                        }
                    ]),
                    code_span: CodeSpan { start_line: 3, end_line: 3, start_column: 27, end_column: 71 }
                },
                Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 3, end_line: 3, start_column: 71, end_column: 72 } }
            ]
        );
    }

    #[test]
    fn test_struct_declaration_lexing() {
        let result = lexer("struct Point { x:i, y:i }");
        println!("RESULT: {:#?}", result);
        assert_eq!(
            result,
            [Token {
                token_type: StructDeclaration(StructDeclarationData {
                    name: "Point".to_string(),
                    fields: vec![
                        StructDeclarationDataField { name: "x".to_string(), data_type: NailDataTypeDescriptor::Int },
                        StructDeclarationDataField { name: "y".to_string(), data_type: NailDataTypeDescriptor::Int }
                    ]
                }),
                code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 26 }
            }]
        );
    }

    #[test]
    fn test_enum_assignment_lexing() {
        let result = lexer("v color:enum:Color = Color::Red;");
        println!("RESULT: {:#?}", result);
        assert!(result.eq(&[
            Token { token_type: VariableDeclaration, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 2 } },
            Token { token_type: Identifier("color".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 3, end_column: 8 } },
            Token { token_type: TypeDeclaration(NailDataTypeDescriptor::Enum("Color".to_string())), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 8, end_column: 19 } },
            Token { token_type: Assignment, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 20, end_column: 21 } },
            Token {
                token_type: EnumVariant(EnumVariantData { name: "Color".to_string(), variant: "Red".to_string() }),
                code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 22, end_column: 32 }
            },
            Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 32, end_column: 33 } },
        ]));
    }

    #[test]
    fn test_struct_assignment_lexing() {
        let result = lexer("v point:struct:Point = Point { x: 10, y: 20 };");
        println!("RESULT: {:#?}", result);
        assert!(result.eq(&[
            Token { token_type: VariableDeclaration, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 2 } },
            Token { token_type: Identifier("point".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 3, end_column: 8 } },
            Token { token_type: TypeDeclaration(NailDataTypeDescriptor::Struct("Point".to_string())), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 8, end_column: 21 } },
            Token { token_type: Assignment, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 22, end_column: 23 } },
            Token {
                token_type: StructInstantiation(StructInstantiationData {
                    name: "Point".to_string(),
                    fields: vec![
                        StructInstantiationDataField {
                            name: "x".to_string(),
                            value: Token { token_type: Integer("10".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 35, end_column: 37 } }
                        },
                        StructInstantiationDataField {
                            name: "y".to_string(),
                            value: Token { token_type: Integer("20".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 42, end_column: 44 } }
                        },
                    ],
                }),
                code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 24, end_column: 46 }
            },
            Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 46, end_column: 47 } },
        ]));
    }

    #[test]
    fn test_struct_dot_get_field_notation() {
        let result = lexer("v point_on_x_struct:i = point.x;");
        println!("RESULT: {:#?}", result);
        assert!(result.eq(&[
            Token { token_type: VariableDeclaration, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 2 } },
            Token { token_type: Identifier("point_on_x_struct".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 3, end_column: 20 } },
            Token { token_type: TypeDeclaration(NailDataTypeDescriptor::Int), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 20, end_column: 22 } },
            Token { token_type: Assignment, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 23, end_column: 24 } },
            Token { token_type: StructFieldAccess("point".to_string(), "x".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 25, end_column: 32 } },
            Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 32, end_column: 33 } },
        ]));
    }

    #[test]
    fn test_enum_lexing() {
        let input = "enum Color { Red, Green, Blue }";
        let tokens = lexer(input);
        println!("RESULT: {:#?}", tokens);
        assert!(tokens.eq(&[Token {
            token_type: EnumDeclaration(EnumDeclarationData {
                name: "Color".to_string(),
                variants: vec![
                    Token {
                        token_type: EnumVariant(EnumVariantData { name: "Color".to_string(), variant: "Red".to_string() }),
                        code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 14, end_column: 17 }
                    },
                    Token {
                        token_type: EnumVariant(EnumVariantData { name: "Color".to_string(), variant: "Green".to_string() }),
                        code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 19, end_column: 24 }
                    },
                    Token {
                        token_type: EnumVariant(EnumVariantData { name: "Color".to_string(), variant: "Blue".to_string() }),
                        code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 26, end_column: 30 }
                    },
                ],
            }),
            code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 32 }
        },]));
    }

    #[test]
    fn test_simple_ident() {
        let result = lexer(SIMPLE_IDENT);
        println!("RESULT: {:#?}", result);
        assert!(result.eq(&[
            Token { token_type: Identifier("bob".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 4 } },
            Token { token_type: TypeDeclaration(NailDataTypeDescriptor::Int), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 4, end_column: 6 } },
        ]));
    }

    #[test]
    fn test_any_of_type() {
        let result = lexer(
            r#"
        c every_nail_type:any(i|f|s|b|a:i|a:f|a:struct:any|a:enum:any);

"#,
        );
        println!("RESULT: {:#?}", result);
        assert!(result.eq(&[
            Token { token_type: ConstDeclaration, code_span: CodeSpan { start_line: 2, end_line: 2, start_column: 9, end_column: 10 } },
            Token { token_type: Identifier("every_nail_type".to_string()), code_span: CodeSpan { start_line: 2, end_line: 2, start_column: 11, end_column: 26 } },
            Token {
                token_type: TypeDeclaration(NailDataTypeDescriptor::Any(vec![
                    NailDataTypeDescriptor::Int,
                    NailDataTypeDescriptor::Float,
                    NailDataTypeDescriptor::String,
                    NailDataTypeDescriptor::Boolean,
                    NailDataTypeDescriptor::ArrayInt,
                    NailDataTypeDescriptor::ArrayFloat,
                    NailDataTypeDescriptor::ArrayStruct("any".to_string()),
                    NailDataTypeDescriptor::ArrayEnum("any".to_string()),
                ])),
                code_span: CodeSpan { start_line: 2, end_line: 2, start_column: 26, end_column: 71 }
            },
            Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 2, end_line: 2, start_column: 71, end_column: 72 } },
        ]));
    }

    #[test]
    fn test_const_assignment() {
        let result = lexer(CONST_ASSIGNMENT);
        println!("RESULT: {:#?}", result);
        assert!(result.eq(&[
            Token { token_type: ConstDeclaration, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 2 } },
            Token { token_type: Identifier("x".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 3, end_column: 4 } },
            Token { token_type: TypeDeclaration(NailDataTypeDescriptor::Int), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 4, end_column: 6 } },
            Token { token_type: Assignment, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 7, end_column: 8 } },
            Token { token_type: Integer("10".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 9, end_column: 11 } },
            Token { token_type: EndStatementOrExpression, code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 11, end_column: 12 } },
        ]));
    }

    #[test]
    fn test_multiline_string() {
        let result = lexer(MULTILINE_STRING);
        println!("RESULT: {:#?}", result);
        assert!(result.eq(&[Token {
            token_type: StringLiteral(
                "This is a story all about how my life\ngot flipped turned upside down, and I'd like to take a minute just sit right\nthere, I'll tell you how I became the\nprince of a town called Bel-Air.".to_string(),
            ),
            code_span: CodeSpan {  start_line: 1,
            end_line: 4,
            start_column: 1,
            end_column: 34,}
        },]));
    }

    #[test]
    fn test_rust_escape_hatch() {
        let result = lexer(RUST_ESCAPE);
        println!("RESULT: {:#?}", result);
        assert!(result.eq(&[Token {
            token_type: RustEscape(vec![Token { token_type: RustLiteral(" let x:i = 10; ".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 19 } }]),
            code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 19 }
        }]));
    }

    #[test]
    fn test_rust_escape_hatch_multiline() {
        let result = lexer(RUST_ESCAPE_MULTILINE);
        println!("RESULT: {:#?}", result);
        assert!(result.eq(&[Token {
            token_type: RustEscape(vec![Token { token_type: RustLiteral("\nx:i = 10;\n".to_string()), code_span: CodeSpan { start_line: 1, end_line: 3, start_column: 1, end_column: 2 } }]),
            code_span: CodeSpan { start_line: 1, end_line: 3, start_column: 1, end_column: 2 }
        }]));
    }

    #[test]
    fn test_rust_escape_nested_blocks() {
        let result = lexer(RUST_ESCAPE_NESTED_BLOCKS);
        println!("RESULT: {:#?}", result);
        assert!(result.eq(&[Token {
            token_type: RustEscape(vec![Token { token_type: RustLiteral(" { let x:i = 10; } ".to_string()), code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 23 } }]),
            code_span: CodeSpan { start_line: 1, end_line: 1, start_column: 1, end_column: 23 }
        }]));
    }

    #[test]
    fn test_rust_escape_nail_injection() {
        let result = lexer(RUST_ESCAPE_NAIL_INJECTION);
        println!("RESULT: {:#?}", result);
        assert_eq!(result.len(), 1);
        if let TokenType::RustEscape(content) = &result[0].token_type {
            assert_eq!(content.len(), 5);
            assert!(matches!(&content[0].token_type, TokenType::RustLiteral(s) if s == " println!(\"Hello, "));
            if let TokenType::RustNailInsert(nail_tokens) = &content[1].token_type {
                assert_eq!(nail_tokens.len(), 1);
                assert!(matches!(&nail_tokens[0].token_type, TokenType::Identifier(s) if s == "name"));
            } else {
                panic!("Expected RustNailInsert");
            }
            assert!(matches!(&content[2].token_type, TokenType::RustLiteral(s) if s == "! You are "));
            if let TokenType::RustNailInsert(nail_tokens) = &content[3].token_type {
                assert_eq!(nail_tokens.len(), 3);
                assert!(matches!(&nail_tokens[0].token_type, TokenType::Integer(s) if s == "18"));
                assert!(matches!(&nail_tokens[1].token_type, TokenType::Operator(Operation::Add)));
                assert!(matches!(&nail_tokens[2].token_type, TokenType::Integer(s) if s == "8"));
            } else {
                panic!("Expected RustNailInsert");
            }
            assert!(matches!(&content[4].token_type, TokenType::RustLiteral(s) if s == " years old.\"); "));
        } else {
            panic!("Expected RustEscape token");
        }
    }
}
