//! STL, binary and ASCII. Triangles only: polygons are triangulated on export
//! and imported triangles share points with bit-identical positions (the
//! `stl_io` reader welds them), so an imported closed STL is a closed mesh.
//! STL has no units or up axis; coordinates are taken and written as-is.
use super::{RawFace, RawMesh, WorldMesh, num};
use crate::Result;
use glam::DVec3;
use std::fmt::Write;

pub(super) fn read(bytes: &[u8], stem: &str, _notes: &mut Vec<String>) -> Result<Vec<RawMesh>> {
    let mut cursor = std::io::Cursor::new(bytes);
    let indexed = stl_io::read_stl(&mut cursor).map_err(|e| format!("Invalid STL: {e}"))?;
    Ok(vec![RawMesh {
        name: stem.into(),
        points: indexed
            .vertices
            .iter()
            .map(|v| DVec3::new(v[0] as f64, v[1] as f64, v[2] as f64))
            .collect(),
        faces: indexed
            .faces
            .iter()
            .map(|t| RawFace {
                vertices: t.vertices.iter().map(|&i| i as u32).collect(),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }])
}

/// Every surface triangle of every mesh, with its unit normal.
fn triangles(meshes: &[WorldMesh], notes: &mut Vec<String>) -> Vec<(DVec3, [DVec3; 3])> {
    let mut out = Vec::new();
    let mut lines = 0;
    for m in meshes {
        lines += m.polyline.iter().filter(|&&p| p).count();
        for f in m.surface_faces() {
            let face = &m.mesh.faces[f];
            let n = m.normal(f);
            for [a, b, c] in m.triangles(f) {
                let p = |i: usize| m.mesh.vertices[face.vertices[i] as usize];
                out.push((n, [p(a), p(b), p(c)]));
            }
        }
    }
    if lines > 0 {
        notes.push(format!(
            "{lines} polyline(s) skipped: STL holds only triangles"
        ));
    }
    out
}

pub(super) fn write_binary(meshes: &[WorldMesh], notes: &mut Vec<String>) -> Result<Vec<u8>> {
    let tris = triangles(meshes, notes);
    let v = |p: DVec3| stl_io::Vector::new([p.x as f32, p.y as f32, p.z as f32]);
    let mut out = Vec::new();
    stl_io::write_stl(
        &mut out,
        tris.iter().map(|(n, [a, b, c])| stl_io::Triangle {
            normal: v(*n),
            vertices: [v(*a), v(*b), v(*c)],
        }),
    )
    .map_err(|e| format!("Cannot encode STL: {e}"))?;
    Ok(out)
}

pub(super) fn write_ascii(meshes: &[WorldMesh], notes: &mut Vec<String>) -> Vec<u8> {
    let tris = triangles(meshes, notes);
    let v = |p: DVec3| format!("{} {} {}", num(p.x), num(p.y), num(p.z));
    let mut out = String::from("solid Operandi\n");
    for (n, [a, b, c]) in tris {
        let _ = write!(
            out,
            "  facet normal {}\n    outer loop\n      vertex {}\n      vertex {}\n      vertex {}\n    endloop\n  endfacet\n",
            v(n),
            v(a),
            v(b),
            v(c)
        );
    }
    out.push_str("endsolid Operandi\n");
    out.into_bytes()
}
