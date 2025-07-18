use tokio;
use Nail::std_lib;
use Nail::std_lib::string::from;

#[tokio::main]
async fn main() {
    let x_value: f64 = 16.0;
    let result: String =from(x_value.clone());
    println!("{}", result);
}
