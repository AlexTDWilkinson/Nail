use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    #[derive(Debug, Clone)]
    struct Player {
        player_name: String,
        health: i64,
        level: i64,
    }
    let player: Player = Player { player_name: "Hero".to_string(),  health: 100,  level: 1 };
    #[derive(Debug, PartialEq)]
    enum Status {
        Active,
        Paused,
        Stopped,
    }
    let current: Status =     Status::Active;
    fn divide(num: i64, den: i64) -> Result<i64, String> {
        if den == 0 {
            return Err(format!("[divide] {}", "Cannot divide by zero!".to_string()))        }
        else {
            return Ok(num / den)        }
    }
    let result: i64 = match     divide(10, 2) { Ok(v) => v, Err(e) => (|e: String| -> i64 { std_lib::print::print(e.clone());     return 0 })(e) };
    let result_msg: Vec<String> = vec! ["10 / 2 = ".to_string(), std_lib::string::from(result.clone())];
    std_lib::print::print(std_lib::string::concat(result_msg.clone()));
    let safe_result: i64 =     divide(10, 2).unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let critical_result: i64 =     divide(100, 10).unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let name: String = "Alice".to_string();
    let age: i64 = 25;
    let score: f64 = 95.7;
    fn greet(person: String) -> String {
        let parts: Vec<String> = vec! ["Hello, ".to_string(), person.clone(), "!".to_string()];
        return std_lib::string::concat(parts.clone())    }
    std_lib::print::print(    greet(name.clone()));
    let (task1, task2, fast_calc) = tokio::join!(
        async { std_lib::string::from(42) },
        async { std_lib::time::now() },
        async { 100 * 50 }
    );
    std_lib::print::print(std_lib::string::concat(vec! ["Task 1 result: ".to_string(), task1.clone()]));
    std_lib::print::print(std_lib::string::concat(vec! ["Fast calculation: ".to_string(), std_lib::string::from(fast_calc.clone())]));
    let numbers: Vec<i64> = vec! [10, 20, 30, 40, 50];
    let names: Vec<String> = vec! ["Alice".to_string(), "Bob".to_string(), "Charlie".to_string()];
    let nums: Vec<i64> = std_lib::array_functional::range(1, 5);
    fn double_func(n: i64) -> i64 {
        return n * 2    }
    fn is_even_func(n: i64) -> bool {
        return n % 2 == 0    }
    fn add_func(acc: i64, n: i64) -> i64 {
        return acc + n    }
    fn square_func(n: i64) -> i64 {
        return n * n    }
    let doubled: Vec<i64> = std_lib::array_functional::map_int(nums.clone(), double_func.clone()).await;
    let evens: Vec<i64> = std_lib::array_functional::filter_int(nums.clone(), is_even_func.clone()).await;
    let sum: i64 = std_lib::array_functional::reduce_int(nums.clone(), 0, add_func.clone()).await;
    let sum_msg: Vec<String> = vec! ["Sum 1-5: ".to_string(), std_lib::string::from(sum.clone())];
    std_lib::print::print(std_lib::string::concat(sum_msg.clone()));
    let sum_squares: i64 = std_lib::array_functional::reduce_int(std_lib::array_functional::map_int(nums.clone(), square_func.clone()).await, 0, add_func.clone()).await;
    let squares_msg: Vec<String> = vec! ["Sum of squares: ".to_string(), std_lib::string::from(sum_squares.clone())];
    std_lib::print::print(std_lib::string::concat(squares_msg.clone()));
    if current ==     Status::Active {
        std_lib::print::print("System is active".to_string());
    }
    else {
        std_lib::print::print("System inactive".to_string());
    }
    let current_time: i64 = std_lib::time::now();
    let square_root: f64 = std_lib::math::sqrt(16.0);
    std_lib::print::print("Welcome to Nail programming!".to_string());
    let array_length: i64 = std_lib::array::len(numbers.clone());
    std_lib::print::print(std_lib::string::from(array_length.clone()));
    std_lib::print::print(std_lib::string::from(square_root.clone()));
    let final_message: String = "Nail makes parallel programming easy!".to_string();
}
