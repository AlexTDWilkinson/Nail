//! HTML module stdlib registry entries.
//!
//! Reading HTML somebody else wrote. Writing it is what the template and
//! markdown modules are for.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Html:
        "html_text" [Scraper] => "std_lib::html::text", (html: s) -> s,
            "Returns all the text of an HTML document with the tags removed and whitespace collapsed.",
            "words:s = html_text(page);";
        "html_select_text" [Scraper] => "std_lib::html::select_text", (html: s, selector: s) -> ([s]!e),
            "Returns the text of every element matching the CSS selector. A selector matching nothing gives an empty array.",
            "headlines:a:s = danger(html_select_text(page, `h2.title`));";
        "html_select_html" [Scraper] => "std_lib::html::select_html", (html: s, selector: s) -> ([s]!e),
            "Returns the markup inside every element matching the CSS selector.",
            "fragments:a:s = danger(html_select_html(page, `div.post`));";
        "html_select_attribute" [Scraper] => "std_lib::html::select_attribute", (html: s, selector: s, attribute: s) -> ([s]!e),
            "Returns one attribute of every matching element, skipping elements that do not carry it.",
            "addresses:a:s = danger(html_select_attribute(page, `a`, `href`));";
        "html_count" [Scraper] => "std_lib::html::count", (html: s, selector: s) -> (i!e),
            "Returns how many elements match the CSS selector.",
            "posts:i = danger(html_count(page, `div.post`));";
        "html_links" [Scraper] => "std_lib::html::links", (html: s) -> ([s]!e),
            "Returns every address an anchor on the page points at, in document order and exactly as written, so a relative link stays relative.",
            "links:a:s = danger(html_links(page));";
        "html_images" [Scraper] => "std_lib::html::images", (html: s) -> ([s]!e),
            "Returns every image source on the page, in document order and exactly as written.",
            "images:a:s = danger(html_images(page));";
        "html_title" [Scraper] => "std_lib::html::title", (html: s) -> (s!e),
            "Returns the document's title, or an error when it has none - which usually means the page is not the page that was wanted.",
            "title:s = danger(html_title(page));";
        "html_meta" [Scraper] => "std_lib::html::meta", (html: s, meta_name: s) -> (s!e),
            "Returns the content of a meta tag by name, checking both the name and property spellings so Open Graph tags are found too.",
            "summary:s = danger(html_meta(page, `description`));";
    }
}
