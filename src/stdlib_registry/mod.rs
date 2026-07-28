//! Nail standard library registry - the single source of truth for every
//! stdlib function's metadata (Rust path, crate dependencies, signature).
//!
//! One file per module lives in this directory; each exposes a `register`
//! function that inserts its entries into the shared map. To add a new
//! library: create the module file, declare it below, and call its
//! `register` in STDLIB_FUNCTIONS. New crates go in the crate_dependencies!
//! table; heavy crates get a `feature:` so the nail crate stays fast to
//! build (see CrateDependency::nail_feature).

pub(crate) use crate::lexer::NailDataTypeDescriptor;
use lazy_static::lazy_static;
pub(crate) use std::collections::HashMap;
use std::collections::HashSet;

/// Shorthand for NailDataTypeDescriptor values in registry entries, mirroring
/// Nail's own type syntax:
///   i, f, s, b, v (void), any, never    - primitive types
///   T (any other bare ident)            - type variable
///   [X]                                 - array of X
///   (X!e)                               - result of X
///   (h K V)                             - hashmap from K to V
/// Compose by nesting, e.g. ([s]!e) is a result of an array of strings. A
/// result of a hashmap must parenthesize the hashmap: ((h s s)!e).
macro_rules! nail_type {
    (i) => { NailDataTypeDescriptor::Int };
    (f) => { NailDataTypeDescriptor::Float };
    (s) => { NailDataTypeDescriptor::String };
    (b) => { NailDataTypeDescriptor::Boolean };
    (v) => { NailDataTypeDescriptor::Void };
    (any) => { NailDataTypeDescriptor::Any };
    (never) => { NailDataTypeDescriptor::Never };
    ([ $($inner:tt)+ ]) => { NailDataTypeDescriptor::Array(Box::new(nail_type!($($inner)+))) };
    ((h $key:tt $value:tt)) => { NailDataTypeDescriptor::HashMap(Box::new(nail_type!($key)), Box::new(nail_type!($value))) };
    (($inner:tt !e)) => { NailDataTypeDescriptor::Result(Box::new(nail_type!($inner))) };
    ($type_var:ident) => { NailDataTypeDescriptor::TypeVar(stringify!($type_var).to_string()) };
}

/// Builds one StdlibParameter. Wrap the type in (& ...) for pass-by-reference:
///   nail_param!(input: s)         - by value
///   nail_param!(map: (&(h s s)))  - by reference
macro_rules! nail_param {
    ($pname:ident: (& $($ptype:tt)+)) => {
        StdlibParameter { name: stringify!($pname).to_string(), param_type: nail_type!($($ptype)+), pass_by_reference: true }
    };
    ($pname:ident: $ptype:tt) => {
        StdlibParameter { name: stringify!($pname).to_string(), param_type: nail_type!($ptype), pass_by_reference: false }
    };
}

/// Registers the common case of a stdlib function - no struct derives, no
/// custom type imports, not diverging - in one entry per function:
///
/// simple_fns! { m, String:
///     "string_trim" => "std_lib::string::trim", (input: s) -> s,
///         "Removes leading and trailing whitespace.",
///         "trimmed:s = string_trim(`  hi  `);";
/// }
///
/// Crate dependencies go in square brackets after the Nail name:
///     "regex_match" [Regex] => "std_lib::regex::match_pattern", ...
///
/// Entries needing struct_derives, custom_type_imports, or diverging use the
/// full StdlibFunction struct literal instead.
macro_rules! simple_fns {
    ($m:ident, $module:ident:
        $( $name:literal $([$($dep:ident),+])? => $path:literal, ($($pname:ident: $ptype:tt),*) -> $ret:tt, $desc:literal, $example:literal; )*
    ) => {
        $(
            $m.insert($name, StdlibFunction {
                rust_path: $path.to_string(),
                crate_deps: vec![ $($(CrateDependency::$dep),+)? ],
                struct_derives: vec![],
                custom_type_imports: vec![],
                module: StdlibModule::$module,
                parameters: vec![ $( nail_param!($pname: $ptype) ),* ],
                return_type: nail_type!($ret),
                diverging: false,
                description: $desc,
                example: $example,
            });
        )*
    };
}

