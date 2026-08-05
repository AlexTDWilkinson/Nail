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

/// The host a URL points at, with any leading `www.` taken off - the name a
/// program compares or displays. `https://www.example.com/a?b` is, for every
/// practical purpose, a link to `example.com`.
pub fn domain(text: String) -> Result<String, String> {
    let parts = parse(text.clone()).map_err(|_| format!("url_domain: '{}' is not a URL", text))?;
    if parts.host.is_empty() {
        return Err(format!("url_domain: '{}' has no host to take a domain from", text));
    }
    return Ok(parts.host.strip_prefix("www.").unwrap_or(&parts.host).to_string());
}

/// The origin of a URL - scheme://host, with the port when the URL named one.
/// This is the piece browsers compare for CORS and cookies, so two URLs with
/// the same origin are "the same site" in the way that actually matters.
pub fn origin(text: String) -> Result<String, String> {
    let parts = parse(text.clone()).map_err(|_| format!("url_origin: '{}' is not a URL", text))?;
    if parts.host.is_empty() {
        return Err(format!("url_origin: '{}' has no host, so it has no origin", text));
    }
    let mut built = std::format!("{}://", parts.scheme);
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
    return Ok(built);
}

/// Whether the text is an absolute URL - one with a scheme and a host, so it
/// can be fetched on its own. `/about` and `example.com/path` are not.
pub fn is_absolute(text: String) -> bool {
    return match parse(text) {
        Ok(parts) => !parts.host.is_empty(),
        Err(_) => false,
    };
}

/// Whether a query field name is one of the tracking parameters analytics
/// tools staple onto shared links.
fn is_tracking_field(name: &str) -> bool {
    let lowered = name.to_lowercase();
    return lowered.starts_with("utm_") || matches!(lowered.as_str(), "fbclid" | "gclid" | "msclkid" | "mc_eid");
}

/// Removes the tracking parameters - utm_*, fbclid, gclid, msclkid, mc_eid -
/// that analytics tools staple onto shared links, keeping every other query
/// field in its original order. A URL with no query comes back unchanged.
pub fn strip_tracking(text: String) -> Result<String, String> {
    let mut parts = parse(text.clone()).map_err(|_| format!("url_strip_tracking: '{}' is not a URL", text))?;
    if parts.query.is_empty() {
        return Ok(text.trim().to_string());
    }
    let kept: Vec<&str> = parts.query.split('&').filter(|pair| !is_tracking_field(pair.split('=').next().unwrap_or(""))).collect();
    parts.query = kept.join("&");
    return Ok(format(&parts));
}

/// The path of a URL split into its slash-separated segments, each one
/// percent-decoded. The root path `/` is an empty array, and a segment whose
/// encoding is broken is kept as it was rather than guessed at.
pub fn path_segments(text: String) -> Result<Vec<String>, String> {
    let parts = parse(text.clone()).map_err(|_| format!("url_path_segments: '{}' is not a URL", text))?;
    return Ok(parts
        .path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| urlencoding::decode(segment).map(|decoded| decoded.to_string()).unwrap_or_else(|_| segment.to_string()))
        .collect());
}

/// One group of robots.txt rules and the agent names it applies to.
struct RobotsGroup {
    agents: Vec<String>,
    /// Each rule keeps whether it allows and the path pattern it matches.
    rules: Vec<(bool, String)>,
}

/// Whether a robots.txt rule pattern matches a request path. A pattern matches
/// from the start of the path, `*` matches any run of characters, and a
/// trailing `$` requires the match to reach the very end.
fn robots_rule_matches(pattern: &str, path: &str) -> bool {
    let (body, anchored) = match pattern.strip_suffix('$') {
        Some(rest) => (rest, true),
        None => (pattern, false),
    };
    let segments: Vec<&str> = body.split('*').collect();
    if segments.len() == 1 {
        if anchored {
            return path == body;
        }
        return path.starts_with(body);
    }
    if !path.starts_with(segments[0]) {
        return false;
    }
    let mut position = segments[0].len();
    let last = segments.len() - 1;
    for (index, segment) in segments.iter().enumerate().skip(1) {
        // The segment before the anchor must sit at the very end of the path,
        // with the `*` before it absorbing whatever lies between.
        if index == last && anchored {
            return path.len() >= position + segment.len() && path.ends_with(segment);
        }
        match path[position..].find(segment) {
            Some(found) => position += found + segment.len(),
            None => return false,
        }
    }
    return true;
}

