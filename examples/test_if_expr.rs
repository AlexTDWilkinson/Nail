use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let number: i64 = 5;
    let result: i64 = if number > 0 { 1 } else { 0 };
    println!("{}", std_lib::convert::to_string(result.clone()));
}
