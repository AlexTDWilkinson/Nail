use crate::lexer::NailDataTypeDescriptor;
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CrateDependency {
    Axum,
    Tokio,
    SerdeJson,
    Serde,
    Regex,
    Rand,
    DashMap,
    Pulldown,
    Reqwest,
    Sha2,
    Md5,
    Uuid,
}

impl CrateDependency {
    pub fn to_cargo_dep(&self) -> &'static str {
        match self {
            CrateDependency::Axum => "axum = \"0.7\"",
            CrateDependency::Tokio => "tokio = { version = \"1\", features = [\"rt-multi-thread\", \"macros\"] }",
            CrateDependency::SerdeJson => "serde_json = \"1.0\"",
            CrateDependency::Serde => "serde = { version = \"1.0\", features = [\"derive\"] }",
            CrateDependency::Regex => "regex = \"1.10\"",
            CrateDependency::Rand => "rand = \"0.8\"",
            CrateDependency::DashMap => "dashmap = \"6.1.0\"",
            CrateDependency::Pulldown => "pulldown-cmark = \"0.9\"",
            CrateDependency::Reqwest => "reqwest = \"0.11\"",
            CrateDependency::Sha2 => "sha2 = \"0.10\"",
            CrateDependency::Md5 => "md5 = \"0.7\"",
            CrateDependency::Uuid => "uuid = { version = \"1.0\", features = [\"v4\"] }",
        }
    }

    pub fn to_crate_name(&self) -> &'static str {
        match self {
            CrateDependency::Axum => "axum",
            CrateDependency::Tokio => "tokio",
            CrateDependency::SerdeJson => "serde_json",
            CrateDependency::Serde => "serde",
            CrateDependency::Regex => "regex",
            CrateDependency::Rand => "rand",
            CrateDependency::DashMap => "dashmap",
            CrateDependency::Pulldown => "pulldown-cmark",
            CrateDependency::Reqwest => "reqwest",
            CrateDependency::Sha2 => "sha2",
            CrateDependency::Md5 => "md5",
            CrateDependency::Uuid => "uuid",
        }
    }

    pub fn to_rust_import(&self) -> &'static str {
        match self {
            CrateDependency::Axum => "use axum;",
            CrateDependency::Tokio => "use tokio;",
            CrateDependency::SerdeJson => "use serde_json;",
            CrateDependency::Serde => "use serde;",
            CrateDependency::Regex => "use regex;",
            CrateDependency::Rand => "use rand;",
            CrateDependency::DashMap => "use dashmap;",
            CrateDependency::Pulldown => "use pulldown_cmark;",
            CrateDependency::Reqwest => "use reqwest;",
            CrateDependency::Sha2 => "use sha2;",
            CrateDependency::Md5 => "use md5;",
            CrateDependency::Uuid => "use uuid;",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StructDerive {
    SerdeSerialize,
    SerdeDeserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
}

impl StructDerive {
    pub fn to_derive_attr(&self) -> &'static str {
        match self {
            StructDerive::SerdeSerialize => "serde::Serialize",
            StructDerive::SerdeDeserialize => "serde::Deserialize",
            StructDerive::Clone => "Clone",
            StructDerive::Debug => "Debug",
            StructDerive::PartialEq => "PartialEq",
            StructDerive::Eq => "Eq",
            StructDerive::Hash => "Hash",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StdlibModule {
    Http,
    Fs,
    Json,
    String,
    Int,
    Float,
    Array,
    Math,
    Time,
    Env,
    Process,
    Path,
    Error,
    HashMap,
    IO,
    Print,
    Markdown,
    Crypto,
}

impl StdlibModule {
    pub fn to_module_path(&self) -> &'static str {
        match self {
            StdlibModule::Http => "std_lib::http",
            StdlibModule::Fs => "std_lib::fs",
            StdlibModule::Json => "std_lib::json",
            StdlibModule::String => "std_lib::string",
            StdlibModule::Int => "std_lib::int",
            StdlibModule::Float => "std_lib::float",
            StdlibModule::Array => "std_lib::array",
            StdlibModule::Math => "std_lib::math",
            StdlibModule::Time => "std_lib::time",
            StdlibModule::Env => "std_lib::env",
            StdlibModule::Process => "std_lib::process",
            StdlibModule::Path => "std_lib::path",
            StdlibModule::Error => "std_lib::error",
            StdlibModule::HashMap => "std_lib::hashmap",
            StdlibModule::IO => "std_lib::io",
            StdlibModule::Print => "std_lib::print",
            StdlibModule::Markdown => "std_lib::markdown",
            StdlibModule::Crypto => "std_lib::crypto",
        }
    }
}

#[derive(Clone, Debug)]
pub struct StdlibParameter {
    pub name: String,
    pub param_type: NailDataTypeDescriptor,
    pub pass_by_reference: bool,
}

#[derive(Clone, Debug)]
pub enum TypeInferenceRule {
    Fixed(NailDataTypeDescriptor),
    ParameterType(usize),
    ResultInnerType(usize),
    ArrayElementType(usize),
    ArrayOfParameterType(usize),
    ReturnType,
    ReturnTypeAsArray(usize),
    UseExpectedType,
    HashMapValueType(usize),
    HashMapKeyArray(usize),
    HashMapValueArray(usize),
}

#[derive(Clone, Debug)]
pub struct StdlibFunction {
    /// The Rust path to call this function (e.g., "std_lib::http::http_server_start")
    pub rust_path: String,

    /// External crate dependencies required for this function
    pub crate_deps: Vec<CrateDependency>,
    /// Additional derives needed for structs/enums when this function is used
    pub struct_derives: Vec<StructDerive>,
    /// Custom types (structs/enums) to import when this function is used
    /// Format: ("TypeName", "module_path") e.g., ("HTTP_Response", "nail::std_lib::http")
    pub custom_type_imports: Vec<(&'static str, &'static str)>,
    /// The module group this function belongs to
    pub module: StdlibModule,
    pub parameters: Vec<StdlibParameter>,
    pub return_type: NailDataTypeDescriptor,
    pub type_inference: Option<TypeInferenceRule>,
    /// Whether this function never returns (like panic! or exit)
    pub diverging: bool,
    /// Description of what the function does
    pub description: &'static str,
    /// Example usage of the function
    pub example: &'static str,
}

lazy_static! {
    pub static ref STDLIB_FUNCTIONS: HashMap<&'static str, StdlibFunction> = {
        let mut m = HashMap::new();

        // HTTP server function - THE ONE AND ONLY
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
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // File system functions (future)
        m.insert("fs_read", StdlibFunction {
            rust_path: "std_lib::fs::read_file".to_string(),

            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Fs,
            parameters: vec![
                StdlibParameter { name: "path".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::String)),
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("fs_write", StdlibFunction {
            rust_path: "std_lib::fs::write_file".to_string(),

            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Fs,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // JSON functions (future)
        m.insert("json_parse", StdlibFunction {
            rust_path: "std_lib::json::parse".to_string(),

            crate_deps: vec![CrateDependency::SerdeJson, CrateDependency::Serde],
            struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
            custom_type_imports: vec![],
            module: StdlibModule::Json,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("json_stringify", StdlibFunction {
            rust_path: "std_lib::json::stringify".to_string(),

            crate_deps: vec![CrateDependency::SerdeJson, CrateDependency::Serde],
            struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
            custom_type_imports: vec![],
            module: StdlibModule::Json,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // Type conversion functions
        m.insert("string_from", StdlibFunction {
            rust_path: "std_lib::string::from".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        // Array to string conversion functions
        m.insert("string_from_array_i64", StdlibFunction {
            rust_path: "std_lib::string::from_array_i64".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)), pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_from_array_f64", StdlibFunction {
            rust_path: "std_lib::string::from_array_f64".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Float)), pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_from_array_string", StdlibFunction {
            rust_path: "std_lib::string::from_array_string".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)), pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_from_array_bool", StdlibFunction {
            rust_path: "std_lib::string::from_array_bool".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Boolean)), pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("int_from", StdlibFunction {
            rust_path: "std_lib::int::from".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Int,
            parameters: vec![
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Int)),
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("float_from", StdlibFunction {
            rust_path: "std_lib::float::from".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Float,
            parameters: vec![
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Float)),
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // IO functions
        m.insert("print", StdlibFunction {
            rust_path: "print_macro!".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Print,
            parameters: vec![
                StdlibParameter { 
                    name: "message".to_string(), 
                    param_type: NailDataTypeDescriptor::Any, 
                    pass_by_reference: false 
                }
            ],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "Print a value to stdout with a newline",
            example: "print(\"Hello, World!\")",
        });

        m.insert("eprintln", StdlibFunction {
            rust_path: "eprintln!".to_string(),  // Macro, needs special handling

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("print_no_newline", StdlibFunction {
            rust_path: "std_lib::print::print_no_newline".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Print,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("io_read_line", StdlibFunction {
            rust_path: "std_lib::io::read_line".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::IO,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("io_read_line_prompt", StdlibFunction {
            rust_path: "std_lib::io::read_line_prompt".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::IO,
            parameters: vec![
                StdlibParameter { name: "prompt".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::String)),
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("io_read_int", StdlibFunction {
            rust_path: "std_lib::io::read_int".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::IO,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("io_read_int_prompt", StdlibFunction {
            rust_path: "std_lib::io::read_int_prompt".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::IO,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("io_read_float", StdlibFunction {
            rust_path: "std_lib::io::read_float".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::IO,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("io_read_float_prompt", StdlibFunction {
            rust_path: "std_lib::io::read_float_prompt".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::IO,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("print_clear_screen", StdlibFunction {
            rust_path: "std_lib::print::print_clear_screen".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Print,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("print_debug", StdlibFunction {
            rust_path: "std_lib::print::print_debug".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Print,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // String manipulation
        m.insert("string_concat", StdlibFunction {
            rust_path: "std_lib::string::concat".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_split", StdlibFunction {
            rust_path: "std_lib::string::split".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "delimiter".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)),
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_trim", StdlibFunction {
            rust_path: "std_lib::string::trim".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_contains", StdlibFunction {
            rust_path: "std_lib::string::contains".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "pattern".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Boolean,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_replace", StdlibFunction {
            rust_path: "std_lib::string::replace".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "from".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "to".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_len", StdlibFunction {
            rust_path: "std_lib::string::len".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Int,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_to_uppercase", StdlibFunction {
            rust_path: "std_lib::string::to_uppercase".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_to_lowercase", StdlibFunction {
            rust_path: "std_lib::string::to_lowercase".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // New string functions
        m.insert("string_starts_with", StdlibFunction {
            rust_path: "std_lib::string::starts_with".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "prefix".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Boolean,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_ends_with", StdlibFunction {
            rust_path: "std_lib::string::ends_with".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "suffix".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Boolean,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_index_of", StdlibFunction {
            rust_path: "std_lib::string::index_of".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "substring".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Int)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_last_index_of", StdlibFunction {
            rust_path: "std_lib::string::last_index_of".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "substring".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Int)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_substring", StdlibFunction {
            rust_path: "std_lib::string::substring".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "start".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false },
                StdlibParameter { name: "end".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::String)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_repeat", StdlibFunction {
            rust_path: "std_lib::string::repeat".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "count".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_reverse", StdlibFunction {
            rust_path: "std_lib::string::reverse".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_join", StdlibFunction {
            rust_path: "std_lib::string::join".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)), pass_by_reference: false },
                StdlibParameter { name: "separator".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_chars", StdlibFunction {
            rust_path: "std_lib::string::chars".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_is_empty", StdlibFunction {
            rust_path: "std_lib::string::is_empty".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Boolean,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_pad_start", StdlibFunction {
            rust_path: "std_lib::string::pad_start".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "target_length".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false },
                StdlibParameter { name: "pad_str".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_pad_end", StdlibFunction {
            rust_path: "std_lib::string::pad_end".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "target_length".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false },
                StdlibParameter { name: "pad_str".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_trim_start", StdlibFunction {
            rust_path: "std_lib::string::trim_start".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_trim_end", StdlibFunction {
            rust_path: "std_lib::string::trim_end".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_replace_first", StdlibFunction {
            rust_path: "std_lib::string::replace_first".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "from".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "to".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        // More new string functions
        m.insert("string_to_snake_case", StdlibFunction {
            rust_path: "std_lib::string::to_snake_case".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_to_kebab_case", StdlibFunction {
            rust_path: "std_lib::string::to_kebab_case".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_to_title_case", StdlibFunction {
            rust_path: "std_lib::string::to_title_case".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_to_sentence_case", StdlibFunction {
            rust_path: "std_lib::string::to_sentence_case".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_split_lines", StdlibFunction {
            rust_path: "std_lib::string::split_lines".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_split_whitespace", StdlibFunction {
            rust_path: "std_lib::string::split_whitespace".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_pad_left", StdlibFunction {
            rust_path: "std_lib::string::pad_left".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "target_length".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false },
                StdlibParameter { name: "pad_str".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_pad_right", StdlibFunction {
            rust_path: "std_lib::string::pad_right".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "target_length".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false },
                StdlibParameter { name: "pad_str".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_replace_all", StdlibFunction {
            rust_path: "std_lib::string::replace_all".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "from".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "to".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_is_alphabetic", StdlibFunction {
            rust_path: "std_lib::string::is_alphabetic".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Boolean,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_is_digits_only", StdlibFunction {
            rust_path: "std_lib::string::is_digits_only".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Boolean,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_is_alphanumeric", StdlibFunction {
            rust_path: "std_lib::string::is_alphanumeric".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Boolean,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_is_numeric", StdlibFunction {
            rust_path: "std_lib::string::is_numeric".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Boolean,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_count", StdlibFunction {
            rust_path: "std_lib::string::count".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "substring".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Int,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_capitalize", StdlibFunction {
            rust_path: "std_lib::string::capitalize".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_slice", StdlibFunction {
            rust_path: "std_lib::string::slice".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "start".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false },
                StdlibParameter { name: "end".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::String)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("string_is_numeric", StdlibFunction {
            rust_path: "std_lib::string::is_numeric".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![
                StdlibParameter { name: "s".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Boolean,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });


        // Array operations
        m.insert("array_len", StdlibFunction {
            rust_path: "std_lib::array::len".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Int,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_push", StdlibFunction {
            rust_path: "std_lib::array::push".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "item".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ParameterType(0)),

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_pop", StdlibFunction {
            rust_path: "std_lib::array::pop".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_contains", StdlibFunction {
            rust_path: "std_lib::array::contains".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Any)), pass_by_reference: false },
                StdlibParameter { name: "item".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Boolean,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_join", StdlibFunction {
            rust_path: "std_lib::array::join".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Any)), pass_by_reference: false },
                StdlibParameter { name: "separator".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_sort", StdlibFunction {
            rust_path: "std_lib::array::sort".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ParameterType(0)),

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_reverse", StdlibFunction {
            rust_path: "std_lib::array::reverse".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ParameterType(0)),

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_concat", StdlibFunction {
            rust_path: "std_lib::array::concat".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "first".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "second".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ParameterType(0)),

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_get", StdlibFunction {
            rust_path: "std_lib::array::get".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "index".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Any)),
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("get_index", StdlibFunction {
            rust_path: "std_lib::array::get".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "index".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Any)),
            type_inference: Some(TypeInferenceRule::ArrayElementType(0)),

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("len", StdlibFunction {
            rust_path: "std_lib::array::len".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Int,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("push", StdlibFunction {
            rust_path: "std_lib::array::push".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "item".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ParameterType(0)),

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_first", StdlibFunction {
            rust_path: "std_lib::array::first".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Any)), pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Any)),
            type_inference: Some(TypeInferenceRule::ArrayElementType(0)),

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_last", StdlibFunction {
            rust_path: "std_lib::array::last".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Any)), pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Any)),
            type_inference: Some(TypeInferenceRule::ArrayElementType(0)),

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_slice", StdlibFunction {
            rust_path: "std_lib::array::slice".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Any)), pass_by_reference: false },
                StdlibParameter { name: "start".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false },
                StdlibParameter { name: "end".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Any)))),
            type_inference: Some(TypeInferenceRule::ParameterType(0)),

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_take", StdlibFunction {
            rust_path: "std_lib::array::take".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "n".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_skip", StdlibFunction {
            rust_path: "std_lib::array::skip".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "count".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ParameterType(0)),

            diverging: false,
            description: "",
            example: "",
        });

        // Range functions - moved to Array module
        m.insert("array_range", StdlibFunction {
            rust_path: "std_lib::array::array_range".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "start".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false },
                StdlibParameter { name: "end".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)),
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });
        m.insert("array_range_inclusive", StdlibFunction {
            rust_path: "std_lib::array::array_range_inclusive".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "start".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false },
                StdlibParameter { name: "end".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)),
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // New array functions
        m.insert("array_find", StdlibFunction {
            rust_path: "std_lib::array::find".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Int)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_find_last", StdlibFunction {
            rust_path: "std_lib::array::find_last".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Int)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_repeat", StdlibFunction {
            rust_path: "std_lib::array::repeat".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "count".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ArrayOfParameterType(0)),
            diverging: false,
            description: "",
            example: "",
        });

        // NOTE: Removed array_sum/product/average - use reduce instead

        m.insert("array_chunk", StdlibFunction {
            rust_path: "std_lib::array::chunk".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "size".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Any)))),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        // New array functions
        m.insert("array_flatten_deep", StdlibFunction {
            rust_path: "std_lib::array::flatten_deep".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_deduplicate", StdlibFunction {
            rust_path: "std_lib::array::deduplicate".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ParameterType(0)),
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_intersect", StdlibFunction {
            rust_path: "std_lib::array::intersect".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr1".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "arr2".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ParameterType(0)),
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_difference", StdlibFunction {
            rust_path: "std_lib::array::difference".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr1".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "arr2".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ParameterType(0)),
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_union", StdlibFunction {
            rust_path: "std_lib::array::union".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr1".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "arr2".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ParameterType(0)),
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_rotate", StdlibFunction {
            rust_path: "std_lib::array::rotate".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "n".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ParameterType(0)),
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_shuffle", StdlibFunction {
            rust_path: "std_lib::array::shuffle".to_string(),
            crate_deps: vec![CrateDependency::Rand],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ParameterType(0)),
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_rotate_left", StdlibFunction {
            rust_path: "std_lib::array::rotate_left".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "n".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ParameterType(0)),
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_rotate_right", StdlibFunction {
            rust_path: "std_lib::array::rotate_right".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "n".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ParameterType(0)),
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_partition", StdlibFunction {
            rust_path: "std_lib::array::partition".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "predicate".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Any)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("array_group_by", StdlibFunction {
            rust_path: "std_lib::array::group_by".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Array,
            parameters: vec![
                StdlibParameter { name: "arr".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "key_fn".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::Any), Box::new(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Any)))),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        // Integer functions
        m.insert("int_abs", StdlibFunction {
            rust_path: "std_lib::int::abs".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Int,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("int_min", StdlibFunction {
            rust_path: "std_lib::int::min".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Int,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("int_max", StdlibFunction {
            rust_path: "std_lib::int::max".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Int,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("int_pow", StdlibFunction {
            rust_path: "std_lib::int::pow".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Int,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // Float functions
        m.insert("float_abs", StdlibFunction {
            rust_path: "std_lib::float::abs".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Float,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("float_sqrt", StdlibFunction {
            rust_path: "std_lib::float::sqrt".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Float,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("float_pow", StdlibFunction {
            rust_path: "std_lib::float::pow".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Float,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("float_round", StdlibFunction {
            rust_path: "std_lib::float::round".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Float,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("float_floor", StdlibFunction {
            rust_path: "std_lib::float::floor".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Float,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("float_ceil", StdlibFunction {
            rust_path: "std_lib::float::ceil".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Float,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("float_min", StdlibFunction {
            rust_path: "std_lib::float::min".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Float,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("float_max", StdlibFunction {
            rust_path: "std_lib::float::max".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Float,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("float_random", StdlibFunction {
            rust_path: "std_lib::float::random".to_string(),

            crate_deps: vec![CrateDependency::Rand],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Float,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // Math functions
        m.insert("math_abs", StdlibFunction {
            rust_path: "std_lib::float::abs".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_sqrt", StdlibFunction {
            rust_path: "std_lib::float::sqrt".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_pow", StdlibFunction {
            rust_path: "std_lib::float::pow".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "base".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false },
                StdlibParameter { name: "exponent".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_round", StdlibFunction {
            rust_path: "std_lib::float::round".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_floor", StdlibFunction {
            rust_path: "std_lib::float::floor".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_ceil", StdlibFunction {
            rust_path: "std_lib::float::ceil".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_min", StdlibFunction {
            rust_path: "std_lib::float::min".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "first".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false },
                StdlibParameter { name: "second".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_max", StdlibFunction {
            rust_path: "std_lib::float::max".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "first".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false },
                StdlibParameter { name: "second".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_random", StdlibFunction {
            rust_path: "std_lib::float::random".to_string(),

            crate_deps: vec![CrateDependency::Rand],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_divide", StdlibFunction {
            rust_path: "std_lib::math::divide".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "numerator".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "denominator".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Any)),
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // New math functions
        m.insert("math_gcd", StdlibFunction {
            rust_path: "std_lib::math::gcd".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "a".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false },
                StdlibParameter { name: "b".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Int,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_lcm", StdlibFunction {
            rust_path: "std_lib::math::lcm".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "a".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false },
                StdlibParameter { name: "b".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Int,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_factorial", StdlibFunction {
            rust_path: "std_lib::math::factorial".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "n".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Int)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_is_prime", StdlibFunction {
            rust_path: "std_lib::math::is_prime".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "n".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Boolean,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_sigmoid", StdlibFunction {
            rust_path: "std_lib::math::sigmoid".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "x".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_lerp", StdlibFunction {
            rust_path: "std_lib::math::lerp".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "a".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false },
                StdlibParameter { name: "b".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false },
                StdlibParameter { name: "t".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_sin", StdlibFunction {
            rust_path: "std_lib::math::sin".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "x".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_cos", StdlibFunction {
            rust_path: "std_lib::math::cos".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "x".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_tan", StdlibFunction {
            rust_path: "std_lib::math::tan".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "x".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_asin", StdlibFunction {
            rust_path: "std_lib::math::asin".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "x".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Float)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_acos", StdlibFunction {
            rust_path: "std_lib::math::acos".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "x".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Float)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_atan", StdlibFunction {
            rust_path: "std_lib::math::atan".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "x".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_log", StdlibFunction {
            rust_path: "std_lib::math::log".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "x".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Float)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_log10", StdlibFunction {
            rust_path: "std_lib::math::log10".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "x".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Float)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_log2", StdlibFunction {
            rust_path: "std_lib::math::log2".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "x".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Float)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_clamp", StdlibFunction {
            rust_path: "std_lib::math::clamp".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false },
                StdlibParameter { name: "min".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false },
                StdlibParameter { name: "max".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("math_exp", StdlibFunction {
            rust_path: "std_lib::math::exp".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Math,
            parameters: vec![
                StdlibParameter { name: "x".to_string(), param_type: NailDataTypeDescriptor::Float, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Float,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        // Time functions
        m.insert("time_now", StdlibFunction {
            rust_path: "std_lib::time::now".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Time,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Int,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("time_sleep", StdlibFunction {
            rust_path: "std_lib::time::sleep".to_string(),

            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Time,
            parameters: vec![
                StdlibParameter { 
                    name: "seconds".to_string(), 
                    param_type: NailDataTypeDescriptor::Float, 
                    pass_by_reference: false 
                },
            ],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("time_format", StdlibFunction {
            rust_path: "std_lib::time::format".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![("TimeFormat", "nail::std_lib::time")],
            module: StdlibModule::Time,
            parameters: vec![
                StdlibParameter { name: "timestamp".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false },
                StdlibParameter { name: "format".to_string(), param_type: NailDataTypeDescriptor::Enum("TimeFormat".to_string()), pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("time_parse", StdlibFunction {
            rust_path: "std_lib::time::parse".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![("TimeFormat", "nail::std_lib::time")],
            module: StdlibModule::Time,
            parameters: vec![
                StdlibParameter { name: "time_str".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "format".to_string(), param_type: NailDataTypeDescriptor::Enum("TimeFormat".to_string()), pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Int)),
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("time_add_seconds", StdlibFunction {
            rust_path: "std_lib::time::add_seconds".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Time,
            parameters: vec![
                StdlibParameter { name: "timestamp".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false },
                StdlibParameter { name: "seconds".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Int,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("time_diff", StdlibFunction {
            rust_path: "std_lib::time::diff".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Time,
            parameters: vec![
                StdlibParameter { name: "timestamp1".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false },
                StdlibParameter { name: "timestamp2".to_string(), param_type: NailDataTypeDescriptor::Int, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Int,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("time_now_millis", StdlibFunction {
            rust_path: "std_lib::time::now_millis".to_string(),
            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Time,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Int,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        // Environment functions
        m.insert("env_get", StdlibFunction {
            rust_path: "std_lib::env::get".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Env,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("env_set", StdlibFunction {
            rust_path: "std_lib::env::set".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Env,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("env_args", StdlibFunction {
            rust_path: "std_lib::env::args".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Env,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)),
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // Process functions
        m.insert("process_exit", StdlibFunction {
            rust_path: "std_lib::process::exit".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Process,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("process_run", StdlibFunction {
            rust_path: "std_lib::process::run".to_string(),

            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Process,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // Async/spawning functions
        m.insert("spawn", StdlibFunction {
            rust_path: "std_lib::process::spawn".to_string(),
            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Process,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });


        // HTTP client functions
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
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // Database functions (future)
        m.insert("db_connect", StdlibFunction {
            rust_path: "std_lib::db::connect".to_string(),

            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Http, // TODO: Add Database module when available,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("db_query", StdlibFunction {
            rust_path: "std_lib::db::query".to_string(),

            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Http, // TODO: Add Database module when available,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("db_execute", StdlibFunction {
            rust_path: "std_lib::db::execute".to_string(),

            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Http, // TODO: Add Database module when available,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // Crypto functions
        m.insert("crypto_hash_sha256", StdlibFunction {
            rust_path: "std_lib::crypto::hash_sha256".to_string(),
            crate_deps: vec![CrateDependency::Sha2],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Crypto,
            parameters: vec![
                StdlibParameter { name: "input".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("crypto_hash_md5", StdlibFunction {
            rust_path: "std_lib::crypto::hash_md5".to_string(),
            crate_deps: vec![CrateDependency::Md5],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Crypto,
            parameters: vec![
                StdlibParameter { name: "input".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("crypto_uuid_v4", StdlibFunction {
            rust_path: "std_lib::crypto::uuid_v4".to_string(),
            crate_deps: vec![CrateDependency::Uuid],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Crypto,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,
            diverging: false,
            description: "",
            example: "",
        });

        m.insert("crypto_hash", StdlibFunction {
            rust_path: "std_lib::crypto::hash".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String, // TODO: Add Crypto module when available,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("crypto_encrypt", StdlibFunction {
            rust_path: "std_lib::crypto::encrypt".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String, // TODO: Add Crypto module when available,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("crypto_decrypt", StdlibFunction {
            rust_path: "std_lib::crypto::decrypt".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String, // TODO: Add Crypto module when available,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // Path/File system utilities
        m.insert("path_join", StdlibFunction {
            rust_path: "std_lib::path::join".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Path,
            parameters: vec![
                StdlibParameter { name: "base".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false },
                StdlibParameter { name: "path".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("path_exists", StdlibFunction {
            rust_path: "std_lib::path::exists".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Path,
            parameters: vec![
                StdlibParameter { name: "path".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Boolean,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("fs_create_dir", StdlibFunction {
            rust_path: "std_lib::fs::create_dir".to_string(),

            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Fs,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("fs_remove_file", StdlibFunction {
            rust_path: "std_lib::fs::remove_file".to_string(),

            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Fs,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("fs_copy", StdlibFunction {
            rust_path: "std_lib::fs::copy".to_string(),

            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Fs,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("fs_move", StdlibFunction {
            rust_path: "std_lib::fs::move_file".to_string(),

            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Fs,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // Regex functions
        m.insert("regex_match", StdlibFunction {
            rust_path: "std_lib::regex::match_pattern".to_string(),

            crate_deps: vec![CrateDependency::Regex],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("regex_replace", StdlibFunction {
            rust_path: "std_lib::regex::replace".to_string(),

            crate_deps: vec![CrateDependency::Regex],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // Base64 encoding/decoding
        m.insert("base64_encode", StdlibFunction {
            rust_path: "std_lib::encoding::base64_encode".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("base64_decode", StdlibFunction {
            rust_path: "std_lib::encoding::base64_decode".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // URL encoding/decoding
        m.insert("url_encode", StdlibFunction {
            rust_path: "std_lib::encoding::url_encode".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("url_decode", StdlibFunction {
            rust_path: "std_lib::encoding::url_decode".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::String,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // Error handling functions
        m.insert("safe", StdlibFunction {
            rust_path: "std_lib::error::safe".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Error,
            parameters: vec![
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "handler".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ResultInnerType(0)),

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("danger", StdlibFunction {
            rust_path: "std_lib::error::danger".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Error,
            parameters: vec![
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ResultInnerType(0)),

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("expect", StdlibFunction {
            rust_path: "std_lib::error::expect".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Error,
            parameters: vec![
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::ResultInnerType(0)),

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("dangerous", StdlibFunction {
            rust_path: "std_lib::error::dangerous".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Error,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("panic", StdlibFunction {
            rust_path: "std_lib::panic::panic".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Error,
            parameters: vec![
                StdlibParameter { name: "message".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Never,
            type_inference: None,

            diverging: true,
            description: "",
            example: "",
        });

        m.insert("todo", StdlibFunction {
            rust_path: "std_lib::panic::todo".to_string(),

            crate_deps: vec![],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Error,
            parameters: vec![
                StdlibParameter { name: "message".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Never,
            type_inference: None,

            diverging: true,
            description: "",
            example: "",
        });

        // HashMap functions
        m.insert("hashmap_new", StdlibFunction {
            rust_path: "std_lib::hashmap::new".to_string(),

            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::HashMap,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)),
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("hashmap_insert", StdlibFunction {
            rust_path: "std_lib::hashmap::insert".to_string(),

            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::HashMap,
            parameters: vec![
                StdlibParameter { name: "map".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: true },
                StdlibParameter { name: "key".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false },
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("hashmap_get", StdlibFunction {
            rust_path: "std_lib::hashmap::get".to_string(),

            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::HashMap,
            parameters: vec![
                StdlibParameter { name: "map".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: true },
                StdlibParameter { name: "key".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: true }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::HashMapValueType(0)),

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("hashmap_remove", StdlibFunction {
            rust_path: "std_lib::hashmap::remove".to_string(),

            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::HashMap,
            parameters: vec![
                StdlibParameter { name: "map".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: true },
                StdlibParameter { name: "key".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: true }
            ],
            return_type: NailDataTypeDescriptor::Any,
            type_inference: Some(TypeInferenceRule::HashMapValueType(0)),

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("hashmap_contains_key", StdlibFunction {
            rust_path: "std_lib::hashmap::contains_key".to_string(),

            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::HashMap,
            parameters: vec![
                StdlibParameter { name: "map".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: true },
                StdlibParameter { name: "key".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: true }
            ],
            return_type: NailDataTypeDescriptor::Boolean,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("hashmap_len", StdlibFunction {
            rust_path: "std_lib::hashmap::len".to_string(),

            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::HashMap,
            parameters: vec![
                StdlibParameter { name: "map".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: true }
            ],
            return_type: NailDataTypeDescriptor::Int,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("hashmap_is_empty", StdlibFunction {
            rust_path: "std_lib::hashmap::is_empty".to_string(),

            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::HashMap,
            parameters: vec![
                StdlibParameter { name: "map".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: true }
            ],
            return_type: NailDataTypeDescriptor::Boolean,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("hashmap_clear", StdlibFunction {
            rust_path: "std_lib::hashmap::clear".to_string(),

            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::HashMap,
            parameters: vec![
                StdlibParameter { name: "map".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: true }
            ],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("hashmap_keys", StdlibFunction {
            rust_path: "std_lib::hashmap::keys".to_string(),

            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::HashMap,
            parameters: vec![
                StdlibParameter { name: "map".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: true }
            ],
            return_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)),
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("hashmap_values", StdlibFunction {
            rust_path: "std_lib::hashmap::values".to_string(),

            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::HashMap,
            parameters: vec![
                StdlibParameter { name: "map".to_string(), param_type: NailDataTypeDescriptor::Any, pass_by_reference: true }
            ],
            return_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)),
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("hashmap_to_vec", StdlibFunction {
            rust_path: "std_lib::hashmap::to_vec".to_string(),

            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::HashMap,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("hashmap_from_vec", StdlibFunction {
            rust_path: "std_lib::hashmap::from_vec".to_string(),

            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::HashMap,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("hashmap_entry_or_insert", StdlibFunction {
            rust_path: "std_lib::hashmap::entry_or_insert".to_string(),

            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::HashMap,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("hashmap_merge", StdlibFunction {
            rust_path: "std_lib::hashmap::merge".to_string(),

            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::HashMap,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        // Markdown functions
        m.insert("markdown_to_html", StdlibFunction {
            rust_path: "std_lib::markdown::to_html".to_string(),

            crate_deps: vec![CrateDependency::Pulldown],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Markdown,
            parameters: vec![
                StdlibParameter { name: "markdown".to_string(), param_type: NailDataTypeDescriptor::String, pass_by_reference: false }
            ],
            return_type: NailDataTypeDescriptor::String,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m.insert("markdown_to_html_with_options", StdlibFunction {
            rust_path: "std_lib::markdown::to_html_with_options".to_string(),

            crate_deps: vec![CrateDependency::Pulldown],
            struct_derives: vec![],
            custom_type_imports: vec![],
            module: StdlibModule::Markdown,
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
            type_inference: None,

            diverging: false,
            description: "",
            example: "",
        });

        m
    };
}

/// Check if a function name is a stdlib function
pub fn is_stdlib_function(name: &str) -> bool {
    STDLIB_FUNCTIONS.contains_key(name)
}

/// Get stdlib function info
pub fn get_stdlib_function(name: &str) -> Option<&'static StdlibFunction> {
    STDLIB_FUNCTIONS.get(name)
}

/// Information about a stdlib struct/type
#[derive(Clone, Debug)]
pub struct StdlibTypeInfo {
    pub name: String,
    pub fields: HashMap<String, NailDataTypeDescriptor>,
}

lazy_static! {
    /// Registry of all stdlib types and their field information
    pub static ref STDLIB_TYPES: HashMap<&'static str, StdlibTypeInfo> = {
        let mut m = HashMap::new();
        
        // HTTP_Route struct
        m.insert("HTTP_Route", StdlibTypeInfo {
            name: "HTTP_Route".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("path".to_string(), NailDataTypeDescriptor::String);
                fields.insert("content".to_string(), NailDataTypeDescriptor::String);
                fields.insert("content_type".to_string(), NailDataTypeDescriptor::String);
                fields.insert("status_code".to_string(), NailDataTypeDescriptor::Int);
                fields
            }
        });
        
        // HTTP_Response struct
        m.insert("HTTP_Response", StdlibTypeInfo {
            name: "HTTP_Response".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("status".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("body".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });
        
        m
    };
}

/// Get all stdlib type names (structs/enums defined in stdlib)
pub fn get_stdlib_type_names() -> HashSet<String> {
    STDLIB_TYPES.keys().map(|k| k.to_string()).collect()
}

/// Get field type for a stdlib struct
pub fn get_stdlib_struct_field_type(struct_name: &str, field_name: &str) -> Option<NailDataTypeDescriptor> {
    STDLIB_TYPES.get(struct_name)
        .and_then(|info| info.fields.get(field_name))
        .cloned()
}

/// Check if a struct is a stdlib struct
pub fn is_stdlib_struct(name: &str) -> bool {
    STDLIB_TYPES.contains_key(name)
}

