use tokio;
use Nail::std_lib;
use Nail::std_lib::string::from;

#[tokio::main]
async fn main() {
    let abs_test: f64 = std_lib::math::abs(-5.7);
    let sqrt_test: f64 = std_lib::math::sqrt(16.0);
    let pow_test: f64 = std_lib::math::pow(2.0, 3.0);
    let round_test: f64 = std_lib::math::round(3.7);
    let floor_test: f64 = std_lib::math::floor(3.7);
    let ceil_test: f64 = std_lib::math::ceil(3.2);
    let min_test: f64 = std_lib::math::min(5.0, 3.0);
    let max_test: f64 = std_lib::math::max(5.0, 3.0);
    let random_test: f64 = std_lib::math::random();
    let result: String = std_lib::string::concat(vec! ["abs(-5.7) = ".to_string(),from(abs_test.clone()), ", sqrt(16) = ".to_string(),from(sqrt_test.clone()), ", pow(2,3) = ".to_string(),from(pow_test.clone()), ", round(3.7) = ".to_string(),from(round_test.clone()), ", floor(3.7) = ".to_string(),from(floor_test.clone()), ", ceil(3.2) = ".to_string(),from(ceil_test.clone()), ", min(5,3) = ".to_string(),from(min_test.clone()), ", max(5,3) = ".to_string(),from(max_test.clone()), ", random = ".to_string(),from(random_test.clone())]);
    println!("{}", result);
}
