use crate::lexer::{NailDataTypeDescriptor, Operation};
use crate::parser::ASTNode;
use crate::stdlib_registry;
use std::fmt::Write;

pub struct Transpiler {
    indent_level: usize,
    scope_level: usize,
    current_function_return_type: Option<NailDataTypeDescriptor>,
    current_function_name: Option<String>,
}

impl Transpiler {
    pub fn new() -> Self {
        Transpiler { 
            indent_level: 0, 
            scope_level: 0,
            current_function_return_type: None,
            current_function_name: None,
        }
    }

    pub fn transpile(&mut self, node: &ASTNode) -> Result<String, std::fmt::Error> {
        let mut output = String::new();
        writeln!(output, "use tokio;")?;
        writeln!(output, "use Nail::std_lib;")?;
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
                write!(output, "{}fn {}(", self.indent(), name)?;
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
                write!(output, "{}let {}: {} = ", self.indent(), name, self.rust_type(data_type, name))?;
                self.transpile_node_internal(value, output, false)?;
                writeln!(output, ";")?;
            }
            ASTNode::IfStatement { condition_branches, else_branch, .. } => {
                // Check if this is being used as an expression or statement
                // If add_semicolons is false, we're in expression context
                if !add_semicolons {
                    // Expression context - generate Rust if expression
                    for (i, (condition, branch)) in condition_branches.iter().enumerate() {
                        if i == 0 {
                            write!(output, "if ")?;
                        } else {
                            write!(output, " else if ")?;
                        }
                        self.transpile_node_internal(condition, output, false)?;
                        write!(output, " {{ ")?;
                        // For expressions, we need to extract the return value from the block
                        if let ASTNode::Block { statements, .. } = branch.as_ref() {
                            if let Some(last) = statements.last() {
                                if let ASTNode::ReturnDeclaration { statement, .. } = last {
                                    self.transpile_node_internal(statement, output, false)?;
                                } else {
                                    // If no explicit return, the last expression is the value
                                    self.transpile_node_internal(last, output, false)?;
                                }
                            }
                        }
                        write!(output, " }}")?;
                    }
                    if let Some(branch) = else_branch {
                        write!(output, " else {{ ")?;
                        if let ASTNode::Block { statements, .. } = branch.as_ref() {
                            if let Some(last) = statements.last() {
                                if let ASTNode::ReturnDeclaration { statement, .. } = last {
                                    self.transpile_node_internal(statement, output, false)?;
                                } else {
                                    self.transpile_node_internal(last, output, false)?;
                                }
                            }
                        }
                        write!(output, " }}")?;
                    }
                } else {
                    // Statement context - generate regular if statement
                    for (i, (condition, branch)) in condition_branches.iter().enumerate() {
                        if i == 0 {
                            write!(output, "{}if ", self.indent())?;
                        } else {
                            write!(output, "{}else if ", self.indent())?;
                        }
                        self.transpile_node_internal(condition, output, false)?;
                        writeln!(output, " {{")?;
                        self.indent_level += 1;
                        self.transpile_node_internal(branch, output, add_semicolons)?;
                        self.indent_level -= 1;
                        writeln!(output, "{}}}", self.indent())?;
                    }
                    if let Some(branch) = else_branch {
                        writeln!(output, "{}else {{", self.indent())?;
                        self.indent_level += 1;
                        self.transpile_node_internal(branch, output, add_semicolons)?;
                        self.indent_level -= 1;
                        writeln!(output, "{}}}", self.indent())?;
                    }
                }
            }
            ASTNode::Block { statements, .. } => {
                for stmt in statements {
                    self.transpile_node_internal(stmt, output, add_semicolons)?;
                }
            }
            ASTNode::ParallelBlock { statements, .. } => {
                self.transpile_parallel_block(statements, output)?;
            }
            ASTNode::ParallelAssignment { assignments, .. } => {
                self.transpile_parallel_assignment(assignments, output)?;
            }
            ASTNode::BinaryOperation { left, operator, right, .. } => {
                self.transpile_node_internal(left, output, false)?;
                write!(output, " {} ", self.rust_operator(operator))?;
                self.transpile_node_internal(right, output, false)?;
            }
            ASTNode::UnaryOperation { operator, operand, .. } => {
                write!(output, "{}", self.rust_operator(operator))?;
                self.transpile_node_internal(operand, output, false)?;
            }
            ASTNode::Identifier { name, .. } => {
                write!(output, "{}", name)?;
            }
            ASTNode::NumberLiteral { value, .. } => {
                write!(output, "{}", value)?;
            }
            ASTNode::StringLiteral { value, .. } => {
                write!(output, "\"{}\".to_string()", value.replace("\"", "\\\""))?;
            }

