use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let numbers: Vec<i64> = vec! [1, 2, 3, 4, 5];
    let doubled: Vec<i64> = std_lib::array_functional::map_int(numbers.clone(), |n: i64| -> i64 {     return n * 2 }).await;
    let sum: i64 = std_lib::array_functional::reduce_int(doubled.clone(), 0, |acc: i64, n: i64| -> i64 {     return acc + n }).await;
    std_lib::print::print(sum.clone());
}
