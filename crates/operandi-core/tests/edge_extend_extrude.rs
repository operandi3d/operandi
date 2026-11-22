//! ME-T03 Edge Extend and ME-T02 Edge Extrude geometry.
use glam::DVec3;
use operandi_core::*;

fn quad_strip() -> Mesh {
    let face = |vertices: Vec<u32>| Face {
        vertices,
        uv: None,
        material: "Strip".into(),
        material_label: None,
        smooth: false,
        part: Some("P".into()),
    };
    Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(2., 0., 0.),
            DVec3::new(0., 1., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(2., 1., 0.),
        ],
        faces: vec![face(vec![0, 1, 4, 3]), face(vec![1, 2, 5, 4])].into(),
        ..Default::default()
    }
}

#[test]
fn extend_one_edge_and_a_chain() {
    let m = quad_strip();
    // Top boundary edge 3-4 extended up by 1 in two segments.
    let (out, outer) = edge_extend_geometry(&m, &[(3, 4)], DVec3::Y, 2).unwrap();
    out.validate().unwrap();
    assert_eq!(out.faces.len(), 4);
    assert_eq!(out.vertices.len(), 10);
    assert!(
        out.attributes.extrude_edge_uses.is_empty(),
        "wound like the neighbour"
    );
    let (x, y) = outer[0];
    assert_eq!(out.vertices[x as usize].y, 2.);
    assert_eq!(out.vertices[y as usize].y, 2.);
    assert!(out.faces.iter().all(|f| f.part.as_deref() == Some("P")));
    // The whole top chain 3-4-5 extends as one connected row.
    let (out, outer) =
        edge_extend_geometry(&m, &[(3, 4), (4, 5)], DVec3::new(0., 0.5, 0.2), 1).unwrap();
    out.validate().unwrap();
    assert_eq!(
        out.vertices.len(),
        9,
        "the shared point 4 gets one new point"
    );
    assert_eq!(outer.len(), 2);
    // Interior edge 1-4 (it runs along Y) extends as a fin across it, but
    // not along itself, which would make a flat polygon.
    assert!(edge_extend_geometry(&m, &[(1, 4)], DVec3::Z, 1).is_ok());
    assert!(edge_extend_geometry(&m, &[(1, 4)], DVec3::Y, 1).is_err());
    // Zero offset is a no-op.
    assert_eq!(
        edge_extend_geometry(&m, &[(3, 4)], DVec3::ZERO, 1)
            .unwrap()
            .0,
        m
    );
}

#[test]
fn extrude_a_cube_edge_into_a_ridge() {
    let cube = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1).unwrap();
    // Find the top front edge (y = 1, z = 1).
    let (a, b) = cube
        .edges()
        .into_iter()
        .find(|&(a, b)| {
            let (p, q) = (cube.vertices[a as usize], cube.vertices[b as usize]);
            p.y > 0.5 && q.y > 0.5 && p.z > 0.5 && q.z > 0.5
        })
        .unwrap();
    let (out, raised) = edge_extrude_geometry(&cube, &[(a, b)], 0.3, 0.25).unwrap();
    out.validate().unwrap();
    assert!(out.attributes.extrude_edge_uses.is_empty());
    // 6 original faces + 2 ridge quads + 2 end caps.
    assert_eq!(out.faces.len(), 10);
    // The cube's corners at the edge's ends are cut away: each end polygon
    // runs straight between the two slid points, the caps are triangles and
    // the old corner points are gone (Jason, 2026-11-04).
    assert_eq!(out.vertices.len(), 8 - 2 + 4 + 2);
    for corner in [a, b] {
        let old = cube.vertices[corner as usize];
        assert!(out.vertices.iter().all(|&v| v.distance(old) > 1e-9));
    }
    let mut sizes: Vec<usize> = out.faces.iter().map(|f| f.vertices.len()).collect();
    sizes.sort();
    assert_eq!(sizes, [3, 3, 4, 4, 4, 4, 4, 4, 5, 5]);
    let (ra, rb) = raised[0];
    let lift = (out.vertices[ra as usize] - cube.vertices[a as usize])
        .length()
        .min((out.vertices[ra as usize] - cube.vertices[b as usize]).length());
    assert!(
        (lift - 0.3).abs() < 1e-9,
        "raised along the average normal by 0.3"
    );
    let mid = (out.vertices[ra as usize] + out.vertices[rb as usize]) / 2.;
    assert!(mid.y > 1. && mid.z > 1.);
    // Width must stay shorter than the side edges; Width 0 changes nothing
    // (extrude-2026-11-05, EE02).
    assert!(edge_extrude_geometry(&cube, &[(a, b)], 0.3, 3.).is_err());
    assert_eq!(
        edge_extrude_geometry(&cube, &[(a, b)], 0.3, 0.).unwrap().0,
        cube
    );
}

