use crate::lexer::*;
use crate::CodeError;
use std::iter::Peekable;
use std::vec::IntoIter;

#[derive(Debug, PartialEq, Clone)]
pub enum ASTNode {
    Program(Vec<ASTNode>),
    FunctionDeclaration { name: String, params: Vec<(String, NailDataTypeDescriptor)>, return_type: NailDataTypeDescriptor, body: Box<ASTNode> },
    LambdaDeclaration { params: Vec<(String, NailDataTypeDescriptor)>, return_type: NailDataTypeDescriptor, body: Box<ASTNode> },
    FunctionCall { name: String, args: Vec<ASTNode> },
    VariableDeclaration { name: String, data_type: NailDataTypeDescriptor, value: Box<ASTNode> },
    ConstDeclaration { name: String, data_type: NailDataTypeDescriptor, value: Box<ASTNode> },
    IfStatement { condition_branch_pairs: Vec<(Box<ASTNode>, Box<ASTNode>)>, else_branch: Option<Box<ASTNode>> },
    Block(Vec<ASTNode>),
    BinaryOperation { left: Box<ASTNode>, operator: Operation, right: Box<ASTNode> },
    UnaryOperation { operator: Operation, operand: Box<ASTNode> },
    StructDeclaration { name: String, fields: Vec<ASTNode> },
    StructDeclarationField { name: String, data_type: NailDataTypeDescriptor },
    StructInstantiation { name: String, fields: Vec<ASTNode> },
    StructInstantiationField { name: String, value: Box<ASTNode> },
    EnumDeclaration { name: String, variants: Vec<ASTNode> },
    EnumVariant { name: String, variant: String },
    ArrayLiteral(Vec<ASTNode>),
    Identifier(String),
    NumberLiteral(String),
    StringLiteral(String),
    RustEscape(Vec<ASTNode>),
    RustLiteral(String),
    NailInjection(Vec<ASTNode>),
    ReturnStatement(Box<ASTNode>),
}

pub struct ParserState {
    tokens: Peekable<IntoIter<Token>>,
    current_token: Option<Token>,
    previous_token: Option<Token>,
}

pub fn parse(tokens: Vec<Token>) -> Result<ASTNode, CodeError> {
    let mut state = ParserState { tokens: tokens.into_iter().peekable(), current_token: None, previous_token: None };
    parse_inner(&mut state)
}

fn parse_inner(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let mut program = vec![];
    while state.tokens.peek().is_some() {
        program.push(parse_statement(state)?);
    }
    Ok(ASTNode::Program(program))
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
            TokenType::Identifier(name) => {
                advance(state);
                if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::ParenthesisOpen)) {
                    parse_function_call(state, name)
                } else {
                    Ok(ASTNode::Identifier(name))
                }
            }
            TokenType::Float(value) | TokenType::Integer(value) => {
                advance(state);
                Ok(ASTNode::NumberLiteral(value))
            }
            TokenType::StringLiteral(value) => {
                advance(state);
                Ok(ASTNode::StringLiteral(value))
            }
            TokenType::RustLiteral(value) => {
                advance(state);
                Ok(ASTNode::RustLiteral(value))
            }
            TokenType::ParenthesisOpen => {
                advance(state);
                let expr = parse_expression(state, 0)?;
                expect_token(state, TokenType::ParenthesisClose)?;
                Ok(expr)
            }
            TokenType::EnumVariant(variant) => {
                advance(state);
                Ok(ASTNode::EnumVariant { name: variant.name, variant: variant.variant })
            }
            TokenType::Array(_) => parse_array_literal(state),
            _ => Err(CodeError { message: format!("Unexpected token {:?}", token.token_type), line: token.code_span.start_line, column: token.code_span.start_column }),
        }
    } else {
        Err(CodeError {
            message: "Unexpected end of file".to_string(),
            line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
        })
    }
}

