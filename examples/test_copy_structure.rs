use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let x_value: f64 = 16.0;
    let result: String = std_lib::convert::to_string(x_value.clone());
    println!("{}", result);
}
