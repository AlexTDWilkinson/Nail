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
}
