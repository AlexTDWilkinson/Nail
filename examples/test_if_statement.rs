use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let age: i64 = 25;
    let score: f64 = 85.5;
    if age >= 18 {
        println!("{}", "Adult".string_from());
    }
    if score >= 90.0 {
        println!("{}", "Excellent score!".string_from());
    }
    else {
        println!("{}", "Good score!".string_from());
    }
    if age < 13 {
        println!("{}", "Child".string_from());
    }
    else if age < 20 {
        println!("{}", "Teenager".string_from());
    }
    else if age < 65 {
        println!("{}", "Adult".string_from());
    }
    else {
        println!("{}", "Senior".string_from());
    }
    let status_code: i64 = 200;
    if status_code == 200 {
        println!("{}", "OK".string_from());
    }
    else if status_code == 404 {
        println!("{}", "Not Found".string_from());
    }
    else if status_code == 500 {
        println!("{}", "Server Error".string_from());
    }
    else {
        println!("{}", "Unknown Status".string_from());
    }
    println!("{}", "If statement test complete".string_from());
}
