use crate::common::{CodeError, CodeSpan};
use crate::lexer::{NailDataTypeDescriptor, Operation};
use crate::parser::ASTNode;
use crate::stdlib_registry::get_stdlib_function;
use std::collections::{HashMap, HashSet};

pub use crate::common::{ERROR_SCOPE, GLOBAL_SCOPE};

#[derive(Debug, Clone, PartialEq)]
pub struct Scope {
    pub symbols: HashMap<String, Symbol>,
    pub parent: usize,
    pub is_lambda: bool,
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
        ScopeArena { scopes: vec![Scope { symbols: HashMap::new(), parent: ERROR_SCOPE, is_lambda: false }] }
    }

    pub fn push_scope(&mut self) -> usize {
        let parent = self.current_scope();
        self.scopes.push(Scope { symbols: HashMap::new(), parent, is_lambda: false });
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

    pub fn mark_as_lambda(&mut self, scope_index: usize) {
        if let Some(scope) = self.scopes.get_mut(scope_index) {
            scope.is_lambda = true;
        }
    }

    pub fn is_in_lambda(&self) -> bool {
        // Check if any scope in the current chain is a lambda
        let mut current = self.current_scope();
        while current != ERROR_SCOPE && current != GLOBAL_SCOPE {
            if self.scopes[current].is_lambda {
                return true;
            }
            current = self.scopes[current].parent;
        }
        false
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ReturnContext {
    None,
    Function,
    CollectionExpression,  // map, filter, reduce, etc.
}

struct AnalyzerState {
    scope_arena: ScopeArena,
    errors: Vec<CodeError>,
    return_context: ReturnContext,
    enum_variants: HashMap<String, HashSet<String>>,
    structs: HashMap<String, Vec<ASTNode>>,
    struct_spans: HashMap<String, CodeSpan>,
    in_loop: bool,
    function_scope: Option<usize>,  // Track the scope of the current function
}

/// Type names the transpiler emits unqualified in generated Rust (String, Vec<...>,
/// Result<..., String>, ...). A user struct or enum with one of these names would
/// shadow the real type inside the generated code and produce Rust that cannot
/// compile, so declaring them is rejected outright.
const RESERVED_RUST_TYPE_NAMES: &[&str] = &["String", "Vec", "Box", "Result", "Option", "HashMap", "DashMap"];

fn new_analyzer_state() -> AnalyzerState {
    AnalyzerState { 
        scope_arena: ScopeArena::new(), 
        errors: Vec::new(), 
        return_context: ReturnContext::None,
        enum_variants: HashMap::new(),
        structs: HashMap::new(),
        struct_spans: HashMap::new(),
        in_loop: false,
        function_scope: None,
    }
}

pub fn checker(ast: &mut ASTNode) -> Result<(), Vec<CodeError>> {
    let mut state = new_analyzer_state();

    if let ASTNode::Program { scope, .. } = ast {
        *scope = GLOBAL_SCOPE;
    }

    visit_node(ast, &mut state);
    check_recursive_structs(&mut state);
    check_unused_symbols(&mut state);
    if state.errors.is_empty() {
        Ok(())
    } else {
        Err(state.errors)
    }
}

fn update_node_scope(node: &mut ASTNode, new_scope: usize) {
    match node {
        ASTNode::Program { scope, .. }
        | ASTNode::ConstDeclaration { scope, .. }
        | ASTNode::StringLiteral { scope, .. }
        | ASTNode::NumberLiteral { scope, .. }
        | ASTNode::BooleanLiteral { scope, .. }
        | ASTNode::UnaryOperation { scope, .. }
        | ASTNode::BinaryOperation { scope, .. }
        | ASTNode::Assignment { scope, .. }
        | ASTNode::Identifier { scope, .. }
        | ASTNode::IfStatement { scope, .. }
        | ASTNode::ForLoop { scope, .. }
        | ASTNode::MapExpression { scope, .. }
        | ASTNode::FilterExpression { scope, .. }
        | ASTNode::ReduceExpression { scope, .. }
        | ASTNode::EachExpression { scope, .. }
        | ASTNode::FindExpression { scope, .. }
        | ASTNode::AllExpression { scope, .. }
        | ASTNode::AnyExpression { scope, .. }
        | ASTNode::WhileLoop { scope, .. }
        | ASTNode::Loop { scope, .. }
        | ASTNode::SpawnBlock { scope, .. }
        | ASTNode::BreakStatement { scope, .. }
        | ASTNode::ContinueStatement { scope, .. }
        | ASTNode::StructDeclaration { scope, .. }
        | ASTNode::EnumDeclaration { scope, .. }
        | ASTNode::ArrayLiteral { scope, .. }
        | ASTNode::FunctionCall { scope, .. }
        | ASTNode::ReturnDeclaration { scope, .. }
        | ASTNode::YieldDeclaration { scope, .. }
        | ASTNode::Block { scope, .. }
        | ASTNode::ParallelBlock { scope, .. }
        | ASTNode::ConcurrentBlock { scope, .. }
        | ASTNode::LambdaDeclaration { scope, .. }
        | ASTNode::FunctionDeclaration { scope, .. }
        | ASTNode::StructInstantiation { scope, .. }
        | ASTNode::StructInstantiationField { scope, .. }
        | ASTNode::StructFieldAccess { scope, .. }
        | ASTNode::NestedFieldAccess { scope, .. }
        | ASTNode::StructDeclarationField { scope, .. }
        | ASTNode::EnumVariant { scope, .. } => {
            *scope = new_scope;
        }
    }
    
    // Recursively update children
    match node {
        ASTNode::Program { statements, .. } => {
            for stmt in statements {
                update_node_scope(stmt, new_scope);
            }
        }
        ASTNode::Block { statements, .. } | ASTNode::ParallelBlock { statements, .. } | ASTNode::ConcurrentBlock { statements, .. } => {
            for stmt in statements {
                update_node_scope(stmt, new_scope);
            }
        }
        ASTNode::ForLoop { iterable, body, .. } => {
            update_node_scope(iterable, new_scope);
            update_node_scope(body, new_scope);
        }
        ASTNode::WhileLoop { condition, body, .. } => {
            update_node_scope(condition, new_scope);
            update_node_scope(body, new_scope);
        }
        ASTNode::Loop { body, .. } => {
            update_node_scope(body, new_scope);
        }
        ASTNode::SpawnBlock { body, .. } => {
            update_node_scope(body, new_scope);
        }
        ASTNode::MapExpression { iterable, body, .. } | ASTNode::EachExpression { iterable, body, .. } => {
            update_node_scope(iterable, new_scope);
            update_node_scope(body, new_scope);
        }
        ASTNode::ReduceExpression { iterable, initial_value, body, .. } => {
            update_node_scope(iterable, new_scope);
            update_node_scope(initial_value, new_scope);
            update_node_scope(body, new_scope);
        }
        ASTNode::FilterExpression { iterable, body, .. } | ASTNode::FindExpression { iterable, body, .. } | ASTNode::AllExpression { iterable, body, .. } | ASTNode::AnyExpression { iterable, body, .. } => {
            update_node_scope(iterable, new_scope);
            update_node_scope(body, new_scope);
        }
        ASTNode::IfStatement { condition_branches, else_branch, .. } => {
            for (cond, branch) in condition_branches {
                update_node_scope(cond, new_scope);
                update_node_scope(branch, new_scope);
            }
            if let Some(else_br) = else_branch {
                update_node_scope(else_br, new_scope);
            }
        }
        ASTNode::BinaryOperation { left, right, .. } => {
            update_node_scope(left, new_scope);
            update_node_scope(right, new_scope);
        }
        ASTNode::UnaryOperation { operand, .. } => {
            update_node_scope(operand, new_scope);
        }
        ASTNode::FunctionCall { args, .. } => {
            for arg in args {
                update_node_scope(arg, new_scope);
            }
        }
        ASTNode::ReturnDeclaration { statement, .. } => {
            update_node_scope(statement, new_scope);
        }
        ASTNode::YieldDeclaration { statement, .. } => {
            update_node_scope(statement, new_scope);
        }
        ASTNode::ArrayLiteral { elements, .. } => {
            for elem in elements {
                update_node_scope(elem, new_scope);
            }
        }
        ASTNode::StructInstantiation { fields, .. } => {
            for field in fields {
                update_node_scope(field, new_scope);
            }
        }
        ASTNode::StructInstantiationField { value, .. } => {
            update_node_scope(value, new_scope);
        }
        ASTNode::StructFieldAccess { .. } => {
            // struct_name is a String, not an ASTNode
        }
        ASTNode::NestedFieldAccess { object, .. } => {
            update_node_scope(object, new_scope);
        }
        // Terminal nodes that don't have children to update
        ASTNode::Identifier { .. } |
        ASTNode::NumberLiteral { .. } |
        ASTNode::StringLiteral { .. } |
        ASTNode::BooleanLiteral { .. } |
        ASTNode::BreakStatement { .. } |
        ASTNode::ContinueStatement { .. } |
        ASTNode::StructDeclarationField { .. } |
        ASTNode::EnumVariant { .. } => {
            // These nodes don't have child nodes to update
        }
        // Handle the remaining cases
        ASTNode::LambdaDeclaration { body, .. } => {
            update_node_scope(body, new_scope);
        }
        ASTNode::FunctionDeclaration { body, .. } => {
            update_node_scope(body, new_scope);
        }
        ASTNode::StructDeclaration { fields, .. } => {
            for field in fields {
                update_node_scope(field, new_scope);
            }
        }
        ASTNode::EnumDeclaration { variants, .. } => {
            for variant in variants {
                update_node_scope(variant, new_scope);
            }
        }
        ASTNode::ConstDeclaration { value, .. } => {
            update_node_scope(value, new_scope);
        }
        ASTNode::Assignment { left, right, .. } => {
            update_node_scope(left, new_scope);
            update_node_scope(right, new_scope);
        }
        _ => {
            // All cases should be handled explicitly
            eprintln!("Unhandled node type in update_node_scope: {:?}", node);
            panic!("update_node_scope: unhandled node type");
        }
    }
}

fn visit_node(node: &mut ASTNode, state: &mut AnalyzerState) {
    let current_scope = state.scope_arena.current_scope();

    // Set the scope for all nodes to the current scope
    // This ensures all nodes have their scope properly set during traversal
    match node {
        ASTNode::Program { scope, .. }
        | ASTNode::ConstDeclaration { scope, .. }
        | ASTNode::StringLiteral { scope, .. }
        | ASTNode::NumberLiteral { scope, .. }
        | ASTNode::BooleanLiteral { scope, .. }
        | ASTNode::UnaryOperation { scope, .. }
        | ASTNode::BinaryOperation { scope, .. }
        | ASTNode::Assignment { scope, .. }
        | ASTNode::Identifier { scope, .. }
        | ASTNode::IfStatement { scope, .. }
        | ASTNode::ForLoop { scope, .. }
        | ASTNode::MapExpression { scope, .. }
        | ASTNode::FilterExpression { scope, .. }
        | ASTNode::ReduceExpression { scope, .. }
        | ASTNode::EachExpression { scope, .. }
        | ASTNode::FindExpression { scope, .. }
        | ASTNode::AllExpression { scope, .. }
        | ASTNode::AnyExpression { scope, .. }
        | ASTNode::WhileLoop { scope, .. }
        | ASTNode::Loop { scope, .. }
        | ASTNode::SpawnBlock { scope, .. }
        | ASTNode::BreakStatement { scope, .. }
        | ASTNode::ContinueStatement { scope, .. }
        | ASTNode::StructDeclaration { scope, .. }
        | ASTNode::EnumDeclaration { scope, .. }
        | ASTNode::ArrayLiteral { scope, .. }
        | ASTNode::FunctionCall { scope, .. }
        | ASTNode::ReturnDeclaration { scope, .. }
        | ASTNode::YieldDeclaration { scope, .. }
        | ASTNode::Block { scope, .. }
        | ASTNode::ParallelBlock { scope, .. }
        | ASTNode::ConcurrentBlock { scope, .. }
        | ASTNode::LambdaDeclaration { scope, .. }
        | ASTNode::FunctionDeclaration { scope, .. }
        | ASTNode::StructInstantiation { scope, .. }
        | ASTNode::StructInstantiationField { scope, .. }
        | ASTNode::StructFieldAccess { scope, .. }
        | ASTNode::NestedFieldAccess { scope, .. }
        | ASTNode::StructDeclarationField { scope, .. }
        | ASTNode::EnumVariant { scope, .. } => {
            *scope = current_scope;
        }
    }

    match node {
        ASTNode::Program { statements, .. } => {
            // Hoist top-level function signatures before visiting bodies so a
            // function can call one declared later in the file (mutual
            // recursion). Duplicate detection lives here because the hoisted
            // symbol already exists by the time the declaration is visited.
            let mut hoisted_functions = HashSet::new();
            for statement in statements.iter() {
                if let ASTNode::FunctionDeclaration { name, params, data_type, code_span, .. } = statement {
                    if !hoisted_functions.insert(name.clone()) {
                        add_error(state, format!("Function '{}' is already defined", name), &mut code_span.clone());
                        continue;
                    }
                    let param_types: Vec<NailDataTypeDescriptor> = params.iter().map(|(_, t)| resolve_custom_type(state, t)).collect();
                    let function_type = NailDataTypeDescriptor::Fn(param_types, Box::new(resolve_custom_type(state, data_type)));
                    add_symbol(state, Symbol { name: name.clone(), data_type: function_type, is_used: false });
                }
            }

            // Check each top-level statement
            for statement in statements.iter_mut() {
                // Check if this is a function call used as a statement
                if let ASTNode::FunctionCall { name, args, code_span, scope } = statement {
                    let return_type = visit_function_call(name, args, state, *scope, code_span);
                    // Functions that return non-void values must have their results used
                    if return_type != NailDataTypeDescriptor::Void && 
                       return_type != NailDataTypeDescriptor::Never && 
                       return_type != NailDataTypeDescriptor::FailedToResolve {
                        add_error_with_help(state, format!("The value returned by '{}' is silently discarded here", name), Some(format!("assign it to a variable: result:<type> = {}(...); Nail has no bare expression statements for value-returning calls", name)), code_span);
                    }
                } else {
                    visit_node(statement, state);
                }
            }
        }
        ASTNode::FunctionDeclaration { name, params, data_type, body, code_span, .. } => visit_function_declaration(name, params, data_type, body, state, code_span),
        ASTNode::ConstDeclaration { name, data_type, value, code_span, .. } => visit_const_declaration(name, data_type, value, state, code_span),
        ASTNode::BinaryOperation { left, operator, right, code_span, .. } => visit_binary_operation(left, operator, right, state, code_span),
        ASTNode::UnaryOperation { operator, operand, code_span, .. } => visit_unary_operation(operator, operand, state, code_span),
        ASTNode::Identifier { name, code_span, .. } => {
            // Check if we're in a function and the variable is from outside
            if let Some(func_scope) = state.function_scope {
                if !check_variable_in_function_scope(state, name, func_scope) {
                    add_error(state, format!("Cannot access variable '{}' from outside function scope. Functions can only use parameters and local variables.", name), code_span);
                    return;
                }
            }
            
            if !mark_symbol_as_used(state, name) {
                let help = best_suggestion(name, visible_symbol_names(state).iter().map(|s| s.as_str())).map(|suggestion| format!("did you mean '{}'?", suggestion));
                add_error_with_help(state, format!("There is no variable named '{}' in this scope", name), help, code_span);
            }
        }
        ASTNode::IfStatement { condition_branches, else_branch, .. } => visit_if_statement(condition_branches, else_branch, state),
        ASTNode::ForLoop { iterator, iterable, initial_value, filter, body, scope, .. } => visit_for_loop(iterator, iterable, initial_value, filter, body, state, *scope),
        ASTNode::MapExpression { iterator, index_iterator, iterable, body, scope, .. } => visit_map_expression(iterator, index_iterator, iterable, body, state, *scope),
        ASTNode::FilterExpression { iterator, index_iterator, iterable, body, scope, .. } => visit_filter_expression(iterator, index_iterator, iterable, body, state, *scope),
        ASTNode::ReduceExpression { accumulator, iterator, index_iterator, iterable, initial_value, body, scope, .. } => visit_reduce_expression(accumulator, iterator, index_iterator, iterable, initial_value, body, state, *scope),
        ASTNode::EachExpression { iterator, index_iterator, iterable, body, scope, .. } => visit_each_expression(iterator, index_iterator, iterable, body, state, *scope),
        ASTNode::FindExpression { iterator, index_iterator, iterable, body, scope, .. } => visit_find_expression(iterator, index_iterator, iterable, body, state, *scope),
        ASTNode::AllExpression { iterator, index_iterator, iterable, body, scope, .. } => visit_all_expression(iterator, index_iterator, iterable, body, state, *scope),
        ASTNode::AnyExpression { iterator, index_iterator, iterable, body, scope, .. } => visit_any_expression(iterator, index_iterator, iterable, body, state, *scope),
        ASTNode::WhileLoop { condition, initial_value, body, max_iterations, .. } => visit_while_loop(condition, initial_value, body, max_iterations, state),
        ASTNode::Loop { index_iterator, body, scope, .. } => visit_loop(index_iterator, body, state, *scope),
        ASTNode::SpawnBlock { body, .. } => visit_spawn_block(body, state),
        ASTNode::BreakStatement { code_span, .. } => visit_break_statement(state, code_span),
        ASTNode::ContinueStatement { code_span, .. } => visit_continue_statement(state, code_span),
        ASTNode::StructDeclaration { name, fields, code_span, .. } => visit_struct_declaration(name, fields, state, code_span),
        ASTNode::StructFieldAccess { struct_name, field_name, code_span, .. } => visit_struct_field_access(struct_name, field_name, state, code_span),
        ASTNode::NestedFieldAccess { object, field_name, code_span, .. } => visit_nested_field_access(object, field_name, state, code_span),
        ASTNode::EnumDeclaration { name, variants, code_span, .. } => visit_enum_declaration(name, variants, state, code_span),
        ASTNode::ArrayLiteral { elements, code_span, .. } => visit_array_literal(elements, state, code_span),
        ASTNode::FunctionCall { name, args, code_span, scope } => {
            // Just visit the function call, don't check return usage here
            // The check for unused returns should only happen at the statement level
            visit_function_call(name, args, state, *scope, code_span);
        }
        ASTNode::ReturnDeclaration { statement, code_span, .. } => visit_return_declaration(statement, state, code_span),
        ASTNode::YieldDeclaration { statement, code_span, .. } => visit_yield_declaration(statement, state, code_span),
        ASTNode::ParallelBlock { statements, .. } => statements.iter_mut().for_each(|statement| visit_node(statement, state)),
        ASTNode::ConcurrentBlock { statements, .. } => statements.iter_mut().for_each(|statement| visit_node(statement, state)),
        ASTNode::Block { statements, scope, .. } => {
            // If scope is already set (e.g., for function bodies), use it
            // Otherwise create a new scope for blocks
            let needs_new_scope = *scope == 0;
            if needs_new_scope {
                let block_scope = state.scope_arena.push_scope();
                *scope = block_scope;
            }

            // Visit all statements in the block
            for statement in statements.iter_mut() {
                // Check if this is a function call used as a statement
                if let ASTNode::FunctionCall { name, args, code_span, scope } = statement {
                    let return_type = visit_function_call(name, args, state, *scope, code_span);
                    // Functions that return non-void values must have their results used
                    if return_type != NailDataTypeDescriptor::Void && 
                       return_type != NailDataTypeDescriptor::Never && 
                       return_type != NailDataTypeDescriptor::FailedToResolve {
                        add_error_with_help(state, format!("The value returned by '{}' is silently discarded here", name), Some(format!("assign it to a variable: result:<type> = {}(...); Nail has no bare expression statements for value-returning calls", name)), code_span);
                    }
                } else {
                    visit_node(statement, state);
                }
            }

            // NOTE: We don't pop the scope here because type checking happens later
            // and needs access to all scopes. Scopes will be preserved throughout
            // the entire type checking process.
        }
        ASTNode::LambdaDeclaration { params, body, code_span, .. } => visit_lambda_declaration(params, body, state, code_span),
        ASTNode::StringLiteral { .. } => {},  // Literals don't need additional processing
        ASTNode::NumberLiteral { .. } => {},
        ASTNode::BooleanLiteral { .. } => {},
        ASTNode::StructInstantiationField { value, .. } => visit_node(value, state),
        ASTNode::StructInstantiation { name, fields, code_span, .. } => {
            let name = name.clone();
            let mut code_span = code_span.clone();
            visit_struct_instantiation(&name, fields, state, &mut code_span);
        }
        ASTNode::EnumVariant { .. } => {},  // Enum variants don't need additional processing
        ASTNode::Assignment { left, right, .. } => {
            // Visit both sides of assignment
            visit_node(left, state);
            visit_node(right, state);
        }
        _ => {
            panic!("visit_node: unhandled node type");
        }
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
    // Check if the function name is "main" - this is not allowed in Nail
    if name == "main" {
        add_error(state, "Function name 'main' is reserved and cannot be used".to_string(), code_span);
        return;
    }

    // Check if the function name collides with a stdlib function. Stdlib names
    // resolve before user symbols, so allowing this would silently shadow the
    // user's function - error loudly instead.
    if crate::stdlib_registry::is_stdlib_function(name) {
        add_error_with_help(
            state,
            format!("Cannot define function '{}': this name belongs to a standard library function", name),
            Some(format!("choose a different name that does not collide with the standard library, e.g. 'my_{}'", name)),
            code_span,
        );
        return;
    }

    // Duplicate function definitions are detected during signature hoisting in
    // the Program visit; here only the parameters still need validation.

    // Reject duplicate parameter names
    let mut seen_params = HashSet::new();
    for (param_name, _) in params {
        if !seen_params.insert(param_name.clone()) {
            add_error(state, format!("Duplicate parameter '{}' in function '{}'", param_name, name), code_span);
        }
    }

    // Create the function type, resolving enum names the parser typed as structs
    let param_types: Vec<NailDataTypeDescriptor> = params.iter().map(|(_, t)| resolve_custom_type(state, t)).collect();
    let function_type = NailDataTypeDescriptor::Fn(param_types, Box::new(resolve_custom_type(state, return_type)));

    // Add the function to the current scope (parent scope)
    add_symbol(state, Symbol { name: name.to_string(), data_type: function_type, is_used: false });

    // Now push a new scope for the function body
    let function_scope = state.scope_arena.push_scope();

    // Update the function body's scope
    if let ASTNode::Block { scope, .. } = body.as_mut() {
        *scope = function_scope;
    }

    // Track that we're in a function scope
    let previous_function_scope = state.function_scope;
    state.function_scope = Some(function_scope);
    state.return_context = ReturnContext::Function;

    // Add parameters to the function's scope
    params.iter().for_each(|(param_name, param_type)| {
        let param_type = resolve_custom_type(state, param_type);
        add_symbol(state, Symbol { name: param_name.clone(), data_type: param_type, is_used: false });
    });

    // Visit the function body
    visit_node(body, state);

    // Check function return
    check_function_return(name, return_type, body, state, code_span);

    // NOTE: We don't pop the function scope here because type checking happens later
    // and needs access to all scopes. Scopes will be preserved throughout
    // the entire type checking process.

    // Restore the previous function scope context
    state.function_scope = previous_function_scope;
    state.return_context = ReturnContext::None;
}

/// The parser cannot tell enum names from struct names in type annotations, so
/// every custom type arrives as Struct(name). Resolve names that belong to a
/// declared enum into Enum(name) so type comparisons see the real kind.
fn resolve_custom_type(state: &AnalyzerState, data_type: &NailDataTypeDescriptor) -> NailDataTypeDescriptor {
    match data_type {
        NailDataTypeDescriptor::Struct(name) if state.enum_variants.contains_key(name) => NailDataTypeDescriptor::Enum(name.clone()),
        NailDataTypeDescriptor::Array(inner) => NailDataTypeDescriptor::Array(Box::new(resolve_custom_type(state, inner))),
        NailDataTypeDescriptor::Result(inner) => NailDataTypeDescriptor::Result(Box::new(resolve_custom_type(state, inner))),
        NailDataTypeDescriptor::HashMap(key, value) => NailDataTypeDescriptor::HashMap(Box::new(resolve_custom_type(state, key)), Box::new(resolve_custom_type(state, value))),
        other => other.clone(),
    }
}

fn visit_const_declaration(name: &str, data_type: &NailDataTypeDescriptor, value: &mut ASTNode, state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    // Check if the value is an if expression without an else branch
    if let ASTNode::IfStatement { else_branch, .. } = value {
        if else_branch.is_none() && *data_type != NailDataTypeDescriptor::Void {
            add_error(state, format!("If expression used as value must have an else branch to ensure all cases return a value"), code_span);
        }
    }

    // Visit the value node to ensure proper scope setup for collection expressions
    visit_node(value, state);

    // Check the declared type against the value's inferred type. Only fully
    // concrete inferred types are compared, so an earlier inference failure
    // (already reported) doesn't cascade into a second confusing error here.
    let data_type = &resolve_custom_type(state, data_type);
    let value_type = check_type(value, state);
    if let NailDataTypeDescriptor::Result(inner) = &value_type {
        if !matches!(data_type, NailDataTypeDescriptor::Result(_)) {
            add_error_with_help(
                state,
                format!("'{}' is declared as {} but its value is {}", name, data_type.describe(), value_type.describe()),
                Some(format!(
                    "results must be handled where they occur: danger(...) to crash on error, expect(...) to fall back to a default, or safe(..., handler) to run an error handler. For example: {}:{} = danger(...);",
                    name, inner
                )),
                code_span,
            );
        }
    } else if is_concrete_type(data_type) && is_concrete_type(&value_type) && *data_type != value_type {
        add_error_with_help(
            state,
            format!("'{}' is declared as {} but its value is {}", name, data_type.describe(), value_type.describe()),
            Some(format!("either change the declaration to '{}:{}' or make the value {}", name, value_type, data_type.describe())),
            code_span,
        );
    }

    // Add the symbol using the declared type
    add_symbol(state, Symbol { name: name.to_string(), data_type: data_type.clone(), is_used: false });
}

fn visit_unary_operation(_operator: &Operation, operand: &mut ASTNode, state: &mut AnalyzerState, _code_span: &mut CodeSpan) {
    // Type checking will happen in a separate phase after visiting
    // Just visit the operand for now
    visit_node(operand, state);
}

fn visit_binary_operation(left: &mut ASTNode, operator: &Operation, right: &mut ASTNode, state: &mut AnalyzerState, _code_span: &mut CodeSpan) {
    // Type checking happens in a separate phase; here we visit both operands so
    // errors inside them (undefined variables, bad calls, ...) are reported.
    visit_node(left, state);
    visit_node(right, state);
}

fn visit_if_statement(condition_branches: &mut [(Box<ASTNode>, Box<ASTNode>)], else_branch: &mut Option<Box<ASTNode>>, state: &mut AnalyzerState) {
    // When if is used as an expression (e.g., x = if {...}), the branches can contain return statements
    // to return values from the if expression itself. We need to temporarily allow returns.
    let prev_context = state.return_context.clone();

    // Check if this if statement is being used as an expression by looking at context
    // For now, we'll allow returns in all if branches since they can be used as expressions
    state.return_context = ReturnContext::Function;

    for (condition, branch) in condition_branches.iter_mut() {
        visit_node(condition, state);
        visit_node(branch, state);
    }
    if let Some(branch) = else_branch {
        visit_node(branch, state);
    }

    // Restore the original context
    state.return_context = prev_context;
}

fn visit_map_expression(iterator: &str, index_iterator: &Option<String>, iterable: &mut ASTNode, body: &mut ASTNode, state: &mut AnalyzerState, _scope: usize) {
    // First visit the iterable to ensure it's valid
    visit_node(iterable, state);
    
    // Create a new scope for the map body
    let map_scope = state.scope_arena.push_scope();
    
    // Get the type of the iterable
    let iterable_type = check_type(iterable, state);
    
    // Determine the iterator type based on the iterable type
    let iterator_type = match &iterable_type {
        NailDataTypeDescriptor::Array(element_type) => (**element_type).clone(),
        NailDataTypeDescriptor::String => NailDataTypeDescriptor::String, // Each character
        _ => {
            if !iterable_type.is_unresolved() {
                add_error_with_help(state, format!("'map' works on arrays and strings, but this is neither — it is just {}", iterable_type.describe()), collection_op_help("map", iterable), &mut iterable.code_span());
            }
            NailDataTypeDescriptor::FailedToResolve
        }
    };
    
    // Add the iterator variable to the map scope
    if let Some(scope_data) = state.scope_arena.scopes.get_mut(map_scope) {
        scope_data.symbols.insert(
            iterator.to_string(),
            Symbol {
                name: iterator.to_string(),
                data_type: iterator_type,
                is_used: true, // Mark as used since it's a map variable
            },
        );
        
        // If there's an index iterator, add it to the scope
        if let Some(idx_name) = index_iterator {
            scope_data.symbols.insert(
                idx_name.clone(),
                Symbol {
                    name: idx_name.clone(),
                    data_type: NailDataTypeDescriptor::Int,
                    is_used: true,
                },
            );
        }
    }
    
    // Update the body scope before visiting
    update_node_scope(body, map_scope);
    
    // Set context to allow return statements in map body
    let prev_context = state.return_context.clone();
    state.return_context = ReturnContext::CollectionExpression;
    
    // Visit the body with the new scope
    visit_node(body, state);
    
    // Restore the context
    state.return_context = prev_context;
    
    // Verify the body contains a yield statement (or return statement for compatibility)
    let mut has_yield = false;
    if let ASTNode::Block { statements, .. } = body {
        for stmt in statements {
            if matches!(stmt, ASTNode::YieldDeclaration { .. } | ASTNode::ReturnDeclaration { .. }) {
                has_yield = true;
                break;
            }
        }
    }
    
    if !has_yield {
        add_error(state, "Map expression must contain a yield statement".to_string(), &mut body.code_span());
    }
    
    // NOTE: We don't pop the scope here because type checking happens later
    // and needs access to all scopes. Scopes will be preserved throughout
    // the entire type checking process.
}

fn visit_for_loop(iterator: &str, iterable: &mut ASTNode, initial_value: &mut Option<Box<ASTNode>>, filter: &mut Option<Box<ASTNode>>, body: &mut ASTNode, state: &mut AnalyzerState, _scope: usize) {
    // First visit the iterable to ensure it's valid
    visit_node(iterable, state);
    
    // Create a new scope for the loop body
    let loop_scope = state.scope_arena.push_scope();
    
    // Get the type of the iterable
    let iterable_type = check_type(iterable, state);
    
    // Determine the iterator type based on the iterable type
    let iterator_type = match &iterable_type {
        NailDataTypeDescriptor::Array(element_type) => (**element_type).clone(),
        NailDataTypeDescriptor::String => NailDataTypeDescriptor::String, // Could be char in future
        _ => {
            if !iterable_type.is_unresolved() {
                add_error_with_help(state, format!("'for' loops over arrays and strings, but this is neither — it is just {}", iterable_type.describe()), collection_op_help("for", iterable), &mut iterable.code_span());
            }
            NailDataTypeDescriptor::FailedToResolve
        }
    };
    
    // Add the iterator variable to the loop scope
    if let Some(scope_data) = state.scope_arena.scopes.get_mut(loop_scope) {
        scope_data.symbols.insert(
            iterator.to_string(),
            Symbol {
                name: iterator.to_string(),
                data_type: iterator_type,
                is_used: true, // Mark as used since it's a loop variable
            },
        );
    }
    
    // Update the body's scope to the loop scope
    update_node_scope(body, loop_scope);
    
    // Loops that can collect values need return statements
    let prev_context = state.return_context.clone();
    state.return_context = ReturnContext::CollectionExpression;
    
    // Visit the body
    visit_node(body, state);
    
    // Restore the original context
    state.return_context = prev_context;
    
    // NOTE: We don't pop the loop scope here because type checking happens later
    // and needs access to all scopes. Scopes will be preserved throughout
    // the entire type checking process.
}

fn visit_filter_expression(iterator: &str, index_iterator: &Option<String>, iterable: &mut ASTNode, body: &mut ASTNode, state: &mut AnalyzerState, _scope: usize) {
    // First visit the iterable
    visit_node(iterable, state);
    
    // Create a new scope for the filter
    let filter_scope = state.scope_arena.push_scope();
    
    // Get the type of the iterable
    let iterable_type = check_type(iterable, state);
    
    // Determine the iterator type
    let iterator_type = match &iterable_type {
        NailDataTypeDescriptor::Array(element_type) => (**element_type).clone(),
        _ => {
            if !iterable_type.is_unresolved() {
                add_error_with_help(state, format!("'filter' works on arrays of values, but this is not an array — it is just {}", iterable_type.describe()), collection_op_help("filter", iterable), &mut iterable.code_span());
            }
            NailDataTypeDescriptor::FailedToResolve
        }
    };
    
    // Add iterator variables to scope
    if let Some(scope_data) = state.scope_arena.scopes.get_mut(filter_scope) {
        scope_data.symbols.insert(
            iterator.to_string(),
            Symbol {
                name: iterator.to_string(),
                data_type: iterator_type,
                is_used: true,
            },
        );
        
        if let Some(idx_name) = index_iterator {
            scope_data.symbols.insert(
                idx_name.clone(),
                Symbol {
                    name: idx_name.clone(),
                    data_type: NailDataTypeDescriptor::Int,
                    is_used: true,
                },
            );
        }
    }
    
    // Update body scope and visit
    update_node_scope(body, filter_scope);
    
    // Set return context for collection operation
    let prev_context = state.return_context.clone();
    state.return_context = ReturnContext::CollectionExpression;
    
    visit_node(body, state);
    
    // Restore the original context
    state.return_context = prev_context;
    
    // Check that body returns boolean
    let body_type = check_type(body, state);
    if body_type != NailDataTypeDescriptor::Boolean && !body_type.is_unresolved() {
        add_error(state, format!("The body of 'filter' must return a boolean (whether to keep each item), but it returns {}", body_type.describe()), &mut body.code_span());
    }
    
    // NOTE: We don't pop the scope here because type checking happens later
    // and needs access to all scopes. Scopes will be preserved throughout
    // the entire type checking process.
}

fn visit_reduce_expression(accumulator: &str, iterator: &str, index_iterator: &Option<String>, iterable: &mut ASTNode, initial_value: &mut ASTNode, body: &mut ASTNode, state: &mut AnalyzerState, _scope: usize) {
    // Visit the iterable and initial value
    visit_node(iterable, state);
    visit_node(initial_value, state);
    
    // Create a new scope for the reduce
    let reduce_scope = state.scope_arena.push_scope();
    
    // Get types
    let iterable_type = check_type(iterable, state);
    let accumulator_type = check_type(initial_value, state);
    
    // Determine the iterator type
    let iterator_type = match &iterable_type {
        NailDataTypeDescriptor::Array(element_type) => (**element_type).clone(),
        _ => {
            if !iterable_type.is_unresolved() {
                add_error_with_help(state, format!("'reduce' works on arrays of values, but this is not an array — it is just {}", iterable_type.describe()), collection_op_help("reduce", iterable), &mut iterable.code_span());
            }
            NailDataTypeDescriptor::FailedToResolve
        }
    };
    
    // Add variables to scope
    if let Some(scope_data) = state.scope_arena.scopes.get_mut(reduce_scope) {
        scope_data.symbols.insert(
            accumulator.to_string(),
            Symbol {
                name: accumulator.to_string(),
                data_type: accumulator_type.clone(),
                is_used: true,
            },
        );
        
        scope_data.symbols.insert(
            iterator.to_string(),
            Symbol {
                name: iterator.to_string(),
                data_type: iterator_type,
                is_used: true,
            },
        );
        
        if let Some(idx_name) = index_iterator {
            scope_data.symbols.insert(
                idx_name.clone(),
                Symbol {
                    name: idx_name.clone(),
                    data_type: NailDataTypeDescriptor::Int,
                    is_used: true,
                },
            );
        }
    }
    
    // Update body scope
    update_node_scope(body, reduce_scope);
    
    // Set context to allow return statements
    let prev_context = state.return_context.clone();
    state.return_context = ReturnContext::CollectionExpression;
    
    visit_node(body, state);
    
    // Restore context
    state.return_context = prev_context;
    
    // NOTE: We don't pop the scope here because type checking happens later
    // and needs access to all scopes. Scopes will be preserved throughout
    // the entire type checking process.
}

fn visit_each_expression(iterator: &str, index_iterator: &Option<String>, iterable: &mut ASTNode, body: &mut ASTNode, state: &mut AnalyzerState, _scope: usize) {
    // Visit the iterable
    visit_node(iterable, state);
    
    // Create a new scope
    let each_scope = state.scope_arena.push_scope();
    
    // Get the type of the iterable
    let iterable_type = check_type(iterable, state);
    
    // Determine the iterator type
    let iterator_type = match &iterable_type {
        NailDataTypeDescriptor::Array(element_type) => (**element_type).clone(),
        _ => {
            if !iterable_type.is_unresolved() {
                add_error_with_help(state, format!("'each' works on arrays of values, but this is not an array — it is just {}", iterable_type.describe()), collection_op_help("each", iterable), &mut iterable.code_span());
            }
            NailDataTypeDescriptor::FailedToResolve
        }
    };
    
    // Add iterator variables to scope
    if let Some(scope_data) = state.scope_arena.scopes.get_mut(each_scope) {
        scope_data.symbols.insert(
            iterator.to_string(),
            Symbol {
                name: iterator.to_string(),
                data_type: iterator_type,
                is_used: true,
            },
        );
        
        if let Some(idx_name) = index_iterator {
            scope_data.symbols.insert(
                idx_name.clone(),
                Symbol {
                    name: idx_name.clone(),
                    data_type: NailDataTypeDescriptor::Int,
                    is_used: true,
                },
            );
        }
    }
    
    // Update body scope and visit
    update_node_scope(body, each_scope);
    visit_node(body, state);
    
    // NOTE: We don't pop the scope here because type checking happens later
    // and needs access to all scopes. Scopes will be preserved throughout
    // the entire type checking process.
}

fn visit_find_expression(iterator: &str, index_iterator: &Option<String>, iterable: &mut ASTNode, body: &mut ASTNode, state: &mut AnalyzerState, _scope: usize) {
    // Visit the iterable
    visit_node(iterable, state);
    
    // Create a new scope
    let find_scope = state.scope_arena.push_scope();
    
    // Get the type of the iterable
    let iterable_type = check_type(iterable, state);
    
    // Determine the iterator type
    let iterator_type = match &iterable_type {
        NailDataTypeDescriptor::Array(element_type) => (**element_type).clone(),
        _ => {
            if !iterable_type.is_unresolved() {
                add_error_with_help(state, format!("'find' works on arrays of values, but this is not an array — it is just {}", iterable_type.describe()), collection_op_help("find", iterable), &mut iterable.code_span());
            }
            NailDataTypeDescriptor::FailedToResolve
        }
    };
    
    // Add iterator variables to scope
    if let Some(scope_data) = state.scope_arena.scopes.get_mut(find_scope) {
        scope_data.symbols.insert(
            iterator.to_string(),
            Symbol {
                name: iterator.to_string(),
                data_type: iterator_type,
                is_used: true,
            },
        );
        
        if let Some(idx_name) = index_iterator {
            scope_data.symbols.insert(
                idx_name.clone(),
                Symbol {
                    name: idx_name.clone(),
                    data_type: NailDataTypeDescriptor::Int,
                    is_used: true,
                },
            );
        }
    }
    
    // Update body scope and visit
    update_node_scope(body, find_scope);
    
    // Set return context for collection operation
    let prev_context = state.return_context.clone();
    state.return_context = ReturnContext::CollectionExpression;
    
    visit_node(body, state);
    
    // Restore the original context
    state.return_context = prev_context;
    
    // Check that body returns boolean
    let body_type = check_type(body, state);
    if body_type != NailDataTypeDescriptor::Boolean && !body_type.is_unresolved() {
        add_error(state, format!("The body of 'find' must return a boolean (whether each item is the one to find), but it returns {}", body_type.describe()), &mut body.code_span());
    }
    
    // NOTE: We don't pop the scope here because type checking happens later
    // and needs access to all scopes. Scopes will be preserved throughout
    // the entire type checking process.
}

fn visit_all_expression(iterator: &str, index_iterator: &Option<String>, iterable: &mut ASTNode, body: &mut ASTNode, state: &mut AnalyzerState, _scope: usize) {
    // Visit the iterable
    visit_node(iterable, state);
    
    // Create a new scope
    let all_scope = state.scope_arena.push_scope();
    
    // Get the type of the iterable
    let iterable_type = check_type(iterable, state);
    
    // Determine the iterator type
    let iterator_type = match &iterable_type {
        NailDataTypeDescriptor::Array(element_type) => (**element_type).clone(),
        _ => {
            if !iterable_type.is_unresolved() {
                add_error_with_help(state, format!("'all' works on arrays of values, but this is not an array — it is just {}", iterable_type.describe()), collection_op_help("all", iterable), &mut iterable.code_span());
            }
            NailDataTypeDescriptor::FailedToResolve
        }
    };
    
    // Add iterator variables to scope
    if let Some(scope_data) = state.scope_arena.scopes.get_mut(all_scope) {
        scope_data.symbols.insert(
            iterator.to_string(),
            Symbol {
                name: iterator.to_string(),
                data_type: iterator_type,
                is_used: true,
            },
        );
        
        if let Some(idx_name) = index_iterator {
            scope_data.symbols.insert(
                idx_name.clone(),
                Symbol {
                    name: idx_name.clone(),
                    data_type: NailDataTypeDescriptor::Int,
                    is_used: true,
                },
            );
        }
    }
    
    // Update body scope and visit
    update_node_scope(body, all_scope);
    
    // Set return context for collection operation
    let prev_context = state.return_context.clone();
    state.return_context = ReturnContext::CollectionExpression;
    
    visit_node(body, state);
    
    // Restore the original context
    state.return_context = prev_context;
    
    // Check that body returns boolean
    let body_type = check_type(body, state);
    if body_type != NailDataTypeDescriptor::Boolean && !body_type.is_unresolved() {
        add_error(state, format!("The body of 'all' must return a boolean (whether each item passes the check), but it returns {}", body_type.describe()), &mut body.code_span());
    }
    
    // NOTE: We don't pop the scope here because type checking happens later
    // and needs access to all scopes. Scopes will be preserved throughout
    // the entire type checking process.
}

fn visit_any_expression(iterator: &str, index_iterator: &Option<String>, iterable: &mut ASTNode, body: &mut ASTNode, state: &mut AnalyzerState, _scope: usize) {
    // Visit the iterable
    visit_node(iterable, state);
    
    // Create a new scope
    let any_scope = state.scope_arena.push_scope();
    
    // Get the type of the iterable
    let iterable_type = check_type(iterable, state);
    
    // Determine the iterator type
    let iterator_type = match &iterable_type {
        NailDataTypeDescriptor::Array(element_type) => (**element_type).clone(),
        _ => {
            if !iterable_type.is_unresolved() {
                add_error_with_help(state, format!("'any' works on arrays of values, but this is not an array — it is just {}", iterable_type.describe()), collection_op_help("any", iterable), &mut iterable.code_span());
            }
            NailDataTypeDescriptor::FailedToResolve
        }
    };
    
    // Add iterator variables to scope
    if let Some(scope_data) = state.scope_arena.scopes.get_mut(any_scope) {
        scope_data.symbols.insert(
            iterator.to_string(),
            Symbol {
                name: iterator.to_string(),
                data_type: iterator_type,
                is_used: true,
            },
        );
        
        if let Some(idx_name) = index_iterator {
            scope_data.symbols.insert(
                idx_name.clone(),
                Symbol {
                    name: idx_name.clone(),
                    data_type: NailDataTypeDescriptor::Int,
                    is_used: true,
                },
            );
        }
    }
    
    // Update body scope and visit
    update_node_scope(body, any_scope);
    
    // Set return context for collection operation
    let prev_context = state.return_context.clone();
    state.return_context = ReturnContext::CollectionExpression;
    
    visit_node(body, state);
    
    // Restore the original context
    state.return_context = prev_context;
    
    // Check that body returns boolean
    let body_type = check_type(body, state);
    if body_type != NailDataTypeDescriptor::Boolean && !body_type.is_unresolved() {
        add_error(state, format!("The body of 'any' must return a boolean (whether each item passes the check), but it returns {}", body_type.describe()), &mut body.code_span());
    }
    
    // NOTE: We don't pop the scope here because type checking happens later
    // and needs access to all scopes. Scopes will be preserved throughout
    // the entire type checking process.
}


fn visit_while_loop(condition: &mut ASTNode, initial_value: &mut Option<Box<ASTNode>>, body: &mut ASTNode, max_iterations: &mut Option<Box<ASTNode>>, state: &mut AnalyzerState) {
    // Mark that we're in a loop context for break/continue validation
    let previous_in_loop = state.in_loop;
    state.in_loop = true;
    
    // Visit the condition
    visit_node(condition, state);
    
    // Check that condition is boolean
    let condition_type = check_type(condition, state);
    if condition_type != NailDataTypeDescriptor::Boolean && !condition_type.is_unresolved() {
        add_error(state, format!("The 'while' condition must be a boolean, but it is {}", condition_type.describe()), &mut condition.code_span());
    }
    
    // Visit initial value if present
    if let Some(init) = initial_value {
        visit_node(init, state);
    }
    
    // Visit max iterations if present
    if let Some(max_iter) = max_iterations {
        visit_node(max_iter, state);
        
        // Check that max iterations is an integer
        let max_type = check_type(max_iter, state);
        if max_type != NailDataTypeDescriptor::Int && !max_type.is_unresolved() {
            add_error(state, format!("The 'while' loop's maximum iteration count must be an integer, but it is {}", max_type.describe()), &mut max_iter.code_span());
        }
    }
    
    // Visit the body
    visit_node(body, state);
    
    // Restore previous loop context
    state.in_loop = previous_in_loop;
}

fn visit_loop(index_iterator: &Option<String>, body: &mut ASTNode, state: &mut AnalyzerState, _scope: usize) {
    // Mark that we're in a loop context for break/continue validation
    let previous_in_loop = state.in_loop;
    state.in_loop = true;
    
    // Create a new scope for the loop body if there's an index iterator
    if index_iterator.is_some() {
        let loop_scope = state.scope_arena.push_scope();
        update_node_scope(body, loop_scope);
        
        // Add the index iterator to the scope
        if let Some(idx_name) = index_iterator {
            let scope_data = state.scope_arena.get_scope_mut(loop_scope);
            scope_data.symbols.insert(
                idx_name.clone(),
                Symbol {
                    name: idx_name.clone(),
                    data_type: NailDataTypeDescriptor::Int,
                    is_used: false,
                },
            );
        }
    }
    
    // Visit the body
    visit_node(body, state);
    
    // Pop the scope if we created one
    if index_iterator.is_some() {
        let _ = state.scope_arena.pop_scope();
    }
    
    // Restore previous loop context
    state.in_loop = previous_in_loop;
}

fn visit_spawn_block(body: &mut ASTNode, state: &mut AnalyzerState) {
    // Visit the body - spawn blocks run asynchronously
    visit_node(body, state);
}

fn visit_break_statement(state: &mut AnalyzerState, code_span: &CodeSpan) {
    // Check if we're in a loop
    if !state.in_loop {
        add_error(state, "break statement can only be used inside a loop".to_string(), &mut code_span.clone());
    }
}

fn visit_continue_statement(state: &mut AnalyzerState, code_span: &CodeSpan) {
    // Check if we're in a loop
    if !state.in_loop {
        add_error(state, "continue statement can only be used inside a loop".to_string(), &mut code_span.clone());
    }
}

fn visit_struct_field_access(struct_name: &str, field_name: &str, state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    // Check if the struct variable exists
    if !mark_symbol_as_used(state, struct_name) {
        let help = best_suggestion(struct_name, visible_symbol_names(state).iter().map(|s| s.as_str())).map(|suggestion| format!("did you mean '{}'?", suggestion));
        add_error_with_help(state, format!("There is no variable named '{}' in this scope", struct_name), help, code_span);
        return;
    }

    // Get the struct type from the symbol table
    let symbol_type = lookup_symbol(&state.scope_arena, state.scope_arena.current_scope(), struct_name).map(|s| s.data_type.clone());
    if let Some(data_type) = symbol_type {
        match &data_type {
            NailDataTypeDescriptor::Struct(struct_type_name) => {
                // Check if the struct type exists
                if let Some(struct_fields) = state.structs.get(struct_type_name) {
                    // Check if the field exists in the struct
                    let field_exists = struct_fields.iter().any(|field| if let ASTNode::StructDeclarationField { name, .. } = field { name == field_name } else { false });

                    if !field_exists {
                        add_error(state, format!("Field '{}' does not exist in struct '{}'", field_name, struct_type_name), code_span);
                    }
                } else {
                    // Check if it's a stdlib struct
                    if !crate::stdlib_registry::is_stdlib_struct(struct_type_name) {
                        add_error(state, format!("There is no struct named '{}' defined anywhere in this program", struct_type_name), code_span);
                    } else {
                        // For stdlib structs, check if the field exists using the registry
                        if crate::stdlib_registry::get_stdlib_struct_field_type(struct_type_name, field_name).is_none() {
                            add_error(state, format!("Field '{}' does not exist in stdlib struct '{}'", field_name, struct_type_name), code_span);
                        }
                    }
                }
            }
            _ => {
                if !data_type.is_unresolved() {
                    add_error(state, format!("Cannot access field '{}' here: '{}' is {}, not a struct", field_name, struct_name, data_type.describe()), code_span);
                }
            }
        }
    }
}

fn visit_nested_field_access(object: &mut Box<ASTNode>, field_name: &str, state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    // First visit the object to check its validity
    visit_node(object, state);

    // Get the type of the object
    let object_type = check_type(object, state);

    match &object_type {
        NailDataTypeDescriptor::Struct(struct_type_name) => {
            // Check if the struct type exists
            if let Some(struct_fields) = state.structs.get(struct_type_name) {
                // Check if the field exists in the struct
                let field_exists = struct_fields.iter().any(|field| if let ASTNode::StructDeclarationField { name, .. } = field { name == field_name } else { false });

                if !field_exists {
                    add_error(state, format!("Field '{}' does not exist in struct '{}'", field_name, struct_type_name), code_span);
                }
            } else {
                // Check if it's a stdlib struct
                if !crate::stdlib_registry::is_stdlib_struct(struct_type_name) {
                    add_error(state, format!("There is no struct named '{}' defined anywhere in this program", struct_type_name), code_span);
                } else {
                    // For stdlib structs, check if the field exists using the registry
                    if crate::stdlib_registry::get_stdlib_struct_field_type(struct_type_name, field_name).is_none() {
                        add_error(state, format!("Field '{}' does not exist in stdlib struct '{}'", field_name, struct_type_name), code_span);
                    }
                }
            }
        }
        _ => {
            if !object_type.is_unresolved() {
                add_error(state, format!("Cannot access field '{}' here: this value is {}, not a struct", field_name, object_type.describe()), code_span);
            }
        }
    }
}

fn visit_struct_declaration(name: &str, fields: &[ASTNode], state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    // Check if this struct name conflicts with stdlib types
    let stdlib_types = crate::stdlib_registry::get_stdlib_type_names();
    if stdlib_types.contains(name) {
        add_error(state, format!("Cannot define struct '{}': this name is reserved for a standard library type", name), code_span);
        return;
    }

    if RESERVED_RUST_TYPE_NAMES.contains(&name) {
        add_error(state, format!("Cannot define struct '{}': this name collides with a built-in type used by generated code", name), code_span);
        return;
    }

    if state.enum_variants.contains_key(name) {
        add_error(state, format!("Cannot define struct '{}': an enum with this name is already defined", name), code_span);
        return;
    }

    // Register the struct in the type system. Any redefinition is an error, even
    // an identical one: the transpiler emits one Rust struct per declaration, so
    // two declarations produce duplicate type definitions.
    if state.structs.contains_key(name) {
        add_error(state, format!("Struct '{}' is already defined", name), code_span);
    } else {
        state.structs.insert(name.to_string(), fields.to_vec());
        state.struct_spans.insert(name.to_string(), code_span.clone());
    }

    // Reject duplicate field names within the declaration
    let mut seen_fields = HashSet::new();
    for field in fields {
        if let ASTNode::StructDeclarationField { name: field_name, .. } = field {
            if !seen_fields.insert(field_name.clone()) {
                add_error(state, format!("Duplicate field '{}' in struct '{}'", field_name, name), code_span);
            }
        }
    }

    // Nested structs are now allowed - just validate they exist
    for field in fields {
        if let ASTNode::StructDeclarationField { name: _field_name, data_type, .. } = field {
            match data_type {
                NailDataTypeDescriptor::Struct(_struct_name) => {
                    // Allow forward references and self-references for now
                    // More sophisticated checking could be added later
                }
                NailDataTypeDescriptor::Enum(_enum_name) => {
                    // Allow forward references for now
                }
                NailDataTypeDescriptor::Int | NailDataTypeDescriptor::Float | NailDataTypeDescriptor::String | NailDataTypeDescriptor::Boolean => {
                    // Basic types are always valid
                }
                NailDataTypeDescriptor::Array(_) => {
                    // Array types are valid
                }
                NailDataTypeDescriptor::Any => {
                    // Any type is valid
                }
                _ => {
                    panic!("visit_struct_declaration: unhandled field type: {:?}", data_type);
                }
            }
        }
    }
}

fn visit_struct_instantiation(name: &str, fields: &mut [ASTNode], state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    for field in fields.iter_mut() {
        visit_node(field, state);
    }

    // Resolve the declared field list from the user struct or the stdlib registry
    let declared: Vec<(String, NailDataTypeDescriptor)> = if let Some(struct_fields) = state.structs.get(name) {
        struct_fields
            .iter()
            .filter_map(|f| if let ASTNode::StructDeclarationField { name, data_type, .. } = f { Some((name.clone(), data_type.clone())) } else { None })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(field_name, field_type)| (field_name, resolve_custom_type(state, &field_type)))
            .collect()
    } else if let Some(stdlib_fields) = crate::stdlib_registry::get_stdlib_struct_fields(name) {
        stdlib_fields
    } else {
        add_error(state, format!("There is no struct named '{}' defined anywhere in this program", name), code_span);
        return;
    };

    let mut seen = HashSet::new();
    for field in fields.iter() {
        if let ASTNode::StructInstantiationField { name: field_name, value, code_span: field_span, .. } = field {
            if !seen.insert(field_name.clone()) {
                add_error(state, format!("Duplicate field '{}' in instantiation of struct '{}'", field_name, name), &mut field_span.clone());
                continue;
            }
            match declared.iter().find(|(declared_name, _)| declared_name == field_name) {
                None => {
                    add_error(state, format!("Field '{}' does not exist in struct '{}'", field_name, name), &mut field_span.clone());
                }
                Some((_, declared_type)) => {
                    let value_type = check_type(value, state);
                    if is_concrete_type(declared_type) && is_concrete_type(&value_type) && *declared_type != value_type && value_type != NailDataTypeDescriptor::Any {
                        add_error(
                            state,
                            format!("Field '{}' of struct '{}' is declared as {} but its value is {}", field_name, name, declared_type.describe(), value_type.describe()),
                            &mut field_span.clone(),
                        );
                    }
                }
            }
        }
    }

    for (declared_name, _) in &declared {
        if !seen.contains(declared_name) {
            add_error(state, format!("Missing field '{}' in instantiation of struct '{}'", declared_name, name), code_span);
        }
    }
}

/// Structs that contain themselves through a chain of direct struct fields have
/// infinite size in the generated Rust. Arrays and hashmaps introduce heap
/// indirection, so only direct struct-typed fields form cycle edges.
fn check_recursive_structs(state: &mut AnalyzerState) {
    let edges: HashMap<String, Vec<String>> = state
        .structs
        .iter()
        .map(|(name, fields)| {
            let targets = fields
                .iter()
                .filter_map(|f| {
                    if let ASTNode::StructDeclarationField { data_type: NailDataTypeDescriptor::Struct(target), .. } = f {
                        Some(target.clone())
                    } else {
                        None
                    }
                })
                .collect();
            (name.clone(), targets)
        })
        .collect();

    let mut reported = HashSet::new();
    for start in edges.keys() {
        if reported.contains(start) {
            continue;
        }
        // Depth-first walk from `start`; a path back to `start` means its size is infinite
        let mut stack: Vec<&String> = vec![start];
        let mut visited = HashSet::new();
        while let Some(current) = stack.pop() {
            for target in edges.get(current).into_iter().flatten() {
                if target == start {
                    reported.insert(start.clone());
                    let mut span = state.struct_spans.get(start).cloned().unwrap_or_default();
                    add_error(
                        state,
                        format!("Struct '{}' contains itself (directly or through other structs), which would make it infinitely sized; store it in an array field or restructure the types", start),
                        &mut span,
                    );
                    stack.clear();
                    break;
                }
                if visited.insert(target.clone()) {
                    if let Some(target_key) = edges.get_key_value(target) {
                        stack.push(target_key.0);
                    }
                }
            }
        }
    }
}

fn visit_enum_declaration(name: &str, variants: &[ASTNode], state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    // Check if this enum name conflicts with stdlib types
    let stdlib_types = crate::stdlib_registry::get_stdlib_type_names();
    if stdlib_types.contains(name) {
        add_error(state, format!("Cannot define enum '{}': this name is reserved for a standard library type", name), code_span);
        return;
    }

    if RESERVED_RUST_TYPE_NAMES.contains(&name) {
        add_error(state, format!("Cannot define enum '{}': this name collides with a built-in type used by generated code", name), code_span);
        return;
    }

    if state.structs.contains_key(name) {
        add_error(state, format!("Cannot define enum '{}': a struct with this name is already defined", name), code_span);
        return;
    }

    if state.enum_variants.contains_key(name) {
        add_error(state, format!("Enum '{}' is already defined", name), code_span);
        return;
    }

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
    // First, visit all elements to ensure they're properly typed
    for element in elements.iter() {
        let mut element_clone = element.clone();
        visit_node(&mut element_clone, state);
    }

    // Empty arrays are allowed now - no type checking needed
    if elements.is_empty() {
        return;
    }

    // Now check that all elements have the same type
    let first_type = check_type(&elements[0], state);
    
    // Skip checking if first type couldn't be resolved
    if matches!(first_type, NailDataTypeDescriptor::OneOf(ref v) if v.is_empty()) {
        return;
    }
    
    elements.iter().skip(1).for_each(|element| {
        let element_type = check_type(element, state);
        // Skip checking if element type couldn't be resolved
        if matches!(element_type, NailDataTypeDescriptor::OneOf(ref v) if v.is_empty()) {
            return;
        }
        if element_type != first_type && element_type != NailDataTypeDescriptor::Any && first_type != NailDataTypeDescriptor::Any {
            // Only report error if both types were properly resolved
            if !matches!(first_type, NailDataTypeDescriptor::OneOf(ref v) if v.is_empty()) &&
               !matches!(element_type, NailDataTypeDescriptor::OneOf(ref v) if v.is_empty()) {
                add_error(state, format!("All elements of an array must have the same type: this array starts with {}, but a later element is {}", first_type.describe(), element_type.describe()), code_span);
            }
        }
    });
}

fn visit_function_call(name: &str, args: &mut [ASTNode], state: &mut AnalyzerState, call_scope: usize, code_span: &mut CodeSpan) -> NailDataTypeDescriptor {
    // The error constructor e(message) builds an error value for early returns
    if crate::stdlib_registry::get_intrinsic(name) == Some(crate::stdlib_registry::Intrinsic::ErrorConstructor) {
        if args.len() != 1 {
            add_error_with_help(
                state,
                format!("'e' expects exactly 1 argument (the error message), got {}", args.len()),
                Some("construct an error like: r e(`something went wrong`);".to_string()),
                code_span,
            );
        }
        for arg in args.iter_mut() {
            visit_node(arg, state);
        }
        return NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Error));
    }

    // Special handling for print function (variable arguments)
    if name == "print" {
        if args.is_empty() {
            add_error(state, "print expects at least 1 argument, got 0".to_string(), code_span);
        }
        // Type check all arguments (print accepts any type)
        for arg in args.iter_mut() {
            visit_node(arg, state);
        }
        return NailDataTypeDescriptor::Void;
    }

    // Special handling for safe function to enforce error handler parameter types
    if name == "safe" {
        if args.len() != 2 {
            add_error(state, format!("safe expects 2 arguments, got {}", args.len()), code_span);
            return NailDataTypeDescriptor::FailedToResolve;
        }

        // Visit both arguments
        for arg in args.iter_mut() {
            visit_node(arg, state);
        }

        // Check that the second argument (error handler) accepts error type
        if let ASTNode::LambdaDeclaration { params, .. } = &args[1] {
            if params.len() == 1 {
                let (_, param_type) = &params[0];
                if *param_type != NailDataTypeDescriptor::Error {
                    add_error(state, format!("The error handler passed to 'safe' must take a parameter of type :e (an error), but this parameter is {}", param_type.describe()), code_span);
                }
            }
        } else if let ASTNode::Identifier { name: handler_name, .. } = &args[1] {
            // Check if it's a function that takes error type
            let handler_type = lookup_symbol(&state.scope_arena, state.scope_arena.current_scope(), handler_name).map(|s| s.data_type.clone());
            if let Some(NailDataTypeDescriptor::Fn(param_types, _)) = handler_type {
                if param_types.len() == 1 && param_types[0] != NailDataTypeDescriptor::Error {
                    add_error(state, format!("The error handler '{}' passed to 'safe' must accept a parameter of type :e (an error), but its parameter is {}", handler_name, param_types[0].describe()), code_span);
                }
            }
        }

        // Return the inner type of the first argument (Result type)
        let first_arg_type = check_type(&args[0], state);
        return match first_arg_type {
            NailDataTypeDescriptor::Result(inner) => (*inner).clone(),
            _ => NailDataTypeDescriptor::FailedToResolve,
        };
    }

    // Check if this is a stdlib function first
    if let Some(func_type) = get_stdlib_function(name) {
        // Check argument count
        if args.len() != func_type.parameters.len() {
            add_error(state, format!("{} expects {} arguments, got {}", name, func_type.parameters.len(), args.len()), code_span);
        }

        // Type check each argument against expected parameter types, unifying
        // any type variables (e.g. array_first(a:T) -> T!e) along the way
        let mut type_var_bindings: HashMap<String, NailDataTypeDescriptor> = HashMap::new();
        for (i, arg) in args.iter_mut().enumerate() {
            visit_node(arg, state);

            // Check if we have a parameter definition for this argument
            if let Some(expected_param) = func_type.parameters.get(i) {
                let actual_type = check_type(arg, state);
                // Skip type checking if actual type is FailedToResolve (failed to infer)
                if actual_type != NailDataTypeDescriptor::FailedToResolve && !unify_types(&expected_param.param_type, &actual_type, &mut type_var_bindings) {
                    // Show type variables as whatever earlier arguments bound them
                    // to, so the message names a concrete type instead of 'T'
                    let expected_display = substitute_bound_type_vars(&expected_param.param_type, &type_var_bindings);
                    add_error(
                        state,
                        format!("'{}' expects its parameter '{}' to be {}, but this argument is {}", name, expected_param.name, expected_display.describe(), actual_type.describe()),
                        code_span,
                    );
                }
            }
        }

        // Resolve type variables in the return type from the argument bindings;
        // unbound variables become Any and the declared type at the call site wins
        return substitute_type_vars(&func_type.return_type, &type_var_bindings);
    }

    let symbol_type = match lookup_symbol(&state.scope_arena, call_scope, name).map(|s| s.data_type.clone()) {
        Some(t) => t,
        None => {
            let visible = visible_symbol_names(state);
            let candidates = crate::stdlib_registry::STDLIB_FUNCTIONS.keys().copied().chain(visible.iter().map(|s| s.as_str()));
            let help = best_suggestion(name, candidates).map(|suggestion| format!("did you mean '{}'?", suggestion));
            add_error_with_help(state, format!("There is no function named '{}'", name), help, code_span);
            return NailDataTypeDescriptor::FailedToResolve;
        }
    };

    match &symbol_type {
        NailDataTypeDescriptor::Fn(param_types, return_type) => {
            if param_types.len() != args.len() {
                add_error(state, format!("Function '{}' called with wrong number of arguments. Expected {}, got {}", name, param_types.len(), args.len()), code_span);
                return NailDataTypeDescriptor::FailedToResolve;
            }

            let mut has_error = false;
            for (i, (arg, expected_type)) in args.iter().zip(param_types.iter()).enumerate() {
                let arg_type = check_type(arg, state);
                if arg_type != *expected_type {
                    add_error(state, format!("Argument {} of '{}' should be {}, but this argument is {}", i + 1, name, expected_type.describe(), arg_type.describe()), code_span);
                    has_error = true;
                }
            }

            if has_error {
                NailDataTypeDescriptor::FailedToResolve
            } else {
                (**return_type).clone()
            }
        }
        _ => {
            add_error(state, format!("'{}' is not a function", name), code_span);
            NailDataTypeDescriptor::FailedToResolve
        }
    }
}

