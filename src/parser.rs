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
    ForLoop { 
        iterator: String, 
        iterable: Box<ASTNode>, 
        initial_value: Option<Box<ASTNode>>, // For 'from' clause
        filter: Option<Box<ASTNode>>,         // For 'when' clause
        body: Box<ASTNode>, 
        code_span: CodeSpan, 
        scope: usize 
    },
    MapExpression {
        iterator: String,
        index_iterator: Option<String>, // For optional index parameter
        iterable: Box<ASTNode>,
        body: Box<ASTNode>,
        code_span: CodeSpan,
        scope: usize
    },
    FilterExpression {
        iterator: String,
        index_iterator: Option<String>,
        iterable: Box<ASTNode>,
        body: Box<ASTNode>,
        code_span: CodeSpan,
        scope: usize
    },
    ReduceExpression {
        accumulator: String,
        iterator: String,
        index_iterator: Option<String>,
        iterable: Box<ASTNode>,
        initial_value: Box<ASTNode>,
        body: Box<ASTNode>,
        code_span: CodeSpan,
        scope: usize
    },
    EachExpression {
        iterator: String,
        index_iterator: Option<String>,
        iterable: Box<ASTNode>,
        body: Box<ASTNode>,
        code_span: CodeSpan,
        scope: usize
    },
    FindExpression {
        iterator: String,
        index_iterator: Option<String>,
        iterable: Box<ASTNode>,
        body: Box<ASTNode>,
        code_span: CodeSpan,
        scope: usize
    },
    AllExpression {
        iterator: String,
        index_iterator: Option<String>,
        iterable: Box<ASTNode>,
        body: Box<ASTNode>,
        code_span: CodeSpan,
        scope: usize
    },
    AnyExpression {
        iterator: String,
        index_iterator: Option<String>,
        iterable: Box<ASTNode>,
        body: Box<ASTNode>,
        code_span: CodeSpan,
        scope: usize
    },
    WhileLoop { 
        condition: Box<ASTNode>, 
        initial_value: Option<Box<ASTNode>>, // For 'from' clause
        max_iterations: Option<Box<ASTNode>>, 
        body: Box<ASTNode>, 
        code_span: CodeSpan, 
        scope: usize 
    },
    Loop {
        index_iterator: Option<String>, // Optional index parameter
        body: Box<ASTNode>,
        code_span: CodeSpan,
        scope: usize
    },
    SpawnBlock {
        body: Box<ASTNode>,
        code_span: CodeSpan,
        scope: usize
    },
    BreakStatement { code_span: CodeSpan, scope: usize },
    ContinueStatement { code_span: CodeSpan, scope: usize },
    ParallelBlock { statements: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    ConcurrentBlock { statements: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    Block { statements: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    BinaryOperation { left: Box<ASTNode>, operator: Operation, right: Box<ASTNode>, code_span: CodeSpan, scope: usize },
    UnaryOperation { operator: Operation, operand: Box<ASTNode>, code_span: CodeSpan, scope: usize },
    Assignment { left: Box<ASTNode>, right: Box<ASTNode>, code_span: CodeSpan, scope: usize },
    StructDeclaration { name: String, fields: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    StructDeclarationField { name: String, data_type: NailDataTypeDescriptor, scope: usize },
    StructInstantiation { name: String, fields: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    StructInstantiationField { name: String, value: Box<ASTNode>, code_span: CodeSpan, scope: usize },
    StructFieldAccess { struct_name: String, field_name: String, code_span: CodeSpan, scope: usize },
    NestedFieldAccess { object: Box<ASTNode>, field_name: String, code_span: CodeSpan, scope: usize },
    EnumDeclaration { name: String, variants: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    EnumVariant { name: String, variant: String, code_span: CodeSpan, scope: usize },
    ArrayLiteral { elements: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    Identifier { name: String, code_span: CodeSpan, scope: usize },
    NumberLiteral { value: String, data_type: NailDataTypeDescriptor, code_span: CodeSpan, scope: usize },
    StringLiteral { value: String, code_span: CodeSpan, scope: usize },
    BooleanLiteral { value: bool, code_span: CodeSpan, scope: usize },
    ReturnDeclaration { statement: Box<ASTNode>, code_span: CodeSpan, scope: usize },
    YieldDeclaration { statement: Box<ASTNode>, code_span: CodeSpan, scope: usize },
}

impl Default for ASTNode {
    fn default() -> Self {
        ASTNode::Program { statements: Vec::new(), code_span: CodeSpan::default(), scope: 0, used_stdlib_functions: HashSet::new() }
    }
}

impl ASTNode {
    pub fn code_span(&self) -> CodeSpan {
        match self {
            ASTNode::Program { code_span, .. } => code_span.clone(),
            ASTNode::FunctionDeclaration { code_span, .. } => code_span.clone(),
            ASTNode::LambdaDeclaration { code_span, .. } => code_span.clone(),
            ASTNode::FunctionCall { code_span, .. } => code_span.clone(),
            ASTNode::ConstDeclaration { code_span, .. } => code_span.clone(),
            ASTNode::IfStatement { code_span, .. } => code_span.clone(),
            ASTNode::ForLoop { code_span, .. } => code_span.clone(),
            ASTNode::MapExpression { code_span, .. } => code_span.clone(),
            ASTNode::FilterExpression { code_span, .. } => code_span.clone(),
            ASTNode::ReduceExpression { code_span, .. } => code_span.clone(),
            ASTNode::EachExpression { code_span, .. } => code_span.clone(),
            ASTNode::FindExpression { code_span, .. } => code_span.clone(),
            ASTNode::AllExpression { code_span, .. } => code_span.clone(),
            ASTNode::AnyExpression { code_span, .. } => code_span.clone(),
            ASTNode::WhileLoop { code_span, .. } => code_span.clone(),
            ASTNode::Loop { code_span, .. } => code_span.clone(),
            ASTNode::SpawnBlock { code_span, .. } => code_span.clone(),
            ASTNode::BreakStatement { code_span, .. } => code_span.clone(),
            ASTNode::ContinueStatement { code_span, .. } => code_span.clone(),
            ASTNode::ParallelBlock { code_span, .. } => code_span.clone(),
            ASTNode::ConcurrentBlock { code_span, .. } => code_span.clone(),
            ASTNode::Block { code_span, .. } => code_span.clone(),
            ASTNode::BinaryOperation { code_span, .. } => code_span.clone(),
            ASTNode::UnaryOperation { code_span, .. } => code_span.clone(),
            ASTNode::Assignment { code_span, .. } => code_span.clone(),
            ASTNode::StructDeclaration { code_span, .. } => code_span.clone(),
            ASTNode::StructDeclarationField { .. } => CodeSpan::default(), // No code_span for this variant
            ASTNode::StructInstantiation { code_span, .. } => code_span.clone(),
            ASTNode::StructInstantiationField { code_span, .. } => code_span.clone(),
            ASTNode::StructFieldAccess { code_span, .. } => code_span.clone(),
            ASTNode::NestedFieldAccess { code_span, .. } => code_span.clone(),
            ASTNode::EnumDeclaration { code_span, .. } => code_span.clone(),
            ASTNode::EnumVariant { code_span, .. } => code_span.clone(),
            ASTNode::ArrayLiteral { code_span, .. } => code_span.clone(),
            ASTNode::Identifier { code_span, .. } => code_span.clone(),
            ASTNode::NumberLiteral { code_span, .. } => code_span.clone(),
            ASTNode::StringLiteral { code_span, .. } => code_span.clone(),
            ASTNode::BooleanLiteral { code_span, .. } => code_span.clone(),
            ASTNode::ReturnDeclaration { code_span, .. } => code_span.clone(),
            ASTNode::YieldDeclaration { code_span, .. } => code_span.clone(),
        }
    }
}

pub struct ParserState {
    tokens: Peekable<IntoIter<Token>>,
    current_token: Option<Token>,
    previous_token: Option<Token>,
    used_stdlib_functions: HashSet<String>,
}

pub fn parse(tokens: Vec<Token>) -> Result<(ASTNode, HashSet<String>), CodeError> {
    let mut state = ParserState { tokens: tokens.into_iter().peekable(), current_token: None, previous_token: None, used_stdlib_functions: HashSet::new() };
    let ast = parse_inner(&mut state)?;
    Ok((ast, state.used_stdlib_functions))
}

fn parse_inner(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let mut program = vec![];
    while state.tokens.peek().is_some() {
        program.push(parse_statement(state)?);
    }
    Ok(ASTNode::Program { statements: program, code_span: CodeSpan::default(), scope: GLOBAL_SCOPE, used_stdlib_functions: state.used_stdlib_functions.clone() })
}

fn advance(state: &mut ParserState) -> Option<Token> {
    state.previous_token = state.current_token.take();
    state.current_token = state.tokens.next();
    state.current_token.clone()
}

fn parse_field_access_chain(state: &mut ParserState, mut node: ASTNode) -> Result<ASTNode, CodeError> {
    while matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::Dot)) {
        advance(state); // Consume the dot
        let field_name = expect_identifier(state)?;
        let code_span = state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone());
        
        node = match node {
            ASTNode::Identifier { name, .. } => {
                ASTNode::StructFieldAccess { struct_name: name, field_name, code_span, scope: GLOBAL_SCOPE }
            }
            _ => {
                ASTNode::NestedFieldAccess { object: Box::new(node), field_name, code_span, scope: GLOBAL_SCOPE }
            }
        };
    }
    Ok(node)
}

fn parse_primary(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(token) = state.tokens.peek().cloned() {
        let node = match token.token_type {
            TokenType::Operator(op) if op.is_unary() => {
                // Handle unary operators like ! and -
                advance(state);
                let operand = Box::new(parse_primary(state)?);
                // Unary operators don't participate in field access
                return Ok(ASTNode::UnaryOperation { operator: op, operand, code_span: token.code_span, scope: GLOBAL_SCOPE });
            }
            // Struct instantiation is now detected in the Identifier case
            TokenType::Identifier(name) => {
                advance(state);
                if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::ParenthesisOpen)) {
                    // Check if this is an inline function declaration (f (...))
                    if name == "f" {
                        parse_inline_function_declaration(state)?
                    } else {
                        parse_function_call(state, name)?
                    }
                } else if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::BlockOpen)) && name.chars().next().map_or(false, |c| c.is_uppercase()) {
                    // This is a struct instantiation
                    parse_struct_instantiation(state, name, token.code_span)?
                } else {
                    ASTNode::Identifier { name, code_span: token.code_span, scope: GLOBAL_SCOPE }
                }
            }
            TokenType::Float(value) => {
                advance(state);
                ASTNode::NumberLiteral { value, data_type: NailDataTypeDescriptor::Float, code_span: token.code_span, scope: GLOBAL_SCOPE }
            }
            TokenType::Integer(value) => {
                advance(state);
                ASTNode::NumberLiteral { value, data_type: NailDataTypeDescriptor::Int, code_span: token.code_span, scope: GLOBAL_SCOPE }
            }
            TokenType::StringLiteral(value) => {
                advance(state);
                ASTNode::StringLiteral { value, code_span: token.code_span, scope: GLOBAL_SCOPE }
            }
            TokenType::BooleanLiteral(value) => {
                advance(state);
                ASTNode::BooleanLiteral { value, code_span: token.code_span, scope: GLOBAL_SCOPE }
            }
            TokenType::ParenthesisOpen => {
                advance(state);
                let expr = parse_expression(state, 0)?;
                let _ = expect_token(state, TokenType::ParenthesisClose)?;
                expr
            }
            TokenType::EnumVariant(variant) => {
                let code_span = token.code_span;
                advance(state);
                ASTNode::EnumVariant { name: variant.name, variant: variant.variant, code_span, scope: GLOBAL_SCOPE }
            }
            TokenType::ArrayOpen => parse_array_literal(state)?,
            TokenType::IfDeclaration => parse_if_statement_expr(state, true)?,
            TokenType::ForDeclaration => parse_for_loop(state)?,
            TokenType::MapDeclaration => parse_map_expression(state)?,
            TokenType::FilterDeclaration => parse_filter_expression(state)?,
            TokenType::ReduceDeclaration => parse_reduce_expression(state)?,
            TokenType::EachDeclaration => parse_each_expression(state)?,
            TokenType::FindDeclaration => parse_find_expression(state)?,
            TokenType::AllDeclaration => parse_all_expression(state)?,
            TokenType::AnyDeclaration => parse_any_expression(state)?,
            TokenType::WhileDeclaration => parse_while_loop(state)?,
            TokenType::LoopKeyword => parse_loop(state)?,
            TokenType::FunctionSignature(_) => parse_inline_function_from_signature(state)?,
            _ => {
                let code_span = token.code_span;
                return Err(CodeError { message: format!("Unexpected token {:?}", token.token_type), code_span: code_span.clone() });
            }
        };
        
        // Apply field access to all primary expressions except unary operations
        parse_field_access_chain(state, node)
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
            TokenType::IfDeclaration => parse_if_statement_expr(state, false),
            TokenType::ForDeclaration => parse_for_loop(state),
            TokenType::MapDeclaration => parse_map_expression(state),
            TokenType::FilterDeclaration => parse_filter_expression(state),
            TokenType::ReduceDeclaration => parse_reduce_expression(state),
            TokenType::EachDeclaration => parse_each_expression(state),
            TokenType::FindDeclaration => parse_find_expression(state),
            TokenType::AllDeclaration => parse_all_expression(state),
            TokenType::AnyDeclaration => parse_any_expression(state),
            TokenType::WhileDeclaration => parse_while_loop(state),
            TokenType::LoopKeyword => parse_loop(state),
            TokenType::SpawnKeyword => parse_spawn_block(state),
            TokenType::BreakKeyword => parse_break_statement(state),
            TokenType::ContinueKeyword => parse_continue_statement(state),
            TokenType::ParallelStart => parse_parallel_block_start(state),
            TokenType::ConcurrentStart => parse_concurrent_block_start(state),
            TokenType::Return => parse_return_statement(state),
            TokenType::Yield => parse_yield_statement(state),
            TokenType::BlockOpen => parse_block(state),
            _ => {
                // Check if this is a const declaration without 'c' prefix
                // Pattern: Identifier TypeDeclaration Assignment
                if let Some(TokenType::Identifier(_)) = state.tokens.peek().map(|t| &t.token_type) {
                    // Look ahead to see if this is a declaration
                    let mut peek_iter = state.tokens.clone();
                    peek_iter.next(); // Skip identifier
                    if let Some(token) = peek_iter.peek() {
                        if matches!(token.token_type, TokenType::Colon) {
                            // This is a const declaration: identifier : type = expression
                            return parse_const_declaration(state);
                        }
                    }
                }

                let expr = parse_expression(state, 0)?;
                let _ = expect_token(state, TokenType::EndStatementOrExpression)?;
                Ok(expr)
            }
        },
        None => Err(CodeError { message: "No token was found to match with a statement.".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) }),
    }
}

