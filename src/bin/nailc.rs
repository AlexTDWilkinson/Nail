use std::env;
use std::fs;
use std::process;

use nail::lexer::lex_program;
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

/// Every string literal in a token stream, in order. Formatting must never
/// change one, and comparing them is how `fmt` proves it did not.
fn string_literals(tokens: &[nail::lexer::Token]) -> Vec<String> {
    tokens
        .iter()
        .filter_map(|token| if let nail::lexer::TokenType::StringLiteral { value, .. } = &token.token_type { Some(value.clone()) } else { None })
        .collect()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <input.nail> [options]", args[0]);
        eprintln!("Options:");
        eprintln!("  --lex-only     Only run lexer and print tokens");
        eprintln!("  --parse-only   Only run lexer and parser, print AST");
        eprintln!("  --check-only   Run lexer, parser, and type checker. Prints `ok` on success");
        eprintln!("  --ast          With --check-only, print the type-checked AST");
        eprintln!("  --json         Report the result as one line of JSON on stdout, for tools");
        eprintln!("                 driving the compiler: status, stage, and every diagnostic");
        eprintln!("                 with its file, position, and help text");
        eprintln!("  fmt <file>     Format a file in place. --stdout prints instead of writing,");
        eprintln!("                 --check only reports whether the file would change");
        eprintln!("  agents         Write the language primer into ./AGENTS.md, the briefing");
        eprintln!("                 file coding agent tools read before touching a project");
        eprintln!("  --docs=<name>  What the library or the specification says about <name>.");
        eprintln!("                 Bare --docs= lists everything");
        eprintln!("  --docs-json    The whole standard library registry as JSON");
        eprintln!("  --transpile    Run full pipeline and output Rust code");
        eprintln!("  --skip-check   Skip type checking and transpile directly");
        eprintln!("  --deps-only    Only output required Cargo dependencies");
        eprintln!("  --cargo-toml   Output a complete Cargo.toml for the transpiled program");
        eprintln!("                 (use --nail-path=<path> to set the nail crate path, default \"..\",");
        eprintln!("                  and --package-name=<name> to override the package name)");
        eprintln!("  --cargo-toml-superset  Output a Cargo.toml requiring every stdlib crate");
        eprintln!("                 (no input file; used by the bundle build to warm the dep cache)");
        eprintln!("  --dump-examples=<dir>  Write every documentation example to <dir> as a");
        eprintln!("                 runnable .nail file (no input file; test_doc_examples.sh");
        eprintln!("                 compiles them so a broken example cannot ship)");
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

    // The registry as data rather than prose, for tools that drive the
    // compiler. Same source of truth as --docs, one line of JSON.
    if args.iter().any(|arg| arg == "--docs-json") {
        let functions = nail::parser::std_lib::stdlib::functions();
        println!("{}", serde_json::to_string(&functions).expect("the registry serializes"));
        return;
    }

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
            // The headings themselves, not a guessed keyword: guessing gave
            // two different sections the name "error" and turned another into
            // "if". Matching is loose enough that any word out of one works.
            println!("\nThe language itself, rather than its library. `nail docs <any word below>`:");
            let topics = nail::docs::topics();
            for pair in topics.chunks(2) {
                match pair {
                    [left, right] => println!("  {:<38} {}", left, right),
                    [only] => println!("  {}", only),
                    _ => {}
                }
            }
            println!("\nMeeting Nail cold, or briefing a coding agent? `nail docs primer` is the whole language on one page.");
            return;
        }

        // The primer outranks everything: it is the page for someone (or
        // some model) who does not yet know what to ask for.
        if matches!(needle.as_str(), "primer" | "agent" | "agents" | "llm" | "llms" | "ai") {
            println!("{}", nail::docs::primer());
            return;
        }

        // A language topic beats a fuzzy library match, because "errors" is a
        // question about the language and no function is called that.
        if !query.is_empty() && !functions.iter().any(|function| function.name == query) {
            if let Some(section) = nail::docs::section(query) {
                println!("{}", section);
                return;
            }
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

    // Every documentation example as a file on disk, one per function. The
    // examples are whole programs, so the only thing to add is the version
    // line. `test_doc_examples.sh` compiles what this writes: an example is
    // what the editor inserts and the website shows, so it has to be a
    // program that builds, not only one that type checks.
    if let Some(directory) = args.iter().find_map(|arg| arg.strip_prefix("--dump-examples=")) {
        if let Err(error) = fs::create_dir_all(directory) {
            eprintln!("Cannot write examples to '{}': {}", directory, error);
            process::exit(1);
        }
        let mut written = 0;
        for function in nail::parser::std_lib::stdlib::functions() {
            if function.example.is_empty() {
                continue;
            }
            let path = Path::new(directory).join(format!("{}.nail", function.name));
            if let Err(error) = fs::write(&path, format!("nail latest\n{}\n", function.example)) {
                eprintln!("Cannot write '{}': {}", path.display(), error);
                process::exit(1);
            }
            written += 1;
        }
        println!("{}", written);
        return;
    }

    // Superset manifest needs no input file - it is derived from the registry alone
    if args[1] == "--cargo-toml-superset" {
        let package_name = args.iter().find_map(|arg| arg.strip_prefix("--package-name=")).unwrap_or("nail_transpilation");
        print!("{}", Transpiler::generate_cargo_toml_superset(package_name, &nail_path));
        return;
    }

    // `nailc agents`: write the embedded primer into ./AGENTS.md, the file
    // coding agent tools read before touching a project. One command instead
    // of knowing to paste `--docs=primer` output by hand, and the text comes
    // from this compiler, so it describes the Nail that will compile the
    // project. Refuses to clobber, the same as `nail new`.
    if args[1] == "agents" {
        use std::io::IsTerminal;
        let path = Path::new("AGENTS.md");
        if path.exists() {
            eprintln!("AGENTS.md already exists. Delete it first, or merge by hand: `nail docs primer` prints the text");
            process::exit(1);
        }
        if let Err(error) = fs::write(path, nail::docs::primer()) {
            eprintln!("Cannot write AGENTS.md: {}", error);
            process::exit(1);
        }
        println!("AGENTS.md");
        if std::io::stdout().is_terminal() {
            println!("  Coding agents read this file before writing Nail. If the project also");
            println!("  keeps a CLAUDE.md, mention AGENTS.md there, since some tools read only");
            println!("  the first briefing file they find.");
        }
        return;
    }

    // `nailc fmt <file>`: the IDE's formatter from the command line, so code
    // written by tools and agents can be normalized without opening the
    // editor. The file must already parse. The formatter's guarantees hold
    // for valid programs, and rewriting a broken file would only move its
    // errors around. After formatting, the result is lexed and parsed again
    // and every string literal is compared, so a formatter bug refuses to
    // write rather than quietly damaging the file.
    if args[1] == "fmt" {
        let path = match args.get(2).filter(|argument| !argument.starts_with("--")) {
            Some(path) => path,
            None => {
                eprintln!("usage: nailc fmt <file> [--stdout | --check]");
                process::exit(1);
            }
        };
        let source = match fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("Error reading file '{}': {}", path, error);
                process::exit(1);
            }
        };

        let program = lex_program(&source, Some(Path::new(path)));
        let lexer_errors = nail::lexer::collect_lexer_errors(&program.tokens);
        if !lexer_errors.is_empty() {
            for error in &lexer_errors {
                eprint!("{}", error.render_with_map(&program.source_map));
            }
            eprintln!("'{}' does not lex, so it was not formatted. Fix the errors first", path);
            process::exit(1);
        }
        let original_strings = string_literals(&program.tokens);
        if let Err(error) = parse(program.tokens) {
            eprint!("{}", error.render_with_map(&program.source_map));
            eprintln!("'{}' does not parse, so it was not formatted. Fix the errors first", path);
            process::exit(1);
        }

        let lines: Vec<String> = source.lines().map(String::from).collect();
        let mut formatted = nail::formatter::format_nail_code(&lines).join("\n");
        formatted.push('\n');

        let reformatted = lex_program(&formatted, Some(Path::new(path)));
        let survived = nail::lexer::collect_lexer_errors(&reformatted.tokens).is_empty() && string_literals(&reformatted.tokens) == original_strings && parse(reformatted.tokens).is_ok();
        if !survived {
            eprintln!("Formatting '{}' would change what the code means, so nothing was written. This is a formatter bug worth reporting", path);
            process::exit(1);
        }

        let changed = formatted != source;
        if args.iter().any(|arg| arg == "--check") {
            if changed {
                println!("{}", path);
                process::exit(1);
            }
            return;
        }
        if args.iter().any(|arg| arg == "--stdout") {
            print!("{}", formatted);
            return;
        }
        if changed {
            if let Err(error) = fs::write(path, &formatted) {
                eprintln!("Error writing '{}': {}", path, error);
                process::exit(1);
            }
            println!("{}", path);
        }
        return;
    }

    let filename = &args[1];
    let mut mode = args.get(2).map(|s| s.as_str()).unwrap_or("--transpile");
    let skip_check = args.iter().any(|arg| arg == "--skip-check");
    let to_stdout = args.iter().any(|arg| arg == "--stdout");
    let no_profile = args.iter().any(|arg| arg == "--no-profile");
    let wasm_target = args.iter().any(|arg| arg == "--target=wasm");
    let json_output = args.iter().any(|arg| arg == "--json");
    let ast_dump = args.iter().any(|arg| arg == "--ast");
    let output_path = args.iter().position(|arg| arg == "-o").and_then(|i| args.get(i + 1)).cloned();
    // Flags in the mode position mean the default mode with that flag applied
    if mode == "-o" || mode == "--stdout" || mode == "--no-profile" || mode == "--target=wasm" || mode == "--json" || mode == "--ast" {
        mode = "--transpile";
    }
    // Mode flags win wherever they sit among the arguments, so `--json
    // --check-only` and `--check-only --json` mean the same thing.
    if args.iter().any(|arg| arg == "--check-only") || ast_dump {
        mode = "--check-only";
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

    // Machine-readable modes must not print pipeline banners to stdout.
    // --check-only is one of them: tools run it after every edit, and the
    // answer they want is `ok` or the errors, not a narration of stages.
    let quiet = json_output || matches!(mode, "--transpile" | "--transpile-skip-check" | "--deps-only" | "--cargo-toml" | "--stamp" | "--run" | "--build" | "--check-only");
    
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
        if json_output {
            let error = nail::common::CodeError {
                message: format!("'{}' has no version line", filename),
                code_span: nail::common::CodeSpan::default(),
                help: Some(format!("add `nail latest` as line one, or run: nailc {} --stamp=latest", filename)),
            };
            println!("{}", nail::common::diagnostics_json("version", &[error], &nail::common::SourceMap::single(filename, &input)));
            process::exit(1);
        }
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
    let program = lex_program(&input, Some(Path::new(filename)));
    let source_map = program.source_map;
    let tokens = program.tokens;
    stage_timings.push(("lex", stage_start.elapsed()));

    let lexer_errors = nail::lexer::collect_lexer_errors(&tokens);
    if !lexer_errors.is_empty() {
        if json_output {
            println!("{}", nail::common::diagnostics_json("lex", &lexer_errors, &source_map));
        } else {
            for error in &lexer_errors {
                eprint!("{}", error.render_with_map(&source_map));
            }
            let count = lexer_errors.len();
            eprintln!("{} error{} found before parsing", count, if count == 1 { "" } else { "s" });
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
            if json_output {
                println!("{}", nail::common::diagnostics_json("parse", std::slice::from_ref(&e), &source_map));
            } else {
                eprint!("{}", e.render_with_map(&source_map));
                eprintln!("1 error found while parsing");
            }
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
                if json_output {
                    println!("{}", nail::common::diagnostics_json("check", &errors, &source_map));
                } else {
                    for error in &errors {
                        eprint!("{}", error.render_with_map(&source_map));
                    }
                    let count = errors.len();
                    eprintln!("{} error{} found", count, if count == 1 { "" } else { "s" });
                    if mode != "--check-only" {
                        eprintln!("\nUse --skip-check to transpile anyway");
                    }
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
        // Success is one word. Tools run this after every edit, and the
        // type-checked AST this used to dump buried the answer under
        // thousands of lines nobody asked for. The dump is still there,
        // behind --ast, for when the tree itself is the question.
        if json_output {
            println!("{}", serde_json::json!({"status": "ok"}));
        } else if ast_dump {
            println!("{:#?}", checked_ast);
        } else {
            println!("ok");
        }
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
        // Running is iterating, so it gets the quick profile and its
        // sub-second rebuilds. Building produces the binary that ships, so
        // it pays for the full release profile. Only a release binary is
        // ever copied beside the source, which keeps one invariant: a binary
        // sitting next to its .nail file is always the shippable one.
        let profile = if mode == "--run" { nail::toolchain::BuildProfile::Quick } else { nail::toolchain::BuildProfile::Release };
        let build_dir = Path::new(filename).parent().unwrap_or(Path::new(".")).join(".nail-build");
        if let Err(error) = fs::create_dir_all(build_dir.join("src")) {
            eprintln!("Cannot create {}: {}", build_dir.display(), error);
            process::exit(1);
        }

        // An installed release builds with its own pinned toolchain and its
        // own copy of the nail crate. A development checkout uses the system
        // cargo and the crate next door.
        let bundle = nail::toolchain::BundledToolchain::detect();
        let explicit_nail_path = args.iter().any(|arg| arg.starts_with("--nail-path="));
        let nail_crate_path = match &bundle {
            Some(bundle) => bundle.nail_crate_path().display().to_string(),
            // The build directory sits beside the source rather than beside
            // the crate, so a relative --nail-path would resolve from the
            // wrong place. An explicit --nail-path is anchored to where the
            // command was run. Without one, the checkout this nailc was
            // compiled from is the crate, wherever the command was run from.
            None if explicit_nail_path => fs::canonicalize(&nail_path).map(|path| path.display().to_string()).unwrap_or_else(|_| nail_path.clone()),
            None => env!("CARGO_MANIFEST_DIR").to_string(),
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
        let status = cargo.arg("build").arg("--profile").arg(profile.name()).current_dir(&build_dir).status();
        match status {
            Ok(status) if status.success() => {}
            Ok(_) => process::exit(1),
            Err(error) => {
                eprintln!("Cannot run cargo: {}", error);
                process::exit(1);
            }
        }

        let binary = match &bundle {
            Some(bundle) => bundle.built_binary_path(package_name, profile),
            None => build_dir.join("target").join(profile.name()).join(package_name),
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
        // The program runs in its source file's directory, the same rule
        // imports follow, so a program that reads a file beside itself works
        // no matter where `nail run` was typed. The binary path is made
        // absolute first: a relative path would resolve against the new
        // working directory and miss.
        let run_dir = Path::new(filename).parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let binary = fs::canonicalize(&binary).unwrap_or(binary);
        let error = std::os::unix::process::CommandExt::exec(process::Command::new(&binary).args(program_args).current_dir(run_dir));
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