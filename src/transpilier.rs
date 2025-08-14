use crate::lexer::{NailDataTypeDescriptor, Operation};
use crate::parser::ASTNode;
use crate::stdlib_registry::{self, CrateDependency};
use std::collections::HashSet;
use std::fmt::Write;

pub struct Transpiler {
    indent_level: usize,
    scope_level: usize,
    current_function_return_type: Option<NailDataTypeDescriptor>,
    current_function_name: Option<String>,
    used_stdlib_functions: HashSet<String>,
    in_collection_operation: bool,
}

impl Transpiler {
    pub fn new() -> Self {
        Transpiler { indent_level: 0, scope_level: 0, current_function_return_type: None, current_function_name: None, used_stdlib_functions: HashSet::new(), in_collection_operation: false }
    }

    fn has_return_statements(&self, node: &ASTNode) -> bool {
        match node {
            ASTNode::ReturnDeclaration { .. } => true,
            ASTNode::YieldDeclaration { .. } => true,
            ASTNode::Block { statements, .. } => {
                statements.iter().any(|stmt| self.has_return_statements(stmt))
            }
            ASTNode::IfStatement { condition_branches, else_branch, .. } => {
                condition_branches.iter().any(|(_, branch)| self.has_return_statements(branch)) ||
                else_branch.as_ref().map_or(false, |branch| self.has_return_statements(branch))
            }
            _ => false,
        }
    }
    

    pub fn get_required_dependencies(&self) -> HashSet<CrateDependency> {
        let mut required_crates = HashSet::new();

        // Check stdlib functions for their dependencies
        for func_name in &self.used_stdlib_functions {
            if let Some(func) = stdlib_registry::get_stdlib_function(func_name) {
                required_crates.extend(func.crate_deps.clone());
            }
        }

        required_crates
    }
    
    fn collect_used_functions(&mut self, node: &ASTNode) {
        match node {
            ASTNode::Program { statements, .. } => {
                for stmt in statements {
                    self.collect_used_functions(stmt);
                }
            }
            ASTNode::Block { statements, .. } => {
                for stmt in statements {
                    self.collect_used_functions(stmt);
                }
            }
            ASTNode::FunctionDeclaration { body, .. } => {
                self.collect_used_functions(body);
            }
            ASTNode::FunctionCall { name, args, .. } => {
                if stdlib_registry::get_stdlib_function(name).is_some() {
                    self.used_stdlib_functions.insert(name.clone());
                }
                for arg in args {
                    self.collect_used_functions(arg);
                }
            }
            ASTNode::ConstDeclaration { value, .. } => {
                self.collect_used_functions(value);
            }
            ASTNode::LambdaDeclaration { body, .. } => {
                self.collect_used_functions(body);
            }
            ASTNode::StructInstantiation { fields, .. } => {
                for field in fields {
                    if let ASTNode::StructInstantiationField { value, .. } = field {
                        self.collect_used_functions(value);
                    }
                }
            }
            ASTNode::IfStatement { condition_branches, else_branch, .. } => {
                for (condition, branch) in condition_branches {
                    self.collect_used_functions(condition);
                    self.collect_used_functions(branch);
                }
                if let Some(else_b) = else_branch {
                    self.collect_used_functions(else_b);
                }
            }
            ASTNode::ForLoop { iterable, body, .. } => {
                self.collect_used_functions(iterable);
                self.collect_used_functions(body);
            }
            ASTNode::MapExpression { iterable, body, .. } => {
                self.collect_used_functions(iterable);
                self.collect_used_functions(body);
            }
            ASTNode::FilterExpression { iterable, body, .. } => {
                self.collect_used_functions(iterable);
                self.collect_used_functions(body);
            }
            ASTNode::ReduceExpression { iterable, initial_value, body, .. } => {
                self.collect_used_functions(iterable);
                self.collect_used_functions(initial_value);
                self.collect_used_functions(body);
            }
            ASTNode::EachExpression { iterable, body, .. } => {
                self.collect_used_functions(iterable);
                self.collect_used_functions(body);
            }
            ASTNode::FindExpression { iterable, body, .. } => {
                self.collect_used_functions(iterable);
                self.collect_used_functions(body);
            }
            ASTNode::AllExpression { iterable, body, .. } => {
                self.collect_used_functions(iterable);
                self.collect_used_functions(body);
            }
            ASTNode::AnyExpression { iterable, body, .. } => {
                self.collect_used_functions(iterable);
                self.collect_used_functions(body);
            }
            ASTNode::WhileLoop { condition, max_iterations, body, .. } => {
                self.collect_used_functions(condition);
                if let Some(max) = max_iterations {
                    self.collect_used_functions(max);
                }
                self.collect_used_functions(body);
            }
            ASTNode::Loop { body, .. } => {
                self.collect_used_functions(body);
            }
            ASTNode::SpawnBlock { body, .. } => {
                self.collect_used_functions(body);
            }
            ASTNode::BinaryOperation { left, right, .. } => {
                self.collect_used_functions(left);
                self.collect_used_functions(right);
            }
            ASTNode::ReturnDeclaration { statement, .. } => {
                self.collect_used_functions(statement);
            }
            ASTNode::YieldDeclaration { statement, .. } => {
                self.collect_used_functions(statement);
            }
            ASTNode::ArrayLiteral { elements, .. } => {
                for elem in elements {
                    self.collect_used_functions(elem);
                }
            }
            ASTNode::StructFieldAccess { .. } => {
                // No nested expressions in simple field access
            }
            ASTNode::NestedFieldAccess { object, .. } => {
                self.collect_used_functions(object);
            }
            ASTNode::ParallelBlock { statements, .. } => {
                for stmt in statements {
                    self.collect_used_functions(stmt);
                }
            }
            ASTNode::UnaryOperation { operand, .. } => {
                self.collect_used_functions(operand);
            }
            // Terminal nodes (literals, identifiers, etc.) that don't contain function calls
            ASTNode::StringLiteral { .. } | 
            ASTNode::NumberLiteral { .. } | 
            ASTNode::BooleanLiteral { .. } | 
            ASTNode::Identifier { .. } |
            ASTNode::BreakStatement { .. } |
            ASTNode::ContinueStatement { .. } |
            ASTNode::StructDeclaration { .. } |
            ASTNode::EnumDeclaration { .. } |
            ASTNode::StructDeclarationField { .. } |
            ASTNode::EnumVariant { .. } => {
                // These nodes don't contain function calls or other expressions
            }
            ASTNode::Assignment { left, right, .. } => {
                // Collect functions from both sides of assignment
                self.collect_used_functions(left);
                self.collect_used_functions(right);
            }
            _ => {
                panic!("collect_used_functions: unhandled node type");
            }
        }
    }

