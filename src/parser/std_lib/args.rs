use std::env;

/// Get command line argument by index (0 is the program name)
pub async fn get(index: i64) -> Result<String, String> {
    let args: Vec<String> = env::args().collect();
    
    if index < 0 {
        return Err(format!("Invalid argument index: {}", index));
    }
    
    let idx = index as usize;
    if idx >= args.len() {
        return Err(format!("Argument index {} out of bounds (have {} arguments)", index, args.len()));
    }
    
    Ok(args[idx].clone())
}

/// Check if a flag exists (e.g., --flag or -f)
pub async fn flag(name: String) -> bool {
    let args: Vec<String> = env::args().collect();
    
    // Check for both long form (--name) and short form (-n)
    let long_form = format!("--{}", name);
    let short_form = if name.len() == 1 {
        format!("-{}", name)
    } else {
        // For multi-character names, check if there's a common short form
        format!("-{}", name.chars().next().unwrap_or('?'))
    };
    
    args.iter().any(|arg| arg == &long_form || (name.len() == 1 && arg == &short_form))
}

/// Get value for a named argument (e.g., --name=value or --name value)
pub async fn value(name: String) -> Result<String, String> {
    let args: Vec<String> = env::args().collect();
    
    let flag_with_equals = format!("--{}=", name);
    let flag_without_equals = format!("--{}", name);
    
    for (i, arg) in args.iter().enumerate() {
        // Check for --name=value format
        if arg.starts_with(&flag_with_equals) {
            let value = arg[flag_with_equals.len()..].to_string();
            if value.is_empty() {
                return Err(format!("Empty value for argument --{}", name));
            }
            return Ok(value);
        }
        
        // Check for --name value format (value is next argument)
        if arg == &flag_without_equals {
            if i + 1 < args.len() {
                let next_arg = &args[i + 1];
                // Make sure the next arg is not another flag
                if !next_arg.starts_with('-') {
                    return Ok(next_arg.clone());
                }
            }
            return Err(format!("No value provided for argument --{}", name));
        }
    }
    
    Err(format!("Argument --{} not found", name))
}

/// Get the number of command line arguments (including program name)
pub async fn count() -> i64 {
    env::args().count() as i64
}