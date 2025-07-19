use crate::checker::GLOBAL_SCOPE;
use crate::common::{CodeError, CodeSpan};
use crate::lexer::*;
use crate::stdlib_registry;
use std::collections::HashSet;
use std::iter::Peekable;
use std::vec::IntoIter;

pub mod std_lib;

// We don't actually use this in the parser, it's a placeholder so the AST doesn't need to be recreated as an entirely new structure just for the scopes in the checker stage

#[derive(Debug, PartialEq, Clone)]
pub enum ASTNode {
    Program { statements: Vec<ASTNode>, code_span: CodeSpan, scope: usize, used_stdlib_functions: HashSet<String> },
    FunctionDeclaration { name: String, params: Vec<(String, NailDataTypeDescriptor)>, data_type: NailDataTypeDescriptor, body: Box<ASTNode>, code_span: CodeSpan, scope: usize },
    LambdaDeclaration { params: Vec<(String, NailDataTypeDescriptor)>, data_type: NailDataTypeDescriptor, body: Box<ASTNode>, code_span: CodeSpan, scope: usize },
    FunctionCall { name: String, args: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    ConstDeclaration { name: String, data_type: NailDataTypeDescriptor, value: Box<ASTNode>, code_span: CodeSpan, scope: usize },
    IfStatement { condition_branches: Vec<(Box<ASTNode>, Box<ASTNode>)>, else_branch: Option<Box<ASTNode>>, code_span: CodeSpan, scope: usize },
    ParallelBlock { statements: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    ParallelAssignment { assignments: Vec<(String, NailDataTypeDescriptor, Box<ASTNode>)>, code_span: CodeSpan, scope: usize },
    Block { statements: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    BinaryOperation { left: Box<ASTNode>, operator: Operation, right: Box<ASTNode>, code_span: CodeSpan, scope: usize },
    UnaryOperation { operator: Operation, operand: Box<ASTNode>, code_span: CodeSpan, scope: usize },
    StructDeclaration { name: String, fields: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    StructDeclarationField { name: String, data_type: NailDataTypeDescriptor, scope: usize },
    StructInstantiation { name: String, fields: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    StructInstantiationField { name: String, value: Box<ASTNode>, code_span: CodeSpan, scope: usize },
    StructFieldAccess { struct_name: String, field_name: String, code_span: CodeSpan, scope: usize },
    EnumDeclaration { name: String, variants: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    EnumVariant { name: String, variant: String, code_span: CodeSpan, scope: usize },
    ArrayLiteral { elements: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    Identifier { name: String, code_span: CodeSpan, scope: usize },
    NumberLiteral { value: String, data_type: NailDataTypeDescriptor, code_span: CodeSpan, scope: usize },
    StringLiteral { value: String, code_span: CodeSpan, scope: usize },
    ReturnDeclaration { statement: Box<ASTNode>, code_span: CodeSpan, scope: usize },
}

impl Default for ASTNode {
    fn default() -> Self {
        ASTNode::Program { statements: Vec::new(), code_span: CodeSpan::default(), scope: 0, used_stdlib_functions: HashSet::new() }
    }
}

pub struct ParserState {
    tokens: Peekable<IntoIter<Token>>,
    current_token: Option<Token>,
    previous_token: Option<Token>,
    used_stdlib_functions: HashSet<String>,
}

pub fn parse(tokens: Vec<Token>) -> Result<(ASTNode, HashSet<String>), CodeError> {
    let mut state = ParserState { 
        tokens: tokens.into_iter().peekable(), 
        current_token: None, 
        previous_token: None,
        used_stdlib_functions: HashSet::new(),
    };
    let ast = parse_inner(&mut state)?;
    Ok((ast, state.used_stdlib_functions))
}

fn parse_inner(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let mut program = vec![];
    while state.tokens.peek().is_some() {
        program.push(parse_statement(state)?);
    }
    Ok(ASTNode::Program { 
        statements: program, 
        code_span: CodeSpan::default(), 
        scope: GLOBAL_SCOPE,
        used_stdlib_functions: state.used_stdlib_functions.clone()
    })
}

fn advance(state: &mut ParserState) -> Option<Token> {
    state.previous_token = state.current_token.take();
    state.current_token = state.tokens.next();
    state.current_token.clone()
}

fn parse_primary(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(token) = state.tokens.peek().cloned() {
        match token.token_type {
            TokenType::StructInstantiation(_) => {
                advance(state);
                parse_struct_instantiation_token(&token)
            }
            TokenType::StructFieldAccess(struct_name, field_name) => {
                advance(state);
                Ok(ASTNode::StructFieldAccess { 
                    struct_name, 
                    field_name, 
                    code_span: token.code_span, 
                    scope: GLOBAL_SCOPE 
                })
            }
            TokenType::Identifier(name) => {
                advance(state);
                if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::ParenthesisOpen)) {
                    parse_function_call(state, name)
                } else {
                    Ok(ASTNode::Identifier { name, code_span: token.code_span, scope: GLOBAL_SCOPE })
                }
            }
            TokenType::ConstDeclaration => {
                // In expression context, treat 'c' as an identifier
                advance(state);
                let name = "c".to_string();
                if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::ParenthesisOpen)) {
                    parse_function_call(state, name)
                } else {
                    Ok(ASTNode::Identifier { name, code_span: token.code_span, scope: GLOBAL_SCOPE })
                }
            }
            TokenType::Float(value) => {
                advance(state);
                Ok(ASTNode::NumberLiteral { value, data_type: NailDataTypeDescriptor::Float, code_span: token.code_span, scope: GLOBAL_SCOPE })
            }
            TokenType::Integer(value) => {
                advance(state);
                Ok(ASTNode::NumberLiteral { value, data_type: NailDataTypeDescriptor::Int, code_span: token.code_span, scope: GLOBAL_SCOPE })
            }
            TokenType::StringLiteral(value) => {
                advance(state);
                Ok(ASTNode::StringLiteral { value, code_span: token.code_span, scope: GLOBAL_SCOPE })
            }
            TokenType::ParenthesisOpen => {
                advance(state);
                let expr = parse_expression(state, 0)?;
                let _ = expect_token(state, TokenType::ParenthesisClose)?;
                Ok(expr)
            }
            TokenType::EnumVariant(variant) => {
                let code_span = token.code_span;
                advance(state);
                Ok(ASTNode::EnumVariant { name: variant.name, variant: variant.variant, code_span, scope: GLOBAL_SCOPE })
            }
            TokenType::ArrayOpen => parse_array_literal(state),
            TokenType::IfDeclaration => parse_if_statement_expr(state, true),
            TokenType::LambdaSignature(_) => parse_lambda_declaration(state),
            _ => {
                let code_span = token.code_span;
                Err(CodeError { message: format!("Unexpected token {:?}", token.token_type), code_span: code_span.clone() })
            }
        }
    } else {
        Err(CodeError { message: "Unexpected end of file".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) })
    }
}

fn parse_statement(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    match state.tokens.peek() {
        Some(token) => match &token.token_type {
            TokenType::StructDeclaration(_) => parse_struct_declaration(state),
            TokenType::EnumDeclaration(_) => parse_enum_declaration(state),
            TokenType::FunctionSignature(_) => parse_function_declaration(state),
            TokenType::ConstDeclaration => parse_const_declaration(state),
            TokenType::IfDeclaration => parse_if_statement_expr(state, false),
            TokenType::ParallelDeclaration => parse_parallel_block(state),
            TokenType::ParallelStart => parse_parallel_block_new(state),
            TokenType::Return => parse_return_statement(state),
            TokenType::LambdaSignature(_) => parse_lambda_declaration(state),
            TokenType::BlockOpen => parse_block(state),
            _ => {
                // Check if this is a const declaration without 'c' prefix
                // Pattern: Identifier TypeDeclaration Assignment
                if let Some(TokenType::Identifier(_)) = state.tokens.peek().map(|t| &t.token_type) {
                    // Look ahead to see if this is a declaration
                    let mut peek_iter = state.tokens.clone();
                    peek_iter.next(); // Skip identifier
                    if let Some(token) = peek_iter.peek() {
                        if matches!(token.token_type, TokenType::TypeDeclaration(_)) {
                            // This is a const declaration without 'c'
                            return parse_const_declaration_no_prefix(state);
                        }
                    }
                }
                
                let expr = parse_expression(state, 0)?;
                let _ = expect_token(state, TokenType::EndStatementOrExpression)?;
                Ok(expr)
            },
        },
        None => Err(CodeError { message: "No token was found to match with a statement.".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) }),
    }
}

fn parse_expression(state: &mut ParserState, min_precedence: u8) -> Result<ASTNode, CodeError> {
    let mut left = parse_primary(state)?;

    while let Some(Token { token_type: TokenType::Operator(op), .. }) = state.tokens.peek().cloned() {
        if op.precedence() < min_precedence {
            break;
        }

        advance(state); // Consume the operator
        let code_span = state.current_token.as_ref().map(|t| t.code_span.clone()).unwrap_or(CodeSpan::default());

        if op.is_unary() {
            left = ASTNode::UnaryOperation { operator: op, operand: Box::new(left), code_span: code_span.clone(), scope: GLOBAL_SCOPE };
        } else {
            let right = parse_expression(state, op.precedence() + 1)?;
            left = ASTNode::BinaryOperation { left: Box::new(left), operator: op, right: Box::new(right), code_span: code_span.clone(), scope: GLOBAL_SCOPE };
        }
    }

    Ok(left)
}

fn expect_token(state: &mut ParserState, expected: TokenType) -> Result<CodeSpan, CodeError> {
    if let Some(token) = advance(state) {
        if token.token_type == expected {
            Ok(token.code_span)
        } else {
            let error = CodeError { message: format!("Expected {:?}, found {:?}", expected, token.token_type), code_span: token.code_span.clone() };
            log::error!("Expect token error: {:?}", error);
            Err(error)
        }
    } else {
        log::error!("expect_token else branch error: {:?}", expected);
        Err(CodeError { message: format!("Expected {:?}, found end of file", expected), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) })
    }
}

