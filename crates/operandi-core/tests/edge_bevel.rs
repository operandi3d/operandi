//! ME-T01 Edge Bevel / ME-T07 Edge Chamfer geometry.
use glam::DVec3;
use operandi_core::*;

fn cube() -> Mesh {
    Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap()
}
fn top_front(m: &Mesh) -> (u32, u32) {
    m.edges()
        .into_iter()
        .find(|&(a, b)| {
            let (p, q) = (m.vertices[a as usize], m.vertices[b as usize]);
            p.y > 0.4 && q.y > 0.4 && p.z > 0.4 && q.z > 0.4
        })
        .unwrap()
}

#[test]
fn bevel_a_box_edge_into_a_flat_strip() {
    let c = cube();
    let (out, rails) = edge_bevel_geometry(&c, &[top_front(&c)], 0.1, 1, 100., false).unwrap();
    out.validate().unwrap();
    // 8 corners - 2 + 4 new points; 6 sides + 1 strip.
    assert_eq!(out.vertices.len(), 10);
    assert_eq!(out.faces.len(), 7);
    // EBC-J-01: the strip's four edges.
    assert_eq!(rails.len(), 4);
    // Neighbours stay in their planes and shrink by the inset.
    for v in &out.vertices {
        let on_top = (v.y - 0.5).abs() < 1e-12;
        let on_front = (v.z - 0.5).abs() < 1e-12;
        if on_top && v.z > 0.3 {
            assert!((v.z - 0.4).abs() < 1e-12, "{v}");
        }
        if on_front && v.y > 0.3 {
            assert!((v.y - 0.4).abs() < 1e-12, "{v}");
        }
    }
    // The strip faces outward (between top and front).
    let strip = out.faces.len() - 1;
    let n = out.normal(strip);
    assert!(n.y > 0.5 && n.z > 0.5, "{n}");
    // Every polygon faces away from the center.
    for f in 0..out.faces.len() {
        assert!(out.normal(f).dot(out.center(f)) > 0.);
    }
}

#[test]
fn round_level_rows_arc_toward_the_source_edge() {
    let c = cube();
    let (out, _) = edge_bevel_geometry(&c, &[top_front(&c)], 0.2, 3, 100., false).unwrap();
    out.validate().unwrap();
    assert_eq!(out.faces.len(), 6 + 3);
    // Middle rows bulge beyond the flat chord (y + z = 0.8) toward the old
    // corner; Depth 0 keeps every point on or inside it.
    assert!(out.vertices.iter().any(|v| v.y + v.z > 0.8 + 1e-6));
    let (flat, _) = edge_bevel_geometry(&c, &[top_front(&c)], 0.2, 3, 0., false).unwrap();
    assert!(flat.vertices.iter().all(|v| v.y + v.z <= 0.8 + 1e-9));
}

#[test]
fn chamfer_offsets_in_parallel_where_bevel_slides_along_sides() {
    // A wedge: the edge meets a slanted side, so the two modes differ.
    let m = Mesh {
        vertices: [
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [1., 0., 1.],
            [1., 1., 1.],
            [0., 1., 1.],
        ]
        .iter()
        .map(|&p| DVec3::from_array(p))
        .collect(),
        faces: [
            vec![0, 3, 2, 1],
            vec![4, 5, 6, 7],
            vec![0, 4, 7, 3],
            vec![1, 2, 6, 5],
            vec![0, 1, 5, 4],
            vec![3, 7, 6, 2],
        ]
        .into_iter()
        .map(|vertices| Face {
            vertices,
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        })
        .collect(),
        ..Default::default()
    };
    let mut wedge = m.clone();
    wedge.vertices[6].x = 2.; // top-front-right corner leans out
    wedge.validate().unwrap();
    let edge = (2, 6);
    let (bevel, _) = edge_bevel_geometry(&wedge, &[edge], 0.1, 1, 100., false).unwrap();
    let (chamfer, _) = edge_bevel_geometry(&wedge, &[edge], 0.1, 1, 100., true).unwrap();
    bevel.validate().unwrap();
    chamfer.validate().unwrap();
    assert_ne!(bevel.vertices, chamfer.vertices);
    assert!(edge_bevel_geometry(&wedge, &[edge], 2., 1, 100., true).is_err());
}

