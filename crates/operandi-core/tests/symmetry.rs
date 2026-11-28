//! Symmetry measured cases (specifications/symmetry-2026-11-01/ORACLES.json).
//! Inputs and expected values come from the spec's oracle file; the fixtures
//! are small inline meshes, no reference scene.
#![cfg_attr(not(operandi_specs), allow(dead_code, unused_imports))]
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;

#[cfg(operandi_specs)]
fn oracles() -> Vec<Value> {
    let text = std::fs::read_to_string(concat!(
        env!("OPERANDI_SPECS_DIR"),
        "/specifications/symmetry-2026-11-01/ORACLES.json"
    ))
    .unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    doc["oracles"].as_array().unwrap().clone()
}
#[cfg(operandi_specs)]
fn oracle(id: &str) -> Value {
    oracles()
        .into_iter()
        .find(|o| o["id"] == id)
        .unwrap_or_else(|| panic!("missing oracle {id}"))
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
fn indices(v: &Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u32)
        .collect()
}
fn close(a: DVec3, b: DVec3) {
    assert!((a - b).length() < 1e-5, "{a:?} != {b:?}");
}
/// An editor holding the oracle's input mesh, Symmetry from its settings.
fn editor(o: &Value) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let input = &o["input"];
    let mesh = Mesh {
        vertices: input["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(vec3)
            .collect(),
        faces: input["polygons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| Face {
                vertices: indices(p),
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            })
            .collect(),
        ..Default::default()
    };
    e.scene.active_mut().unwrap().mesh = mesh.into();
    let s = &o["settings"];
    let axis = match s["symmetry_axis"].as_str() {
        Some("x") => Some(0),
        Some("y") => Some(1),
        Some("z") => Some(2),
        _ => None,
    };
    let offset = s["offset"].as_f64().unwrap_or(0.);
    e.dispatch(Command::Symmetry(Symmetry {
        axis,
        offset,
        ..Symmetry::OFF
    }))
    .unwrap();
    e
}
fn select_vertices(e: &mut Editor, vertices: &[u32]) {
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = vertices.to_vec();
}
fn points(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.clone()
}
/// A typed transform as the oracles were recorded: by script, with the
/// tool's centre at the world origin. Changed for the ACS rulebook: ACS-03
/// makes the side the centre is on the primary side (on the plane counts
/// as −), which is SYM-TR-02's "negative side as entered" for a centre at
/// the origin; the centre is now placed there explicitly (ACS-07) because
/// Operandi's own handle sits on the + half (ACS-01).
fn transform(e: &mut Editor, tool: Tool, delta: DVec3) {
    e.dispatch(Command::Activate(tool)).unwrap();
    e.dispatch(Command::SetActionCenter(DVec3::ZERO)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
}

/// SYM-TR-01 (SYM-O-01): only the selected point moves, mirrored on the
/// positive side; its unselected counterpart stays; selection is unchanged.
#[test]
#[cfg(operandi_specs)]
fn sym_o01_one_sided_move_leaves_counterpart() {
    let o = oracle("SYM-O-01");
    let mut e = editor(&o);
    select_vertices(&mut e, &indices(&o["settings"]["selected_vertices"]));
    transform(&mut e, Tool::Move, vec3(&o["settings"]["move"]));
    for (got, want) in points(&e)
        .iter()
        .zip(o["expected"]["points"].as_array().unwrap())
    {
        close(*got, vec3(want));
    }
    assert_eq!(
        e.scene.selection.vertices,
        indices(&o["expected"]["selected_vertices"])
    );
}

/// SYM-TR-02/03 (SYM-O-02): a selected mirrored pair stays mirrored, each
/// point moved once.
#[test]
#[cfg(operandi_specs)]
fn sym_o02_mirrored_pair_stays_mirrored() {
    let o = oracle("SYM-O-02");
    let mut e = editor(&o);
    select_vertices(&mut e, &indices(&o["settings"]["selected_vertices"]));
    transform(&mut e, Tool::Move, vec3(&o["settings"]["move"]));
    for (got, want) in points(&e)
        .iter()
        .zip(o["expected"]["points"].as_array().unwrap())
    {
        close(*got, vec3(want));
    }
}

/// SYM-TR-02: negative side as entered; positive side with X negated; axis Y.
#[test]
#[cfg(operandi_specs)]
fn sym_tr02_side_rules() {
    let o = oracle("SYM-O-01");
    for (vertex, delta, want) in [
        (5, DVec3::new(0.25, 0.1, 0.), DVec3::new(-1.75, 1.1, 0.)),
        (9, DVec3::new(0.25, 0.1, 0.), DVec3::new(1.75, 1.1, 0.)),
        (8, DVec3::new(0.25, 0.1, 0.3), DVec3::new(0.75, 1.1, 0.3)),
    ] {
        let mut e = editor(&o);
        select_vertices(&mut e, &[vertex]);
        transform(&mut e, Tool::Move, delta);
        close(points(&e)[vertex as usize], want);
    }
    let mut e = editor(&o);
    e.dispatch(Command::Symmetry(Symmetry {
        axis: Some(1),
        offset: 0.,
        ..Symmetry::OFF
    }))
    .unwrap();
    select_vertices(&mut e, &[9]);
    transform(&mut e, Tool::Move, DVec3::new(0.25, 0.1, 0.));
    close(points(&e)[9], DVec3::new(2.25, 0.9, 0.));
}

/// SYM-TR-04 (SYM-O-03): a point on the plane never crosses it.
#[test]
#[cfg(operandi_specs)]
fn sym_o03_point_on_plane_stays_on_it() {
    let o = oracle("SYM-O-03");
    let moves = o["settings"]["moves_each_from_original"]
        .as_array()
        .unwrap();
    let wants = o["expected"]["point7_after_each"].as_array().unwrap();
    for (delta, want) in moves.iter().zip(wants) {
        let mut e = editor(&o);
        select_vertices(&mut e, &[7]);
        transform(&mut e, Tool::Move, vec3(delta));
        close(points(&e)[7], vec3(want));
    }
}

/// SYM-TR-05 (SYM-O-05, SYM-O-06): Rotate about Y at the origin mirrors the
/// rotation for positive-side points; Symmetry Off is the plain rotation.
/// Then Scale X 0.5, Y 2 on points 9 and 5.
#[test]
#[cfg(operandi_specs)]
fn sym_o05_o06_rotate_and_scale() {
    for id in ["SYM-O-05", "SYM-O-06"] {
        let o = oracle(id);
        let mut e = editor(&o);
        e.dispatch(Command::ActionContext(ActionContext {
            center: CenterMode::Origin,
            axis: AxisOrientation::World,
        }))
        .unwrap();
        select_vertices(&mut e, &indices(&o["settings"]["selected_vertices"]));
        transform(&mut e, Tool::Rotate, DVec3::new(0., 30., 0.));
        let p = points(&e);
        for (k, want) in o["expected"]["points_by_index"].as_object().unwrap() {
            close(p[k.parse::<usize>().unwrap()], vec3(want));
        }
        if id == "SYM-O-05" {
            select_vertices(&mut e, &[9, 5]);
            transform(&mut e, Tool::Scale, DVec3::new(0.5, 2., 1.));
            let p = points(&e);
            close(p[5], DVec3::new(-0.866025, 2., 1.));
            close(p[9], DVec3::new(0.866025, 2., 1.));
        }
    }
}

/// SYM-SEL-01 (SYM-O-04) in vertex, edge and polygon modes.
#[test]
#[cfg(operandi_specs)]
fn sym_o04_select_symmetrical_each_mode() {
    let o = oracle("SYM-O-04");
    let mut e = editor(&o);
    select_vertices(&mut e, &[9, 8]);
    e.dispatch(Command::SelectSymmetrical).unwrap();
    assert_eq!(
        e.scene.selection.vertices,
        indices(&o["expected"]["vertices"])
    );

    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.scene.selection.edges = vec![(8, 9)];
    e.dispatch(Command::SelectSymmetrical).unwrap();
    let mut edges: Vec<_> = e
        .scene
        .selection
        .edges
        .iter()
        .map(|&(a, b)| vec![a.min(b), a.max(b)])
        .collect();
    edges.sort();
    let want: Vec<Vec<u32>> = o["expected"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(indices)
        .collect();
    assert_eq!(edges, want);

    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![3];
    e.dispatch(Command::SelectSymmetrical).unwrap();
    assert_eq!(e.scene.selection.faces, indices(&o["expected"]["polygons"]));
}

/// SYM-SEL-02/03/04: Off does nothing; a point on the plane is its own
/// counterpart; an isolated point pairs by position.
#[test]
#[cfg(operandi_specs)]
fn sym_sel02_04_off_plane_and_isolated_points() {
    let o = oracle("SYM-O-04");
    let mut e = editor(&o);
    select_vertices(&mut e, &[9, 7]);
    e.dispatch(Command::SelectSymmetrical).unwrap();
    assert_eq!(e.scene.selection.vertices, vec![9, 7, 5]);

    let undo_depth = e.history.undo.len();
    e.dispatch(Command::Symmetry(Symmetry::OFF)).unwrap();
    select_vertices(&mut e, &[9]);
    let before = e.scene.clone();
    e.dispatch(Command::SelectSymmetrical).unwrap();
    assert_eq!(e.scene, before);
    assert_eq!(
        e.history.undo.len(),
        undo_depth,
        "Off records no history step"
    );

    // Two points in no polygon, 10 and 11, mirrored across x = 0.
    let mut e = editor(&o);
    e.scene
        .active_mut()
        .unwrap()
        .mesh
        .vertices
        .extend([DVec3::new(-2., 0., 5.), DVec3::new(2., 0., 5.)]);
    select_vertices(&mut e, &[10]);
    e.dispatch(Command::SelectSymmetrical).unwrap();
    assert_eq!(e.scene.selection.vertices, vec![10, 11]);
}

/// SYM-SEL-05: tolerance scales with the mesh. 5e-6 matches on the 4 m
/// strip, 8e-6 does not; ×100 and ×0.01 scale both thresholds.
#[test]
#[cfg(operandi_specs)]
fn sym_sel05_matching_tolerance_scales_with_mesh() {
    let o = oracle("SYM-O-04");
    for scale in [1., 100., 0.01] {
        for (error, matches) in [(5e-6, true), (8e-6, false)] {
            let mut e = editor(&o);
            let mesh = &mut e.scene.active_mut().unwrap().mesh;
            for v in mesh.vertices.iter_mut() {
                *v *= scale;
            }
            mesh.vertices[5].x -= error * scale;
            select_vertices(&mut e, &[9]);
            e.dispatch(Command::SelectSymmetrical).unwrap();
            let want = if matches { vec![9, 5] } else { vec![9] };
            assert_eq!(
                e.scene.selection.vertices, want,
                "scale {scale} error {error}"
            );
        }
    }
}

/// SYM-ST-02 (SYM-O-09, selection part): the plane is the item's own x = 0.
#[test]
#[cfg(operandi_specs)]
fn sym_o09_plane_in_item_coordinates() {
    let o = oracle("SYM-O-09");
    let mut e = editor(&o);
    let id = e.scene.active().unwrap().id;
    e.dispatch(Command::ItemTransform {
        item: id,
        transform: scene::Transform {
            translation: DVec3::new(3., 0., 0.),
            ..Default::default()
        },
    })
    .unwrap();
    select_vertices(&mut e, &indices(&o["settings"]["selected_vertices"]));
    e.dispatch(Command::SelectSymmetrical).unwrap();
    assert_eq!(
        e.scene.selection.vertices,
        indices(&o["expected"]["select_symmetrical_result"])
    );
    select_vertices(&mut e, &[9]);
    transform(&mut e, Tool::Move, DVec3::new(0.25, 0., 0.));
    close(
        points(&e)[9],
        vec3(&o["expected"]["point9_local_after_move_x_0.25"]),
    );
}

/// SYM-ST-03 / SYM-TR-06 (SYM-O-10): Offset 0.5 moves the plane and the sides.
#[test]
#[cfg(operandi_specs)]
fn sym_o10_offset_plane() {
    let o = oracle("SYM-O-10");
    let x = &o["expected"];
    for (v, key) in [
        (9, "select_symmetrical_of_9"),
        (8, "select_symmetrical_of_8"),
    ] {
        let mut e = editor(&o);
        select_vertices(&mut e, &[v]);
        e.dispatch(Command::SelectSymmetrical).unwrap();
        assert_eq!(e.scene.selection.vertices, indices(&x[key]));
    }
    for (v, key) in [
        (9, "point9_after_move_x_0.25"),
        (6, "point6_after_move_x_0.25"),
    ] {
        let mut e = editor(&o);
        select_vertices(&mut e, &[v]);
        transform(&mut e, Tool::Move, DVec3::new(0.25, 0., 0.));
        close(points(&e)[v as usize], vec3(&x[key]));
    }
}

/// SYM-ST-04: axes Y and Z use the planes y = 0 and z = 0.
#[test]
fn sym_st04_y_and_z_planes() {
    for axis in [1, 2] {
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        let mut vertices = Vec::new();
        for j in 0..3 {
            for i in 0..3 {
                let (a, b) = (i as f64 - 1., j as f64 - 1.);
                vertices.push(if axis == 1 {
                    DVec3::new(a, b, 0.)
                } else {
                    DVec3::new(a, 0., b)
                });
            }
        }
        e.scene.active_mut().unwrap().mesh = Mesh {
            vertices,
            ..Default::default()
        }
        .into();
        e.dispatch(Command::Symmetry(Symmetry {
            axis: Some(axis),
            offset: 0.,
            ..Symmetry::OFF
        }))
        .unwrap();
        select_vertices(&mut e, &[0]);
        e.dispatch(Command::SelectSymmetrical).unwrap();
        assert_eq!(e.scene.selection.vertices, vec![0, 6]);
    }
}

/// SYM-PR-01/02 (SYM-O-07): a Cube made with Symmetry X is made twice; the
/// mirrored copy follows with X negated and the winding reversed keeping the
/// first corner. The specified copy's own point order is the primitives
/// topic, so this compares the mirrored half against the first half.
#[test]
#[cfg(operandi_specs)]
fn sym_o07_cube_is_doubled() {
    let o = oracle("SYM-O-07");
    let x = &o["expected"];
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        position: DVec3::new(1., 0., 0.),
        size: DVec3::ONE,
        make_uvs: true,
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    let mesh = &e.scene.active().unwrap().mesh;
    assert_eq!(mesh.vertices.len(), x["points"].as_array().unwrap().len());
    assert_eq!(mesh.faces.len(), x["polygons"].as_array().unwrap().len());
    let (n, m) = (8, 6);
    for i in 0..n {
        let p = mesh.vertices[i];
        close(mesh.vertices[n + i], DVec3::new(-p.x, p.y, p.z));
    }
    for k in 0..m {
        let f = &mesh.faces[k].vertices;
        let g: Vec<u32> = mesh.faces[m + k]
            .vertices
            .iter()
            .map(|v| v - n as u32)
            .collect();
        assert_eq!(g, vec![f[0], f[3], f[2], f[1]]);
        // Normals stay outward: the copy's normal is the original's mirrored.
        let (a, b) = (mesh.normal(k), mesh.normal(m + k));
        close(b, DVec3::new(-a.x, a.y, a.z));
    }
    // The oracle's mirrored half follows the same rule from its first half.
    let pts: Vec<DVec3> = x["points"].as_array().unwrap().iter().map(vec3).collect();
    let polys: Vec<Vec<u32>> = x["polygons"]
        .as_array()
        .unwrap()
        .iter()
        .map(indices)
        .collect();
    for k in 0..m {
        let f = &polys[k];
        let g: Vec<u32> = polys[m + k].iter().map(|v| v - n as u32).collect();
        assert_eq!(g, vec![f[0], f[3], f[2], f[1]]);
    }
    for i in 0..n {
        close(pts[n + i], DVec3::new(-pts[i].x, pts[i].y, pts[i].z));
    }
    // UVs follow their corners.
    let uv = &mesh.uv_maps["Texture"];
    for k in 0..m {
        let (f, g) = (&uv[k], &uv[m + k]);
        assert_eq!(g, &vec![f[0], f[3], f[2], f[1]]);
    }
    // SYM-PR-04: nothing selected after creation.
    assert!(e.scene.selection.vertices.is_empty());
    assert!(e.scene.selection.faces.is_empty());
    // Undo removes both copies in one step.
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.active().unwrap().mesh.vertices.is_empty());
}

/// SYM-SZ-01/02 (SYM-O-11): Symmetrize Positive and Negative.
#[test]
#[cfg(operandi_specs)]
fn sym_o11_symmetrize() {
    let o = oracle("SYM-O-11");
    for (side, key) in [
        (SymmetrizeSide::Positive, "positive"),
        (SymmetrizeSide::Negative, "negative"),
    ] {
        let mut e = editor(&o);
        e.dispatch(Command::Symmetrize(side)).unwrap();
        let p = points(&e);
        close(p[9], vec3(&o["expected"][key]["p9"]));
        close(p[5], vec3(&o["expected"][key]["p5"]));
        close(p[7], DVec3::new(0., 1., 0.));
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(points(&e), editor(&o).scene.active().unwrap().mesh.vertices);
    }
    let mut e = editor(&o);
    e.dispatch(Command::Symmetry(Symmetry::OFF)).unwrap();
    assert!(
        e.dispatch(Command::Symmetrize(SymmetrizeSide::Positive))
            .is_err()
    );
}

/// Cancel leaves the scene as it was; undo/redo and save/reopen agree with
/// the committed mirrored move.
#[test]
#[cfg(operandi_specs)]
fn mirrored_move_cancel_undo_and_reopen() {
    let o = oracle("SYM-O-01");
    let mut e = editor(&o);
    select_vertices(&mut e, &[9, 5]);
    let before = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::new(0.25, 0.1, 0.),
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, before);

    transform(&mut e, Tool::Move, DVec3::new(0.25, 0.1, 0.));
    let after = e.scene.clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
    let reopened = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    assert_eq!(reopened, e.scene);
}

