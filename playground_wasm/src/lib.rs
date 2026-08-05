//! The website playground: the real Nail compiler compiled to WebAssembly.
//! One export takes a source string and returns JSON with the compiler's
//! verdict and the generated Rust, so the page needs no server round trip.
//!
//! import() is the one thing that cannot work here: it reads files at lex
//! time and a browser has no files, so it surfaces as an ordinary lexer
//! error rather than anything special.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn playground_review(source: String) -> String {
    review(&source).to_string()
}

fn review(source: &str) -> serde_json::Value {
    let filename = "playground.nail";

    let tokens = nail::lexer::lexer(source);
    let lexer_errors = nail::lexer::collect_lexer_errors(&tokens);
    if !lexer_errors.is_empty() {
        let verdict: String = lexer_errors.iter().map(|error| error.render(filename, source)).collect();
        return refusal(verdict);
    }

    let mut ast = match nail::parser::parse(tokens) {
        Ok(ast) => ast,
        Err(error) => return refusal(error.render(filename, source)),
    };

    if let Err(errors) = nail::checker::checker(&mut ast) {
        let count = errors.len();
        let mut verdict: String = errors.iter().map(|error| error.render(filename, source)).collect();
        verdict.push_str(&format!("{} error{} found\n", count, if count == 1 { "" } else { "s" }));
        return refusal(verdict);
    }

    let mut transpiler = nail::transpiler::Transpiler::new();
    match transpiler.transpile(&ast) {
        Ok(rust) => serde_json::json!({ "ok": true, "verdict": "Compiles clean.", "rust": rust, "highlighted": highlight(source) }),
        Err(error) => refusal(error.render(filename, source)),
    }
}

// The page repaints an edited pane with the compiler's own colorizer when
// focus leaves it, so hand-typed code ends up looking like the originals.
fn highlight(source: &str) -> String {
    nail::std_lib::code::highlight_html(source.to_string())
}

fn refusal(verdict: String) -> serde_json::Value {
    serde_json::json!({ "ok": false, "verdict": verdict, "rust": "", "highlighted": "" })
}

#[cfg(test)]
mod tests {
    use super::review;

    #[test]
    fn clean_program_returns_rust() {
        let result = review("f double_it(value:i):i { r value * 2; }\nprint(double_it(21));");
        assert_eq!(result["ok"], true);
        assert_eq!(result["verdict"], "Compiles clean.");
        assert!(result["rust"].as_str().unwrap().contains("fn double_it"));
    }

    #[test]
    fn type_error_is_rendered_with_caret() {
        let result = review("wrong:i = `text`;");
        assert_eq!(result["ok"], false);
        let verdict = result["verdict"].as_str().unwrap();
        assert!(verdict.contains("error:"), "got: {}", verdict);
        assert!(verdict.contains("playground.nail"), "got: {}", verdict);
        assert_eq!(result["rust"], "");
    }

    #[test]
    fn nonsense_still_answers_instead_of_panicking() {
        let result = review("}}}{{{");
        assert_eq!(result["ok"], false);
    }
}