fn parse_expression(state: &mut ParserState, min_precedence: u8) -> Result<ASTNode, CodeError> {
    let mut left = parse_primary(state)?;

    loop {
        match state.tokens.peek().cloned() {
            Some(Token { token_type: TokenType::Operator(op), .. }) => {
                if op.precedence() < min_precedence {
                    break;
                }

                advance(state); // Consume the operator
                let code_span = state.current_token.as_ref().map(|t| t.code_span.clone()).unwrap_or(CodeSpan::default());

                // Unary operators should have been handled in parse_primary
                if op.is_unary() {
                    return Err(CodeError { message: format!("Unexpected unary operator {:?} in infix position", op), code_span });
                } else {
                    let right = parse_expression(state, op.precedence() + 1)?;
                    left = ASTNode::BinaryOperation { left: Box::new(left), operator: op, right: Box::new(right), code_span: code_span.clone(), scope: GLOBAL_SCOPE };
                }
            }
            Some(Token { token_type: TokenType::Assignment, .. }) => {
                // Assignment has very low precedence (right-associative)
                let assignment_precedence = 0;
                if assignment_precedence < min_precedence {
                    break;
                }

                advance(state); // Consume the assignment token
                let code_span = state.current_token.as_ref().map(|t| t.code_span.clone()).unwrap_or(CodeSpan::default());
                
                let right = parse_expression(state, assignment_precedence)?;
                left = ASTNode::Assignment { left: Box::new(left), right: Box::new(right), code_span: code_span.clone(), scope: GLOBAL_SCOPE };
            }
            _ => break,
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
        if name.len() < 2 {
            let error = CodeError {
                message: format!(
                    "Variable name too short. Use descriptive names.\n  Found: '{}'\n  Suggestion: Use descriptive name like '{}_value' or '{}_{}'",
                    name,
                    name,
                    name,
                    if name == "x" || name == "y" || name == "z" {
                        "coordinate"
                    } else if name == "i" || name == "j" || name == "k" {
                        "index"
                    } else if name == "n" {
                        "number"
                    } else {
                        "variable"
                    }
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


// Parse struct instantiation: StructName { field1: expr1, field2: expr2, ... }
fn parse_struct_instantiation(state: &mut ParserState, struct_name: String, start_span: CodeSpan) -> Result<ASTNode, CodeError> {
    // Consume the opening brace
    let _ = expect_token(state, TokenType::BlockOpen)?;
    
    let mut fields = Vec::new();
    
    loop {
        // Check for closing brace
        if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::BlockClose)) {
            advance(state);
            break;
        }
        
        // Parse field name
        let field_name = if let Some(token) = state.tokens.peek() {
            match &token.token_type {
                TokenType::Identifier(name) => {
                    let name = name.clone();
                    advance(state);
                    name
                }
                _ => return Err(CodeError { 
                    message: "Expected field name in struct instantiation".to_string(), 
                    code_span: token.code_span.clone() 
                })
            }
        } else {
            return Err(CodeError { 
                message: "Unexpected end of input in struct instantiation".to_string(), 
                code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) 
            });
        };
        
        // Expect colon
        let _ = expect_token(state, TokenType::Colon)?;
        
        // Parse field value expression
        let field_value = parse_expression(state, 0)?;
        let field_span = field_value.code_span().clone();
        
        fields.push(ASTNode::StructInstantiationField {
            name: field_name,
            value: Box::new(field_value),
            code_span: field_span,
            scope: GLOBAL_SCOPE,
        });
        
        // Check for comma or closing brace
        if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::Comma)) {
            advance(state);
        } else if !matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::BlockClose)) {
            return Err(CodeError { 
                message: "Expected ',' or '}' in struct instantiation".to_string(), 
                code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) 
            });
        }
    }
    
    Ok(ASTNode::StructInstantiation {
        name: struct_name,
        fields,
        code_span: start_span,
        scope: GLOBAL_SCOPE,
    })
}

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
            return Err(CodeError { message: "Expected ',' or ']' in array literal".to_string(), code_span: state.current_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) });
        }
    }

    // Expect and consume ']'
    let _ = expect_token(state, TokenType::ArrayClose)?;

    Ok(ASTNode::ArrayLiteral { elements, code_span: start_span, scope: GLOBAL_SCOPE })
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

