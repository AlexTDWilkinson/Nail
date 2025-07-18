use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let greeting: String = "Hello, Nail!".string_from();
    let magic_number: i64 = 42;
    let result_number: i64 = magic_number + 10;
    let number_string: String = string_from(magic_number.clone());
    println!("{:#?}", greeting);
    println!("{:#?}", number_string);
    println!("{:#?}", string_from(result_number.clone()));
}
