use crate::common::{CodeError, CodeSpan};
use crate::lexer::{NailDataTypeDescriptor, Operation};
use crate::parser::ASTNode;
use crate::stdlib_types;
use std::collections::{HashMap, HashSet};

pub const ERROR_SCOPE: usize = usize::MAX;
pub const GLOBAL_SCOPE: usize = 0;


#[derive(Debug, Clone, PartialEq)]
pub struct Scope {
    pub symbols: HashMap<String, Symbol>,
    pub parent: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub data_type: NailDataTypeDescriptor,
    pub is_used: bool,
}

#[derive(Debug)]
pub struct ScopeArena {
    scopes: Vec<Scope>,
}

impl ScopeArena {
    pub fn new() -> Self {
        // I guess this is where we'd set up global scope?
        // Note that global scope has no parent, so it's just set to ERROR_SCOPE to cause an error if it's parent is accessed for some reason
        ScopeArena { scopes: vec![Scope { symbols: HashMap::new(), parent: ERROR_SCOPE }] }
    }

    pub fn push_scope(&mut self) -> usize {
        let parent = self.current_scope();
        self.scopes.push(Scope { symbols: HashMap::new(), parent });
        self.scopes.len() - 1
    }

    pub fn pop_scope(&mut self) -> Result<(), &'static str> {
        if self.scopes.len() == 1 {
            return Err("Cannot pop global scope");
        }
        self.scopes.pop();
        Ok(())
    }

    pub fn current_scope(&self) -> usize {
        // log::info!("Current scope: {}", self.scopes.len().checked_sub(1).unwrap_or(ERROR_SCOPE));
        self.scopes.len().checked_sub(1).unwrap_or(ERROR_SCOPE)
    }

    pub fn get_scope(&self, index: usize) -> &Scope {
        if index == ERROR_SCOPE {
            panic!("Attempted to access error scope");
        }
        self.scopes.get(index).expect("Failed to get scope. THIS SHOULD NEVER HAPPEN.")
    }

    pub fn get_scope_mut(&mut self, index: usize) -> &mut Scope {
        if index == ERROR_SCOPE {
            panic!("Attempted to access error scope");
        }
        self.scopes.get_mut(index).expect("Failed to get mutable scope. THIS SHOULD NEVER HAPPEN.")
    }

    pub fn clear_above_scope(&mut self, index: usize) {
        self.scopes.truncate(index);
    }
}

struct AnalyzerState {
    scope_arena: ScopeArena,
    errors: Vec<CodeError>,
    in_function: bool,
    enum_variants: HashMap<String, HashSet<String>>,
    structs: HashMap<String, Vec<ASTNode>>,
}

fn new_analyzer_state() -> AnalyzerState {
    AnalyzerState { scope_arena: ScopeArena::new(), errors: Vec::new(), in_function: false, enum_variants: HashMap::new(), structs: HashMap::new() }
}

pub fn checker(ast: &mut ASTNode) -> Result<(), Vec<CodeError>> {
    let mut state = new_analyzer_state();

    if let ASTNode::Program { scope, .. } = ast {
        *scope = GLOBAL_SCOPE;
    }

    visit_node(ast, &mut state);
    check_unused_symbols(&mut state);
    if state.errors.is_empty() {
        Ok(())
    } else {
        Err(state.errors)
    }
}

