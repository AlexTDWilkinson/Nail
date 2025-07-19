// Comprehensive test framework for the entire Nail language
// Tests lexer, parser, type checker, and transpiler

use Nail::checker::*;
use Nail::common::*;
use Nail::lexer::*;
use Nail::parser::*;

/// Test helper to run the entire pipeline: code -> tokens -> AST -> type check
pub fn test_full_pipeline(code: &str) -> TestResult {
    let mut result = TestResult::new(code);

    // Step 1: Lexer
    let tokens = lexer(code);
    result.tokens = Some(tokens.clone());

    // Step 2: Parser
    match parse(tokens) {
        Ok((mut ast, _)) => {
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

/// Simplified test for just lexer
pub fn test_lexer_only(code: &str) -> Vec<Token> {
    lexer(code)
}

/// Test result structure
#[derive(Debug)]
pub struct TestResult {
    pub code: String,
    pub tokens: Option<Vec<Token>>,
    pub ast: Option<ASTNode>,
    pub lexer_errors: Vec<CodeError>,
    pub parser_errors: Vec<CodeError>,
    pub checker_errors: Vec<CodeError>,
    pub checker_success: bool,
}

impl TestResult {
    fn new(code: &str) -> Self {
        Self { code: code.to_string(), tokens: None, ast: None, lexer_errors: vec![], parser_errors: vec![], checker_errors: vec![], checker_success: false }
    }

    pub fn has_any_errors(&self) -> bool {
        !self.lexer_errors.is_empty() || !self.parser_errors.is_empty() || !self.checker_errors.is_empty()
    }

    pub fn should_succeed(&self) -> bool {
        !self.has_any_errors() && self.checker_success
    }

    pub fn print_summary(&self) {
        println!("=== TEST RESULT ===");
        println!("Code: {}", self.code);
        println!("Lexer errors: {}", self.lexer_errors.len());
        println!("Parser errors: {}", self.parser_errors.len());
        println!("Checker errors: {}", self.checker_errors.len());
        println!("Checker success: {}", self.checker_success);

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
    }
}