mod args;
mod array;
mod compress;
mod crypto;
mod database;
mod duckdb;
mod env;
mod error;
mod float;
mod fs;
mod hashmap;
mod http;
mod int;
mod io;
mod json;
mod markdown;
mod math;
mod panic;
mod path;
mod print;
mod process;
mod regex;
mod string;
mod time;
mod url;

/// Defines the CrateDependency enum and all its lookup methods from a single
/// table so adding a crate is one line instead of four match arms.
macro_rules! crate_dependencies {
    ($($variant:ident => { cargo: $cargo:literal, name: $name:literal, import: $import:literal $(, feature: $feature:literal)? }),* $(,)?) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub enum CrateDependency {
            $($variant,)*
        }

        impl CrateDependency {
            /// Exact line for a generated Cargo.toml [dependencies] section
            pub fn to_cargo_dep(&self) -> &'static str {
                match self { $(CrateDependency::$variant => $cargo,)* }
            }

            pub fn to_crate_name(&self) -> &'static str {
                match self { $(CrateDependency::$variant => $name,)* }
            }

            pub fn to_rust_import(&self) -> &'static str {
                match self { $(CrateDependency::$variant => $import,)* }
            }

            /// Cargo feature of the `nail` crate that must be enabled for this
            /// dependency. Heavy dependencies are optional in the nail crate so
            /// the compiler itself builds fast; generated projects enable the
            /// feature only when a function needing it is actually used.
            pub fn nail_feature(&self) -> Option<&'static str> {
                match self { $(CrateDependency::$variant => crate_dependencies!(@feature $($feature)?),)* }
            }
        }
    };
    (@feature $feature:literal) => { Some($feature) };
    (@feature) => { None };
}

crate_dependencies! {
    Axum => { cargo: "axum = \"0.7\"", name: "axum", import: "use axum;" },
    Tokio => { cargo: "tokio = { version = \"1\", features = [\"rt-multi-thread\", \"macros\"] }", name: "tokio", import: "use tokio;" },
    SerdeJson => { cargo: "serde_json = \"1.0\"", name: "serde_json", import: "use serde_json;" },
    Serde => { cargo: "serde = { version = \"1.0\", features = [\"derive\"] }", name: "serde", import: "use serde;" },
    Regex => { cargo: "regex = \"1.10\"", name: "regex", import: "use regex;" },
    Rand => { cargo: "rand = \"0.8\"", name: "rand", import: "use rand;" },
    DashMap => { cargo: "dashmap = \"6.1.0\"", name: "dashmap", import: "use dashmap;" },
    Pulldown => { cargo: "pulldown-cmark = \"0.9\"", name: "pulldown-cmark", import: "use pulldown_cmark;" },
    Reqwest => { cargo: "reqwest = \"0.11\"", name: "reqwest", import: "use reqwest;" },
    Sha2 => { cargo: "sha2 = \"0.10\"", name: "sha2", import: "use sha2;" },
    Md5 => { cargo: "md5 = \"0.7\"", name: "md5", import: "use md5;" },
    Uuid => { cargo: "uuid = { version = \"1.0\", features = [\"v4\"] }", name: "uuid", import: "use uuid;" },
    UrlEncoding => { cargo: "urlencoding = \"2.1\"", name: "urlencoding", import: "use urlencoding;" },
    Flate2 => { cargo: "flate2 = \"1.0\"", name: "flate2", import: "use flate2;" },
    Base64 => { cargo: "base64 = \"0.21\"", name: "base64", import: "use base64;" },
    Rusqlite => { cargo: "rusqlite = { version = \"0.31\", features = [\"bundled\"] }", name: "rusqlite", import: "use rusqlite;" },
    Duckdb => { cargo: "duckdb = { version = \"1.10504.0\", features = [\"bundled\"] }", name: "duckdb", import: "use duckdb;", feature: "duckdb" },
}

