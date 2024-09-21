use crate::lexer::*;
use crate::CodeError;
use std::collections::HashMap;
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
    StructDeclaration { name: String, fields: Vec<(String, NailDataTypeDescriptor)> },
    EnumDeclaration { name: String, variants: Vec<String> },
    StructInstantiation { name: String, fields: Vec<(String, Box<ASTNode>)> },
    EnumVariant { enum_name: String, variant: String },
    ArrayLiteral(Vec<ASTNode>),
    Identifier(String),
    NumberLiteral(String),
    StringLiteral(String),
    RustEscape(Vec<ASTNode>),
    RustLiteral(String),
    NailInjection(Box<ASTNode>),
    ReturnStatement(Box<ASTNode>),
}

#[derive(Debug, Clone)]
pub struct Scope {
    variables: HashMap<String, NailDataTypeDescriptor>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    fn new() -> Self {
        Scope { variables: HashMap::new(), parent: None }
    }

    fn with_parent(parent: Scope) -> Self {
        Scope { variables: HashMap::new(), parent: Some(Box::new(parent)) }
    }

    fn declare(&mut self, name: String, data_type: NailDataTypeDescriptor) {
        self.variables.insert(name, data_type);
    }

    fn get(&self, name: &str) -> Option<&NailDataTypeDescriptor> {
        if let Some(data_type) = self.variables.get(name) {
            Some(data_type)
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            None
        }
    }
}

fn enter_scope(state: &mut ParserState) {
    let new_scope = Scope::with_parent(state.current_scope.clone());
    state.current_scope = new_scope;
}

fn exit_scope(state: &mut ParserState) {
    if let Some(parent) = state.current_scope.parent.take() {
        state.current_scope = *parent;
    }
}

pub struct ParserState {
    tokens: Peekable<IntoIter<Token>>,
    current_token: Option<Token>,
    previous_token: Option<Token>,
    current_scope: Scope,
}

pub fn parse(tokens: Vec<Token>) -> Result<ASTNode, CodeError> {
    let mut state = ParserState { tokens: tokens.into_iter().peekable(), current_token: None, previous_token: None, current_scope: Scope::new() };
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
    match advance(state) {
        Some(token) => match token.token_type {
            TokenType::Identifier(name) => parse_identifier(state, name),
            TokenType::Float(value) | TokenType::Integer(value) => Ok(ASTNode::NumberLiteral(value)),
            TokenType::StringLiteral(value) => Ok(ASTNode::StringLiteral(value)),
            TokenType::RustLiteral(value) => Ok(ASTNode::RustLiteral(value)),
            TokenType::ParenthesisOpen => {
                let expr = parse_expression(state)?;
                expect_token(state, TokenType::ParenthesisClose)?;
                Ok(expr)
            }
            TokenType::StructInstantiation(struct_name, fields) => parse_struct_instantiation(state, struct_name, fields),
            TokenType::EnumVariant(variant) => Ok(ASTNode::EnumVariant { enum_name: "".to_string(), variant }),
            TokenType::Array(elements) => parse_array_literal(state, elements),
            _ => Err(CodeError { message: format!("Unexpected token {:?}", token.token_type), line: token.start_line, column: token.start_column }),
        },
        None => Err(CodeError {
            message: "Unexpected end of file".to_string(),
            line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
        }),
    }
}

fn parse_statement(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    match state.tokens.peek() {
        Some(token) => match &token.token_type {
            TokenType::StructDeclaration(_) => parse_struct_declaration(state),
            TokenType::EnumDeclaration(_) => parse_enum_declaration(state),
            TokenType::RustEscape(_) => parse_rust_escape(state),
            TokenType::FunctionSignature(_) => parse_function_declaration(state),
            TokenType::ConstDeclaration => parse_const_declaration(state),
            TokenType::VariableDeclaration => parse_variable_declaration(state),
            TokenType::IfDeclaration => parse_if_statement(state),
            TokenType::Return => parse_return_statement(state),
            TokenType::LambdaSignature(_) => parse_lambda_declaration(state),
            TokenType::BlockOpen => parse_block(state),
            _ => parse_expression(state),
        },
        None => Err(CodeError {
            message: "Unexpected end of file".to_string(),
            line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
        }),
    }
}

