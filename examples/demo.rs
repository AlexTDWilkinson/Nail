use Nail::std_lib::string::from;
use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let first_number: i64 = 42;
    let second_number: i64 = 24;
    let result_sum: i64 = first_number + second_number;
    let temperature: f64 = 98.6;
    let celsius: f64 = temperature - 32.0 * 5.0 / 9.0;
    let user_name: String = "Grug".to_string();
    let (parallel_result_0, parallel_result_1, parallel_result_2) = tokio::join!(
        async {
            println!("{}", "Processing task one".to_string());
            ()
        },
        async {
            println!("{}", "Processing task two".to_string());
            ()
        },
        async {
            println!("{}", "Processing task three".to_string());
            ()
        }
    );
    let current_time: i64 = std_lib::time::now();
    let square_root: f64 = std_lib::math::sqrt(16.0);
    println!("{}", user_name);
    println!("{}",from(result_sum.clone()));
    println!("{}",from(celsius.clone()));
    println!("{}",from(square_root.clone()));
}
