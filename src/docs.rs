//! The language reference, compiled into the compiler.
//!
//! The standard library can be searched because the registry describes itself,
//! but the language cannot: somebody meeting `a:i`, `h<s,s>` or `!e` for the
//! first time has nowhere to look without leaving the terminal. The
//! specification already explains all of it, so it is embedded here rather
//! than summarised, which would be a second copy to keep true.
//!
//! Embedding rather than reading from disk matters for the same reason the
//! standard library listing comes from the registry: the answer belongs to the
//! version that will compile the code, not to whatever happens to be checked
//! out or installed beside it.

const SPECIFICATION: &str = include_str!("../nail_language_spec.md");

/// The agent primer: the whole language on one page, written to be loaded
/// into a model's context (or a person's) before writing any Nail. Embedded
/// for the same reason the specification is: the answer belongs to the
/// version that will compile the code.
const PRIMER: &str = include_str!("../nail_for_agents.md");

/// The one-page briefing `nail docs primer` prints and the website serves at
/// /llms.txt for tools that fetch documentation over the network.
pub fn primer() -> &'static str {
    return PRIMER;
}

/// The heading of every top-level section, in the order the specification puts
/// them.
pub fn topics() -> Vec<&'static str> {
    return SPECIFICATION.lines().filter_map(|line| line.strip_prefix("## ")).collect();
}

/// The section a query names, heading included, up to the next section.
///
/// Matching is deliberately loose, because nobody types "Error Handling" when
/// they want to know about errors. A trailing plural is dropped so `types`
/// finds the section on the type system, and a query matches anywhere in the
/// heading so `pinning` finds versioning.
pub fn section(query: &str) -> Option<String> {
    let wanted = query.trim().to_lowercase();
    let stem = wanted.strip_suffix('s').unwrap_or(&wanted);

    let mut lines = SPECIFICATION.lines().peekable();
    while let Some(line) = lines.next() {
        let heading = match line.strip_prefix("## ") {
            Some(heading) => heading,
            None => continue,
        };
        let heading_lower = heading.to_lowercase();
        if !heading_lower.contains(stem) {
            continue;
        }

        let mut body = String::from(line);
        body.push('\n');
        for line in lines.by_ref() {
            if line.starts_with("## ") {
                break;
            }
            body.push_str(line);
            body.push('\n');
        }
        return Some(body.trim_end().to_string());
    }
    return None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_specification_came_along() {
        assert!(SPECIFICATION.len() > 50_000, "the specification should be embedded whole");
        assert!(topics().len() > 20, "expected the specification's sections, found {}", topics().len());
    }

    #[test]
    fn a_topic_is_found_by_the_word_someone_would_type() {
        // Nobody types "Error Handling" when they want to know about errors.
        for (query, expected) in [("errors", "Error Handling"), ("types", "Type System"), ("enums", "Enums"), ("structs", "Struct")] {
            let found = section(query).unwrap_or_else(|| panic!("`{}` should find a section", query));
            assert!(found.contains(expected), "`{}` should have found {}, got: {}", query, expected, found.lines().next().unwrap_or(""));
        }
    }

    #[test]
    fn a_section_stops_at_the_next_one() {
        let found = section("enums").expect("enums is a section");
        assert_eq!(found.matches("\n## ").count(), 0, "a section should not run into the next");
    }

    #[test]
    fn an_unknown_topic_is_not_invented() {
        assert!(section("zzzznotatopic").is_none());
    }
}

