//! Bridge in Edge mode (ME-T04, bridge-2026-11-05 BR16/BR17): two edges,
//! edge runs or loops, with Segments, Mode, Tension, Twist, Reverse
//! Direction and Flip Polygons, under any work plane, action center or
//! falloff.
use glam::DVec3;
use operandi_core::*;
use std::collections::BTreeSet;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}
fn mesh(points: &[[f64; 3]], faces: &[&[u32]]) -> Mesh {
    Mesh {
        vertices: points.iter().map(|&p| DVec3::from_array(p)).collect(),
        faces: faces.iter().map(|f| face(f.to_vec())).collect(),
        ..Default::default()
    }
}

/// Two quads facing +Z with a one-unit gap; their facing borders are
/// 1–2 (x = 1) and 4–7 (x = 2).
fn gap() -> Mesh {
    mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [2., 0., 0.],
            [3., 0., 0.],
            [3., 1., 0.],
            [2., 1., 0.],
        ],
        &[&[0, 1, 2, 3], &[4, 5, 6, 7]],
    )
}

/// Two rows of two quads facing +Z with a gap: border runs 2–5–8 (x = 2)
/// and 9–12–15 (x = 3), two edges each.
fn gap_runs() -> Mesh {
    let mut points = Vec::new();
    for y in 0..3 {
        for x in [0., 1., 2.] {
            points.push([x, y as f64, 0.]);
        }
    }
    for y in 0..3 {
        for x in [3., 4., 5.] {
            points.push([x, y as f64, 0.]);
        }
    }
    let quad = |o: u32, x: u32, y: u32| {
        vec![
            o + y * 3 + x,
            o + y * 3 + x + 1,
            o + y * 3 + x + 4,
            o + y * 3 + x + 3,
        ]
    };
    let faces: Vec<Vec<u32>> = [0, 9]
        .into_iter()
        .flat_map(|o| (0..2).flat_map(move |y| (0..2).map(move |x| quad(o, x, y))))
        .collect();
    let refs: Vec<&[u32]> = faces.iter().map(|f| &f[..]).collect();
    mesh(&points, &refs)
}

/// Every directed edge is used once at most: the bridge's polygons wind
/// the same way as their neighbours.
fn consistent(m: &Mesh) -> bool {
    let mut seen = BTreeSet::new();
    m.faces.iter().all(|f| {
        let n = f.vertices.len();
        (0..n).all(|i| seen.insert((f.vertices[i], f.vertices[(i + 1) % n])))
    })
}

fn params(f: impl FnOnce(&mut BridgeParameters)) -> BridgeParameters {
    let mut p = BridgeParameters::default();
    f(&mut p);
    p
}

fn editor(m: Mesh, edges: Vec<(u32, u32)>) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = m.into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = edges;
    e
}

#[test]
fn two_border_edges_bridge_under_any_work_plane_action_center_and_falloff() {
    let mut e = editor(gap(), vec![(1, 2), (4, 7)]);
    // Bottom-view work plane, Selection action center and a linear
    // falloff: none of them stops Bridge any more.
    e.scene.workplane = scene::Workplane {
        fixed: true,
        plane: 1,
        center: DVec3::new(0., -2., 0.),
    };
    e.action_context.center = CenterMode::Selection;
    e.spatial_falloff = Some(Falloff::Linear(LinearFalloff {
        start: DVec3::ZERO,
        end: DVec3::X,
        ..Default::default()
    }));
    e.dispatch(Command::ArmBridge).unwrap();
    e.bridge_eligibility().unwrap();
    let before = e.scene.clone();
    e.dispatch(Command::Apply).unwrap();
    let out = e.scene.active().unwrap().mesh.clone();
    out.validate().unwrap();
    assert_eq!(out.faces.len(), 3);
    assert!(consistent(&out));
    assert!(out.normal(2).z > 0.9, "the bridge faces up like its sides");
    // The source edges stay selected; mode stays Edge (BR16/BR17).
    assert_eq!(e.scene.selection.mode, Mode::Edge);
    assert_eq!(e.scene.selection.edges, vec![(1, 2), (4, 7)]);
    assert_eq!(e.undo.last().unwrap().label, "Bridge Apply");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(
        e.scene.active().unwrap().mesh,
        before.active().unwrap().mesh
    );
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.clone(), out);
}

