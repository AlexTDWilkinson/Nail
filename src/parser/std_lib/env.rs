use std::env as std_env;

pub async fn get(key: String) -> Result<String, String> {
    std_env::var(&key).map_err(|_| format!("env_get: environment variable '{}' is not set", key))
}

pub async fn set(key: String, value: String) -> Result<(), String> {
    std_env::set_var(key, value);
    Ok(())
}

pub async fn args() -> Vec<String> {
    std_env::args().collect()
}