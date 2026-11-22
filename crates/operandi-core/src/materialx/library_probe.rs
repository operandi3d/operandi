//! An opt-in probe for MaterialX libraries (groundwork for online
//! libraries, A34): every `.mtlx` under a folder is imported as Operandi
//! would, and the report says, per material, whether it became tree
//! layers, a graph layer, or was refused, and why.
//!
//! ```text
//! OPERANDI_MX_LIBRARY=<folder> cargo test --locked --workspace -- \
//!     library_probe --ignored --nocapture
//! ```
use std::path::{Path, PathBuf};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            files(&p, out);
        } else if p
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("mtlx"))
        {
            out.push(p);
        }
    }
}

/// One line per material: "layers", "graph" or "refused: <why>".
pub fn report(dir: &Path) -> Vec<(String, String)> {
    let mut paths = Vec::new();
    files(dir, &mut paths);
    let mut out = Vec::new();
    for path in paths {
        let rel = path
            .strip_prefix(dir)
            .unwrap_or(&path)
            .display()
            .to_string();
        let doc = match super::read(&path) {
            Ok(d) => d,
            Err(e) => {
                out.push((rel, format!("refused: {e}")));
                continue;
            }
        };
        let names = super::material_names(&doc);
        if names.is_empty() {
            out.push((rel, "no materials (a library or look file)".into()));
            continue;
        }
        for name in names {
            let folder = path.parent().unwrap_or(Path::new(""));
            let what = match super::import::preset(&doc, Some(&name), folder, &|_| Some((2, 2))) {
                Ok((p, _)) if p.scene.shader_tree.node_graphs.graphs.is_empty() => {
                    "layers".to_string()
                }
                Ok(_) => "graph".to_string(),
                Err(e) => format!("refused: {e}"),
            };
            out.push((format!("{rel} {name}"), what));
        }
    }
    out
}

#[test]
#[ignore = "needs OPERANDI_MX_LIBRARY=<folder of .mtlx files>"]
fn library_probe() {
    let Ok(dir) = std::env::var("OPERANDI_MX_LIBRARY") else {
        eprintln!("set OPERANDI_MX_LIBRARY to a folder of .mtlx files");
        return;
    };
    let rows = report(Path::new(&dir));
    let refused = rows
        .iter()
        .filter(|(_, w)| w.starts_with("refused"))
        .count();
    for (m, w) in &rows {
        println!("{m}: {w}");
    }
    println!("{} materials, {refused} refused", rows.len());
}

/// MaterialX's own example materials (fixtures/materialx-examples) all
/// import, but for the Disney principled and hair shaders, which have no
/// Cycles translation yet.
#[test]
fn materialx_examples_import() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/materialx-examples");
    let rows = report(&dir);
    assert!(rows.len() >= 60, "{} materials", rows.len());
    let refused: Vec<&str> = rows
        .iter()
        .filter(|(_, w)| w.starts_with("refused"))
        .map(|(m, _)| m.as_str())
        .collect();
    assert!(
        refused
            .iter()
            .all(|m| m.starts_with("DisneyPrincipled/") || m.starts_with("SimpleHair/")),
        "{refused:?}"
    );
    assert_eq!(refused.len(), 6, "{refused:?}");
    for want in ["standard_surface_brick_procedural", "gltf_pbr_boombox"] {
        assert!(
            rows.iter().any(|(m, w)| m.contains(want) && w == "graph"),
            "{want}: {rows:?}"
        );
    }
}
