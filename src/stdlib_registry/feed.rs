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
}
