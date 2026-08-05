use dashmap::DashMap;

/// URL encode a string
pub fn encode(text: String) -> String {
    urlencoding::encode(&text).to_string()
}

/// URL decode a string
pub fn decode(text: String) -> Result<String, String> {
    urlencoding::decode(&text)
        .map(|s| s.to_string())
        .map_err(|e| format!("url_decode: could not decode '{}': {}", text, e))
}

/// Percent-decoding for one query or form field. Browsers encode a space as
/// '+' in both query strings and form bodies, and percent-decoding alone does
/// not undo that, so it is translated first.
fn decode_field(field: &str) -> String {
    let plus_decoded = field.replace('+', " ");
    urlencoding::decode(&plus_decoded).map(|decoded| decoded.to_string()).unwrap_or(plus_decoded)
}

/// Parse a query string into a hashmap. A POST form body uses this same
/// encoding, so it parses submitted forms too.
pub fn parse_query(query: String) -> DashMap<String, String> {
    let map = DashMap::new();

    // Remove leading ? if present
    let query = query.trim_start_matches('?');

    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }

        let parts: Vec<&str> = pair.splitn(2, '=').collect();
        let key = parts[0];
        let value = parts.get(1).unwrap_or(&"");

        map.insert(decode_field(key), decode_field(value));
    }

    map
}

/// Build a query string from a hashmap
pub fn build_query(params: &DashMap<String, String>) -> String {
    let mut parts = Vec::new();
    
    for entry in params.iter() {
        let key = urlencoding::encode(entry.key());
        let value = urlencoding::encode(entry.value());
        parts.push(format!("{}={}", key, value));
    }
    
    parts.join("&")
}
/// A URL taken apart. `port` is 0 when the URL did not name one, since the
/// default depends on the scheme; the string fields are empty when that piece
/// was absent, so a program can put them back together without deciding what a
/// missing piece meant.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct URL_Parts {
    /// `https`, `http`, `postgres` - always lower case, never with the colon.
    pub scheme: String,
    /// Anything before the `@` in the authority, credentials included. Rare,
    /// and worth noticing when it is there.
    pub user: String,
    /// The host, with the brackets taken off an IPv6 address.
    pub host: String,
    /// The port the URL named, or 0 when it left the scheme's default.
    pub port: i64,
    /// The path, starting with `/`. Empty only when the URL had none.
    pub path: String,
    /// The query without its `?`. Hand it to url_parse_query for the fields.
    pub query: String,
    /// The fragment without its `#`, which never leaves the browser.
    pub fragment: String,
}

/// Takes a URL apart into its pieces. Errors when there is no scheme, since
/// `example.com/path` could as easily be a path as a URL and guessing is how
/// requests end up somewhere nobody meant.
pub fn parse(text: String) -> Result<URL_Parts, String> {
    let trimmed = text.trim();
    let scheme_end = match trimmed.find(':') {
        Some(position) if position > 0 => position,
        _ => return Err(format!("url_parse: '{}' has no scheme, so it is not a URL", text)),
    };

    let scheme = trimmed[..scheme_end].to_lowercase();
    if !scheme.starts_with(|character: char| character.is_ascii_alphabetic()) || !scheme.chars().all(|character| character.is_ascii_alphanumeric() || character == '+' || character == '-' || character == '.') {
        return Err(format!("url_parse: '{}' does not start with a scheme", text));
    }

    let mut rest = &trimmed[scheme_end + 1..];
    let mut user = String::new();
    let mut host = String::new();
    let mut port = 0i64;

    // `//` introduces an authority; without it everything after the colon is
    // the path, which is how `mailto:` and `urn:` are shaped.
    if let Some(after_slashes) = rest.strip_prefix("//") {
        let authority_end = after_slashes.find(['/', '?', '#']).unwrap_or(after_slashes.len());
        let authority = &after_slashes[..authority_end];
        rest = &after_slashes[authority_end..];

        let host_port = match authority.rfind('@') {
            Some(position) => {
                user = authority[..position].to_string();
                &authority[position + 1..]
            }
            None => authority,
        };

        // An IPv6 address is bracketed precisely so its colons are not ports.
        let (host_text, port_text) = if let Some(closing) = host_port.rfind(']') {
            (&host_port[..closing + 1], host_port[closing + 1..].strip_prefix(':'))
        } else {
            match host_port.rfind(':') {
                Some(position) => (&host_port[..position], Some(&host_port[position + 1..])),
                None => (host_port, None),
            }
        };

        host = host_text.trim_start_matches('[').trim_end_matches(']').to_lowercase();
        if let Some(number) = port_text {
            if !number.is_empty() {
                port = number.parse::<i64>().map_err(|_| format!("url_parse: '{}' is not a port number in '{}'", number, text))?;
                if !(1..=65535).contains(&port) {
                    return Err(format!("url_parse: port {} in '{}' is outside 1 to 65535", port, text));
                }
            }
        }
        if host.is_empty() {
            return Err(format!("url_parse: '{}' has no host", text));
        }
    }

    let (before_fragment, fragment) = match rest.split_once('#') {
        Some((before, after)) => (before, after.to_string()),
        None => (rest, String::new()),
    };
    let (path, query) = match before_fragment.split_once('?') {
        Some((before, after)) => (before.to_string(), after.to_string()),
        None => (before_fragment.to_string(), String::new()),
    };

    return Ok(URL_Parts { scheme, user, host, port, path, query, fragment });
}

