use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let result: i64 = 5;
    if result > 0 {
        println!("{}", "positive".to_string());
    }
    println!("{}", "done".to_string());
}
