//! Solid Sketch data kept with the mesh (spec `solid-sketch` v0.3, section
//! 10, [OP]): the node tree and the session's options as a mesh attribute,
//! not as hidden polygons, so it saves with the scene, copies with the item
//! and never reaches exported files as stray lines (spec 8.14, 8.16).
use crate::solid_sketch::{SketchOptions, SolidSketch};
use crate::{Editor, Mesh, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredSketch {
    pub sketch: SolidSketch,
    pub options: SketchOptions,
    /// Current node when the tool was dropped (Q10: picked up as current).
    #[serde(default)]
    pub current: Option<usize>,
}

/// The mesh's stored sketch, if it has a valid one.
pub fn stored(mesh: &Mesh) -> Option<StoredSketch> {
    mesh.attributes
        .solid_sketch
        .as_deref()
        .filter(|s| s.sketch.validate().is_ok())
        .cloned()
}

/// What activating Solid Sketch on the active mesh needs to ask (spec 5.5):
/// `None` when there is no stored sketch; otherwise how many polygons
/// picking it up would remove (the layer's polygons the sketch didn't make).
pub fn resume_question(editor: &Editor) -> Option<usize> {
    let item = editor.scene.active().filter(|i| i.is_mesh())?;
    let data = stored(&item.mesh)?;
    let made = data.sketch.cage(&data.options).map_or(0, |m| m.faces.len());
    Some(item.mesh.faces.len().saturating_sub(made))
}

impl Editor {
    /// `operandi.solidSketch.strip`: removes the sketch data from the
    /// selected meshes, leaving plain polygons. One undo step.
    pub(crate) fn strip_solid_sketch(&mut self) -> Result<()> {
        if self.session.has_active_task() {
            return Err("Drop or Cancel the active tool before stripping sketch data".into());
        }
        let ids = self.scene.selected_items();
        let mut candidate = self.scene.clone();
        let mut stripped = 0;
        for item in candidate.items.iter_mut().filter(|i| ids.contains(&i.id)) {
            if item.mesh.attributes.solid_sketch.is_some() {
                let mut mesh = Mesh::clone(&item.mesh);
                mesh.attributes.solid_sketch = None;
                item.mesh = mesh.into();
                stripped += 1;
            }
        }
        if stripped == 0 {
            return Err("The selected meshes have no Solid Sketch data".into());
        }
        let before = std::mem::replace(&mut self.scene, candidate);
        self.record("Strip Solid Sketch".into(), before);
        Ok(())
    }
}
