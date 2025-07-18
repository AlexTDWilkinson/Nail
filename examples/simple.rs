use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let greeting: String = "Hello, Nail!".to_string();
    let magic_number: i64 = 42;
    let result_number: i64 = magic_number + 10;
    let number_string: String = std_lib::convert::to_string(magic_number.clone());
    println!("{}", greeting);
    println!("{}", number_string);
    println!("{}", std_lib::convert::to_string(result_number.clone()));
}
