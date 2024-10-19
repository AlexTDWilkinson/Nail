use crate::lexer::CodeSpan;
use crate::lexer::NailDataTypeDescriptor;
use crate::lexer::Operation;
use crate::parser::ASTNode;
use crate::CodeError;
use std::collections::{HashMap, HashSet, VecDeque};

pub const NO_SCOPE: usize = usize::MAX;
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
        // Note that global scope has no parent, so it's just set to NO_SCOPE to cause an error if it's parent is accessed for some reason
        ScopeArena { scopes: vec![Scope { symbols: HashMap::new(), parent: NO_SCOPE }] }
    }

    pub fn push_scope(&mut self) -> usize {
        let new_index = self.scopes.len();
        let parent = new_index.checked_sub(1).unwrap_or(NO_SCOPE);
        self.scopes.push(Scope { symbols: HashMap::new(), parent });
        new_index
    }

    pub fn pop_scope(&mut self) -> Result<(), &'static str> {
        if self.scopes.len() == 1 {
            return Err("Cannot pop global scope");
        }
        self.scopes.pop();
        Ok(())
    }

    pub fn current_scope(&self) -> usize {
        self.scopes.len() - 1
    }

    pub fn get_scope(&self, index: usize) -> Option<&Scope> {
        self.scopes.get(index)
    }

    pub fn get_scope_mut(&mut self, index: usize) -> Option<&mut Scope> {
        self.scopes.get_mut(index)
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
}

fn new_analyzer_state() -> AnalyzerState {
    AnalyzerState { scope_arena: ScopeArena::new(), errors: Vec::new(), in_function: false, enum_variants: HashMap::new() }
}

pub fn checker(ast: &mut ASTNode) -> Result<(), Vec<CodeError>> {
    let mut state = new_analyzer_state();
    visit_node(ast, &mut state);
    check_unused_symbols(&mut state);

    if state.errors.is_empty() {
        Ok(())
    } else {
        Err(state.errors)
    }
}

