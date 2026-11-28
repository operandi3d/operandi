//! SPEC-EDGE-EXTEND-ROTATE-SCALE v1: Edge Extend's Rotate, Scale, Local
//! Inset/Shift, Make UVs and Shift+click continue (EXT-RS-01..04 and the
//! tool lifecycle around them).
use glam::{DVec2, DVec3};
use operandi_core::*;

/// A 2 m square polygon in XZ centred at the origin; its 4 edges are the
/// boundary loop the cases extend.
fn square() -> Mesh {
    Mesh {
        vertices: vec![
            DVec3::new(-1., 0., -1.),
            DVec3::new(-1., 0., 1.),
            DVec3::new(1., 0., 1.),
            DVec3::new(1., 0., -1.),
        ],
        faces: vec![Face {
            vertices: vec![0, 1, 2, 3],
            uv: None,
            material: "Default".into(),
            smooth: false,
            part: None,
            material_label: None,
        }]
        .into(),
        ..Default::default()
    }
}
const LOOP: [(u32, u32); 4] = [(0, 1), (1, 2), (2, 3), (0, 3)];

fn editor(center: CenterMode) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = square().into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = LOOP.to_vec();
    e.dispatch(Command::ActionContext(ActionContext {
        center,
        axis: AxisOrientation::World,
    }))
    .unwrap();
    e.dispatch(Command::Activate(Tool::EdgeExtend)).unwrap();
    e
}
fn preview(e: &mut Editor, f: impl FnOnce(&mut Parameters)) {
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    f(&mut p);
    e.dispatch(Command::Preview(p)).unwrap();
}
fn mesh(e: &Editor) -> Mesh {
    (*e.scene.active().unwrap().mesh).clone()
}
/// The points of the selected (outer) edges, sorted.
fn outer_points(e: &Editor) -> Vec<DVec3> {
    let m = mesh(e);
    let mut v: Vec<u32> = e
        .scene
        .selection
        .edges
        .iter()
        .flat_map(|&(a, b)| [a, b])
        .collect();
    v.sort();
    v.dedup();
    let mut p: Vec<DVec3> = v.iter().map(|&i| m.vertices[i as usize]).collect();
    p.sort_by(|a, b| a.to_array().partial_cmp(&b.to_array()).unwrap());
    p
}
fn assert_square(points: &[DVec3], half: f64, y: f64) {
    assert_eq!(points.len(), 4, "{points:?}");
    for p in points {
        assert!((p.x.abs() - half).abs() < 1e-9, "{points:?}");
        assert!((p.z.abs() - half).abs() < 1e-9, "{points:?}");
        assert!((p.y - y).abs() < 1e-9, "{points:?}");
    }
}

#[test]
fn ext_rs_01_scale_half_makes_a_1m_square_joined_by_4_quads() {
    for center in [CenterMode::Automatic, CenterMode::None, CenterMode::Origin] {
        let mut e = editor(center);
        preview(&mut e, |p| p.scale_delta = DVec3::splat(0.5));
        let m = mesh(&e);
        m.validate().unwrap();
        assert_eq!(m.faces.len(), 5, "4 new quads join the loops");
        assert_eq!(m.vertices.len(), 8);
        assert_square(&outer_points(&e), 0.5, 0.);
    }
}

#[test]
fn ext_rs_02_offset_down_scales_about_the_moved_centre() {
    let mut e = editor(CenterMode::Automatic);
    preview(&mut e, |p| {
        p.scale_delta = DVec3::splat(0.5);
        p.delta = DVec3::new(0., -1., 0.);
    });
    assert_square(&outer_points(&e), 0.5, -1.);
}

#[test]
fn ext_rs_03_rotate_y_45_turns_the_square() {
    let mut e = editor(CenterMode::Automatic);
    preview(&mut e, |p| p.rotation_delta = DVec3::new(0., 45., 0.));
    let points = outer_points(&e);
    let r = 2_f64.sqrt();
    for p in &points {
        // Corners of the turned square sit on the axes, √2 from the centre.
        let on_axis = (p.x.abs() < 1e-9 && (p.z.abs() - r).abs() < 1e-9)
            || (p.z.abs() < 1e-9 && (p.x.abs() - r).abs() < 1e-9);
        assert!(on_axis && p.y.abs() < 1e-9, "{points:?}");
    }
    mesh(&e).validate().unwrap();
}

#[test]
fn ext_rs_04_shift_click_extends_again_from_the_new_loop() {
    let mut e = editor(CenterMode::Automatic);
    preview(&mut e, |p| {
        p.scale_delta = DVec3::splat(0.5);
        p.delta = DVec3::new(0., -1., 0.);
    });
    let first = mesh(&e);
    e.dispatch(Command::Continue).unwrap();
    // The same values again, at once, relative to the first new loop.
    assert_square(&outer_points(&e), 0.25, -2.);
    assert_eq!(mesh(&e).faces.len(), 9);
    let live = e.session.live.as_ref().unwrap();
    assert_eq!(live.params.scale_delta, DVec3::splat(0.5));
    assert_eq!(*live.base.active().unwrap().mesh, first);
    // Drop keeps both; Undo takes them back one extension at a time.
    e.dispatch(Command::Drop).unwrap();
    assert!(e.session.live.is_none());
    assert_square(&outer_points(&e), 0.25, -2.);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e), first);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e), square());
}

