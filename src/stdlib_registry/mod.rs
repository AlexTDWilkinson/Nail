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

mod archive;
mod args;
mod array;
mod audio;
mod base32;
mod base58;
mod base64;
mod binary;
mod bits;
mod boolean;
mod cache;
mod chart;
mod code;
mod color;
mod compress;
mod convert;
mod crypto;
mod csv;
mod database;
mod datafusion;
mod diff;
mod draw;
mod email;
mod env;
mod error;
mod float;
mod feed;
mod finance;
mod format;
mod fs;
mod game;
mod game3d;
mod geo;
mod hashmap;
mod hex;
mod i18n;
mod ini;
mod html;
mod http;
mod image;
mod int;
mod io;
mod json;
mod jwt;
mod linalg;
mod log;
mod markdown;
mod math;
mod mcp;
mod mime;
mod ml;
mod money;
mod net;
mod panic;
mod path;
mod pdf;
mod postgres;
mod print;
mod process;
mod rand;
mod valkey;
mod regex;
mod semver;
mod sched;
mod stats;
mod stdlib;
mod string;
mod sys;
mod template;
mod term;
mod test;
mod time;
mod toml;
mod tui;
mod url;
mod validate;
mod xlsx;
mod xml;
mod yaml;

/// Defines the CrateDependency enum and all its lookup methods from a single
/// table so adding a crate is one line instead of four match arms.
macro_rules! crate_dependencies {
    ($($variant:ident => { cargo: $cargo:literal, name: $name:literal, import: $import:literal $(, feature: $feature:literal)? $(, system_libraries: $system:literal)? }),* $(,)?) => {
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

            /// Whether this crate needs libraries that must already be on the
            /// machine, found through pkg-config at build time.
            ///
            /// The bundle exists to make builds work with nothing installed,
            /// so it cannot contain these: there is no system to find the
            /// libraries on. They stay usable in a source checkout, where the
            /// developer has the dev packages, and a bundled install cannot
            /// build a program that reaches them.
            pub fn needs_system_libraries(&self) -> bool {
                match self { $(CrateDependency::$variant => crate_dependencies!(@system $($system)?),)* }
            }
        }
    };
    (@feature $feature:literal) => { Some($feature) };
    (@feature) => { None };
    (@system $system:literal) => { $system };
    (@system) => { false };
}

crate_dependencies! {
    Futures => { cargo: "futures = \"0.3\"", name: "futures", import: "use futures;" },
    Axum => { cargo: "axum = { version = \"0.7\", features = [\"ws\"] }", name: "axum", import: "use axum;" },
    TowerHttp => { cargo: "tower-http = { version = \"0.5\", features = [\"fs\"] }", name: "tower-http", import: "use tower_http;" },
    Tokio => { cargo: "tokio = { version = \"1\", features = [\"rt-multi-thread\", \"macros\"] }", name: "tokio", import: "use tokio;" },
    SerdeJson => { cargo: "serde_json = \"1.0\"", name: "serde_json", import: "use serde_json;" },
    Serde => { cargo: "serde = { version = \"1.0\", features = [\"derive\"] }", name: "serde", import: "use serde;" },
    Regex => { cargo: "regex = \"1.10\"", name: "regex", import: "use regex;" },
    Rand => { cargo: "rand = \"0.8\"", name: "rand", import: "use rand;" },
    Csv => { cargo: "csv = \"1.3\"", name: "csv", import: "use csv;" },
    DashMap => { cargo: "dashmap = { version = \"6.1.0\", features = [\"serde\"] }", name: "dashmap", import: "use dashmap;" },
    Pulldown => { cargo: "pulldown-cmark = \"0.9\"", name: "pulldown-cmark", import: "use pulldown_cmark;" },
    Reqwest => { cargo: "reqwest = { version = \"0.11\", default-features = false, features = [\"json\", \"rustls-tls\", \"multipart\"] }", name: "reqwest", import: "use reqwest;" },
    Sha2 => { cargo: "sha2 = \"0.10\"", name: "sha2", import: "use sha2;" },
    Md5 => { cargo: "md5 = \"0.7\"", name: "md5", import: "use md5;" },
    Uuid => { cargo: "uuid = { version = \"1.0\", features = [\"v4\", \"v7\"] }", name: "uuid", import: "use uuid;" },
    Argon2 => { cargo: "argon2 = \"0.5\"", name: "argon2", import: "use argon2;" },
    Toml => { cargo: "toml = \"0.8\"", name: "toml", import: "use toml;" },
    QuickXml => { cargo: "quick-xml = { version = \"0.36\", features = [\"serialize\"] }", name: "quick-xml", import: "use quick_xml;" },
    FeedRs => { cargo: "feed-rs = \"2\"", name: "feed-rs", import: "use feed_rs;" },
    Notify => { cargo: "notify = \"6\"", name: "notify", import: "use notify;" },
    PrintPdf => { cargo: "printpdf = \"0.7\"", name: "printpdf", import: "use printpdf;", feature: "pdf" },
    PdfExtract => { cargo: "pdf-extract = \"0.7\"", name: "pdf-extract", import: "use pdf_extract;", feature: "pdf" },
    Calamine => { cargo: "calamine = \"0.24\"", name: "calamine", import: "use calamine;", feature: "xlsx" },
    RustXlsxWriter => { cargo: "rust_xlsxwriter = \"0.64\"", name: "rust_xlsxwriter", import: "use rust_xlsxwriter;", feature: "xlsx" },
    SerdeYaml => { cargo: "serde_yaml = \"0.9\"", name: "serde_yaml", import: "use serde_yaml;" },
    UrlEncoding => { cargo: "urlencoding = \"2.1\"", name: "urlencoding", import: "use urlencoding;" },
    Flate2 => { cargo: "flate2 = \"1.0\"", name: "flate2", import: "use flate2;" },
    Zip => { cargo: "zip = { version = \"2\", default-features = false, features = [\"deflate\"] }", name: "zip", import: "use zip;" },
    Tar => { cargo: "tar = \"0.4\"", name: "tar", import: "use tar;" },
    Base64 => { cargo: "base64 = \"0.21\"", name: "base64", import: "use base64;" },
    AesGcm => { cargo: "aes-gcm = \"0.10\"", name: "aes-gcm", import: "use aes_gcm;" },
    Hmac => { cargo: "hmac = \"0.12\"", name: "hmac", import: "use hmac;" },
    Crossterm => { cargo: "crossterm = \"0.28\"", name: "crossterm", import: "use crossterm;" },
    Chrono => { cargo: "chrono = \"0.4\"", name: "chrono", import: "use chrono;" },
    Rusqlite => { cargo: "rusqlite = { version = \"0.31\", features = [\"bundled\"] }", name: "rusqlite", import: "use rusqlite;" },
    DataFusion => { cargo: "datafusion = \"50\"", name: "datafusion", import: "use datafusion;", feature: "datafusion" },
    Rodio => { cargo: "rodio = \"0.19\"", name: "rodio", import: "use rodio;", feature: "audio", system_libraries: true },
    TokioPostgres => { cargo: "tokio-postgres = \"0.7\"", name: "tokio-postgres", import: "use tokio_postgres;", feature: "postgres" },
    Image => { cargo: "image = { version = \"0.25\", default-features = false, features = [\"png\", \"jpeg\", \"gif\", \"webp\", \"bmp\", \"tiff\"] }", name: "image", import: "use image;", feature: "image" },
    Scraper => { cargo: "scraper = \"0.20\"", name: "scraper", import: "use scraper;", feature: "html" },
    Lettre => { cargo: "lettre = { version = \"0.11\", default-features = false, features = [\"smtp-transport\", \"tokio1-rustls-tls\", \"builder\"] }", name: "lettre", import: "use lettre;", feature: "email" },
    Crc32Fast => { cargo: "crc32fast = \"1.4\"", name: "crc32fast", import: "use crc32fast;" },
    Sha1 => { cargo: "sha1 = \"0.10\"", name: "sha1", import: "use sha1;" },
    Blake3 => { cargo: "blake3 = \"1.5\"", name: "blake3", import: "use blake3;" },
    UnicodeSegmentation => { cargo: "unicode-segmentation = \"1.11\"", name: "unicode-segmentation", import: "use unicode_segmentation;" },
    UnicodeNormalization => { cargo: "unicode-normalization = \"0.1\"", name: "unicode-normalization", import: "use unicode_normalization;" },
    Dirs => { cargo: "dirs = \"5\"", name: "dirs", import: "use dirs;" },
    Diffy => { cargo: "diffy = \"0.4\"", name: "diffy", import: "use diffy;" },
    QrCode => { cargo: "qrcode = { version = \"0.14\", default-features = false, features = [\"svg\"] }", name: "qrcode", import: "use qrcode;" },
    EncodingRs => { cargo: "encoding_rs = \"0.8\"", name: "encoding_rs", import: "use encoding_rs;" },
    Zstd => { cargo: "zstd = \"0.13\"", name: "zstd", import: "use zstd;", feature: "compress" },
    Brotli => { cargo: "brotli = \"6\"", name: "brotli", import: "use brotli;", feature: "compress" },
    JsonSchema => { cargo: "jsonschema = { version = \"0.18\", default-features = false }", name: "jsonschema", import: "use jsonschema;", feature: "jsonschema" },
    ChronoTz => { cargo: "chrono-tz = \"0.9\"", name: "chrono-tz", import: "use chrono_tz;", feature: "timezones" },
    SysInfo => { cargo: "sysinfo = \"0.31\"", name: "sysinfo", import: "use sysinfo;", feature: "sys" },
    Ammonia => { cargo: "ammonia = \"4\"", name: "ammonia", import: "use ammonia;", feature: "html" },
    TokioTungstenite => { cargo: "tokio-tungstenite = { version = \"0.23\", features = [\"rustls-tls-webpki-roots\"] }", name: "tokio-tungstenite", import: "use tokio_tungstenite;", feature: "websocket" },
    ValkeyClient => { cargo: "redis = { version = \"0.27\", features = [\"tokio-comp\"] }", name: "redis", import: "use redis;", feature: "valkey" },
    Winit => { cargo: "winit = \"0.30\"", name: "winit", import: "use winit;", feature: "game", system_libraries: true },
    Softbuffer => { cargo: "softbuffer = \"0.4\"", name: "softbuffer", import: "use softbuffer;", feature: "game", system_libraries: true },
    TinySkia => { cargo: "tiny-skia = \"0.11\"", name: "tiny-skia", import: "use tiny_skia;", feature: "game" },
    Fontdue => { cargo: "fontdue = \"0.9\"", name: "fontdue", import: "use fontdue;", feature: "game" },
    Gltf => { cargo: "gltf = { version = \"1\", default-features = false, features = [\"import\", \"utils\"] }", name: "gltf", import: "use gltf;", feature: "game" },
    Idna => { cargo: "idna = \"1\"", name: "idna", import: "use idna;" },
    Ed25519 => { cargo: "ed25519-dalek = { version = \"2\", features = [\"rand_core\"] }", name: "ed25519-dalek", import: "use ed25519_dalek;" },
    Htmd => { cargo: "htmd = \"0.5\"", name: "htmd", import: "use htmd;", feature: "html" },
    HickoryResolver => { cargo: "hickory-resolver = \"0.24\"", name: "hickory-resolver", import: "use hickory_resolver;", feature: "dns" },
    TokioRustls => { cargo: "tokio-rustls = \"0.26\"", name: "tokio-rustls", import: "use tokio_rustls;", feature: "tls" },
    WebpkiRoots => { cargo: "webpki-roots = \"1\"", name: "webpki-roots", import: "use webpki_roots;", feature: "tls" },
    X509Parser => { cargo: "x509-parser = \"0.18\"", name: "x509-parser", import: "use x509_parser;", feature: "tls" },
}

