use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let (parallel_result_0, parallel_result_1) = tokio::join!(
        async {
            println!("{}", "test1".string_from());
            ()
        },
        async {
            println!("{}", "test2".string_from());
            ()
        }
    );
}
