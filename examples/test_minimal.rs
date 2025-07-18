use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let input_value: f64 = 16.0;
    println!("{}", std_lib::convert::to_string(input_value.clone()));
}
