//! Duplicate tools v1 (specifications/duplicate-tools). Acceptance
//! criteria:
//!
//! 1. Starting values match the spec (MR section 2, LT/RA section 3, CL and
//!    AR section 2); Lathe is labelled Lathe (DS-61).
//! 2. Activating changes nothing until the first click or Apply (DS-30);
//!    Cancel leaves the scene as it was; an empty mesh still arms (DS-50).
//! 3. Mirror: the beginner half-cube welds into one closed outward mesh
//!    (MR-50/51), Distance 0 leaves the seam open (MR-52), Angle hinges
//!    (MR-53), Replace Source and Flip Polygons (MR-54/55), Item mode makes
//!    a new mirrored item with positive scale (MR-56).
//! 4. Lathe: a profile line round Y closes with poles and no doubled seam
//!    (LT-40), part turns stay open or are capped (LT-41/42), a single edge
//!    makes a tube (LT-43), Offset makes a spring (LT-44), drawing
//!    direction doesn't change facing (LT-45).
//! 5. Clone, Array and Radial Array place their copies as CL-30..33,
//!    AR-30..34 and RA-30..34 say.
//! 6. Drop is one undo step named after the tool; Undo removes it whole
//!    (MR-57); the result saves and reopens unchanged; options carry to the
//!    next session, the placement doesn't.
use glam::DVec3;
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "M".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

fn cube_at(center: DVec3) -> Mesh {
    Mesh::box_mesh(center, DVec3::ONE, 0).unwrap()
}

/// MR-50: a 1 m cube on x = 0..1 with its x = 0 face removed.
fn half_cube(seam: f64) -> Mesh {
    let mut m = cube_at(DVec3::new(0.5, 0., 0.));
    for v in &mut m.vertices {
        if v.x < 0.5 {
            v.x = seam;
        }
    }
    let seam_face = m
        .faces
        .iter()
        .position(|f| f.vertices.iter().all(|&v| m.vertices[v as usize].x < 0.5))
        .unwrap();
    m.faces.remove(seam_face);
    m.validate().unwrap();
    m
}

/// An open polyline through `points`.
fn polyline(points: &[DVec3]) -> Mesh {
    let mut m = Mesh {
        vertices: points.to_vec(),
        ..Default::default()
    };
    m.faces.push(face((0..points.len() as u32).collect()));
    m.attributes.representations = vec![PolygonRepresentation::Polyline {
        closed: false,
        curve: CurveKind::Line,
    }];
    m.validate().unwrap();
    m
}

fn editor(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(mode)).unwrap();
    e
}
fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}
fn params(e: &Editor) -> Parameters {
    e.session.live.as_ref().unwrap().params.clone()
}
/// Activate `tool`, change its settings with `set`, place it (the first
/// click or Apply) and Drop.
fn run(e: &mut Editor, tool: Tool, set: impl FnOnce(&mut DuplicateParameters)) {
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = params(e);
    set(&mut p.duplicate);
    p.duplicate.placed = true;
    e.session.tweak_held = true;
    e.dispatch(Command::Preview(p)).unwrap();
    e.end_tweak();
    e.dispatch(Command::Drop).unwrap();
}

/// Signed volume: positive when every polygon faces outward.
fn volume(m: &Mesh) -> f64 {
    m.faces
        .iter()
        .enumerate()
        .filter(|(f, _)| !m.is_polyline(*f))
        .map(|(_, f)| {
            let p = |i: usize| m.vertices[f.vertices[i] as usize];
            (1..f.vertices.len() - 1)
                .map(|i| p(0).dot(p(i).cross(p(i + 1))) / 6.)
                .sum::<f64>()
        })
        .sum()
}

/// Every polygon edge is used exactly twice: a closed surface.
fn closed(m: &Mesh) -> bool {
    m.edge_counts().values().all(|&c| c == 2)
}

fn points_near(m: &Mesh, p: DVec3) -> usize {
    m.vertices.iter().filter(|v| v.distance(p) < 1e-9).count()
}

fn centres(m: &Mesh, per: usize) -> Vec<DVec3> {
    m.faces
        .chunks(per)
        .map(|c| {
            let pts: Vec<DVec3> = c
                .iter()
                .flat_map(|f| f.vertices.iter().map(|&v| m.vertices[v as usize]))
                .collect();
            pts.iter().sum::<DVec3>() / pts.len() as f64
        })
        .collect()
}

