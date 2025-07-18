use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let message: String = "Hello, Nail without c!".to_string();
    println!("{}", message);
    let base: i64 = 10;
    let squared: i64 = base * base;
    println!("{}", std_lib::convert::to_string(squared.clone()));
    let age: i64 = 21;
    let status: String = if age >= 18 { "adult".to_string() } else { "minor".to_string() };
    println!("{}", status);
    let result: i64 = if squared > 50 { squared * 2 } else { squared / 2 };
    println!("{}", std_lib::convert::to_string(result.clone()));
    println!("{}", "All done!".to_string());
}