/// Whether a robots.txt file lets a user agent fetch a path - the polite
/// scraper's question, asked before every crawl.
///
/// This reads the subset of robots.txt that real files use. `User-agent` lines
/// open groups, agents are matched case-insensitively by substring, the
/// longest matching agent name wins, and the `*` group is the fallback when no
/// name matches. Within the winning group the longest matching rule between
/// `Allow` and `Disallow` decides, with `Allow` winning a tie. In a rule `*`
/// matches any run of characters and a trailing `$` anchors the end. An empty
/// `Disallow` line restricts nothing, a missing file given as an empty string
/// allows everything, and a path no rule speaks to is allowed. `Crawl-delay`,
/// `Sitemap` and the nonstandard extensions are ignored.
pub fn robots_allowed(robots_txt: String, user_agent: String, path: String) -> bool {
    let path = if path.is_empty() { "/".to_string() } else { path };
    let mut groups: Vec<RobotsGroup> = Vec::new();
    // Consecutive User-agent lines share one group, so the group stays open
    // for more names until a rule arrives.
    let mut collecting_agents = false;

    for raw_line in robots_txt.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((directive, value)) = line.split_once(':') else {
            continue;
        };
        let directive = directive.trim().to_lowercase();
        let value = value.trim().to_string();
        match directive.as_str() {
            "user-agent" => {
                if !collecting_agents {
                    groups.push(RobotsGroup { agents: Vec::new(), rules: Vec::new() });
                    collecting_agents = true;
                }
                if let Some(group) = groups.last_mut() {
                    group.agents.push(value.to_lowercase());
                }
            }
            "allow" | "disallow" => {
                collecting_agents = false;
                // A rule before any User-agent line belongs to nobody.
                if let Some(group) = groups.last_mut() {
                    group.rules.push((directive == "allow", value));
                }
            }
            _ => {}
        }
    }

    // The most specific matching agent name wins, with `*` counting as the
    // least specific match of all.
    let agent_lowered = user_agent.to_lowercase();
    let mut best_specificity: i64 = -1;
    for group in &groups {
        for agent in &group.agents {
            let specificity = if agent == "*" {
                0
            } else if !agent.is_empty() && agent_lowered.contains(agent.as_str()) {
                agent.len() as i64
            } else {
                continue;
            };
            if specificity > best_specificity {
                best_specificity = specificity;
            }
        }
    }
    if best_specificity < 0 {
        // No group speaks to this agent at all.
        return true;
    }

    // Every group naming the winning agent contributes its rules, so a file
    // with two groups for the same crawler behaves as one.
    let mut best_allow: i64 = -1;
    let mut best_disallow: i64 = -1;
    for group in &groups {
        let applies = group.agents.iter().any(|agent| {
            if agent == "*" {
                return best_specificity == 0;
            }
            return !agent.is_empty() && agent_lowered.contains(agent.as_str()) && agent.len() as i64 == best_specificity;
        });
        if !applies {
            continue;
        }
        for (allows, pattern) in &group.rules {
            // An empty Disallow allows everything, which as a rule means it
            // restricts nothing, and an empty Allow claims nothing either.
            if pattern.is_empty() {
                continue;
            }
            if !robots_rule_matches(pattern, &path) {
                continue;
            }
            let length = pattern.len() as i64;
            if *allows {
                best_allow = best_allow.max(length);
            } else {
                best_disallow = best_disallow.max(length);
            }
        }
    }
    // The longest match decides and Allow wins a tie. With no match at all
    // both sit at their starting value and the path is allowed.
    return best_allow >= best_disallow;
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

    #[test]
    fn the_domain_is_the_host_without_its_www() {
        assert_eq!(domain("https://www.example.com/a?b".to_string()).expect("a URL"), "example.com");
        assert_eq!(domain("https://blog.example.com/post".to_string()).expect("a URL"), "blog.example.com");
        assert_eq!(domain("HTTPS://WWW.Example.COM".to_string()).expect("a URL"), "example.com");
        assert!(domain("mailto:alex@example.com".to_string()).unwrap_err().contains("no host"));
        assert!(domain("example.com/path".to_string()).unwrap_err().contains("is not a URL"));
    }

    #[test]
    fn the_origin_is_what_cors_compares() {
        assert_eq!(origin("https://example.com/deep/path?q=1#top".to_string()).expect("a URL"), "https://example.com");
        assert_eq!(origin("http://localhost:8080/health".to_string()).expect("a URL"), "http://localhost:8080");
        assert_eq!(origin("http://[::1]:9000/".to_string()).expect("a URL"), "http://[::1]:9000");
        assert!(origin("mailto:alex@example.com".to_string()).unwrap_err().contains("no origin"));
        assert!(origin("/relative/path".to_string()).unwrap_err().contains("is not a URL"));
    }

    #[test]
    fn absolute_means_a_scheme_and_a_host() {
        assert!(is_absolute("https://example.com/path".to_string()));
        assert!(is_absolute("http://localhost:8080".to_string()));
        assert!(!is_absolute("/about".to_string()));
        assert!(!is_absolute("example.com/path".to_string()));
        assert!(!is_absolute("mailto:alex@example.com".to_string()));
        assert!(!is_absolute("".to_string()));
    }

    #[test]
    fn stripping_tracking_keeps_the_honest_parameters_in_order() {
        let cleaned = strip_tracking("https://example.com/a?keep=1&utm_source=news&page=2&fbclid=abc123&sort=asc&utm_campaign=x".to_string()).expect("a URL");
        assert_eq!(cleaned, "https://example.com/a?keep=1&page=2&sort=asc");
        let all_trackers = strip_tracking("https://example.com/a?gclid=1&msclkid=2&mc_eid=3&UTM_Medium=4".to_string()).expect("a URL");
        assert_eq!(all_trackers, "https://example.com/a");
    }

    #[test]
    fn a_url_with_no_query_comes_back_unchanged() {
        assert_eq!(strip_tracking("https://example.com/a/b#top".to_string()).expect("a URL"), "https://example.com/a/b#top");
        assert!(strip_tracking("not a url".to_string()).unwrap_err().contains("is not a URL"));
    }

    #[test]
    fn the_path_comes_apart_into_decoded_segments() {
        assert_eq!(path_segments("https://example.com/blog/2026/post".to_string()).expect("a URL"), vec!["blog", "2026", "post"]);
        assert_eq!(path_segments("https://example.com/a%20b/c%2Fd".to_string()).expect("a URL"), vec!["a b", "c/d"]);
        assert!(path_segments("https://example.com/".to_string()).expect("a URL").is_empty());
        assert!(path_segments("https://example.com".to_string()).expect("a URL").is_empty());
        assert!(path_segments("no scheme here".to_string()).unwrap_err().contains("is not a URL"));
    }
}

