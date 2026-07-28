//! Http module stdlib registry entries

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("http_server", StdlibFunction {
        rust_path: "std_lib::http::http_server".to_string(),

        crate_deps: vec![CrateDependency::Axum, CrateDependency::Tokio],
        struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
        custom_type_imports: vec![("HTTP_Route", "nail::std_lib::http")],
        module: StdlibModule::Http,
        parameters: vec![
            StdlibParameter { 
                name: "port".to_string(), 
                param_type: NailDataTypeDescriptor::Int, 
                pass_by_reference: false 
            },
            StdlibParameter { 
                name: "routes".to_string(), 
                param_type: NailDataTypeDescriptor::HashMap(
                    Box::new(NailDataTypeDescriptor::String),
                    Box::new(NailDataTypeDescriptor::Struct("HTTP_Route".to_string()))
                ), 
                pass_by_reference: false 
            }
        ],
        return_type: NailDataTypeDescriptor::Void,

        diverging: false,
        description: "Starts an HTTP server on the given port, serving the routes hashmap (path to HTTP_Route). Blocks forever.",
        example: "http_server(8080, routes);",
    });

m.insert("http_request", StdlibFunction {
        rust_path: "std_lib::http::http_request".to_string(),

        crate_deps: vec![CrateDependency::Tokio, CrateDependency::Reqwest, CrateDependency::DashMap],
        struct_derives: vec![],
        custom_type_imports: vec![("HTTP_Response", "nail::std_lib::http")],
        module: StdlibModule::Http,
        parameters: vec![
            StdlibParameter { 
                name: "method".to_string(), 
                param_type: NailDataTypeDescriptor::String, 
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
        example: "response:HTTP_Response = danger(http_request(`GET`, `https://example.com`, headers, ``));",
    });
}
