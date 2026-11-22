//! VideoScape GEO (`3DG1`): a text header, a point count, one `x y z` line
//! per point, then one polygon per line as `count i0 i1 … color` with
//! 0-based indices. Polygon colors become materials named `GEO color N` so
//! they survive a round trip; other materials export as color 1.
//!
//! Provisional: coordinates and corner order are taken and written verbatim.
//! The format has no documented handedness we rely on.
use super::{RawFace, RawMesh, WorldMesh, num};
use crate::Result;
use glam::DVec3;
use std::fmt::Write;

const MATERIAL_PREFIX: &str = "GEO color ";

pub(super) fn read(bytes: &[u8], stem: &str, _notes: &mut Vec<String>) -> Result<Vec<RawMesh>> {
    let text = String::from_utf8_lossy(bytes);
    let mut tokens = text.split_whitespace();
    match tokens.next() {
        Some("3DG1") => {}
        Some(other) => {
            return Err(format!(
                "Unsupported VideoScape variant '{other}'; only 3DG1 is read"
            ));
        }
        None => return Err("Empty GEO file".into()),
    }
    let mut next_usize = |what: &str| -> Result<usize> {
        tokens
            .next()
            .and_then(|t| t.parse::<i64>().ok())
            .filter(|&n| n >= 0)
            .map(|n| n as usize)
            .ok_or_else(|| format!("GEO: expected {what}"))
    };
    let count = next_usize("point count")?;
    if count > 5_000_000 {
        return Err("GEO point count exceeds the mesh budget".into());
    }
    // Re-split so floats parse with their own errors.
    let mut rest = text.split_whitespace().skip(2);
    let mut points = Vec::with_capacity(count);
    for i in 0..count {
        let mut c = [0.0; 3];
        for v in &mut c {
            *v = rest
                .next()
                .and_then(|t| t.parse::<f64>().ok())
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("GEO: point {i} is not three numbers"))?;
        }
        points.push(DVec3::from_array(c));
    }
    let mut faces = Vec::new();
    while let Some(word) = rest.next() {
        let n: usize = word
            .parse()
            .map_err(|_| format!("GEO: '{word}' is not a polygon corner count"))?;
        if n > 4096 {
            return Err("GEO: polygon has too many corners".into());
        }
        let mut vertices = Vec::with_capacity(n);
        for _ in 0..n {
            let i: u32 = rest
                .next()
                .and_then(|t| t.parse().ok())
                .ok_or("GEO: polygon index is not a number")?;
            if i as usize >= points.len() {
                return Err(format!("GEO: polygon index {i} is out of range"));
            }
            vertices.push(i);
        }
        let color: i64 = rest
            .next()
            .and_then(|t| t.parse().ok())
            .ok_or("GEO: polygon is missing its color")?;
        faces.push(RawFace {
            vertices,
            material: format!("{MATERIAL_PREFIX}{color}"),
            ..Default::default()
        });
    }
    Ok(vec![RawMesh {
        name: stem.into(),
        points,
        faces,
        ..Default::default()
    }])
}

pub(super) fn write(meshes: &[WorldMesh], notes: &mut Vec<String>) -> Result<Vec<u8>> {
    // GEO has one point list, so all items share it.
    let total: usize = meshes.iter().map(|m| m.mesh.vertices.len()).sum();
    let mut out = format!("3DG1\n{total}\n");
    for m in meshes {
        for p in &m.mesh.vertices {
            let _ = writeln!(out, "{} {} {}", num(p.x), num(p.y), num(p.z));
        }
    }
    let mut base = 0;
    let mut lines = 0;
    for m in meshes {
        for (f, face) in m.mesh.faces.iter().enumerate() {
            if m.polyline[f] {
                lines += 1;
                continue;
            }
            let color = face
                .material
                .strip_prefix(MATERIAL_PREFIX)
                .and_then(|c| c.parse::<i64>().ok())
                .unwrap_or(1);
            let _ = write!(out, "{}", face.vertices.len());
            for v in &face.vertices {
                let _ = write!(out, " {}", base + *v as usize);
            }
            let _ = writeln!(out, " {color}");
        }
        base += m.mesh.vertices.len();
    }
    if lines > 0 {
        notes.push(format!("{lines} polyline(s) skipped in GEO"));
    }
    Ok(out.into_bytes())
}
