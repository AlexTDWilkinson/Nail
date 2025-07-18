use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let damage: i64 = 42;
    let healing: f64 = 10.0;
    let upper_name: String = std_lib::string::to_uppercase("test".to_string());
    println!("{}", upper_name);
}
