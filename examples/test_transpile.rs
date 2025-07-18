use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let greeting: String = "Hello from Nail!".string_from();
    let number: i64 = 42;
    let pi: f64 = 3.14;
}
