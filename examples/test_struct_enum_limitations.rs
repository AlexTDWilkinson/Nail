use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    #[derive(Debug, Clone)]
    struct User {
        username: String,
        user_id: i64,
        is_admin: bool,
    }
    #[derive(Debug, PartialEq)]
    enum Status {
        Active,
        Inactive,
        Pending,
        Suspended,
    }
    let message: String = "Structs and enums can be declared".to_string();
    let count: i64 = 42;
    println!("{}", message);
    println!("{}", "Currently only declarations are supported".to_string());
}
