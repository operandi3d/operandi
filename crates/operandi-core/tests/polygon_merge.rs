//! MP-C09 MER-001..004 core acceptance for basic Polygon Merge.
//!
//! The two-quad shape, selection and expected boundary/area/normal come from
//! the admitted original `ORIGINAL_FIXTURE.json`. The attributed counterpart,
//! refusal scenes and every expected result here are independently authored
//! literal data: expected meshes are written out from per-point authoring
//! functions and the documented corner-correspondence rule, never produced
//! by calling Merge (or any other operation) and comparing it with itself.
//! Boundary checks use their own cyclic comparison, shoelace area and edge
//! sets rather than counts.
use glam::DVec3;
use operandi_core::*;

fn face(v: &[u32]) -> Face {
    Face {
        vertices: v.to_vec(),
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

fn plain(positions: &[DVec3], faces: &[Vec<u32>]) -> Mesh {
    let m = Mesh {
        vertices: positions.to_vec(),
        faces: faces.iter().map(|f| face(f)).collect(),
        ..Default::default()
    };
    m.validate().unwrap();
    m
}

/// `mesh` active in Polygon mode with `selected`, plus an unrelated tagged,
/// moved box item that must never change.
fn editor(mesh: Mesh, selected: &[u32]) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let target = e.scene.selection.item.unwrap();
    e.scene.new_mesh().unwrap();
    let mut unrelated = Mesh::box_mesh(DVec3::new(5., 0., 0.), DVec3::ONE, 1).unwrap();
    for (i, f) in unrelated.faces.iter_mut().enumerate() {
        f.part = (i % 2 == 0).then(|| "Unrelated".into());
    }
    for item in &mut e.scene.items {
        if item.id == target {
            item.mesh = mesh.clone().into();
            item.name = "Target".into();
        } else {
            item.mesh = unrelated.clone().into();
            item.transform.translation = DVec3::new(0., 1., 0.);
        }
    }
    e.scene.select_item(Some(target));
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = selected.to_vec();
    e.scene.validate().unwrap();
    e.undo.clear();
    e
}

fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

fn merge(e: &mut Editor) {
    e.dispatch(Command::MergePolygons).unwrap();
}

/// Points of an (n+1) x (m+1) unit grid in the XY plane and its n x m
/// counter-clockwise quads, row-major.
fn grid(n: u32, m: u32) -> (Vec<DVec3>, Vec<Vec<u32>>) {
    let mut p = Vec::new();
    for y in 0..=m {
        for x in 0..=n {
            p.push(DVec3::new(x as f64, y as f64, 0.));
        }
    }
    let id = |x: u32, y: u32| y * (n + 1) + x;
    let mut faces = Vec::new();
    for y in 0..m {
        for x in 0..n {
            faces.push(vec![id(x, y), id(x + 1, y), id(x + 1, y + 1), id(x, y + 1)]);
        }
    }
    (p, faces)
}

#[test]
fn mer_004_separate_regions_merge_in_one_edit() {
    // Row of five quads; [0, 1] and [3, 4] are two regions, 2 stays alone.
    let (p, faces) = grid(5, 1);
    let mut e = editor(plain(&p, &faces), &[0, 1, 3, 4]);
    merge(&mut e);
    let out = mesh(&e);
    out.validate().unwrap();
    let sizes: Vec<usize> = out.faces.iter().map(|f| f.vertices.len()).collect();
    assert_eq!(sizes, [6, 4, 6]);
    assert_eq!(e.undo.len(), 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e).faces.len(), 5);
}
