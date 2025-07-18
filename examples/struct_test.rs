use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    #[derive(Debug, Clone)]
    struct Player {
        player_name: String,
        player_level: i64,
        health_points: f64,
    }
    #[derive(Debug, PartialEq)]
    enum GameState {
        MainMenu,
        Playing,
        GameOver,
    }
    println!("{}", "Struct and enum declarations compiled successfully!".to_string());
}
