use tokio;
use nail::std_lib;
use nail::print_macro;
use std::boxed::Box;
use rayon::prelude::*;
use rayon::iter::IntoParallelIterator;
use futures::future;

#[tokio::main]
async fn main() {
    async fn divide(numerator: i64, denominator: i64) -> Result<i64, String> {
        if denominator.clone() == 0 {
            return Err(format!("[divide] {}", "Cannot divide by zero!".to_string()));
        } else {
            return Ok(numerator.clone() / denominator.clone());
        }
    }
    let result: i64 = divide(10, 2).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    print_macro!("10 / 2 = ".to_string());
    print_macro!(result.clone());
    async fn handle_div_error(err: String) -> i64 {
        print_macro!("Error occurred: ".to_string());
;
                print_macro!(err.clone());
;
                return 0;
    }
    let safe_result: i64 = match divide(10, 0).await { Ok(v) => v, Err(e) => (handle_div_error.clone())(e).await };
    print_macro!("Result with error handling: ".to_string());
    print_macro!(safe_result.clone());
}
