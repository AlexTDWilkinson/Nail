//! Reading HTML that somebody else wrote.
//!
//! `markdown_to_html` and `template_render` produce HTML; this reads it - a
//! page fetched with `http_request`, a feed, a saved export. Real HTML on the
//! internet is not well-formed XML and never will be, so it is parsed the way
//! a browser parses it: unclosed tags, stray text and mismatched nesting are
//! all recovered from rather than refused.
//!
//! Elements are found by CSS selector, because that is the query language
//! everybody already knows from stylesheets and browser dev tools - the same
//! `div.post > a` that highlights the links in an inspector picks them out
//! here.
//!
//! Nothing here writes HTML. Building markup by hand is what `template_render`
//! is for, and building it by string surgery on parsed input is how a program
//! ends up serving whatever was in the input.

use scraper::{Html, Selector};

/// Parses a selector, saying what is wrong with it rather than what the
/// selector library's own error type says, which mentions parser internals.
fn parse_selector(selector: &str, function_name: &str) -> Result<Selector, String> {
    return Selector::parse(selector).map_err(|_| format!("{}: `{}` is not a CSS selector", function_name, selector));
}

/// Collapses the whitespace HTML treats as insignificant, so text pulled out of
/// markup reads as a line rather than as the indentation it was written with.
fn tidy(text: String) -> String {
    return text.split_whitespace().collect::<Vec<&str>>().join(" ");
}

/// All the text of a document, with the tags removed and whitespace collapsed.
/// What a page says, for searching, counting words or storing alongside the
/// markup.
pub fn text(html: String) -> String {
    let document = Html::parse_document(&html);
    return tidy(document.root_element().text().collect::<Vec<&str>>().join(" "));
}

/// The text of every element matching the selector, one entry each. No matches
/// gives an empty array rather than an error - a page not having any comments is
/// an ordinary state of a page.
pub fn select_text(html: String, selector: String) -> Result<Vec<String>, String> {
    let chosen = parse_selector(&selector, "html_select_text")?;
    let document = Html::parse_document(&html);
    return Ok(document.select(&chosen).map(|element| tidy(element.text().collect::<Vec<&str>>().join(" "))).collect());
}

/// The markup inside every element matching the selector. For handing a fragment
/// on to something that renders HTML - never for pulling apart with string
/// operations, which is what selectors are for.
pub fn select_html(html: String, selector: String) -> Result<Vec<String>, String> {
    let chosen = parse_selector(&selector, "html_select_html")?;
    let document = Html::parse_document(&html);
    return Ok(document.select(&chosen).map(|element| element.inner_html()).collect());
}

/// One attribute of every element matching the selector. Elements that match but
/// do not carry the attribute are skipped, so the result holds only values that
/// were actually there.
pub fn select_attribute(html: String, selector: String, attribute: String) -> Result<Vec<String>, String> {
    let chosen = parse_selector(&selector, "html_select_attribute")?;
    let document = Html::parse_document(&html);
    return Ok(document.select(&chosen).filter_map(|element| element.value().attr(&attribute).map(|value| value.to_string())).collect());
}

/// How many elements match the selector, without pulling anything out of them.
pub fn count(html: String, selector: String) -> Result<i64, String> {
    let chosen = parse_selector(&selector, "html_count")?;
    let document = Html::parse_document(&html);
    return Ok(document.select(&chosen).count() as i64);
}

/// Every address an `<a href>` on the page points at, in document order. The
/// values are exactly as written, so a relative link comes back relative - what
/// it is relative to is the address the page came from, which this does not know.
pub fn links(html: String) -> Result<Vec<String>, String> {
    return select_attribute(html, "a[href]".to_string(), "href".to_string()).map_err(|detail| detail.replace("html_select_attribute", "html_links"));
}

/// Every image source on the page, with the same caveat about relative paths.
pub fn images(html: String) -> Result<Vec<String>, String> {
    return select_attribute(html, "img[src]".to_string(), "src".to_string()).map_err(|detail| detail.replace("html_select_attribute", "html_images"));
}

/// The document's title. An error when there is not one, because a page without
/// a title is usually a page that is not the page that was wanted - an error
/// page, or a login form.
pub fn title(html: String) -> Result<String, String> {
    let titles = select_text(html, "title".to_string()).map_err(|detail| detail.replace("html_select_text", "html_title"))?;
    return match titles.into_iter().next() {
        Some(title) if !title.is_empty() => Ok(title),
        _ => Err("html_title: the document has no title".to_string()),
    };
}

