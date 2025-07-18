use tokio;
use Nail::std_lib;
use Nail::std_lib::string::from;

#[tokio::main]
async fn main() {
    let number_list: Vec<i64> = vec! [1, 2, 3, 4, 5];
    println!("{}",from(std_lib::array::len(number_list.clone())));
}
