use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let global_var: i64 = 42;
    let test: i64 = global_var;
}