/// The content of a `<meta>` tag by name - `description`, `og:title` and the
/// rest of what a page says about itself. Both spellings are checked, since
/// `name` is what the HTML standard uses and `property` is what Open Graph uses.
pub fn meta(html: String, meta_name: String) -> Result<String, String> {
    let document = Html::parse_document(&html);
    let chosen = parse_selector("meta", "html_meta")?;
    for element in document.select(&chosen) {
        let value = element.value();
        let matches_name = value.attr("name").map(|found| found.eq_ignore_ascii_case(&meta_name)).unwrap_or(false);
        let matches_property = value.attr("property").map(|found| found.eq_ignore_ascii_case(&meta_name)).unwrap_or(false);
        if matches_name || matches_property {
            if let Some(content) = value.attr("content") {
                return Ok(content.to_string());
            }
        }
    }
    return Err(format!("html_meta: the document has no meta tag named `{}`", meta_name));
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r#"
        <!doctype html>
        <html>
          <head>
            <title>  A Page  </title>
            <meta name="description" content="what the page is about">
            <meta property="og:title" content="A Page, shared">
          </head>
          <body>
            <h1>Heading</h1>
            <div class="post">
              <a href="/first">First</a>
              <img src="/one.png" alt="one">
            </div>
            <div class="post">
              <a href="https://example.com/second">Second</a>
              <a>No address at all</a>
            </div>
            <p>Some
               text across lines.</p>
          </body>
        </html>
    "#;

    #[test]
    fn the_text_of_a_page_has_no_tags_and_no_stray_whitespace() {
        let found = text(PAGE.to_string());
        assert!(found.contains("Heading"));
        assert!(found.contains("Some text across lines."), "got: {}", found);
        assert!(!found.contains('<'), "got: {}", found);
        assert!(!found.contains("  "), "got: {}", found);
    }

    #[test]
    fn elements_are_found_by_selector() {
        let posts = select_text(PAGE.to_string(), ".post a".to_string()).expect("a valid selector");
        assert_eq!(posts, vec!["First".to_string(), "Second".to_string(), "No address at all".to_string()]);

        let headings = select_text(PAGE.to_string(), "h1".to_string()).expect("a valid selector");
        assert_eq!(headings, vec!["Heading".to_string()]);
    }

    #[test]
    fn a_selector_matching_nothing_gives_nothing_rather_than_an_error() {
        assert_eq!(select_text(PAGE.to_string(), ".comment".to_string()).expect("a valid selector"), Vec::<String>::new());
        assert_eq!(count(PAGE.to_string(), ".comment".to_string()).expect("a valid selector"), 0);
    }

    #[test]
    fn markup_inside_an_element_can_be_taken_whole() {
        let posts = select_html(PAGE.to_string(), "div.post".to_string()).expect("a valid selector");
        assert_eq!(posts.len(), 2);
        assert!(posts[0].contains("<a href=\"/first\">First</a>"), "got: {}", posts[0]);
    }

    #[test]
    fn an_attribute_is_read_from_every_element_that_has_it() {
        let addresses = select_attribute(PAGE.to_string(), "a".to_string(), "href".to_string()).expect("a valid selector");
        // The third link has no href, so it contributes nothing.
        assert_eq!(addresses, vec!["/first".to_string(), "https://example.com/second".to_string()]);
    }

    #[test]
    fn links_and_images_are_listed_in_document_order() {
        assert_eq!(links(PAGE.to_string()).expect("a page"), vec!["/first".to_string(), "https://example.com/second".to_string()]);
        assert_eq!(images(PAGE.to_string()).expect("a page"), vec!["/one.png".to_string()]);
    }

    #[test]
    fn matching_elements_can_be_counted() {
        assert_eq!(count(PAGE.to_string(), "div.post".to_string()).expect("a valid selector"), 2);
        assert_eq!(count(PAGE.to_string(), "a".to_string()).expect("a valid selector"), 3);
    }

    #[test]
    fn the_title_comes_back_tidied() {
        assert_eq!(title(PAGE.to_string()).expect("a titled page"), "A Page");
        assert!(title("<html><body>no title</body></html>".to_string()).is_err());
    }

    #[test]
    fn meta_tags_are_read_by_either_spelling() {
        assert_eq!(meta(PAGE.to_string(), "description".to_string()).expect("a described page"), "what the page is about");
        assert_eq!(meta(PAGE.to_string(), "og:title".to_string()).expect("a shared page"), "A Page, shared");
        assert!(meta(PAGE.to_string(), "author".to_string()).is_err());
    }

    /// Real HTML is not well-formed, and refusing it would make this useless.
    #[test]
    fn broken_markup_is_recovered_from_rather_than_refused() {
        let broken = "<div><p>unclosed<p>another<span>and text".to_string();
        let paragraphs = select_text(broken.clone(), "p".to_string()).expect("a valid selector");
        assert_eq!(paragraphs.len(), 2);
        assert!(text(broken).contains("unclosed"));
    }

    #[test]
    fn a_selector_that_is_not_a_selector_says_so() {
        let failure = select_text(PAGE.to_string(), "div..".to_string()).unwrap_err();
        assert!(failure.contains("is not a CSS selector"), "got: {}", failure);
        assert!(count(PAGE.to_string(), ">>>".to_string()).is_err());
    }

    #[test]
    fn an_empty_document_is_handled_like_any_other() {
        assert_eq!(text(String::new()), "");
        assert_eq!(links(String::new()).expect("a document"), Vec::<String>::new());
    }
}