    pub fn transpile(&mut self, node: &ASTNode) -> Result<String, std::fmt::Error> {
        // First pass: collect all used functions by traversing the AST
        self.collect_used_functions(node);
        
        let mut output = String::new();
        writeln!(output, "use tokio;")?;
        writeln!(output, "use nail::std_lib;")?;
        writeln!(output, "use nail::print_macro;")?;
        // Always import Box in case of recursive functions
        writeln!(output, "use std::boxed::Box;")?;
        // Always import rayon and futures since map/filter/reduce are so common
        writeln!(output, "use rayon::prelude::*;")?;
        writeln!(output, "use rayon::iter::IntoParallelIterator;")?;
        writeln!(output, "use futures::future;")?;
        
        // Collect and import all custom types from used stdlib functions
        let mut custom_type_imports = std::collections::HashSet::new();
        for func_name in &self.used_stdlib_functions {
            if let Some(func) = stdlib_registry::get_stdlib_function(func_name) {
                for (type_name, module_path) in &func.custom_type_imports {
                    custom_type_imports.insert((*type_name, *module_path));
                }
            }
        }
        
        // Generate imports for custom types
        for (type_name, module_path) in custom_type_imports {
            writeln!(output, "use {}::{};", module_path, type_name)?;
        }
        
        // Generate imports for required crates
        let required_deps = self.get_required_dependencies();
        if required_deps.contains(&CrateDependency::DashMap) {
            writeln!(output, "use dashmap::DashMap;")?;
        }
        
        writeln!(output)?;
        writeln!(output, "#[tokio::main]")?;
        writeln!(output, "async fn main() {{")?;
        self.indent_level += 1;
        self.transpile_node(node, &mut output)?;
        self.indent_level -= 1;
        writeln!(output, "}}")?;
        Ok(output)
    }

    fn transpile_node(&mut self, node: &ASTNode, output: &mut String) -> Result<(), std::fmt::Error> {
        self.transpile_node_internal(node, output, true)
    }

