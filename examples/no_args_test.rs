use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let test: i64 = std_lib::time::now();
}
