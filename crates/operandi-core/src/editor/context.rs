//! The action context, Symmetry, the pivot frame and falloffs the live
//! tools work in.
use super::*;

impl Editor {
    /// Read-only availability check for UI affordances. Dispatch resolves the
    /// same frame again before publishing any context or live-state change.
    pub fn validate_action_context(&self, context: crate::ActionContext) -> Result<()> {
        self.action_context_frame(context).map(|_| ())
    }
    pub(super) fn action_context_frame(
        &self,
        context: crate::ActionContext,
    ) -> Result<Option<crate::ActionFrame>> {
        // Selection, Parent and Item axes come from the scene (the
        // selection, the item hierarchy), not from the setting alone.
        if !matches!(
            context.axis,
            crate::AxisOrientation::Selection
                | crate::AxisOrientation::Parent
                | crate::AxisOrientation::Item
        ) {
            context.axis.basis()?;
        }
        if context.axis == crate::AxisOrientation::Selection && self.live.is_none() {
            // The same world-space frame a tool gets on activation, so a
            // moved mesh doesn't grey the choice out.
            Self::extrude_frame(context, Tool::Move, &self.scene)?;
        }
        if let Some(live) = &self.live {
            if live.action_frame.is_none() {
                return Err(
                    "Drop or Cancel this tool before changing Action Center or axes".into(),
                );
            }
            // A tool in a UV view: the frame in UV (UVT-133).
            if live.params.uv.is_some() {
                return self.uv_action_frame(context);
            }
            Ok(Some(
                crate::pivot_service::PivotRequest {
                    context,
                    ..self.pivot_request(&live.base, live.tool)
                }
                .frame()?,
            ))
        } else {
            Ok(None)
        }
    }
    /// Change the Symmetry setting (tool context, not saved). Turning it off
    /// remembers the last setting for Toggle Symmetry.
    ///
    /// Tool chain (Jason, 2026-11-14): Symmetry is a layer under every tool,
    /// so it changes at any time and the open tool follows at once: its
    /// handle moves to the picked side (or back), and whatever it shows is
    /// rebuilt under the new plane. Not an undo step of its own. If the
    /// open tool can't take the new setting, everything stays as it was.
    pub(super) fn set_symmetry(&mut self, symmetry: crate::Symmetry) -> Result<()> {
        symmetry.validate()?;
        let previous = self.symmetry;
        let staged = (
            self.scene.clone(),
            self.live.clone(),
            self.cone.clone(),
            self.ellipsoid.clone(),
            self.shape.clone(),
            self.solid_sketch.clone(),
        );
        self.symmetry = symmetry;
        if let Err(error) = self.follow_symmetry(previous) {
            self.symmetry = previous;
            (
                self.scene,
                self.live,
                self.cone,
                self.ellipsoid,
                self.shape,
                self.solid_sketch,
            ) = staged;
            return Err(error);
        }
        if previous.axis.is_some() && symmetry.axis.is_none() {
            self.symmetry_resume = Some(previous);
        }
        // Section 8's unpaired warning follows the new plane too.
        self.note_unpaired_points();
        Ok(())
    }
    /// Off, or back on with the last setting (X the first time).
    pub(super) fn toggle_symmetry(&mut self) -> Result<()> {
        let next = if self.symmetry.axis.is_some() {
            crate::Symmetry::OFF
        } else {
            self.symmetry_resume.unwrap_or_else(crate::Symmetry::x)
        };
        self.set_symmetry(next)
    }
    /// The open tool picks up a Symmetry change (see `set_symmetry`).
    pub(super) fn follow_symmetry(&mut self, previous: crate::Symmetry) -> Result<()> {
        let previous = previous.through(self.scene.workplane.center);
        let current = self.active_symmetry();
        if let Some(live) = &self.live
            && let Some(frame) = live.action_frame
            && live.tool != Tool::Bend
        {
            // The handle follows the pivot service under the new plane
            // (ACS-01). A centre the user placed (SetActionCenter, ACS-07)
            // is kept.
            let request = |symmetry| crate::pivot_service::PivotRequest {
                symmetry,
                ..self.pivot_request(&live.base, live.tool)
            };
            // A centre the user placed stays put, but its Selection axes
            // still follow the side it is on under the new plane (S-02).
            let _ = (previous, frame);
            let now = crate::pivot_service::PivotRequest {
                placed: live.placed_centre,
                ..request(current)
            }
            .frame()?;
            if let Some(live) = &mut self.live {
                live.action_frame = Some(now);
            }
        }
        if let Some(parameters) = self
            .live
            .as_ref()
            .filter(|live| live.applied)
            .map(|live| live.params.clone())
        {
            self.preview(parameters, false)?;
        }
        if let Some(session) = self.cone.as_ref().filter(|s| s.preview.is_some()) {
            self.configure_cone(session.requested.clone())?;
        }
        if let Some(session) = self.ellipsoid.as_ref().filter(|s| s.preview.is_some()) {
            self.configure_ellipsoid(session.requested.clone())?;
        }
        if let Some(session) = self.shape.as_ref().filter(|s| s.preview.is_some()) {
            self.configure_shape(session.requested.clone())?;
        }
        let mirror = crate::solid_sketch_symmetry::SketchMirror::of(current);
        if let Some(session) = &mut self.solid_sketch {
            session.follow_symmetry(mirror);
        }
        Ok(())
    }

