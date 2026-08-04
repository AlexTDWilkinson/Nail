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
///   i, f, s, b, v (void), e, never      - primitive types
///   T (any other bare ident)            - type variable, accepts any type
///   (T: i|f)                            - bounded type variable, accepts only the listed types
///   [X]                                 - array of X
///   (X!e)                               - result of X
///   (h K V)                             - hashmap from K to V
///   (fn(X, Y) -> Z)                     - function from X, Y to Z
/// Compose by nesting, e.g. ([s]!e) is a result of an array of strings. A
/// result of a hashmap must parenthesize the hashmap: ((h s s)!e).
macro_rules! nail_type {
    (i) => { NailDataTypeDescriptor::Int };
    (f) => { NailDataTypeDescriptor::Float };
    (s) => { NailDataTypeDescriptor::String };
    (b) => { NailDataTypeDescriptor::Boolean };
    (v) => { NailDataTypeDescriptor::Void };
    (e) => { NailDataTypeDescriptor::Error };
    (never) => { NailDataTypeDescriptor::Never };
    ([ $($inner:tt)+ ]) => { NailDataTypeDescriptor::Array(Box::new(nail_type!($($inner)+))) };
    ((h $key:tt $value:tt)) => { NailDataTypeDescriptor::HashMap(Box::new(nail_type!($key)), Box::new(nail_type!($value))) };
    (($inner:tt !e)) => { NailDataTypeDescriptor::Result(Box::new(nail_type!($inner))) };
    ((fn( $($param:tt),* ) -> $ret:tt)) => { NailDataTypeDescriptor::Fn(vec![ $( nail_type!($param) ),* ], Box::new(nail_type!($ret))) };
    (($type_var:ident : $($bound:tt)|+)) => { NailDataTypeDescriptor::TypeVar(stringify!($type_var).to_string(), vec![ $( nail_type!($bound) ),+ ]) };
    ($type_var:ident) => { NailDataTypeDescriptor::TypeVar(stringify!($type_var).to_string(), vec![]) };
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
mod base64;
mod code;
mod compress;
mod crypto;
mod csv;
mod database;
mod datafusion;
mod env;
mod error;
mod float;
mod fs;
mod hashmap;
mod hex;
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
            /// Every crate the stdlib registry can ever require. The bundle
            /// build uses this to pre-compile the full dependency superset so
            /// user machines never touch crates.io.
            pub fn all() -> Vec<CrateDependency> {
                vec![$(CrateDependency::$variant,)*]
            }

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
    TowerHttp => { cargo: "tower-http = { version = \"0.5\", features = [\"fs\"] }", name: "tower-http", import: "use tower_http;" },
    Tokio => { cargo: "tokio = { version = \"1\", features = [\"rt-multi-thread\", \"macros\"] }", name: "tokio", import: "use tokio;" },
    SerdeJson => { cargo: "serde_json = \"1.0\"", name: "serde_json", import: "use serde_json;" },
    Serde => { cargo: "serde = { version = \"1.0\", features = [\"derive\"] }", name: "serde", import: "use serde;" },
    Regex => { cargo: "regex = \"1.10\"", name: "regex", import: "use regex;" },
    Rand => { cargo: "rand = \"0.8\"", name: "rand", import: "use rand;" },
    Csv => { cargo: "csv = \"1.3\"", name: "csv", import: "use csv;" },
    DashMap => { cargo: "dashmap = { version = \"6.1.0\", features = [\"serde\"] }", name: "dashmap", import: "use dashmap;" },
    Pulldown => { cargo: "pulldown-cmark = \"0.9\"", name: "pulldown-cmark", import: "use pulldown_cmark;" },
    Reqwest => { cargo: "reqwest = { version = \"0.11\", default-features = false, features = [\"json\", \"rustls-tls\"] }", name: "reqwest", import: "use reqwest;" },
    Sha2 => { cargo: "sha2 = \"0.10\"", name: "sha2", import: "use sha2;" },
    Md5 => { cargo: "md5 = \"0.7\"", name: "md5", import: "use md5;" },
    Uuid => { cargo: "uuid = { version = \"1.0\", features = [\"v4\"] }", name: "uuid", import: "use uuid;" },
    UrlEncoding => { cargo: "urlencoding = \"2.1\"", name: "urlencoding", import: "use urlencoding;" },
    Flate2 => { cargo: "flate2 = \"1.0\"", name: "flate2", import: "use flate2;" },
    Base64 => { cargo: "base64 = \"0.21\"", name: "base64", import: "use base64;" },
    Hmac => { cargo: "hmac = \"0.12\"", name: "hmac", import: "use hmac;" },
    Rusqlite => { cargo: "rusqlite = { version = \"0.31\", features = [\"bundled\"] }", name: "rusqlite", import: "use rusqlite;" },
    DataFusion => { cargo: "datafusion = \"50\"", name: "datafusion", import: "use datafusion;", feature: "datafusion" },
}

