use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let message: String = "Hello from pure Nail!".to_string();
    println!("{}", message);
    let number: i64 = 42;
    let doubled: i64 = number * 2;
    println!("{}", string_from(doubled.clone()));
    let age: i64 = 25;
    if age >= 18 {
        println!("{}", "You are an adult".to_string());
    }
    else {
        println!("{}", "You are a minor".to_string());
    }
    println!("{}", "Program complete".to_string());
}
