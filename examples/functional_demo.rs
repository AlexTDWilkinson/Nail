use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let numbers: Vec<i64> = std_lib::array_functional::range(1, 10);
    let squared: Vec<i64> = std_lib::array_functional::map_int(numbers.clone(), |n: i64| -> i64 {     return n * n }).await;
    println!("{:#?}", "Squared first 10 numbers".to_string());
    let evens: Vec<i64> = std_lib::array_functional::filter_int(numbers.clone(), |n: i64| -> bool {     return n % 2 == 0 }).await;
    println!("{:#?}", "Found even numbers".to_string());
    let sum: i64 = std_lib::array_functional::reduce_int(numbers.clone(), 0, |acc: i64, n: i64| -> i64 {     return acc + n }).await;
    let product: i64 = std_lib::array_functional::reduce_int(std_lib::array_functional::range(1, 5), 1, |acc: i64, n: i64| -> i64 {     return acc * n }).await;
    println!("{:#?}", std_lib::string::concat(vec! ["Sum 1-10: ".to_string(), string_from(sum.clone())]));
    println!("{:#?}", std_lib::string::concat(vec! ["Product 1-5: ".to_string(), string_from(product.clone())]));
    let doubled_evens: Vec<i64> = std_lib::array_functional::map_int(std_lib::array_functional::filter_int(numbers.clone(), |n: i64| -> bool {     return n % 2 == 0 }).await, |n: i64| -> i64 {     return n * 2 }).await;
    let sum_of_squares: i64 = std_lib::array_functional::reduce_int(std_lib::array_functional::map_int(std_lib::array_functional::range(1, 5), |n: i64| -> i64 {     return n * n }).await, 0, |acc: i64, n: i64| -> i64 {     return acc + n }).await;
    println!("{:#?}", std_lib::string::concat(vec! ["Sum of squares 1-5: ".to_string(), string_from(sum_of_squares.clone())]));
    println!("{:#?}", "Done! No loops were used.".to_string());
}
