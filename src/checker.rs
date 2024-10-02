use crate::lexer::NailDataTypeDescriptor;
use crate::lexer::Operation;
use crate::parser::ASTNode;
use crate::CodeError;
use std::collections::{HashMap, HashSet, VecDeque};

type SymbolTable = HashMap<String, Symbol>;
type ScopeStack = VecDeque<SymbolTable>;
type EnumVariants = HashMap<String, HashSet<String>>;

#[derive(Debug, Clone)]
struct Symbol {
    name: String,
    data_type: NailDataTypeDescriptor,
    is_mutable: bool,
    is_used: bool,
}

struct AnalyzerState {
    scope_stack: ScopeStack,
    errors: Vec<CodeError>,
    warnings: Vec<CodeError>,
    in_function: bool,
    enum_variants: EnumVariants,
}

fn new_analyzer_state() -> AnalyzerState {
    AnalyzerState { scope_stack: VecDeque::from([HashMap::new()]), errors: Vec::new(), warnings: Vec::new(), in_function: false, enum_variants: HashMap::new() }
}

pub fn checker(ast: &ASTNode) -> Result<(), Vec<CodeError>> {
    let mut state = new_analyzer_state();
    visit_node(ast, &mut state);
    check_unused_symbols(&mut state);

    if state.errors.is_empty() {
        Ok(())
    } else {
        Err(state.errors)
    }
}

fn visit_node(node: &ASTNode, state: &mut AnalyzerState) {
    match node {
        ASTNode::Program(statements) => statements.iter().for_each(|stmt| visit_node(stmt, state)),
        ASTNode::FunctionDeclaration { name, params, return_type, body } => visit_function_declaration(name, params, return_type, body, state),
        ASTNode::VariableDeclaration { name, data_type, value } => visit_variable_declaration(name, data_type, value, state, true),
        ASTNode::ConstDeclaration { name, data_type, value } => visit_variable_declaration(name, data_type, value, state, false),
        ASTNode::BinaryOperation { left, operator, right } => visit_binary_operation(left, operator, right, state),
        ASTNode::Identifier(name) => {
            if !mark_symbol_as_used(state, name) {
                add_error(state, format!("Undefined variable: {}", name));
            }
        }
        ASTNode::IfStatement { condition_branch_pairs, else_branch } => visit_if_statement(condition_branch_pairs, else_branch, state),
        ASTNode::StructDeclaration { name, fields } => visit_struct_declaration(name, fields, state),
        ASTNode::EnumDeclaration { name, variants } => visit_enum_declaration(name, variants, state),
        ASTNode::ArrayLiteral(elements) => visit_array_literal(elements, state),
        // ASTNode::FunctionCall { name, args } => visit_function_call(name, args, state),
        ASTNode::ReturnStatement(expr) => visit_return_statement(expr, state),
        _ => {} // Handle other cases as needed
    }
}

fn visit_function_declaration(name: &str, params: &[(String, NailDataTypeDescriptor)], return_type: &NailDataTypeDescriptor, body: &ASTNode, state: &mut AnalyzerState) {
    state.scope_stack.push_front(HashMap::new());
    state.in_function = true;

    params.iter().for_each(|(param_name, param_type)| {
        add_symbol(state, Symbol { name: param_name.clone(), data_type: param_type.clone(), is_mutable: false, is_used: false });
    });

    visit_node(body, state);
    check_function_return(name, return_type, body, state);

    state.in_function = false;
    state.scope_stack.pop_front();
}

fn visit_variable_declaration(name: &str, data_type: &NailDataTypeDescriptor, value: &ASTNode, state: &mut AnalyzerState, is_mutable: bool) {
    let value_type = check_type(value, state);
    if *data_type != value_type {
        add_error(state, format!("Type mismatch in variable declaration: expected {:?}, got {:?}", data_type, value_type));
    }
    add_symbol(state, Symbol { name: name.to_string(), data_type: data_type.clone(), is_mutable, is_used: false });
}

fn visit_binary_operation(left: &ASTNode, operator: &Operation, right: &ASTNode, state: &mut AnalyzerState) {
    let left_type = check_type(left, state);
    let right_type = check_type(right, state);
    if left_type != right_type {
        add_error(state, format!("Type mismatch in binary operation: left operand is {:?}, right operand is {:?}", left_type, right_type));
    }
    // TODO: Check if the operator is valid for the given types
}

fn visit_if_statement(condition_branch_pairs: &[(Box<ASTNode>, Box<ASTNode>)], else_branch: &Option<Box<ASTNode>>, state: &mut AnalyzerState) {
    condition_branch_pairs.iter().for_each(|(condition, branch)| {
        visit_node(condition, state);
        visit_node(branch, state);
    });
    else_branch.as_ref().map(|branch| visit_node(branch, state));
}

fn visit_struct_declaration(name: &str, fields: &[ASTNode], state: &mut AnalyzerState) {
    fields.iter().for_each(|field| {
        if let ASTNode::StructDeclarationField { name: field_name, data_type } = field {
            if matches!(data_type, NailDataTypeDescriptor::Struct(_) | NailDataTypeDescriptor::Enum(_)) {
                add_error(state, format!("Nested structs or enums are not allowed in struct '{}', field '{}'", name, field_name));
            }
        }
    });
}

