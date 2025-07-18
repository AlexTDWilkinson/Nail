use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let test: String = std_lib::convert::to_string(42);
}