/// The Nail in the documentation, compiled.
///
/// The documentation teaches the language by showing it, and `nail docs
/// <topic>` prints those same blocks in the terminal. Nobody was checking
/// them, so a block could teach a syntax the language never had and nothing
/// would notice - the blog example's posts shipped years of pre-rewrite Nail
/// exactly that way. So every markdown file in the repository is swept, not a
/// curated list: a document that opts out by omission is the failure mode
/// this module exists to close. What a fenced block claims about itself
/// decides how hard it is checked:
///
///   ```nail            a whole program: lexes, parses and type checks
///   ```nail-fragment   a piece of one: lexes and parses, its context is prose
///   ```nail-refused    code the compiler must refuse, shown to say why
///
/// Any other fence (js, ebnf, bash, plain) is not Nail and is not checked.
/// The point of three names rather than one is that each says what it wants
/// to be true, so nothing is skipped by omission.
#[cfg(test)]
mod documentation_code_tests {
    /// Every markdown file in the repository, read from disk, as
    /// (path relative to the root, contents). Read rather than embedded:
    /// these are documentation for people reading the repository, and nothing
    /// at runtime wants them. The specification is the one exception - the
    /// embedded copy is the one that ships inside the compiler, so that is
    /// the copy the tests check, and its on-disk twin is skipped here.
    /// Directories of generated or foreign files (build output, wasm-pack's
    /// pkg folders) are not documentation and are left out.
    pub fn repository_markdown() -> Vec<(String, String)> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let skipped_directories = [".git", "target", "node_modules", "pkg", ".nail-build"];
        let mut found = Vec::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(&directory).expect("a repository directory is readable") {
                let path = entry.expect("a repository directory entry is readable").path();
                let name = path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
                if path.is_dir() {
                    if !skipped_directories.contains(&name.as_str()) {
                        pending.push(path);
                    }
                    continue;
                }
                if !name.ends_with(".md") {
                    continue;
                }
                let relative = path.strip_prefix(root).expect("walked down from the root").to_string_lossy().to_string();
                if relative == "nail_language_spec.md" {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{} is readable", relative));
                found.push((relative, text));
            }
        }
        found.sort();
        return found;
    }

    /// The directory an example's `import` is resolved against. A block that
    /// imports `math_helpers.nail` is showing what a reader's own project
    /// looks like, so the file it names has to exist somewhere: these are in
    /// tests/docs_imports/, and they are ordinary Nail checked like any other.
    const IMPORT_BASE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/docs_imports/block.nail");

    struct Fence {
        source: String,
        language: String,
        document: String,
        line: usize,
    }

    /// Every fenced block in a markdown document, with the line it starts on
    /// so a failure can be found by eye.
    fn fences(document: &str, text: &str) -> Vec<Fence> {
        let mut found = Vec::new();
        let mut lines = text.lines().enumerate();
        while let Some((index, line)) = lines.next() {
            let language = match line.strip_prefix("```") {
                Some(language) if !language.is_empty() => language.trim().to_string(),
                _ => continue,
            };
            let mut body = String::new();
            for (_, line) in lines.by_ref() {
                if line.starts_with("```") {
                    break;
                }
                body.push_str(line);
                body.push('\n');
            }
            found.push(Fence { source: body, language, document: document.to_string(), line: index + 1 });
        }
        return found;
    }

    /// Documentation shows the code without the version line most of the time,
    /// because the line is noise once it has been explained. The compiler
    /// still wants it.
    fn as_a_file(source: &str) -> String {
        let first = source.lines().next().unwrap_or("").trim();
        let already_pinned = first.starts_with("nail ") && first.split_whitespace().count() == 2;
        if already_pinned {
            return source.to_string();
        }
        return format!("nail latest\n{}", source);
    }

    fn all_fences() -> Vec<Fence> {
        let mut all = fences("nail_language_spec.md", super::SPECIFICATION);
        for (document, text) in repository_markdown() {
            all.extend(fences(&document, &text));
        }
        return all;
    }

    #[test]
    fn every_nail_block_in_the_documentation_is_a_whole_program() {
        let mut broken: Vec<String> = Vec::new();
        for fence in all_fences().iter().filter(|fence| fence.language == "nail") {
            let source = as_a_file(&fence.source);
            let tokens = crate::lexer::lexer_with_context(&source, Some(std::path::Path::new(IMPORT_BASE)));
            let mut ast = match crate::parser::parse(tokens) {
                Ok(ast) => ast,
                Err(error) => {
                    broken.push(format!("  {}:{} does not parse: {}", fence.document, fence.line, error.message.lines().next().unwrap_or("").trim()));
                    continue;
                }
            };
            if let Err(errors) = crate::checker::checker(&mut ast) {
                let first = errors.first().map(|error| error.message.lines().next().unwrap_or("").trim().to_string()).unwrap_or_default();
                broken.push(format!("  {}:{} does not type check: {}", fence.document, fence.line, first));
            }
        }
        broken.sort();
        assert!(
            broken.is_empty(),
            "a ```nail block is a program someone can copy whole. Mark it ```nail-fragment if it needs context, or ```nail-refused if the compiler is meant to reject it:\n{}",
            broken.join("\n")
        );
    }

    #[test]
    fn every_nail_fragment_in_the_documentation_is_nail() {
        let mut broken: Vec<String> = Vec::new();
        for fence in all_fences().iter().filter(|fence| fence.language == "nail-fragment") {
            let source = as_a_file(&fence.source);
            let tokens = crate::lexer::lexer_with_context(&source, Some(std::path::Path::new(IMPORT_BASE)));
            if let Err(error) = crate::parser::parse(tokens) {
                broken.push(format!("  {}:{} does not parse: {}", fence.document, fence.line, error.message.lines().next().unwrap_or("").trim()));
            }
        }
        broken.sort();
        assert!(broken.is_empty(), "a fragment still has to be Nail, whatever context it is missing:\n{}", broken.join("\n"));
    }

    #[test]
    fn every_refused_block_in_the_documentation_is_still_refused() {
        let mut accepted: Vec<String> = Vec::new();
        for fence in all_fences().iter().filter(|fence| fence.language == "nail-refused") {
            let source = as_a_file(&fence.source);
            let tokens = crate::lexer::lexer_with_context(&source, Some(std::path::Path::new(IMPORT_BASE)));
            let refused = match crate::parser::parse(tokens) {
                Ok(mut ast) => crate::checker::checker(&mut ast).is_err(),
                Err(_) => true,
            };
            if !refused {
                accepted.push(format!("  {}:{}", fence.document, fence.line));
            }
        }
        accepted.sort();
        assert!(
            accepted.is_empty(),
            "these blocks are shown as code the compiler rejects, and it now accepts them, so either the text or the compiler is wrong:\n{}",
            accepted.join("\n")
        );
    }
}

