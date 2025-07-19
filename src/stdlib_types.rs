use crate::lexer::NailDataTypeDescriptor;
use std::collections::HashMap;
use lazy_static::lazy_static;

pub struct StdlibParameter {
    pub name: String,
    pub param_type: NailDataTypeDescriptor,
}

pub struct StdlibFunctionType {
    pub parameters: Vec<StdlibParameter>,
    pub return_type: NailDataTypeDescriptor,
}

lazy_static! {
    pub static ref STDLIB_FUNCTION_TYPES: HashMap<&'static str, StdlibFunctionType> = {
        let mut m = HashMap::new();
        
        // Math functions - all take floats and return floats
        m.insert("math_abs", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Float }],
            return_type: NailDataTypeDescriptor::Float,
        });
        m.insert("math_sqrt", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Float }],
            return_type: NailDataTypeDescriptor::Float,
        });
        m.insert("math_pow", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "base".to_string(), param_type: NailDataTypeDescriptor::Float },
                StdlibParameter { name: "exponent".to_string(), param_type: NailDataTypeDescriptor::Float }
            ],
            return_type: NailDataTypeDescriptor::Float,
        });
        m.insert("math_round", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Float }],
            return_type: NailDataTypeDescriptor::Float,
        });
        m.insert("math_floor", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Float }],
            return_type: NailDataTypeDescriptor::Float,
        });
        m.insert("math_ceil", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Float }],
            return_type: NailDataTypeDescriptor::Float,
        });
        m.insert("math_min", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "a".to_string(), param_type: NailDataTypeDescriptor::Float },
                StdlibParameter { name: "b".to_string(), param_type: NailDataTypeDescriptor::Float }
            ],
            return_type: NailDataTypeDescriptor::Float,
        });
        m.insert("math_max", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "a".to_string(), param_type: NailDataTypeDescriptor::Float },
                StdlibParameter { name: "b".to_string(), param_type: NailDataTypeDescriptor::Float }
            ],
            return_type: NailDataTypeDescriptor::Float,
        });
        m.insert("math_random", StdlibFunctionType {
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Float,
        });
        
        // String functions
        m.insert("string_concat", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "strings".to_string(), param_type: NailDataTypeDescriptor::ArrayString }],
            return_type: NailDataTypeDescriptor::String,
        });
        m.insert("string_split", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "text".to_string(), param_type: NailDataTypeDescriptor::String },
                StdlibParameter { name: "delimiter".to_string(), param_type: NailDataTypeDescriptor::String }
            ],
            return_type: NailDataTypeDescriptor::ArrayString,
        });
        m.insert("string_trim", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "text".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::String,
        });
        m.insert("string_contains", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "text".to_string(), param_type: NailDataTypeDescriptor::String },
                StdlibParameter { name: "pattern".to_string(), param_type: NailDataTypeDescriptor::String }
            ],
            return_type: NailDataTypeDescriptor::Boolean,
        });
        m.insert("string_replace", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "text".to_string(), param_type: NailDataTypeDescriptor::String },
                StdlibParameter { name: "from".to_string(), param_type: NailDataTypeDescriptor::String },
                StdlibParameter { name: "to".to_string(), param_type: NailDataTypeDescriptor::String }
            ],
            return_type: NailDataTypeDescriptor::String,
        });
        m.insert("string_len", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "text".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::Int,
        });
        m.insert("string_to_uppercase", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "text".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::String,
        });
        m.insert("string_to_lowercase", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "text".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::String,
        });
        
        // Array functions - array_len is generic, always returns Int
        m.insert("array_len", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::Unknown }], // Accept any array type
            return_type: NailDataTypeDescriptor::Int,
        });
        m.insert("array_join", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::ArrayString },
                StdlibParameter { name: "separator".to_string(), param_type: NailDataTypeDescriptor::String }
            ],
            return_type: NailDataTypeDescriptor::String,
        });
        
        // Safe array indexing functions - need specific types for type checker
        // array_get for integers
        m.insert("array_get", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::ArrayInt },
                StdlibParameter { name: "index".to_string(), param_type: NailDataTypeDescriptor::Int }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Int)),
        });
        m.insert("array_first", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::ArrayInt }],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Int)),
        });
        m.insert("array_last", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::ArrayInt }],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Int)),
        });
        m.insert("array_slice", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::ArrayInt },
                StdlibParameter { name: "start".to_string(), param_type: NailDataTypeDescriptor::Int },
                StdlibParameter { name: "end".to_string(), param_type: NailDataTypeDescriptor::Int }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::ArrayInt)),
        });
        m.insert("array_take", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::ArrayInt },
                StdlibParameter { name: "n".to_string(), param_type: NailDataTypeDescriptor::Int }
            ],
            return_type: NailDataTypeDescriptor::ArrayInt,
        });
        m.insert("array_skip", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::ArrayInt },
                StdlibParameter { name: "n".to_string(), param_type: NailDataTypeDescriptor::Int }
            ],
            return_type: NailDataTypeDescriptor::ArrayInt,
        });
        
        // Type conversion - string_from replaced to_string
        m.insert("string_from", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Unknown }], // Accept any type
            return_type: NailDataTypeDescriptor::String,
        });
        m.insert("int_from", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Int)),
        });
        m.insert("float_from", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Float)),
        });
        m.insert("to_int", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::Int,
        });
        m.insert("to_float", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::Float,
        });
        
        // Time functions
        m.insert("time_now", StdlibFunctionType {
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Int,
        });
        m.insert("time_format", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "timestamp".to_string(), param_type: NailDataTypeDescriptor::Int },
                StdlibParameter { name: "format".to_string(), param_type: NailDataTypeDescriptor::String }
            ],
            return_type: NailDataTypeDescriptor::String,
        });
        
        // Path functions
        m.insert("path_join", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "path1".to_string(), param_type: NailDataTypeDescriptor::String },
                StdlibParameter { name: "path2".to_string(), param_type: NailDataTypeDescriptor::String }
            ],
            return_type: NailDataTypeDescriptor::String,
        });
        m.insert("path_exists", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "path".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::Boolean,
        });
        
        // Environment
        m.insert("env_args", StdlibFunctionType {
            parameters: vec![],
            return_type: NailDataTypeDescriptor::ArrayString,
        });
        m.insert("env_get", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "key".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::String,
        });
        m.insert("env_set", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "key".to_string(), param_type: NailDataTypeDescriptor::String },
                StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::String }
            ],
            return_type: NailDataTypeDescriptor::Void,
        });
        
        // Process
        m.insert("process_exit", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "code".to_string(), param_type: NailDataTypeDescriptor::Int }],
            return_type: NailDataTypeDescriptor::Void,
        });
        
        // HTTP
        m.insert("http_server_start", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "port".to_string(), param_type: NailDataTypeDescriptor::Int },
                StdlibParameter { name: "handler".to_string(), param_type: NailDataTypeDescriptor::String }
            ],
            return_type: NailDataTypeDescriptor::Void,
        });
        
        // IO
        m.insert("print", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "message".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::Void,
        });
        
        m.insert("print_no_newline", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "message".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::Void,
        });
        
        m.insert("io_read_line", StdlibFunctionType {
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::String)),
        });
        
        m.insert("io_read_line_prompt", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "prompt".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::String)),
        });
        
        m.insert("io_read_int", StdlibFunctionType {
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Int)),
        });
        
        m.insert("io_read_int_prompt", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "prompt".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Int)),
        });
        
        m.insert("io_read_float", StdlibFunctionType {
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Float)),
        });
        
        m.insert("io_read_float_prompt", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "prompt".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Float)),
        });
        
        m.insert("print_clear_screen", StdlibFunctionType {
            parameters: vec![],
            return_type: NailDataTypeDescriptor::Void,
        });
        
        m.insert("print_debug", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "value".to_string(), param_type: NailDataTypeDescriptor::Unknown }],
            return_type: NailDataTypeDescriptor::Void,
        });
        
        // File system functions
        m.insert("fs_read", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "path".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::String)),
        });
        
        m.insert("fs_write", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "path".to_string(), param_type: NailDataTypeDescriptor::String },
                StdlibParameter { name: "content".to_string(), param_type: NailDataTypeDescriptor::String }
            ],
            return_type: NailDataTypeDescriptor::Result(Box::new(NailDataTypeDescriptor::Void)),
        });
        
        // Markdown functions
        m.insert("markdown_to_html", StdlibFunctionType {
            parameters: vec![StdlibParameter { name: "markdown".to_string(), param_type: NailDataTypeDescriptor::String }],
            return_type: NailDataTypeDescriptor::String,
        });
        
        // safe() handles errors by providing a fallback
        m.insert("safe", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "result".to_string(), param_type: NailDataTypeDescriptor::Unknown }, // Any result type
                StdlibParameter { name: "error_handler".to_string(), param_type: NailDataTypeDescriptor::Unknown } // Lambda function
            ],
            return_type: NailDataTypeDescriptor::Unknown, // Returns the base type
        });
        
        // dangerous() unwraps a result type, propagating the error if it fails
        m.insert("dangerous", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "result".to_string(), param_type: NailDataTypeDescriptor::Unknown } // Any result type
            ],
            return_type: NailDataTypeDescriptor::Unknown, // Returns the base type
        });
        
        // expect() is semantically identical to dangerous but with different intent
        m.insert("expect", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "result".to_string(), param_type: NailDataTypeDescriptor::Unknown } // Any result type
            ],
            return_type: NailDataTypeDescriptor::Unknown, // Returns the base type
        });
        
        // e() creates an error value from a string
        m.insert("e", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "message".to_string(), param_type: NailDataTypeDescriptor::String }
            ],
            return_type: NailDataTypeDescriptor::Error,
        });
        
        // Functional array operations
        m.insert("range", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "start".to_string(), param_type: NailDataTypeDescriptor::Int },
                StdlibParameter { name: "end".to_string(), param_type: NailDataTypeDescriptor::Int }
            ],
            return_type: NailDataTypeDescriptor::ArrayInt,
        });
        
        m.insert("map_int", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::ArrayInt },
                StdlibParameter { name: "function".to_string(), param_type: NailDataTypeDescriptor::Unknown } // Lambda function
            ],
            return_type: NailDataTypeDescriptor::ArrayInt,
        });
        
        m.insert("filter_int", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::ArrayInt },
                StdlibParameter { name: "function".to_string(), param_type: NailDataTypeDescriptor::Unknown } // Lambda function
            ],
            return_type: NailDataTypeDescriptor::ArrayInt,
        });
        
        m.insert("reduce_int", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::ArrayInt },
                StdlibParameter { name: "initial".to_string(), param_type: NailDataTypeDescriptor::Int },
                StdlibParameter { name: "function".to_string(), param_type: NailDataTypeDescriptor::Unknown } // Lambda function
            ],
            return_type: NailDataTypeDescriptor::Int,
        });
        
        m.insert("reduce_struct", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::Unknown }, // Accept any array of structs
                StdlibParameter { name: "initial".to_string(), param_type: NailDataTypeDescriptor::Unknown }, // Struct type
                StdlibParameter { name: "function".to_string(), param_type: NailDataTypeDescriptor::Unknown } // Lambda function
            ],
            return_type: NailDataTypeDescriptor::Unknown, // Returns same type as initial struct
        });
        
        m.insert("each_int", StdlibFunctionType {
            parameters: vec![
                StdlibParameter { name: "array".to_string(), param_type: NailDataTypeDescriptor::ArrayInt },
                StdlibParameter { name: "function".to_string(), param_type: NailDataTypeDescriptor::Unknown } // Lambda function
            ],
            return_type: NailDataTypeDescriptor::Void,
        });
        
        m
    };
}

pub fn get_stdlib_function_type(name: &str) -> Option<&'static StdlibFunctionType> {
    STDLIB_FUNCTION_TYPES.get(name)
}