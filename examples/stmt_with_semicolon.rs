use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    println!("{}", "hello".string_from());
    println!("{}", "world".string_from());
}
