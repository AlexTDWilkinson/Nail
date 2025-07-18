use Nail::std_lib::string::string_from;
use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let name: String = "Alice".to_string();
    let age: i64 = 25;
    let score: f64 = 95.7;
    let is_active: bool = true;
    #[derive(Debug, Clone)]
    struct Player {
        name: String,
        health: i64,
        level: i64,
        experience: f64,
    }
    let player: Player = Player { name: "Hero".to_string(),  health: 100,  level: 1,  experience: 0.0 };
    #[derive(Debug, PartialEq)]
    enum GameState {
        MainMenu,
        Playing,
        Paused,
        GameOver,
    }
    let current_state: GameState =     GameState::Playing;
    async fn divide(num: i64, den: i64) -> Result<i64, String> {
        if den == 0 {
            return Err(format!("[divide] {}", "Cannot divide by zero!".to_string()))        }
        else {
            return Ok(num / den)        }
    }
    let result: i64 = match     divide(10, 2).await { Ok(v) => v, Err(e) => (|err: String| -> i64 { println!("{}", std_lib::string::concat(vec! ["Error: ".to_string(), err.clone()]));     return 0 })(e) };
    println!("{}", std_lib::string::concat(vec! ["10 / 2 = ".to_string(), string_from(result.clone())]));
    async fn greet(person_name: String) -> String {
        return std_lib::string::concat(vec! ["Hello, ".to_string(), person_name.clone(), "!".to_string()])    }
    let greeting: String =     greet(name.clone()).await;
    println!("{}", greeting);
    let numbers: Vec<i64> = vec! [1, 2, 3, 4, 5];
    let names: Vec<String> = vec! ["Alice".to_string(), "Bob".to_string(), "Charlie".to_string()];
    let scores: Vec<f64> = vec! [95.5, 87.3, 92.0];
    if current_state ==     GameState::Playing {
        println!("{}", "Game is running!".to_string());
    }
    else if current_state ==     GameState::Paused {
        println!("{}", "Game is paused".to_string());
    }
    else {
        println!("{}", "Game is not active".to_string());
    }
    let (parallel_result_0, parallel_result_1, parallel_result_2, parallel_result_3) = tokio::join!(
        async {
            let task1: String =             greet("World".to_string()).await;
            ()
        },
        async {
            let task2: i64 = std_lib::time::now();
            ()
        },
        async {
            let task3: f64 = std_lib::math::sqrt(16.0);
            ()
        },
        async {
            println!("{}", "Running tasks in parallel!".to_string());
            ()
        }
    );
    let player_info: String = "Player created successfully!".to_string();
    println!("{}", player_info);
    let current_time: i64 = std_lib::time::now();
    let square_root: f64 = std_lib::math::sqrt(25.0);
    let random_num: f64 = std_lib::math::random();
    println!("{}", "Welcome to Nail - where simplicity meets power!".to_string());
}
