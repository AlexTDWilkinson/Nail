use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let abs_result: f64 = std_lib::math::abs(-5.7);
    let sqrt_result: f64 = std_lib::math::sqrt(16.0);
    let pow_result: f64 = std_lib::math::pow(2.0, 3.0);
    let abs_str: String = std_lib::convert::to_string(abs_result);
    let sqrt_str: String = std_lib::convert::to_string(sqrt_result);
    let pow_str: String = std_lib::convert::to_string(pow_result);
    let mut results: Vec<String> = vec! [abs_str, " ".to_string(), sqrt_str, " ".to_string(), pow_str];
    let output: String = std_lib::string::concat(results);
    println!("{}", output);
}