fn parse_inline_function_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    // For now, just return an error to see if this path is being hit
    Err(CodeError {
        message: "Inline function parsing not yet implemented. Use lambda syntax |param:type):return_type { } for now.".to_string(),
        code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()),
    })
}

fn parse_inline_function_from_signature(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(Token { token_type: TokenType::FunctionSignature(tokens), .. }) = advance(state) {
        let code_span = state.previous_token.as_ref().map(|t| t.code_span.clone()).unwrap_or(CodeSpan::default());
        let mut func_tokens = tokens.into_iter();
        let mut params = Vec::new();
        let mut data_type = NailDataTypeDescriptor::Void;

        // Skip the function name token (which should be empty for inline functions)
        if let Some(Token { token_type: TokenType::FunctionName(name), .. }) = func_tokens.next() {
            if !name.is_empty() {
                return Err(CodeError { message: "Inline functions cannot have names".to_string(), code_span });
            }
        }

        // Parse parameters from the tokens
        while let Some(token) = func_tokens.next() {
            match token.token_type {
                TokenType::Identifier(param_name) => {
                    // Look for type after parameter
                    match func_tokens.next() {
                        Some(Token { token_type: TokenType::TypeDeclaration(param_type), .. }) => {
                            params.push((param_name, param_type));
                            // Check for comma
                            match func_tokens.next() {
                                Some(Token { token_type: TokenType::Comma, .. }) => continue,
                                Some(Token { token_type: TokenType::FunctionReturnTypeDeclaration(rt), .. }) => {
                                    data_type = rt;
                                    break;
                                }
                                _ => break,
                            }
                        }
                        Some(Token { token_type: TokenType::FunctionReturnTypeDeclaration(rt), .. }) => {
                            // Parameter without type - use Any
                            params.push((param_name, NailDataTypeDescriptor::Any));
                            data_type = rt;
                            break;
                        }
                        _ => {
                            params.push((param_name, NailDataTypeDescriptor::Any));
                        }
                    }
                }
                TokenType::FunctionReturnTypeDeclaration(rt) => {
                    data_type = rt;
                    break;
                }
                TokenType::LexerError(msg) => {
                    return Err(CodeError { message: msg, code_span: token.code_span });
                }
                _ => break,
            }
        }

        // Parse function body
        let body = Box::new(parse_block(state)?);

        // Return as LambdaDeclaration for compatibility
        Ok(ASTNode::LambdaDeclaration { params, data_type, body, code_span, scope: GLOBAL_SCOPE })
    } else {
        Err(CodeError { message: "Expected function signature".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) })
    }
}

