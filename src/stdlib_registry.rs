use lazy_static::lazy_static;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct StdlibFunction {
    /// The Rust path to call this function (e.g., "std_lib::http::http_server_start")
    pub rust_path: String,
    /// Whether this function is async
    pub is_async: bool,
}

lazy_static! {
    pub static ref STDLIB_FUNCTIONS: HashMap<&'static str, StdlibFunction> = {
        let mut m = HashMap::new();
        
        // HTTP functions
        m.insert("http_server_start", StdlibFunction {
            rust_path: "std_lib::http::http_server_start".to_string(),
            is_async: true
            
        });
        
        m.insert("http_server_route", StdlibFunction {
            rust_path: "std_lib::http::http_server_route".to_string(),
            is_async: true
            
        });
        
        // File system functions (future)
        m.insert("fs_read", StdlibFunction {
            rust_path: "std_lib::fs::read_file".to_string(),
            is_async: true
            
        });
        
        m.insert("fs_write", StdlibFunction {
            rust_path: "std_lib::fs::write_file".to_string(),
            is_async: true
            
        });
        
        // JSON functions (future)
        m.insert("json_parse", StdlibFunction {
            rust_path: "std_lib::json::parse".to_string(),
            is_async: false
            
        });
        
        m.insert("json_stringify", StdlibFunction {
            rust_path: "std_lib::json::stringify".to_string(),
            is_async: false
            
        });
        
        // Type conversion functions
        m.insert("to_string", StdlibFunction {
            rust_path: "std_lib::convert::to_string".to_string(),
            is_async: false
            
        });
        
        m.insert("to_int", StdlibFunction {
            rust_path: "std_lib::convert::to_int".to_string(),
            is_async: false
            
        });
        
        m.insert("to_float", StdlibFunction {
            rust_path: "std_lib::convert::to_float".to_string(),
            is_async: false
            
        });
        
        // IO functions
        m.insert("print", StdlibFunction {
            rust_path: "println!".to_string(),  // Macro, needs special handling
            is_async: false
            
        });
        
        m.insert("eprintln", StdlibFunction {
            rust_path: "eprintln!".to_string(),  // Macro, needs special handling
            is_async: false
            
        });
        
        // String manipulation
        m.insert("string_concat", StdlibFunction {
            rust_path: "std_lib::string::concat".to_string(),
            is_async: false
            
        });
        
        m.insert("string_split", StdlibFunction {
            rust_path: "std_lib::string::split".to_string(),
            is_async: false
            
        });
        
        m.insert("string_trim", StdlibFunction {
            rust_path: "std_lib::string::trim".to_string(),
            is_async: false
            
        });
        
        m.insert("string_contains", StdlibFunction {
            rust_path: "std_lib::string::contains".to_string(),
            is_async: false
            
        });
        
        m.insert("string_replace", StdlibFunction {
            rust_path: "std_lib::string::replace".to_string(),
            is_async: false
            
        });
        
        m.insert("string_len", StdlibFunction {
            rust_path: "std_lib::string::len".to_string(),
            is_async: false
            
        });
        
        m.insert("string_to_uppercase", StdlibFunction {
            rust_path: "std_lib::string::to_uppercase".to_string(),
            is_async: false
            
        });
        
        m.insert("string_to_lowercase", StdlibFunction {
            rust_path: "std_lib::string::to_lowercase".to_string(),
            is_async: false
            
        });
        
        // Array operations
        m.insert("array_len", StdlibFunction {
            rust_path: "std_lib::array::len".to_string(),
            is_async: false
            
        });
        
        m.insert("array_push", StdlibFunction {
            rust_path: "std_lib::array::push".to_string(),
            is_async: false
            
        });
        
        m.insert("array_pop", StdlibFunction {
            rust_path: "std_lib::array::pop".to_string(),
            is_async: false
            
        });
        
        m.insert("array_contains", StdlibFunction {
            rust_path: "std_lib::array::contains".to_string(),
            is_async: false
            
        });
        
        m.insert("array_join", StdlibFunction {
            rust_path: "std_lib::array::join".to_string(),
            is_async: false
            
        });
        
        m.insert("array_sort", StdlibFunction {
            rust_path: "std_lib::array::sort".to_string(),
            is_async: false
            
        });
        
        m.insert("array_reverse", StdlibFunction {
            rust_path: "std_lib::array::reverse".to_string(),
            is_async: false
            
        });
        
        // Functional array operations
        m.insert("map_int", StdlibFunction {
            rust_path: "std_lib::array_functional::map_int".to_string(),
            is_async: true
            
        });
        m.insert("map_float", StdlibFunction {
            rust_path: "std_lib::array_functional::map_float".to_string(),
            is_async: true
            
        });
        m.insert("map_string", StdlibFunction {
            rust_path: "std_lib::array_functional::map_string".to_string(),
            is_async: true
            
        });
        m.insert("filter_int", StdlibFunction {
            rust_path: "std_lib::array_functional::filter_int".to_string(),
            is_async: true
            
        });
        m.insert("filter_float", StdlibFunction {
            rust_path: "std_lib::array_functional::filter_float".to_string(),
            is_async: true
            
        });
        m.insert("filter_string", StdlibFunction {
            rust_path: "std_lib::array_functional::filter_string".to_string(),
            is_async: true
            
        });
        m.insert("reduce_int", StdlibFunction {
            rust_path: "std_lib::array_functional::reduce_int".to_string(),
            is_async: true
            
        });
        m.insert("reduce_float", StdlibFunction {
            rust_path: "std_lib::array_functional::reduce_float".to_string(),
            is_async: true
            
        });
        m.insert("reduce_string", StdlibFunction {
            rust_path: "std_lib::array_functional::reduce_string".to_string(),
            is_async: true
            
        });
        m.insert("each_int", StdlibFunction {
            rust_path: "std_lib::array_functional::each_int".to_string(),
            is_async: true
            
        });
        m.insert("each_float", StdlibFunction {
            rust_path: "std_lib::array_functional::each_float".to_string(),
            is_async: true
            
        });
        m.insert("each_string", StdlibFunction {
            rust_path: "std_lib::array_functional::each_string".to_string(),
            is_async: true
            
        });
        m.insert("range", StdlibFunction {
            rust_path: "std_lib::array_functional::range".to_string(),
            is_async: false
            
        });
        m.insert("range_exclusive", StdlibFunction {
            rust_path: "std_lib::array_functional::range_exclusive".to_string(),
            is_async: false
            
        });
        
        // Math functions
        m.insert("math_abs", StdlibFunction {
            rust_path: "std_lib::math::abs".to_string(),
            is_async: false
            
        });
        
        m.insert("math_sqrt", StdlibFunction {
            rust_path: "std_lib::math::sqrt".to_string(),
            is_async: false
            
        });
        
        m.insert("math_pow", StdlibFunction {
            rust_path: "std_lib::math::pow".to_string(),
            is_async: false
            
        });
        
        m.insert("math_round", StdlibFunction {
            rust_path: "std_lib::math::round".to_string(),
            is_async: false
            
        });
        
        m.insert("math_floor", StdlibFunction {
            rust_path: "std_lib::math::floor".to_string(),
            is_async: false
            
        });
        
        m.insert("math_ceil", StdlibFunction {
            rust_path: "std_lib::math::ceil".to_string(),
            is_async: false
            
        });
        
        m.insert("math_min", StdlibFunction {
            rust_path: "std_lib::math::min".to_string(),
            is_async: false
            
        });
        
        m.insert("math_max", StdlibFunction {
            rust_path: "std_lib::math::max".to_string(),
            is_async: false
            
        });
        
        m.insert("math_random", StdlibFunction {
            rust_path: "std_lib::math::random".to_string(),
            is_async: false
            
        });
        
        // Time functions
        m.insert("time_now", StdlibFunction {
            rust_path: "std_lib::time::now".to_string(),
            is_async: false
            
        });
        
        m.insert("time_sleep", StdlibFunction {
            rust_path: "std_lib::time::sleep".to_string(),
            is_async: true
            
        });
        
        m.insert("time_format", StdlibFunction {
            rust_path: "std_lib::time::format".to_string(),
            is_async: false
            
        });
        
        // Environment functions
        m.insert("env_get", StdlibFunction {
            rust_path: "std_lib::env::get".to_string(),
            is_async: false
            
        });
        
        m.insert("env_set", StdlibFunction {
            rust_path: "std_lib::env::set".to_string(),
            is_async: false
            
        });
        
        m.insert("env_args", StdlibFunction {
            rust_path: "std_lib::env::args".to_string(),
            is_async: false
            
        });
        
        // Process functions
        m.insert("process_exit", StdlibFunction {
            rust_path: "std_lib::process::exit".to_string(),
            is_async: false
            
        });
        
        m.insert("process_run", StdlibFunction {
            rust_path: "std_lib::process::run".to_string(),
            is_async: true
            
        });
        
        // HTTP client functions
        m.insert("http_get", StdlibFunction {
            rust_path: "std_lib::http::get".to_string(),
            is_async: true
            
        });
        
        m.insert("http_post", StdlibFunction {
            rust_path: "std_lib::http::post".to_string(),
            is_async: true
            
        });
        
        // Database functions (future)
        m.insert("db_connect", StdlibFunction {
            rust_path: "std_lib::db::connect".to_string(),
            is_async: true
            
        });
        
        m.insert("db_query", StdlibFunction {
            rust_path: "std_lib::db::query".to_string(),
            is_async: true
            
        });
        
        m.insert("db_execute", StdlibFunction {
            rust_path: "std_lib::db::execute".to_string(),
            is_async: true
            
        });
        
        // Crypto functions
        m.insert("crypto_hash", StdlibFunction {
            rust_path: "std_lib::crypto::hash".to_string(),
            is_async: false
            
        });
        
        m.insert("crypto_encrypt", StdlibFunction {
            rust_path: "std_lib::crypto::encrypt".to_string(),
            is_async: false
            
        });
        
        m.insert("crypto_decrypt", StdlibFunction {
            rust_path: "std_lib::crypto::decrypt".to_string(),
            is_async: false
            
        });
        
        // Path/File system utilities
        m.insert("path_join", StdlibFunction {
            rust_path: "std_lib::path::join".to_string(),
            is_async: false
            
        });
        
        m.insert("path_exists", StdlibFunction {
            rust_path: "std_lib::path::exists".to_string(),
            is_async: false
            
        });
        
        m.insert("fs_create_dir", StdlibFunction {
            rust_path: "std_lib::fs::create_dir".to_string(),
            is_async: true
            
        });
        
        m.insert("fs_remove_file", StdlibFunction {
            rust_path: "std_lib::fs::remove_file".to_string(),
            is_async: true
            
        });
        
        m.insert("fs_copy", StdlibFunction {
            rust_path: "std_lib::fs::copy".to_string(),
            is_async: true
            
        });
        
        m.insert("fs_move", StdlibFunction {
            rust_path: "std_lib::fs::move_file".to_string(),
            is_async: true
            
        });
        
        // Regex functions
        m.insert("regex_match", StdlibFunction {
            rust_path: "std_lib::regex::match_pattern".to_string(),
            is_async: false
            
        });
        
        m.insert("regex_replace", StdlibFunction {
            rust_path: "std_lib::regex::replace".to_string(),
            is_async: false
            
        });
        
        // Base64 encoding/decoding
        m.insert("base64_encode", StdlibFunction {
            rust_path: "std_lib::encoding::base64_encode".to_string(),
            is_async: false
            
        });
        
        m.insert("base64_decode", StdlibFunction {
            rust_path: "std_lib::encoding::base64_decode".to_string(),
            is_async: false
            
        });
        
        // URL encoding/decoding
        m.insert("url_encode", StdlibFunction {
            rust_path: "std_lib::encoding::url_encode".to_string(),
            is_async: false
            
        });
        
        m.insert("url_decode", StdlibFunction {
            rust_path: "std_lib::encoding::url_decode".to_string(),
            is_async: false
            
        });
        
        // Error handling functions
        m.insert("safe", StdlibFunction {
            rust_path: "std_lib::error::safe".to_string(),
            is_async: false
            
        });
        
        m.insert("dangerous", StdlibFunction {
            rust_path: "std_lib::error::dangerous".to_string(),
            is_async: false
            
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