// ... Implement other parsing functions (parse_function_call, parse_lambda_declaration, etc.) ...

fn parse_expression(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    parse_precedence(state, 0)
}

fn parse_precedence(state: &mut ParserState, min_precedence: u8) -> Result<ASTNode, CodeError> {
    let mut left = parse_primary(state)?;

    while let Some(Token { token_type: TokenType::Operator(op), .. }) = state.tokens.peek().cloned() {
        if op.precedence() < min_precedence {
            break;
        }

        advance(state); // Consume the operator

        if op.is_unary() {
            left = ASTNode::UnaryOperation { operator: op, operand: Box::new(left) };
        } else {
            let right = parse_precedence(state, op.precedence() + 1)?;
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
            let error = CodeError { message: format!("Expected {:?}, found {:?}", expected, token.token_type), line: token.start_line, column: token.start_column };
            log::error!("Expect token error: {:?}", error);
            Err(error)
        }
    } else {
        log::error!("expect_token else branch error: {:?}", expected);
        Err(CodeError {
            message: format!("Expected {:?}, found end of file", expected),
            line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
        })
    }
}

fn expect_identifier(state: &mut ParserState) -> Result<String, CodeError> {
    if let Some(Token { token_type: TokenType::Identifier(name), .. }) = advance(state) {
        Ok(name)
    } else {
        let error = CodeError {
            message: format!("Expected identifier, found {:?}", state.tokens.peek().map(|token| token.token_type.clone()).unwrap_or(TokenType::EndOfFile)),
            line: state.tokens.peek().map_or(0, |token| token.start_line),
            column: state.tokens.peek().map_or(0, |token| token.start_column),
        };
        log::error!("Expect identifier error: {:?}", error);
        Err(error)
    }
}
fn parse_function_call(state: &mut ParserState, name: String) -> Result<ASTNode, CodeError> {
    expect_token(state, TokenType::ParenthesisOpen)?;
    let mut args = vec![];
    while state.tokens.peek().map_or(false, |token| token.token_type != TokenType::ParenthesisClose) {
        args.push(parse_expression(state)?);
        if state.tokens.peek().map_or(false, |token| token.token_type == TokenType::Comma) {
            advance(state); // Consume the comma
        } else {
            break;
        }
    }
    expect_token(state, TokenType::ParenthesisClose)?;

    // Check if we need to expect an EndStatementOrExpression token
    if state.tokens.peek().map_or(false, |token| token.token_type != TokenType::ParenthesisClose) {
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
        while let Some(token) = lambda_tokens.next() {
            match token.token_type {
                TokenType::Identifier(param_name) => {
                    if let Some(Token { token_type: TokenType::TypeDeclaration(type_desc), .. }) = lambda_tokens.next() {
                        params.push((param_name.clone(), type_desc.clone()));
                    } else {
                        return Err(CodeError { message: "Expected type declaration for lambda parameter".to_string(), line: token.start_line, column: token.start_column });
                    }
                }
                TokenType::LambdaReturnTypeDeclaration(rt) => {
                    return_type = rt;
                    break;
                }
                _ => {
                    return Err(CodeError { message: format!("Unexpected token in lambda declaration: {:?}", token.token_type), line: token.start_line, column: token.start_column });
                }
            }
        }

        // Enter a new scope for the lambda body
        enter_scope(state);

        // Declare parameters in the new scope
        for (param_name, param_type) in &params {
            state.current_scope.declare(param_name.clone(), param_type.clone());
        }

        // Parse the lambda body
        let body = Box::new(parse_block(state)?);

        // Exit the lambda scope
        exit_scope(state);

        Ok(ASTNode::LambdaDeclaration { params, return_type, body })
    } else {
        Err(CodeError {
            message: "Expected lambda declaration".to_string(),
            line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
        })
    }
}

fn parse_identifier(state: &mut ParserState, name: String) -> Result<ASTNode, CodeError> {
    if state.current_scope.get(&name).is_some() {
        Ok(ASTNode::Identifier(name))
    } else {
        Err(CodeError {
            message: format!("'{}' is not defined", name),
            line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
        })
    }
}

