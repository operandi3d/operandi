//! Item transforms through the live tools, typed values
//! (specifications/item-transforms-2026-11-02, IT-01/03/04/05/06/07).
//! Handle-drag cases (IT-02/08/09) depend on view scale and aren't here.
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;

fn oracles() -> Vec<Value> {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specifications/item-transforms-2026-11-02/ORACLES.json"
    ))
    .unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    doc["oracles"].as_array().unwrap().clone()
}
fn vec3(v: &Value) -> DVec3 {
    let a: Vec<f64> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    DVec3::new(a[0], a[1], a[2])
}
fn transform(v: &Value) -> scene::Transform {
    scene::Transform {
        translation: vec3(&v["position_m"]),
        rotation: vec3(&v["rotation_degrees"]).map(f64::to_radians),
        scale: vec3(&v["scale"]),
        zxy: v["rotation_order"] == "ZXY",
        order: None,
    }
}

#[test]
fn typed_item_transforms() {
    let mut checked = 0;
    for o in oracles() {
        let typed = &o["gesture"]["typed_values"];
        let Some((field, value)) = typed.as_object().and_then(|m| m.iter().next()) else {
            continue;
        };
        let id = o["id"].as_str().unwrap();
        let mut e = Editor::default();
        let mut ids = std::collections::BTreeMap::new();
        for name in ["main", "parent", "child"] {
            e.dispatch(Command::NewMesh).unwrap();
            let item = e.scene.items.last_mut().unwrap();
            item.transform = transform(&o["input"]["items"][name]);
            item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
            ids.insert(name, item.id);
        }
        e.scene
            .items
            .iter_mut()
            .find(|i| i.id == ids["child"])
            .unwrap()
            .parent = Some(ids["parent"]);
        let target = o["selection"]["items"][0].as_str().unwrap();
        e.dispatch(Command::Mode(Mode::Item)).unwrap();
        e.dispatch(Command::SelectItem(Some(ids[target]))).unwrap();
        let number: f64 = value
            .as_str()
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .trim_end_matches('%')
            .parse()
            .unwrap();
        let tool = match o["settings"]["tool"].as_str().unwrap() {
            "Rotate" => Tool::Rotate,
            "Scale" => Tool::Scale,
            _ => Tool::Move,
        };
        let axis = if field.ends_with('Y') {
            1
        } else if field.ends_with('Z') {
            2
        } else {
            0
        };
        let mut delta = if tool == Tool::Scale {
            DVec3::ONE
        } else {
            DVec3::ZERO
        };
        delta[axis] = if tool == Tool::Scale {
            number / 100.
        } else {
            number
        };
        e.dispatch(Command::Activate(tool)).unwrap();
        // IT-05 has Negative Scale off; the others on.
        let negative_scale = id != "IT-O-IT-05";
        let _ = e.dispatch(Command::Preview(Parameters {
            delta,
            negative_scale,
            ..Default::default()
        }));
        e.dispatch(Command::Drop).unwrap();
        for (name, item) in &ids {
            let want = transform(&o["expected"]["items"][*name]);
            let got = &e
                .scene
                .items
                .iter()
                .find(|i| i.id == *item)
                .unwrap()
                .transform;
            assert!(
                (got.translation - want.translation).length() < 1e-9,
                "{id} {name}"
            );
            assert!(
                (got.rotation - want.rotation).length() < 1e-9,
                "{id} {name}: {:?}",
                got.rotation
            );
            assert!(
                (got.scale - want.scale).length() < 1e-9,
                "{id} {name}: {:?}",
                got.scale
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 6);
}

/// IT-PHY-01: changing the rotation order keeps the Euler values, so the
/// item turns (its world matrix matches the measured ZXY one).
#[test]
fn rotation_order_change_keeps_the_angles() {
    let o = oracles()
        .into_iter()
        .find(|o| o["id"] == "IT-O-H3-rotation-order")
        .unwrap();
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    let id = e.scene.active().unwrap().id;
    let before = scene::Transform {
        rotation: DVec3::new(30., 45., 60.).map(f64::to_radians),
        ..Default::default()
    };
    e.scene.active_mut().unwrap().transform = before.clone();
    e.dispatch(Command::ItemTransform {
        item: id,
        transform: scene::Transform {
            zxy: true,
            order: None,
            ..before.clone()
        },
    })
    .unwrap();
    let t = e.scene.active().unwrap().transform.clone();
    assert!(t.zxy);
    assert_eq!(t.rotation, before.rotation);
    let m = t.matrix();
    let want = &o["expected"]["item"]["world_matrix"];
    for i in 0..4 {
        for j in 0..4 {
            let w = want[i][j].as_f64().unwrap();
            assert!(
                (m.col(i)[j] - w).abs() < 1e-5,
                "[{i}][{j}] {} vs {w}",
                m.col(i)[j]
            );
        }
    }
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().transform, before);
}
