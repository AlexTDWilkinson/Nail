use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    fn divide(a: i64, b: i64) -> Result<i64, String> {
        if b == 0 {
            return Err(format!("[divide] {}", "Cannot divide by zero".to_string()))        }
        else {
            return Ok(a / b)        }
    }
    let result: Result<i64, String> =     divide(10, 2);
    let result: i64 =     divide(10, 2).unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    std_lib::print::print(std_lib::string::from(result.clone()));
}
