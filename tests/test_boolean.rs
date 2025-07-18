// Specific boolean type and comparison operator tests

use Nail::checker::*;
use Nail::common::*;
use Nail::lexer::*;
use Nail::parser::*;

/// Helper function to run full pipeline and return detailed results
fn test_boolean_pipeline(code: &str) -> BooleanTestResult {
    let mut result = BooleanTestResult::new(code);

    // Step 1: Lexer
    let tokens = lexer(code);
    result.tokens = Some(tokens.clone());

    // Step 2: Parser
    match parse(tokens) {
        Ok(mut ast) => {
            result.ast = Some(ast.clone());

            // Step 3: Type Checker
            match checker(&mut ast) {
                Ok(()) => {
                    result.checker_success = true;
                }
                Err(errors) => {
                    result.checker_errors = errors;
                }
            }
        }
        Err(error) => {
            result.parser_errors = vec![error];
        }
    }

    result
}

#[derive(Debug)]
struct BooleanTestResult {
    code: String,
    tokens: Option<Vec<Token>>,
    ast: Option<ASTNode>,
    lexer_errors: Vec<CodeError>,
    parser_errors: Vec<CodeError>,
    checker_errors: Vec<CodeError>,
    checker_success: bool,
}

impl BooleanTestResult {
    fn new(code: &str) -> Self {
        Self { code: code.to_string(), tokens: None, ast: None, lexer_errors: vec![], parser_errors: vec![], checker_errors: vec![], checker_success: false }
    }

    fn print_detailed(&self) {
        println!("=== BOOLEAN TEST RESULT ===");
        println!("Code: {}", self.code);
        println!("Lexer success: {}", self.lexer_errors.is_empty());
        println!("Parser success: {}", self.parser_errors.is_empty());
        println!("Checker success: {}", self.checker_success);

        if let Some(tokens) = &self.tokens {
            println!("Tokens ({}):", tokens.len());
            for (i, token) in tokens.iter().enumerate() {
                println!("  {}: {:?}", i, token.token_type);
            }
        }

        if !self.lexer_errors.is_empty() {
            println!("LEXER ERRORS:");
            for error in &self.lexer_errors {
                println!("  - {}", error.message);
            }
        }

        if !self.parser_errors.is_empty() {
            println!("PARSER ERRORS:");
            for error in &self.parser_errors {
                println!("  - {}", error.message);
            }
        }

        if !self.checker_errors.is_empty() {
            println!("CHECKER ERRORS:");
            for error in &self.checker_errors {
                println!("  - {}", error.message);
            }
        }

        if let Some(ast) = &self.ast {
            println!("AST: {:?}", ast);
        }

        println!("========================");
    }
}

#[test]
fn boolean_step_1_type_exists() {
    println!("BOOLEAN STEP 1: Test Boolean type exists");

    // Just check if we can create the type
    let bool_type = NailDataTypeDescriptor::Boolean;
    println!("Boolean type: {:?}", bool_type);

    assert_eq!(format!("{:?}", bool_type), "Boolean");
    println!("✅ Boolean type exists");
}

#[test]
fn boolean_step_2_boolean_literals() {
    println!("BOOLEAN STEP 2: Test boolean literals");

    let test_cases = vec!["true", "false"];

    for literal in test_cases {
        println!("Testing literal: {}", literal);
        let result = test_boolean_pipeline(literal);
        result.print_detailed();

        // Check what token type we get
        if let Some(tokens) = &result.tokens {
            if !tokens.is_empty() {
                println!("Literal '{}' tokenized as: {:?}", literal, tokens[0].token_type);
            }
        }
    }
}

#[test]
fn boolean_step_3_boolean_type_annotation() {
    println!("BOOLEAN STEP 3: Test boolean type annotation");

    let code = "x:b";
    let result = test_boolean_pipeline(code);
    result.print_detailed();

    // Check if 'b' is recognized as a type
    if let Some(tokens) = &result.tokens {
        for token in tokens {
            if let TokenType::Identifier(ref name) = token.token_type {
                if name == "b" {
                    println!("Type 'b' tokenized as: {:?}", token.token_type);
                }
            }
        }
    }
}

#[test]
fn boolean_step_4_simple_boolean_function() {
    println!("BOOLEAN STEP 4: Test simple boolean function");

    let code = "fn test():b { r true; }";
    let result = test_boolean_pipeline(code);
    result.print_detailed();

    if result.checker_success {
        println!("✅ Simple boolean function works!");
    } else {
        println!("❌ Simple boolean function fails");
        if !result.checker_errors.is_empty() {
            println!("Specific errors:");
            for error in &result.checker_errors {
                println!("  - {}", error.message);
            }
        }
    }
}

#[test]
fn boolean_step_5_integer_comparison() {
    println!("BOOLEAN STEP 5: Test integer comparison");

    let code = "5 == 5";
    let result = test_boolean_pipeline(code);
    result.print_detailed();

    // Check what the parser thinks this expression is
    if let Some(ast) = &result.ast {
        println!("Comparison AST structure: {:?}", ast);
    }
}

#[test]
fn boolean_step_6_comparison_in_function_returning_int() {
    println!("BOOLEAN STEP 6: Test comparison in function returning Int (current behavior)");

    let code = "fn test():i { r 5 == 5; }";
    let result = test_boolean_pipeline(code);
    result.print_detailed();

    if result.checker_success {
        println!("✅ Comparison returning Int works (current buggy behavior)");
    } else {
        println!("❌ Even comparison returning Int fails");
    }
}