fn parse_struct_instantiation(state: &mut ParserState, struct_name: String, field_tokens: Vec<Token>) -> Result<ASTNode, CodeError> {
    let mut fields = Vec::new();
    for token in field_tokens {
        match token.token_type {
            TokenType::StructFieldName(name) => {
                let value = parse_expression(state)?;
                fields.push((name, Box::new(value)));
            }
            _ => return Err(CodeError { message: format!("Unexpected token in struct instantiation: {:?}", token.token_type), line: token.start_line, column: token.start_column }),
        }
    }
    Ok(ASTNode::StructInstantiation { name: struct_name, fields })
}

fn parse_array_literal(state: &mut ParserState, element_tokens: Vec<Token>) -> Result<ASTNode, CodeError> {
    let mut elements = Vec::new();
    for _ in element_tokens {
        let element = parse_expression(state)?;
        elements.push(element);
    }
    Ok(ASTNode::ArrayLiteral(elements))
}

fn parse_struct_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(Token { token_type: TokenType::StructDeclaration(tokens), .. }) = advance(state) {
        let mut struct_tokens = tokens.into_iter();

        // Parse struct name
        let name = if let Some(Token { token_type: TokenType::StructName(name), .. }) = struct_tokens.next() {
            name
        } else {
            return Err(CodeError {
                message: "Expected struct name".to_string(),
                line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
                column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
            });
        };

        // Expect opening brace
        if !matches!(struct_tokens.next(), Some(Token { token_type: TokenType::BlockOpen, .. })) {
            return Err(CodeError {
                message: "Expected opening brace after struct name".to_string(),
                line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
                column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
            });
        }

        // Parse fields
        let mut fields = Vec::new();
        while let Some(token) = struct_tokens.next() {
            match token.token_type {
                TokenType::StructFieldName(field_name) => {
                    if let Some(Token { token_type: TokenType::TypeDeclaration(type_desc), .. }) = struct_tokens.next() {
                        fields.push((field_name, type_desc));
                    } else {
                        return Err(CodeError { message: "Expected type declaration for struct field".to_string(), line: token.start_line, column: token.start_column });
                    }
                }
                TokenType::BlockClose => break,
                _ => return Err(CodeError { message: format!("Unexpected token in struct declaration: {:?}", token.token_type), line: token.start_line, column: token.start_column }),
            }
        }

        Ok(ASTNode::StructDeclaration { name, fields })
    } else {
        Err(CodeError {
            message: "Expected struct declaration".to_string(),
            line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
        })
    }
}

fn parse_enum_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(Token { token_type: TokenType::EnumDeclaration(tokens), .. }) = advance(state) {
        let mut enum_tokens = tokens.into_iter();

        // Parse enum name
        let name = if let Some(Token { token_type: TokenType::EnumName(name), .. }) = enum_tokens.next() {
            name
        } else {
            return Err(CodeError {
                message: "Expected enum name".to_string(),
                line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
                column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
            });
        };

        // Expect opening brace
        if !matches!(enum_tokens.next(), Some(Token { token_type: TokenType::BlockOpen, .. })) {
            return Err(CodeError {
                message: "Expected opening brace after enum name".to_string(),
                line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
                column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
            });
        }

        // Parse variants
        let mut variants = Vec::new();
        while let Some(token) = enum_tokens.next() {
            match token.token_type {
                TokenType::EnumVariant(variant) => {
                    variants.push(variant);
                }
                TokenType::BlockClose => break,
                _ => return Err(CodeError { message: format!("Unexpected token in enum declaration: {:?}", token.token_type), line: token.start_line, column: token.start_column }),
            }
        }

        Ok(ASTNode::EnumDeclaration { name, variants })
    } else {
        Err(CodeError {
            message: "Expected enum declaration".to_string(),
            line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
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
                line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
                column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
            });
        };

        // Enter a new scope for the function
        enter_scope(state);

        let mut params = Vec::new();
        let mut return_type = NailDataTypeDescriptor::Void;

        // Parse parameters
        loop {
            match func_tokens.next() {
                Some(Token { token_type: TokenType::Identifier(param_name), .. }) => {
                    if let Some(Token { token_type: TokenType::TypeDeclaration(type_desc), .. }) = func_tokens.next() {
                        params.push((param_name.clone(), type_desc.clone()));
                        state.current_scope.declare(param_name, type_desc);

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
                                    line: other.start_line,
                                    column: other.start_column,
                                })
                            }
                            None => {
                                return Err(CodeError {
                                    message: "Unexpected end of function declaration".to_string(),
                                    line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
                                    column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
                                })
                            }
                        }
                    } else {
                        return Err(CodeError {
                            message: "Expected type declaration for function parameter".to_string(),
                            line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
                            column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
                        });
                    }
                }
                Some(Token { token_type: TokenType::FunctionReturnTypeDeclaration(rt), .. }) => {
                    return_type = rt;
                    break;
                }
                Some(other) => return Err(CodeError { message: format!("Unexpected token in function declaration: {:?}", other.token_type), line: other.start_line, column: other.start_column }),
                None => {
                    return Err(CodeError {
                        message: "Unexpected end of function declaration".to_string(),
                        line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
                        column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
                    })
                }
            }
        }

        // Parse function body
        let body = Box::new(parse_block(state)?);

        // Exit the function scope
        exit_scope(state);

        Ok(ASTNode::FunctionDeclaration { name, params, return_type, body })
    } else {
        Err(CodeError {
            message: "Expected function declaration".to_string(),
            line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
            column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
        })
    }
}

