//! Shared inputs for the missing-tools group 1 tests (G4, G6 grids) and
//! shape checks that ignore point order (the spec compares positions).
#![allow(dead_code)]
use glam::DVec3;
use operandi_core::*;

pub fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

pub fn mesh(points: &[[f64; 3]], polygons: &[&[u32]]) -> Mesh {
    Mesh {
        vertices: points.iter().map(|p| DVec3::from_array(*p)).collect(),
        faces: polygons.iter().map(|f| face(f.to_vec())).collect(),
        ..Default::default()
    }
}

/// An n×n quad grid: points (i, j, 0), index (n+1)j + i; quad nj + i.
pub fn grid(n: u32) -> Mesh {
    grid_x(&(0..=n).map(f64::from).collect::<Vec<_>>(), n)
}

/// A grid with column positions `xs` and rows y = 0..rows.
pub fn grid_x(xs: &[f64], rows: u32) -> Mesh {
    let w = xs.len() as u32;
    let mut m = Mesh::default();
    for j in 0..=rows {
        for &x in xs {
            m.vertices.push(DVec3::new(x, f64::from(j), 0.));
        }
    }
    for j in 0..rows {
        for i in 0..w - 1 {
            let a = w * j + i;
            m.faces.push(face(vec![a, a + 1, a + 1 + w, a + w]));
        }
    }
    m
}

/// An editor holding `mesh` as its only, active item, in `mode`.
pub fn editor(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(mode)).unwrap();
    e
}

pub fn active(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

fn near(a: DVec3, b: DVec3) -> bool {
    (a - b).abs().max_element() <= 1e-9
}

/// The used points' positions.
pub fn used_positions(m: &Mesh) -> Vec<DVec3> {
    let used = m.used_points();
    m.vertices
        .iter()
        .zip(used)
        .filter(|(_, u)| *u)
        .map(|(p, _)| *p)
        .collect()
}

/// Every polygon as corner positions.
pub fn polygons(m: &Mesh) -> Vec<Vec<DVec3>> {
    m.faces
        .iter()
        .map(|f| f.vertices.iter().map(|&v| m.vertices[v as usize]).collect())
        .collect()
}

fn cyclic(a: &[DVec3], b: &[DVec3]) -> bool {
    a.len() == b.len()
        && (0..a.len()).any(|r| (0..a.len()).all(|i| near(a[i], b[(i + r) % b.len()])))
}

/// `want` (polygons as (x, y) corner lists, z = 0) are among `got`'s
/// polygons, matched cyclically in winding order.
pub fn has_polygons(got: &Mesh, want: &[&[[f64; 2]]]) -> std::result::Result<(), String> {
    let all = polygons(got);
    let mut used = vec![false; all.len()];
    for w in want {
        let w: Vec<DVec3> = w.iter().map(|p| DVec3::new(p[0], p[1], 0.)).collect();
        let Some(j) = (0..all.len()).find(|&j| !used[j] && cyclic(&all[j], &w)) else {
            return Err(format!("polygon {w:?} not found"));
        };
        used[j] = true;
    }
    Ok(())
}

/// The positions of the selected points.
pub fn selected_positions(e: &Editor) -> Vec<DVec3> {
    let m = active(e);
    let mut v: Vec<DVec3> = e
        .scene
        .selection
        .vertices
        .iter()
        .map(|&i| m.vertices[i as usize])
        .collect();
    v.sort_by(|a, b| a.to_array().partial_cmp(&b.to_array()).unwrap());
    v
}

/// Undo then redo returns the result; save and reopen keeps it.
pub fn undo_redo_reopen(e: &mut Editor, before: &Mesh, label: &str) {
    let after = active(e).clone();
    assert_eq!(e.undo.last().unwrap().label, label);
    let steps = e.undo.len();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(active(e), before, "undo restores the mesh");
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(active(e), &after, "redo repeats it");
    assert_eq!(e.undo.len(), steps);
    let path = std::env::temp_dir().join(format!(
        "{}-{}.opscene",
        label.replace(' ', "-"),
        std::process::id()
    ));
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    std::fs::remove_file(&path).ok();
    assert_eq!(&*back.active().unwrap().mesh, &after);
}
