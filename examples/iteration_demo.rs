use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let numbers: Vec<i64> = std_lib::array_functional::range(1, 10);
    let doubled: Vec<i64> = std_lib::array_functional::map_int(numbers.clone(), |n: i64| -> i64 {     return n * 2 }).await;
    let evens: Vec<i64> = std_lib::array_functional::filter_int(numbers.clone(), |n: i64| -> bool {     return n % 2 == 0 }).await;
    let sum: i64 = std_lib::array_functional::reduce_int(numbers.clone(), 0, |acc: i64, n: i64| -> i64 {     return acc + n }).await;
    std_lib::array_functional::each_int(numbers.clone(), |n: i64| -> () { println!("{}", std_lib::string::concat(vec! ["Number: ".to_string(), std_lib::convert::to_string(n.clone())])) }).await;
    println!("{}", "Iteration in Nail is functional, not imperative!".to_string());
}