fn visit_node(node: &mut ASTNode, state: &mut AnalyzerState) {
    let current_scope = state.scope_arena.current_scope();

    // Always set the scope for every node
    match node {
        ASTNode::Program { scope, .. }
        | ASTNode::ConstDeclaration { scope, .. }
        | ASTNode::StringLiteral { scope, .. }
        | ASTNode::NumberLiteral { scope, .. }
        | ASTNode::UnaryOperation { scope, .. }
        | ASTNode::BinaryOperation { scope, .. }
        | ASTNode::Identifier { scope, .. }
        | ASTNode::IfStatement { scope, .. }
        | ASTNode::StructDeclaration { scope, .. }
        | ASTNode::EnumDeclaration { scope, .. }
        | ASTNode::ArrayLiteral { scope, .. }
        | ASTNode::FunctionCall { scope, .. }
        | ASTNode::ReturnDeclaration { scope, .. }
        | ASTNode::Block { scope, .. }
        | ASTNode::ParallelBlock { scope, .. }
        | ASTNode::ParallelAssignment { scope, .. }
        | ASTNode::LambdaDeclaration { scope, .. }
        | ASTNode::FunctionDeclaration { scope, .. }
        | ASTNode::StructInstantiation { scope, .. }
        | ASTNode::StructInstantiationField { scope, .. }
        | ASTNode::StructDeclarationField { scope, .. }
        | ASTNode::EnumVariant { scope, .. } => {
            *scope = current_scope;
        }
    }

    match node {
        ASTNode::Program { statements, code_span, .. } => statements.iter_mut().for_each(|statement| visit_node(statement, state)),
        ASTNode::FunctionDeclaration { name, params, data_type, body, code_span, .. } => visit_function_declaration(name, params, data_type, body, state, code_span),
        ASTNode::ConstDeclaration { name, data_type, value, code_span, .. } => visit_const_declaration(name, data_type, value, state, code_span),
        ASTNode::BinaryOperation { left, operator, right, code_span, .. } => visit_binary_operation(left, operator, right, state, code_span),
        ASTNode::Identifier { name, code_span, .. } => {
            if !mark_symbol_as_used(state, name) {
                add_error(state, format!("Undefined variable: {}", name), code_span);
            }
        }
        ASTNode::IfStatement { condition_branches, else_branch, code_span, .. } => visit_if_statement(condition_branches, else_branch, state),
        ASTNode::StructDeclaration { name, fields, code_span, .. } => visit_struct_declaration(name, fields, state, code_span),
        ASTNode::EnumDeclaration { name, variants, code_span, .. } => visit_enum_declaration(name, variants, state, code_span),
        ASTNode::ArrayLiteral { elements, code_span, .. } => visit_array_literal(elements, state, code_span),
        ASTNode::FunctionCall { name, args, code_span, scope } => {
            visit_function_call(name, args, state, *scope, code_span);
        }
        ASTNode::ReturnDeclaration { statement, code_span, .. } => visit_return_declaration(statement, state, code_span),
        ASTNode::ParallelBlock { statements, .. } => statements.iter_mut().for_each(|statement| visit_node(statement, state)),
        ASTNode::ParallelAssignment { assignments, .. } => visit_parallel_assignment(assignments, state),
        ASTNode::Block { statements, .. } => statements.iter_mut().for_each(|statement| visit_node(statement, state)),
        _ => {} // Handle other cases as needed
    }
}

fn visit_function_declaration(
    name: &str,
    params: &[(String, NailDataTypeDescriptor)],
    return_type: &NailDataTypeDescriptor,
    body: &mut Box<ASTNode>,
    state: &mut AnalyzerState,
    code_span: &mut CodeSpan,
) {
    // Create the function type
    let param_types: Vec<NailDataTypeDescriptor> = params.iter().map(|(_, t)| t.clone()).collect();
    let function_type = NailDataTypeDescriptor::Fn(param_types, Box::new(return_type.clone()));

    // Add the function to the current scope (parent scope)
    add_symbol(state, Symbol { name: name.to_string(), data_type: function_type, is_used: false });

    // Now push a new scope for the function body
    let function_scope = state.scope_arena.push_scope();

    // Update the function declaration's scope
    if let ASTNode::FunctionDeclaration { scope, .. } = body.as_mut() {
        *scope = function_scope;
    }

    state.in_function = true;

    // Add parameters to the function's scope
    params.iter().for_each(|(param_name, param_type)| {
        add_symbol(state, Symbol { name: param_name.clone(), data_type: param_type.clone(), is_used: false });
    });

    // Visit the function body
    visit_node(body, state);

    // Check function return
    check_function_return(name, return_type, body, state, code_span);

    state.in_function = false;

    state.scope_arena.pop_scope().expect("Failed to pop function scope");
}

