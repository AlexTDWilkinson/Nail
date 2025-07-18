use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let (parallel_result_0, parallel_result_1, parallel_result_2) = tokio::join!(
        async {
            println!("{}", "Fetching from API 1".string_from());
            ()
        },
        async {
            println!("{}", "Fetching from API 2".string_from());
            ()
        },
        async {
            println!("{}", "Fetching from API 3".string_from());
            ()
        }
    );
}
