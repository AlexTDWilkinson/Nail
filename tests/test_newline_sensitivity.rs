extern crate Nail;
use Nail::{lexer, parser};

fn test_code(name: &str, code: &str) {
    println!("\n=== Testing: {} ===", name);
    println!("Code: {:?}", code);

    let tokens = lexer::lexer(code);
    println!("Tokens: {:#?}", tokens);

    match parser::parse(tokens) {
        Ok((ast, _)) => println!("Parse successful: {:#?}", ast),
        Err(e) => println!("Parse failed: {:?}", e),
    }
}

#[test]
fn test_newline_sensitivity() {
    println!("Testing Nail parser newline sensitivity\n");

    // Test 1: Simple statement without trailing newline
    test_code("Simple without newline", "print(`hello`);");

    // Test 2: Simple statement with trailing newline
    test_code("Simple with newline", "print(`hello`);\n");

    // Test 3: Simple statement with multiple trailing newlines
    test_code("Simple with multiple newlines", "print(`hello`);\n\n");

    // Test 4: Simple statement without semicolon and no newline
    test_code("No semicolon, no newline", "print(`hello`)");

    // Test 5: Simple statement without semicolon but with newline
    test_code("No semicolon, with newline", "print(`hello`)\n");

    // Test 6: Variable declaration without newline
    test_code("Variable decl without newline", "c x:i = 42;");

    // Test 7: Variable declaration with newline
    test_code("Variable decl with newline", "c x:i = 42;\n");

    // Test 8: Multiple statements without final newline
    test_code("Multiple statements without newline", "c x:i = 42; print(to_string(x));");

    // Test 9: Multiple statements with final newline
    test_code("Multiple statements with newline", "c x:i = 42; print(to_string(x));\n");
}
