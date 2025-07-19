use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let numbers: Vec<i64> = vec! [10, 20, 30, 40, 50];
    std_lib::print::print("Testing array indexing functions...".to_string());
    std_lib::print::print("Numbers: [10, 20, 30, 40, 50]".to_string());
    let taken: Vec<i64> = std_lib::array::take(&numbers, 3);
    std_lib::print::print(std_lib::string::concat(vec! ["First 3 elements: ".to_string(), std_lib::string::from(std_lib::array::len(taken.clone()))]));
    let skipped: Vec<i64> = std_lib::array::skip(&numbers, 2);
    std_lib::print::print(std_lib::string::concat(vec! ["After skipping 2: ".to_string(), std_lib::string::from(std_lib::array::len(skipped.clone())), " elements remain".to_string()]));
    std_lib::print::print("\nDone testing!".to_string());
}