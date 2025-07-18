// Comprehensive type checker tests for Nail language

use Nail::checker::*;
use Nail::common::*;
use Nail::lexer::*;
use Nail::parser::*;

/// Helper function to run full pipeline: code -> tokens -> AST -> type check
fn check_code(code: &str) -> Result<(), CodeError> {
    // Lexer
    let tokens = lexer(code);

    // Parser
    let mut ast = parse(tokens)?;

    // Type checker
    checker(&mut ast).map_err(|errors| if errors.is_empty() { CodeError { message: "Unknown checker error".to_string(), code_span: CodeSpan::default() } } else { errors[0].clone() })
}

#[test]
fn test_simple_integer_function() {
    let code = "fn test():i { r 42; }";
    let result = check_code(code);

    println!("Simple integer function result: {:?}", result);

    // This should work
    assert!(result.is_ok(), "Simple integer function should type check");
}

#[test]
fn test_integer_arithmetic() {
    let test_cases = vec!["fn test():i { r 1 + 2; }", "fn test():i { r 5 - 3; }", "fn test():i { r 2 * 4; }", "fn test():i { r 8 / 2; }", "fn test():i { r 7 % 3; }"];

    for code in test_cases {
        let result = check_code(code);
        println!("Arithmetic test '{}' result: {:?}", code, result);

        assert!(result.is_ok(), "Arithmetic should type check: {}", code);
    }
}

#[test]
fn test_integer_function_with_params() {
    let code = "fn add(x:i, y:i):i { r x + y; }";
    let result = check_code(code);

    println!("Function with params result: {:?}", result);

    assert!(result.is_ok(), "Function with parameters should type check");
}

#[test]
fn test_variable_declarations() {
    let test_cases = vec!["x_val:i = 42;", "y_val:i = 1 + 2;"];

    for code in test_cases {
        let result = check_code(code);
        println!("Variable declaration '{}' result: {:?}", code, result);

        assert!(result.is_ok(), "Variable declaration should type check: {}", code);
    }
}

#[test]
fn test_comparison_operators_current_behavior() {
    // These tests verify that comparisons now correctly return Boolean (not Int)
    let test_cases = vec![
        ("fn test():i { r 5 == 5; }", "equality comparison"),
        ("fn test():i { r 5 != 3; }", "inequality comparison"),
        ("fn test():i { r 5 < 10; }", "less than comparison"),
        ("fn test():i { r 10 > 5; }", "greater than comparison"),
        ("fn test():i { r 5 <= 5; }", "less than or equal comparison"),
        ("fn test():i { r 5 >= 5; }", "greater than or equal comparison"),
    ];

    for (code, desc) in test_cases {
        let result = check_code(code);
        println!("Comparison test {} result: {:?}", desc, result);

        // Since we fixed the bug, comparisons return Boolean, so these should fail with type mismatch
        assert!(result.is_err(), "Comparison should fail when returning Boolean as Int: {}", desc);
    }
}

#[test]
fn test_comparison_operators_expected_behavior() {
    // These tests show what SHOULD happen (returning Boolean)
    let test_cases = vec![
        ("fn test():b { r 5 == 5; }", "equality comparison"),
        ("fn test():b { r 5 != 3; }", "inequality comparison"),
        ("fn test():b { r 5 < 10; }", "less than comparison"),
        ("fn test():b { r 10 > 5; }", "greater than comparison"),
        ("fn test():b { r 5 <= 5; }", "less than or equal comparison"),
        ("fn test():b { r 5 >= 5; }", "greater than or equal comparison"),
    ];

    for (code, desc) in test_cases {
        let result = check_code(code);
        println!("Boolean comparison test {} result: {:?}", desc, result);

        // These currently FAIL with type mismatch (expected Boolean, got Int)
        // This is the bug we need to fix
        if result.is_err() {
            println!("Expected failure for {}: {:?}", desc, result);
        }
    }
}

#[test]
fn test_the_original_problem() {
    // This is the exact case from the user's screenshot
    let code = "fn is_even_func(n:i):b { r n % 2 == 0; }";
    let result = check_code(code);

    println!("Original problem case result: {:?}", result);

    // This currently fails with "Type mismatch in return statement of function 'is_even_func': expected Boolean, got Int"
    match result {
        Ok(()) => println!("✅ Original problem is fixed!"),
        Err(error) => {
            println!("❌ Original problem still exists:");
            println!("  - {}", error.message);
        }
    }
}