    fn transpile_node_internal(&mut self, node: &ASTNode, output: &mut String, add_semicolons: bool) -> Result<(), std::fmt::Error> {
        match node {
            ASTNode::StructDeclarationField { .. } => {
                // This is handled in StructDeclaration
            }
            ASTNode::StructInstantiationField { .. } => {
                // This is handled in StructInstantiation
            }
            ASTNode::Program { statements, .. } => {
                for stmt in statements {
                    self.transpile_node_internal(stmt, output, add_semicolons)?;
                    if !add_semicolons {
                        writeln!(output)?;
                    }
                }
            }
            ASTNode::FunctionDeclaration { name, params, data_type, body, .. } => {
                write!(output, "{}async fn {}(", self.indent(), name)?;
                for (i, (param_name, param_type)) in params.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ")?;
                    }
                    write!(output, "{}: {}", param_name, self.rust_type(param_type, name))?;
                }
                writeln!(output, ") -> {} {{", self.rust_type(data_type, name))?;

                // Store the current function's context
                let prev_return_type = self.current_function_return_type.clone();
                let prev_name = self.current_function_name.clone();
                self.current_function_return_type = Some(data_type.clone());
                self.current_function_name = Some(name.clone());

                self.indent_level += 1;
                self.transpile_node_internal(body, output, add_semicolons)?;
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;

                // Restore previous function context
                self.current_function_return_type = prev_return_type;
                self.current_function_name = prev_name;
            }
            ASTNode::FunctionCall { name, args, .. } => {
                if add_semicolons {
                    self.transpile_function_call(name, args, output, true)?;
                } else {
                    self.transpile_function_call(name, args, output, false)?;
                }
            }
            ASTNode::ConstDeclaration { name, data_type, value, .. } => {
                if add_semicolons {
                    write!(output, "{}let {}: {} = ", self.indent(), name, self.rust_type(data_type, name))?;
                } else {
                    // Inside expression context (like lambdas), don't add indent
                    write!(output, "let {}: {} = ", name, self.rust_type(data_type, name))?;
                }
                self.transpile_node_internal(value, output, false)?;
                if add_semicolons {
                    writeln!(output, ";")?;
                }
            }
            ASTNode::IfStatement { condition_branches, else_branch, .. } => {
                // Check if this is being used as an expression or statement
                // If add_semicolons is false, we're in expression context
                if !add_semicolons {
                    // Expression context - generate Rust if expression
                    let _num_conditions = condition_branches.len();
                    for (i, (condition, branch)) in condition_branches.iter().enumerate() {
                        if i == 0 {
                            write!(output, "if ")?;
                        } else {
                            write!(output, " else if ")?;
                        }
                        
                        // Always write the condition
                        self.transpile_node_internal(condition, output, false)?;
                        write!(output, " {{ ")?;
                        
                        // For expressions, we need to output all statements and return the last value
                        if let ASTNode::Block { statements, .. } = branch.as_ref() {
                            // Check if any statement in this branch is diverging
                            let mut _found_diverging = false;
                            for (idx, stmt) in statements.iter().enumerate() {
                                if self.statement_contains_diverging_call(stmt) {
                                    // This statement diverges - output it with semicolon and stop
                                    self.transpile_node_internal(stmt, output, true)?;
                                    write!(output, " ")?;
                                    _found_diverging = true;
                                    break;
                                } else if idx < statements.len() - 1 {
                                    // Not the last statement and not diverging - add semicolon
                                    self.transpile_node_internal(stmt, output, true)?;
                                    write!(output, " ")?;
                                } else {
                                    // Last statement and not diverging - it's the return value
                                    if let ASTNode::ReturnDeclaration { statement, .. } = stmt {
                                        self.transpile_node_internal(statement, output, false)?;
                                    } else {
                                        self.transpile_node_internal(stmt, output, false)?;
                                    }
                                }
                            }
                        }
                        write!(output, " }}")?;
                    }
                    if let Some(branch) = else_branch {
                        write!(output, " else {{ ")?;
                        if let ASTNode::Block { statements, .. } = branch.as_ref() {
                            // Check if any statement in this branch is diverging
                            let mut _found_diverging = false;
                            for (idx, stmt) in statements.iter().enumerate() {
                                if self.statement_contains_diverging_call(stmt) {
                                    // This statement diverges - output it with semicolon and stop
                                    self.transpile_node_internal(stmt, output, true)?;
                                    write!(output, " ")?;
                                    _found_diverging = true;
                                    break;
                                } else if idx < statements.len() - 1 {
                                    // Not the last statement and not diverging - add semicolon
                                    self.transpile_node_internal(stmt, output, true)?;
                                    write!(output, " ")?;
                                } else {
                                    // Last statement and not diverging - it's the return value
                                    if let ASTNode::ReturnDeclaration { statement, .. } = stmt {
                                        self.transpile_node_internal(statement, output, false)?;
                                    } else {
                                        self.transpile_node_internal(stmt, output, false)?;
                                    }
                                }
                            }
                        }
                        write!(output, " }}")?;
                    } else {
                        // No else branch - add unreachable for if expressions
                        write!(output, " else {{ unreachable!(\"Non-exhaustive if expression reached else branch\") }}")?;
                    }
                } else {
                    // Statement context - generate regular if statement
                    let _num_conditions = condition_branches.len();
                    for (i, (condition, branch)) in condition_branches.iter().enumerate() {
                        if i == 0 {
                            write!(output, "{}if ", self.indent())?;
                            self.transpile_node_internal(condition, output, false)?;
                            writeln!(output, " {{")?;
                        } else {
                            write!(output, " else if ")?;
                            self.transpile_node_internal(condition, output, false)?;
                            writeln!(output, " {{")?;
                        }
                        
                        self.indent_level += 1;
                        self.transpile_node_internal(branch, output, add_semicolons)?;
                        self.indent_level -= 1;
                        write!(output, "{}}}", self.indent())?;
                    }
                    if let Some(branch) = else_branch {
                        write!(output, " else {{")?;
                        writeln!(output)?;
                        self.indent_level += 1;
                        self.transpile_node_internal(branch, output, add_semicolons)?;
                        self.indent_level -= 1;
                        write!(output, "{}}}", self.indent())?;
                    }
                    writeln!(output)?;
                }
            }
            ASTNode::Block { statements, .. } => {
                for (i, stmt) in statements.iter().enumerate() {
                    self.transpile_node_internal(stmt, output, add_semicolons)?;
                    if i < statements.len() - 1 {
                        // Add semicolon and newline between statements
                        writeln!(output, ";")?;
                        write!(output, "{}", self.indent())?;
                    }
                }
            }
            ASTNode::ForLoop { iterator, iterable, initial_value, filter, body, .. } => {
                // Check if the loop body contains return statements (collecting values)
                let has_returns = self.has_return_statements(body);
                
                if has_returns {
                    // Generate an imperative collecting loop (since everything is async)
                    if add_semicolons {
                        write!(output, "{}", self.indent())?;
                    }
                    write!(output, "{{")?;
                    writeln!(output)?;
                    self.indent_level += 1;
                    writeln!(output, "{}let mut __result = Vec::new();", self.indent())?;
                    write!(output, "{}for {} in ", self.indent(), iterator)?;
                    self.transpile_node_internal(iterable, output, false)?;
                    writeln!(output, " {{")?;
                    self.indent_level += 1;
                    
                    // Extract and transpile the yield expression
                    if let ASTNode::Block { statements, .. } = body.as_ref() {
                        for stmt in statements {
                            if let ASTNode::YieldDeclaration { statement, .. } = stmt {
                                write!(output, "{}__result.push(", self.indent())?;
                                self.transpile_node_internal(statement, output, false)?;
                                writeln!(output, ");")?;
                                break;
                            } else if let ASTNode::ReturnDeclaration { statement, .. } = stmt {
                                // Legacy return statement support
                                write!(output, "{}__result.push(", self.indent())?;
                                self.transpile_node_internal(statement, output, false)?;
                                writeln!(output, ");")?;
                                break;
                            } else {
                                // Regular statement before yield/return
                                self.transpile_node_internal(stmt, output, true)?;
                            }
                        }
                    }
                    
                    self.indent_level -= 1;
                    writeln!(output, "{}}}", self.indent())?;
                    writeln!(output, "{}__result", self.indent())?;
                    self.indent_level -= 1;
                    write!(output, "{}}}", self.indent())?;
                } else {
                    // Generate a simple for loop for side effects
                    if !add_semicolons {
                        // When used as an expression, wrap in braces to return ()
                        write!(output, "{{")?;
                    }
                    
                    if add_semicolons {
                        write!(output, "{}for {} in ", self.indent(), iterator)?;
                    } else {
                        write!(output, " for {} in ", iterator)?;
                    }
                    self.transpile_node_internal(iterable, output, false)?;
                    writeln!(output, " {{")?;
                    self.indent_level += 1;
                    self.transpile_node_internal(body, output, true)?;
                    self.indent_level -= 1;
                    if add_semicolons {
                        writeln!(output, "{}}}", self.indent())?;
                    } else {
                        write!(output, "}}; () }}")?;
                    }
                }
            }
            ASTNode::MapExpression { iterator, index_iterator, iterable, body, .. } => {
                // Map expressions collect values using Rayon for parallelism
                if add_semicolons {
                    write!(output, "{}", self.indent())?;
                }
                write!(output, "{{")?;
                writeln!(output)?;
                self.indent_level += 1;
                writeln!(output, "{}use rayon::prelude::*;", self.indent())?;
                writeln!(output, "{}use rayon::iter::IntoParallelIterator;", self.indent())?;
                writeln!(output, "{}use futures::future;", self.indent())?;
                
                // Use Rayon to create futures in parallel, then await them all
                write!(output, "{}let __futures: Vec<_> = ", self.indent())?;
                self.transpile_node_internal(iterable, output, false)?;
                writeln!(output, ".into_par_iter().enumerate().map(|(_idx, {})| {{", iterator)?;
                self.indent_level += 1;
                
                // Create async block that captures variables by cloning
                writeln!(output, "{}async move {{", self.indent())?;
                self.indent_level += 1;
                
                // Convert index to i64 if needed
                if let Some(idx) = index_iterator {
                    writeln!(output, "{}let {} = _idx as i64;", self.indent(), idx)?;
                }
                
                // Transpile the body statements and collect result
                if let ASTNode::Block { statements, .. } = body.as_ref() {
                    let num_statements = statements.len();
                    for (i, stmt) in statements.iter().enumerate() {
                        if let ASTNode::YieldDeclaration { statement, .. } = stmt {
                            // This is the yield statement - it's the return value
                            self.transpile_node_internal(statement, output, false)?;
                            if i < num_statements - 1 {
                                writeln!(output)?;
                            }
                        } else if let ASTNode::ReturnDeclaration { statement, .. } = stmt {
                            // Return statement - it's the return value
                            self.transpile_node_internal(statement, output, false)?;
                            if i < num_statements - 1 {
                                writeln!(output)?;
                            }
                        } else {
                            // Regular statement in the map body
                            self.transpile_node_internal(stmt, output, true)?;
                            if i < num_statements - 1 {
                                writeln!(output)?;
                            }
                        }
                    }
                }
                
                self.indent_level -= 1;
                writeln!(output)?;
                writeln!(output, "{}}}", self.indent())?;
                
                self.indent_level -= 1;
                writeln!(output, "{}}}).collect();", self.indent())?;
                
                writeln!(output, "{}let __result = future::join_all(__futures).await;", self.indent())?;
                writeln!(output, "{}__result", self.indent())?;
                self.indent_level -= 1;
                write!(output, "{}}}", self.indent())?;
            }
            ASTNode::FilterExpression { iterator, index_iterator, iterable, body, .. } => {
                // Filter expressions collect values that match a condition using Rayon for parallelism
                if add_semicolons {
                    write!(output, "{}", self.indent())?;
                }
                write!(output, "{{")?;
                writeln!(output)?;
                self.indent_level += 1;
                writeln!(output, "{}use rayon::prelude::*;", self.indent())?;
                writeln!(output, "{}use rayon::iter::IntoParallelIterator;", self.indent())?;
                writeln!(output, "{}use futures::future;", self.indent())?;
                
                // Use Rayon to create futures in parallel, then await them all
                write!(output, "{}let __futures: Vec<_> = ", self.indent())?;
                self.transpile_node_internal(iterable, output, false)?;
                writeln!(output, ".into_par_iter().enumerate().map(|(_idx, {})| async move {{", iterator)?;
                self.indent_level += 1;
                
                // Convert index to i64 if needed
                if let Some(idx) = index_iterator {
                    writeln!(output, "{}let {} = _idx as i64;", self.indent(), idx)?;
                }
                
                // Execute the body block and check the result
                writeln!(output, "{}let condition_result = {{", self.indent())?;
                self.indent_level += 1;
                
                // Set collection operation context
                let prev_context = self.in_collection_operation;
                self.in_collection_operation = true;
                
                // Transpile the body block
                self.transpile_node_internal(body, output, false)?;
                
                // Restore previous context
                self.in_collection_operation = prev_context;
                
                self.indent_level -= 1;
                writeln!(output, "{}}};", self.indent())?;
                
                // Return Some(value) if condition is true, None otherwise
                writeln!(output, "{}if condition_result {{", self.indent())?;
                self.indent_level += 1;
                writeln!(output, "{}Some({}.clone())", self.indent(), iterator)?;
                self.indent_level -= 1;
                writeln!(output, "{}}} else {{", self.indent())?;
                self.indent_level += 1;
                writeln!(output, "{}None", self.indent())?;
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;
                
                self.indent_level -= 1;
                writeln!(output, "{}}}).collect();", self.indent())?;
                writeln!(output, "{}let __results = future::join_all(__futures).await;", self.indent())?;
                writeln!(output, "{}let __result: Vec<_> = __results.into_iter().filter_map(|x| x).collect();", self.indent())?;
                writeln!(output, "{}__result", self.indent())?;
                self.indent_level -= 1;
                write!(output, "{}}}", self.indent())?;
            }
            ASTNode::ReduceExpression { iterator, index_iterator, iterable, initial_value, accumulator, body, .. } => {
                // Reduce expressions fold values into a single result
                // Note: We use sequential iteration for reduce to maintain order-dependent operations
                if add_semicolons {
                    write!(output, "{}", self.indent())?;
                }
                write!(output, "{{")?;
                writeln!(output)?;
                self.indent_level += 1;
                
                // Initialize accumulator
                write!(output, "{}let mut {} = ", self.indent(), accumulator)?;
                self.transpile_node_internal(initial_value, output, false)?;
                writeln!(output, ";")?;
                
                // Use regular iteration for sequential reduce
                write!(output, "{}for (_idx, {}) in ", self.indent(), iterator)?;
                self.transpile_node_internal(iterable, output, false)?;
                writeln!(output, ".into_iter().enumerate() {{")?;
                self.indent_level += 1;
                
                // Convert index to i64 if needed
                if let Some(idx) = index_iterator {
                    writeln!(output, "{}let {} = _idx as i64;", self.indent(), idx)?;
                }
                
                // Transpile the body statements
                if let ASTNode::Block { statements, .. } = body.as_ref() {
                    for stmt in statements {
                        if let ASTNode::YieldDeclaration { statement, .. } = stmt {
                            // This is the yield statement - assign to accumulator
                            write!(output, "{}{} = ", self.indent(), accumulator)?;
                            self.transpile_node_internal(statement, output, false)?;
                            writeln!(output, ";")?;
                        } else if let ASTNode::ReturnDeclaration { statement, .. } = stmt {
                            // Return statement - assign to accumulator
                            write!(output, "{}{} = ", self.indent(), accumulator)?;
                            self.transpile_node_internal(statement, output, false)?;
                            writeln!(output, ";")?;
                        } else {
                            // Regular statement in the reduce body
                            self.transpile_node_internal(stmt, output, true)?;
                        }
                    }
                }
                
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;
                writeln!(output, "{}{}", self.indent(), accumulator)?;
                self.indent_level -= 1;
                write!(output, "{}}}", self.indent())?;
            }
            ASTNode::EachExpression { iterator, index_iterator, iterable, body, .. } => {
                // Each expressions are for side effects only
                if add_semicolons {
                    write!(output, "{}", self.indent())?;
                }
                write!(output, "{{")?;
                writeln!(output)?;
                self.indent_level += 1;
                
                // Always use enumerate()
                write!(output, "{}for (_idx, {}) in ", self.indent(), iterator)?;
                self.transpile_node_internal(iterable, output, false)?;
                writeln!(output, ".into_iter().enumerate() {{")?;
                self.indent_level += 1;
                
                // Convert index to i64 if needed
                if let Some(idx) = index_iterator {
                    writeln!(output, "{}let {} = _idx as i64;", self.indent(), idx)?;
                }
                
                // Transpile the body statements (no return collection)
                self.transpile_node_internal(body, output, true)?;
                
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;
                writeln!(output, "{}()", self.indent())?; // Each returns unit
                self.indent_level -= 1;
                write!(output, "{}}}", self.indent())?;
            }
            ASTNode::FindExpression { iterator, index_iterator, iterable, body, .. } => {
                // Find expressions return Result<T>
                if add_semicolons {
                    write!(output, "{}", self.indent())?;
                }
                write!(output, "{{")?;
                writeln!(output)?;
                self.indent_level += 1;
                writeln!(output, "{}let mut __found = None;", self.indent())?;
                
                // Always use enumerate()
                write!(output, "{}for (_idx, {}) in ", self.indent(), iterator)?;
                self.transpile_node_internal(iterable, output, false)?;
                writeln!(output, ".into_iter().enumerate() {{")?;
                self.indent_level += 1;
                
                // Convert index to i64 if needed
                if let Some(idx) = index_iterator {
                    writeln!(output, "{}let {} = _idx as i64;", self.indent(), idx)?;
                }
                
                // Execute the body block and check the result
                writeln!(output, "{}let condition_result = {{", self.indent())?;
                self.indent_level += 1;
                
                // Set collection operation context
                let prev_context = self.in_collection_operation;
                self.in_collection_operation = true;
                
                // Transpile the body block
                self.transpile_node_internal(body, output, false)?;
                
                // Restore previous context
                self.in_collection_operation = prev_context;
                
                self.indent_level -= 1;
                writeln!(output, "{}}};", self.indent())?;
                
                // Use the result as condition
                writeln!(output, "{}if condition_result {{", self.indent())?;
                self.indent_level += 1;
                writeln!(output, "{}__found = Some({}.clone());", self.indent(), iterator)?;
                writeln!(output, "{}break;", self.indent())?;
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;
                
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;
                writeln!(output, "{}__found.ok_or_else(|| \"Element not found\".to_string())", self.indent())?;
                self.indent_level -= 1;
                write!(output, "{}}}", self.indent())?;
            }
            ASTNode::AllExpression { iterator, index_iterator, iterable, body, .. } => {
                // All expressions check if all elements match a condition
                if add_semicolons {
                    write!(output, "{}", self.indent())?;
                }
                write!(output, "{{")?;
                writeln!(output)?;
                self.indent_level += 1;
                writeln!(output, "{}let mut __all_match = true;", self.indent())?;
                
                // Always use enumerate()
                write!(output, "{}for (_idx, {}) in ", self.indent(), iterator)?;
                self.transpile_node_internal(iterable, output, false)?;
                writeln!(output, ".into_iter().enumerate() {{")?;
                self.indent_level += 1;
                
                // Convert index to i64 if needed
                if let Some(idx) = index_iterator {
                    writeln!(output, "{}let {} = _idx as i64;", self.indent(), idx)?;
                }
                
                // Execute the body block and check the result
                writeln!(output, "{}let condition_result = {{", self.indent())?;
                self.indent_level += 1;
                
                // Set collection operation context
                let prev_context = self.in_collection_operation;
                self.in_collection_operation = true;
                
                // Transpile the body block
                self.transpile_node_internal(body, output, false)?;
                
                // Restore previous context
                self.in_collection_operation = prev_context;
                
                self.indent_level -= 1;
                writeln!(output, "{}}};", self.indent())?;
                
                // Use the result as condition (negated for All)
                writeln!(output, "{}if !condition_result {{", self.indent())?;
                self.indent_level += 1;
                writeln!(output, "{}__all_match = false;", self.indent())?;
                writeln!(output, "{}break;", self.indent())?;
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;
                
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;
                writeln!(output, "{}__all_match", self.indent())?;
                self.indent_level -= 1;
                write!(output, "{}}}", self.indent())?;
            }
            ASTNode::AnyExpression { iterator, index_iterator, iterable, body, .. } => {
                // Any expressions check if any element matches a condition
                if add_semicolons {
                    write!(output, "{}", self.indent())?;
                }
                write!(output, "{{")?;
                writeln!(output)?;
                self.indent_level += 1;
                writeln!(output, "{}let mut __any_match = false;", self.indent())?;
                
                // Always use enumerate()
                write!(output, "{}for (_idx, {}) in ", self.indent(), iterator)?;
                self.transpile_node_internal(iterable, output, false)?;
                writeln!(output, ".into_iter().enumerate() {{")?;
                self.indent_level += 1;
                
                // Convert index to i64 if needed
                if let Some(idx) = index_iterator {
                    writeln!(output, "{}let {} = _idx as i64;", self.indent(), idx)?;
                }
                
                // Execute the body block and check the result
                writeln!(output, "{}let condition_result = {{", self.indent())?;
                self.indent_level += 1;
                
                // Set collection operation context
                let prev_context = self.in_collection_operation;
                self.in_collection_operation = true;
                
                // Transpile the body block
                self.transpile_node_internal(body, output, false)?;
                
                // Restore collection operation context
                self.in_collection_operation = prev_context;
                
                self.indent_level -= 1;
                writeln!(output, "{}}};", self.indent())?;
                
                // Use the result as condition
                writeln!(output, "{}if condition_result {{", self.indent())?;
                self.indent_level += 1;
                writeln!(output, "{}__any_match = true;", self.indent())?;
                writeln!(output, "{}break;", self.indent())?;
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;
                
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;
                writeln!(output, "{}__any_match", self.indent())?;
                self.indent_level -= 1;
                write!(output, "{}}}", self.indent())?;
            }
            ASTNode::WhileLoop { condition, max_iterations, body, .. } => {
                if let Some(max_iter) = max_iterations {
                    // Generate a bounded while loop
                    if add_semicolons {
                        writeln!(output, "{}{{", self.indent())?;
                        writeln!(output, "{}    let mut _iterations = 0;", self.indent())?;
                        write!(output, "{}    let _max_iterations = ", self.indent())?;
                        self.transpile_node_internal(max_iter, output, false)?;
                        writeln!(output, ";")?;
                        write!(output, "{}    while ", self.indent())?;
                    } else {
                        writeln!(output, "{{")?;
                        writeln!(output, "    let mut _iterations = 0;")?;
                        write!(output, "    let _max_iterations = ")?;
                        self.transpile_node_internal(max_iter, output, false)?;
                        writeln!(output, ";")?;
                        write!(output, "    while ")?;
                    }
                    self.transpile_node_internal(condition, output, false)?;
                    writeln!(output, " && _iterations < _max_iterations {{")?;
                    self.indent_level += 2;
                    self.transpile_node_internal(body, output, true)?;
                    if add_semicolons {
                        writeln!(output, "{}_iterations += 1;", self.indent())?;
                    } else {
                        writeln!(output, "        _iterations += 1;")?;
                    }
                    self.indent_level -= 2;
                    if add_semicolons {
                        writeln!(output, "{}    }}", self.indent())?;
                        writeln!(output, "{}}}", self.indent())?;
                    } else {
                        writeln!(output, "    }}")?;
                        write!(output, "}}")?;
                    }
                } else {
                    // Regular unbounded while loop
                    if add_semicolons {
                        write!(output, "{}while ", self.indent())?;
                    } else {
                        write!(output, "while ")?;
                    }
                    self.transpile_node_internal(condition, output, false)?;
                    writeln!(output, " {{")?;
                    self.indent_level += 1;
                    self.transpile_node_internal(body, output, true)?;
                    self.indent_level -= 1;
                    if add_semicolons {
                        writeln!(output, "{}}}", self.indent())?;
                    } else {
                        write!(output, "}}")?;
                    }
                }
            }
            ASTNode::Loop { index_iterator, body, .. } => {
                if let Some(index_name) = index_iterator {
                    // Loop with index iterator - needs mutable counter outside loop
                    if add_semicolons {
                        writeln!(output, "{}{{", self.indent())?;
                        self.indent_level += 1;
                        writeln!(output, "{}let mut __loop_index: i64 = 0;", self.indent())?;
                        writeln!(output, "{}loop {{", self.indent())?;
                    } else {
                        writeln!(output, "{{")?;
                        writeln!(output, "let mut __loop_index: i64 = 0;")?;
                        writeln!(output, "loop {{")?;
                    }
                    self.indent_level += 1;
                    // Make index available inside loop as immutable
                    writeln!(output, "{}let {} = __loop_index;", self.indent(), index_name)?;
                    writeln!(output, "{}__loop_index += 1;", self.indent())?;
                    self.transpile_node_internal(body, output, true)?;
                    self.indent_level -= 1;
                    if add_semicolons {
                        writeln!(output, "{}}}", self.indent())?;
                        self.indent_level -= 1;
                        writeln!(output, "{}}}", self.indent())?;
                    } else {
                        writeln!(output, "}}")?;
                        write!(output, "}}")?;
                    }
                } else {
                    // Simple loop without index
                    if add_semicolons {
                        writeln!(output, "{}loop {{", self.indent())?;
                    } else {
                        writeln!(output, "loop {{")?;
                    }
                    self.indent_level += 1;
                    self.transpile_node_internal(body, output, true)?;
                    self.indent_level -= 1;
                    if add_semicolons {
                        writeln!(output, "{}}}", self.indent())?;
                    } else {
                        write!(output, "}}")?;
                    }
                }
            }
            ASTNode::SpawnBlock { body, .. } => {
                // Spawn a new async task
                if add_semicolons {
                    writeln!(output, "{}tokio::spawn(async move {{", self.indent())?;
                } else {
                    writeln!(output, "tokio::spawn(async move {{")?;
                }
                self.indent_level += 1;
                self.transpile_node_internal(body, output, true)?;
                self.indent_level -= 1;
                if add_semicolons {
                    writeln!(output, "{}}}){};", self.indent(), if add_semicolons { "" } else { "" })?;
                } else {
                    write!(output, "}})")?;
                }
            }
            ASTNode::BreakStatement { .. } => {
                if add_semicolons {
                    writeln!(output, "{}break;", self.indent())?;
                } else {
                    write!(output, "break")?;
                }
            }
            ASTNode::ContinueStatement { .. } => {
                if add_semicolons {
                    writeln!(output, "{}continue;", self.indent())?;
                } else {
                    write!(output, "continue")?;
                }
            }
            ASTNode::ParallelBlock { statements, .. } => {
                self.transpile_parallel_block(statements, output)?;
            }
            ASTNode::BinaryOperation { left, operator, right, .. } => {
                // No string concatenation with + allowed in Nail - use array_join instead
                self.transpile_node_internal(left, output, false)?;
                write!(output, " {} ", self.rust_operator(operator))?;
                self.transpile_node_internal(right, output, false)?;
            }
            ASTNode::UnaryOperation { operator, operand, .. } => {
                write!(output, "{}", self.rust_operator(operator))?;
                self.transpile_node_internal(operand, output, false)?;
            }
            ASTNode::Identifier { name, .. } => {
                // Always clone identifiers to avoid ownership issues
                write!(output, "{}.clone()", name)?;
            }
            ASTNode::NumberLiteral { value, .. } => {
                write!(output, "{}", value)?;
            }
            ASTNode::StringLiteral { value, .. } => {
                // For multiline strings or strings with backslashes, use raw strings
                // Quotes don't need escaping in backtick strings and work fine in raw strings
                if value.contains('\n') || value.contains('\t') || value.contains('\\') || value.contains('"') {
                    // Use raw string literal with enough # symbols to avoid conflicts
                    let mut delimiter = String::from("#");
                    while value.contains(&format!("\"{}", delimiter)) || value.contains(&format!("#{}", delimiter)) {
                        delimiter.push('#');
                    }
                    write!(output, "r{0}\"{1}\"{0}.to_string()", delimiter, value)?;
                } else {
                    // Use regular string literal for simple strings
                    write!(output, "\"{}\".to_string()", value)?;
                }
            }
            ASTNode::BooleanLiteral { value, .. } => {
                write!(output, "{}", value)?;
            }
            ASTNode::NestedFieldAccess { object, field_name, .. } => {
                self.transpile_node_internal(object, output, false)?;
                write!(output, ".{}", field_name)?;
            }

