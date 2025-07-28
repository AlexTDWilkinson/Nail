use std::io::{self, BufRead};
use crate::parser::std_lib::print::print_no_newline;

/// Read a line from stdin
pub async fn read_line() -> Result<String, String> {
    let stdin = io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(_) => {
            // Remove trailing newline
            if line.ends_with('\n') {
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
            }
            Ok(line)
        }
        Err(e) => Err(format!("Failed to read line: {}", e))
    }
}

/// Read a line with a prompt
pub async fn read_line_prompt(prompt: String) -> Result<String, String> {
    print_no_newline(prompt).await;
    read_line().await
}

/// Read an integer from stdin
pub async fn read_int() -> Result<i64, String> {
    match read_line().await {
        Ok(line) => {
            match line.trim().parse::<i64>() {
                Ok(n) => Ok(n),
                Err(_) => Err(format!("Invalid integer: {}", line))
            }
        }
        Err(e) => Err(e)
    }
}

/// Read an integer with a prompt
pub async fn read_int_prompt(prompt: String) -> Result<i64, String> {
    print_no_newline(prompt).await;
    read_int().await
}

/// Read a float from stdin
pub async fn read_float() -> Result<f64, String> {
    match read_line().await {
        Ok(line) => {
            match line.trim().parse::<f64>() {
                Ok(n) => Ok(n),
                Err(_) => Err(format!("Invalid float: {}", line))
            }
        }
        Err(e) => Err(e)
    }
}

/// Read a float with a prompt
pub async fn read_float_prompt(prompt: String) -> Result<f64, String> {
    print_no_newline(prompt).await;
    read_float().await
}