/// Changing Symmetry during a live move re-applies it; invalid settings are
/// refused and leave the setting alone.
#[test]
#[cfg(operandi_specs)]
fn symmetry_change_updates_live_move_and_refuses_invalid() {
    let o = oracle("SYM-O-01");
    let mut e = editor(&o);
    select_vertices(&mut e, &[9]);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    // The centre at the origin, as the oracle was recorded (ACS-03/07:
    // on the plane, so the − side is primary and +X point 9 is mirrored).
    e.dispatch(Command::SetActionCenter(DVec3::ZERO)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::new(0.25, 0., 0.),
        ..Default::default()
    }))
    .unwrap();
    close(points(&e)[9], DVec3::new(1.75, 1., 0.));
    e.dispatch(Command::Symmetry(Symmetry::OFF)).unwrap();
    close(points(&e)[9], DVec3::new(2.25, 1., 0.));
    assert!(
        e.dispatch(Command::Symmetry(Symmetry {
            axis: Some(4),
            offset: 0.,
            ..Symmetry::OFF
        }))
        .is_err()
    );
    assert!(
        e.dispatch(Command::Symmetry(Symmetry {
            axis: Some(0),
            offset: f64::NAN,
            ..Symmetry::OFF
        }))
        .is_err()
    );
    assert_eq!(e.symmetry, Symmetry::OFF);
}

