//! An MCP server in a few lines: declare the tools, write one handle_tool
//! function, call mcp_serve. The Model Context Protocol is how AI assistants
//! call outside tools, and a Nail binary that speaks it plugs straight into
//! any MCP client as a single file with no runtime to install.
//!
//! The transport is the standard one, JSON-RPC over stdin and stdout, so
//! stdout belongs entirely to the protocol while serving. Anything the
//! program wants to say to a person goes through the log functions, which
//! write to stderr and stay out of the way.

use std::future::Future;
use std::pin::Pin;

pub type ToolFuture = Pin<Box<dyn Future<Output = Result<String, String>> + Send>>;

/// One tool the server offers. The input schema is JSON Schema as text, the
/// same shape every MCP client shows to its model.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MCP_Tool {
    pub name: String,
    pub description: String,
    pub input_schema: String,
}

fn rpc_result(id: &serde_json::Value, result: serde_json::Value) -> String {
    return serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string();
}

fn rpc_error(id: &serde_json::Value, code: i64, message: &str) -> String {
    return serde_json::json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }).to_string();
}

/// One request line in, at most one response line out. Notifications answer
/// with nothing. Kept separate from the read loop so tests can drive it.
async fn answer_line<F>(line: &str, name: &str, version: &str, tools: &[MCP_Tool], handler: &F) -> Option<String>
where
    F: Fn(String, String) -> ToolFuture + Clone + Send + Sync + 'static,
{
    let parsed: serde_json::Value = match serde_json::from_str(line.trim()) {
        Ok(value) => value,
        Err(_) => return Some(rpc_error(&serde_json::Value::Null, -32700, "that line is not JSON-RPC")),
    };
    let id = parsed.get("id").cloned().unwrap_or(serde_json::Value::Null);
    let is_notification = parsed.get("id").is_none();
    let method = parsed.get("method").and_then(|m| m.as_str()).unwrap_or("");

    let response = match method {
        "initialize" => {
            let requested = parsed
                .pointer("/params/protocolVersion")
                .and_then(|v| v.as_str())
                .unwrap_or("2025-03-26")
                .to_string();
            rpc_result(
                &id,
                serde_json::json!({
                    "protocolVersion": requested,
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": name, "version": version }
                }),
            )
        }
        "ping" => rpc_result(&id, serde_json::json!({})),
        "tools/list" => {
            let listed: Vec<serde_json::Value> = tools
                .iter()
                .map(|tool| {
                    let schema: serde_json::Value = serde_json::from_str(&tool.input_schema).expect("schemas are checked before serving");
                    serde_json::json!({ "name": tool.name, "description": tool.description, "inputSchema": schema })
                })
                .collect();
            rpc_result(&id, serde_json::json!({ "tools": listed }))
        }
        "tools/call" => {
            let tool_name = parsed.pointer("/params/name").and_then(|v| v.as_str()).unwrap_or("");
            if !tools.iter().any(|tool| tool.name == tool_name) {
                return Some(rpc_error(&id, -32602, &format!("no tool is called `{}`", tool_name)));
            }
            let arguments = parsed.pointer("/params/arguments").cloned().unwrap_or_else(|| serde_json::json!({}));
            let answer = handler(tool_name.to_string(), arguments.to_string()).await;
            let (text, is_error) = match answer {
                Ok(text) => (text, false),
                Err(text) => (text, true),
            };
            rpc_result(
                &id,
                serde_json::json!({ "content": [ { "type": "text", "text": text } ], "isError": is_error }),
            )
        }
        _ if method.starts_with("notifications/") => return None,
        _ => rpc_error(&id, -32601, &format!("this server does not answer `{}`", method)),
    };

    if is_notification {
        return None;
    }
    return Some(response);
}

