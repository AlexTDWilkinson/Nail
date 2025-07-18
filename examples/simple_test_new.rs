use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let name: String = "Alice".string_from();
    println!("{}", name);
}
