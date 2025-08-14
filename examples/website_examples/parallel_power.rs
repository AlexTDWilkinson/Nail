use tokio;
use nail::std_lib;
use nail::print_macro;
use std::boxed::Box;
use rayon::prelude::*;
use rayon::iter::IntoParallelIterator;
use futures::future;

#[tokio::main]
async fn main() {
    async fn fetch_user_data() -> String {
        std_lib::time::sleep(0.1).await;
;
                return r#"{name: "Alice", id: 42}"#.to_string();
    }
    async fn fetch_posts() -> i64 {
        std_lib::time::sleep(0.1).await;
;
                return 15;
    }
    async fn calculate_stats() -> i64 {
        std_lib::time::sleep(0.1).await;
;
                return 1337;
    }
    let start_time: i64 = std_lib::time::now().await;
    let (user_data, post_count, total_views) = tokio::join!(
        async { fetch_user_data().await },
        async { fetch_posts().await },
        async { calculate_stats().await }
    );
    let end_time: i64 = std_lib::time::now().await;
    print_macro!("Fetched in parallel:".to_string());
    print_macro!(user_data.clone());
    print_macro!("Posts: ".to_string());
    print_macro!(post_count.clone());
    print_macro!("Views: ".to_string());
    print_macro!(total_views.clone());
    print_macro!("Time taken (ms): ".to_string());
    print_macro!(end_time.clone() - start_time.clone());
}
