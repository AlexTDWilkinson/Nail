//! Feeds: what a site publishes when it wants to be followed.
//!
//! RSS and Atom are different formats with the same meaning, and nobody should
//! care which one a site chose. `feed_parse` reads either and hands back the
//! same shape: the feed's own name, and its entries newest-first as the feed
//! ordered them. Absent values are empty strings and zero timestamps, because
//! feeds omit whatever they feel like and Nail has no null to represent that
//! with.
//!
//! Fetch the text with `http_request`, parse it here, and store what is new -
//! that is the whole of a feed reader.

use serde::{Deserialize, Serialize};

/// One item of a feed: a post, an episode, a release.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FEED_Entry {
    /// The entry's own identifier, for telling what has been seen before.
    pub id: String,
    pub title: String,
    pub link: String,
    /// A summary or the whole body, whichever the feed carries; may hold HTML.
    pub summary: String,
    /// When it was published, as a Unix timestamp; 0 when the feed does not say.
    pub published: i64,
}

/// A feed and its entries, in the order the feed gave them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FEED_Feed {
    pub title: String,
    pub link: String,
    pub description: String,
    pub entries: Vec<FEED_Entry>,
}

fn first_link(links: &[feed_rs::model::Link]) -> String {
    return links.first().map(|link| link.href.clone()).unwrap_or_default();
}

/// Reads an RSS or Atom document into one shape, whichever it is.
pub fn parse(text: String) -> Result<FEED_Feed, String> {
    let parsed = feed_rs::parser::parse(text.as_bytes()).map_err(|failure| format!("feed_parse: this is not an RSS or Atom feed: {}", failure))?;

    let entries = parsed
        .entries
        .into_iter()
        .map(|entry| FEED_Entry {
            id: entry.id,
            title: entry.title.map(|title| title.content).unwrap_or_default(),
            link: first_link(&entry.links),
            summary: entry.summary.map(|summary| summary.content).or_else(|| entry.content.and_then(|content| content.body)).unwrap_or_default(),
            published: entry.published.or(entry.updated).map(|when| when.timestamp()).unwrap_or(0),
        })
        .collect();

    return Ok(FEED_Feed {
        title: parsed.title.map(|title| title.content).unwrap_or_default(),
        link: first_link(&parsed.links),
        description: parsed.description.map(|description| description.content).unwrap_or_default(),
        entries,
    });
}

/// The five characters XML reserves, made safe for element text and for
/// attribute values alike.
fn xml_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            other => escaped.push(other),
        }
    }
    return escaped;
}

/// A timestamp as a calendar moment, or an error naming whose timestamp was
/// beyond what a date can hold.
fn moment(timestamp: i64, owner: &str, caller: &str) -> Result<chrono::DateTime<chrono::Utc>, String> {
    return chrono::DateTime::from_timestamp(timestamp, 0).ok_or_else(|| format!("{}: '{}' carries a timestamp {} too far from 1970 to be a date", caller, owner, timestamp));
}

/// A feed without a title or a link would be a document nothing can follow,
/// so building one is refused up front.
fn feed_must_be_publishable(feed: &FEED_Feed, caller: &str) -> Result<(), String> {
    if feed.title.is_empty() {
        return Err(format!("{}: a feed needs a title before it can be published", caller));
    }
    if feed.link.is_empty() {
        return Err(format!("{}: a feed needs a link before it can be published", caller));
    }
    return Ok(());
}