/// Defines the StdlibModule enum, its runtime module path, the namespace
/// every name in that module wears, and the name people read it by, from one
/// table. The namespace is not decoration: a Nail program has one flat name
/// space, so `csv_open` and `CSV_Options` say which library they belong to
/// without an import list.
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

            /// What the module is called anywhere a person reads it - the IDE's
            /// library browser, the website's function list, documentation.
            /// It is the module's whole function prefix without the trailing
            /// underscore, so `db_sqlite_` reads `db_sqlite` and `ml_boost_`
            /// reads `ml_boost`. Derived rather than written by hand so the
            /// name a listing shows can never drift from the name a program
            /// calls.
            pub fn display_name(&self) -> &'static str {
                // The whole prefix, so families read as their own groups:
                // db_postgres and ml_boost, not one giant db or ml.
                let prefix = self.name_prefix();
                let trimmed = prefix.trim_end_matches('_');
                if trimmed.is_empty() { prefix } else { trimmed }
            }

            pub fn all() -> &'static [StdlibModule] {
                &[$(StdlibModule::$variant,)*]
            }
        }
    };
}

stdlib_modules! {
    Http => "std_lib::http", "http_",
    Mcp => "std_lib::mcp", "mcp_",
    Fs => "std_lib::fs", "fs_",
    Json => "std_lib::json", "json_",
    Toml => "std_lib::toml", "toml_",
    Ini => "std_lib::ini", "ini_",
    Yaml => "std_lib::yaml", "yaml_",
    Xml => "std_lib::xml", "xml_",
    Feed => "std_lib::feed", "feed_",
    Pdf => "std_lib::pdf", "pdf_",
    Xlsx => "std_lib::xlsx", "xlsx_",
    String => "std_lib::string", "string_",
    Int => "std_lib::int", "int_",
    Float => "std_lib::float", "float_",
    Bool => "std_lib::boolean", "bool_",
    Format => "std_lib::format", "format_",
    Convert => "std_lib::convert", "convert_",
    Color => "std_lib::color", "color_",
    Geo => "std_lib::geo", "geo_",
    Array => "std_lib::array", "array_",
    Math => "std_lib::math", "math_",
    Linalg => "std_lib::linalg", "linalg_",
    Money => "std_lib::money", "money_",
    Finance => "std_lib::finance", "finance_",
    Stats => "std_lib::stats", "stats_",
    Semver => "std_lib::semver", "semver_",
    Ml => "std_lib::ml", "ml_",
    MlBoost => "std_lib::ml", "ml_boost_",
    MlForest => "std_lib::ml", "ml_forest_",
    MlTree => "std_lib::ml", "ml_tree_",
    MlLinear => "std_lib::ml", "ml_linear_",
    Bits => "std_lib::bits", "bits_",
    Rand => "std_lib::rand", "rand_",
    Time => "std_lib::time", "time_",
    Env => "std_lib::env", "env_",
    Sys => "std_lib::sys", "sys_",
    Sched => "std_lib::sched", "sched_",
    I18n => "std_lib::i18n", "i18n_",
    Process => "std_lib::process", "process_",
    Path => "std_lib::path", "path_",
    Error => "std_lib::error", "error_",
    Panic => "std_lib::panic", "panic_",
    HashMap => "std_lib::hashmap", "hashmap_",
    IO => "std_lib::io", "io_",
    Print => "std_lib::print", "print",
    Log => "std_lib::log", "log_",
    Term => "std_lib::term", "term_",
    Tui => "std_lib::tui", "tui_",
    Test => "std_lib::test", "test_",
    Html => "std_lib::html", "html_",
    Markdown => "std_lib::markdown", "markdown_",
    Mime => "std_lib::mime", "mime_",
    Template => "std_lib::template", "template_",
    Draw => "std_lib::draw", "draw_",
    Image => "std_lib::image", "image_",
    Chart => "std_lib::chart", "chart_",
    Audio => "std_lib::audio", "audio_",
    Game => "std_lib::game", "game_",
    Game3d => "std_lib::game3d", "game3d_",
    Code => "std_lib::code", "code_",
    Crypto => "std_lib::crypto", "crypto_",
    Email => "std_lib::email", "email_",
    Jwt => "std_lib::jwt", "jwt_",
    Validate => "std_lib::validate", "validate_",
    Regex => "std_lib::regex", "regex_",
    Args => "std_lib::args", "args_",
    Url => "std_lib::url", "url_",
    Diff => "std_lib::diff", "diff_",
    Base64 => "std_lib::base64", "base64_",
    Base32 => "std_lib::base32", "base32_",
    Base58 => "std_lib::base58", "base58_",
    Hex => "std_lib::hex", "hex_",
    Binary => "std_lib::binary", "binary_",
    Cache => "std_lib::cache", "cache_",
    Csv => "std_lib::csv", "csv_",
    Compress => "std_lib::compress", "compress_",
    Archive => "std_lib::archive", "archive_",
    Net => "std_lib::net", "net_",
    Sqlite => "std_lib::database", "db_sqlite_",
    DataFusion => "std_lib::datafusion", "db_datafusion_",
    Postgres => "std_lib::postgres", "db_postgres_",
    Valkey => "std_lib::valkey", "db_valkey_",
    Stdlib => "std_lib::stdlib", "stdlib_",
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
        archive::register(&mut m);
        args::register(&mut m);
        array::register(&mut m);
        audio::register(&mut m);
        base64::register(&mut m);
        bits::register(&mut m);
        chart::register(&mut m);
        code::register(&mut m);
        base32::register(&mut m);
        base58::register(&mut m);
        binary::register(&mut m);
        cache::register(&mut m);
        color::register(&mut m);
        compress::register(&mut m);
        convert::register(&mut m);
        crypto::register(&mut m);
        csv::register(&mut m);
        database::register(&mut m);
        datafusion::register(&mut m);
        diff::register(&mut m);
        draw::register(&mut m);
        email::register(&mut m);
        env::register(&mut m);
        error::register(&mut m);
        boolean::register(&mut m);
        float::register(&mut m);
        feed::register(&mut m);
        finance::register(&mut m);
        format::register(&mut m);
        fs::register(&mut m);
        game::register(&mut m);
        game3d::register(&mut m);
        geo::register(&mut m);
        hashmap::register(&mut m);
        hex::register(&mut m);
        i18n::register(&mut m);
        ini::register(&mut m);
        html::register(&mut m);
        http::register(&mut m);
        image::register(&mut m);
        int::register(&mut m);
        io::register(&mut m);
        json::register(&mut m);
        jwt::register(&mut m);
        linalg::register(&mut m);
        log::register(&mut m);
        markdown::register(&mut m);
        math::register(&mut m);
        mcp::register(&mut m);
        mime::register(&mut m);
        ml::register(&mut m);
        money::register(&mut m);
        net::register(&mut m);
        panic::register(&mut m);
        path::register(&mut m);
        pdf::register(&mut m);
        postgres::register(&mut m);
        print::register(&mut m);
        process::register(&mut m);
        rand::register(&mut m);
        valkey::register(&mut m);
        regex::register(&mut m);
        semver::register(&mut m);
        sched::register(&mut m);
        stats::register(&mut m);
        stdlib::register(&mut m);
        string::register(&mut m);
        sys::register(&mut m);
        template::register(&mut m);
        term::register(&mut m);
        test::register(&mut m);
        time::register(&mut m);
        toml::register(&mut m);
        tui::register(&mut m);
        url::register(&mut m);
        validate::register(&mut m);
        xlsx::register(&mut m);
        xml::register(&mut m);
        yaml::register(&mut m);
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
const SYNC_STDLIB_FUNCTIONS: &[&str] =
    &[
        "http_path_matches",
        "http_path_params",
        "http_default_cookie",
        "http_build_cookie",
        "http_parse_cookies",
        "http_default_config",
        "http_default_retry",
        "http_part_text",
        "http_part_file",
        "process_default_options",
        "net_ip_in_cidr",
        "net_ip_is_private",
        "net_ip_is_loopback",
        "net_ip_version",
        "net_ip_to_int",
        "net_ip_from_int",
    ];

/// One callback argument of a stdlib function, and where its parameters come
/// from: `over` lists the argument positions of the arrays to walk in step, one
/// array per callback parameter.
///
/// `array_sort_by(books, book_year)` has its callback at position 1 taking one
/// element from the array at position 0, so `over` is `[0]`.
/// `array_zip_with(prices, counts, line_total)` has its callback at position 2
/// taking one element from each of positions 0 and 1, so `over` is `[0, 1]`.
pub struct CallbackArgument {
    pub position: usize,
    pub over: &'static [usize],
}

/// Stdlib functions that take a Nail function and want its results rather than
/// the function itself - `array_sort_by(books, book_year)` and the rest of the
/// `_by` family, plus `array_zip_with`.
///
/// Nothing calls the program back while these work. The transpiler runs the
/// callback over the arrays first, in a loop it can await in, and then hands the
/// results alongside the arrays to the implementation the registry names. That
/// is what lets a key function read a file: sorting never has to wait for
/// anything, because the waiting all happened before it started.
///
/// The generated call is every non-callback argument in order, then one results
/// vector per callback in order. So `array_sort_by` reaches `sort_by_keys(items,
/// keys)` and `array_zip_with` reaches `zip_with_values(first, second, combined)`.
const CALLBACK_PRECOMPUTES: &[(&str, &[CallbackArgument])] = &[
    ("array_sort_by", &[CallbackArgument { position: 1, over: &[0] }]),
    ("array_sort_by_descending", &[CallbackArgument { position: 1, over: &[0] }]),
    ("array_min_by", &[CallbackArgument { position: 1, over: &[0] }]),
    ("array_max_by", &[CallbackArgument { position: 1, over: &[0] }]),
    ("array_sum_by", &[CallbackArgument { position: 1, over: &[0] }]),
    ("array_group_by", &[CallbackArgument { position: 1, over: &[0] }]),
    ("array_count_by", &[CallbackArgument { position: 1, over: &[0] }]),
    ("array_take_while", &[CallbackArgument { position: 1, over: &[0] }]),
    ("array_skip_while", &[CallbackArgument { position: 1, over: &[0] }]),
    ("array_deduplicate_by", &[CallbackArgument { position: 1, over: &[0] }]),
    ("array_zip_with", &[CallbackArgument { position: 2, over: &[0, 1] }]),
];

/// The callback arguments this stdlib function wants worked out before it is
/// called, or None if it takes no callback at all.
pub fn precomputed_callbacks(name: &str) -> Option<&'static [CallbackArgument]> {
    return CALLBACK_PRECOMPUTES.iter().find(|(function_name, _)| *function_name == name).map(|(_, callbacks)| *callbacks);
}