#[test]
fn cancel_leaves_the_mesh_as_it_was() {
    let mut e = editor(CenterMode::Automatic);
    preview(&mut e, |p| p.rotation_delta = DVec3::new(10., 20., 30.));
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(mesh(&e), square());
    assert_eq!(e.scene.selection.edges, LOOP.to_vec());
}

#[test]
fn continue_needs_an_extension_first() {
    let mut e = editor(CenterMode::Automatic);
    assert!(e.dispatch(Command::Continue).is_err());
    assert_eq!(mesh(&e), square());
}

#[test]
fn local_inset_and_shift_follow_each_source_polygon() {
    let mut e = editor(CenterMode::Automatic);
    // Inset moves each new edge outward in the polygon's plane (mitred at
    // the corners); Shift along its normal (this square faces down, −Y,
    // as wound).
    preview(&mut e, |p| {
        p.inset = 0.5;
        p.shift = 0.25;
    });
    let n = mesh(&e).normal(0);
    let points = outer_points(&e);
    assert_eq!(points.len(), 4);
    for p in &points {
        assert!(
            (p.x.abs() - 1.5).abs() < 1e-9 && (p.z.abs() - 1.5).abs() < 1e-9,
            "{points:?}"
        );
        assert!((p.y - n.y * 0.25).abs() < 1e-9, "{points:?}");
    }
}

#[test]
fn segments_share_each_value_row_by_row() {
    let m = square();
    let shape = ExtendShape {
        scale: DVec3::splat(0.5),
        offset: DVec3::new(0., -2., 0.),
        segments: 2,
        ..ExtendShape::offset(DVec3::ZERO, 2)
    };
    let (out, outer) = edge_extend_shaped(&m, &LOOP, glam::DMat4::IDENTITY, &shape).unwrap();
    out.validate().unwrap();
    assert_eq!(out.faces.len(), 9);
    // The middle row is half way: 75 % size, 1 m down.
    let mid = out.vertices[4];
    assert!((mid.y + 1.).abs() < 1e-9 && (mid.x.abs() - 0.75).abs() < 1e-9);
    let far = out.vertices[outer[0].0 as usize];
    assert!((far.y + 2.).abs() < 1e-9 && (far.x.abs() - 0.5).abs() < 1e-9);
}

#[test]
fn a_moved_item_extends_in_world_space() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = square().into();
    e.scene.active_mut().unwrap().transform.translation = DVec3::new(5., 0., 0.);
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = LOOP.to_vec();
    e.dispatch(Command::Activate(Tool::EdgeExtend)).unwrap();
    preview(&mut e, |p| p.scale_delta = DVec3::splat(0.5));
    // Scaled about the selection's world centre (5, 0, 0): local ±0.5.
    assert_square(&outer_points(&e), 0.5, 0.);
}

#[test]
fn make_uvs_fills_the_texture_map_along_the_chosen_direction() {
    let m = square();
    for (uvs, along_u) in [(ExtendUvs::U, true), (ExtendUvs::V, false)] {
        let shape = ExtendShape {
            make_uvs: uvs,
            ..ExtendShape::offset(DVec3::new(0., 1., 0.), 1)
        };
        let (out, _) = edge_extend_shaped(&m, &LOOP, glam::DMat4::IDENTITY, &shape).unwrap();
        out.validate().unwrap();
        let map = &out.uv_maps[EXTEND_UV_MAP];
        assert_eq!(map.len(), 5);
        assert!(
            map[0].iter().all(Option::is_none),
            "old polygons keep theirs"
        );
        for corners in &map[1..5] {
            let c: Vec<DVec2> = corners.iter().map(|c| c.unwrap()).collect();
            // Corners 0, 1 on the source edge, 2, 3 on the new one.
            let out_dir = |p: DVec2| if along_u { p.x } else { p.y };
            assert_eq!(out_dir(c[0]), 0.);
            assert_eq!(out_dir(c[1]), 0.);
            assert_eq!(out_dir(c[2]), 1.);
            assert_eq!(out_dir(c[3]), 1.);
        }
        // Along the loop the values run 0..1 by length, the closing edge
        // ending at 1.
        let mut along: Vec<f64> = (1..5)
            .flat_map(|f| {
                map[f]
                    .iter()
                    .map(|c| if along_u { c.unwrap().y } else { c.unwrap().x })
            })
            .collect();
        along.sort_by(f64::total_cmp);
        assert_eq!(along.first(), Some(&0.));
        assert_eq!(along.last(), Some(&1.));
    }
}

#[test]
fn bad_values_are_refused_and_change_nothing() {
    let m = square();
    let shape = ExtendShape {
        rotate: DVec3::new(f64::NAN, 0., 0.),
        ..ExtendShape::offset(DVec3::ZERO, 1)
    };
    assert!(edge_extend_shaped(&m, &LOOP, glam::DMat4::IDENTITY, &shape).is_err());
    let mut e = editor(CenterMode::Automatic);
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.scale_delta.x = f64::INFINITY;
    assert!(e.dispatch(Command::Preview(p)).is_err());
    assert_eq!(mesh(&e), square());
    // Identity values leave the mesh alone.
    preview(&mut e, |p| p.scale_delta = DVec3::ONE);
    assert_eq!(mesh(&e), square());
}