fn visit_return_declaration(expr: &mut ASTNode, state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    match state.return_context {
        ReturnContext::None => {
            add_error(state, "Return statement outside of function or collection operation".to_string(), code_span);
        }
        ReturnContext::Function => {
            // Normal function return - visit the expression so errors inside it
            // (undefined variables, scope violations, ...) are reported, then
            // type check it
            visit_node(expr, state);
            check_type(expr, state);
        }
        ReturnContext::CollectionExpression => {
            // Return in collection operation should be an error - use yield instead
            add_error(state, "Use 'y' (yield) instead of 'r' (return) in collection operations".to_string(), code_span);
        }
    }
}

fn visit_yield_declaration(expr: &ASTNode, state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    match state.return_context {
        ReturnContext::None => {
            add_error(state, "Yield statement outside of collection operation".to_string(), code_span);
        }
        ReturnContext::Function => {
            add_error_with_help(
                state,
                "'y' (yield) cannot be used here — it is only valid inside collection operation blocks (map, filter, reduce, ...)".to_string(),
                Some("use 'r' (return) to produce a value from this block".to_string()),
                code_span,
            );
        }
        ReturnContext::CollectionExpression => {
            // Collection yield - type check the expression
            check_type(expr, state);
        }
    }
}

fn find_return_types_in_node(node: &ASTNode, state: &AnalyzerState) -> NailDataTypeDescriptor {
    let mut return_types = Vec::new();
    collect_return_types(node, &mut return_types, state);
    
    if return_types.is_empty() {
        // No return types found, return OneOf<> to match the error message
        NailDataTypeDescriptor::OneOf(vec![])
    } else if return_types.len() == 1 {
        return_types.into_iter().next().unwrap()
    } else {
        NailDataTypeDescriptor::OneOf(return_types)
    }
}

