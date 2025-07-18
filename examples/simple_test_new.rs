use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let name: String = "Alice".to_string();
    println!("{}", name);
}