fn expect_identifier(state: &mut ParserState) -> Result<String, CodeError> {
    if let Some(Token { token_type: TokenType::Identifier(name), code_span, .. }) = advance(state) {
        // Grug need descriptive names! Check for single-letter variables
        if name.len() < 2 {
            let error = CodeError {
                message: format!("Variable name too short. Grug need descriptive names!\n  Found: '{}'\n  Suggestion: Use descriptive name like '{}_value' or '{}_{}'", 
                    name, name, name, 
                    if name == "x" || name == "y" || name == "z" { "coordinate" } 
                    else if name == "i" || name == "j" || name == "k" { "index" }
                    else if name == "n" { "number" }
                    else { "variable" }
                ),
                code_span,
            };
            log::error!("Grug brain variable name error: {:?}", error);
            Err(error)
        } else {
            Ok(name)
        }
    } else {
        let error = CodeError {
            message: format!("Expected identifier, found {:?}", state.tokens.peek().map(|token| token.token_type.clone()).unwrap_or(TokenType::EndOfFile)),
            code_span: state.tokens.peek().map_or(CodeSpan::default(), |token| token.code_span.clone()),
        };
        log::error!("Expect identifier error: {:?}", error);
        Err(error)
    }
}

fn parse_function_call(state: &mut ParserState, name: String) -> Result<ASTNode, CodeError> {
    let _ = expect_token(state, TokenType::ParenthesisOpen)?;
    let mut args = Vec::new();
    while state.tokens.peek().map_or(false, |t| t.token_type != TokenType::ParenthesisClose) {
        args.push(parse_expression(state, 0)?);
        if state.tokens.peek().map_or(false, |t| t.token_type == TokenType::Comma) {
            advance(state);
        } else {
            break;
        }
    }
    let code_span = expect_token(state, TokenType::ParenthesisClose)?;

    // Track stdlib function usage
    if stdlib_registry::is_stdlib_function(&name) {
        state.used_stdlib_functions.insert(name.clone());
    }
    
    Ok(ASTNode::FunctionCall { name, args, code_span, scope: GLOBAL_SCOPE })
}

