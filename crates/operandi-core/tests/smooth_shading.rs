//! Smooth shading: the angle rule, amount, smoothing groups, the Smoothing
//! command's undo, and save/reopen (smooth-shading spec v0, provisional).
use glam::DVec3;
use operandi_core::shading::{SmoothingGroups, SmoothingSettings, mesh_corner_normals};
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// A capped `sides`-sided cylinder of radius 1 and height 2 around Y.
fn cylinder(sides: u32) -> Mesh {
    let mut vertices = Vec::new();
    for y in [-1.0, 1.0] {
        for i in 0..sides {
            let a = std::f64::consts::TAU * i as f64 / sides as f64;
            vertices.push(DVec3::new(a.cos(), y, -a.sin()));
        }
    }
    let mut faces: Vec<Face> = (0..sides)
        .map(|i| {
            let j = (i + 1) % sides;
            face(vec![i, j, sides + j, sides + i])
        })
        .collect();
    faces.push(face((0..sides).rev().collect()));
    faces.push(face((sides..2 * sides).collect()));
    let mesh = Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    };
    mesh.validate().unwrap();
    mesh
}

fn cube() -> Mesh {
    Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 1).unwrap()
}

fn with(f: impl FnOnce(&mut SmoothingSettings)) -> SmoothingSettings {
    let mut s = SmoothingSettings::default();
    f(&mut s);
    s
}

fn close(a: DVec3, b: DVec3) -> bool {
    (a - b).length() < 1e-9
}

#[test]
fn defaults_are_the_spec_values() {
    let s = SmoothingSettings::default();
    assert_eq!((s.amount, s.angle), (100.0, 40.0));
    assert!(s.crease_by_angle);
    assert_eq!(s.groups, SmoothingGroups::Crease);
    assert_eq!(Scene::default().smoothing, s);
}

#[test]
fn cube_stays_crisp_at_40_and_rounds_at_100() {
    let mesh = cube();
    // SS-A01: each corner normal is its own face normal.
    let normals = mesh_corner_normals(&mesh, &SmoothingSettings::default());
    for (f, corners) in normals.iter().enumerate() {
        let flat = mesh.normal(f).normalize();
        assert!(corners.iter().all(|&n| close(n, flat)));
    }
    // SS-A02: every corner points along its point's diagonal.
    let normals = mesh_corner_normals(&mesh, &with(|s| s.angle = 100.));
    for (f, corners) in normals.iter().enumerate() {
        for (c, &n) in corners.iter().enumerate() {
            let p = mesh.vertices[mesh.faces[f].vertices[c] as usize];
            assert!(close(n, p.normalize()), "{n} at {p}");
        }
    }
    // Crease by Smoothing Angle off acts like 180°.
    let off = mesh_corner_normals(&mesh, &with(|s| s.crease_by_angle = false));
    assert_eq!(off, normals);
}

#[test]
fn cylinder_sides_blend_and_caps_stay_hard() {
    // SS-A03: 24 sides are 15° apart; caps meet the sides at 90°.
    let mesh = cylinder(24);
    let normals = mesh_corner_normals(&mesh, &SmoothingSettings::default());
    let side = 0;
    for (c, &v) in mesh.faces[side].vertices.iter().enumerate() {
        let p = mesh.vertices[v as usize];
        let radial = DVec3::new(p.x, 0., p.z).normalize();
        assert!(close(normals[side][c], radial), "{}", normals[side][c]);
    }
    for cap in [24, 25] {
        let flat = mesh.normal(cap).normalize();
        assert!(normals[cap].iter().all(|&n| close(n, flat)));
    }
    // A 6-sided prism has 60° steps: faceted at 40°.
    let hex = cylinder(6);
    let normals = mesh_corner_normals(&hex, &SmoothingSettings::default());
    let flat = hex.normal(0).normalize();
    assert!(normals[0].iter().all(|&n| close(n, flat)));
}

#[test]
fn amount_blends_from_flat_to_smooth() {
    let mesh = cylinder(24);
    let flat = mesh.normal(0).normalize();
    // SS-A05: 0 % is faceted.
    let zero = mesh_corner_normals(&mesh, &with(|s| s.amount = 0.));
    assert!(zero[0].iter().all(|&n| close(n, flat)));
    let full = mesh_corner_normals(&mesh, &SmoothingSettings::default())[0][0];
    let half = mesh_corner_normals(&mesh, &with(|s| s.amount = 50.))[0][0];
    let over = mesh_corner_normals(&mesh, &with(|s| s.amount = 200.))[0][0];
    let angle = |n: DVec3| n.angle_between(flat);
    // Linear blend then normalize (provisional, SS-Q04): half way sits in
    // between, overshoot bends further than fully smooth.
    assert!(angle(half) > 1e-3 && angle(half) < angle(full));
    assert!((angle(half) - angle(full) / 2.).abs() < 1e-3);
    assert!(angle(over) > angle(full) + 1e-3);
    for n in [full, half, over] {
        assert!((n.length() - 1.).abs() < 1e-12);
    }
}