fn parse_statement(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    match state.tokens.peek() {
        Some(token) => match &token.token_type {
            TokenType::StructDeclaration(_) => parse_struct_declaration(state),
            TokenType::EnumDeclaration(_) => parse_enum_declaration(state),
            TokenType::RustNailInsert(_) => parse_inner(state),
            TokenType::FunctionSignature(_) => parse_function_declaration(state),
            TokenType::ConstDeclaration => parse_const_declaration(state),
            TokenType::VariableDeclaration => parse_variable_declaration(state),
            TokenType::IfDeclaration => parse_if_statement(state),
            TokenType::Return => parse_return_statement(state),
            TokenType::LambdaSignature(_) => parse_lambda_declaration(state),
            TokenType::BlockOpen => parse_block(state),
            _ => parse_expression(state, 0),
        },
        None => Err(CodeError {
            message: "No token was found to match with a statement.".to_string(),
            line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
        }),
    }
}

fn parse_expression(state: &mut ParserState, min_precedence: u8) -> Result<ASTNode, CodeError> {
    let mut left = parse_primary(state)?;

    while let Some(Token { token_type: TokenType::Operator(op), .. }) = state.tokens.peek().cloned() {
        if op.precedence() < min_precedence {
            break;
        }

        advance(state); // Consume the operator

        if op.is_unary() {
            left = ASTNode::UnaryOperation { operator: op, operand: Box::new(left) };
        } else {
            let right = parse_expression(state, op.precedence() + 1)?;
            left = ASTNode::BinaryOperation { left: Box::new(left), operator: op, right: Box::new(right) };
        }
    }

    Ok(left)
}

fn expect_token(state: &mut ParserState, expected: TokenType) -> Result<(), CodeError> {
    if let Some(token) = advance(state) {
        if token.token_type == expected {
            Ok(())
        } else {
            let error = CodeError { message: format!("Expected {:?}, found {:?}", expected, token.token_type), line: token.code_span.start_line, column: token.code_span.start_column };
            log::error!("Expect token error: {:?}", error);
            Err(error)
        }
    } else {
        log::error!("expect_token else branch error: {:?}", expected);
        Err(CodeError {
            message: format!("Expected {:?}, found end of file", expected),
            line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
        })
    }
}

fn expect_identifier(state: &mut ParserState) -> Result<String, CodeError> {
    if let Some(Token { token_type: TokenType::Identifier(name), .. }) = advance(state) {
        Ok(name)
    } else {
        let error = CodeError {
            message: format!("Expected identifier, found {:?}", state.tokens.peek().map(|token| token.token_type.clone()).unwrap_or(TokenType::EndOfFile)),
            line: state.tokens.peek().map_or(0, |token| token.code_span.start_line),
            column: state.tokens.peek().map_or(0, |token| token.code_span.start_column),
        };
        log::error!("Expect identifier error: {:?}", error);
        Err(error)
    }
}

fn parse_function_call(state: &mut ParserState, name: String) -> Result<ASTNode, CodeError> {
    expect_token(state, TokenType::ParenthesisOpen)?;
    let mut args = Vec::new();
    while state.tokens.peek().map_or(false, |t| t.token_type != TokenType::ParenthesisClose) {
        args.push(parse_expression(state, 0)?);
        if state.tokens.peek().map_or(false, |t| t.token_type == TokenType::Comma) {
            advance(state);
        } else {
            break;
        }
    }
    expect_token(state, TokenType::ParenthesisClose)?;

    // it should have a ; if the next token after is not a ) for stuff like fun(yay(times)); so it doesnt need a bunch of ugly ; like fun(yay(times););
    if state.tokens.peek().map_or(true, |t| t.token_type != TokenType::ParenthesisClose) {
        expect_token(state, TokenType::EndStatementOrExpression)?;
    }

    Ok(ASTNode::FunctionCall { name, args })
}

