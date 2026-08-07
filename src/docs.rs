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
/// The specification and the README teach the language by showing it, and
/// `nail docs <topic>` prints those same blocks in the terminal. Nobody was
/// checking them, so a block could teach a syntax the language never had and
/// nothing would notice. What a fenced block claims about itself decides how
/// hard it is checked:
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
    /// Read rather than embedded: the README is documentation for people
    /// reading the repository, and nothing at runtime wants it. Embedding it
    /// would make it a file every user build has to be shipped.
    fn readme() -> String {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/README.md");
        return std::fs::read_to_string(path).expect("the README is beside Cargo.toml");
    }

    /// The directory an example's `import` is resolved against. A block that
    /// imports `math_helpers.nail` is showing what a reader's own project
    /// looks like, so the file it names has to exist somewhere: these are in
    /// tests/docs_imports/, and they are ordinary Nail checked like any other.
    const IMPORT_BASE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/docs_imports/block.nail");

    struct Fence {
        source: String,
        language: String,
        document: &'static str,
        line: usize,
    }

    /// Every fenced block in a markdown document, with the line it starts on
    /// so a failure can be found by eye.
    fn fences(document: &'static str, text: &str) -> Vec<Fence> {
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
            found.push(Fence { source: body, language, document, line: index + 1 });
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
        all.extend(fences("README.md", &readme()));
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
