//! Starting a tool (`handle_activate`) and continuing one (`handle_continue`).
use super::*;

impl Editor {
    /// Start tool `t`, dropping or keeping the current task as its rules
    /// require. Selection inference runs first (pipe layer 0): a point or
    /// edge tool started from another mode starts on the converted set as
    /// if in its own mode, and the mode you're in stays.
    pub(crate) fn handle_activate(&mut self, t: Tool) -> Result<AfterCommand> {
        // An open tool keeps its own inference; the native start refuses.
        if self.session.has_active_task() {
            return self.handle_activate_native(t);
        }
        let saved = self.scene.selection.clone();
        if let Err(e) = self.infer_tool_selection(t) {
            self.scene.selection = saved;
            return Err(e);
        }
        let Some(target) = self.inferred_mode_for(t) else {
            return self.handle_activate_native(t);
        };
        self.scene.selection.mode = target;
        match self.handle_activate_native(t) {
            Ok(after) => {
                self.scene.selection.mode = saved.mode;
                if let Some(live) = self.session.live.as_mut() {
                    live.base.selection.mode = saved.mode;
                }
                Ok(after)
            }
            Err(e) => {
                self.scene.selection = saved;
                self.inference = None;
                Err(e)
            }
        }
    }
    pub(super) fn handle_activate_native(&mut self, t: Tool) -> Result<AfterCommand> {
        // RPT-08: a new tool session starts with nothing to repeat.
        self.session.last_transform_step = None;
        if self.spatial_falloff.is_some() && !t.card().layers.falloff {
            return Err(crate::tool_card::falloff_refusal());
        }
        // Another open tool was dropped first by `tool_switch`.
        if self.session.has_active_task() {
            return Err("Drop or Cancel the active tool first".into());
        }
        self.multi_extrude = None;
        // Several active meshes: the tool starts on each (multi_tools.rs).
        if let Some(targets) = Self::tool_mesh_targets(t, &self.scene) {
            self.activate_on_meshes(t, &targets)?;
            return Ok(AfterCommand::BumpRevision);
        }
        // A Backdrop is moved, rotated and scaled like any item (BD-41),
        // whatever the selection mode: its transform tools switch to Item
        // mode (Jason, 2026-11-04). It has no components to keep.
        // Cameras and lights too: they are edited in Item mode only (CL-1).
        if self.scene.active().is_some_and(|i| !i.is_mesh())
            && t.transforms_items()
            && self.scene.selection.mode != Mode::Item
        {
            self.dispatch(Command::Mode(Mode::Item))?;
        }
        let item = self
            .scene
            .active()
            .ok_or("Select a mesh; use New Mesh before creating a primitive")?;
        let item_transform = self.scene.selection.mode == Mode::Item && t.transforms_items();
        // RG-04: a rig piece drives the transform the tool would change;
        // dragging it would only edit the hidden own value.
        if item_transform {
            let groups: &[&str] = match t {
                Tool::Move => &["pos"],
                Tool::Rotate => &["rot"],
                Tool::Scale => &["scl"],
                _ => &["pos", "rot", "scl"],
            };
            for id in self.scene.selected_items() {
                for g in groups {
                    for a in ["X", "Y", "Z"] {
                        if let Some(p) = crate::rig::driven_by(&self.scene, id, &format!("{g}.{a}"))
                        {
                            return Err(format!("Driven by {}", crate::rig::label(&p.kind)));
                        }
                    }
                }
            }
        }
        // Other tools have no geometry to edit on a Backdrop.
        if item.backdrop.is_some() && !item_transform {
            return Err(BACKDROP_NO_GEOMETRY.into());
        }
        // Group Locators (GL-03) and texture items too.
        if !item.is_mesh() && !item_transform {
            return Err(LOCATOR_NO_GEOMETRY.into());
        }
        if !self.scene.shows(item) && !item_transform {
            return Err("Show Mesh with its eye before component editing".into());
        }
        // Component tools work on moved/rotated/scaled items: frames and
        // handles are resolved in world space, edits map back to local.
        // Primitives are drawn in world space and mapped into the item's
        // local space (`Scene::world_to_local`), which refuses a zero scale
        // with its own message.
        if t.is_primitive() {
            self.scene.world_to_local(item.id)?;
        } else if !item_transform && self.scene.world_matrix(item.id)?.determinant() == 0. {
            return Err("Component tools need an invertible item transform".into());
        }
        // Move/Rotate/Scale also work on loose points (FAL-O weights are
        // measured on point-only meshes).
        // (The per-drag-undo tools: the transforms and Tweak.)
        let loose_points = t.is_transform() && !item.mesh.vertices.is_empty();
        if !t.is_primitive()
            && t != Tool::VertexCreate
            // Contour adds its curves to an empty mesh too (MT13-10).
            && t != Tool::Contour
            && !t.is_duplicate()
            && !loose_points
            // Item-mode transforms move the item, not its geometry, so an
            // empty mesh (or one shaped only by its Mesh Stack) transforms
            // like any other item (GitHub issue #3).
            && !item_transform
            && item.is_mesh()
            && item.mesh.faces.is_empty()
        {
            return Err("The selected mesh is empty".into());
        }
        if matches!(t, Tool::Bevel | Tool::TwistBevel | Tool::Extrude) || t.is_smooth_shift() {
            if self.scene.selection.mode != Mode::Polygon {
                return Err("Select polygons explicitly in Polygon mode".into());
            }
            if t == Tool::Extrude {
                // PE-O-03: with nothing selected Extrude works on every
                // polygon.
                let faces = &self.scene.selection.faces;
                let all: Vec<u32>;
                let faces = if faces.is_empty() {
                    all = (0..item.mesh.faces.len() as u32).collect();
                    &all
                } else {
                    faces
                };
                item.mesh.validate_extrude_selection(faces)?;
            } else if t.is_bevel() && self.scene.selection.faces.is_empty() {
                // PB-O-01: with nothing selected Bevel works on every polygon.
                // Checked per polygon here; a closed mesh grouped as one
                // region is refused by the preview, where Group is known.
                let all: Vec<u32> = (0..item.mesh.faces.len() as u32).collect();
                item.mesh.validate_polygon_selection(&all, false)?;
            } else if !t.is_smooth_shift() {
                item.mesh
                    .validate_polygon_selection(&self.scene.selection.faces, true)?;
            }
        }
        if t == Tool::SymmetryTool && self.active_symmetry().axis.is_none() {
            return Err("Turn Symmetry on (X, Y or Z) to use the Symmetry Tool".into());
        }
        if t.card().family == crate::tool_card::Family::Deform
            && self.scene.selection.mode == Mode::Item
        {
            return Err(format!(
                "{} works on points, edges or polygons; pick a component mode first",
                t.label()
            ));
        }
        let params = Parameters {
            position: self.scene.workplane.center,
            // User-admitted Operandi sample settings, not factory defaults.
            radial: t.initial_radial(),
            delta: if t == Tool::Scale {
                DVec3::ONE
            } else {
                DVec3::ZERO
            },
            ..Default::default()
        };
        // Bend v2 (BN-02, BN-24 revised): the pivot is the action centre
        // and the spine is fitted from it over the target's box.
        let mut bend_frame = None;
        let params = if t == Tool::Bend {
            let plane = self.scene.workplane.plane;
            // With Symmetry on, the centre and spine come from the side the
            // user works on; the other side bends with the mirror image
            // (SYM3-12).
            let one_side =
                crate::symmetry_sides::primary_side_scene(&self.scene, self.active_symmetry());
            let target = one_side.as_ref().unwrap_or(&self.scene);
            let frame = Self::extrude_frame(self.action_context, t, target)?;
            bend_frame = Some(frame);
            let fitted = crate::BendOptions::fitted(target, frame.center, plane, frame.basis)?;
            // BN-47: later uses start from the last use's values. The
            // centre still comes from the action centre (BN-02; BH-Q13,
            // provisional), so the ring and the pivot agree.
            let bend = match *self.presets.get::<crate::bend::BendPreset>() {
                Some(last) => crate::BendOptions {
                    base: fitted.base,
                    ..last
                },
                None => fitted,
            };
            Parameters { bend, ..params }
        } else {
            params
        };
        if t.card().family == crate::tool_card::Family::Edge {
            // TA-06 (tool-activation v1): Edge Bevel with nothing selected
            // acts on every edge, and so does Edge Extrude (extrude-2026-11-05,
            // EE07). The other edge tools still need a selection.
            let all_edges;
            let edges = if matches!(t, Tool::EdgeBevel | Tool::EdgeExtrude)
                && self.scene.selection.mode == Mode::Edge
                && self.scene.selection.edges.is_empty()
            {
                all_edges = item.mesh.edges();
                &all_edges
            } else {
                &self.scene.selection.edges
            };
            if self.scene.selection.mode != Mode::Edge || edges.is_empty() {
                return Err(format!("{} needs selected edges in Edge mode", t.label()));
            }
            // Probe eligibility with a tiny change; the session starts at zero.
            if t == Tool::EdgeExtend {
                // Any axis not parallel to the edges proves eligibility
                // (a vertical edge cannot extend along Y).
                let probes = [DVec3::Y, DVec3::X, DVec3::Z]
                    .map(|d| crate::edge_extend_geometry(&item.mesh, edges, d * 1e-6, 1));
                if probes.iter().all(|r| r.is_err()) {
                    probes.into_iter().next().unwrap()?;
                }
            } else if t == Tool::EdgeSlide {
                crate::edge_slide_geometry(&item.mesh, edges, 0.)?;
            } else if t == Tool::EdgeCrease {
                crate::edge_crease_geometry(&item.mesh, edges, 0., false)?;
            } else if matches!(t, Tool::EdgeBevel | Tool::EdgeChamfer) {
                crate::edge_bevel_geometry(
                    &item.mesh,
                    edges,
                    1e-6,
                    1,
                    100.,
                    t == Tool::EdgeChamfer,
                )?;
            } else if t == Tool::EdgeRelax {
                crate::edge_relax_geometry(&item.mesh, edges, 0.)?;
            } else {
                crate::edge_extrude_geometry(&item.mesh, edges, 1e-6, 1e-6)?;
            }
        }
        if t.card().family == crate::tool_card::Family::Vertex {
            // Vertex Extrude with nothing selected acts on every point
            // (extrude-2026-11-05, VE08).
            let all_points: Vec<u32>;
            let points = if t == Tool::VertexExtrude
                && self.scene.selection.mode == Mode::Vertex
                && self.scene.selection.vertices.is_empty()
            {
                all_points = (0..item.mesh.vertices.len() as u32).collect();
                &all_points
            } else {
                &self.scene.selection.vertices
            };
            if self.scene.selection.mode != Mode::Vertex || points.is_empty() {
                return Err(format!(
                    "{} needs selected points in Vertex mode",
                    t.label()
                ));
            }
            // Probe eligibility with a tiny change; the session starts at zero.
            match t {
                Tool::VertexBevel => {
                    crate::vertex_bevel_geometry(&item.mesh, points, 1e-9)?;
                }
                Tool::VertexExtrude => {
                    crate::vertex_extrude_geometry(&item.mesh, points, 0., 1e-9)?;
                }
                Tool::VertexMerge => {
                    crate::vertex_merge_geometry(&item.mesh, points, 0.)?;
                }
                Tool::VertexTear => {
                    crate::vertex_tear_geometry(
                        &item.mesh,
                        points,
                        &self.scene.selection.faces,
                        DVec3::X * 1e-9,
                    )?;
                }
                _ => {
                    crate::vertex_slide_geometry(&item.mesh, points, DVec3::X, 1e-9)?;
                }
            }
        }
        if t == Tool::VertexCreate && self.scene.selection.mode != Mode::Vertex {
            return Err("Create works in Vertex mode".into());
        }
        // The path tools start only in the modes their card names; the
        // copier buttons (DX-11/30) work on items in any mode.
        if matches!(t, Tool::CurveExtrude | Tool::CurveClone)
            && !t.card().modes.contains(self.scene.selection.mode)
            && !self.copier_row_open(t)
        {
            return Err(format!(
                "{} works in {}",
                t.label(),
                t.card().modes.describe()
            ));
        }
        // The action-centre layer: a tool that needs the neutral context
        // refuses any other, and the modes its card leaves out.
        let card = t.card();
        if card.layers.action_centre == crate::tool_card::ActionCentre::Neutral
            && (!card.modes.contains(self.scene.selection.mode)
                || self.action_context != crate::ActionContext::default()
                || self.spatial_falloff.is_some())
        {
            return Err(format!(
                "{} requires {} and neutral context",
                t.label(),
                card.modes.describe()
            ));
        }
        if t == Tool::Chamfer {
            let probe = crate::ChamferParameters {
                offset: 1e-6,
                ..*self.presets.get::<crate::ChamferPreset>()
            };
            let selected = &self.scene.selection.faces;
            if let Err(error) = crate::chamfer_geometry(&item.mesh, selected, probe) {
                // A chamfer of several segments starts the tool too; its
                // Source Segments is set next (CE12-CE15, rule 6).
                let across = crate::ChamferParameters {
                    source_segments: (selected.len() as u32).max(1),
                    ..probe
                };
                if across.source_segments == probe.source_segments
                    || crate::chamfer_geometry(&item.mesh, selected, across).is_err()
                {
                    return Err(error);
                }
            }
        }
        if matches!(t, Tool::Inset | Tool::Shift) {
            if self.scene.selection.mode != Mode::Polygon {
                return Err(format!("{} works in Polygon mode", t.label()));
            }
            // Probe eligibility; nothing selected means every polygon (IN10).
            crate::inset_geometry(&item.mesh, &self.scene.selection.faces, 0.)?;
        }
        // Poly Haul: Tear and Slide pick points, so Vertex mode works too (C8).
        if t == Tool::PolyHaul && !self.scene.selection.faces.is_empty() {
            crate::poly_haul_geometry(&item.mesh, &self.scene.selection.faces, 0.)?;
        }
        if t == Tool::SketchExtrude {
            crate::validate_sketch_target(&item.mesh, &self.scene.selection.faces)?;
        }
        // Edge Weight starts at the first selected edge's current weight.
        let params = if t == Tool::EdgeCrease {
            let first = self.scene.selection.edges.first().copied();
            let weight = first.and_then(|(a, b)| {
                let (a, b) = (a.min(b), a.max(b));
                item.mesh
                    .attributes
                    .creases
                    .iter()
                    .find(|c| (c.a, c.b) == (a, b))
                    .map(|c| c.weight)
            });
            // With Additive the field is the amount added, from 0.
            Parameters {
                shift: if params.edge_weight_additive {
                    0.
                } else {
                    weight.unwrap_or(0.)
                },
                ..params
            }
        } else {
            params
        };
        // Edge and Vertex Extrude start with a Width, so dragging the Extrude
        // arrow alone extrudes (Width 0 changes nothing). Provisional
        // (EXW-Q01).
        let params = match t {
            Tool::EdgeExtrude => Parameters {
                inset: crate::edge_extrude_start_width(&item.mesh, &self.scene.selection.edges)
                    .unwrap_or(0.),
                ..params
            },
            Tool::VertexExtrude => Parameters {
                inset: crate::vertex_extrude_start_width(
                    &item.mesh,
                    &self.scene.selection.vertices,
                )
                .unwrap_or(0.),
                ..params
            },
            _ => params,
        };
        // Vertex Slide's rail runs from the first point to its first
        // neighbour, in world space.
        let params = if t == Tool::VertexSlide {
            let rail = crate::vertex_slide_direction(&item.mesh, &self.scene.selection.vertices)
                .unwrap_or(DVec3::X);
            let world = self.scene.world_matrix(item.id)?.transform_vector3(rail);
            Parameters {
                delta: world.normalize_or_zero(),
                ..params
            }
        } else {
            params
        };
        let params = if t == Tool::Chamfer {
            Parameters {
                chamfer: *self.presets.get::<crate::ChamferPreset>(),
                ..params
            }
        } else {
            params
        };
        let params = if matches!(t, Tool::EdgeBevel | Tool::EdgeChamfer) {
            let preset = if t == Tool::EdgeBevel {
                self.presets.get::<crate::EdgeBevelToolPreset>()
            } else {
                self.presets.get::<crate::EdgeChamferToolPreset>()
            };
            Parameters {
                chamfer: crate::ChamferParameters {
                    segments: preset.segments,
                    depth: preset.depth,
                    ..params.chamfer
                },
                edge_bevel: preset.options.clone(),
                ..params
            }
        } else {
            params
        };
        let params = if t == Tool::PolyHaul {
            Parameters {
                poly_haul_offset: *self.presets.get::<crate::PolyHaulPreset>(),
                ..params
            }
        } else {
            params
        };
        // Merge Tool: Distance 0.001 after Reset, else the last use's value
        // (VTO-T04).
        let params = if t == Tool::VertexMerge {
            Parameters {
                shift: self
                    .presets
                    .get::<crate::VertexMergePreset>()
                    .unwrap_or(MERGE_TOOL_DISTANCE),
                ..params
            }
        } else {
            params
        };
        let params = if t == Tool::SketchExtrude {
            Parameters {
                sketch_precision: self
                    .presets
                    .get::<crate::SketchPrecisionPreset>()
                    .unwrap_or(30.),
                ..params
            }
        } else {
            params
        };
        let params = if t == Tool::Extrude {
            self.presets.get::<crate::ExtrudePreset>().clone()
        } else {
            params
        };
        let params = if t.is_duplicate() {
            let mut duplicate = self
                .presets
                .get::<crate::duplicate::DuplicatePreset>()
                .options_only();
            // A copier comes only from the open preset row (DX-20/30), so a
            // plain button never inherits the last row's.
            duplicate.copier = self
                .session
                .preset
                .as_ref()
                .filter(|p| p.tool == t)
                .and_then(|p| p.row().actor.copier);
            Parameters {
                duplicate: Box::new(duplicate),
                ..params
            }
        } else {
            params
        };
        let params = if t.is_slice() {
            Parameters {
                slice: crate::slice::start(
                    t,
                    &self.scene,
                    self.presets.get::<crate::slice::SlicePreset>(),
                ),
                ..params
            }
        } else {
            params
        };
        let params = if t.is_smooth_shift() {
            Parameters {
                smooth_shift: if t == Tool::Thicken {
                    self.presets
                        .get::<crate::ThickenPreset>()
                        .unwrap_or_else(crate::SmoothShiftParameters::thicken_initial)
                } else {
                    *self.presets.get::<crate::SmoothShiftPreset>()
                },
                ..params
            }
        } else {
            params
        };
        // Jason decision 2026-11-01 (BOX-Q09/SP-Q08/CYL-Q11): primitives
        // start with the segment settings they had last time, like Cone and
        // Ellipsoid; Reset restores the starting values. Size and position
        // still come from the drag.
        let params = match t {
            Tool::Box | Tool::Sphere | Tool::Cylinder => Parameters {
                make_uvs: !self.primitive_uvs_off[primitive_slot(t)],
                ..params
            },
            _ => params,
        };
        let params = match t {
            Tool::Box => Parameters {
                segments: self.presets.get::<crate::CubePreset>().segments,
                ..params
            },
            Tool::Sphere | Tool::Cylinder => {
                let mut params = params;
                if let (Some(radial), Some([sides, segments])) =
                    (params.radial.as_mut(), self.unit_resolution(t))
                {
                    radial.sides = sides;
                    radial.segments = segments;
                }
                if let (Tool::Cylinder, Some((caps, axis))) = (t, self.cylinder_shape) {
                    params.cylinder_caps = caps;
                    params.radial_axis = axis;
                }
                if let (Tool::Sphere, Some((axis, level))) = (t, self.sphere_shape) {
                    params.radial_axis = axis;
                    params.sphere_level = level;
                }
                params
            }
            _ => params,
        };
        self.session.live = Some(Live {
            identity_base: Some(self.identities.current.clone()),
            identity_preview: None,
            tool: t,
            base: self.scene.clone(),
            params,
            applied: false,
            preview_error: None,
            pivots: Default::default(),
            // The action-centre layer: tools that work about a frame
            // capture it now.
            placed_centre: None,
            action_frame: if let Some(frame) = bend_frame {
                Some(frame)
            } else if t.card().layers.action_centre == crate::tool_card::ActionCentre::Frame {
                Some(self.pivot_frame(&self.scene, t)?)
            } else {
                None
            },
        });
        // Smooth works straight away at its starting Strength; Drag and the
        // fields then adjust that one result (deform.rs). So do the Deform
        // finish tools (deform_finish.rs).
        if t.shows_result_at_once() {
            let p = self
                .session
                .live
                .as_ref()
                .ok_or("No active tool")?
                .params
                .clone();
            self.preview(p, false)?;
        }
        // Loop Slice with a ring from the selection shows its slice at once
        // (part of the activation step). A ring it can't cut is dropped, and
        // the tool waits for a click on an edge.
        if t == Tool::LoopSlice
            && let Some(p) = self.session.live.as_ref().map(|l| l.params.clone())
            && !p.slice.loop_slice.rings.is_empty()
            && self.preview(p, false).is_err()
            && let Some(live) = self.session.live.as_mut()
        {
            live.params.slice.loop_slice.rings.clear();
        }
        Ok(AfterCommand::BumpRevision)
    }

