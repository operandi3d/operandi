//! New Mesh against `specifications/new-mesh-2026-11-05`: an empty mesh
//! named "Mesh" ("Mesh (2)" when taken), the only selected item, top level
//! for New Mesh Item and beside the selection for the item list's New.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "new-mesh-2026-11-05";

fn mode(name: &str) -> Mode {
    match name {
        "vertex" => Mode::Vertex,
        "edge" => Mode::Edge,
        "polygon" => Mode::Polygon,
        _ => Mode::Item,
    }
}

fn check(e: &Editor, want: &Value, want_mode: &str) -> std::result::Result<(), String> {
    let want = want.as_array().unwrap();
    if e.scene.items.len() != want.len() {
        return Err(format!(
            "{} items, expected {}",
            e.scene.items.len(),
            want.len()
        ));
    }
    let selected: Vec<u64> = e
        .scene
        .selection
        .item
        .into_iter()
        .chain(e.scene.selection.also.iter().copied())
        .collect();
    for (item, w) in e.scene.items.iter().zip(want) {
        let parent = item
            .parent
            .and_then(|p| e.scene.items.iter().find(|i| i.id == p))
            .map(|p| p.name.as_str());
        if item.name != w["name"].as_str().unwrap()
            || parent != w["parent"].as_str()
            || selected.contains(&item.id) != w["selected"].as_bool().unwrap()
            || item.mesh.vertices.len() as u64 != w["n_points"].as_u64().unwrap()
        {
            return Err(format!(
                "item {} (parent {parent:?}) doesn't match {w}",
                item.name
            ));
        }
    }
    if e.scene.selection.mode != mode(want_mode) {
        return Err("selection mode changed".into());
    }
    Ok(())
}

fn run(c: &Value) -> std::result::Result<(), String> {
    let mut e = Editor::default();
    let mut ids = Vec::new();
    for w in c["items_before"].as_array().unwrap() {
        e.scene.new_mesh().unwrap();
        let n = w["n_points"].as_u64().unwrap() as usize;
        let parent = w["parent"]
            .as_str()
            .and_then(|p| e.scene.items.iter().find(|i| i.name == p))
            .map(|i| i.id);
        let item = e.scene.items.last_mut().unwrap();
        item.parent = parent;
        item.name = w["name"].as_str().unwrap().into();
        let m = Mesh {
            vertices: (0..n).map(|i| glam::DVec3::new(i as f64, 0., 0.)).collect(),
            ..Default::default()
        };
        item.mesh = m.into();
        ids.push((
            e.scene.items.last().unwrap().id,
            w["selected"].as_bool().unwrap(),
        ));
    }
    let chosen: Vec<u64> = ids.iter().filter(|(_, s)| *s).map(|(i, _)| *i).collect();
    e.scene.selection = Selection {
        item: chosen.first().copied(),
        also: chosen.iter().skip(1).copied().collect(),
        mode: mode(c["selection_mode"].as_str().unwrap()),
        ..Default::default()
    };
    e.synchronize_identities().unwrap();
    let command = if c["command"].as_str().unwrap().starts_with("New Layer") {
        Command::NewMeshBeside
    } else {
        Command::NewMesh
    };
    e.dispatch(command.clone())
        .map_err(|x| format!("refused: {x}"))?;
    let want = &c["expected"];
    let want_mode = want["selection_mode_after"].as_str().unwrap();
    check(&e, &want["items_after"], want_mode)?;
    if !want["items_after_second_call"].is_null() {
        e.dispatch(command).unwrap();
        check(&e, &want["items_after_second_call"], want_mode)?;
    }
    // One undo step removes the new mesh.
    let count = e.scene.items.len();
    e.dispatch(Command::Undo).unwrap();
    if e.scene.items.len() != count - 1 {
        return Err("undo didn't remove the new mesh".into());
    }
    Ok(())
}

#[test]
fn every_case_matches() {
    let mut failures = Vec::new();
    for c in cases(SPEC) {
        if let Err(e) = run(&c) {
            failures.push(format!("{}: {e}", c["id"].as_str().unwrap()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
