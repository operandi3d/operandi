//! Bloom and Glare through the bridge: render_settings_get reports them,
//! render_settings_set changes them in one undo step, refusals change
//! nothing, and they are saved with the scene.
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

fn get(b: &mut Bridge) -> Value {
    call(b, "render_settings_get", json!({})).unwrap()
}

#[test]
fn bloom_and_glare_are_read_set_undone_and_saved() {
    let mut b = Bridge::new();
    let got = get(&mut b);
    assert_eq!(got["bloom"], false);
    assert_eq!(got["glare"], false);
    assert_eq!(got["glare_streaks"], 4);
    assert!(
        got["units"]["bloom_radius"]
            .as_str()
            .unwrap()
            .contains("width")
    );
    call(
        &mut b,
        "render_settings_set",
        json!({"bloom": true, "bloom_threshold": 2.0, "bloom_intensity": 0.8,
               "bloom_radius": 0.1, "bloom_tint": [1.0, 0.5, 0.25],
               "glare": true, "glare_streaks": 6, "glare_angle": 0.5,
               "glare_length": 0.3, "glare_intensity": 0.2, "glare_threshold": 4.0,
               "glare_tint": [0.5, 0.5, 1.0]}),
    )
    .unwrap();
    let got = get(&mut b);
    assert_eq!(got["bloom"], true);
    assert_eq!(got["bloom_threshold"], 2.0);
    assert_eq!(got["bloom_intensity"], 0.8);
    assert_eq!(got["bloom_radius"], 0.1);
    assert_eq!(got["bloom_tint"], json!([1.0, 0.5, 0.25]));
    assert_eq!(got["glare"], true);
    assert_eq!(got["glare_streaks"], 6);
    assert!((got["glare_angle"].as_f64().unwrap() - 0.5).abs() < 1e-12);
    assert_eq!(got["glare_length"], 0.3);
    assert_eq!(got["glare_tint"], json!([0.5, 0.5, 1.0]));
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(
        info["undo"].as_array().unwrap().last().unwrap(),
        "Render Settings"
    );
    // Refusals change nothing.
    for bad in [
        json!({"glare_streaks": 5}),
        json!({"bloom_radius": 0}),
        json!({"bloom_intensity": -1}),
        json!({"glare_tint": [1, 1]}),
    ] {
        assert!(
            call(&mut b, "render_settings_set", bad.clone()).is_err(),
            "{bad}"
        );
    }
    assert_eq!(get(&mut b)["glare_streaks"], 6);
    // Saved and read back.
    let dir = std::env::temp_dir().join(format!("operandi-post-effects-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("glow.opscene");
    call(&mut b, "scene_save", json!({"path": path})).unwrap();
    let mut c = Bridge::new();
    call(&mut c, "scene_load", json!({"path": path})).unwrap();
    assert_eq!(get(&mut c)["bloom_radius"], 0.1);
    assert_eq!(get(&mut c)["glare_streaks"], 6);
    let _ = std::fs::remove_dir_all(&dir);
    // Undo turns them off again.
    call(&mut b, "undo", json!({})).unwrap();
    assert_eq!(get(&mut b)["bloom"], false);
}