fn collect_return_types(node: &ASTNode, return_types: &mut Vec<NailDataTypeDescriptor>, state: &AnalyzerState) {
    match node {
        ASTNode::ReturnDeclaration { statement, .. } => {
            let return_type = check_type(statement, state);
            if !return_types.contains(&return_type) {
                return_types.push(return_type);
            }
        }
        ASTNode::YieldDeclaration { statement, .. } => {
            let return_type = check_type(statement, state);
            if !return_types.contains(&return_type) {
                return_types.push(return_type);
            }
        }
        ASTNode::Block { statements, .. } => {
            for stmt in statements {
                collect_return_types(stmt, return_types, state);
            }
        }
        ASTNode::IfStatement { condition_branches, else_branch, .. } => {
            for (_, branch_body) in condition_branches {
                collect_return_types(branch_body, return_types, state);
            }
            if let Some(else_body) = else_branch {
                collect_return_types(else_body, return_types, state);
            }
        }
        // Don't recurse into nested functions or loops
        ASTNode::ForLoop { .. } | ASTNode::WhileLoop { .. } | ASTNode::MapExpression { .. } => {}
        _ => {
            panic!("collect_return_types: unhandled node type");
        }
    }
}

fn check_type(node: &ASTNode, state: &AnalyzerState) -> NailDataTypeDescriptor {
    match node {
        ASTNode::NumberLiteral { data_type, .. } => data_type.clone(),
        ASTNode::StringLiteral { .. } => NailDataTypeDescriptor::String,
        ASTNode::BooleanLiteral { .. } => NailDataTypeDescriptor::Boolean,
        ASTNode::Identifier { name, .. } => lookup_symbol(&state.scope_arena, state.scope_arena.current_scope(), name).map_or(NailDataTypeDescriptor::OneOf(vec![]), |s| s.data_type.clone()),
        ASTNode::ReturnDeclaration { statement, .. } => check_type(statement, state),
        ASTNode::YieldDeclaration { statement, .. } => check_type(statement, state),
        ASTNode::UnaryOperation { operator, operand, .. } => {
            match operator {
                Operation::Not => {
                    // Not operator always returns boolean
                    let operand_type = check_type(operand, state);
                    // Check that operand is boolean
                    if operand_type != NailDataTypeDescriptor::Boolean && !operand_type.is_unresolved() {
                        // Type error will be caught elsewhere
                    }
                    NailDataTypeDescriptor::Boolean
                }
                Operation::Neg => {
                    // Negation returns the same type as operand
                    check_type(operand, state)
                }
                _ => NailDataTypeDescriptor::FailedToResolve,
            }
        }
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
                        NailDataTypeDescriptor::FailedToResolve
                    }
                }
                // Arithmetic operators return operand type
                Operation::Add | Operation::Sub | Operation::Mul | Operation::Div | Operation::Mod => {
                    if left_type == right_type {
                        left_type
                    } else {
                        NailDataTypeDescriptor::FailedToResolve
                    }
                }
                // Logical operators return Boolean
                Operation::And | Operation::Or => {
                    if left_type == right_type && left_type == NailDataTypeDescriptor::Boolean {
                        NailDataTypeDescriptor::Boolean
                    } else {
                        NailDataTypeDescriptor::FailedToResolve
                    }
                }
                // Other operators
                _ => {
                    if left_type == right_type {
                        left_type
                    } else {
                        NailDataTypeDescriptor::FailedToResolve
                    }
                }
            }
        }
        ASTNode::Program { statements, .. } => statements.last().map_or(NailDataTypeDescriptor::OneOf(vec![]), |stmt| check_type(stmt, state)),
        ASTNode::FunctionDeclaration { data_type, .. } => data_type.clone(),
        ASTNode::LambdaDeclaration { data_type, .. } => data_type.clone(),
        ASTNode::IfStatement { condition_branches, else_branch, .. } => {
            // For if expressions, check all branches and ensure they return the same type
            let mut return_type: Option<NailDataTypeDescriptor> = None;

            // Check all condition branches
            for (_, branch) in condition_branches {
                let branch_type = check_type(branch, state);
                if return_type.is_none() {
                    return_type = Some(branch_type); // First branch sets the type
                } else if let Some(ref current_type) = return_type {
                    if branch_type != *current_type && !matches!(branch_type, NailDataTypeDescriptor::OneOf(_)) {
                        // Type mismatch between branches - this is an error but we'll return the first type
                        break;
                    }
                }
            }

            // Check else branch if present
            if let Some(else_node) = else_branch {
                let else_type = check_type(else_node, state);
                if return_type.is_none() {
                    return_type = Some(else_type);
                } else if let Some(ref current_type) = return_type {
                    if else_type != *current_type && !matches!(else_type, NailDataTypeDescriptor::OneOf(_)) {
                        // Type mismatch with else branch
                        // Keep the first branch type
                    }
                }
            }

            // If this if expression has a return type (is used as a value),
            // it must have an else branch to ensure exhaustiveness
            if return_type.is_some() && return_type != Some(NailDataTypeDescriptor::Void) && else_branch.is_none() {
                // This will be caught in visit_if_statement when used in expression context
            }

            return_type.unwrap_or(NailDataTypeDescriptor::OneOf(vec![]))
        }
        ASTNode::Block { statements, .. } => {
            // For blocks, check if there's a return or yield statement
            for stmt in statements {
                if let ASTNode::ReturnDeclaration { statement, .. } = stmt {
                    return check_type(statement, state);
                }
                if let ASTNode::YieldDeclaration { statement, .. } = stmt {
                    return check_type(statement, state);
                }
            }
            // Otherwise, the type is the type of the last statement
            statements.last().map_or(NailDataTypeDescriptor::OneOf(vec![]), |stmt| check_type(stmt, state))
        }
        ASTNode::ForLoop { body, iterator, iterable, scope, .. } => {
            // For loops return the type of their body
            // First, get the type of the iterable
            let iterable_type = check_type(iterable, state);
            let element_type = match &iterable_type {
                NailDataTypeDescriptor::Array(elem_type) => (**elem_type).clone(),
                _ => NailDataTypeDescriptor::FailedToResolve,
            };
            
            // If the body contains return statements, it's collecting values
            if let ASTNode::Block { statements, .. } = body.as_ref() {
                // Look for return or yield statements in the body
                for stmt in statements {
                    if let ASTNode::ReturnDeclaration { statement, .. } = stmt {
                        // The loop will collect values of the return type
                        // For simple identifiers, check if it's the iterator
                        if let ASTNode::Identifier { name, .. } = statement.as_ref() {
                            if name == iterator {
                                return NailDataTypeDescriptor::Array(Box::new(element_type));
                            }
                        }
                        
                        // For more complex expressions, we need to temporarily add the iterator 
                        // to the scope to type check the return expression properly
                        // Since check_type is immutable, we'll use a simpler approach:
                        // If it's a binary operation involving the iterator, infer the type
                        let return_type = match statement.as_ref() {
                            ASTNode::BinaryOperation { left, operator, right, .. } => {
                                // Check if one side is the iterator
                                let left_is_iterator = matches!(left.as_ref(), ASTNode::Identifier { name, .. } if name == iterator);
                                let right_is_iterator = matches!(right.as_ref(), ASTNode::Identifier { name, .. } if name == iterator);
                                
                                if left_is_iterator || right_is_iterator {
                                    // Infer type based on operation and element type
                                    match (operator, &element_type) {
                                        (Operation::Add | Operation::Sub | Operation::Mul | Operation::Div | Operation::Mod, 
                                         NailDataTypeDescriptor::Int | NailDataTypeDescriptor::Float) => element_type.clone(),
                                        // String concatenation with + is not supported in Nail
                                        _ => NailDataTypeDescriptor::FailedToResolve,
                                    }
                                } else {
                                    check_type(statement, state)
                                }
                            }
                            _ => check_type(statement, state),
                        };
                        
                        return NailDataTypeDescriptor::Array(Box::new(return_type));
                    }
                    if let ASTNode::YieldDeclaration { statement, .. } = stmt {
                        // The loop will collect values of the yield type
                        // For simple identifiers, check if it's the iterator
                        if let ASTNode::Identifier { name, .. } = statement.as_ref() {
                            if name == iterator {
                                return NailDataTypeDescriptor::Array(Box::new(element_type));
                            }
                        }
                        
                        // For more complex expressions, we need to temporarily add the iterator 
                        // to the scope to type check the yield expression properly
                        // Since check_type is immutable, we'll use a simpler approach:
                        // If it's a binary operation involving the iterator, infer the type
                        let return_type = match statement.as_ref() {
                            ASTNode::BinaryOperation { left, operator, right, .. } => {
                                // Check if one side is the iterator
                                let left_is_iterator = matches!(left.as_ref(), ASTNode::Identifier { name, .. } if name == iterator);
                                let right_is_iterator = matches!(right.as_ref(), ASTNode::Identifier { name, .. } if name == iterator);
                                
                                if left_is_iterator || right_is_iterator {
                                    // Infer type based on operation and element type
                                    match (operator, &element_type) {
                                        (Operation::Add | Operation::Sub | Operation::Mul | Operation::Div | Operation::Mod, 
                                         NailDataTypeDescriptor::Int | NailDataTypeDescriptor::Float) => element_type.clone(),
                                        // String concatenation with + is not supported in Nail
                                        _ => NailDataTypeDescriptor::FailedToResolve,
                                    }
                                } else {
                                    check_type(statement, state)
                                }
                            }
                            _ => check_type(statement, state),
                        };
                        
                        return NailDataTypeDescriptor::Array(Box::new(return_type));
                    }
                }
            }
            // If no return statements, it's a void loop
            NailDataTypeDescriptor::Void
        }
        ASTNode::MapExpression { iterable, .. } => {
            // Map expressions always return an array
            // For proper type checking, we rely on the visit phase which has the proper scope
            // Here we just return a generic array type based on the iterable
            
            let iterable_type = check_type(iterable, state);
            match iterable_type {
                NailDataTypeDescriptor::Array(element_type) => {
                    // Map transforms elements, so we can't know the output type without analyzing the body
                    // Return Array<Any> and let the visit phase do proper type checking
                    NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Any))
                }
                _ => NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Any))
            }
        }
        ASTNode::FilterExpression { iterable, .. } => {
            // Filter returns the same array type as the input
            check_type(iterable, state)
        }
        ASTNode::ReduceExpression { initial_value, .. } => {
            // Reduce returns the type of the accumulator
            check_type(initial_value, state)
        }
        ASTNode::EachExpression { .. } => {
            // Each returns void as it's for side effects
            NailDataTypeDescriptor::Void
        }
        ASTNode::FindExpression { iterable, .. } => {
            // Find returns Result<element_type>
            let iterable_type = check_type(iterable, state);
            match iterable_type {
                NailDataTypeDescriptor::Array(element_type) => {
                    NailDataTypeDescriptor::Result(element_type)
                }
                _ => NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::FailedToResolve))
            }
        }
        ASTNode::AllExpression { .. } => {
            // All returns boolean
            NailDataTypeDescriptor::Boolean
        }
        ASTNode::AnyExpression { .. } => {
            // Any returns boolean
            NailDataTypeDescriptor::Boolean
        }
        ASTNode::WhileLoop { .. } => NailDataTypeDescriptor::Void,
        ASTNode::Loop { .. } => NailDataTypeDescriptor::Void,
        ASTNode::SpawnBlock { .. } => NailDataTypeDescriptor::Void,
        ASTNode::BreakStatement { .. } => NailDataTypeDescriptor::Never,
        ASTNode::ContinueStatement { .. } => NailDataTypeDescriptor::Never,
        ASTNode::FunctionCall { name, args, .. } => {
            // Special handling for error constructor
            if name == "e" {
                // e() always returns an error (Result with Error)
                return NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Error));
            }

            // Check if this is a stdlib function first
            if let Some(func_type) = get_stdlib_function(name) {
                // Unify type variables in the signature against the argument
                // types, then resolve them in the return type
                let mut type_var_bindings: HashMap<String, NailDataTypeDescriptor> = HashMap::new();
                for (i, arg) in args.iter().enumerate() {
                    if let Some(expected_param) = func_type.parameters.get(i) {
                        let actual_type = check_type(arg, state);
                        unify_types(&expected_param.param_type, &actual_type, &mut type_var_bindings);
                    }
                }
                return substitute_type_vars(&func_type.return_type, &type_var_bindings);
            }
            // Otherwise look up in symbol table
            lookup_symbol(&state.scope_arena, state.scope_arena.current_scope(), name).map_or(NailDataTypeDescriptor::OneOf(vec![]), |s| match &s.data_type {
                NailDataTypeDescriptor::Fn(_, return_type) => (**return_type).clone(),
                _ => s.data_type.clone(),
            })
        }
        ASTNode::ConstDeclaration { data_type, .. } => data_type.clone(),
        ASTNode::StructDeclarationField { data_type, .. } => data_type.clone(),
        ASTNode::ParallelBlock { statements, .. } => statements.last().map_or(NailDataTypeDescriptor::OneOf(vec![]), |stmt| check_type(stmt, state)),
        ASTNode::ConcurrentBlock { statements, .. } => statements.last().map_or(NailDataTypeDescriptor::OneOf(vec![]), |stmt| check_type(stmt, state)),
        ASTNode::StructDeclaration { name, .. } => NailDataTypeDescriptor::Struct(name.to_string()),
        ASTNode::StructInstantiation { name, .. } => NailDataTypeDescriptor::Struct(name.to_string()),
        ASTNode::StructInstantiationField { value, .. } => check_type(value, state),
        ASTNode::EnumDeclaration { name, .. } => NailDataTypeDescriptor::Enum(name.to_string()),
        ASTNode::EnumVariant { name, .. } => NailDataTypeDescriptor::Enum(name.to_string()),
        ASTNode::ArrayLiteral { elements, .. } => {
            if elements.is_empty() {
                NailDataTypeDescriptor::FailedToResolve
            } else {
                let element_type = check_type(&elements[0], state);
                NailDataTypeDescriptor::Array(Box::new(element_type))
            }
        }
        ASTNode::StructFieldAccess { struct_name, field_name, .. } => {
            // Get the struct type from the symbol table
            if let Some(symbol) = lookup_symbol(&state.scope_arena, state.scope_arena.current_scope(), struct_name) {
                match &symbol.data_type {
                    NailDataTypeDescriptor::Struct(struct_type_name) => {
                        // Find the field type in the struct definition
                        if let Some(struct_fields) = state.structs.get(struct_type_name) {
                            for field in struct_fields {
                                if let ASTNode::StructDeclarationField { name, data_type, .. } = field {
                                    if name == field_name {
                                        return data_type.clone();
                                    }
                                }
                            }
                        } else {
                            // For stdlib structs, get field type from registry
                            if let Some(field_type) = crate::stdlib_registry::get_stdlib_struct_field_type(struct_type_name, field_name) {
                                return field_type;
                            }
                        }
                    }
                    // Not a struct: visit_struct_field_access reports the error;
                    // here we just fail to resolve the field's type
                    _ => return NailDataTypeDescriptor::FailedToResolve,
                }
            }
            NailDataTypeDescriptor::FailedToResolve
        }
        ASTNode::NestedFieldAccess { object, field_name, .. } => {
            // Get the type of the object
            let object_type = check_type(object, state);

            match &object_type {
                NailDataTypeDescriptor::Struct(struct_type_name) => {
                    // Find the field type in the struct definition
                    if let Some(struct_fields) = state.structs.get(struct_type_name) {
                        for field in struct_fields {
                            if let ASTNode::StructDeclarationField { name, data_type, .. } = field {
                                if name == field_name {
                                    return data_type.clone();
                                }
                            }
                        }
                    } else {
                        // For stdlib structs, get field type from registry
                        if let Some(field_type) = crate::stdlib_registry::get_stdlib_struct_field_type(struct_type_name, field_name) {
                            return field_type;
                        }
                    }
                }
                _ => {
                    // Field access on a non-struct or not-yet-resolved object;
                    // fall through to FailedToResolve so the caller skips
                    // type comparisons instead of crashing
                }
            }
            NailDataTypeDescriptor::FailedToResolve
        }
        ASTNode::Assignment { left, .. } => {
            // For assignments, the type is the type of the left-hand side (variable being assigned to)
            check_type(left, state)
        }
    }
}

