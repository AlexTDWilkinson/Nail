use tokio;
use Nail::std_lib;
use Nail::std_lib::string::from;

#[tokio::main]
async fn main() {
    let input_value: f64 = 16.0;
    println!("{}", from(input_value.clone()));
}
