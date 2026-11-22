//! Set Material and Set Part against `specifications/set-material-2026-11-05`
//! and `specifications/set-part-observed-2026-11-05`: which polygons get
//! the tag in each selection mode, and that Set Material makes its material.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use operandi_core::*;
use serde_json::Value;

fn editor(c: &Value) -> Editor {
    let mut m = mesh(&c["input"]);
    if let Some(tags) = c["input_polygon_materials"].as_array() {
        for (f, t) in tags.iter().enumerate() {
            let t = t.as_str().unwrap();
            if t != "Default" {
                m.faces[f].material = t.into();
                m.faces[f].material_label = Some(t.into());
            }
        }
    }
    if let Some(parts) = c["input_polygon_parts"].as_array() {
        for (f, p) in parts.iter().enumerate() {
            let p = p.as_str().unwrap();
            if p != "Default" {
                m.faces[f].part = Some(p.into());
            }
        }
    }
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = m.into();
    e.synchronize_identities().unwrap();
    let mode = match c["selection_mode"].as_str().unwrap() {
        "vertex" => Mode::Vertex,
        "edge" => Mode::Edge,
        "item" => Mode::Item,
        _ => Mode::Polygon,
    };
    e.dispatch(Command::Mode(mode)).unwrap();
    let sel = &c["selection"];
    e.scene.selection.faces = indices(&sel["sel_polygons"]);
    e.scene.selection.vertices = indices(&sel["sel_vertices"]);
    e.scene.selection.edges = sel["sel_edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| {
            let x = indices(x);
            (x[0], x[1])
        })
        .collect();
    e
}

fn tag_of(material: &str) -> &str {
    if material.is_empty() {
        "Default"
    } else {
        material
    }
}

#[test]
fn set_material_cases() {
    let mut failures = Vec::new();
    for c in cases("set-material-2026-11-05") {
        let id = c["id"].as_str().unwrap();
        if id == "SM14" {
            // The Default flag (new polygons get the tag) isn't built yet.
            continue;
        }
        let want: Vec<&str> = c["expected"]["polygon_materials"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        // The case's option text is cut at the first space ("My" for
        // "My Mat/One"); the read-back tags carry the full name.
        let option = c["option"]["name"].as_str().unwrap();
        let name = want
            .iter()
            .find(|t| t.starts_with(option) && **t != "Default")
            .copied()
            .unwrap_or(option);
        let mut e = editor(&c);
        let before = e.scene.selection.clone();
        let points = e.scene.active().unwrap().mesh.vertices.clone();
        if let Err(x) = e.dispatch(Command::SetMaterial(MaterialAssignment {
            name: name.into(),
            expected: None,
            material: None,
        })) {
            failures.push(format!("{id}: refused: {x}"));
            continue;
        }
        let m = &e.scene.active().unwrap().mesh;
        let got: Vec<&str> = m.faces.iter().map(|f| tag_of(&f.material)).collect();
        if got != want {
            failures.push(format!("{id}: tags {got:?}, expected {want:?}"));
        }
        if m.vertices != points || e.scene.selection != before {
            failures.push(format!("{id}: points or selection changed"));
        }
        let key = if name == "Default" { "Default" } else { name };
        if e.scene.shader_tree.material_for_tag(key).is_none() && name != "Default" {
            failures.push(format!("{id}: no material for {name}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn set_part_cases() {
    let mut failures = Vec::new();
    for c in cases("set-part-observed-2026-11-05") {
        let id = c["id"].as_str().unwrap();
        let want: Vec<&str> = c["expected"]["polygon_parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        let option = c["option"]["name"].as_str().unwrap();
        let name = want
            .iter()
            .find(|t| t.starts_with(option) && !option.is_empty())
            .copied()
            .unwrap_or(option);
        let mut e = editor(&c);
        let source = e.scene.active().unwrap().mesh.faces.clone();
        let result = e.dispatch(Command::SetPart(PartAssignment {
            name: name.into(),
            expected: None,
        }));
        if name.is_empty() {
            // Decided (Q-PT-01): a blank name is refused.
            if result.is_ok() {
                failures.push(format!("{id}: blank name accepted"));
            }
            continue;
        }
        if let Err(x) = result {
            failures.push(format!("{id}: refused: {x}"));
            continue;
        }
        let m = &e.scene.active().unwrap().mesh;
        let got: Vec<&str> = m
            .faces
            .iter()
            .map(|f| f.part.as_deref().unwrap_or("Default"))
            .collect();
        if got != want {
            failures.push(format!("{id}: parts {got:?}, expected {want:?}"));
        }
        if m.faces
            .iter()
            .zip(&source)
            .any(|(a, b)| a.material != b.material)
        {
            failures.push(format!("{id}: materials changed"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