/// How a stdlib function that folds a file line by line is put together: which
/// runtime functions open, read and close the file, and how many lines to take at
/// a time. The transpiler emits the loop from this, so it needs no function names
/// of its own - the loop has to be emitted rather than written in Rust because the
/// program's step function may be async, and only the emitter knows whether it is.
pub struct FileFold {
    pub open: &'static str,
    pub next_lines: &'static str,
    pub close: &'static str,
    pub lines_at_a_time: usize,
}

pub fn file_fold(name: &str) -> Option<FileFold> {
    return match name {
        "fs_reduce_lines" => Some(FileFold { open: "std_lib::fs::open_reader", next_lines: "std_lib::fs::next_lines", close: "std_lib::fs::close_reader", lines_at_a_time: 1000 }),
        _ => None,
    };
}

/// Deny phrases for sandboxed-code policy, shared between the module defaults
/// and the per-function overrides below.
const SANDBOX_TOUCHES_MACHINE: &str = "touches the machine";
const SANDBOX_READS_MACHINE_STATE: &str = "reads machine state";
const SANDBOX_HOLDS_GLOBAL_STATE: &str = "holds global state";
const SANDBOX_SEIZES_RESOURCE: &str = "seizes a resource";

/// Functions denied inside sandboxed code even though their module is otherwise
/// allowed, each with the phrase explaining why.
const SANDBOX_DENIED_FUNCTIONS: &[(&str, &str)] = &[
    // Time arithmetic is pure, but sleeping holds the thread hostage
    ("time_sleep", SANDBOX_SEIZES_RESOURCE),
    // CSV text work is pure, but these take a file path or hold a file handle
    ("csv_write", SANDBOX_TOUCHES_MACHINE),
    ("csv_open", SANDBOX_TOUCHES_MACHINE),
    ("csv_next_rows", SANDBOX_TOUCHES_MACHINE),
    ("csv_close", SANDBOX_TOUCHES_MACHINE),
    // Hashing is pure, but these two read the file themselves
    ("crypto_hash_file_sha256", SANDBOX_TOUCHES_MACHINE),
    ("crypto_hash_file_blake3", SANDBOX_TOUCHES_MACHINE),
    // Path strings are pure, but these two consult the real filesystem
    ("path_exists", SANDBOX_READS_MACHINE_STATE),
    ("path_absolute", SANDBOX_READS_MACHINE_STATE),
    // Terminal styling builds strings, but these three ask the real terminal
    ("term_is_tty", SANDBOX_READS_MACHINE_STATE),
    ("term_width", SANDBOX_READS_MACHINE_STATE),
    ("term_height", SANDBOX_READS_MACHINE_STATE),
];

/// Functions allowed inside sandboxed code even though their module is otherwise
/// denied.
const SANDBOX_ALLOWED_FUNCTIONS: &[&str] = &[
    // Pure IP and CIDR arithmetic on values passed in, no sockets involved
    "net_ip_in_cidr",
    "net_ip_is_private",
    "net_ip_is_loopback",
    "net_ip_version",
    "net_ip_to_int",
    "net_ip_from_int",
    // Writes to stderr, which sandboxed code may use like log_* does
    "print_error",
];