/// Defines the StdlibModule enum, its runtime module path, and the namespace
/// every name in that module wears, from one table. The namespace is not
/// decoration: a Nail program has one flat name space, so `csv_open` and
/// `CSV_Options` say which library they belong to without an import list.
macro_rules! stdlib_modules {
    ($($variant:ident => $path:literal, $prefix:literal),* $(,)?) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub enum StdlibModule {
            $($variant,)*
        }

        impl StdlibModule {
            pub fn to_module_path(&self) -> &'static str {
                match self { $(StdlibModule::$variant => $path,)* }
            }

            /// The prefix every function this module exports must start with.
            pub fn name_prefix(&self) -> &'static str {
                match self { $(StdlibModule::$variant => $prefix,)* }
            }

            pub fn all() -> &'static [StdlibModule] {
                &[$(StdlibModule::$variant,)*]
            }
        }
    };
}

stdlib_modules! {
    Http => "std_lib::http", "http_",
    Fs => "std_lib::fs", "fs_",
    Json => "std_lib::json", "json_",
    String => "std_lib::string", "string_",
    Int => "std_lib::int", "int_",
    Float => "std_lib::float", "float_",
    Array => "std_lib::array", "array_",
    Math => "std_lib::math", "math_",
    Time => "std_lib::time", "time_",
    Env => "std_lib::env", "env_",
    Process => "std_lib::process", "process_",
    Path => "std_lib::path", "path_",
    Error => "std_lib::error", "error_",
    Panic => "std_lib::panic", "panic_",
    HashMap => "std_lib::hashmap", "hashmap_",
    IO => "std_lib::io", "io_",
    Print => "std_lib::print", "print",
    Markdown => "std_lib::markdown", "markdown_",
    Code => "std_lib::code", "code_",
    Crypto => "std_lib::crypto", "crypto_",
    Regex => "std_lib::regex", "regex_",
    Args => "std_lib::args", "args_",
    Url => "std_lib::url", "url_",
    Base64 => "std_lib::base64", "base64_",
    Hex => "std_lib::hex", "hex_",
    Csv => "std_lib::csv", "csv_",
    Compress => "std_lib::compress", "compress_",
    Database => "std_lib::database", "db_",
    DataFusion => "std_lib::datafusion", "db_datafusion_",
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
        base64::register(&mut m);
        code::register(&mut m);
        compress::register(&mut m);
        crypto::register(&mut m);
        csv::register(&mut m);
        database::register(&mut m);
        datafusion::register(&mut m);
        env::register(&mut m);
        error::register(&mut m);
        float::register(&mut m);
        fs::register(&mut m);
        hashmap::register(&mut m);
        hex::register(&mut m);
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

lazy_static! {
    /// Stdlib calls that have a lazy Rust-iterator form. When such a call is
    /// the iterable of a collection operation or for loop, the transpiler can
    /// emit this template (with {0}, {1}, ... as the transpiled argument
    /// expressions) instead of materializing a Vec through the async function.
    /// This keeps function-specific knowledge in the registry — the transpiler
    /// only knows the generic mechanism.
    static ref ITERATOR_FORMS: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("array_range", "({0}..{1})");
        m.insert("array_range_inclusive", "({0}..={1})");
        m
    };
}

/// Lazy iterator template for a stdlib call in iterable position, if one exists.
pub fn get_iterator_form(name: &str) -> Option<&'static str> {
    ITERATOR_FORMS.get(name).copied()
}

