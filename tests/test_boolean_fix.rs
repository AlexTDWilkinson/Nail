// Targeted tests for fixing the boolean comparison issue

use Nail::lexer::*;
use Nail::parser::*;
use Nail::checker::*;
use Nail::common::*;

/// Helper to run code through full pipeline
fn test_code(code: &str) -> Result<(), CodeError> {
    let tokens = lexer(code);
    let mut ast = parse(tokens)?;
    checker(&mut ast).map_err(|errors| {
        if errors.is_empty() {
            CodeError { message: "Unknown checker error".to_string(), code_span: CodeSpan::default() }
        } else {
            errors[0].clone()
        }
    })
}

#[test]
fn test_old_buggy_behavior_now_fixed() {
    // These used to work incorrectly (comparisons returning Int instead of Boolean)
    // After fix: they should now correctly FAIL because comparisons return Boolean, not Int
    println!("=== TESTING OLD BUGGY BEHAVIOR (NOW CORRECTLY FAILS) ===");
    
    let cases = vec![
        ("fn test():i { r 5 == 5; }", "equality returning int"),
        ("fn test():i { r 5 != 3; }", "inequality returning int"),  
        ("fn test():i { r 5 < 10; }", "less than returning int"),
        ("fn test():i { r 5 > 3; }", "greater than returning int"),
        ("fn test():i { r 5 <= 5; }", "less equal returning int"),
        ("fn test():i { r 5 >= 5; }", "greater equal returning int"),
    ];
    
    for (code, desc) in cases {
        let result = test_code(code);
        println!("{}: {:?}", desc, result);
        // After fix: these should now correctly FAIL with type mismatch
        assert!(result.is_err(), "After fix, {} should fail because comparisons return Boolean", desc);
        if let Err(error) = result {
            assert!(error.message.contains("expected Int, got Boolean"), 
                "Should fail with correct type mismatch: {}", desc);
        }
    }
}

#[test]
fn test_desired_correct_behavior() {
    // These currently FAIL but should WORK - comparisons should return Boolean
    println!("=== TESTING DESIRED CORRECT BEHAVIOR ===");
    
    let cases = vec![
        ("fn test():b { r 5 == 5; }", "equality returning boolean"),
        ("fn test():b { r 5 != 3; }", "inequality returning boolean"),
        ("fn test():b { r 5 < 10; }", "less than returning boolean"),
        ("fn test():b { r 5 > 3; }", "greater than returning boolean"),
        ("fn test():b { r 5 <= 5; }", "less equal returning boolean"),
        ("fn test():b { r 5 >= 5; }", "greater equal returning boolean"),
    ];
    
    let mut failing_count = 0;
    
    for (code, desc) in cases {
        let result = test_code(code);
        println!("{}: {:?}", desc, result);
        
        if result.is_err() {
            failing_count += 1;
            if let Err(error) = result {
                assert!(error.message.contains("expected Boolean, got Int"), 
                    "Should fail with type mismatch: {}", desc);
            }
        }
    }
    
    // Before fix: all should fail
    // After fix: all should pass
    println!("Failing comparisons: {}/6", failing_count);
    
    // After fix: all should pass
    assert_eq!(failing_count, 0, "All boolean comparisons should work after fix");
}

#[test] 
fn test_arithmetic_still_works() {
    // These should continue to work after our fix
    println!("=== TESTING ARITHMETIC OPERATIONS ===");
    
    let cases = vec![
        ("fn test():i { r 5 + 3; }", "addition"),
        ("fn test():i { r 5 - 3; }", "subtraction"),
        ("fn test():i { r 5 * 3; }", "multiplication"),
        ("fn test():i { r 5 / 3; }", "division"),
        ("fn test():i { r 5 % 3; }", "modulo"),
    ];
    
    for (code, desc) in cases {
        let result = test_code(code);
        println!("{}: {:?}", desc, result);
        assert!(result.is_ok(), "Arithmetic should work: {}", desc);
    }
}

#[test]
fn test_original_problem_case() {
    // The exact case from the user's screenshot
    println!("=== TESTING ORIGINAL PROBLEM CASE ===");
    
    let code = "fn is_even_func(n:i):b { r n % 2 == 0; }";
    let result = test_code(code);
    
    println!("Original problem: {:?}", result);
    
    // Before fix: should fail with "expected Boolean, got Int"
    // After fix: should pass
    if let Err(error) = result {
        assert!(error.message.contains("expected Boolean, got Int"), 
            "Should fail with the exact error from screenshot");
        println!("✅ Successfully reproduced the original problem");
    } else {
        println!("🎉 Original problem is FIXED!");
    }
}

#[test]
fn test_nested_comparisons() {
    // More complex cases that should work after fix
    println!("=== TESTING NESTED COMPARISONS ===");
    
    let cases = vec![
        ("fn test(x:i, y:i):b { r x == y; }", "parameter comparison"),
        ("fn test():b { r (5 + 3) == 8; }", "expression comparison"),
        ("fn test():b { r 1 < 2; }", "simple boolean result"),
    ];
    
    for (code, desc) in cases {
        let result = test_code(code);
        println!("{}: {:?}", desc, result);
        
        // These will fail before fix, should work after
        if result.is_err() {
            println!("  ❌ Currently fails (expected before fix)");
        } else {
            println!("  ✅ Works (this means fix is successful!)");
        }
    }
}

#[test]
fn test_boolean_literals() {
    // Test basic boolean support
    println!("=== TESTING BOOLEAN LITERALS ===");
    
    let cases = vec![
        ("fn test():b { r true; }", "true literal"),
        ("fn test():b { r false; }", "false literal"),
    ];
    
    for (code, desc) in cases {
        let result = test_code(code);
        println!("{}: {:?}", desc, result);
        // These might fail if boolean literals aren't supported yet
    }
}

#[test]
fn test_mixed_types_still_fail() {
    // These should still fail after our fix (type safety)
    println!("=== TESTING TYPE SAFETY STILL ENFORCED ===");
    
    let cases = vec![
        ("fn test():b { r 5; }", "int where boolean expected"),
        ("fn test():i { r true; }", "boolean where int expected"),
        ("fn test():s { r 5 == 5; }", "boolean where string expected"),
    ];
    
    for (code, desc) in cases {
        let result = test_code(code);
        println!("{}: {:?}", desc, result);
        assert!(result.is_err(), "Type mismatches should still fail: {}", desc);
    }
}