#[test]
fn boolean_step_7_comparison_in_function_returning_boolean() {
    println!("BOOLEAN STEP 7: Test comparison in function returning Boolean (desired behavior)");

    let code = "fn test():b { r 5 == 5; }";
    let result = test_boolean_pipeline(code);
    result.print_detailed();

    if result.checker_success {
        println!("✅ Comparison returning Boolean works!");
    } else {
        println!("❌ Comparison returning Boolean fails (this is the bug)");
        if !result.checker_errors.is_empty() {
            println!("Error messages:");
            for error in &result.checker_errors {
                println!("  - {}", error.message);
            }
        }
    }
}

#[test]
fn boolean_step_8_the_exact_problem() {
    println!("BOOLEAN STEP 8: Test the exact problem case");

    let code = "fn is_even_func(n:i):b { r n % 2 == 0; }";
    let result = test_boolean_pipeline(code);
    result.print_detailed();

    if result.checker_success {
        println!("✅ THE PROBLEM IS FIXED!");
    } else {
        println!("❌ The problem still exists");
        if !result.checker_errors.is_empty() {
            println!("Exact error messages:");
            for error in &result.checker_errors {
                println!("  - {}", error.message);
            }
        }
    }
}

#[test]
fn boolean_step_9_all_comparison_operators() {
    println!("BOOLEAN STEP 9: Test all comparison operators");

    let operators = vec!["==", "!=", "<", ">", "<=", ">="];

    for op in operators {
        println!("Testing operator: {}", op);
        let code = format!("fn test():b {{ r 5 {} 3; }}", op);
        let result = test_boolean_pipeline(&code);

        println!("Operator {} result:", op);
        if result.checker_success {
            println!("  ✅ Works");
        } else {
            println!("  ❌ Fails");
            if !result.checker_errors.is_empty() {
                for error in &result.checker_errors {
                    println!("    - {}", error.message);
                }
            }
        }
    }
}

#[test]
fn boolean_step_10_isolated_binary_operation_check() {
    println!("BOOLEAN STEP 10: Isolated binary operation type checking");

    // Try to understand what check_type returns for different operations
    let test_cases = vec![
        ("5 + 3", "arithmetic"),
        ("5 - 3", "arithmetic"),
        ("5 * 3", "arithmetic"),
        ("5 / 3", "arithmetic"),
        ("5 % 3", "arithmetic"),
        ("5 == 3", "equality"),
        ("5 != 3", "inequality"),
        ("5 < 3", "less than"),
        ("5 > 3", "greater than"),
        ("5 <= 3", "less equal"),
        ("5 >= 3", "greater equal"),
    ];

    for (expr, desc) in test_cases {
        println!("Testing {} operation: {}", desc, expr);
        let code = format!("r {};", expr);
        let result = test_boolean_pipeline(&code);

        if !result.lexer_errors.is_empty() || !result.parser_errors.is_empty() {
            println!("  Lexer/Parser failed");
        } else if let Some(ast) = &result.ast {
            println!("  AST: {:?}", ast);
        }
    }
}

#[test]
fn boolean_comprehensive_test() {
    println!("=== COMPREHENSIVE BOOLEAN TEST ===");

    let test_cases = vec![
        // Basic types that should work
        ("fn test():i { r 42; }", "basic integer", true),
        ("fn test():s { r `hello`; }", "basic string", true),
        // Boolean type tests
        ("fn test():b { r true; }", "boolean with true", false),   // Might fail
        ("fn test():b { r false; }", "boolean with false", false), // Might fail
        // Arithmetic (should work)
        ("fn test():i { r 5 + 3; }", "addition", true),
        ("fn test():i { r 5 - 3; }", "subtraction", true),
        ("fn test():i { r 5 * 3; }", "multiplication", true),
        ("fn test():i { r 5 / 3; }", "division", true),
        ("fn test():i { r 5 % 3; }", "modulo", true),
        // Comparisons returning int (current buggy behavior)
        ("fn test():i { r 5 == 3; }", "equality as int", true),
        ("fn test():i { r 5 != 3; }", "inequality as int", true),
        ("fn test():i { r 5 < 3; }", "less than as int", true),
        ("fn test():i { r 5 > 3; }", "greater than as int", true),
        // Comparisons returning boolean (the fix we want)
        ("fn test():b { r 5 == 3; }", "equality as boolean", false),    // Currently fails
        ("fn test():b { r 5 != 3; }", "inequality as boolean", false),  // Currently fails
        ("fn test():b { r 5 < 3; }", "less than as boolean", false),    // Currently fails
        ("fn test():b { r 5 > 3; }", "greater than as boolean", false), // Currently fails
        // The original problem
        ("fn is_even(n:i):b { r n % 2 == 0; }", "the original problem", false),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for (code, desc, should_work) in test_cases {
        println!("Testing: {}", desc);
        let result = test_boolean_pipeline(code);

        let actually_works = result.checker_success && result.lexer_errors.is_empty() && result.parser_errors.is_empty();

        if should_work == actually_works {
            println!("  ✅ {} (as expected)", if actually_works { "PASS" } else { "FAIL" });
            passed += 1;
        } else {
            println!("  ❌ {} (unexpected!)", if actually_works { "PASS" } else { "FAIL" });
            failed += 1;

            if !result.checker_errors.is_empty() {
                for error in &result.checker_errors {
                    println!("    Error: {}", error.message);
                }
            }
        }
    }

    println!("=== SUMMARY ===");
    println!("Expected results: {} passed, {} failed", passed, failed);
    println!("Total tests: {}", passed + failed);
}
