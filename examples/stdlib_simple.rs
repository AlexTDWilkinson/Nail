use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let result: String = std_lib::string::concat(vec! ["Math: ".to_string(), std_lib::convert::to_string(std_lib::math::sqrt(16.0)), " | String: ".to_string(), std_lib::string::to_uppercase("hello".to_string())]);
    println!("{}", result);
}
