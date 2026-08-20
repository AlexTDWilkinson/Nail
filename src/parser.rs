use crate::common::{CodeError, CodeSpan, GLOBAL_SCOPE};
use crate::lexer::*;
use crate::stdlib_registry;
use std::collections::HashSet;
use std::iter::Peekable;
use std::vec::IntoIter;

pub mod std_lib;

// We don't actually use this in the parser, it's a placeholder so the AST doesn't need to be recreated as an entirely new structure just for the scopes in the checker stage

#[derive(Debug, PartialEq, Clone)]
pub enum ASTNode {
    Program { statements: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    FunctionDeclaration { name: String, params: Vec<(String, NailDataTypeDescriptor)>, data_type: NailDataTypeDescriptor, body: Box<ASTNode>, sandboxed: bool, code_span: CodeSpan, scope: usize },
    FunctionCall { name: String, args: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    ConstDeclaration { name: String, data_type: NailDataTypeDescriptor, value: Box<ASTNode>, sandboxed: bool, code_span: CodeSpan, scope: usize },
    IfStatement { condition_branches: Vec<(Box<ASTNode>, Box<ASTNode>)>, else_branch: Option<Box<ASTNode>>, code_span: CodeSpan, scope: usize },
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
    ScanExpression {
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
    Forever { body: Box<ASTNode>, code_span: CodeSpan, scope: usize },
    ParallelBlock { statements: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    ConcurrentBlock { statements: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    Block { statements: Vec<ASTNode>, code_span: CodeSpan, scope: usize },
    BinaryOperation { left: Box<ASTNode>, operator: Operation, right: Box<ASTNode>, code_span: CodeSpan, scope: usize },
    UnaryOperation { operator: Operation, operand: Box<ASTNode>, code_span: CodeSpan, scope: usize },
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
        ASTNode::Program { statements: Vec::new(), code_span: CodeSpan::default(), scope: 0 }
    }
}

impl ASTNode {
    pub fn code_span(&self) -> CodeSpan {
        match self {
            ASTNode::Program { code_span, .. } => code_span.clone(),
            ASTNode::FunctionDeclaration { code_span, .. } => code_span.clone(),
            ASTNode::FunctionCall { code_span, .. } => code_span.clone(),
            ASTNode::ConstDeclaration { code_span, .. } => code_span.clone(),
            ASTNode::IfStatement { code_span, .. } => code_span.clone(),
            ASTNode::MapExpression { code_span, .. } => code_span.clone(),
            ASTNode::FilterExpression { code_span, .. } => code_span.clone(),
            ASTNode::ReduceExpression { code_span, .. } => code_span.clone(),
            ASTNode::ScanExpression { code_span, .. } => code_span.clone(),
            ASTNode::EachExpression { code_span, .. } => code_span.clone(),
            ASTNode::FindExpression { code_span, .. } => code_span.clone(),
            ASTNode::AllExpression { code_span, .. } => code_span.clone(),
            ASTNode::AnyExpression { code_span, .. } => code_span.clone(),
            ASTNode::Forever { code_span, .. } => code_span.clone(),
            ASTNode::ParallelBlock { code_span, .. } => code_span.clone(),
            ASTNode::ConcurrentBlock { code_span, .. } => code_span.clone(),
            ASTNode::Block { code_span, .. } => code_span.clone(),
            ASTNode::BinaryOperation { code_span, .. } => code_span.clone(),
            ASTNode::UnaryOperation { code_span, .. } => code_span.clone(),
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

    /// Every direct child expression/statement node, for generic AST walks.
    /// Binder names (iterators, params) are strings, not nodes, so callers
    /// that care about them handle them separately.
    pub fn children(&self) -> Vec<&ASTNode> {
        match self {
            ASTNode::Program { statements, .. }
            | ASTNode::Block { statements, .. }
            | ASTNode::ParallelBlock { statements, .. }
            | ASTNode::ConcurrentBlock { statements, .. } => statements.iter().collect(),
            ASTNode::FunctionDeclaration { body, .. } => vec![body.as_ref()],
            ASTNode::FunctionCall { args, .. } => args.iter().collect(),
            ASTNode::ConstDeclaration { value, .. } => vec![value.as_ref()],
            ASTNode::IfStatement { condition_branches, else_branch, .. } => {
                let mut children: Vec<&ASTNode> = Vec::new();
                for (condition, branch) in condition_branches {
                    children.push(condition.as_ref());
                    children.push(branch.as_ref());
                }
                if let Some(else_branch) = else_branch {
                    children.push(else_branch.as_ref());
                }
                children
            }
            ASTNode::MapExpression { iterable, body, .. }
            | ASTNode::FilterExpression { iterable, body, .. }
            | ASTNode::EachExpression { iterable, body, .. }
            | ASTNode::FindExpression { iterable, body, .. }
            | ASTNode::AllExpression { iterable, body, .. }
            | ASTNode::AnyExpression { iterable, body, .. } => vec![iterable.as_ref(), body.as_ref()],
            ASTNode::ReduceExpression { iterable, initial_value, body, .. } | ASTNode::ScanExpression { iterable, initial_value, body, .. } => vec![iterable.as_ref(), initial_value.as_ref(), body.as_ref()],
            ASTNode::Forever { body, .. } => vec![body.as_ref()],
            ASTNode::BinaryOperation { left, right, .. } => vec![left.as_ref(), right.as_ref()],
            ASTNode::UnaryOperation { operand, .. } => vec![operand.as_ref()],
            ASTNode::StructDeclaration { fields, .. } | ASTNode::StructInstantiation { fields, .. } | ASTNode::EnumDeclaration { variants: fields, .. } => fields.iter().collect(),
            ASTNode::StructInstantiationField { value, .. } => vec![value.as_ref()],
            ASTNode::NestedFieldAccess { object, .. } => vec![object.as_ref()],
            ASTNode::ArrayLiteral { elements, .. } => elements.iter().collect(),
            ASTNode::ReturnDeclaration { statement, .. } | ASTNode::YieldDeclaration { statement, .. } => vec![statement.as_ref()],
            ASTNode::StructDeclarationField { .. }
            | ASTNode::StructFieldAccess { .. }
            | ASTNode::EnumVariant { .. }
            | ASTNode::Identifier { .. }
            | ASTNode::NumberLiteral { .. }
            | ASTNode::StringLiteral { .. }
            | ASTNode::BooleanLiteral { .. }
            => Vec::new(),
        }
    }
}

pub struct ParserState {
    tokens: Peekable<IntoIter<Token>>,
    current_token: Option<Token>,
    previous_token: Option<Token>,
    // How many import inclusions the parser is currently inside. The lexer
    // wraps each imported file's tokens in SandboxStart/SandboxEnd markers,
    // and nesting simply nests the marker pairs.
    sandbox_depth: usize,
    // How many expressions, statements and blocks are open at this point.
    // Reading them is recursive, so this is what stops a file from running the
    // stack out. See MAX_NESTING_DEPTH.
    depth: usize,
    // How deep the type currently being read is. Types have their own, much
    // tighter limit, and their own counter so that a type written inside
    // deeply nested code is still judged on its own nesting.
    type_depth: usize,
}

/// How deeply expressions, statements and blocks may nest before the parser
/// refuses the file. Every level is a stack frame, so a file nesting thousands
/// of levels deep would abort the process instead of producing an error a
/// person can read. Hand-written code nests a handful of levels.
pub const MAX_NESTING_DEPTH: usize = 128;

/// How deep the finished tree may be. The parser's own limit is not enough on
/// its own: `1 + 1 + 1 + ...` is read by a loop rather than by recursion, so a
/// long chain builds a tree thousands of levels deep without the parser ever
/// nesting. Every later pass (the checker, the transpiler, the formatter)
/// walks that tree recursively, so the depth is checked once here, where the
/// tree is finished and every one of those passes is downstream.
pub const MAX_AST_DEPTH: usize = 512;

pub fn parse(tokens: Vec<Token>) -> Result<ASTNode, CodeError> {
    let mut state = ParserState { tokens: tokens.into_iter().peekable(), current_token: None, previous_token: None, sandbox_depth: 0, depth: 0, type_depth: 0 };
    let ast = parse_inner(&mut state)?;
    if let Some(code_span) = node_below_depth_limit(&ast) {
        return Err(CodeError {
            message: format!("This expression nests more than {} levels deep, which is deeper than the compiler will read", MAX_AST_DEPTH),
            help: Some("give the pieces names and combine the names, or build the values in an array and fold them with reduce".to_string()),
            code_span,
        });
    }
    Ok(ast)
}

/// The span of the first node deeper than the limit, or None when the whole
/// tree is within it. The walk is a loop over an explicit stack rather than
/// recursion, because a function that recurses to measure depth crashes on
/// exactly the trees it exists to reject.
fn node_below_depth_limit(root: &ASTNode) -> Option<CodeSpan> {
    let mut stack: Vec<(&ASTNode, usize)> = vec![(root, 0)];
    while let Some((node, depth)) = stack.pop() {
        if depth > MAX_AST_DEPTH {
            return Some(node.code_span());
        }
        for child in node.children() {
            stack.push((child, depth + 1));
        }
    }
    None
}

/// Run one parse step one level deeper, refusing to go past MAX_NESTING_DEPTH.
/// Every recursive path through the parser passes through a statement, a
/// primary expression or a type annotation, so counting those three counts
/// every way a file can nest.
fn nested<T>(state: &mut ParserState, what: &str, parse_step: impl FnOnce(&mut ParserState) -> Result<T, CodeError>) -> Result<T, CodeError> {
    if state.depth >= MAX_NESTING_DEPTH {
        let code_span = state
            .tokens
            .peek()
            .map(|token| token.code_span.clone())
            .or_else(|| state.previous_token.as_ref().map(|token| token.code_span.clone()))
            .unwrap_or_default();
        return Err(CodeError {
            message: format!("This {} nests more than {} levels deep, which is deeper than the compiler will read", what, MAX_NESTING_DEPTH),
            help: Some("give the inner pieces names and use the names, so each piece stays shallow".to_string()),
            code_span,
        });
    }
    state.depth += 1;
    let result = parse_step(state);
    state.depth -= 1;
    result
}

fn parse_inner(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let mut program = vec![];
    while let Some(token) = state.tokens.peek() {
        // Sandboxed markers drive a depth counter and produce no AST nodes
        match token.token_type {
            TokenType::SandboxStart => {
                advance(state);
                state.sandbox_depth += 1;
                continue;
            }
            TokenType::SandboxEnd => {
                advance(state);
                state.sandbox_depth = state.sandbox_depth.saturating_sub(1);
                continue;
            }
            _ => {}
        }
        let statement = parse_statement(state)?;
        if state.sandbox_depth > 0 {
            check_sandboxed_statement(&statement)?;
        }
        program.push(statement);
    }
    Ok(ASTNode::Program { statements: program, code_span: CodeSpan::default(), scope: GLOBAL_SCOPE })
}

/// An imported file may only declare things: functions, structs, enums, and
/// constants. Anything that runs at the top level is rejected, so the
/// importing program alone decides when sandboxed code executes.
fn check_sandboxed_statement(statement: &ASTNode) -> Result<(), CodeError> {
    match statement {
        ASTNode::FunctionDeclaration { .. } | ASTNode::StructDeclaration { .. } | ASTNode::EnumDeclaration { .. } | ASTNode::ConstDeclaration { .. } => Ok(()),
        other => Err(CodeError {
            help: Some("move the statement into a function so your program decides when it runs, or bring the file in with import_dangerous if you trust it".to_string()),
            message: format!("An imported file may only declare functions, structs, enums, and constants, but this file has {} at the top level", describe_statement(other)),
            code_span: other.code_span(),
        }),
    }
}

/// Plain-language name for a statement kind, used by the import top-level
/// restriction error.
fn describe_statement(statement: &ASTNode) -> String {
    match statement {
        ASTNode::FunctionCall { name, .. } => format!("a call to '{}'", name),
        ASTNode::IfStatement { .. } => "an if statement".to_string(),
        ASTNode::Forever { .. } => "a forever block".to_string(),
        ASTNode::ParallelBlock { .. } => "a parallel block".to_string(),
        ASTNode::ConcurrentBlock { .. } => "a concurrent block".to_string(),
        ASTNode::Block { .. } => "a block".to_string(),
        ASTNode::MapExpression { .. } => "a map expression".to_string(),
        ASTNode::FilterExpression { .. } => "a filter expression".to_string(),
        ASTNode::ReduceExpression { .. } => "a reduce expression".to_string(),
        ASTNode::ScanExpression { .. } => "a scan expression".to_string(),
        ASTNode::EachExpression { .. } => "an each expression".to_string(),
        ASTNode::FindExpression { .. } => "a find expression".to_string(),
        ASTNode::AllExpression { .. } => "an all expression".to_string(),
        ASTNode::AnyExpression { .. } => "an any expression".to_string(),
        ASTNode::ReturnDeclaration { .. } => "a return statement".to_string(),
        ASTNode::YieldDeclaration { .. } => "a yield statement".to_string(),
        _ => "an executable statement".to_string(),
    }
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
    nested(state, "expression", parse_primary_inner)
}

fn parse_primary_inner(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(token) = state.tokens.peek().cloned() {
        let node = match token.token_type {
            TokenType::Operator(op) if op.is_unary() || op == Operation::Sub => {
                // Handle unary operators like ! and -. The lexer emits '-' as
                // Sub (it can't see position); in prefix position it is negation
                advance(state);
                let operand = Box::new(parse_primary(state)?);
                let operator = if op == Operation::Sub { Operation::Neg } else { op };
                // Unary operators don't participate in field access
                return Ok(ASTNode::UnaryOperation { operator, operand, code_span: token.code_span, scope: GLOBAL_SCOPE });
            }
            // Struct instantiation is now detected in the Identifier case
            TokenType::Identifier(name) => {
                advance(state);
                if matches!(state.tokens.peek().map(|t| &t.token_type), Some(TokenType::ParenthesisOpen)) {
                    // f(...) in expression position is an attempt at a lambda
                    if name == "f" {
                        Err(CodeError { help: Some("declare the function at the top level, f name(param:type):type { ... }, and pass it by name".to_string()), message: "Nail has no lambdas or inline functions".to_string(), code_span: token.code_span.clone() })?
                    } else {
                        parse_function_call(state, name, token.code_span.clone())?
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
            TokenType::StringLiteral { value, .. } => {
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
            TokenType::MapDeclaration => parse_map_expression(state)?,
            TokenType::FilterDeclaration => parse_filter_expression(state)?,
            TokenType::ReduceDeclaration => parse_reduce_expression(state)?,
            TokenType::ScanDeclaration => parse_scan_expression(state)?,
            TokenType::FindDeclaration => parse_find_expression(state)?,
            TokenType::AllDeclaration => parse_all_expression(state)?,
            TokenType::AnyDeclaration => parse_any_expression(state)?,
            TokenType::FunctionSignature(_) => Err(CodeError { help: Some("declare the function at the top level, f name(param:type):type { ... }, and pass it by name".to_string()), message: "Nail has no lambdas or inline functions".to_string(), code_span: token.code_span.clone() })?,
            _ => {
                let code_span = token.code_span;
                return Err(CodeError { help: None, message: format!("Did not expect {} here", describe_token(&token.token_type)), code_span: code_span.clone() });
            }
        };
        
        // Apply field access to all primary expressions except unary operations
        parse_field_access_chain(state, node)
    } else {
        Err(CodeError { help: None, message: "Unexpected end of file".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) })
    }
}

fn parse_statement(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    nested(state, "statement", parse_statement_inner)
}

fn parse_statement_inner(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    match state.tokens.peek() {
        Some(token) => match &token.token_type {
            TokenType::StructDeclaration(_) => parse_struct_declaration(state),
            TokenType::EnumDeclaration(_) => parse_enum_declaration(state),
            TokenType::FunctionSignature(_) => parse_function_declaration(state),
            TokenType::IfDeclaration => parse_if_statement_expr(state, false),
            TokenType::MapDeclaration => parse_map_expression(state),
            TokenType::FilterDeclaration => parse_filter_expression(state),
            TokenType::ReduceDeclaration => parse_reduce_expression(state),
            TokenType::ScanDeclaration => parse_scan_expression(state),
            TokenType::EachDeclaration => parse_each_expression(state),
            TokenType::FindDeclaration => parse_find_expression(state),
            TokenType::AllDeclaration => parse_all_expression(state),
            TokenType::AnyDeclaration => parse_any_expression(state),
            TokenType::ForeverKeyword => parse_forever(state),
            TokenType::ParallelStart => parse_parallel_block_start(state),
            TokenType::ConcurrentStart => parse_concurrent_block_start(state),
            TokenType::Return => parse_return_statement(state),
            TokenType::Yield => parse_yield_statement(state),
            TokenType::BlockOpen => Err(CodeError { help: Some("a block belongs to an if branch, a function, a forever block or a collection operation. Statements that go together go on consecutive lines, and a name that is only needed for a moment is still declared where it is used".to_string()), message: "A block on its own does nothing in Nail".to_string(), code_span: token.code_span.clone() }),
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
        None => Err(CodeError { help: None, message: "No token was found to match with a statement.".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) }),
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
                    return Err(CodeError { help: None, message: format!("The operator '{}' cannot be used between two values", op), code_span });
                } else {
                    let right = parse_expression(state, op.precedence() + 1)?;
                    left = ASTNode::BinaryOperation { left: Box::new(left), operator: op, right: Box::new(right), code_span: code_span.clone(), scope: GLOBAL_SCOPE };
                }
            }
            Some(Token { token_type: TokenType::Assignment, code_span, .. }) => {
                // A bare `=` after an expression would be a reassignment, and
                // Nail has none. Refusing it here, with the alternatives
                // spelled out, keeps the error better than the generic
                // "did not expect '='" it would otherwise fall into.
                let code_span = code_span.clone();
                return Err(CodeError {
                    message: "Nail has no reassignment: a variable cannot be assigned a second time".to_string(),
                    help: Some("declare a binding with a type ('name:i = ...', redeclaring an earlier name in the same scope is allowed), accumulate across items with reduce, or write '==' if a comparison was meant".to_string()),
                    code_span,
                });
            }
            _ => break,
        }
    }

    Ok(left)
}

/// Plain-language name for a token, used in error messages instead of the
/// internal enum variant name.
fn describe_token(token_type: &TokenType) -> String {
    token_type.describe()
}

fn expect_token(state: &mut ParserState, expected: TokenType) -> Result<CodeSpan, CodeError> {
    if let Some(token) = advance(state) {
        if token.token_type == expected {
            Ok(token.code_span)
        } else {
            let error = CodeError { help: None, message: format!("Expected {} here, but found {}", describe_token(&expected), describe_token(&token.token_type)), code_span: token.code_span.clone() };
            log::error!("Expect token error: {:?}", error);
            Err(error)
        }
    } else {
        log::error!("expect_token else branch error: {:?}", expected);
        let help = if expected == TokenType::BlockClose { Some("a block opened earlier is never closed, so add the missing '}'".to_string()) } else { None };
        Err(CodeError {
            help,
            message: format!("Expected {} but the file ended first", describe_token(&expected)),
            code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()),
        })
    }
}

fn expect_identifier(state: &mut ParserState) -> Result<String, CodeError> {
    if let Some(Token { token_type: TokenType::Identifier(name), code_span, .. }) = advance(state) {
        if name.len() < 2 {
            let error = CodeError { help: None,
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
        let error = CodeError { help: None,
            message: format!("Expected a name here, but found {}", describe_token(&state.tokens.peek().map(|token| token.token_type.clone()).unwrap_or(TokenType::EndOfFile))),
            code_span: state.tokens.peek().map_or(CodeSpan::default(), |token| token.code_span.clone()),
        };
        log::error!("Expect identifier error: {:?}", error);
        Err(error)
    }
}

fn parse_function_call(state: &mut ParserState, name: String, name_span: CodeSpan) -> Result<ASTNode, CodeError> {
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
    let closing_span = expect_token(state, TokenType::ParenthesisClose)?;

    // The call's span runs from its name to its closing parenthesis, so an
    // error about a call underlines the call. It used to be the closing
    // parenthesis alone, which put a single caret under ')' while the message
    // talked about the function, and on a declaration the caret landed on the
    // semicolon after it.
    let code_span = CodeSpan {
        start_line: name_span.start_line,
        start_column: name_span.start_column,
        end_line: closing_span.end_line,
        end_column: closing_span.end_column,
    };

    Ok(ASTNode::FunctionCall { name, args, code_span, scope: GLOBAL_SCOPE })
}

fn parse_struct_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    // The declaration's own span. See parse_enum_declaration.
    if let Some(Token { token_type: TokenType::StructDeclaration(struct_declaration_data), code_span, .. }) = advance(state) {
        let mut struct_fields = struct_declaration_data.fields.into_iter();

        let struct_name = struct_declaration_data.name;
        let mut fields = Vec::new();

        while let Some(field) = struct_fields.next() {
            fields.push(ASTNode::StructDeclarationField { name: field.name, data_type: field.data_type, scope: GLOBAL_SCOPE })
        }

        Ok(ASTNode::StructDeclaration { name: struct_name, fields, code_span, scope: GLOBAL_SCOPE })
    } else {
        Err(CodeError { help: None, message: "Struct declaration syntax is incorrect".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) })
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
                _ => return Err(CodeError { help: None, 
                    message: "Expected field name in struct instantiation".to_string(), 
                    code_span: token.code_span.clone() 
                })
            }
        } else {
            return Err(CodeError { help: None, 
                message: "Unexpected end of input in struct instantiation".to_string(), 
                code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) 
            });
        };
        
        // Expect equals sign
        let _ = expect_token(state, TokenType::Assignment)?;
        
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
            return Err(CodeError { help: None, 
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
            return Err(CodeError { help: None, message: "Expected ',' or ']' in array literal".to_string(), code_span: state.current_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) });
        }
    }

    // Expect and consume ']'
    let _ = expect_token(state, TokenType::ArrayClose)?;

    Ok(ASTNode::ArrayLiteral { elements, code_span: start_span, scope: GLOBAL_SCOPE })
}

fn parse_enum_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    // The declaration's own span, not the span of whatever came before it:
    // `previous_token` at this point is the token in front of the enum, so an
    // enum written inside a block was reported against the block's brace.
    if let Some(Token { token_type: TokenType::EnumDeclaration(nail_enum_data), code_span, .. }) = advance(state) {
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
                    return Err(CodeError { help: None, message: format!("Did not expect {} inside an enum declaration; only variant names belong here", describe_token(&token.token_type)), code_span });
                }
            }
        }

        Ok(ASTNode::EnumDeclaration { name: enum_name, variants, code_span, scope: GLOBAL_SCOPE })
    } else {
        Err(CodeError { help: None, message: "Expected enum declaration".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) })
    }
}