fn visit_const_declaration(name: &str, data_type: &NailDataTypeDescriptor, value: &mut ASTNode, state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    let value_type = check_type(value, state);

    // Check for type compatibility, allowing struct/enum name mismatches
    let types_compatible = match (data_type, &value_type) {
        // Exact match
        (a, b) if a == b => true,
        // Allow struct annotation with enum value (or vice versa) if they have the same name
        (NailDataTypeDescriptor::Struct(struct_name), NailDataTypeDescriptor::Enum(enum_name)) => struct_name == enum_name,
        (NailDataTypeDescriptor::Enum(enum_name), NailDataTypeDescriptor::Struct(struct_name)) => enum_name == struct_name,
        _ => false,
    };

    if !types_compatible {
        add_error(state, format!("Type mismatch in constant declaration named `{}`: expected {:?}, got {:?}", name, data_type, value_type), code_span);
    }

    // Use the actual value type for the symbol, not the annotation type
    let symbol_type = if types_compatible && data_type != &value_type {
        value_type.clone()
    } else {
        data_type.clone()
    };

    add_symbol(state, Symbol { name: name.to_string(), data_type: symbol_type, is_used: false });

    visit_node(value, state);
}

fn visit_binary_operation(left: &ASTNode, operator: &Operation, right: &ASTNode, state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    let left_type = check_type(left, state);
    let right_type = check_type(right, state);

    // If either operand is a function call, we need to use its return type
    let left_type = if let ASTNode::FunctionCall { name, args, scope, .. } = left { visit_function_call(name, args, state, *scope, code_span) } else { left_type };

    let right_type = if let ASTNode::FunctionCall { name, args, scope, .. } = right { visit_function_call(name, args, state, *scope, code_span) } else { right_type };

    if left_type != right_type {
        add_error(state, format!("Type mismatch in binary operation: left operand is {:?}, right operand is {:?}", left_type, right_type), code_span);
    } else {
        // Determine the result type based on the operator and operand types
        match operator {
            Operation::Add | Operation::Sub | Operation::Mul | Operation::Div => {
                if left_type != NailDataTypeDescriptor::Int && left_type != NailDataTypeDescriptor::Float {
                    add_error(state, format!("Invalid operand type for arithmetic operation: {:?}", left_type), code_span);
                }
            }
            Operation::Eq | Operation::Ne | Operation::Lt | Operation::Lte | Operation::Gt | Operation::Gte => {
                // Allow equality comparisons for all types
                if matches!(operator, Operation::Eq | Operation::Ne) {
                    // Equality is allowed for all types including enums, structs, etc.
                } else {
                    // Ordering comparisons (Lt, Lte, Gt, Gte) only allowed for Int, Float, String
                    if left_type != NailDataTypeDescriptor::Int && left_type != NailDataTypeDescriptor::Float && left_type != NailDataTypeDescriptor::String {
                        add_error(state, format!("Invalid operand type for ordering comparison operation: {:?}", left_type), code_span);
                    }
                }
            }
            _ => {
                add_error(state, format!("Unsupported operator: {:?}", operator), code_span);
            }
        }
    }
}

fn visit_if_statement(condition_branches: &mut [(Box<ASTNode>, Box<ASTNode>)], else_branch: &mut Option<Box<ASTNode>>, state: &mut AnalyzerState) {
    for (condition, branch) in condition_branches.iter_mut() {
        visit_node(condition, state);
        visit_node(branch, state);
    }
    if let Some(branch) = else_branch {
        visit_node(branch, state);
    }
}