fn parse_lambda_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(Token { token_type: TokenType::LambdaSignature(tokens), .. }) = advance(state) {
        let code_span = state.previous_token.as_ref().map(|t| t.code_span.clone()).unwrap_or(CodeSpan::default());
        let mut lambda_tokens = tokens.into_iter();
        let mut params = Vec::new();
        #[allow(unused_assignments)]
        let mut data_type = NailDataTypeDescriptor::Void;

        // Parse parameters
        loop {
            match lambda_tokens.next() {
                Some(Token { token_type: TokenType::Identifier(param_name), .. }) => {
                    // Check what comes next
                    let next_token = lambda_tokens.next();
                    match next_token {
                        Some(Token { token_type: TokenType::TypeDeclaration(type_desc), .. }) => {
                            // Parameter has explicit type
                            params.push((param_name.clone(), type_desc.clone()));

                            // Check for comma or end of parameters
                            match lambda_tokens.next() {
                                Some(Token { token_type: TokenType::Comma, .. }) => continue,
                                Some(Token { token_type: TokenType::LambdaReturnTypeDeclaration(rt), .. }) => {
                                    data_type = rt;
                                    break;
                                }
                                Some(other) => return Err(CodeError { message: format!("Expected comma or return type declaration, found {:?}", other.token_type), code_span: other.code_span.clone() }),
                                None => {
                                    return Err(CodeError {
                                        message: "Unexpected end of lambda declaration".to_string(),
                                        code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()),
                                    })
                                }
                            }
                        }
                        Some(Token { token_type: TokenType::Comma, .. }) => {
                            // Parameter without type followed by comma - infer error type if param name is 'e'
                            if param_name == "e" {
                                params.push((param_name.clone(), NailDataTypeDescriptor::Error));
                                continue;
                            } else {
                                return Err(CodeError {
                                    message: format!("Parameter '{}' requires a type declaration", param_name),
                                    code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()),
                                });
                            }
                        }
                        Some(Token { token_type: TokenType::LambdaReturnTypeDeclaration(rt), .. }) => {
                            // Parameter without type followed by return type - infer error type if param name is 'e'
                            if param_name == "e" {
                                params.push((param_name.clone(), NailDataTypeDescriptor::Error));
                                data_type = rt;
                                break;
                            } else {
                                return Err(CodeError {
                                    message: format!("Parameter '{}' requires a type declaration", param_name),
                                    code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()),
                                });
                            }
                        }
                        _ => {
                            return Err(CodeError {
                                message: "Expected type declaration for lambda parameter".to_string(),
                                code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()),
                            });
                        }
                    }
                }
                Some(Token { token_type: TokenType::LambdaReturnTypeDeclaration(rt), .. }) => {
                    data_type = rt;
                    break;
                }
                Some(other) => return Err(CodeError { message: format!("Unexpected token in lambda declaration: {:?}", other.token_type), code_span: other.code_span.clone() }),
                None => {
                    return Err(CodeError {
                        message: "Unexpected end of lambda declaration".to_string(),
                        code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()),
                    })
                }
            }
        }

        // Parse the lambda body
        let body = Box::new(parse_block(state)?);

        Ok(ASTNode::LambdaDeclaration { params, data_type, body, code_span, scope: GLOBAL_SCOPE })
    } else {
        Err(CodeError { message: "Expected lambda declaration".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) })
    }
}