#[test]
fn edge_runs_bridge_with_segments_and_close_the_surface() {
    let m = gap_runs();
    let edges = [(2, 5), (5, 8), (9, 12), (12, 15)];
    let (out, sel) = edge_bridge(&m, &edges, params(|p| p.segments = 3)).unwrap();
    out.validate().unwrap();
    // 2 spans × 3 rows; 2 rings of 3 new points.
    assert_eq!(out.faces.len(), 8 + 6);
    assert_eq!(out.vertices.len(), 18 + 6);
    assert!(consistent(&out));
    assert!((8..14).all(|f| out.normal(f).z > 0.9));
    // Coplanar facing runs: Curve spaces rings evenly, like Linear.
    for (k, x) in [(18, 2. + 1. / 3.), (21, 2. + 2. / 3.)] {
        assert!((out.vertices[k].x - x).abs() < 1e-9, "{}", out.vertices[k]);
    }
    assert_eq!(sel.len(), 4);
}

#[test]
fn mode_and_tension_shape_the_rows_between_angled_edges() {
    // A flat quad facing up, and a wall quad facing back towards it (-X),
    // standing one unit past it and one up: the bridge leaves A along +X
    // and arrives into B going up +Z.
    let m = mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [2., 0., 1.],
            [2., 1., 1.],
            [2., 1., 2.],
            [2., 0., 2.],
        ],
        &[&[0, 1, 2, 3], &[4, 7, 6, 5]],
    );
    m.validate().unwrap();
    let edges = [(1, 2), (4, 5)];
    assert!(consistent(
        &edge_bridge(&m, &edges, params(|_| ())).unwrap().0
    ));
    let ring = |p: BridgeParameters| edge_bridge(&m, &edges, p).unwrap().0.vertices[8];
    let linear = ring(params(|p| {
        p.segments = 3;
        p.mode = BridgeMode::Linear;
    }));
    assert!((linear - DVec3::new(1. + 1. / 3., 0., 1. / 3.)).length() < 1e-9);
    let curve = ring(params(|p| p.segments = 3));
    let smooth = ring(params(|p| {
        p.segments = 3;
        p.mode = BridgeMode::Smooth;
    }));
    let slack = ring(params(|p| {
        p.segments = 3;
        p.tension = 0.;
    }));
    // Curve leaves A flat (along +X), so its first ring sits lower than
    // the straight line; Smooth matches it here (one edge each side).
    assert!(curve.z < linear.z - 1e-3, "{curve} vs {linear}");
    assert!((smooth - curve).length() < 1e-9);
    // Tension 0 drops the tangents: an eased straight line.
    assert!((slack.z - slack.x + 1.).abs() < 1e-9);
    assert!((slack - curve).length() > 1e-3 && (slack - linear).length() > 1e-3);
    for p in [linear, curve, slack] {
        assert!(p.y.abs() < 1e-9);
    }
}

#[test]
fn flip_and_reverse_do_not_apply_to_edge_bridges() {
    // An edge bridge always faces like the polygons it joins; flipping or
    // reversing it would turn it inside out against them (BRI-Q08).
    let m = gap();
    let edges = [(1, 2), (4, 7)];
    let plain = edge_bridge(&m, &edges, params(|_| ())).unwrap();
    for p in [
        params(|p| p.flip = true),
        params(|p| p.reverse = true),
        params(|p| p.remove = false),
    ] {
        assert_eq!(edge_bridge(&m, &edges, p).unwrap(), plain);
    }
    // Sides facing opposite ways would need a crossed bridge: refused.
    let mut turned = m.clone();
    turned.faces[1].vertices.reverse();
    let e = edge_bridge(&turned, &edges, params(|_| ())).unwrap_err();
    assert!(e.to_string().contains("cross itself"), "{e}");
}