fn add_symbol(state: &mut AnalyzerState, symbol: Symbol) {
    let scope = state.scope_arena.current_scope();
    let scope = state.scope_arena.get_scope_mut(scope);
    scope.symbols.insert(symbol.name.clone(), symbol);
}

fn lookup_symbol<'a>(arena: &'a ScopeArena, scope: usize, name: &str) -> Option<&'a Symbol> {
    // we don't want to modify the original scope, so we'll use a copy for traversal
    let mut scope_for_traversal = scope;
    loop {
        let scope_data = arena.get_scope(scope_for_traversal);
        if let Some(symbol) = scope_data.symbols.get(name) {
            return Some(symbol);
        }
        if scope_for_traversal == GLOBAL_SCOPE {
            break; // We've checked the global scope and haven't found the symbol
        }
        scope_for_traversal = scope_data.parent;
    }
    None
}

fn check_variable_in_function_scope(state: &AnalyzerState, name: &str, function_scope: usize) -> bool {
    // Check if the variable exists in the function scope or its child scopes
    let mut current_scope = state.scope_arena.current_scope();
    
    // Walk up from current scope to function scope
    loop {
        let scope = state.scope_arena.get_scope(current_scope);
        
        // Check if variable exists in this scope
        if scope.symbols.contains_key(name) {
            return true;  // Found in function scope or child scope
        }
        
        // If we've reached the function scope, stop
        if current_scope == function_scope {
            break;
        }
        
        // If we've gone past the function scope to a parent, the variable is external
        if current_scope == GLOBAL_SCOPE || scope.parent < function_scope {
            return false;
        }
        
        current_scope = scope.parent;
    }
    
    false  // Not found in function scope
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
                // state.errors.push(CodeError { help: None, message: format!("Unused variable: {}", symbol.name), code_span: CodeSpan::default() });
            }
        }
    }
}