fn parse_lambda_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(Token { token_type: TokenType::LambdaSignature(tokens), .. }) = advance(state) {
        let mut lambda_tokens = tokens.into_iter();
        let mut params = Vec::new();
        let mut return_type = NailDataTypeDescriptor::Void;

        // Parse parameters
        loop {
            match lambda_tokens.next() {
                Some(Token { token_type: TokenType::Identifier(param_name), .. }) => {
                    if let Some(Token { token_type: TokenType::TypeDeclaration(type_desc), .. }) = lambda_tokens.next() {
                        params.push((param_name.clone(), type_desc.clone()));

                        // Check for comma or end of parameters
                        match lambda_tokens.next() {
                            Some(Token { token_type: TokenType::Comma, .. }) => continue,
                            Some(Token { token_type: TokenType::LambdaReturnTypeDeclaration(rt), .. }) => {
                                return_type = rt;
                                break;
                            }
                            Some(other) => {
                                return Err(CodeError {
                                    message: format!("Expected comma or return type declaration, found {:?}", other.token_type),
                                    line: other.code_span.start_line,
                                    column: other.code_span.start_column,
                                })
                            }
                            None => {
                                return Err(CodeError {
                                    message: "Unexpected end of lambda declaration".to_string(),
                                    line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
                                    column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
                                })
                            }
                        }
                    } else {
                        return Err(CodeError {
                            message: "Expected type declaration for lambda parameter".to_string(),
                            line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
                            column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
                        });
                    }
                }
                Some(Token { token_type: TokenType::LambdaReturnTypeDeclaration(rt), .. }) => {
                    return_type = rt;
                    break;
                }
                Some(other) => {
                    return Err(CodeError {
                        message: format!("Unexpected token in lambda declaration: {:?}", other.token_type),
                        line: other.code_span.start_line,
                        column: other.code_span.start_column,
                    })
                }
                None => {
                    return Err(CodeError {
                        message: "Unexpected end of lambda declaration".to_string(),
                        line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
                        column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
                    })
                }
            }
        }

        // Parse the lambda body
        let body = Box::new(parse_block(state)?);

        Ok(ASTNode::LambdaDeclaration { params, return_type, body })
    } else {
        Err(CodeError {
            message: "Expected lambda declaration".to_string(),
            line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
        })
    }
}

fn parse_struct_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(Token { token_type: TokenType::StructDeclaration(struct_declaration_data), .. }) = advance(state) {
        let mut struct_fields = struct_declaration_data.fields.into_iter();

        let struct_name = struct_declaration_data.name;
        let mut fields = Vec::new();

        while let Some(field) = struct_fields.next() {
            fields.push(ASTNode::StructDeclarationField { name: field.name, data_type: field.data_type })
        }

        Ok(ASTNode::StructDeclaration { name: struct_name, fields })
    } else {
        Err(CodeError {
            message: "Struct declaration syntax is incorrect".to_string(),
            line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
        })
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
                    TokenType::Identifier(name) => ASTNode::Identifier(name.clone()),
                    TokenType::Integer(value) => ASTNode::NumberLiteral(value.clone()),
                    TokenType::Float(value) => ASTNode::NumberLiteral(value.clone()),
                    TokenType::StringLiteral(value) => ASTNode::StringLiteral(value.clone()),
                    _ => {
                        return Err(CodeError {
                            message: format!("Unexpected token in struct field: {:?}", field.value.token_type),
                            line: field.value.code_span.start_line,
                            column: field.value.code_span.start_column,
                        });
                    }
                }),
            });
        }
        Ok(ASTNode::StructInstantiation { name: struct_name, fields })
    } else {
        Err(CodeError { message: format!("Expected struct instantiation, found {:?}", token.token_type), line: token.code_span.start_line, column: token.code_span.start_column })
    }
}

// For standalone struct instantiations outside of arrays
fn parse_struct_instantiation(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(token @ Token { token_type: TokenType::StructInstantiation(_), .. }) = state.tokens.peek().cloned() {
        advance(state); // Consume the StructInstantiation token
        parse_struct_instantiation_token(&token)
    } else {
        Err(CodeError {
            message: "Expected struct instantiation".to_string(),
            line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
        })
    }
}
fn parse_array_literal(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(Token { token_type: TokenType::Array(element_tokens), .. }) = state.tokens.peek().cloned() {
        let mut elements = Vec::new();
        // These tkoens are not part of state, they are internal to the array literal
        for token in element_tokens {
            match token.token_type {
                TokenType::StructInstantiation(_) => elements.push(parse_struct_instantiation_token(&token)?),
                TokenType::Integer(value) => elements.push(ASTNode::NumberLiteral(value)),
                TokenType::Float(value) => elements.push(ASTNode::NumberLiteral(value)),
                TokenType::StringLiteral(value) => elements.push(ASTNode::StringLiteral(value)),
                TokenType::Identifier(name) => elements.push(ASTNode::Identifier(name)),

                _ => return Err(CodeError { message: format!("Unexpected token in array: {:?}", token.token_type), line: token.code_span.start_line, column: token.code_span.start_column }),
            }
        }

        advance(state); // Consume the Array token

        Ok(ASTNode::ArrayLiteral(elements))
    } else {
        Err(CodeError {
            message: "Array literal syntax is incorrect".to_string(),
            line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
        })
    }
}

