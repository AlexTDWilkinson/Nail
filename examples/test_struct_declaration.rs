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
    #[derive(Debug, Clone)]
    struct Item {
        item_id: i64,
        item_name: String,
        item_weight: f64,
        is_equipped: bool,
    }
    #[derive(Debug, Clone)]
    struct Position {
        x: f64,
        y: f64,
        z: f64,
    }
    #[derive(Debug, Clone)]
    struct Character {
        char_name: String,
        char_health: i64,
        char_mana: i64,
    }
    println!("{}", "Struct declarations test completed".to_string());
}
