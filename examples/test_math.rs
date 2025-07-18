use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let x_value: f64 = 16.0;
    let result: f64 = std_lib::math::sqrt(x_value.clone());
    let output: String = string_from(result.clone());
    println!("{}", output);
}
