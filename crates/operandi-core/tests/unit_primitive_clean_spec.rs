//! Unit Primitives against `specifications/unit-primitive-2026-11-05`
//! (part 2, UP cases): Ctrl+click adds a unit shape to the current mesh,
//! Shift+click makes a new mesh item named after the shape. Tool settings
//! are ignored.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "unit-primitive-2026-11-05";

/// Not built.
const SKIP: &[(&str, &str)] = &[];

fn command(id: &str) -> Command {
    use UnitDestination::*;
    match id {
        "UP01" | "UP02" | "UP03" => Command::InsertUnit {
            tool: Tool::Box,
            destination: ExistingMesh,
        },
        "UP04" => Command::InsertUnit {
            tool: Tool::Sphere,
            destination: ExistingMesh,
        },
        "UP05" => Command::InsertEllipsoidUnit(ExistingMesh),
        "UP06" => Command::InsertShapeUnit(UnitShape::Cone, ExistingMesh),
        "UP07" => Command::InsertUnit {
            tool: Tool::Cylinder,
            destination: ExistingMesh,
        },
        "UP08" => Command::InsertShapeUnit(UnitShape::Capsule, ExistingMesh),
        "UP09" => Command::InsertShapeUnit(UnitShape::Torus, ExistingMesh),
        "UP10" => Command::InsertUnit {
            tool: Tool::Box,
            destination: NewMesh,
        },
        "UP11" => Command::InsertUnit {
            tool: Tool::Sphere,
            destination: NewMesh,
        },
        // Plane Item (missing-tools-g2 MT5-05).
        "UP12" => Command::InsertShapeUnit(UnitShape::Plane, NewMesh),
        other => panic!("{other}"),
    }
}

fn run(c: &Value) -> std::result::Result<(), String> {
    let id = c["id"].as_str().unwrap();
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    if !c["input"].is_null() {
        e.scene.active_mut().unwrap().mesh = mesh(&c["input"]).into();
    }
    e.synchronize_identities().unwrap();
    if id == "UP03" {
        // Leave the Cube tool at other settings first (rule 9).
        e.dispatch(Command::Activate(Tool::Box)).unwrap();
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.size.x = 3.;
        p.segments = [2, 2, 2];
        e.dispatch(Command::Preview(p)).map_err(|x| x.to_string())?;
        e.dispatch(Command::Cancel).unwrap();
    }
    let before = e.scene.items.len();
    e.dispatch(command(id))
        .map_err(|x| format!("refused: {x}"))?;
    let want = &c["expected"];
    let items_after = want["items_after"].as_array().unwrap().len();
    let items_before = want["items_before"].as_array().unwrap().len();
    if e.scene.items.len() - before != items_after - items_before {
        return Err(format!("{} items", e.scene.items.len()));
    }
    let selected = want["selected_item_after"][0].as_str().unwrap();
    let active = e.scene.active().ok_or("no active item")?;
    if items_after > items_before && !selected.starts_with(&active.name) {
        return Err(format!(
            "active item {:?}, expected {selected:?}",
            active.name
        ));
    }
    same_mesh(&active.mesh, &want["meshes"][selected], 1e-5)
}

#[test]
fn every_unit_case_matches() {
    let mut failures = Vec::new();
    for c in cases(SPEC) {
        let id = c["id"].as_str().unwrap();
        if !id.starts_with("UP") || SKIP.iter().any(|(s, _)| *s == id) {
            continue;
        }
        if let Err(e) = run(&c) {
            failures.push(format!("{id}: {e}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Part 1 (UC cases, Cube): Position, Size, Min and Max are one box. Each
/// entry step goes through the same routes as the panel fields (Min/Max
/// hold the opposite bound; Position and Size edit directly), the read-back
/// is checked, then the Cube is built.
#[test]
fn cube_box_fields_follow_the_rules() {
    let mut failures = Vec::new();
    for c in cases(SPEC) {
        let id = c["id"].as_str().unwrap();
        let Some(steps) = c["entry_steps"].as_array() else {
            continue;
        };
        if c["description"].as_str().unwrap().starts_with("Sphere") {
            continue;
        }
        let (mut position, mut size) = (glam::DVec3::ZERO, glam::DVec3::ONE);
        let mut bad = None;
        for step in steps {
            let key = step["set"][0].as_str().unwrap();
            // Handle Mode and Axis don't change these fields (rules 2, 7).
            if matches!(key, "handleMode" | "axis") {
                continue;
            }
            let value = step["set"][1].as_f64().unwrap();
            let axis = "XYZ".find(&key[key.len() - 1..]).unwrap();
            match &key[..key.len() - 1] {
                "cen" => position[axis] = value,
                "size" => size[axis] = value,
                side => {
                    // A Min/Max field edit is a Resize of that side.
                    let b = BoxBounds {
                        min: position - size * 0.5,
                        max: position + size * 0.5,
                    }
                    .edit_side(axis, side == "max", value, BoxHandleMode::Resize)
                    .unwrap();
                    position = b.position();
                    size = b.size();
                }
            }
            let r = &step["readback"];
            for (i, a) in ["X", "Y", "Z"].iter().enumerate() {
                let want_c = r[format!("cen{a}")].as_f64().unwrap();
                let want_s = r[format!("size{a}")].as_f64().unwrap();
                if (position[i] - want_c).abs() > 1e-9 || (size[i] - want_s).abs() > 1e-9 {
                    bad = Some(format!("{key}: centre {position} size {size}"));
                }
            }
        }
        if let Some(b) = bad {
            failures.push(format!("{id}: {b}"));
            continue;
        }
        match operandi_core::primitive::segmented_cube(position, size, [1, 1, 1]) {
            Ok(m) => {
                if let Err(e) = same_mesh(&m, &c["expected"], 1e-5) {
                    failures.push(format!("{id}: {e}"));
                }
            }
            Err(e) => failures.push(format!("{id}: refused: {e}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
