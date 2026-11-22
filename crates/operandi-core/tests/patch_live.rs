//! Live patches (G7 PC-7, Jason 2026-11-21): a patch polygon on 3 or 4
//! corner points whose surface follows the curves between them.
use glam::DVec3;
use operandi_core::curves::CurveKind;
use operandi_core::evaluate::Purpose;
use operandi_core::patch_live::{broken_patches, has_patches};
use operandi_core::*;

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

/// Curves around a loop through `corners` (each with a raised middle
/// point), sharing their end points, and a patch polygon on the corners.
fn looped(corners: &[DVec3]) -> Mesh {
    let n = corners.len() as u32;
    let mut m = Mesh {
        vertices: corners.to_vec(),
        ..Default::default()
    };
    for k in 0..n {
        let (a, b) = (corners[k as usize], corners[((k + 1) % n) as usize]);
        m.vertices.push((a + b) / 2. + DVec3::Y * 0.3);
        m.faces.push(face(vec![k, n + k, (k + 1) % n]));
        m.attributes
            .representations
            .push(PolygonRepresentation::Polyline {
                closed: false,
                curve: CurveKind::Curve {
                    start_control: false,
                    end_control: false,
                },
            });
    }
    m.faces.push(face((0..n).collect()));
    m.attributes
        .representations
        .push(PolygonRepresentation::Patch);
    m.validate().unwrap();
    m
}

fn square() -> Mesh {
    looped(&[
        DVec3::new(-1., 0., -1.),
        DVec3::new(1., 0., -1.),
        DVec3::new(1., 0., 1.),
        DVec3::new(-1., 0., 1.),
    ])
}

fn editor_with(mesh: Mesh) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e
}

fn shown(e: &Editor) -> Mesh {
    e.evaluated(Purpose::Final).items[0].mesh.to_owned_value()
}

/// Polygons the evaluated scene shows as surface (not curves).
fn surface_polygons(m: &Mesh) -> usize {
    (0..m.faces.len()).filter(|&f| !m.is_polyline(f)).count()
}

#[test]
fn a_live_patch_draws_sixteen_by_sixteen_cells() {
    let e = editor_with(square());
    let m = shown(&e);
    assert_eq!(surface_polygons(&m), 256);
    assert!(!has_patches(&m));
    // The document keeps the one patch polygon.
    assert!(has_patches(&e.scene.items[0].mesh));
    // The same unchanged mesh gives the same shown mesh (caches keep it).
    let again = e.evaluated(Purpose::Final);
    let first = e.evaluated(Purpose::Final);
    assert!(MeshSlot::same(&again.items[0].mesh, &first.items[0].mesh));
}

#[test]
fn a_live_patch_round_trips_save_and_reopen() {
    let e = editor_with(square());
    let bytes = persistence::encode(&e.scene).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("\"version\": 57"), "native version 57");
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back, e.scene);
    assert!(has_patches(&back.items[0].mesh));
}

#[test]
fn moving_a_boundary_curve_point_reshapes_the_surface() {
    let mut e = editor_with(square());
    let before = shown(&e);
    // Lift the first curve's middle point.
    e.scene.active_mut().unwrap().mesh.vertices[4].y += 1.;
    let after = shown(&e);
    assert_eq!(before.faces.len(), after.faces.len(), "same cells");
    let moved = before
        .vertices
        .iter()
        .zip(&after.vertices)
        .filter(|(a, b)| (**a - **b).length() > 1e-6)
        .count();
    assert!(
        moved > 20,
        "the surface follows the curve ({moved} points moved)"
    );
}

#[test]
fn freeze_turns_a_patch_into_sixteen_by_sixteen_quads() {
    let mut e = editor_with(square());
    e.dispatch(Command::Freeze(Default::default())).unwrap();
    let m = e.scene.active().unwrap().mesh.to_owned_value();
    assert!(!has_patches(&m));
    let quads = (0..m.faces.len())
        .filter(|&f| !m.is_polyline(f) && m.faces[f].vertices.len() == 4)
        .count();
    assert_eq!(quads, 256);
}

#[test]
fn freeze_of_a_three_corner_patch_gives_triangles_at_the_corner() {
    let mut e = editor_with(looped(&[
        DVec3::new(-1., 0., -1.),
        DVec3::new(1., 0., -1.),
        DVec3::new(0., 0., 1.),
    ]));
    e.dispatch(Command::Freeze(Default::default())).unwrap();
    let m = e.scene.active().unwrap().mesh.to_owned_value();
    let surface: Vec<usize> = (0..m.faces.len()).filter(|&f| !m.is_polyline(f)).collect();
    let tris = surface
        .iter()
        .filter(|&&f| m.faces[f].vertices.len() == 3)
        .count();
    let quads = surface
        .iter()
        .filter(|&&f| m.faces[f].vertices.len() == 4)
        .count();
    assert_eq!((quads, tris), (240, 16));
    // The collapsed corner is one point, the patch's third corner, shared by
    // every triangle (the frozen curves through it are long polygons).
    let at_apex = surface
        .iter()
        .filter(|&&f| m.faces[f].vertices.len() == 3 && m.faces[f].vertices.contains(&2))
        .count();
    assert_eq!(at_apex, 16);
}

#[test]
fn deleting_a_boundary_curve_breaks_the_patch() {
    let mut m = square();
    m.faces.remove(1);
    m.attributes.representations.remove(1);
    m.validate().unwrap();
    assert_eq!(broken_patches(&m), vec![3]);
    let e = editor_with(m);
    let shown = shown(&e);
    assert_eq!(surface_polygons(&shown), 0, "a broken patch draws nothing");
    // It stays in the document, flagged, and survives a save.
    let bytes = persistence::encode(&e.scene).unwrap();
    assert_eq!(persistence::decode(&bytes).unwrap(), e.scene);
}

#[test]
fn patch_subdivisions_set_the_cells() {
    let mut e = editor_with(square());
    let item = e.scene.active_mut().unwrap();
    let mut s = item.subdivision.clone().unwrap_or_default();
    s.patch_subdivisions = 4;
    item.subdivision = Some(s);
    assert_eq!(surface_polygons(&shown(&e)), 16);
    let bytes = persistence::encode(&e.scene).unwrap();
    assert_eq!(persistence::decode(&bytes).unwrap(), e.scene);
}

#[test]
fn undo_and_redo_of_adding_a_live_patch_agree() {
    let mut m = square();
    // Start from the curves alone; add the patch as the tool does.
    let last = m.faces.len() - 1;
    m.faces.remove(last);
    m.attributes.representations.remove(last);
    let mut e = editor_with(m);
    let before = e.scene.clone();
    let mut patch = Mesh {
        vertices: e.scene.items[0].mesh.vertices[..4].to_vec(),
        ..Default::default()
    };
    patch.faces.push(face(vec![0, 1, 2, 3]));
    patch.attributes.representations = vec![PolygonRepresentation::Patch];
    let item = e.scene.items[0].id;
    e.dispatch(Command::AddGeometry(Box::new(AddGeometry {
        item: Some(item),
        mesh: patch,
        label: "Draw Patch".into(),
        weld: vec![(0, 0), (1, 1), (2, 2), (3, 3)],
        join: Vec::new(),
    })))
    .unwrap();
    let after = e.scene.clone();
    let mesh = &after.items[0].mesh;
    assert_eq!(mesh.vertices.len(), 8, "corners reused, no new points");
    assert!(has_patches(mesh));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
}