fn parse_enum_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(Token { token_type: TokenType::EnumDeclaration(nail_enum_data), .. }) = advance(state) {
        let enum_name = nail_enum_data.name;
        let mut enum_tokens = nail_enum_data.variants.into_iter();

        // Parse variants
        let mut variants = Vec::new();
        while let Some(token) = enum_tokens.next() {
            match token.token_type {
                TokenType::EnumVariant(variant) => {
                    variants.push(ASTNode::EnumVariant { name: enum_name.clone(), variant: variant.variant });
                }
                TokenType::BlockClose => break,
                _ => {
                    return Err(CodeError { message: format!("Unexpected token in enum declaration: {:?}", token.token_type), line: token.code_span.start_line, column: token.code_span.start_column })
                }
            }
        }

        Ok(ASTNode::EnumDeclaration { name: enum_name, variants })
    } else {
        Err(CodeError {
            message: "Expected enum declaration".to_string(),
            line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
        })
    }
}

fn parse_function_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(Token { token_type: TokenType::FunctionSignature(tokens), .. }) = advance(state) {
        let mut func_tokens = tokens.into_iter();

        // Parse function name
        let name = if let Some(Token { token_type: TokenType::FunctionName(name), .. }) = func_tokens.next() {
            name
        } else {
            return Err(CodeError {
                message: "Expected function name".to_string(),
                line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
                column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
            });
        };

        let mut params = Vec::new();
        let mut return_type = NailDataTypeDescriptor::Void;

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
                                return_type = rt;
                                break;
                            }
                            Some(other) => {
                                return Err(CodeError {
                                    message: format!("Expected comma or return type declaration, found {:?}", other.token_type),
                                    line: other.code_span.start_line,
                                    column: other.code_span.start_column,
                                })
                            }
                            None => {
                                return Err(CodeError {
                                    message: "Unexpected end of function declaration".to_string(),
                                    line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
                                    column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
                                })
                            }
                        }
                    } else {
                        return Err(CodeError {
                            message: "Expected type declaration for function parameter".to_string(),
                            line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
                            column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
                        });
                    }
                }
                Some(Token { token_type: TokenType::FunctionReturnTypeDeclaration(rt), .. }) => {
                    return_type = rt;
                    break;
                }
                Some(other) => {
                    return Err(CodeError {
                        message: format!("Unexpected token in function declaration: {:?}", other.token_type),
                        line: other.code_span.start_line,
                        column: other.code_span.start_column,
                    })
                }
                None => {
                    return Err(CodeError {
                        message: "Unexpected end of function declaration".to_string(),
                        line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
                        column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
                    })
                }
            }
        }

        // Parse function body
        let body = Box::new(parse_block(state)?);

        Ok(ASTNode::FunctionDeclaration { name, params, return_type, body })
    } else {
        Err(CodeError {
            message: "Expected function declaration".to_string(),
            line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
        })
    }
}

fn parse_const_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    expect_token(state, TokenType::ConstDeclaration)?;
    let name = expect_identifier(state)?;
    let data_type = parse_type_declaration(state)?;
    expect_token(state, TokenType::Assignment)?;
    let value = Box::new(parse_expression(state, 0)?);
    expect_token(state, TokenType::EndStatementOrExpression)?;

    Ok(ASTNode::ConstDeclaration { name, data_type, value })
}

fn parse_variable_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    expect_token(state, TokenType::VariableDeclaration)?;
    let name = expect_identifier(state)?;
    let data_type = parse_type_declaration(state)?;
    expect_token(state, TokenType::Assignment)?;
    let value = Box::new(parse_expression(state, 0)?);
    expect_token(state, TokenType::EndStatementOrExpression)?;

    Ok(ASTNode::VariableDeclaration { name, data_type, value })
}

