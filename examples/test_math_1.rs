use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    println!("{}", std_lib::string::concat(vec! ["abs(-5.7) = ".to_string(), string_from(std_lib::math::abs(-5.7)), ", sqrt(16) = ".to_string(), string_from(std_lib::math::sqrt(16.0)), ", pow(2,3) = ".to_string(), string_from(std_lib::math::pow(2.0, 3.0))]));
}
