//! INI text in and out, for the config files that never went away. Both `=`
//! and `:` separate a key from its value, whitespace around everything is
//! forgiven, later duplicate keys win for reading, and full line comments
//! start with `;` or `#`. An empty section name means the top of the file
//! before any header.

/// The name inside the brackets when the line is a section header.
fn header_name(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            return Some(rest[..end].trim());
        }
    }
    return None;
}

/// Blank lines and full line comments carry no data.
fn is_comment_or_blank(line: &str) -> bool {
    let trimmed = line.trim();
    return trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#');
}

/// The key and raw value either side of the first `=` or `:`.
fn key_value(line: &str) -> Option<(&str, &str)> {
    let sep = line.find(|c| c == '=' || c == ':')?;
    let key = line[..sep].trim();
    if key.is_empty() {
        return None;
    }
    return Some((key, &line[sep + 1..]));
}

/// A value ready to hand back. One layer of matching quotes is removed, and
/// an unquoted value loses its inline `;` or `#` comment and its whitespace.
fn clean_value(raw: &str) -> String {
    let trimmed = raw.trim();
    for quote in ['"', '\''] {
        if trimmed.len() >= 2 && trimmed.starts_with(quote) {
            if let Some(end) = trimmed[1..].find(quote) {
                return trimmed[1..1 + end].to_string();
            }
        }
    }
    let cut = match trimmed.find(|c| c == ';' || c == '#') {
        Some(i) => &trimmed[..i],
        None => trimmed,
    };
    return cut.trim().to_string();
}

/// Walks the lines once. Says whether the section was ever seen and holds
/// the value of the last matching key, since later duplicates win.
fn lookup(text: &str, wanted_section: &str, wanted_key: &str) -> (bool, Option<String>) {
    let mut section_seen = wanted_section.is_empty();
    let mut found = None;
    let mut current = String::new();
    for line in text.lines() {
        if let Some(name) = header_name(line) {
            current = name.to_string();
            if current == wanted_section {
                section_seen = true;
            }
            continue;
        }
        if is_comment_or_blank(line) || current != wanted_section {
            continue;
        }
        if let Some((key_name, raw)) = key_value(line) {
            if key_name == wanted_key {
                found = Some(clean_value(raw));
            }
        }
    }
    return (section_seen, found);
}

/// The section each line belongs to, with a header line owning itself and
/// lines before any header owned by the empty name.
fn owners(lines: &[String]) -> Vec<String> {
    let mut current = String::new();
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        if let Some(name) = header_name(line) {
            current = name.to_string();
        }
        out.push(current.clone());
    }
    return out;
}

/// Joins edited lines back into text, keeping the original's choice about a
/// trailing newline.
fn rejoin(lines: Vec<String>, original: &str) -> String {
    let mut out = lines.join("\n");
    if original.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }
    return out;
}

/// The value under a section, where an empty section name means the top of
/// the file before any header. The value comes back trimmed, unquoted and
/// with any inline comment stripped. A missing section or key is named.
pub fn get(text: String, section: String, key: String) -> Result<String, String> {
    let wanted_section = section.trim();
    let wanted_key = key.trim();
    let (section_seen, found) = lookup(&text, wanted_section, wanted_key);
    if let Some(value) = found {
        return Ok(value);
    }
    if !section_seen {
        return Err(format!("ini_get: this ini text has no [{}] section", wanted_section));
    }
    if wanted_section.is_empty() {
        return Err(format!("ini_get: no `{}` key at the top of the file", wanted_key));
    }
    return Err(format!("ini_get: no `{}` key in [{}]", wanted_key, wanted_section));
}

/// Section header names in order of first appearance, without duplicates.
pub fn sections(text: String) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        if let Some(name) = header_name(line) {
            if !out.iter().any(|seen| seen == name) {
                out.push(name.to_string());
            }
        }
    }
    return out;
}

/// The keys of one section in order of first appearance, without duplicates.
/// An empty section name means the top of the file. A missing section is
/// named.
pub fn keys(text: String, section: String) -> Result<Vec<String>, String> {
    let wanted_section = section.trim();
    let mut section_seen = wanted_section.is_empty();
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in text.lines() {
        if let Some(name) = header_name(line) {
            current = name.to_string();
            if current == wanted_section {
                section_seen = true;
            }
            continue;
        }
        if is_comment_or_blank(line) || current != wanted_section {
            continue;
        }
        if let Some((key_name, _)) = key_value(line) {
            if !out.iter().any(|seen| seen == key_name) {
                out.push(key_name.to_string());
            }
        }
    }
    if !section_seen {
        return Err(format!("ini_keys: this ini text has no [{}] section", wanted_section));
    }
    return Ok(out);
}

/// Whether the section holds the key.
pub fn has(text: String, section: String, key: String) -> bool {
    let (_, found) = lookup(&text, section.trim(), key.trim());
    return found.is_some();
}

