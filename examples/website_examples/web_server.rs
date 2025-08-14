use tokio;
use nail::std_lib;
use nail::print_macro;
use std::boxed::Box;
use rayon::prelude::*;
use rayon::iter::IntoParallelIterator;
use futures::future;

#[tokio::main]
async fn main() {
    let port: i64 = 3000;
    let html: String = "<h1>Hello from Nail!</h1>".to_string();
    std_lib::http::http_server_start(port.clone(), html.clone()).await;
}
