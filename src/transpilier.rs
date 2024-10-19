use crate::lexer::NailDataTypeDescriptor;
use crate::parser::ASTNode;

use crate::lexer::Operation;

use std::fmt::Write;

pub struct Transpiler {
    indent_level: usize,
    scope_level: usize,
}

impl Transpiler {
    pub fn new() -> Self {
        Transpiler { indent_level: 0, scope_level: 0 }
    }

    pub fn transpile(&mut self, node: &ASTNode) -> Result<String, std::fmt::Error> {
        let mut output = String::new();
        writeln!(output, "use tokio;")?;
        writeln!(output)?;
        writeln!(output, "#[tokio::main]")?;
        writeln!(output, "async fn main() {{")?;
        self.indent_level += 1;
        self.transpile_node(node, &mut output)?;
        self.indent_level -= 1;
        writeln!(output, "}}")?;
        let output = insert_semicolons(output);
        Ok(output)
    }

    fn transpile_node(&mut self, node: &ASTNode, output: &mut String) -> Result<(), std::fmt::Error> {
        match node {
            ASTNode::StructDeclarationField { .. } => todo!(),
            ASTNode::StructInstantiationField { .. } => todo!(),
            ASTNode::Program { statements, .. } => {
                for stmt in statements {
                    self.transpile_node(stmt, output)?;
                    writeln!(output)?;
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
                writeln!(output, ") -> {} {{", self.rust_async_return_type(data_type, name))?;
                self.indent_level += 1;
                self.transpile_node(body, output)?;
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;
            }
            ASTNode::FunctionCall { name, args, .. } => {
                write!(output, "{}{}(", self.indent(), name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ")?;
                    }
                    self.transpile_node(arg, output)?;
                }
                writeln!(output, ").await")?;
            }
            ASTNode::ConstDeclaration { name, data_type, value, .. } => {
                write!(output, "{}let {}: {} = ", self.indent(), name, self.rust_type(data_type, name))?;
                self.transpile_node(value, output)?;
                writeln!(output)?;
            }
            ASTNode::VariableDeclaration { name, data_type, value, .. } => {
                write!(output, "{}let mut {}: {} = ", self.indent(), name, self.rust_type(data_type, name))?;
                self.transpile_node(value, output)?;
                writeln!(output)?;
            }
            ASTNode::IfStatement { condition_branches, else_branch, .. } => {
                // turn into if else ifs
                for (i, (condition, branch)) in condition_branches.iter().enumerate() {
                    if i == 0 {
                        write!(output, "{}if ", self.indent())?;
                    } else {
                        write!(output, "{}else if ", self.indent())?;
                    }
                    self.transpile_node(condition, output)?;
                    writeln!(output, " {{")?;
                    self.indent_level += 1;
                    self.transpile_node(branch, output)?;
                    self.indent_level -= 1;
                    writeln!(output, "{}}}", self.indent())?;
                }
                if let Some(branch) = else_branch {
                    writeln!(output, "{}else {{", self.indent())?;
                    self.indent_level += 1;
                    self.transpile_node(branch, output)?;
                    self.indent_level -= 1;
                    writeln!(output, "{}}}", self.indent())?;
                }
            }
            ASTNode::Block { statements, .. } => {
                for stmt in statements {
                    self.transpile_node(stmt, output)?;
                }
            }
            ASTNode::BinaryOperation { left, operator, right, .. } => {
                self.transpile_node(left, output)?;
                write!(output, " {} ", self.rust_operator(operator))?;
                self.transpile_node(right, output)?;
            }
            ASTNode::UnaryOperation { operator, operand, .. } => {
                write!(output, "{}", self.rust_operator(operator))?;
                self.transpile_node(operand, output)?;
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
                self.transpile_node(statement, output)?;
            }
            ASTNode::StructDeclaration { name, fields, .. } => {
                // writeln!(output, "{}#[derive(Debug)]", self.indent())?;
                // writeln!(output, "{}struct {} {{", self.indent(), name)?;
                // self.indent_level += 1;
                // for (name, f) in fields {
                //     writeln!(output, "{}{}: {},", self.indent(), name, self.rust_type(field_type, ""))?;
                // }
                // self.indent_level -= 1;
                // writeln!(output, "{}}}", self.indent())?;
            }
            ASTNode::EnumDeclaration { name, variants, .. } => {
                writeln!(output, "{}#[derive(Debug)]", self.indent())?;
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
                write!(output, "{}|", self.indent())?;
                for (i, (param_name, param_type)) in params.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ")?;
                    }
                    write!(output, "{}: {}", param_name, self.rust_type(param_type, ""))?;
                }
                writeln!(output, "| -> {} {{", self.rust_async_return_type(data_type, ""))?;
                self.indent_level += 1;
                self.transpile_node(body, output)?;
                self.indent_level -= 1;
                writeln!(output, "{}}}", self.indent())?;
            }
            ASTNode::StructInstantiation { name, fields, .. } => {
                // write!(output, "{}{} {{", self.indent(), name)?;
                // for (i, (field_name, field_value)) in fields.iter().enumerate() {
                //     if i > 0 {
                //         write!(output, ", ")?;
                //     }
                //     write!(output, "{}: ", field_name)?;
                //     self.transpile_node(field_value, output)?;
                // }
                // writeln!(output, "}}")?;
            }
            ASTNode::EnumVariant { name, variant, .. } => {
                write!(output, "{}{}::{}", self.indent(), name, variant)?;
            }
            ASTNode::ArrayLiteral { elements, .. } => {
                write!(output, "{}vec! [", self.indent())?;
                for (i, value) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(output, ", ")?;
                    }
                    self.transpile_node(value, output)?;
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
            NailDataTypeDescriptor::Error => "Result<NailDataType, String>".to_string(),
            NailDataTypeDescriptor::ArrayInt => "Vec<i64>".to_string(),
            NailDataTypeDescriptor::ArrayFloat => "Vec<f64>".to_string(),
            NailDataTypeDescriptor::ArrayString => "Vec<String>".to_string(),
            NailDataTypeDescriptor::ArrayBoolean => "Vec<bool>".to_string(),
            NailDataTypeDescriptor::ArrayStruct(name) => format!("Vec<{}>", name),
            NailDataTypeDescriptor::ArrayEnum(name) => format!("Vec<{}>", name),
            NailDataTypeDescriptor::Any(_) => panic!("NailDataTypeDescriptor::Any data type found during transpilation. This should not happen."),
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