            ASTNode::ReturnDeclaration { statement, .. } => {
                write!(output, "{}return ", self.indent())?;
                
                // Check if we need to wrap in Ok() for result types
                let needs_ok_wrap = if let Some(return_type) = &self.current_function_return_type {
                    match return_type {
                        NailDataTypeDescriptor::Any(types) => {
                            types.len() == 2 && types[1] == NailDataTypeDescriptor::Error
                        }
                        _ => false
                    }
                } else {
                    false
                };
                
                // Check if the statement is already an error (e() call)
                let is_error_call = match statement.as_ref() {
                    ASTNode::FunctionCall { name, .. } => name == "e",
                    _ => false
                };
                
                if needs_ok_wrap && !is_error_call {
                    write!(output, "Ok(")?;
                    self.transpile_node_internal(statement, output, false)?;
                    write!(output, ")")?;
                } else {
                    self.transpile_node_internal(statement, output, false)?;
                }
            }
            ASTNode::StructDeclaration { name, fields, .. } => {
                writeln!(output, "{}#[derive(Debug, Clone)]", self.indent())?;
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
                writeln!(output, "{}#[derive(Debug, PartialEq)]", self.indent())?;
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
            ASTNode::LambdaDeclaration { params, data_type, body, .. } => {
                // Lambdas should be inline, not formatted with newlines
                write!(output, "|")?;
                for (i, (param_name, param_type)) in params.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ")?;
                    }
                    write!(output, "{}: {}", param_name, self.rust_type(param_type, ""))?;
                }
                write!(output, "| -> {} {{ ", self.rust_type(data_type, ""))?;
                
                // Transpile the body inline
                if let ASTNode::Block { statements, .. } = body.as_ref() {
                    for (i, stmt) in statements.iter().enumerate() {
                        if i > 0 {
                            write!(output, "; ")?;
                        }
                        self.transpile_node_internal(stmt, output, false)?;
                    }
                }
                
