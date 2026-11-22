//! `operandi-mcp`: a headless Operandi session served over MCP stdio.
//!
//! `operandi-mcp`                 serve MCP on stdin/stdout (headless scene)
//! `operandi-mcp --attach [SOCK]` forward MCP on stdin/stdout to the running
//!                                app (Window > MCP Bridge), so the agent
//!                                drives the scene on screen
//! `operandi-mcp --recipe FILE`   run a JSON recipe (array of
//!                                `{"tool": name, "arguments": {...}}`) and
//!                                print one result line per step
use operandi_mcp::{Bridge, protocol};
use serde_json::{Value, json};
use std::io::{BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.as_slice() {
        [] => serve(),
        [flag, path] if flag == "--recipe" => recipe(path),
        [flag] if flag == "--attach" => attach(None),
        [flag, path] if flag == "--attach" => attach(Some(path)),
        _ => {
            eprintln!("usage: operandi-mcp [--recipe FILE | --attach [SOCKET]]");
            2
        }
    };
    std::process::exit(code);
}

#[cfg(unix)]
fn attach(path: Option<&String>) -> i32 {
    let path = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(operandi_mcp::server::default_socket_path);
    match operandi_mcp::server::attach(&path) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("operandi-mcp: {e}");
            1
        }
    }
}
#[cfg(not(unix))]
fn attach(_: Option<&String>) -> i32 {
    eprintln!("operandi-mcp: --attach needs a Unix socket (macOS or Linux)");
    2
}

fn serve() -> i32 {
    let mut bridge = Bridge::new();
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { return 1 };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = protocol::handle_line(&mut bridge, &line)
            && (writeln!(out, "{response}").is_err() || out.flush().is_err())
        {
            return 1;
        }
    }
    0
}

fn recipe(path: &str) -> i32 {
    let steps: Vec<Value> = match std::fs::read_to_string(path)
        .map_err(|e| e.to_string())
        .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
    {
        Ok(steps) => steps,
        Err(e) => {
            eprintln!("{path}: {e}");
            return 2;
        }
    };
    let mut bridge = Bridge::new();
    let mut failed = false;
    for (i, step) in steps.iter().enumerate() {
        let name = step["tool"].as_str().unwrap_or_default();
        let line = match bridge.call(name, &step["arguments"]) {
            Ok(result) => json!({"step": i, "tool": name, "ok": true, "result": result}),
            Err(error) => {
                failed = true;
                json!({"step": i, "tool": name, "ok": false, "error": error})
            }
        };
        println!("{line}");
        if failed {
            break;
        }
    }
    i32::from(failed)
}