/// SYM-B2-02 (SYM-O-S2-02-*): the matching tolerance follows the largest
/// absolute coordinate, so the same extents match differently when moved.
#[test]
#[cfg(operandi_specs)]
fn sym_b2_02_tolerance_follows_largest_coordinate() {
    let mut checked = 0;
    for o in oracles()
        .into_iter()
        .filter(|o| o["id"].as_str().unwrap().starts_with("SYM-O-S2-02"))
    {
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        e.scene.active_mut().unwrap().mesh = Mesh {
            vertices: o["input"]["points"]
                .as_array()
                .unwrap()
                .iter()
                .map(vec3)
                .collect(),
            faces: o["input"]["polygons"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| Face {
                    vertices: indices(p),
                    uv: None,
                    material: String::new(),
                    smooth: false,
                    part: None,
                    material_label: None,
                })
                .collect(),
            ..Default::default()
        }
        .into();
        e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
        select_vertices(&mut e, &indices(&o["selection"]["sel_vertices"]));
        e.dispatch(Command::SelectSymmetrical).unwrap();
        assert_eq!(
            e.scene.selection.vertices,
            indices(&o["expected"]["sel_vertices"]),
            "{}",
            o["id"]
        );
        checked += 1;
    }
    assert_eq!(checked, 58);
}