fn visit_struct_declaration(name: &str, fields: &[ASTNode], state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    // Register the struct in the type system
    if let Some(existing) = state.structs.get(name) {
        if existing != fields {
            add_error(state, format!("Struct '{}' is already defined", name), code_span);
        }
    } else {
        state.structs.insert(name.to_string(), fields.to_vec());
    }
    
    // Nested structs are now allowed - just validate they exist
    for field in fields {
        if let ASTNode::StructDeclarationField { name: field_name, data_type, .. } = field {
            match data_type {
                NailDataTypeDescriptor::Struct(struct_name) => {
                    // Allow forward references and self-references for now
                    // More sophisticated checking could be added later
                }
                NailDataTypeDescriptor::Enum(enum_name) => {
                    // Allow forward references for now
                }
                _ => {}
            }
        }
    }
}

fn visit_enum_declaration(name: &str, variants: &[ASTNode], state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    let mut variant_set = HashSet::new();
    variants.iter().for_each(|variant| {
        if let ASTNode::EnumVariant { variant: variant_name, code_span, .. } = variant {
            if !variant_set.insert(variant_name.clone()) {
                add_error(state, format!("Duplicate variant '{}' in enum '{}'", variant_name, name), &mut code_span.clone());
            }
        }
    });
    state.enum_variants.insert(name.to_string(), variant_set);
}

fn visit_array_literal(elements: &[ASTNode], state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    if elements.is_empty() {
        add_error(state, "Empty array literals are not allowed".to_string(), code_span);
        return;
    }

    let first_type = check_type(&elements[0], state);
    elements.iter().skip(1).for_each(|element| {
        let element_type = check_type(element, state);
        if element_type != first_type {
            add_error(state, format!("Inconsistent types in array literal: expected {:?}, got {:?}", first_type, element_type), code_span);
        }
    });
}

fn visit_function_call(name: &str, args: &[ASTNode], state: &mut AnalyzerState, call_scope: usize, code_span: &mut CodeSpan) -> NailDataTypeDescriptor {
    // Check if this is a stdlib function first
    if let Some(func_type) = stdlib_types::get_stdlib_function_type(name) {
        // Check argument count
        if args.len() != func_type.parameters.len() {
            add_error(state, format!("{} expects {} arguments, got {}", name, func_type.parameters.len(), args.len()), code_span);
        }
        
        // Type check each argument against expected parameter types
        for (i, arg) in args.iter().enumerate() {
            // Use a mutable reference to allow visiting
            let mut arg_clone = arg.clone();
            visit_node(&mut arg_clone, state);
            
            // Check if we have a parameter definition for this argument
            if let Some(expected_param) = func_type.parameters.get(i) {
                let actual_type = check_type(&arg_clone, state);
                // Skip type checking if expected type is Unknown (accepts any type) or actual type is Unknown (failed to infer)
                if expected_param.param_type != NailDataTypeDescriptor::Unknown && 
                   actual_type != NailDataTypeDescriptor::Unknown && 
                   actual_type != expected_param.param_type {
                    add_error(state, format!("{} parameter '{}' expects type {:?}, got {:?}", 
                        name, expected_param.name, expected_param.param_type, actual_type), code_span);
                }
            }
        }
        
        return func_type.return_type.clone();
    }
    
    let symbol = match lookup_symbol(&state.scope_arena, call_scope, name) {
        Some(s) => s.clone(),
        None => {
            add_error(state, format!("Undefined function: {}", name), code_span);
            return NailDataTypeDescriptor::Unknown;
        }
    };

    match &symbol.data_type {
        NailDataTypeDescriptor::Fn(param_types, return_type) => {
            if param_types.len() != args.len() {
                add_error(state, format!("Function '{}' called with wrong number of arguments. Expected {}, got {}", name, param_types.len(), args.len()), code_span);
                return NailDataTypeDescriptor::Unknown;
            }

            let mut has_error = false;
            for (i, (arg, expected_type)) in args.iter().zip(param_types.iter()).enumerate() {
                let arg_type = check_type(arg, state);
                if arg_type != *expected_type {
                    add_error(state, format!("Type mismatch in argument {} of function '{}': expected {:?}, got {:?}", i + 1, name, expected_type, arg_type), code_span);
                    has_error = true;
                }
            }

            if has_error {
                NailDataTypeDescriptor::Unknown
            } else {
                (**return_type).clone()
            }
        }
        _ => {
            add_error(state, format!("'{}' is not a function", name), code_span);
            NailDataTypeDescriptor::Unknown
        }
    }
}