#[test]
fn smoothing_groups_crease_and_force() {
    // SS-A06/A07: 24 sides in bands of 6 tagged A, B, A, B.
    let mut mesh = cylinder(24);
    let mut labels: Vec<Option<String>> = (0..24)
        .map(|i| Some(if (i / 6) % 2 == 0 { "A" } else { "B" }.to_string()))
        .collect();
    labels.extend([None, None]);
    mesh.attributes.smoothing_labels = labels;
    let normals = mesh_corner_normals(&mesh, &SmoothingSettings::default());
    // Polygon 5 (A) and 6 (B) share points 6 and 30 but don't blend there:
    // polygon 5's corners at that edge lean only toward the A side.
    let at = |f: usize, v: u32| {
        let c = mesh.faces[f].vertices.iter().position(|&x| x == v).unwrap();
        normals[f][c]
    };
    assert!(!close(at(5, 6), at(6, 6)));
    // Inside a band they blend as before.
    assert!(close(at(4, 5), at(5, 5)));
    // Untagged caps share nothing with tagged sides (provisional, SS-Q07).
    let flat = mesh.normal(24).normalize();
    assert!(normals[24].iter().all(|&n| close(n, flat)));
    // "AB" shares with both neighbours.
    mesh.attributes.smoothing_labels[6] = Some("AB".into());
    let normals = mesh_corner_normals(&mesh, &SmoothingSettings::default());
    let c5 = mesh.faces[5].vertices.iter().position(|&x| x == 6).unwrap();
    let c6 = mesh.faces[6].vertices.iter().position(|&x| x == 6).unwrap();
    assert!(close(normals[5][c5], normals[6][c6]));
    // Off ignores the labels: the A/B edge blends again.
    mesh.attributes.smoothing_labels[6] = Some("B".into());
    let off = mesh_corner_normals(&mesh, &with(|s| s.groups = SmoothingGroups::Off));
    assert!(close(off[5][c5], off[6][c6]));

    // SS-A08: two quads at 60°, both tagged "A".
    let h = 60f64.to_radians();
    let mut pair = Mesh {
        vertices: vec![
            DVec3::new(-1., 0., 0.),
            DVec3::new(0., 0., 0.),
            DVec3::new(0., 0., 1.),
            DVec3::new(-1., 0., 1.),
            DVec3::new(h.cos(), h.sin(), 0.),
            DVec3::new(h.cos(), h.sin(), 1.),
        ],
        faces: vec![face(vec![0, 1, 2, 3]), face(vec![1, 4, 5, 2])].into(),
        ..Default::default()
    };
    pair.attributes.smoothing_labels = vec![Some("A".into()), Some("A".into())];
    let crease = mesh_corner_normals(&pair, &SmoothingSettings::default());
    assert!(close(crease[0][1], pair.normal(0).normalize()));
    let force = mesh_corner_normals(&pair, &with(|s| s.groups = SmoothingGroups::Force));
    assert!(close(force[0][1], force[1][0]));
    assert!(!close(force[0][1], pair.normal(0).normalize()));
}

fn cube_editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = cube().into();
    e
}

fn set(e: &mut Editor, settings: SmoothingSettings, continuing: bool) -> Result<()> {
    e.dispatch(Command::SetSmoothing {
        settings,
        continuing,
    })
}

#[test]
fn smoothing_command_undoes_and_a_field_drag_is_one_step() {
    let mut e = cube_editor();
    let start = e.scene.clone();
    set(&mut e, with(|s| s.angle = 50.), false).unwrap();
    set(&mut e, with(|s| s.angle = 60.), true).unwrap();
    set(&mut e, with(|s| s.angle = 70.), true).unwrap();
    assert_eq!(e.scene.smoothing.angle, 70.);
    // One drag, one step.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, start);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.smoothing.angle, 70.);
    // A new edit (not continuing) is its own step.
    set(&mut e, with(|s| s.amount = 50.), false).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.smoothing.angle, 70.);
    assert_eq!(e.scene.smoothing.amount, 100.);
    // Invalid values are refused and change nothing.
    let before = e.scene.clone();
    for bad in [
        with(|s| s.angle = -1.),
        with(|s| s.angle = f64::NAN),
        with(|s| s.amount = 1e6),
    ] {
        assert!(set(&mut e, bad, false).is_err());
        assert_eq!(e.scene, before);
    }
    // The same value again makes no step.
    let steps = e.undo.len();
    set(&mut e, before.smoothing.clone(), false).unwrap();
    assert_eq!(e.undo.len(), steps);
}

#[test]
fn smoothing_survives_save_and_reopen() {
    let mut e = cube_editor();
    // Default settings write nothing new: older readers still open it.
    let plain = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(!plain.contains("smoothing"));
    assert!(!plain.contains("\"version\": 12"));
    let settings = SmoothingSettings {
        amount: 75.,
        angle: 25.,
        crease_by_angle: false,
        groups: SmoothingGroups::Force,
    };
    set(&mut e, settings.clone(), false).unwrap();
    let bytes = persistence::encode(&e.scene).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("\"version\": 12"));
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back.smoothing, settings);
    assert_eq!(back, e.scene);
    // An older version claiming smoothing settings is refused.
    let old = String::from_utf8(bytes)
        .unwrap()
        .replace("\"version\": 12", "\"version\": 11");
    assert!(persistence::decode(old.as_bytes()).is_err());
}
