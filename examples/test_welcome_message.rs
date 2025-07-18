use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    #[derive(Debug, Clone)]
    struct Position {
        x: f64,
        y: f64,
    }
    #[derive(Debug, Clone)]
    struct Player {
        player_name: String,
        player_level: i64,
        health_points: f64,
        position: Position,
    }
    #[derive(Debug, PartialEq)]
    enum GameState {
        MainMenu,
        Playing,
        GameOver,
    }
    let name: String = "Alice".to_string();
    let age: i64 = 25;
    let score: f64 = 95.7;
    let greeting: String = string_from(age.clone());
    let timestamp: i64 = std_lib::time::now();
    println!("{}", "Hello from Nail!".to_string());
    let result: i64 = age + 10;
    let average: f64 = score + 85.3 / 2.0;
    let (parallel_result_0, parallel_result_1, parallel_result_2, parallel_result_3) = tokio::join!(
        async {
            let task1: String = string_from(42);
            ()
        },
        async {
            let task2: i64 = std_lib::time::now();
            ()
        },
        async {
            println!("{}", "Running in parallel!".to_string());
            ()
        },
        async {
            let fast_calc: i64 = 100 * 50;
            ()
        }
    );
    let numbers: Vec<i64> = vec! [10, 20, 30, 40, 50];
    let array_length: i64 = std_lib::array::len(numbers.clone());
    let current_time: i64 = std_lib::time::now();
    let square_root: f64 = std_lib::math::sqrt(16.0);
    println!("{}", "Welcome to Nail programming!".to_string());
    println!("{}", string_from(array_length.clone()));
    println!("{}", string_from(square_root.clone()));
    let final_message: String = "Nail makes parallel programming easy!".to_string();
}