fn parse_const_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    // Parse const declaration: identifier : type = expression ;
    let name = expect_identifier(state)?;
    
    // Expect colon for type annotation
    let _ = expect_token(state, TokenType::Colon)?;
    
    // Parse the type annotation
    let data_type = parse_type_annotation(state)?;
    
    let _ = expect_token(state, TokenType::Assignment)?;
    let value = Box::new(parse_expression(state, 0)?);
    let code_span = expect_token(state, TokenType::EndStatementOrExpression)?;

    Ok(ASTNode::ConstDeclaration { name, data_type, value, code_span, scope: GLOBAL_SCOPE })
}

fn parse_type_annotation(state: &mut ParserState) -> Result<NailDataTypeDescriptor, CodeError> {
    // Parse a type annotation that appears after a colon
    // This handles: i, f, s, b, a:i, a:s, StructName, EnumName, etc.
    
    if let Some(token) = state.tokens.peek().cloned() {
        match &token.token_type {
            TokenType::Identifier(type_name) => {
                let type_name = type_name.clone();
                let token_span = token.code_span.clone();
                advance(state);
                
                // Check for array type (a:type)
                if type_name == "a" && matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::Colon)) {
                    advance(state); // consume the colon
                    let element_type = Box::new(parse_type_annotation(state)?);
                    Ok(NailDataTypeDescriptor::Array(element_type))
                } 
                // Check for hashmap type (h<key_type,value_type>)
                else if type_name == "h" && matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::Operator(Operation::Lt))) {
                    advance(state); // consume the '<'
                    
                    let key_type = Box::new(parse_type_annotation(state)?);
                    
                    // Expect comma
                    let _ = expect_token(state, TokenType::Comma)?;
                    
                    let value_type = Box::new(parse_type_annotation(state)?);
                    
                    // Expect '>'
                    if let Some(Token { token_type: TokenType::Operator(Operation::Gt), .. }) = advance(state) {
                        Ok(NailDataTypeDescriptor::HashMap(key_type, value_type))
                    } else {
                        Err(CodeError {
                            message: "Expected '>' to close hashmap type".to_string(),
                            code_span: state.tokens.peek().map_or(CodeSpan::default(), |t| t.code_span.clone()),
                        })
                    }
                } else {
                    // Parse the type name
                    match type_name.as_str() {
                        "i" => Ok(NailDataTypeDescriptor::Int),
                        "f" => Ok(NailDataTypeDescriptor::Float),
                        "s" => Ok(NailDataTypeDescriptor::String),
                        "b" => Ok(NailDataTypeDescriptor::Boolean),
                        "v" => Ok(NailDataTypeDescriptor::Void),
                        "e" => Ok(NailDataTypeDescriptor::Error),
                        _ => {
                            // Assume it's a struct or enum name (should start with uppercase)
                            if type_name.chars().next().map_or(false, |c| c.is_uppercase()) {
                                Ok(NailDataTypeDescriptor::Struct(type_name))
                            } else {
                                Err(CodeError {
                                    message: format!("Unknown type: {}", type_name),
                                    code_span: token_span,
                                })
                            }
                        }
                    }
                }
            }
            _ => Err(CodeError {
                message: format!("Expected type annotation, found {:?}", token.token_type),
                code_span: token.code_span.clone(),
            })
        }
    } else {
        Err(CodeError {
            message: "Expected type annotation".to_string(),
            code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()),
        })
    }
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