#[cfg(test)]
mod robots_tests {
    use super::robots_allowed;

    fn allowed(robots: &str, agent: &str, path: &str) -> bool {
        return robots_allowed(robots.to_string(), agent.to_string(), path.to_string());
    }

    #[test]
    fn a_missing_file_allows_everything() {
        assert!(allowed("", "NailBot", "/anything"));
        assert!(allowed("   \n  ", "NailBot", "/anything"));
    }

    #[test]
    fn the_wildcard_group_is_the_fallback() {
        let robots = "User-agent: *\nDisallow: /private/";
        assert!(!allowed(robots, "NailBot", "/private/page"));
        assert!(allowed(robots, "NailBot", "/public/page"));
    }

    #[test]
    fn a_specific_agent_group_beats_the_wildcard() {
        let robots = "User-agent: *\nDisallow: /\n\nUser-agent: NailBot\nDisallow: /private/";
        // The wildcard bans everything, but NailBot has its own gentler group.
        assert!(allowed(robots, "NailBot/1.0", "/public/page"));
        assert!(!allowed(robots, "NailBot/1.0", "/private/page"));
        assert!(!allowed(robots, "OtherBot", "/public/page"));
    }

    #[test]
    fn the_most_specific_agent_name_wins_and_matching_is_substring() {
        let robots = "User-agent: Nail\nDisallow: /a/\n\nUser-agent: NailBot\nDisallow: /b/";
        // Both names are substrings of the agent, and the longer one wins.
        assert!(allowed(robots, "Mozilla/5.0 NailBot/1.0", "/a/page"));
        assert!(!allowed(robots, "Mozilla/5.0 NailBot/1.0", "/b/page"));
        // Case does not matter on either side.
        assert!(!allowed(robots, "mozilla nailbot", "/b/page"));
    }

