//! Bulk mesh data for scripts (kits plan K9): a mesh item's points and
//! polygons read in one go, and written back as one undo step.
//!
//! Python's `operandi.mesh_get` / `operandi.mesh_set` and the
//! `operandi.mesh.setData` command (MCP `command_run`) all come here, so
//! a script, the bridge and a typed line make the same edit.
//!
//! Writing points alone (same count, no polygons given) keeps the mesh's
//! polygons, UVs, maps and selection. Writing polygons builds a new mesh:
//! its UV maps, vertex maps and component selection go, since they number
//! the old elements; polygons take the first old polygon's material (or
//! Default).
use crate::{Editor, Face, Mesh, MeshSlot, Result};
use glam::DVec3;
use serde_json::{Map, Value, json};

/// A mesh's points and polygons, in item space (metres).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshData {
    pub points: Vec<[f64; 3]>,
    pub polygons: Vec<Vec<u32>>,
}

/// The mesh item `item`, or the active item.
fn mesh_item(e: &Editor, item: Option<u64>) -> Result<u64> {
    let id = item
        .or(e.scene.selection.item)
        .ok_or("No mesh item: select one or give item")?;
    match e.scene.item(id) {
        Some(i) if i.is_mesh() => Ok(id),
        Some(_) => Err(format!("Item {id} isn't a mesh")),
        None => Err(format!("No item {id}")),
    }
}

/// The points and polygons of `item` (default the active item).
pub fn mesh_data(e: &Editor, item: Option<u64>) -> Result<MeshData> {
    let id = mesh_item(e, item)?;
    let mesh = e.scene.item(id).expect("checked").mesh.open();
    Ok(MeshData {
        points: mesh.vertices.iter().map(|p| p.to_array()).collect(),
        polygons: mesh.faces.iter().map(|f| f.vertices.clone()).collect(),
    })
}

impl Editor {
    /// Sets `item`'s points, and its polygons when `polygons` is given, as
    /// one undo step "Set Mesh Data". A bad mesh (index out of range,
    /// polygon under three points, non-finite point) is refused and the
    /// scene stays as it was.
    pub fn set_mesh_data(
        &mut self,
        item: Option<u64>,
        points: Vec<[f64; 3]>,
        polygons: Option<Vec<Vec<u32>>>,
    ) -> Result<()> {
        let id = mesh_item(self, item)?;
        let points: Vec<DVec3> = points.into_iter().map(DVec3::from_array).collect();
        let old = self.scene.item(id).expect("checked").mesh.open().clone();
        let topology = polygons.is_some();
        let mesh = match polygons {
            None => {
                if points.len() != old.vertices.len() {
                    return Err(format!(
                        "{} points given for a mesh of {}; give polygons too to change the count",
                        points.len(),
                        old.vertices.len()
                    ));
                }
                Mesh {
                    vertices: points,
                    ..old
                }
            }
            Some(polygons) => {
                let material = old
                    .faces
                    .first()
                    .map_or(crate::material::DEFAULT_MATERIAL.to_owned(), |f| {
                        f.material.clone()
                    });
                let faces = polygons
                    .into_iter()
                    .map(|vertices| Face {
                        vertices,
                        uv: None,
                        material: material.clone(),
                        smooth: true,
                        part: None,
                        material_label: None,
                    })
                    .collect::<Vec<_>>();
                Mesh {
                    vertices: points,
                    faces: crate::Shared::new(faces),
                    ..Default::default()
                }
            }
        };
        mesh.validate()?;
        self.anim_step("Set Mesh Data", move |e| {
            let i = e.scene.item_mut(id).expect("checked");
            i.mesh = MeshSlot::from(mesh);
            if topology && e.scene.selection.item == Some(id) {
                let s = &mut e.scene.selection;
                s.vertices.clear();
                s.edges.clear();
                s.faces.clear();
            }
            Ok(true)
        })
    }
}

fn points_arg(v: &Value) -> Result<Vec<[f64; 3]>> {
    let list = v.as_array().ok_or("points must be a list of [x, y, z]")?;
    list.iter()
        .map(|p| {
            let p = p.as_array().filter(|p| p.len() == 3);
            let p = p.ok_or("Each point is [x, y, z]")?;
            let n = |v: &Value| v.as_f64().ok_or("Point coordinates are numbers");
            Ok([n(&p[0])?, n(&p[1])?, n(&p[2])?])
        })
        .collect()
}

fn polygons_arg(v: &Value) -> Result<Vec<Vec<u32>>> {
    let list = v
        .as_array()
        .ok_or("polygons must be a list of point index lists")?;
    list.iter()
        .map(|f| {
            f.as_array()
                .ok_or("Each polygon is a list of point indices")?
                .iter()
                .map(|i| {
                    i.as_u64()
                        .and_then(|i| u32::try_from(i).ok())
                        .ok_or_else(|| "Point indices are whole numbers".to_owned())
                })
                .collect()
        })
        .collect()
}

/// `operandi.mesh.setData item? points polygons?` (registry handler).
pub fn run(e: &mut Editor, _name: &str, a: &Map<String, Value>) -> Result<Option<crate::Command>> {
    let item = match a.get("item") {
        None | Some(Value::Null) => None,
        Some(v) => Some(v.as_u64().ok_or("item is an item id")?),
    };
    let points = points_arg(a.get("points").ok_or("points is required")?)?;
    let polygons = match a.get("polygons") {
        None | Some(Value::Null) => None,
        Some(v) => Some(polygons_arg(v)?),
    };
    e.set_mesh_data(item, points, polygons)?;
    Ok(None)
}

/// The mesh as JSON (`{"item", "points", "polygons"}`), for scripts.
pub fn to_json(e: &Editor, item: Option<u64>) -> Result<Value> {
    let id = mesh_item(e, item)?;
    let d = mesh_data(e, Some(id))?;
    Ok(json!({"item": id, "points": d.points, "polygons": d.polygons}))
}