/// Builds an RSS 2.0 document from the same shapes `feed_parse` reads, so what
/// one program publishes another parses straight back. Every value is
/// XML-escaped, entry dates are written the RFC 2822 way RSS wants, and a zero
/// timestamp leaves the date off just as parsing an absent date gives zero.
pub fn build_rss(feed: FEED_Feed, entries: &Vec<FEED_Entry>) -> Result<String, String> {
    feed_must_be_publishable(&feed, "feed_rss")?;

    let mut document = String::new();
    document.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    document.push_str("<rss version=\"2.0\"><channel>\n");
    document.push_str(&format!("<title>{}</title>\n", xml_escape(&feed.title)));
    document.push_str(&format!("<link>{}</link>\n", xml_escape(&feed.link)));
    document.push_str(&format!("<description>{}</description>\n", xml_escape(&feed.description)));

    for entry in entries {
        document.push_str("<item>\n");
        if !entry.id.is_empty() {
            document.push_str(&format!("<guid isPermaLink=\"false\">{}</guid>\n", xml_escape(&entry.id)));
        }
        document.push_str(&format!("<title>{}</title>\n", xml_escape(&entry.title)));
        if !entry.link.is_empty() {
            document.push_str(&format!("<link>{}</link>\n", xml_escape(&entry.link)));
        }
        if !entry.summary.is_empty() {
            document.push_str(&format!("<description>{}</description>\n", xml_escape(&entry.summary)));
        }
        if entry.published != 0 {
            document.push_str(&format!("<pubDate>{}</pubDate>\n", moment(entry.published, &entry.title, "feed_rss")?.to_rfc2822()));
        }
        document.push_str("</item>\n");
    }

    document.push_str("</channel></rss>\n");
    return Ok(document);
}

