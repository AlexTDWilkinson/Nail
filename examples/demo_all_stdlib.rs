use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let test_number: f64 = 16.0;
    let greeting_text: String = "Hello, Grug!".string_from();
    let number_array: Vec<i64> = vec![1, 2, 3, 4, 5];
    let string_array: Vec<String> = vec!["Nail".string_from(), "is".string_from(), "simple".string_from()];
    let sqrt_result: String = string_from(std_lib::math::sqrt(test_number.clone()));
    let abs_result: String = string_from(std_lib::math::abs(-5.7));
    let upper_text: String = std_lib::string::to_uppercase(greeting_text.clone());
    let array_size: String = string_from(std_lib::array::len(number_array.clone()));
    let joined_text: String = std_lib::array::join(string_array.clone(), " ".string_from());
    let time_now_str: String = string_from(std_lib::time::now());
    let path_result: String = std_lib::path::join("/home".string_from(), "grug".string_from());
    let output_parts: Vec<String> = vec![
        "Math: sqrt=".string_from(),
        sqrt_result.clone(),
        ", abs=".string_from(),
        abs_result.clone(),
        " | String: ".string_from(),
        upper_text.clone(),
        " | Array: len=".string_from(),
        array_size.clone(),
        ", joined=".string_from(),
        joined_text.clone(),
        " | Path: ".string_from(),
        path_result.clone(),
        " | Time: ".string_from(),
        time_now_str.clone(),
    ];
    println!("{:#?}", std_lib::string::concat(output_parts.clone()));
}
