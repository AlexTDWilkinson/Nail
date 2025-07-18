use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    println!("{}", "hello".to_string());
    println!("{}", "world".to_string());
}
