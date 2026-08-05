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

/// The segments of a normalized path. Both spellings of an empty answer,
/// `/` for an absolute path and `.` for a relative one, give an empty list.
fn normalized_segments(normalized: &str) -> Vec<String> {
    if normalized == "." {
        return Vec::new();
    }
    return normalized.split('/').filter(|segment| !segment.is_empty()).map(|segment| segment.to_string()).collect();
}

/// How many segments the path has below its root or its start: `a/b/c.txt` is
/// 3, `/` is 0 and `.` is 0. Counted on the normalized path, so `a/b/..` is 1.
pub fn depth(path: String) -> i64 {
    return normalized_segments(&normalize(path)).len() as i64;
}

/// The pieces of the path between separators, with the empties a doubled or
/// trailing separator would make dropped, so `/a//b/` gives `a` and `b`.
pub fn segments(path: String) -> Vec<String> {
    return path.split('/').filter(|segment| !segment.is_empty()).map(|segment| segment.to_string()).collect();
}

/// Whether the file name starts with a dot, the Unix spelling of hidden.
/// Judged on the final component only, so `a/.git/config` asks about
/// `config` and is not hidden.
pub fn is_hidden(path: String) -> bool {
    return Path::new(&path).file_name().and_then(|s| s.to_str()).map(|name| name.starts_with('.')).unwrap_or(false);
}

/// Whether the candidate stays inside the base once both are normalized, with
/// `..` tricks accounted for. The check a file server runs before serving a
/// requested path. Pure string work on the normalized forms, no filesystem
/// access, so a symlink that leads out of the base is not caught here.
pub fn within(base: String, candidate: String) -> bool {
    let normalized_base = normalize(base);
    let normalized_candidate = normalize(candidate);
    // An absolute path and a relative one cannot vouch for each other.
    if normalized_base.starts_with('/') != normalized_candidate.starts_with('/') {
        return false;
    }

    let base_segments = normalized_segments(&normalized_base);
    let candidate_segments = normalized_segments(&normalized_candidate);
    if candidate_segments.len() < base_segments.len() {
        return false;
    }
    if candidate_segments[..base_segments.len()] != base_segments[..] {
        return false;
    }
    // On a normalized relative path every surviving `..` sits at the front,
    // so one left after the shared prefix means the candidate climbed out.
    return !candidate_segments[base_segments.len()..].iter().any(|segment| segment == "..");
}

/// The same path carrying a different file name, with the directory and the
/// extension kept: `logs/app.log` with the stem `backup` gives
/// `logs/backup.log`. Errors on an empty stem or a path with no file name.
pub fn with_stem(path: String, stem: String) -> Result<String, String> {
    if stem.is_empty() {
        return Err("path_with_stem: the stem is empty".to_string());
    }
    let parsed = Path::new(&path);
    if parsed.file_name().is_none() {
        return Err(format!("path_with_stem: '{}' has no file name to swap", path));
    }
    let file_name = match parsed.extension().and_then(|s| s.to_str()) {
        Some(extension) => format!("{}.{}", stem, extension),
        None => stem,
    };
    let mut buffer = PathBuf::from(&path);
    buffer.set_file_name(file_name);
    return Ok(buffer.to_string_lossy().to_string());
}

/// Makes untrusted text safe to use as a single file name, the guard every
/// upload handler needs. Path separators, `..`, control characters and the
/// characters Windows refuses become underscores, leading dots become
/// underscores too (so the result cannot hide itself or name a parent), and
/// empty input gives `file`.
pub fn sanitize_filename(name: String) -> String {
    if name.is_empty() {
        return "file".to_string();
    }
    let replaced: String = name
        .chars()
        .map(|character| match character {
            '/' | '\\' => '_',
            '<' | '>' | ':' | '"' | '|' | '?' | '*' => '_',
            control if control.is_control() => '_',
            keep => keep,
        })
        .collect();
    let no_parent = replaced.replace("..", "__");
    let mut characters: Vec<char> = no_parent.chars().collect();
    for character in characters.iter_mut() {
        if *character != '.' {
            break;
        }
        *character = '_';
    }
    return characters.into_iter().collect();
}

