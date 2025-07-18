use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let result: i64 = if 1 > 0 { 1 } else { 0 };
}
