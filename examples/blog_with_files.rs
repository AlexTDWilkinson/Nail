use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    let port: i64 = 8081;
    let post1_path: String = "blog_posts/welcome.md".to_string();
    let post1_result: Result<String, String> = std_lib::fs::read_file(post1_path.clone()).await;
    let post1_md: String = post1_result.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let post2_path: String = "blog_posts/web-server-tutorial.md".to_string();
    let post2_result: Result<String, String> = std_lib::fs::read_file(post2_path.clone()).await;
    let post2_md: String = post2_result.unwrap_or_else(|nail_error| panic!("🔨 Nail Error: {}", nail_error));
    let post1_html: String = std_lib::markdown::to_html(post1_md.clone());
    let post2_html: String = std_lib::markdown::to_html(post2_md.clone());
    let blog_html: String = std_lib::string::concat(vec! ["
<!DOCTYPE html>
<html lang=\"en\">
<head>
    <meta charset=\"UTF-8\">
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">
    <title>Nail Programming Blog</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            line-height: 1.6;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f5f5f5;
        }
        .container {
            background: white;
            padding: 40px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        h1, h2, h3 {
            color: #333;
        }
        h1 {
            border-bottom: 3px solid #0066cc;
            padding-bottom: 10px;
        }
        .post {
            margin-bottom: 60px;
            padding-bottom: 40px;
            border-bottom: 1px solid #eee;
        }
        .post:last-child {
            border-bottom: none;
        }
        .post-meta {
            color: #666;
            font-size: 0.9em;
            margin-bottom: 20px;
        }
        code {
            background: #f4f4f4;
            padding: 2px 5px;
            border-radius: 3px;
            font-family: 'Courier New', monospace;
        }
        pre {
            background: #f4f4f4;
            padding: 15px;
            border-radius: 5px;
            overflow-x: auto;
        }
        pre code {
            background: none;
            padding: 0;
        }
        .header {
            text-align: center;
            margin-bottom: 40px;
        }
        a {
            color: #0066cc;
            text-decoration: none;
        }
        a:hover {
            text-decoration: underline;
        }
        .nav {
            text-align: center;
            margin-bottom: 30px;
        }
        .nav a {
            margin: 0 15px;
            font-weight: bold;
        }
    </style>
</head>
<body>
    <div class=\"container\">
        <div class=\"header\">
            <h1>🔨 The Nail Programming Blog</h1>
            <p>Learn about Nail's unique approach to simplicity and performance</p>
        </div>
        
        <div class=\"nav\">
            <a href=\"#post1\">Welcome to Nail</a>
            <a href=\"#post2\">Web Server Tutorial</a>
        </div>
        
        <div class=\"post\" id=\"post1\">
            <div class=\"post-meta\">Published: January 15, 2024</div>
            ".to_string(), post1_html.clone(), "
        </div>
        
        <div class=\"post\" id=\"post2\">
            <div class=\"post-meta\">Published: January 20, 2024</div>
            ".to_string(), post2_html.clone(), "
        </div>
        
        <div style=\"text-align: center; margin-top: 40px; color: #666;\">
            <p>Built with Nail - The language without complexity</p>
            <p>Blog posts loaded from markdown files using fs_read()</p>
        </div>
    </div>
</body>
</html>".to_string()]);
    std_lib::print::print(std_lib::string::concat(vec! ["Starting blog server on port ".to_string(), std_lib::string::from(port.clone()), "...".to_string()]));
    std_lib::print::print("Loading posts from examples/blog_posts/*.md".to_string());
    std_lib::http::http_server_start(port.clone(), blog_html.clone()).await.unwrap();
}
