use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let test_number: f64 = 16.0;
    let greeting_text: String = "Hello, Grug!".to_string();
    let number_array: Vec<i64> = vec! [1, 2, 3, 4, 5];
    let string_array: Vec<String> = vec! ["Nail".to_string(), "is".to_string(), "simple".to_string()];
    let sqrt_result: String = std_lib::convert::to_string(std_lib::math::sqrt(test_number.clone()));
    let abs_result: String = std_lib::convert::to_string(std_lib::math::abs(-5.7));
    let upper_text: String = std_lib::string::to_uppercase(greeting_text.clone());
    let array_size: String = std_lib::convert::to_string(std_lib::array::len(number_array.clone()));
    let joined_text: String = std_lib::array::join(string_array.clone(), " ".to_string());
    let time_now_str: String = std_lib::convert::to_string(std_lib::time::now());
    let path_result: String = std_lib::path::join("/home".to_string(), "grug".to_string());
    let output_parts: Vec<String> = vec! ["Math: sqrt=".to_string(), sqrt_result.clone(), ", abs=".to_string(), abs_result.clone(), " | String: ".to_string(), upper_text.clone(), " | Array: len=".to_string(), array_size.clone(), ", joined=".to_string(), joined_text.clone(), " | Path: ".to_string(), path_result.clone(), " | Time: ".to_string(), time_now_str.clone()];
    println!("{:#?}", std_lib::string::concat(output_parts.clone()));
}
