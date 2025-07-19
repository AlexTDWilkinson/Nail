#[cfg(test)]
mod test_error_handling_transpilation {
    use Nail::lexer::*;
    use Nail::parser::*;
    use Nail::checker::*;
    use Nail::transpilier::Transpiler;
    use Nail::common::*;

    #[test]
    fn test_error_type_return_wrapping() {
        let nail_code = r#"
f divide(a:i, b:i):i!e {
    if {
        b == 0 => { r e(`Division by zero`); },
        else => { r a / b; }
    }
}

result:i = dangerous(divide(10, 2));
"#;

        // Lex
        let tokens = lexer(nail_code);
        
        // Parse
        let (mut ast, _) = parse(tokens).expect("Parsing failed");
        
        // Check
        checker(&mut ast).expect("Type checking failed");
        
        // Transpile
        let mut transpiler = Transpiler::new();
        let rust_code = transpiler.transpile(&ast).expect("Transpilation failed");
        
        println!("Generated Rust code:\n{}", rust_code);
        
        // Verify the transpiled code
        assert!(rust_code.contains("fn divide(a: i64, b: i64) -> Result<i64, String>"), 
                "Function signature should have Result return type");
        
        // Check that error case is handled correctly
        assert!(rust_code.contains("return Err(format!(\"[divide] {}\", \"Division by zero\".to_string()))"),
                "Error case should return Err");
        
        // Check that success case is wrapped in Ok
        assert!(rust_code.contains("return Ok(a / b)"),
                "Success case should be wrapped in Ok(), but found: {}", 
                rust_code);
    }

    #[test]
    fn test_simple_error_function() {
        let nail_code = r#"
f safe_divide(x:i, y:i):i!e {
    if {
        y == 0 => { r e(`Cannot divide by zero`); },
        else => { r x / y; }
    }
}
"#;

        let tokens = lexer(nail_code);
        let (mut ast, _) = parse(tokens).expect("Parsing failed");
        checker(&mut ast).expect("Type checking failed");
        let mut transpiler = Transpiler::new();
        let rust_code = transpiler.transpile(&ast).expect("Transpilation failed");
        
        // Should have Result type
        assert!(rust_code.contains("-> Result<i64, String>"));
        
        // Error case
        assert!(rust_code.contains("return Err("));
        
        // Success case should be wrapped
        assert!(rust_code.contains("return Ok(x / y)"), 
                "Expected 'return Ok(x / y)' but got: {}", rust_code);
    }

    #[test]
    fn test_nested_if_with_error_returns() {
        let nail_code = r#"
f complex_calc(a:i, b:i, c:i):i!e {
    if {
        a == 0 => { r e(`a cannot be zero`); },
        b == 0 => { r e(`b cannot be zero`); },
        else => {
            if {
                c > 10 => { r (a + b) * c; },
                else => { r a + b + c; }
            }
        }
    }
}
"#;

        let tokens = lexer(nail_code);
        let (mut ast, _) = parse(tokens).expect("Parsing failed");
        checker(&mut ast).expect("Type checking failed");
        let mut transpiler = Transpiler::new();
        let rust_code = transpiler.transpile(&ast).expect("Transpilation failed");
        
        // All non-error returns should be wrapped in Ok
        assert!(rust_code.contains("return Ok((a + b) * c)") || 
                rust_code.contains("Ok((a + b) * c)") ||
                rust_code.contains("return Ok(a + b * c)") ||
                rust_code.contains("Ok(a + b * c)"),
                "Nested return should be wrapped in Ok()");
        
        assert!(rust_code.contains("return Ok(a + b + c)") || 
                rust_code.contains("Ok(a + b + c)"),
                "Second nested return should be wrapped in Ok()");
    }
}