fn visit_return_declaration(expr: &ASTNode, state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    if !state.in_function {
        add_error(state, "Return statement outside of function".to_string(), code_span);
    }
    check_type(expr, state);
}

fn check_type(node: &ASTNode, state: &AnalyzerState) -> NailDataTypeDescriptor {
    match node {
        ASTNode::NumberLiteral { data_type, .. } => data_type.clone(),
        ASTNode::StringLiteral { .. } => NailDataTypeDescriptor::String,
        ASTNode::Identifier { name, .. } => lookup_symbol(&state.scope_arena, state.scope_arena.current_scope(), name).map_or(NailDataTypeDescriptor::Unknown, |s| s.data_type.clone()),
        ASTNode::ReturnDeclaration { statement, .. } => check_type(statement, state),
        ASTNode::UnaryOperation { operand, .. } => check_type(operand, state),
        ASTNode::BinaryOperation { left, right, operator, .. } => {
            let left_type = check_type(left, state);
            let right_type = check_type(right, state);
            
            // Comparison operators should return Boolean, not operand type
            match operator {
                Operation::Eq | Operation::Ne | Operation::Lt | Operation::Lte | Operation::Gt | Operation::Gte => {
                    // Comparison operators return Boolean regardless of operand types
                    // (as long as operands are compatible)
                    if left_type == right_type {
                        NailDataTypeDescriptor::Boolean
                    } else {
                        NailDataTypeDescriptor::Unknown
                    }
                }
                // Arithmetic operators return operand type
                Operation::Add | Operation::Sub | Operation::Mul | Operation::Div | Operation::Mod => {
                    if left_type == right_type {
                        left_type
                    } else {
                        NailDataTypeDescriptor::Unknown
                    }
                }
                // Other operators (logical, etc.) - for now return operand type
                _ => {
                    if left_type == right_type {
                        left_type
                    } else {
                        NailDataTypeDescriptor::Unknown
                    }
                }
            }
        }
        ASTNode::Program { statements, .. } => statements.last().map_or(NailDataTypeDescriptor::Unknown, |stmt| check_type(stmt, state)),
        ASTNode::FunctionDeclaration { data_type, .. } => data_type.clone(),
        ASTNode::LambdaDeclaration { data_type, .. } => data_type.clone(),
        ASTNode::FunctionCall { name, args, scope, .. } => {
            // Special handling for error-handling functions that need type inference
            if name == "safe" || name == "dangerous" || name == "expect" {
                // These functions take a Result<T, E> and return T
                // We need to infer T from the first argument
                if let Some(first_arg) = args.first() {
                    let arg_type = check_type(first_arg, state);
                    // If the argument is a Result type (Any with 2 types where second is Error)
                    if let NailDataTypeDescriptor::Any(types) = arg_type {
                        if types.len() == 2 && types[1] == NailDataTypeDescriptor::Error {
                            // Return the base type (first type in the Any)
                            return types[0].clone();
                        }
                    }
                }
                // If we can't infer, return Unknown
                return NailDataTypeDescriptor::Unknown;
            }
            
            // Check if this is a stdlib function first
            if let Some(func_type) = stdlib_types::get_stdlib_function_type(name) {
                return func_type.return_type.clone();
            }
            // Otherwise look up in symbol table
            lookup_symbol(&state.scope_arena, state.scope_arena.current_scope(), name).map_or(NailDataTypeDescriptor::Unknown, |s| {
                match &s.data_type {
                    NailDataTypeDescriptor::Fn(_, return_type) => (**return_type).clone(),
                    _ => s.data_type.clone()
                }
            })
        },
        ASTNode::ConstDeclaration { data_type, .. } => data_type.clone(),
        ASTNode::StructDeclarationField { data_type, .. } => data_type.clone(),
        ASTNode::IfStatement { condition_branches, .. } => condition_branches.first().map_or(NailDataTypeDescriptor::Unknown, |(_, branch)| check_type(branch, state)),
        ASTNode::Block { statements, .. } => statements.last().map_or(NailDataTypeDescriptor::Unknown, |stmt| check_type(stmt, state)),
        ASTNode::ParallelBlock { statements, .. } => statements.last().map_or(NailDataTypeDescriptor::Unknown, |stmt| check_type(stmt, state)),
        ASTNode::ParallelAssignment { assignments, .. } => {
            // Return the type of the last assignment, or Unknown if no assignments
            assignments.last().map_or(NailDataTypeDescriptor::Unknown, |(_, data_type, _)| data_type.clone())
        },
        ASTNode::StructDeclaration { name, .. } => NailDataTypeDescriptor::Struct(name.to_string()),
        ASTNode::StructInstantiation { name, .. } => NailDataTypeDescriptor::Struct(name.to_string()),
        ASTNode::StructInstantiationField { value, .. } => check_type(value, state),
        ASTNode::EnumDeclaration { name, .. } => NailDataTypeDescriptor::Enum(name.to_string()),
        ASTNode::EnumVariant { name, .. } => NailDataTypeDescriptor::Enum(name.to_string()),
        ASTNode::ArrayLiteral { elements, .. } => {
            if elements.is_empty() {
                NailDataTypeDescriptor::Unknown
            } else {
                let element_type = check_type(&elements[0], state);
                match element_type {
                    NailDataTypeDescriptor::Int => NailDataTypeDescriptor::ArrayInt,
                    NailDataTypeDescriptor::Float => NailDataTypeDescriptor::ArrayFloat,
                    NailDataTypeDescriptor::String => NailDataTypeDescriptor::ArrayString,
                    NailDataTypeDescriptor::Boolean => NailDataTypeDescriptor::ArrayBoolean,
                    NailDataTypeDescriptor::Struct(name) => NailDataTypeDescriptor::ArrayStruct(name),
                    NailDataTypeDescriptor::Enum(name) => NailDataTypeDescriptor::ArrayEnum(name),
                    _ => NailDataTypeDescriptor::Unknown,
                }
            }
        },
    }
}

