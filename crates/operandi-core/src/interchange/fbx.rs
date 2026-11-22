//! FBX import through ufbx (MIT or Unlicense), which reads binary and ASCII
//! FBX of every common version. ufbx converts the file to Y-up metres, so
//! the result follows the same Up Axis turn as glTF. One item per mesh
//! node, world transform baked into the points (like the other flat
//! readers), polygons kept as n-gons, the first UV set as Texture, FBX
//! materials as polygon tags with their diffuse colour, and smoothing
//! groups as smooth flags. Skinning, blend shapes, animation, cameras and
//! lights are not imported.
use super::{RawFace, RawMesh};
use crate::Result;
use glam::{DVec2, DVec3};

pub(super) fn read(bytes: &[u8], stem: &str, notes: &mut Vec<String>) -> Result<Vec<RawMesh>> {
    let opts = ufbx::LoadOpts {
        target_axes: ufbx::CoordinateAxes {
            right: ufbx::CoordinateAxis::PositiveX,
            up: ufbx::CoordinateAxis::PositiveY,
            front: ufbx::CoordinateAxis::PositiveZ,
        },
        target_unit_meters: 1.0,
        space_conversion: ufbx::SpaceConversion::ModifyGeometry,
        ..Default::default()
    };
    let scene =
        ufbx::load_memory(bytes, opts).map_err(|e| format!("Invalid FBX: {}", e.description))?;
    let mut meshes = Vec::new();
    let mut skipped = Vec::new();
    for node in scene.nodes.iter() {
        if node.camera.is_some() {
            skipped.push("camera");
        }
        if node.light.is_some() {
            skipped.push("light");
        }
        let Some(mesh) = node.mesh.as_ref() else {
            continue;
        };
        if !mesh.skin_deformers.is_empty() || !mesh.blend_deformers.is_empty() {
            skipped.push("deformer");
        }
        let matrix = &node.geometry_to_world;
        let mirrored = ufbx::matrix_determinant(matrix) < 0.0;
        let points: Vec<DVec3> = mesh
            .vertices
            .iter()
            .map(|&v| {
                let p = ufbx::transform_position(matrix, v);
                DVec3::new(p.x, p.y, p.z)
            })
            .collect();
        let uv = &mesh.vertex_uv;
        let mut raw = RawMesh {
            name: if node.element.name.is_empty() {
                stem.to_owned()
            } else {
                node.element.name.to_string()
            },
            points,
            ..Default::default()
        };
        for (f, face) in mesh.faces.iter().enumerate() {
            let corners = face.index_begin as usize..(face.index_begin + face.num_indices) as usize;
            let mut vertices: Vec<u32> = corners.clone().map(|i| mesh.vertex_indices[i]).collect();
            let mut uvs: Option<Vec<DVec2>> = uv.exists.then(|| {
                corners
                    .clone()
                    .map(|i| {
                        let t = uv.values[uv.indices[i] as usize];
                        DVec2::new(t.x, t.y)
                    })
                    .collect()
            });
            if mirrored {
                vertices[1..].reverse();
                if let Some(u) = &mut uvs {
                    u[1..].reverse();
                }
            }
            let material = mesh
                .face_material
                .get(f)
                .and_then(|&m| mesh.materials.get(m as usize))
                .map(|m| m.element.name.to_string())
                .unwrap_or_default();
            if !material.is_empty() && !raw.materials.contains_key(&material) {
                let m = &mesh.materials[mesh.face_material[f] as usize];
                if let Some(settings) = read_material(m) {
                    raw.materials.insert(material.clone(), settings);
                }
            }
            raw.faces.push(RawFace {
                vertices,
                uv: uvs,
                material,
                smooth: mesh.face_smoothing.get(f).copied().unwrap_or(false),
            });
        }
        meshes.push(raw);
    }
    skipped.sort_unstable();
    skipped.dedup();
    if !skipped.is_empty() {
        notes.push(format!(
            "FBX {} data not imported (geometry only)",
            skipped.join(", ")
        ));
    }
    Ok(meshes)
}

/// The material's diffuse colour as Operandi settings, PBR base colour
/// first where the file has one.
fn read_material(m: &ufbx::Material) -> Option<crate::Material> {
    let map = if m.pbr.base_color.has_value {
        &m.pbr.base_color
    } else if m.fbx.diffuse_color.has_value {
        &m.fbx.diffuse_color
    } else {
        return None;
    };
    let c = map.value_vec4;
    let material = crate::Material {
        color: [c.x, c.y, c.z].map(|v| v.clamp(0.0, 1.0)),
        diffuse: 1.0,
        ..Default::default()
    };
    material.validate().is_ok().then_some(material)
}
