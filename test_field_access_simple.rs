use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    #[derive(Debug, Clone)]
    struct Player {
        name: String,
        health: i64,
    }
    let player: Player = Player { name: "Test".to_string(),  health: 100 };
    std_lib::print::print(player.name);
}