/// Puts a URL back together from its pieces - the other direction of
/// url_parse, so a program can change one piece and keep the rest.
pub fn format(parts: &URL_Parts) -> String {
    let mut built = String::new();
    built.push_str(&parts.scheme);
    built.push(':');

    if !parts.host.is_empty() {
        built.push_str("//");
        if !parts.user.is_empty() {
            built.push_str(&parts.user);
            built.push('@');
        }
        // A host with colons in it is IPv6 and goes back in its brackets.
        if parts.host.contains(':') {
            built.push('[');
            built.push_str(&parts.host);
            built.push(']');
        } else {
            built.push_str(&parts.host);
        }
        if parts.port != 0 {
            built.push_str(&std::format!(":{}", parts.port));
        }
    }

    built.push_str(&parts.path);
    if !parts.query.is_empty() {
        built.push('?');
        built.push_str(&parts.query);
    }
    if !parts.fragment.is_empty() {
        built.push('#');
        built.push_str(&parts.fragment);
    }
    return built;
}

/// Resolves a link found on a page against the page's own URL, the way a
/// browser does: `/about`, `../two`, `?page=2`, `#top` and a whole URL all
/// come out as the address to actually fetch. Any crawler, feed reader or
/// scraper needs exactly this and gets it wrong by hand.
pub fn join(base: String, reference: String) -> Result<String, String> {
    let target = reference.trim();
    // A reference with its own scheme replaces the base outright.
    if parse(target.to_string()).is_ok() {
        return Ok(target.to_string());
    }

    let mut resolved = parse(base.clone()).map_err(|_| format!("url_join: the base '{}' is not a URL", base))?;

    if target.is_empty() {
        resolved.fragment = String::new();
        return Ok(format(&resolved));
    }
    if let Some(fragment) = target.strip_prefix('#') {
        resolved.fragment = fragment.to_string();
        return Ok(format(&resolved));
    }
    if let Some(query) = target.strip_prefix('?') {
        let (query, fragment) = split_fragment(query);
        resolved.query = query;
        resolved.fragment = fragment;
        return Ok(format(&resolved));
    }
    // `//host/path` keeps only the scheme.
    if target.starts_with("//") {
        let borrowed = parse(std::format!("{}:{}", resolved.scheme, target))?;
        return Ok(format(&borrowed));
    }

    let (path_and_query, fragment) = split_fragment(target);
    let (path, query) = match path_and_query.split_once('?') {
        Some((path, query)) => (path.to_string(), query.to_string()),
        None => (path_and_query, String::new()),
    };

    let merged = if path.starts_with('/') {
        path
    } else {
        // Everything up to and including the base's last slash is the
        // directory the relative path is relative to.
        let directory = match resolved.path.rfind('/') {
            Some(position) => resolved.path[..position + 1].to_string(),
            None => "/".to_string(),
        };
        std::format!("{}{}", directory, path)
    };

    resolved.path = remove_dot_segments(&merged);
    resolved.query = query;
    resolved.fragment = fragment;
    return Ok(format(&resolved));
}

/// Splits a reference at its `#`, since the fragment goes along untouched.
fn split_fragment(text: &str) -> (String, String) {
    return match text.split_once('#') {
        Some((before, after)) => (before.to_string(), after.to_string()),
        None => (text.to_string(), String::new()),
    };
}