/// Serve the tools over stdin and stdout until the client hangs up. Each
/// tools/call is passed to the program's handle_tool function, whose Ok text
/// becomes the tool result and whose error becomes a tool error the model
/// can read. Blocks until stdin closes, which is how MCP sessions end.
pub async fn serve<F>(name: String, version: String, tools: Vec<MCP_Tool>, handler: F) -> Result<(), String>
where
    F: Fn(String, String) -> ToolFuture + Clone + Send + Sync + 'static,
{
    if name.trim().is_empty() {
        return Err("mcp_serve: the server needs a name to introduce itself with".to_string());
    }
    if tools.is_empty() {
        return Err("mcp_serve: there are no tools to offer".to_string());
    }
    for tool in &tools {
        if tool.name.trim().is_empty() {
            return Err("mcp_serve: every tool needs a name".to_string());
        }
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&tool.input_schema);
        if parsed.is_err() {
            return Err(format!("mcp_serve: the input_schema of `{}` is not JSON", tool.name));
        }
    }

    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let stdin = tokio::io::stdin();
    let mut lines = tokio::io::BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = answer_line(&line, &name, &version, &tools, &handler).await {
            let written = stdout.write_all(response.as_bytes()).await.and(stdout.write_all(b"\n").await);
            if written.is_err() {
                break;
            }
            let _ = stdout.flush().await;
        }
    }
    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weather_tools() -> Vec<MCP_Tool> {
        return vec![MCP_Tool {
            name: "get_weather".to_string(),
            description: "Weather for a city".to_string(),
            input_schema: r#"{"type":"object","properties":{"city":{"type":"string"}},"required":["city"]}"#.to_string(),
        }];
    }

    fn echo_handler() -> impl Fn(String, String) -> ToolFuture + Clone + Send + Sync + 'static {
        return |tool: String, arguments: String| {
            Box::pin(async move {
                if arguments.contains("Mordor") {
                    return Err("nobody reports from there".to_string());
                }
                return Ok(format!("{} got {}", tool, arguments));
            }) as ToolFuture
        };
    }

    async fn ask(line: &str) -> Option<String> {
        let tools = weather_tools();
        let handler = echo_handler();
        return answer_line(line, "weather", "1.0.0", &tools, &handler).await;
    }

    #[tokio::test]
    async fn the_handshake_introduces_the_server_and_echoes_the_protocol() {
        let response = ask(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed.pointer("/result/protocolVersion").unwrap(), "2025-06-18");
        assert_eq!(parsed.pointer("/result/serverInfo/name").unwrap(), "weather");
        assert!(parsed.pointer("/result/capabilities/tools").is_some());
    }

    #[tokio::test]
    async fn the_tool_list_carries_the_parsed_schema() {
        let response = ask(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed.pointer("/result/tools/0/name").unwrap(), "get_weather");
        assert_eq!(parsed.pointer("/result/tools/0/inputSchema/type").unwrap(), "object");
    }

    #[tokio::test]
    async fn a_call_reaches_the_handler_and_an_error_is_a_tool_error() {
        let response = ask(r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_weather","arguments":{"city":"Edmonton"}}}"#).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed.pointer("/result/isError").unwrap(), false);
        assert!(parsed.pointer("/result/content/0/text").unwrap().as_str().unwrap().contains("Edmonton"));

        let response = ask(r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"get_weather","arguments":{"city":"Mordor"}}}"#).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed.pointer("/result/isError").unwrap(), true);
        assert!(parsed.pointer("/result/content/0/text").unwrap().as_str().unwrap().contains("nobody reports"));
    }

    #[tokio::test]
    async fn the_edges_answer_the_way_json_rpc_asks() {
        let response = ask(r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"missing","arguments":{}}}"#).await.unwrap();
        assert!(response.contains("-32602"));
        let response = ask(r#"{"jsonrpc":"2.0","id":6,"method":"resources/list"}"#).await.unwrap();
        assert!(response.contains("-32601"));
        let response = ask("this is not json").await.unwrap();
        assert!(response.contains("-32700"));
        assert!(ask(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).await.is_none());
    }

    #[tokio::test]
    async fn serving_nothing_or_broken_schemas_is_refused_before_listening() {
        let handler = echo_handler();
        assert!(serve("weather".to_string(), "1.0.0".to_string(), vec![], handler.clone()).await.unwrap_err().contains("no tools"));
        let broken = vec![MCP_Tool { name: "x".to_string(), description: String::new(), input_schema: "not json".to_string() }];
        assert!(serve("weather".to_string(), "1.0.0".to_string(), broken, handler).await.unwrap_err().contains("not JSON"));
    }
}
