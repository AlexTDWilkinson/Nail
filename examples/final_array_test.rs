use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let test_array: Vec<i64> = vec! [10, 20, 30];
    println!("{}", std_lib::convert::to_string(std_lib::array::len(test_array.clone())));
}