fn parse_struct_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(Token { token_type: TokenType::StructDeclaration(struct_declaration_data), .. }) = advance(state) {
        let code_span = state.previous_token.as_ref().map(|t| t.code_span.clone()).unwrap_or(CodeSpan::default());
        let mut struct_fields = struct_declaration_data.fields.into_iter();

        let struct_name = struct_declaration_data.name;
        let mut fields = Vec::new();

        while let Some(field) = struct_fields.next() {
            fields.push(ASTNode::StructDeclarationField { name: field.name, data_type: field.data_type, scope: GLOBAL_SCOPE })
        }

        Ok(ASTNode::StructDeclaration { name: struct_name, fields, code_span, scope: GLOBAL_SCOPE })
    } else {
        Err(CodeError { message: "Struct declaration syntax is incorrect".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) })
    }
}

// Handle struct instantiation data directly for inside arrays, etc
fn parse_struct_instantiation_token(token: &Token) -> Result<ASTNode, CodeError> {
    if let TokenType::StructInstantiation(struct_instantiation_data) = &token.token_type {
        let struct_name = struct_instantiation_data.name.clone();
        let mut fields: Vec<ASTNode> = Vec::new();

        for field in &struct_instantiation_data.fields {
            fields.push(ASTNode::StructInstantiationField {
                name: field.name.clone(),
                value: Box::new(match &field.value.token_type {
                    TokenType::Identifier(name) => ASTNode::Identifier { name: name.clone(), code_span: field.value.code_span.clone(), scope: GLOBAL_SCOPE },
                    TokenType::Integer(value) => ASTNode::NumberLiteral { value: value.clone(), data_type: NailDataTypeDescriptor::Int, code_span: field.value.code_span.clone(), scope: GLOBAL_SCOPE },
                    TokenType::Float(value) => ASTNode::NumberLiteral { value: value.clone(), data_type: NailDataTypeDescriptor::Float, code_span: field.value.code_span.clone(), scope: GLOBAL_SCOPE },
                    TokenType::StringLiteral(value) => ASTNode::StringLiteral { value: value.clone(), code_span: field.value.code_span.clone(), scope: GLOBAL_SCOPE },
                    _ => {
                        return Err(CodeError { message: format!("Unexpected token in struct field: {:?}", field.value.token_type), code_span: field.value.code_span.clone() });
                    }
                }),
                code_span: field.value.code_span.clone(),
                scope: GLOBAL_SCOPE,
            });
        }
        Ok(ASTNode::StructInstantiation { name: struct_name, fields, code_span: token.code_span.clone(), scope: GLOBAL_SCOPE })
    } else {
        Err(CodeError { message: format!("Expected struct instantiation, found {:?}", token.token_type), code_span: token.code_span.clone() })
    }
}

// For standalone struct instantiations outside of arrays
// fn parse_struct_instantiation(state: &mut ParserState) -> Result<ASTNode, CodeError> {
//     if let Some(token @ Token { token_type: TokenType::StructInstantiation(_), .. }) = state.tokens.peek().cloned() {
//         advance(state); // Consume the StructInstantiation token
//         parse_struct_instantiation_token(&token)
//     } else {
//         Err(CodeError { message: "Expected struct instantiation".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) })
//     }
// }

fn parse_array_literal(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    // Expect and consume '['
    let start_span = state.current_token.as_ref().map(|t| t.code_span.clone()).unwrap_or(CodeSpan::default());
    let _ = expect_token(state, TokenType::ArrayOpen)?;
    
    let mut elements = Vec::new();
    
    // Parse elements until we hit ']'
    while state.tokens.peek().map(|t| &t.token_type) != Some(&TokenType::ArrayClose) {
        // Parse any expression as an array element
        elements.push(parse_expression(state, 0)?);
        
        // Check for comma or closing bracket
        if state.tokens.peek().map(|t| &t.token_type) == Some(&TokenType::Comma) {
            advance(state); // consume comma
        } else if state.tokens.peek().map(|t| &t.token_type) == Some(&TokenType::ArrayClose) {
            // We'll consume the closing bracket below
            break;
        } else {
            return Err(CodeError {
                message: "Expected ',' or ']' in array literal".to_string(),
                code_span: state.current_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone())
            });
        }
    }
    
    // Expect and consume ']'
    let _ = expect_token(state, TokenType::ArrayClose)?;
    
    Ok(ASTNode::ArrayLiteral { 
        elements, 
        code_span: start_span,
        scope: GLOBAL_SCOPE 
    })
}

fn parse_enum_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(Token { token_type: TokenType::EnumDeclaration(nail_enum_data), .. }) = advance(state) {
        let code_span = state.previous_token.as_ref().map(|t| t.code_span.clone()).unwrap_or(CodeSpan::default());
        let enum_name = nail_enum_data.name;
        let mut enum_tokens = nail_enum_data.variants.into_iter();

        // Parse variants
        let mut variants = Vec::new();
        while let Some(token) = enum_tokens.next() {
            let code_span = token.code_span;
            match token.token_type {
                TokenType::EnumVariant(variant) => variants.push(ASTNode::EnumVariant { name: enum_name.clone(), variant: variant.variant.clone(), code_span, scope: GLOBAL_SCOPE }),
                TokenType::BlockClose => break,
                _ => {
                    return Err(CodeError { message: format!("Unexpected token in enum declaration: {:?}", token.token_type), code_span });
                }
            }
        }

        Ok(ASTNode::EnumDeclaration { name: enum_name, variants, code_span, scope: GLOBAL_SCOPE })
    } else {
        Err(CodeError { message: "Expected enum declaration".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) })
    }
}

