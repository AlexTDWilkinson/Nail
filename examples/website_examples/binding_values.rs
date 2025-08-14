use tokio;
use nail::std_lib;
use nail::print_macro;
use std::boxed::Box;
use rayon::prelude::*;
use rayon::iter::IntoParallelIterator;
use futures::future;

#[tokio::main]
async fn main() {
    let name: String = "Alice".to_string();
    let age: i64 = 30;
    let scores: Vec<i64> = vec! [95, 87, 92];
    print_macro!(name.clone());
    print_macro!(age.clone());
    print_macro!(scores.clone());
}
