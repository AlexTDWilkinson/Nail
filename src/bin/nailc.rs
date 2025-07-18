use std::env;
use std::fs;
use std::process;

use Nail::lexer::lexer;
use Nail::parser::parse;
use Nail::checker::checker;
use Nail::transpilier::Transpiler;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <input.nail> [options]", args[0]);
        eprintln!("Options:");
        eprintln!("  --lex-only     Only run lexer and print tokens");
        eprintln!("  --parse-only   Only run lexer and parser, print AST");
        eprintln!("  --check-only   Run lexer, parser, and type checker");
        eprintln!("  --transpile    Run full pipeline and output Rust code");
        eprintln!("  --skip-check   Skip type checking and transpile directly");
        process::exit(1);
    }
    
    let filename = &args[1];
    let mode = args.get(2).map(|s| s.as_str()).unwrap_or("--transpile");
    
    // Read the input file
    let input = match fs::read_to_string(filename) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", filename, e);
            process::exit(1);
        }
    };
    
    // Run lexer
    println!("=== Lexing {} ===", filename);
    let tokens = lexer(&input);
    
    if mode == "--lex-only" {
        println!("Tokens:");
        for token in &tokens {
            println!("{:#?}", token);
        }
        return;
    }
    
    // Run parser
    println!("\n=== Parsing ===");
    let ast = match parse(tokens) {
        Ok(ast) => {
            println!("Parse successful!");
            ast
        }
        Err(e) => {
            eprintln!("Parse error: {:?}", e);
            process::exit(1);
        }
    };
    
    if mode == "--parse-only" {
        println!("\nAST:");
        println!("{:#?}", ast);
        return;
    }
    
    // Skip type checking if requested
    let checked_ast = if mode == "--skip-check" {
        println!("\n=== Skipping Type Check ===");
        ast
    } else {
        // Run type checker
        println!("\n=== Type Checking ===");
        let mut checked_ast = ast;
        match checker(&mut checked_ast) {
            Ok(_) => {
                println!("Type check successful!");
                checked_ast
            }
            Err(errors) => {
                eprintln!("Type check errors:");
                for error in errors {
                    eprintln!("  {}", error);
                }
                if mode != "--check-only" {
                    eprintln!("\nUse --skip-check to transpile anyway");
                }
                process::exit(1);
            }
        }
    };
    
    if mode == "--check-only" {
        println!("\nType-checked AST:");
        println!("{:#?}", checked_ast);
        return;
    }
    
    // Run transpiler
    println!("\n=== Transpiling to Rust ===");
    let mut transpiler = Transpiler::new();
    let rust_code = match transpiler.transpile(&checked_ast) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Transpilation error: {}", e);
            process::exit(1);
        }
    };
    
    println!("\nGenerated Rust code:");
    println!("{}", rust_code);
    
    // Optionally save the Rust code
    let output_filename = filename.replace(".nail", ".rs");
    match fs::write(&output_filename, &rust_code) {
        Ok(_) => println!("\nRust code saved to: {}", output_filename),
        Err(e) => eprintln!("Error writing output file: {}", e),
    }
}