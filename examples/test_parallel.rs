use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let first_value: i64 = 10;
    let second_value: i64 = 20;
    println!("{}", std_lib::convert::to_string(first_value.clone()));
    println!("{}", std_lib::convert::to_string(second_value.clone()));
}
