use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    #[derive(Debug, Clone)]
    struct Player {
        name: String,
        health: i64,
        gold: i64,
    }
    let max_health: i64 = 100;
    let initial_gold: i64 = 10;
    std_lib::print::print("Welcome to the Adventure Game!".to_string());
    std_lib::print::print("================================\n".to_string());
    let player_name: String = std_lib::io::read_line_prompt("Enter your character name: ".to_string()).unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let player: Player = Player { name: player_name, health: max_health, gold: initial_gold };
    std_lib::print::print(std_lib::string::concat(vec!["\nWelcome, ".to_string(), player.name.clone(), "!".to_string()]));
    std_lib::print::print(std_lib::string::concat(vec!["Starting Health: ".to_string(), std_lib::string::from(player.health.clone())]));
    std_lib::print::print(std_lib::string::concat(vec!["Starting Gold: ".to_string(), std_lib::string::from(player.gold.clone())]));
    std_lib::print::print("\nYour adventure begins...\n".to_string());
    std_lib::print::print("=== Turn 1 ===".to_string());
    std_lib::print::print("You encounter a goblin!".to_string());
    let choice1: String = std_lib::io::read_line_prompt("Fight (f) or run (r)? ".to_string()).unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let health_after_fight: i64 = if std_lib::string::contains(choice1.clone(), "f".to_string()) { player.health.clone() - 10 } else { player.health.clone() };
    let gold_after_fight: i64 = if std_lib::string::contains(choice1.clone(), "f".to_string()) { player.gold.clone() + 20 } else { player.gold.clone() };
    let current_name: String = player.name.clone();
    let player_turn1: Player = Player { name: current_name, health: health_after_fight, gold: gold_after_fight };
    std_lib::print::print(std_lib::string::concat(vec![
        "Health: ".to_string(),
        std_lib::string::from(player_turn1.health.clone()),
        " Gold: ".to_string(),
        std_lib::string::from(player_turn1.gold.clone()),
    ]));
    std_lib::print::print("\n=== Turn 2 ===".to_string());
    std_lib::print::print("You find a treasure chest with 30 gold!".to_string());
    let name_turn2: String = player_turn1.name.clone();
    let health_turn2: i64 = player_turn1.health.clone();
    let gold_turn2: i64 = player_turn1.gold.clone() + 30;
    let player_turn2: Player = Player { name: name_turn2, health: health_turn2, gold: gold_turn2 };
    std_lib::print::print(std_lib::string::concat(vec!["You now have ".to_string(), std_lib::string::from(player_turn2.gold.clone()), " gold!".to_string()]));
    std_lib::print::print("\n=== Turn 3 ===".to_string());
    std_lib::print::print("A merchant offers to heal you for 20 gold.".to_string());
    let choice3: String = std_lib::io::read_line_prompt("Accept (y) or decline (n)? ".to_string()).unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let can_afford: bool = player_turn2.gold.clone() >= 20;
    let final_health: i64 = if std_lib::string::contains(choice3.clone(), "y".to_string()) {
        if can_afford {
            max_health
        } else {
            player_turn2.health.clone()
        }
    } else {
        player_turn2.health.clone()
    };
    let final_gold: i64 = if std_lib::string::contains(choice3.clone(), "y".to_string()) {
        if can_afford {
            player_turn2.gold.clone() - 20
        } else {
            player_turn2.gold.clone()
        }
    } else {
        player_turn2.gold.clone()
    };
    let final_name: String = player_turn2.name.clone();
    let player_final: Player = Player { name: final_name, health: final_health, gold: final_gold };
    std_lib::print::print(std_lib::string::concat(vec![
        "Health: ".to_string(),
        std_lib::string::from(player_final.health.clone()),
        " Gold: ".to_string(),
        std_lib::string::from(player_final.gold.clone()),
    ]));
    std_lib::print::print("\n=== Game Over ===".to_string());
    std_lib::print::print(std_lib::string::concat(vec!["Thanks for playing, ".to_string(), player_final.name.clone(), "!".to_string()]));
    std_lib::print::print("Final stats:".to_string());
    std_lib::print::print("- Survived 3 turns".to_string());
    std_lib::print::print(std_lib::string::concat(vec![
        "- Final health: ".to_string(),
        std_lib::string::from(player_final.health.clone()),
        "/".to_string(),
        std_lib::string::from(max_health.clone()),
    ]));
    std_lib::print::print(std_lib::string::concat(vec!["- Final gold: ".to_string(), std_lib::string::from(player_final.gold.clone()), " coins".to_string()]));
    std_lib::print::print("\nRun the game again for a different adventure!".to_string());
}