// ---- 1, 2: starting values, arming ----

#[test]
fn starting_values_follow_the_spec() {
    let p = DuplicateParameters::default();
    assert_eq!(p.mirror.axis, 0);
    assert_eq!(p.mirror.angle, 0.);
    assert_eq!(p.mirror.mode, MirrorMode::Axis);
    assert!(p.mirror.snap_world_axis);
    assert!(p.mirror.copies.merge && !p.mirror.copies.flip && !p.mirror.copies.replace_source);
    assert_eq!(p.mirror.copies.distance, 0.);
    // MR-22: Axis X has Left = Y, Up = Z.
    assert_eq!((p.mirror.left, p.mirror.up), (DVec3::Y, DVec3::Z));
    assert_eq!((p.lathe.circle.count, p.lathe.circle.axis), (8, 1));
    assert_eq!(p.lathe.circle.direction, DVec3::Y);
    assert_eq!((p.lathe.circle.start, p.lathe.circle.end), (0., 360.));
    assert!(p.lathe.sweep.cap_start && p.lathe.sweep.cap_end);
    assert_eq!(p.lathe.sweep.uvs_around, UvsAround::V);
    assert_eq!(p.radial.circle.count, 8);
    assert_eq!(p.clone.count, 1);
    assert_eq!(p.clone.scale, DVec3::splat(100.));
    assert_eq!(p.array.count, [2, 1, 2]);
    assert_eq!(p.array.offset, DVec3::ONE);
    assert_eq!(p.array.scale, DVec3::splat(100.));
    assert!(!DetriangulateOptions::default().keep_sets);
    for copies in [p.clone.copies, p.array.copies, p.radial.copies] {
        assert!(!copies.merge && !copies.flip && !copies.replace_source);
    }
    assert_eq!(Tool::Lathe.label(), "Radial Sweep");
    assert!(Tool::RadialArray.is_duplicate() && !Tool::AxisSlice.is_duplicate());
}

#[test]
fn arming_changes_nothing_and_cancel_restores() {
    for tool in [
        Tool::Mirror,
        Tool::Lathe,
        Tool::Clone,
        Tool::Array,
        Tool::RadialArray,
    ] {
        let mut e = editor(half_cube(0.), Mode::Polygon);
        let before = e.scene.clone();
        e.dispatch(Command::Activate(tool)).unwrap();
        assert_eq!(e.scene, before, "{tool:?} armed");
        let mut p = params(&e);
        p.duplicate.placed = true;
        e.dispatch(Command::Preview(p)).unwrap();
        assert_ne!(e.scene, before, "{tool:?} placed");
        e.dispatch(Command::Cancel).unwrap();
        assert_eq!(e.scene, before, "{tool:?} cancelled");
    }
    // DS-50: an empty mesh still arms; placing changes nothing.
    let mut e = editor(Mesh::default(), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::Mirror)).unwrap();
    let mut p = params(&e);
    p.duplicate.placed = true;
    e.dispatch(Command::Preview(p)).unwrap();
    assert!(mesh(&e).faces.is_empty());
    assert!(duplicate::nothing_to_copy(&e.scene));
}

// ---- 3: Mirror ----

#[test]
fn mirror_half_cube_welds_closed() {
    // MR-50 / MR-03.
    let mut e = editor(half_cube(0.), Mode::Polygon);
    run(&mut e, Tool::Mirror, |_| {});
    let m = mesh(&e);
    assert_eq!((m.faces.len(), m.vertices.len()), (10, 12));
    assert!(closed(m));
    assert!(volume(m) > 0.);
    // MR-51: a seam just off zero welds at Distance 1 mm, exactly onto
    // x = 0; at the starting Distance (0) it doesn't.
    let mut e = editor(half_cube(0.0004), Mode::Polygon);
    run(&mut e, Tool::Mirror, |d| d.mirror.copies.distance = 0.001);
    let m = mesh(&e);
    assert_eq!((m.faces.len(), m.vertices.len()), (10, 12));
    let seam: Vec<_> = m.vertices.iter().filter(|v| v.x.abs() < 0.01).collect();
    assert_eq!(seam.len(), 4);
    assert!(seam.iter().all(|v| v.x == 0.));
    // MR-52: Distance 0 leaves two touching open halves.
    let mut e = editor(half_cube(0.0004), Mode::Polygon);
    run(&mut e, Tool::Mirror, |d| d.mirror.copies.distance = 0.);
    let m = mesh(&e);
    assert_eq!((m.faces.len(), m.vertices.len()), (10, 16));
    assert!(!closed(m));
}

