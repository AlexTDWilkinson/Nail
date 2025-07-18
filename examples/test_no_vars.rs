use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let message: String = "Hello from pure Nail!".string_from();
    println!("{}", message);
    let number: i64 = 42;
    let doubled: i64 = number * 2;
    println!("{}", string_from(doubled.clone()));
    let age: i64 = 25;
    let status: String = if age >= 18 { "adult".string_from() } else { "minor".string_from() };
    println!("{}", status);
}
