use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let player_name: String = "Grug the Warrior".to_string();
    let player_level: i64 = 15;
    let player_health: i64 = 75;
    let player_gold: i64 = 1337;
    let inventory: Vec<String> = vec! ["Sword".to_string(), "Shield".to_string(), "Potion".to_string(), "Scroll".to_string()];
    let item_count: i64 = std_lib::array::len(inventory.clone());
    let item_list: String = std_lib::array::join(inventory.clone(), ", ".to_string());
    let upper_name: String = std_lib::string::to_uppercase(player_name.clone());
    let health_str: String = std_lib::convert::to_string(player_health.clone());
    let level_str: String = std_lib::convert::to_string(player_level.clone());
    let gold_str: String = std_lib::convert::to_string(player_gold.clone());
    let count_str: String = std_lib::convert::to_string(item_count.clone());
    let output_lines: Vec<String> = vec! ["===== GAME STATS =====".to_string(), "Player: ".to_string(), upper_name.clone(), "\nLevel: ".to_string(), level_str.clone(), " | Health: ".to_string(), health_str.clone(), " | Gold: ".to_string(), gold_str.clone(), "\nInventory (".to_string(), count_str.clone(), " items): ".to_string(), item_list.clone(), "\n=====================".to_string()];
    println!("{}", std_lib::string::concat(output_lines.clone()));
}
