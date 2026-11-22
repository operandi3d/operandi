//! The in-app transport: a socket server whose requests a host answers on its
//! own thread, and `operandi-mcp --attach` forwarding stdio to it.
#![cfg(unix)]
use operandi_mcp::{Bridge, protocol, server::Server};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

fn socket(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("op-mcp-{}-{name}.sock", std::process::id()))
}

/// Answer requests like the app does, until `stop`.
fn host(server: Server, stop: Arc<AtomicBool>) -> std::thread::JoinHandle<Bridge> {
    std::thread::spawn(move || {
        let mut bridge = Bridge::new();
        while !stop.load(Ordering::SeqCst) {
            for r in server.take_requests() {
                let response = protocol::handle_line(&mut bridge, &r.line);
                r.respond(response);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        drop(server);
        bridge
    })
}

fn rpc(
    out: &mut impl Write,
    input: &mut impl BufRead,
    id: u64,
    method: &str,
    params: Value,
) -> Value {
    let msg = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
    writeln!(out, "{msg}").unwrap();
    out.flush().unwrap();
    let mut line = String::new();
    input.read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}

#[test]
fn socket_clients_drive_the_host_editor() {
    let path = socket("drive");
    let (woke, wakes) = mpsc::channel();
    let server = Server::start(&path, move || {
        let _ = woke.send(());
    })
    .unwrap();
    // Owner-only, and a second server can't take the path over.
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(
        Server::start(&path, || {})
            .err()
            .unwrap()
            .contains("already serving")
    );
    let stop = Arc::new(AtomicBool::new(false));
    let hosted = host(server, stop.clone());

    let stream = UnixStream::connect(&path).unwrap();
    let mut out = stream.try_clone().unwrap();
    let mut input = BufReader::new(stream);
    let init = rpc(&mut out, &mut input, 1, "initialize", json!({}));
    assert_eq!(init["result"]["serverInfo"]["name"], "operandi-mcp");
    assert!(
        wakes.recv_timeout(Duration::from_secs(5)).is_ok(),
        "host was woken"
    );
    // A notification gets no reply, so the next answer is for the next call.
    writeln!(
        out,
        r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#
    )
    .unwrap();
    let r = rpc(
        &mut out,
        &mut input,
        2,
        "tools/call",
        json!({"name": "tool_run", "arguments": {"tool": "prim.cube"}}),
    );
    assert_eq!(r["id"], 2);
    assert_eq!(r["result"]["structuredContent"]["active"]["faces"], 6);

    stop.store(true, Ordering::SeqCst);
    let bridge = hosted.join().unwrap();
    assert_eq!(bridge.editor.scene.active().unwrap().mesh.faces.len(), 6);
    // Switching off ends the connection and removes the socket file.
    let mut line = String::new();
    assert_eq!(input.read_line(&mut line).unwrap_or(0), 0);
    assert!(!path.exists());
}

#[test]
fn a_stale_socket_file_is_replaced() {
    let path = socket("stale");
    // A crash leaves the socket file behind with nobody listening.
    drop(std::os::unix::net::UnixListener::bind(&path).unwrap());
    assert!(path.exists());
    let server = Server::start(&path, || {}).unwrap();
    assert!(UnixStream::connect(&path).is_ok());
    drop(server);
    assert!(!path.exists());
    // Switching off releases the path for the next start.
    let again = Server::start(&path, || {}).unwrap();
    drop(again);
}

#[test]
fn attach_forwards_stdio_to_the_running_host() {
    let path = socket("attach");
    let server = Server::start(&path, || {}).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let hosted = host(server, stop.clone());

    let mut child = Command::new(env!("CARGO_BIN_EXE_operandi-mcp"))
        .arg("--attach")
        .arg(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let r = rpc(
        &mut stdin,
        &mut stdout,
        1,
        "tools/call",
        json!({"name": "tool_run", "arguments": {"tool": "prim.sphere"}}),
    );
    assert_eq!(r["result"]["isError"], false, "{r}");
    let info = rpc(
        &mut stdin,
        &mut stdout,
        2,
        "tools/call",
        json!({"name": "scene_info", "arguments": {}}),
    );
    assert_eq!(info["id"], 2);
    drop(stdin); // client hangs up
    assert!(child.wait().unwrap().success());

    stop.store(true, Ordering::SeqCst);
    let bridge = hosted.join().unwrap();
    assert!(!bridge.editor.scene.active().unwrap().mesh.faces.is_empty());
}

#[test]
fn attach_without_a_running_app_explains() {
    let out = Command::new(env!("CARGO_BIN_EXE_operandi-mcp"))
        .args(["--attach", "/nonexistent/operandi.sock"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("MCP Bridge"));
}
