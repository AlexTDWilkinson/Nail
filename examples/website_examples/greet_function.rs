use tokio;
use nail::std_lib;
use nail::print_macro;
use std::boxed::Box;
use rayon::prelude::*;
use rayon::iter::IntoParallelIterator;
use futures::future;

#[tokio::main]
async fn main() {
    async fn greet(name: String) -> String {
        return std_lib::array::join(vec! ["Hello, ".to_string(), name.clone(), "!".to_string()], "".to_string()).await;
    }
    let message: String = greet("World".to_string()).await;
}
