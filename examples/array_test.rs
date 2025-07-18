use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let arr: Vec<i64> = vec! [1, 2, 3];
    let len: i64 = std_lib::array::len(arr.clone());
}
