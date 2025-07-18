use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let num: i64 = 42;
    let test: String = string_from(num.clone());
}
