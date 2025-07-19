# Building a Web Server in 10 Lines of Nail

One of Nail's strengths is how easy it makes common tasks. Let's build a web server!

## The Complete Code

```nail
port:i = 3000;
html:s = `
<!DOCTYPE html>
<html>
<head><title>My Nail Server</title></head>
<body>
    <h1>Hello from Nail!</h1>
    <p>This server was built in just a few lines of code.</p>
</body>
</html>
`;
http_server_start(port, html);
```

That's it! Let's break it down:

## How It Works

1. **Define the port** - We use port 3000
2. **Create HTML content** - Using Nail's multi-line string literals
3. **Start the server** - One function call and you're done!

## Advanced Features

Nail's HTTP library also supports:

- **Routing** - Handle different endpoints
- **JSON APIs** - Parse and return JSON data
- **File serving** - Serve static files
- **WebSockets** - Real-time communication

## Performance

Thanks to Nail's automatic parallelization and Rust backend:
- Handles thousands of concurrent connections
- Sub-millisecond response times
- Efficient memory usage
- Built on battle-tested Axum framework

## Next Steps

Try adding:
- Database connections
- Authentication
- REST API endpoints
- WebSocket support

Happy coding with Nail! 🔨