use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    #[derive(Debug, Clone)]
    struct Point {
        x: f64,
        y: f64,
    }
    #[derive(Debug, Clone)]
    struct Rectangle {
        top_left: Point,
        bottom_right: Point,
    }
    #[derive(Debug, Clone)]
    struct Circle {
        center: Point,
        radius: f64,
    }
    #[derive(Debug, Clone)]
    struct Player {
        name: String,
        position: Point,
        health: i64,
    }
    #[derive(Debug, Clone)]
    struct GameWorld {
        player_one: Player,
        player_two: Player,
        world_bounds: Rectangle,
    }
    println!("{}", "Nested struct declarations test".to_string());
}
