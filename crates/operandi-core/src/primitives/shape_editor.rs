//! Numeric session shared by Torus, Capsule, Arc and Tube: arm on the
//! active mesh, preview each accepted setting, Apply appends and keeps the
//! tool, Drop appends (when there is a preview) and closes, Cancel discards.
//! The same lifecycle as the numeric Ellipsoid, for four shapes at once.
use crate::*;

impl Editor {
    fn shape_target(&self, target: u64, kind: ShapeKind) -> Result<&crate::scene::Item> {
        let name = kind.label();
        let item = self
            .scene
            .items
            .iter()
            .find(|i| i.id == target && i.visible)
            .ok_or_else(|| format!("Select a visible mesh for {name}"))?;
        // A moved mesh takes the shape where it is drawn (mapped into its
        // local space); only a zero-scale transform is refused.
        self.scene.world_to_local(target)?;
        if self.action_context != ActionContext::default() {
            // Spec: NPR-Q11
            return Err(format!(
                "{name} supports only the None or World action center"
            ));
        }
        Ok(item)
    }
    pub(crate) fn shape_target_ok(&self, target: u64, kind: ShapeKind) -> Result<()> {
        self.shape_target(target, kind).map(|_| ())
    }
    pub(crate) fn arm_shape(&mut self, kind: ShapeKind) -> Result<()> {
        let name = kind.label();
        self.numeric_arm_guard(name)?;
        let target = self
            .scene
            .active()
            .ok_or_else(|| format!("Select a mesh; use New Mesh before {name}"))?
            .id;
        self.shape_target(target, kind)?;
        let requested = self.presets.get::<crate::ShapePreset>().get(kind);
        self.session.shape = Some(NumericShape::armed(target, requested.clone()));
        // Torus, Capsule, Arc and Tube are drawn in the viewport, like Cone
        // and Ellipsoid: nothing shows until a drag (or a panel edit) places
        // one (Jason, 2026-11-05). The instant shape is the Unit option
        // (Ctrl/Shift-click). Text shows its remembered lettering at once.
        if matches!(&requested, ShapeParameters::Text(t) if !t.text.is_empty()) {
            let _ = self.configure_shape(requested);
        }
        Ok(())
    }
    fn shape_with_symmetry(&self, p: &ShapeParameters, target: u64) -> Result<Mesh> {
        // #35: the Torus and Capsule are shaped by the tool falloff before
        // they're doubled.
        let mesh = match p {
            ShapeParameters::Torus(t) => self.tool_falloff_shaped(
                p.geometry()?,
                &t.falloff,
                t.position,
                t.axis,
                false,
                target,
            )?,
            ShapeParameters::Capsule(c) => self.tool_falloff_shaped(
                p.geometry()?,
                &c.falloff,
                c.position,
                c.axis,
                false,
                target,
            )?,
            _ => p.geometry()?,
        };
        crate::user_defaults::primitive_symmetry(self.active_symmetry()).doubled(mesh)
    }
    fn normalized_shape(p: ShapeParameters) -> ShapeParameters {
        match p {
            ShapeParameters::Torus(t) => ShapeParameters::Torus(t.normalized()),
            ShapeParameters::Capsule(c) => ShapeParameters::Capsule(c.normalized()),
            ShapeParameters::Arc(a) => ShapeParameters::Arc(a.normalized()),
            ShapeParameters::Tube(t) => ShapeParameters::Tube(t.normalized()),
            ShapeParameters::Text(t) => ShapeParameters::Text(t.normalized()),
            ShapeParameters::Teapot(t) => ShapeParameters::Teapot(t),
        }
    }
    pub(crate) fn configure_shape(&mut self, p: ShapeParameters) -> Result<()> {
        let session = self
            .session
            .shape
            .as_ref()
            .ok_or("Choose Torus, Capsule, Arc or Tube first")?;
        if session.kind() != p.kind() {
            return Err(format!(
                "{} is active; drop it before setting {} values",
                session.kind().label(),
                p.kind().label()
            ));
        }
        let target = session.target;
        let p = Self::normalized_shape(p);
        // A Tube being drawn point by point has nothing to show until its
        // second point; that isn't an error (Jason, 2026-11-05).
        if matches!(&p, ShapeParameters::Tube(t) if t.points.len() < 2)
            || matches!(&p, ShapeParameters::Text(t) if t.text.trim().is_empty() && t.validate().is_ok())
        {
            self.session
                .shape
                .as_mut()
                .expect("checked above")
                .publish_empty(p);
            return Ok(());
        }
        let result = self
            .shape_target(target, p.kind())
            .and_then(|_| self.shape_with_symmetry(&p, target))
            .and_then(|mesh| {
                self.shape_scene(target, mesh.clone())?;
                Ok(mesh)
            });
        let session = self.session.shape.as_mut().expect("checked above");
        session.publish(p, result, true)
    }
    fn shape_scene(&self, target: u64, mesh: Mesh) -> Result<Scene> {
        // `mesh` is in world space; the preview keeps it that way.
        let mesh = self.scene.mesh_world_to_local(target, mesh)?;
        let candidate = self.scene_with_appended(target, mesh, "The primitive's mesh is gone")?;
        candidate.validate()?;
        Ok(candidate)
    }
    /// The scene with the preview appended, for drawing.
    pub fn shape_preview_scene(&self) -> Result<Option<Scene>> {
        let Some(s) = &self.session.shape else {
            return Ok(None);
        };
        s.preview
            .as_ref()
            .map(|m| self.shape_scene(s.target, m.clone()))
            .transpose()
    }
    pub(crate) fn reset_shape(&mut self) -> Result<()> {
        let kind = self
            .session
            .shape
            .as_ref()
            .ok_or("Choose Torus, Capsule, Arc or Tube first")?
            .kind();
        let mut staged = self.clone();
        staged
            .presets
            .get_mut::<crate::ShapePreset>()
            .set(ShapeParameters::reset(kind));
        let reset = ShapeParameters::reset(kind);
        if matches!(&reset, ShapeParameters::Tube(_)) {
            // Reset Tube has no points: nothing to preview yet.
            staged
                .session
                .shape
                .as_mut()
                .expect("checked above")
                .publish_empty(reset);
        } else {
            staged.configure_shape(reset)?;
        }
        *self = staged;
        Ok(())
    }
    /// Apply (`drop` false) appends and keeps the tool for another one;
    /// Drop appends the shown preview and closes the tool.
    pub(crate) fn commit_shape(&mut self, drop: bool) -> Result<()> {
        let s = self
            .session
            .shape
            .as_ref()
            .ok_or("Choose Torus, Capsule, Arc or Tube first")?;
        let kind = s.kind();
        if let Some(e) = &s.error {
            return Err(format!(
                "Correct the {} values or Cancel before Apply/Drop: {e}",
                kind.label()
            ));
        }
        let (target, p) = (s.target, s.requested.clone());
        if !drop || s.preview.is_some() {
            let mesh = self.shape_with_symmetry(&p, target)?;
            let candidate = self.shape_scene(target, mesh)?;
            let before = std::mem::replace(&mut self.scene, candidate);
            self.record(kind.label().into(), before);
        }
        self.presets.get_mut::<crate::ShapePreset>().set(p);
        if drop {
            self.session.shape = None;
        } else if let Some(s) = &mut self.session.shape {
            s.preview = None;
        }
        Ok(())
    }
}
