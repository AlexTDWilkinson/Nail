use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let input_value: f64 = 16.0;
    println!("{}", string_from(input_value.clone()));
}