// Check if two types are compatible, handling special cases like Any types
/// Unify a stdlib signature type (which may contain TypeVar placeholders like T)
/// against a concrete argument type, accumulating bindings. Returns false on a
/// genuine mismatch (including two arguments binding the same variable to
/// incompatible types).
fn unify_types(expected: &NailDataTypeDescriptor, actual: &NailDataTypeDescriptor, bindings: &mut HashMap<String, NailDataTypeDescriptor>) -> bool {
    match (expected, actual) {
        // Unresolvable/wildcard actuals never fail unification (matches type_compatible)
        (_, NailDataTypeDescriptor::Any) | (_, NailDataTypeDescriptor::FailedToResolve) | (_, NailDataTypeDescriptor::OneOf(_)) => true,

        (NailDataTypeDescriptor::TypeVar(name, bounds), _) => {
            if let Some(existing) = bindings.get(name).cloned() {
                type_compatible(&existing, actual)
            } else if !bounds.is_empty() && !bounds.contains(actual) {
                false
            } else {
                bindings.insert(name.clone(), actual.clone());
                true
            }
        }

        (NailDataTypeDescriptor::Array(expected_elem), NailDataTypeDescriptor::Array(actual_elem)) => unify_types(expected_elem, actual_elem, bindings),
        (NailDataTypeDescriptor::Result(expected_inner), NailDataTypeDescriptor::Result(actual_inner)) => unify_types(expected_inner, actual_inner, bindings),
        (NailDataTypeDescriptor::HashMap(exp_key, exp_val), NailDataTypeDescriptor::HashMap(act_key, act_val)) => {
            unify_types(exp_key, act_key, bindings) && unify_types(exp_val, act_val, bindings)
        }
        (NailDataTypeDescriptor::Fn(exp_params, exp_ret), NailDataTypeDescriptor::Fn(act_params, act_ret)) => {
            exp_params.len() == act_params.len()
                && exp_params.iter().zip(act_params.iter()).all(|(e, a)| unify_types(e, a, bindings))
                && unify_types(exp_ret, act_ret, bindings)
        }

        // Anything without type variables falls back to plain compatibility
        _ => type_compatible(expected, actual),
    }
}

