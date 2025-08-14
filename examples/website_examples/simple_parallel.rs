use tokio;
use nail::std_lib;
use nail::print_macro;
use std::boxed::Box;
use rayon::prelude::*;
use rayon::iter::IntoParallelIterator;
use futures::future;

#[tokio::main]
async fn main() {
    let (_, _, _) = tokio::join!(
        async { print_macro!("Task 1: Starting download...".to_string()) },
        async { print_macro!("Task 2: Processing data...".to_string()) },
        async { print_macro!("Task 3: Sending email...".to_string()) }
    );
    print_macro!("All parallel tasks started!".to_string());
    tokio::spawn(async move {
        print_macro!("Background: Cleaning up old files...".to_string());
    });
    tokio::spawn(async move {
        print_macro!("Background: Updating cache...".to_string());
    });
    print_macro!("Main program continues immediately!".to_string());
}
