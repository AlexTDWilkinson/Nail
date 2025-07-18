use std::env as std_env;

pub fn get(key: String) -> Result<String, String> {
    std_env::var(key).map_err(|e| e.to_string())
}

pub fn set(key: String, value: String) -> Result<(), String> {
    std_env::set_var(key, value);
    Ok(())
}

pub fn args() -> Vec<String> {
    std_env::args().collect()
}