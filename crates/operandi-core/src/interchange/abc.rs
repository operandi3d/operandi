//! Alembic (.abc) through the Alembic library (BSD-3-Clause), in the
//! `operandi-alembic` crate. Read: every polygon and subdivision mesh at its
//! first sample, transforms baked in, the `uv` parameter as Texture. Written:
//! one polygon mesh per visible item, world space, with Texture UVs. Ogawa
//! archives only; Alembic has no unit, so numbers are metres, Y up.
//! Animation, curves, points, cameras and materials are not read or written.
use super::{RawFace, RawMesh, WorldMesh};
use crate::Result;
use glam::{DVec2, DVec3};
use std::path::Path;

pub(super) fn read(path: &Path, stem: &str) -> Result<Vec<RawMesh>> {
    let meshes = operandi_alembic::read(path).map_err(|e| format!("Invalid Alembic file: {e}"))?;
    let mut out = Vec::new();
    for m in meshes {
        let mut raw = RawMesh {
            name: if m.name.is_empty() {
                stem.to_owned()
            } else {
                m.name
            },
            points: m.points.iter().map(|p| DVec3::from_array(*p)).collect(),
            ..Default::default()
        };
        let mut at = 0usize;
        for &count in &m.face_counts {
            let n = usize::try_from(count).map_err(|_| "Alembic: negative polygon size")?;
            let corners = m
                .indices
                .get(at..at + n)
                .ok_or("Alembic: polygon sizes run past the index list")?;
            let mut vertices = Vec::with_capacity(n);
            for &i in corners {
                vertices.push(u32::try_from(i).map_err(|_| "Alembic: negative point index")?);
            }
            let uv = m.uvs.as_ref().map(|u| {
                u[at..at + n]
                    .iter()
                    .map(|t| DVec2::new(t[0] as f64, t[1] as f64))
                    .collect()
            });
            raw.faces.push(RawFace {
                vertices,
                uv,
                ..Default::default()
            });
            at += n;
        }
        out.push(raw);
    }
    Ok(out)
}

pub(super) fn write(path: &Path, meshes: &[WorldMesh], notes: &mut Vec<String>) -> Result<()> {
    let mut names = std::collections::BTreeSet::new();
    let mut lines = 0;
    let mut out = Vec::new();
    for m in meshes {
        lines += m.polyline.iter().filter(|&&p| p).count();
        let faces: Vec<usize> = m.surface_faces().collect();
        if faces.is_empty() {
            continue;
        }
        // Alembic children need unique, non-empty names without slashes.
        let base = m.name.replace('/', "_");
        let base = if base.trim().is_empty() {
            "Mesh".to_owned()
        } else {
            base
        };
        let mut name = base.clone();
        let mut n = 2;
        while !names.insert(name.clone()) {
            name = format!("{base} {n}");
            n += 1;
        }
        let mut mesh = operandi_alembic::Mesh {
            name,
            points: m.mesh.vertices.iter().map(|p| p.to_array()).collect(),
            uvs: m.uvs.as_ref().map(|_| Vec::new()),
            ..Default::default()
        };
        for f in faces {
            let face = &m.mesh.faces[f];
            mesh.face_counts.push(face.vertices.len() as i32);
            mesh.indices.extend(face.vertices.iter().map(|&v| v as i32));
            if let Some(uvs) = &mut mesh.uvs {
                for c in 0..face.vertices.len() {
                    let t = m
                        .uvs
                        .as_ref()
                        .and_then(|u| u.get(f))
                        .and_then(|u| u.get(c).copied().flatten())
                        .unwrap_or(DVec2::ZERO);
                    uvs.push([t.x as f32, t.y as f32]);
                }
            }
        }
        out.push(mesh);
    }
    if lines > 0 {
        notes.push(format!(
            "{lines} polyline(s) skipped: Alembic export writes polygons only"
        ));
    }
    operandi_alembic::write(path, &out).map_err(|e| format!("Cannot write Alembic: {e}"))
}