    /// The Symmetry setting in effect: with Work Plane on, its plane runs
    /// through the work plane centre (SYM-B2-08).
    pub fn active_symmetry(&self) -> crate::Symmetry {
        self.symmetry.through(self.scene.workplane.center)
    }
    /// SYM3-30: the mirror counterpart of a picked component, when
    /// Symmetry is on and it differs from the pick.
    pub(super) fn mirror_pick(&self, item: u64, hit: crate::Component) -> Option<crate::Component> {
        self.scene
            .symmetric_counterpart(item, hit, self.active_symmetry())
            .filter(|&m| m != hit)
    }
    /// The live tool's frame from the pivot service (pivot_service.rs):
    /// the rule sheet's centre and axes; under Symmetry the + half's
    /// centre (S-01) and, for Selection axes, the + half's axes (S-02).
    pub(super) fn pivot_frame(&self, scene: &Scene, tool: Tool) -> Result<crate::ActionFrame> {
        self.pivot_request(scene, tool).frame()
    }
    /// The whole active falloff (the active one, the ones added on top and
    /// the selected weight map), or `None` with Falloff None.
    pub fn active_falloff(&self) -> Option<crate::ActiveFalloff> {
        Some(crate::ActiveFalloff {
            first: self.spatial_falloff?,
            added: self.added_falloffs.clone(),
            weight_map: self.selected_weight_map.clone(),
            element: self.falloff_element,
            symmetry: self.active_symmetry(),
            stamps: self.falloff_stamps.clone(),
            path: self.falloff_path.clone(),
        })
    }
    /// The tool falloff as the Cube, Sphere and Cylinder read it (#15), or
    /// `None` with no falloff or one that can't weigh new points
    /// ([`Self::primitive_falloff_note`] says why).
    pub fn primitive_weights(&self) -> Option<crate::ShapeWeights> {
        crate::ShapeWeights::from_active(&self.active_falloff()?).ok()
    }
    /// Fit Falloff to Shape (#15): a Linear falloff along the live Cube,
    /// Sphere or Cylinder's axis, full at its top and none at its base:
    /// the spine ramp, in one click. Keeps the Linear preset's Shape and
    /// Use World Transforms.
    pub fn primitive_fit_falloff(&self) -> Result<crate::Falloff> {
        let live = self
            .live
            .as_ref()
            .filter(|l| matches!(l.tool, Tool::Box | Tool::Sphere | Tool::Cylinder))
            .ok_or("Start a Cube, Sphere or Cylinder first")?;
        let p = &live.params;
        let axis = p.radial_axis.min(2);
        let half = if live.tool == Tool::Box {
            p.size[axis].abs() * 0.5
        } else {
            p.radial.ok_or("Draw the shape first")?.radius[axis].abs()
        };
        if !half.is_finite() || half <= 0. {
            return Err("Give the shape some height along its axis first".into());
        }
        let (mut top, mut base) = (p.position, p.position);
        top[axis] += half;
        base[axis] -= half;
        let preset = self.linear_falloff_preset;
        if !preset.world
            && let Some(item) = self.scene.active()
        {
            let m = self.scene.world_to_local(item.id)?;
            top = m.transform_point3(top);
            base = m.transform_point3(base);
        }
        Ok(crate::Falloff::Linear(crate::LinearFalloff {
            start: top,
            end: base,
            ..preset
        }))
    }
    /// `mesh` (a numeric Cone, Capsule, Torus or Ellipsoid in world space)
    /// shaped by the tool falloff, as the Cube, Sphere and Cylinder are
    /// (#35). `target` is the mesh it goes into, for falloffs with Use World
    /// Transforms off.
    pub(crate) fn tool_falloff_shaped(
        &self,
        mesh: crate::Mesh,
        falloff: &crate::ShapeFalloff,
        centre: glam::DVec3,
        axis: usize,
        respace: bool,
        target: u64,
    ) -> Result<crate::Mesh> {
        let weights = self.primitive_weights();
        let to_local = self.scene.world_to_local(target).unwrap_or_default();
        let weigh = weights.as_ref().map(|w| w.weigher(to_local));
        let weight = weigh.as_ref().map(|f| f as &dyn Fn(glam::DVec3) -> f64);
        crate::primitive_shapes::shaped(mesh, falloff, centre, axis, respace, weight)
    }
    /// After a falloff change: a numeric Cone, Capsule, Torus or Ellipsoid
    /// that shows a preview rebuilds it, so the shape follows the falloff
    /// while the tool is open (#35). A rebuild that fails says why in the
    /// tool's panel, as a field edit would.
    pub(crate) fn refresh_falloff_primitives(&mut self) {
        let shown = |preview: &Option<crate::Mesh>, error: &Option<String>| {
            preview.is_some() || error.is_some()
        };
        if let Some(p) = self
            .cone
            .as_ref()
            .filter(|s| shown(&s.preview, &s.error))
            .map(|s| s.requested.clone())
        {
            let _ = self.configure_cone(p);
        }
        if let Some(p) = self
            .ellipsoid
            .as_ref()
            .filter(|s| shown(&s.preview, &s.error))
            .map(|s| s.requested.clone())
        {
            let _ = self.configure_ellipsoid(p);
        }
        if let Some(p) = self
            .shape
            .as_ref()
            .filter(|s| shown(&s.preview, &s.error))
            .map(|s| s.requested.clone())
            .filter(|p| {
                matches!(
                    p,
                    crate::ShapeParameters::Torus(_) | crate::ShapeParameters::Capsule(_)
                )
            })
        {
            let _ = self.configure_shape(p);
        }
    }
    /// Why the active falloff can't shape a primitive, if it can't.
    pub fn primitive_falloff_note(&self) -> Option<String> {
        crate::ShapeWeights::from_active(&self.active_falloff()?).err()
    }
    /// The one direction a tool means by "the action axis" (Twist Bevel's
    /// Rotation Axis = Action Axis), from the action frame: its Y, world Y
    /// by default. With Selection or Element axes it is the frame's Z, the
    /// column that carries the normal under A-SEL and A-ELEM (AC-RULES
    /// v6); the normal was on Y before, so those tools keep turning about
    /// it.
    pub fn action_axis(&self, frame: crate::ActionFrame) -> DVec3 {
        if matches!(
            self.action_context.axis,
            crate::AxisOrientation::Selection | crate::AxisOrientation::Element(_)
        ) {
            frame.basis.z_axis
        } else {
            frame.basis.y_axis
        }
    }
    /// The action axes Bend leans along (World when the action axis can't
    /// be had for this selection, e.g. Selection axes without polygons).
    /// BN-45 (BH-Q12, provisional): the status bar text while Bend's axis
    /// runs along its spine, so the bend can't move anything.
    pub fn bend_notice(&self) -> Option<&'static str> {
        let live = self.live.as_ref().filter(|l| l.tool == Tool::Bend)?;
        live.params
            .bend
            .axis_along_spine(self.bend_axes(&self.scene))
            .then_some(crate::bend::AXIS_ALONG_SPINE)
    }
    pub fn bend_axes(&self, scene: &Scene) -> glam::DMat3 {
        self.live
            .as_ref()
            .and_then(|l| l.action_frame)
            .or_else(|| self.action_context.resolve(scene).ok())
            .map_or(glam::DMat3::IDENTITY, |f| f.basis)
    }
    /// Falloff weight of every point of item `id`; `None` with no falloff.
    /// An Element falloff is in the stack (FO-45).
    pub(crate) fn has_element_falloff(&self) -> bool {
        self.spatial_falloff
            .iter()
            .chain(self.added_falloffs.iter().map(|a| &a.0))
            .any(|f| matches!(f, crate::Falloff::Element(_)))
    }
    pub(crate) fn falloff_weights(&self, scene: &Scene, id: u64) -> Result<Option<Vec<f64>>> {
        let Some(field) = self.active_falloff() else {
            return Ok(None);
        };
        field.weights(scene, id).map(Some)
    }

    /// Action frame of a tool session, resolved against world-space positions
    /// for component editing (shared by the editor and viewport handles).
    pub fn extrude_frame(
        context: crate::ActionContext,
        tool: Tool,
        scene: &Scene,
    ) -> Result<crate::ActionFrame> {
        // Parent and Pivot centres come from the item hierarchy (ACS
        // table), before the world transform is baked in below.
        if scene.selection.mode != Mode::Item
            && let Some(frame) = crate::pivot_service::item_pivot_frame(context, scene)
        {
            return frame;
        }
        // Item-mode transforms use the item frame; every component tool
        // resolves its frame against world-space positions.
        let transform = tool.transforms_items();
        if scene.selection.mode == Mode::Item && tool != Tool::Extrude && !tool.is_smooth_shift() {
            // The handle sits where rigged items show (RG-01), not at
            // their own unconstrained place.
            let rigged = (!scene.animation.rig.pieces.is_empty())
                .then(|| crate::rig_eval::apply(scene, &crate::rig_eval::evaluate(scene), false))
                .flatten();
            let scene = rigged.as_ref().unwrap_or(scene);
            let frame = context.resolve(scene)?;
            // Several items: the handle at their shared pivot.
            if transform
                && let Some(targets) = scene.item_transform_targets()
                && !targets.is_empty()
            {
                return Ok(crate::ActionFrame {
                    center: scene.item_pivots(&targets)?[0],
                    ..frame
                });
            }
            return Ok(frame);
        }
        // Several active meshes: one frame over all their selections.
        if (transform || crate::multi_tools::tool_runs_on_each_mesh(tool))
            && let Some(targets) = scene.component_transform_meshes()
        {
            return context.resolve(&scene.combined_component_probe(&targets)?);
        }
        let item = scene.active().ok_or("Select a mesh")?;
        let world = scene.world_matrix(item.id)?;
        let mut basis_scene = scene.clone();
        let item = basis_scene.active_mut().unwrap();
        for v in &mut item.mesh.vertices {
            *v = world.transform_point3(*v);
        }
        item.transform = Default::default();
        item.parent = None;
        let frame = context.resolve(&basis_scene)?;
        let item = scene.active().ok_or("Select a mesh")?;
        // C-BORDER with no border edge is Selection's centre.
        let as_selection = context.center.uses_selection_box()
            || (context.center == crate::CenterMode::SelectionBorder
                && crate::selection_axes::selection_border_points(&item.mesh, &scene.selection)
                    .is_empty());
        if !as_selection {
            return Ok(frame);
        }
        // The selection's box is taken in the item's own space and its
        // middle placed in the world, so a turned item's box turns with
        // it (AC-rotate-Selection; AC-Q20 provisional).
        let local = crate::box_centre(
            crate::transform_vertices(scene)?
                .iter()
                .map(|&v| item.mesh.vertices[v as usize]),
        );
        Ok(crate::ActionFrame {
            center: world.transform_point3(local),
            ..frame
        })
    }
    /// BN-30/32: put Bend's centre (the action centre, `bend.base`) at a
    /// world point. The ring and the bend start there; the spine keeps its
    /// vector, so its end moves too (BH-Q05, provisional). Works before and
    /// after the bend has been applied.
    /// Move Bend's centre to `point`. With `refit` (a click in a view on
    /// that work plane, BN-24/BN-30b revised 2026-11-16) the spine is
    /// refitted over the target and the Axis goes to the plane's normal;
    /// without it (scripts, `SetActionCenter`) only the centre moves.
    pub(super) fn set_bend_center(&mut self, point: DVec3, refit: Option<u8>) -> Result<()> {
        if !point.is_finite() || point.abs().max_element() > 1e9 {
            return Err("The center must be a finite point".into());
        }
        let live = self.live.as_ref().ok_or("No active tool")?;
        let mut p = live.params.clone();
        if let Some(plane) = refit {
            // The target is the scene the tool started from, so the bend
            // in progress doesn't change the box; with Symmetry on, the
            // side the user works on (SYM3-12).
            let one_side =
                crate::symmetry_sides::primary_side_scene(&live.base, self.active_symmetry());
            let target = one_side.as_ref().unwrap_or(&live.base);
            let axes = live.action_frame.map_or(glam::DMat3::IDENTITY, |f| f.basis);
            let fitted = crate::BendOptions::fitted(target, point, plane, axes)?;
            p.bend.spine = fitted.spine;
            p.bend.axis = fitted.axis;
        }
        p.bend.base = point;
        let live = self.live.as_mut().ok_or("No active tool")?;
        if live.applied {
            self.preview(p, false)
        } else {
            live.params = p;
            if let Some(frame) = live.action_frame.as_mut() {
                frame.center = point;
            }
            Ok(())
        }
    }
}
