use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let sqrt_result: f64 = std_lib::math::sqrt(16.0);
    println!("{}", string_from(sqrt_result.clone()));
    let upper_result: String = std_lib::string::to_uppercase("hello".to_string());
    println!("{}", upper_result);
    let test_array: Vec<i64> = vec! [1, 2, 3];
    let array_length: i64 = std_lib::array::len(test_array.clone());
    println!("{}", string_from(array_length.clone()));
    let random_num: f64 = std_lib::math::random();
    println!("{}", string_from(random_num.clone()));
}