fn add_symbol(state: &mut AnalyzerState, symbol: Symbol) {
    let scope = state.scope_arena.current_scope();
    let scope = state.scope_arena.get_scope_mut(scope);
    scope.symbols.insert(symbol.name.clone(), symbol);
}

fn lookup_symbol(arena: &ScopeArena, scope: usize, name: &str) -> Option<Symbol> {
    // we don't want to modify the original scope, so we'll use a copy for traversal
    let mut scope_for_traversal = scope;
    loop {
        let scope_data = arena.get_scope(scope_for_traversal);
        if let Some(symbol) = scope_data.symbols.get(name) {
            return Some(symbol.clone());
        }
        if scope_for_traversal == GLOBAL_SCOPE {
            break; // We've checked the global scope and haven't found the symbol
        }
        scope_for_traversal = scope_data.parent;
    }
    None
}

fn mark_symbol_as_used(state: &mut AnalyzerState, name: &str) -> bool {
    let mut current_scope = state.scope_arena.current_scope();
    loop {
        let scope = state.scope_arena.get_scope_mut(current_scope);
        if let Some(symbol) = scope.symbols.get_mut(name) {
            symbol.is_used = true;
            return true;
        }
        if current_scope == GLOBAL_SCOPE {
            break;
        }
        current_scope = scope.parent;
    }
    false
}

fn check_unused_symbols(state: &mut AnalyzerState) {
    for scope_data in 0..state.scope_arena.scopes.len() {
        let scope = state.scope_arena.get_scope(scope_data);
        for symbol in scope.symbols.values() {
            if !symbol.is_used {
                // this works but is annoying, we need something else for this
                // state.errors.push(CodeError { message: format!("Unused variable: {}", symbol.name), code_span: CodeSpan::default() });
            }
        }
    }
}

