//! Shared helpers for the clean 2026-11-05 specs' `cases.json` files.
//!
//! The specs compare geometry, not indices: every expected point and polygon
//! must match one result point or polygon (polygons by corner positions in
//! the same cyclic order, any starting corner). Polygon order is free.
#![allow(dead_code)]

use glam::DVec3;
use operandi_core::{Face, Mesh};
use serde_json::Value;

/// Loads `specifications/<spec>/cases.json`.
pub fn cases(spec: &str) -> Vec<Value> {
    let path = format!(
        "{}/../../specifications/{spec}/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let root: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    root["cases"].as_array().unwrap().clone()
}

pub fn case<'a>(all: &'a [Value], id: &str) -> &'a Value {
    all.iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("case {id} missing"))
}

pub fn point(v: &Value) -> DVec3 {
    let a = v.as_array().unwrap();
    DVec3::new(
        a[0].as_f64().unwrap(),
        a[1].as_f64().unwrap(),
        a[2].as_f64().unwrap(),
    )
}

pub fn points(v: &Value) -> Vec<DVec3> {
    v.as_array().unwrap().iter().map(point).collect()
}

/// Index list; a missing list means none.
pub fn indices(v: &Value) -> Vec<u32> {
    v.as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|i| i.as_u64().unwrap() as u32)
        .collect()
}

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

/// Builds a mesh from `{points, polygons}`.
pub fn mesh(v: &Value) -> Mesh {
    Mesh {
        vertices: points(&v["points"]),
        faces: v["polygons"]
            .as_array()
            .map(|p| p.iter().map(|f| face(indices(f))).collect())
            .unwrap_or_default(),
        ..Default::default()
    }
}

fn near(a: DVec3, b: DVec3, tol: f64) -> bool {
    (a - b).abs().max_element() <= tol
}

fn cyclic_match(a: &[DVec3], b: &[DVec3], tol: f64) -> bool {
    a.len() == b.len()
        && (0..a.len()).any(|r| (0..a.len()).all(|i| near(a[i], b[(i + r) % b.len()], tol)))
}

/// Corner positions of each polygon.
pub fn polygon_positions(m: &Mesh, faces: &[u32]) -> Vec<Vec<DVec3>> {
    faces
        .iter()
        .map(|&f| {
            m.faces[f as usize]
                .vertices
                .iter()
                .map(|&v| m.vertices[v as usize])
                .collect()
        })
        .collect()
}

/// Matches two polygon lists as multisets of cyclic corner-position lists.
pub fn same_polygons(got: &[Vec<DVec3>], want: &[Vec<DVec3>], tol: f64) -> Result<(), String> {
    if got.len() != want.len() {
        return Err(format!("{} polygons, expected {}", got.len(), want.len()));
    }
    let mut used = vec![false; got.len()];
    for (i, w) in want.iter().enumerate() {
        let Some(j) = (0..got.len()).find(|&j| !used[j] && cyclic_match(&got[j], w, tol)) else {
            return Err(format!("expected polygon {i} {w:?} not found"));
        };
        used[j] = true;
    }
    Ok(())
}

/// Compares a result mesh with `expected` (`points`, `polygons`) by geometry.
/// Unused result points are ignored only when `expected` has the same count.
pub fn same_mesh(got: &Mesh, expected: &Value, tol: f64) -> Result<(), String> {
    let want = mesh(expected);
    if got.vertices.len() != want.vertices.len() {
        return Err(format!(
            "{} points, expected {}",
            got.vertices.len(),
            want.vertices.len()
        ));
    }
    let mut used = vec![false; got.vertices.len()];
    for (i, w) in want.vertices.iter().enumerate() {
        let Some(j) = (0..got.vertices.len()).find(|&j| !used[j] && near(got.vertices[j], *w, tol))
        else {
            return Err(format!("expected point {i} {w:?} not found"));
        };
        used[j] = true;
    }
    let all = |m: &Mesh| (0..m.faces.len() as u32).collect::<Vec<_>>();
    same_polygons(
        &polygon_positions(got, &all(got)),
        &polygon_positions(&want, &all(&want)),
        tol,
    )
}

/// Compares selected result polygons with `expected.selection_after`.
pub fn same_selected_polygons(
    got: &Mesh,
    selected: &[u32],
    expected: &Value,
    list: &Value,
    tol: f64,
) -> Result<(), String> {
    let want = mesh(expected);
    same_polygons(
        &polygon_positions(got, selected),
        &polygon_positions(&want, &indices(list)),
        tol,
    )
    .map_err(|e| format!("selection after: {e}"))
}

pub fn tolerance(c: &Value) -> f64 {
    c["tolerance"].as_f64().unwrap_or(1e-5)
}

/// For each expected polygon, the matching result polygon and the rotation
/// (result corner `(j + r) % n` is expected corner `j`).
pub fn polygon_matches(got: &Mesh, want: &Mesh, tol: f64) -> Result<Vec<(usize, usize)>, String> {
    let all = |m: &Mesh| (0..m.faces.len() as u32).collect::<Vec<_>>();
    let g = polygon_positions(got, &all(got));
    let w = polygon_positions(want, &all(want));
    if g.len() != w.len() {
        return Err(format!("{} polygons, expected {}", g.len(), w.len()));
    }
    let mut used = vec![false; g.len()];
    let mut out = Vec::new();
    for (i, wp) in w.iter().enumerate() {
        let found = (0..g.len()).find_map(|j| {
            if used[j] || g[j].len() != wp.len() {
                return None;
            }
            (0..wp.len())
                .find(|&r| (0..wp.len()).all(|k| near(g[j][(k + r) % wp.len()], wp[k], tol)))
                .map(|r| (j, r))
        });
        let Some((j, r)) = found else {
            return Err(format!("expected polygon {i} {wp:?} not found"));
        };
        used[j] = true;
        out.push((j, r));
    }
    Ok(out)
}
