use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let name: String = "Alice".to_string();
    let age: i64 = 25;
    let score: f64 = 95.7;
    let greeting: String = std_lib::convert::to_string(age.clone());
    let timestamp: i64 = std_lib::time::now();
    println!("{:#?}", "Hello from Nail!".to_string());
    let result: i64 = age + 10;
    let average: f64 = score + 85.3 / 2.0;
    let (parallel_result_0, parallel_result_1, parallel_result_2, parallel_result_3) = tokio::join!(
        async {
            let task1: String = std_lib::convert::to_string(42);
            ()
        },
        async {
            let task2: i64 = std_lib::time::now();
            ()
        },
        async {
            println!("{:#?}", "Running in parallel!".to_string());
            ()
        },
        async {
            let fast_calc: i64 = 100 * 50;
            ()
        }
    );
    println!("{:#?}", "Welcome to Nail programming!".to_string());
    let full_name: String = std_lib::string::concat(vec! [name.clone(), " Johnson".to_string()]);
    let message: String = std_lib::string::concat(vec! ["User ".to_string(), name.clone(), " scored ".to_string(), std_lib::convert::to_string(score.clone())]);
    let bonus: i64 = age * 2;
    let current_time: i64 = std_lib::time::now();
    let name_length: i64 = std_lib::string::len(name.clone());
    let final_message: String = "Nail makes parallel programming easy!".to_string();
}