fn add_error(state: &mut AnalyzerState, message: String, code_span: &mut CodeSpan) {
    state.errors.push(CodeError { message, code_span: code_span.clone() });
}

// TODO: Add comprehensive tests once the lexer/parser interfaces are stable
// Current issues:
// 1. LexerState::new() doesn't exist
// 2. tokenize() function not found  
// 3. ParserState::new() doesn't exist
// 4. parse_program() function not found
// 5. Test data in lexer.rs using non-existent TokenType::Array variant
//
// These tests document the current type checker issues:
// - Comparison operators (==, !=, <, >, etc.) should return Boolean but currently return operand type
// - Boolean literals need to be properly supported in lexer
// - Result types (i!e syntax) need proper type checking

fn has_return_statement(node: &ASTNode) -> bool {
    match node {
        ASTNode::ReturnDeclaration { .. } => true,
        ASTNode::Block { statements, .. } => {
            statements.last().map_or(false, |stmt| has_return_statement(stmt))
        }
        ASTNode::IfStatement { condition_branches, else_branch, .. } => {
            // All branches must return for the if statement to return
            let mut all_branches_return = true;
            
            // Check each condition branch
            for (_, branch) in condition_branches {
                if !has_return_statement(branch) {
                    all_branches_return = false;
                    break;
                }
            }
            
            // Check else branch if it exists
            if let Some(else_branch) = else_branch {
                if !has_return_statement(else_branch) {
                    all_branches_return = false;
                }
            } else {
                // No else branch means not all paths return
                all_branches_return = false;
            }
            
            all_branches_return
        }
        _ => false,
    }
}

fn check_function_return(name: &str, data_type: &NailDataTypeDescriptor, body: &ASTNode, state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    let last_statement = match body {
        ASTNode::Block { statements, .. } => statements.last(),
        _ => None,
    };

    match last_statement {
        Some(ASTNode::ReturnDeclaration { statement, .. }) => {
            let actual_return_type = check_type(statement, state);
            
            // Check if the return type is compatible with the expected type
            let types_compatible = match (data_type, &actual_return_type) {
                // Exact match
                (expected, actual) if expected == actual => true,
                // Check if actual type is one of the types in Any
                (NailDataTypeDescriptor::Any(expected_types), actual) => {
                    expected_types.contains(actual)
                }
                _ => false,
            };
            
            if !types_compatible {
                add_error(state, format!("Type mismatch in return statement of function '{}': expected {:?}, got {:?}", name, data_type, actual_return_type), code_span);
            }
        }
        Some(ASTNode::IfStatement { condition_branches, else_branch, .. }) => {
            // If statement can provide a return value if all branches return
            let mut all_branches_return = true;
            
            // Check each condition branch
            for (_, branch) in condition_branches {
                if !has_return_statement(branch) {
                    all_branches_return = false;
                    break;
                }
            }
            
            // Check else branch if it exists
            if let Some(else_branch) = else_branch {
                if !has_return_statement(else_branch) {
                    all_branches_return = false;
                }
            } else {
                // No else branch means not all paths return
                all_branches_return = false;
            }
            
            if !all_branches_return && *data_type != NailDataTypeDescriptor::Void {
                add_error(state, format!("Missing return statement in function '{}'", name), code_span);
            }
        }
        _ => {
            if *data_type != NailDataTypeDescriptor::Void {
                add_error(state, format!("Missing return statement in function '{}'", name), code_span);
            }
        }
    }
}

fn visit_parallel_assignment(assignments: &mut [(String, NailDataTypeDescriptor, Box<ASTNode>)], state: &mut AnalyzerState) {
    for (name, data_type, value) in assignments.iter_mut() {
        // Check the value expression
        visit_node(value, state);
        
        // Type check the assignment
        let value_type = check_type(value, state);
        // For now, skip type compatibility check for parallel assignments
        // The variables will be available in the current scope after tokio::join! completes
        
        // Add the variable to the current scope
        add_symbol(state, Symbol {
            name: name.clone(),
            data_type: data_type.clone(),
            is_used: false,
        });
    }
}
