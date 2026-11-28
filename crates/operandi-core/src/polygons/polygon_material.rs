//! Set Material (MAT-001/004). The dialog names a material tag for the
//! selected polygons, creating it when it is new; a polygon has one material
//! at a time, so assigning overwrites the previous tag. When the dialog also
//! sends material settings, the named material is created or updated in the
//! scene's material table (material.rs) in the same undo step.
//!
//! `Default` (or an empty tag) is the default material: assigning it clears
//! the polygons' tag, and its settings are stored under `Default`.
//!
//! Like Set Part (SP-05), with nothing selected every surface polygon of the
//! active mesh is tagged.
use crate::{Editor, Mode, Result, Scene};

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialTarget {
    document: crate::DocumentToken,
    scene: Scene,
    /// Target face indices: the selection in selection order, or every
    /// surface polygon in index order when nothing is selected.
    faces: Vec<u32>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialAssignment {
    pub name: String,
    pub expected: Option<MaterialTarget>,
    /// Settings to store for the material; `None` assigns the tag only and
    /// leaves any stored settings as they are (`poly.setMaterial {Name}`).
    pub material: Option<crate::Material>,
}
impl Editor {
    pub fn set_material_target(&self) -> Result<MaterialTarget> {
        if self.i2_active() {
            return Err("Finish or Cancel the Extrude stage before Set Material".into());
        }
        if self.session.pen_draft.is_some() {
            return Err("Finish or Cancel Pen before Set Material".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish the selection gesture before Set Material".into());
        }
        let item = self
            .scene
            .active()
            .filter(|i| self.scene.shows(i))
            .ok_or("Select a visible mesh before Set Material")?;
        let faces = crate::tag_target_faces_in(&item.mesh, &self.scene.selection);
        // A Vertex or Edge selection tags no polygon but still creates the
        // material (decided Q-SM-01).
        let component = matches!(self.scene.selection.mode, Mode::Vertex | Mode::Edge);
        if faces.is_empty() && !component {
            return Err("The mesh has no polygons to tag".into());
        }
        for &f in &faces {
            item.mesh
                .faces
                .get(f as usize)
                .ok_or("Selected polygon no longer exists")?;
            if item.mesh.is_polyline(f as usize) {
                return Err("Set Material requires surface polygons".into());
            }
        }
        Ok(MaterialTarget {
            document: self.identities.document_token(),
            scene: self.scene.clone(),
            faces,
        })
    }
    pub fn common_material(target: &MaterialTarget) -> Option<String> {
        let mesh = &target.scene.active()?.mesh;
        let mut names = target
            .faces
            .iter()
            .map(|&f| &mesh.faces[f as usize].material);
        let first = names.next()?;
        if crate::material::is_default_name(first) {
            return names
                .all(|n| crate::material::is_default_name(n))
                .then(|| crate::material::DEFAULT_MATERIAL.to_owned());
        }
        names.all(|n| n == first).then(|| first.clone())
    }
    pub(crate) fn set_material(&mut self, assignment: MaterialAssignment) -> Result<()> {
        let name = &assignment.name;
        if name.trim().is_empty() || name.len() > 1024 || name.chars().any(char::is_control) {
            return Err("Enter a material name".into());
        }
        if let Some(material) = &assignment.material {
            material.validate()?;
        }
        let target = self.set_material_target()?;
        if assignment
            .expected
            .as_ref()
            .is_some_and(|expected| *expected != target)
        {
            return Err("Set Material target changed since the dialog opened".into());
        }
        let default = crate::material::is_default_name(name);
        // What the selected polygons' tag fields become.
        let (tag, label) = if default {
            (String::new(), None)
        } else {
            (name.clone(), Some(name.clone()))
        };
        let item = target.scene.active().ok_or("Select a mesh")?;
        let selected = &target.faces;
        if !default {
            // A new name creates the tag; an existing one must be unambiguous.
            let ambiguous = item.mesh.faces.iter().enumerate().any(|(i, f)| {
                !selected.contains(&(i as u32))
                    && f.material == *name
                    && f.material_label.as_deref() != Some(name.as_str())
            });
            if ambiguous {
                return Err("Existing material tag is ambiguous".into());
            }
        }
        // The material table after the assignment. Default settings equal
        // to the factory ones aren't stored, so the viewport keeps its theme
        // color until the default material is really changed.
        let key = if default {
            crate::material::DEFAULT_MATERIAL
        } else {
            name.as_str()
        };
        let mut materials = self.scene.materials.clone();
        if let Some(material) = &assignment.material {
            if default && *material == crate::Material::default() {
                materials.remove(key);
            } else {
                materials.insert(key.to_owned(), material.clone());
            }
        }
        let tags_unchanged = selected.iter().all(|&f| {
            let face = &item.mesh.faces[f as usize];
            face.material == tag && face.material_label == label
        });
        // The material is created even when no polygon is tagged (a Vertex or
        // Edge selection, Q-SM-01): only an existing material is a no-op.
        let exists = self.scene.shader_tree.material_for_tag(key).is_some();
        if tags_unchanged && materials == self.scene.materials && (exists || !selected.is_empty()) {
            return Ok(());
        }
        let before = self.scene.clone();
        self.scene.materials = materials;
        // The shader tree shows each material tag as a group holding its
        // material, in the same undo step (ST-1).
        let shown = self.scene.material_or_default(key);
        let mapped = self.scene.map_materials_into_default_tree();
        if let Err(e) = mapped.and_then(|()| {
            if assignment.material.is_some() {
                self.scene.shader_tree.mirror_material(key, &shown)
            } else {
                self.scene
                    .shader_tree
                    .ensure_material(key, &shown)
                    .map(|_| ())
            }
        }) {
            self.scene = before;
            return Err(e);
        }
        if !tags_unchanged {
            let mesh = &mut self.scene.active_mut().ok_or("Select a mesh")?.mesh;
            for &f in selected {
                let face = &mut mesh.faces[f as usize];
                face.material = tag.clone();
                face.material_label = label.clone();
            }
        }
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.observe(&self.scene))
        {
            self.scene = before;
            return Err(e);
        }
        self.record("Set Material".into(), before);
        Ok(())
    }
}