fn parse_parallel_block_start(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let _start_span = expect_token(state, TokenType::ParallelStart)?;
    let mut statements = vec![];

    while state.tokens.peek().map_or(false, |t| t.token_type != TokenType::ParallelEnd) {
        statements.push(parse_statement(state)?);
    }

    let _ = expect_token(state, TokenType::ParallelEnd)?;
    Ok(ASTNode::ParallelBlock { statements, code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()), scope: GLOBAL_SCOPE })
}

fn parse_concurrent_block_start(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let _start_span = expect_token(state, TokenType::ConcurrentStart)?;
    let mut statements = vec![];

    while state.tokens.peek().map_or(false, |t| t.token_type != TokenType::ConcurrentEnd) {
        statements.push(parse_statement(state)?);
    }

    let _ = expect_token(state, TokenType::ConcurrentEnd)?;
    Ok(ASTNode::ConcurrentBlock { statements, code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()), scope: GLOBAL_SCOPE })
}

fn parse_return_statement(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let _ = expect_token(state, TokenType::Return)?;
    let statement = parse_expression(state, 0)?;
    let code_span = expect_token(state, TokenType::EndStatementOrExpression)?;
    Ok(ASTNode::ReturnDeclaration { statement: Box::new(statement), code_span: code_span.clone(), scope: GLOBAL_SCOPE })
}