/// Replace TypeVar placeholders in a stdlib return type with their bindings.
/// Unbound variables become Any: the declared type at the call site then
/// drives the concrete type (e.g. db_sqlite_query into a declared a:Person).
fn substitute_type_vars(data_type: &NailDataTypeDescriptor, bindings: &HashMap<String, NailDataTypeDescriptor>) -> NailDataTypeDescriptor {
    match data_type {
        NailDataTypeDescriptor::TypeVar(name, _) => bindings.get(name).cloned().unwrap_or(NailDataTypeDescriptor::Any),
        NailDataTypeDescriptor::Array(inner) => NailDataTypeDescriptor::Array(Box::new(substitute_type_vars(inner, bindings))),
        NailDataTypeDescriptor::Result(inner) => NailDataTypeDescriptor::Result(Box::new(substitute_type_vars(inner, bindings))),
        NailDataTypeDescriptor::HashMap(key, value) => {
            NailDataTypeDescriptor::HashMap(Box::new(substitute_type_vars(key, bindings)), Box::new(substitute_type_vars(value, bindings)))
        }
        NailDataTypeDescriptor::Fn(params, ret) => {
            NailDataTypeDescriptor::Fn(params.iter().map(|p| substitute_type_vars(p, bindings)).collect(), Box::new(substitute_type_vars(ret, bindings)))
        }
        other => other.clone(),
    }
}

