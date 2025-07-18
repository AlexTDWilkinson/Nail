use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let first_value: i64 = 10;
    let second_value: i64 = 20;
    println!("{}", string_from(first_value.clone()));
    println!("{}", string_from(second_value.clone()));
}