fn visit_enum_declaration(name: &str, variants: &[ASTNode], state: &mut AnalyzerState) {
    let mut variant_set = HashSet::new();
    variants.iter().for_each(|variant| {
        if let ASTNode::EnumVariant { name: variant_name, .. } = variant {
            if !variant_set.insert(variant_name.clone()) {
                add_error(state, format!("Duplicate variant '{}' in enum '{}'", variant_name, name));
            }
        }
    });
    state.enum_variants.insert(name.to_string(), variant_set);
}

fn visit_array_literal(elements: &[ASTNode], state: &mut AnalyzerState) {
    if elements.is_empty() {
        add_error(state, "Empty array literals are not allowed".to_string());
        return;
    }

    let first_type = check_type(&elements[0], state);
    elements.iter().skip(1).for_each(|element| {
        let element_type = check_type(element, state);
        if element_type != first_type {
            add_error(state, format!("Inconsistent types in array literal: expected {:?}, got {:?}", first_type, element_type));
        }
    });
}

// fn visit_function_call(name: &str, args: &[ASTNode], state: &mut AnalyzerState) {
//     let symbol = match lookup_symbol(state, name) {
//         Some(s) => s,
//         None => {
//             add_error(state, format!("Undefined function: {}", name));
//             return;
//         }
//     };

//     let (param_types, _) = match &symbol.data_type {
//         NailDataTypeDescriptor::Fn(param_types, return_type) => (param_types, return_type),
//         _ => {
//             add_error(state, format!("'{}' is not a function", name));
//             return;
//         }
//     };

//     if param_types.len() != args.len() {
//         add_error(state, format!("Function '{}' called with wrong number of arguments. Expected {}, got {}", name, param_types.len(), args.len()));
//         return;
//     }

//     args.iter().zip(param_types.iter()).enumerate().for_each(|(i, (arg, expected_type))| {
//         let arg_type = check_type(arg, state);
//         if arg_type != *expected_type {
//             add_error(state, format!("Type mismatch in argument {} of function '{}': expected {:?}, got {:?}", i + 1, name, expected_type, arg_type));
//         }
//     });
// }

fn visit_return_statement(expr: &ASTNode, state: &mut AnalyzerState) {
    if !state.in_function {
        add_error(state, "Return statement outside of function".to_string());
    }
    check_type(expr, state);
}

fn check_type(node: &ASTNode, state: &AnalyzerState) -> NailDataTypeDescriptor {
    match node {
        ASTNode::NumberLiteral(_) => NailDataTypeDescriptor::Int,
        ASTNode::StringLiteral(_) => NailDataTypeDescriptor::String,
        ASTNode::Identifier(name) => lookup_symbol(state, name).map_or(NailDataTypeDescriptor::Error, |s| s.data_type.clone()),
        ASTNode::BinaryOperation { left, operator: _, right } => {
            let left_type = check_type(left, state);
            let right_type = check_type(right, state);
            if left_type == right_type {
                left_type
            } else {
                NailDataTypeDescriptor::Error
            }
        }
        _ => NailDataTypeDescriptor::Error, // Handle other cases as needed
    }
}

fn add_symbol(state: &mut AnalyzerState, symbol: Symbol) {
    if let Some(scope) = state.scope_stack.front_mut() {
        if scope.contains_key(&symbol.name) {
            add_error(state, format!("Symbol '{}' is already defined in this scope", symbol.name));
        } else {
            scope.insert(symbol.name.clone(), symbol);
        }
    }
}

fn lookup_symbol<'a>(state: &'a AnalyzerState, name: &str) -> Option<&'a Symbol> {
    state.scope_stack.iter().find_map(|scope| scope.get(name))
}

fn mark_symbol_as_used(state: &mut AnalyzerState, name: &str) -> bool {
    state.scope_stack.iter_mut().any(|scope| {
        if let Some(symbol) = scope.get_mut(name) {
            symbol.is_used = true;
            true
        } else {
            false
        }
    })
}

fn add_error(state: &mut AnalyzerState, message: String) {
    state.errors.push(CodeError {
        message,
        line: 0,   // You'd want to pass in line numbers from the AST
        column: 0, // You'd want to pass in column numbers from the AST
    });
}

fn check_unused_symbols(state: &mut AnalyzerState) {
    state.scope_stack.iter().flat_map(|scope| scope.values()).filter(|symbol| !symbol.is_used).for_each(|symbol| {
        state.warnings.push(CodeError {
            message: format!("Unused variable: {}", symbol.name),
            line: 0,   // You'd want to pass in line numbers from the AST
            column: 0, // You'd want to pass in column numbers from the AST
        });
    });
}

fn check_function_return(name: &str, return_type: &NailDataTypeDescriptor, body: &ASTNode, state: &mut AnalyzerState) {
    let (last_stmt, is_return) = match body {
        ASTNode::Block(statements) => statements.last().map_or((None, false), |stmt| (Some(stmt), matches!(stmt, ASTNode::ReturnStatement(_)))),
        _ => (None, false),
    };

    if !is_return {
        add_error(state, format!("Function '{}' must end with an explicit return statement", name));
        return;
    }

    if let Some(ASTNode::ReturnStatement(return_expr)) = last_stmt {
        let expr_type = check_type(return_expr, state);
        if expr_type != *return_type {
            add_error(state, format!("Function '{}' declares return type {:?} but returns {:?}", name, return_type, expr_type));
        }
    }
}
