use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let input_number: f64 = 16.0;
    println!("{}", string_from(std_lib::math::sqrt(input_number.clone())));
}