fn parse_rust_escape(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(token) = advance(state) {
        if let TokenType::RustEscape(content) = token.token_type {
            let mut ast_nodes = Vec::new();
            for inner_token in content {
                match inner_token.token_type {
                    TokenType::RustLiteral(literal) => {
                        ast_nodes.push(ASTNode::RustLiteral(literal));
                    }
                    TokenType::RustNailInsert(nail_tokens) => {
                        let mut nail_state = ParserState {
                            tokens: nail_tokens.into_iter().peekable(),
                            current_token: None,
                            previous_token: None,
                            current_scope: state.current_scope.clone(), // Use the current parser state's scope
                        };
                        let nail_ast = parse_inner(&mut nail_state)?;
                        ast_nodes.push(ASTNode::NailInjection(Box::new(nail_ast)));
                    }
                    _ => {
                        return Err(CodeError { message: format!("Unexpected token in RustEscape: {:?}", inner_token.token_type), line: inner_token.start_line, column: inner_token.start_column });
                    }
                }
            }
            Ok(ASTNode::RustEscape(ast_nodes))
        } else {
            Err(CodeError { message: format!("Expected RustEscape, found {:?}", token.token_type), line: token.start_line, column: token.start_column })
        }
    } else {
        Err(CodeError { message: "Unexpected end of file".to_string(), line: 0, column: 0 })
    }
}

fn parse_const_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    expect_token(state, TokenType::ConstDeclaration)?;
    let name = expect_identifier(state)?;
    let data_type = parse_type_declaration(state)?;
    expect_token(state, TokenType::Assignment)?;
    let value = Box::new(parse_expression(state)?);
    expect_token(state, TokenType::EndStatementOrExpression)?;

    // Register the constant in the current scope
    state.current_scope.declare(name.clone(), data_type.clone());

    Ok(ASTNode::ConstDeclaration { name, data_type, value })
}

fn parse_variable_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    expect_token(state, TokenType::VariableDeclaration)?;
    let name = expect_identifier(state)?;
    let data_type = parse_type_declaration(state)?;
    expect_token(state, TokenType::Assignment)?;
    let value = Box::new(parse_expression(state)?);
    expect_token(state, TokenType::EndStatementOrExpression)?;

    // Register the constant in the current scope
    state.current_scope.declare(name.clone(), data_type.clone());

    Ok(ASTNode::VariableDeclaration { name, data_type, value })
}