/// SYM-B2-05: a Cube centred on the plane is made once; off the plane, twice.
#[test]
fn sym_b2_05_cube_on_the_plane_is_not_doubled() {
    for (x, points) in [(0., 8), (0.25, 16)] {
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
        e.dispatch(Command::Activate(Tool::Box)).unwrap();
        e.dispatch(Command::Preview(Parameters {
            position: DVec3::new(x, 0., 0.),
            // Not the starting size, so the preview builds.
            size: DVec3::splat(2.),
            ..e.session.live.as_ref().unwrap().params.clone()
        }))
        .unwrap();
        e.dispatch(Command::Drop).unwrap();
        assert_eq!(
            e.scene.active().unwrap().mesh.vertices.len(),
            points,
            "x = {x}"
        );
    }
}

/// SYM-B2-10: with Symmetry on, a pick adds the mirror counterpart (polygon
/// 3 → [3, 0]); then a move of the pair stays mirrored. Off, a pick is just
/// the pick. Removing a pick removes its counterpart too (provisional).
#[test]
#[cfg(operandi_specs)]
fn sym_b2_10_pick_adds_the_counterpart() {
    let o = oracle("SYM-O-01");
    let mut e = editor(&o);
    let id = e.scene.active().unwrap().id;
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    let pick = |e: &mut Editor, hit: Component, remove: bool| {
        e.dispatch(Command::Select {
            item: id,
            hit,
            replace: !remove,
            remove,
        })
        .unwrap();
    };
    pick(&mut e, Component::Polygon(3), false);
    assert_eq!(e.scene.selection.faces, vec![3, 0]);
    pick(&mut e, Component::Polygon(0), true);
    assert!(e.scene.selection.faces.is_empty());

    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    pick(&mut e, Component::Vertex(9), false);
    assert_eq!(e.scene.selection.vertices, vec![9, 5]);
    transform(&mut e, Tool::Move, DVec3::new(0.25, 0.1, 0.));
    close(points(&e)[9], DVec3::new(1.75, 1.1, 0.));
    close(points(&e)[5], DVec3::new(-1.75, 1.1, 0.));

    let mut off = editor(&o);
    off.dispatch(Command::Symmetry(Symmetry::OFF)).unwrap();
    off.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    off.dispatch(Command::Select {
        item: id,
        hit: Component::Polygon(3),
        replace: true,
        remove: false,
    })
    .unwrap();
    assert_eq!(off.scene.selection.faces, vec![3]);
}