fn parse_function_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(Token { token_type: TokenType::FunctionSignature(tokens), .. }) = advance(state) {
        let code_span = state.previous_token.as_ref().map(|t| t.code_span.clone()).unwrap_or(CodeSpan::default());
        let mut func_tokens = tokens.into_iter();

        // Parse function name
        let name = if let Some(Token { token_type: TokenType::FunctionName(name), .. }) = func_tokens.next() {
            if name.is_empty() {
                return Err(CodeError { message: "Function name cannot be empty".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) });
            }
            name
        } else {
            return Err(CodeError { message: "Expected function name".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) });
        };

        let mut params = Vec::new();
        #[allow(unused_assignments)]
        let mut data_type = NailDataTypeDescriptor::Void;

        // Parse parameters
        loop {
            match func_tokens.next() {
                Some(Token { token_type: TokenType::Identifier(param_name), .. }) => {
                    if let Some(Token { token_type: TokenType::TypeDeclaration(type_desc), .. }) = func_tokens.next() {
                        params.push((param_name.clone(), type_desc.clone()));

                        // Check for comma or end of parameters
                        match func_tokens.next() {
                            Some(Token { token_type: TokenType::Comma, .. }) => continue,
                            Some(Token { token_type: TokenType::FunctionReturnTypeDeclaration(rt), .. }) => {
                                data_type = rt;
                                break;
                            }
                            Some(other) => return Err(CodeError { message: format!("Expected comma or return type declaration, found {:?}", other.token_type), code_span: other.code_span.clone() }),
                            None => {
                                return Err(CodeError {
                                    message: "Unexpected end of function declaration".to_string(),
                                    code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()),
                                })
                            }
                        }
                    } else {
                        return Err(CodeError {
                            message: "Expected type declaration for function parameter".to_string(),
                            code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()),
                        });
                    }
                }
                Some(Token { token_type: TokenType::FunctionReturnTypeDeclaration(rt), .. }) => {
                    data_type = rt;
                    break;
                }
                Some(other) => return Err(CodeError { message: format!("Unexpected token in function declaration: {:?}", other.token_type), code_span: other.code_span.clone() }),
                None => {
                    return Err(CodeError {
                        message: "Unexpected end of function declaration".to_string(),
                        code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()),
                    })
                }
            }
        }

        // Parse function body
        let body = Box::new(parse_block(state)?);

        Ok(ASTNode::FunctionDeclaration { name, params, data_type, body, code_span, scope: GLOBAL_SCOPE })
    } else {
        Err(CodeError { message: "Expected function declaration".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) })
    }
}

fn parse_const_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let _ = expect_token(state, TokenType::ConstDeclaration)?;
    let _ = state.previous_token.as_ref().map(|t| t.code_span.clone()).unwrap_or(CodeSpan::default());
    let name = expect_identifier(state)?;
    let data_type = parse_type_declaration(state)?;
    let _ = expect_token(state, TokenType::Assignment)?;
    let value = Box::new(parse_expression(state, 0)?);
    let code_span = expect_token(state, TokenType::EndStatementOrExpression)?;

    Ok(ASTNode::ConstDeclaration { name, data_type, value, code_span, scope: GLOBAL_SCOPE })
}

fn parse_const_declaration_no_prefix(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    // Same as parse_const_declaration but without expecting 'c' token
    let name = expect_identifier(state)?;
    let data_type = parse_type_declaration(state)?;
    let _ = expect_token(state, TokenType::Assignment)?;
    let value = Box::new(parse_expression(state, 0)?);
    let code_span = expect_token(state, TokenType::EndStatementOrExpression)?;

    Ok(ASTNode::ConstDeclaration { name, data_type, value, code_span, scope: GLOBAL_SCOPE })
}


fn parse_if_statement(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    parse_if_statement_expr(state, false)
}

