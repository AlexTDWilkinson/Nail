//! Http module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("http_server", StdlibFunction {
        rust_path: "std_lib::http::http_server".to_string(),

        crate_deps: vec![CrateDependency::Axum, CrateDependency::Tokio, CrateDependency::TowerHttp, CrateDependency::UrlEncoding],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("HTTP_Request", "nail::std_lib::http"), ("HTTP_Response", "nail::std_lib::http"), ("HTTP_Config", "nail::std_lib::http"), ("HTTP_Static", "nail::std_lib::http")],
        module: StdlibModule::Http,
        parameters: vec![
            StdlibParameter {
                name: "port".to_string(),
                param_type: NailDataTypeDescriptor::Int,
                pass_by_reference: false
            },
            StdlibParameter {
                name: "config".to_string(),
                param_type: NailDataTypeDescriptor::Struct("HTTP_Config".to_string()),
                pass_by_reference: false
            }
        ],
        return_type: NailDataTypeDescriptor::Void,

        diverging: false,
        description: "Starts an HTTP server on the given port. Every request is passed to the program's handle_request(request:HTTP_Request, state:h<s,s>):HTTP_Response function, along with the config's state hashmap. Blocks forever.",
        example: "http_server(8080, config);",
    });

    m.insert("http_path_matches", StdlibFunction {
        rust_path: "std_lib::http::http_path_matches".to_string(),

        crate_deps: vec![CrateDependency::UrlEncoding],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Http,
        parameters: vec![
            StdlibParameter { name: "pattern".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
            StdlibParameter { name: "path".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
        ],
        return_type: NailDataTypeDescriptor::Boolean,

        diverging: false,
        description: "Whether a request path matches a route pattern. Pattern segments beginning with ':' match any single segment, and a trailing '*' matches the rest of the path.",
        example: "matched:b = http_path_matches(`/dictionary/:word`, request.path);",
    });

    m.insert("http_path_params", StdlibFunction {
        rust_path: "std_lib::http::http_path_params".to_string(),

        crate_deps: vec![CrateDependency::UrlEncoding, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Http,
        parameters: vec![
            StdlibParameter { name: "pattern".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
            StdlibParameter { name: "path".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
        ],
        return_type: NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)),

        diverging: false,
        description: "The named segments a route pattern binds, so `/dictionary/:word` against `/dictionary/cat` gives {word: cat}. Empty when the pattern does not match.",
        example: "params:h<s,s> = http_path_params(`/dictionary/:word`, request.path);",
    });

m.insert("http_request", StdlibFunction {
        rust_path: "std_lib::http::http_request".to_string(),

        crate_deps: vec![CrateDependency::Tokio, CrateDependency::Reqwest, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("HTTP_Response", "nail::std_lib::http"), ("HTTP_Method", "nail::std_lib::http")],
        module: StdlibModule::Http,
        parameters: vec![
            StdlibParameter {
                name: "method".to_string(),
                param_type: NailDataTypeDescriptor::Enum("HTTP_Method".to_string()),
                pass_by_reference: false
            },
            StdlibParameter { 
                name: "url".to_string(), 
                param_type: NailDataTypeDescriptor::String, 
                pass_by_reference: false 
            },
            StdlibParameter { 
                name: "headers".to_string(), 
                param_type: NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)), 
                pass_by_reference: false 
            },
            StdlibParameter { 
                name: "body".to_string(), 
                param_type: NailDataTypeDescriptor::String, 
                pass_by_reference: false 
            },
        ],
        return_type: NailDataTypeDescriptor::Result(
            Box::new(NailDataTypeDescriptor::Struct("HTTP_Response".to_string()))
        ),

        diverging: false,
        description: "Makes an HTTP request (GET, POST, PUT, DELETE, or PATCH) and returns the response status and body.",
        example: "response:HTTP_Response = danger(http_request(HTTP_Method::Get, `https://example.com`, headers, ``));",
    });

    m.insert("http_part_text", StdlibFunction {
        rust_path: "std_lib::http::http_part_text".to_string(),
        crate_deps: vec![],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("HTTP_Part", "nail::std_lib::http")],
        module: StdlibModule::Http,
        parameters: vec![
            StdlibParameter { name: "name".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
            StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Struct("HTTP_Part".to_string()),
        diverging: false,
        description: "One text field of a multipart form, the way a browser sends a filled-in text box.",
        example: "purpose:HTTP_Part = http_part_text(`purpose`, `avatar`);",
    });

    m.insert("http_part_file", StdlibFunction {
        rust_path: "std_lib::http::http_part_file".to_string(),
        crate_deps: vec![],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("HTTP_Part", "nail::std_lib::http")],
        module: StdlibModule::Http,
        parameters: vec![
            StdlibParameter { name: "name".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
            StdlibParameter { name: "file_path".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Struct("HTTP_Part".to_string()),
        diverging: false,
        description: "One file field of a multipart form. The file is read when the request is sent, so its bytes never have to pass through the program, and its name and media type are taken from the path.",
        example: "upload:HTTP_Part = http_part_file(`file`, `report.pdf`);",
    });

    m.insert("http_request_multipart", StdlibFunction {
        rust_path: "std_lib::http::http_request_multipart".to_string(),
        crate_deps: vec![CrateDependency::Tokio, CrateDependency::Reqwest, CrateDependency::DashMap],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("HTTP_Response", "nail::std_lib::http"), ("HTTP_Method", "nail::std_lib::http"), ("HTTP_Part", "nail::std_lib::http")],
        module: StdlibModule::Http,
        parameters: vec![
            StdlibParameter { name: "method".to_string(), param_type: NailDataTypeDescriptor::Enum("HTTP_Method".to_string()), pass_by_reference: false },
            StdlibParameter { name: "url".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
            StdlibParameter { name: "headers".to_string(), param_type: NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)), pass_by_reference: false },
            StdlibParameter { name: "parts".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct("HTTP_Part".to_string()))), pass_by_reference: true },
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("HTTP_Response".to_string()))),
        diverging: false,
        description: "Sends a multipart/form-data request, the encoding file uploads use. Takes Post, Put or Patch, and sets Content-Type itself from the body's boundary, so headers must not carry one.",
        example: "response:HTTP_Response = danger(http_request_multipart(HTTP_Method::Post, `https://api.example.com/files`, headers, parts));",
    });

    m.insert("http_default_retry", StdlibFunction {
        rust_path: "std_lib::http::http_default_retry".to_string(),
        crate_deps: vec![],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("HTTP_Retry", "nail::std_lib::http")],
        module: StdlibModule::Http,
        parameters: vec![],
        return_type: NailDataTypeDescriptor::Struct("HTTP_Retry".to_string()),
        diverging: false,
        description: "Retry settings worth having: three attempts, a wait starting at 250ms and doubling to at most 5s, and a 30s deadline for each attempt.",
        example: "retry:HTTP_Retry = http_default_retry();",
    });

    m.insert("http_request_retry", StdlibFunction {
        rust_path: "std_lib::http::http_request_retry".to_string(),
        crate_deps: vec![CrateDependency::Tokio, CrateDependency::Reqwest, CrateDependency::DashMap, CrateDependency::Rand],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("HTTP_Response", "nail::std_lib::http"), ("HTTP_Method", "nail::std_lib::http"), ("HTTP_Retry", "nail::std_lib::http")],
        module: StdlibModule::Http,
        parameters: vec![
            StdlibParameter { name: "method".to_string(), param_type: NailDataTypeDescriptor::Enum("HTTP_Method".to_string()), pass_by_reference: false },
            StdlibParameter { name: "url".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
            StdlibParameter { name: "headers".to_string(), param_type: NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)), pass_by_reference: false },
            StdlibParameter { name: "body".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
            StdlibParameter { name: "retry".to_string(), param_type: NailDataTypeDescriptor::Struct("HTTP_Retry".to_string()), pass_by_reference: false },
        ],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("HTTP_Response".to_string()))),
        diverging: false,
        description: "Makes an HTTP request, sending it again while it fails in a way that might not fail next time: no answer at all, or a 408, 429, 500, 502, 503 or 504. Waits longer between attempts each time, honours a Retry-After header, and returns the last response whatever its status. The request is sent again unchanged, so an API that must not act twice wants an idempotency key in the headers.",
        example: "response:HTTP_Response = danger(http_request_retry(HTTP_Method::Get, url, headers, ``, http_default_retry()));",
    });

    m.insert("http_default_cookie", StdlibFunction {
        rust_path: "std_lib::http::http_default_cookie".to_string(),
        crate_deps: vec![],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("HTTP_Cookie", "nail::std_lib::http")],
        module: StdlibModule::Http,
        parameters: vec![
            StdlibParameter { name: "name".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
            StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
        ],
        return_type: NailDataTypeDescriptor::Struct("HTTP_Cookie".to_string()),
        diverging: false,
        description: "A cookie with the safe defaults filled in: site-wide path, session lifetime, HttpOnly, Secure, SameSite=Lax. Change the fields that need changing.",
        example: "cookie:HTTP_Cookie = http_default_cookie(`sid`, session_id);",
    });

    m.insert("http_build_cookie", StdlibFunction {
        rust_path: "std_lib::http::http_build_cookie".to_string(),
        crate_deps: vec![],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("HTTP_Cookie", "nail::std_lib::http")],
        module: StdlibModule::Http,
        parameters: vec![StdlibParameter { name: "cookie".to_string(), param_type: NailDataTypeDescriptor::Struct("HTTP_Cookie".to_string()), pass_by_reference: false }],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::String)),
        diverging: false,
        description: "Builds the Set-Cookie header value for a cookie. Errors on a name, value or SameSite setting a browser would reject.",
        example: "header:s = danger(http_build_cookie(cookie));",
    });

    m.insert("http_parse_cookies", StdlibFunction {
        rust_path: "std_lib::http::http_parse_cookies".to_string(),
        crate_deps: vec![CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Http,
        parameters: vec![StdlibParameter { name: "header".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }],
        return_type: NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)),
        diverging: false,
        description: "Parses the browser's Cookie header, which holds every cookie for the site at once, into a hashmap of name to value.",
        example: "cookies:h<s,s> = http_parse_cookies(raw_cookie_header);",
    });

    m.insert("http_default_config", StdlibFunction {
        rust_path: "std_lib::http::http_default_config".to_string(),
        crate_deps: vec![CrateDependency::Axum, CrateDependency::DashMap],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("HTTP_Config", "nail::std_lib::http"), ("HTTP_Static", "nail::std_lib::http")],
        module: StdlibModule::Http,
        parameters: vec![],
        return_type: NailDataTypeDescriptor::Struct("HTTP_Config".to_string()),
        diverging: false,
        description: "The default server configuration: no static mounts, 8 MiB body limit, 30 second handler timeout, empty state. Nail has no default field values, so this saves spelling out every field of HTTP_Config.",
        example: "config:HTTP_Config = http_default_config();",
    });

    m.insert("http_multipart_extract", StdlibFunction {
        rust_path: "std_lib::http::multipart_extract".to_string(),
        crate_deps: vec![CrateDependency::Tokio, CrateDependency::DashMap, CrateDependency::Uuid],
        struct_derives: vec![],
        custom_type_imports: vec![],
        module: StdlibModule::Http,
        parameters: vec![nail_param!(body_path: s), nail_param!(content_type: s), nail_param!(into_directory: s)],
        return_type: nail_type!(((h s s)!e)),
        diverging: false,
        description: "Takes a multipart/form-data body apart: file parts are written into the directory and text parts come back as values, in one hashmap where `name` is a value or a written path, `name.filename` is the cleaned-up name the client gave, and `name.type` is the declared content type. Read in blocks, so a large upload costs no more memory than a small one.",
        example: "fields:h<s,s> = danger(http_multipart_extract(request.body_path, danger(hashmap_get(request.headers, `content-type`)), `uploads`));",
    });

    m.insert("http_server_realtime", StdlibFunction {
        rust_path: "std_lib::http::http_server_realtime".to_string(),
        crate_deps: vec![CrateDependency::Axum, CrateDependency::Tokio, CrateDependency::TowerHttp, CrateDependency::UrlEncoding, CrateDependency::Futures],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("HTTP_Request", "nail::std_lib::http"), ("HTTP_Response", "nail::std_lib::http"), ("HTTP_Config", "nail::std_lib::http"), ("HTTP_Static", "nail::std_lib::http")],
        module: StdlibModule::Http,
        parameters: vec![nail_param!(port: i), StdlibParameter { name: "config".to_string(), param_type: NailDataTypeDescriptor::Struct("HTTP_Config".to_string()), pass_by_reference: false }, nail_param!(live_path: s)],
        return_type: NailDataTypeDescriptor::Void,
        diverging: false,
        description: "http_server with a live endpoint beside the ordinary routes: a GET to live_path is a server-sent-event stream of everything http_live_send broadcasts, a websocket upgrade on the same path joins the same channel, and each text frame a client sends is answered by the program's handle_message function. ?channel=name picks the channel.",
        example: "http_server_realtime(8080, config, `/live`);",
    });

    simple_fns! { m, Http:
        "http_live_send" [Tokio] => "std_lib::http::http_live_send", (channel: s, message: s) -> i,
            "Sends a message to every SSE stream and websocket subscribed to the channel, returning how many there were. Nobody listening is 0, not an error.",
            "heard_by:i = http_live_send(`chat`, rendered_message);";
        "http_live_count" [Tokio] => "std_lib::http::http_live_count", (channel: s) -> i,
            "How many live subscribers a channel has right now.",
            "watching:i = http_live_count(`chat`);";
    }

    let websocket_parameter = || StdlibParameter { name: "socket".to_string(), param_type: NailDataTypeDescriptor::Struct("HTTP_Websocket".to_string()), pass_by_reference: true };
    let websocket_import = || vec![("HTTP_Websocket", "nail::std_lib::http")];
    let websocket_deps = || vec![CrateDependency::TokioTungstenite, CrateDependency::Tokio, CrateDependency::DashMap, CrateDependency::Uuid, CrateDependency::Serde, CrateDependency::Futures];

    m.insert("http_ws_connect", StdlibFunction {
        rust_path: "std_lib::http::ws_connect".to_string(),
        crate_deps: websocket_deps(),
        struct_derives: vec![],
        custom_type_imports: websocket_import(),
        module: StdlibModule::Http,
        parameters: vec![nail_param!(url: s)],
        return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Struct("HTTP_Websocket".to_string()))),
        diverging: false,
        description: "Opens a websocket to a ws:// or wss:// URL - the client half of http_server_realtime. This is how a program consumes a streaming API: an exchange feed, a chat bridge, another Nail program.",
        example: "feed:HTTP_Websocket = danger(http_ws_connect(`wss://stream.example.com/live`));",
    });

    m.insert("http_ws_send", StdlibFunction {
        rust_path: "std_lib::http::ws_send".to_string(),
        crate_deps: websocket_deps(),
        struct_derives: vec![],
        custom_type_imports: websocket_import(),
        module: StdlibModule::Http,
        parameters: vec![websocket_parameter(), nail_param!(text: s)],
        return_type: nail_type!((v!e)),
        diverging: false,
        description: "Sends one text frame.",
        example: "danger(http_ws_send(feed, subscribe_message));",
    });

    m.insert("http_ws_receive", StdlibFunction {
        rust_path: "std_lib::http::ws_receive".to_string(),
        crate_deps: websocket_deps(),
        struct_derives: vec![],
        custom_type_imports: websocket_import(),
        module: StdlibModule::Http,
        parameters: vec![websocket_parameter(), nail_param!(timeout_milliseconds: i)],
        return_type: nail_type!((s!e)),
        diverging: false,
        description: "The next text frame the other side sends. Waits up to the timeout, or forever when the timeout is 0. Pings are answered quietly. A closed connection is an error and forgets the handle.",
        example: "update:s = danger(http_ws_receive(feed, 30000));",
    });

    m.insert("http_ws_close", StdlibFunction {
        rust_path: "std_lib::http::ws_close".to_string(),
        crate_deps: websocket_deps(),
        struct_derives: vec![],
        custom_type_imports: websocket_import(),
        module: StdlibModule::Http,
        parameters: vec![websocket_parameter()],
        return_type: nail_type!((v!e)),
        diverging: false,
        description: "Says goodbye properly and forgets the handle. Closing twice is not an error.",
        example: "danger(http_ws_close(feed));",
    });
}
