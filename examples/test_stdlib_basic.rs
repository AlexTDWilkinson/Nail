use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let greeting: String = "Hello, World!".to_string();
    let upper: String = std_lib::string::to_uppercase(greeting.clone());
    let x_value: f64 = std_lib::math::sqrt(16.0);
    let rounded: f64 = std_lib::math::round(3.7);
    let result: String = std_lib::convert::to_string(x_value.clone());
    println!("{}", result);
}
