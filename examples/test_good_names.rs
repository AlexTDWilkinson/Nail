use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let input_number: f64 = 16.0;
    println!("{}", std_lib::convert::to_string(std_lib::math::sqrt(input_number.clone())));
}
