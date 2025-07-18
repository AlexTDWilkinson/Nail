use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    #[derive(Debug, Clone)]
    struct Point {
        x: f64,
        y: f64,
    }
    #[derive(Debug, Clone)]
    struct Rectangle {
        top_left: Point,
        bottom_right: Point,
    }
    println!("{}", "Testing struct: prefix syntax".string_from());
}
