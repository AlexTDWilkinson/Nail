use tokio;
use Nail::std_lib;
use Nail::std_lib::string::string_from;

#[tokio::main]
async fn main() {
    let port: i64 = 3000;
    let port_string: String = string_from(port.clone());
    let html_content: String = std_lib::string::concat(vec! ["
<!DOCTYPE html>
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
        code { 
            background: #e0e0e0; 
            padding: 2px 4px; 
            border-radius: 3px;
        }
    </style>
</head>
<body>
    <div class=\"nail-box\">
        <h1>🔨 Hello from Nail!</h1>
        <p>This webpage is being served by a Nail program using Axum under the hood.</p>
        <p>The server is running on port <code>".string_from(), port_string.clone(), "</code></p>
        <p>Nail features demonstrated:</p>
        <ul>
            <li>String interpolation with backticks</li>
            <li>Automatic standard library imports</li>
            <li>Simple HTTP server with one function call</li>
        </ul>
        <p>Grug happy. Complexity bad. Nail good. 🎉</p>
    </div>
</body>
</html>
".string_from()]);
    std_lib::http::http_server_start(port.clone(), html_content.clone()).await.unwrap();
}
