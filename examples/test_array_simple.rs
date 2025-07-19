use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let numbers: Vec<i64> = vec![10, 20, 30, 40, 50];
    std_lib::print::print("Testing array functions...".to_string());
    std_lib::print::print(std_lib::string::concat(vec!["Array length: ".to_string(), std_lib::string::from(std_lib::array::len(numbers.clone()))]));
    let taken: Vec<i64> = std_lib::array::take(&numbers, 3);
    std_lib::print::print(std_lib::string::concat(vec!["Took 3 elements, new length: ".to_string(), std_lib::string::from(std_lib::array::len(taken.clone()))]));
    let skipped: Vec<i64> = std_lib::array::skip(&numbers, 2);
    std_lib::print::print(std_lib::string::concat(vec!["Skipped 2 elements, new length: ".to_string(), std_lib::string::from(std_lib::array::len(skipped.clone()))]));
    std_lib::print::print("Done!".to_string());
}