/// Defines the StdlibModule enum and its runtime module path from one table.
macro_rules! stdlib_modules {
    ($($variant:ident => $path:literal),* $(,)?) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub enum StdlibModule {
            $($variant,)*
        }

        impl StdlibModule {
            pub fn to_module_path(&self) -> &'static str {
                match self { $(StdlibModule::$variant => $path,)* }
            }
        }
    };
}

stdlib_modules! {
    Http => "std_lib::http",
    Fs => "std_lib::fs",
    Json => "std_lib::json",
    String => "std_lib::string",
    Int => "std_lib::int",
    Float => "std_lib::float",
    Array => "std_lib::array",
    Math => "std_lib::math",
    Time => "std_lib::time",
    Env => "std_lib::env",
    Process => "std_lib::process",
    Path => "std_lib::path",
    Error => "std_lib::error",
    Panic => "std_lib::panic",
    HashMap => "std_lib::hashmap",
    IO => "std_lib::io",
    Print => "std_lib::print",
    Markdown => "std_lib::markdown",
    Crypto => "std_lib::crypto",
    Regex => "std_lib::regex",
    Args => "std_lib::args",
    Url => "std_lib::url",
    Compress => "std_lib::compress",
    Database => "std_lib::database",
    Duckdb => "std_lib::duckdb",
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

#[derive(Clone, Debug)]
pub struct StdlibParameter {
    pub name: String,
    pub param_type: NailDataTypeDescriptor,
    pub pass_by_reference: bool,
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
        args::register(&mut m);
        array::register(&mut m);
        compress::register(&mut m);
        crypto::register(&mut m);
        database::register(&mut m);
        duckdb::register(&mut m);
        env::register(&mut m);
        error::register(&mut m);
        float::register(&mut m);
        fs::register(&mut m);
        hashmap::register(&mut m);
        http::register(&mut m);
        int::register(&mut m);
        io::register(&mut m);
        json::register(&mut m);
        markdown::register(&mut m);
        math::register(&mut m);
        panic::register(&mut m);
        path::register(&mut m);
        print::register(&mut m);
        process::register(&mut m);
        regex::register(&mut m);
        string::register(&mut m);
        time::register(&mut m);
        url::register(&mut m);
        m
    };
}

/// Built-in functions the compiler must treat specially. The checker and
/// transpiler dispatch on these kinds — never on function names — so renaming
/// or adding an intrinsic is a registry-only change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intrinsic {
    /// print(...) — variadic, accepts any types
    Print,
    /// e(message) — constructs an error value inside a function returning T!e
    ErrorConstructor,
    /// safe(expr, handler) — unwraps a result, invoking the :e handler on error
    SafeUnwrap,
    /// danger(expr) / expect(expr) — unwraps a result, panicking on error
    PanicUnwrap,
}

