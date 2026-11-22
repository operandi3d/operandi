//! Socket-free checks of the JSON-RPC request and tool response envelopes.
use operandi_mcp::{Bridge, protocol};
use serde_json::{Value, json};

#[test]
fn parses_ping_and_preserves_a_string_id() {
    let mut bridge = Bridge::new();
    assert_eq!(
        protocol::handle_line(
            &mut bridge,
            r#"{"jsonrpc":"2.0","id":"ping-1","method":"ping"}"#,
        ),
        Some(json!({"jsonrpc": "2.0", "id": "ping-1", "result": {}})),
    );
}

#[test]
fn invalid_requests_and_methods_get_protocol_errors() {
    let mut bridge = Bridge::new();
    for (request, id, code, message) in [
        (
            "{",
            Value::Null,
            -32700,
            "Parse error: EOF while parsing an object at line 1 column 1",
        ),
        (
            r#"{"jsonrpc":"2.0"}"#,
            Value::Null,
            -32600,
            "Invalid Request",
        ),
        (
            r#"{"jsonrpc":"2.0","id":4,"method":42}"#,
            json!(4),
            -32600,
            "Invalid Request",
        ),
        (
            r#"{"jsonrpc":"2.0","id":5,"method":"missing"}"#,
            json!(5),
            -32601,
            "Method not found: missing",
        ),
    ] {
        assert_eq!(
            protocol::handle_line(&mut bridge, request),
            Some(json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})),
            "{request}",
        );
    }
}

#[test]
fn rejects_invalid_jsonrpc_versions_before_dispatch() {
    let mut bridge = Bridge::new();
    for version in [json!("1.0"), json!("2.1"), json!(2.0), Value::Null] {
        for (method, params) in [
            ("ping", Value::Null),
            (
                "tools/call",
                json!({"name": "scene_new", "arguments": {"empty_mesh": false}}),
            ),
        ] {
            let request = json!({"jsonrpc": version, "id": "bad-version", "method": method,
                                 "params": params});
            assert_eq!(
                protocol::handle_line(&mut bridge, &request.to_string()),
                Some(json!({"jsonrpc": "2.0", "id": "bad-version",
                            "error": {"code": -32600, "message": "Invalid Request"}})),
                "{request}",
            );
            assert_eq!(protocol::called_tool(&request), None, "{request}");
        }
    }
    assert!(bridge.editor.scene.active().is_some());
}

#[test]
fn malformed_tool_calls_and_tool_failures_have_different_envelopes() {
    let mut bridge = Bridge::new();
    for (params, message) in [
        (Value::Null, "tools/call needs a name"),
        (json!({"name": 8}), "tools/call needs a name"),
        (json!({"name": "missing"}), "Unknown tool: missing"),
    ] {
        let request = json!({"jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": params});
        assert_eq!(
            protocol::handle(&mut bridge, &request),
            Some(json!({"jsonrpc": "2.0", "id": 6, "error": {"code": -32602, "message": message}})),
            "{request}",
        );
    }

    let request = json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call",
                         "params": {"name": "scene_info", "arguments": []}});
    assert_eq!(
        protocol::handle(&mut bridge, &request),
        Some(json!({"jsonrpc": "2.0", "id": 7, "result": {
            "content": [{"type": "text", "text": "Arguments must be an object"}],
            "isError": true
        }})),
    );
}

