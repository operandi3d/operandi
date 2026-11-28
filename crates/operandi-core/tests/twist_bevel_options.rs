//! Twist Bevel's opt-ins from the TwistBevel notes (2026-11-25): Stop
//! Before Fold, Even Interior and Keep Cap UVs, plus maps and morphs on new
//! points, SubD walls, exact undo/redo and world axes on a rotated item.
use glam::DVec3;
use operandi_core::*;

fn face(vertices: &[u32]) -> Face {
    Face {
        vertices: vertices.to_vec(),
        uv: None,
        material: "M".into(),
        material_label: None,
        smooth: false,
        part: None,
    }
}

/// A 3 × 3 grid of 1 m quads on z = 0, with UVs u = x / 3, v = y / 3.
fn grid() -> Mesh {
    let mut vertices = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            vertices.push(DVec3::new(x as f64, y as f64, 0.));
        }
    }
    let mut faces = Vec::new();
    for y in 0..3u32 {
        for x in 0..3u32 {
            let v = 4 * y + x;
            faces.push(face(&[v, v + 1, v + 5, v + 4]));
        }
    }
    let mut m = Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    };
    let rows = m
        .faces
        .iter()
        .map(|f| {
            f.vertices
                .iter()
                .map(|&v| Some(m.vertices[v as usize].truncate() / 3.))
                .collect()
        })
        .collect();
    m.uv_maps.insert("Texture".into(), rows);
    m
}

fn one_quad() -> Mesh {
    Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(0., 1., 0.),
        ],
        faces: vec![face(&[0, 1, 2, 3])].into(),
        ..Default::default()
    }
}

fn options(shift: f64, inset: f64) -> TwistBevelOptions {
    TwistBevelOptions {
        shift,
        inset,
        segments: 1,
        ..Default::default()
    }
}

fn twist(m: &Mesh, faces: &[u32], o: &TwistBevelOptions) -> Result<Mesh> {
    twist_bevel_geometry(m, faces, o, &TwistBevelPipe::default()).map(|r| r.mesh)
}

fn cap_points(m: &Mesh, f: usize) -> Vec<DVec3> {
    m.faces[f]
        .vertices
        .iter()
        .map(|&v| m.vertices[v as usize])
        .collect()
}

fn area_z(points: &[DVec3]) -> f64 {
    (1..points.len() - 1)
        .map(|i| (points[i] - points[0]).cross(points[i + 1] - points[0]).z)
        .sum::<f64>()
        / 2.
}

#[test]
fn stop_before_fold_stops_short_instead_of_refusing() {
    let quad = one_quad();
    // Default: an Inset past the middle folds the cap and is refused.
    assert!(twist(&quad, &[0], &options(0.5, 0.8)).is_err());
    let o = TwistBevelOptions {
        stop_at_fold: true,
        ..options(0.5, 0.8)
    };
    let m = twist(&quad, &[0], &o).unwrap();
    let cap = cap_points(&m, 0);
    assert!(area_z(&cap) > 0.);
    assert!(
        cap.iter().all(|c| (c.x - 0.5).abs() < 1e-5
            && (c.y - 0.5).abs() < 1e-5
            && (c.z - 0.5).abs() < 1e-9),
        "{cap:?}"
    );
    // An Inset that doesn't fold is untouched by the option.
    let small = TwistBevelOptions {
        stop_at_fold: true,
        ..options(0.5, 0.2)
    };
    assert_eq!(
        twist(&quad, &[0], &small).unwrap(),
        twist(&quad, &[0], &options(0.5, 0.2)).unwrap()
    );
}

#[test]
fn even_interior_shrinks_a_grouped_cap_evenly() {
    let g = grid();
    let all: Vec<u32> = (0..9).collect();
    let plain = twist(&g, &all, &options(0.5, 0.3)).unwrap();
    let o = TwistBevelOptions {
        even_interior: true,
        ..options(0.5, 0.3)
    };
    let even = twist(&g, &all, &o).unwrap();
    // Cap polygon 0 is the bottom-left quad: corners 0, 1, 5, 4.
    let at = |m: &Mesh, c: usize| cap_points(m, 0)[c];
    // Default: only the outline moves; the interior point stays.
    assert!((at(&plain, 1) - DVec3::new(1., 0.3, 0.5)).length() < 1e-9);
    assert!((at(&plain, 2) - DVec3::new(1., 1., 0.5)).length() < 1e-9);
    // Even Interior: the straight-run point keeps its third of the moved
    // edge (0.3 + 2.4 / 3) and the interior point follows.
    assert!((at(&even, 0) - DVec3::new(0.3, 0.3, 0.5)).length() < 1e-9);
    assert!((at(&even, 1) - DVec3::new(1.1, 0.3, 0.5)).length() < 1e-9);
    assert!((at(&even, 2) - DVec3::new(1.1, 1.1, 0.5)).length() < 1e-6);
    let sizes: Vec<f64> = (0..9).map(|f| area_z(&cap_points(&even, f))).collect();
    assert!(sizes.iter().all(|s| (s - 0.64).abs() < 1e-6), "{sizes:?}");
}

