//! MCP over JSON-RPC 2.0: `initialize`, `ping`, `tools/list`, `tools/call`.
//! One JSON message per line on stdio (the MCP stdio transport).
use crate::Bridge;
use serde_json::{Value, json};

/// Protocol revisions this server speaks, newest first.
pub const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

fn request_method(message: &Value) -> Option<&str> {
    if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return None;
    }
    message.get("method").and_then(Value::as_str)
}

/// Handle one incoming message. Returns the response, or `None` for a
/// notification (no `id`).
pub fn handle(bridge: &mut Bridge, message: &Value) -> Option<Value> {
    let id = message.get("id").cloned();
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let result = match request_method(message) {
        None => Err((-32600, "Invalid Request".to_string())),
        Some(_) if id.is_none() => return None,
        Some("initialize") => Ok(initialize(&params)),
        Some("ping") => Ok(json!({})),
        Some("tools/list") => Ok(json!({"tools": Bridge::tool_definitions()})),
        Some("tools/call") => tools_call(bridge, &params),
        Some(other) => Err((-32601, format!("Method not found: {other}"))),
    };
    let id = id.unwrap_or(Value::Null);
    Some(match result {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err((code, message)) => {
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
        }
    })
}

/// The tool a host can dispatch or check against its own state.
/// Requests rejected here fall through to [`handle`] for the response.
pub fn called_tool(message: &Value) -> Option<&str> {
    if request_method(message) != Some("tools/call") || message.get("id").is_none() {
        return None;
    }
    let params = message.get("params")?;
    let name = params.get("name")?.as_str()?;
    // The app can save directly to update its document. Validate before it
    // takes that path, and let the bridge format any argument failure.
    if name == "scene_save" {
        let args = params.get("arguments")?.as_object()?;
        crate::bridge::scene_save_args(args).ok()?;
    }
    Some(name)
}

/// A `tools/call` answer that refuses the call with `message`, in the same
/// shape as a failed tool, so the agent reads it rather than seeing a
/// protocol error.
pub fn refusal(message: &Value, text: &str) -> Value {
    answer(message, Err(text.to_string()))
}

/// The `tools/call` answer to `message` for a tool that returned `result`.
pub fn answer(message: &Value, result: Result<Value, String>) -> Value {
    json!({"jsonrpc": "2.0", "id": message.get("id").cloned().unwrap_or(Value::Null),
        "result": tool_result(result)})
}

/// The `tools/call` answer to `message` with a PNG image next to the
/// structured result (MCP image content), for viewport captures.
pub fn answer_with_image(message: &Value, value: Value, png: &[u8]) -> Value {
    json!({"jsonrpc": "2.0", "id": message.get("id").cloned().unwrap_or(Value::Null),
    "result": {
        "content": [
            {"type": "text", "text": value.to_string()},
            {"type": "image", "data": base64(png), "mimeType": "image/png"},
        ],
        "structuredContent": value,
        "isError": false,
    }})
}

/// Standard base64 with padding (RFC 4648).
pub fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn tool_result(result: Result<Value, String>) -> Value {
    match result {
        Ok(value) => json!({
            "content": [{"type": "text", "text": value.to_string()}],
            "structuredContent": value,
            "isError": false,
        }),
        // Tool failures are results, not protocol errors, so the agent sees them.
        Err(message) => json!({
            "content": [{"type": "text", "text": message}],
            "isError": true,
        }),
    }
}

/// Tools that leave the scene, selection and history unchanged.
/// Kept in sorted order (case-insensitive), so parallel branches add
/// lines in different places (AGENTS.md, "Code"); a test checks it.
pub const NON_EDITING_TOOLS: [&str; 12] = [
    "command_help",
    "item_get",
    "material_get",
    "mesh_get",
    "panel_show",
    "preset_save",
    "render_settings_get",
    "render_status",
    "scene_info",
    "scene_save",
    "shader_tree_get",
    "viewport_capture",
];

/// Parse one line and handle it; malformed JSON gets a parse error.
pub fn handle_line(bridge: &mut Bridge, line: &str) -> Option<Value> {
    match serde_json::from_str::<Value>(line) {
        Ok(message) => handle(bridge, &message),
        Err(e) => Some(json!({"jsonrpc": "2.0", "id": null,
            "error": {"code": -32700, "message": format!("Parse error: {e}")}})),
    }
}

fn initialize(params: &Value) -> Value {
    let requested = params.get("protocolVersion").and_then(Value::as_str);
    let version = requested
        .filter(|v| PROTOCOL_VERSIONS.contains(v))
        .unwrap_or(PROTOCOL_VERSIONS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": {"tools": {"listChanged": false}},
        "serverInfo": {"name": "operandi-mcp", "version": env!("CARGO_PKG_VERSION")},
        "instructions": "Operandi modeling session (headless, or the running app when attached). Read state with scene_info and mesh_get; edit with select, selection_op, tool_run and command_run; every edit is one undo step."
    })
}

fn tools_call(bridge: &mut Bridge, params: &Value) -> Result<Value, (i64, String)> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((-32602, "tools/call needs a name".to_string()))?;
    if !Bridge::tool_definitions().iter().any(|t| t["name"] == name) {
        return Err((-32602, format!("Unknown tool: {name}")));
    }
    let args = params.get("arguments").unwrap_or(&Value::Null);
    Ok(tool_result(bridge.call(name, args)))
}
