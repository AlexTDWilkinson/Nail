// Comprehensive parser tests for Nail language

use Nail::lexer::*;
use Nail::parser::*;
use Nail::common::*;

/// Helper function to parse code and return AST or errors
fn parse_code(code: &str) -> Result<(ASTNode, std::collections::HashSet<String>), CodeError> {
    let tokens = lexer(code);
    parse(tokens)
}

#[test]
fn test_simple_function() {
    let code = "f test():i { r 42; }";
    let result = parse_code(code);
    
    assert!(result.is_ok(), "Failed to parse simple function: {:?}", result);
    
    let (ast, _) = result.unwrap();
    println!("Simple function AST: {:?}", ast);
    
    // Check it's a program with a function
    match ast {
        ASTNode::Program { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                ASTNode::FunctionDeclaration { name, .. } => {
                    assert_eq!(name, "test");
                }
                _ => panic!("Expected function node"),
            }
        }
        _ => panic!("Expected program node"),
    }
}

#[test]
fn test_function_with_parameters() {
    let code = "f add(x:i, y:i):i { r x + y; }";
    let result = parse_code(code);
    
    assert!(result.is_ok(), "Failed to parse function with parameters: {:?}", result);
    
    let (ast, _) = result.unwrap();
    println!("Function with params AST: {:?}", ast);
    
    match ast {
        ASTNode::Program { statements, .. } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                ASTNode::FunctionDeclaration { name, params, .. } => {
                    assert_eq!(name, "add");
                    assert_eq!(params.len(), 2);
                }
                _ => panic!("Expected function node"),
            }
        }
        _ => panic!("Expected program node"),
    }
}

#[test]
fn test_variable_declaration() {
    let code = "x_value:i = 42;";
    let result = parse_code(code);
    
    assert!(result.is_ok(), "Failed to parse variable declaration: {:?}", result);
    
    let (ast, _) = result.unwrap();
    println!("Variable declaration AST: {:?}", ast);
}

#[test]
fn test_binary_operations() {
    let test_cases = vec![
        "x + y",
        "x - y", 
        "x * y",
        "x / y",
        "x % y",
        "x == y",
        "x != y",
        "x < y",
        "x > y",
        "x <= y",
        "x >= y",
    ];
    
    for expr in test_cases {
        let code = format!("r {};", expr);
        let result = parse_code(&code);
        
        assert!(result.is_ok(), "Failed to parse binary operation '{}': {:?}", expr, result);
        
        let (ast, _) = result.unwrap();
        println!("Binary operation '{}' AST: {:?}", expr, ast);
    }
}

#[test]
fn test_boolean_literals() {
    let test_cases = vec!["true", "false"];
    
    for literal in test_cases {
        let code = format!("r {};", literal);
        let result = parse_code(&code);
        
        println!("Boolean literal '{}' result: {:?}", literal, result);
        
        // For now, just check if it parses without crashing
        // We'll see what the actual behavior is
        if let Ok((ast, _)) = result {
            println!("Boolean literal '{}' AST: {:?}", literal, ast);
        }
    }
}

#[test]
fn test_function_calls() {
    let code = "test(1, 2, 3)";
    let result = parse_code(&format!("r {};", code));
    
    assert!(result.is_ok(), "Failed to parse function call: {:?}", result);
    
    let (ast, _) = result.unwrap();
    println!("Function call AST: {:?}", ast);
}

#[test]
fn test_nested_expressions() {
    let code = "r (x + y) * (a - b);";
    let result = parse_code(code);
    
    assert!(result.is_ok(), "Failed to parse nested expressions: {:?}", result);
    
    let (ast, _) = result.unwrap();
    println!("Nested expressions AST: {:?}", ast);
}

#[test]
fn test_comparison_in_function() {
    let code = "f is_even(n:i):b { r n % 2 == 0; }";
    let result = parse_code(code);
    
    println!("Comparison function result: {:?}", result);
    
    if let Ok((ast, _)) = result {
        println!("Comparison function AST: {:?}", ast);
        
        // Check the structure
        match ast {
            ASTNode::Program { statements, .. } => {
                assert_eq!(statements.len(), 1);
                match &statements[0] {
                    ASTNode::FunctionDeclaration { name, data_type, body, .. } => {
                        assert_eq!(name, "is_even");
                        println!("Return type: {:?}", data_type);
                        println!("Body: {:?}", body);
                    }
                    _ => panic!("Expected function node"),
                }
            }
            _ => panic!("Expected program node"),
        }
    }
}

#[test]
fn test_type_annotations() {
    let test_cases = vec![
        ("x:i", "integer"),
        ("x:s", "string"),
        ("x:b", "boolean"),
    ];
    
    for (decl, desc) in test_cases {
        let code = format!("{} = 0;", decl);
        let result = parse_code(&code);
        
        println!("Type annotation {} ({}) result: {:?}", decl, desc, result);
        
        if let Ok((ast, _)) = result {
            println!("Type annotation {} AST: {:?}", decl, ast);
        }
    }
}

#[test]
fn test_multiple_statements() {
    let code = r#"
f test():i {
    x_val:i = 5;
    y_val:i = 10;
    r x_val + y_val;
}
"#;
    let result = parse_code(code);
    
    assert!(result.is_ok(), "Failed to parse multiple statements: {:?}", result);
    
    let (ast, _) = result.unwrap();
    println!("Multiple statements AST: {:?}", ast);
}

#[test]
fn test_string_literals() {
    let code = "r `hello world`;";
    let result = parse_code(code);
    
    assert!(result.is_ok(), "Failed to parse string literal: {:?}", result);
    
    let (ast, _) = result.unwrap();
    println!("String literal AST: {:?}", ast);
}

#[test]
fn test_error_cases() {
    let error_cases = vec![
        "f test() {",  // Missing closing brace
        "x_val:i =;",       // Missing expression
        "f ():i {}",   // Missing function name
        "+ 5",          // Invalid expression start
    ];
    
    for bad_code in error_cases {
        let result = parse_code(bad_code);
        println!("Error case '{}' result: {:?}", bad_code, result);
        
        // These should either error or handle gracefully
        assert!(result.is_err(), "Expected error for bad code: '{}'", bad_code);
    }
}

#[test]
fn test_operator_precedence() {
    let test_cases = vec![
        "1 + 2 * 3",     // Should be 1 + (2 * 3)
        "2 * 3 + 1",     // Should be (2 * 3) + 1
        "1 == 2 + 3",    // Should be 1 == (2 + 3)
        "x < y + z",     // Should be x < (y + z)
    ];
    
    for expr in test_cases {
        let code = format!("r {};", expr);
        let result = parse_code(&code);
        
        println!("Precedence test '{}' result: {:?}", expr, result);
        
        if let Ok((ast, _)) = result {
            println!("Precedence '{}' AST: {:?}", expr, ast);
        }
    }
}

#[test]
fn test_return_statements() {
    let test_cases = vec![
        "r 42;",
        "r x + y;",
        "r test();",
        "r n % 2 == 0;",
    ];
    
    for ret_stmt in test_cases {
        let result = parse_code(ret_stmt);
        
        println!("Return statement '{}' result: {:?}", ret_stmt, result);
        
        if let Ok((ast, _)) = result {
            println!("Return statement '{}' AST: {:?}", ret_stmt, ast);
        }
    }
}