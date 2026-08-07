//! Feed module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("feed_parse", StdlibFunction {
        rust_path: "std_lib::feed::parse".to_string(),
        crate_deps: vec![CrateDependency::FeedRs, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("FEED_Feed", "nail::std_lib::feed"), ("FEED_Entry", "nail::std_lib::feed")],
        module: StdlibModule::Feed,
        parameters: vec![nail_param!(text: s)],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("FEED_Feed".to_string()))),
        diverging: false,
        description: "Reads an RSS or Atom document into one shape, whichever it is: the feed's title and link, and its entries in the feed's own order. What a feed omits is empty rather than missing.",
        example: "feed:FEED_Feed = danger(feed_parse(danger(http_request(HTTP_Method::Get, url, hashmap_new(), ``)).body));",
    });

    m.insert("feed_rss", StdlibFunction {
        rust_path: "std_lib::feed::build_rss".to_string(),
        crate_deps: vec![CrateDependency::Chrono, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("FEED_Feed", "nail::std_lib::feed"), ("FEED_Entry", "nail::std_lib::feed")],
        module: StdlibModule::Feed,
        parameters: vec![
            StdlibParameter { name: "feed".to_string(), param_type: NailDataTypeDescriptor::Struct("FEED_Feed".to_string()), pass_by_reference: false },
            StdlibParameter { name: "entries".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct("FEED_Entry".to_string()))), pass_by_reference: true },
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::String)),
        diverging: false,
        description: "Builds an RSS 2.0 document from the same shapes feed_parse reads, so what one program publishes another parses straight back. Every value is XML-escaped, dates are written the RFC 2822 way RSS wants, a zero timestamp leaves the date off, and a feed with no title or no link is an error rather than a feed nothing can follow.",
        example: "rss:s = danger(feed_rss(feed, entries));",
    });

    m.insert("feed_atom", StdlibFunction {
        rust_path: "std_lib::feed::build_atom".to_string(),
        crate_deps: vec![CrateDependency::Chrono, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("FEED_Feed", "nail::std_lib::feed"), ("FEED_Entry", "nail::std_lib::feed")],
        module: StdlibModule::Feed,
        parameters: vec![
            StdlibParameter { name: "feed".to_string(), param_type: NailDataTypeDescriptor::Struct("FEED_Feed".to_string()), pass_by_reference: false },
            StdlibParameter { name: "entries".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct("FEED_Entry".to_string()))), pass_by_reference: true },
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::String)),
        diverging: false,
        description: "Builds an Atom 1.0 document from the same shapes feed_parse reads, the twin of feed_rss for the other format. Dates are RFC 3339, each entry is identified by its id or by its link when it has none, and a feed with no title or no link is an error rather than a feed nothing can follow.",
        example: "atom:s = danger(feed_atom(feed, entries));",
    });

    simple_fns! { m, Feed:
        "feed_sitemap" => "std_lib::feed::build_sitemap", (urls: (&[s])) -> (s!e),
            "Builds the sitemap a search engine reads to learn which pages exist without following every link to find them, from a list of absolute URLs. Duplicates are dropped and the order given is kept. A relative URL is an error, because a sitemap is read with no page to be relative to.",
            "sitemap:s = danger(feed_sitemap(page_urls));";
    }
}