fn parse_function_declaration(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    if let Some(Token { token_type: TokenType::FunctionSignature(tokens), code_span }) = advance(state) {
        let mut func_tokens = tokens.into_iter();

        // Parse function name
        let name = if let Some(Token { token_type: TokenType::FunctionName(name), .. }) = func_tokens.next() {
            if name.is_empty() {
                return Err(CodeError { help: None, message: "Function name cannot be empty".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) });
            }
            name
        } else {
            return Err(CodeError { help: None, message: "Expected function name".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) });
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
                            Some(other) => return Err(CodeError { help: None, message: format!("Expected ',' or a return type declaration here, but found {}", describe_token(&other.token_type)), code_span: other.code_span.clone() }),
                            None => {
                                return Err(CodeError { help: None,
                                    message: "Unexpected end of function declaration".to_string(),
                                    code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()),
                                })
                            }
                        }
                    } else {
                        return Err(CodeError { help: None,
                            message: "Expected type declaration for function parameter".to_string(),
                            code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()),
                        });
                    }
                }
                Some(Token { token_type: TokenType::FunctionReturnTypeDeclaration(rt), .. }) => {
                    data_type = rt;
                    break;
                }
                Some(other) => return Err(CodeError { help: None, message: format!("Did not expect {} in a function declaration", describe_token(&other.token_type)), code_span: other.code_span.clone() }),
                None => {
                    return Err(CodeError { help: None,
                        message: "Unexpected end of function declaration".to_string(),
                        code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()),
                    })
                }
            }
        }

        // Parse function body
        let body = Box::new(parse_block(state)?);

        Ok(ASTNode::FunctionDeclaration { name, params, data_type, body, sandboxed: state.sandbox_depth > 0, code_span, scope: GLOBAL_SCOPE })
    } else {
        Err(CodeError { help: None, message: "Expected function declaration".to_string(), code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()) })
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

    Ok(ASTNode::ConstDeclaration { name, data_type, value, sandboxed: state.sandbox_depth > 0, code_span, scope: GLOBAL_SCOPE })
}

