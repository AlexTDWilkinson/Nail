use Nail::std_lib::string::from;
use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let greeting: String = "Hello, Nail!".to_string();
    let count: i64 = 42;
    let pi: f64 = 3.14159;
    let is_ready: bool = true;
    println!("{}", greeting);
    #[derive(Debug, Clone)]
    struct Point {
        x: i64,
        y: i64,
    }
    let origin: Point = Point { x: 0,  y: 0 };
    #[derive(Debug, PartialEq)]
    enum Status {
        Active,
        Inactive,
        Pending,
    }
    let status: Status =     Status::Active;
    async fn safe_divide(num: i64, den: i64) -> Result<i64, String> {
        if den == 0 {
            return Err(format!("[safe_divide] {}", "Division by zero!".to_string()))        }
        else {
            return Ok(num / den)        }
    }
    let result: i64 = match     safe_divide(10, 2).await { Ok(v) => v, Err(e) => (|err: String| -> i64 { println!("{}", std_lib::string::concat(vec! ["Error occurred: ".to_string(), err.clone()]));     return -1 })(e) };
    println!("{}", std_lib::string::concat(vec! ["10 / 2 = ".to_string(),from(result.clone())]));
    async fn add(first: i64, second: i64) -> i64 {
        return first + second    }
    let total: i64 =     add(5, 3).await;
    println!("{}", std_lib::string::concat(vec! ["5 + 3 = ".to_string(),from(total.clone())]));
    if status ==     Status::Active {
        println!("{}", "System is active".to_string());
    }
    else {
        println!("{}", "System is not active".to_string());
    }
    let (parallel_result_0, parallel_result_1, parallel_result_2) = tokio::join!(
        async {
            println!("{}", "Task 1 running...".to_string());
            ()
        },
        async {
            println!("{}", "Task 2 running...".to_string());
            ()
        },
        async {
            println!("{}", "Task 3 running...".to_string());
            ()
        }
    );
    println!("{}", "Welcome to Nail!".to_string());
}