/// Why a stdlib function is denied inside sandboxed (import) code, as a
/// short phrase for error messages, or None when the function is safe there.
/// Sandboxed code may only compute: module-level defaults with per-function
/// overrides, so all policy lives here and the checker asks one question.
pub fn sandbox_deny_reason(name: &str) -> Option<&'static str> {
    if SANDBOX_ALLOWED_FUNCTIONS.contains(&name) {
        return None;
    }
    if let Some((_, reason)) = SANDBOX_DENIED_FUNCTIONS.iter().find(|(denied, _)| *denied == name) {
        return Some(reason);
    }
    match get_stdlib_function(name).map(|f| &f.module) {
        // Touches the world: filesystem, network, databases, external
        // processes, and every file-format module that reads or writes files
        Some(
            StdlibModule::Fs
            | StdlibModule::Http
            | StdlibModule::Mcp
            | StdlibModule::Net
            | StdlibModule::Feed
            | StdlibModule::Email
            | StdlibModule::Process
            | StdlibModule::Archive
            | StdlibModule::Audio
            | StdlibModule::Image
            | StdlibModule::Pdf
            | StdlibModule::Xlsx
            | StdlibModule::Sqlite
            | StdlibModule::DataFusion
            | StdlibModule::Postgres
            | StdlibModule::Valkey,
        ) => Some(SANDBOX_TOUCHES_MACHINE),
        // Reads machine or invocation state: environment, system facts,
        // command-line arguments, and stdin
        Some(StdlibModule::Env | StdlibModule::Sys | StdlibModule::Args | StdlibModule::IO) => Some(SANDBOX_READS_MACHINE_STATE),
        // Process-global state visible across the whole program
        Some(StdlibModule::Cache | StdlibModule::I18n) => Some(SANDBOX_HOLDS_GLOBAL_STATE),
        // Seizes a resource the program owns: stdout, the terminal, or the
        // scheduler
        Some(StdlibModule::Print | StdlibModule::Tui | StdlibModule::Sched) => Some(SANDBOX_SEIZES_RESOURCE),
        // Everything else is pure computation on values passed in, plus
        // log_* (stderr cannot exfiltrate and keeps sandboxed code debuggable)
        Some(_) => None,
        None => None,
    }
}

/// Whether sandboxed (import) code may call this stdlib function.
pub fn is_sandbox_safe(name: &str) -> bool {
    sandbox_deny_reason(name).is_none()
}

pub fn is_stdlib_fn_async(name: &str) -> bool {
    // Per-function overrides of the module default
    if name == "time_sleep" || name == "tui_run" || name == "game_run" || name == "game3d_mesh_load" || name == "crypto_hash_file_sha256" || name == "crypto_hash_file_blake3" || name == "csv_write" || name == "sys_cpu_usage_percent" || name == "sys_process_cpu_percent" {
        return true;
    }
    if SYNC_STDLIB_FUNCTIONS.contains(&name) {
        return false;
    }
    matches!(
        get_stdlib_function(name).map(|f| &f.module),
        Some(StdlibModule::Fs | StdlibModule::Http | StdlibModule::IO | StdlibModule::Sqlite | StdlibModule::DataFusion | StdlibModule::Process | StdlibModule::Archive | StdlibModule::Net | StdlibModule::Email | StdlibModule::Postgres | StdlibModule::Image | StdlibModule::Pdf | StdlibModule::Xlsx | StdlibModule::Sched | StdlibModule::Valkey | StdlibModule::Mcp)
    )
}

