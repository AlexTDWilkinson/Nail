use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let test_array: Vec<i64> = vec! [10, 20, 30];
    println!("{}", string_from(std_lib::array::len(test_array.clone())));
}
