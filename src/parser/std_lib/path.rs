use std::path::{Path, PathBuf};

pub fn join(base: String, path: String) -> String {
    Path::new(&base).join(&path).to_string_lossy().to_string()
}

pub fn exists(path: String) -> bool {
    Path::new(&path).exists()
}

/// Get the filename from a path
pub fn basename(path: String) -> String {
    Path::new(&path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

/// Get the directory from a path
pub fn dirname(path: String) -> String {
    Path::new(&path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("")
        .to_string()
}

/// Get the file extension from a path
pub fn extension(path: String) -> Result<String, String> {
    Path::new(&path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("path_extension: no file extension in '{}'", path))
}

/// Check if a path is absolute
pub fn is_absolute(path: String) -> bool {
    Path::new(&path).is_absolute()
}

/// Normalize a path (resolve . and ..)
pub fn normalize(path: String) -> String {
    let parsed = Path::new(&path);
    let from_the_root = parsed.is_absolute();
    let mut components: Vec<String> = Vec::new();

    for component in parsed.components() {
        use std::path::Component;
        match component {
            // The leading separator is put back at the end, so it cannot end
            // up doubled by the join.
            Component::RootDir | Component::Prefix(_) => {}
            Component::CurDir => {}
            Component::ParentDir => match components.last().map(|name| name.as_str()) {
                // A `..` after a real name cancels that name out.
                Some(name) if name != ".." => {
                    components.pop();
                }
                // There is nothing above the root, so there it is dropped. On
                // a relative path it has to be kept: `../a` is not `a`.
                _ => {
                    if !from_the_root {
                        components.push("..".to_string());
                    }
                }
            },
            named => components.push(named.as_os_str().to_string_lossy().to_string()),
        }
    }

    if from_the_root {
        return format!("/{}", components.join("/"));
    }
    if components.is_empty() {
        return ".".to_string();
    }
    return components.join("/");
}
/// Whether a path matches a glob pattern - the shell spelling, since that is
/// the one everybody already knows:
///
///   `*`  matches any run of characters within one path segment
///   `?`  matches one character within one segment
///   `**` matches any number of whole segments, including none
///   `[abc]` and `[a-z]` match one of the characters listed, and `[!abc]` one
///   that is not
///
/// The reason `*` stops at a separator is so `*.nail` means the files here and
/// `**/*.nail` means the files anywhere below - which is what a person writing
/// either one intends.
pub fn matches_glob(pattern: &String, path: &String) -> bool {
    let pattern_segments: Vec<&str> = pattern.split('/').collect();
    let path_segments: Vec<&str> = path.split('/').collect();
    return segments_match(&pattern_segments, &path_segments);
}

/// Matches whole path segments, so `**` can consume as many as it needs. Tried
/// shortest-first and backtracking, which is slower than a compiled matcher but
/// is a few lines and cannot get a pattern wrong.
fn segments_match(pattern: &[&str], path: &[&str]) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }
    if pattern[0] == "**" {
        // A trailing `**` takes everything that is left, including nothing.
        for taken in 0..=path.len() {
            if segments_match(&pattern[1..], &path[taken..]) {
                return true;
            }
        }
        return false;
    }
    if path.is_empty() {
        return false;
    }
    if !segment_matches(pattern[0], path[0]) {
        return false;
    }
    return segments_match(&pattern[1..], &path[1..]);
}

/// Matches one segment against one pattern segment.
fn segment_matches(pattern: &str, segment: &str) -> bool {
    let pattern_characters: Vec<char> = pattern.chars().collect();
    let segment_characters: Vec<char> = segment.chars().collect();
    return characters_match(&pattern_characters, &segment_characters);
}

fn characters_match(pattern: &[char], text: &[char]) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }
    match pattern[0] {
        '*' => {
            for taken in 0..=text.len() {
                if characters_match(&pattern[1..], &text[taken..]) {
                    return true;
                }
            }
            return false;
        }
        '?' => {
            if text.is_empty() {
                return false;
            }
            return characters_match(&pattern[1..], &text[1..]);
        }
        '[' => {
            if text.is_empty() {
                return false;
            }
            let close = match pattern.iter().position(|character| *character == ']') {
                Some(position) if position > 1 => position,
                // A `[` with no `]` after it is a literal bracket, which is
                // what a shell does with it too.
                _ => return text[0] == '[' && characters_match(&pattern[1..], &text[1..]),
            };
            let (negated, class_start) = if pattern[1] == '!' || pattern[1] == '^' { (true, 2) } else { (false, 1) };
            let mut found = false;
            let mut position = class_start;
            while position < close {
                // A hyphen between two characters is a range, unless it is the
                // first or last thing in the class.
                if position + 2 < close && pattern[position + 1] == '-' {
                    if pattern[position] <= text[0] && text[0] <= pattern[position + 2] {
                        found = true;
                    }
                    position += 3;
                } else {
                    if pattern[position] == text[0] {
                        found = true;
                    }
                    position += 1;
                }
            }
            if found == negated {
                return false;
            }
            return characters_match(&pattern[close + 1..], &text[1..]);
        }
        literal => {
            if text.is_empty() || text[0] != literal {
                return false;
            }
            return characters_match(&pattern[1..], &text[1..]);
        }
    }
}