#[test]
fn mirror_angle_turns_the_plane_about_left() {
    // MR-53 probe case: a unit square on the floor, Axis X, Angle 90°,
    // merge off: the copy lands beside it, flipped in Z, on the shared edge.
    let square = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(0., 0., 1.),
            DVec3::new(1., 0., 1.),
            DVec3::new(1., 0., 0.),
        ],
        faces: vec![face(vec![0, 1, 2, 3])].into(),
        ..Default::default()
    };
    let mut e = editor(square, Mode::Polygon);
    run(&mut e, Tool::Mirror, |d| {
        d.mirror.angle = 90.;
        d.mirror.copies.merge = false;
    });
    let m = mesh(&e);
    assert_eq!((m.faces.len(), m.vertices.len()), (2, 8));
    let copy: Vec<DVec3> = m.faces[1]
        .vertices
        .iter()
        .map(|&v| m.vertices[v as usize])
        .collect();
    let e9 = 1e-9;
    assert!(
        copy.iter()
            .all(|p| (-e9..=1. + e9).contains(&p.x) && (-1. - e9..=e9).contains(&p.z)),
        "{copy:?}"
    );
    assert_eq!(copy.iter().filter(|p| p.z.abs() < e9).count(), 2);
    // Merged, the two share that edge.
    let mut e = editor(m.clone(), Mode::Polygon);
    e.scene.active_mut().unwrap().mesh = Mesh {
        faces: vec![m.faces[0].clone()].into(),
        vertices: m.vertices[..4].to_vec(),
        ..Default::default()
    }
    .into();
    run(&mut e, Tool::Mirror, |d| d.mirror.angle = 90.);
    assert_eq!(mesh(&e).vertices.len(), 6);
}

#[test]
fn mirror_replace_and_flip() {
    // MR-54: only the mirrored half remains.
    let mut e = editor(half_cube(0.), Mode::Polygon);
    run(&mut e, Tool::Mirror, |d| {
        d.mirror.copies.replace_source = true
    });
    let m = mesh(&e);
    assert_eq!((m.faces.len(), m.vertices.len()), (5, 8));
    assert!(m.vertices.iter().all(|v| v.x <= 1e-12));
    // MR-55: copies face inward; the original is untouched.
    let mut e = editor(half_cube(0.), Mode::Polygon);
    let original = mesh(&e).faces.clone();
    run(&mut e, Tool::Mirror, |d| {
        d.mirror.copies.flip = true;
        d.mirror.copies.merge = false;
    });
    let m = mesh(&e);
    assert_eq!(&m.faces[..5], &original[..]);
    let copy = Mesh {
        vertices: m.vertices.clone(),
        faces: (m.faces[5..].to_vec()).into(),
        ..Default::default()
    };
    assert!(volume(&copy) < 0.);
}

#[test]
fn mirror_item_mode_makes_a_positive_scale_item() {
    // MR-56.
    let mut e = editor(cube_at(DVec3::new(2., 0., 0.)), Mode::Item);
    let name = e.scene.active().unwrap().name.clone();
    let source = e.scene.active().unwrap().clone();
    run(&mut e, Tool::Mirror, |_| {});
    assert_eq!(e.scene.items.len(), 2);
    let copy = &e.scene.items[1];
    assert_eq!(copy.name, format!("{name} (2)"));
    assert!(copy.transform.scale.min_element() > 0.);
    assert_eq!(e.scene.items[0], source, "the source is untouched");
    assert_eq!(e.scene.selection.item, Some(source.id));
    let world = e.scene.world_matrix(copy.id).unwrap();
    let pts: Vec<DVec3> = copy
        .mesh
        .vertices
        .iter()
        .map(|&v| world.transform_point3(v))
        .collect();
    let centre = pts.iter().sum::<DVec3>() / pts.len() as f64;
    assert!(centre.abs_diff_eq(DVec3::new(-2., 0., 0.), 1e-9));
    assert!(volume(&copy.mesh) > 0.);
}