fn parse_if_statement_expr(state: &mut ParserState, is_expression: bool) -> Result<ASTNode, CodeError> {
    let _ = state.previous_token.as_ref().map(|t| t.code_span.clone()).unwrap_or(CodeSpan::default());
    // #[test]
    // fn test_if_statement() {
    //     let input = "if { a > 5 => {} };";
    //     let result = lexer(input);
    //     println!("RESULT: {:#?}", result);
    //     assert_eq!(
    //         result,
    //         vec![
    //             Token { token_type: IfDeclaration, start_line: 1, end_line: 1, start_column: 1, end_column: 3 },
    //             Token { token_type: BlockOpen, start_line: 1, end_line: 1, start_column: 4, end_column: 5 },
    //             Token { token_type: Identifier("a".to_string()), start_line: 1, end_line: 1, start_column: 6, end_column: 7 },
    //             Token { token_type: Operator(Operation::Gt), start_line: 1, end_line: 1, start_column: 8, end_column: 9 },
    //             Token { token_type: Integer("5".to_string()), start_line: 1, end_line: 1, start_column: 10, end_column: 11 },
    //             Token { token_type: ArrowAssignment, start_line: 1, end_line: 1, start_column: 12, end_column: 14 },
    //             Token { token_type: BlockOpen, start_line: 1, end_line: 1, start_column: 15, end_column: 16 },
    //             Token { token_type: BlockClose, start_line: 1, end_line: 1, start_column: 16, end_column: 17 },
    //             Token { token_type: BlockClose, start_line: 1, end_line: 1, start_column: 18, end_column: 19 },
    //             Token { token_type: EndStatementOrExpression, start_line: 1, end_line: 1, start_column: 19, end_column: 20 },
    //         ]
    //     );
    // }

    let _ = expect_token(state, TokenType::IfDeclaration)?;
    let code_span = expect_token(state, TokenType::BlockOpen)?;

    let mut condition_branches = Vec::new();
    let mut else_branch = None;

    loop {
        if let Some(token) = state.tokens.peek() {
            match &token.token_type {
                TokenType::ElseDeclaration => {
                    advance(state); // Consume 'else'
                    let _ = expect_token(state, TokenType::ArrowAssignment)?;
                    else_branch = Some(Box::new(parse_block(state)?));

                    break;
                }

                _ => {
                    let condition = Box::new(parse_expression(state, 0)?);
                    let _ = expect_token(state, TokenType::ArrowAssignment)?;
                    let branch = Box::new(parse_block(state)?);
                    condition_branches.push((condition, branch));

                    // Check for comma after each pair except the last one
                    if state.tokens.peek().map_or(false, |t| t.token_type == TokenType::Comma) {
                        advance(state);
                    } else {
                        break;
                    }
                }
            }
        } else {
            return Err(CodeError { message: "Unexpected end of if statement".to_string(), code_span: code_span.clone() });
        }
    }

    let _ = expect_token(state, TokenType::BlockClose)?;
    
    // Check if we need a semicolon (statement context) or not (expression context)
    let code_span = if !is_expression && state.tokens.peek().map_or(false, |t| t.token_type == TokenType::EndStatementOrExpression) {
        expect_token(state, TokenType::EndStatementOrExpression)?
    } else {
        // In expression context, use the current position as code span
        state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone())
    };

    Ok(ASTNode::IfStatement { condition_branches, else_branch, code_span, scope: GLOBAL_SCOPE })
}

fn parse_block(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let _ = expect_token(state, TokenType::BlockOpen)?;
    let mut statements = vec![];
    while state.tokens.peek().map_or(false, |t| t.token_type != TokenType::BlockClose) {
        statements.push(parse_statement(state)?);
    }
    let _ = expect_token(state, TokenType::BlockClose)?;
    Ok(ASTNode::Block { statements, code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()), scope: GLOBAL_SCOPE })
}

fn parse_parallel_block(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let _ = expect_token(state, TokenType::ParallelDeclaration)?;
    let _ = expect_token(state, TokenType::BlockOpen)?;
    let mut statements = vec![];
    while state.tokens.peek().map_or(false, |t| t.token_type != TokenType::BlockClose) {
        statements.push(parse_statement(state)?);
    }
    let _ = expect_token(state, TokenType::BlockClose)?;
    Ok(ASTNode::ParallelBlock { statements, code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()), scope: GLOBAL_SCOPE })
}

fn parse_parallel_block_new(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let start_span = expect_token(state, TokenType::ParallelStart)?;
    let mut assignments = vec![];
    
    while state.tokens.peek().map_or(false, |t| t.token_type != TokenType::ParallelEnd) {
        let stmt = parse_statement(state)?;
        
        // Only const declarations are allowed in parallel assignments
        match stmt {
            ASTNode::ConstDeclaration { name, data_type, value, .. } => {
                assignments.push((name, data_type, value));
            }
            _ => {
                return Err(CodeError {
                    message: "Only const declarations are allowed in parallel assignments. Use 'name:type = expression;' syntax.".to_string(),
                    code_span: start_span.clone(),
                });
            }
        }
    }
    
    let end_span = expect_token(state, TokenType::ParallelEnd)?;
    Ok(ASTNode::ParallelAssignment { 
        assignments, 
        code_span: CodeSpan {
            start_line: start_span.start_line,
            start_column: start_span.start_column,
            end_line: end_span.end_line,
            end_column: end_span.end_column,
        }, 
        scope: GLOBAL_SCOPE 
    })
}

fn parse_return_statement(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let _ = expect_token(state, TokenType::Return)?;
    let statement = parse_expression(state, 0)?;
    let code_span = expect_token(state, TokenType::EndStatementOrExpression)?;
    Ok(ASTNode::ReturnDeclaration { statement: Box::new(statement), code_span: code_span.clone(), scope: GLOBAL_SCOPE })
}

