use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let message: String = "Hello withoutprefix!".to_string();
    println!("{}", message);
    let number: i64 = 42;
    let doubled: i64 = number * 2;
    println!("{}", std_lib::convert::to_string(doubled.clone()));
}