/// Whether a stdlib function is async in its Rust implementation (its call
/// sites need `.await` and callers must be async). I/O-performing modules are
/// async; pure computation modules are plain sync functions. Per-function
/// exceptions to a module's default live here too, so the transpiler stays
/// completely generic.
/// Functions whose Rust implementations are synchronous even though their
/// module is otherwise async, so no `.await` may be emitted for them.
const SYNC_STDLIB_FUNCTIONS: &[&str] = &["http_path_matches", "http_path_params", "http_default_cookie", "http_build_cookie", "http_parse_cookies"];

pub fn is_stdlib_fn_async(name: &str) -> bool {
    // Per-function overrides of the module default
    if name == "time_sleep" {
        return true;
    }
    if SYNC_STDLIB_FUNCTIONS.contains(&name) {
        return false;
    }
    matches!(
        get_stdlib_function(name).map(|f| &f.module),
        Some(StdlibModule::Fs | StdlibModule::Http | StdlibModule::IO | StdlibModule::Database | StdlibModule::DataFusion | StdlibModule::Process)
    )
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
        
        // CSV_Reader struct
        m.insert("CSV_Reader", StdlibTypeInfo {
            name: "CSV_Reader".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("handle".to_string(), NailDataTypeDescriptor::String);
                fields.insert("path".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        // CSV_Options struct
        m.insert("CSV_Options", StdlibTypeInfo {
            name: "CSV_Options".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("delimiter".to_string(), NailDataTypeDescriptor::String);
                fields.insert("quote".to_string(), NailDataTypeDescriptor::String);
                fields.insert("escape".to_string(), NailDataTypeDescriptor::String);
                fields.insert("double_quote".to_string(), NailDataTypeDescriptor::Boolean);
                fields.insert("comment".to_string(), NailDataTypeDescriptor::String);
                fields.insert("has_headers".to_string(), NailDataTypeDescriptor::Boolean);
                fields.insert("flexible".to_string(), NailDataTypeDescriptor::Boolean);
                fields.insert("trim".to_string(), NailDataTypeDescriptor::Enum("CSV_Trim".to_string()));
                fields.insert("skip_rows".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("n_rows".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("null_values".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)));
                fields.insert("ignore_errors".to_string(), NailDataTypeDescriptor::Boolean);
                fields.insert("eol_char".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        // HTTP_Static struct
        m.insert("HTTP_Static", StdlibTypeInfo {
            name: "HTTP_Static".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("prefix".to_string(), NailDataTypeDescriptor::String);
                fields.insert("directory".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        // HTTP_Config struct
        m.insert("HTTP_Config", StdlibTypeInfo {
            name: "HTTP_Config".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("static_mounts".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct("HTTP_Static".to_string()))));
                fields.insert("max_body_bytes".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("timeout_seconds".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("state".to_string(), NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)));
                fields
            }
        });

        // HTTP_Cookie struct
        m.insert("HTTP_Cookie", StdlibTypeInfo {
            name: "HTTP_Cookie".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("name".to_string(), NailDataTypeDescriptor::String);
                fields.insert("value".to_string(), NailDataTypeDescriptor::String);
                fields.insert("path".to_string(), NailDataTypeDescriptor::String);
                fields.insert("max_age".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("http_only".to_string(), NailDataTypeDescriptor::Boolean);
                fields.insert("secure".to_string(), NailDataTypeDescriptor::Boolean);
                fields.insert("same_site".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        // HTTP_Request struct
        m.insert("HTTP_Request", StdlibTypeInfo {
            name: "HTTP_Request".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("method".to_string(), NailDataTypeDescriptor::String);
                fields.insert("path".to_string(), NailDataTypeDescriptor::String);
                fields.insert("query".to_string(), NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)));
                fields.insert("headers".to_string(), NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)));
                fields.insert("body".to_string(), NailDataTypeDescriptor::String);
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
                fields.insert("content_type".to_string(), NailDataTypeDescriptor::String);
                fields.insert("headers".to_string(), NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)));
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

        // DB_DataFusion struct
        m.insert("DB_DataFusion", StdlibTypeInfo {
            name: "DB_DataFusion".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("handle".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        // DB_DataFusion_Result struct (DataFusion has no rowids, so no last_insert_id)
        m.insert("DB_DataFusion_Result", StdlibTypeInfo {
            name: "DB_DataFusion_Result".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("rows_affected".to_string(), NailDataTypeDescriptor::Int);
                fields
            }
        });

        m
    };
}

/// A user-defined Nail function that a stdlib function calls back into, such
/// as the request handler `http_server` dispatches to. The transpiler passes a
/// reference to it as a trailing argument, and the checker requires the program
/// to define it with exactly this signature.
///
/// Declared here rather than in the transpiler or checker so that no core
/// compiler stage ever names a specific function.
pub struct HandlerCallback {
    pub function_name: &'static str,
    pub parameter_types: Vec<NailDataTypeDescriptor>,
    pub return_type: NailDataTypeDescriptor,
}

lazy_static! {
    /// Stdlib function name -> the Nail function it calls back into.
    pub static ref HANDLER_CALLBACKS: HashMap<&'static str, HandlerCallback> = {
        let mut m = HashMap::new();
        m.insert("http_server", HandlerCallback {
            function_name: "handle_request",
            parameter_types: vec![
                NailDataTypeDescriptor::Struct("HTTP_Request".to_string()),
                NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)),
            ],
            return_type: NailDataTypeDescriptor::Struct("HTTP_Response".to_string()),
        });
        m
    };
}

/// The Nail function a stdlib function dispatches to, if it takes one.
pub fn get_handler_callback(name: &str) -> Option<&'static HandlerCallback> {
    HANDLER_CALLBACKS.get(name)
}

/// Whether a user function is the target of some stdlib callback. Such a
/// function is invoked from async glue, so it can never be emitted as a plain
/// sync Rust function.
pub fn is_handler_callback_target(function_name: &str) -> bool {
    HANDLER_CALLBACKS.values().any(|callback| callback.function_name == function_name)
}

lazy_static! {
    /// Enums the stdlib provides. Kept beside STDLIB_TYPES rather than inside
    /// it because an enum is a variant list, not a field map.
    pub static ref STDLIB_ENUMS: HashMap<&'static str, Vec<&'static str>> = {
        let mut m = HashMap::new();
        m.insert("CSV_Trim", vec!["None", "Headers", "Fields", "All"]);
        m.insert("HTTP_Method", vec!["Get", "Post", "Put", "Delete", "Patch"]);
        m
    };
}

/// The variants of a stdlib enum, if the name names one.
pub fn get_stdlib_enum_variants(name: &str) -> Option<&'static Vec<&'static str>> {
    STDLIB_ENUMS.get(name)
}

pub fn is_stdlib_enum(name: &str) -> bool {
    STDLIB_ENUMS.contains_key(name)
}

/// Get all stdlib type names (structs/enums defined in stdlib)
pub fn get_stdlib_type_names() -> HashSet<String> {
    STDLIB_TYPES.keys().chain(STDLIB_ENUMS.keys()).map(|k| k.to_string()).collect()
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

/// Get the full field list of a stdlib struct
pub fn get_stdlib_struct_fields(name: &str) -> Option<Vec<(String, NailDataTypeDescriptor)>> {
    STDLIB_TYPES.get(name).map(|info| info.fields.iter().map(|(field_name, field_type)| (field_name.clone(), field_type.clone())).collect())
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
            NailDataTypeDescriptor::HashMap(_, _) => serde_json::json!({}),
            // A unit-only enum serializes as its variant name, so any declared
            // variant is a valid stand-in value.
            NailDataTypeDescriptor::Enum(enum_name) => {
                let variants = STDLIB_ENUMS.get(enum_name.as_str()).unwrap_or_else(|| panic!("dummy_json_for: '{}' is not a registered stdlib enum", enum_name));
                serde_json::json!(variants[0])
            }
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

    /// Every variant the registry claims must deserialize into the real Rust
    /// enum, so a renamed or removed variant fails here instead of at runtime.
    fn assert_enum_matches_registry<T: DeserializeOwned>(enum_name: &str) {
        let variants = STDLIB_ENUMS.get(enum_name).unwrap_or_else(|| panic!("{} not in STDLIB_ENUMS", enum_name));
        for variant in variants.iter() {
            let _: T = serde_json::from_value(serde_json::json!(variant))
                .unwrap_or_else(|e| panic!("STDLIB_ENUMS lists '{}::{}' but the real enum rejects it: {}", enum_name, variant, e));
        }
    }

    #[test]
    fn stdlib_enums_match_real_enums() {
        assert_enum_matches_registry::<crate::parser::std_lib::csv::CSV_Trim>("CSV_Trim");
        assert_enum_matches_registry::<crate::parser::std_lib::http::HTTP_Method>("HTTP_Method");
    }

    #[test]
    fn all_stdlib_enums_are_drift_tested() {
        let covered = ["CSV_Trim", "HTTP_Method"];
        for enum_name in STDLIB_ENUMS.keys() {
            assert!(covered.contains(enum_name), "STDLIB_ENUMS entry '{}' has no drift test", enum_name);
        }
    }

    /// Every stdlib name wears its library's namespace: functions carry the
    /// module prefix (`csv_open`, `http_server`), and types carry the
    /// upper-case one (`CSV_Options`, `HTTP_Config`, `TIME_Format`). Nail has
    /// one flat name space and no imports, so the prefix is what says where a
    /// name comes from - a name without one reads like a language keyword.
    #[test]
    fn stdlib_function_names_carry_their_namespace() {
        // The language's own words, which belong to no library and are spelled
        // the way the grammar spells them.
        const LANGUAGE_BUILTINS: &[&str] = &["danger", "safe", "expect", "panic", "todo", "spawn", "print", "print_no_newline"];
        for (name, function) in STDLIB_FUNCTIONS.iter() {
            if LANGUAGE_BUILTINS.contains(name) {
                continue;
            }
            let prefix = function.module.name_prefix();
            assert!(
                name.starts_with(prefix),
                "stdlib function '{}' is in the {:?} module, so it must be named '{}...'",
                name,
                function.module,
                prefix
            );
        }
    }

    /// The type side of the same rule. Every stdlib struct and enum starts with
    /// a module namespace in upper case, so a Nail program can tell a library
    /// type from one of its own at a glance.
    #[test]
    fn stdlib_type_names_carry_their_namespace() {
        let namespaces: Vec<String> = StdlibModule::all()
            .iter()
            .map(|module| module.name_prefix().trim_end_matches('_').to_uppercase() + "_")
            .collect();
        let named = STDLIB_TYPES.keys().copied().chain(STDLIB_ENUMS.keys().copied());
        for name in named {
            assert!(
                namespaces.iter().any(|namespace| name.starts_with(namespace.as_str())),
                "stdlib type '{}' must start with its library's namespace, e.g. CSV_ or HTTP_",
                name
            );
        }
    }

    #[test]
    fn stdlib_types_match_real_structs() {
        assert_matches_registry::<crate::parser::std_lib::csv::CSV_Options>("CSV_Options");
        assert_matches_registry::<crate::parser::std_lib::csv::CSV_Reader>("CSV_Reader");
        assert_matches_registry::<crate::parser::std_lib::http::HTTP_Config>("HTTP_Config");
        assert_matches_registry::<crate::parser::std_lib::http::HTTP_Static>("HTTP_Static");
        assert_matches_registry::<crate::parser::std_lib::http::HTTP_Cookie>("HTTP_Cookie");
        assert_matches_registry::<crate::parser::std_lib::http::HTTP_Request>("HTTP_Request");
        assert_matches_registry::<crate::parser::std_lib::http::HTTP_Response>("HTTP_Response");
        assert_matches_registry::<crate::parser::std_lib::database::DB_SQLite>("DB_SQLite");
        assert_matches_registry::<crate::parser::std_lib::database::DB_Result>("DB_Result");
        #[cfg(feature = "datafusion")]
        {
            assert_matches_registry::<crate::parser::std_lib::datafusion::DB_DataFusion>("DB_DataFusion");
            assert_matches_registry::<crate::parser::std_lib::datafusion::DB_DataFusion_Result>("DB_DataFusion_Result");
        }
    }

    /// Every type name in STDLIB_TYPES must be covered by
    /// stdlib_types_match_real_structs above - fails when someone adds a new
    /// stdlib type without extending the drift test.
    #[test]
    fn all_stdlib_types_are_drift_tested() {
        let covered = ["CSV_Options", "CSV_Reader", "HTTP_Config", "HTTP_Cookie", "HTTP_Static", "HTTP_Request", "HTTP_Response", "DB_SQLite", "DB_Result", "DB_DataFusion", "DB_DataFusion_Result"];
        for type_name in STDLIB_TYPES.keys() {
            assert!(covered.contains(type_name), "STDLIB_TYPES entry '{}' has no drift test - add it to stdlib_types_match_real_structs", type_name);
        }
    }
}