#[cfg(test)]
mod glob_tests {
    use super::*;

    fn matches(pattern: &str, path: &str) -> bool {
        return matches_glob(&pattern.to_string(), &path.to_string());
    }

    #[test]
    fn a_star_stays_inside_one_segment() {
        assert!(matches("*.nail", "main.nail"));
        assert!(!matches("*.nail", "tests/main.nail"));
        assert!(matches("tests/*.nail", "tests/main.nail"));
        assert!(matches("*", "anything"));
    }

    #[test]
    fn two_stars_cross_segments() {
        assert!(matches("**/*.nail", "tests/deep/main.nail"));
        assert!(matches("**/*.nail", "main.nail"));
        assert!(matches("src/**", "src/parser/std_lib/path.rs"));
        assert!(matches("src/**/mod.rs", "src/mod.rs"));
        assert!(matches("src/**/mod.rs", "src/parser/std_lib/mod.rs"));
        assert!(!matches("src/**/mod.rs", "tests/parser/mod.rs"));
    }

    #[test]
    fn a_question_mark_is_one_character() {
        assert!(matches("test_?.nail", "test_1.nail"));
        assert!(!matches("test_?.nail", "test_11.nail"));
        assert!(!matches("test_?.nail", "test_.nail"));
    }

    #[test]
    fn a_class_matches_one_of_the_characters_listed() {
        assert!(matches("test_[123].nail", "test_2.nail"));
        assert!(!matches("test_[123].nail", "test_4.nail"));
        assert!(matches("[a-z]*.rs", "main.rs"));
        assert!(!matches("[a-z]*.rs", "Main.rs"));
        assert!(matches("test_[!x].nail", "test_1.nail"));
        assert!(!matches("test_[!x].nail", "test_x.nail"));
    }

    #[test]
    fn a_pattern_with_no_wildcards_is_a_path() {
        assert!(matches("src/main.rs", "src/main.rs"));
        assert!(!matches("src/main.rs", "src/other.rs"));
        assert!(!matches("src/main.rs", "src/main.rs.bak"));
    }

    #[test]
    fn the_extension_dot_is_not_a_wildcard() {
        assert!(!matches("*.nail", "mainXnail"));
    }
}

/// The file name with its extension taken off - `report.tar.gz` gives
/// `report.tar`, because only the last extension is one. The half of a
/// filename a program actually names things after.
pub fn stem(path: String) -> String {
    return Path::new(&path).file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
}

/// The same path written from somewhere else: `path_relative_to("/a/b/c.txt",
/// "/a")` is `b/c.txt`. Errors when the path is not underneath the base, since
/// the honest answer then would be a pile of `..` nobody asked for.
pub fn relative_to(path: String, base: String) -> Result<String, String> {
    let normalized_path = normalize(path.clone());
    let normalized_base = normalize(base.clone());
    if normalized_base == "." {
        return Ok(normalized_path);
    }

    let path_segments: Vec<&str> = normalized_path.split('/').collect();
    let base_segments: Vec<&str> = normalized_base.split('/').collect();
    if path_segments.len() < base_segments.len() || path_segments[..base_segments.len()] != base_segments[..] {
        return Err(format!("path_relative_to: '{}' is not inside '{}'", path, base));
    }

    let rest = &path_segments[base_segments.len()..];
    if rest.is_empty() {
        return Ok(".".to_string());
    }
    return Ok(rest.join("/"));
}