    /// Continue the live tool on a new stage (e.g. Shift-click on caps).
    pub(super) fn handle_continue(&mut self) -> Result<()> {
        let live = self
            .session
            .live
            .as_ref()
            .ok_or("Activate Bevel or Extrude first")?;
        if !live.tool.card().continues {
            return Err("Continuation is available for Bevel, Extrude and Edge Extend only".into());
        }
        if live.tool == Tool::EdgeExtend {
            return self.continue_edge_extend();
        }
        if let Some(error) = &live.preview_error {
            return Err(format!(
                "Correct the current stage before continuation: {error}"
            ));
        }
        self.scene.validate()?;
        let mut next = live.clone();
        if live.tool == Tool::Extrude
            && live.applied
            && let Some(multi) = self.multi_extrude.as_ref()
        {
            // Several meshes: each continues from its own moving caps.
            next.base = self.scene.clone();
            for (&id, caps) in &multi.moved {
                next.base.selection.set_faces_of(id, caps.clone());
            }
        } else if live.tool == Tool::Extrude && live.applied && !self.extrude_moved_caps.is_empty()
        {
            next.base = self.scene.clone();
            next.base.selection.faces = self.extrude_moved_caps.clone();
        } else {
            next.base = self.scene.clone();
        }
        if next.tool.is_smooth_shift() {
            next.params.smooth_shift.offset = 0.;
            next.params.smooth_shift.scale = 1.;
        }
        next.params.shift = 0.;
        next.params.inset = 0.;
        // Twist Bevel's fresh stage keeps its shape settings (twist, bend,
        // profile); only its size starts again.
        next.params.twist_bevel.shift = 0.;
        next.params.twist_bevel.inset = 0.;
        next.params.extent = DVec3::ZERO;
        next.applied = false;
        // DS-34: a Duplicate tool starts a new placement on the result.
        next.params.duplicate.placed = false;
        next.action_frame = if next.tool.is_duplicate() {
            None
        } else {
            Some(self.pivot_frame(&next.base, next.tool)?)
        };
        next.placed_centre = None;
        // Inside Bevel's tool session the finished bevel stays a step of the
        // tool's own (`tweak_before`); Drop records the session from its
        // start (`tweak_session_base`).
        let tool_step = live.tool.is_bevel() && self.tweak_session().is_some();
        if live.base != self.scene && !tool_step {
            self.identities.record_before = live.identity_base.clone();
            let before = live.base.clone();
            let label = live.tool.label().to_string();
            self.record(label, before);
        }
        next.identity_base = Some(self.identities.current.clone());
        next.identity_preview = None;
        self.scene.selection = next.base.selection.clone();
        self.session.live = Some(next);
        Ok(())
    }

    /// Edge Extend's Shift+click: keep this extension and start a new one
    /// from the edges it made (selected by the preview), with the same
    /// values, shown at once (spec: "extend it back once, Shift-click,
    /// extend it back twice"). The action frame is resolved again on the
    /// new edges, so the centre follows them.
    pub(super) fn continue_edge_extend(&mut self) -> Result<()> {
        let live = self
            .session
            .live
            .as_ref()
            .ok_or("Activate Edge Extend first")?;
        if !live.applied || live.base == self.scene {
            return Err("Extend the edges first; Shift+click then extends again".into());
        }
        let mut next = live.clone();
        next.base = self.scene.clone();
        next.applied = false;
        next.action_frame = Some(self.pivot_frame(&next.base, Tool::EdgeExtend)?);
        let tool_step = self.tweak_session().is_some();
        if !tool_step {
            self.identities.record_before = live.identity_base.clone();
            let before = live.base.clone();
            self.record("Extend".to_string(), before);
        }
        next.identity_base = Some(self.identities.current.clone());
        next.identity_preview = None;
        let params = next.params.clone();
        self.session.live = Some(next);
        self.preview(params, false)
    }
}