    #[test]
    fn the_longest_rule_wins_between_allow_and_disallow() {
        let robots = "User-agent: *\nDisallow: /shop/\nAllow: /shop/catalogue/";
        assert!(!allowed(robots, "NailBot", "/shop/basket"));
        assert!(allowed(robots, "NailBot", "/shop/catalogue/hammers"));
    }

    #[test]
    fn allow_wins_a_tie_of_equal_length() {
        let robots = "User-agent: *\nDisallow: /page\nAllow: /page";
        assert!(allowed(robots, "NailBot", "/page"));
    }

    #[test]
    fn a_dollar_anchors_the_end_of_the_path() {
        let robots = "User-agent: *\nDisallow: /*.pdf$";
        assert!(!allowed(robots, "NailBot", "/report.pdf"));
        assert!(!allowed(robots, "NailBot", "/deep/nested/report.pdf"));
        assert!(allowed(robots, "NailBot", "/report.pdf.html"));

        let exact = "User-agent: *\nDisallow: /private$";
        assert!(!allowed(exact, "NailBot", "/private"));
        assert!(allowed(exact, "NailBot", "/private/page"));
    }

    #[test]
    fn a_star_in_a_rule_matches_any_run() {
        let robots = "User-agent: *\nDisallow: /search*results";
        assert!(!allowed(robots, "NailBot", "/searchresults"));
        assert!(!allowed(robots, "NailBot", "/search/all/results/page"));
        assert!(allowed(robots, "NailBot", "/search/all"));
    }

    #[test]
    fn an_empty_disallow_restricts_nothing() {
        let robots = "User-agent: *\nDisallow:";
        assert!(allowed(robots, "NailBot", "/anything/at/all"));
    }

    #[test]
    fn comments_and_unknown_directives_are_ignored() {
        let robots = "# a note\nUser-agent: * # everyone\nCrawl-delay: 10\nSitemap: https://example.com/map.xml\nDisallow: /private/ # keep out";
        assert!(!allowed(robots, "NailBot", "/private/page"));
        assert!(allowed(robots, "NailBot", "/public"));
    }

    #[test]
    fn shared_user_agent_lines_share_one_group() {
        let robots = "User-agent: AlphaBot\nUser-agent: BetaBot\nDisallow: /private/";
        assert!(!allowed(robots, "AlphaBot", "/private/page"));
        assert!(!allowed(robots, "BetaBot", "/private/page"));
        // No wildcard group, so an unnamed agent is free.
        assert!(allowed(robots, "GammaBot", "/private/page"));
    }
}