/// The longest directory prefix every path shares, whole segments at a time,
/// so `/apple/x` and `/app/y` share only the root and not `/app`. Paths are
/// normalized first. An empty array, or paths with nothing shared, give an
/// empty string, and absolute paths that share only the root give `/`.
pub fn common_prefix(paths: Vec<String>) -> String {
    if paths.is_empty() {
        return String::new();
    }
    let normalized: Vec<String> = paths.into_iter().map(normalize).collect();
    let from_the_root = normalized[0].starts_with('/');
    // A mix of absolute and relative paths shares nothing.
    if normalized.iter().any(|path| path.starts_with('/') != from_the_root) {
        return String::new();
    }

    let split: Vec<Vec<String>> = normalized.iter().map(|path| normalized_segments(path)).collect();
    let mut shared: Vec<String> = split[0].clone();
    for one in &split[1..] {
        let mut kept = 0;
        while kept < shared.len() && kept < one.len() && shared[kept] == one[kept] {
            kept += 1;
        }
        shared.truncate(kept);
    }

    if from_the_root {
        return format!("/{}", shared.join("/"));
    }
    return shared.join("/");
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

    #[test]
    fn depth_counts_segments_below_the_start() {
        assert_eq!(depth("a/b/c.txt".to_string()), 3);
        assert_eq!(depth("/".to_string()), 0);
        assert_eq!(depth(".".to_string()), 0);
        assert_eq!(depth("/etc/hosts".to_string()), 2);
        assert_eq!(depth("a/b/..".to_string()), 1);
    }

    #[test]
    fn segments_are_the_pieces_between_separators() {
        assert_eq!(segments("a/b/c.txt".to_string()), vec!["a", "b", "c.txt"]);
        assert_eq!(segments("/a//b/".to_string()), vec!["a", "b"]);
        assert!(segments("/".to_string()).is_empty());
        assert!(segments("".to_string()).is_empty());
    }

    #[test]
    fn hidden_asks_about_the_file_name_only() {
        assert!(is_hidden(".env".to_string()));
        assert!(is_hidden("a/.git".to_string()));
        assert!(!is_hidden("a/.git/config".to_string()));
        assert!(!is_hidden("notes.txt".to_string()));
        assert!(!is_hidden("..".to_string()));
    }

    #[test]
    fn within_keeps_a_candidate_inside_the_base() {
        assert!(within("/srv/files".to_string(), "/srv/files/a/b.txt".to_string()));
        assert!(within("/srv/files".to_string(), "/srv/files".to_string()));
        assert!(within("base".to_string(), "base/sub/../file".to_string()));
        assert!(within(".".to_string(), "a/b".to_string()));
    }

    #[test]
    fn within_catches_the_dot_dot_escapes() {
        assert!(!within("base".to_string(), "base/../../x".to_string()));
        assert!(!within("/srv/files".to_string(), "/srv/files/../secrets".to_string()));
        assert!(!within(".".to_string(), "../x".to_string()));
        assert!(!within("/srv/files".to_string(), "relative/path".to_string()));
        // A shared run of characters that is not a whole segment does not count.
        assert!(!within("/srv/files".to_string(), "/srv/filesystem".to_string()));
    }

    #[test]
    fn a_stem_swap_keeps_the_directory_and_extension() {
        assert_eq!(with_stem("logs/app.log".to_string(), "backup".to_string()).expect("a file name"), "logs/backup.log");
        assert_eq!(with_stem("app.log".to_string(), "backup".to_string()).expect("a file name"), "backup.log");
        assert_eq!(with_stem("/tmp/README".to_string(), "NOTES".to_string()).expect("a file name"), "/tmp/NOTES");
    }

    #[test]
    fn a_stem_swap_needs_a_stem_and_a_file_name() {
        assert!(with_stem("logs/app.log".to_string(), "".to_string()).unwrap_err().contains("empty"));
        assert!(with_stem("/".to_string(), "backup".to_string()).unwrap_err().contains("no file name"));
        assert!(with_stem("..".to_string(), "backup".to_string()).unwrap_err().contains("no file name"));
    }

    #[test]
    fn sanitizing_leaves_a_single_safe_file_name() {
        let cleaned = sanitize_filename("../../etc/passwd".to_string());
        assert!(!cleaned.contains('/'));
        assert!(!cleaned.contains(".."));
        assert_eq!(sanitize_filename("report<final>.pdf".to_string()), "report_final_.pdf");
        assert_eq!(sanitize_filename(".env".to_string()), "_env");
        assert_eq!(sanitize_filename("..".to_string()), "__");
        assert_eq!(sanitize_filename("".to_string()), "file");
        assert_eq!(sanitize_filename("tab\there".to_string()), "tab_here");
        assert_eq!(sanitize_filename("plain name.txt".to_string()), "plain name.txt");
    }

    #[test]
    fn the_common_prefix_is_whole_shared_segments() {
        assert_eq!(common_prefix(vec!["/srv/app/logs/a.log".to_string(), "/srv/app/data/b.db".to_string()]), "/srv/app");
        assert_eq!(common_prefix(vec!["a/b/c".to_string(), "a/b".to_string()]), "a/b");
        // A shared run of characters that is not a whole segment does not count.
        assert_eq!(common_prefix(vec!["/apple/x".to_string(), "/app/y".to_string()]), "/");
        assert_eq!(common_prefix(vec!["a/x".to_string(), "b/y".to_string()]), "");
        assert_eq!(common_prefix(vec![]), "");
        assert_eq!(common_prefix(vec!["/a/b".to_string(), "relative".to_string()]), "");
        assert_eq!(common_prefix(vec!["a/b/c".to_string()]), "a/b/c");
    }
}
