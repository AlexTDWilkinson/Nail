use std::process;

pub fn panic(message: String) -> ! {
    eprintln!("PANIC: {}", message);
    process::exit(1);
}

pub fn todo(message: String) -> ! {
    eprintln!("TODO: {}", message);
    process::exit(1);
}