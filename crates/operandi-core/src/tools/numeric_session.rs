//! The numeric tool sessions (Cone, Ellipsoid, the Torus/Capsule/Arc/Tube/Text
//! shapes, Spike and Bridge) share one shape: the mesh item they work on, the
//! requested values and, for the primitives, the shown preview and the last
//! field error. What they have in common lives here; each tool keeps its own
//! eligibility rules and Apply/Drop in its `*_editor.rs`.
//!
//! Polygon Reduction is not one of these: it has no target until it is
//! initialized and keeps a captured source scene instead of a preview, so it
//! keeps its own `NumericReduction` and only shares the Arm guard.
use crate::*;

#[derive(Clone, Debug, PartialEq)]
pub struct NumericSession<P> {
    /// Id of the mesh item the session works on.
    pub target: u64,
    /// The values the panel shows. Spike's is its Strength.
    pub requested: P,
    /// The geometry shown for `requested`, local to `target`. Spike and
    /// Bridge change the mesh only on Apply and never preview.
    pub preview: Option<Mesh>,
    /// Why `requested` has no valid result; Apply and Drop refuse while set.
    pub error: Option<String>,
    /// UIC-78: the undo gesture of the last Apply's result, which stays
    /// live (value edits reshape it) while it is the newest step and still
    /// shown. Bridge only for now.
    pub applied_step: Option<u64>,
}

pub type NumericCone = NumericSession<ConeParameters>;
pub type NumericEllipsoid = NumericSession<EllipsoidParameters>;
/// Torus, Capsule, Arc, Tube or Text.
pub type NumericShape = NumericSession<ShapeParameters>;
pub type NumericSpike = NumericSession<f64>;
pub type NumericBridge = NumericSession<BridgeParameters>;

impl<P> NumericSession<P> {
    /// A session on `target` with nothing shown yet.
    pub fn armed(target: u64, requested: P) -> Self {
        Self {
            target,
            requested,
            preview: None,
            error: None,
            applied_step: None,
        }
    }

    /// Keep accepted values together with the outcome of building their
    /// preview. On an error the values are still kept (the panel shows what
    /// was typed) and the error is returned. `clear_preview_on_error` says
    /// whether the last good preview goes (the shapes) or stays on screen
    /// (Cone, Ellipsoid); they differed before the sessions were merged and
    /// still do.
    pub(crate) fn publish(
        &mut self,
        requested: P,
        preview: Result<Mesh>,
        clear_preview_on_error: bool,
    ) -> Result<()> {
        self.requested = requested;
        match preview {
            Ok(mesh) => {
                self.preview = Some(mesh);
                self.error = None;
                Ok(())
            }
            Err(e) => {
                if clear_preview_on_error {
                    self.preview = None;
                }
                self.error = Some(e.clone());
                Err(e)
            }
        }
    }

    /// Keep values that have nothing to show yet (a Tube with one point,
    /// empty Text, a Reset Tube). Not an error.
    pub(crate) fn publish_empty(&mut self, requested: P) {
        self.requested = requested;
        self.preview = None;
        self.error = None;
    }
}

impl NumericShape {
    pub fn kind(&self) -> ShapeKind {
        self.requested.kind()
    }
}

impl Editor {
    /// The check every numeric Arm makes first, so they all refuse the same
    /// things: another open task (normally already dropped by `tool_switch`
    /// before an Arm runs), an unfinished selection gesture, and a scene that
    /// doesn't validate.
    pub(crate) fn numeric_arm_guard(&self, name: &str) -> Result<()> {
        if self.session.has_active_task() {
            return Err(format!("Drop or Cancel the active tool before {name}"));
        }
        if self.selection_start.is_some() {
            return Err(format!("Finish selection before {name}"));
        }
        self.scene.validate()
    }

    /// The current scene with `mesh` appended to item `target`'s mesh. Not
    /// validated; callers that validated before still do.
    pub(crate) fn scene_with_appended(
        &self,
        target: u64,
        mesh: Mesh,
        missing: &str,
    ) -> Result<Scene> {
        let mut candidate = self.scene.clone();
        // Primitives take the user's Default Material and Texture Map name.
        crate::append_geometry(&mut candidate.item_mut(target).ok_or(missing)?.mesh, mesh)?;
        Ok(candidate)
    }
}
