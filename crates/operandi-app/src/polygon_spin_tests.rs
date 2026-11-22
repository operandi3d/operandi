//! Synthetic app routes; physical input and native UI remain separate.
use super::*;
#[test]
fn saved_alt_v_binding_is_not_displaced() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../development/m1-spin-quad-20261029/input-map-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("spin-map-{}.json", std::process::id()));
    assert!(!path.exists());
    let mut map = BTreeMap::from([("Triple".to_string(), "Alt+V".to_string())]);
    std::fs::write(&path, serde_json::to_vec(&map).unwrap()).unwrap();
    let b = input::load_bindings(&path);
    assert_eq!(b["Triple"], "Alt+V");
    assert_eq!(b["Spin Quad"], "");
    map.insert("Spin Quad".into(), "".into());
    std::fs::write(&path, serde_json::to_vec(&map).unwrap()).unwrap();
    assert_eq!(input::load_bindings(&path)["Spin Quad"], "");
    std::fs::remove_file(path).unwrap();
}
