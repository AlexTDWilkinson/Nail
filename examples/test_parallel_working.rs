use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let (parallel_result_0, parallel_result_1, parallel_result_2) = tokio::join!(
        async {
            println!("{}", "Fetching from API 1".to_string());
            ()
        },
        async {
            println!("{}", "Fetching from API 2".to_string());
            ()
        },
        async {
            println!("{}", "Fetching from API 3".to_string());
            ()
        }
    );
}
