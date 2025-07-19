use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let max_health: i64 = 100;
    let initial_gold: i64 = 10;
    std_lib::print::print("Welcome to the Adventure Game!".to_string());
    std_lib::print::print("================================\n".to_string());
    let player_name: String = std_lib::io::read_line_prompt("Enter your character name: ".to_string()).unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    std_lib::print::print(std_lib::string::concat(vec! ["\nWelcome, ".to_string(), player_name.clone(), "!".to_string()]));
    std_lib::print::print(std_lib::string::concat(vec! ["Starting Health: ".to_string(), std_lib::string::from(max_health.clone())]));
    std_lib::print::print(std_lib::string::concat(vec! ["Starting Gold: ".to_string(), std_lib::string::from(initial_gold.clone())]));
    std_lib::print::print("\nYour adventure begins...\n".to_string());
    std_lib::print::print("=== Turn 1 ===".to_string());
    std_lib::print::print("You encounter a goblin!".to_string());
    let choice1: String = std_lib::io::read_line_prompt("Fight (f) or run (r)? ".to_string()).unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let health_after_1: i64 = if std_lib::string::contains(choice1.clone(), "f".to_string()) { max_health - 10 } else { max_health };
    let gold_after_1: i64 = if std_lib::string::contains(choice1.clone(), "f".to_string()) { initial_gold + 20 } else { initial_gold };
    std_lib::print::print(std_lib::string::concat(vec! ["Health: ".to_string(), std_lib::string::from(health_after_1.clone()), " Gold: ".to_string(), std_lib::string::from(gold_after_1.clone())]));
    std_lib::print::print("\n=== Turn 2 ===".to_string());
    std_lib::print::print("You find a treasure chest with 30 gold!".to_string());
    let gold_after_2: i64 = gold_after_1 + 30;
    std_lib::print::print(std_lib::string::concat(vec! ["You now have ".to_string(), std_lib::string::from(gold_after_2.clone()), " gold!".to_string()]));
    std_lib::print::print("\n=== Turn 3 ===".to_string());
    std_lib::print::print("A merchant offers to heal you for 20 gold.".to_string());
    let choice3: String = std_lib::io::read_line_prompt("Accept (y) or decline (n)? ".to_string()).unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let can_afford: bool = gold_after_2 >= 20;
    let health_after_3: i64 = if std_lib::string::contains(choice3.clone(), "y".to_string()) { if can_afford { max_health } else { health_after_1 } } else { health_after_1 };
    let gold_after_3: i64 = if std_lib::string::contains(choice3.clone(), "y".to_string()) { if can_afford { gold_after_2 - 20 } else { gold_after_2 } } else { gold_after_2 };
    std_lib::print::print(std_lib::string::concat(vec! ["Health: ".to_string(), std_lib::string::from(health_after_3.clone()), " Gold: ".to_string(), std_lib::string::from(gold_after_3.clone())]));
    std_lib::print::print("\n=== Game Over ===".to_string());
    std_lib::print::print(std_lib::string::concat(vec! ["Thanks for playing, ".to_string(), player_name.clone(), "!".to_string()]));
    std_lib::print::print("Final stats:".to_string());
    std_lib::print::print("- Survived 3 turns".to_string());
    std_lib::print::print(std_lib::string::concat(vec! ["- Final health: ".to_string(), std_lib::string::from(health_after_3.clone()), "/".to_string(), std_lib::string::from(max_health.clone())]));
    std_lib::print::print(std_lib::string::concat(vec! ["- Final gold: ".to_string(), std_lib::string::from(gold_after_3.clone()), " coins".to_string()]));
    std_lib::print::print("\nRun the game again for a different adventure!".to_string());
}