#[test]
fn host_detects_tool_names_only_in_tool_calls() {
    let mut request = json!({"jsonrpc": "2.0", "id": 8, "method": "tools/call",
                             "params": {"name": "viewport_capture"}});
    assert_eq!(protocol::called_tool(&request), Some("viewport_capture"));
    request.as_object_mut().unwrap().remove("jsonrpc");
    assert_eq!(
        protocol::handle(&mut Bridge::new(), &request),
        Some(json!({"jsonrpc": "2.0", "id": 8,
                    "error": {"code": -32600, "message": "Invalid Request"}})),
    );
    assert_eq!(protocol::called_tool(&request), None);
    request["jsonrpc"] = json!("2.0");
    request["method"] = json!("tools/list");
    assert_eq!(protocol::called_tool(&request), None);
    request["method"] = json!("tools/call");
    request["params"]["name"] = json!(3);
    assert_eq!(protocol::called_tool(&request), None);
    assert_eq!(
        protocol::called_tool(&json!({"jsonrpc": "2.0", "id": 8, "method": "tools/call"})),
        None
    );
}

#[test]
fn host_formats_structured_results_images_and_refusals() {
    let request = json!({"id": "capture"});
    let value = json!({"width": 2});
    assert_eq!(
        protocol::answer(&request, Ok(value.clone())),
        json!({"jsonrpc": "2.0", "id": "capture", "result": {
            "content": [{"type": "text", "text": "{\"width\":2}"}],
            "structuredContent": {"width": 2},
            "isError": false
        }}),
    );
    assert_eq!(
        protocol::answer_with_image(&request, value, b"\x89PNG\r\n\x1a\n"),
        json!({"jsonrpc": "2.0", "id": "capture", "result": {
            "content": [
                {"type": "text", "text": "{\"width\":2}"},
                {"type": "image", "data": "iVBORw0KGgo=", "mimeType": "image/png"}
            ],
            "structuredContent": {"width": 2},
            "isError": false
        }}),
    );
    assert_eq!(
        protocol::refusal(&json!({}), "The viewport is unavailable"),
        json!({"jsonrpc": "2.0", "id": null, "result": {
            "content": [{"type": "text", "text": "The viewport is unavailable"}],
            "isError": true
        }}),
    );
}

#[test]
fn stdio_skips_blank_lines_and_notifications_and_recovers_after_parse_error() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(env!("CARGO_BIN_EXE_operandi-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            br#"
{"jsonrpc":"2.0","method":"notifications/initialized"}
{
{"jsonrpc":"1.0","id":8,"method":"ping"}
{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"render_settings_get"}}
"#,
        )
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", output.status);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let responses: Vec<Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(responses.len(), 3, "{stdout}");
    assert_eq!(responses[0]["error"]["code"], -32700);
    assert_eq!(
        responses[1],
        json!({"jsonrpc": "2.0", "id": 8,
               "error": {"code": -32600, "message": "Invalid Request"}}),
    );

    let response = &responses[2];
    assert_eq!(response["id"], 9);
    assert_eq!(response["result"]["isError"], false);
    let value = &response["result"]["structuredContent"];
    assert_eq!(value["render_camera"], Value::Null);
    assert_eq!(value["frame"], json!([1920, 1080]));
    assert_eq!(
        response["result"]["content"],
        json!([{"type": "text", "text": value.to_string()}]),
    );
}

/// Contributor rule (the maintainer, 2026-11-19): the bridge's tool lists
/// take one added line per new tool, in sorted order, so branches don't
/// clash.
#[test]
fn bridge_tool_lists_stay_sorted() {
    use operandi_mcp::bridge::{APP_TOOLS, TOOLS};
    let lists: [(&str, Vec<&str>); 3] = [
        ("bridge.rs TOOLS", TOOLS.iter().map(|(id, _)| *id).collect()),
        ("bridge.rs APP_TOOLS", APP_TOOLS.to_vec()),
        (
            "protocol.rs NON_EDITING_TOOLS",
            protocol::NON_EDITING_TOOLS.to_vec(),
        ),
    ];
    for (name, ids) in &lists {
        if let Some(i) = ids
            .windows(2)
            .position(|w| w[0].to_lowercase() > w[1].to_lowercase())
        {
            panic!(
                "{name} must stay in sorted order (case-insensitive); `{}` belongs before `{}`",
                ids[i + 1],
                ids[i]
            );
        }
    }
}
