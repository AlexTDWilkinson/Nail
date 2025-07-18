use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    println!("{}", "First line".string_from());
    println!("{}", "Second line".string_from());
    println!("{}", "Third line".string_from());
}