fn parse_if_statement(state: &mut ParserState) -> Result<ASTNode, CodeError> {
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

    expect_token(state, TokenType::IfDeclaration)?;
    expect_token(state, TokenType::BlockOpen)?;

    let mut condition_branch_pairs = Vec::new();
    let mut else_branch = None;

    loop {
        if let Some(token) = state.tokens.peek() {
            match &token.token_type {
                TokenType::ElseDeclaration => {
                    advance(state); // Consume 'else'
                    expect_token(state, TokenType::ArrowAssignment)?;
                    else_branch = Some(Box::new(parse_block(state)?));

                    break;
                }

                _ => {
                    let condition = Box::new(parse_expression(state, 0)?);
                    expect_token(state, TokenType::ArrowAssignment)?;
                    let branch = Box::new(parse_block(state)?);
                    condition_branch_pairs.push((condition, branch));

                    // Check for comma after each pair except the last one
                    if state.tokens.peek().map_or(false, |t| t.token_type == TokenType::Comma) {
                        advance(state);
                    } else {
                        break;
                    }
                }
            }
        } else {
            return Err(CodeError {
                message: "Unexpected end of if statement".to_string(),
                line: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_line),
                column: state.previous_token.as_ref().map_or(0, |t| t.code_span.start_column),
            });
        }
    }

    expect_token(state, TokenType::BlockClose)?;
    expect_token(state, TokenType::EndStatementOrExpression)?;

    Ok(ASTNode::IfStatement { condition_branch_pairs, else_branch })
}

fn parse_block(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    expect_token(state, TokenType::BlockOpen)?;
    let mut statements = vec![];
    while state.tokens.peek().map_or(false, |t| t.token_type != TokenType::BlockClose) {
        statements.push(parse_statement(state)?);
    }
    expect_token(state, TokenType::BlockClose)?;
    Ok(ASTNode::Block(statements))
}

fn parse_return_statement(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    expect_token(state, TokenType::Return)?;
    let value = Box::new(parse_expression(state, 0)?);
    expect_token(state, TokenType::EndStatementOrExpression)?;
    Ok(ASTNode::ReturnStatement(value))
}