fn parse_if_statement(state: &mut ParserState) -> Result<ASTNode, CodeError> {
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
                TokenType::BlockClose => {
                    advance(state); // Consume closing brace
                    break;
                }
                _ => {
                    let condition = Box::new(parse_expression(state)?);
                    expect_token(state, TokenType::ArrowAssignment)?;
                    let branch = Box::new(parse_block(state)?);
                    condition_branch_pairs.push((condition, branch));

                    // Check for comma after each pair except the last one
                    if let Some(TokenType::Comma) = state.tokens.peek().map(|t| &t.token_type) {
                        advance(state);
                    }
                }
            }
        } else {
            return Err(CodeError {
                message: "Unexpected end of if statement".to_string(),
                line: state.previous_token.as_ref().map_or(0, |t| t.start_line),
                column: state.previous_token.as_ref().map_or(0, |t| t.start_column),
            });
        }
    }

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
    let value = Box::new(parse_expression(state)?);
    expect_token(state, TokenType::EndStatementOrExpression)?;
    Ok(ASTNode::ReturnStatement(value))
}

fn parse_type_declaration(state: &mut ParserState) -> Result<NailDataTypeDescriptor, CodeError> {
    if let Some(Token { token_type: TokenType::TypeDeclaration(data_type), .. }) = state.tokens.peek().cloned() {
        advance(state); // Consume the type declaration token
        Ok(data_type)
    } else {
        let error = CodeError {
            message: format!("Expected type declaration, found {:?}", state.tokens.peek().map(|token| token.token_type.clone()).unwrap_or(TokenType::EndOfFile)),
            line: state.tokens.peek().map_or(0, |token| token.start_line),
            column: state.tokens.peek().map_or(0, |token| token.start_column),
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
    fn test_parse_rust_escape() {
        let input = r#"c name:s = "bob"; R{println!("Hello, ^[name]^!"); }"#;
        let result = parse(lexer(input)).unwrap();
        // println!("RESULT: {:#?}", result);
        let what_the_ast_should_be: String = r#"Program(
            [
                ConstDeclaration {
                    name: "name",
                    data_type: String,
                    value: StringLiteral(
                        "bob",
                    ),
                },
                RustEscape(
                    [
                        RustLiteral(
                            "println!(\"Hello, ",
                        ),
                        NailInjection(
                            Program(
                                [
                                    Identifier(
                                        "name",
                                    ),
                                ],
                            ),
                        ),
                        RustLiteral(
                            "!\"); ",
                        ),
                    ],
                ),
            ],
        )"#
        .to_string();

        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(&what_the_ast_should_be));
    }

    #[test]
    fn test_if_statement() {
        let input = "if { a > 5 => {} };";
        let result = parse(lexer(input)).unwrap();
        let expected = r#"
        Program(
            [
                IfStatement {
                    condition: BinaryOperation {
                        left: Identifier(
                            "a",
                        ),
                        operator: Gt,
                        right: NumberLiteral(
                            "5",
                        ),
                    },
                    then_branch: Block(
                        [],
                    ),
                    else_branch: None,
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
        Program(
            [
                IfStatement {
                    condition: BinaryOperation {
                        left: Identifier(
                            "a",
                        ),
                        operator: Gt,
                        right: NumberLiteral(
                            "5",
                        ),
                    },
                    then_branch: Block(
                        [],
                    ),
                    else_branch: Some(
                        Block(
                            [],
                        ),
                    ),
                },
            ],
        )
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
    fn test_array_declaration() {
        let input = "v test_array:a:i = [1, 2, 3];";
        let result = parse(lexer(input)).unwrap();
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
    fn test_struct_declaration() {
        let input = "struct Point { x:i, y:i }";
        let result = parse(lexer(input)).unwrap();
        let expected = r#"
        Program(
            [
                StructDeclaration {
                    name: "Point",
                    fields: [
                        (
                            "x",
                            Int,
                        ),
                        (
                            "y",
                            Int,
                        ),
                    ],
                },
            ],
        )
        "#;
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }

    #[test]
    fn test_enum_declaration() {
        let input = "enum Color { Red, Green, Blue }";
        let result = parse(lexer(input)).unwrap();
        let expected = r#"
        Program(
            [
                EnumDeclaration {
                    name: "Color",
                    variants: [
                        "Red",
                        "Green",
                        "Blue",
                    ],
                },
            ],
        )
        "#;
        assert_eq!(remove_whitespace(&format!("{:#?}", result)), remove_whitespace(expected));
    }
}