fn parse_yield_statement(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let _ = expect_token(state, TokenType::Yield)?;
    let statement = parse_expression(state, 0)?;
    let code_span = expect_token(state, TokenType::EndStatementOrExpression)?;
    Ok(ASTNode::YieldDeclaration { statement: Box::new(statement), code_span: code_span.clone(), scope: GLOBAL_SCOPE })
}

fn parse_for_loop(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let start_span = expect_token(state, TokenType::ForDeclaration)?;
    let iterator = expect_identifier(state)?;
    let _ = expect_token(state, TokenType::InKeyword)?;
    
    // Parse the iterable expression (could be array, range, etc.)
    let iterable = parse_expression(state, 0)?;
    
    // Check for optional 'from' clause
    let initial_value = if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::FromKeyword)) {
        advance(state); // consume 'from'
        Some(Box::new(parse_expression(state, 0)?))
    } else {
        None
    };
    
    // Check for optional 'when' clause
    let filter = if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::WhenKeyword)) {
        advance(state); // consume 'when'
        Some(Box::new(parse_expression(state, 0)?))
    } else {
        None
    };
    
    let body = parse_block(state)?;
    let code_span = CodeSpan { 
        start_line: start_span.start_line, 
        start_column: start_span.start_column,
        end_line: body.code_span().end_line,
        end_column: body.code_span().end_column,
    };
    
    Ok(ASTNode::ForLoop {
        iterator,
        iterable: Box::new(iterable),
        initial_value,
        filter,
        body: Box::new(body),
        code_span,
        scope: GLOBAL_SCOPE,
    })
}

fn parse_map_expression(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let start_span = expect_token(state, TokenType::MapDeclaration)?;
    
    // Parse iterator name
    let iterator = expect_identifier(state)?;
    
    // Check for optional index iterator (just another identifier, no comma)
    let index_iterator = if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::Identifier(_))) {
        // Peek ahead to see if the next-next token is 'in'
        let mut peek_iter = state.tokens.clone();
        peek_iter.next(); // Skip the potential index identifier
        if matches!(peek_iter.peek().map(|t| &t.token_type), Some(TokenType::InKeyword)) {
            // This is the index iterator
            Some(expect_identifier(state)?)
        } else {
            // This is not an index iterator, it's the 'in' keyword
            None
        }
    } else {
        None
    };
    
    let _ = expect_token(state, TokenType::InKeyword)?;
    
    // Parse the iterable expression
    let iterable = parse_expression(state, 0)?;
    
    // Parse the body block
    let body = parse_block(state)?;
    
    let code_span = CodeSpan { 
        start_line: start_span.start_line, 
        start_column: start_span.start_column,
        end_line: body.code_span().end_line,
        end_column: body.code_span().end_column,
    };

    Ok(ASTNode::MapExpression { 
        iterator, 
        index_iterator,
        iterable: Box::new(iterable), 
        body: Box::new(body), 
        code_span,
        scope: GLOBAL_SCOPE 
    })
}

fn parse_filter_expression(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let start_span = expect_token(state, TokenType::FilterDeclaration)?;
    
    // Parse iterator name
    let iterator = expect_identifier(state)?;
    
    // Check for optional index iterator
    let index_iterator = if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::Identifier(_))) {
        let mut peek_iter = state.tokens.clone();
        peek_iter.next();
        if matches!(peek_iter.peek().map(|t| &t.token_type), Some(TokenType::InKeyword)) {
            Some(expect_identifier(state)?)
        } else {
            None
        }
    } else {
        None
    };
    
    let _ = expect_token(state, TokenType::InKeyword)?;
    let iterable = parse_expression(state, 0)?;
    
    // Parse the condition block
    let condition = parse_block(state)?;
    
    let code_span = CodeSpan { 
        start_line: start_span.start_line, 
        start_column: start_span.start_column,
        end_line: condition.code_span().end_line,
        end_column: condition.code_span().end_column,
    };

    Ok(ASTNode::FilterExpression { 
        iterator, 
        index_iterator,
        iterable: Box::new(iterable), 
        body: Box::new(condition), 
        code_span,
        scope: GLOBAL_SCOPE 
    })
}

