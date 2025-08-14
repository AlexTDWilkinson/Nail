// This is a wrapper that builds and runs the Nail website for Render deployment
use std::process::Command;
use std::env;

fn main() {
    println!("Starting Nail website deployment process...");
    
    // Check if we're running on Render (they set PORT environment variable)
    if env::var("PORT").is_ok() || env::var("RENDER").is_ok() {
        println!("Detected Render environment, building website...");
        
        // Run the build script
        let build_status = Command::new("bash")
            .arg("build_website.sh")
            .status()
            .expect("Failed to execute build script");
            
        if !build_status.success() {
            eprintln!("Build failed!");
            std::process::exit(1);
        }
        
        // Execute the website binary
        println!("Starting website server...");
        let mut website = Command::new("./target/release/nail_website")
            .spawn()
            .expect("Failed to start website");
            
        // Wait for the website process
        let _ = website.wait();
    } else {
        println!("Not in Render environment. To run the website locally, use:");
        println!("  ./build_website.sh");
        println!("  ./target/release/nail_website");
    }
}