//! Url module stdlib registry entries.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Url:
        "url_encode" [UrlEncoding] => "std_lib::url::encode", (text: s) -> s,
            "Percent-encodes a string for safe use in a URL.",
            "safe:s = url_encode(`a b&c`);";
        "url_decode" [UrlEncoding] => "std_lib::url::decode", (text: s) -> (s!e),
            "Decodes a percent-encoded URL string. Errors on invalid encoding.",
            "plain:s = danger(url_decode(`a%20b`));";
        "url_parse_query" [UrlEncoding, DashMap] => "std_lib::url::parse_query", (query: s) -> (h s s),
            "Parses a query string like a=1&b=2 into a hashmap.",
            "params:h<s,s> = url_parse_query(`page=2&sort=asc`);";
        "url_build_query" [UrlEncoding, DashMap] => "std_lib::url::build_query", (params: (&(h s s))) -> s,
            "Builds a percent-encoded query string from a hashmap.",
            "query:s = url_build_query(params);";
        "url_join" => "std_lib::url::join", (base: s, reference: s) -> (s!e),
            "Resolves a link against the page it was found on, the way a browser does: /about, ../two, ?page=2, #top and a whole URL all come out as the address to fetch. Errors if the base is not a URL.",
            "target:s = danger(url_join(page_url, link));";
    }

    // The two functions that speak in pieces of a URL use the full struct form.
    m.insert("url_parse", StdlibFunction {
        rust_path: "std_lib::url::parse".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("URL_Parts", "nail::std_lib::url")],
        module: StdlibModule::Url,
        parameters: vec![nail_param!(text: s)],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("URL_Parts".to_string()))),
        diverging: false,
        description: "Takes a URL apart into scheme, user, host, port, path, query and fragment. The port is 0 when the URL did not name one. Errors when there is no scheme, since guessing turns a path into a request somewhere nobody meant.",
        example: "parts:URL_Parts = danger(url_parse(`https://example.com/blog?page=2`));",
    });

    m.insert("url_format", StdlibFunction {
        rust_path: "std_lib::url::format".to_string(),
        crate_deps: vec![],
        struct_derives: vec![],
        custom_type_imports: vec![("URL_Parts", "nail::std_lib::url")],
        module: StdlibModule::Url,
        parameters: vec![StdlibParameter { name: "parts".to_string(), param_type: NailDataTypeDescriptor::Struct("URL_Parts".to_string()), pass_by_reference: true }],
        return_type: nail_type!(s),
        diverging: false,
        description: "Puts a URL back together from its pieces, so a program can change one and keep the rest.",
        example: "address:s = url_format(parts);",
    });
}