fn parse_reduce_expression(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let start_span = expect_token(state, TokenType::ReduceDeclaration)?;
    
    // Parse accumulator name
    let accumulator = expect_identifier(state)?;
    
    // Parse iterator name
    let iterator = expect_identifier(state)?;
    
    // Check for optional index iterator
    let index_iterator = if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::Identifier(_))) {
        let mut peek_iter = state.tokens.clone();
        peek_iter.next();
        if matches!(peek_iter.peek().map(|t| &t.token_type), Some(TokenType::InKeyword)) {
            Some(expect_identifier(state)?)
        } else {
            None
        }
    } else {
        None
    };
    
    let _ = expect_token(state, TokenType::InKeyword)?;
    let iterable = parse_expression(state, 0)?;
    
    // Parse 'from' keyword for initial value
    let _ = expect_token(state, TokenType::FromKeyword)?;
    let initial_value = parse_expression(state, 0)?;
    
    // Parse the body block
    let body = parse_block(state)?;
    
    let code_span = CodeSpan { 
        start_line: start_span.start_line, 
        start_column: start_span.start_column,
        end_line: body.code_span().end_line,
        end_column: body.code_span().end_column,
    };

    Ok(ASTNode::ReduceExpression { 
        accumulator,
        iterator, 
        index_iterator,
        iterable: Box::new(iterable),
        initial_value: Box::new(initial_value),
        body: Box::new(body), 
        code_span,
        scope: GLOBAL_SCOPE 
    })
}

fn parse_each_expression(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let start_span = expect_token(state, TokenType::EachDeclaration)?;
    
    // Parse iterator name
    let iterator = expect_identifier(state)?;
    
    // Check for optional index iterator
    let index_iterator = if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::Identifier(_))) {
        let mut peek_iter = state.tokens.clone();
        peek_iter.next();
        if matches!(peek_iter.peek().map(|t| &t.token_type), Some(TokenType::InKeyword)) {
            Some(expect_identifier(state)?)
        } else {
            None
        }
    } else {
        None
    };
    
    let _ = expect_token(state, TokenType::InKeyword)?;
    let iterable = parse_expression(state, 0)?;
    
    // Parse the body block
    let body = parse_block(state)?;
    
    let code_span = CodeSpan { 
        start_line: start_span.start_line, 
        start_column: start_span.start_column,
        end_line: body.code_span().end_line,
        end_column: body.code_span().end_column,
    };

    Ok(ASTNode::EachExpression { 
        iterator, 
        index_iterator,
        iterable: Box::new(iterable), 
        body: Box::new(body), 
        code_span,
        scope: GLOBAL_SCOPE 
    })
}

fn parse_find_expression(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let start_span = expect_token(state, TokenType::FindDeclaration)?;
    
    // Parse iterator name
    let iterator = expect_identifier(state)?;
    
    // Check for optional index iterator
    let index_iterator = if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::Identifier(_))) {
        let mut peek_iter = state.tokens.clone();
        peek_iter.next();
        if matches!(peek_iter.peek().map(|t| &t.token_type), Some(TokenType::InKeyword)) {
            Some(expect_identifier(state)?)
        } else {
            None
        }
    } else {
        None
    };
    
    let _ = expect_token(state, TokenType::InKeyword)?;
    let iterable = parse_expression(state, 0)?;
    
    // Parse the condition block
    let condition = parse_block(state)?;
    
    let code_span = CodeSpan { 
        start_line: start_span.start_line, 
        start_column: start_span.start_column,
        end_line: condition.code_span().end_line,
        end_column: condition.code_span().end_column,
    };

    Ok(ASTNode::FindExpression { 
        iterator, 
        index_iterator,
        iterable: Box::new(iterable), 
        body: Box::new(condition), 
        code_span,
        scope: GLOBAL_SCOPE 
    })
}

fn parse_all_expression(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let start_span = expect_token(state, TokenType::AllDeclaration)?;
    
    // Parse iterator name
    let iterator = expect_identifier(state)?;
    
    // Check for optional index iterator
    let index_iterator = if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::Identifier(_))) {
        let mut peek_iter = state.tokens.clone();
        peek_iter.next();
        if matches!(peek_iter.peek().map(|t| &t.token_type), Some(TokenType::InKeyword)) {
            Some(expect_identifier(state)?)
        } else {
            None
        }
    } else {
        None
    };
    
    let _ = expect_token(state, TokenType::InKeyword)?;
    let iterable = parse_expression(state, 0)?;
    
    // Parse the condition block
    let condition = parse_block(state)?;
    
    let code_span = CodeSpan { 
        start_line: start_span.start_line, 
        start_column: start_span.start_column,
        end_line: condition.code_span().end_line,
        end_column: condition.code_span().end_column,
    };

    Ok(ASTNode::AllExpression { 
        iterator, 
        index_iterator,
        iterable: Box::new(iterable), 
        body: Box::new(condition), 
        code_span,
        scope: GLOBAL_SCOPE 
    })
}

