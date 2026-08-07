//! HTML module stdlib registry entries.
//!
//! Reading HTML somebody else wrote. Writing it is what the template and
//! markdown modules are for.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Html:
        "html_text" [Scraper] => "std_lib::html::text", (html: s) -> s,
            "Returns all the text of an HTML document with the tags removed and whitespace collapsed.",
            "page:s = `<html><head><title>Nail</title></head><body><h2 class=\"title\">Hello</h2></body></html>`;\nwords:s = html_text(page);";
        "html_select_text" [Scraper] => "std_lib::html::select_text", (html: s, selector: s) -> ([s]!e),
            "Returns the text of every element matching the CSS selector. A selector matching nothing gives an empty array.",
            "page:s = `<html><head><title>Nail</title></head><body><h2 class=\"title\">Hello</h2></body></html>`;\nheadlines:a:s = danger(html_select_text(page, `h2.title`));";
        "html_select_html" [Scraper] => "std_lib::html::select_html", (html: s, selector: s) -> ([s]!e),
            "Returns the markup inside every element matching the CSS selector.",
            "page:s = `<html><head><title>Nail</title></head><body><h2 class=\"title\">Hello</h2></body></html>`;\nfragments:a:s = danger(html_select_html(page, `div.post`));";
        "html_select_attribute" [Scraper] => "std_lib::html::select_attribute", (html: s, selector: s, attribute: s) -> ([s]!e),
            "Returns one attribute of every matching element, skipping elements that do not carry it.",
            "page:s = `<html><head><title>Nail</title></head><body><h2 class=\"title\">Hello</h2></body></html>`;\naddresses:a:s = danger(html_select_attribute(page, `a`, `href`));";
        "html_count" [Scraper] => "std_lib::html::count", (html: s, selector: s) -> (i!e),
            "Returns how many elements match the CSS selector.",
            "page:s = `<html><head><title>Nail</title></head><body><h2 class=\"title\">Hello</h2></body></html>`;\nposts:i = danger(html_count(page, `div.post`));";
        "html_links" [Scraper] => "std_lib::html::links", (html: s) -> ([s]!e),
            "Returns every address an anchor on the page points at, in document order and exactly as written, so a relative link stays relative.",
            "page:s = `<html><head><title>Nail</title></head><body><h2 class=\"title\">Hello</h2></body></html>`;\nlinks:a:s = danger(html_links(page));";
        "html_images" [Scraper] => "std_lib::html::images", (html: s) -> ([s]!e),
            "Returns every image source on the page, in document order and exactly as written.",
            "page:s = `<html><head><title>Nail</title></head><body><h2 class=\"title\">Hello</h2></body></html>`;\nimages:a:s = danger(html_images(page));";
        "html_title" [Scraper] => "std_lib::html::title", (html: s) -> (s!e),
            "Returns the document's title, or an error when it has none - which usually means the page is not the page that was wanted.",
            "page:s = `<html><head><title>Nail</title></head><body><h2 class=\"title\">Hello</h2></body></html>`;\ntitle:s = danger(html_title(page));";
        "html_meta" [Scraper] => "std_lib::html::meta", (html: s, meta_name: s) -> (s!e),
            "Returns the content of a meta tag by name, checking both the name and property spellings so Open Graph tags are found too.",
            "page:s = `<html><head><title>Nail</title><meta name=\"description\" content=\"A language that refuses to surprise you\"></head><body><h2 class=\"title\">Hello</h2></body></html>`;\nsummary:s = danger(html_meta(page, `description`));";
        "html_sanitize" [Ammonia] => "std_lib::html::sanitize", (dirty: s) -> s,
            "Cleans untrusted HTML so it is safe to serve: scripts, event handlers and javascript: links are removed, ordinary formatting is kept. Anything a person typed must pass through here - including markdown_to_html's rendering of it - before being put in a page.",
            "comment:s = `**bold** and <script>alert(1)</script>`;\nclean:s = html_sanitize(markdown_to_html(comment));";
        "html_to_markdown" [Htmd] => "std_lib::html::to_markdown", (document: s) -> (s!e),
            "The page as markdown, keeping headings, lists, links, emphasis and code and dropping the rest. The other direction from markdown_to_html, for a page fetched off the internet that has to be stored, diffed or searched. html_text is the one that leaves only the words.",
            "page:s = `<html><head><title>Nail</title></head><body><h2 class=\"title\">Hello</h2></body></html>`;\nnotes:s = danger(html_to_markdown(page));";
    }
}