/// Like substitute_type_vars, but leaves unbound variables in place instead of
/// collapsing them to Any. Used for error messages, where a still-unbound
/// variable should display as its name and bounds rather than as Any.
fn substitute_bound_type_vars(data_type: &NailDataTypeDescriptor, bindings: &HashMap<String, NailDataTypeDescriptor>) -> NailDataTypeDescriptor {
    match data_type {
        NailDataTypeDescriptor::TypeVar(name, _) => bindings.get(name).cloned().unwrap_or_else(|| data_type.clone()),
        NailDataTypeDescriptor::Array(inner) => NailDataTypeDescriptor::Array(Box::new(substitute_bound_type_vars(inner, bindings))),
        NailDataTypeDescriptor::Result(inner) => NailDataTypeDescriptor::Result(Box::new(substitute_bound_type_vars(inner, bindings))),
        NailDataTypeDescriptor::HashMap(key, value) => {
            NailDataTypeDescriptor::HashMap(Box::new(substitute_bound_type_vars(key, bindings)), Box::new(substitute_bound_type_vars(value, bindings)))
        }
        NailDataTypeDescriptor::Fn(params, ret) => {
            NailDataTypeDescriptor::Fn(params.iter().map(|p| substitute_bound_type_vars(p, bindings)).collect(), Box::new(substitute_bound_type_vars(ret, bindings)))
        }
        other => other.clone(),
    }
}