#[test]
fn mirror_drop_is_one_step_and_saves() {
    // MR-57 and criterion 6.
    let mut e = editor(half_cube(0.), Mode::Polygon);
    let before = e.scene.clone();
    let steps = e.history.undo.len();
    run(&mut e, Tool::Mirror, |_| {});
    assert_eq!(e.history.undo.len(), steps + 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Mirror");
    let after = e.scene.clone();
    let dir = std::env::temp_dir().join(format!("op-dup-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mirror.opscene");
    e.save_committed(&path).unwrap();
    let reopened = operandi_core::persistence::load(&path).unwrap();
    assert_eq!(reopened.items[0].mesh, after.items[0].mesh);
    std::fs::remove_dir_all(&dir).ok();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    // Options carry over; the placement doesn't.
    e.dispatch(Command::Activate(Tool::Mirror)).unwrap();
    let mut p = params(&e);
    assert!(!p.duplicate.placed);
    p.duplicate.mirror.axis = 2;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Activate(Tool::Mirror)).unwrap();
    assert_eq!(params(&e).duplicate.mirror.axis, 2);
}

// ---- 4: Lathe ----

fn glass() -> Vec<DVec3> {
    vec![
        DVec3::new(0., 1., 0.),
        DVec3::new(1., 0.5, 0.),
        DVec3::new(0.5, 0., 0.),
        DVec3::new(0., 0., 0.),
    ]
}

#[test]
fn lathe_full_turn_closes_with_poles() {
    // LT-40.
    let mut e = editor(polyline(&glass()), Mode::Polygon);
    run(&mut e, Tool::Lathe, |d| d.lathe.circle.count = 8);
    let m = mesh(&e);
    assert_eq!((m.faces.len(), m.vertices.len()), (24, 18));
    assert_eq!(points_near(m, DVec3::Y), 1);
    assert_eq!(points_near(m, DVec3::ZERO), 1);
    assert!(closed(m));
    assert!(volume(m) > 0.);
    assert!(!m.has_polylines(), "the profile line is used up (LT-13)");
    assert!(m.uv_maps.contains_key("Texture"));
    // LT-45: drawn the other way, the same facing.
    let mut reversed = glass();
    reversed.reverse();
    let mut e = editor(polyline(&reversed), Mode::Polygon);
    run(&mut e, Tool::Lathe, |d| d.lathe.circle.count = 8);
    assert!(volume(mesh(&e)) > 0.);
}

#[test]
fn lathe_part_turn_open_or_capped() {
    // LT-41: open half shell.
    let mut e = editor(polyline(&glass()), Mode::Polygon);
    run(&mut e, Tool::Lathe, |d| {
        d.lathe.circle.count = 8;
        d.lathe.circle.end = 180.;
        d.lathe.sweep.cap_start = false;
        d.lathe.sweep.cap_end = false;
    });
    let m = mesh(&e);
    assert_eq!((m.faces.len(), m.vertices.len()), (24, 20));
    assert!(!closed(m));
    // LT-42: capped, closed.
    let mut e = editor(polyline(&glass()), Mode::Polygon);
    run(&mut e, Tool::Lathe, |d| {
        d.lathe.circle.count = 8;
        d.lathe.circle.end = 180.;
        d.lathe.sweep.cap_start = true;
        d.lathe.sweep.cap_end = true;
    });
    let m = mesh(&e);
    assert_eq!(m.faces.len(), 26);
    assert!(closed(m));
    assert!(volume(m) > 0.);
}

#[test]
fn lathe_single_edge_makes_a_tube() {
    // LT-43, in Edge mode on a line's edge.
    let mut e = editor(
        polyline(&[DVec3::new(0.5, 0., 0.), DVec3::new(0.5, 1., 0.)]),
        Mode::Polygon,
    );
    run(&mut e, Tool::Lathe, |d| d.lathe.circle.count = 16);
    let m = mesh(&e);
    assert_eq!((m.faces.len(), m.vertices.len()), (16, 32));
    assert!(m.faces.iter().all(|f| f.vertices.len() == 4));
    assert!(m.edge_counts().values().all(|&c| c <= 2));
    assert!(volume(m).abs() < 1e-9 || volume(m) > 0.);
}

#[test]
fn lathe_offset_makes_a_spring() {
    // LT-44: a small square face, three turns rising.
    let square = Mesh {
        vertices: vec![
            DVec3::new(1., 0., -0.1),
            DVec3::new(1.2, 0., -0.1),
            DVec3::new(1.2, 0., 0.1),
            DVec3::new(1., 0., 0.1),
        ],
        faces: vec![face(vec![0, 3, 2, 1])].into(),
        ..Default::default()
    };
    let mut e = editor(square, Mode::Polygon);
    run(&mut e, Tool::Lathe, |d| {
        d.lathe.circle.count = 48;
        d.lathe.circle.end = 1080.;
        d.lathe.circle.offset = 0.1;
    });
    let m = mesh(&e);
    // The face stays (LT-13), four sides per step, and the two caps.
    assert_eq!(m.faces.len(), 1 + 4 * 48 + 2);
    let top = m.vertices.iter().map(|v| v.y).fold(f64::MIN, f64::max);
    assert!((top - 4.8).abs() < 1e-9);
}

/// An open Curve (passes through every point) on `points`.
fn curve(points: &[DVec3]) -> Mesh {
    let mut m = polyline(points);
    m.attributes.representations = vec![PolygonRepresentation::Polyline {
        closed: false,
        curve: CurveKind::Curve {
            start_control: false,
            end_control: false,
        },
    }];
    m
}

#[test]
fn radial_sweep_cuts_a_curve_about_seven_per_span() {
    // RS-06 table: a 9 point half circle off the axis, Count 8, makes 56
    // pieces: 448 polygons on 456 points (it was 36 a span: 2,304).
    let arc: Vec<DVec3> = (0..9)
        .map(|i| {
            let a = (-90. + 22.5 * i as f64).to_radians();
            DVec3::new(2. + a.cos(), a.sin(), 0.)
        })
        .collect();
    let mut e = editor(curve(&arc), Mode::Polygon);
    run(&mut e, Tool::Lathe, |d| d.lathe.circle.count = 8);
    let m = mesh(&e);
    assert_eq!((m.faces.len(), m.vertices.len()), (448, 456));
    assert!(!m.has_polylines());
}

#[test]
fn radial_sweep_bottle_is_hundreds_not_thousands() {
    // RS-12: Jason's test, a 15 point bottle with both ends on the axis
    // at Count 8 made 4,032 polygons; the reference makes about 700.
    let bottle = [
        (0., 0.),
        (0.8, 0.),
        (1., 0.1),
        (1., 1.),
        (1., 2.),
        (0.95, 2.5),
        (0.7, 3.),
        (0.4, 3.3),
        (0.3, 3.6),
        (0.3, 4.),
        (0.3, 4.3),
        (0.35, 4.4),
        (0.3, 4.5),
        (0.15, 4.5),
        (0., 4.5),
    ]
    .map(|(x, y)| DVec3::new(x, y, 0.));
    let pieces = operandi_core::curves::sweep_sample(
        &bottle,
        CurveKind::Curve {
            start_control: false,
            end_control: false,
        },
        false,
    )
    .len()
        - 1;
    let mut e = editor(curve(&bottle), Mode::Polygon);
    run(&mut e, Tool::Lathe, |d| d.lathe.circle.count = 8);
    let m = mesh(&e);
    assert_eq!(m.faces.len(), pieces * 8);
    assert!(
        (500..900).contains(&m.faces.len()),
        "{} polygons",
        m.faces.len()
    );
    // RS-05: each end on the axis is one point with a fan of triangles.
    let tris = m.faces.iter().filter(|f| f.vertices.len() == 3).count();
    assert_eq!(tris, 16);
    assert_eq!(m.vertices.len(), (pieces - 1) * 8 + 2);
    assert_eq!(points_near(m, DVec3::ZERO), 1);
    assert_eq!(points_near(m, DVec3::new(0., 4.5, 0.)), 1);
    assert!(closed(m));
}

// ---- 5: Clone, Array, Radial Array ----

#[test]
fn clone_places_a_row() {
    // CL-30.
    let mut e = editor(cube_at(DVec3::ZERO), Mode::Polygon);
    run(&mut e, Tool::Clone, |d| {
        d.clone.count = 3;
        d.clone.offset = DVec3::new(2., 0., 0.);
    });
    let c = centres(mesh(&e), 6);
    assert_eq!(c.len(), 4);
    for (i, centre) in c.iter().enumerate() {
        assert!(centre.abs_diff_eq(DVec3::new(2. * i as f64, 0., 0.), 1e-9));
    }
    // CL-31: touching cubes weld into one strip (internal walls go).
    let mut e = editor(cube_at(DVec3::ZERO), Mode::Polygon);
    run(&mut e, Tool::Clone, |d| {
        d.clone.count = 3;
        d.clone.offset = DVec3::X;
        d.clone.copies.merge = true;
    });
    let m = mesh(&e);
    assert!(closed(m));
    // A 4 × 1 × 1 strip: five slices of four points, 4 × 4 sides + 2 ends.
    assert_eq!((m.faces.len(), m.vertices.len()), (18, 20));
    assert!(volume(m) > 0.);
    // CL-32: Rotate Y 45° turns the copies 45°, 90°, 135°.
    let mut e = editor(cube_at(DVec3::ZERO), Mode::Polygon);
    run(&mut e, Tool::Clone, |d| {
        d.clone.count = 3;
        d.clone.offset = DVec3::new(2., 0., 0.);
        d.clone.rotate = DVec3::new(0., 45., 0.);
    });
    let m = mesh(&e);
    for i in 1..=3 {
        // The +Z face (polygon 1 of each cube) turns i × 45° about Y.
        let n = m.normal(6 * i + 1);
        let a = (45. * i as f64).to_radians();
        assert!(
            n.abs_diff_eq(DVec3::new(a.sin(), 0., a.cos()), 1e-9),
            "{i}: {n}"
        );
    }
    // CL-34: Scale 50 % compounds: 0.5, 0.25, 0.125 m at x = 2, 4, 6.
    let mut e = editor(cube_at(DVec3::ZERO), Mode::Polygon);
    run(&mut e, Tool::Clone, |d| {
        d.clone.count = 3;
        d.clone.offset = DVec3::new(2., 0., 0.);
        d.clone.scale = DVec3::splat(50.);
    });
    let m = mesh(&e);
    for i in 1..=3usize {
        let pts: Vec<DVec3> = m.faces[6 * i..6 * i + 6]
            .iter()
            .flat_map(|f| f.vertices.iter().map(|&v| m.vertices[v as usize]))
            .collect();
        let lo = pts.iter().fold(DVec3::INFINITY, |a, b| a.min(*b));
        let hi = pts.iter().fold(DVec3::NEG_INFINITY, |a, b| a.max(*b));
        let size = 0.5f64.powi(i as i32);
        assert!(((hi - lo).x - size).abs() < 1e-9, "{i}");
        assert!((((hi + lo) / 2.).x - 2. * i as f64).abs() < 1e-9);
    }
    // CL-33: Item mode, three new items.
    let mut e = editor(cube_at(DVec3::ZERO), Mode::Item);
    run(&mut e, Tool::Clone, |d| {
        d.clone.count = 3;
        d.clone.offset = DVec3::new(0., 0., 2.);
    });
    assert_eq!(e.scene.items.len(), 4);
    for (i, item) in e.scene.items.iter().enumerate() {
        assert!(
            item.transform
                .translation
                .abs_diff_eq(DVec3::new(0., 0., 2. * i as f64), 1e-9)
        );
    }
}

#[test]
fn array_fills_a_grid() {
    // AR-30.
    let mut e = editor(cube_at(DVec3::ZERO), Mode::Polygon);
    run(&mut e, Tool::Array, |d| {
        d.array.count = [3, 1, 2];
        d.array.offset = DVec3::new(1., 0., 1.);
    });
    assert_eq!(centres(mesh(&e), 6).len(), 6);
    // AR-31: Between spreads them first to last.
    let mut e = editor(cube_at(DVec3::ZERO), Mode::Polygon);
    run(&mut e, Tool::Array, |d| {
        d.array.count = [3, 1, 1];
        d.array.offset = DVec3::new(4., 0., 0.);
        d.array.between = true;
    });
    let c = centres(mesh(&e), 6);
    assert!(c[1].abs_diff_eq(DVec3::new(2., 0., 0.), 1e-9));
    assert!(c[2].abs_diff_eq(DVec3::new(4., 0., 0.), 1e-9));
    // AR-33: too many copies is refused.
    let mut e = editor(cube_at(DVec3::ZERO), Mode::Polygon);
    e.dispatch(Command::Activate(Tool::Array)).unwrap();
    let mut p = params(&e);
    p.duplicate.placed = true;
    p.duplicate.array.count = [600, 600, 1];
    let err = e.dispatch(Command::Preview(p)).unwrap_err();
    assert!(err.contains("Too many copies"), "{err}");
    // AR-34: a negative scale mirrors the copies, still facing out.
    let mut e = editor(cube_at(DVec3::ZERO), Mode::Polygon);
    run(&mut e, Tool::Array, |d| {
        d.array.count = [2, 1, 1];
        d.array.offset = DVec3::new(3., 0., 0.);
        d.array.scale = DVec3::new(-100., 100., 100.);
    });
    assert!(volume(mesh(&e)) > 0.);
}

#[test]
fn radial_array_round_a_circle() {
    // RA-30.
    let mut e = editor(cube_at(DVec3::new(2., 0., 0.)), Mode::Polygon);
    run(&mut e, Tool::RadialArray, |d| d.radial.circle.count = 4);
    let c = centres(mesh(&e), 6);
    let want = [
        DVec3::new(2., 0., 0.),
        DVec3::new(0., 0., -2.),
        DVec3::new(-2., 0., 0.),
        DVec3::new(0., 0., 2.),
    ];
    for (got, want) in c.iter().zip(want) {
        assert!(got.abs_diff_eq(want, 1e-9), "{got} vs {want}");
    }
    assert!(volume(mesh(&e)) > 0.);
    // RA-32: a two-turn spiral rising 0.25 a step.
    let mut e = editor(cube_at(DVec3::new(2., 0., 0.)), Mode::Polygon);
    run(&mut e, Tool::RadialArray, |d| {
        d.radial.circle.count = 24;
        d.radial.circle.end = 720.;
        d.radial.circle.offset = 0.25;
    });
    let c = centres(mesh(&e), 6);
    assert_eq!(c.len(), 24);
    assert!((c[23].y - 23. * 0.25).abs() < 1e-9);
    // RA-34: Replace Source with Start 30°.
    let mut e = editor(cube_at(DVec3::new(2., 0., 0.)), Mode::Polygon);
    run(&mut e, Tool::RadialArray, |d| {
        d.radial.circle.count = 4;
        d.radial.circle.start = 30.;
        d.radial.copies.replace_source = true;
    });
    let c = centres(mesh(&e), 6);
    assert_eq!(c.len(), 4);
    let first = DVec3::new(
        2. * 30f64.to_radians().cos(),
        0.,
        -2. * 30f64.to_radians().sin(),
    );
    assert!(c[0].abs_diff_eq(first, 1e-9), "{}", c[0]);
}

#[test]
fn radial_array_wedges_weld_into_a_ring() {
    // RA-33: a 60° wedge of a flat ring, six round Y, merged.
    let ring = |a: f64, r: f64| DVec3::new(r * a.cos(), 0., -r * a.sin());
    let a = 60f64.to_radians();
    let wedge = Mesh {
        vertices: vec![ring(0., 1.), ring(0., 2.), ring(a, 2.), ring(a, 1.)],
        faces: vec![face(vec![0, 3, 2, 1])].into(),
        ..Default::default()
    };
    let mut e = editor(wedge, Mode::Polygon);
    run(&mut e, Tool::RadialArray, |d| {
        d.radial.circle.count = 6;
        d.radial.copies.merge = true;
    });
    let m = mesh(&e);
    assert_eq!((m.faces.len(), m.vertices.len()), (6, 12));
    // One connected ring: every inner edge shared, borders round both rims.
    let counts = m.edge_counts();
    assert_eq!(counts.values().filter(|&&c| c == 2).count(), 6);
    assert_eq!(counts.values().filter(|&&c| c == 1).count(), 12);
}