/// Which intrinsic, if any, a function name refers to.
pub fn get_intrinsic(name: &str) -> Option<Intrinsic> {
    match name {
        "print" => Some(Intrinsic::Print),
        "e" => Some(Intrinsic::ErrorConstructor),
        "safe" => Some(Intrinsic::SafeUnwrap),
        "danger" | "expect" => Some(Intrinsic::PanicUnwrap),
        _ => None,
    }
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
        
        // DB_SQLite struct
        m.insert("DB_SQLite", StdlibTypeInfo {
            name: "DB_SQLite".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("handle".to_string(), NailDataTypeDescriptor::String);
                fields.insert("path".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });
        
        // DB_Result struct
        m.insert("DB_Result", StdlibTypeInfo {
            name: "DB_Result".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("rows_affected".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("last_insert_id".to_string(), NailDataTypeDescriptor::Int);
                fields
            }
        });

        // DB_DuckDB struct
        m.insert("DB_DuckDB", StdlibTypeInfo {
            name: "DB_DuckDB".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("handle".to_string(), NailDataTypeDescriptor::String);
                fields.insert("path".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        // DB_DuckDB_Result struct (DuckDB has no rowids, so no last_insert_id)
        m.insert("DB_DuckDB_Result", StdlibTypeInfo {
            name: "DB_DuckDB_Result".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("rows_affected".to_string(), NailDataTypeDescriptor::Int);
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

#[cfg(test)]
mod stdlib_types_drift_tests {
    //! Guards against drift between the hand-written STDLIB_TYPES field maps
    //! and the real Rust structs in parser/std_lib. For each registered type,
    //! a JSON object is synthesized from the registry's field map and must
    //! round-trip through the real struct: deserialization fails if the
    //! registry is missing a field or has a wrong type; the reserialized key
    //! set differs if the registry lists a field the struct doesn't have.

    use super::*;
    use serde::{de::DeserializeOwned, Serialize};

    fn dummy_json_for(nail_type: &NailDataTypeDescriptor) -> serde_json::Value {
        match nail_type {
            NailDataTypeDescriptor::Int => serde_json::json!(0),
            NailDataTypeDescriptor::Float => serde_json::json!(0.0),
            NailDataTypeDescriptor::String => serde_json::json!(""),
            NailDataTypeDescriptor::Boolean => serde_json::json!(false),
            NailDataTypeDescriptor::Array(_) => serde_json::json!([]),
            other => panic!("dummy_json_for: unsupported field type in STDLIB_TYPES: {:?}", other),
        }
    }

    fn assert_matches_registry<T: DeserializeOwned + Serialize>(type_name: &str) {
        let info = STDLIB_TYPES.get(type_name).unwrap_or_else(|| panic!("{} not in STDLIB_TYPES", type_name));

        let mut object = serde_json::Map::new();
        for (field_name, field_type) in &info.fields {
            object.insert(field_name.clone(), dummy_json_for(field_type));
        }

        // Registry missing a field or wrong type => deserialization error
        let instance: T = serde_json::from_value(serde_json::Value::Object(object))
            .unwrap_or_else(|e| panic!("STDLIB_TYPES for '{}' does not match the real struct: {}", type_name, e));

        // Registry listing a field the struct lacks => key sets differ
        let reserialized = serde_json::to_value(&instance).expect("reserialize");
        let struct_keys: std::collections::BTreeSet<String> = reserialized.as_object().expect("expected object").keys().cloned().collect();
        let registry_keys: std::collections::BTreeSet<String> = info.fields.keys().cloned().collect();
        assert_eq!(registry_keys, struct_keys, "STDLIB_TYPES field set for '{}' differs from the real struct", type_name);
    }

    #[test]
    fn stdlib_types_match_real_structs() {
        assert_matches_registry::<crate::parser::std_lib::http::HTTP_Route>("HTTP_Route");
        assert_matches_registry::<crate::parser::std_lib::http::HTTP_Response>("HTTP_Response");
        assert_matches_registry::<crate::parser::std_lib::database::DB_SQLite>("DB_SQLite");
        assert_matches_registry::<crate::parser::std_lib::database::DB_Result>("DB_Result");
        #[cfg(feature = "duckdb")]
        {
            assert_matches_registry::<crate::parser::std_lib::duckdb::DB_DuckDB>("DB_DuckDB");
            assert_matches_registry::<crate::parser::std_lib::duckdb::DB_DuckDB_Result>("DB_DuckDB_Result");
        }
    }

    /// Every type name in STDLIB_TYPES must be covered by
    /// stdlib_types_match_real_structs above - fails when someone adds a new
    /// stdlib type without extending the drift test.
    #[test]
    fn all_stdlib_types_are_drift_tested() {
        let covered = ["HTTP_Route", "HTTP_Response", "DB_SQLite", "DB_Result", "DB_DuckDB", "DB_DuckDB_Result"];
        for type_name in STDLIB_TYPES.keys() {
            assert!(covered.contains(type_name), "STDLIB_TYPES entry '{}' has no drift test - add it to stdlib_types_match_real_structs", type_name);
        }
    }
}

