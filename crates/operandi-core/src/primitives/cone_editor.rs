//! Numeric-only Cone pipe. Preview meshes are transient snapshots, separate from
//! the committed scene and the existing viewport-draw Live transaction.
use crate::*;
impl Editor {
    pub(crate) fn arm_cone(&mut self) -> Result<()> {
        // Another open tool was dropped first by `tool_switch`.
        self.numeric_arm_guard("numeric Cone")?;
        let target = self
            .scene
            .active()
            .filter(|i| i.visible)
            .ok_or("Select a visible mesh before numeric Cone")?
            .id;
        self.session.cone = Some(NumericCone::armed(
            target,
            self.presets.get::<crate::ConePreset>().clone(),
        ));
        Ok(())
    }
    pub(crate) fn configure_cone(&mut self, p: ConeParameters) -> Result<()> {
        let previous = self.session.cone.as_ref().ok_or("Arm numeric Cone first")?;
        if p.polygon == PolygonRepresentation::Face
            && previous.requested.polygon == PolygonRepresentation::Face
            && p.edge_weight != previous.requested.edge_weight
        {
            return Err(
                "Edge Weight is disabled in Face; choose a subdivision representation to edit it"
                    .into(),
            );
        }
        let target = previous.target;
        let p = p.normalized();
        let result = self.cone_local_mesh(target, &p);
        let session = self.session.cone.as_mut().ok_or("Arm numeric Cone first")?;
        session.publish(p, result, false)
    }
    fn cone_local_mesh(&self, target: u64, p: &ConeParameters) -> Result<Mesh> {
        let item = self
            .scene
            .items
            .iter()
            .find(|i| i.id == target && i.visible)
            .ok_or("Cone target is missing or hidden")?;
        let inverse = self.scene.world_to_local(item.id)?;
        // SYM-B2-04: the shared Symmetry makes the Cone twice.
        let mut mesh = crate::user_defaults::primitive_symmetry(self.active_symmetry()).doubled(
            self.tool_falloff_shaped(
                cone_geometry(p)?,
                &p.falloff,
                p.position,
                p.axis,
                true,
                target,
            )?,
        )?;
        for v in &mut mesh.vertices {
            *v = inverse.transform_point3(*v);
        }
        mesh.validate()?;
        Ok(mesh)
    }
    pub(crate) fn apply_cone(&mut self) -> Result<()> {
        let session = self.session.cone.as_ref().ok_or("Arm numeric Cone first")?;
        if let Some(error) = &session.error {
            return Err(format!("Correct Cone fields before Apply: {error}"));
        }
        let target = session.target;
        let p = session.requested.clone();
        let mesh = self.cone_local_mesh(target, &p)?;
        let candidate = self.scene_with_appended(target, mesh, "Missing Cone target")?;
        candidate.validate()?;
        // Like Ellipsoid's Apply: a plain "Cone" step. Undo takes back the
        // geometry only; it never reopens or re-arms the tool.
        let before = std::mem::replace(&mut self.scene, candidate);
        self.record("Cone".into(), before);
        self.session.cone = Some(NumericCone::armed(target, p));
        Ok(())
    }
    pub(crate) fn drop_cone(&mut self) -> Result<()> {
        let session = self.session.cone.as_ref().ok_or("No numeric Cone")?;
        if let Some(error) = &session.error {
            return Err(format!(
                "Correct Cone fields or Cancel before Drop: {error}"
            ));
        }
        // Drop keeps the Cone that is shown, like Ellipsoid and the drawn
        // primitives: the preview becomes one "Cone" step, and one Undo
        // removes it with the tool closed. Armed with no preview, Drop is
        // neutral and records nothing.
        let (target, p) = (session.target, session.requested.clone());
        if session.preview.is_some() {
            let mesh = self.cone_local_mesh(target, &p)?;
            let candidate = self.scene_with_appended(target, mesh, "Missing Cone target")?;
            candidate.validate()?;
            let before = std::mem::replace(&mut self.scene, candidate);
            self.record("Cone".into(), before);
        }
        *self.presets.get_mut::<crate::ConePreset>() = p;
        self.session.cone = None;
        Ok(())
    }
    /// Viewport consumption only. Committed scene/picking state is unchanged.
    pub fn cone_preview_scene(&self) -> Result<Option<Scene>> {
        let Some(session) = &self.session.cone else {
            return Ok(None);
        };
        let Some(mesh) = &session.preview else {
            return Ok(None);
        };
        // Not validated, as before: the mesh was checked when it was built.
        self.scene_with_appended(session.target, mesh.clone(), "Missing Cone target")
            .map(Some)
    }
}