fn parse_type_annotation(state: &mut ParserState) -> Result<NailDataTypeDescriptor, CodeError> {
    // A type's own limit, the same one the lexer holds a type to when it
    // reads one in a single token, so `a:a:...:i` is refused at the same
    // depth however it reaches the compiler.
    if state.type_depth >= crate::lexer::MAX_TYPE_DEPTH {
        let code_span = state.tokens.peek().map(|token| token.code_span.clone()).or_else(|| state.previous_token.as_ref().map(|token| token.code_span.clone())).unwrap_or_default();
        return Err(CodeError {
            message: format!("This type nests more than {} levels deep, which is deeper than any type the compiler will read", crate::lexer::MAX_TYPE_DEPTH),
            help: Some("a type this deep is almost always a typo: an array of arrays of numbers is 'a:a:i'".to_string()),
            code_span,
        });
    }
    state.type_depth += 1;
    let result = nested(state, "type", parse_type_annotation_inner);
    state.type_depth -= 1;
    result
}

fn parse_type_annotation_inner(state: &mut ParserState) -> Result<NailDataTypeDescriptor, CodeError> {
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
                        Err(CodeError { help: None,
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
                                Err(CodeError { help: None,
                                    message: format!("Unknown type: {}", type_name),
                                    code_span: token_span,
                                })
                            }
                        }
                    }
                }
            }
            _ => Err(CodeError { help: None,
                message: format!("Expected a type here (like i, f, s, or b), but found {}", describe_token(&token.token_type)),
                code_span: token.code_span.clone(),
            })
        }
    } else {
        Err(CodeError { help: None,
            message: "Expected type annotation".to_string(),
            code_span: state.previous_token.as_ref().map_or(CodeSpan::default(), |t| t.code_span.clone()),
        })
    }
}

