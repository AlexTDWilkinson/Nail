use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    #[derive(Debug, PartialEq)]
    enum GameState {
        MainMenu,
        Playing,
        Paused,
        GameOver,
    }
    #[derive(Debug, PartialEq)]
    enum Direction {
        North,
        South,
        East,
        West,
    }
    #[derive(Debug, PartialEq)]
    enum ItemType {
        Weapon,
        Armor,
        Consumable,
        QuestItem,
    }
    #[derive(Debug, PartialEq)]
    enum PlayerAction {
        Move,
        Attack,
        Defend,
        UseItem,
        Talk,
        Rest,
    }
    println!("{}", "Enum declarations test completed".string_from());
}
