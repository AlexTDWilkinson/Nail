use lazy_static::lazy_static;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CrateDependency {
    Axum,
    Tokio,
    SerdeJson,
    Serde,
    Regex,
    Rand,
    DashMap,
}

impl CrateDependency {
    pub fn to_cargo_dep(&self) -> &'static str {
        match self {
            CrateDependency::Axum => "axum",
            CrateDependency::Tokio => "tokio",
            CrateDependency::SerdeJson => "serde_json",
            CrateDependency::Serde => "serde",
            CrateDependency::Regex => "regex",
            CrateDependency::Rand => "rand",
            CrateDependency::DashMap => "dashmap",
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
    ArrayFunctional,
    Math,
    Time,
    Env,
    Process,
    Path,
    Error,
    HashMap,
    IO,
    Print,
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
            StdlibModule::ArrayFunctional => "std_lib::array_functional",
            StdlibModule::Math => "std_lib::math",
            StdlibModule::Time => "std_lib::time",
            StdlibModule::Env => "std_lib::env",
            StdlibModule::Process => "std_lib::process",
            StdlibModule::Path => "std_lib::path",
            StdlibModule::Error => "std_lib::error",
            StdlibModule::HashMap => "std_lib::hashmap",
            StdlibModule::IO => "std_lib::io",
            StdlibModule::Print => "std_lib::print",
        }
    }
}

#[derive(Clone, Debug)]
pub struct StdlibFunction {
    /// The Rust path to call this function (e.g., "std_lib::http::http_server_start")
    pub rust_path: String,
    /// Whether this function is async
    pub is_async: bool,
    /// External crate dependencies required for this function
    pub crate_deps: Vec<CrateDependency>,
    /// Additional derives needed for structs/enums when this function is used
    pub struct_derives: Vec<StructDerive>,
    /// The module group this function belongs to
    pub module: StdlibModule,
}

