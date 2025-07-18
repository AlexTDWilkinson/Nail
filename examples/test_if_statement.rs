use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let age: i64 = 25;
    let score: f64 = 85.5;
    if age >= 18 {
        println!("{}", "Adult".to_string());
    }
    if score >= 90.0 {
        println!("{}", "Excellent score!".to_string());
    } else {
        println!("{}", "Good score!".to_string());
    }
    if age < 13 {
        println!("{}", "Child".to_string());
    } else if age < 20 {
        println!("{}", "Teenager".to_string());
    } else if age < 65 {
        println!("{}", "Adult".to_string());
    } else {
        println!("{}", "Senior".to_string());
    }
    let status_code: i64 = 200;
    if status_code == 200 {
        println!("{}", "OK".to_string());
    } else if status_code == 404 {
        println!("{}", "Not Found".to_string());
    } else if status_code == 500 {
        println!("{}", "Server Error".to_string());
    } else {
        println!("{}", "Unknown Status".to_string());
    }
    println!("{}", "If statement test complete".to_string());
}
