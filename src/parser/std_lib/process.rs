use tokio::process::Command as TokioCommand;

pub async fn exit(code: i64) -> ! {
    std::process::exit(code as i32)
}

pub async fn run(command: String, args: Vec<String>) -> Result<String, String> {
    let output = TokioCommand::new(command)
        .args(args)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    
    if output.status.success() {
        String::from_utf8(output.stdout)
            .map_err(|e| e.to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}