/// Jason, 2026-11-02, revised by the ACS rulebook (2026-11-19): after
/// picking a +X point (its partner joins), the handle sits on the + half
/// (ACS-01) and the values, dragged or typed alike, apply as entered on the
/// side the pivot is on, the partner mirrored (ACS-03/04). The old rule
/// "typed values as entered on the negative side" (SYM-TR-02) is replaced
/// by ACS-03.
#[test]
#[cfg(operandi_specs)]
fn handle_sits_on_the_pick_and_drags_as_dragged() {
    let o = oracle("SYM-O-01");
    let mut e = editor(&o);
    let id = e.scene.active().unwrap().id;
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.dispatch(Command::Select {
        item: id,
        hit: Component::Vertex(9),
        replace: true,
        remove: false,
    })
    .unwrap();
    assert_eq!(e.scene.selection.vertices, vec![9, 5]);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    close(
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center,
        DVec3::new(2., 1., 0.),
    );
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::new(0.25, 0.1, 0.),
        ..Default::default()
    }))
    .unwrap();
    close(points(&e)[9], DVec3::new(2.25, 1.1, 0.));
    close(points(&e)[5], DVec3::new(-2.25, 1.1, 0.));
    // Scale about the pick: each side scales about its own point.
    e.dispatch(Command::Cancel).unwrap();
    e.dispatch(Command::Activate(Tool::Scale)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::splat(2.),
        ..Default::default()
    }))
    .unwrap();
    close(points(&e)[9], DVec3::new(2., 1., 0.));
    close(points(&e)[5], DVec3::new(-2., 1., 0.));
}

