use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let port: i64 = 8080;

    let html_content: String = "<!DOCTYPE html>
<html>
<head>
    <title>Nail Web Server</title>
    <style>
        body { 
            font-family: Arial, sans-serif; 
            max-width: 600px; 
            margin: 50px auto; 
            padding: 20px;
            background-color: #f0f0f0;
        }
        h1 { color: #333; }
        .nail-box {
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
    </style>
</head>
<body>
    <div class=\"nail-box\">
        <h1>🔨 Hello from Nail!</h1>
        <p>This webpage is being served by a Nail program using Axum.</p>
        <p>The server is running on port 3000</p>
        <p>Grug happy. Complexity bad. Nail good. 🎉</p>
    </div>
</body>
</html>".string_from();

    std_lib::http::http_server_start(port, html_content).await.unwrap()

}