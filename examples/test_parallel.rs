use tokio;
use Nail::std_lib;
use Nail::std_lib::string::from;

#[tokio::main]
async fn main() {
    let first_value: i64 = 10;
    let second_value: i64 = 20;
    println!("{}", from(first_value.clone()));
    println!("{}", from(second_value.clone()));
}