fn mesh_of(points: &Value, polygons: &Value) -> Mesh {
    Mesh {
        vertices: points.as_array().unwrap().iter().map(vec3).collect(),
        faces: polygons
            .as_array()
            .unwrap()
            .iter()
            .map(|p| Face {
                vertices: indices(p),
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            })
            .collect(),
        ..Default::default()
    }
}

/// SYM-B2-07 (Topology), SYM-B2-08 (Work Plane) and SYM-B2-09 (Arbitrary):
/// Select Symmetrical and a typed Move against the measured results.
#[test]
#[cfg(operandi_specs)]
fn sym_b2_07_09_topology_work_plane_arbitrary() {
    for o in oracles().into_iter().filter(|o| {
        let id = o["id"].as_str().unwrap();
        id.starts_with("SYM-O-S2-08") || id.starts_with("SYM-O-S2-10")
    }) {
        let id = o["id"].as_str().unwrap().to_string();
        let s = &o["settings"];
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        e.scene.active_mut().unwrap().mesh =
            mesh_of(&o["input"]["points"], &o["input"]["polygons"]).into();
        let mut symmetry = Symmetry::x();
        if s["Symmetry"] == "Arbitrary" {
            symmetry.axis = Some(3);
            symmetry.vector = vec3(&s["Arbitrary Axis"]);
        }
        symmetry.topology = s["Topology"].as_bool().unwrap_or(false);
        if s["Work Plane"].as_bool() == Some(true) {
            symmetry.work_plane = true;
            e.dispatch(Command::Workplane(scene::Workplane {
                fixed: true,
                plane: 0,
                center: vec3(&s["Work Plane center m"]),
            }))
            .unwrap();
        }
        e.dispatch(Command::Symmetry(symmetry)).unwrap();
        select_vertices(&mut e, &indices(&o["selection"]["sel_vertices"]));
        if s["operation"] == "Move" && symmetry.work_plane {
            // ACS-Q8 (Work Plane open): the oracle doesn't record where the
            // scripted centre sat relative to the moved plane, so this case
            // runs with Operandi's own centre, the selection's + half
            // (ACS-01), which makes + primary as the oracle shows.
            e.dispatch(Command::Activate(Tool::Move)).unwrap();
            e.dispatch(Command::Preview(Parameters {
                delta: vec3(&s["Move m"]),
                ..Default::default()
            }))
            .unwrap();
            e.dispatch(Command::Drop).unwrap();
            for (got, want) in points(&e)
                .iter()
                .zip(o["expected"]["points"].as_array().unwrap())
            {
                assert!(
                    (*got - vec3(want)).length() < 1e-5,
                    "{id}: {got:?} vs {want:?}"
                );
            }
        } else if s["operation"] == "Move" {
            transform(&mut e, Tool::Move, vec3(&s["Move m"]));
            for (got, want) in points(&e)
                .iter()
                .zip(o["expected"]["points"].as_array().unwrap())
            {
                assert!(
                    (*got - vec3(want)).length() < 1e-5,
                    "{id}: {got:?} vs {want:?}"
                );
            }
        } else {
            e.dispatch(Command::SelectSymmetrical).unwrap();
            assert_eq!(
                e.scene.selection.vertices,
                indices(&o["expected"]["sel_vertices"]),
                "{id}"
            );
        }
    }
}