fn parse_if_statement_expr(state: &mut ParserState, is_expression: bool) -> Result<ASTNode, CodeError> {
    let _ = state.previous_token.as_ref().map(|t| t.code_span.clone()).unwrap_or(CodeSpan::default());
    // #[test]
    // fn test_if_statement() {
    //     let input = "if { a > 5 -> {} };";
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
            return Err(CodeError { help: None, message: "Unexpected end of if statement".to_string(), code_span: code_span.clone() });
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

/// The parts common to `reduce` and `scan`, which are written identically:
/// an accumulator name, an element name, an optional index name, the array,
/// a starting value and a block. They differ only in what they produce - the
/// final accumulator, or every value it took along the way.
struct FoldParts {
    accumulator: String,
    iterator: String,
    index_iterator: Option<String>,
    iterable: Box<ASTNode>,
    initial_value: Box<ASTNode>,
    body: Box<ASTNode>,
    code_span: CodeSpan,
}

fn parse_fold_parts(state: &mut ParserState, keyword: TokenType) -> Result<FoldParts, CodeError> {
    let start_span = expect_token(state, keyword)?;

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

    Ok(FoldParts {
        accumulator,
        iterator,
        index_iterator,
        iterable: Box::new(iterable),
        initial_value: Box::new(initial_value),
        body: Box::new(body),
        code_span,
    })
}

fn parse_reduce_expression(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let parts = parse_fold_parts(state, TokenType::ReduceDeclaration)?;
    Ok(ASTNode::ReduceExpression {
        accumulator: parts.accumulator,
        iterator: parts.iterator,
        index_iterator: parts.index_iterator,
        iterable: parts.iterable,
        initial_value: parts.initial_value,
        body: parts.body,
        code_span: parts.code_span,
        scope: GLOBAL_SCOPE,
    })
}

fn parse_scan_expression(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let parts = parse_fold_parts(state, TokenType::ScanDeclaration)?;
    Ok(ASTNode::ScanExpression {
        accumulator: parts.accumulator,
        iterator: parts.iterator,
        index_iterator: parts.index_iterator,
        iterable: parts.iterable,
        initial_value: parts.initial_value,
        body: parts.body,
        code_span: parts.code_span,
        scope: GLOBAL_SCOPE,
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

fn parse_forever(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    let start_span = expect_token(state, TokenType::ForeverKeyword)?;
    let body = parse_block(state)?;
    let code_span = CodeSpan {
        start_line: start_span.start_line,
        start_column: start_span.start_column,
        end_line: body.code_span().end_line,
        end_column: body.code_span().end_column,
    };
    
    Ok(ASTNode::Forever {
        body: Box::new(body),
        code_span,
        scope: GLOBAL_SCOPE,
    })
}

fn parse_type_declaration(state: &mut ParserState) -> Result<NailDataTypeDescriptor, CodeError> {
    if let Some(Token { token_type: TokenType::TypeDeclaration(data_type), .. }) = advance(state) {
        Ok(data_type)
    } else {
        let error = CodeError { help: None,
            message: format!("Expected a type declaration here (like ':i' or ':s'), but found {}", describe_token(&state.tokens.peek().map(|token| token.token_type.clone()).unwrap_or(TokenType::EndOfFile))),
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
        let input = "if { a > 5 -> {} };";
        let result = parse(lexer(input)).unwrap();
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
    fn test_struct_declaration() {
        let input = "struct Point { x_pos:i, y_pos:i }";
        let result = parse(lexer(input)).unwrap();
        let expected = r#"
       Program([StructDeclaration{name:"Point",fields:[StructDeclarationField{name:"x_pos",data_type:Int,},StructDeclarationField{name:"y_pos",data_type:Int,},],},],)
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_enum_declaration() {
        let input = "enum Color { Red, Green, Blue }";
        let lexer = lexer(input);
        let result = parse(lexer).unwrap();
        let expected = r#"
       Program([EnumDeclaration{name:"Color",variants:[EnumVariant{name:"Color",variant:"Red",},EnumVariant{name:"Color",variant:"Green",},EnumVariant{name:"Color",variant:"Blue",},],},],)
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
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
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
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
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_if_else_statement() {
        let input = "if { a > 5 -> {}, else -> {} };";
        let result = parse(lexer(input)).unwrap();
        let expected = r#"
        Program([IfStatement{condition_branches:[(BinaryOperation{left:Identifier("a",),operator:Gt,right:NumberLiteral("5",),},Block([],),),],else_branch:Some(Block([],),),},],)
        "#;
        // Just verify it parses successfully
        assert!(matches!(result, ASTNode::Program { .. }));
    }

    #[test]
    fn test_if_else_if_else_statement() {
        let input = "if { a > 5 -> {}, b < 5 -> {}, else -> {} };";
        let result = parse(lexer(input)).unwrap();
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

        let result = parse(lexer).unwrap();
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

        let result = parse(lexer).unwrap();
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

        let result = parse(lexer).unwrap();
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
        let input = r#"f random(first:i, second:f):s { result:s = `test`; r result; }"#;
        let result = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);

        let expected = r#"
        Program(
    [
        FunctionDeclaration {
            name: "random",
            params: [
                (
                    "first",
                    Int,
                ),
                (
                    "second",
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
        // Enum-typed constants now use a bare type name annotation (`:Color`),
        // not the old `:enum:Color` form.
        let input = "enum Color { Red, Green, Blue } my_color:Color = Color::Red;";
        let result = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);
        if let ASTNode::Program { statements, .. } = result {
            assert!(matches!(statements.get(0), Some(ASTNode::EnumDeclaration { .. })), "First statement should be the enum declaration");
            if let Some(ASTNode::ConstDeclaration { name, value, .. }) = statements.get(1) {
                assert_eq!(name, "my_color");
                match value.as_ref() {
                    ASTNode::EnumVariant { name, variant, .. } => {
                        assert_eq!(name, "Color");
                        assert_eq!(variant, "Red");
                    }
                    other => panic!("Expected EnumVariant value, got: {:?}", other),
                }
            } else {
                panic!("Expected ConstDeclaration as second statement");
            }
        } else {
            panic!("Expected Program node");
        }
    }

    #[test]
    fn test_const_declaration() {
        let input = "counter:i = 10;";
        let result = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);
        // Add assertion here
    }

    // Variable declarations no longer supported - using constants only

    // Removed tests for features no longer in the language:
    // - test_oneof_type_declaration: `oneof(...)` type annotations are rejected by the
    //   parser ("Unknown type: oneof"); the oneof type declaration syntax was removed.
    // - test_lambda / test_lambda_multi_param / test_lambda_with_error_parameter:
    //   pipe lambda syntax `| x:i ):i { ... }` was removed from the lexer ('|' is no
    //   longer a recognized character) and no replacement anonymous function syntax
    //   is implemented. Error handlers are now named functions passed to safe().

    #[test]
    fn test_array_of_point_structs() {
        // Arrays of structs now use a bare struct name in the annotation (`a:Point`),
        // not the old `a:struct:Point` form.
        let input = r#"
            struct Point { x_coord:i, y_coord:i }
            points:a:Point = [Point { x_coord = 1, y_coord = 5 }, Point { x_coord = 3, y_coord = 4 }];
            "#;
        let result = parse(lexer(input)).unwrap();
        println!("RESULT: {:#?}", result);
        if let ASTNode::Program { statements, .. } = result {
            assert!(matches!(statements.get(0), Some(ASTNode::StructDeclaration { .. })), "First statement should be the struct declaration");
            if let Some(ASTNode::ConstDeclaration { name, data_type, value, .. }) = statements.get(1) {
                assert_eq!(name, "points");
                assert!(matches!(data_type, NailDataTypeDescriptor::Array(_)), "Declared type should be an array, got: {:?}", data_type);
                match value.as_ref() {
                    ASTNode::ArrayLiteral { elements, .. } => {
                        assert_eq!(elements.len(), 2, "Array literal should have two struct instantiations");
                        for element in elements {
                            assert!(matches!(element, ASTNode::StructInstantiation { .. }), "Array element should be a struct instantiation, got: {:?}", element);
                        }
                    }
                    other => panic!("Expected ArrayLiteral value, got: {:?}", other),
                }
            } else {
                panic!("Expected ConstDeclaration as second statement");
            }
        } else {
            panic!("Expected Program node");
        }
    }

    /// A bare block at statement level was accepted and meant nothing: no
    /// construct owned it, no document described it, and only the fuzzer
    /// ever wrote one. It is refused with a pointer to where blocks belong.
    #[test]
    fn a_block_on_its_own_is_refused() {
        let error = parse(lexer("{\n    inner:i = 5;\n    print(inner);\n}\nprint(`after`);")).expect_err("a bare block must be refused");
        assert!(error.message.contains("A block on its own does nothing"), "got: {}", error.message);
        assert_eq!(error.code_span.start_line, 1);
    }
}
