use tokio;
use Nail::std_lib;
use Nail::std_lib::string::from;

#[tokio::main]
async fn main() {
    let num: i64 = 42;
    let test: String = from(num.clone());
}