#[test]
fn keep_cap_uvs_leaves_the_caps_uvs_exact() {
    let g = grid();
    let uv = |m: &Mesh, f: usize| m.uv_maps["Texture"][f].clone();
    for group_polygons in [false, true] {
        let base = TwistBevelOptions {
            group_polygons,
            rotation: 30.,
            segments: 3,
            ..options(0.5, 0.2)
        };
        let off = twist(&g, &[4], &base).unwrap();
        assert_ne!(uv(&off, 4), uv(&g, 4), "the cap's UVs shrink by default");
        let on = twist(
            &g,
            &[4],
            &TwistBevelOptions {
                keep_cap_uvs: true,
                ..base
            },
        )
        .unwrap();
        assert_eq!(uv(&on, 4), uv(&g, 4));
        for f in g.faces.len()..on.faces.len() {
            assert!(uv(&on, f).iter().all(Option::is_some), "wall {f} has UVs");
        }
        assert_eq!(on.validate(), Ok(()));
    }
}

#[test]
fn new_points_carry_weights_colours_and_morphs() {
    let mut m = one_quad();
    let a = &mut m.attributes;
    a.relative_morphs.insert(
        "Bulge".into(),
        (0..4).map(|v| (v, DVec3::Z * v as f64)).collect(),
    );
    a.point_weights.insert(
        "Weight".into(),
        (0..4).map(|v| (v, v as f64 / 4.)).collect(),
    );
    a.corner_weights.insert(
        "Corner".into(),
        vec![vec![Some(0.1), Some(0.2), Some(0.3), Some(0.4)]],
    );
    let out = twist(&m, &[0], &options(0.5, 0.2)).unwrap();
    let at = &out.attributes;
    for (c, &v) in out.faces[0].vertices.iter().enumerate() {
        assert_eq!(at.relative_morphs["Bulge"][&v], DVec3::Z * c as f64);
        assert_eq!(at.point_weights["Weight"][&v], c as f64 / 4.);
    }
    assert_eq!(
        at.corner_weights["Corner"][0],
        vec![Some(0.1), Some(0.2), Some(0.3), Some(0.4)]
    );
    assert_eq!(out.validate(), Ok(()));
    // Plain Bevel still refuses morphs (BV-Q04), as before.
    let p = BevelOptions::default();
    assert!(bevel_geometry_with_profile(&m, &[0], 0.5, 0.2, false, &p, None, None).is_err());
}

fn subd_editor() -> Editor {
    let mut m = grid();
    m.attributes.representations = vec![PolygonRepresentation::Subdivs; m.faces.len()];
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = m.into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![4];
    e.synchronize_identities().unwrap();
    e
}

fn mesh(e: &Editor) -> Mesh {
    e.scene.active().unwrap().mesh.to_owned_value()
}

#[test]
fn subd_walls_and_exact_undo_redo_on_a_uv_mapped_subd_mesh() {
    let mut e = subd_editor();
    let before = mesh(&e);
    e.dispatch(Command::Activate(Tool::TwistBevel)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.twist_bevel = TwistBevelOptions {
        shift: 0.6,
        inset: 0.2,
        rotation: 45.,
        bend: 30.,
        keep_cap_uvs: true,
        even_interior: true,
        stop_at_fold: true,
        ..p.twist_bevel
    };
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let after = mesh(&e);
    assert!(after.faces.len() > before.faces.len());
    // Walls match their polygon's type: SubD stays SubD.
    assert!(
        after
            .attributes
            .representations
            .iter()
            .all(|&r| r == PolygonRepresentation::Subdivs)
    );
    assert_eq!(after.uv_maps["Texture"][4], before.uv_maps["Texture"][4]);
    e.dispatch(Command::Undo).unwrap();
    assert!(mesh(&e) == before, "undo");
    e.dispatch(Command::Redo).unwrap();
    assert!(mesh(&e) == after, "redo");
}

#[test]
fn bend_direction_zero_is_world_x_on_a_rotated_item() {
    let mut e = subd_editor();
    let id = e.scene.active().unwrap().id;
    e.scene.item_mut(id).unwrap().transform.rotation = DVec3::new(0., 0., 90f64.to_radians());
    let world = e.scene.world_matrix(id).unwrap();
    let base = world.transform_point3(e.scene.active().unwrap().mesh.center(4));
    e.dispatch(Command::Activate(Tool::TwistBevel)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.twist_bevel = TwistBevelOptions {
        shift: 1.,
        bend: 90.,
        segments: 8,
        ..p.twist_bevel
    };
    e.dispatch(Command::Preview(p)).unwrap();
    let m = mesh(&e);
    let cap = world.transform_point3(m.center(4));
    // The tube curls toward world +X, not the item's own X (world Y).
    assert!(cap.x - base.x > 0.5, "{base} -> {cap}");
    assert!((cap.y - base.y).abs() < 1e-6, "{base} -> {cap}");
}