            ASTNode::ReturnDeclaration { statement, .. } => {
                // In collection operations, return statements should just be the expression value
                if self.in_collection_operation {
                    self.transpile_node_internal(statement, output, false)?;
                } else {
                    if add_semicolons {
                        write!(output, "{}return ", self.indent())?;
                    } else {
                        write!(output, "return ")?;
                    }

                    // Check if we need to wrap in Ok() for result types
                    let needs_ok_wrap = if let Some(return_type) = &self.current_function_return_type {
                        match return_type {
                            NailDataTypeDescriptor::Any => false,
                            NailDataTypeDescriptor::Result(_) => true,
                            _ => false,
                        }
                    } else {
                        false
                    };

                    // Check if the statement is already an error (e() call)
                    let is_error_call = match statement.as_ref() {
                        ASTNode::FunctionCall { name, .. } => name == "e",
                        _ => false,
                    };

                    if needs_ok_wrap && !is_error_call {
                        write!(output, "Ok(")?;
                        self.transpile_node_internal(statement, output, false)?;
                        write!(output, ")")?;
                    } else {
                        self.transpile_node_internal(statement, output, false)?;
                    }
                    
                    if add_semicolons {
                        writeln!(output, ";")?;
                    }
                }
            }
            ASTNode::YieldDeclaration { statement, .. } => {
                // Yield statements should only be used in collection operations
                if self.in_collection_operation {
                    self.transpile_node_internal(statement, output, false)?;
                } else {
                    // This should be caught by the type checker, but just in case
                    write!(output, "/* ERROR: yield outside collection operation */")?;
                }
            }
            ASTNode::StructDeclaration { name, fields, .. } => {
                writeln!(output, "{}#[derive(Debug, Clone, PartialEq)]", self.indent())?;
                writeln!(output, "{}struct {} {{", self.indent(), name)?;
                self.indent_level += 1;
                for field in fields {
                    match field {
                        ASTNode::StructDeclarationField { name: field_name, data_type, .. } => {
                            writeln!(output, "{}{}: {},", self.indent(), field_name, self.rust_type(data_type, field_name))?;
                        }
                        _ => return Err(std::fmt::Error),
                    }
                }
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;
            }
            ASTNode::EnumDeclaration { name, variants, .. } => {
                writeln!(output, "{}#[derive(Debug, Clone, PartialEq)]", self.indent())?;
                writeln!(output, "{}enum {} {{", self.indent(), name)?;
                self.indent_level += 1;
                for variant in variants {
                    match variant {
                        ASTNode::EnumVariant { variant, .. } => {
                            writeln!(output, "{}{},", self.indent(), variant)?;
                        }
                        _ => return Err(std::fmt::Error),
                    }
                }
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;
            }
            ASTNode::LambdaDeclaration { params, body, .. } => {
                // Generate a regular closure that returns an async block
                write!(output, "move |")?;
                for (i, (param_name, param_type)) in params.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ")?;
                    }
                    write!(output, "{}: {}", param_name, self.rust_type(param_type, ""))?;
                }
                write!(output, "| {{ async move {{ ")?;

                // Transpile the body inline
                if let ASTNode::Block { statements, .. } = body.as_ref() {
                    for (i, stmt) in statements.iter().enumerate() {
                        if i > 0 {
                            write!(output, "; ")?;
                        }
                        self.transpile_node_internal(stmt, output, false)?;
                    }
                }

                write!(output, " }} }}")?;
            }
            ASTNode::StructInstantiation { name, fields, .. } => {
                write!(output, "{} {{", name)?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ")?;
                    }
                    match field {
                        ASTNode::StructInstantiationField { name: field_name, value, .. } => {
                            write!(output, " {}: ", field_name)?;
                            self.transpile_node_internal(value, output, false)?;
                        }
                        _ => return Err(std::fmt::Error),
                    }
                }
                write!(output, " }}")?;
            }
            ASTNode::StructFieldAccess { struct_name, field_name, .. } => {
                write!(output, "{}.{}.clone()", struct_name, field_name)?;
            }
            ASTNode::EnumVariant { name, variant, .. } => {
                write!(output, "{}{}::{}", self.indent(), name, variant)?;
            }
            ASTNode::ArrayLiteral { elements, .. } => {
                write!(output, "vec! [")?;
                for (i, value) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ")?;
                    }
                    self.transpile_node_internal(value, output, false)?;
                }
                write!(output, "]")?;
            }
            ASTNode::Assignment { left, right, .. } => {
                // Transpile assignment: left = right
                // For assignment left-hand side, don't clone - just use the variable name
                if let ASTNode::Identifier { name, .. } = left.as_ref() {
                    write!(output, "{}", name)?;
                } else {
                    self.transpile_node_internal(left, output, false)?;
                }
                write!(output, " = ")?;
                self.transpile_node_internal(right, output, false)?;
            }
        }
        Ok(())
    }

    fn rust_type(&self, data_type: &NailDataTypeDescriptor, _name: &str) -> String {
        match data_type {
            NailDataTypeDescriptor::String => "String".to_string(),
            NailDataTypeDescriptor::Int => "i64".to_string(),
            NailDataTypeDescriptor::Float => "f64".to_string(),
            NailDataTypeDescriptor::Boolean => "bool".to_string(),
            NailDataTypeDescriptor::Struct(name) => name.to_string(),
            NailDataTypeDescriptor::Enum(name) => name.to_string(),
            NailDataTypeDescriptor::Void => "()".to_string(),
            NailDataTypeDescriptor::Never => "!".to_string(),
            NailDataTypeDescriptor::Error => "String".to_string(),
            NailDataTypeDescriptor::Array(inner) => format!("Vec<{}>", self.rust_type(inner, _name)),
            NailDataTypeDescriptor::Any => "Box<dyn std::any::Any>".to_string(),
            NailDataTypeDescriptor::Result(inner_type) => {
                format!("Result<{}, String>", self.rust_type(inner_type, _name))
            }
            NailDataTypeDescriptor::Fn(_, _) => panic!("NailDataTypeDescriptor::Fn data type found during transpilation. This should not happen."),
            NailDataTypeDescriptor::OneOf(_) => panic!("NailDataTypeDescriptor::OneOf found during transpilation. This should not happen."),
            NailDataTypeDescriptor::HashMap(key_type, value_type) => {
                format!("DashMap<{}, {}>", self.rust_type(key_type, _name), self.rust_type(value_type, _name))
            }
            NailDataTypeDescriptor::FailedToResolve => panic!("NailDataTypeDescriptor::FailedToResolve found during transpilation. This should not happen."),
        }
    }

    fn rust_async_return_type(&self, data_type: &NailDataTypeDescriptor, name: &str) -> String {
        format!("{}", self.rust_type(data_type, name))
    }

    fn rust_operator(&self, op: &Operation) -> &'static str {
        match op {
            Operation::Add => "+",
            Operation::Sub => "-",
            Operation::Mul => "*",
            Operation::Div => "/",
            Operation::Mod => "%",
            Operation::Eq => "==",
            Operation::Ne => "!=",
            Operation::Lt => "<",
            Operation::Lte => "<=",
            Operation::Gt => ">",
            Operation::Gte => ">=",
            Operation::Not => "!",
            Operation::Neg => "-",
            Operation::And => "&&",
            Operation::Or => "||",
        }
    }

    fn indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }


    fn transpile_function_call(&mut self, name: &str, args: &[ASTNode], output: &mut String, add_indent: bool) -> Result<(), std::fmt::Error> {
        // Special handling for error-related functions
        if name == "e" {
            // e(message) - create an error with context
            if args.len() != 1 {
                return Err(std::fmt::Error);
            }

            if add_indent {
                write!(output, "{}", self.indent())?;
            }

            // Add function context to error messages
            write!(output, "Err(format!(\"[{}] {{}}\", ", self.current_function_name.as_ref().unwrap_or(&"unknown".to_string()))?;
            self.transpile_node_internal(&args[0], output, false)?;
            write!(output, "))")?;

            if add_indent {
                writeln!(output, ";")?;
            }
            return Ok(());
        } else if name == "safe" {
            // safe(expression, |e|:T { handler })
            if args.len() != 2 {
                return Err(std::fmt::Error);
            }

            if add_indent {
                write!(output, "{}", self.indent())?;
            }

            // Generate: match expression { Ok(v) => v, Err(e) => (handler)(e).await }
            write!(output, "match ")?;
            self.transpile_node_internal(&args[0], output, false)?;
            write!(output, " {{ Ok(v) => v, Err(e) => (")?;

            // The second argument should be a lambda
            self.transpile_node_internal(&args[1], output, false)?;
            write!(output, ")(e).await }}")?;

            if add_indent {
                writeln!(output, ";")?;
            }
            return Ok(());
        } else if name == "danger" {
            // danger(expression) - unwrap with a custom panic message
            if args.len() != 1 {
                return Err(std::fmt::Error);
            }

            if add_indent {
                write!(output, "{}", self.indent())?;
            }

            // Check if the argument is a function call that needs .await
            if let ASTNode::FunctionCall { name: inner_name, args: inner_args, .. } = &args[0] {
                // It's a function call - transpile it properly with .await if needed
                self.transpile_function_call(inner_name, inner_args, output, false)?;
            } else {
                // Not a function call, transpile normally
                self.transpile_node_internal(&args[0], output, false)?;
            }
            write!(output, ".unwrap_or_else(|nail_error| panic!(\"🔨 Nail Error: {{}}\", nail_error))")?;

            if add_indent {
                writeln!(output, ";")?;
            }
            return Ok(());
        } else if name == "expect" {
            // expect(expression) - semantically identical to danger but with different intent
            if args.len() != 1 {
                return Err(std::fmt::Error);
            }

            if add_indent {
                write!(output, "{}", self.indent())?;
            }

            // Check if the argument is a function call that needs .await
            if let ASTNode::FunctionCall { name: inner_name, args: inner_args, .. } = &args[0] {
                // It's a function call - transpile it properly with .await if needed
                self.transpile_function_call(inner_name, inner_args, output, false)?;
            } else {
                // Not a function call, transpile normally
                self.transpile_node_internal(&args[0], output, false)?;
            }
            write!(output, ".unwrap_or_else(|nail_error| panic!(\"🔨 Nail Error: {{}}\", nail_error))")?;

            if add_indent {
                writeln!(output, ";")?;
            }
            return Ok(());
        }

        // Check if it's a stdlib function
        if let Some(stdlib_fn) = stdlib_registry::get_stdlib_function(name) {
            // Track that we're using this stdlib function
            self.used_stdlib_functions.insert(name.to_string());
            // All stdlib functions are regular function calls now
            if add_indent {
                write!(output, "{}{}", self.indent(), stdlib_fn.rust_path)?;
            } else {
                write!(output, "{}", stdlib_fn.rust_path)?;
            }

            // Special case for macros
            if stdlib_fn.rust_path.ends_with("!") {
                write!(output, "(")?;
                // For print, we need to handle the macro call format
                if name == "print" {
                    // print_macro!(arg1, arg2, arg3)
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            write!(output, ", ")?;
                        }
                        self.transpile_node_internal(arg, output, false)?;
                    }
                } else {
                    // Other macros
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            write!(output, ", ")?;
                        }
                        self.transpile_node_internal(arg, output, false)?;
                    }
                }
                write!(output, ")")?;
            } else {
                // Regular functions
                write!(output, "(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ")?;
                    }
                    
                    // Check if this parameter should be passed by reference
                    let pass_by_ref = if i < stdlib_fn.parameters.len() {
                        stdlib_fn.parameters[i].pass_by_reference
                    } else {
                        false
                    };
                    
                    if pass_by_ref {
                        // Pass by reference
                        if let ASTNode::Identifier { name, .. } = arg {
                            write!(output, "&{}", name)?;
                        } else {
                            write!(output, "&")?;
                            self.transpile_node_internal(arg, output, false)?;
                        }
                    } else {
                        self.transpile_node_internal(arg, output, false)?;
                    }
                }

                write!(output, ")")?;
                
                // Add .await for all non-macro functions (everything is async in Nail)
                if stdlib_registry::get_stdlib_function(name).map(|f| !f.rust_path.ends_with("!")).unwrap_or(false) {
                    write!(output, ".await")?;
                }

                // Note: Result types should be handled by the type checker and
                // explicit error handling functions like danger() or safe().
                // The transpiler should not automatically unwrap Results.
            }
            if add_indent {
                writeln!(output, ";")?;
            }
        } else {
            // User-defined function - ALL Nail functions are async
            if add_indent {
                write!(output, "{}", self.indent())?;
            }
            
            // Check if this is a recursive call
            let is_recursive = self.current_function_name.as_ref()
                .map(|current| current == name)
                .unwrap_or(false);
            
            if is_recursive {
                // Wrap recursive calls in Box::pin to avoid infinite-sized futures
                write!(output, "Box::pin({}(", name)?;
            } else {
                write!(output, "{}(", name)?;
            }
            
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    write!(output, ", ")?;
                }
                self.transpile_node_internal(arg, output, false)?;
            }
            
            if is_recursive {
                write!(output, ")).await")?;
            } else {
                write!(output, ").await")?;
            }
            
            if add_indent {
                writeln!(output, ";")?;
            }
        }

        Ok(())
    }

    fn transpile_parallel_block(&mut self, statements: &[ASTNode], output: &mut String) -> Result<(), std::fmt::Error> {
        if statements.is_empty() {
            return Ok(());
        }

        // Extract variable names from const declarations and generate expressions
        let mut var_names = Vec::new();
        let mut expressions = Vec::new();

        for stmt in statements.iter() {
            match stmt {
                ASTNode::ConstDeclaration { name, value, .. } => {
                    var_names.push(name.clone());
                    expressions.push(value.as_ref());
                }
                ASTNode::FunctionCall { .. } => {
                    // Function calls that don't assign to variables get a placeholder name
                    var_names.push("_".to_string());
                    expressions.push(stmt);
                }
                _ => {
                    // Other statements get placeholder names
                    var_names.push("_".to_string());
                    expressions.push(stmt);
                }
            }
        }

        // Generate the destructuring assignment with actual variable names
        write!(output, "{}let (", self.indent())?;
        for (i, var_name) in var_names.iter().enumerate() {
            if i > 0 {
                write!(output, ", ")?;
            }
            write!(output, "{}", var_name)?;
        }
        writeln!(output, ") = tokio::join!(")?;

        // Generate the async blocks that return the computed values
        self.indent_level += 1;
        for (i, expr) in expressions.iter().enumerate() {
            if i > 0 {
                writeln!(output, ",")?;
            }
            write!(output, "{}async {{ ", self.indent())?;

            // For const declarations, return just the value expression
            // For other expressions, return the expression itself
            self.transpile_node_internal(expr, output, false)?;

            write!(output, " }}")?;
        }
        self.indent_level -= 1;
        writeln!(output)?;
        writeln!(output, "{});", self.indent())?;

        Ok(())
    }

    fn transpile_parallel_assignment(&mut self, assignments: &[(String, NailDataTypeDescriptor, Box<ASTNode>)], output: &mut String) -> Result<(), std::fmt::Error> {
        if assignments.is_empty() {
            return Ok(());
        }

        // Generate the destructuring assignment with actual variable names
        write!(output, "{}let (", self.indent())?;
        for (i, (var_name, _, _)) in assignments.iter().enumerate() {
            if i > 0 {
                write!(output, ", ")?;
            }
            write!(output, "{}", var_name)?;
        }
        writeln!(output, ") = tokio::join!(")?;

        // Generate the async blocks that return the computed values
        self.indent_level += 1;
        for (i, (_, _, value)) in assignments.iter().enumerate() {
            if i > 0 {
                writeln!(output, ",")?;
            }
            write!(output, "{}async {{ ", self.indent())?;

            // Return the value expression
            self.transpile_node_internal(value, output, false)?;

            write!(output, " }}")?;
        }
        self.indent_level -= 1;
        writeln!(output)?;
        writeln!(output, "{});", self.indent())?;

        Ok(())
    }

    /// Check if a statement contains a diverging function call (like panic or todo)
    fn statement_contains_diverging_call(&self, stmt: &ASTNode) -> bool {
        match stmt {
            ASTNode::FunctionCall { name, .. } => {
                if let Some(stdlib_fn) = stdlib_registry::get_stdlib_function(name) {
                    stdlib_fn.diverging
                } else {
                    false
                }
            }
            ASTNode::ReturnDeclaration { statement, .. } => {
                self.statement_contains_diverging_call(statement)
            }
            ASTNode::YieldDeclaration { statement, .. } => {
                self.statement_contains_diverging_call(statement)
            }
            // Check inside blocks for the last statement
            ASTNode::Block { statements, .. } => {
                statements.last()
                    .map(|s| self.statement_contains_diverging_call(s))
                    .unwrap_or(false)
            }
            _ => false
        }
    }
    
}

fn insert_semicolons(code: String) -> String {
    let lines: Vec<&str> = code.lines().collect();
    let mut result = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let mut new_line = line.to_string();

        // check if the line ends with an await, .to_string(), or a number with a white space or a \n after it and add a ; in that case or if ends with a number
        if trimmed.ends_with("await") || trimmed.ends_with(".to_string()") || trimmed.chars().last().unwrap_or_default().is_ascii_digit() {
            // or if it ends with a number ||
            let next_line = lines.get(i + 1).unwrap_or(&"");
            if next_line.trim().is_empty() || next_line.trim().starts_with("//") {
                new_line.push(';');
            }
        }

        result.push(new_line);
    }

    result.join("\n")
}