/// The path as an absolute one, resolved against the directory the program is
/// running in. Does not touch the filesystem, so it works for a file that does
/// not exist yet.
pub fn absolute(path: String) -> Result<String, String> {
    if is_absolute(path.clone()) {
        return Ok(normalize(path));
    }
    let current = std::env::current_dir().map_err(|e| format!("path_absolute: could not read the current directory: {}", e))?;
    let joined = current.join(&path);
    return Ok(normalize(joined.to_string_lossy().to_string()));
}

/// The same path carrying a different extension, with or without the dot in
/// the extension you pass. An empty extension takes the extension off.
pub fn with_extension(path: String, extension: String) -> String {
    let wanted = extension.strip_prefix('.').unwrap_or(&extension);
    let mut buffer = PathBuf::from(&path);
    if wanted.is_empty() {
        buffer.set_extension("");
    } else {
        buffer.set_extension(wanted);
    }
    return buffer.to_string_lossy().to_string();
}

#[cfg(test)]
mod path_tests {
    use super::*;

    #[test]
    fn normalizing_keeps_the_leading_separator_and_what_it_cannot_resolve() {
        assert_eq!(normalize("/a/./b/../c".to_string()), "/a/c");
        assert_eq!(normalize("a/./b/../c".to_string()), "a/c");
        assert_eq!(normalize("/".to_string()), "/");
        assert_eq!(normalize("./".to_string()), ".");
        // Nothing lives above the root, but on a relative path `..` has to stay.
        assert_eq!(normalize("/../etc".to_string()), "/etc");
        assert_eq!(normalize("../etc".to_string()), "../etc");
        assert_eq!(normalize("../../a".to_string()), "../../a");
    }

    #[test]
    fn a_stem_drops_only_the_last_extension() {
        assert_eq!(stem("report.txt".to_string()), "report");
        assert_eq!(stem("/tmp/report.tar.gz".to_string()), "report.tar");
        assert_eq!(stem("/tmp/README".to_string()), "README");
        assert_eq!(stem("".to_string()), "");
    }

    #[test]
    fn a_relative_path_is_what_is_left_below_the_base() {
        assert_eq!(relative_to("/a/b/c.txt".to_string(), "/a".to_string()).expect("inside"), "b/c.txt");
        assert_eq!(relative_to("/a/b".to_string(), "/a/b".to_string()).expect("inside"), ".");
        assert_eq!(relative_to("src/main.rs".to_string(), "src".to_string()).expect("inside"), "main.rs");
    }

    #[test]
    fn a_path_outside_the_base_is_an_error() {
        assert!(relative_to("/a/b".to_string(), "/other".to_string()).unwrap_err().contains("not inside"));
        // A shared prefix that is not a whole segment does not count.
        assert!(relative_to("/apple/b".to_string(), "/app".to_string()).unwrap_err().contains("not inside"));
    }

    #[test]
    fn an_absolute_path_is_left_alone() {
        assert_eq!(absolute("/tmp/../tmp/file.txt".to_string()).expect("absolute"), "/tmp/file.txt");
    }

    #[test]
    fn a_relative_path_is_resolved_against_the_current_directory() {
        let resolved = absolute("file.txt".to_string()).expect("a readable current directory");
        assert!(resolved.starts_with('/'));
        assert!(resolved.ends_with("/file.txt"));
    }

    #[test]
    fn an_extension_can_be_swapped_or_removed() {
        assert_eq!(with_extension("report.txt".to_string(), "md".to_string()), "report.md");
        assert_eq!(with_extension("report.txt".to_string(), ".md".to_string()), "report.md");
        assert_eq!(with_extension("/tmp/report".to_string(), "json".to_string()), "/tmp/report.json");
        assert_eq!(with_extension("report.txt".to_string(), "".to_string()), "report");
    }
}
