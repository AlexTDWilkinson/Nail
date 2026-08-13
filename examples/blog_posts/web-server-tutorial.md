# Building a Web Server in 20 Lines of Nail

One of Nail's strengths is how easy it makes common tasks. Let's build a web server!

## The Complete Code

```nail
f handle_request(request:HTTP_Request, state:h<s,s>):HTTP_Response {
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
    r HTTP_Response {
        status = 200,
        body = html,
        content_type = `text/html`,
        headers = hashmap_new()
    };
}

http_server(3000, http_default_config());
```

That's it! Let's break it down:

## How It Works

1. **Define `handle_request`** - Every incoming request is handed to this function, along with the server's state hashmap
2. **Return an `HTTP_Response`** - Status, body, content type, and headers, all in one struct
3. **Start the server** - One `http_server` call with a port and a config, and `http_default_config()` fills in sensible defaults

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
