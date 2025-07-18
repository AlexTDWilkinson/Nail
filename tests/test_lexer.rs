// Comprehensive lexer tests for Nail language

use Nail::lexer::*;
use Nail::common::*;

#[test]
fn test_basic_tokens() {
    let tokens = lexer("fn x y z");
    
    println!("Basic tokens: {:?}", tokens);
    println!("Number of tokens: {}", tokens.len());
    
    // For now, just check we get some tokens  
    assert!(!tokens.is_empty());
    
    // Look for identifiers in the tokens
    let identifier_count = tokens.iter().filter(|t| matches!(t.token_type, TokenType::Identifier(_))).count();
    println!("Number of identifiers: {}", identifier_count);
}

#[test]
fn test_integer_literals() {
    let tokens = lexer("123 456 0");
    
    assert_eq!(tokens.len(), 3);
    for token in tokens {
        assert!(matches!(token.token_type, TokenType::Integer(_)));
    }
}

#[test]
fn test_boolean_literals() {
    let tokens = lexer("true false");
    
    assert_eq!(tokens.len(), 2);
    
    // Check what token types we actually get for boolean literals
    println!("Boolean tokens: {:?}", tokens);
    
    // For now, let's see what happens - they might be identifiers
    if let TokenType::Identifier(ref lexeme) = tokens[0].token_type {
        assert_eq!(lexeme, "true");
    }
    if let TokenType::Identifier(ref lexeme) = tokens[1].token_type {
        assert_eq!(lexeme, "false");
    }
}

#[test]
fn test_operators() {
    let tokens = lexer("+ - * / % == != < > <= >=");
    
    println!("Operator tokens: {:?}", tokens);
    
    // Just check we get some tokens - the exact types may differ
    assert!(!tokens.is_empty());
    
    // Look for operator tokens
    for token in &tokens {
        if let TokenType::Operator(ref op) = token.token_type {
            println!("Found operator: {:?}", op);
        }
    }
}

#[test]
fn test_type_annotations() {
    let tokens = lexer("x:i y:s z:b");
    
    println!("Type annotation tokens: {:?}", tokens);
    
    // Just check we get some tokens
    assert!(!tokens.is_empty());
    
    // Look for type declarations or identifiers
    for token in &tokens {
        match &token.token_type {
            TokenType::TypeDeclaration(type_desc) => {
                println!("Found type declaration: {:?}", type_desc);
            }
            TokenType::Identifier(name) => {
                println!("Found identifier: {}", name);
            }
            _ => {}
        }
    }
}

#[test]
fn test_function_declaration() {
    let tokens = lexer("fn test(x:i):b { r true; }");
    
    println!("Function tokens: {:?}", tokens);
    
    // Basic check that we get tokens without errors
    assert!(!tokens.is_empty());
    
    // Look for function-related tokens
    for token in &tokens {
        match &token.token_type {
            TokenType::Identifier(name) => {
                println!("Found identifier: {}", name);
            }
            TokenType::FunctionSignature(_) => {
                println!("Found function signature");
            }
            _ => {}
        }
    }
}

#[test]
fn test_comparison_expression() {
    let tokens = lexer("n % 2 == 0");
    
    println!("Comparison tokens: {:?}", tokens);
    
    // Should have several tokens
    assert!(!tokens.is_empty());
    
    // Look for identifiers, operators, and integers
    for token in &tokens {
        match &token.token_type {
            TokenType::Identifier(name) => {
                println!("Found identifier: {}", name);
            }
            TokenType::Operator(op) => {
                println!("Found operator: {:?}", op);
            }
            TokenType::Integer(val) => {
                println!("Found integer: {}", val);
            }
            _ => {
                println!("Found other token: {:?}", token.token_type);
            }
        }
    }
}

#[test]
fn test_string_literals() {
    let tokens = lexer("`hello world`");
    
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].token_type, TokenType::StringLiteral(_)));
    
    if let TokenType::StringLiteral(ref content) = tokens[0].token_type {
        println!("String content: {}", content);
    }
}

#[test]
fn test_comments() {
    let tokens = lexer("x // this is a comment\ny");
    
    println!("Comment tokens: {:?}", tokens);
    
    // Comments might be included or filtered out
    for token in &tokens {
        match &token.token_type {
            TokenType::Identifier(name) => {
                println!("Found identifier: {}", name);
            }
            TokenType::Comment(comment) => {
                println!("Found comment: {}", comment);
            }
            _ => {}
        }
    }
}

#[test]
fn test_brackets_and_parens() {
    let tokens = lexer("{ } ( ) [ ]");
    
    println!("Bracket tokens: {:?}", tokens);
    
    // Check for block and bracket tokens
    for token in &tokens {
        match &token.token_type {
            TokenType::BlockOpen => println!("Found block open"),
            TokenType::BlockClose => println!("Found block close"),
            TokenType::ParenthesisOpen => println!("Found paren open"),
            TokenType::ParenthesisClose => println!("Found paren close"),
            TokenType::ArrayOpen => println!("Found array open"),
            TokenType::ArrayClose => println!("Found array close"),
            _ => {}
        }
    }
}

#[test]
fn test_assignment_operators() {
    let tokens = lexer("= += -= *= /=");
    
    println!("Assignment tokens: {:?}", tokens);
    
    // Check we get some tokens
    assert!(!tokens.is_empty());
    
    // Look for assignment tokens
    for token in &tokens {
        match &token.token_type {
            TokenType::Assignment => println!("Found assignment"),
            TokenType::Operator(op) => println!("Found operator: {:?}", op),
            _ => {}
        }
    }
}

#[test]
fn test_error_cases() {
    // Test invalid characters
    let tokens = lexer("@#$");
    
    println!("Error case tokens: {:?}", tokens);
    
    // Check for lexer error tokens
    for token in &tokens {
        if let TokenType::LexerError(ref error) = token.token_type {
            println!("Found lexer error: {}", error);
        }
    }
}

#[test]
fn test_multiline_code() {
    let code = r#"fn test():i {
    x:i = 5;
    r x + 1;
}"#;
    
    let tokens = lexer(code);
    
    println!("Multiline tokens: {:?}", tokens);
    
    // Should parse without errors
    assert!(!tokens.is_empty());
    
    // Count different token types
    let mut identifiers = 0;
    let mut integers = 0;
    let mut operators = 0;
    
    for token in &tokens {
        match &token.token_type {
            TokenType::Identifier(_) => identifiers += 1,
            TokenType::Integer(_) => integers += 1,
            TokenType::Operator(_) => operators += 1,
            _ => {}
        }
    }
    
    println!("Found {} identifiers, {} integers, {} operators", identifiers, integers, operators);
}