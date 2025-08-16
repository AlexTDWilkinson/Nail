use std::process::Command;
use std::fs;
use std::path::Path;

fn main() {
    println!("Building Nail website...");
    
    // First, transpile the nail_website.nail file
    let output = Command::new("./target/debug/nailc")
        .args(&["examples/nail_website.nail", "--transpile"])
        .output()
        .expect("Failed to run nailc");
    
    if !output.status.success() {
        eprintln!("Failed to transpile nail_website.nail:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(1);
    }
    
    println!("Transpilation successful!");
    
    // Check if the transpiled file exists
    if !Path::new("examples/nail_website.rs").exists() {
        eprintln!("Transpiled file not found at examples/nail_website.rs");
        std::process::exit(1);
    }
    
    // Compile the transpiled Rust code
    println!("Compiling transpiled website...");
    let output = Command::new("rustc")
        .args(&[
            "examples/nail_website.rs",
            "-o", "nail_website_server",
            "-L", "target/debug/deps",
        ])
        .output()
        .expect("Failed to compile transpiled code");
    
    if !output.status.success() {
        eprintln!("Failed to compile transpiled code:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(1);
    }
    
    println!("Website compiled successfully!");
    println!("Run ./nail_website_server to start the website");
}