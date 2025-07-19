use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    #[derive(Debug, Clone)]
    struct Player {
        name: String,
        health: i64,
        gold: i64,
        turn: i64,
    }
    #[derive(Debug, Clone)]
    struct Event {
        turn: i64,
        description: String,
        health_change: i64,
        gold_change: i64,
    }
    let max_health: i64 = 100;
    let initial_gold: i64 = 10;
    std_lib::print::print("Welcome to the 100-Turn Adventure Game!".to_string());
    std_lib::print::print("========================================\n".to_string());
    let player_name: String = std_lib::io::read_line_prompt("Enter your character name: ".to_string()).unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let initial_player: Player = Player { name: player_name,  health: max_health,  gold: initial_gold,  turn: 0 };
    let all_events: Vec<Event> = vec! [Event { turn: 1,  description: "You encounter a goblin! Fight and earn 20 gold but lose 10 health.".to_string(),  health_change: -10,  gold_change: 20 }, Event { turn: 2,  description: "You find a treasure chest! Gain 30 gold.".to_string(),  health_change: 0,  gold_change: 30 }, Event { turn: 3,  description: "A friendly healer restores 15 health.".to_string(),  health_change: 15,  gold_change: 0 }, Event { turn: 4,  description: "You fall into a trap! Lose 5 health.".to_string(),  health_change: -5,  gold_change: 0 }, Event { turn: 5,  description: "You trade with a merchant for 25 gold.".to_string(),  health_change: 0,  gold_change: 25 }, Event { turn: 6,  description: "Bandits attack! Lose 8 health but gain 15 gold.".to_string(),  health_change: -8,  gold_change: 15 }, Event { turn: 7,  description: "You rest at an inn. Restore 20 health.".to_string(),  health_change: 20,  gold_change: 0 }, Event { turn: 8,  description: "You find rare herbs! Gain 10 health and 5 gold.".to_string(),  health_change: 10,  gold_change: 5 }, Event { turn: 9,  description: "A dragon appears! Lose 25 health but gain 100 gold.".to_string(),  health_change: -25,  gold_change: 100 }, Event { turn: 10,  description: "You discover a magic fountain! Restore to full health.".to_string(),  health_change: 100,  gold_change: 0 }];
    std_lib::print::print(std_lib::string::concat(vec! ["\nWelcome, ".to_string(), initial_player.name.clone(), "! Starting your 100-turn adventure...\n".to_string()]));
    std_lib::print::print(std_lib::string::concat(vec! ["Starting Health: ".to_string(), std_lib::string::from(initial_player.health.clone())]));
    std_lib::print::print(std_lib::string::concat(vec! ["Starting Gold: ".to_string(), std_lib::string::from(initial_player.gold.clone()), "\n".to_string()]));
    let final_player: Player = std_lib::array_functional::reduce_struct(all_events.clone(), initial_player.clone(), |acc: Player, event: Event| -> Player {     let new_health: i64 = if acc.health.clone() + event.health_change.clone() > max_health { max_health } else if acc.health.clone() + event.health_change.clone() < 1 { 1 } else { acc.health.clone() + event.health_change.clone() };
;     let new_gold: i64 = if acc.gold.clone() + event.gold_change.clone() < 0 { 0 } else { acc.gold.clone() + event.gold_change.clone() };
;     let current_name: String = acc.name.clone();
;     let current_turn: i64 = acc.turn.clone() + 1;
;     let updated_player: Player = Player { name: current_name,  health: new_health,  gold: new_gold,  turn: current_turn };
;     let should_print: bool = acc.turn.clone() + 1 % 10 == 0;
; if should_print { std_lib::print::print(std_lib::string::concat(vec! ["  Health: ".to_string(), std_lib::string::from(new_health.clone()), ", Gold: ".to_string(), std_lib::string::from(new_gold.clone())])) } else {  };     return updated_player });
    std_lib::print::print("\n========================================".to_string());
    std_lib::print::print("🏆 ADVENTURE COMPLETE! 🏆".to_string());
    std_lib::print::print("========================================".to_string());
    std_lib::print::print(std_lib::string::concat(vec! ["Hero: ".to_string(), final_player.name.clone()]));
    std_lib::print::print(std_lib::string::concat(vec! ["Survived: ".to_string(), std_lib::string::from(final_player.turn.clone()), " turns".to_string()]));
    std_lib::print::print(std_lib::string::concat(vec! ["Final Health: ".to_string(), std_lib::string::from(final_player.health.clone()), "/".to_string(), std_lib::string::from(max_health.clone())]));
    std_lib::print::print(std_lib::string::concat(vec! ["Final Gold: ".to_string(), std_lib::string::from(final_player.gold.clone()), " coins".to_string()]));
    let final_health: i64 = final_player.health.clone();
    let final_gold: i64 = final_player.gold.clone();
    let success_message: String = if final_health >= 80 { if final_gold >= 1000 { "LEGENDARY HERO! Outstanding adventure!".to_string() } else { "GREAT HERO! Excellent health!".to_string() } } else if final_health >= 50 { if final_gold >= 500 { "GREAT HERO! Excellent job surviving!".to_string() } else { "GOOD HERO! Nice job!".to_string() } } else if final_health >= 20 { "BRAVE HERO! Good effort!".to_string() } else { "BARELY SURVIVED! Better luck next time!".to_string() };
    std_lib::print::print(success_message.clone());
    std_lib::print::print("\nThanks for playing the 100-Turn Adventure Game!".to_string());
}