lazy_static! {
    pub static ref STDLIB_FUNCTIONS: HashMap<&'static str, StdlibFunction> = {
        let mut m = HashMap::new();

        // HTTP functions
        m.insert("http_server_start", StdlibFunction {
            rust_path: "std_lib::http::http_server_start".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Axum, CrateDependency::Tokio],
            struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
            module: StdlibModule::Http,
        });

        m.insert("http_server_route", StdlibFunction {
            rust_path: "std_lib::http::http_server_route".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Axum, CrateDependency::Tokio],
            struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
            module: StdlibModule::Http,
        });

        // File system functions (future)
        m.insert("fs_read", StdlibFunction {
            rust_path: "std_lib::fs::read_file".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            module: StdlibModule::Fs,
        });

        m.insert("fs_write", StdlibFunction {
            rust_path: "std_lib::fs::write_file".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            module: StdlibModule::Fs,
        });

        // JSON functions (future)
        m.insert("json_parse", StdlibFunction {
            rust_path: "std_lib::json::parse".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::SerdeJson, CrateDependency::Serde],
            struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
            module: StdlibModule::Json,
        });

        m.insert("json_stringify", StdlibFunction {
            rust_path: "std_lib::json::stringify".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::SerdeJson, CrateDependency::Serde],
            struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
            module: StdlibModule::Json,
        });

        // Type conversion functions
        m.insert("string_from", StdlibFunction {
            rust_path: "std_lib::string::from".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String,
        });

        m.insert("int_from", StdlibFunction {
            rust_path: "std_lib::int::from".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Int,
        });

        m.insert("float_from", StdlibFunction {
            rust_path: "std_lib::float::from".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Float,
        });

        // IO functions
        m.insert("print", StdlibFunction {
            rust_path: "std_lib::print::print".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Print,
        });

        m.insert("eprintln", StdlibFunction {
            rust_path: "eprintln!".to_string(),  // Macro, needs special handling
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String,
        });

        m.insert("print_no_newline", StdlibFunction {
            rust_path: "std_lib::print::print_no_newline".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Print,
        });

        m.insert("io_read_line", StdlibFunction {
            rust_path: "std_lib::io::read_line".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::IO,
        });

        m.insert("io_read_line_prompt", StdlibFunction {
            rust_path: "std_lib::io::read_line_prompt".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::IO,
        });

        m.insert("io_read_int", StdlibFunction {
            rust_path: "std_lib::io::read_int".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::IO,
        });

        m.insert("io_read_int_prompt", StdlibFunction {
            rust_path: "std_lib::io::read_int_prompt".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::IO,
        });

        m.insert("io_read_float", StdlibFunction {
            rust_path: "std_lib::io::read_float".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::IO,
        });

        m.insert("io_read_float_prompt", StdlibFunction {
            rust_path: "std_lib::io::read_float_prompt".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::IO,
        });

        m.insert("print_clear_screen", StdlibFunction {
            rust_path: "std_lib::print::print_clear_screen".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Print,
        });

        m.insert("print_debug", StdlibFunction {
            rust_path: "std_lib::print::print_debug".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Print,
        });

        // String manipulation
        m.insert("string_concat", StdlibFunction {
            rust_path: "std_lib::string::concat".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String,
        });

        m.insert("string_split", StdlibFunction {
            rust_path: "std_lib::string::split".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String,
        });

        m.insert("string_trim", StdlibFunction {
            rust_path: "std_lib::string::trim".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String,
        });

        m.insert("string_contains", StdlibFunction {
            rust_path: "std_lib::string::contains".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String,
        });

        m.insert("string_replace", StdlibFunction {
            rust_path: "std_lib::string::replace".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String,
        });

        m.insert("string_len", StdlibFunction {
            rust_path: "std_lib::string::len".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String,
        });

        m.insert("string_to_uppercase", StdlibFunction {
            rust_path: "std_lib::string::to_uppercase".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String,
        });

        m.insert("string_to_lowercase", StdlibFunction {
            rust_path: "std_lib::string::to_lowercase".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String,
        });

        // Array operations
        m.insert("array_len", StdlibFunction {
            rust_path: "std_lib::array::len".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Array,
        });

        m.insert("array_push", StdlibFunction {
            rust_path: "std_lib::array::push".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Array,
        });

        m.insert("array_pop", StdlibFunction {
            rust_path: "std_lib::array::pop".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Array,
        });

        m.insert("array_contains", StdlibFunction {
            rust_path: "std_lib::array::contains".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Array,
        });

        m.insert("array_join", StdlibFunction {
            rust_path: "std_lib::array::join".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Array,
        });

        m.insert("array_sort", StdlibFunction {
            rust_path: "std_lib::array::sort".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Array,
        });

        m.insert("array_reverse", StdlibFunction {
            rust_path: "std_lib::array::reverse".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Array,
        });
        
        m.insert("array_get", StdlibFunction {
            rust_path: "std_lib::array::get".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Array,
        });
        
        m.insert("array_first", StdlibFunction {
            rust_path: "std_lib::array::first".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Array,
        });
        
        m.insert("array_last", StdlibFunction {
            rust_path: "std_lib::array::last".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Array,
        });
        
        m.insert("array_slice", StdlibFunction {
            rust_path: "std_lib::array::slice".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Array,
        });
        
        m.insert("array_take", StdlibFunction {
            rust_path: "std_lib::array::take".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Array,
        });
        
        m.insert("array_skip", StdlibFunction {
            rust_path: "std_lib::array::skip".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Array,
        });

        // Functional array operations
        m.insert("map_int", StdlibFunction {
            rust_path: "std_lib::array_functional::map_int".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio, ],
            struct_derives: vec![],
            module: StdlibModule::ArrayFunctional,
        });
        m.insert("map_float", StdlibFunction {
            rust_path: "std_lib::array_functional::map_float".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio, ],
            struct_derives: vec![],
            module: StdlibModule::ArrayFunctional,
        });
        m.insert("map_string", StdlibFunction {
            rust_path: "std_lib::array_functional::map_string".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio, ],
            struct_derives: vec![],
            module: StdlibModule::ArrayFunctional,
        });
        m.insert("filter_int", StdlibFunction {
            rust_path: "std_lib::array_functional::filter_int".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio, ],
            struct_derives: vec![],
            module: StdlibModule::ArrayFunctional,
        });
        m.insert("filter_float", StdlibFunction {
            rust_path: "std_lib::array_functional::filter_float".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio, ],
            struct_derives: vec![],
            module: StdlibModule::ArrayFunctional,
        });
        m.insert("filter_string", StdlibFunction {
            rust_path: "std_lib::array_functional::filter_string".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio, ],
            struct_derives: vec![],
            module: StdlibModule::ArrayFunctional,
        });
        m.insert("reduce_int", StdlibFunction {
            rust_path: "std_lib::array_functional::reduce_int".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio, ],
            struct_derives: vec![],
            module: StdlibModule::ArrayFunctional,
        });
        m.insert("reduce_float", StdlibFunction {
            rust_path: "std_lib::array_functional::reduce_float".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio, ],
            struct_derives: vec![],
            module: StdlibModule::ArrayFunctional,
        });
        m.insert("reduce_string", StdlibFunction {
            rust_path: "std_lib::array_functional::reduce_string".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio, ],
            struct_derives: vec![],
            module: StdlibModule::ArrayFunctional,
        });
        m.insert("reduce_struct", StdlibFunction {
            rust_path: "std_lib::array_functional::reduce_struct".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio, ],
            struct_derives: vec![],
            module: StdlibModule::ArrayFunctional,
        });
        m.insert("each_int", StdlibFunction {
            rust_path: "std_lib::array_functional::each_int".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio, ],
            struct_derives: vec![],
            module: StdlibModule::ArrayFunctional,
        });
        m.insert("each_float", StdlibFunction {
            rust_path: "std_lib::array_functional::each_float".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio, ],
            struct_derives: vec![],
            module: StdlibModule::ArrayFunctional,
        });
        m.insert("each_string", StdlibFunction {
            rust_path: "std_lib::array_functional::each_string".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio, ],
            struct_derives: vec![],
            module: StdlibModule::ArrayFunctional,
        });
        m.insert("range", StdlibFunction {
            rust_path: "std_lib::array_functional::range".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::ArrayFunctional,
        });
        m.insert("range_exclusive", StdlibFunction {
            rust_path: "std_lib::array_functional::range_exclusive".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::ArrayFunctional,
        });

        // Integer functions
        m.insert("int_abs", StdlibFunction {
            rust_path: "std_lib::int::abs".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Int,
        });

        m.insert("int_min", StdlibFunction {
            rust_path: "std_lib::int::min".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Int,
        });

        m.insert("int_max", StdlibFunction {
            rust_path: "std_lib::int::max".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Int,
        });

        m.insert("int_pow", StdlibFunction {
            rust_path: "std_lib::int::pow".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Int,
        });

        // Float functions
        m.insert("float_abs", StdlibFunction {
            rust_path: "std_lib::float::abs".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Float,
        });

        m.insert("float_sqrt", StdlibFunction {
            rust_path: "std_lib::float::sqrt".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Float,
        });

        m.insert("float_pow", StdlibFunction {
            rust_path: "std_lib::float::pow".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Float,
        });

        m.insert("float_round", StdlibFunction {
            rust_path: "std_lib::float::round".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Float,
        });

        m.insert("float_floor", StdlibFunction {
            rust_path: "std_lib::float::floor".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Float,
        });

        m.insert("float_ceil", StdlibFunction {
            rust_path: "std_lib::float::ceil".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Float,
        });

        m.insert("float_min", StdlibFunction {
            rust_path: "std_lib::float::min".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Float,
        });

        m.insert("float_max", StdlibFunction {
            rust_path: "std_lib::float::max".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Float,
        });

        m.insert("float_random", StdlibFunction {
            rust_path: "std_lib::float::random".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::Rand],
            struct_derives: vec![],
            module: StdlibModule::Float,
        });

        // Math functions
        m.insert("math_abs", StdlibFunction {
            rust_path: "std_lib::math::abs".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Math,
        });

        m.insert("math_sqrt", StdlibFunction {
            rust_path: "std_lib::math::sqrt".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Math,
        });

        m.insert("math_pow", StdlibFunction {
            rust_path: "std_lib::math::pow".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Math,
        });

        m.insert("math_round", StdlibFunction {
            rust_path: "std_lib::math::round".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Math,
        });

        m.insert("math_floor", StdlibFunction {
            rust_path: "std_lib::math::floor".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Math,
        });

        m.insert("math_ceil", StdlibFunction {
            rust_path: "std_lib::math::ceil".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Math,
        });

        m.insert("math_min", StdlibFunction {
            rust_path: "std_lib::math::min".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Math,
        });

        m.insert("math_max", StdlibFunction {
            rust_path: "std_lib::math::max".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Math,
        });

        m.insert("math_random", StdlibFunction {
            rust_path: "std_lib::math::random".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::Rand],
            struct_derives: vec![],
            module: StdlibModule::Math,
        });

        // Time functions
        m.insert("time_now", StdlibFunction {
            rust_path: "std_lib::time::now".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Time,
        });

        m.insert("time_sleep", StdlibFunction {
            rust_path: "std_lib::time::sleep".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            module: StdlibModule::Time,
        });

        m.insert("time_format", StdlibFunction {
            rust_path: "std_lib::time::format".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Time,
        });

        // Environment functions
        m.insert("env_get", StdlibFunction {
            rust_path: "std_lib::env::get".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Env,
        });

        m.insert("env_set", StdlibFunction {
            rust_path: "std_lib::env::set".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Env,
        });

        m.insert("env_args", StdlibFunction {
            rust_path: "std_lib::env::args".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Env,
        });

        // Process functions
        m.insert("process_exit", StdlibFunction {
            rust_path: "std_lib::process::exit".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Process,
        });

        m.insert("process_run", StdlibFunction {
            rust_path: "std_lib::process::run".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            module: StdlibModule::Process,
        });

        // HTTP client functions
        m.insert("http_get", StdlibFunction {
            rust_path: "std_lib::http::get".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Axum, CrateDependency::Tokio],
            struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
            module: StdlibModule::Http,
        });

        m.insert("http_post", StdlibFunction {
            rust_path: "std_lib::http::post".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Axum, CrateDependency::Tokio],
            struct_derives: vec![StructDerive::SerdeSerialize, StructDerive::SerdeDeserialize],
            module: StdlibModule::Http,
        });

        // Database functions (future)
        m.insert("db_connect", StdlibFunction {
            rust_path: "std_lib::db::connect".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            module: StdlibModule::Http, // TODO: Add Database module when available
        });

        m.insert("db_query", StdlibFunction {
            rust_path: "std_lib::db::query".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            module: StdlibModule::Http, // TODO: Add Database module when available
        });

        m.insert("db_execute", StdlibFunction {
            rust_path: "std_lib::db::execute".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            module: StdlibModule::Http, // TODO: Add Database module when available
        });

        // Crypto functions
        m.insert("crypto_hash", StdlibFunction {
            rust_path: "std_lib::crypto::hash".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String, // TODO: Add Crypto module when available
        });

        m.insert("crypto_encrypt", StdlibFunction {
            rust_path: "std_lib::crypto::encrypt".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String, // TODO: Add Crypto module when available
        });

        m.insert("crypto_decrypt", StdlibFunction {
            rust_path: "std_lib::crypto::decrypt".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String, // TODO: Add Crypto module when available
        });

        // Path/File system utilities
        m.insert("path_join", StdlibFunction {
            rust_path: "std_lib::path::join".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Path,
        });

        m.insert("path_exists", StdlibFunction {
            rust_path: "std_lib::path::exists".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Path,
        });

        m.insert("fs_create_dir", StdlibFunction {
            rust_path: "std_lib::fs::create_dir".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            module: StdlibModule::Fs,
        });

        m.insert("fs_remove_file", StdlibFunction {
            rust_path: "std_lib::fs::remove_file".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            module: StdlibModule::Fs,
        });

        m.insert("fs_copy", StdlibFunction {
            rust_path: "std_lib::fs::copy".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            module: StdlibModule::Fs,
        });

        m.insert("fs_move", StdlibFunction {
            rust_path: "std_lib::fs::move_file".to_string(),
            is_async: true,
            crate_deps: vec![CrateDependency::Tokio],
            struct_derives: vec![],
            module: StdlibModule::Fs,
        });

        // Regex functions
        m.insert("regex_match", StdlibFunction {
            rust_path: "std_lib::regex::match_pattern".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::Regex],
            struct_derives: vec![],
            module: StdlibModule::String,
        });

        m.insert("regex_replace", StdlibFunction {
            rust_path: "std_lib::regex::replace".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::Regex],
            struct_derives: vec![],
            module: StdlibModule::String,
        });

        // Base64 encoding/decoding
        m.insert("base64_encode", StdlibFunction {
            rust_path: "std_lib::encoding::base64_encode".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String,
        });

        m.insert("base64_decode", StdlibFunction {
            rust_path: "std_lib::encoding::base64_decode".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String,
        });

        // URL encoding/decoding
        m.insert("url_encode", StdlibFunction {
            rust_path: "std_lib::encoding::url_encode".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String,
        });

        m.insert("url_decode", StdlibFunction {
            rust_path: "std_lib::encoding::url_decode".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::String,
        });

        // Error handling functions
        m.insert("safe", StdlibFunction {
            rust_path: "std_lib::error::safe".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Error,
        });

        m.insert("dangerous", StdlibFunction {
            rust_path: "std_lib::error::dangerous".to_string(),
            is_async: false,
            crate_deps: vec![],
            struct_derives: vec![],
            module: StdlibModule::Error,
        });

        // HashMap functions
        m.insert("hashmap_new", StdlibFunction {
            rust_path: "std_lib::hashmap::new".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            module: StdlibModule::HashMap,
        });

        m.insert("hashmap_insert", StdlibFunction {
            rust_path: "std_lib::hashmap::insert".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            module: StdlibModule::HashMap,
        });

        m.insert("hashmap_get", StdlibFunction {
            rust_path: "std_lib::hashmap::get".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            module: StdlibModule::HashMap,
        });

        m.insert("hashmap_remove", StdlibFunction {
            rust_path: "std_lib::hashmap::remove".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            module: StdlibModule::HashMap,
        });

        m.insert("hashmap_contains_key", StdlibFunction {
            rust_path: "std_lib::hashmap::contains_key".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            module: StdlibModule::HashMap,
        });

        m.insert("hashmap_len", StdlibFunction {
            rust_path: "std_lib::hashmap::len".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            module: StdlibModule::HashMap,
        });

        m.insert("hashmap_is_empty", StdlibFunction {
            rust_path: "std_lib::hashmap::is_empty".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            module: StdlibModule::HashMap,
        });

        m.insert("hashmap_clear", StdlibFunction {
            rust_path: "std_lib::hashmap::clear".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            module: StdlibModule::HashMap,
        });

        m.insert("hashmap_keys", StdlibFunction {
            rust_path: "std_lib::hashmap::keys".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            module: StdlibModule::HashMap,
        });

        m.insert("hashmap_values", StdlibFunction {
            rust_path: "std_lib::hashmap::values".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            module: StdlibModule::HashMap,
        });

        m.insert("hashmap_to_vec", StdlibFunction {
            rust_path: "std_lib::hashmap::to_vec".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            module: StdlibModule::HashMap,
        });

        m.insert("hashmap_from_vec", StdlibFunction {
            rust_path: "std_lib::hashmap::from_vec".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            module: StdlibModule::HashMap,
        });

        m.insert("hashmap_entry_or_insert", StdlibFunction {
            rust_path: "std_lib::hashmap::entry_or_insert".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            module: StdlibModule::HashMap,
        });

        m.insert("hashmap_merge", StdlibFunction {
            rust_path: "std_lib::hashmap::merge".to_string(),
            is_async: false,
            crate_deps: vec![CrateDependency::DashMap],
            struct_derives: vec![],
            module: StdlibModule::HashMap,
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
