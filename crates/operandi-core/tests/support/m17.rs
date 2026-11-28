//! Adapter for admitted, original local-use fixtures. Not reference exports.
use glam::{DVec2, DVec3};
use operandi_core::*;
use serde_json::Value;
use std::collections::BTreeMap;
pub fn editor_from(f: &Value) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.history.undo.clear();
    let vertices = serde_json::from_value::<Vec<DVec3>>(f["vertices"].clone()).unwrap();
    let loops = serde_json::from_value::<Vec<Vec<u32>>>(f["faces"].clone()).unwrap();
    let faces = loops
        .into_iter()
        .enumerate()
        .map(|(i, vertices)| Face {
            uv: Some(
                (0..vertices.len())
                    .map(|c| DVec2::new(i as f64, c as f64))
                    .collect(),
            ),
            vertices,
            material: format!("material_{i}"),
            smooth: i % 2 == 0,
            part: Some(format!("part_{i}")),
            material_label: Some(format!("label_{i}")),
        })
        .collect();
    let mut uv_maps = BTreeMap::new();
    if let Some(maps) = f["attributes"]["corner_maps"].as_object() {
        for (name, values) in maps {
            uv_maps.insert(
                name.clone(),
                serde_json::from_value(values.clone()).unwrap(),
            );
        }
    }
    e.scene.active_mut().unwrap().mesh = Mesh {
        attributes: Default::default(),
        vertices,
        faces,
        uv_maps: uv_maps.into(),
    }
    .into();
    e.scene.selection.faces =
        serde_json::from_value(f["selected_face_indices"].clone()).unwrap_or(vec![0]);
    e
}
pub fn counts(m: &Mesh) -> [usize; 3] {
    [
        m.faces
            .iter()
            .flat_map(|f| f.vertices.iter())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        m.edges().len(),
        m.faces.len(),
    ]
}
pub fn set(
    e: &mut Editor,
    tool: Tool,
    shift: f64,
    inset: f64,
    extent: DVec3,
    sides: u32,
    grouped: bool,
) {
    if e.session.live.is_none() {
        e.dispatch(Command::Activate(tool)).unwrap();
    }
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.shift = shift;
    p.inset = inset;
    p.extent = extent;
    p.extrude_sides = sides;
    p.extrude_mode = ExtrudeMode::Shift;
    p.extrude_uvs = ExtrudeUvs::None;
    p.group_polygons = grouped;
    e.dispatch(Command::Preview(p)).unwrap();
}
