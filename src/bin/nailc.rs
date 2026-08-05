use std::env;
use std::fs;
use std::process;

use nail::lexer::{lexer, lexer_with_context};
use nail::parser::parse;
use nail::checker::checker;
use nail::transpiler::Transpiler;
use std::path::Path;

/// Compiler stage timings on stderr, only when a human is watching. Piped and
/// scripted runs (tests, deploys, the website's run buttons) never see them.
fn print_stage_timings(stages: &[(&str, std::time::Duration)]) {
    use std::io::IsTerminal;
    if stages.is_empty() || !std::io::stderr().is_terminal() {
        return;
    }
    let total: std::time::Duration = stages.iter().map(|(_, took)| *took).sum();
    let parts: Vec<String> = stages.iter().map(|(stage, took)| format!("{} {:.1}ms", stage, took.as_secs_f64() * 1000.0)).collect();
    eprintln!("compiler timings: {}, total {:.1}ms", parts.join(", "), total.as_secs_f64() * 1000.0);
}

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
        eprintln!("  --deps-only    Only output required Cargo dependencies");
        eprintln!("  --cargo-toml   Output a complete Cargo.toml for the transpiled program");
        eprintln!("                 (use --nail-path=<path> to set the nail crate path, default \"..\",");
        eprintln!("                  and --package-name=<name> to override the package name)");
        eprintln!("  --cargo-toml-superset  Output a Cargo.toml requiring every stdlib crate");
        eprintln!("                 (no input file; used by the bundle build to warm the dep cache)");
        eprintln!("  -o <path>      Write transpiled Rust to <path> instead of next to the source");
        eprintln!("  --stdout       Write transpiled Rust to stdout, don't touch the filesystem");
        eprintln!("  --no-profile   Emit no runtime profiling (profiling is on by default: every");
        eprintln!("                 user function is timed, a sheet prints at exit when stderr is");
        eprintln!("                 a terminal, and .nail/profile.json is refreshed every second)");
        eprintln!("  --target=wasm  Build for the browser: emits a wasm-bindgen start function");
        eprintln!("                 instead of a main, and refuses programs that call stdlib");
        eprintln!("                 functions needing an operating system (files, servers, etc.)");
        process::exit(1);
    }

    let nail_path = args.iter().find_map(|arg| arg.strip_prefix("--nail-path=")).unwrap_or("..").to_string();

    // Superset manifest needs no input file - it is derived from the registry alone
    if args[1] == "--cargo-toml-superset" {
        let package_name = args.iter().find_map(|arg| arg.strip_prefix("--package-name=")).unwrap_or("nail_transpilation");
        print!("{}", Transpiler::generate_cargo_toml_superset(package_name, &nail_path));
        return;
    }

    let filename = &args[1];
    let mut mode = args.get(2).map(|s| s.as_str()).unwrap_or("--transpile");
    let skip_check = args.iter().any(|arg| arg == "--skip-check");
    let to_stdout = args.iter().any(|arg| arg == "--stdout");
    let no_profile = args.iter().any(|arg| arg == "--no-profile");
    let wasm_target = args.iter().any(|arg| arg == "--target=wasm");
    let output_path = args.iter().position(|arg| arg == "-o").and_then(|i| args.get(i + 1)).cloned();
    // Flags in the mode position mean the default mode with that flag applied
    if mode == "-o" || mode == "--stdout" || mode == "--no-profile" || mode == "--target=wasm" {
        mode = "--transpile";
    }
    
    // If --skip-check is present with --transpile, handle it specially
    if skip_check && mode == "--transpile" {
        mode = "--transpile-skip-check";
    }

    // Machine-readable modes must not print pipeline banners to stdout
    let quiet = matches!(mode, "--transpile" | "--transpile-skip-check" | "--deps-only" | "--cargo-toml");
    
    // Read the input file
    let input = match fs::read_to_string(filename) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", filename, e);
            process::exit(1);
        }
    };
    
    
    let mut stage_timings: Vec<(&str, std::time::Duration)> = Vec::new();

    // Run lexer
    if !quiet {
        println!("=== Lexing {} ===", filename);
    }
    let stage_start = std::time::Instant::now();
    let tokens = lexer_with_context(&input, Some(Path::new(filename)));
    stage_timings.push(("lex", stage_start.elapsed()));

    let lexer_errors = nail::lexer::collect_lexer_errors(&tokens);
    if !lexer_errors.is_empty() {
        for error in &lexer_errors {
            eprint!("{}", error.render(filename, &input));
        }
        process::exit(1);
    }

    if mode == "--lex-only" {
        println!("Tokens:");
        for token in &tokens {
            println!("{:#?}", token);
        }
        print_stage_timings(&stage_timings);
        return;
    }

    // Run parser
    if !quiet {
        println!("\n=== Parsing ===");
    }
    let stage_start = std::time::Instant::now();
    let ast = match parse(tokens) {
        Ok(ast) => {
            if !quiet {
                println!("Parse successful!");
            }
            ast
        }
        Err(e) => {
            eprint!("{}", e.render(filename, &input));
            process::exit(1);
        }
    };
    stage_timings.push(("parse", stage_start.elapsed()));

    if mode == "--parse-only" {
        println!("\nAST:");
        println!("{:#?}", ast);
        print_stage_timings(&stage_timings);
        return;
    }
    
    // Skip type checking if requested
    let checked_ast = if skip_check || mode == "--transpile-skip-check" {
        if !quiet {
            println!("\n=== Skipping Type Check ===");
        }
        ast
    } else {
        // Run type checker
        if !quiet {
            println!("\n=== Type Checking ===");
        }
        let stage_start = std::time::Instant::now();
        let mut checked_ast = ast;
        match checker(&mut checked_ast) {
            Ok(_) => {
                if !quiet {
                    println!("Type check successful!");
                }
                stage_timings.push(("check", stage_start.elapsed()));
                checked_ast
            }
            Err(errors) => {
                for error in &errors {
                    eprint!("{}", error.render(filename, &input));
                }
                let count = errors.len();
                eprintln!("{} error{} found", count, if count == 1 { "" } else { "s" });
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
        print_stage_timings(&stage_timings);
        return;
    }

    // Run transpiler
    if !quiet {
        println!("\n=== Transpiling to Rust ===");
    }
    let stage_start = std::time::Instant::now();
    let mut transpiler = Transpiler::new();
    transpiler.profile = !no_profile;
    transpiler.profile_source_hash = Some(nail::prof::source_fingerprint(&input));
    // Profiling writes files and reads clocks, neither of which a browser
    // program can do, so the wasm build is always unprofiled.
    transpiler.wasm_target = wasm_target;
    if wasm_target {
        transpiler.profile = false;
    }
    let rust_code = match transpiler.transpile(&checked_ast) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Transpilation error: {}", e);
            process::exit(1);
        }
    };
    stage_timings.push(("transpile", stage_start.elapsed()));
    print_stage_timings(&stage_timings);

    if wasm_target {
        let blockers = transpiler.wasm_unsupported_functions();
        if !blockers.is_empty() {
            eprintln!("This program cannot run in a browser. These calls need an operating system underneath:");
            for name in &blockers {
                eprintln!("  {}", name);
            }
            eprintln!("A browser has no files, processes, terminals or servers. Remove these calls or build natively.");
            process::exit(1);
        }
    }

    if mode == "--deps-only" {
        // Output dependencies in a machine-readable format
        let dependencies = transpiler.get_required_dependencies();
        for dep in dependencies {
            println!("{}", dep.to_cargo_dep());
        }
        return;
    }

    if mode == "--cargo-toml" {
        let package_name = args
            .iter()
            .find_map(|arg| arg.strip_prefix("--package-name="))
            .map(|name| name.to_string())
            .unwrap_or_else(|| Path::new(filename).file_stem().and_then(|stem| stem.to_str()).unwrap_or("nail_program").replace('-', "_"));
        if wasm_target {
            print!("{}", transpiler.generate_cargo_toml_wasm(&package_name, &nail_path));
        } else {
            print!("{}", transpiler.generate_cargo_toml(&package_name, &nail_path));
        }
        return;
    }
    
    if !quiet {
        println!("\nGenerated Rust code:");
    }
    
    if to_stdout {
        print!("{}", rust_code);
        return;
    }

    let output_filename = output_path.unwrap_or_else(|| filename.replace(".nail", ".rs"));

    // For --transpile mode, just output the code directly
    if mode == "--transpile" || mode == "--transpile-skip-check" {
        // Save the Rust code to file
        match fs::write(&output_filename, &rust_code) {
            Ok(_) => {
                // Don't print anything, just save the file
            }
            Err(e) => {
                eprintln!("Error writing output file: {}", e);
                process::exit(1);
            }
        }
    } else {
        // For other modes, print the code and save message
        println!("{}", rust_code);
        match fs::write(&output_filename, &rust_code) {
            Ok(_) => println!("\nRust code saved to: {}", output_filename),
            Err(e) => eprintln!("Error writing output file: {}", e),
        }
    }
}