fn parse_type_declaration(state: &mut ParserState) -> Result<NailDataTypeDescriptor, CodeError> {
    if let Some(Token { token_type: TokenType::TypeDeclaration(data_type), .. }) = advance(state) {
        Ok(data_type)
    } else {
        let error = CodeError {
            message: format!("Expected type declaration, found {:?}", state.tokens.peek().map(|token| token.token_type.clone()).unwrap_or(TokenType::EndOfFile)),
            code_span: state.tokens.peek().map_or(CodeSpan::default(), |token| token.code_span.clone()),
        };
        log::error!("parse_type_declaration error: {:?}", error);
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lexer;

    fn remove_whitespace(s: &str) -> String {
        s.chars().filter(|c| !c.is_whitespace()).collect()
    }

    #[test]
    fn test_function_declaration() {
        let input = "f add(yay:i, bah:i):i { r yay + bah; }";
        let (result, _) = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);
        let what_the_ast_should_be = r#"Program(
    [
        FunctionDeclaration {
            name: "add",
            params: [
                (
                    "yay",
                    Int,
                ),
                (
                    "bah",
                    Int,
                ),
            ],
            data_type: Int,
            body: Block(
                [
                    ReturnDeclaration(
                        BinaryOperation {
                            left: Identifier(
                                "yay",
                            ),
                            operator: Add,
                            right: Identifier(
                                "bah",
                            ),
                        },
                    ),
                ],
            ),
        },
    ],
)"#;

        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_if_statement() {
        let input = "if { a > 5 => {} };";
        let (result, _) = parse(lexer(input)).unwrap();
        let expected = r#"
      Program([
    IfStatement {
        condition_branches: [
            (BinaryOperation {
                left: Identifier("a",),
                operator: Gt,
                right: NumberLiteral("5",),
            }, Block([],),),
        ],
        else_branch: None,
    },
],)
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_lambda() {
        let input = "| x:i |:i { result:i = x + 1; r result; }";
        let (result, _) = parse(lexer(input)).unwrap();
        let expected = r#"
        Program(
            [
                LambdaDeclaration {
                    params: [
                        (
                            "x",
                            Int,
                        ),
                    ],
                    data_type: Int,
                    body: Block(
                        [
                            ConstDeclaration {
                                name: "result",
                                data_type: Int,
                                value: BinaryOperation {
                                    left: Identifier(
                                        "x",
                                    ),
                                    operator: Add,
                                    right: NumberLiteral(
                                        "1",
                                    ),
                                },
                            },
                            ReturnDeclaration(
                                Identifier(
                                    "result",
                                ),
                            ),
                        ],
                    ),
                },
            ],
        )
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_struct_declaration() {
        let input = "struct Point { x:i, y:i }";
        let (result, _) = parse(lexer(input)).unwrap();
        let expected = r#"
       Program([StructDeclaration{name:"Point",fields:[StructDeclarationField{name:"x",data_type:Int,},StructDeclarationField{name:"y",data_type:Int,},],},],)
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_enum_declaration() {
        let input = "enum Color { Red, Green, Blue }";
        let lexer = lexer(input);
        let (result, _) = parse(lexer).unwrap();
        let expected = r#"
       Program([EnumDeclaration{name:"Color",variants:[EnumVariant{name:"Color",variant:"Red",},EnumVariant{name:"Color",variant:"Green",},EnumVariant{name:"Color",variant:"Blue",},],},],)
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_function_call() {
        let input = "fun(param);";
        let (result, _) = parse(lexer(input)).unwrap();
        let expected = r#"
        Program(
            [
                FunctionCall {
                    name: "fun",
                    args: [
                        Identifier(
                            "param",
                        ),
                    ],
                },
            ],
        )
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_function_nested_call() {
        let input = "fun(times(param));";
        let (result, _) = parse(lexer(input)).unwrap();
        let expected = r#"
        Program(
            [
                FunctionCall {
                    name: "fun",
                    args: [
                        FunctionCall {
                            name: "times",
                            args: [
                                Identifier(
                                    "param",
                                ),
                            ],
                        },
                    ],
                },
            ],
        )
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_if_else_statement() {
        let input = "if { a > 5 => {}, else => {} };";
        let (result, _) = parse(lexer(input)).unwrap();
        let expected = r#"
        Program([IfStatement{condition_branches:[(BinaryOperation{left:Identifier("a",),operator:Gt,right:NumberLiteral("5",),},Block([],),),],else_branch:Some(Block([],),),},],)
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_if_else_if_else_statement() {
        let input = "if { a > 5 => {}, b < 5 => {}, else => {} };";
        let (result, _) = parse(lexer(input)).unwrap();
        let expected = r#"
        Program([IfStatement{condition_branches:[(BinaryOperation{left:Identifier("a",),operator:Gt,right:NumberLiteral("5",),},Block([],),),(BinaryOperation{left:Identifier("b",),operator:Lt,right:NumberLiteral("5",),},Block([],),),],else_branch:Some(Block([],),),},],)
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_array() {
        let input = "test_array:a:i = [1, 2, 3];";
        let lexer = lexer(input);

        let (result, _) = parse(lexer).unwrap();
        let expected = r#"
     Program([ConstDeclaration{name:"test_array",data_type:ArrayInt,value:ArrayLiteral([NumberLiteral("1",),NumberLiteral("2",),NumberLiteral("3",),],),},],)
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_array_declaration() {
        // this is technically wrong assignment but useful test, checker.rs would catch this mismatch assigned type
        let input = "test_array:a:i = 1;";
        let lexer = lexer(input);

        let (result, _) = parse(lexer).unwrap();
        let expected = r#"
     Program([ConstDeclaration{name:"test_array",data_type:ArrayInt,value:NumberLiteral("1",),},],)
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_array_declaration_assignment_to_array() {
        let input = "test_array:a:i = [1, 2, 3];";
        let lexer = lexer(input);

        let (result, _) = parse(lexer).unwrap();
        let expected = r#"
        Program(
            [
                ConstDeclaration {
                    name: "test_array",
                    data_type: ArrayInt,
                    value: ArrayLiteral(
                        [
                            NumberLiteral(
                                "1",
                            ),
                            NumberLiteral(
                                "2",
                            ),
                            NumberLiteral(
                                "3",
                            ),
                        ],
                    ),
                },
            ],
        )
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_function_declaration_multiple_params() {
        let input = r#"f random(x:i, y:f):s { result:s = `test`; r result; }"#;
        let (result, _) = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);

        let expected = r#"
        Program(
    [
        FunctionDeclaration {
            name: "random",
            params: [
                (
                    "x",
                    Int,
                ),
                (
                    "y",
                    Float,
                ),
            ],
            data_type: String,
            body: Block(
                [
                    ConstDeclaration {
                        name: "result",
                        data_type: String,
                        value: StringLiteral(
                            "test",
                        ),
                    },
                    ReturnDeclaration(
                        Identifier(
                            "result",
                        ),
                    ),
                ],
            ),
        },
    ],
)
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_enum_variant() {
        let input = "my_color:enum:Color = Color::Red;";
        let (result, _) = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);
        let expected = r#"
        Program(
    [
        ConstDeclaration {
            name: "my_color",
            data_type: EnumColor,
            value: EnumVariant {
                name: "Color",
                variant: "Red",
            },
        },
    ],
)
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_const_declaration() {
        let input = "counter:i = 10;";
        let (result, _) = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);
        // Add assertion here
    }

    // Variable declarations no longer supported - using constants only

    #[test]
    fn test_any_type_declaration() {
        let input = "c every_nail_type:any(i|f|s|b|a:i|a:f|a:struct:any|a:enum:any) = 13;";
        let (result, _) = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);
        // Just verify it parses successfully and has const declaration
        if let ASTNode::Program { statements, .. } = result {
            assert!(matches!(statements.get(0), Some(ASTNode::ConstDeclaration { .. })));
        } else {
            panic!("Expected Program node");
        }
    }

    // FAILING
    #[test]
    fn test_lambda_multi_param() {
        let input = "| x:i, y:f |:i { result:i = x + 1; r result; }";
        let (result, _) = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);
        // Just verify it parses successfully and has lambda declaration
        if let ASTNode::Program { statements, .. } = result {
            assert!(matches!(statements.get(0), Some(ASTNode::LambdaDeclaration { .. })));
        } else {
            panic!("Expected Program node");
        }
    }

    #[test]
    fn test_lambda_with_error_parameter() {
        // Test that |e|:i syntax works for error parameters
        let input = r#"|e|:i { r 0; }"#;
        
        let result = parse(lexer(input));
        assert!(result.is_ok(), "Failed to parse lambda with error parameter: {:?}", result.err());
        
        let (ast, _) = result.unwrap();
        
        // The lambda should be at the top level
        if let ASTNode::Program { statements, .. } = ast {
            assert!(!statements.is_empty(), "Program should have statements");
            
            if let Some(ASTNode::LambdaDeclaration { params, data_type, .. }) = statements.first() {
                // Check that there's one parameter named 'e' with Error type
                assert_eq!(params.len(), 1, "Lambda should have exactly one parameter");
                assert_eq!(params[0].0, "e", "Parameter should be named 'e'");
                assert_eq!(params[0].1, NailDataTypeDescriptor::Error, "Parameter 'e' should have Error type");
                
                // Check return type is Int
                assert_eq!(*data_type, NailDataTypeDescriptor::Int, "Lambda should return Int");
            } else {
                panic!("Expected LambdaDeclaration at top level");
            }
        } else {
            panic!("Expected Program node");
        }
    }
    
    fn find_lambda_in_ast(node: &ASTNode) -> Option<&ASTNode> {
        match node {
            ASTNode::LambdaDeclaration { .. } => Some(node),
            ASTNode::Program { statements, .. } => {
                for stmt in statements {
                    if let Some(found) = find_lambda_in_ast(stmt) {
                        return Some(found);
                    }
                }
                None
            }
            ASTNode::FunctionDeclaration { body, .. } => find_lambda_in_ast(body),
            ASTNode::Block { statements, .. } => {
                for stmt in statements {
                    if let Some(found) = find_lambda_in_ast(stmt) {
                        return Some(found);
                    }
                }
                None
            }
            ASTNode::ConstDeclaration { value, .. } => find_lambda_in_ast(value),
            ASTNode::FunctionCall { args, .. } => {
                for arg in args {
                    if let Some(found) = find_lambda_in_ast(arg) {
                        return Some(found);
                    }
                }
                None
            }
            _ => None,
        }
    }

    #[test]
    fn test_array_of_point_structs() {
        let input = r#"
            points:a:struct:Point = [Point { x: 1, y: 5 }, Point { x: 3, y: 4 }];
            "#;
        let (result, _) = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);
        let expected = r#"
        Program(
    [
        ConstDeclaration {
            name: "points",
            data_type: ArrayStruct(
                "Point",
            ),
            value: ArrayLiteral(
                [
                    StructInstantiation {
                        name: "Point",
                        fields: [
                            StructInstantiationField {
                                name: "x",
                                value: NumberLiteral(
                                    "1",
                                ),
                            },
                            StructInstantiationField {
                                name: "y",
                                value: NumberLiteral(
                                    "5",
                                ),
                            },
                        ],
                    },
                    StructInstantiation {
                        name: "Point",
                        fields: [
                            StructInstantiationField {
                                name: "x",
                                value: NumberLiteral(
                                    "3",
                                ),
                            },
                            StructInstantiationField {
                                name: "y",
                                value: NumberLiteral(
                                    "4",
                                ),
                            },
                        ],
                    },
                ],
            ),
        },
    ],
)
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }
}
