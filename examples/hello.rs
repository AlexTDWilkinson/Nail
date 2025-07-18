use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let message: String = "Hello from Nail!".string_from();
    let answer: i64 = 42;
    let pi: f64 = 3.14159;
}
