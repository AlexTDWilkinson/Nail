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