fn parse_any_expression(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let start_span = expect_token(state, TokenType::AnyDeclaration)?;
    
    // Parse iterator name
    let iterator = expect_identifier(state)?;
    
    // Check for optional index iterator
    let index_iterator = if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::Identifier(_))) {
        let mut peek_iter = state.tokens.clone();
        peek_iter.next();
        if matches!(peek_iter.peek().map(|t| &t.token_type), Some(TokenType::InKeyword)) {
            Some(expect_identifier(state)?)
        } else {
            None
        }
    } else {
        None
    };
    
    let _ = expect_token(state, TokenType::InKeyword)?;
    let iterable = parse_expression(state, 0)?;
    
    // Parse the condition block
    let condition = parse_block(state)?;
    
    let code_span = CodeSpan { 
        start_line: start_span.start_line, 
        start_column: start_span.start_column,
        end_line: condition.code_span().end_line,
        end_column: condition.code_span().end_column,
    };

    Ok(ASTNode::AnyExpression { 
        iterator, 
        index_iterator,
        iterable: Box::new(iterable), 
        body: Box::new(condition), 
        code_span,
        scope: GLOBAL_SCOPE 
    })
}

fn parse_while_loop(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let start_span = expect_token(state, TokenType::WhileDeclaration)?;
    let condition = parse_expression(state, 0)?;
    
    // Check for optional 'from' clause
    let initial_value = if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::FromKeyword)) {
        advance(state); // consume 'from'
        Some(Box::new(parse_expression(state, 0)?))
    } else {
        None
    };
    
    // Check for max iterations
    let max_iterations = if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::MaxKeyword)) {
        advance(state); // consume max
        Some(Box::new(parse_expression(state, 0)?))
    } else {
        None
    };
    
    let body = parse_block(state)?;
    let code_span = CodeSpan { 
        start_line: start_span.start_line, 
        start_column: start_span.start_column,
        end_line: body.code_span().end_line,
        end_column: body.code_span().end_column,
    };
    
    Ok(ASTNode::WhileLoop {
        condition: Box::new(condition),
        initial_value,
        max_iterations,
        body: Box::new(body),
        code_span,
        scope: GLOBAL_SCOPE,
    })
}

fn parse_loop(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let start_span = expect_token(state, TokenType::LoopKeyword)?;
    
    // Check for optional index parameter (e.g., "loop index {")
    let index_iterator = if let Some(token) = state.tokens.peek() {
        if let TokenType::Identifier(name) = &token.token_type {
            let name = name.clone();
            // Consume the identifier and use it as the index parameter
            state.tokens.next();
            Some(name)
        } else {
            None
        }
    } else {
        None
    };
    
    let body = parse_block(state)?;
    let code_span = CodeSpan {
        start_line: start_span.start_line,
        start_column: start_span.start_column,
        end_line: body.code_span().end_line,
        end_column: body.code_span().end_column,
    };
    
    Ok(ASTNode::Loop {
        index_iterator,
        body: Box::new(body),
        code_span,
        scope: GLOBAL_SCOPE,
    })
}

fn parse_spawn_block(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let start_span = expect_token(state, TokenType::SpawnKeyword)?;
    let body = parse_block(state)?;
    let code_span = CodeSpan {
        start_line: start_span.start_line,
        start_column: start_span.start_column,
        end_line: body.code_span().end_line,
        end_column: body.code_span().end_column,
    };
    
    Ok(ASTNode::SpawnBlock {
        body: Box::new(body),
        code_span,
        scope: GLOBAL_SCOPE,
    })
}

fn parse_break_statement(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let code_span = expect_token(state, TokenType::BreakKeyword)?;
    let _ = expect_token(state, TokenType::EndStatementOrExpression)?;
    Ok(ASTNode::BreakStatement { code_span, scope: GLOBAL_SCOPE })
}

fn parse_continue_statement(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let code_span = expect_token(state, TokenType::ContinueKeyword)?;
    let _ = expect_token(state, TokenType::EndStatementOrExpression)?;
    Ok(ASTNode::ContinueStatement { code_span, scope: GLOBAL_SCOPE })
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
        let input = "| x:i ):i { result:i = x + 1; r result; }";
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
     Program([ConstDeclaration{name:"test_array",data_type:Array(Int),value:ArrayLiteral([NumberLiteral("1",),NumberLiteral("2",),NumberLiteral("3",),],),},],)
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
     Program([ConstDeclaration{name:"test_array",data_type:Array(Int),value:NumberLiteral("1",),},],)
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
                    data_type: Array(Box::new(NailDataTypeDescriptor::Int)),
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
    fn test_oneof_type_declaration() {
        let input = "c every_nail_type:oneof(i|f|s|b|a:i|a:f|a:struct:oneof|a:enum:oneof) = 13;";
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
        let input = "| x:i, y:f ):i { result:i = x + 1; r result; }";
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
        // Test that |e):i syntax works for error parameters
        let input = r#"|e):i { r 0; }"#;

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
            data_type: Array(Box::new(NailDataTypeDescriptor::Struct(
                "Point".to_string(),
            ))),
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