#[test]
fn bevel_through_the_editor_is_one_step_and_refuses_shared_points() {
    let c = cube();
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = c.clone().into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![top_front(&c)];
    for tool in [Tool::EdgeBevel, Tool::EdgeChamfer] {
        let before = e.scene.clone();
        e.dispatch(Command::Activate(tool)).unwrap();
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.chamfer.offset = 0.1;
        p.chamfer.segments = 2;
        e.dispatch(Command::Preview(p)).unwrap();
        e.dispatch(Command::Drop).unwrap();
        assert_eq!(e.history.undo.last().unwrap().label, tool.label());
        assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 8);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, before);
    }
    let corner: Vec<(u32, u32)> = c
        .edges()
        .into_iter()
        .filter(|&(a, b)| a == 6 || b == 6)
        .collect();
    // EBC-J-03: three edges at a corner end on a triangle.
    let (out, _) = edge_bevel_geometry(&c, &corner, 0.1, 1, 100., false).unwrap();
    assert_eq!((out.vertices.len(), out.faces.len()), (13, 10));
}

#[test]
fn relax_rounds_a_flat_bevel_and_widens_with_offset() {
    let c = cube();
    // Flat 3-row bevel (Depth 0): rows lie on the chord.
    let (flat, rails) = edge_bevel_geometry(&c, &[top_front(&c)], 0.2, 3, 0., false).unwrap();
    // Select every row edge of the strip, outer rails included.
    let rows: Vec<(u32, u32)> = flat
        .edges()
        .into_iter()
        .filter(|&(a, b)| {
            let (p, q) = (flat.vertices[a as usize], flat.vertices[b as usize]);
            (p.x - q.x).abs() > 0.9 && p.y > 0.29 && p.z > 0.29
        })
        .collect();
    assert_eq!(rows.len(), 4);
    // The strip's long edges (both sides and the rows) are all selected.
    let long: Vec<_> = rails
        .iter()
        .filter(|&&(a, b)| (flat.vertices[a as usize].x - flat.vertices[b as usize].x).abs() > 0.9)
        .collect();
    assert_eq!(long.len(), 4);
    assert!(long.iter().all(|r| rows.contains(r)));
    let relaxed = edge_relax_geometry(&flat, &rows, 0.).unwrap();
    relaxed.validate().unwrap();
    assert_eq!(relaxed.faces, flat.faces);
    // Inner rows moved off the chord toward the old corner.
    let moved =
        (0..flat.vertices.len()).filter(|&v| relaxed.vertices[v].distance(flat.vertices[v]) > 1e-6);
    assert_eq!(moved.count(), 4);
    for v in &relaxed.vertices {
        if v.y > 0.3 && v.z > 0.3 && v.y < 0.5 - 1e-9 && v.z < 0.5 - 1e-9 {
            assert!(v.y + v.z > 0.8 + 1e-6, "{v}");
        }
    }
    // Offset widens: the outer rails slide 0.05 further along the sides.
    let wider = edge_relax_geometry(&flat, &rows, 0.05).unwrap();
    let low = wider
        .vertices
        .iter()
        .filter(|v| (v.z - 0.5).abs() < 1e-12)
        .map(|v| v.y)
        .fold(f64::INFINITY, f64::min);
    assert!(
        wider
            .vertices
            .iter()
            .any(|v| (v.z - 0.5).abs() < 1e-12 && (v.y - 0.25).abs() < 1e-9),
        "{low}"
    );
    assert!(edge_relax_geometry(&flat, &rows[..1], 0.).is_err());
}