#[test]
fn loops_bridge_and_twist_turns_the_pairing() {
    // Open-top box and a lid one unit above it, facing up.
    let mut c = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap();
    let top = (0..6).find(|&f| c.normal(f).y > 0.9).unwrap();
    let rim = c.faces.remove(top).vertices;
    let base = c.vertices.len() as u32;
    for &v in &rim {
        c.vertices.push(c.vertices[v as usize] + DVec3::Y);
    }
    c.faces.push(face((base..base + 4).collect()));
    let mut loops: Vec<(u32, u32)> = (0..4).map(|i| (rim[i], rim[(i + 1) % 4])).collect();
    loops.extend((0..4).map(|i| (base + i, base + (i + 1) % 4)));
    let (out, _) = edge_bridge(&c, &loops, params(|p| p.segments = 2)).unwrap();
    out.validate().unwrap();
    assert!(consistent(&out));
    assert_eq!(out.faces.len(), 6 + 8);
    // Each wall rises straight: the middle ring sits right above the rim.
    for &v in &rim {
        let p = c.vertices[v as usize];
        assert!(
            out.vertices[c.vertices.len()..]
                .iter()
                .any(|q| (*q - p - DVec3::Y * 0.5).length() < 1e-9)
        );
    }
    // Rim points joined, in a new polygon, to the lid point straight above.
    let above = |v: u32| base + rim.iter().position(|&r| r == v).unwrap() as u32;
    let straight = |m: &Mesh| -> usize {
        m.faces[6..]
            .iter()
            .map(|f| {
                f.vertices
                    .iter()
                    .filter(|&&v| rim.contains(&v) && f.vertices.contains(&above(v)))
                    .count()
            })
            .sum()
    };
    let (plain, _) = edge_bridge(&c, &loops, params(|_| ())).unwrap();
    assert_eq!(straight(&plain), 8);
    // Twist 1 turns the lid's pairing one corner on.
    let (twisted, _) = edge_bridge(&c, &loops, params(|p| p.twist = 1)).unwrap();
    twisted.validate().unwrap();
    assert!(consistent(&twisted));
    assert_eq!(straight(&twisted), 4);
}

#[test]
fn refusals_leave_the_scene_as_it_was() {
    let m = gap_runs();
    let p = BridgeParameters::default();
    // One run, unequal runs, a shared point, a branch.
    assert!(edge_bridge(&m, &[(2, 5), (5, 8)], p).is_err());
    assert!(edge_bridge(&m, &[(2, 5), (5, 8), (9, 12)], p).is_err());
    assert!(edge_bridge(&m, &[(2, 5), (5, 8), (4, 5), (9, 12)], p).is_err());
    let mut e = editor(m, vec![(2, 5), (9, 12), (12, 15)]);
    e.dispatch(Command::ArmBridge).unwrap();
    let before = e.scene.clone();
    assert!(e.bridge_eligibility().is_ok());
    assert!(e.dispatch(Command::Apply).is_err());
    assert_eq!(e.scene, before);
    // A single edge is greyed with a reason.
    e.scene.selection.edges = vec![(2, 5)];
    assert!(e.bridge_eligibility().is_err());
}

#[test]
fn edges_inside_the_surface_are_refused_with_a_reason() {
    // A row of four quads; the edges at x = 1 and x = 3 already have
    // polygons on both sides.
    let points: Vec<[f64; 3]> = (0..2)
        .flat_map(|y| (0..5).map(move |x| [x as f64, y as f64, 0.]))
        .collect();
    let faces: Vec<Vec<u32>> = (0..4).map(|x| vec![x, x + 1, x + 6, x + 5]).collect();
    let refs: Vec<&[u32]> = faces.iter().map(|f| &f[..]).collect();
    let m = mesh(&points, &refs);
    let e = edge_bridge(&m, &[(1, 6), (3, 8)], BridgeParameters::default()).unwrap_err();
    assert!(e.to_string().contains("border edges"), "{e}");
}
