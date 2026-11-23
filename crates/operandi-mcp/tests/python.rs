//! `python_run` through the bridge (Python scripting preview): a script
//! keys a bouncing ball and builds a mesh, all one undo step; a Python
//! error is an error that leaves the scene as it was. Without the
//! `python` feature the tool says Python isn't in this build.
use operandi_mcp::{Bridge, protocol};
use serde_json::{Value, json};

fn call(b: &mut Bridge, name: &str, arguments: Value) -> Result<Value, String> {
    let r = protocol::handle(
        b,
        &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": {"name": name, "arguments": arguments}}),
    )
    .expect("a request gets a response");
    let result = &r["result"];
    if result["isError"] == json!(true) {
        Err(result["content"][0]["text"].as_str().unwrap().to_string())
    } else {
        Ok(result["structuredContent"].clone())
    }
}

#[test]
fn python_run_is_listed() {
    let r = protocol::handle(
        &mut Bridge::new(),
        &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
    )
    .unwrap();
    let tools = r["result"]["tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t["name"] == "python_run"));
}

#[cfg(not(feature = "python"))]
#[test]
fn without_python_the_tool_says_so() {
    let e = call(
        &mut Bridge::new(),
        "python_run",
        json!({"code": "print(1)"}),
    )
    .unwrap_err();
    assert!(e.contains("Python isn't in this build"), "{e}");
}

#[cfg(feature = "python")]
#[test]
fn a_script_keys_a_ball_and_builds_a_mesh_in_one_step() {
    let mut b = Bridge::new();
    let undo = b.editor.undo.len();
    let out = call(
        &mut b,
        "python_run",
        json!({"code": "import operandi
operandi.run('mesh.new')
operandi.mesh_set([(0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0)], [[0, 1, 2, 3]])
ball = operandi.mesh_get()['item']
for frame, y in [(0, 2), (12, 0), (24, 2)]:
    operandi.set_time(frame)
    operandi.set_channel(ball, 'pos.Y', y)
    operandi.set_key(ball, 'pos.Y')
q = operandi.query('operandi.anim.query', frames='12')
print(q['channels'][0]['values']['12'])
"}),
    )
    .unwrap();
    assert_eq!(out["output"], "0.0\n");
    assert_eq!(b.editor.undo.len(), undo + 1, "the script is one step");
}

#[cfg(feature = "python")]
#[test]
fn a_python_error_is_an_error_and_changes_nothing() {
    let mut b = Bridge::new();
    let before = b.editor.scene.clone();
    let e = call(
        &mut b,
        "python_run",
        json!({"code": "import operandi\nprint('made it here')\noperandi.run('mesh.new')\nraise RuntimeError('boom')"}),
    )
    .unwrap_err();
    assert!(
        e.contains("made it here") && e.contains("RuntimeError: boom"),
        "{e}"
    );
    assert_eq!(b.editor.scene, before);
    // And Python still runs.
    let ok = call(&mut b, "python_run", json!({"code": "print(2 + 2)"})).unwrap();
    assert_eq!(ok["output"], "4\n");
}
