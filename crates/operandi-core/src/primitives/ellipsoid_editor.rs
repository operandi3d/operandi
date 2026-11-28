//! Numeric Ellipsoid pipe: transient replacement, atomic append and inactive redo.
use crate::*;

impl Editor {
    fn ellipsoid_target(&self, target: u64) -> Result<&crate::scene::Item> {
        let item = self
            .scene
            .items
            .iter()
            .find(|i| i.id == target && i.visible)
            .ok_or("Select a visible mesh for Ellipsoid; empty item selection targets nothing")?;
        // A moved mesh takes the Ellipsoid where it is drawn (mapped into its
        // local space); only a zero-scale transform is refused.
        self.scene.world_to_local(target)?;
        if self.action_context != ActionContext::default() {
            return Err(
                // Spec: EA-Q03
                "Ellipsoid supports only the None or World action center".into(),
            );
        }
        Ok(item)
    }
    pub(crate) fn ellipsoid_target_ok(&self, target: u64) -> Result<()> {
        self.ellipsoid_target(target).map(|_| ())
    }
    pub(crate) fn arm_ellipsoid(&mut self) -> Result<()> {
        self.numeric_arm_guard("Ellipsoid")?;
        let target = self
            .scene
            .active()
            .ok_or("Select a mesh; use New Mesh before Ellipsoid")?
            .id;
        self.ellipsoid_target(target)?;
        if self
            .presets
            .get::<crate::EllipsoidPreset>()
            .radius
            .to_array()
            .contains(&0.)
        {
            // Spec: EA-Q01
            return Err(
                "Can't resume an Ellipsoid with zero radius; use Reset or enter the values".into(),
            );
        }
        self.session.ellipsoid = Some(NumericEllipsoid::armed(
            target,
            self.presets.get::<crate::EllipsoidPreset>().clone(),
        ));
        Ok(())
    }
    /// The Ellipsoid, made twice when the shared Symmetry is on and the
    /// tool's own Numeric Symmetry is off (SYM-B2-04).
    fn ellipsoid_with_symmetry(&self, p: &EllipsoidParameters, target: u64) -> Result<crate::Mesh> {
        // #35: shaped by the tool falloff before it's doubled.
        let mesh = self.tool_falloff_shaped(
            ellipsoid_geometry(p)?,
            &p.falloff,
            p.position,
            p.axis,
            false,
            target,
        )?;
        if p.symmetry_axis.is_some() {
            return Ok(mesh);
        }
        crate::user_defaults::primitive_symmetry(self.active_symmetry()).doubled(mesh)
    }
    pub(crate) fn configure_ellipsoid(&mut self, p: EllipsoidParameters) -> Result<()> {
        let target = self
            .session
            .ellipsoid
            .as_ref()
            .ok_or("Arm Ellipsoid first")?
            .target;
        let p = p.normalized();
        let result = self
            .ellipsoid_target(target)
            .and_then(|_| self.ellipsoid_with_symmetry(&p, target))
            .and_then(|mesh| {
                // Check append/identity budgets and the full scene before accepting
                // a preview, not for the first time at Apply or viewport consumption.
                self.ellipsoid_scene(target, mesh.clone())?;
                Ok(mesh)
            });
        let session = self
            .session
            .ellipsoid
            .as_mut()
            .ok_or("Arm Ellipsoid first")?;
        session.publish(p, result, false)
    }
    fn ellipsoid_scene(&self, target: u64, mesh: Mesh) -> Result<Scene> {
        self.ellipsoid_target(target)?;
        // `mesh` is in world space; the preview keeps it that way.
        let mesh = self.scene.mesh_world_to_local(target, mesh)?;
        let candidate = self.scene_with_appended(target, mesh, "Missing Ellipsoid target")?;
        candidate.validate()?;
        Ok(candidate)
    }
    pub fn ellipsoid_preview_scene(&self) -> Result<Option<Scene>> {
        let Some(s) = &self.session.ellipsoid else {
            return Ok(None);
        };
        s.preview
            .as_ref()
            .map(|m| self.ellipsoid_scene(s.target, m.clone()))
            .transpose()
    }
    pub(crate) fn commit_ellipsoid(&mut self, drop: bool) -> Result<()> {
        let s = self
            .session
            .ellipsoid
            .as_ref()
            .ok_or("Arm Ellipsoid first")?;
        if let Some(e) = &s.error {
            return Err(format!(
                "Correct Ellipsoid fields or Cancel before Apply/Drop: {e}"
            ));
        }
        let (target, p) = (s.target, s.requested.clone());
        // Armed-without-preview Drop is neutral. Apply explicitly constructs.
        if !drop || s.preview.is_some() {
            let mesh = self.ellipsoid_with_symmetry(&p, target)?;
            let candidate = self.ellipsoid_scene(target, mesh)?;
            let before = std::mem::replace(&mut self.scene, candidate);
            self.record("Ellipsoid".into(), before);
        }
        *self.presets.get_mut::<crate::EllipsoidPreset>() = p;
        if drop {
            self.session.ellipsoid = None;
        } else if let Some(s) = &mut self.session.ellipsoid {
            s.preview = None;
        }
        Ok(())
    }
    pub(crate) fn insert_ellipsoid_unit(&mut self, destination: UnitDestination) -> Result<()> {
        if self.session.has_active_task() || self.selection_start.is_some() {
            return Err(
                "Finish the current tool and the selection gesture before Ellipsoid unit insertion"
                    .into(),
            );
        }
        let mut staged = self.clone();
        if destination == UnitDestination::NewMesh {
            if staged.scene.items.len() >= crate::MAX_SCENE_ITEMS {
                return Err("Scene item budget exhausted".into());
            }
            staged.scene.next_id = staged.scene.next_id.max(staged.next_item_id);
            if staged.scene.next_id >= u64::MAX - 1 {
                return Err("Scene identity budget exhausted".into());
            }
            staged.scene.new_mesh()?;
            staged.scene.active_mut().ok_or("Missing new mesh")?.name = "Ellipsoid".into();
        }
        let target = staged
            .scene
            .active()
            .ok_or("Select a visible mesh for Ellipsoid unit insertion")?
            .id;
        let result =
            staged.ellipsoid_scene(target, ellipsoid_geometry(&EllipsoidParameters::default())?)?;
        let before = self.scene.clone();
        self.scene = result;
        self.next_item_id = self.next_item_id.max(self.scene.next_id);
        self.record("Unit Ellipsoid".into(), before);
        Ok(())
    }
}
