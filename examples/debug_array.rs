use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let number_list: Vec<i64> = vec! [1, 2, 3, 4, 5];
    println!("{:#?}", std_lib::convert::to_string(std_lib::array::len(number_list.clone())));
}
