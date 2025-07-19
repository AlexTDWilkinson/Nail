use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    fn complex_calc(a: i64, b: i64, c: i64) -> Result<i64, String> {
        if a == 0 {
            return Err(format!("[complex_calc] {}", "a cannot be zero".to_string()))        }
        else if b == 0 {
            return Err(format!("[complex_calc] {}", "b cannot be zero".to_string()))        }
        else {
            if c > 10 {
                return Ok(a + b * c)            }
            else {
                return Ok(a + b + c)            }
        }
    }
}