/// The text with the key set to the value. An existing key is replaced in
/// place, at the occurrence reading would pick, and every other line stays
/// as it was. An absent key is appended to its section and an absent
/// section is created at the end.
pub fn set(text: String, section: String, key: String, value: String) -> String {
    let wanted_section = section.trim();
    let wanted_key = key.trim();
    let mut lines: Vec<String> = text.lines().map(|line| line.to_string()).collect();
    let owner = owners(&lines);
    let new_line = format!("{} = {}", wanted_key, value);

    let mut replace_at = None;
    for (i, line) in lines.iter().enumerate() {
        if header_name(line).is_some() || is_comment_or_blank(line) || owner[i] != wanted_section {
            continue;
        }
        if let Some((key_name, _)) = key_value(line) {
            if key_name == wanted_key {
                replace_at = Some(i);
            }
        }
    }
    if let Some(i) = replace_at {
        lines[i] = new_line;
        return rejoin(lines, &text);
    }

    let section_exists = wanted_section.is_empty() || lines.iter().any(|line| header_name(line) == Some(wanted_section));
    if section_exists {
        let mut insert_after = None;
        for (i, line) in lines.iter().enumerate() {
            if owner[i] == wanted_section && !line.trim().is_empty() {
                insert_after = Some(i);
            }
        }
        match insert_after {
            Some(i) => lines.insert(i + 1, new_line),
            None => lines.insert(0, new_line),
        }
        return rejoin(lines, &text);
    }

    if let Some(last) = lines.last() {
        if !last.trim().is_empty() {
            lines.push(String::new());
        }
    }
    lines.push(format!("[{}]", wanted_section));
    lines.push(new_line);
    return rejoin(lines, &text);
}

/// The text with every line setting that key in that section removed.
/// Removing what is absent returns the text unchanged.
pub fn remove(text: String, section: String, key: String) -> String {
    let wanted_section = section.trim();
    let wanted_key = key.trim();
    let lines: Vec<String> = text.lines().map(|line| line.to_string()).collect();
    let owner = owners(&lines);
    let mut kept: Vec<String> = Vec::new();
    let mut removed_any = false;
    for (i, line) in lines.iter().enumerate() {
        let is_data = header_name(line).is_none() && !is_comment_or_blank(line);
        if is_data && owner[i] == wanted_section {
            if let Some((key_name, _)) = key_value(line) {
                if key_name == wanted_key {
                    removed_any = true;
                    continue;
                }
            }
        }
        kept.push(line.clone());
    }
    if !removed_any {
        return text;
    }
    return rejoin(kept, &text);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        return [
            "; application config",
            "name = \"My App\"  ; the quotes keep the space",
            "debug: true",
            "",
            "[server]",
            "# bound for local use",
            "host = 127.0.0.1  # loopback",
            "port : 8080",
            "port = 9090",
            "",
            "[paths]",
            "data = /var/lib/app",
        ]
        .join("\n");
    }

    #[test]
    fn a_realistic_file_reads_back_cleanly() {
        assert_eq!(get(sample(), "".to_string(), "name".to_string()).unwrap(), "My App");
        assert_eq!(get(sample(), "".to_string(), "debug".to_string()).unwrap(), "true");
        assert_eq!(get(sample(), "server".to_string(), "host".to_string()).unwrap(), "127.0.0.1");
        assert_eq!(get(sample(), "paths".to_string(), "data".to_string()).unwrap(), "/var/lib/app");
        assert!(has(sample(), "server".to_string(), "port".to_string()));
        assert!(!has(sample(), "server".to_string(), "tls".to_string()));
        assert!(!has(sample(), "nowhere".to_string(), "port".to_string()));
    }

    #[test]
    fn later_duplicates_win_and_listings_keep_order() {
        assert_eq!(get(sample(), "server".to_string(), "port".to_string()).unwrap(), "9090");
        assert_eq!(sections(sample()), vec!["server", "paths"]);
        assert_eq!(keys(sample(), "server".to_string()).unwrap(), vec!["host", "port"]);
        assert_eq!(keys(sample(), "".to_string()).unwrap(), vec!["name", "debug"]);
    }

    #[test]
    fn missing_sections_and_keys_are_named() {
        assert!(get(sample(), "missing".to_string(), "x".to_string()).unwrap_err().contains("no [missing] section"));
        assert!(get(sample(), "server".to_string(), "tls".to_string()).unwrap_err().contains("no `tls` key in [server]"));
        assert!(get(sample(), "".to_string(), "tls".to_string()).unwrap_err().contains("top of the file"));
        assert!(keys(sample(), "missing".to_string()).unwrap_err().contains("no [missing] section"));
    }

    #[test]
    fn set_then_get_round_trips_and_comments_survive() {
        let updated = set(sample(), "server".to_string(), "port".to_string(), "1234".to_string());
        assert_eq!(get(updated.clone(), "server".to_string(), "port".to_string()).unwrap(), "1234");
        assert!(updated.contains("# bound for local use"));
        assert!(updated.contains("; application config"));

        let appended = set(sample(), "server".to_string(), "tls".to_string(), "on".to_string());
        assert_eq!(get(appended, "server".to_string(), "tls".to_string()).unwrap(), "on");

        let grown = set(sample(), "logging".to_string(), "level".to_string(), "info".to_string());
        assert_eq!(get(grown.clone(), "logging".to_string(), "level".to_string()).unwrap(), "info");
        assert_eq!(sections(grown), vec!["server", "paths", "logging"]);
    }

    #[test]
    fn one_set_case_is_pinned_exactly() {
        let text = "[a]\nx = 1\n\n[b]\ny = 2".to_string();
        assert_eq!(set(text, "a".to_string(), "z".to_string(), "3".to_string()), "[a]\nx = 1\nz = 3\n\n[b]\ny = 2");
        let short = "[a]\nx = 1".to_string();
        assert_eq!(set(short, "b".to_string(), "y".to_string(), "2".to_string()), "[a]\nx = 1\n\n[b]\ny = 2");
    }

    #[test]
    fn remove_then_has_says_false_and_absent_removal_changes_nothing() {
        let removed = remove(sample(), "server".to_string(), "port".to_string());
        assert!(!has(removed.clone(), "server".to_string(), "port".to_string()));
        assert!(!removed.contains("8080"));
        assert!(!removed.contains("9090"));
        assert_eq!(remove(sample(), "server".to_string(), "nothing".to_string()), sample());
        assert_eq!(remove(sample(), "nowhere".to_string(), "port".to_string()), sample());
    }
}
