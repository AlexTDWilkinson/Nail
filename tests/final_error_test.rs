use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    async fn divide(a: i64, b: i64) -> Result<i64, String> {
        if b == 0 {
            return Err(format!("[divide] {}", "Cannot divide by zero".to_string()))        }
        else {
            return Ok(a / b)        }
    }
    async fn main() -> () {
        println!("{}", "Test 1: dangerous with valid division".to_string());
        let result1: i64 =         divide(10, 2).await.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
        println!("{}", std_lib::string::concat(vec! ["10 / 2 = ".to_string(), string_from(result1)]));
        println!("{}", "\nTest 2: safe with division by zero".to_string());
        let result2: i64 = match         divide(10, 0).await { Ok(v) => v, Err(e) => (|err: String| -> i64 { println!("{}", std_lib::string::concat(vec! ["Caught error: ".to_string(), err]));         return -1 })(e) };
        println!("{}", std_lib::string::concat(vec! ["Result: ".to_string(), string_from(result2)]));
        println!("{}", "\nTest 3: Chained operations".to_string());
        let dividend: i64 = 20;
        let divisor: i64 = 0;
        let final_result: i64 = match         divide(dividend, divisor).await { Ok(v) => v, Err(e) => (|err: String| -> i64 { println!("{}", std_lib::string::concat(vec! ["Error in calculation: ".to_string(), err]));         return 0 })(e) };
        println!("{}", std_lib::string::concat(vec! ["Final result: ".to_string(), string_from(final_result)]));
    }
}