fn parse_type_declaration(state: &mut ParserState) -> Result<NailDataTypeDescriptor, CodeError> {
    if let Some(Token { token_type: TokenType::TypeDeclaration(data_type), .. }) = advance(state) {
        Ok(data_type)
    } else {
        let error = CodeError {
            message: format!("Expected type declaration, found {:?}", state.tokens.peek().map(|token| token.token_type.clone()).unwrap_or(TokenType::EndOfFile)),
            line: state.tokens.peek().map_or(0, |token| token.code_span.start_line),
            column: state.tokens.peek().map_or(0, |token| token.code_span.start_column),
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
        let input = "fn add(yay:i, bah:i):i { r yay + bah; }";
        let result = parse(lexer(input)).unwrap();
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
            return_type: Int,
            body: Block(
                [
                    ReturnStatement(
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

        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(&what_the_ast_should_be));
    }

    #[test]
    fn test_if_statement() {
        let input = "if { a > 5 => {} };";
        let result = parse(lexer(input)).unwrap();
        let expected = r#"
      Program([
    IfStatement {
        condition_branch_pairs: [
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
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }

    #[test]
    fn test_lambda() {
        let input = "| x:i |:i { v result:i = x + 1; r result; }";
        let result = parse(lexer(input)).unwrap();
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
                    return_type: Int,
                    body: Block(
                        [
                            VariableDeclaration {
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
                            ReturnStatement(
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
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }

    #[test]
    fn test_struct_declaration() {
        let input = "struct Point { x:i, y:i }";
        let result = parse(lexer(input)).unwrap();
        let expected = r#"
       Program([StructDeclaration{name:"Point",fields:[StructDeclarationField{name:"x",data_type:Int,},StructDeclarationField{name:"y",data_type:Int,},],},],)
        "#;
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }

    #[test]
    fn test_enum_declaration() {
        let input = "enum Color { Red, Green, Blue }";
        let lexer = lexer(input);
        let result = parse(lexer).unwrap();
        let expected = r#"
       Program([EnumDeclaration{name:"Color",variants:[EnumVariant{name:"Color",variant:"Red",},EnumVariant{name:"Color",variant:"Green",},EnumVariant{name:"Color",variant:"Blue",},],},],)
        "#;
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }

    #[test]
    fn test_function_call() {
        let input = "fun(param);";
        let result = parse(lexer(input)).unwrap();
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
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }

    #[test]
    fn test_function_nested_call() {
        let input = "fun(times(param));";
        let result = parse(lexer(input)).unwrap();
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
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }

    #[test]
    fn test_if_else_statement() {
        let input = "if { a > 5 => {}, else => {} };";
        let result = parse(lexer(input)).unwrap();
        let expected = r#"
        Program([IfStatement{condition_branch_pairs:[(BinaryOperation{left:Identifier("a",),operator:Gt,right:NumberLiteral("5",),},Block([],),),],else_branch:Some(Block([],),),},],)
        "#;
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }

    #[test]
    fn test_if_else_if_else_statement() {
        let input = "if { a > 5 => {}, b < 5 => {}, else => {} };";
        let result = parse(lexer(input)).unwrap();
        let expected = r#"
        Program([IfStatement{condition_branch_pairs:[(BinaryOperation{left:Identifier("a",),operator:Gt,right:NumberLiteral("5",),},Block([],),),(BinaryOperation{left:Identifier("b",),operator:Lt,right:NumberLiteral("5",),},Block([],),),],else_branch:Some(Block([],),),},],)
        "#;
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }

    #[test]
    fn test_array() {
        let input = "[1, 2, 3]";
        let lexer = lexer(input);

        let result = parse(lexer).unwrap();
        let expected = r#"
     Program([ArrayLiteral([NumberLiteral("1",),NumberLiteral("2",),NumberLiteral("3",),],),],)
        "#;
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }

    #[test]
    fn test_array_declaration() {
        // this is technically wrong assignment but useful test, checker.rs would catch this mismatch assigned type
        let input = "v test_array:a:i = 1;";
        let lexer = lexer(input);

        let result = parse(lexer).unwrap();
        let expected = r#"
     Program([VariableDeclaration{name:"test_array",data_type:ArrayInt,value:NumberLiteral("1",),},],)
        "#;
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }

    #[test]
    fn test_array_declaration_assignment_to_array() {
        let input = "v test_array:a:i = [1, 2, 3];";
        let lexer = lexer(input);

        let result = parse(lexer).unwrap();
        let expected = r#"
        Program(
            [
                VariableDeclaration {
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
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }

    #[test]
    fn test_function_declaration_multiple_params() {
        let input = r#"fn random(x:i, y:f):s { v result:s = `test`; r result; }"#;
        let result = parse(lexer(input)).unwrap();
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
            return_type: String,
            body: Block(
                [
                    VariableDeclaration {
                        name: "result",
                        data_type: String,
                        value: StringLiteral(
                            "test",
                        ),
                    },
                    ReturnStatement(
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
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }

    #[test]
    fn test_enum_variant() {
        let input = "Color::Red";
        let result = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);
        let expected = r#"
        Program(
    [
        EnumVariant {
            name: "Color",
            variant: "Red",
        },
    ],
)
        "#;
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }

    #[test]
    fn test_const_declaration() {
        let input = "c x:i = 10;";
        let result = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);
        // Add assertion here
    }

    #[test]
    fn test_variable_declaration() {
        let input = "v y:i = 20;";
        let result = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);
        // Add assertion here
    }

    #[test]
    fn test_any_type_declaration() {
        let input = "c every_nail_type:any(i|f|s|b|a:i|a:f|a:struct:any|a:enum:any) = 13;";
        let result = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);
        let expected = r#"
        Program(
    [
        ConstDeclaration {
            name: "every_nail_type",
            data_type: Any(
                [
                    Int,
                    Float,
                    String,
                    Boolean,
                    ArrayInt,
                    ArrayFloat,
                    ArrayStruct(
                        "any",
                    ),
                    ArrayEnum(
                        "any",
                    ),
                ],
            ),
            value: NumberLiteral(
                "13",
            ),
        },
    ],
)
        "#;
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }

    // FAILING
    #[test]
    fn test_lambda_multi_param() {
        let input = "| x:i, y:f |:i { v result:i = x + 1; r result; }";
        let result = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);
        let expected = r#"
        Program(
    [
        LambdaDeclaration {
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
            return_type: Int,
            body: Block(
                [
                    VariableDeclaration {
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
                    ReturnStatement(
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
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }

    #[test]
    fn test_array_of_point_structs() {
        let input = r#"
            v points:a:struct:Point = [Point { x: 1, y: 5 }, Point { x: 3, y: 4 }];
            "#;
        let result = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);
        let expected = r#"
        Program(
    [
        VariableDeclaration {
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
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }
}
