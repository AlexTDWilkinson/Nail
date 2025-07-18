use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let player_name: String = "Grug the Warrior".string_from();
    let player_level: i64 = 15;
    let player_health: i64 = 75;
    let player_gold: i64 = 1337;
    let inventory: Vec<String> = vec!["Sword".string_from(), "Shield".string_from(), "Potion".string_from(), "Scroll".string_from()];
    let item_count: i64 = std_lib::array::len(inventory.clone());
    let item_list: String = std_lib::array::join(inventory.clone(), ", ".string_from());
    let upper_name: String = std_lib::string::to_uppercase(player_name.clone());
    let health_str: String = string_from(player_health.clone());
    let level_str: String = string_from(player_level.clone());
    let gold_str: String = string_from(player_gold.clone());
    let count_str: String = string_from(item_count.clone());
    let output_lines: Vec<String> = vec![
        "===== GAME STATS =====".string_from(),
        "Player: ".string_from(),
        upper_name.clone(),
        "\nLevel: ".string_from(),
        level_str.clone(),
        " | Health: ".string_from(),
        health_str.clone(),
        " | Gold: ".string_from(),
        gold_str.clone(),
        "\nInventory (".string_from(),
        count_str.clone(),
        " items): ".string_from(),
        item_list.clone(),
        "\n=====================".string_from(),
    ];
    println!("{}", std_lib::string::concat(output_lines.clone()));
}