/// Resolves `.` and `..` in a URL path, keeping a trailing slash where the
/// path had one - `/a/b/../` is `/a/`, not `/a`.
fn remove_dot_segments(path: &str) -> String {
    let from_the_root = path.starts_with('/');
    let ends_in_a_directory = path.ends_with('/') || path.ends_with("/.") || path.ends_with("/..") || path == "." || path == "..";
    let mut kept: Vec<&str> = Vec::new();

    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if kept.pop().is_none() && !from_the_root {
                    // A relative path can still climb above where it started.
                    kept.push("..");
                }
            }
            named => kept.push(named),
        }
    }

    let mut built = kept.join("/");
    if from_the_root {
        built.insert(0, '/');
    }
    if ends_in_a_directory && !built.ends_with('/') {
        built.push('/');
    }
    return built;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_url_comes_apart_into_its_pieces() {
        let parts = parse("https://example.com/blog/post?page=2#top".to_string()).expect("a URL");
        assert_eq!(parts.scheme, "https");
        assert_eq!(parts.host, "example.com");
        assert_eq!(parts.port, 0);
        assert_eq!(parts.path, "/blog/post");
        assert_eq!(parts.query, "page=2");
        assert_eq!(parts.fragment, "top");
        assert_eq!(parts.user, "");
    }

    #[test]
    fn a_port_credentials_and_an_ipv6_host_are_recognised() {
        let parts = parse("http://admin:secret@localhost:8080/health".to_string()).expect("a URL");
        assert_eq!(parts.user, "admin:secret");
        assert_eq!(parts.host, "localhost");
        assert_eq!(parts.port, 8080);

        let bracketed = parse("http://[::1]:9000/".to_string()).expect("a URL");
        assert_eq!(bracketed.host, "::1");
        assert_eq!(bracketed.port, 9000);
        assert_eq!(format(&bracketed), "http://[::1]:9000/");
    }

    #[test]
    fn the_scheme_and_host_are_lower_cased_but_the_path_is_not() {
        let parts = parse("HTTPS://Example.COM/Path/To".to_string()).expect("a URL");
        assert_eq!(parts.scheme, "https");
        assert_eq!(parts.host, "example.com");
        assert_eq!(parts.path, "/Path/To");
    }

    #[test]
    fn a_url_without_an_authority_keeps_everything_as_a_path() {
        let parts = parse("mailto:alex@example.com".to_string()).expect("a URL");
        assert_eq!(parts.scheme, "mailto");
        assert_eq!(parts.host, "");
        assert_eq!(parts.path, "alex@example.com");
    }

    #[test]
    fn something_that_is_not_a_url_is_an_error() {
        assert!(parse("example.com/path".to_string()).unwrap_err().contains("no scheme"));
        assert!(parse("https://".to_string()).unwrap_err().contains("no host"));
        assert!(parse("http://host:notaport/".to_string()).unwrap_err().contains("not a port number"));
        assert!(parse("http://host:99999/".to_string()).unwrap_err().contains("outside 1 to 65535"));
    }

    #[test]
    fn taking_a_url_apart_and_putting_it_back_gives_the_same_url() {
        for original in [
            "https://example.com/blog/post?page=2#top",
            "http://admin@localhost:8080/health",
            "https://example.com/",
            "mailto:alex@example.com",
        ] {
            let parts = parse(original.to_string()).expect("a URL");
            assert_eq!(format(&parts), original);
        }
    }

    fn joined(base: &str, reference: &str) -> String {
        return join(base.to_string(), reference.to_string()).expect("a resolvable link");
    }

    #[test]
    fn a_link_resolves_the_way_a_browser_resolves_it() {
        let base = "https://example.com/blog/2026/post.html?page=2#here";
        assert_eq!(joined(base, "https://other.org/x"), "https://other.org/x");
        assert_eq!(joined(base, "/about"), "https://example.com/about");
        assert_eq!(joined(base, "next.html"), "https://example.com/blog/2026/next.html");
        assert_eq!(joined(base, "../index.html"), "https://example.com/blog/index.html");
        assert_eq!(joined(base, "../../"), "https://example.com/");
        assert_eq!(joined(base, "?page=3"), "https://example.com/blog/2026/post.html?page=3");
        assert_eq!(joined(base, "#top"), "https://example.com/blog/2026/post.html?page=2#top");
        assert_eq!(joined(base, "//cdn.example.com/logo.png"), "https://cdn.example.com/logo.png");
        assert_eq!(joined(base, ""), "https://example.com/blog/2026/post.html?page=2");
    }

    #[test]
    fn climbing_past_the_root_stops_at_the_root() {
        assert_eq!(joined("https://example.com/a/b", "../../../c"), "https://example.com/c");
    }

    #[test]
    fn a_base_that_is_not_a_url_is_an_error() {
        assert!(join("example.com".to_string(), "/about".to_string()).unwrap_err().contains("is not a URL"));
    }

    #[test]
    fn a_trailing_slash_survives_resolution() {
        assert_eq!(joined("https://example.com/a/b/c", "../"), "https://example.com/a/");
        assert_eq!(joined("https://example.com/a/b/c", "./d/"), "https://example.com/a/b/d/");
    }
}
