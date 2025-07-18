use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let greeting: String = "Hello, Nail!".string_from();
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
    #[derive(Debug)]
    enum Status {
        Active,
        Inactive,
        Pending,
    }
    let status: Status =     Status::Active;
    async fn safe_divide(num: i64, den: i64) -> Result<i64, String> {
        if den == 0 {
            return Err(format!("[safe_divide] {}", "Division by zero!".string_from()))        }
        else {
            return Ok(num / den)        }
    }
    let result: i64 = match     safe_divide(10, 2).await { Ok(v) => v, Err(e) => (|err: String| -> i64 { println!("{}", "Error occurred: ".string_from() + err);     return -1 })(e) };
    println!("{}", "10 / 2 = ".string_from() + string_from(result));
    async fn add(first: i64, second: i64) -> i64 {
        return first + second    }
    let total: i64 =     add(5, 3).await;
    println!("{}", "5 + 3 = ".string_from() + string_from(total));
    if status ==     Status::Active {
        println!("{}", "System is active".string_from());
    }
    else {
        println!("{}", "System is not active".string_from());
    }
    let (parallel_result_0, parallel_result_1, parallel_result_2) = tokio::join!(
        async {
            println!("{}", "Task 1 running...".string_from());
            ()
        },
        async {
            println!("{}", "Task 2 running...".string_from());
            ()
        },
        async {
            println!("{}", "Task 3 running...".string_from());
            ()
        }
    );
    println!("{}", "Welcome to Nail!".string_from());
}
