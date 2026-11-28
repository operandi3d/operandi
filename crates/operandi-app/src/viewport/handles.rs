//! The live tool's handles: which ones it shows, where they sit and point
//! (their frames), and which one the pointer is over.
use super::*;

impl Operandi {
    /// Axis Rotate (TC-30): the one ring it shows, the work plane's normal
    /// (plane 0 is XZ, 1 YZ, 2 XY).
    pub(crate) fn axis_rotate_axis(&self) -> Option<usize> {
        let p = self.editor.session.preset.as_ref()?;
        if !p.row().actor.single_ring {
            return None;
        }
        Some([1, 0, 2][self.actual_plane().min(2) as usize])
    }
    pub(crate) fn handles(&self, r: Rect) -> Vec<(usize, Pos2, Pos2, String)> {
        let all = self.all_handles(r);
        match self.axis_rotate_axis() {
            Some(axis) => all.into_iter().filter(|h| h.0 == axis).collect(),
            None => all,
        }
    }
    pub(super) fn all_handles(&self, r: Rect) -> Vec<(usize, Pos2, Pos2, String)> {
        self.handle_origin.set(None);
        let Some(l) = &self.editor.session.live else {
            // #37: a drawn Cone, Ellipsoid, Torus, Capsule or Text.
            return self.session_handles(r);
        };
        // CS-01: an armed tool draws nothing until its first click.
        if self.editor.is_armed() {
            return Vec::new();
        }
        // CP-A02: transform tools show nothing on a locked item.
        if self.editor.scene.selection.mode == Mode::Item
            && l.tool.transforms_items()
            && self
                .editor
                .scene
                .active()
                .is_some_and(operandi_core::Item::is_locked)
        {
            return Vec::new();
        }
        if !self.handle_preferences.selected_shown() {
            return Vec::new();
        }
        // On-screen handle lengths follow Handle Scale: Standard, and with
        // original-compatible A1 the zoom at the handle's centre.
        let s_at = |center: DVec3| {
            let zoom = self.handle_zoom_at(center);
            self.handle_zoom.set(zoom);
            self.handle_preferences.scale * zoom
        };
        // The handle layout is on the tool's card. Tools without handles
        // grab, paint, draw or click (Tweak's click-to-show handle is TWK-3).
        let card = l.tool.card().handles;
        if card.kind == HandleKind::None {
            return Vec::new();
        }
        if card.kind == HandleKind::RadialBounds {
            let Some(radial) = l.params.radial else {
                return Vec::new();
            };
            if !l.applied && radial.radius.min_element() > 0. {
                return Vec::new();
            }
            let mut out = Vec::new();
            // N21 (Jason, 2026-11-17; provisional, Q-DC-04): once the shape
            // is drawn, three move arrows at its centre position it, as the
            // Cube's do; the squares still shape it.
            if radial.radius.max_element() > 0.
                && let Some((center, _)) = self.camera.project(l.params.position, r)
            {
                self.handle_origin.set(Some(l.params.position));
                let s = s_at(l.params.position);
                for axis in 0..3 {
                    let direction = [DVec3::X, DVec3::Y, DVec3::Z][axis];
                    if let Some((_, arrow)) =
                        self.handle_arrow(l.params.position, direction, 72. * s, r)
                    {
                        out.push((axis, center, center + arrow, ["X", "Y", "Z"][axis].into()));
                    }
                }
            }
            for axis in 0..3 {
                let direction = [DVec3::X, DVec3::Y, DVec3::Z][axis];
                let (Some((center, _)), Some((unit, _))) = (
                    self.camera.project(l.params.position, r),
                    self.camera.project(l.params.position + direction, r),
                ) else {
                    continue;
                };
                if center.distance(unit) < 1. {
                    continue;
                }
                for maximum in [false, true] {
                    let sign = if maximum { 1. } else { -1. };
                    let point = l.params.position + direction * radial.radius[axis] * sign;
                    if let Some((mut point, _)) = self.camera.project(point, r) {
                        if radial.radius[axis] == 0. {
                            point = center
                                + (unit - center).normalized()
                                    * 22.
                                    * s_at(l.params.position)
                                    * sign as f32;
                        }
                        out.push((
                            3 + axis * 2 + usize::from(maximum),
                            point,
                            point,
                            format!(
                                "{}{} {}",
                                if maximum { "+" } else { "−" },
                                ["X", "Y", "Z"][axis],
                                if l.tool == Tool::Sphere {
                                    "radius"
                                } else {
                                    "bound"
                                }
                            ),
                        ));
                    }
                }
            }
            return out;
        }
        if l.tool == Tool::EdgeExtend {
            return self.edge_extend_handles(l, r);
        }
        if card.kind == HandleKind::Spine {
            return self.twist_bevel_handles(l, r);
        }
        if l.tool == Tool::Transform {
            let center = self.transform_handle_center();
            let Some((origin, _)) = self.camera.project(center, r) else {
                return Vec::new();
            };
            let s = s_at(center);
            let mut out = Vec::new();
            for axis in 0..3 {
                let direction = self.handle_axis(axis);
                if let Some((_, arrow)) = self.handle_arrow(center, direction, 72. * s, r) {
                    out.push((
                        axis,
                        origin,
                        origin + arrow,
                        format!("Move {}", ["X", "Y", "Z"][axis]),
                    ));
                    out.push((
                        6 + axis,
                        origin + arrow * (30. / 72.),
                        origin + arrow * (44. / 72.),
                        format!("Scale {}", ["X", "Y", "Z"][axis]),
                    ));
                }
                if let Some(end) = self.rotation_ring(axis, r).last() {
                    out.push((
                        3 + axis,
                        origin,
                        *end,
                        format!("Rotate {}", ["X", "Y", "Z"][axis]),
                    ));
                }
            }
            // PH-32: the move part's plane circles.
            out.extend(self.plane_handles(l.tool, r));
            out.push((FREE_HANDLE, origin, origin, "Free move".into()));
            return out;
        }
        if l.tool == Tool::Box && !l.applied {
            return Vec::new();
        }
        if l.tool.is_smooth_shift() && !l.applied {
            return Vec::new();
        }
        // User correction 2026-10-30: Extrude shows no gizmo until the first
        // viewport click starts the extrusion.
        if l.tool == Tool::Extrude && !l.applied && !self.editor.i2_active() {
            return Vec::new();
        }
        let item = self.editor.scene.active();
        // The pivot service's handle under Symmetry (ACS-01): the polygon
        // stage follows the primary side's live caps.
        let polygon_symmetry_centre = self
            .editor
            .symmetry_handle_centre()
            .filter(|_| card.anchor == HandleAnchor::PolygonStage);
        let origin = match card.anchor {
            // Edge Bevel's handles ride on the bevel as it grows (2026-11-23),
            // as Polygon Bevel's ride on the moved cap: once the mesh has
            // changed, the centre of the live selection (the new strips'
            // edges). Before that, the selected edges themselves.
            HandleAnchor::Edge
                if l.tool == Tool::EdgeBevel
                    && (l.params.chamfer.offset > 0. || l.params.edge_bevel.support > 0.) =>
            {
                self.edge_bevel_centre()
                    .or_else(|| self.edge_handle_frame().map(|f| f.0))
                    .unwrap_or(DVec3::ZERO)
            }
            HandleAnchor::Edge => self.edge_handle_frame().map(|f| f.0).unwrap_or(DVec3::ZERO),
            HandleAnchor::EdgeRail => self.edge_slide_frame().map(|f| f.0).unwrap_or(DVec3::ZERO),
            HandleAnchor::Vertex => self.vertex_frame().map(|f| f.0).unwrap_or(DVec3::ZERO),
            HandleAnchor::None => unreachable!("{} has no handles", l.tool.label()),
            HandleAnchor::ChamferStrip => l
                .base
                .active()
                .and_then(|item| {
                    operandi_core::chamfer_strips(&item.mesh, &l.base.selection.faces)
                        .first()
                        .filter(|&&face| (face as usize) < item.mesh.faces.len())
                        .map(|&face| item.mesh.center(face as usize))
                })
                .unwrap_or(DVec3::ZERO),
            HandleAnchor::Position => l.params.position,
            HandleAnchor::PolygonStage => polygon_symmetry_centre.unwrap_or_else(|| {
                let frame = l.action_frame;
                if self.editor.action_context.center == CenterMode::Origin {
                    frame.map(|f| f.center).unwrap_or(DVec3::ZERO)
                } else if self.editor.action_context.center == CenterMode::Selection
                    || (l.tool == Tool::Bevel && self.editor.scene.selection.faces.len() > 1)
                {
                    // Follow the current cap translation/normal field while the
                    // stage orientation stays captured from its immutable base.
                    // Default multi-face Bevel uses the whole selection, not
                    // the first cap. An explicit Origin still takes precedence.
                    // World-space resolve, so moved items place the handle right.
                    operandi_core::Editor::extrude_frame(
                        ActionContext {
                            center: CenterMode::Selection,
                            axis: AxisOrientation::World,
                        },
                        l.tool,
                        &self.editor.scene,
                    )
                    .map(|f| f.center)
                    .unwrap_or(DVec3::ZERO)
                } else {
                    item.and_then(|i| {
                        self.editor
                            .scene
                            .selection
                            .faces
                            .first()
                            .map(|&f| i.mesh.center(f as usize))
                    })
                    .unwrap_or(DVec3::ZERO)
                }
            }),
            // The first selected polygon of the drag's base (polygon 0 when
            // nothing is selected and every polygon is affected).
            HandleAnchor::FirstPolygon => l
                .base
                .active()
                .and_then(|i| {
                    let f = l.base.selection.faces.first().copied().unwrap_or(0) as usize;
                    (f < i.mesh.faces.len()).then(|| i.mesh.center(f))
                })
                .unwrap_or(DVec3::ZERO),
            HandleAnchor::ActionCentre => self.transform_handle_center(),
        };
        // Face-center origins above are item-local; map them to world.
        let local_origin = card.anchor.item_local()
            || (l.tool == Tool::Bevel
                && self.editor.action_context.center != CenterMode::Origin
                && self.editor.action_context.center != CenterMode::Selection
                && self.editor.scene.selection.faces.len() <= 1);
        let origin = if polygon_symmetry_centre.is_none()
            && (local_origin
                || (l.tool.is_smooth_shift()
                    && self.editor.action_context.center != CenterMode::Origin))
        {
            item.and_then(|i| self.editor.scene.world_matrix(i.id).ok())
                .map(|m| m.transform_point3(origin))
                .unwrap_or(origin)
        } else {
            origin
        };
        // With Symmetry on, the handle sits on the primary pivot (the pivot
        // service, ACS-01/03), not halfway between the two halves.
        let origin = if matches!(
            card.anchor,
            HandleAnchor::Edge | HandleAnchor::Vertex | HandleAnchor::EdgeRail
        ) && !l.tool.transforms_items()
        {
            self.editor.symmetry_handle_centre().unwrap_or(origin)
        } else {
            origin
        };
        let s = s_at(origin);
        self.handle_origin.set(Some(origin));
        let mut out = Vec::new();
        for axis in 0..card.kind.axis_count() {
            if l.tool == Tool::Rotate {
                let ring = self.rotation_ring(axis, r);
                if let Some((o, _)) = self.camera.project(self.transform_handle_center(), r)
                    && let Some(end) = ring.last()
                {
                    out.push((axis, o, *end, ["X", "Y", "Z"][axis].into()));
                }
                continue;
            }
            let direction = self.handle_axis(axis);
            if let Some((a, arrow)) = self.handle_arrow(origin, direction, 72. * s, r) {
                // ACS-Q-HANDLE1: the box-centre tools' arms start about
                // 25 px out from the centre. The Bevels' arrows start on the
                // selection (2026-11-23): set off, an edge's arrows floated
                // beside the edge and read as pointing the wrong way.
                // The gap shortens with the arrow, as part of the same 3D line.
                let gap = if operandi_core::handle_pick(l.tool)
                    == operandi_core::HandlePick::BoxCentre
                    && card.anchor != HandleAnchor::PolygonStage
                    && card.axes != HandleAxes::SelectionNormal
                {
                    25. / 72.
                } else {
                    0.
                };
                let a = a + arrow * gap;
                out.push((
                    axis,
                    a,
                    a + arrow,
                    if l.tool == Tool::Chamfer {
                        "Offset".into()
                    } else if l.tool.is_smooth_shift() {
                        if axis == 0 { "Offset" } else { "Scale" }.into()
                    } else if l.tool == Tool::Bevel {
                        if axis == 0 { "Shift" } else { "Inset" }.into()
                    } else if l.tool == Tool::EdgeExtrude {
                        if axis == 0 { "Extrude" } else { "Width" }.into()
                    } else if l.tool == Tool::EdgeSlide {
                        "Slide".into()
                    } else if l.tool == Tool::EdgeCrease {
                        "Weight".into()
                    } else if l.tool == Tool::EdgeBevel {
                        if axis == 1 {
                            "Mitering Offset"
                        } else if l.params.edge_bevel.mode == operandi_core::EdgeBevelMode::Width {
                            "Width"
                        } else {
                            "Inset"
                        }
                        .into()
                    } else if matches!(l.tool, Tool::EdgeChamfer | Tool::EdgeRelax) {
                        "Offset".into()
                    } else if l.tool == Tool::VertexBevel {
                        "Inset".into()
                    } else if l.tool == Tool::VertexExtrude {
                        if axis == 0 { "Extrude" } else { "Width" }.into()
                    } else if l.tool == Tool::VertexMerge {
                        "Distance".into()
                    } else if l.tool == Tool::VertexSlide {
                        "Slide".into()
                    } else if l.tool == Tool::Inset {
                        "Inset".into()
                    } else if l.tool == Tool::Shift {
                        "Shift".into()
                    } else {
                        ["X", "Y", "Z"][axis].into()
                    },
                ));
            }
        }
        if matches!(l.tool, Tool::Move | Tool::Rotate)
            && let Some((center, _)) = self.camera.project(self.transform_handle_center(), r)
        {
            out.push((
                FREE_HANDLE,
                center,
                center,
                if l.tool == Tool::Move {
                    "Free move"
                } else {
                    "Free rotate"
                }
                .into(),
            ));
        }
        // The Scale tool's center handle (where the three axis
        // handles meet) scales the selection uniformly.
        if l.tool == Tool::Scale
            && let Some((center, _)) = self.camera.project(origin, r)
        {
            out.push((SCALE_UNIFORM_HANDLE, center, center, "Uniform".into()));
        }
        // PH-30/31: Move's and Scale's plane circles.
        out.extend(self.plane_handles(l.tool, r));
        if l.tool == Tool::Box {
            for axis in 0..3 {
                for maximum in [false, true] {
                    // At zero depth both bounds coincide. One positive-bound
                    // square owns the center hit so upward/outward hauling can
                    // grow a solid instead of selecting the crossed min bound.
                    if l.params.size[axis] == 0. && !maximum {
                        continue;
                    }
                    let mut point = l.params.position;
                    point[axis] += l.params.size[axis] * if maximum { 0.5 } else { -0.5 };
                    if let Some((point, _)) = self.camera.project(point, r) {
                        out.push((
                            3 + axis * 2 + usize::from(maximum),
                            point,
                            point,
                            format!(
                                "{}{}",
                                if maximum { "+" } else { "−" },
                                ["X", "Y", "Z"][axis]
                            ),
                        ));
                    }
                }
            }
        }
        out
    }
    pub(crate) fn handle_axis(&self, axis: usize) -> DVec3 {
        // Frame-based tools point their handles along their card's frame.
        if let Some(l) = self.editor.session.live.as_ref() {
            match l.tool.card().handles.axes {
                HandleAxes::Tool => {}
                HandleAxes::VertexRail | HandleAxes::VertexNormal => {
                    let Some((_, normal, rail, _)) = self.vertex_frame() else {
                        return DVec3::X;
                    };
                    return if l.tool.card().handles.axes == HandleAxes::VertexNormal && axis == 0 {
                        normal
                    } else if l.tool == Tool::VertexSlide {
                        l.params.delta.normalize_or(rail)
                    } else {
                        rail
                    };
                }
                HandleAxes::EdgeRail => {
                    return self.edge_slide_frame().map(|f| f.1).unwrap_or(DVec3::X);
                }
                HandleAxes::EdgeSide => {
                    return self.edge_handle_frame().map(|f| f.2).unwrap_or(DVec3::X);
                }
                HandleAxes::EdgeNormal => {
                    return self
                        .edge_handle_frame()
                        .map(|(_, up, side)| if axis == 0 { up } else { side })
                        .unwrap_or(DVec3::Z);
                }
                // The Bevels: the selection's average normal; the handle
                // element's own normal when the normals cancel out.
                HandleAxes::SelectionNormal => {
                    let points = l.tool.card().handles.anchor == HandleAnchor::Vertex;
                    let of = if points {
                        operandi_core::NormalOf::Points
                    } else {
                        operandi_core::NormalOf::Edges
                    };
                    let normal = self.bevel_normal(of).unwrap_or_else(|| {
                        if points {
                            self.vertex_frame().map_or(DVec3::Z, |f| f.1)
                        } else {
                            self.edge_handle_frame().map_or(DVec3::Z, |f| f.1)
                        }
                    });
                    if axis == 0 || points {
                        return normal;
                    }
                    // Edge Bevel's Support Edges handle: into the first
                    // edge's polygon, square to the bevel arrow.
                    let side = self.edge_handle_frame().map_or(DVec3::X, |f| f.2);
                    return side.reject_from(normal).normalize_or(side);
                }
            }
        }
        if axis > 2
            && self
                .editor
                .session
                .live
                .as_ref()
                .is_some_and(|l| l.tool == Tool::Scale)
        {
            return DVec3::ZERO;
        }
        let Some(l) = &self.editor.session.live else {
            return DVec3::ZERO;
        };
        let a = [DVec3::X, DVec3::Y, DVec3::Z][axis];
        // Chamfer Edit's single (blue) widget points along the strip normal.
        if l.tool == Tool::Chamfer
            && let Some(i) = l.base.active()
            && let Some(&f) = operandi_core::chamfer_strips(&i.mesh, &l.base.selection.faces)
                .first()
                .filter(|&&f| (f as usize) < i.mesh.faces.len())
        {
            let direction = i.mesh.normal(f as usize);
            return l
                .base
                .world_matrix(i.id)
                .map(|m| m.transform_vector3(direction).normalize_or_zero())
                .unwrap_or(direction);
        }
        // Inset: Bevel's Inset direction (centre away from the first corner);
        // Shift: the polygon's normal. Frozen from the drag's base.
        if matches!(l.tool, Tool::Inset | Tool::Shift)
            && let Some(i) = l.base.active()
        {
            let fi = l.base.selection.faces.first().copied().unwrap_or(0) as usize;
            let Some(f) = i.mesh.faces.get(fi) else {
                return DVec3::Z;
            };
            let direction = if l.tool == Tool::Shift {
                i.mesh.normal(fi)
            } else {
                (i.mesh.center(fi) - i.mesh.vertices[f.vertices[0] as usize]).normalize_or_zero()
            };
            return l
                .base
                .world_matrix(i.id)
                .map(|m| m.transform_vector3(direction).normalize_or_zero())
                .unwrap_or(direction);
        }
        if l.tool.is_smooth_shift()
            && let Some(i) = l.base.active()
        {
            let fi = l.base.selection.faces.first().copied().unwrap_or(0) as usize;
            let f = &i.mesh.faces[fi];
            let direction = if axis == 0 {
                i.mesh.normal(fi)
            } else {
                (i.mesh.center(fi) - i.mesh.vertices[f.vertices[0] as usize]).normalize_or_zero()
            };
            return l
                .base
                .world_matrix(i.id)
                .map(|m| m.transform_vector3(direction).normalize_or_zero())
                .unwrap_or(direction);
        }
        if l.tool == Tool::Bevel {
            if self.editor.action_context.axis == AxisOrientation::Selection
                && let Some(frame) = l.action_frame
            {
                // The Shift arrow follows the selection's average normal,
                // as without Selection axes; the Selection frame's Z when
                // the normals cancel (A-SEL puts a flat selection's normal
                // on Z; it was on Y before AC-RULES v6). The Inset arrow is
                // the frame's X, the selection's long direction, laid
                // square to it.
                let normal = self
                    .bevel_normal(operandi_core::NormalOf::Polygons)
                    .unwrap_or(frame.basis.z_axis);
                return if axis == 0 {
                    normal
                } else {
                    frame
                        .basis
                        .x_axis
                        .reject_from(normal)
                        .try_normalize()
                        .unwrap_or_else(|| {
                            frame.basis.y_axis.reject_from(normal).normalize_or_zero()
                        })
                };
            }
            // Existing face-local Bevel roles; freeze directions from stage base.
            if let Some(i) = l.base.active()
                && let Some(&f) = l.base.selection.faces.first()
            {
                let local = if axis == 0 {
                    i.mesh.normal(f as usize)
                } else {
                    let face = &i.mesh.faces[f as usize];
                    (i.mesh.center(f as usize) - i.mesh.vertices[face.vertices[0] as usize])
                        .normalize_or_zero()
                };
                // Face-local directions shown in world space for moved items.
                let world = l
                    .base
                    .world_matrix(i.id)
                    .map(|m| m.transform_vector3(local).normalize_or_zero())
                    .unwrap_or(local);
                // The normal arrow follows the whole selection's average
                // normal (2026-11-23), not only the first polygon's; the
                // Inset arrow stays square to it.
                return match self.bevel_normal(operandi_core::NormalOf::Polygons) {
                    Some(normal) if axis == 0 => normal,
                    Some(normal) => world.reject_from(normal).normalize_or(world),
                    None => world,
                };
            }
            a
        } else if let Some(frame) = self.editor.handle_frame().or(l.action_frame) {
            // The transform tools' arrows are the pivot service's handle
            // (under Local the last selected element's cluster, AC-75);
            // the other frame tools show their action frame.
            frame.basis * a
        } else {
            a
        }
    }
    /// Original-compatible A1: handles have a size in the scene, so they
    /// shrink on screen as the view zooms in on them and grow as it zooms
    /// out; 1 (constant screen size) with A1 off. See `Camera::handle_zoom`.
    pub(crate) fn handle_zoom_at(&self, center: DVec3) -> f32 {
        if crate::original_compatible::is_original(WORLD_SIZE_HANDLES) {
            self.camera.handle_zoom(center) as f32
        } else {
            1.
        }
    }
    /// Handle Scale: Standard times `handle_zoom_at(center)`.
    pub(crate) fn handle_scale_at(&self, center: DVec3) -> f32 {
        self.handle_preferences.scale * self.handle_zoom_at(center)
    }
    /// A handle arrow along `direction` from `origin`, drawn in perspective
    /// (tester's request, 2026-11-24): it has a fixed length in the scene,
    /// chosen so it is `pixels` long on screen when seen side-on at
    /// `origin` (so still constant at any zoom, PH-04), and it shortens as
    /// it turns toward the camera. Before, every arrow was stretched to
    /// `pixels` along its on-screen direction, so one pointing nearly at
    /// the camera was drawn full length along an unstable direction and
    /// swung around as the view turned. Returns the arrow's on-screen
    /// start and full vector, or `None` when it points so nearly at the
    /// camera that it would be a stub too short to see or grab, or its
    /// tip is behind the camera. Drags don't use this length; they measure
    /// along the axis themselves (`update_handle`).
    pub(crate) fn handle_arrow(
        &self,
        origin: DVec3,
        direction: DVec3,
        pixels: f32,
        r: Rect,
    ) -> Option<(Pos2, Vec2)> {
        /// Shorter than this share of its side-on length, an arrow hides.
        const SHORTEST: f32 = 0.1;
        let direction = direction.try_normalize()?;
        let (a, _) = self.camera.project(origin, r)?;
        // Pixels per scene unit at the origin's depth, measured across the
        // view where nothing is foreshortened.
        let probe = self.camera.distance * 0.01;
        let (side, _) = self
            .camera
            .project(origin + self.camera.basis().1 * probe, r)?;
        let per_unit = a.distance(side) as f64 / probe;
        if !per_unit.is_finite() || per_unit <= 0. {
            return None;
        }
        let (b, _) = self
            .camera
            .project(origin + direction * (pixels as f64 / per_unit), r)?;
        let v = b - a;
        (v.length() >= pixels * SHORTEST).then_some((a, v))
    }
    /// The live transform handle's frame before the drag's delta: where
    /// the handle sits at `delta = 0` and the axes a unit of `delta` moves
    /// it along. Under Local (AC-75) that's the last selected element's
    /// cluster frame, which is also what the core moves that cluster by;
    /// otherwise the action frame, with the polygon stage's Symmetry
    /// centre for component tools. Snapping measures in this same frame.
    pub(crate) fn transform_handle_frame(&self) -> Option<operandi_core::ActionFrame> {
        let l = self.editor.session.live.as_ref()?;
        let frame = l.action_frame?;
        // The pivot service places the frame (ACS-01/07); the polygon
        // stage's handle follows its live caps.
        if let Some(handle) = self.editor.handle_frame() {
            return Some(handle);
        }
        let center = if l.tool.transforms_items() {
            frame.center
        } else {
            self.editor.symmetry_handle_centre().unwrap_or(frame.center)
        };
        Some(operandi_core::ActionFrame {
            center,
            basis: frame.basis,
        })
    }
    pub(crate) fn transform_handle_center(&self) -> DVec3 {
        let Some(l) = self.editor.session.live.as_ref() else {
            return DVec3::ZERO;
        };
        self.transform_handle_frame()
            .map(|frame| {
                frame.center
                    + if matches!(l.tool, Tool::Move | Tool::Transform | Tool::EdgeExtend) {
                        frame.basis * l.params.delta
                    } else {
                        DVec3::ZERO
                    }
            })
            .unwrap_or(DVec3::ZERO)
    }
    pub(super) fn rotation_ring(&self, axis: usize, r: Rect) -> Vec<Pos2> {
        let center = self.transform_handle_center();
        let Some((_, depth)) = self.camera.project(center, r) else {
            return vec![];
        };
        let radius = if self
            .editor
            .session
            .live
            .as_ref()
            .is_some_and(|l| combined_handles(l.tool))
        {
            96.
        } else {
            72.
        } * self.handle_scale_at(center) as f64;
        let world_radius = radius
            * if self.camera.view == View::Perspective {
                depth
            } else {
                self.camera.distance
            }
            / (r.height() as f64 * 0.9);
        let u = self.handle_axis((axis + 1) % 3);
        let v = self.handle_axis((axis + 2) % 3);
        (0..=40)
            .filter_map(|i| {
                let angle = i as f64 * std::f64::consts::TAU / 48.;
                self.camera
                    .project(
                        center + (u * angle.cos() + v * angle.sin()) * world_radius,
                        r,
                    )
                    .map(|p| p.0)
            })
            .collect()
    }
    /// The handle under `p`. PH-11 (provisional order): the center handle,
    /// then the arrows and rings, then the plane circles.
    pub(crate) fn handle_hit(&self, p: Pos2, r: Rect) -> Option<usize> {
        // The Snaps and Precision presets' handles sit on top.
        if let Some(id) = self.precision_hit(p, r) {
            return Some(id);
        }
        match self.handle_hit_axes(p, r) {
            // Transform's rotation rings pass through the corner where its
            // plane circles sit (PH-07): a press on a circle takes the
            // circle, while a press on the ring elsewhere still turns.
            Some(ring)
                if (3..6).contains(&ring)
                    && self
                        .editor
                        .session
                        .live
                        .as_ref()
                        .is_some_and(|l| combined_handles(l.tool)) =>
            {
                self.plane_handle_hit(p, r).or(Some(ring))
            }
            Some(hit) => Some(hit),
            None => self.plane_handle_hit(p, r),
        }
    }
    pub(super) fn handle_hit_axes(&self, p: Pos2, r: Rect) -> Option<usize> {
        if self
            .editor
            .session
            .live
            .as_ref()
            .is_some_and(|l| l.tool.card().handles.kind == HandleKind::Spine)
        {
            return self.twist_bevel_hit(p, r);
        }
        if let Some(live) = &self.editor.session.live
            && matches!(live.tool, Tool::Move | Tool::Rotate | Tool::Transform)
            && let Some(free) = self.handles(r).into_iter().find(|h| {
                h.0 == FREE_HANDLE && Rect::from_center_size(h.2, vec2(12., 12.)).contains(p)
            })
        {
            return Some(free.0);
        }
        if let Some(live) = &self.editor.session.live
            && combined_handles(live.tool)
        {
            let handles = self.handles(r);
            let mut hits: Vec<_> = handles
                .iter()
                .filter(|h| {
                    !(3..6).contains(&h.0) && !is_plane_handle(h.0) && p.distance(h.2) <= 7.
                })
                .map(|h| h.0)
                .collect();
            if hits.is_empty() {
                for axis in 0..3 {
                    // Edge Extend draws its rings only with Rotate on.
                    if !handles.iter().any(|h| h.0 == axis + 3) {
                        continue;
                    }
                    if self
                        .rotation_ring(axis, r)
                        .windows(2)
                        .any(|s| input::segment_distance(p, s[0], s[1]) <= 5.)
                    {
                        hits.push(axis + 3);
                    }
                }
            }
            return if hits.len() == 1 { Some(hits[0]) } else { None };
        }
        if self
            .editor
            .session
            .live
            .as_ref()
            .is_some_and(|l| l.tool == Tool::Rotate)
        {
            return (0..3).find(|&axis| {
                self.rotation_ring(axis, r)
                    .windows(2)
                    .any(|s| input::segment_distance(p, s[0], s[1]) <= 5.)
            });
        }
        let handles = self.handles(r);
        if let Some(center) = handles.iter().find(|h| {
            h.0 == SCALE_UNIFORM_HANDLE
                && self
                    .editor
                    .session
                    .live
                    .as_ref()
                    .is_some_and(|l| l.tool == Tool::Scale)
                && self.scale_center_contains(h.2, p)
        }) {
            return Some(center.0);
        }
        if self
            .editor
            .session
            .live
            .as_ref()
            // Every primitive the same way (Jason, 2026-11-21): its side
            // squares first, then the move arrows along their whole shaft,
            // not only at the tip (Sphere and Cylinder took the tip only,
            // so a press on the arrow dragged the nearest side instead).
            .map_or(self.session_shape().is_some(), |live| {
                live.tool.is_primitive()
            })
            && let Some(side) = handles.iter().filter(|h| h.0 >= 3).find(|h| {
                Rect::from_center_size(h.2, Vec2::splat(self.handle_preferences.point_hit_size))
                    .contains(p)
            })
        {
            return Some(side.0);
        }
        handles
            .iter()
            .filter(|h| !is_plane_handle(h.0))
            .find(|(_, a, b, _)| {
                Rect::from_center_size(*b, Vec2::splat(self.handle_preferences.point_hit_size))
                    .contains(p)
                    || input::segment_distance(p, *a, *b)
                        <= self.handle_preferences.line_hit_width / 2.
            })
            .map(|h| h.0)
    }
    /// Edge Slide's handle frame (`HandleAnchor::EdgeRail`), once per drag;
    /// see `operandi_core::edge_slide_handle_frame`.
    pub(super) fn edge_slide_frame(&self) -> Option<(DVec3, DVec3, f64)> {
        let l = self.editor.session.live.as_ref()?;
        let symmetry = self.editor.active_symmetry();
        self.edge_slide_frame_memo.get_with(
            &l.base,
            (symmetry, operandi_core::HandlePick::FirstPicked),
            || operandi_core::edge_slide_handle_frame(&l.base, symmetry),
        )
    }
    /// The vertex tools' handle frame (`HandleAnchor::Vertex`), once per
    /// drag; see `operandi_core::vertex_handle_frame`.
    pub(super) fn vertex_frame(&self) -> Option<(DVec3, DVec3, DVec3, f64)> {
        let l = self.editor.session.live.as_ref()?;
        let symmetry = self.editor.active_symmetry();
        let pick = operandi_core::handle_pick(l.tool);
        self.vertex_frame_memo
            .get_with(&l.base, (symmetry, pick), || {
                operandi_core::vertex_handle_frame(&l.base, symmetry, pick)
            })
    }
    /// The selection's average normal (`operandi_core::selection_normal`)
    /// for the live tool's base, once per drag: the Bevels' gizmo direction.
    fn bevel_normal(&self, of: operandi_core::NormalOf) -> Option<DVec3> {
        let l = self.editor.session.live.as_ref()?;
        let key = (self.editor.active_symmetry(), of);
        self.selection_normal_memo.get_with(&l.base, key, || {
            operandi_core::selection_normal(&l.base, key.0, of)
        })
    }
    /// Where Edge Bevel's handles sit once it has bevelled: the middle of
    /// the live selected edges (the bevel strips' edges), in world space.
    /// With Symmetry on, `handles` still moves it to the primary pivot.
    fn edge_bevel_centre(&self) -> Option<DVec3> {
        let item = self.editor.scene.active()?;
        let edges = &self.editor.scene.selection.edges;
        if edges.is_empty() {
            return None;
        }
        let v = &item.mesh.vertices;
        let sum = edges.iter().try_fold(DVec3::ZERO, |sum, e| {
            Some(sum + (*v.get(e.0 as usize)? + *v.get(e.1 as usize)?) / 2.)
        })?;
        let centre = sum / edges.len() as f64;
        let world = self.editor.scene.world_matrix(item.id).ok()?;
        Some(world.transform_point3(centre))
    }

    /// The edge tools' handle frame (`HandleAnchor::Edge`), once per drag;
    /// see `operandi_core::edge_handle_frame`.
    pub(super) fn edge_handle_frame(&self) -> Option<(DVec3, DVec3, DVec3)> {
        let l = self.editor.session.live.as_ref()?;
        let symmetry = self.editor.active_symmetry();
        let pick = operandi_core::handle_pick(l.tool);
        self.edge_frame_memo
            .get_with(&l.base, (symmetry, pick), || {
                operandi_core::edge_handle_frame(&l.base, symmetry, pick)
            })
    }
}