#[test]
fn test_boolean_type_declaration() {
    // Test if we can declare boolean types at all
    let test_cases = vec![
        "fn test():b { r true; }", // If boolean literals work
        "x:b = true;",             // Boolean variable
    ];

    for code in test_cases {
        let result = check_code(code);
        println!("Boolean declaration test '{}' result: {:?}", code, result);

        // These may fail if boolean support is incomplete
    }
}

#[test]
fn test_type_mismatches() {
    // These should properly error with type mismatches
    let error_cases =
        vec![("fn test():i { r `hello`; }", "string return in int function"), ("fn test():s { r 42; }", "int return in string function"), ("fn add(x:i, y:s):i { r x + y; }", "adding int and string")];

    for (code, desc) in error_cases {
        let result = check_code(code);
        println!("Type mismatch test {} result: {:?}", desc, result);

        assert!(result.is_err(), "Should get type error for: {}", desc);
    }
}

#[test]
fn test_nested_expressions() {
    let test_cases = vec!["fn test():i { r (1 + 2) * 3; }", "fn test():i { r 1 + 2 * 3; }", "fn test():i { r (x + y) - (a * b); }"];

    for code in test_cases {
        let result = check_code(code);
        println!("Nested expression test '{}' result: {:?}", code, result);
    }
}

#[test]
fn test_function_calls() {
    // This would require multiple functions, might not work in current setup
    let code = r#"
fn helper():i { r 42; }
fn test():i { r helper(); }
"#;
    let result = check_code(code);
    println!("Function call test result: {:?}", result);
}

#[test]
fn test_string_operations() {
    let test_cases = vec![
        "fn test():s { r `hello`; }",
        // String concatenation if supported
        // "fn test():s { r `hello` + ` world`; }",
    ];

    for code in test_cases {
        let result = check_code(code);
        println!("String operation test '{}' result: {:?}", code, result);
    }
}

#[test]
fn test_edge_cases() {
    let test_cases = vec![
        "fn test():i { r 0; }",  // Zero
        "fn test():i { r -1; }", // Negative (if supported)
        "fn test():s { r ``; }", // Empty string
    ];

    for code in test_cases {
        let result = check_code(code);
        println!("Edge case test '{}' result: {:?}", code, result);
    }
}

#[test]
fn test_modulo_operation() {
    // Specifically test the modulo operation since that's part of the bug
    let test_cases = vec![
        "fn test():i { r 5 % 2; }", // Basic modulo
        "fn test():i { r n % 2; }", // Modulo with variable (might fail without parameter)
    ];

    for code in test_cases {
        let result = check_code(code);
        println!("Modulo test '{}' result: {:?}", code, result);
    }
}

#[test]
fn test_step_by_step_breakdown() {
    println!("=== STEP BY STEP BREAKDOWN ===");

    // Step 1: Basic integer works
    println!("STEP 1: Basic integer");
    let result1 = check_code("fn test():i { r 42; }");
    println!("Result: {:?}", result1);

    // Step 2: Basic arithmetic works
    println!("STEP 2: Basic arithmetic");
    let result2 = check_code("fn test():i { r 1 + 2; }");
    println!("Result: {:?}", result2);

    // Step 3: Modulo operation
    println!("STEP 3: Modulo operation");
    let result3 = check_code("fn test():i { r 5 % 2; }");
    println!("Result: {:?}", result3);

    // Step 4: Comparison (problematic)
    println!("STEP 4: Comparison returning Int (current behavior)");
    let result4 = check_code("fn test():i { r 5 == 5; }");
    println!("Result: {:?}", result4);

    // Step 5: The full problematic expression
    println!("STEP 5: Full expression n % 2 == 0 returning Int");
    let result5 = check_code("fn test(n:i):i { r n % 2 == 0; }");
    println!("Result: {:?}", result5);

    // Step 6: Boolean function declaration (might fail)
    println!("STEP 6: Boolean function declaration");
    let result6 = check_code("fn test():b { r true; }");
    println!("Result: {:?}", result6);

    // Step 7: The actual problem
    println!("STEP 7: The actual problem - Boolean return with comparison");
    let result7 = check_code("fn test(n:i):b { r n % 2 == 0; }");
    println!("Result: {:?}", result7);
}
