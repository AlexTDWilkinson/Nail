use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let sqrt_result: f64 = std_lib::math::sqrt(16.0);
    println!("{}", std_lib::convert::to_string(sqrt_result.clone()));
    let upper_result: String = std_lib::string::to_uppercase("hello".to_string());
    println!("{}", upper_result);
    let test_array: Vec<i64> = vec! [1, 2, 3];
    let array_length: i64 = std_lib::array::len(test_array.clone());
    println!("{}", std_lib::convert::to_string(array_length.clone()));
    let random_num: f64 = std_lib::math::random();
    println!("{}", std_lib::convert::to_string(random_num.clone()));
}