fn type_compatible(expected: &NailDataTypeDescriptor, actual: &NailDataTypeDescriptor) -> bool {
    match (expected, actual) {
        // Exact match
        (a, b) if a == b => true,
        
        // Any type matches anything
        (_, NailDataTypeDescriptor::Any) => true,
        (NailDataTypeDescriptor::Any, _) => true,
        
        // FailedToResolve means we couldn't determine the type, so allow it
        (_, NailDataTypeDescriptor::FailedToResolve) => true,
        
        // OneOf type is compatible with anything
        (_, NailDataTypeDescriptor::OneOf(_)) => true,
        (NailDataTypeDescriptor::OneOf(_), _) => true,
        
        // Array types - check element type compatibility recursively
        (NailDataTypeDescriptor::Array(expected_elem), NailDataTypeDescriptor::Array(actual_elem)) => {
            type_compatible(expected_elem, actual_elem)
        }
        
        // Result types - check inner type compatibility
        (NailDataTypeDescriptor::Result(expected_inner), NailDataTypeDescriptor::Result(actual_inner)) => {
            type_compatible(expected_inner, actual_inner)
        }
        
        // HashMap types - check key and value type compatibility
        (NailDataTypeDescriptor::HashMap(exp_key, exp_val), NailDataTypeDescriptor::HashMap(act_key, act_val)) => {
            type_compatible(exp_key, act_key) && type_compatible(exp_val, act_val)
        }
        
        // Allow struct/enum confusion if they have the same name
        (NailDataTypeDescriptor::Struct(struct_name), NailDataTypeDescriptor::Enum(enum_name)) => struct_name == enum_name,
        (NailDataTypeDescriptor::Enum(enum_name), NailDataTypeDescriptor::Struct(struct_name)) => enum_name == struct_name,
        
        // Function types - would need to check parameter and return types
        (NailDataTypeDescriptor::Fn(exp_params, exp_ret), NailDataTypeDescriptor::Fn(act_params, act_ret)) => {
            exp_params.len() == act_params.len() &&
            exp_params.iter().zip(act_params.iter()).all(|(e, a)| type_compatible(e, a)) &&
            type_compatible(exp_ret, act_ret)
        }
        
        _ => false,
    }
}

fn add_error(state: &mut AnalyzerState, message: String, code_span: &mut CodeSpan) {
    state.errors.push(CodeError { help: None, message, code_span: code_span.clone() });
}

/// Help text for using a collection operation on a scalar value: explains that
/// the operation is per-element and shows how to wrap the value if a
/// one-element collection was intended.
fn collection_op_help(op: &str, iterable: &ASTNode) -> Option<String> {
    let wrapped = match iterable {
        ASTNode::Identifier { name, .. } => format!("[{}]", name),
        _ => "[...]".to_string(),
    };
    Some(format!("'{}' runs its body once for each element of a collection, and a single value has no elements to loop over; if you meant a one-element array, write {}", op, wrapped))
}

fn add_error_with_help(state: &mut AnalyzerState, message: String, help: Option<String>, code_span: &mut CodeSpan) {
    state.errors.push(CodeError { help, message, code_span: code_span.clone() });
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { prev } else { prev + 1 };
            prev = row[j + 1];
            row[j + 1] = cost.min(row[j] + 1).min(row[j + 1] + 1);
        }
    }
    row[b.len()]
}

/// Closest candidate within an edit distance small enough to be a plausible
/// typo, used for "did you mean ...?" suggestions.
fn best_suggestion<'a>(target: &str, candidates: impl Iterator<Item = &'a str>) -> Option<String> {
    let max_distance = (target.chars().count() / 3).clamp(1, 3);
    candidates
        .filter(|candidate| *candidate != target)
        .map(|candidate| (levenshtein(target, candidate), candidate))
        .filter(|(distance, _)| *distance <= max_distance)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate.to_string())
}

/// All symbol names visible from the current scope, for typo suggestions.
fn visible_symbol_names(state: &AnalyzerState) -> Vec<String> {
    let mut names = Vec::new();
    let mut current_scope = state.scope_arena.current_scope();
    loop {
        let scope = state.scope_arena.get_scope(current_scope);
        names.extend(scope.symbols.keys().cloned());
        if current_scope == GLOBAL_SCOPE {
            break;
        }
        current_scope = scope.parent;
    }
    names
}

/// A type is concrete when it contains no inference placeholders; mismatch
/// errors are only reported between concrete types to avoid cascading noise
/// after an earlier inference failure.
fn is_concrete_type(data_type: &NailDataTypeDescriptor) -> bool {
    match data_type {
        NailDataTypeDescriptor::FailedToResolve | NailDataTypeDescriptor::Any | NailDataTypeDescriptor::TypeVar(_, _) | NailDataTypeDescriptor::OneOf(_) => false,
        NailDataTypeDescriptor::Array(inner) | NailDataTypeDescriptor::Result(inner) => is_concrete_type(inner),
        NailDataTypeDescriptor::HashMap(key, value) => is_concrete_type(key) && is_concrete_type(value),
        NailDataTypeDescriptor::Fn(params, ret) => params.iter().all(is_concrete_type) && is_concrete_type(ret),
        _ => true,
    }
}

fn has_return_statement(node: &ASTNode) -> bool {
    match node {
        ASTNode::ReturnDeclaration { .. } => true,
        ASTNode::YieldDeclaration { .. } => true,
        ASTNode::Block { statements, .. } => statements.last().map_or(false, |stmt| has_return_statement(stmt)),
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

fn visit_lambda_declaration(params: &[(String, NailDataTypeDescriptor)], body: &mut Box<ASTNode>, state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    // Check if we're already inside a lambda (nested lambdas not allowed)
    if state.scope_arena.current_scope() > GLOBAL_SCOPE + 1 {
        // Check if any parent scope is a lambda scope
        if state.scope_arena.is_in_lambda() {
            add_error(state, "Nested lambdas are not allowed in Nail".to_string(), code_span);
            return;
        }
    }

    // Create a new scope for the lambda
    let lambda_scope = state.scope_arena.push_scope();
    state.scope_arena.mark_as_lambda(lambda_scope);

    // Add parameters to the lambda scope
    for (param_name, param_type) in params {
        add_symbol(state, Symbol { name: param_name.clone(), data_type: param_type.clone(), is_used: false });
    }

    // Update the lambda body's scope
    if let ASTNode::Block { scope, .. } = body.as_mut() {
        *scope = lambda_scope;
    }

    // Visit the lambda body
    visit_node(body, state);

    // NOTE: We don't pop the lambda scope here because type checking happens later
    // and needs access to all scopes. Scopes will be preserved throughout
    // the entire type checking process.
}

fn check_function_return(name: &str, data_type: &NailDataTypeDescriptor, body: &ASTNode, state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    // First check the overall body type
    let _ = check_type(body, state);
    let data_type = &resolve_custom_type(state, data_type);

    let last_statement = match body {
        ASTNode::Block { statements, .. } => statements.last(),
        _ => None,
    };

    match last_statement {
        Some(ASTNode::ReturnDeclaration { statement, .. }) => {
            let actual_return_type = check_type(statement, state);

            // Compare with the same compatibility rules declarations use:
            // type_compatible treats unresolvable actuals (Any,
            // FailedToResolve — e.g. empty array literals — and OneOf
            // sentinels) as wildcards instead of false mismatches
            let types_compatible = match data_type {
                // For Result types, allow returning the bare inner type
                // (it will be wrapped in Ok()) or an e() error constructor
                NailDataTypeDescriptor::Result(expected_inner) => {
                    matches!(statement.as_ref(), ASTNode::FunctionCall { name, .. } if name == "e")
                        || type_compatible(data_type, &actual_return_type)
                        || type_compatible(expected_inner, &actual_return_type)
                }
                _ => type_compatible(data_type, &actual_return_type),
            };

            if !types_compatible {
                add_error_with_help(
                    state,
                    format!("Function '{}' is declared to return {} but this return statement gives {}", name, data_type.describe(), actual_return_type.describe()),
                    Some(format!("either change the function's declared return type to '{}' or return {}", actual_return_type, data_type.describe())),
                    code_span,
                );
            }
        }
        Some(ASTNode::YieldDeclaration { .. }) => {
            // Yield statements are not allowed in function returns
            add_error(state, format!("Cannot use 'y' (yield) in function '{}' - use 'r' (return) instead", name), code_span);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lexer;
    use crate::parser::parse;

    // Removed test_error_type_display: it asserted a "expected f, got f!e" const-declaration
    // type-mismatch error, but the checker no longer validates const declaration values
    // against their declared types, so that error message no longer exists. The Display
    // formatting of result types ("f!e") is still covered by test_result_type_display below.

    #[test]
    fn test_dangerous_unwraps_result_type() {
        // Test that danger() properly unwraps Result types. Result types can no longer be
        // stored in variables (`result:f!e = ...` is a parse error); results must be
        // handled immediately at the call site.
        let code = "unwrapped:f = danger(float_from(`42`));";

        let tokens = lexer(code);
        let parse_result = parse(tokens);
        let ast = match parse_result {
            Ok(ast) => ast,
            Err(e) => panic!("Parse error: {}", e),
        };

        let mut ast_mut = ast.clone();
        let result = checker(&mut ast_mut);

        // Should type check successfully
        assert!(result.is_ok(), "danger() should properly unwrap f!e to f, got errors: {:?}", result.err());
    }

    #[test]
    fn test_result_type_display() {
        // Test Display implementation for Result types
        let result_int = NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Int));
        assert_eq!(format!("{}", result_int), "i!e");

        let result_float = NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Float));
        assert_eq!(format!("{}", result_float), "f!e");

        let result_string = NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::String));
        assert_eq!(format!("{}", result_string), "s!e");
    }
}
