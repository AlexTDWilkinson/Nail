use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let num: i64 = 42;
    let test: String = std_lib::convert::to_string(num.clone());
}
