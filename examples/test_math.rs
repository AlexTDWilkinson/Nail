use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let x_value: f64 = 16.0;
    let result: f64 = std_lib::math::sqrt(x_value.clone());
    let output: String = std_lib::convert::to_string(result.clone());
    println!("{}", output);
}