/// SYM-B2-04: with Symmetry X, the Ellipsoid and Cone are made twice, the
/// copy mirrored with the first corner kept (SYM-O-S2-04/05).
#[test]
#[cfg(operandi_specs)]
fn sym_b2_04_ellipsoid_and_cone_are_mirrored() {
    for (id, cone) in [("SYM-O-S2-04-X", false), ("SYM-O-S2-05-X", true)] {
        let o = oracle(id);
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
        if cone {
            e.dispatch(Command::ArmCone).unwrap();
            e.dispatch(Command::ConfigureCone(ConeParameters {
                position: DVec3::X,
                size: DVec3::new(0.5, 0.75, 0.5),
                sides: 8,
                segments: 1,
                ..Default::default()
            }))
            .unwrap();
        } else {
            e.dispatch(Command::ArmEllipsoid).unwrap();
            e.dispatch(Command::ConfigureEllipsoid(EllipsoidParameters {
                position: DVec3::X,
                radius: DVec3::splat(0.5),
                sides: 8,
                segments: 4,
                ..Default::default()
            }))
            .unwrap();
        }
        e.dispatch(Command::Apply).unwrap();
        e.dispatch(Command::Drop).unwrap();
        let mesh = &e.scene.active().unwrap().mesh;
        let x = &o["expected"];
        assert_eq!(
            mesh.vertices.len(),
            x["points"].as_array().unwrap().len(),
            "{id}"
        );
        assert_eq!(
            mesh.faces.len(),
            x["polygons"].as_array().unwrap().len(),
            "{id}"
        );
        let n = mesh.vertices.len() / 2;
        let m = mesh.faces.len() / 2;
        for i in 0..n {
            let p = mesh.vertices[i];
            close(mesh.vertices[n + i], DVec3::new(-p.x, p.y, p.z));
        }
        for k in 0..m {
            let f = &mesh.faces[k].vertices;
            let g: Vec<u32> = mesh.faces[m + k]
                .vertices
                .iter()
                .map(|v| v - n as u32)
                .collect();
            let mut want = vec![f[0]];
            want.extend(f[1..].iter().rev());
            assert_eq!(g, want, "{id} polygon {k}");
        }
    }
}
