use glam::DVec3;
use operandi_core::*;

/// A unit cube whose top was Polygon-Beveled (shift 0.2, inset 0.2): the four
/// slanted ring quads form a chamfer chain around the corners.
fn beveled_cube() -> (Mesh, u32, Vec<u32>, Vec<u32>) {
    let cube = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap();
    let top = (0..cube.faces.len())
        .find(|&f| cube.normal(f).y > 0.9)
        .unwrap() as u32;
    let r = bevel_geometry(
        &cube,
        &[top],
        0.2,
        0.2,
        false,
        &BevelOptions::default(),
        None,
    )
    .unwrap();
    let mesh = r.mesh;
    let cap = r.selected[0];
    let walls: Vec<u32> = (0..mesh.faces.len() as u32)
        .filter(|&f| mesh.normal(f as usize).y.abs() < 1e-9)
        .collect();
    let ring: Vec<u32> = (0..mesh.faces.len() as u32)
        .filter(|&f| {
            let n = mesh.normal(f as usize);
            n.y > 1e-3 && n.y < 0.99
        })
        .collect();
    assert_eq!((walls.len(), ring.len()), (4, 4));
    (mesh, cap, walls, ring)
}

#[test]
fn chamfer_edit_wraps_a_bevel_ring_around_its_corners() {
    let (mesh, cap, walls, ring) = beveled_cube();
    let mut outer = walls.clone();
    outer.push(cap);
    // The four ring quads are found; the bottom (all edges on walls) is not.
    assert_eq!(chamfer_strips(&mesh, &outer), ring);
    let p = ChamferParameters {
        offset: 0.05,
        segments: 3,
        source_segments: 1,
        depth: 100.,
        ..Default::default()
    };
    let out = chamfer_geometry(&mesh, &outer, p).unwrap();
    out.validate().unwrap();
    assert_eq!(out.faces.len(), mesh.faces.len() + 4 * 2);
    // Each strip gets 2 shared profile points per corner seam.
    assert_eq!(out.vertices.len(), mesh.vertices.len() + 4 * 2);
    // Cap: still flat at y = 0.7 and inset by 0.05 on every side.
    let cap_points: Vec<DVec3> = out.faces[cap as usize]
        .vertices
        .iter()
        .map(|&v| out.vertices[v as usize])
        .collect();
    for q in &cap_points {
        assert!((q.y - 0.7).abs() < 1e-9, "{q}");
        assert!(
            (q.x.abs() - 0.25).abs() < 1e-9 && (q.z.abs() - 0.25).abs() < 1e-9,
            "{q}"
        );
    }
    // Walls: their top edge moved down by 0.05; walls stay vertical.
    for &w in &walls {
        let top = out.faces[w as usize]
            .vertices
            .iter()
            .map(|&v| out.vertices[v as usize].y)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((top - 0.45).abs() < 1e-9, "{top}");
        assert!(out.normal(w as usize).y.abs() < 1e-9);
    }
    // Selecting the ring itself gives the same edit.
    assert_eq!(chamfer_geometry(&mesh, &ring, p).unwrap(), out);
    // Offset alone (one segment) keeps the topology.
    let widened = chamfer_geometry(&mesh, &outer, ChamferParameters { segments: 1, ..p }).unwrap();
    assert_eq!(widened.faces.len(), mesh.faces.len());
    // Too wide: the walls' side edges are only 0.5 tall above the bottom.
    assert!(chamfer_geometry(&mesh, &outer, ChamferParameters { offset: 0.6, ..p }).is_err());
}

#[test]
fn chamfer_edit_ring_through_the_editor_is_one_undo_step() {
    let (mesh, cap, walls, _) = beveled_cube();
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.clone().into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = walls.iter().copied().chain([cap]).collect();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Activate(Tool::Chamfer)).unwrap();
    e.dispatch(Command::ConfigureChamfer(ChamferParameters {
        offset: 0.05,
        segments: 2,
        source_segments: 1,
        depth: 100.,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(
        e.scene.active().unwrap().mesh.faces.len(),
        mesh.faces.len() + 4
    );
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh, mesh);
}
