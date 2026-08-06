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
        eprintln!("  --stamp=<v>    Rewrite the file's version line to <v> or to `latest`, but");
        eprintln!("                 only if it still type checks. This is the half of");
        eprintln!("                 `nail update` that a release has to do, because deciding");
        eprintln!("                 whether a file survives a migration means compiling it.");
        process::exit(1);
    }

    let nail_path = args.iter().find_map(|arg| arg.strip_prefix("--nail-path=")).unwrap_or("..").to_string();

    // Looking a function up needs no input file either: the registry is the
    // whole answer, and it is the registry this compiler was built with, so
    // the answer always matches the compiler that will run the code.
    if let Some(query) = args.iter().find_map(|arg| arg.strip_prefix("--docs=")) {
        let functions = nail::parser::std_lib::stdlib::functions();
        let needle = query.to_lowercase();

        // No name asked about means the whole library, grouped the way the
        // namespaces group it. It is long on purpose: it is the answer to
        // "what can this thing do", and it pipes into grep and less.
        if query.is_empty() {
            let mut current_module = "";
            for function in &functions {
                if function.module != current_module {
                    if !current_module.is_empty() {
                        println!();
                    }
                    println!("{}", function.module);
                    current_module = &function.module;
                }
                println!("  {}", function.signature);
            }
            let modules = nail::parser::std_lib::stdlib::modules();
            println!("\n{} functions in {} libraries. `nail docs <name>` for one of them.", functions.len(), modules.len());
            return;
        }

        if let Some(exact) = functions.iter().find(|function| function.name == query) {
            println!("{}", exact.signature);
            println!("  {}", exact.description);
            if !exact.example.is_empty() {
                println!();
                for line in exact.example.lines() {
                    println!("  {}", line);
                }
            }
            println!("\n  {} library", exact.module);
            return;
        }

        let matches: Vec<&nail::parser::std_lib::stdlib::STDLIB_Function> =
            functions.iter().filter(|function| function.name.to_lowercase().contains(&needle) || function.description.to_lowercase().contains(&needle)).collect();
        if matches.is_empty() {
            eprintln!("Nothing in the standard library matches '{}'", query);
            process::exit(1);
        }
        for function in matches.iter().take(40) {
            println!("{}", function.signature);
        }
        if matches.len() > 40 {
            println!("... and {} more", matches.len() - 40);
        }
        return;
    }

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
    if args.iter().any(|arg| arg == "--run") {
        mode = "--run";
    }
    if args.iter().any(|arg| arg == "--build") {
        mode = "--build";
    }

    // Stamping is its own mode: check the file, and rewrite line one only if
    // the check passed. A file that no longer compiles keeps its old version line
    // and keeps working, which is what makes migration optional forever.
    let stamp_to = match args.iter().find_map(|arg| arg.strip_prefix("--stamp=")) {
        Some(text) => match text.parse::<nail::version_line::Pin>() {
            Ok(pin) => {
                mode = "--stamp";
                Some(pin)
            }
            Err(_) => {
                eprintln!("`{}` is not a version like 0.3.1, or the word `latest`", text);
                process::exit(1);
            }
        },
        None => None,
    };
    
    // If --skip-check is present with --transpile, handle it specially
    if skip_check && mode == "--transpile" {
        mode = "--transpile-skip-check";
    }

    // Machine-readable modes must not print pipeline banners to stdout
    let quiet = matches!(mode, "--transpile" | "--transpile-skip-check" | "--deps-only" | "--cargo-toml" | "--stamp" | "--run" | "--build");
    
    // Read the input file
    let input = match fs::read_to_string(filename) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", filename, e);
            process::exit(1);
        }
    };

    // A Nail file has to say which compiler it was written for. Leaving it out
    // would mean the file compiles with whatever is newest, which is exactly
    // the drift the version line exists to prevent, so it is an error rather
    // than a default. The IDE writes the line on save and `--stamp` adds it to
    // a file that has none, so this is reachable mainly by hand-made files.
    //
    // Only the entry file needs one. Imported files inherit it, because one
    // compiler compiles everything it reaches and only the entry decides which.
    // The launcher stays deliberately permissive here and falls back to the newest
    // installed version: refusing to launch a file is a far worse failure than
    // compiling it, and by the time this runs the right compiler was chosen.
    if stamp_to.is_none() && nail::version_line::scan_header(input.as_bytes()).pin.is_none() {
        eprintln!("error: '{}' has no version line", filename);
        eprintln!();
        eprintln!("Every Nail file says on its first line which compiler wrote it, so that it");
        eprintln!("keeps compiling the same way forever. Add one of these as line one:");
        eprintln!();
        eprintln!("  nail {}     this exact compiler, frozen", env!("CARGO_PKG_VERSION"));
        eprintln!("  nail latest    whichever version is installed, for code you are still writing");
        eprintln!();
        eprintln!("Or let a tool do it: `nailc {} --stamp=latest`", filename);
        process::exit(1);
    }

    
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
    
    if let Some(pin) = &stamp_to {
        // The type check above is the whole gate: reaching here means the file
        // compiles under this compiler, so it is safe to say so on line one.
        let stamped = nail::version_line::stamp(&input, pin);
        if stamped == input {
            return;
        }
        if let Err(error) = fs::write(filename, stamped) {
            eprintln!("Error writing '{}': {}", filename, error);
            process::exit(1);
        }
        return;
    }

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

    // Compile the program, and run it unless only the binary was asked for.
    // `nail run` and `nail build` are the same work up to the last step.
    // Same steps the IDE performs for F7: write the generated Rust and its
    // manifest into a build directory beside the source, build with the
    // bundled cargo when there is one, then hand over to the result.
    if mode == "--run" || mode == "--build" {
        let package_name = "nail_transpilation";
        let build_dir = Path::new(filename).parent().unwrap_or(Path::new(".")).join(".nail-build");
        if let Err(error) = fs::create_dir_all(build_dir.join("src")) {
            eprintln!("Cannot create {}: {}", build_dir.display(), error);
            process::exit(1);
        }

        // An installed release builds with its own pinned toolchain and its
        // own copy of the nail crate. A development checkout uses the system
        // cargo and the crate next door.
        let bundle = nail::toolchain::BundledToolchain::detect();
        let nail_crate_path = match &bundle {
            Some(bundle) => bundle.nail_crate_path().display().to_string(),
            // The build directory sits beside the source rather than beside
            // the crate, so a relative --nail-path would resolve from the
            // wrong place. Anchor it to where the command was run.
            None => fs::canonicalize(&nail_path).map(|path| path.display().to_string()).unwrap_or_else(|_| nail_path.clone()),
        };

        // Written only when changed, so cargo's mtime fingerprints survive and
        // an unchanged program does not relink.
        let manifest = transpiler.generate_cargo_toml(package_name, &nail_crate_path);
        let manifest_path = build_dir.join("Cargo.toml");
        if fs::read_to_string(&manifest_path).map(|existing| existing != manifest).unwrap_or(true) {
            if let Err(error) = fs::write(&manifest_path, &manifest) {
                eprintln!("Cannot write {}: {}", manifest_path.display(), error);
                process::exit(1);
            }
        }
        let main_path = build_dir.join("src/main.rs");
        if fs::read_to_string(&main_path).map(|existing| existing != rust_code).unwrap_or(true) {
            if let Err(error) = fs::write(&main_path, &rust_code) {
                eprintln!("Cannot write {}: {}", main_path.display(), error);
                process::exit(1);
            }
        }

        let mut cargo = match &bundle {
            Some(bundle) => bundle.cargo_command(),
            None => process::Command::new("cargo"),
        };
        let status = cargo.arg("build").arg("--release").current_dir(&build_dir).status();
        match status {
            Ok(status) if status.success() => {}
            Ok(_) => process::exit(1),
            Err(error) => {
                eprintln!("Cannot run cargo: {}", error);
                process::exit(1);
            }
        }

        let binary = match &bundle {
            Some(bundle) => bundle.built_binary_path(package_name),
            None => build_dir.join("target/release").join(package_name),
        };
        if mode == "--build" {
            // Beside the source, named after it, so what was produced is
            // obvious and it is not buried in a build directory.
            let destination = Path::new(filename).with_extension("");
            if let Err(error) = fs::copy(&binary, &destination) {
                eprintln!("Cannot write {}: {}", destination.display(), error);
                process::exit(1);
            }
            println!("{}", destination.display());
            return;
        }

        // Arguments after the file are the program's, not ours.
        let program_args: Vec<&String> = args.iter().skip(2).filter(|arg| !arg.starts_with("--")).collect();
        let error = std::os::unix::process::CommandExt::exec(process::Command::new(&binary).args(program_args));
        eprintln!("Cannot run {}: {}", binary.display(), error);
        process::exit(1);
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