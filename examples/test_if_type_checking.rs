use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let age: i64 = 25;
    if age > 18 {
        println!("{}", "adult".string_from());
    }
    else {
        let temp: i64 = 100;
        println!("{}", string_from(temp.clone()));
    }
    if age > 18 {
        println!("{}", "over 18".string_from());
    }
    if age > 18 {
    }
    else {
    }
    if age < 18 {
        println!("{}", "minor".string_from());
        println!("{}", "restricted access".string_from());
    }
    else if age < 65 {
        println!("{}", "adult".string_from());
        println!("{}", "full access".string_from());
    }
    else {
        println!("{}", "senior".string_from());
        println!("{}", "special benefits".string_from());
    }
    println!("{}", "Type checking test complete".string_from());
}
