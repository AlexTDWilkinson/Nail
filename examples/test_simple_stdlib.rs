use Nail::std_lib::string::string_from;
use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let greeting: String = "Hello, World!".to_string();
    let upper: String = std_lib::string::to_uppercase(greeting.clone());
    println!("{}", upper);
    let len: i64 = std_lib::string::len(greeting.clone());
    let len_str: String = string_from(len.clone());
    println!("{}", len_str);
    let x_val: f64 = -5.7;
    let abs_x: f64 = std_lib::math::abs(x_val.clone());
    let abs_str: String = string_from(abs_x.clone());
    println!("{}", abs_str);
    let sqrt_val: f64 = std_lib::math::sqrt(16.0);
    let sqrt_str: String = string_from(sqrt_val.clone());
    println!("{}", sqrt_str);
    let numbers: Vec<i64> = vec! [1, 2, 3, 4, 5];
    let array_length: i64 = std_lib::array::len(numbers.clone());
    let length_str: String = string_from(array_length.clone());
    println!("{}", length_str);
    println!("{}", "Test complete".to_string());
}
