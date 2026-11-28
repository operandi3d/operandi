//! Edge Bevel / Edge Chamfer on a mesh with a vertex normal map (forum
//! report 2026-11-22: after bridging a box into a tunnel and hardening
//! edges, Edge Bevel did nothing on the tunnel's edges; the hard edges'
//! "Vertex Normal" map made it refuse every edge). Generated geometry only.
use glam::DVec3;
use operandi_core::vertex_normals::{self, NormalOptions};
use operandi_core::*;
use std::collections::BTreeSet;

fn key(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

type Edges = Vec<(u32, u32)>;

/// A 3x3x3 box bridged through its ±X centre polygons with Bridge's
/// defaults, then the tunnel's rims hardened (which makes the normal map).
/// Returns the mesh, the +X rim loop and the tunnel's ring of edges.
fn hardened_tunnel() -> (Mesh, Edges, Edges) {
    let m = primitive::segmented_cube(DVec3::ZERO, DVec3::splat(1.), [3, 3, 3]).unwrap();
    let centre_of = |f: &Face| {
        f.vertices
            .iter()
            .map(|&v| m.vertices[v as usize])
            .sum::<DVec3>()
            / f.vertices.len() as f64
    };
    let at = |x: f64| {
        m.faces
            .iter()
            .position(|f| centre_of(f).distance(DVec3::new(x, 0., 0.)) < 1e-9)
            .unwrap() as u32
    };
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = m.clone().into();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = vec![at(0.5), at(-0.5)];
    e.dispatch(Command::ArmBridge).unwrap();
    e.dispatch(Command::Apply).unwrap();
    let t: Mesh = (*e.scene.active().unwrap().mesh).clone();
    // Bridge welds the tunnel to the box: every edge has two polygons.
    assert!(t.edge_counts().values().all(|&c| c == 2), "open edges");
    let p = |v: u32| t.vertices[v as usize];
    let inside = |v: u32| p(v).y.abs() < 0.2 && p(v).z.abs() < 0.2;
    let (mut rim, mut ring) = (BTreeSet::new(), BTreeSet::new());
    for f in t.faces.iter() {
        let n = f.vertices.len();
        for i in 0..n {
            let (a, b) = (f.vertices[i], f.vertices[(i + 1) % n]);
            if !(inside(a) && inside(b)) {
                continue;
            }
            if p(a).x > 0.4 && p(b).x > 0.4 {
                rim.insert(key(a, b));
            }
            if (p(a).x - p(b).x).abs() > 0.5 {
                ring.insert(key(a, b));
            }
        }
    }
    assert_eq!((rim.len(), ring.len()), (4, 4));
    let hard: BTreeSet<(u32, u32)> = rim.iter().copied().collect();
    let t = vertex_normals::harden(&t, &hard, false, &NormalOptions::default()).unwrap();
    assert!(!t.attributes.corner_normals.is_empty(), "no normal map");
    (t, rim.into_iter().collect(), ring.into_iter().collect())
}

fn editor(mesh: &Mesh, edges: &[(u32, u32)], tool: Tool) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.clone().into();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.scene.selection.edges = edges.to_vec();
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.chamfer.offset = 0.05;
    e.dispatch(Command::Preview(p)).unwrap();
    e
}

fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

#[test]
fn edge_bevel_and_chamfer_work_on_a_mesh_with_a_normal_map() {
    let (t, rim, ring) = hardened_tunnel();
    for tool in [Tool::EdgeBevel, Tool::EdgeChamfer] {
        for (what, edges) in [("rim loop", &rim), ("ring", &ring)] {
            let e = editor(&t, edges, tool);
            let live = e.session.live.as_ref().unwrap();
            assert_eq!(live.preview_error, None, "{tool:?} {what}");
            let out = mesh(&e);
            assert!(out.faces.len() > t.faces.len(), "{tool:?} {what}: no bevel");
            out.validate().unwrap();
            // Every normal map has a row per polygon, a value per corner.
            for rows in out.attributes.corner_normals.values() {
                assert_eq!(rows.len(), out.faces.len());
                for (r, f) in rows.iter().zip(out.faces.iter()) {
                    assert_eq!(r.len(), f.vertices.len());
                }
            }
        }
    }
}

#[test]
fn untouched_polygons_keep_their_normals_and_new_ones_are_unassigned() {
    let (t, rim, _) = hardened_tunnel();
    let e = editor(&t, &rim, Tool::EdgeBevel);
    let out = mesh(&e);
    let (name, before) = t.attributes.corner_normals.iter().next().unwrap();
    let after = &out.attributes.corner_normals[name];
    let touched: BTreeSet<u32> = rim.iter().flat_map(|&(a, b)| [a, b]).collect();
    for (fi, f) in t.faces.iter().enumerate() {
        if f.vertices.iter().all(|v| !touched.contains(v)) {
            // Same polygon (points may be renumbered), same normals.
            let at = |m: &Mesh, f: &Face| -> Vec<DVec3> {
                f.vertices.iter().map(|&v| m.vertices[v as usize]).collect()
            };
            assert_eq!(at(out, &out.faces[fi]), at(&t, f), "polygon {fi}");
            assert_eq!(after[fi], before[fi], "polygon {fi}");
        } else {
            // A rebuilt polygon's corners all carry a normal from its
            // source corners (the hardened map has every corner set).
            assert!(after[fi].iter().all(Option::is_some), "polygon {fi}");
        }
    }
    for rows in &after[t.faces.len()..] {
        assert!(rows.iter().all(Option::is_none));
    }
}

#[test]
fn drop_undo_redo_and_save_reopen_agree() {
    let (t, _, ring) = hardened_tunnel();
    let mut e = editor(&t, &ring, Tool::EdgeBevel);
    let mut preview = mesh(&e).clone();
    e.dispatch(Command::Drop).unwrap();
    // Drop prunes edge-set entries whose edges are gone (the cut rims'
    // HardEdges); the preview still lists them. Everything else agrees.
    preview.attributes.edge_sets = mesh(&e).attributes.edge_sets.clone();
    assert_eq!(*mesh(&e), preview);
    let dropped = mesh(&e).clone();
    let reopened = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    assert_eq!(*reopened.active().unwrap().mesh, dropped);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(*mesh(&e), t);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(*mesh(&e), dropped);
}

#[test]
fn other_corner_data_is_still_refused() {
    let (mut t, rim, _) = hardened_tunnel();
    let rows: Vec<Vec<Option<f64>>> = t
        .faces
        .iter()
        .map(|f| vec![Some(0.5); f.vertices.len()])
        .collect();
    t.attributes.corner_weights.insert("w".into(), rows);
    let s = EdgeBevelSettings {
        width: 0.05,
        rows: 1,
        depth: 100.,
        chamfer: false,
        options: Default::default(),
    };
    let err = edge_bevel_with(&t, &rim, &s, None).unwrap_err();
    assert!(err.contains("cannot yet remap corner weights"), "{err}");
}