#[test]
fn slide_moves_an_edge_along_its_rails() {
    // Strip of two quads; the shared edge 1-4 slides toward either end.
    let m = quad_strip();
    let left = edge_slide_geometry(&m, &[(1, 4)], 0.5).unwrap();
    let right = edge_slide_geometry(&m, &[(1, 4)], -0.5).unwrap();
    left.validate().unwrap();
    assert_eq!(left.faces, m.faces, "topology unchanged");
    let (l, r) = (left.vertices[1].x, right.vertices[1].x);
    assert!((l - r).abs() > 0.9, "opposite sides: {l} vs {r}");
    assert!([0.5, 1.5].contains(&l) && [0.5, 1.5].contains(&r));
    assert_eq!(left.vertices[1].y, 0.);
    // Boundary edge: no rail on the outer side, so those points stay.
    let out = edge_slide_geometry(&m, &[(0, 1)], 0.5).unwrap();
    let moved = (0..6).filter(|&v| out.vertices[v] != m.vertices[v]).count();
    assert!(moved <= 2);
    // ES-05: no clamping; a non-finite amount is still refused.
    assert!(edge_slide_geometry(&m, &[(1, 4)], f64::NAN).is_err());
}

#[test]
fn crease_set_clear_and_save_reopen() {
    let cube = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1).unwrap();
    let creased = edge_crease_geometry(&cube, &[(1, 0), (2, 3)], 0.75, false).unwrap();
    assert_eq!(creased.vertices, cube.vertices);
    assert_eq!(creased.faces, cube.faces);
    assert_eq!(creased.attributes.creases.len(), 2);
    assert!(
        creased
            .attributes
            .creases
            .iter()
            .all(|c| c.weight == 0.75 && c.a < c.b)
    );
    let cleared = edge_crease_geometry(&creased, &[(0, 1)], 0., false).unwrap();
    assert_eq!(cleared.attributes.creases.len(), 1);
    assert!(
        edge_crease_geometry(&cube, &[(0, 6)], 0.5, false).is_err(),
        "not an edge"
    );
    // EW2-01: typed values are kept; only the surface clamps the effect.
    let clamped = edge_crease_geometry(&cube, &[(0, 1)], 1.5, false).unwrap();
    assert_eq!(clamped.attributes.creases[0].weight, 1.5);
    // Native save/reopen keeps edge identity (endpoints) and weight.
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = creased.clone().into();
    let path = std::env::temp_dir().join(format!("operandi-crease-{}.opscene", std::process::id()));
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(
        back.active().unwrap().mesh.attributes.creases,
        creased.attributes.creases
    );
}

#[test]
fn extend_activates_on_an_edge_parallel_to_y() {
    // Regression: activation used to probe only along +Y.
    let m = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(0., 1., 0.),
        ],
        faces: vec![Face {
            vertices: vec![0, 1, 2, 3],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        }]
        .into(),
        ..Default::default()
    };
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = m.into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![(1, 2)];
    e.dispatch(Command::Activate(Tool::EdgeExtend)).unwrap();
}

#[test]
fn extend_an_interior_cube_edge_as_a_fin() {
    // Jason, 2026-11-04: Extend also works on edges with more than one
    // neighbouring polygon. Provisional fin off the first neighbour (EXW-Q02).
    let cube = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1).unwrap();
    let (out, outer) =
        edge_extend_geometry(&cube, &[(0, 1)], DVec3::new(0., -0.5, -0.5), 1).unwrap();
    out.validate().unwrap();
    assert_eq!(out.faces.len(), 7);
    assert_eq!(out.vertices.len(), 10);
    assert_eq!(outer.len(), 1);
    // The source edge now has three polygons, recorded as a known exception.
    assert_eq!(out.attributes.extrude_edge_uses.len(), 1);
    // Re-extending the new outer edge (a border edge) keeps working.
    let (again, _) = edge_extend_geometry(&out, &outer, DVec3::new(0., -0.5, 0.), 1).unwrap();
    again.validate().unwrap();
    assert_eq!(again.faces.len(), 8);
}
