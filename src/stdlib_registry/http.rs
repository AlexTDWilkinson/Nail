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

    m.insert("http_default_config", StdlibFunction {
        rust_path: "std_lib::http::http_default_config".to_string(),
        crate_deps: vec![CrateDependency::Axum, CrateDependency::DashMap],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("HTTP_Config", "nail::std_lib::http"), ("HTTP_Static", "nail::std_lib::http")],
        module: StdlibModule::Http,
        parameters: vec![],
        return_type: NailDataTypeDescriptor::Struct("HTTP_Config".to_string()),
        diverging: false,
        description: "The default server configuration: no static mounts, 1 MiB body limit, 30 second handler timeout, empty state. Nail has no default field values, so this saves spelling out every field of HTTP_Config.",
        example: "config:HTTP_Config = http_default_config();",
    });
}