fn visit_node(node: &mut ASTNode, state: &mut AnalyzerState) {
    match node {
        ASTNode::Program { statements, code_span, scope } => statements.iter_mut().for_each(|statement| visit_node(statement, state)),
        ASTNode::FunctionDeclaration { name, params, data_type, body, code_span, scope } => visit_function_declaration(name, params, data_type, body, state, code_span, scope),
        ASTNode::VariableDeclaration { name, data_type, value, code_span, .. } => visit_variable_declaration(name, data_type, value, state, code_span),
        ASTNode::ConstDeclaration { name, data_type, value, code_span, .. } => visit_const_declaration(name, data_type, value, state, code_span),
        ASTNode::BinaryOperation { left, operator, right, code_span, .. } => visit_binary_operation(left, operator, right, state, code_span),
        ASTNode::Identifier { name, code_span, scope, .. } => {
            if !mark_symbol_as_used(state, name) {
                add_error(state, format!("Undefined variable: {}", name), code_span);
            }
        }
        ASTNode::IfStatement { condition_branches, else_branch, code_span, scope } => visit_if_statement(condition_branches, else_branch, state),
        ASTNode::StructDeclaration { name, fields, code_span, scope } => visit_struct_declaration(name, fields, state, code_span),
        ASTNode::EnumDeclaration { name, variants, code_span, scope } => visit_enum_declaration(name, variants, state, code_span),
        ASTNode::ArrayLiteral { elements, code_span, scope } => visit_array_literal(elements, state, code_span),
        ASTNode::FunctionCall { name, args, code_span, scope } => {
            visit_function_call(name, args, state, *scope, code_span);
        }
        ASTNode::ReturnDeclaration { statement, code_span, scope } => visit_return_declaration(statement, state, code_span),
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
    scope: &mut usize,
) {
    *scope = state.scope_arena.push_scope();

    state.in_function = true;

    // Create the function type
    let param_types: Vec<NailDataTypeDescriptor> = params.iter().map(|(_, t)| t.clone()).collect();
    let function_type = NailDataTypeDescriptor::Fn(param_types, Box::new(return_type.clone()));

    // Add the function to the current scope
    add_symbol(state, Symbol { name: name.to_string(), data_type: function_type, is_used: false });

    // Add parameters to the function's scope
    params.iter().for_each(|(param_name, param_type)| {
        add_symbol(state, Symbol { name: param_name.clone(), data_type: param_type.clone(), is_used: false });
    });

    // Visit the function body
    visit_node(body, state);

    // Check function return
    check_function_return(name, return_type, body, state, code_span);

    state.in_function = false;
}

fn visit_variable_declaration(name: &str, data_type: &NailDataTypeDescriptor, value: &ASTNode, state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    let value_type = check_type(value, state);
    if *data_type != value_type {
        add_error(state, format!("Type mismatch in variable declaration named `{}`: expected {:?}, got {:?}", name, data_type, value_type), code_span);
    }
    add_symbol(state, Symbol { name: name.to_string(), data_type: data_type.clone(), is_used: false });
}

fn visit_const_declaration(name: &str, data_type: &NailDataTypeDescriptor, value: &ASTNode, state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    let value_type = check_type(value, state);
    if *data_type != value_type {
        add_error(state, format!("Type mismatch in const declaration named `{}`: expected {:?}, got {:?}", name, data_type, value_type), code_span);
    }
    add_symbol(state, Symbol { name: name.to_string(), data_type: data_type.clone(), is_used: false });
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
                if left_type != NailDataTypeDescriptor::Int && left_type != NailDataTypeDescriptor::Float && left_type != NailDataTypeDescriptor::String {
                    add_error(state, format!("Invalid operand type for comparison operation: {:?}", left_type), code_span);
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
    fields.iter().for_each(|field| {
        if let ASTNode::StructDeclarationField { name: field_name, data_type } = field {
            if matches!(data_type, NailDataTypeDescriptor::Struct(_) | NailDataTypeDescriptor::Enum(_)) {
                add_error(state, format!("Nested structs or enums are not allowed in struct '{}', field '{}'", name, field_name), code_span);
            }
        }
    });
}

fn visit_enum_declaration(name: &str, variants: &[ASTNode], state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    let mut variant_set = HashSet::new();
    variants.iter().for_each(|variant| {
        if let ASTNode::EnumVariant { name: variant_name, code_span, .. } = variant {
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
        ASTNode::Identifier { name, scope, .. } => lookup_symbol(&state.scope_arena, *scope, name).map_or(NailDataTypeDescriptor::Unknown, |s| s.data_type.clone()),
        ASTNode::ReturnDeclaration { statement, .. } => check_type(statement, state),
        ASTNode::UnaryOperation { operand, .. } => check_type(operand, state),
        ASTNode::BinaryOperation { left, right, .. } => {
            let left_type = check_type(left, state);
            let right_type = check_type(right, state);
            if left_type == right_type {
                left_type
            } else {
                NailDataTypeDescriptor::Unknown
            }
        }
        ASTNode::Program { statements, .. } => statements.last().map_or(NailDataTypeDescriptor::Unknown, |stmt| check_type(stmt, state)),
        ASTNode::FunctionDeclaration { data_type, .. } => data_type.clone(),
        ASTNode::LambdaDeclaration { data_type, .. } => data_type.clone(),
        ASTNode::FunctionCall { name, scope, .. } => lookup_symbol(&state.scope_arena, *scope, name).map_or(NailDataTypeDescriptor::Unknown, |s| s.data_type.clone()),
        ASTNode::VariableDeclaration { data_type, .. } => data_type.clone(),
        ASTNode::ConstDeclaration { data_type, .. } => data_type.clone(),
        ASTNode::StructDeclarationField { data_type, .. } => data_type.clone(),
        ASTNode::IfStatement { condition_branches, .. } => condition_branches.first().map_or(NailDataTypeDescriptor::Unknown, |(_, branch)| check_type(branch, state)),
        ASTNode::Block { statements, .. } => statements.last().map_or(NailDataTypeDescriptor::Unknown, |stmt| check_type(stmt, state)),
        ASTNode::StructDeclaration { name, .. } => NailDataTypeDescriptor::Struct(name.to_string()),
        ASTNode::StructInstantiation { name, scope, .. } => lookup_symbol(&state.scope_arena, *scope, name).map_or(NailDataTypeDescriptor::Unknown, |s| s.data_type.clone()),
        ASTNode::StructInstantiationField { value, .. } => check_type(value, state),
        ASTNode::EnumDeclaration { name, .. } => NailDataTypeDescriptor::Enum(name.to_string()),
        ASTNode::EnumVariant { name, .. } => NailDataTypeDescriptor::Enum(name.to_string()),
        ASTNode::ArrayLiteral { elements, .. } => elements.first().map_or(NailDataTypeDescriptor::Unknown, |element| check_type(element, state)),
    }
}

fn add_symbol(state: &mut AnalyzerState, symbol: Symbol) {
    // Get the current scope index
    let current_scope_data = state.scope_arena.current_scope();

    // Add the symbol to the current scope
    if let Some(current_scope) = state.scope_arena.get_scope_mut(current_scope_data) {
        current_scope.symbols.insert(symbol.name.clone(), symbol);
    } else {
        // This should never happen if scopes are managed correctly
        panic!("Current scope does not exist in ScopeArena");
    }
}

fn lookup_symbol(arena: &ScopeArena, scope: usize, name: &str) -> Option<Symbol> {
    // we don't want to modify the original scope, so we'll use a copy for traversal
    let mut scope_for_traversal = scope;
    loop {
        if let Some(scope_data) = arena.get_scope(scope_for_traversal) {
            if let Some(symbol) = scope_data.symbols.get(name) {
                return Some(symbol.clone());
            }
            if scope_for_traversal == GLOBAL_SCOPE {
                break; // We've checked the global scope and haven't found the symbol
            }
            scope_for_traversal = scope_data.parent;
        } else {
            panic!("SCOPE TRAVERSAL ERROR. THIS SHOULD NEVER HAPPEN.");
        }
    }
    None
}

fn mark_symbol_as_used(state: &mut AnalyzerState, name: &str) -> bool {
    let mut current_scope = state.scope_arena.current_scope();
    loop {
        if let Some(scope) = state.scope_arena.get_scope_mut(current_scope) {
            if let Some(symbol) = scope.symbols.get_mut(name) {
                symbol.is_used = true;
                return true;
            }
            if current_scope == GLOBAL_SCOPE {
                break;
            }
            current_scope = scope.parent;
        } else {
            break;
        }
    }
    false
}

fn check_unused_symbols(state: &mut AnalyzerState) {
    for scope_data in 0..state.scope_arena.scopes.len() {
        if let Some(scope) = state.scope_arena.get_scope(scope_data) {
            for symbol in scope.symbols.values() {
                if !symbol.is_used {
                    // this works but is annoying, we need something else for this
                    // state.errors.push(CodeError { message: format!("Unused variable: {}", symbol.name), code_span: CodeSpan::default() });
                }
            }
        }
    }
}

fn add_error(state: &mut AnalyzerState, message: String, code_span: &mut CodeSpan) {
    state.errors.push(CodeError { message, code_span: code_span.clone() });
}

fn check_function_return(name: &str, data_type: &NailDataTypeDescriptor, body: &ASTNode, state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    let last_statement = match body {
        ASTNode::Block { statements, .. } => statements.last(),
        _ => None,
    };

    if let Some(ASTNode::ReturnDeclaration { statement, .. }) = last_statement {
        let actual_return_type = check_type(statement, state);
        if actual_return_type != *data_type {
            add_error(state, format!("Type mismatch in return statement of function '{}': expected {:?}, got {:?}", name, data_type, actual_return_type), code_span);
        }
    } else if *data_type != NailDataTypeDescriptor::Void {
        add_error(state, format!("Missing return statement in function '{}'", name), code_span);
    }
}
