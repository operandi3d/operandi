//! Set Part (PART-001/004, upgraded to spec set-part-2026-11-04): assign one
//! part name to polygons of the active mesh as a single undoable edit.
//!
//! Supported context: one visible active mesh, Polygon mode, no live/armed
//! tool, I2 stage or selection gesture. Selected polygons are tagged; with
//! nothing selected every surface polygon of the mesh is (SP-05, provisional
//! SP-Q01). Operandi has one active mesh at a time, so SP-06 (several active
//! meshes) reduces to that mesh. Blank names and other modes are refused
//! before mutation (SP-Q02, SP-Q04). Only `Face::part` is written (SP-02);
//! the name is stored exactly as given (SP-03).
use crate::{Editor, Mesh, Mode, Result, Shared};

/// Operandi safety bound for one part name, matching item names.
pub const MAX_PART_NAME_BYTES: usize = 1024;

/// An eligible target captured from the current explicit selection. A dialog
/// holds one while open; commit refuses unless the live context still equals it.
#[derive(Clone, Debug, PartialEq)]
pub struct PartTarget {
    /// Owning document (see `DocumentToken`); reading it changes nothing.
    document: crate::DocumentToken,
    pub item: u64,
    /// Target face indices: the selection in selection order, or every
    /// surface polygon in index order when nothing is selected.
    pub faces: Vec<u32>,
    mesh: Shared<Mesh>,
}

/// `name` is applied to the current eligible target, or only to `expected`
/// when given (the dialog route).
#[derive(Clone, Debug, PartialEq)]
pub struct PartAssignment {
    pub name: String,
    pub expected: Option<PartTarget>,
}

/// Blank or whitespace-only names are refused; clearing a tag is not part of
/// the basic contract. Nonblank input is otherwise kept byte-for-byte.
pub fn validate_part_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err("Enter a part name; blank names are not assigned".into());
    }
    if name.len() > MAX_PART_NAME_BYTES {
        return Err(format!(
            "Part name must be at most {MAX_PART_NAME_BYTES} bytes"
        ));
    }
    Ok(())
}

impl Editor {
    /// Read-only Set Part eligibility. Also used before opening the dialog so
    /// an unsupported context is refused without touching other drafts.
    pub fn set_part_target(&self) -> Result<PartTarget> {
        if self.i2_active() {
            return Err("Finish or Cancel the Extrude stage before Set Part".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish the selection gesture before Set Part".into());
        }
        let item = self.scene.active().ok_or("Select a mesh before Set Part")?;
        if !self.scene.shows(item) {
            return Err("Show the mesh before Set Part".into());
        }
        let faces = crate::tag_target_faces_in(&item.mesh, &self.scene.selection);
        // A Vertex or Edge selection names nothing, silently (decided Q-PT-02).
        let component = matches!(self.scene.selection.mode, Mode::Vertex | Mode::Edge);
        if faces.is_empty() && !component {
            return Err("The mesh has no polygons to name".into());
        }
        for &f in &faces {
            if f as usize >= item.mesh.faces.len() {
                return Err("Selected polygon no longer exists".into());
            }
            if item.mesh.is_polyline(f as usize) {
                return Err("Set Part supports surface polygons only".into());
            }
        }
        Ok(PartTarget {
            document: self.identities.document_token(),
            item: item.id,
            faces,
            mesh: item.mesh.clone().into(),
        })
    }
    /// Every distinct part name in the scene, sorted, for the dialog list.
    pub fn part_names(&self) -> Vec<String> {
        self.scene.polygon_tag_names(crate::PolygonTagKind::Part)
    }
    /// The single part name shared by all target faces, if any. Mixed or
    /// absent tags give `None` rather than a stand-in name.
    pub fn common_part(target: &PartTarget) -> Option<String> {
        let mut parts = target
            .faces
            .iter()
            .map(|&f| target.mesh.faces[f as usize].part.as_deref());
        let first = parts.next()??;
        parts.all(|p| p == Some(first)).then(|| first.to_owned())
    }
    /// One atomic edit. An identical assignment changes nothing and records
    /// no history. Topology is untouched, so component identities are kept.
    pub(crate) fn set_part(&mut self, assignment: PartAssignment) -> Result<()> {
        validate_part_name(&assignment.name)?;
        let target = self.set_part_target()?;
        if let Some(expected) = &assignment.expected {
            if expected.document != target.document {
                return Err(
                    "Set Part was opened for another document; nothing was assigned".into(),
                );
            }
            if *expected != target {
                return Err("Selection changed since Set Part opened; nothing was assigned".into());
            }
        }
        let name = assignment.name;
        if target
            .faces
            .iter()
            .all(|&f| target.mesh.faces[f as usize].part.as_deref() == Some(name.as_str()))
        {
            return Ok(());
        }
        let before = self.scene.clone();
        let item = self
            .scene
            .active_mut()
            .ok_or("Select a mesh before Set Part")?;
        let mesh: &mut Mesh = &mut item.mesh;
        for &f in &target.faces {
            mesh.faces[f as usize].part = Some(name.clone());
        }
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.observe(&self.scene))
        {
            self.scene = before;
            return Err(e);
        }
        self.record("Set Part".into(), before);
        Ok(())
    }
}
