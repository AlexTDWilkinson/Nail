use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    println!("{}", "First line".to_string());
    println!("{}", "Second line".to_string());
    println!("{}", "Third line".to_string());
}
