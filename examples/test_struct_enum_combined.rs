use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    #[derive(Debug, Clone)]
    struct PlayerStats {
        strength: i64,
        dexterity: i64,
        intelligence: i64,
        vitality: i64,
    }
    #[derive(Debug, PartialEq)]
    enum EquipmentSlot {
        Head,
        Chest,
        Legs,
        Feet,
        MainHand,
        OffHand,
    }
    #[derive(Debug, PartialEq)]
    enum CharacterClass {
        Warrior,
        Mage,
        Rogue,
        Cleric,
    }
    #[derive(Debug, Clone)]
    struct GameObject {
        object_id: i64,
        object_name: String,
        position_x: f64,
        position_y: f64,
        is_active: bool,
    }
    #[derive(Debug, PartialEq)]
    enum WorldState {
        Loading,
        Active,
        Saving,
        Shutting_Down,
    }
    println!("{}", "Combined struct and enum test completed".to_string());
}