/// Builds an Atom 1.0 document from the same shapes `feed_parse` reads - the
/// twin of `feed_rss` for the other format. Dates are RFC 3339, each entry is
/// identified by its id or by its link when it has none, and the feed's own
/// required date is the newest entry's.
pub fn build_atom(feed: FEED_Feed, entries: &Vec<FEED_Entry>) -> Result<String, String> {
    feed_must_be_publishable(&feed, "feed_atom")?;
    let atom_date = |timestamp: i64, owner: &str| -> Result<String, String> {
        return Ok(moment(timestamp, owner, "feed_atom")?.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    };
    let newest = entries.iter().map(|entry| entry.published).max().unwrap_or(0);

    let mut document = String::new();
    document.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    document.push_str("<feed xmlns=\"http://www.w3.org/2005/Atom\">\n");
    document.push_str(&format!("<title>{}</title>\n", xml_escape(&feed.title)));
    document.push_str(&format!("<link href=\"{}\"/>\n", xml_escape(&feed.link)));
    document.push_str(&format!("<id>{}</id>\n", xml_escape(&feed.link)));
    if !feed.description.is_empty() {
        document.push_str(&format!("<subtitle>{}</subtitle>\n", xml_escape(&feed.description)));
    }
    document.push_str(&format!("<updated>{}</updated>\n", atom_date(newest, &feed.title)?));

    for entry in entries {
        let id = if entry.id.is_empty() { &entry.link } else { &entry.id };
        if id.is_empty() {
            return Err(format!("feed_atom: entry '{}' has neither an id nor a link to be identified by", entry.title));
        }
        document.push_str("<entry>\n");
        document.push_str(&format!("<id>{}</id>\n", xml_escape(id)));
        document.push_str(&format!("<title>{}</title>\n", xml_escape(&entry.title)));
        if !entry.link.is_empty() {
            document.push_str(&format!("<link href=\"{}\"/>\n", xml_escape(&entry.link)));
        }
        if !entry.summary.is_empty() {
            document.push_str(&format!("<summary>{}</summary>\n", xml_escape(&entry.summary)));
        }
        document.push_str(&format!("<updated>{}</updated>\n", atom_date(entry.published, &entry.title)?));
        document.push_str("</entry>\n");
    }

    document.push_str("</feed>\n");
    return Ok(document);
}

#[cfg(test)]
mod tests {
    use super::*;

    const RSS: &str = r#"<?xml version="1.0"?>
<rss version="2.0"><channel>
  <title>Nail Blog</title>
  <link>https://nail-lang.org/blog</link>
  <description>What changed and why</description>
  <item>
    <guid>post-2</guid>
    <title>Second post</title>
    <link>https://nail-lang.org/blog/2</link>
    <description>The newer one</description>
    <pubDate>Tue, 04 Aug 2026 12:00:00 GMT</pubDate>
  </item>
  <item>
    <guid>post-1</guid>
    <title>First post</title>
    <link>https://nail-lang.org/blog/1</link>
    <description>The older one</description>
    <pubDate>Mon, 03 Aug 2026 12:00:00 GMT</pubDate>
  </item>
</channel></rss>"#;

    const ATOM: &str = r#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Nail Releases</title>
  <link href="https://nail-lang.org/releases"/>
  <id>urn:nail:releases</id>
  <updated>2026-08-04T12:00:00Z</updated>
  <entry>
    <id>release-1.2.0</id>
    <title>1.2.0</title>
    <link href="https://nail-lang.org/releases/1.2.0"/>
    <summary>Streaming files</summary>
    <updated>2026-08-04T12:00:00Z</updated>
  </entry>
</feed>"#;

    #[test]
    fn an_rss_feed_reads_into_the_shape() {
        let feed = parse(RSS.to_string()).expect("a valid feed");
        assert_eq!(feed.title, "Nail Blog");
        assert_eq!(feed.description, "What changed and why");
        assert_eq!(feed.entries.len(), 2);
        assert_eq!(feed.entries[0].title, "Second post");
        assert_eq!(feed.entries[0].link, "https://nail-lang.org/blog/2");
        assert!(feed.entries[0].published > feed.entries[1].published, "the newer entry carries the later time");
    }

    #[test]
    fn an_atom_feed_reads_into_the_same_shape() {
        let feed = parse(ATOM.to_string()).expect("a valid feed");
        assert_eq!(feed.title, "Nail Releases");
        assert_eq!(feed.entries.len(), 1);
        assert_eq!(feed.entries[0].id, "release-1.2.0");
        assert_eq!(feed.entries[0].summary, "Streaming files");
        assert!(feed.entries[0].published > 0);
    }

    #[test]
    fn what_a_feed_omits_is_empty_rather_than_missing() {
        let sparse = r#"<rss version="2.0"><channel><title>Sparse</title><item><guid>only</guid></item></channel></rss>"#;
        let feed = parse(sparse.to_string()).expect("a valid feed");
        assert_eq!(feed.entries[0].title, "");
        assert_eq!(feed.entries[0].published, 0);
    }

    #[test]
    fn text_that_is_not_a_feed_says_so() {
        let failure = parse("<html><body>a page</body></html>".to_string()).unwrap_err();
        assert!(failure.contains("not an RSS or Atom feed"), "got: {}", failure);
    }

    /// The blog a website would publish: two entries, every field filled in.
    fn a_blog() -> (FEED_Feed, Vec<FEED_Entry>) {
        let feed = FEED_Feed { title: "Nail Blog".to_string(), link: "https://nail-lang.org/blog".to_string(), description: "What changed and why".to_string(), entries: Vec::new() };
        let entries = vec![
            FEED_Entry {
                id: "post-2".to_string(),
                title: "Second post".to_string(),
                link: "https://nail-lang.org/blog/2".to_string(),
                summary: "The newer one".to_string(),
                published: 1_786_190_400,
            },
            FEED_Entry {
                id: "post-1".to_string(),
                title: "First post".to_string(),
                link: "https://nail-lang.org/blog/1".to_string(),
                summary: "The older one".to_string(),
                published: 1_786_104_000,
            },
        ];
        return (feed, entries);
    }

    fn entries_survive(parsed: &FEED_Feed, entries: &[FEED_Entry]) {
        assert_eq!(parsed.entries.len(), entries.len());
        for (round_tripped, original) in parsed.entries.iter().zip(entries.iter()) {
            assert_eq!(round_tripped.id, original.id);
            assert_eq!(round_tripped.title, original.title);
            assert_eq!(round_tripped.link, original.link);
            assert_eq!(round_tripped.summary, original.summary);
            assert_eq!(round_tripped.published, original.published);
        }
    }

    /// The whole point of the builders: what one Nail program publishes,
    /// another reads back as the same values.
    #[test]
    fn an_rss_document_built_here_parses_back_to_the_same_values() {
        let (feed, entries) = a_blog();
        let document = build_rss(feed.clone(), &entries).expect("a publishable feed");
        let parsed = parse(document).expect("what was built must parse");
        assert_eq!(parsed.title, feed.title);
        assert_eq!(parsed.link, feed.link);
        assert_eq!(parsed.description, feed.description);
        entries_survive(&parsed, &entries);
    }

    #[test]
    fn an_atom_document_built_here_parses_back_to_the_same_values() {
        let (feed, entries) = a_blog();
        let document = build_atom(feed.clone(), &entries).expect("a publishable feed");
        let parsed = parse(document).expect("what was built must parse");
        assert_eq!(parsed.title, feed.title);
        assert_eq!(parsed.link, feed.link);
        assert_eq!(parsed.description, feed.description);
        entries_survive(&parsed, &entries);
    }

    /// A title is text, however much it looks like markup, and must come back
    /// as the same text rather than breaking the document around it.
    #[test]
    fn reserved_characters_in_a_title_survive_both_round_trips() {
        let hostile = r#"Ampersands & <b>tags</b> and "quotes""#;
        let (mut feed, mut entries) = a_blog();
        feed.title = hostile.to_string();
        entries[0].title = hostile.to_string();
        entries[0].summary = hostile.to_string();

        let rss = parse(build_rss(feed.clone(), &entries).expect("a publishable feed")).expect("escaped RSS must parse");
        assert_eq!(rss.title, hostile);
        assert_eq!(rss.entries[0].title, hostile);
        assert_eq!(rss.entries[0].summary, hostile);

        let atom = parse(build_atom(feed.clone(), &entries).expect("a publishable feed")).expect("escaped Atom must parse");
        assert_eq!(atom.title, hostile);
        assert_eq!(atom.entries[0].title, hostile);
    }

    #[test]
    fn every_reserved_character_is_escaped() {
        assert_eq!(xml_escape(r#"<b>&"'"#), "&lt;b&gt;&amp;&quot;&apos;");
        assert_eq!(xml_escape("plain words"), "plain words");
    }

    #[test]
    fn an_entry_without_an_id_falls_back_to_its_link_in_atom() {
        let (feed, mut entries) = a_blog();
        entries[0].id = String::new();
        let parsed = parse(build_atom(feed, &entries).expect("a publishable feed")).expect("what was built must parse");
        assert_eq!(parsed.entries[0].id, entries[0].link);
    }

    #[test]
    fn an_atom_entry_with_nothing_to_identify_it_by_is_refused() {
        let (feed, mut entries) = a_blog();
        entries[0].id = String::new();
        entries[0].link = String::new();
        let failure = build_atom(feed, &entries).unwrap_err();
        assert!(failure.contains("neither an id nor a link"), "got: {}", failure);
    }

    #[test]
    fn a_feed_missing_its_title_or_link_is_refused_by_both_builders() {
        let (feed, entries) = a_blog();

        let mut untitled = feed.clone();
        untitled.title = String::new();
        assert!(build_rss(untitled.clone(), &entries).unwrap_err().contains("needs a title"));
        assert!(build_atom(untitled, &entries).unwrap_err().contains("needs a title"));

        let mut unlinked = feed;
        unlinked.link = String::new();
        assert!(build_rss(unlinked.clone(), &entries).unwrap_err().contains("needs a link"));
        assert!(build_atom(unlinked, &entries).unwrap_err().contains("needs a link"));
    }

    /// Zero means the feed did not say, so building writes no date and parsing
    /// finds none - the same zero comes back.
    #[test]
    fn a_zero_timestamp_round_trips_as_zero_through_rss() {
        let (feed, mut entries) = a_blog();
        entries[0].published = 0;
        let parsed = parse(build_rss(feed, &entries).expect("a publishable feed")).expect("what was built must parse");
        assert_eq!(parsed.entries[0].published, 0);
        assert_eq!(parsed.entries[1].published, entries[1].published);
    }
}
