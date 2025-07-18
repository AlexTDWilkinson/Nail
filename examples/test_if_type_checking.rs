use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let age: i64 = 25;
    if age > 18 {
        println!("{}", "adult".to_string());
    }
    else {
        let temp: i64 = 100;
        println!("{}", string_from(temp.clone()));
    }
    if age > 18 {
        println!("{}", "over 18".to_string());
    }
    if age > 18 {
    }
    else {
    }
    if age < 18 {
        println!("{}", "minor".to_string());
        println!("{}", "restricted access".to_string());
    }
    else if age < 65 {
        println!("{}", "adult".to_string());
        println!("{}", "full access".to_string());
    }
    else {
        println!("{}", "senior".to_string());
        println!("{}", "special benefits".to_string());
    }
    println!("{}", "Type checking test complete".to_string());
}
