use tokio;
use Nail::std_lib;
use Nail::std_lib::string::from;

#[tokio::main]
async fn main() {
    let x_value: f64 = 16.0;
    let sqrt_result: f64 = std_lib::math::sqrt(x_value.clone());
    let abs_result: f64 = std_lib::math::abs(-5.7);
    let power_result: f64 = std_lib::math::pow(2.0, 3.0);
    let greeting: String = "Hello, World!".to_string();
    let upper: String = std_lib::string::to_uppercase(greeting.clone());
    let numbers: Vec<i64> = vec![1, 2, 3, 4, 5];
    let arr_len: i64 = std_lib::array::len(numbers.clone());
    let final_str: String = from(sqrt_result.clone());
    println!("{:#?}", final_str);
}