                write!(output, " }}")?;
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
            ASTNode::EnumVariant { name, variant, .. } => {
                write!(output, "{}{}::{}", self.indent(), name, variant)?;
            }
            ASTNode::ArrayLiteral { elements, .. } => {
                write!(output, "vec! [")?;
                for (i, value) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ")?;
                    }
                    // Clone identifiers in arrays to avoid ownership issues
                    if let ASTNode::Identifier { .. } = value {
                        self.transpile_node_internal(value, output, false)?;
                        write!(output, ".clone()")?;
                    } else {
                        self.transpile_node_internal(value, output, false)?;
                    }
                }
                write!(output, "]")?;
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
            NailDataTypeDescriptor::Error => "String".to_string(),
            NailDataTypeDescriptor::ArrayInt => "Vec<i64>".to_string(),
            NailDataTypeDescriptor::ArrayFloat => "Vec<f64>".to_string(),
            NailDataTypeDescriptor::ArrayString => "Vec<String>".to_string(),
            NailDataTypeDescriptor::ArrayBoolean => "Vec<bool>".to_string(),
            NailDataTypeDescriptor::ArrayStruct(name) => format!("Vec<{}>", name),
            NailDataTypeDescriptor::ArrayEnum(name) => format!("Vec<{}>", name),
            NailDataTypeDescriptor::Any(types) => {
                // Handle result types (base_type|e)
                if types.len() == 2 && types[1] == NailDataTypeDescriptor::Error {
                    // It's a result type
                    format!("Result<{}, String>", self.rust_type(&types[0], _name))
                } else {
                    panic!("Unsupported Any type combination during transpilation")
                }
            },
            NailDataTypeDescriptor::Fn(_, _) => panic!("NailDataTypeDescriptor::Fn data type found during transpilation. This should not happen."),
            NailDataTypeDescriptor::Unknown => panic!("NailDataTypeDescriptor::Unknown data type found during transpilation. This should not happen."),
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
            write!(output, "Err(format!(\"[{}] {{}}\", ", 
                self.current_function_name.as_ref().unwrap_or(&"unknown".to_string()))?;
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
            
            // Generate: match expression { Ok(v) => v, Err(e) => (handler)(e) }
            write!(output, "match ")?;
            self.transpile_node_internal(&args[0], output, false)?;
            write!(output, " {{ Ok(v) => v, Err(e) => (")?;
            
            // The second argument should be a lambda
            self.transpile_node_internal(&args[1], output, false)?;
            write!(output, ")(e) }}")?;
            
            if add_indent {
                writeln!(output, ";")?;
            }
            return Ok(());
        } else if name == "dangerous" {
            // dangerous(expression) - unwrap with a custom panic message
            if args.len() != 1 {
                return Err(std::fmt::Error);
            }
            
            if add_indent {
                write!(output, "{}", self.indent())?;
            }
            
            // Generate: expression.unwrap_or_else(|e| panic!("Nail Error: {}", e))
            self.transpile_node_internal(&args[0], output, false)?;
            write!(output, ".unwrap_or_else(|nail_error| panic!(\"🔨 Nail Error: {{}}\", nail_error))")?;
            
            if add_indent {
                writeln!(output, ";")?;
            }
            return Ok(());
        } else if name == "expect" {
            // expect(expression) - semantically identical to dangerous but with different intent
            if args.len() != 1 {
                return Err(std::fmt::Error);
            }
            
            if add_indent {
                write!(output, "{}", self.indent())?;
            }
            
            // Generate: expression.unwrap_or_else(|e| panic!("Nail Error: {}", e))
            self.transpile_node_internal(&args[0], output, false)?;
            write!(output, ".unwrap_or_else(|nail_error| panic!(\"🔨 Nail Error: {{}}\", nail_error))")?;
            
            if add_indent {
                writeln!(output, ";")?;
            }
            return Ok(());
        }
        
        // Check if it's a stdlib function
        if let Some(stdlib_fn) = stdlib_registry::get_stdlib_function(name) {
            // All stdlib functions are regular function calls now
            if add_indent {
                write!(output, "{}{}", self.indent(), stdlib_fn.rust_path)?;
            } else {
                write!(output, "{}", stdlib_fn.rust_path)?;
            }

            // Special case for macros
            if stdlib_fn.rust_path.ends_with("!") {
                write!(output, "(")?;
                // For print, we need to handle format strings
                if name == "print" {
                    if args.is_empty() {
                        write!(output, "\"\"")?;
                    } else {
                        // Use pretty-print debug format for automatic printing of any type
                        write!(output, "\"{{:#?}}\", ")?;
                        self.transpile_node_internal(&args[0], output, false)?;
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
                    // Clone identifiers for stdlib functions since they take ownership
                    if let ASTNode::Identifier { .. } = arg {
                        self.transpile_node_internal(arg, output, false)?;
                        write!(output, ".clone()")?;
                    } else {
                        self.transpile_node_internal(arg, output, false)?;
                    }
                }
                write!(output, ")")?;

                if stdlib_fn.is_async {
                    write!(output, ".await")?;
                }

                // For now, unwrap all Results
                // TODO: Proper error handling
                if name.starts_with("http_") || name.starts_with("fs_") || name.starts_with("env_") || name.starts_with("process_run") || name == "to_int" || name == "to_float" {
                    write!(output, ".unwrap()")?;
                }
            }
            if add_indent {
                writeln!(output, ";")?;
            }
        } else {
            // User-defined function
            write!(output, "{}{}(", self.indent(), name)?;
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    write!(output, ", ")?;
                }
                // Clone identifiers to avoid ownership issues
                if let ASTNode::Identifier { .. } = arg {
                    self.transpile_node_internal(arg, output, false)?;
                    write!(output, ".clone()")?;
                } else {
                    self.transpile_node_internal(arg, output, false)?;
                }
            }
            write!(output, ")")?;
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