/// The paths in the documentation, resolved.
///
/// Prose that names a file keeps working only while something checks it. When
/// the shell scripts moved into scripts/, every README that said
/// `./test_e2e.sh` went quietly stale, and one agent definition spent months
/// telling its reader to run a script deleted long before. These tests make
/// that class of rot a failing build instead of an archaeology find.
#[cfg(test)]
mod documentation_reference_tests {
    fn root() -> &'static std::path::Path {
        return std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    }

    /// Every shell-script path mentioned in a text, as written. A token is a
    /// run of path characters ending in `.sh`.
    fn shell_script_mentions(text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let mut token = String::new();
        for character in text.chars().chain(std::iter::once('\n')) {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '/' | '-') {
                token.push(character);
                continue;
            }
            if token.ends_with(".sh") && token.trim_start_matches("./").len() > ".sh".len() {
                found.push(token.clone());
            }
            token.clear();
        }
        return found;
    }

    #[test]
    fn every_shell_script_named_in_the_documentation_exists() {
        // A path with a directory must exist exactly where it says, and `./`
        // means the repository root, because that is where the reader will
        // type it. A bare name is looser: it may live in any of the
        // directories scripts are kept. Absolute paths are somebody else's
        // machine (droplet instructions) and are not checked.
        let script_homes = ["", "scripts", "bundle", "deploy"];
        let mut missing: Vec<String> = Vec::new();
        for (document, text) in super::documentation_code_tests::repository_markdown() {
            for mention in shell_script_mentions(&text) {
                if mention.starts_with('/') {
                    continue;
                }
                let path = mention.trim_start_matches("./");
                let exists = if path.contains('/') {
                    root().join(path).is_file()
                } else {
                    script_homes.iter().any(|home| root().join(home).join(path).is_file())
                };
                if !exists {
                    missing.push(format!("  {} names `{}`, which does not exist", document, mention));
                }
            }
        }
        missing.sort();
        missing.dedup();
        assert!(missing.is_empty(), "a script the documentation tells someone to run has to exist where it says:\n{}", missing.join("\n"));
    }

    #[test]
    fn the_deploy_scripts_data_paths_exist_and_are_documented() {
        // The website reads these at runtime, so a missing entry is a deployed
        // site that panics on startup, and an undocumented one is a runbook
        // that lies about what ships.
        let script = std::fs::read_to_string(root().join("scripts/deploy.sh")).expect("scripts/deploy.sh is readable");
        let runbook = std::fs::read_to_string(root().join("deploy/README.md")).expect("deploy/README.md is readable");
        let mut entries: Vec<String> = Vec::new();
        let mut inside = false;
        for line in script.lines() {
            let line = line.trim();
            if line.starts_with("DATA_PATHS=(") {
                inside = true;
                continue;
            }
            if inside {
                if line == ")" {
                    break;
                }
                if !line.is_empty() && !line.starts_with('#') {
                    entries.push(line.to_string());
                }
            }
        }
        assert!(entries.len() >= 4, "DATA_PATHS should have been found in scripts/deploy.sh, got {} entries", entries.len());
        let mut problems: Vec<String> = Vec::new();
        for entry in &entries {
            if !root().join(entry).exists() {
                problems.push(format!("  {} is shipped by scripts/deploy.sh but does not exist in the repository", entry));
            }
            if !runbook.contains(entry) {
                problems.push(format!("  {} is shipped by scripts/deploy.sh but deploy/README.md does not mention it", entry));
            }
        }
        assert!(problems.is_empty(), "DATA_PATHS and the deploy runbook have drifted apart:\n{}", problems.join("\n"));
    }
}
