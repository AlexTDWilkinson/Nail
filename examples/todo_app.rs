use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let priority_high: i64 = 3;
    let priority_medium: i64 = 2;
    let priority_low: i64 = 1;
    fn format_task(task_name: String, priority: i64) -> String {
        let priority_text: String = if priority == priority_high { "[HIGH]".to_string() } else if priority == priority_medium { "[MED] ".to_string() } else if priority == priority_low { "[LOW] ".to_string() } else { "".to_string() };
        let formatted: String = std_lib::string::concat(vec! [priority_text.clone(), " ".to_string(), task_name.clone()]);
        return formatted    }
    fn calculate_stats(task_count: i64, completed_count: i64) -> f64 {
        let total_float: f64 = std_lib::float::from(std_lib::string::from(task_count.clone())).unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
        let completed_float: f64 = std_lib::float::from(std_lib::string::from(completed_count.clone())).unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
        let percentage: f64 = std_lib::math::round(std_lib::math::max(0.0, completed_float / total_float * 100.0));
        return percentage    }
    let app_title: String = "=== Nail Todo List Manager ===".to_string();
    let separator: String = "==============================".to_string();
    let task_one: String =     format_task("Write Nail documentation".to_string(), priority_high.clone());
    let task_two: String =     format_task("Test array functions".to_string(), priority_medium.clone());
    let task_three: String =     format_task("Fix parser bugs".to_string(), priority_high.clone());
    let task_four: String =     format_task("Add more examples".to_string(), priority_low.clone());
    let all_tasks: Vec<String> = vec! [task_one.clone(), task_two.clone(), task_three.clone(), task_four.clone()];
    let total_tasks: i64 = std_lib::array::len(all_tasks.clone());
    let completed_tasks: i64 = 2;
    let completion_rate: f64 =     calculate_stats(total_tasks.clone(), completed_tasks.clone());
    let header: String = std_lib::string::concat(vec! [app_title.clone(), "\n".to_string(), separator.clone()]);
    let stats_line: String = std_lib::string::concat(vec! ["Total Tasks: ".to_string(), std_lib::string::from(total_tasks.clone()), " | Completed: ".to_string(), std_lib::string::from(completed_tasks.clone()), " (".to_string(), std_lib::string::from(completion_rate.clone()), "%)".to_string()]);
    let task_list: String = std_lib::array::join(all_tasks.clone(), "\n".to_string());
    let timestamp: String = std_lib::string::concat(vec! ["\nGenerated at: ".to_string(), std_lib::string::from(std_lib::time::now()), " seconds since epoch".to_string()]);
    let output_parts: Vec<String> = vec! [header.clone(), "\n\n".to_string(), stats_line.clone(), "\n\n".to_string(), "Tasks:\n".to_string(), task_list.clone(), "\n".to_string(), separator.clone(), timestamp.clone()];
    let final_output: String = std_lib::string::concat(output_parts.clone());
    println!("{:#?}", final_output);
}