/// Whether a stdlib function can exist in a browser. This mirrors the
/// cfg(not(target_arch = "wasm32")) gates in std_lib/mod.rs: a module the
/// wasm build does not contain cannot be called from a wasm program, and
/// `nailc --target=wasm` refuses such programs up front with a list of the
/// offending calls instead of a rustc error from deep inside a build.
pub fn is_stdlib_fn_wasm_safe(name: &str) -> bool {
    // Per-function exceptions inside otherwise portable modules: these few
    // touch the disk from modules that are pure computation everywhere else.
    if matches!(name, "csv_write" | "game_sprite_load") {
        return false;
    }
    return !matches!(
        get_stdlib_function(name).map(|f| &f.module),
        Some(
            StdlibModule::Archive
                | StdlibModule::Cache
                | StdlibModule::Code
                | StdlibModule::Compress
                | StdlibModule::Crypto
                | StdlibModule::DataFusion
                | StdlibModule::Email
                | StdlibModule::Env
                | StdlibModule::Fs
                | StdlibModule::Html
                | StdlibModule::Http
                | StdlibModule::IO
                | StdlibModule::Image
                | StdlibModule::Jwt
                | StdlibModule::Log
                | StdlibModule::Mcp
                | StdlibModule::Net
                | StdlibModule::Path
                | StdlibModule::Pdf
                | StdlibModule::Postgres
                | StdlibModule::Process
                | StdlibModule::Sched
                | StdlibModule::Sqlite
                | StdlibModule::Sys
                | StdlibModule::Term
                | StdlibModule::Time
                | StdlibModule::Tui
                | StdlibModule::Valkey
                | StdlibModule::Xlsx
        )
    );
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
        
        // PROCESS_Options struct
        m.insert("PROCESS_Options", StdlibTypeInfo {
            name: "PROCESS_Options".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("directory".to_string(), NailDataTypeDescriptor::String);
                fields.insert("environment".to_string(), NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)));
                fields.insert("input".to_string(), NailDataTypeDescriptor::String);
                fields.insert("timeout_seconds".to_string(), NailDataTypeDescriptor::Int);
                fields
            }
        });

        // PROCESS_Result struct
        m.insert("PROCESS_Result", StdlibTypeInfo {
            name: "PROCESS_Result".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("stdout".to_string(), NailDataTypeDescriptor::String);
                fields.insert("stderr".to_string(), NailDataTypeDescriptor::String);
                fields.insert("exit_code".to_string(), NailDataTypeDescriptor::Int);
                fields
            }
        });

        // URL_Parts struct
        m.insert("URL_Parts", StdlibTypeInfo {
            name: "URL_Parts".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("scheme".to_string(), NailDataTypeDescriptor::String);
                fields.insert("user".to_string(), NailDataTypeDescriptor::String);
                fields.insert("host".to_string(), NailDataTypeDescriptor::String);
                fields.insert("port".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("path".to_string(), NailDataTypeDescriptor::String);
                fields.insert("query".to_string(), NailDataTypeDescriptor::String);
                fields.insert("fragment".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

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

        // The Postgres structs, the same shape as the SQLite ones: a
        // connection is a handle, and a statement reports what it changed.
        m.insert("DB_Postgres", StdlibTypeInfo {
            name: "DB_Postgres".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("handle".to_string(), NailDataTypeDescriptor::String);
                fields.insert("database".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        m.insert("DB_PostgresResult", StdlibTypeInfo {
            name: "DB_PostgresResult".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("rows_affected".to_string(), NailDataTypeDescriptor::Int);
                fields
            }
        });

        m.insert("FS_Watcher", StdlibTypeInfo {
            name: "FS_Watcher".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("handle".to_string(), NailDataTypeDescriptor::String);
                fields.insert("path".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        m.insert("FEED_Entry", StdlibTypeInfo {
            name: "FEED_Entry".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("id".to_string(), NailDataTypeDescriptor::String);
                fields.insert("title".to_string(), NailDataTypeDescriptor::String);
                fields.insert("link".to_string(), NailDataTypeDescriptor::String);
                fields.insert("summary".to_string(), NailDataTypeDescriptor::String);
                fields.insert("published".to_string(), NailDataTypeDescriptor::Int);
                fields
            }
        });

        m.insert("FEED_Feed", StdlibTypeInfo {
            name: "FEED_Feed".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("title".to_string(), NailDataTypeDescriptor::String);
                fields.insert("link".to_string(), NailDataTypeDescriptor::String);
                fields.insert("description".to_string(), NailDataTypeDescriptor::String);
                fields.insert("entries".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct("FEED_Entry".to_string()))));
                fields
            }
        });

        // FS_Reader struct
        m.insert("FS_Reader", StdlibTypeInfo {
            name: "FS_Reader".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("handle".to_string(), NailDataTypeDescriptor::String);
                fields.insert("path".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        // GEO_Point struct
        m.insert("GEO_Point", StdlibTypeInfo {
            name: "GEO_Point".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("latitude".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("longitude".to_string(), NailDataTypeDescriptor::Float);
                fields
            }
        });

        // PROCESS_Handle struct
        m.insert("PROCESS_Handle", StdlibTypeInfo {
            name: "PROCESS_Handle".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("handle".to_string(), NailDataTypeDescriptor::String);
                fields.insert("command".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        // HTTP_Websocket struct
        m.insert("HTTP_Websocket", StdlibTypeInfo {
            name: "HTTP_Websocket".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("handle".to_string(), NailDataTypeDescriptor::String);
                fields.insert("url".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        // HTTP_Events struct
        m.insert("HTTP_Events", StdlibTypeInfo {
            name: "HTTP_Events".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("handle".to_string(), NailDataTypeDescriptor::String);
                fields.insert("url".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        // MCP_Tool struct
        m.insert("MCP_Tool", StdlibTypeInfo {
            name: "MCP_Tool".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("name".to_string(), NailDataTypeDescriptor::String);
                fields.insert("description".to_string(), NailDataTypeDescriptor::String);
                fields.insert("input_schema".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        // SCHED_Job struct
        m.insert("SCHED_Job", StdlibTypeInfo {
            name: "SCHED_Job".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("name".to_string(), NailDataTypeDescriptor::String);
                fields.insert("cron".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        // DB_Valkey struct
        m.insert("DB_Valkey", StdlibTypeInfo {
            name: "DB_Valkey".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("handle".to_string(), NailDataTypeDescriptor::String);
                fields.insert("url".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        // EMAIL_Attachment struct
        m.insert("EMAIL_Attachment", StdlibTypeInfo {
            name: "EMAIL_Attachment".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("path".to_string(), NailDataTypeDescriptor::String);
                fields.insert("file_name".to_string(), NailDataTypeDescriptor::String);
                fields.insert("mime_type".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        // EMAIL_Server struct
        m.insert("EMAIL_Server", StdlibTypeInfo {
            name: "EMAIL_Server".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("host".to_string(), NailDataTypeDescriptor::String);
                fields.insert("port".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("username".to_string(), NailDataTypeDescriptor::String);
                fields.insert("password".to_string(), NailDataTypeDescriptor::String);
                fields.insert("from_address".to_string(), NailDataTypeDescriptor::String);
                fields.insert("from_name".to_string(), NailDataTypeDescriptor::String);
                fields.insert("use_tls".to_string(), NailDataTypeDescriptor::Boolean);
                fields
            }
        });

        // STDLIB_Function struct
        m.insert("STDLIB_Function", StdlibTypeInfo {
            name: "STDLIB_Function".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("name".to_string(), NailDataTypeDescriptor::String);
                fields.insert("module".to_string(), NailDataTypeDescriptor::String);
                fields.insert("signature".to_string(), NailDataTypeDescriptor::String);
                fields.insert("description".to_string(), NailDataTypeDescriptor::String);
                fields.insert("example".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        // ARGS_Option struct
        m.insert("ARGS_Option", StdlibTypeInfo {
            name: "ARGS_Option".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("name".to_string(), NailDataTypeDescriptor::String);
                fields.insert("short".to_string(), NailDataTypeDescriptor::String);
                fields.insert("description".to_string(), NailDataTypeDescriptor::String);
                fields.insert("takes_value".to_string(), NailDataTypeDescriptor::Boolean);
                fields.insert("required".to_string(), NailDataTypeDescriptor::Boolean);
                fields
            }
        });

        // ARGS_Parsed struct
        m.insert("ARGS_Parsed", StdlibTypeInfo {
            name: "ARGS_Parsed".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("command".to_string(), NailDataTypeDescriptor::String);
                fields.insert("positional".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)));
                fields.insert("values".to_string(), NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)));
                fields.insert("flags".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)));
                fields
            }
        });

        // The machine learning structs. A fitted model is data, so every one
        // of these can be printed, stored as JSON and predicted with later.
        m.insert("ML_Split", StdlibTypeInfo {
            name: "ML_Split".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("train_features".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Float)))));
                fields.insert("train_labels".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)));
                fields.insert("test_features".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Float)))));
                fields.insert("test_labels".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)));
                fields
            }
        });

        m.insert("ML_Linear", StdlibTypeInfo {
            name: "ML_Linear".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("weights".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Float)));
                fields.insert("intercept".to_string(), NailDataTypeDescriptor::Float);
                fields
            }
        });

        m.insert("ML_Tree", StdlibTypeInfo {
            name: "ML_Tree".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("feature".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)));
                fields.insert("threshold".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Float)));
                fields.insert("left".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)));
                fields.insert("right".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)));
                fields.insert("prediction".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)));
                fields
            }
        });

        m.insert("ML_Clusters", StdlibTypeInfo {
            name: "ML_Clusters".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("centroids".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Float)))));
                fields.insert("assignments".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)));
                fields
            }
        });

        m.insert("ML_Scores", StdlibTypeInfo {
            name: "ML_Scores".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("true_positive".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("false_positive".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("true_negative".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("false_negative".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("accuracy".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("precision".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("recall".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("f1".to_string(), NailDataTypeDescriptor::Float);
                fields
            }
        });

        m.insert("ML_BoostConfig", StdlibTypeInfo {
            name: "ML_BoostConfig".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("trees".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("learning_rate".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("max_depth".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("min_samples_leaf".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("bins".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("lambda_l2".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("objective".to_string(), NailDataTypeDescriptor::Enum("ML_Objective".to_string()));
                fields.insert("early_stopping_rounds".to_string(), NailDataTypeDescriptor::Int);
                fields
            }
        });

        m.insert("ML_Boost", StdlibTypeInfo {
            name: "ML_Boost".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("base_score".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("roots".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)));
                fields.insert("feature".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)));
                fields.insert("threshold".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Float)));
                fields.insert("left".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)));
                fields.insert("right".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)));
                fields.insert("value".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Float)));
                fields.insert("gain".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Float)));
                fields.insert("default_left".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Boolean)));
                fields.insert("columns".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("objective".to_string(), NailDataTypeDescriptor::Enum("ML_Objective".to_string()));
                fields.insert("trees_used".to_string(), NailDataTypeDescriptor::Int);
                fields
            }
        });

        m.insert("ML_OneHot", StdlibTypeInfo {
            name: "ML_OneHot".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("categories".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)));
                fields.insert("columns".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Float)))));
                fields
            }
        });

        m.insert("ML_Forest", StdlibTypeInfo {
            name: "ML_Forest".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("roots".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)));
                fields.insert("feature".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)));
                fields.insert("threshold".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Float)));
                fields.insert("left".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)));
                fields.insert("right".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)));
                fields.insert("prediction".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Int)));
                fields.insert("columns".to_string(), NailDataTypeDescriptor::Int);
                fields
            }
        });

        m.insert("ML_Regression", StdlibTypeInfo {
            name: "ML_Regression".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("r_squared".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("mae".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("rmse".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("mape".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("median_ape".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("within_ten_percent".to_string(), NailDataTypeDescriptor::Float);
                fields
            }
        });

        // The terminal interface structs. A screen is data, so what a program
        // would draw can be checked without a terminal to draw it on.
        m.insert("TUI_Line", StdlibTypeInfo {
            name: "TUI_Line".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("text".to_string(), NailDataTypeDescriptor::String);
                fields.insert("color".to_string(), NailDataTypeDescriptor::Enum("TERM_Color".to_string()));
                fields.insert("bold".to_string(), NailDataTypeDescriptor::Boolean);
                fields.insert("selected".to_string(), NailDataTypeDescriptor::Boolean);
                fields
            }
        });

        m.insert("TUI_Screen", StdlibTypeInfo {
            name: "TUI_Screen".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("title".to_string(), NailDataTypeDescriptor::String);
                fields.insert("lines".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct("TUI_Line".to_string()))));
                fields.insert("status".to_string(), NailDataTypeDescriptor::String);
                fields.insert("quit".to_string(), NailDataTypeDescriptor::Boolean);
                fields
            }
        });

        m.insert("TUI_Event", StdlibTypeInfo {
            name: "TUI_Event".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("key".to_string(), NailDataTypeDescriptor::String);
                fields.insert("tick".to_string(), NailDataTypeDescriptor::Boolean);
                fields.insert("width".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("height".to_string(), NailDataTypeDescriptor::Int);
                fields
            }
        });

        // The game structs. A frame is data the same way a TUI screen is, so
        // what a game would draw can be checked without a window to draw in.
        m.insert("GAME_Config", StdlibTypeInfo {
            name: "GAME_Config".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("title".to_string(), NailDataTypeDescriptor::String);
                fields.insert("width".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("height".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("target_fps".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("pixel_size".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("physics_hz".to_string(), NailDataTypeDescriptor::Int);
                fields
            }
        });

        m.insert("GAME_Shape", StdlibTypeInfo {
            name: "GAME_Shape".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("kind".to_string(), NailDataTypeDescriptor::String);
                fields.insert("x_coordinate".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("y_coordinate".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("width".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("height".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("end_x".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("end_y".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("third_x".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("third_y".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("radius".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("thickness".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("color".to_string(), NailDataTypeDescriptor::String);
                fields.insert("text".to_string(), NailDataTypeDescriptor::String);
                fields.insert("size".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("sprite".to_string(), NailDataTypeDescriptor::Int);
                fields
            }
        });

        m.insert("GAME_Frame", StdlibTypeInfo {
            name: "GAME_Frame".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("shapes".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct("GAME_Shape".to_string()))));
                fields.insert("background".to_string(), NailDataTypeDescriptor::String);
                fields.insert("quit".to_string(), NailDataTypeDescriptor::Boolean);
                fields
            }
        });

        m.insert("GAME_Input", StdlibTypeInfo {
            name: "GAME_Input".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("keys_down".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)));
                fields.insert("keys_pressed".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)));
                fields.insert("mouse_x".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("mouse_y".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("mouse_down".to_string(), NailDataTypeDescriptor::Boolean);
                fields.insert("mouse_right".to_string(), NailDataTypeDescriptor::Boolean);
                fields.insert("scroll".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("delta_ms".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("touches".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Float)));
                fields
            }
        });

        m.insert("GAME3D_Camera", StdlibTypeInfo {
            name: "GAME3D_Camera".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("position_x".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("position_y".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("position_z".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("target_x".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("target_y".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("target_z".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("field_of_view".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("viewport_width".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("viewport_height".to_string(), NailDataTypeDescriptor::Float);
                fields
            }
        });

        // The linear algebra structs. A vector is its components and nothing
        // else, so a program can build one as a literal and read x and y back
        // without going through a function.
        m.insert("LINALG_Vec2", StdlibTypeInfo {
            name: "LINALG_Vec2".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("x_coordinate".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("y_coordinate".to_string(), NailDataTypeDescriptor::Float);
                fields
            }
        });

        m.insert("LINALG_Vec3", StdlibTypeInfo {
            name: "LINALG_Vec3".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("x_coordinate".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("y_coordinate".to_string(), NailDataTypeDescriptor::Float);
                fields.insert("z_coordinate".to_string(), NailDataTypeDescriptor::Float);
                fields
            }
        });

        m.insert("LINALG_Mat3", StdlibTypeInfo {
            name: "LINALG_Mat3".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("values".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Float)));
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
                fields.insert("cors_origins".to_string(), NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::String)));
                fields.insert("security_headers".to_string(), NailDataTypeDescriptor::Boolean);
                fields.insert("rate_limit_per_minute".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("rate_limit_message".to_string(), NailDataTypeDescriptor::String);
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
                fields.insert("same_site".to_string(), NailDataTypeDescriptor::Enum("HTTP_SameSite".to_string()));
                fields
            }
        });

        // HTTP_Part struct
        m.insert("HTTP_Part", StdlibTypeInfo {
            name: "HTTP_Part".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("name".to_string(), NailDataTypeDescriptor::String);
                fields.insert("value".to_string(), NailDataTypeDescriptor::String);
                fields.insert("file_path".to_string(), NailDataTypeDescriptor::String);
                fields.insert("file_name".to_string(), NailDataTypeDescriptor::String);
                fields.insert("content_type".to_string(), NailDataTypeDescriptor::String);
                fields
            }
        });

        // HTTP_Retry struct
        m.insert("HTTP_Retry", StdlibTypeInfo {
            name: "HTTP_Retry".to_string(),
            fields: {
                let mut fields = HashMap::new();
                fields.insert("attempts".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("initial_delay_ms".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("max_delay_ms".to_string(), NailDataTypeDescriptor::Int);
                fields.insert("timeout_ms".to_string(), NailDataTypeDescriptor::Int);
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
                fields.insert("body_path".to_string(), NailDataTypeDescriptor::String);
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
    /// Whether a program may leave this one out. An optional callback still
    /// has to match its signature when it is defined, but a program that
    /// never names it gets a stand-in that returns the argument named by
    /// `stand_in_argument`, counting from zero.
    pub optional_stand_in: Option<usize>,
}

lazy_static! {
    /// Stdlib function name -> the Nail functions it calls back into, in the
    /// order the transpiler passes them as trailing arguments.
    ///
    /// A signature here may name a type variable that also appears in the
    /// stdlib function's own parameters - `tui_run(initial: T)` binds `T` from
    /// its argument, and `view` and `update` are then checked against that
    /// binding. That is what lets a stdlib function call back into the program
    /// with a type only the program knows.
    pub static ref HANDLER_CALLBACKS: HashMap<&'static str, Vec<HandlerCallback>> = {
        let mut m = HashMap::new();
        m.insert("http_server", vec![HandlerCallback {
            function_name: "handle_request",
            parameter_types: vec![
                NailDataTypeDescriptor::Struct("HTTP_Request".to_string()),
                NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)),
            ],
            return_type: NailDataTypeDescriptor::Struct("HTTP_Response".to_string()),
            optional_stand_in: None,
        }]);
        m.insert("http_server_realtime", vec![
            HandlerCallback {
                function_name: "handle_request",
                parameter_types: vec![
                    NailDataTypeDescriptor::Struct("HTTP_Request".to_string()),
                    NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)),
                ],
                return_type: NailDataTypeDescriptor::Struct("HTTP_Response".to_string()),
                optional_stand_in: None,
            },
            // Called once per websocket text frame; the returned text goes back
            // to that one client, and the empty string means no reply.
            HandlerCallback {
                function_name: "handle_message",
                parameter_types: vec![
                    NailDataTypeDescriptor::String,
                    NailDataTypeDescriptor::HashMap(Box::new(NailDataTypeDescriptor::String), Box::new(NailDataTypeDescriptor::String)),
                ],
                return_type: NailDataTypeDescriptor::String,
                optional_stand_in: None,
            },
        ]);
        m.insert("tui_run", vec![
            HandlerCallback {
                function_name: "view",
                parameter_types: vec![NailDataTypeDescriptor::TypeVar("T".to_string(), vec![])],
                return_type: NailDataTypeDescriptor::Struct("TUI_Screen".to_string()),
                optional_stand_in: None,
            },
            HandlerCallback {
                function_name: "update",
                parameter_types: vec![NailDataTypeDescriptor::TypeVar("T".to_string(), vec![]), NailDataTypeDescriptor::Struct("TUI_Event".to_string())],
                return_type: NailDataTypeDescriptor::TypeVar("T".to_string(), vec![]),
                optional_stand_in: None,
            },
        ]);
        // game_run mirrors tui_run exactly: same callback names, same type
        // variable binding, a window instead of a terminal.
        m.insert("game_run", vec![
            HandlerCallback {
                function_name: "view",
                parameter_types: vec![NailDataTypeDescriptor::TypeVar("T".to_string(), vec![])],
                return_type: NailDataTypeDescriptor::Struct("GAME_Frame".to_string()),
                optional_stand_in: None,
            },
            HandlerCallback {
                function_name: "update",
                parameter_types: vec![NailDataTypeDescriptor::TypeVar("T".to_string(), vec![]), NailDataTypeDescriptor::Struct("GAME_Input".to_string())],
                return_type: NailDataTypeDescriptor::TypeVar("T".to_string(), vec![]),
                optional_stand_in: None,
            },
            // Only wanted by a game whose physics runs at its own rate: given
            // the two most recent states and how far the frame sits between
            // them, it returns the state to draw. Left out, the newer state
            // is drawn as it stands, which is the second argument.
            HandlerCallback {
                function_name: "blend",
                parameter_types: vec![
                    NailDataTypeDescriptor::TypeVar("T".to_string(), vec![]),
                    NailDataTypeDescriptor::TypeVar("T".to_string(), vec![]),
                    NailDataTypeDescriptor::Float,
                ],
                return_type: NailDataTypeDescriptor::TypeVar("T".to_string(), vec![]),
                optional_stand_in: Some(1),
            },
        ]);
        // Both schedulers dispatch to the same handle_job, so one function
        // hears every job and branches on the name it is given.
        m.insert("sched_run", vec![HandlerCallback {
            function_name: "handle_job",
            parameter_types: vec![NailDataTypeDescriptor::String],
            return_type: NailDataTypeDescriptor::Void,
            optional_stand_in: None,
        }]);
        m.insert("sched_every", vec![HandlerCallback {
            function_name: "handle_job",
            parameter_types: vec![NailDataTypeDescriptor::String],
            return_type: NailDataTypeDescriptor::Void,
            optional_stand_in: None,
        }]);
        m.insert("net_tcp_serve", vec![HandlerCallback {
            function_name: "handle_line",
            parameter_types: vec![NailDataTypeDescriptor::String],
            return_type: NailDataTypeDescriptor::String,
            optional_stand_in: None,
        }]);
        m.insert("net_udp_serve", vec![HandlerCallback {
            function_name: "handle_packet",
            parameter_types: vec![NailDataTypeDescriptor::String],
            return_type: NailDataTypeDescriptor::String,
            optional_stand_in: None,
        }]);
        // The Ok text is the tool's answer and the error is a tool error the
        // model reads, so the callback itself returns a result.
        m.insert("mcp_serve", vec![HandlerCallback {
            function_name: "handle_tool",
            parameter_types: vec![NailDataTypeDescriptor::String, NailDataTypeDescriptor::String],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::String)),
            optional_stand_in: None,
        }]);
        m
    };
}

/// The Nail functions a stdlib function dispatches to, if it takes any.
pub fn get_handler_callbacks(name: &str) -> Option<&'static Vec<HandlerCallback>> {
    HANDLER_CALLBACKS.get(name)
}

/// Whether a user function is the target of some stdlib callback. Such a
/// function is invoked from async glue, so it can never be emitted as a plain
/// sync Rust function.
pub fn is_handler_callback_target(function_name: &str) -> bool {
    HANDLER_CALLBACKS.values().flatten().any(|callback| callback.function_name == function_name)
}

lazy_static! {
    /// Enums the stdlib provides. Kept beside STDLIB_TYPES rather than inside
    /// it because an enum is a variant list, not a field map.
    pub static ref STDLIB_ENUMS: HashMap<&'static str, Vec<&'static str>> = {
        let mut m = HashMap::new();
        m.insert("CSV_Trim", vec!["None", "Headers", "Fields", "All"]);
        m.insert("CONVERT_FuelEconomy", vec!["LitersPer100Km", "MpgUs", "MpgImperial"]);
        m.insert(
            "CONVERT_Unit",
            vec![
                // length
                "Millimeter", "Centimeter", "Meter", "Kilometer", "Inch", "Foot", "Yard", "Mile", "NauticalMile",
                // mass
                "Milligram", "Gram", "Kilogram", "Tonne", "Ounce", "Pound", "Stone",
                // volume
                "Milliliter", "Liter", "Gallon", "Quart", "Pint", "Cup", "FluidOunce", "Tablespoon", "Teaspoon",
                // data, decimal then binary
                "Byte", "Kilobyte", "Megabyte", "Gigabyte", "Terabyte", "Kibibyte", "Mebibyte", "Gibibyte", "Tebibyte",
                // speed
                "MetersPerSecond", "KilometersPerHour", "MilesPerHour", "Knot",
                // area
                "SquareMeter", "SquareKilometer", "SquareFoot", "SquareMile", "Acre", "Hectare",
                // energy
                "Joule", "Kilojoule", "Megajoule", "WattHour", "KilowattHour", "Calorie", "Kilocalorie", "Btu",
                // power
                "Watt", "Kilowatt", "Megawatt", "Horsepower",
                // pressure
                "Pascal", "Kilopascal", "Megapascal", "Bar", "Psi", "Atmosphere", "MillimeterOfMercury",
                // frequency
                "Hertz", "Kilohertz", "Megahertz", "Gigahertz", "Rpm",
                // angle
                "Degree", "Radian", "Gradian", "Turn", "Arcminute", "Arcsecond",
                // temperature
                "Celsius", "Fahrenheit", "Kelvin",
            ],
        );
        m.insert("DRAW_Anchor", vec!["Start", "Middle", "End"]);
        m.insert("HTTP_Method", vec!["Get", "Post", "Put", "Delete", "Patch"]);
        m.insert("IMAGE_Mirror", vec!["LeftRight", "TopBottom"]);
        m.insert("IMAGE_Turn", vec!["Clockwise", "UpsideDown", "CounterClockwise"]);
        m.insert("HTTP_SameSite", vec!["Strict", "Lax", "None"]);
        m.insert("LOG_Level", vec!["Debug", "Info", "Warn", "Error"]);
        m.insert("ML_Objective", vec!["Squared", "Logistic"]);
        m.insert("TIME_Format", vec!["Unix", "UnixMillis", "ISO8601", "RFC3339", "RFC2822"]);
        m.insert("TIME_Nth", vec!["First", "Second", "Third", "Fourth", "Fifth", "Last"]);
        m.insert("TIME_Weekday", vec!["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"]);
        m.insert("VALIDATE_Country", vec!["UnitedStates", "Canada", "UnitedKingdom", "Germany", "France", "Netherlands", "Australia"]);
        m.insert(
            "TERM_Color",
            vec!["Black", "Red", "Green", "Yellow", "Blue", "Magenta", "Cyan", "White", "BrightBlack", "BrightRed", "BrightGreen", "BrightYellow", "BrightBlue", "BrightMagenta", "BrightCyan", "BrightWhite"],
        );
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

lazy_static! {
    /// Where each stdlib type lives in Rust, collected from the
    /// `custom_type_imports` the functions already declare rather than from a
    /// second hand-written table that could disagree with them. A drift test
    /// asserts no two functions import the same type from different paths.
    static ref STDLIB_TYPE_PATHS: HashMap<&'static str, &'static str> = {
        let mut paths = HashMap::new();
        for function in STDLIB_FUNCTIONS.values() {
            for (type_name, path) in &function.custom_type_imports {
                paths.insert(*type_name, *path);
            }
        }
        paths
    };
}

/// Where a stdlib type lives in Rust. This is what lets a program name a
/// stdlib type in its own function signatures without having called a stdlib
/// function that happens to import it - writing `view` and `update` before
/// ever calling `tui_run`, for instance.
///
/// The path comes from the `custom_type_imports` of whichever function ships
/// the type. A few types no function imports (the linalg structs) fall back
/// to the namespace their name carries: `LINALG_Vec2` starts with the linear
/// algebra module's namespace, so it lives at `nail::std_lib::linalg`. The
/// fallback only matches modules whose prefix is a single word, because a
/// family like `db_sqlite_` and `db_postgres_` shares one `DB_` namespace
/// across different Rust modules, which a name alone cannot tell apart.
pub fn stdlib_type_rust_path(type_name: &str) -> Option<String> {
    if !STDLIB_TYPES.contains_key(type_name) && !STDLIB_ENUMS.contains_key(type_name) {
        return None;
    }
    if let Some(path) = STDLIB_TYPE_PATHS.get(type_name) {
        return Some(format!("{}::{}", path, type_name));
    }
    for module in StdlibModule::all() {
        let prefix = module.name_prefix().trim_end_matches('_');
        if prefix.contains('_') {
            continue;
        }
        let namespace = prefix.to_uppercase() + "_";
        if type_name.starts_with(&namespace) {
            return Some(format!("nail::{}::{}", module.to_module_path(), type_name));
        }
    }
    return None;
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
        assert_enum_matches_registry::<crate::parser::std_lib::convert::CONVERT_FuelEconomy>("CONVERT_FuelEconomy");
        assert_enum_matches_registry::<crate::parser::std_lib::convert::CONVERT_Unit>("CONVERT_Unit");
        assert_enum_matches_registry::<crate::parser::std_lib::csv::CSV_Trim>("CSV_Trim");
        assert_enum_matches_registry::<crate::parser::std_lib::draw::DRAW_Anchor>("DRAW_Anchor");
        assert_enum_matches_registry::<crate::parser::std_lib::http::HTTP_Method>("HTTP_Method");
        assert_enum_matches_registry::<crate::parser::std_lib::http::HTTP_SameSite>("HTTP_SameSite");
        assert_enum_matches_registry::<crate::parser::std_lib::log::LOG_Level>("LOG_Level");
        assert_enum_matches_registry::<crate::parser::std_lib::term::TERM_Color>("TERM_Color");
        assert_enum_matches_registry::<crate::parser::std_lib::time::TIME_Format>("TIME_Format");
        assert_enum_matches_registry::<crate::parser::std_lib::time::TIME_Nth>("TIME_Nth");
        assert_enum_matches_registry::<crate::parser::std_lib::time::TIME_Weekday>("TIME_Weekday");
        #[cfg(feature = "image")]
        assert_enum_matches_registry::<crate::parser::std_lib::image::IMAGE_Turn>("IMAGE_Turn");
        #[cfg(feature = "image")]
        assert_enum_matches_registry::<crate::parser::std_lib::image::IMAGE_Mirror>("IMAGE_Mirror");
        assert_enum_matches_registry::<crate::parser::std_lib::ml::ML_Objective>("ML_Objective");
        assert_enum_matches_registry::<crate::parser::std_lib::validate::VALIDATE_Country>("VALIDATE_Country");
    }

    #[test]
    fn all_stdlib_enums_are_drift_tested() {
        let covered = ["CONVERT_FuelEconomy", "CONVERT_Unit", "CSV_Trim", "DRAW_Anchor", "HTTP_Method", "HTTP_SameSite", "IMAGE_Mirror", "IMAGE_Turn", "LOG_Level", "ML_Objective", "TERM_Color", "TIME_Format", "TIME_Nth", "TIME_Weekday", "VALIDATE_Country"];
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

    /// The type side of the same rule. Every stdlib struct and enum starts
    /// with a module family's namespace in upper case, so a Nail program can
    /// tell a library type from one of its own at a glance. The family is the
    /// first word of the prefix: db_sqlite and db_postgres types both wear
    /// `DB_`, the way ml_boost types wear `ML_`.
    #[test]
    fn stdlib_type_names_carry_their_namespace() {
        let namespaces: Vec<String> = StdlibModule::all()
            .iter()
            .map(|module| module.name_prefix().trim_end_matches('_').split('_').next().unwrap().to_uppercase() + "_")
            .collect();
        let named = STDLIB_TYPES.keys().copied().chain(STDLIB_ENUMS.keys().copied());
        for name in named {
            assert!(
                namespaces.iter().any(|namespace| name.starts_with(namespace.as_str())),
                "stdlib type '{}' must start with its library family's namespace, e.g. CSV_ or DB_",
                name
            );
        }
    }

    /// Every registered type must resolve to its Rust home, through a
    /// function's custom_type_imports or the single-word namespace fallback,
    /// so the transpiler can import it whenever a program names it.
    #[test]
    fn stdlib_types_all_resolve_to_a_rust_path() {
        let named = STDLIB_TYPES.keys().copied().chain(STDLIB_ENUMS.keys().copied());
        for name in named {
            assert!(
                stdlib_type_rust_path(name).is_some(),
                "stdlib type '{}' has no resolvable Rust path: no function imports it and no single-word module namespace matches it",
                name
            );
        }
    }

    /// Two functions importing one type from different paths would make the
    /// resolved import depend on registry iteration order.
    #[test]
    fn stdlib_type_imports_agree_on_paths() {
        let mut seen: HashMap<&str, &str> = HashMap::new();
        for (function_name, function) in STDLIB_FUNCTIONS.iter() {
            for (type_name, path) in &function.custom_type_imports {
                if let Some(existing) = seen.insert(*type_name, *path) {
                    assert_eq!(
                        existing, *path,
                        "stdlib type '{}' is imported from two different paths (second one seen at function '{}')",
                        type_name, function_name
                    );
                }
            }
        }
    }

    #[test]
    fn stdlib_types_match_real_structs() {
        assert_matches_registry::<crate::parser::std_lib::url::URL_Parts>("URL_Parts");
        assert_matches_registry::<crate::parser::std_lib::process::PROCESS_Options>("PROCESS_Options");
        assert_matches_registry::<crate::parser::std_lib::process::PROCESS_Result>("PROCESS_Result");
        assert_matches_registry::<crate::parser::std_lib::csv::CSV_Options>("CSV_Options");
        assert_matches_registry::<crate::parser::std_lib::csv::CSV_Reader>("CSV_Reader");
        assert_matches_registry::<crate::parser::std_lib::http::HTTP_Config>("HTTP_Config");
        assert_matches_registry::<crate::parser::std_lib::http::HTTP_Static>("HTTP_Static");
        assert_matches_registry::<crate::parser::std_lib::http::HTTP_Cookie>("HTTP_Cookie");
        assert_matches_registry::<crate::parser::std_lib::http::HTTP_Part>("HTTP_Part");
        assert_matches_registry::<crate::parser::std_lib::http::HTTP_Retry>("HTTP_Retry");
        assert_matches_registry::<crate::parser::std_lib::http::HTTP_Request>("HTTP_Request");
        assert_matches_registry::<crate::parser::std_lib::http::HTTP_Response>("HTTP_Response");
        assert_matches_registry::<crate::parser::std_lib::stdlib::STDLIB_Function>("STDLIB_Function");
        assert_matches_registry::<crate::parser::std_lib::args::ARGS_Option>("ARGS_Option");
        assert_matches_registry::<crate::parser::std_lib::args::ARGS_Parsed>("ARGS_Parsed");
        assert_matches_registry::<crate::parser::std_lib::ml::ML_Split>("ML_Split");
        assert_matches_registry::<crate::parser::std_lib::ml::ML_Linear>("ML_Linear");
        assert_matches_registry::<crate::parser::std_lib::ml::ML_Tree>("ML_Tree");
        assert_matches_registry::<crate::parser::std_lib::ml::ML_Clusters>("ML_Clusters");
        assert_matches_registry::<crate::parser::std_lib::ml::ML_Scores>("ML_Scores");
        assert_matches_registry::<crate::parser::std_lib::ml::ML_BoostConfig>("ML_BoostConfig");
        assert_matches_registry::<crate::parser::std_lib::ml::ML_Boost>("ML_Boost");
        assert_matches_registry::<crate::parser::std_lib::ml::ML_Regression>("ML_Regression");
        assert_matches_registry::<crate::parser::std_lib::ml::ML_OneHot>("ML_OneHot");
        assert_matches_registry::<crate::parser::std_lib::ml::ML_Forest>("ML_Forest");
        assert_matches_registry::<crate::parser::std_lib::tui::TUI_Line>("TUI_Line");
        assert_matches_registry::<crate::parser::std_lib::tui::TUI_Screen>("TUI_Screen");
        assert_matches_registry::<crate::parser::std_lib::tui::TUI_Event>("TUI_Event");
        assert_matches_registry::<crate::parser::std_lib::linalg::LINALG_Vec2>("LINALG_Vec2");
        assert_matches_registry::<crate::parser::std_lib::linalg::LINALG_Vec3>("LINALG_Vec3");
        assert_matches_registry::<crate::parser::std_lib::linalg::LINALG_Mat3>("LINALG_Mat3");
        #[cfg(feature = "email")]
        {
            assert_matches_registry::<crate::parser::std_lib::email::EMAIL_Server>("EMAIL_Server");
            assert_matches_registry::<crate::parser::std_lib::email::EMAIL_Attachment>("EMAIL_Attachment");
        }
        #[cfg(feature = "game")]
        {
            assert_matches_registry::<crate::parser::std_lib::game::GAME_Config>("GAME_Config");
            assert_matches_registry::<crate::parser::std_lib::game::GAME_Shape>("GAME_Shape");
            assert_matches_registry::<crate::parser::std_lib::game::GAME_Frame>("GAME_Frame");
            assert_matches_registry::<crate::parser::std_lib::game::GAME_Input>("GAME_Input");
            assert_matches_registry::<crate::parser::std_lib::game3d::GAME3D_Camera>("GAME3D_Camera");
        }
        #[cfg(feature = "valkey")]
        assert_matches_registry::<crate::parser::std_lib::valkey::DB_Valkey>("DB_Valkey");
        #[cfg(feature = "websocket")]
        assert_matches_registry::<crate::parser::std_lib::http::HTTP_Websocket>("HTTP_Websocket");
        assert_matches_registry::<crate::parser::std_lib::http::HTTP_Events>("HTTP_Events");
        assert_matches_registry::<crate::parser::std_lib::process::PROCESS_Handle>("PROCESS_Handle");
        assert_matches_registry::<crate::parser::std_lib::sched::SCHED_Job>("SCHED_Job");
        assert_matches_registry::<crate::parser::std_lib::geo::GEO_Point>("GEO_Point");
        assert_matches_registry::<crate::parser::std_lib::mcp::MCP_Tool>("MCP_Tool");
        #[cfg(feature = "postgres")]
        {
            assert_matches_registry::<crate::parser::std_lib::postgres::DB_Postgres>("DB_Postgres");
            assert_matches_registry::<crate::parser::std_lib::postgres::DB_PostgresResult>("DB_PostgresResult");
        }
        assert_matches_registry::<crate::parser::std_lib::fs::FS_Reader>("FS_Reader");
        assert_matches_registry::<crate::parser::std_lib::fs::FS_Watcher>("FS_Watcher");
        assert_matches_registry::<crate::parser::std_lib::feed::FEED_Entry>("FEED_Entry");
        assert_matches_registry::<crate::parser::std_lib::feed::FEED_Feed>("FEED_Feed");
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
        let covered = ["ARGS_Option", "ARGS_Parsed", "ML_Split", "ML_Linear", "ML_Tree", "ML_Clusters", "ML_Scores", "ML_BoostConfig", "ML_Boost", "ML_Regression", "ML_OneHot", "ML_Forest", "TUI_Line", "TUI_Screen", "TUI_Event", "LINALG_Vec2", "LINALG_Vec3", "LINALG_Mat3", "CSV_Options", "CSV_Reader", "HTTP_Config", "HTTP_Cookie", "HTTP_Static", "HTTP_Part", "HTTP_Retry", "HTTP_Request", "HTTP_Response", "DB_SQLite", "DB_Result", "DB_DataFusion", "DB_DataFusion_Result", "EMAIL_Server", "DB_Postgres", "DB_PostgresResult", "STDLIB_Function", "URL_Parts", "PROCESS_Options", "PROCESS_Result", "FS_Reader", "FS_Watcher", "FEED_Entry", "FEED_Feed", "DB_Valkey", "EMAIL_Attachment", "HTTP_Websocket", "HTTP_Events", "PROCESS_Handle", "SCHED_Job", "GEO_Point", "MCP_Tool", "GAME_Config", "GAME_Shape", "GAME_Frame", "GAME_Input", "GAME3D_Camera"];
        for type_name in STDLIB_TYPES.keys() {
            assert!(covered.contains(type_name), "STDLIB_TYPES entry '{}' has no drift test - add it to stdlib_types_match_real_structs", type_name);
        }
    }
}


