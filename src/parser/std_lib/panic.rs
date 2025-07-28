use std::process;

pub async fn panic(message: String) -> ! {
    eprintln!("PANIC: {}", message);
    process::exit(1);
}

pub async fn todo(message: String) -> ! {
    eprintln!("TODO: {}", message);
    process::exit(1);
}