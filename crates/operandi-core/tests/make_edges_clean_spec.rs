//! Make with edges selected against `specifications/make-edges-2026-11-06`.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "make-edges-2026-11-06";

fn run(c: &Value) -> std::result::Result<(), String> {
    let input = &c["input"];
    let want = c["expected"]["after"]["meshes"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    let mut source = mesh(input);
    if let Some(types) = want["polygon_types"].as_array()
        && types.iter().all(|t| t == "line")
        && types.len() == source.faces.len()
    {
        source.attributes.representations = vec![
            PolygonRepresentation::Polyline {
                closed: false,
                curve: Default::default(),
            };
            source.faces.len()
        ];
    }
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = source.clone().into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.scene.selection.edges = want["selected_edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| {
            let x = indices(x);
            (x[0], x[1])
        })
        .collect();
    let result = e.dispatch(Command::MakePolygonAs {
        kind: MakeType::Auto,
        close: false,
    });
    let got = &e.scene.active().unwrap().mesh;
    let made = got.faces.len() > source.faces.len();
    let want_new = want["polygons"].as_array().unwrap().len() > source.faces.len();
    if !want_new {
        // A1, A2: nothing is made (a refusal or a no-op both leave the mesh).
        if made || **got != source {
            return Err(format!("made something ({result:?})"));
        }
        return Ok(());
    }
    result.map_err(|x| format!("refused: {x}"))?;
    same_mesh(got, want, 1e-6)?;
    if e.scene.selection.mode != Mode::Edge {
        return Err("mode changed".into());
    }
    let selected = indices(&want["selected_polygons"]);
    if e.scene.selection.faces.len() != selected.len() {
        return Err(format!("selected polygons {:?}", e.scene.selection.faces));
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
