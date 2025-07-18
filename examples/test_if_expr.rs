use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let number: i64 = 5;
    let result: i64 = if number > 0 { 1 } else { 0 };
    println!("{}", string_from(result.clone()));
}
