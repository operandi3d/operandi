use super::*;
use theme::viewport as palette;
mod extend_handles;
mod numeric_drag;
pub(super) mod plane_handles;
pub(crate) mod precision_handles;
use operandi_core::click_select::{self, Click, ClickHow};
use operandi_core::tool_card::{HandleAnchor, HandleAxes, HandleKind};
use plane_handles::is_plane_handle;
pub(crate) mod bend_view;
pub(crate) mod click_start_view;
pub(crate) mod falloff_screen_view;
pub(crate) mod falloff_view;
pub(crate) mod tweak_view;
pub(crate) mod twist_bevel_handles;

/// Smallest half-width of the Linear falloff wedge's open end, in points,
/// so the wedge stays visible when the falloff points at the camera.
const FALLOFF_WEDGE_MIN_HALF_WIDTH: f32 = 6.;
/// Original-compatible A1: handles have a size in the scene.
const WORLD_SIZE_HANDLES: &str = "A1";

/// Screen outline of a Linear falloff: a point at the Start handle `a`
/// widening to an open end across the End handle `b`, about a quarter of
/// the on-screen length wide. Returns `[tip, left, right]`.
fn linear_falloff_wedge(a: Pos2, b: Pos2) -> [Pos2; 3] {
    let d = b - a;
    let length = d.length();
    // Seen end-on the direction is lost; any perpendicular will do.
    let across = if length > 1e-3 {
        vec2(-d.y, d.x) / length
    } else {
        vec2(1., 0.)
    };
    let half = (length * 0.125).max(FALLOFF_WEDGE_MIN_HALF_WIDTH);
    [a, b + across * half, b - across * half]
}

/// Filled arrowhead ending at `tip`, pointing from `from`.
fn handle_arrow_head(painter: &egui::Painter, from: Pos2, tip: Pos2, color: Color32) {
    let d = tip - from;
    if d.length() < 1e-3 {
        return;
    }
    let dir = d.normalized();
    let side = vec2(-dir.y, dir.x) * 4.5;
    let base = tip - dir * 11.;
    painter.add(Shape::convex_polygon(
        vec![tip, base + side, base - side],
        color,
        Stroke::NONE,
    ));
}

const SCALE_UNIFORM_HANDLE: usize = 3;
/// Pseudo-handle id for dragging an Extrude off its gizmo (free movement).
const EXTRUDE_FREE_HANDLE: usize = 9;
/// Pseudo-handle id for dragging Vertex Bevel anywhere in the viewport.
const VERTEX_BEVEL_FREE_HANDLE: usize = 10;
/// The center handle of Move, Rotate and Transform: free movement in the
/// view plane, or free rotation (Jason, 2026-11-03; provisional).
pub(super) const FREE_HANDLE: usize = 11;
/// How far (points) the second press of an edge double-click may land from
/// the first; egui's own click tolerance.
const EDGE_DOUBLE_CLICK_DISTANCE: f32 = 6.;
/// Pseudo-handle id for a drag anywhere off the handles (Jason, 2026-11-04):
/// up/down acts like the tool's move arrow (its first handle), left/right
/// like its scale widget (its second handle). See `update_off_handle`.
pub(super) const OFF_HANDLE: usize = 12;
/// Edge Extend's Local handles (Inset, Shift); only Edge Extend uses these
/// ids, as Extrude and Vertex Bevel use 9 and 10 for theirs.
pub(super) const EXTEND_INSET_HANDLE: usize = 9;
pub(super) const EXTEND_SHIFT_HANDLE: usize = 10;
/// Tools with Transform's handle layout: 0..3 move, 3..6 rotate rings,
/// 6..9 scale (Edge Extend shows the sets its Handles option picks).
pub(super) fn combined_handles(tool: Tool) -> bool {
    tool.card().handles.kind == HandleKind::Combined
}
/// Pointer travel (points) before a Ctrl haul picks its axis (HM-12).
const HAUL_AXIS_PICK_PIXELS: f32 = 4.;

/// The one affine map taking every point of `from` to the same point of
/// `to`, if there is one: solved from four spread-out points, then checked
/// against all of them (a cheap pass over the points, once per frame).
/// `None` when the points don't span a volume or any point moved
/// differently. A mirroring map is refused too (the cached triangles would
/// face the wrong way).
#[cfg(test)]
pub(super) fn whole_mesh_map(from: &[DVec3], to: &[DVec3]) -> Option<glam::DMat4> {
    whole_mesh_map_from(from, to, whole_mesh_basis(from)?)
}

/// Three points of `from` that span its volume with the first point, for
/// `whole_mesh_map_from`; `None` when the points don't span one. Depends only
/// on `from`, so a drag finds them once (`BaseMemo`).
pub(super) fn whole_mesh_basis(from: &[DVec3]) -> Option<[usize; 3]> {
    if from.len() < 4 {
        return None;
    }
    let farthest = |score: &dyn Fn(DVec3) -> f64| {
        (0..from.len()).max_by(|&a, &b| score(from[a]).total_cmp(&score(from[b])))
    };
    let p0 = from[0];
    let i1 = farthest(&|p| p.distance_squared(p0))?;
    let e1 = (from[i1] - p0).normalize_or_zero();
    let i2 = farthest(&|p| (p - p0).reject_from_normalized(e1).length_squared())?;
    let e2 = (from[i2] - p0).cross(e1).normalize_or_zero();
    let i3 = farthest(&|p| (p - p0).dot(e2).abs())?;
    let basis = glam::DMat3::from_cols(from[i1] - p0, from[i2] - p0, from[i3] - p0);
    let size = from[i1].distance(p0);
    (size > 0. && basis.determinant().abs() > 1e-9 * size * size * size).then_some([i1, i2, i3])
}

/// `whole_mesh_map` with the spanning points already found.
pub(super) fn whole_mesh_map_from(
    from: &[DVec3],
    to: &[DVec3],
    [i1, i2, i3]: [usize; 3],
) -> Option<glam::DMat4> {
    if from.len() != to.len() || from.len() <= i1.max(i2).max(i3) {
        return None;
    }
    let p0 = from[0];
    let basis = glam::DMat3::from_cols(from[i1] - p0, from[i2] - p0, from[i3] - p0);
    let size = from[i1].distance(p0);
    let q0 = to[0];
    let moved = glam::DMat3::from_cols(to[i1] - q0, to[i2] - q0, to[i3] - q0);
    let linear = moved * basis.inverse();
    if !linear.is_finite() || linear.determinant() <= 0. {
        return None;
    }
    let map = glam::DMat4::from_mat3_translation(linear, q0 - linear * p0);
    let tolerance = 1e-9 * size.max(1.);
    let close = |p: DVec3, q: DVec3| p.distance_squared(q) <= tolerance * tolerance;
    // The four spanning points stayed put: either nothing moved (no map
    // needed) or only some points did, which the full pass below would find
    // at the cost of reading every point each frame. Drawing the live mesh
    // itself is right in both cases, and patches only what moved.
    if [0, i1, i2, i3].iter().all(|&i| close(from[i], to[i])) {
        return None;
    }
    from.iter()
        .zip(to)
        .all(|(&p, &q)| close(map.transform_point3(p), q))
        .then_some(map)
}

/// Gestures whose drag depends only on where it started and where the
/// pointer is now, so intermediate pointer moves in a frame can be skipped.
/// Paths (lasso, sketch, pen, draw), painting and navigation keep every
/// move.
fn latest_position_only(kind: &input::GestureKind) -> bool {
    matches!(
        kind,
        input::GestureKind::Handle(..)
            | input::GestureKind::ItemHandle(_)
            | input::GestureKind::Falloff(..)
            | input::GestureKind::FalloffDraw { .. }
            | input::GestureKind::Tweak { .. }
            | input::GestureKind::Numeric(_)
            | input::GestureKind::PolyHaul(_)
            | input::GestureKind::Haul(_)
            // A stroke lays its dabs along the straight line from its last
            // dab, so skipped moves cost nothing but their kinks.
            | input::GestureKind::Sculpt
            | input::GestureKind::SculptSize { .. }
            | input::GestureKind::SculptStrength { .. }
    ) || painting_fast(kind)
}

/// Pixels of right-drag per Selection falloff Step.
pub(crate) const FALLOFF_STEP_PIXELS: f32 = 20.;
/// A right-press that moves less than this is a click (adds one Step).
pub(crate) const FALLOFF_CLICK_PIXELS: f32 = 4.;

/// Tools whose plain drag away from the handles goes through `OFF_HANDLE`
/// (`Handles::drag_anywhere` on the tool's card). The others have a gesture
/// of their own (transforms, primitives, Extrude, Vertex Bevel, Tweak, Poly
/// Haul, Sketch Extrude) or place points by clicking (Create).
pub(super) fn off_handle_drag(tool: Tool) -> bool {
    tool.card().handles.drag_anywhere
}

fn selected_vertex_mask(count: usize, selected: &[u32]) -> Vec<bool> {
    let mut mask = vec![false; count];
    for &index in selected {
        if let Some(value) = mask.get_mut(index as usize) {
            *value = true;
        }
    }
    mask
}

/// Whether `pos` is on a window (an egui area that takes the pointer) drawn
/// above the layer `ui` draws on: what the user clicks there is that window.
fn over_window_above(ui: &Ui, pos: Pos2) -> bool {
    let own = ui.layer_id();
    ui.ctx()
        .layer_id_at(pos)
        .is_some_and(|top| top != own && top.order >= own.order)
}

impl Operandi {
    pub(super) fn sketch_plane_point(
        camera: &Camera,
        rect: Rect,
        pos: Pos2,
        center: DVec3,
        normal: DVec3,
    ) -> Option<DVec3> {
        let (origin, direction) = camera.ray(pos, rect);
        let den = direction.dot(normal);
        if den.abs() < 1e-9 {
            return None;
        }
        let t = (center - origin).dot(normal) / den;
        (t >= 0. && t.is_finite()).then_some(origin + direction * t)
    }
    pub(super) fn sketch_feedback_points(&self) -> Vec<DVec3> {
        if let Some(input::Navigation {
            kind: input::GestureKind::SketchStroke(stroke),
            ..
        }) = &self.navigation
        {
            let mut points = stroke.samples.clone();
            if points
                .last()
                .is_some_and(|p| p.distance(stroke.current) > 1e-9)
            {
                points.push(stroke.current);
            }
            return points;
        }
        self.editor
            .live
            .as_ref()
            .filter(|l| l.tool == Tool::SketchExtrude)
            .map(|l| l.params.sketch_path.clone())
            .unwrap_or_default()
    }
    fn sketch_stroke_move(&mut self, stroke: &mut input::SketchStroke, pos: Pos2, finish: bool) {
        let Some(point) = Self::sketch_plane_point(
            &stroke.camera,
            stroke.rect,
            pos,
            stroke.plane_center,
            stroke.plane_normal,
        ) else {
            self.status = "Sketch Extrude cannot project this pointer onto its frozen plane".into();
            if let Some(live) = &mut self.editor.live {
                live.preview_error = Some(self.status.clone());
            }
            return;
        };
        stroke.current = point;
        if pos.distance(stroke.last_sample) > 1e-4 {
            stroke.started = true;
        }
        if !stroke.started {
            return;
        }
        let mut path = stroke.samples.clone();
        if path
            .last()
            .is_some_and(|p| p.distance(stroke.current) > 1e-9)
        {
            path.push(stroke.current);
        }
        if path.len() > 257 {
            self.status = "Sketch Extrude path exceeds 257 points".into();
            if let Some(live) = &mut self.editor.live {
                live.preview_error = Some(self.status.clone());
            }
            return;
        }
        let Some(live) = self
            .editor
            .live
            .as_ref()
            .filter(|l| l.tool == Tool::SketchExtrude)
        else {
            return;
        };
        let mut params = live.params.clone();
        params.sketch_path = if path.len() >= 2 {
            path.clone()
        } else {
            Vec::new()
        };
        match self.editor.dispatch(Command::Preview(params)) {
            Ok(()) => {
                self.status = "Ready".into();
                self.clear_fields();
                if path.len() >= 2
                    && (finish || pos.distance(stroke.last_sample) as f64 >= stroke.precision)
                {
                    stroke.samples = path;
                    stroke.last_sample = pos;
                }
            }
            Err(error) => {
                self.status = error.clone();
                if let Some(live) = &mut self.editor.live {
                    live.preview_error = Some(error);
                }
            }
        }
    }
    /// One view's toolbar, drawing and input. The view being drawn has its
    /// camera in `self.camera` (views.rs swaps the others in). A passive
    /// view (not the active one) draws but takes no input.
    pub(super) fn view_ui(&mut self, ui: &mut Ui) {
        // Each view kind draws with its stock scheme (colors-2026-11-16
        // section 3): perspective and camera views, or Top/Front/Side.
        let _scheme = theme::viewport::use_scheme(if self.camera.view == View::Perspective {
            theme::viewport::ViewScheme::Perspective
        } else {
            theme::viewport::ViewScheme::Orthographic
        });
        self.sync_layer_locator();
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        {
            // The message line sits under Tool Properties when that is
            // docked; then the view keeps its full height.
            let status_in_view =
                !self.layouts.message_line_here || self.status.starts_with("Help:");
            let status_height = if self.status.starts_with("Help:") {
                140.
            } else if status_in_view {
                24.
            } else {
                0.
            };
            let (r, response) = ui.allocate_exact_size(
                vec2(
                    ui.available_width(),
                    (ui.available_height() - status_height).max(1.),
                ),
                Sense::click_and_drag(),
            );
            self.viewport_rect = r;
            self.keep_world_scale(r);
            self.sync_camera_view(r);
            // A wheel move in Camera view is written once the view holds
            // still: come back then even if nothing else repaints.
            if let Some(wait) = self.camera_view_settle_in() {
                ui.ctx().request_repaint_after(wait);
            }
            let painter = ui.painter_at(r);
            match self.display.settings.drawing.background {
                view_display::Background::None => {
                    painter.rect_filled(r, 0., palette::background());
                }
                view_display::Background::Gradient => {
                    let (top, bottom) = (palette::background_top(), palette::background());
                    let mut mesh = egui::Mesh::default();
                    mesh.colored_vertex(r.left_top(), top);
                    mesh.colored_vertex(r.right_top(), top);
                    mesh.colored_vertex(r.right_bottom(), bottom);
                    mesh.colored_vertex(r.left_bottom(), bottom);
                    mesh.add_triangle(0, 1, 2);
                    mesh.add_triangle(0, 2, 3);
                    painter.add(mesh);
                }
                view_display::Background::Environment => {
                    self.paint_environment_background(&painter, r);
                }
            }
            // HDRI stage 2: the environment's picture, drawn on the card
            // over the gradient above (gradient-only skies keep that path).
            let sky = (self.display.settings.drawing.background
                == view_display::Background::Environment)
                .then(|| self.sky_draw(ui.ctx()))
                .flatten();
            // HDRI stage 3: an environment picture lights the Advanced style.
            let env_light = self.env_light(ui.ctx());
            let (backdrops, placeholders) = self.backdrop_frame(r);
            self.draw_backdrop_placeholders(&painter, r, &placeholders);
            let (eye, _, _, forward) = self.camera.basis();
            let plane = self.actual_plane();
            let plane_center = self.workplane_center();
            let project = |p| self.camera.project(p, r).map(|p| p.0);
            let segment = |a, b, color: Color32, width: f32| {
                // Cut at the near clip, not dropped (as the GPU wire).
                if let Some((a, b)) = self.camera.project_segment(a, b, r) {
                    painter.line_segment([a, b], Stroke::new(width, color));
                }
            };
            let grid_step = grid::spacing(
                &self.camera,
                r,
                if self.plane_display { plane } else { 0 },
                if self.plane_display {
                    plane_center
                } else {
                    DVec3::ZERO
                },
            );
            let grid_label = if self.plane_display {
                format!("Work plane: {grid_step} m minor")
            } else if self.display.settings.drawing.grid {
                format!("Ground XZ: {grid_step} m minor")
            } else {
                "Grid hidden".into()
            };
            let mut grids = Vec::new();
            // Show Grid / Show Work Plane (Viewport Properties > Visibility):
            // the work plane counts as moved once it is fixed or centred
            // off the origin.
            let drawing = self.display.settings.drawing;
            let work_plane_moved = {
                let w = &self.editor.scene.workplane;
                w.fixed || w.center != DVec3::ZERO
            };
            let work_plane_grid = self.plane_display && drawing.work_plane_grid(work_plane_moved);
            let fade = drawing.faded_grid;
            // Separate layers preserve ground visibility even when the
            // coincident workplane grid is made fully transparent.
            if drawing.floor_grid(work_plane_moved) {
                grids.push(grid::PlaneGrid::new(
                    &self.camera,
                    r,
                    grid::Display {
                        plane: crate::camera::ground_plane(),
                        center: DVec3::ZERO,
                        axes: false,
                        workplane: false,
                        visible: true,
                        opacity: 1.,
                        zero_axes: false,
                        fade,
                    },
                    ctx.pixels_per_point(),
                ));
            }
            if work_plane_grid || self.axes {
                grids.push(grid::PlaneGrid::new(
                    &self.camera,
                    r,
                    grid::Display {
                        plane,
                        center: plane_center,
                        axes: self.axes,
                        workplane: true,
                        visible: work_plane_grid,
                        opacity: f32::from(self.viewport_preferences.workplane_opacity) / 100.,
                        zero_axes: false,
                        fade,
                    },
                    ctx.pixels_per_point(),
                ));
            }
            // Ground axes stay on world X/Z, independently of the active
            // construction plane and its display opacity. Draw over grid
            // lattices so a coincident workplane cannot wash out their tint.
            // Never add an idle Y axis; tool and corner-badge Y cues remain.
            // Top/Bottom: the zero lines below lie exactly on the ground axes
            // and replace them; their anti-aliased edges would otherwise let
            // the light tint show through the dark line.
            let zero_plane = if drawing.floor_grid(work_plane_moved) {
                grid::view_zero_plane(&self.camera)
            } else {
                None
            };
            if self.ground_axes
                && drawing.grid_axes
                && zero_plane != Some(crate::camera::ground_plane())
            {
                grids.push(grid::PlaneGrid::new(
                    &self.camera,
                    r,
                    grid::Display {
                        plane: crate::camera::ground_plane(),
                        center: DVec3::ZERO,
                        axes: true,
                        workplane: false,
                        visible: false,
                        opacity: 1.,
                        zero_axes: false,
                        fade,
                    },
                    ctx.pixels_per_point(),
                ));
            }
            // Top/Front/Side: a dark line through world zero on each of the
            // two axes the view shows, drawn last so it reads over the grid
            // and the tinted axes. Follows the grid's visibility.
            if let Some(plane) = zero_plane {
                grids.push(grid::PlaneGrid::new(
                    &self.camera,
                    r,
                    grid::Display {
                        plane,
                        center: DVec3::ZERO,
                        axes: false,
                        workplane: false,
                        visible: false,
                        opacity: 1.,
                        zero_axes: true,
                        fade,
                    },
                    ctx.pixels_per_point(),
                ));
            }
            // Version-local immutable projected snapshot; GPU depth resolves occlusion.
            // A numeric tool's preview (Cone, Ellipsoid, the Shapes) stands
            // in for the scene; otherwise MeshOps (MO-01): items with a
            // stack show its result, the mesh in Edit Base Mesh its Base
            // Mesh (MO-27). Frame All reads the same (`shown_display`).
            let preview_snapshot = self.editor.tool_preview_display();
            let meshops_snapshot = preview_snapshot
                .is_none()
                .then(|| self.editor.evaluated_display())
                .flatten();
            let display_scene = preview_snapshot
                .as_ref()
                .or(meshops_snapshot.as_ref())
                .unwrap_or(&self.editor.scene);
            // G8 RP-3: Replicators' copies draw as wire boxes unless this
            // view's Replicators option shades them.
            let boxed_snapshot = self.boxed_replicas(display_scene);
            let display_scene = boxed_snapshot.as_ref().unwrap_or(display_scene);
            // The picking surface is built only when something this frame
            // needs it (a drag on a large mesh changes it every frame, and
            // nothing picks during the drag).
            let surface_cell: std::cell::OnceCell<std::sync::Arc<picking::Surface>> =
                Default::default();
            let surface_cache = &self.surface_cache;
            let subdivision_cache = &self.subdivision_cache;
            let surface = || {
                surface_cell
                    .get_or_init(|| surface_cache.get(display_scene, subdivision_cache))
                    .clone()
            };
            // Items drawn from geometry kept on the graphics card
            // (gpu_scene.rs): all visible items, except while a Cone or
            // Ellipsoid preview replaces the scene and except an item a
            // primitive tool is drawing.
            let resident_on =
                (self.viewport_gpu.is_some() || self.force_resident) && preview_snapshot.is_none();
            // Only primitive tools draw their item specially (new polygons in
            // the handle color, flat previews); transform tools like Move use
            // the cached path, rebuilt when the mesh changes during a drag.
            let live_item = self
                .editor
                .live
                .as_ref()
                .filter(|l| l.tool.is_primitive())
                .and_then(|l| l.base.active().map(|b| b.id));
            // CP-D01: items whose Display Visible is No (or under No (Hide
            // Children)) leave the view; empty in the common case.
            let display_hidden = display_scene.display_hidden();
            let shown = |i: &operandi_core::Item| i.visible && !display_hidden.contains(&i.id);
            let resident: std::collections::BTreeSet<u64> = display_scene
                .items
                .iter()
                .filter(|i| resident_on && shown(i) && Some(i.id) != live_item)
                .map(|i| i.id)
                .collect();
            let mut faces = Vec::new();
            for item in display_scene
                .items
                .iter()
                .filter(|i| shown(i) && !resident.contains(&i.id))
            {
                // This view's look for the item (view_display.rs); the
                // projected path colours each face the way the shader would
                // (display_shading.rs).
                let Some(look) = self.item_look(item.id) else {
                    continue;
                };
                let wireframe = look.style == view_display::Style::Wireframe;
                // The plain wire follows the overlay as on the GPU path
                // (Show Wireframe in Item Mode off: none, VS-6).
                let plain = look.draws_wire();
                // Scene and transforms are immutable during this preparation pass.
                // Preserve world_point's NaN fallback for invalid hierarchies.
                let world_matrix = display_scene.world_matrix(item.id).ok();
                // Each point is transformed and projected once per frame, not
                // once per polygon corner.
                let projector = self.camera.projector(r);
                let world_points: Vec<DVec3> = item
                    .mesh
                    .vertices
                    .iter()
                    .map(|&v| world_matrix.map_or(DVec3::NAN, |m| m.transform_point3(v)))
                    .collect();
                let screen_points: Vec<Option<(Pos2, f64)>> =
                    world_points.iter().map(|&p| projector.project(p)).collect();
                for (fi, f) in item.mesh.faces.iter().enumerate() {
                    if item.mesh.is_polyline(fi) {
                        continue;
                    }
                    let world: Vec<_> = f
                        .vertices
                        .iter()
                        .map(|&v| world_points[v as usize])
                        .collect();
                    let normal = world
                        .iter()
                        .enumerate()
                        .map(|(i, &p)| {
                            (p - world[0]).cross(world[(i + 1) % world.len()] - world[0])
                        })
                        .sum::<DVec3>()
                        .normalize_or_zero();
                    let center = world.iter().copied().sum::<DVec3>() / world.len() as f64;
                    let flat_preview = self.editor.live.as_ref().is_some_and(|live| {
                        live.tool == Tool::Box
                            && live
                                .params
                                .size
                                .to_array()
                                .iter()
                                .filter(|&&s| s == 0.)
                                .count()
                                == 1
                            && live.base.active().is_some_and(|base| {
                                base.id == item.id && fi >= base.mesh.faces.len()
                            })
                    });
                    if !wireframe
                        && !flat_preview
                        && normal.dot(if self.camera.view == View::Perspective {
                            eye - center
                        } else {
                            -forward
                        }) <= 0.
                    {
                        continue;
                    }
                    let projected: Option<Vec<_>> = f
                        .vertices
                        .iter()
                        .map(|&v| screen_points[v as usize])
                        .collect();
                    if let Some(points) = projected {
                        let depth = points.iter().map(|p| p.1).sum::<f64>() / points.len() as f64;
                        let tessellation = surface_cell
                            .get_or_init(|| surface_cache.get(display_scene, subdivision_cache))
                            .corners(item.id, fi);
                        let smooth = operandi_core::subdivision::is_smooth_polygon(&item.mesh, fi);
                        faces.push((
                            depth,
                            item.id,
                            fi,
                            points,
                            normal,
                            &f.vertices,
                            tessellation,
                            (smooth, wireframe, look.style, center, plain),
                        ));
                    }
                }
            }
            let mut triangles = Vec::new();
            // Shared edges: about half the visible corners.
            let mut outlines = outline::Outlines::with_capacity(
                faces.iter().map(|f| f.3.len()).sum::<usize>() / 2 + 16,
            );
            let hover = response
                .hover_pos()
                .filter(|_| {
                    self.navigation.is_none()
                        && self.preset_browser.drag.is_none()
                        && (self.rollovers_on() || self.drop_drag.is_some())
                        && surface_cache.settled(display_scene)
                })
                .and_then(|p| {
                    // A drop drag targets any item; selection rollovers
                    // see what a click would pick.
                    let only = if self.drop_drag.is_none() {
                        self.pick_only()
                    } else {
                        Vec::new()
                    };
                    let hit = surface().pick_in(&self.camera, p, r, crate::picking::limit(&only));
                    // Closest rollovers: the polygon pre-highlights only
                    // when no point or edge is nearer.
                    if self.closest_rollovers() && self.drop_drag.is_none() {
                        let surface = surface();
                        return hit.filter(|_| {
                            matches!(
                                self.pick_closest_on(p, r, &surface),
                                Some((_, Component::Polygon(_)))
                            )
                        });
                    }
                    hit
                })
                // DA-6: a drop drag pre-highlights the target, never the
                // source.
                .filter(|(id, _, _)| self.drop_drag.is_none_or(|d| d.source != *id));
            // A material dragged from the preset browser highlights the
            // material group it would land on (PB-110).
            let drop_flags = self.preset_browser.drop_hover.as_ref().and_then(|h| {
                let item = display_scene.item(h.item)?;
                let mut flags = vec![false; item.mesh.faces.len()];
                for &f in &h.faces {
                    if let Some(flag) = flags.get_mut(f) {
                        *flag = true;
                    }
                }
                Some((h.item, flags))
            });
            // Polygons pre-highlight in Polygon and Item modes, and in every
            // component mode with Closest rollovers.
            let polygon_rollovers =
                matches!(self.editor.scene.selection.mode, Mode::Polygon | Mode::Item)
                    || self.closest_rollovers();
            // Symmetry: the mirror half of the selection draws dimmer.
            let (mirror_vertices, mirror_faces) = self
                .editor
                .scene
                .symmetry_secondary(self.editor.active_symmetry());
            // The display styles' colours, read once per frame.
            let style_colors = crate::display_shading::StyleColors {
                matcap: self.display.settings.drawing.matcap,
                ..crate::display_shading::StyleColors::live()
            };
            // The camera's axes: every style's light is fixed to the view.
            let view_basis = {
                let (_, right, up, forward) = self.camera.basis();
                crate::display_shading::ViewBasis::new(
                    right.as_vec3(),
                    up.as_vec3(),
                    forward.as_vec3(),
                )
            };
            // The direction to the eye from a face's center.
            let to_eye = |center: DVec3| {
                let d = if self.camera.view == View::Perspective {
                    eye - center
                } else {
                    -forward
                };
                d.as_vec3()
            };
            // N09: every selected item shows selected in Item mode, not
            // only the primary (a set: one lookup per face).
            let item_picked: std::collections::HashSet<u64> =
                self.editor.scene.selected_items().into_iter().collect();
            for (
                _,
                id,
                fi,
                points,
                normal,
                indices,
                tessellation,
                (smooth, wireframe, style, center, plain),
            ) in &faces
            {
                // Every active mesh shows its own selected polygons
                // (multi_active.rs); Item mode highlights every selected
                // item (N09).
                let s = &self.editor.scene.selection;
                let selected = (s.mode == Mode::Item && item_picked.contains(id))
                    || (s.mode == Mode::Polygon
                        && s.tag.is_none()
                        && s.faces_of(*id).contains(&(*fi as u32)));
                let pre = (hover.is_some_and(|(i, f, _)| i == *id && f == *fi as u32)
                    && self.editor.live.is_none()
                    && polygon_rollovers)
                    || drop_flags
                        .as_ref()
                        .is_some_and(|(i, flags)| i == id && flags.get(*fi).copied() == Some(true));
                let mirror = selected
                    && self.editor.scene.selection.item == Some(*id)
                    && mirror_faces.contains(&(*fi as u32));
                let pre_color = if pre {
                    display_scene.item(*id).map_or_else(palette::rollover, |i| {
                        palette::polygon_rollover(&i.mesh, *fi)
                    })
                } else {
                    palette::rollover()
                };
                let base = if mirror {
                    palette::mirror_selection()
                } else if selected {
                    palette::selection_fill()
                } else if pre {
                    pre_color
                } else {
                    palette::solid()
                };
                // Faces are flat on this path; the center stands for the face.
                let fill = crate::display_shading::face_color(
                    *style,
                    base,
                    normal.as_vec3(),
                    to_eye(*center),
                    &view_basis,
                    &style_colors,
                    mirror || selected || pre,
                );
                let color = if self.editor.live.as_ref().is_some_and(|l| {
                    l.tool.is_primitive()
                        && l.base
                            .active()
                            .is_some_and(|i| i.id == *id && *fi >= i.mesh.faces.len())
                }) {
                    palette::handles()[1]
                } else if selected {
                    palette::selection()
                } else if pre {
                    pre_color
                } else {
                    self.plain_wire(*id)
                };
                let vertex =
                    |point: &(Pos2, f64), color| outline::vertex(&self.camera, r, *point, color);
                // A smooth polygon fills with its evaluated surface and traces
                // its wire over it, below.
                if *smooth {
                    continue;
                }
                if !*wireframe {
                    for triangle in *tessellation {
                        for &j in triangle {
                            triangles.push(vertex(&points[j], fill));
                        }
                    }
                }
                let gradient = depth_gradient(&self.camera, r, points);
                // Plain edges only where the overlay draws them; highlighted
                // ones always.
                let edges = if *plain || selected || pre {
                    points.len()
                } else {
                    0
                };
                for i in 0..edges {
                    let a = &points[i];
                    let b = &points[(i + 1) % points.len()];
                    outlines.insert(
                        *id,
                        indices[i],
                        indices[(i + 1) % indices.len()],
                        outline::Edge {
                            ends: [*a, *b],
                            gradient,
                            color,
                            priority: if selected {
                                2
                            } else if pre {
                                1
                            } else {
                                0
                            },
                        },
                    );
                }
            }
            // Subdivision surfaces (SDV-04/12): smooth polygons fill with
            // their display-level surface; selection and rollover follow
            // the cage polygon each piece comes from.
            {
                let item_picked: std::collections::HashSet<u64> =
                    self.editor.scene.selected_items().into_iter().collect();
                for item in display_scene.items.iter().filter(|i| {
                    shown(i)
                        && !resident.contains(&i.id)
                        && operandi_core::subdivision::has_smooth(&i.mesh)
                }) {
                    let Some(look) = self.item_look(item.id) else {
                        continue;
                    };
                    let wireframe = look.style == view_display::Style::Wireframe;
                    let plain = look.draws_wire();
                    let Some(surface) = self.subdivision_cache.get(item) else {
                        continue;
                    };
                    let Ok(matrix) = display_scene.world_matrix(item.id) else {
                        continue;
                    };
                    let (mesh, sources) = (&surface.mesh, &surface.sources);
                    let projector = self.camera.projector(r);
                    let world: Vec<DVec3> = mesh
                        .vertices
                        .iter()
                        .map(|&v| matrix.transform_point3(v))
                        .collect();
                    let screen: Vec<Option<(Pos2, f64)>> =
                        world.iter().map(|&p| projector.project(p)).collect();
                    // Refined point numbers are their own outline key space.
                    let trace_owner = item.id | (1 << 63);
                    let selection = &self.editor.scene.selection;
                    // Per-frame lookups, so the loop below does no searching
                    // or allocating for the (many) interior pieces.
                    let mut picked = vec![false; item.mesh.faces.len()];
                    if selection.mode == Mode::Polygon && selection.tag.is_none() {
                        for &f in selection.faces_of(item.id) {
                            if let Some(p) = picked.get_mut(f as usize) {
                                *p = true;
                            }
                        }
                    }
                    let whole = selection.mode == Mode::Item && item_picked.contains(&item.id);
                    let ndc: Vec<Option<[f32; 3]>> = screen
                        .iter()
                        .map(|p| {
                            p.map(|p| outline::vertex(&self.camera, r, p, Color32::WHITE).position)
                        })
                        .collect();
                    for (sf, face) in mesh.faces.iter().enumerate() {
                        let source = sources[sf];
                        if !operandi_core::subdivision::is_smooth_polygon(&item.mesh, source) {
                            continue;
                        }
                        let v = &face.vertices;
                        let n = v.len();
                        let mut normal = DVec3::ZERO;
                        let mut center = DVec3::ZERO;
                        let p0 = world[v[0] as usize];
                        for i in 0..n {
                            let p = world[v[i] as usize];
                            normal += (p - p0).cross(world[v[(i + 1) % n] as usize] - p0);
                            center += p;
                        }
                        let normal = normal.normalize_or_zero();
                        let center = center / n as f64;
                        if !wireframe
                            && normal.dot(if self.camera.view == View::Perspective {
                                eye - center
                            } else {
                                -forward
                            }) <= 0.
                        {
                            continue;
                        }
                        if v.iter().any(|&i| ndc[i as usize].is_none()) {
                            continue;
                        }
                        let selected = whole || picked[source];
                        let pre = hover.is_some_and(|(i, f, _)| i == item.id && f == source as u32)
                            && self.editor.live.is_none()
                            && polygon_rollovers;
                        if !wireframe {
                            let base = if selected {
                                palette::selection_fill()
                            } else if pre {
                                palette::polygon_rollover(&item.mesh, source)
                            } else {
                                palette::solid()
                            };
                            let fill = crate::display_shading::face_color(
                                look.style,
                                base,
                                normal.as_vec3(),
                                to_eye(center),
                                &view_basis,
                                &style_colors,
                                selected || pre,
                            );
                            for i in 1..n.saturating_sub(1) {
                                for j in [0, i, i + 1] {
                                    let position = ndc[v[j] as usize].expect("checked above");
                                    triangles.push(crate::gpu::Vertex {
                                        position,
                                        color: fill,
                                    });
                                }
                            }
                        }
                        // The cage's edges, traced over the surface (plain
                        // ones only where the overlay draws them).
                        if !surface.traced[sf].iter().any(|&t| t) || !(plain || selected || pre) {
                            continue;
                        }
                        let points: Vec<_> = v
                            .iter()
                            .map(|&i| screen[i as usize].expect("checked above"))
                            .collect();
                        let color = if selected {
                            palette::selection()
                        } else if pre {
                            palette::polygon_rollover(&item.mesh, source)
                        } else {
                            self.plain_wire(item.id)
                        };
                        let gradient = depth_gradient(&self.camera, r, &points);
                        for i in (0..n).filter(|&i| surface.traced[sf][i]) {
                            outlines.insert(
                                trace_owner,
                                v[i],
                                v[(i + 1) % n],
                                outline::Edge {
                                    ends: [points[i], points[(i + 1) % n]],
                                    gradient,
                                    color,
                                    priority: if selected {
                                        2
                                    } else if pre {
                                        1
                                    } else {
                                        0
                                    },
                                },
                            );
                        }
                    }
                }
            }
            // Straight line elements never enter the filled-face tessellator.
            let mut synthetic: u32 = 0x8000_0000;
            for item in display_scene
                .items
                .iter()
                .filter(|i| shown(i) && self.item_look(i.id).is_some())
            {
                // Only polygons with a representation can be lines.
                let maybe_lines = item
                    .mesh
                    .attributes
                    .representations
                    .len()
                    .min(item.mesh.faces.len());
                // The lines' numbers, from one pass over the list (a
                // subdivided mesh lists every polygon; reading each through
                // the mesh slot cost tens of milliseconds a frame).
                let lines: Vec<usize> = item.mesh.attributes.representations[..maybe_lines]
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| {
                        matches!(r, operandi_core::PolygonRepresentation::Polyline { .. })
                    })
                    .map(|(fi, _)| fi)
                    .collect();
                for fi in lines {
                    let s = &self.editor.scene.selection;
                    let selected = (s.item == Some(item.id) && s.mode == Mode::Item)
                        || (s.mode == Mode::Polygon && s.faces_of(item.id).contains(&(fi as u32)));
                    let pre = hover.is_some_and(|(id, f, _)| id == item.id && f == fi as u32);
                    let color = if selected {
                        palette::selection()
                    } else if pre {
                        palette::rollover()
                    } else {
                        // Set Curves Color (CC-30) overrides the plain wire.
                        self.curve_override()
                            .unwrap_or_else(|| self.plain_wire(item.id))
                    };
                    // Smooth curves draw their sampled shape; the keys are
                    // made up so samples never merge with real edges.
                    if let Some(operandi_core::PolygonRepresentation::Polyline { closed, curve }) =
                        item.mesh.attributes.representations.get(fi).copied()
                        && !curve.is_line()
                    {
                        let control: Vec<DVec3> = item.mesh.faces[fi]
                            .vertices
                            .iter()
                            .map(|&v| {
                                display_scene.world_point(item.id, item.mesh.vertices[v as usize])
                            })
                            .collect();
                        let mut samples = operandi_core::curves::sample(
                            &control,
                            curve,
                            closed,
                            operandi_core::curves::FREEZE_STEPS,
                        );
                        if closed && let Some(&first) = samples.first() {
                            samples.push(first);
                        }
                        let projected: Vec<_> =
                            samples.iter().map(|p| self.camera.project(*p, r)).collect();
                        for pair in projected.windows(2) {
                            if let [Some(qa), Some(qb)] = pair {
                                synthetic = synthetic.wrapping_add(2);
                                outlines.insert(
                                    item.id,
                                    synthetic,
                                    synthetic + 1,
                                    outline::Edge {
                                        ends: [*qa, *qb],
                                        gradient: Vec2::ZERO,
                                        color,
                                        priority: if selected { 2 } else { u8::from(pre) },
                                    },
                                );
                            }
                        }
                        continue;
                    }
                    for (a, b) in item.mesh.element_spans(fi) {
                        if let (Some(qa), Some(qb)) = (
                            self.camera.project(
                                display_scene.world_point(item.id, item.mesh.vertices[a as usize]),
                                r,
                            ),
                            self.camera.project(
                                display_scene.world_point(item.id, item.mesh.vertices[b as usize]),
                                r,
                            ),
                        ) {
                            outlines.insert(
                                item.id,
                                a,
                                b,
                                outline::Edge {
                                    ends: [qa, qb],
                                    gradient: Vec2::ZERO,
                                    color,
                                    priority: if selected {
                                        2
                                    } else if pre {
                                        1
                                    } else {
                                        0
                                    },
                                },
                            );
                        }
                    }
                }
            }
            // Approved temporary flat drawing display, separate from scene
            // topology/history/persistence. Exact reference flat mesh is open.
            if let Some(live) = &self.editor.live
                && let Some(radial) = live.params.radial
                && let Some(normal) = (0..3).find(|&axis| radial.radius[axis] == 0.)
            {
                let u = (normal + 1) % 3;
                let v = (normal + 2) % 3;
                let ring: Option<Vec<_>> = (0..64)
                    .map(|i| {
                        let angle = std::f64::consts::TAU * i as f64 / 64.;
                        let mut point = live.params.position;
                        point[u] += radial.radius[u] * angle.cos();
                        point[v] += radial.radius[v] * angle.sin();
                        self.camera.project(point, r)
                    })
                    .collect();
                if let (Some(ring), Some(center)) =
                    (ring, self.camera.project(live.params.position, r))
                {
                    for i in 0..ring.len() {
                        for point in [center, ring[i], ring[(i + 1) % ring.len()]] {
                            triangles.push(outline::vertex(
                                &self.camera,
                                r,
                                point,
                                palette::selection(),
                            ));
                        }
                    }
                }
            }
            // MO-27 ghost: in Edit Base Mesh, the stack's full result drawn
            // as a see-through wire over the Base Mesh (provisional style).
            if let Some(base) = self.editor.base_edit
                && let Some(item) = self.editor.scene.item(base)
                && display_scene.shows(item)
                && let Some(result) = self.editor.meshops.result(&self.editor.scene, item)
            {
                const GHOST_EDGE_BUDGET: usize = 200_000;
                let ghost = u64::MAX - base;
                let color = crate::mesh_ops_ui::ghost_color();
                let edges = result.mesh.edges();
                for &(a, b) in edges.iter().take(GHOST_EDGE_BUDGET) {
                    let world =
                        |v: u32| display_scene.world_point(base, result.mesh.vertices[v as usize]);
                    if let (Some(qa), Some(qb)) = (
                        self.camera.project(world(a), r),
                        self.camera.project(world(b), r),
                    ) {
                        outlines.insert(
                            ghost,
                            a,
                            b,
                            outline::Edge {
                                ends: [qa, qb],
                                gradient: Vec2::ZERO,
                                color,
                                priority: 0,
                            },
                        );
                    }
                }
            }
            let wide_wire = self.wireframe_view();
            let edges = outlines.tessellate(&self.camera, r, wide_wire);
            // The cached items for this view, and the rollover overlay.
            // A component transform moving the whole mesh together draws the
            // mesh as it was before the drag through the drag's map, so
            // nothing is rebuilt per frame (`whole_mesh_drag`).
            // Locked or hidden geometry stays put while the rest moves, so
            // such a mesh can't be drawn through one matrix (LCK-20).
            let whole = resident_on
                .then(|| self.whole_mesh_drag(display_scene))
                .flatten()
                .filter(|(base, _)| base.elements.is_empty());
            crate::gpu_scene::set_show_locked(self.show_locked);
            // HT-5/HT-6: the geometry carries the tints some view shows
            // (it is shared by every view); each view's look picks its own.
            crate::gpu_scene::set_tints(self.view_tints());
            // Graphics memory stand-ins (gpu_memory.rs) always draw as
            // boxes, and the card lets go of their geometry. Bounding Box
            // Threshold: near the memory budget, more items draw as boxes
            // while the view is navigated.
            let over_cap = self.gpu_memory.pressed()
                && self.viewport_preferences.box_threshold
                    != crate::viewport_preferences::BoxThreshold::None
                && matches!(
                    self.navigation.as_ref().map(|g| &g.kind),
                    Some(input::GestureKind::Navigate(_))
                );
            let stand_ins = &self.gpu_memory.stand_ins;
            let boxed: std::collections::BTreeSet<u64> = display_scene
                .items
                .iter()
                .map(|i| i.id)
                .filter(|id| stand_ins.contains(id) || (over_cap && self.draws_as_box(*id, true)))
                .collect();
            let held: std::collections::BTreeSet<u64> =
                resident.difference(stand_ins).copied().collect();
            // Boxed items' wire colour (inactive ones in Inactive Wireframe).
            let box_wire: std::collections::BTreeMap<u64, Color32> =
                boxed.iter().map(|&id| (id, self.plain_wire(id))).collect();
            let radii = self.point_radii();
            // Show Selections as drawn now (off while a brush hides it).
            let selections_drawn = self.shows_selections();
            let world = resident_on.then(|| {
                // A point or edge tool started from another mode shows the
                // elements it works on, on the smooth surface where there
                // is one, as that mode's selection (SI-17, SI-Q05,
                // provisional: the ordinary selected look).
                let inferred_view;
                let selection = match self.editor.inferred_mode() {
                    Some(mode) => {
                        inferred_view = operandi_core::Selection {
                            mode,
                            ..self.editor.scene.selection.clone()
                        };
                        &inferred_view
                    }
                    None => &self.editor.scene.selection,
                };
                let mode = selection.mode;
                let looks =
                    crate::gpu::Look::pair(&self.display, mode).map(|look| crate::gpu::Look {
                        fade_small_wire: self.viewport_preferences.fade_small_wire,
                        checker: self.display.settings.drawing.checker,
                        weights: self.display.settings.drawing.weights,
                        distortion: self.display.settings.drawing.distortion,
                        material_transparency: self.viewport_preferences.material_transparency,
                        ..look
                    });
                // MT7 Topology Mode: the active group's own look
                // (topology_mode.rs).
                let topology = self.display.settings.drawing.topology;
                let mut looks = looks;
                if topology {
                    looks[0] = crate::topology_mode::topology_look(
                        looks[0],
                        self.viewport_preferences.topology_opacity,
                    );
                }
                // The view's own light (camera-fixed, VS-2); scene lights
                // only with Show Scene Lighting on.
                let (key_light, shadows) = {
                    let (_, right, up, forward) = self.camera.basis();
                    let head = crate::display_shading::ViewBasis::new(
                        right.as_vec3(),
                        up.as_vec3(),
                        forward.as_vec3(),
                    )
                    .headlight();
                    crate::display_shading::key_light(
                        display_scene,
                        self.display.settings.drawing.scene_lighting,
                        crate::gpu::KeyLight::Toward(head.to_array()),
                    )
                };
                let mut draw = crate::gpu::WorldDraw {
                    camera: {
                        let mut camera =
                            crate::gpu_scene::camera_uniform(&self.camera, r, wide_wire);
                        camera[22] = crate::gpu_scene::plain_wire_width(
                            wide_wire,
                            ui.ctx().pixels_per_point(),
                        );
                        camera
                    },
                    looks,
                    items: Vec::new(),
                    overlay: Default::default(),
                    overlay_look: crate::gpu::LookGroup::Active,
                    dots: Vec::new(),
                    seams: Vec::new(),
                    key_light,
                    shadows: shadows
                        && self.display.settings.drawing.shadows
                        && looks.iter().any(|l| l.style.shadows()),
                    xray_items: self.editor.scene.shown_active_meshes(),
                    xray_edges: Vec::new(),
                    outline: selections_drawn && self.display.settings.drawing.selected_outline,
                    materials: None,
                    env_picture: None,
                    whole_selected: Default::default(),
                    topology,
                };
                if let Some((env, picture)) = env_light.clone() {
                    for look in &mut draw.looks {
                        look.env = env;
                    }
                    draw.env_picture = Some(picture);
                }
                // Selection x-ray opacity (0 = off) rides in the camera
                // uniform's spare slot.
                draw.camera[23] = self.selection_preferences.xray_opacity();
                // Defer Smoothing During Interaction: flat while a tool drag
                // changes the mesh.
                let all_flat = self.mesh_items_preferences.defer_smoothing
                    && self.navigation.is_some()
                    && self.editor.live.is_some();
                self.resident_cache.sync_materials(display_scene, all_flat);
                // While a tool or drag is live, an item whose placement alone
                // changed reuses its cached geometry through a matrix on the
                // graphics card (gpu_scene.rs `delta`); it is rebuilt exactly
                // once the tool is dropped.
                let live = self.editor.live.is_some() || self.navigation.is_some();
                let placed = |item: &'_ operandi_core::Item, matrix: glam::DMat4| match &whole {
                    Some((base, map)) if base.id == item.id => (base.clone(), matrix * *map),
                    _ => (item.clone(), matrix),
                };
                // Every selected mesh draws as active (multi_active.rs).
                let active_ids = self.editor.scene.shown_active_meshes();
                let remembered = self.editor.scene.remembered_mesh();
                // Show Selections (Viewport Properties > Visibility).
                let shows_selections = selections_drawn;
                let shows_filling =
                    shows_selections && self.display.settings.drawing.selected_filling;
                let item_picked: std::collections::HashSet<u64> =
                    self.editor.scene.selected_items().into_iter().collect();
                for item in display_scene
                    .items
                    .iter()
                    .filter(|i| resident.contains(&i.id))
                {
                    let Ok(matrix) = display_scene.world_matrix(item.id) else {
                        continue;
                    };
                    if boxed.contains(&item.id) {
                        box_strokes(
                            &item.mesh,
                            matrix,
                            box_wire[&item.id],
                            &mut draw.overlay.edges,
                        );
                        continue;
                    }
                    let (item, matrix) = placed(item, matrix);
                    let item = &item;
                    // Subdivision polygons draw as patches on the graphics
                    // card when they can (subd_gpu.rs); the CPU surface is
                    // then only for the rollover, picking and overlays.
                    let patches = self
                        .viewport_gpu
                        .as_ref()
                        .filter(|_| crate::subd_gpu::on())
                        .and_then(|gpu| {
                            self.subdivision_cache
                                .patches(item)
                                .filter(|net| gpu.patches_fit(net))
                        });
                    let smooth = operandi_core::subdivision::has_smooth(&item.mesh)
                        .then(|| self.subdivision_cache.get(item))
                        .flatten();
                    // Only Item and Polygon selections color the mesh; in other
                    // modes the key stays the same, so selecting points or
                    // edges never rebuilds the mesh geometry.
                    // A tag selection mode draws an outline instead (below).
                    // Show Selections / Show Selected Filling off: no fill
                    // (two views that differ re-tint the selected polygons
                    // each frame; VP-Q06).
                    let picked = (shows_filling
                        && ((selection.mode == Mode::Item && item_picked.contains(&item.id))
                            || (selection.mode == Mode::Polygon
                                && selection.tag.is_none()
                                && !selection.faces_of(item.id).is_empty())))
                    .then(|| {
                        let faces = if selection.mode == Mode::Polygon {
                            selection.faces_of(item.id).iter().copied().collect()
                        } else {
                            Default::default()
                        };
                        // Symmetry pairs belong to the primary.
                        let mirror = if selection.item == Some(item.id) {
                            mirror_faces.iter().copied().collect()
                        } else {
                            Default::default()
                        };
                        let picked: (
                            bool,
                            std::collections::BTreeSet<u32>,
                            std::collections::BTreeSet<u32>,
                        ) = (selection.mode == Mode::Item, faces, mirror);
                        picked
                    });
                    // A whole selected item draws its unselected geometry
                    // with the selection applied on the graphics card
                    // (gpu.rs `Selected`), so selecting it or switching
                    // into Item mode rebuilds nothing. With symmetry pairs
                    // (mirror colours per polygon) it is built selected.
                    let whole = matches!(&picked, Some((true, _, mirror)) if mirror.is_empty())
                        && !crate::gpu_scene::reference_drawing();
                    let picked = if whole { None } else { picked };
                    if whole {
                        draw.whole_selected.insert(item.id);
                    }
                    let (geometry, generation, patch, placement) =
                        self.resident_cache.item_patched(
                            item,
                            matrix,
                            picked,
                            if patches.is_some() {
                                None
                            } else {
                                smooth.as_deref()
                            },
                            patches.as_ref(),
                            &display_scene.smoothing,
                            live,
                        );
                    // VP-04/05: every selected mesh item is active, and
                    // the last selected one while nothing is.
                    let group = if selection.has_item(item.id) || remembered == Some(item.id) {
                        crate::gpu::LookGroup::Active
                    } else if self.display.group(false).is_none() {
                        crate::gpu::LookGroup::Hidden
                    } else {
                        crate::gpu::LookGroup::Inactive
                    };
                    // An inactive mesh whose polygons are too small on screen
                    // for their wire keeps only its outline and sharp edges;
                    // the corners of its box show where it is (Fade
                    // Wireframe on Small Polygons, Jason 2026-11-13).
                    let inactive = &draw.looks[1];
                    if group == crate::gpu::LookGroup::Inactive
                        && inactive.fade_small_wire
                        && inactive.style != crate::view_display::Style::Wireframe
                        && inactive.overlay != crate::view_display::Overlay::None
                        && item.mesh.faces.len() >= DENSE_BOX_POLYGONS
                        && let Some((lo, hi)) = item.mesh.shared().bounds()
                        && too_dense_for_wire(
                            &self.camera.projector(r),
                            lo,
                            hi,
                            matrix,
                            item.mesh.faces.len(),
                        )
                    {
                        box_corner_strokes(
                            lo,
                            hi,
                            matrix,
                            crate::theme::viewport::dense_mesh_box(),
                            &mut draw.overlay.edges,
                        );
                    }
                    draw.items
                        .push((item.id, generation, geometry, patch, group, placement));
                    if self.display.settings.drawing.uv_seams
                        && group != crate::gpu::LookGroup::Hidden
                    {
                        let (seams, generation, placement) =
                            self.resident_cache.seams(item, matrix, live);
                        draw.seams.push((item.id, generation, seams, placement));
                    }
                    if let Some((id, f, _)) = hover
                        && id == item.id
                        && self.editor.live.is_none()
                        && polygon_rollovers
                    {
                        draw.overlay_look = group;
                        crate::gpu_scene::rollover(
                            item,
                            matrix,
                            f as usize,
                            smooth.as_deref(),
                            &mut draw.overlay,
                        );
                    }
                    // PB-110: the material group a dragged material would
                    // land on, highlighted as a whole.
                    if let Some(drop) = &self.preset_browser.drop_hover
                        && drop.item == item.id
                    {
                        draw.overlay_look = group;
                        for &f in &drop.faces {
                            crate::gpu_scene::rollover(
                                item,
                                matrix,
                                f,
                                smooth.as_deref(),
                                &mut draw.overlay,
                            );
                        }
                    }
                }
                // Image layers the items' polygons asked for, drawn per pixel.
                draw.materials = self.resident_cache.gpu_materials();
                // Each active mesh's point dots, depth-tested on the graphics
                // card instead of a ray cast per point per frame.
                for item in display_scene
                    .items
                    .iter()
                    .filter(|i| active_ids.contains(&i.id) && resident.contains(&i.id))
                {
                    let Ok(matrix) = display_scene.world_matrix(item.id) else {
                        continue;
                    };
                    let (item, matrix) = placed(item, matrix);
                    let item = &item;
                    let mirror = if selection.item == Some(item.id) {
                        mirror_vertices.iter().copied().collect()
                    } else {
                        Default::default()
                    };
                    let (dots, generation, placement) = self.resident_cache.dots(
                        item,
                        matrix,
                        selection.mode == Mode::Vertex,
                        self.display.draws_points(selection.mode),
                        // LCK-23: locked points show in Vertex and Edge mode.
                        matches!(selection.mode, Mode::Vertex | Mode::Edge),
                        // Show Selections off: no point shows as selected.
                        if shows_selections {
                            selection.vertices_of(item.id)
                        } else {
                            &[]
                        },
                        &mirror,
                        radii,
                        live,
                        &|| self.subdivision_cache.shown_points(item),
                        topology.then(|| {
                            let (eye, _, _, forward) = self.camera.basis();
                            crate::gpu_scene::TopologyDots {
                                color: crate::theme::color(crate::theme::ColorId::TopoVertices),
                                hide_back: self
                                    .viewport_preferences
                                    .topology_hide_back_points
                                    .then(|| {
                                        let perspective =
                                            self.camera.view == crate::camera::View::Perspective;
                                        (if perspective { eye } else { forward }, perspective)
                                    }),
                            }
                        }),
                    );
                    // Selected points come last (six corners each); only
                    // Vertex mode draws every selected one.
                    let mut xray_from = u32::MAX;
                    if selection.mode == Mode::Vertex && shows_selections {
                        let n = item.mesh.vertices.len();
                        let mut seen = vec![false; n];
                        let selected = selection
                            .vertices_of(item.id)
                            .iter()
                            .filter(|&&v| {
                                (v as usize) < n && !std::mem::replace(&mut seen[v as usize], true)
                            })
                            .count();
                        xray_from = (dots.len().saturating_sub(selected * 6)) as u32;
                    }
                    draw.dots
                        .push((item.id, generation, dots, placement, xray_from));
                }
                // Edge mode: the active item's creased and selected edges,
                // depth-tested on the graphics card like the points (a CPU
                // hidden-line test needs the picking surface, which a drag
                // changes every frame). The pre-highlighted edge stays on
                // the CPU path below.
                for item in display_scene.items.iter().filter(|i| {
                    selection.mode == Mode::Edge
                        && active_ids.contains(&i.id)
                        && resident.contains(&i.id)
                }) {
                    let Ok(matrix) = display_scene.world_matrix(item.id) else {
                        continue;
                    };
                    let (item, matrix) = placed(item, matrix);
                    let item = &item;
                    // On the smooth surface where the edge has one (the
                    // cage floats off it), else on the cage.
                    let weights: BTreeMap<(u32, u32), f32> = item
                        .mesh
                        .attributes
                        .creases
                        .iter()
                        .map(|c| {
                            (
                                (c.a.min(c.b), c.a.max(c.b)),
                                c.weight.clamp(-1., 1.).abs() as f32,
                            )
                        })
                        .filter(|&(_, w)| w > 0.)
                        .collect();
                    let selected: std::collections::BTreeSet<(u32, u32)> = selection
                        .edges_of(item.id)
                        .iter()
                        .filter(|_| shows_selections)
                        .map(|&(a, b)| (a.min(b), a.max(b)))
                        .collect();
                    // LCK-23: locked edges in the locked colour.
                    let locked: std::collections::BTreeSet<(u32, u32)> = if self.show_locked {
                        operandi_core::Protection::of(&item.mesh, &item.elements)
                            .map(|p| {
                                p.locked_edges()
                                    .iter()
                                    .copied()
                                    .filter(|&(a, b)| !p.edge_hidden(a, b))
                                    .collect()
                            })
                            .unwrap_or_default()
                    } else {
                        Default::default()
                    };
                    let wanted = weights
                        .keys()
                        .copied()
                        .chain(selected.iter().copied())
                        .chain(locked.iter().copied())
                        .collect();
                    let paths = crate::subdivision_view::edge_segments(
                        &self.subdivision_cache,
                        item,
                        matrix,
                        &wanted,
                    );
                    let out = &mut draw.overlay.edges;
                    for (edge, segments) in &paths {
                        if let Some(&w) = weights.get(edge) {
                            for &(pa, pb) in segments {
                                crate::gpu_scene::stroke_width(
                                    out,
                                    pa,
                                    pb,
                                    1.5 + 2. * w,
                                    palette::edge_weight().gamma_multiply(0.35 + 0.65 * w),
                                );
                            }
                        }
                    }
                    for (edge, segments) in &paths {
                        if locked.contains(edge) {
                            for &(pa, pb) in segments {
                                crate::gpu_scene::stroke_width(out, pa, pb, 2., palette::locked());
                            }
                        }
                    }
                    for (edge, segments) in &paths {
                        if selected.contains(edge) {
                            for &(pa, pb) in segments {
                                crate::gpu_scene::stroke_width(
                                    out,
                                    pa,
                                    pb,
                                    4.,
                                    palette::edge_backing(),
                                );
                                crate::gpu_scene::stroke_width(
                                    out,
                                    pa,
                                    pb,
                                    2.,
                                    palette::selection(),
                                );
                                crate::gpu_scene::stroke_width(
                                    &mut draw.xray_edges,
                                    pa,
                                    pb,
                                    2.,
                                    palette::selection(),
                                );
                            }
                        }
                    }
                }
                // Tag selection modes: the selected regions' outline in the
                // selection colour, depth-tested like selected edges.
                if selection.tag.is_some()
                    && selection.mode == Mode::Polygon
                    && let Some(item) = display_scene
                        .items
                        .iter()
                        .find(|i| Some(i.id) == selection.item && resident.contains(&i.id))
                    && let Ok(matrix) = display_scene.world_matrix(item.id)
                {
                    let (item, matrix) = placed(item, matrix);
                    let item = &item;
                    let wanted = crate::tag_mode_ui::outline(
                        item,
                        &selection.faces,
                        self.editor.history_serial,
                    );
                    let paths = crate::subdivision_view::edge_segments(
                        &self.subdivision_cache,
                        item,
                        matrix,
                        &wanted,
                    );
                    for (_, segments) in &paths {
                        for &(pa, pb) in segments {
                            let out = &mut draw.overlay.edges;
                            crate::gpu_scene::stroke_width(
                                out,
                                pa,
                                pb,
                                4.,
                                palette::edge_backing(),
                            );
                            crate::gpu_scene::stroke_width(out, pa, pb, 2.5, palette::selection());
                            crate::gpu_scene::stroke_width(
                                &mut draw.xray_edges,
                                pa,
                                pb,
                                2.,
                                palette::selection(),
                            );
                        }
                    }
                }
                self.resident_cache.retain(&held);
                draw
            });
            if let Some(gpu) = &mut self.viewport_gpu {
                if resident_on {
                    gpu.retain_resident(&held);
                }
                let scale = ctx.pixels_per_point();
                let drawable = [
                    (r.width() * scale).round() as u32,
                    (r.height() * scale).round() as u32,
                ];
                if self.drawing_view == self.active_view && self.last_drawable != drawable {
                    let record = serde_json::json!({"logical_viewport":[r.width(),r.height()],"framebuffer_viewport":drawable,"pixels_per_point":scale,"native_pixels_per_point":ctx.input(|i|i.viewport().native_pixels_per_point),"gpu":self.gpu,"msaa_samples":gpu.sample_count(),"typography":crate::typography::diagnostics(ctx)});
                    let _ = std::fs::write(
                        self.environment.paths().runtime_viewport(),
                        record.to_string(),
                    );
                    self.last_drawable = drawable;
                }
                let texture = gpu.draw(
                    self.drawing_view,
                    drawable,
                    &triangles,
                    &edges,
                    &grids,
                    &backdrops,
                    sky.as_ref(),
                    world.as_ref(),
                );
                gpu.retain_backdrops(&self.backdrop_images.keys());
                gpu.retain_skies(&self.sky_cache.keys());
                painter.image(
                    texture,
                    r,
                    Rect::from_min_max(Pos2::ZERO, pos2(1., 1.)),
                    Color32::WHITE,
                );
            }
            // TL-71: a playblast records the view without its overlays.
            let hud = !self.playblasting();
            if hud {
                self.draw_backdrop_outline(&painter, r);
                self.draw_locators(&painter, r);
                self.draw_selection_normals(&painter, r);
                self.draw_item_labels(&painter, r);
                self.draw_item_extras(&painter, r);
                // CP-A06: the hovered item's Tooltip.
                if self
                    .editor
                    .scene
                    .items
                    .iter()
                    .any(|i| !i.common.extras.tooltip.is_empty())
                    && self.navigation.is_none()
                    && let Some(p) = ctx.pointer_hover_pos().filter(|p| r.contains(*p))
                    && let Some(tip) = self.item_tooltip(p, r)
                {
                    let galley = painter.layout_no_wrap(
                        tip,
                        FontId::proportional(12.),
                        theme::viewport::selection(),
                    );
                    let at = p + vec2(14., 14.);
                    painter.rect_filled(
                        Rect::from_min_size(at, galley.size()).expand(3.),
                        3.,
                        theme::viewport::colored_wire(),
                    );
                    painter.galley(at, galley, theme::viewport::selection());
                }
                self.draw_item_handles(&painter, r);
                self.draw_camera_overlay(&painter, r);
                self.draw_render_frame(&painter, r);
                self.draw_overlays(&painter, r);
                // Window > Frame Time, or while an orbit benchmark runs.
                if self.show_frame_time || self.orbit_bench.is_some() {
                    painter.text(
                        r.right_top() + vec2(-10., 10.),
                        Align2::RIGHT_TOP,
                        format!(
                            "{}\n{}",
                            self.frame_stats.label(),
                            self.frame_stats.rebuilds_label()
                        ),
                        FontId::monospace(12.),
                        theme::text(),
                    );
                }
            }
            let hovered_component = if self.editor.live.is_none() && self.rollovers_on() {
                response
                    .hover_pos()
                    .and_then(|p| self.rollover_component_on(p, r, &surface()))
            } else {
                None
            };
            // Every shown active mesh draws its points and edges
            // (multi_active.rs).
            let mut shown_active = self.editor.scene.shown_active_meshes();
            if !hud {
                shown_active.clear();
            }
            for item in self
                .editor
                .scene
                .items
                .iter()
                .filter(|i| shown_active.contains(&i.id))
            {
                let s = &self.editor.scene.selection;
                {
                    // Find loose points once per snapshot, not by scanning
                    // every element for every point in surface/edge modes.
                    let all_points = self.display.draws_points(s.mode);
                    // Drawn on the graphics card (all but the rollover): then
                    // only the rollover point is looked at here, instead of
                    // every point and polygon on every frame.
                    let on_gpu = resident.contains(&item.id);
                    let count = item.mesh.vertices.len();
                    let candidates: Vec<usize> = if on_gpu {
                        match hovered_component {
                            Some((id, Component::Vertex(h)))
                                if id == item.id && (h as usize) < count =>
                            {
                                vec![h as usize]
                            }
                            _ => Vec::new(),
                        }
                    } else {
                        (0..count).collect()
                    };
                    let needs_masks = !on_gpu && !candidates.is_empty();
                    let used_mask =
                        (needs_masks && s.mode != Mode::Vertex && !all_points).then(|| {
                            let mut used = vec![false; count];
                            for face in &item.mesh.faces {
                                for &v in &face.vertices {
                                    used[v as usize] = true;
                                }
                            }
                            used
                        });
                    let around =
                        (!needs_masks && !candidates.is_empty()).then(|| item.mesh.point_faces());
                    let connected = |i: usize| {
                        if s.mode == Mode::Vertex || all_points {
                            false
                        } else if let Some(used) = &used_mask {
                            used[i]
                        } else {
                            around.as_ref().is_some_and(|a| !a.of(i as u32).is_empty())
                        }
                    };
                    let selected_mask =
                        needs_masks.then(|| selected_vertex_mask(count, s.vertices_of(item.id)));
                    let is_selected = |i: usize| match &selected_mask {
                        Some(mask) => mask[i],
                        None => s.vertices_of(item.id).contains(&(i as u32)),
                    };
                    let shown = (!candidates.is_empty()).then(|| self.shown_points(item));
                    for i in candidates {
                        let v = shown.as_ref().map_or(item.mesh.vertices[i], |s| s[i]);
                        let selected_i = is_selected(i);
                        // MV-C03: with Show Vertices off, Vertex mode draws
                        // only the selected and pre-highlighted points.
                        let hovered = matches!(hovered_component,Some((id,Component::Vertex(v))) if id==item.id&&v==i as u32);
                        if !hovered && connected(i) {
                            continue;
                        }
                        if s.mode == Mode::Vertex && !all_points && !selected_i && !hovered {
                            continue;
                        }
                        if surface().visible(&self.camera, display_scene.world_point(item.id, v), r)
                            && let Some(p) = project(display_scene.world_point(item.id, v))
                        {
                            painter.circle_filled(
                                    p,
                                    if selected_i {
                                        self.selection_preferences.selected_point_radius()
                                    } else {
                                        self.selection_preferences.point_radius()
                                    },
                                    if matches!(hovered_component,Some((id,Component::Vertex(v))) if id==item.id&&v==i as u32) {palette::rollover()}
                                    else if selected_i && self.editor.scene.selection.item == Some(item.id) && mirror_vertices.contains(&(i as u32)) {
                                        palette::mirror_selection()
                                    } else if selected_i {
                                        palette::selection()
                                    } else {
                                        palette::wireframe()
                                    },
                                );
                        }
                    }
                }
                // Closest rollovers outside Edge mode: the edge under the
                // pointer, on top (Edge mode draws its own below).
                if s.mode != Mode::Edge
                    && let Some((id, Component::Edge(a, b))) = hovered_component
                    && id == item.id
                    && let Ok(matrix) = display_scene.world_matrix(item.id)
                {
                    // On the smooth surface where the edge borders one.
                    let wanted = [(a.min(b), a.max(b))].into();
                    for (_, segments) in crate::subdivision_view::edge_segments(
                        &self.subdivision_cache,
                        item,
                        matrix,
                        &wanted,
                    ) {
                        for (wa, wb) in segments {
                            if let (Some(pa), Some(pb)) = (project(wa), project(wb)) {
                                painter.line_segment(
                                    [pa, pb],
                                    Stroke::new(4_f32, palette::edge_backing()),
                                );
                                painter.line_segment(
                                    [pa, pb],
                                    Stroke::new(2_f32, palette::rollover()),
                                );
                            }
                        }
                    }
                }
                if s.mode == Mode::Edge {
                    // EW-08: weighted edges, brighter and wider with weight,
                    // then the selected and pre-highlighted edges. On the
                    // smooth surface where the edge borders one (the cage
                    // floats off it). Items on the graphics card drew the
                    // weighted and selected ones above.
                    let on_gpu = resident.contains(&item.id);
                    let weights: BTreeMap<(u32, u32), f32> = item
                        .mesh
                        .attributes
                        .creases
                        .iter()
                        .filter(|_| !on_gpu)
                        .map(|c| {
                            (
                                (c.a.min(c.b), c.a.max(c.b)),
                                c.weight.clamp(-1., 1.).abs() as f32,
                            )
                        })
                        .filter(|&(_, w)| w > 0.)
                        .collect();
                    let pre = match hovered_component {
                        Some((id, Component::Edge(x, y))) if id == item.id => {
                            Some((x.min(y), x.max(y)))
                        }
                        _ => None,
                    };
                    let selected: std::collections::BTreeSet<(u32, u32)> = s
                        .edges_of(item.id)
                        .iter()
                        .filter(|_| !on_gpu)
                        .map(|&(a, b)| (a.min(b), a.max(b)))
                        .collect();
                    let wanted = weights
                        .keys()
                        .copied()
                        .chain(selected.iter().copied())
                        .chain(pre)
                        .collect();
                    let paths = match display_scene.world_matrix(item.id) {
                        Ok(matrix) => crate::subdivision_view::edge_segments(
                            &self.subdivision_cache,
                            item,
                            matrix,
                            &wanted,
                        ),
                        Err(_) => Vec::new(),
                    };
                    // Hidden-line test on the cage edge's midpoint (the
                    // picking surface is the cage).
                    let shown = |edge: (u32, u32)| {
                        let va =
                            display_scene.world_point(item.id, item.mesh.vertices[edge.0 as usize]);
                        let vb =
                            display_scene.world_point(item.id, item.mesh.vertices[edge.1 as usize]);
                        surface().visible(&self.camera, va.lerp(vb, 0.5), r)
                    };
                    for (edge, segments) in &paths {
                        if let Some(&w) = weights.get(edge)
                            && shown(*edge)
                        {
                            for &(va, vb) in segments {
                                segment(
                                    va,
                                    vb,
                                    palette::edge_weight().gamma_multiply(0.35 + 0.65 * w),
                                    1.5 + 2. * w,
                                );
                            }
                        }
                    }
                    for (edge, segments) in &paths {
                        let pre = pre == Some(*edge);
                        if !pre && !selected.contains(edge) {
                            continue;
                        }
                        let color = if pre {
                            palette::rollover()
                        } else {
                            palette::selection()
                        };
                        if let [(va, vb)] = segments[..] {
                            // A cage edge: hidden-line test along it.
                            for step in 0..32 {
                                let t = step as f64 / 32.;
                                let u = (step + 1) as f64 / 32.;
                                if surface().visible(&self.camera, va.lerp(vb, (t + u) * 0.5), r) {
                                    segment(
                                        va.lerp(vb, t),
                                        va.lerp(vb, u),
                                        palette::edge_backing(),
                                        4.,
                                    );
                                    segment(va.lerp(vb, t), va.lerp(vb, u), color, 2.);
                                }
                            }
                        } else if shown(*edge) {
                            for &(va, vb) in segments {
                                segment(va, vb, palette::edge_backing(), 4.);
                                segment(va, vb, color, 2.);
                            }
                        }
                    }
                }
            }
            // TWK-02: with Tweak live, what a press would grab.
            if self.navigation.is_none()
                && self
                    .editor
                    .live
                    .as_ref()
                    .is_some_and(|l| l.tool == Tool::Tweak)
            {
                self.draw_tweak_hover(&painter, response.hover_pos(), r);
            }
            if self.topology_pen_open() {
                self.draw_topology_pen(&painter, response.hover_pos(), r);
            }
            if self.navigation.is_none()
                && !self
                    .editor
                    .live
                    .as_ref()
                    .is_some_and(|l| l.tool == Tool::Tweak)
                && self.element_falloff().is_some()
                && self
                    .editor
                    .live
                    .as_ref()
                    // Element Push highlights what a press would push.
                    .is_some_and(|l| l.tool.is_transform() || l.tool == Tool::Push)
            {
                // FO-45: what a press would pick.
                self.draw_tweak_hover(&painter, response.hover_pos(), r);
            }
            // FO-43/44/52: the Screen circle, Lasso outline, Airbrush brush.
            if self
                .navigation
                .as_ref()
                .is_none_or(|g| !matches!(g.kind, input::GestureKind::Navigate(_)))
            {
                self.draw_screen_falloff(&painter, response.hover_pos(), r);
            }
            // SC-10..13: the brush circle follows the pointer.
            if self.sculpt.brush.is_some() {
                self.draw_sculpt_cursor(&painter, response.hover_pos(), r);
            }
            #[cfg(feature = "painting")]
            self.draw_paint_cursor(&painter, response.hover_pos(), r);
            // Pen tablet debug readout (OPERANDI_PEN_DEBUG=1; off by
            // default): live pressure, tilt and pen end.
            if crate::tablet::readout_enabled() {
                painter.text(
                    r.left_bottom() + vec2(8., -8.),
                    Align2::LEFT_BOTTOM,
                    crate::tablet::readout(),
                    FontId::monospace(11.),
                    crate::theme::viewport::tweak_range(),
                );
                ui.ctx().request_repaint();
            }
            // TWR-11/12: the Range sphere shows only while a right-drag
            // resizes it, around where the drag started.
            if let Some(input::GestureKind::TweakRange { center, .. }) =
                self.navigation.as_ref().map(|g| &g.kind)
            {
                self.draw_tweak_range(&painter, *center, r);
            }
            if self.plane_widget {
                let origin = r.left_bottom() + vec2(43., -48.);
                let (_, right, up, _) = self.camera.basis();
                let project_axis =
                    |a: DVec3| vec2(a.dot(right) as f32 * 26., -a.dot(up) as f32 * 26.);
                let (a, b) = match plane {
                    1 => (DVec3::Y, DVec3::Z),
                    2 => (DVec3::X, DVec3::Y),
                    _ => (DVec3::X, DVec3::Z),
                };
                painter.add(Shape::convex_polygon(
                    vec![
                        origin,
                        origin + project_axis(a),
                        origin + project_axis(a + b),
                        origin + project_axis(b),
                    ],
                    palette::work_plane().gamma_multiply(36. / 255.),
                    Stroke::new(1.0f32, palette::work_plane().gamma_multiply(179. / 255.)),
                ));
                for (i, a) in [DVec3::X, DVec3::Y, DVec3::Z].into_iter().enumerate() {
                    let end = origin + project_axis(a);
                    painter.arrow(
                        origin,
                        end - origin,
                        Stroke::new(1.5f32, palette::grid_axes()[i]),
                    );
                    painter.text(
                        end + vec2(3., -3.),
                        Align2::LEFT_BOTTOM,
                        ["X", "Y", "Z"][i],
                        FontId::proportional(11.),
                        palette::grid_axes()[i],
                    );
                }
                painter.text(
                    r.left_bottom() + vec2(12., -8.),
                    Align2::LEFT_BOTTOM,
                    format!(
                        "{} · {}",
                        ["XZ", "YZ", "XY"][plane as usize],
                        if self.editor.scene.workplane.fixed {
                            "Fixed"
                        } else {
                            "Automatic"
                        }
                    ),
                    FontId::proportional(11.),
                    palette::label(),
                );
            }
            // The flat base's height handle (session_draw): drag it up.
            if let Some(handle) = self.session_height_handle(r) {
                painter.rect_filled(
                    Rect::from_center_size(handle, vec2(9., 9.)),
                    1.,
                    Color32::from_rgb(80, 200, 90),
                );
            }
            // Show View Information (off by default) shows this text, and
            // Show Viewport Controls and Information off hides it too.
            if self.display.settings.drawing.controls && self.display.settings.drawing.info {
                painter.text(
                    r.left_top() + vec2(10., 10. + viewport_buttons::HEADER_CLEARANCE),
                    Align2::LEFT_TOP,
                    format!(
                        "{} · {} {} · {}",
                        self.view_title(),
                        if self.editor.scene.workplane.fixed {
                            "Fixed"
                        } else {
                            "Automatic"
                        },
                        ["XZ", "YZ", "XY"][plane as usize],
                        grid_label
                    ),
                    FontId::proportional(11.),
                    palette::label(),
                );
                let selection = &self.editor.scene.selection;
                let count = match selection.mode {
                    Mode::Vertex | Mode::Edge | Mode::Polygon => selection.component_count(),
                    Mode::Item => usize::from(selection.item.is_some()),
                };
                painter.text(
                    r.left_top() + vec2(10., 28. + viewport_buttons::HEADER_CLEARANCE),
                    Align2::LEFT_TOP,
                    format!(
                        "{} · {} selected {:?} · {}",
                        self.editor
                            .scene
                            .active()
                            .map(|i| i.name.as_str())
                            .unwrap_or("No mesh"),
                        count,
                        selection.mode,
                        self.editor.live.as_ref().map(|l| l.tool.label()).unwrap_or(
                            if self.editor.reduction.is_some() {
                                "Polygon Reduction · numeric"
                            } else if self.editor.bridge.is_some() {
                                "Bridge · numeric"
                            } else if self.editor.spike.is_some() {
                                "Spike · numeric"
                            } else if let Some(s) = &self.editor.shape {
                                match s.kind() {
                                    ShapeKind::Torus => "Torus · numeric",
                                    ShapeKind::Capsule => "Capsule · numeric",
                                    ShapeKind::Arc => "Arc · numeric",
                                    ShapeKind::Tube => "Tube · numeric",
                                    ShapeKind::Text => "Text · numeric",
                                    ShapeKind::Teapot => "Teapot · numeric",
                                }
                            } else if self.editor.ellipsoid.is_some() {
                                "Ellipsoid · numeric"
                            } else if self.editor.cone.is_some() {
                                "Cone · numeric"
                            } else {
                                "Selection"
                            }
                        )
                    ),
                    FontId::proportional(11.),
                    palette::label(),
                );
            }
            // How to finish or leave the active tool, with the user's own
            // keys (tool_exit_hint.rs); live tools lead with what they do.
            let exit = self.tool_exit_hint();
            let lead = self.editor.live.as_ref().map(|l| {
                if matches!(l.tool, Tool::Bevel | Tool::TwistBevel | Tool::Extrude) {
                    "Handles edit this stage · Shift-click continues on caps"
                } else {
                    "Live tool · drag handles or edit properties"
                }
            });
            let line = match (lead, exit) {
                (Some(lead), Some(exit)) => Some(format!("{lead} · {exit}")),
                (Some(lead), None) => Some(lead.to_owned()),
                (None, exit) => exit,
            };
            if let Some(line) = line {
                painter.text(
                    r.left_bottom() + vec2(100., -16.),
                    Align2::LEFT_BOTTOM,
                    line,
                    FontId::proportional(11.),
                    palette::label(),
                );
            }
            self.draw_falloff(&painter, r);
            self.draw_bend(
                &painter,
                r,
                response.hover_pos().filter(|_| self.navigation.is_none()),
            );
            // UIC-05: Bend's handles say what dragging them does.
            if let Some(text) = response
                .hover_pos()
                .filter(|_| self.navigation.is_none())
                .and_then(|p| self.bend_tooltip(p, r))
            {
                response.clone().on_hover_text(text);
            }
            // MN-09.1: falloff, camera/light, Tweak and Click to Start.
            if let Some(text) = response
                .hover_pos()
                .filter(|_| self.navigation.is_none())
                .and_then(|p| self.other_handle_tip(p, r))
            {
                response.clone().on_hover_text(text);
            }
            self.draw_slice(&painter, r);
            self.draw_detriangulate_preview(&painter, r);
            self.draw_duplicate(&painter, r);
            // G9 GD-1: guide lines draw with any tool.
            self.draw_guides(&painter, r);
            self.draw_utility(&painter, r);
            self.draw_curve_tool(
                &painter,
                r,
                response.hover_pos().filter(|_| self.navigation.is_none()),
            );
            let sliding = self
                .navigation
                .as_ref()
                .is_some_and(|g| matches!(g.kind, input::GestureKind::EdgeInsert { .. }));
            if sliding && let Some((at, text)) = &self.edge_insert_label {
                painter.text(
                    *at + vec2(12., -14.),
                    Align2::LEFT_BOTTOM,
                    text,
                    FontId::proportional(13.),
                    palette::selection(),
                );
            }
            for (axis, origin, end, label) in self.handles(r) {
                let handle_rect = Rect::from_center_size(
                    end,
                    Vec2::splat(self.handle_preferences.point_hit_size),
                );
                if !self.view_passive {
                    ui.interact(handle_rect, Id::new(("live-handle", axis)), Sense::hover())
                        .widget_info(|| WidgetInfo::labeled(WidgetType::DragValue, true, &label));
                }
                let hovered = response
                    .hover_pos()
                    .is_some_and(|p| self.handle_hit(p, r) == Some(axis));
                // MN-09.2: the tooltip shows while hovered, not mid-drag.
                if hovered && self.navigation.is_none() {
                    let primitive_side = axis >= 3
                        && self
                            .editor
                            .live
                            .as_ref()
                            .is_some_and(|l| l.tool.is_primitive());
                    // What Ctrl does on this handle; it is read at the press
                    // (HM-01/12a).
                    let tool = self.editor.live.as_ref().map(|l| l.tool);
                    let family = match tool {
                        Some(Tool::Transform | Tool::EdgeExtend) if axis < 9 => Some(axis / 3),
                        Some(Tool::Rotate) if axis < 3 => Some(1),
                        Some(Tool::Scale) if axis < 3 => Some(2),
                        _ => None,
                    };
                    let ctrl_hint = if primitive_side {
                        ". Ctrl-drag: move the whole shape without resizing. Hold Ctrl as you press; it can't be added mid-drag."
                    } else if axis == FREE_HANDLE {
                        ". Ctrl-drag: keep to the work-plane axis you head toward first (Rotate: snap to Rotation Snap Angle)."
                    } else {
                        match family {
                            Some(1) => ". Ctrl-drag: turn in steps of Rotation Snap Angle.",
                            Some(2) => ". Ctrl-drag: scale all three axes equally.",
                            _ => "",
                        }
                    };
                    // PH-10: a plane circle's tooltip says what it does (UIC-05).
                    let text = match tool.filter(|_| is_plane_handle(axis)) {
                        Some(t) => Self::plane_handle_tooltip(self.plane_tool(t), axis),
                        // MN-09.1: the other tools' handles say what they
                        // are and what a drag changes.
                        None if tool.is_some_and(|t| {
                            !t.transforms_items()
                                && !t.is_primitive()
                                && t.card().handles.kind != HandleKind::None
                        }) =>
                        {
                            let t = tool.unwrap();
                            format!(
                                "{}{ctrl_hint}",
                                crate::handle_tips::live_tool_tip(t, t.card().handles.kind, axis)
                            )
                        }
                        None => format!(
                            "{} {label}{ctrl_hint}",
                            tool.map(|t| t.label()).unwrap_or("Handle")
                        ),
                    };
                    response.clone().on_hover_text(text);
                }
                let dragging = self.navigation.as_ref().is_some_and(|g| match &g.kind {
                    input::GestureKind::Handle(id, _, _) => *id == axis,
                    input::GestureKind::Primitive(primitive_input::Gesture::RadialSide(
                        a,
                        max,
                        _,
                        _,
                    )) => 3 + *a * 2 + usize::from(*max) == axis,
                    _ => false,
                });
                let fade = self.handle_fade(origin, end, r);
                if self
                    .editor
                    .live
                    .as_ref()
                    .is_some_and(|l| l.tool.card().handles.kind == HandleKind::Spine)
                {
                    self.draw_twist_bevel_handle(
                        &painter,
                        axis,
                        (origin, end),
                        hovered || dragging,
                        fade,
                        r,
                    );
                    continue;
                }
                if is_plane_handle(axis) {
                    self.draw_plane_handle(&painter, axis, r, hovered || dragging, fade);
                    continue;
                }
                if axis == FREE_HANDLE {
                    // The free handle: a small square where the axes meet.
                    let square = Rect::from_center_size(
                        end,
                        Vec2::splat(
                            self.handle_preferences.plane_point_size() * self.handle_zoom.get(),
                        ),
                    );
                    painter.rect_filled(
                        square,
                        1.,
                        if hovered || dragging {
                            palette::handle_active()
                        } else {
                            Color32::WHITE
                        }
                        .gamma_multiply(fade),
                    );
                    painter.rect_stroke(
                        square,
                        1.,
                        Stroke::new(1_f32, Color32::from_gray(40)),
                        egui::StrokeKind::Outside,
                    );
                    continue;
                }
                let uniform = axis == SCALE_UNIFORM_HANDLE
                    && self
                        .editor
                        .live
                        .as_ref()
                        .is_some_and(|l| l.tool == Tool::Scale);
                let color = if hovered || dragging {
                    palette::handle_active()
                } else if uniform {
                    palette::handles()[2]
                } else {
                    palette::handles()[if self.editor.live.as_ref().is_some_and(|l| {
                        // Handle 0 runs along the normal: blue.
                        matches!(
                            l.tool.card().handles.kind,
                            HandleKind::Pair | HandleKind::Normal
                        )
                    }) {
                        if axis == 0 { 2 } else { 0 }
                    } else if self
                        .editor
                        .live
                        .as_ref()
                        .is_some_and(|l| l.tool == Tool::Transform)
                    {
                        axis % 3
                    } else if self
                        .editor
                        .live
                        .as_ref()
                        .is_some_and(|l| l.tool == Tool::EdgeExtend)
                    {
                        // Local handles match Bevel's: red Inset, blue Shift.
                        match axis {
                            EXTEND_INSET_HANDLE => 0,
                            EXTEND_SHIFT_HANDLE => 2,
                            a => a % 3,
                        }
                    } else if axis < 3 {
                        axis
                    } else {
                        (axis - 3) / 2
                    }
                    .min(2)]
                };
                let color = color.gamma_multiply(fade);
                if self.editor.live.as_ref().is_some_and(|l| {
                    let kind = l.tool.card().handles.kind;
                    kind == HandleKind::Rings
                        || (kind == HandleKind::Combined && (3..6).contains(&axis))
                }) {
                    let ring = self.rotation_ring(axis % 3, r);
                    painter.add(Shape::line(
                        ring.clone(),
                        Stroke::new(self.handle_preferences.line_width, color),
                    ));
                    if ring.len() > 1 {
                        handle_arrow_head(
                            &painter,
                            ring[ring.len() - 2],
                            ring[ring.len() - 1],
                            color,
                        );
                    }
                } else {
                    if uniform {
                        // PH-31: a larger circle for uniform scale; only its
                        // rim takes the hover colour (PH-10).
                        let rim = if hovered || dragging {
                            palette::handle_active()
                        } else {
                            palette::handles()[2]
                        };
                        self.draw_scale_center(&painter, end, rim, fade);
                    } else if axis >= 3
                        || self.editor.live.as_ref().is_some_and(|l| {
                            l.tool == Tool::Scale
                                || (axis == 1 && l.tool.card().handles.kind == HandleKind::Pair)
                        })
                    {
                        painter.line_segment(
                            [origin, end],
                            Stroke::new(self.handle_preferences.line_width, color),
                        );
                        painter.rect_filled(
                            Rect::from_center_size(
                                end,
                                Vec2::splat(
                                    self.handle_preferences.large_point_size()
                                        * self.handle_zoom.get(),
                                ),
                            ),
                            0.,
                            color,
                        );
                    } else {
                        painter.line_segment(
                            [origin, end],
                            Stroke::new(self.handle_preferences.line_width, color),
                        );
                        handle_arrow_head(&painter, origin, end, color);
                    }
                }
                // Handle roles stay named for tooltips/accessibility; the
                // viewport draws no ink labels (user correction 2026-10-30).
            }
            // TP-19: the falloff's Size boxes sit on top of the tool's
            // handles, as they win the press where both are (FO-41).
            self.draw_falloff_size_boxes(&painter, r);
            if let Some(tip) = self.draw_precision_handles(
                &painter,
                r,
                response.hover_pos().filter(|_| self.navigation.is_none()),
            ) {
                response.clone().on_hover_text(tip);
            }
            if let Some(tip) = self.draw_solid_sketch(
                &painter,
                r,
                response.hover_pos().filter(|_| self.navigation.is_none()),
            ) {
                response.clone().on_hover_text(tip);
            }
            // SYM3-24: the mirror half Drop will add, drawn dimmer.
            if let Some(mirror) = self.editor.pen_mirror_preview() {
                let dim = palette::selection().gamma_multiply(0.45);
                for ring in &mirror.rings {
                    let mut screen: Vec<_> = ring.iter().filter_map(|&p| project(p)).collect();
                    if screen.len() == ring.len() && screen.len() >= 2 {
                        if !mirror.open {
                            screen.push(screen[0]);
                        }
                        painter.add(Shape::line(screen, Stroke::new(1.5_f32, dim)));
                    }
                }
                for &p in &mirror.points {
                    if let Some(q) = project(p) {
                        painter.circle_stroke(q, 3., Stroke::new(1_f32, dim));
                    }
                }
            }
            // Section 8 seam guide: the symmetry plane across the active
            // mesh, a thin axis-coloured line in orthographic views (seen
            // edge-on) and a faint grid-coloured outline in perspective.
            for ring in self.symmetry_seam_guide() {
                let screen: Vec<_> = ring.iter().filter_map(|&p| project(p)).collect();
                if screen.len() == ring.len() {
                    let colour = if self.camera.view == View::Perspective {
                        palette::grid().gamma_multiply(0.8)
                    } else {
                        palette::grid_axes()[self.editor.symmetry.axis.unwrap_or(0).min(2)]
                            .gamma_multiply(0.7)
                    };
                    let mut closed = screen;
                    closed.push(closed[0]);
                    painter.add(Shape::line(closed, Stroke::new(1_f32, colour)));
                }
            }
            // Points still without a mirror partner, as small yellow
            // squares (at most 10,000 drawn): the Symmetry Tool's, and the
            // unpaired warning when a tool starts (section 8).
            if self.editor.live.is_some()
                && self.viewport_preferences.show_asymmetries
                && let Some(item) = self.editor.scene.active()
                && let Ok(world) = self.editor.scene.world_matrix(item.id)
            {
                let colour = palette::selection();
                for &v in self.editor.symmetry_unpaired.iter().take(10_000) {
                    if let Some(q) = item
                        .mesh
                        .vertices
                        .get(v as usize)
                        .and_then(|&p| project(world.transform_point3(p)))
                    {
                        painter.rect_filled(Rect::from_center_size(q, vec2(6., 6.)), 0., colour);
                    }
                }
            }
            if let Some(draft) = &self.editor.pen_draft {
                let projected: Vec<_> = draft.points().iter().filter_map(|&p| project(p)).collect();
                if projected.len() == draft.points().len() {
                    let output = draft.options().output;
                    let wall = draft.options().wall != WallMode::Off;
                    let strip = draft.options().strip();
                    if strip {
                        // Make Quads previews the strip Drop will build: its
                        // polygons filled and outlined; no path line.
                        for polygon in operandi_core::pen_strip(draft.points(), false) {
                            let ring: Vec<_> =
                                polygon.iter().map(|&i| projected[i as usize]).collect();
                            for k in 1..ring.len() - 1 {
                                painter.add(Shape::convex_polygon(
                                    vec![ring[0], ring[k], ring[k + 1]],
                                    Color32::from_rgba_unmultiplied(225, 164, 54, 55),
                                    Stroke::NONE,
                                ));
                            }
                            let mut closed = ring.clone();
                            closed.push(ring[0]);
                            painter.add(Shape::line(
                                closed,
                                Stroke::new(1.5_f32, palette::selection()),
                            ));
                        }
                        if let [a, b] = projected[..] {
                            // The starting edge before the first quad.
                            painter
                                .line_segment([a, b], Stroke::new(1.5_f32, palette::selection()));
                        }
                    } else if wall {
                        // Pen walls preview the strip Drop will build.
                        let (points, quads) =
                            operandi_core::pen_wall(draft.points(), &draft.options())
                                .unwrap_or_default();
                        let screen: Vec<_> = points.iter().filter_map(|&p| project(p)).collect();
                        if screen.len() == points.len() {
                            for q in quads {
                                let quad: Vec<_> = q.iter().map(|&i| screen[i as usize]).collect();
                                for k in 1..quad.len() - 1 {
                                    painter.add(Shape::convex_polygon(
                                        vec![quad[0], quad[k], quad[k + 1]],
                                        Color32::from_rgba_unmultiplied(225, 164, 54, 55),
                                        Stroke::NONE,
                                    ));
                                }
                                let mut ring = quad.clone();
                                ring.push(quad[0]);
                                painter.add(Shape::line(
                                    ring,
                                    Stroke::new(1_f32, palette::selection()),
                                ));
                            }
                        }
                    }
                    if output.surface() && !wall && !strip && draft.points().len() >= 3 {
                        let mesh = operandi_core::Mesh {
                            vertices: draft.points().to_vec(),
                            faces: (vec![Face {
                                vertices: (0..draft.points().len() as u32).collect(),
                                uv: None,
                                material: String::new(),
                                smooth: false,
                                part: None,
                                material_label: None,
                            }])
                            .into(),
                            ..Default::default()
                        };
                        if let Ok(tris) = mesh.surface_triangles(0) {
                            for [a, b, c] in tris {
                                painter.add(Shape::convex_polygon(
                                    vec![projected[a], projected[b], projected[c]],
                                    Color32::from_rgba_unmultiplied(225, 164, 54, 55),
                                    Stroke::NONE,
                                ));
                            }
                        }
                    }
                    // Vertices draws points only; an open Polyline leaves
                    // its ends apart.
                    if output != PenType::Vertices && !strip {
                        for pair in projected.windows(2) {
                            painter.line_segment(
                                [pair[0], pair[1]],
                                Stroke::new(1.5_f32, palette::selection()),
                            );
                        }
                    }
                    let closed = match output {
                        PenType::Vertices => false,
                        PenType::Polyline | PenType::BSpline => draft.options().close,
                        _ => !wall && !strip,
                    };
                    if closed && projected.len() >= 3 {
                        painter.line_segment(
                            [*projected.last().unwrap(), projected[0]],
                            Stroke::new(1.5_f32, palette::selection()),
                        );
                    }
                    for point in projected {
                        painter.circle_filled(point, 4., palette::selection());
                    }
                }
            }
            let sketch_points = self.sketch_feedback_points();
            let sketch_screen: Vec<_> = sketch_points.iter().filter_map(|&p| project(p)).collect();
            for pair in sketch_screen.windows(2) {
                painter.line_segment([pair[0], pair[1]], Stroke::new(2_f32, palette::selection()));
            }
            for point in sketch_screen {
                painter.circle_filled(point, 4., palette::selection());
            }
            if let Some(g) = &self.navigation
                && let input::GestureKind::Lasso(path, _) = &g.kind
            {
                let mut outline = lasso::outline(self.lasso_style, path);
                if let Some(&first) = outline.first() {
                    outline.push(first);
                }
                painter.add(Shape::line(outline, Stroke::new(1.5f32, palette::lasso())));
            }
            // NAV-07: the box being dragged, in the lasso colour.
            if let Some(g) = &self.navigation
                && matches!(
                    g.kind,
                    input::GestureKind::Navigate(input::NavigationKind::BoxZoom)
                )
            {
                painter.rect_stroke(
                    Rect::from_two_pos(g.start, g.last),
                    0.,
                    Stroke::new(1.5f32, palette::lasso()),
                    egui::StrokeKind::Middle,
                );
            }
            self.paint_weld_drag(&painter, r);
            if self.status.starts_with("Help:") {
                egui::ScrollArea::vertical()
                    .id_salt("command-help-status")
                    .max_height(130.)
                    .show(ui, |ui| {
                        ui.add(Label::new(&self.status).wrap());
                    });
            } else if status_in_view {
                ui.add(Label::new(&self.status).truncate());
            }
            // The header's buttons float over the view's top edge, drawn
            // after (so over) the 3D view; presses on them stay theirs.
            let pane = ui.max_rect();
            self.view_dot(ui, self.drawing_view, r, pane);
            self.view_header(ui, r);
            if !self.view_passive {
                self.viewport_input(ui, &response, r);
            }
            let request =
                std::mem::take(&mut self.selection_context_requested) && !self.view_passive;
            egui::Popup::new(
                Id::new("selection-context"),
                ctx.clone(),
                egui::PopupAnchor::PointerFixed,
                ui.layer_id(),
            )
            .open_memory(request.then_some(egui::SetOpenCommand::Bool(true)))
            .show(|ui| {
                self.menu_owned_input = true;
                ui.label("Lasso Style");
                for style in lasso::Style::ALL {
                    ui.selectable_value(&mut self.lasso_style, style, style.label());
                }
                ui.checkbox(&mut self.lasso_partial, "Partial");
                ui.weak("Facing-only lasso; no depth occlusion");
            });
            if !self.view_passive {
                self.modeling_menu(ui);
            }
        }
    }
    /// The Pen draft's construction plane: fixed by its first point, so
    /// orbiting mid-draft cannot put points on different planes.
    fn pen_plane_point(&self, pos: Pos2, r: Rect) -> Option<DVec3> {
        let (plane, center) = match (&self.editor.pen_draft, self.pen_plane) {
            (Some(d), Some(fixed)) if !d.points().is_empty() => fixed,
            _ => (self.actual_plane(), self.workplane_center()),
        };
        self.work_point(pos, r, plane, center)
    }
    /// Press with a Pen draft: pick a draft point under the pointer, else add
    /// one where the pointer meets the plane. Returns the point's index.
    /// `end` (Ctrl+click with Make Quads) adds one point that closes the
    /// strip with a triangle.
    /// The Pen draft point under `pos`, if any (nearest within 8 px).
    pub(super) fn pen_hit(&self, pos: Pos2, r: Rect) -> Option<usize> {
        let draft = self.editor.pen_draft.as_ref()?;
        draft
            .points()
            .iter()
            .enumerate()
            .filter_map(|(i, &p)| self.camera.project(p, r).map(|(q, _)| (i, q.distance(pos))))
            .filter(|&(_, d)| d <= 8.)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }
    pub(super) fn pen_press(&mut self, pos: Pos2, r: Rect, end: bool) -> Option<usize> {
        let mode = self.point_modes.pen;
        if let Some(i) = self.pen_hit(pos, r) {
            // DPM-04/10: Delete mode removes the clicked point.
            if mode == crate::draw_tools::DrawMode::Delete {
                self.run(Command::PenDelete {
                    index: i,
                    and_after: false,
                });
                self.clear_fields();
                return None;
            }
            self.run(Command::PenSelect(i));
            return Some(i);
        }
        // DPM-03: Edit and Delete clicks in empty space add nothing.
        if mode != crate::draw_tools::DrawMode::Add {
            return None;
        }
        let draft = self.editor.pen_draft.as_ref()?;
        let first = draft.points().is_empty();
        let before = draft.points().len();
        let Some(point) = self.pen_plane_point(pos, r) else {
            self.status = "Click below the horizon: the pointer must meet the Pen plane".into();
            return None;
        };
        // DSN-07: a placed point snaps as a dragged one does.
        self.snap_session = Default::default();
        let point = self.pen_snap(point, None, r);
        // SYM3-23: a point placed near the symmetry plane lands on it.
        let point = self.symmetry_seam_snap(point, r);
        self.sync_pen_uv_view();
        self.run(if end {
            Command::PenCaptureEnd(point)
        } else {
            Command::PenCapture(point)
        });
        self.clear_fields();
        let draft = self.editor.pen_draft.as_ref()?;
        if draft.points().len() <= before {
            return None;
        }
        // A Make Quads click adds the point and its partner; the clicked
        // point (now current) is the one a drag moves.
        let index = draft.current().unwrap_or(before);
        if first {
            self.pen_plane = Some((self.actual_plane(), self.workplane_center()));
        }
        self.sync_pen_plane();
        Some(index)
    }
    /// DPM-06: a middle-click on a Pen draft point or Tube path point
    /// removes it (`and_after`: it and every later one). One undo step.
    fn point_delete_press(&mut self, pos: Pos2, r: Rect, and_after: bool) {
        if self.editor.pen_draft.is_some() {
            if let Some(index) = self.pen_hit(pos, r) {
                self.run(Command::PenDelete { index, and_after });
                self.clear_fields();
            }
        } else if let Some(index) = self.tube_hit(pos, r) {
            self.delete_tube_point(index, and_after);
        }
    }
    /// DSN-08/10: `point` (on the Pen plane) snapped to the active targets;
    /// the draft's other points are targets too, `dragged` (being moved)
    /// isn't. Unchanged when snapping is off or nothing is in range.
    fn pen_snap(&mut self, point: DVec3, dragged: Option<usize>, r: Rect) -> DVec3 {
        let Some(draft) = self.editor.pen_draft.as_ref() else {
            self.note_snap_drag();
            return point;
        };
        let points = draft.points();
        let moving = dragged.and_then(|i| points.get(i).copied());
        let own: Vec<DVec3> = points
            .iter()
            .enumerate()
            .filter(|&(i, _)| Some(i) != dragged)
            .map(|(_, &p)| p)
            .collect();
        // The new point goes after the current one; a dragged one sits
        // after the point before it.
        let from_index = match dragged {
            Some(i) => i.checked_sub(1),
            None => draft.current().or(points.len().checked_sub(1)),
        };
        let from = from_index.and_then(|i| points.get(i)).copied();
        let before = from_index
            .and_then(|i| i.checked_sub(1))
            .and_then(|i| points.get(i))
            .copied();
        let plane = self.pen_plane.map_or(self.actual_plane(), |p| p.0);
        let normal = [DVec3::Y, DVec3::X, DVec3::Z][plane.min(2) as usize];
        let path = crate::snap_kinds::Path {
            from,
            before,
            normal,
        };
        // SNH-Q01: direction locks act only while a point is dragged
        // (press-drag to place, or moving one), never on a plain click.
        self.snap_path_point(point, moving, &own, &path, dragged.is_some(), r)
    }
    /// The Pen's and Pen Slice's shared snap (DSN-08/10, SN2-34..37):
    /// `point` snapped to the active targets (`own` are the path's other
    /// points, targets too; `moving` is the dragged point's old place,
    /// never one); with no point target in range, the segment from
    /// `path.from` locks to World Axis / Straight Line / Right Angle when
    /// those are ticked. Unchanged when snapping is off.
    pub(super) fn snap_path_point(
        &mut self,
        point: DVec3,
        moving: Option<DVec3>,
        own: &[DVec3],
        path: &crate::snap_kinds::Path,
        dragging: bool,
        r: Rect,
    ) -> DVec3 {
        self.note_snap_drag();
        let targets = self.snap_targets();
        if !targets.any() {
            return point;
        }
        let moving: Vec<DVec3> = moving.into_iter().collect();
        // SN2-37: a point target in range wins over a direction lock.
        if let Some(snapped) = self.snap_draw_point(point, &moving, own, None, r) {
            return snapped;
        }
        if !dragging {
            return point;
        }
        let camera = &self.camera;
        crate::snap_kinds::lock_segment(
            targets.world_axis,
            targets.straight_line,
            targets.right_angle,
            path,
            point,
            |p| camera.project(p, r).map(|q| q.0),
            self.snapping.inner_range,
        )
    }
    /// Drag of Pen point `index` to the pointer on the draft's plane.
    fn pen_drag(&mut self, index: usize, pos: Pos2, r: Rect) {
        let Some(point) = self.pen_plane_point(pos, r) else {
            return;
        };
        let point = self.pen_snap(point, Some(index), r);
        let point = self.symmetry_seam_snap(point, r);
        if self.editor.pen_draft.as_ref().and_then(|d| d.current()) != Some(index) {
            self.run(Command::PenSelect(index));
        }
        self.run(Command::PenRefine(point));
        self.clear_fields();
    }
    pub(super) fn actual_plane(&self) -> u8 {
        if self.editor.scene.workplane.fixed {
            return self.editor.scene.workplane.plane;
        }
        self.units_preferences.view_plane(self.camera.basis().3)
    }
    /// The work plane's centre: the scene's, moved along the plane's normal
    /// by Locked Plane Offset while Lock Preferred Plane holds it.
    pub(super) fn workplane_center(&self) -> DVec3 {
        let w = &self.editor.scene.workplane;
        let u = &self.units_preferences;
        if w.fixed || !u.lock_preferred {
            return w.center;
        }
        let normal = match u.preferred_plane {
            1 => DVec3::X,
            2 => DVec3::Z,
            _ => DVec3::Y,
        };
        w.center + normal * u.locked_offset
    }
    /// Where the plane for a newly placed point passes when it extends
    /// `from` (the current point of a sketch or curve). In a view without
    /// perspective (Top, Front, Left ... and free Orthographic) the plane
    /// goes through `from`, so the coordinate along the view's depth keeps
    /// the previous point's value and only the two in-view axes come from
    /// the pointer: a leg drawn in Front and continued in Left stays off
    /// the centre line. Perspective views, a fixed work plane, a locked
    /// preferred plane or no `from` keep the work plane's centre. Only the
    /// depth moves: the in-plane axes stay the work plane's, so Coordinate
    /// Rounding still steps from the grid's origin, not from `from`.
    pub(super) fn placement_center(&self, from: Option<DVec3>) -> DVec3 {
        let mut center = self.workplane_center();
        let w = &self.editor.scene.workplane;
        if let Some(p) = from
            && self.camera.view != View::Perspective
            && !w.fixed
            && !self.units_preferences.lock_preferred
            && p.is_finite()
        {
            // The plane's normal axis: 0 XZ → Y, 1 YZ → X, 2 XY → Z.
            let depth = match self.actual_plane() {
                1 => 0,
                2 => 2,
                _ => 1,
            };
            center[depth] = p[depth];
        }
        center
    }
    #[cfg(test)]
    pub(super) fn pick_face(&self, p: Pos2, r: Rect) -> Option<(u64, u32, f64)> {
        self.pick_surface().pick(&self.camera, p, r)
    }
    /// Axis Rotate (TC-30): the one ring it shows, the work plane's normal
    /// (plane 0 is XZ, 1 YZ, 2 XY).
    pub(super) fn axis_rotate_axis(&self) -> Option<usize> {
        let p = self.editor.preset.as_ref()?;
        if !p.row().actor.single_ring {
            return None;
        }
        Some([1, 0, 2][self.actual_plane().min(2) as usize])
    }
    pub(super) fn handles(&self, r: Rect) -> Vec<(usize, Pos2, Pos2, String)> {
        let all = self.all_handles(r);
        match self.axis_rotate_axis() {
            Some(axis) => all.into_iter().filter(|h| h.0 == axis).collect(),
            None => all,
        }
    }
    fn all_handles(&self, r: Rect) -> Vec<(usize, Pos2, Pos2, String)> {
        let Some(l) = &self.editor.live else {
            return Vec::new();
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
        if self.handle_preferences.selected == crate::handle_preferences::HandleDetail::Invisible {
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
                let s = s_at(l.params.position);
                for axis in 0..3 {
                    let direction = [DVec3::X, DVec3::Y, DVec3::Z][axis];
                    if let Some((b, _)) = self.camera.project(
                        l.params.position + direction * self.camera.distance * 0.1,
                        r,
                    ) && center.distance(b) > 5.
                    {
                        out.push((
                            axis,
                            center,
                            center + (b - center).normalized() * 72. * s,
                            ["X", "Y", "Z"][axis].into(),
                        ));
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
                if let Some((unit, _)) = self
                    .camera
                    .project(center + direction * self.camera.distance * 0.1, r)
                    && unit.distance(origin) > 1.
                {
                    let screen = (unit - origin).normalized();
                    out.push((
                        axis,
                        origin,
                        origin + screen * 72. * s,
                        format!("Move {}", ["X", "Y", "Z"][axis]),
                    ));
                    out.push((
                        6 + axis,
                        origin + screen * 30. * s,
                        origin + screen * 44. * s,
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
            let length = self.camera.distance * 0.1;
            if let (Some((a, _)), Some((b, _))) = (
                self.camera.project(origin, r),
                self.camera.project(origin + direction * length, r),
            ) && a.distance(b) > 5.
            {
                // ACS-Q-HANDLE1: the box-centre tools' arms start about
                // 25 px out from the centre.
                let gap = if operandi_core::handle_pick(l.tool)
                    == operandi_core::HandlePick::BoxCentre
                    && card.anchor != HandleAnchor::PolygonStage
                {
                    25. * s
                } else {
                    0.
                };
                let dir = (b - a).normalized();
                let a = a + dir * gap;
                out.push((
                    axis,
                    a,
                    a + dir * 72. * s,
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
                        "Inset".into()
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
    pub(super) fn handle_axis(&self, axis: usize) -> DVec3 {
        // Frame-based tools point their handles along their card's frame.
        if let Some(l) = self.editor.live.as_ref() {
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
            }
        }
        if axis > 2
            && self
                .editor
                .live
                .as_ref()
                .is_some_and(|l| l.tool == Tool::Scale)
        {
            return DVec3::ZERO;
        }
        let Some(l) = &self.editor.live else {
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
                return if axis == 0 {
                    frame.basis.y_axis
                } else {
                    frame.basis.z_axis
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
                return l
                    .base
                    .world_matrix(i.id)
                    .map(|m| m.transform_vector3(local).normalize_or_zero())
                    .unwrap_or(local);
            }
            a
        } else if let Some(basis) = self.local_handle_basis() {
            // AC-75: under Local the arrows follow the last selected
            // element's cluster.
            basis * a
        } else if let Some(frame) = l.action_frame {
            frame.basis * a
        } else {
            a
        }
    }
    /// Original-compatible A1: handles have a size in the scene, so they
    /// shrink on screen as the view zooms in on them and grow as it zooms
    /// out; 1 (constant screen size) with A1 off. See `Camera::handle_zoom`.
    pub(super) fn handle_zoom_at(&self, center: DVec3) -> f32 {
        if crate::original_compatible::is_original(WORLD_SIZE_HANDLES) {
            self.camera.handle_zoom(center) as f32
        } else {
            1.
        }
    }
    /// Handle Scale: Standard times `handle_zoom_at(center)`.
    pub(super) fn handle_scale_at(&self, center: DVec3) -> f32 {
        self.handle_preferences.scale * self.handle_zoom_at(center)
    }
    /// The live transform handle's frame before the drag's delta: where
    /// the handle sits at `delta = 0` and the axes a unit of `delta` moves
    /// it along. Under Local (AC-75) that's the last selected element's
    /// cluster frame, which is also what the core moves that cluster by;
    /// otherwise the action frame, with the polygon stage's Symmetry
    /// centre for component tools. Snapping measures in this same frame.
    pub(super) fn transform_handle_frame(&self) -> Option<operandi_core::ActionFrame> {
        let l = self.editor.live.as_ref()?;
        let frame = l.action_frame?;
        // The pivot service places the frame (ACS-01/07); the polygon
        // stage's handle follows its live caps.
        if let Some(local) = self.local_handle_frame() {
            return Some(local);
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
    pub(super) fn transform_handle_center(&self) -> DVec3 {
        let Some(l) = self.editor.live.as_ref() else {
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
    pub(super) fn update_handle(
        &mut self,
        axis: usize,
        base: Parameters,
        start: Pos2,
        p: Pos2,
        tool: Tool,
        r: Rect,
    ) {
        if axis == FREE_HANDLE {
            self.update_free_handle(base, start, p, tool, r);
            return;
        }
        if axis == OFF_HANDLE {
            self.update_off_handle(base, start, p, tool, r);
            return;
        }
        if is_plane_handle(axis) {
            self.update_plane_handle(axis, base, start, p, tool, r);
            return;
        }
        if precision_handles::is_precision_handle(axis) {
            self.update_precision_handle(axis, base, p, tool, r);
            return;
        }
        if tool == Tool::TwistBevel {
            self.update_twist_bevel_handle(axis, base, start, p, r);
            return;
        }
        if tool == Tool::EdgeExtend && matches!(axis, EXTEND_INSET_HANDLE | EXTEND_SHIFT_HANDLE) {
            self.update_extend_local_handle(axis, base, start, p, r);
            return;
        }
        let combined = combined_handles(tool);
        let family = axis / 3;
        let axis = if combined { axis % 3 } else { axis };
        let tool = if combined {
            match family {
                0 => Tool::Move,
                1 => Tool::Rotate,
                _ => Tool::Scale,
            }
        } else {
            tool
        };
        if tool == Tool::Extrude && axis == EXTRUDE_FREE_HANDLE {
            // Free movement parallel to the view, at the pivot's depth scale
            // used by the axis handles.
            let (_, right, up, _) = self.camera.basis();
            let scale = self.camera.distance / (r.height() as f64 * 0.9);
            let d = p - start;
            let mut params = base;
            params.extent += (right * d.x as f64 - up * d.y as f64) * scale;
            self.publish_handle_params(tool, params);
            return;
        }
        if tool == Tool::VertexBevel && axis == VERTEX_BEVEL_FREE_HANDLE {
            // Off the handle: dragging right widens the Inset, left narrows it
            // (Jason's GUI test, 2026-10-31), at the axis handles' scale.
            let pixels = (p.x - start.x) as f64;
            let mut params = base;
            params.inset =
                (params.inset + pixels * self.camera.distance / (r.height() as f64 * 0.9)).max(0.);
            self.publish_handle_params(tool, params);
            return;
        }
        if tool == Tool::Scale && axis == SCALE_UNIFORM_HANDLE {
            // Right or up enlarges, left or down shrinks; ratios are kept.
            let pixels = ((p.x - start.x) - (p.y - start.y)) as f64;
            let mut params = base;
            let factor = (1. + pixels * 0.01).max(0.001 / params.delta.min_element().max(1e-9));
            params.delta *= factor;
            self.run(Command::Preview(params));
            self.clear_fields();
            return;
        }
        let direction = self.handle_axis(axis);
        let (_, right, up, _) = self.camera.basis();
        let screen = vec2(direction.dot(right) as f32, -direction.dot(up) as f32);
        let pixels = (p - start).dot(screen) / screen.length_sq().max(0.001);
        let amount = pixels as f64 * self.camera.distance / (r.height() as f64 * 0.9);
        let mut params = base;
        let saved_delta = params.delta;
        if combined {
            params.delta = match family {
                1 => params.rotation_delta,
                2 => params.scale_delta,
                _ => params.delta,
            };
        }
        match tool {
            // Same feel as Bevel's Inset handle: dragging out, away from the
            // polygon's middle, makes the inner copy bigger (less Inset).
            Tool::Inset => params.inset -= amount,
            Tool::Shift => params.shift += amount,
            // Twist Bevel's handles have their own drags (twist_bevel_handles.rs).
            Tool::TwistBevel => return,
            // Tweak has no handles in TWK-1; it drags through its own gesture.
            Tool::PolyHaul | Tool::SketchExtrude | Tool::Tweak => return,
            // No handles: they drag anywhere (`update_off_handle`).
            Tool::Push | Tool::Smooth | Tool::SymmetryTool => return,
            Tool::Jitter | Tool::Quantize | Tool::RadialAlign | Tool::LinearAlign => return,
            // The UV tab's tools have no handles: fields, a click (Project
            // from View) or a drag anywhere (Iterations).
            Tool::UvProject
            | Tool::UvViewProject
            | Tool::UvUnwrap
            | Tool::UvRelax
            | Tool::UvPelt
            | Tool::UvPeeler
            | Tool::UvBoxTransform => {
                return;
            }
            // Bend drags through its own gesture (bend_view.rs).
            Tool::Bend => return,
            // The Slice tools drag through their own gestures (slice_view.rs).
            Tool::LoopSlice
            | Tool::EdgeSlice
            | Tool::PolygonSlice
            | Tool::CurveSlice
            | Tool::PenSlice
            | Tool::PrimitiveSlice
            | Tool::AxisSlice
            | Tool::Contour => return,
            // So do the Duplicate tools (duplicate_view.rs).
            Tool::Mirror
            | Tool::Lathe
            | Tool::Clone
            | Tool::Array
            | Tool::RadialArray
            | Tool::ScatterClone
            | Tool::Symmetrize => return,
            // The path tools drag their path points (path_tool_view.rs).
            Tool::CurveExtrude | Tool::CurveClone => return,
            Tool::Transform => return,
            // N21: the move arrows (0..3) position the shape; its bound
            // squares drag through the primitive gesture.
            Tool::Sphere | Tool::Cylinder if axis < 3 => params.position[axis] += amount,
            Tool::Sphere | Tool::Cylinder => return,
            Tool::Box => params.position[axis] += amount,
            Tool::Chamfer => {
                params.chamfer.offset = (params.chamfer.offset + amount).max(0.);
            }
            Tool::EdgeExtend => params.delta[axis] += amount,
            Tool::EdgeCrease => {
                // Dragging the handle by the camera's quarter-view raises
                // the weight by 100 %; it runs from -100 % to 100 % (EW-01).
                let span = (self.camera.distance * 0.25).max(1e-9);
                params.shift = (params.shift + amount / span).clamp(-1., 1.);
            }
            Tool::EdgeBevel | Tool::EdgeChamfer => {
                params.chamfer.offset = (params.chamfer.offset + amount).max(0.);
            }
            Tool::EdgeRelax => params.shift += amount,
            Tool::VertexCreate => return,
            Tool::VertexBevel => params.inset = (params.inset + amount).max(0.),
            Tool::VertexMerge => params.shift = (params.shift + amount).max(0.),
            Tool::VertexExtrude => {
                if axis == 0 {
                    params.shift += amount;
                } else {
                    params.inset = (params.inset + amount).max(0.);
                }
            }
            Tool::VertexTear => params.delta[axis] += amount,
            Tool::VertexSlide => {
                // World drag along the rail: a share of its length (Percent,
                // ±100 %), or the distance itself in Distance mode.
                if params.edge_slide_distance {
                    params.shift += amount;
                } else {
                    let rail = self.vertex_frame().map_or(1., |f| f.3.max(1e-9));
                    params.shift += amount / rail;
                    // Loop Slide keeps the point on its edge; Stop Negative
                    // Slide keeps the share at 0..100 %.
                    if params.slide_extras.loop_slide {
                        params.shift = params.shift.clamp(-1., 1.);
                    }
                }
                if params.slide_extras.stop_negative {
                    params.shift = params.shift.max(0.);
                }
            }
            Tool::EdgeSlide => {
                // World drag along the rail: a fraction of the rail length, or
                // the distance itself in Distance mode. Not clamped (ES-05).
                if params.edge_slide_distance {
                    params.shift += amount;
                } else {
                    let rail = self.edge_slide_frame().map_or(1., |f| f.2.max(1e-9));
                    params.shift += amount / rail;
                }
                // Stop Negative Slide: the drag can't take the amount below 0
                // (the core also caps each point at its rail's end).
                if params.slide_extras.stop_negative {
                    params.shift = params.shift.max(0.);
                }
            }
            Tool::EdgeExtrude => {
                if axis == 0 {
                    params.shift += amount;
                } else {
                    params.inset = (params.inset + amount).max(0.);
                }
            }
            Tool::Bevel => {
                if axis == 0 {
                    params.shift += amount;
                } else {
                    // User correction 2026-10-30: dragging the square out,
                    // away from the selection center, enlarges the cap (less
                    // Inset); dragging in toward the center shrinks it.
                    params.inset -= amount;
                }
            }
            // D18-A: visible resolved axes write equivalent WORLD Extent XYZ.
            Tool::Extrude => params.extent += direction * amount,
            Tool::SmoothShift | Tool::Thicken => {
                if axis == 0 {
                    params.smooth_shift.offset += amount;
                } else {
                    params.smooth_shift.scale += pixels as f64 * 0.01;
                }
            }
            Tool::Move => params.delta[axis] += amount,
            Tool::Rotate => {
                if let Some((o, _)) = self.camera.project(self.transform_handle_center(), r) {
                    let a = start - o;
                    let b = p - o;
                    let angle = (a.x * b.y - a.y * b.x).atan2(a.dot(b));
                    params.delta[axis] +=
                        angle.to_degrees() as f64 * direction.dot(self.camera.basis().3).signum();
                    // Ctrl held as the drag began: whole steps of Rotation
                    // Snap Angle.
                    if self.handle_constrain {
                        params.delta[axis] = self
                            .handle_preferences
                            .round_to_snap_angle(params.delta[axis]);
                    }
                }
            }
            Tool::Scale => {
                let scaled = (params.delta[axis] + pixels as f64 * 0.01).max(0.001);
                if self.handle_constrain {
                    // HM-30: Ctrl on one axis handle scales all three axes by
                    // the same factor (equal when they start equal).
                    let factor = scaled / params.delta[axis].max(1e-9);
                    params.delta = (params.delta * factor).max(DVec3::splat(0.001));
                } else {
                    params.delta[axis] = scaled;
                }
            }
        }
        if combined && family > 0 {
            if family == 1 {
                params.rotation_delta = params.delta;
            } else {
                params.scale_delta = params.delta;
            }
            params.delta = saved_delta;
        }
        // SN-08: a Move drag's handle snaps to the nearest target; on an
        // axis arrow it stays on that axis (SN2-30b Constraint Snapping).
        let line = (!combined || family == 0).then_some(crate::snapping::AxisDrag {
            axis,
            base_delta: saved_delta,
            moved: p - start,
        });
        let params = self.snap_move(tool, params, line, r);
        self.publish_handle_params(tool, params);
    }
    /// Publish one handle sample: through the captured I2 stage for Extrude
    /// when one is active, else as an ordinary live preview.
    /// A press away from the model with a transform tool live: keep what
    /// was done so far, put the tool's center where the press is (at the
    /// current center's depth) and drag from there like the center handle
    /// (Scale: uniformly). Provisional (Jason, 2026-11-03; VAC-Q02).
    fn relocate_transform_center(&mut self, tool: Tool, pos: Pos2, r: Rect) -> input::GestureKind {
        let center = self.transform_handle_center();
        let (origin, direction) = self.camera.ray(pos, r);
        let (_, _, _, forward) = self.camera.basis();
        let along = direction.dot(forward);
        if along.abs() < 1e-9 {
            return input::GestureKind::Blocked;
        }
        let mut point = origin + direction * ((center - origin).dot(forward) / along);
        // ACS table, Automatic/None: the click's X and Z come from where it
        // lands on the ground work plane; the height stays the centre's.
        let (plane, plane_height) = (
            self.editor.scene.workplane.plane,
            self.editor.scene.workplane.center.y,
        );
        if matches!(
            self.editor.action_context.center,
            CenterMode::Automatic | CenterMode::None
        ) && plane == 0
            && direction.y.abs() > 1e-6
        {
            let t = (plane_height - origin.y) / direction.y;
            if t > 0. {
                let hit = origin + direction * t;
                point = DVec3::new(hit.x, center.y, hit.z);
            }
        }
        // SN-12: the new center snaps (Rotate's and Scale's centre too),
        // except Screen's (AC-12).
        let screen = matches!(self.editor.action_context.center, CenterMode::Screen(_));
        if !screen && let Some(snapped) = self.snap_point(point, None, r) {
            point = snapped;
        }
        if self.editor.live.as_ref().is_some_and(|l| l.applied) {
            self.run(Command::Drop);
            self.activate(tool);
        }
        if !self.run_ok(Command::SetActionCenter(point)) {
            return input::GestureKind::Blocked;
        }
        self.free_transform_drag(tool)
    }
    /// FO-45: an Element falloff press picks the element under the pointer
    /// (the falloff's Mode decides which kinds), puts the action centre on
    /// it and drags the tool freely. Each press is its own tool
    /// application, so its own undo step.
    fn begin_element_falloff_drag(&mut self, tool: Tool, pos: Pos2, r: Rect) -> input::GestureKind {
        let Some(pick) = self.tweak_pick(pos, r) else {
            self.status = "Element falloff: press on a vertex, edge or polygon".into();
            return input::GestureKind::Blocked;
        };
        if self.editor.live.as_ref().is_some_and(|l| l.applied) {
            self.run(Command::Drop);
            self.activate(tool);
        }
        if !self.run_ok(Command::FalloffElementPick(Some(pick.grab)))
            || !self.run_ok(Command::SetActionCenter(pick.at))
        {
            return input::GestureKind::Blocked;
        }
        self.free_transform_drag(tool)
    }
    /// Start a free drag of the live transform tool, as if its center
    /// handle were grabbed (Scale: uniform scale).
    fn free_transform_drag(&mut self, tool: Tool) -> input::GestureKind {
        let axis = if tool == Tool::Scale {
            SCALE_UNIFORM_HANDLE
        } else {
            FREE_HANDLE
        };
        match i2_route::capture(&mut self.editor, axis, false) {
            Ok((params, ticket)) => {
                self.i2_gesture_ticket = ticket;
                self.clear_fields();
                input::GestureKind::Handle(axis, params, tool)
            }
            Err(e) => {
                self.status = e;
                input::GestureKind::Blocked
            }
        }
    }
    /// HM-01: the press decides the modifiers for the whole drag. Ctrl is
    /// the constrain key on every platform; a new press forgets the last
    /// drag's axis lock.
    pub(super) fn begin_constrain(&mut self, modifiers: egui::Modifiers) {
        self.handle_constrain = modifiers.ctrl;
        self.haul_axis = None;
    }
    /// HM-10/12/71: the axis a Ctrl drag is locked to, picked once the
    /// pointer has travelled `HAUL_AXIS_PICK_PIXELS` from the press and kept
    /// until the next press (HM-12a). Shared by Move/Transform hauls and
    /// Tweak so they follow the same rule (HM-70).
    pub(super) fn haul_lock_axis(&mut self, travel: Vec2) -> Option<DVec3> {
        let axes = crate::draw_tools::plane_axes(self.actual_plane());
        self.lock_axis_among(travel, axes)
    }
    /// `haul_lock_axis` choosing between two given axes (a plane handle's,
    /// PH-22).
    pub(super) fn lock_axis_among(&mut self, travel: Vec2, axes: [DVec3; 2]) -> Option<DVec3> {
        if self.haul_axis.is_none() && travel.length() >= HAUL_AXIS_PICK_PIXELS {
            self.haul_axis = Some(self.pick_axis_among(travel, axes));
        }
        self.haul_axis
    }
    /// HM-12 (confirmed): of the work plane's two axes, the one whose
    /// on-screen direction is closest to the pointer's travel so far. The
    /// same axes as the curve tools' Ctrl drag (CRV-05).
    fn pick_axis_among(&self, travel: Vec2, [a, b]: [DVec3; 2]) -> DVec3 {
        let (_, right, up, _) = self.camera.basis();
        let travel = travel.normalized();
        let closeness = |a: DVec3| {
            let screen = vec2(a.dot(right) as f32, -a.dot(up) as f32);
            if screen.length_sq() < 1e-6 {
                0.
            } else {
                screen.normalized().dot(travel).abs()
            }
        };
        if closeness(a) >= closeness(b) { a } else { b }
    }
    /// Dragging the center handle (or off the model, after the click that
    /// put the center there): Move and Transform follow the pointer in the
    /// view plane; Rotate turns about the frame axes nearest the view's up
    /// (left/right drag) and right (up/down drag), half a degree per pixel.
    /// Provisional (Jason, 2026-11-03); the reference's free rules aren't
    /// measured (HND-Q01/Q02).
    fn update_free_handle(&mut self, base: Parameters, start: Pos2, p: Pos2, tool: Tool, r: Rect) {
        let Some(mut frame) = self.editor.live.as_ref().and_then(|l| l.action_frame) else {
            return;
        };
        // AC-75: under Local the handle's own axes.
        if let Some(basis) = self.local_handle_basis() {
            frame.basis = basis;
        }
        let (_, right, up, _) = self.camera.basis();
        let d = p - start;
        let mut params = base;
        match tool {
            Tool::Move | Tool::Transform if self.handle_constrain => {
                // HM-10/11: a Ctrl haul moves along one axis of the work
                // plane, picked once (HM-12) and kept for the whole drag.
                let Some(axis) = self.haul_lock_axis(d) else {
                    return;
                };
                // Distance along the axis as its own arrow would measure it.
                let screen = vec2(axis.dot(right) as f32, -axis.dot(up) as f32);
                let pixels = d.dot(screen) / screen.length_sq().max(0.001);
                let scale = self.camera.distance / (r.height() as f64 * 0.9);
                let world = axis * (pixels as f64 * scale);
                params.delta += frame.basis.transpose() * world;
            }
            Tool::Move | Tool::Transform => {
                let scale = self.camera.distance / (r.height() as f64 * 0.9);
                let world = (right * d.x as f64 - up * d.y as f64) * scale;
                params.delta += frame.basis.transpose() * world;
            }
            Tool::Rotate => {
                let nearest = |v: DVec3| {
                    (0..3)
                        .max_by(|&a, &b| {
                            frame
                                .basis
                                .col(a)
                                .dot(v)
                                .abs()
                                .total_cmp(&frame.basis.col(b).dot(v).abs())
                        })
                        .unwrap_or(0)
                };
                let (about_up, about_right) = (nearest(up), nearest(right));
                params.delta[about_up] +=
                    d.x as f64 * 0.5 * frame.basis.col(about_up).dot(up).signum();
                params.delta[about_right] +=
                    d.y as f64 * 0.5 * frame.basis.col(about_right).dot(right).signum();
                // HM-20/21: Ctrl (held at the press) turns in whole steps of
                // Rotation Snap Angle, on a haul as on a ring.
                if self.handle_constrain {
                    for a in [about_up, about_right] {
                        params.delta[a] =
                            self.handle_preferences.round_to_snap_angle(params.delta[a]);
                    }
                }
            }
            _ => return,
        }
        let params = self.snap_move(tool, params, None, r);
        self.publish_handle_params(tool, params);
    }
    /// A drag away from the handles (Jason, 2026-11-04): up/down acts like
    /// the move arrow, left/right like the scale widget, at the axis
    /// handles' scale. Tools with one size-like value (Inset, Offset,
    /// Distance, Crease) take it from left/right; the slides follow their
    /// rail as if its handle were dragged; Extend and Tear move freely in the
    /// view plane. Provisional (DRAG-Q01).
    fn update_off_handle(&mut self, base: Parameters, start: Pos2, p: Pos2, tool: Tool, r: Rect) {
        let d = p - start;
        let scale = self.camera.distance / (r.height() as f64 * 0.9);
        let vertical = -d.y as f64 * scale;
        let horizontal = d.x as f64 * scale;
        let mut params = base;
        match tool {
            Tool::Bevel => {
                params.shift += vertical;
                // Like the Inset square: out (right) enlarges the cap.
                params.inset -= horizontal;
            }
            Tool::SmoothShift | Tool::Thicken => {
                params.smooth_shift.offset += vertical;
                params.smooth_shift.scale += d.x as f64 * 0.01;
            }
            Tool::EdgeExtrude | Tool::VertexExtrude => {
                params.shift += vertical;
                params.inset = (params.inset + horizontal).max(0.);
            }
            Tool::Chamfer | Tool::EdgeBevel | Tool::EdgeChamfer => {
                params.chamfer.offset = (params.chamfer.offset + horizontal).max(0.);
            }
            // As Bevel's: right enlarges the inner copy (less Inset); Shift
            // follows up/down like a move arrow.
            Tool::Inset => params.inset -= horizontal,
            Tool::Shift => params.shift += vertical,
            // TB-Q08 (provisional): a drag off the handles changes Shift,
            // like Bevel's plain drag.
            Tool::TwistBevel => params.twist_bevel.shift += vertical,
            Tool::EdgeRelax => params.shift += horizontal,
            // Push: up pushes out, down pulls in, like the move arrow.
            Tool::Push => params.shift += vertical,
            // Symmetry Tool: right raises Threshold, left lowers it.
            Tool::SymmetryTool => {
                params.symmetry_fix.threshold =
                    (params.symmetry_fix.threshold + horizontal).max(0.);
            }
            // Jitter: right widens Range on every enabled axis (a world
            // distance, like the move arrows).
            Tool::Jitter => {
                let r = (params.jitter.range.max_element() + horizontal).max(0.);
                params.jitter.range = DVec3::new(
                    if params.jitter.enable[0] {
                        r
                    } else {
                        params.jitter.range.x
                    },
                    if params.jitter.enable[1] {
                        r
                    } else {
                        params.jitter.range.y
                    },
                    if params.jitter.enable[2] {
                        r
                    } else {
                        params.jitter.range.z
                    },
                );
            }
            // Quantize: right makes the grid step bigger on every axis.
            Tool::Quantize => {
                params.quantize.step =
                    DVec3::splat((params.quantize.step.max_element() + horizontal).max(0.));
            }
            // Radial / Linear Align: right raises Weight (200 px = 100%).
            Tool::RadialAlign => {
                params.radial_align.weight =
                    (params.radial_align.weight + d.x as f64 / 200.).clamp(0., 1.);
            }
            Tool::LinearAlign => {
                params.linear_align.weight =
                    (params.linear_align.weight + d.x as f64 / 200.).clamp(0., 1.);
            }
            // Smooth: right raises Strength, left lowers it (200 px = 1).
            Tool::Smooth => {
                params.smooth.strength =
                    (params.smooth.strength + d.x as f64 / 200.).clamp(0., 100.);
            }
            Tool::VertexMerge => params.shift = (params.shift + horizontal).max(0.),
            Tool::EdgeCrease => {
                let span = (self.camera.distance * 0.25).max(1e-9);
                params.shift = (params.shift + horizontal / span).clamp(-1., 1.);
            }
            Tool::EdgeSlide | Tool::VertexSlide => {
                self.update_handle(0, params, start, p, tool, r);
                return;
            }
            // Edge Extend's Haul Local: horizontal = Inset, vertical = Shift.
            Tool::EdgeExtend if params.edge_extend.haul == operandi_core::ExtendHaul::Local => {
                params.inset += horizontal;
                params.shift += vertical;
            }
            Tool::EdgeExtend | Tool::VertexTear => {
                let (_, right, up, _) = self.camera.basis();
                let world = (right * d.x as f64 - up * d.y as f64) * scale;
                let frame = self.editor.live.as_ref().and_then(|l| l.action_frame);
                params.delta += frame.map_or(world, |f| f.basis.transpose() * world);
            }
            // UV Unwrap, Relax and Unwrap & Relax: Iterations (UV-Q6).
            Tool::UvUnwrap | Tool::UvRelax | Tool::UvPelt => {
                match self.uv_iterations_drag(&params, tool, d.x as f64) {
                    Some(p) => params = p,
                    None => return,
                }
            }
            _ => return,
        }
        self.publish_handle_params(tool, params);
    }
    fn publish_handle_params(&mut self, tool: Tool, params: Parameters) {
        if tool == Tool::Extrude && (self.editor.i2_active() || self.i2_gesture_ticket.is_some()) {
            let Some(ticket) = self.i2_gesture_ticket else {
                self.status = "Stale extrusion gesture; recapture the current handle".into();
                return;
            };
            match i2_route::move_captured(&mut self.editor, ticket, params) {
                Ok(()) => {
                    self.i2_gesture_ticket = self.editor.extrude_stage_ticket().ok();
                    self.status = "Ready".into();
                    self.clear_fields();
                }
                Err(error) => self.status = error,
            }
        } else {
            self.run(Command::Preview(params));
            self.clear_fields();
        }
    }
    fn rotation_ring(&self, axis: usize, r: Rect) -> Vec<Pos2> {
        let center = self.transform_handle_center();
        let Some((_, depth)) = self.camera.project(center, r) else {
            return vec![];
        };
        let radius = if self
            .editor
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
    pub(super) fn handle_hit(&self, p: Pos2, r: Rect) -> Option<usize> {
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
    fn handle_hit_axes(&self, p: Pos2, r: Rect) -> Option<usize> {
        if self
            .editor
            .live
            .as_ref()
            .is_some_and(|l| l.tool.card().handles.kind == HandleKind::Spine)
        {
            return self.twist_bevel_hit(p, r);
        }
        if let Some(live) = &self.editor.live
            && matches!(live.tool, Tool::Move | Tool::Rotate | Tool::Transform)
            && let Some(free) = self.handles(r).into_iter().find(|h| {
                h.0 == FREE_HANDLE && Rect::from_center_size(h.2, vec2(12., 12.)).contains(p)
            })
        {
            return Some(free.0);
        }
        if let Some(live) = &self.editor.live
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
                    .live
                    .as_ref()
                    .is_some_and(|l| l.tool == Tool::Scale)
                && self.scale_center_contains(h.2, p)
        }) {
            return Some(center.0);
        }
        if self
            .editor
            .live
            .as_ref()
            // Every primitive the same way (Jason, 2026-11-21): its side
            // squares first, then the move arrows along their whole shaft,
            // not only at the tip (Sphere and Cylinder took the tip only,
            // so a press on the arrow dragged the nearest side instead).
            .is_some_and(|live| live.tool.is_primitive())
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
    pub(super) fn navigate(&mut self, kind: input::NavigationKind, delta: Vec2, r: Rect) {
        // The header's Zoom button heads for the surface at the view
        // centre, as the Zoom In/Out keys do (fix-stuck-zoom).
        if kind == input::NavigationKind::Zoom {
            let factor = self.input_preferences.options.drag_zoom_factor(delta);
            if factor != 1. && self.zoom_toward_surface(r.center(), r, factor) {
                return;
            }
        }
        // Buttons and keys have no pointer position: a trackball turn then
        // starts from the view's centre.
        self.navigate_from(kind, r.center(), r.center() + delta, r);
    }
    /// Navigation for a pointer move from `from` to `to`. Trackball Rotation
    /// (Preferences > Input > Remapping, or the view's own choice) makes
    /// Orbit a free trackball turn; Camera view keeps the two-axis orbit
    /// since a render camera has no roll.
    pub(super) fn navigate_from(
        &mut self,
        kind: input::NavigationKind,
        from: Pos2,
        to: Pos2,
        r: Rect,
    ) {
        let delta = to - from;
        // NAV-03: a flick orbits like Alt+left while held.
        let kind = match kind {
            input::NavigationKind::Flick => input::NavigationKind::Orbit,
            input::NavigationKind::Roll => {
                // NAV-02: the angle the pointer turns about the view centre.
                let (a, b) = (from - r.center(), to - r.center());
                if a.length() > 2. && b.length() > 2. {
                    let angle = (a.x * b.y - a.y * b.x).atan2(a.dot(b)) as f64;
                    self.camera.roll_by(-angle);
                }
                return;
            }
            // NAV-05: sideways motion ignored.
            input::NavigationKind::PanVertical => {
                let (_, _, up, _) = self.camera.basis();
                self.camera.target += up * delta.y as f64 * self.camera.distance
                    / (r.height() as f64 * self.camera.lens);
                return;
            }
            // NAV-07: nothing moves until release (`box_zoom`).
            input::NavigationKind::BoxZoom => return,
            k => k,
        };
        // Orbit Selected (Viewport Properties > Drawing and Control): the
        // eye turns about the selection's centre, not the view's.
        let pivot = (kind == input::NavigationKind::Orbit
            && !self.camera.looks_through()
            && self.display.settings.drawing.orbit_selected)
            .then(|| self.orbit_pivot())
            .flatten();
        let before = pivot.map(|_| self.camera.basis());
        self.orbit_or_move(kind, from, to, delta, r);
        if let (Some(pivot), Some(before)) = (pivot, before) {
            crate::orbit_selected::turn_about(&mut self.camera, before, pivot);
        }
    }
    /// `navigate_from` once Flick, Roll, Pan Vertical and Box Zoom are dealt
    /// with: orbit, pan or zoom.
    fn orbit_or_move(
        &mut self,
        kind: input::NavigationKind,
        from: Pos2,
        to: Pos2,
        delta: Vec2,
        r: Rect,
    ) {
        if kind == input::NavigationKind::Orbit
            && !self.camera.looks_through()
            && self
                .input_preferences
                .options
                .trackball_in(self.display.settings.drawing.trackball)
        {
            let speed = self.input_preferences.options.trackball_speed() as f64;
            self.camera.trackball(from, to, r, speed);
            return;
        }
        match kind {
            input::NavigationKind::Pan => {
                let (_, right, up, _) = self.camera.basis();
                self.camera.target += (-right * delta.x as f64 + up * delta.y as f64)
                    * self.camera.distance
                    / (r.height() as f64 * self.camera.lens);
            }
            input::NavigationKind::Zoom => {
                self.camera.distance = crate::camera::zoomed_distance(
                    self.camera.distance,
                    self.input_preferences.options.drag_zoom_factor(delta),
                )
            }
            input::NavigationKind::Orbit => {
                match self.camera.view {
                    View::Top => {
                        self.camera.yaw = 0.;
                        self.camera.pitch = 1.5;
                        self.camera.roll = 0.;
                        self.camera.view = View::Orthographic;
                    }
                    View::Front => {
                        self.camera.yaw = 0.;
                        self.camera.pitch = 0.;
                        self.camera.roll = 0.;
                        self.camera.view = View::Orthographic;
                    }
                    View::Right => {
                        self.camera.yaw = std::f64::consts::FRAC_PI_2;
                        self.camera.pitch = 0.;
                        self.camera.roll = 0.;
                        self.camera.view = View::Orthographic;
                    }
                    _ => {}
                }
                self.camera.yaw -= delta.x as f64 * 0.008;
                self.camera.pitch = (self.camera.pitch + delta.y as f64 * 0.008).clamp(-1.5, 1.5);
            }
            // Handled above.
            input::NavigationKind::Flick
            | input::NavigationKind::Roll
            | input::NavigationKind::PanVertical
            | input::NavigationKind::BoxZoom => {}
        }
    }
    /// A component Move, Rotate, Scale or Transform drag that moves every
    /// point of the active mesh by one affine map (nothing selected, or
    /// everything): the tool's base item and that map in the mesh's own
    /// coordinates. The viewport then draws the base mesh, whose GPU
    /// geometry and subdivision surface are already cached, through the
    /// map, instead of rebuilding the whole mesh every frame. `None` when
    /// any point moved differently (a partial selection, falloff, symmetry).
    fn whole_mesh_drag(
        &self,
        scene: &operandi_core::Scene,
    ) -> Option<(operandi_core::Item, glam::DMat4)> {
        let live = self.editor.live.as_ref()?;
        if !live.tool.transforms_items() || scene.selection.mode == Mode::Item {
            return None;
        }
        let id = scene.selection.item?;
        let now = scene.item(id)?;
        let base = live.base.item(id)?;
        if operandi_core::Shared::same(now.mesh.shared(), base.mesh.shared())
            || now.transform != base.transform
            || now.parent != base.parent
            || now.mesh.faces.len() != base.mesh.faces.len()
        {
            return None;
        }
        let basis = self
            .whole_mesh_basis_memo
            .get(&live.base, || whole_mesh_basis(&base.mesh.vertices))?;
        let map = whole_mesh_map_from(&base.mesh.vertices, &now.mesh.vertices, basis)?;
        Some((base.clone(), map))
    }
    fn gesture_move(&mut self, g: &input::Navigation, pos: Pos2, r: Rect) {
        match &g.kind {
            input::GestureKind::Falloff(handle, start) => {
                self.drag_falloff(*handle, *start, g.start, pos, r)
            }
            input::GestureKind::FalloffDraw { origin } => self.falloff_draw_drag(*origin, pos, r),
            input::GestureKind::Airbrush(stroke) => self.airbrush_drag(pos, r, *stroke),
            input::GestureKind::AirbrushSize { layer, start } => {
                self.airbrush_size_drag(*layer, *start, g.start, pos)
            }
            input::GestureKind::ItemHandle(grab) => self.drag_item_handle(*grab, pos, r),
            input::GestureKind::SolidDraw {
                cone,
                plane,
                anchor,
            } => self.solid_draw(*cone, *plane, *anchor, pos, r, g.modifiers.ctrl),
            input::GestureKind::SessionHeight { base, axis } => {
                self.session_height(*base, *axis, g.start, pos, r)
            }
            input::GestureKind::EdgeInsert { tool, a, b, ends } => {
                self.edge_insert_slide(*tool, *a, *b, *ends, pos)
            }
            input::GestureKind::Navigate(kind) => {
                self.navigate_gesture(*kind, g.start, g.last, pos, r)
            }
            input::GestureKind::PolyHaul(drag) => self.poly_haul_drag(drag, g.start, pos, r),
            input::GestureKind::Handle(axis, params, tool) => {
                self.update_handle(*axis, params.clone(), g.start, pos, *tool, r);
                self.note_drag_readout(*axis, *tool);
            }
            input::GestureKind::Tweak { pick, base } => {
                self.tweak_drag(*pick, base, g.start, pos, r)
            }
            input::GestureKind::Numeric(drag) => {
                self.numeric_drag(drag, g.start, pos, r, g.modifiers.ctrl)
            }
            input::GestureKind::Utility(drag) => self.utility_drag(drag, g.start, pos, r),
            input::GestureKind::Slice(drag) => self.slice_drag(drag, pos, r, g.modifiers),
            input::GestureKind::Sculpt => self.sculpt_drag(pos, r),
            #[cfg(feature = "painting")]
            input::GestureKind::Paint => self.paint_drag(pos, r),
            #[cfg(feature = "painting")]
            input::GestureKind::PaintSize { start } => self.paint_size_drag(*start, g.start, pos),
            input::GestureKind::SculptSize { start } => {
                self.sculpt_adjust_drag(true, *start, g.start, pos)
            }
            input::GestureKind::SculptStrength { start } => {
                self.sculpt_adjust_drag(false, *start, g.start, pos)
            }
            input::GestureKind::Duplicate(drag) => self.duplicate_drag(drag, pos, r, g.modifiers),
            input::GestureKind::TweakRange {
                metres_per_pixel, ..
            } => self.tweak_range_drag(*metres_per_pixel, g.last, pos),
            input::GestureKind::Bend(drag) => {
                self.bend_drag(drag, g.start, pos, r, g.modifiers.ctrl)
            }
            input::GestureKind::FalloffSteps { layer, start } => {
                self.falloff_steps_drag(*layer, *start, g.start, pos)
            }
            input::GestureKind::FalloffRadius {
                layer,
                start,
                metres_per_pixel,
            } => self.falloff_radius_drag(*layer, *start, *metres_per_pixel, g.start, pos),
            input::GestureKind::Primitive(gesture) => self.update_primitive(gesture, g, pos, r),
            input::GestureKind::Select => {
                if self.editor.scene.selection.mode == Mode::Item {
                    return;
                }
                let steps = g.last.distance(pos).ceil().clamp(1., 8192.) as usize;
                let surface = self.pick_surface();
                // Selecting never changes the meshes, so the picking surface
                // and the candidates hold for the whole move.
                let candidates = self.pick_candidates(g.last, pos, r);
                for step in 1..=steps {
                    let sample = g.last + (pos - g.last) * (step as f32 / steps as f32);
                    let mut mods = g.modifiers;
                    if !mods.ctrl {
                        mods.shift = true;
                    }
                    // Painting samples every pixel along the pointer's path;
                    // most land on a component already painted. Skip those
                    // instead of running a selection command each (that was
                    // most of a fast drag's frame on large meshes).
                    let hit = match self.editor.scene.selection.mode {
                        Mode::Vertex | Mode::Edge => {
                            self.pick_among(sample, r, &surface, &candidates)
                        }
                        _ => self.pick_component_on(sample, r, &surface),
                    };
                    if let Some((item, hit)) = hit
                        && self.paint_changes(item, hit, mods.ctrl)
                    {
                        self.click_select(Click {
                            hit: Some((item, hit)),
                            how: if mods.ctrl {
                                ClickHow::Remove
                            } else {
                                ClickHow::Add
                            },
                            over_skipped_mesh: false,
                        });
                        self.selection_status();
                    }
                }
            }
            _ => {}
        }
    }
    fn viewport_input(&mut self, ui: &Ui, response: &Response, r: Rect) {
        if self.show_plane
            || self.show_diagnostics
            || self.show_about
            || self.modal_owned_input
            || self.menu_owned_input
            || egui::Popup::is_any_open(ui.ctx())
            || self.file_dialog.is_some()
            || self.pending.is_some()
            || self.show_bindings
            || self.recovery.is_some()
            || self.recovery_warning.is_some()
            // A preset dragged out of the browser (PB-110) owns the
            // pointer: the views neither select nor run tools under it.
            || self.preset_browser.drag.is_some()
            // UI build mode: the views only move as panels (Jason,
            // 2026-11-20), they don't select, navigate or run tools.
            || self.dots.build_mode
        {
            if self.editor.selection_start.is_some() {
                self.run(Command::EndSelection { cancel: true });
            }
            self.navigation = None;
            self.drop_drag = None;
            return;
        }
        if !ui.input(|i| i.focused) {
            if self.editor.selection_start.is_some() {
                self.run(Command::EndSelection { cancel: true });
            }
            self.navigation = None;
            self.drop_drag = None;
            return;
        }
        // UIC-71: the frame and its controls take their own presses.
        let header = self.corner.header_hits.get(&self.drawing_view);
        let grip = self
            .dots
            .rects
            .get(&panel_dot::Dot::View(self.drawing_view));
        let drawing_falloff = self.falloff_draw_armed && self.screen_falloff_press().is_some();
        let events: Vec<Event> = ui
            .input(|i| i.events.clone())
            .into_iter()
            .filter(|event| {
                let Event::PointerButton {
                    pos,
                    button,
                    pressed: true,
                    modifiers,
                } = *event
                else {
                    return true;
                };
                // N03: a popover's title bar must not drive the active tool.
                if self.overlay_rects.iter().any(|rect| rect.contains(pos)) {
                    return false;
                }
                // UIC-61/181: a press on a window drawn above this view (a
                // popover such as the Add browser, a floating panel, a
                // menu) is that window's. A panel drawn after this view
                // claims the pointer (`menu_owned_input`) too late for
                // this frame, so ask egui which layer is on top instead: in
                // Item mode the press deselected the mesh, which closed the
                // Mesh Stack's Add Operator browser (bug report #5).
                if over_window_above(ui, pos) {
                    return false;
                }
                let over_grip = grip.is_some_and(|rect| rect.contains(pos));
                if over_grip && button == PointerButton::Primary && drawing_falloff {
                    return true;
                }
                !over_grip
                    && !header.is_some_and(|hits| hits.iter().any(|rect| rect.contains(pos)))
                    && !(r.contains(pos) && panel_dot::grip_shortcut(button, modifiers))
            })
            .collect();
        // Screen Axis searches along the view the tool is used in: note it
        // on each press here (Constrain to Background).
        if self.editor.background_constraint.is_some()
            && events.iter().any(|e| {
                matches!(e, Event::PointerButton { pos, pressed: true, .. } if r.contains(*pos))
            })
        {
            self.note_constraint_view();
        }
        let press_time = ui.input(|i| i.time);
        let double_click_delay = ui.ctx().options(|o| o.input_options.max_double_click_delay);
        let badge = Rect::from_min_max(
            r.left_bottom() + vec2(0., -90.),
            r.left_bottom() + vec2(98., 0.),
        );
        // A release this view didn't see (outside it, or a path that
        // returned early) still ends the held tweak.
        if self.pointer_held
            && events
                .iter()
                .all(|e| !matches!(e, Event::PointerButton { .. }))
            && !ui.input(|i| i.pointer.any_down())
        {
            self.release_pointer();
        }
        // A real mouse reports several moves per frame. A move followed by
        // another before any button change is superseded: gestures that
        // read only the start and the latest position skip it, so a drag
        // runs its tool update once per frame instead of once per event
        // (on a large mesh the per-event updates outran the frame and the
        // drag fell behind further every frame).
        let superseded: Vec<bool> = (0..events.len())
            .map(|i| {
                matches!(events[i], Event::PointerMoved(_))
                    && events[i + 1..]
                        .iter()
                        .take_while(|e| !matches!(e, Event::PointerButton { .. }))
                        .any(|e| matches!(e, Event::PointerMoved(_)))
            })
            .collect();
        let pointer_down = ui.input(|i| i.pointer.any_down());
        for (index, event) in events.into_iter().enumerate() {
            match event {
                Event::PointerMoved(_)
                    if superseded[index]
                        && self
                            .navigation
                            .as_ref()
                            .is_some_and(|g| latest_position_only(&g.kind)) => {}
                // Alt+Shift+middle-click: the default field of view (view_fov.rs).
                Event::PointerButton {
                    pos,
                    button: PointerButton::Middle,
                    pressed: true,
                    modifiers,
                } if r.contains(pos) && self.is_fov_reset_press(modifiers) => {
                    self.reset_fov(press_time);
                }
                Event::PointerButton {
                    pos,
                    button,
                    pressed: true,
                    modifiers,
                } if r.contains(pos)
                    && matches!(button, PointerButton::Secondary | PointerButton::Middle) =>
                {
                    if !self.confirm_fields() {
                        continue;
                    }
                    // Every press to its release is one undo step, whatever
                    // gesture it starts below.
                    self.hold_pointer();
                    // HM-01: modifiers are decided at the press.
                    self.begin_constrain(modifiers);
                    // Blender- and Maya-style presets navigate with these buttons.
                    if let Some(nav) = self
                        .input_preferences
                        .navigation
                        .navigation(button, modifiers)
                    {
                        self.navigation = Some(input::Navigation {
                            kind: input::GestureKind::Navigate(nav),
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // Solid Sketch: right drag scales, middle click deletes.
                    if let Some(kind) = self.solid_sketch_press(pos, r, button, modifiers) {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // SC-22/23: with a sculpt brush, right-drag sizes it and
                    // middle-drag sets its strength; a right-click that
                    // doesn't move still opens the menu (on release).
                    // PNT-53: with a paint tool, right-drag sizes the tip;
                    // middle-drag does nothing.
                    #[cfg(feature = "painting")]
                    if self.painting.brush.tool.is_some() && input::navigation(modifiers).is_none()
                    {
                        let kind = if button == PointerButton::Secondary {
                            self.begin_paint_size(pos)
                        } else {
                            input::GestureKind::Blocked
                        };
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    if self.sculpt.brush.is_some() && input::navigation(modifiers).is_none() {
                        let kind = if button == PointerButton::Secondary {
                            self.begin_sculpt_size(pos)
                        } else {
                            self.begin_sculpt_strength(pos)
                        };
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // MN-03.1: a drawn Cube, Sphere or Cylinder: a middle drag
                    // keeps it and starts a copy at the pointer with the same
                    // settings, which the drag then adjusts.
                    if button == PointerButton::Middle
                        && modifiers.is_none()
                        && self
                            .haul_tool(operandi_core::haul::Button::Middle)
                            .is_none()
                        && let Some(live) =
                            self.editor.live.clone().filter(|l| {
                                matches!(l.tool, Tool::Box | Tool::Sphere | Tool::Cylinder)
                            })
                    {
                        let kind = self.begin_primitive_copy(live, pos, r);
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // HM-04: with a transform tool, a middle press repeats the
                    // previous drag's amounts and a drag goes on from there; a
                    // click without moving keeps the repeat. Navigation
                    // presets that claim the button win (above).
                    if button == PointerButton::Middle
                        && modifiers.is_none()
                        && let Some(tool) = self
                            .editor
                            .live
                            .as_ref()
                            .map(|l| l.tool)
                            .filter(|t| t.transforms_items())
                    {
                        // RPT-02/07: a typed amount still showing counts as
                        // an apply, so record it as its own step first and
                        // the middle-click repeats it (type 15, click: 30°).
                        if self.editor.transform_change_pending() {
                            self.run(Command::CommitTransformStep);
                            self.clear_fields();
                        }
                        // The repeat and the drag after it are one tweak
                        // (the press holds the pointer, above).
                        let kind = match self.editor.repeat_transform_params() {
                            Ok(params) => {
                                self.run(Command::Preview(params));
                                self.free_transform_drag(tool)
                            }
                            Err(e) => {
                                self.status = e;
                                input::GestureKind::Blocked
                            }
                        };
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    if self.draw.is_some()
                        && input::navigation(modifiers).is_none()
                        && let Some(kind) = self.draw_press(
                            pos,
                            r,
                            modifiers,
                            if button == PointerButton::Secondary {
                                1
                            } else {
                                2
                            },
                        )
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // Pen walls: right-drag left/right sets Offset.
                    if button == PointerButton::Secondary
                        && input::navigation(modifiers).is_none()
                        && let Some(draft) = &self.editor.pen_draft
                        && draft.options().wall != WallMode::Off
                    {
                        let at = draft
                            .points()
                            .last()
                            .copied()
                            .unwrap_or(self.workplane_center());
                        let metres_per_pixel = self.camera.metres_per_pixel(at, r).unwrap_or(0.01);
                        self.navigation = Some(input::Navigation {
                            kind: input::GestureKind::PenOffset {
                                start_offset: draft.options().offset,
                                metres_per_pixel,
                            },
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    if button == PointerButton::Secondary
                        && self.utility.is_some()
                        && input::navigation(modifiers).is_none()
                        && let Some(kind) = self.utility_press(pos, r, modifiers, true)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    if button == PointerButton::Secondary
                        && input::navigation(modifiers).is_none()
                        && let Some(live) = self.editor.live.as_ref()
                        && live.tool.is_primitive()
                        // Sphere and Cylinder right-drag through the drag
                        // layer below; Box keeps its own Segments drag.
                        && self.haul_tool(operandi_core::haul::Button::Right).is_none()
                    {
                        self.navigation = Some(input::Navigation {
                            kind: self.begin_primitive_alternate(live.clone(), self.actual_plane()),
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // Duplicate tools: right-drag changes counts (CL-22,
                    // AR-22, RA-21); press to release is one step.
                    if button == PointerButton::Secondary
                        && input::navigation(modifiers).is_none()
                        && let Some(kind) = self.duplicate_count_press(pos)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // SPEC-SELECTION-FALLOFF-FLEX §1, FO-46: with a
                    // Selection falloff in the stack under a live tool, a
                    // right-drag anywhere changes Steps instead of the
                    // tool's own right-drag, and a right-click adds one
                    // (provisional, SF-Q7, FO-Q04).
                    if button == PointerButton::Secondary
                        && input::navigation(modifiers).is_none()
                        && let Some((layer, f)) = falloff_panel::selection_layer(&self.editor)
                        && self
                            .editor
                            .live
                            .as_ref()
                            .is_some_and(|l| l.tool.card().layers.falloff)
                    {
                        self.status = format!(
                            "Selection falloff Steps: {} · drag right for more, left for fewer",
                            f.steps
                        );
                        self.navigation = Some(input::Navigation {
                            kind: input::GestureKind::FalloffSteps {
                                layer,
                                start: f.steps,
                            },
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // FO-44: a right-drag resizes the Airbrush brush.
                    if button == PointerButton::Secondary
                        && input::navigation(modifiers).is_none()
                        && let Some(kind) = self.begin_airbrush_size()
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // MN-03.2: a transform tool's right-drag, where its row in
                    // the hauls table has no right button, sets up the falloff
                    // in use: Linear from the press to the pointer, Radial and
                    // Cylinder around the press, Screen's circle.
                    if button == PointerButton::Secondary
                        && input::navigation(modifiers).is_none()
                        && let Some(kind) = self.right_drag_falloff_setup(pos, r)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // FO-47 / FO-Q05 (provisional): with a Soft Selection
                    // falloff under a live tool, a right-drag changes
                    // Radius, right for more, left for less.
                    if button == PointerButton::Secondary
                        && input::navigation(modifiers).is_none()
                        && let Some((layer, f)) = falloff_panel::soft_selection_layer(&self.editor)
                        && self
                            .editor
                            .live
                            .as_ref()
                            .is_some_and(|l| l.tool.card().layers.falloff)
                        && let Some(kind) = self.begin_falloff_radius(layer, f.radius, pos, r)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // The shared drag layer (DL-01..09): after navigation,
                    // the falloff layer's own right-drag and the tool's
                    // other claims above, a right or middle drag drives the
                    // value the tool's row lists for that button. A middle
                    // click on Smooth Shift still clones it.
                    if input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && let Some(which) = crate::haul_view::button_of(button)
                        && let Some(kind) = {
                            let click = self.haul_click(which, modifiers);
                            self.begin_haul(which, pos, r, click)
                        }
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // TP-20: Topology Pen's right and middle gestures.
                    if matches!(button, PointerButton::Secondary | PointerButton::Middle)
                        && input::navigation(modifiers).is_none()
                        && self.topology_pen_open()
                    {
                        let kind = self.topology_pen_press(button, modifiers, pos, r);
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    // TWR-06/10: with Tweak, a right-drag anywhere resizes
                    // Range; Shift, Ctrl and Cmd change nothing. (Alt drags
                    // stay navigation, above.)
                    if button == PointerButton::Secondary
                        && input::navigation(modifiers).is_none()
                        && self
                            .editor
                            .live
                            .as_ref()
                            .is_some_and(|l| l.tool == Tool::Tweak)
                    {
                        // Press to release is one undo step (TWR-14).
                        self.navigation = Some(input::Navigation {
                            kind: self.begin_tweak_range(pos, r),
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    if button == PointerButton::Middle
                        && input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && let Some(kind) = self.slice_press(pos, r, modifiers, button)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    if button == PointerButton::Middle
                        && input::navigation(modifiers).is_none()
                        && self
                            .editor
                            .live
                            .as_ref()
                            .is_some_and(|l| l.tool.is_smooth_shift())
                    {
                        self.run(Command::CloneSmoothShift);
                        self.clear_fields();
                        continue;
                    }
                    // DPM-06/10 (D1): middle-click a Pen or Tube point
                    // deletes it in any mode; Shift also deletes every later
                    // point. A middle-click elsewhere does nothing.
                    if button == PointerButton::Middle
                        && input::navigation(modifiers).is_none()
                        && !modifiers.ctrl
                        && !modifiers.mac_cmd
                        && (self.editor.pen_draft.is_some() || self.tube_active())
                    {
                        self.point_delete_press(pos, r, modifiers.shift);
                        self.navigation = Some(input::Navigation {
                            kind: input::GestureKind::Blocked,
                            start: pos,
                            last: pos,
                            modifiers,
                            button,
                        });
                        continue;
                    }
                    if self.editor.live.is_some()
                        || self.editor.pen_draft.is_some()
                        || input::navigation(modifiers).is_some()
                    {
                        self.status="This tool/navigation context owns non-primary hauling; selection lasso is unavailable".into();
                        continue;
                    }
                    if self.editor.scene.selection.mode == Mode::Item {
                        if button == PointerButton::Secondary && self.mouse_region_click(pos, r) {
                            continue;
                        }
                        self.status =
                            "Lasso can't select items yet; select them in the Scenes list".into();
                        continue;
                    }
                    self.run(Command::BeginSelection);
                    self.navigation = Some(input::Navigation {
                        kind: input::GestureKind::Lasso(vec![pos], button == PointerButton::Middle),
                        start: pos,
                        last: pos,
                        modifiers,
                        button,
                    });
                }
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers,
                } if r.contains(pos) => {
                    if self.plane_widget && badge.contains(pos) {
                        continue;
                    }
                    self.hold_pointer();
                    if !self.confirm_fields() {
                        if let Some(id) = self.field_focus {
                            ui.memory_mut(|m| m.request_focus(id));
                        }
                        continue;
                    }
                    self.release_field_for_view_press(ui.ctx());
                    if self.editor.solid_sketch.is_some()
                        && !self.text_editor_focused(ui.ctx())
                        && let Some(kind) =
                            self.solid_sketch_press(pos, r, PointerButton::Primary, modifiers)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        ui.ctx().request_repaint();
                        continue;
                    }
                    if self.draw.is_some()
                        && input::navigation(modifiers).is_none()
                        && !self.text_editor_focused(ui.ctx())
                        && let Some(kind) = self.draw_press(pos, r, modifiers, 0)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        continue;
                    }
                    // A Create panel utility (Paste Tool, Ruler, Align…) takes
                    // the press when it wants it.
                    if self.utility.is_some()
                        && input::navigation(modifiers).is_none()
                        && !self.text_editor_focused(ui.ctx())
                        && let Some(kind) = self.utility_press(pos, r, modifiers, false)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        continue;
                    }
                    // A sculpt brush takes a plain, Ctrl or Shift press (SC-60,
                    // SC-61); Alt stays navigation (SC-64) and Cmd keeps its
                    // menu.
                    // PNT-48: a paint tool takes a plain press; Alt stays
                    // navigation, Cmd keeps its menu.
                    #[cfg(feature = "painting")]
                    if (self.painting.brush.tool.is_some() || self.painting.brush.lasso_mode)
                        && input::navigation(modifiers).is_none()
                        && !(modifiers.mac_cmd
                            && self.painting.brush.tool
                                != Some(operandi_core::paint_stroke::PaintTool::Clone))
                        && self
                            .input_preferences
                            .navigation
                            .navigation(PointerButton::Primary, modifiers)
                            .is_none()
                        && !self.text_editor_focused(ui.ctx())
                    {
                        let kind = self.paint_press(pos, r, modifiers);
                        ui.ctx().request_repaint();
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        continue;
                    }
                    if self.sculpt.brush.is_some()
                        && input::navigation(modifiers).is_none()
                        && !modifiers.mac_cmd
                        && self
                            .input_preferences
                            .navigation
                            .navigation(PointerButton::Primary, modifiers)
                            .is_none()
                        && !self.text_editor_focused(ui.ctx())
                    {
                        let kind = self.sculpt_press(pos, r, modifiers);
                        ui.ctx().request_repaint();
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        continue;
                    }
                    // Armed Edge panel tool (Add Loop / Add Point): a click on
                    // an edge adds one; Ctrl places it at the middle, Shift
                    // is an ordinary click (AL-PHY-02/03), and Command does
                    // nothing and opens no menu (AL-PHY-04).
                    if self.edge_click.is_some()
                        && modifiers.mac_cmd
                        && input::navigation(modifiers).is_none()
                    {
                        continue;
                    }
                    if self.edge_click.is_some()
                        && input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && !self.text_editor_focused(ui.ctx())
                    {
                        let (taken, slide) = self.edge_click_at(pos, r, modifiers.ctrl);
                        if let Some(kind) = slide {
                            self.navigation = Some(input::Navigation {
                                kind,
                                start: pos,
                                last: pos,
                                modifiers,
                                button: PointerButton::Primary,
                            });
                        }
                        if taken {
                            continue;
                        }
                    }
                    // CS-02/15: the first press with an armed tool makes it
                    // live at the click, then goes on as that tool's own
                    // press (click_start_view.rs).
                    if self.editor.is_armed()
                        && input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && !self.text_editor_focused(ui.ctx())
                        && !self.start_armed_tool(pos, r)
                    {
                        self.navigation = Some(input::Navigation {
                            kind: input::GestureKind::Blocked,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        continue;
                    }
                    // Duplicate tools take their presses (duplicate_view.rs).
                    if input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && !self.text_editor_focused(ui.ctx())
                        && let Some(kind) = self.duplicate_press(pos, r, modifiers)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        ui.ctx().request_repaint();
                        continue;
                    }
                    // DL-Q04 (provisional): on a Mac trackpad Cmd+drag
                    // stands in for a middle drag and Cmd+Shift+drag for a
                    // right drag, for tools whose drag-control row lists
                    // that button. Handles and the tool's own on-screen
                    // controls still win (DL-01).
                    if input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && !self.text_editor_focused(ui.ctx())
                        && self
                            .input_preferences
                            .navigation
                            .navigation(PointerButton::Primary, modifiers)
                            .is_none()
                        && let Some(which) = crate::haul_view::trackpad_button(modifiers)
                        && self.haul_tool(which).is_some()
                        && self.handle_hit(pos, r).is_none()
                        && !self.haul_on_tool_control(pos, r)
                        && let Some(kind) = {
                            let click = self.haul_click(which, modifiers);
                            self.begin_haul(which, pos, r, click)
                        }
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        ui.ctx().request_repaint();
                        continue;
                    }
                    // Slice tools take their presses (slice_view.rs).
                    if input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && !self.text_editor_focused(ui.ctx())
                        && let Some(kind) =
                            self.slice_press(pos, r, modifiers, PointerButton::Primary)
                    {
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        ui.ctx().request_repaint();
                        continue;
                    }
                    // Weld (WD-01): the press picks a point and drags it.
                    if self.vertex_weld
                        && input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && !self.text_editor_focused(ui.ctx())
                    {
                        let kind = self.weld_press(pos, r);
                        self.navigation = Some(input::Navigation {
                            kind,
                            start: pos,
                            last: pos,
                            modifiers,
                            button: PointerButton::Primary,
                        });
                        ui.ctx().request_repaint();
                        continue;
                    }
                    // A click adding a point with Create. A press on a
                    // handle is left to the handle gesture.
                    if input::navigation(modifiers).is_none()
                        && !self.editor_owned_input
                        && !self.text_editor_focused(ui.ctx())
                        && self
                            .editor
                            .live
                            .as_ref()
                            .is_some_and(|l| l.tool == Tool::VertexCreate)
                        && self.vertex_click_at(pos, r)
                    {
                        continue;
                    }
                    // Draw tools Pen: Shift-click ends this polygon and starts
                    // the next; it adds no point.
                    if self.editor.pen_draft.is_some()
                        && modifiers.shift_only()
                        && !self.editor_owned_input
                        && !self.text_editor_focused(ui.ctx())
                    {
                        if self.run_ok(Command::Drop) {
                            self.run(Command::PenStart);
                            self.status = "Pen · new polygon · click its first point".into();
                        }
                        continue;
                    }
                    // Make Quads: Ctrl+click ends the strip with a triangle.
                    let strip_end = modifiers.ctrl
                        && !modifiers.shift
                        && !modifiers.alt
                        && !modifiers.mac_cmd
                        && self
                            .editor
                            .pen_draft
                            .as_ref()
                            .is_some_and(|d| d.options().strip())
                        && self
                            .input_preferences
                            .navigation
                            .navigation(PointerButton::Primary, modifiers)
                            .is_none();
                    if self.editor.pen_draft.is_some() && (modifiers.is_none() || strip_end) {
                        if self.editor_owned_input || self.text_editor_focused(ui.ctx()) {
                            self.status =
                                "Finish editing the field before capturing a Pen point".into();
                        } else if let Some(point) = self.pen_press(pos, r, strip_end) {
                            // A click adds a vertex; pressing on a
                            // vertex of this session picks it; dragging moves it.
                            self.navigation = Some(input::Navigation {
                                kind: input::GestureKind::PenDrag { point },
                                start: pos,
                                last: pos,
                                modifiers,
                                button: PointerButton::Primary,
                            });
                        }
                        continue;
                    }
                    // Drop Action (DA-4): a press on an item in Item mode
                    // with no tool may become a drop; it waits for release.
                    if self.drop_press(pos, r, modifiers) {
                        ui.ctx().request_repaint();
                        continue;
                    }
                    // HM-01: modifiers are decided at the press.
                    self.begin_constrain(modifiers);
                    // HM-02: Shift-drag with a transform tool continues the
                    // drag just made (its values keep growing); a plain drag
                    // starts fresh.
                    let continues = modifiers.shift
                        && !modifiers.ctrl
                        && !modifiers.alt
                        && !modifiers.mac_cmd
                        && self
                            .input_preferences
                            .navigation
                            .navigation(PointerButton::Primary, modifiers)
                            .is_none()
                        && self
                            .editor
                            .live
                            .as_ref()
                            .is_some_and(|l| l.tool.transforms_items());
                    if continues {
                        self.run(Command::ContinueTransformStep);
                    }
                    // MN-03.3: with the Screen falloff on, Bevel and Twist
                    // Bevel follow the transform rule: a plain drag starts a
                    // fresh edit on the last one's result, Shift keeps
                    // adjusting the same edit.
                    let screen_rule = self.screen_fresh_rule();
                    if screen_rule
                        && modifiers.is_none()
                        && !self.falloff_draw_armed
                        && self.falloff_hit(pos, r).is_none()
                        && self
                            .editor
                            .live
                            .as_ref()
                            .is_some_and(|l| l.applied && self.editor.scene != l.base)
                    {
                        self.run(Command::Continue);
                        self.bevel_dragged = false;
                    }
                    // Soft Drag (DEFORM, FO-43): every press centres the
                    // screen circle on the pointer before the drag moves
                    // what is inside it.
                    if modifiers.is_none() {
                        self.soft_drag_press(pos, r);
                    }
                    let kind = if let Some(nav) = self
                        .input_preferences
                        .navigation
                        .navigation(PointerButton::Primary, modifiers)
                    {
                        input::GestureKind::Navigate(nav)
                    } else if modifiers.is_none() && self.uv_view_project_click(r) {
                        // UVT-19: Project from View projects from this view.
                        input::GestureKind::Blocked
                    } else if let (Some(handle), Some(field)) =
                        (self.falloff_hit(pos, r), self.editor.spatial_falloff)
                    {
                        self.path_drag_base = Some(self.editor.falloff_path.clone());
                        input::GestureKind::Falloff(handle, field)
                    } else if let Some(f) = self.screen_falloff_press()
                        && let Some(stroke) = if matches!(f, Falloff::Airbrush(_)) {
                            // MN-03.4: Shift lowers, Ctrl (Cmd) erases.
                            falloff_screen_view::AirbrushStroke::for_press(modifiers)
                        } else {
                            modifiers
                                .is_none()
                                .then_some(falloff_screen_view::AirbrushStroke::Add)
                        }
                    {
                        // FO-43/52 draw, FO-44 paint.
                        self.begin_screen_falloff(f, pos, r, stroke)
                    } else if modifiers.is_none() && self.path_falloff_click(pos, r) {
                        // FO-53: the click added a Path point.
                        input::GestureKind::Blocked
                    } else if modifiers.is_none() && self.falloff_draw_ready() {
                        // FO-05: the press after choosing a geometric
                        // falloff draws it.
                        self.begin_falloff_draw(pos, r)
                    } else if let Some(click) = self.center_click
                        && modifiers.is_none()
                    {
                        self.place_action_center(click, pos, r);
                        input::GestureKind::Blocked
                    } else if let Some(grab) = self.item_handle_hit(pos, r)
                        && modifiers.is_none()
                    {
                        input::GestureKind::ItemHandle(grab)
                    } else if self.editor.pen_draft.is_some() {
                        self.status = "Pen capture uses an unmodified primary click".into();
                        input::GestureKind::Blocked
                    } else if let Some(height) = self.begin_session_height(pos, r) {
                        height
                    } else if self.editor.reduction.is_some()
                        || self.editor.bridge.is_some()
                        || self.editor.spike.is_some()
                        || self.editor.shape.is_some()
                    {
                        // A drag anywhere drives these numeric sessions too
                        // (Jason, 2026-11-04; DRAG-Q01; was RED-002, BRI-Q05,
                        // SA-Q01, NPR-Q01).
                        if modifiers.is_none() {
                            self.begin_numeric_drag(pos, r)
                        } else {
                            input::GestureKind::Blocked
                        }
                    } else if self.editor.ellipsoid.is_some() {
                        self.begin_solid_draw(false, pos, r)
                    } else if self.editor.cone.is_some() {
                        self.begin_solid_draw(true, pos, r)
                    } else if let Some(l) = self.editor.live.clone() {
                        if l.tool.is_primitive() {
                            self.begin_primitive(l, pos, r, modifiers)
                        } else if l.tool == Tool::Tweak && self.topology_pen_open() {
                            // TP-10/20: the pen's Mode and modifier gestures.
                            self.topology_pen_press(PointerButton::Primary, modifiers, pos, r)
                        } else if l.tool == Tool::Tweak {
                            // HM-61/71: Ctrl locks the drag to one axis.
                            let ctrl_only = modifiers.ctrl
                                && !modifiers.shift
                                && !modifiers.alt
                                && !modifiers.mac_cmd;
                            if modifiers.is_none() || ctrl_only {
                                self.begin_tweak(pos, r)
                            } else {
                                self.status = "Tweak: drag without modifier keys, or with Ctrl to keep to one axis".into();
                                input::GestureKind::Blocked
                            }
                        } else if l.tool == Tool::SketchExtrude && modifiers.is_none() {
                            let center = l
                                .base
                                .active()
                                .and_then(|item| {
                                    l.base
                                        .selection
                                        .faces
                                        .first()
                                        .map(|&face| item.mesh.center(face as usize))
                                })
                                .unwrap_or(DVec3::ZERO);
                            let camera = self.camera.clone();
                            let normal = camera.basis().3;
                            if let Some(anchor) =
                                Self::sketch_plane_point(&camera, r, pos, center, normal)
                            {
                                input::GestureKind::SketchStroke(input::SketchStroke {
                                    camera,
                                    rect: r,
                                    plane_center: center,
                                    plane_normal: normal,
                                    last_sample: pos,
                                    precision: l.params.sketch_precision,
                                    samples: vec![anchor],
                                    current: anchor,
                                    started: false,
                                })
                            } else {
                                self.status =
                                    "Sketch Extrude cannot initialize its camera-facing plane"
                                        .into();
                                input::GestureKind::Blocked
                            }
                        } else if l.tool == Tool::PolyHaul && modifiers.is_none() {
                            // Selection modes: a drag anywhere (Jason,
                            // 2026-11-04; DRAG-Q01). Element modes: the
                            // element under the pointer.
                            self.begin_poly_haul(&l, pos, r)
                        } else if l.tool.is_smooth_shift()
                            && modifiers.shift
                            && !modifiers.ctrl
                            && !modifiers.alt
                        {
                            self.run(Command::Continue);
                            if let Some(l) = &self.editor.live {
                                self.run(Command::Preview(l.params.clone()));
                            }
                            input::GestureKind::Blocked
                        } else if l.tool == Tool::Extrude && !l.applied && !self.editor.i2_active()
                        {
                            // First click creates the extrusion in place
                            // (zero distance); the gizmo appears afterwards.
                            if self.editor.i2_begin_source_supported() {
                                self.run(Command::BeginExtrude);
                            } else {
                                self.run(Command::StartExtrude);
                            }
                            input::GestureKind::Blocked
                        } else if l.tool.is_smooth_shift() && !l.applied {
                            self.run(Command::Preview(l.params));
                            input::GestureKind::Blocked
                        } else if modifiers.ctrl
                            && modifiers.shift
                            && !modifiers.alt
                            && l.tool.transforms_items()
                            && let Some(axis) = self.handle_hit(pos, r)
                        {
                            // IT-PHY-04: Ctrl+Shift on a handle duplicates the
                            // item and drags the copy, leaving the original.
                            // Provisional: a plain copy, not a linked instance.
                            // HM-06: in Vertex/Edge/Polygon mode it copies the
                            // whole selected polygons into the same mesh.
                            let tool = l.tool;
                            let component = self.editor.scene.selection.mode != Mode::Item;
                            self.run(Command::Drop);
                            let copied = if component {
                                self.run_ok(Command::DuplicateSelectedPolygons)
                            } else {
                                self.run_ok(Command::DuplicateItem)
                            };
                            let status = self.status.clone();
                            self.activate(tool);
                            let captured = if copied {
                                i2_route::capture(&mut self.editor, axis, false)
                            } else {
                                // Nothing to copy: the tool comes back and the
                                // refusal stays on the status line.
                                Err(status)
                            };
                            match captured {
                                Ok((params, ticket)) => {
                                    self.i2_gesture_ticket = ticket;
                                    self.clear_fields();
                                    input::GestureKind::Handle(axis, params, tool)
                                }
                                Err(e) => {
                                    self.status = e;
                                    input::GestureKind::Blocked
                                }
                            }
                        } else if let Some(axis) = self.handle_hit(pos, r) {
                            // Jason, 2026-11-09: a second handle drag adjusts
                            // the same bevel while the tool stays on (Drop ends
                            // it); `BEVEL_REDRAG_STACKS` flips back to bevel-drag
                            // G2, where a drag after a released one commits that
                            // bevel and starts the next on its cap. Panel edits
                            // adjust the current bevel either way (TA-02).
                            if self.bevel_redrag_stacks
                                && !screen_rule
                                && l.tool.is_bevel()
                                && modifiers.is_none()
                                && std::mem::take(&mut self.bevel_dragged)
                                && self.editor.scene != l.base
                            {
                                self.run(Command::Continue);
                            }
                            // Jason, 2026-11-09: Shift+drag on a handle keeps
                            // this bevel and shapes a fresh one on its caps.
                            // MN-03.3: not with the Screen falloff on, where
                            // Shift keeps adjusting this one.
                            if l.tool.is_bevel() && modifiers.shift_only() && !screen_rule {
                                self.run(Command::Continue);
                            }
                            match i2_route::capture(&mut self.editor, axis, modifiers.is_none()) {
                                Ok((params, ticket)) => {
                                    self.i2_gesture_ticket = ticket;
                                    self.clear_fields();
                                    input::GestureKind::Handle(axis, params, l.tool)
                                }
                                Err(e) => {
                                    self.status = e;
                                    input::GestureKind::Blocked
                                }
                            }
                        } else if l.tool == Tool::VertexBevel && modifiers.is_none() {
                            match i2_route::capture(
                                &mut self.editor,
                                VERTEX_BEVEL_FREE_HANDLE,
                                false,
                            ) {
                                Ok((params, _)) => {
                                    self.clear_fields();
                                    input::GestureKind::Handle(
                                        VERTEX_BEVEL_FREE_HANDLE,
                                        params,
                                        Tool::VertexBevel,
                                    )
                                }
                                Err(e) => {
                                    self.status = e;
                                    input::GestureKind::Blocked
                                }
                            }
                        } else if l.tool == Tool::Extrude && !modifiers.shift {
                            // Off the gizmo: move the extrusion freely in the
                            // view plane (user correction 2026-10-30).
                            match i2_route::capture(&mut self.editor, EXTRUDE_FREE_HANDLE, false) {
                                Ok((params, ticket)) => {
                                    self.i2_gesture_ticket = ticket;
                                    self.clear_fields();
                                    input::GestureKind::Handle(
                                        EXTRUDE_FREE_HANDLE,
                                        params,
                                        Tool::Extrude,
                                    )
                                }
                                Err(e) => {
                                    self.status = e;
                                    input::GestureKind::Blocked
                                }
                            }
                        } else if l.tool == Tool::EdgeExtend && modifiers.shift_only() {
                            // Spec: Shift+click keeps this extension and
                            // extends again from the new edges with the same
                            // values; a drag then shapes the new one like a
                            // plain drag anywhere.
                            if !self.run_ok(Command::Continue) {
                                input::GestureKind::Blocked
                            } else {
                                match i2_route::capture(&mut self.editor, OFF_HANDLE, false) {
                                    Ok((params, ticket)) => {
                                        self.i2_gesture_ticket = ticket;
                                        self.clear_fields();
                                        input::GestureKind::Handle(OFF_HANDLE, params, l.tool)
                                    }
                                    Err(e) => {
                                        self.status = e;
                                        input::GestureKind::Blocked
                                    }
                                }
                            }
                        } else if l.tool.is_bevel() && modifiers.shift_only() && !screen_rule {
                            // Jason, 2026-11-09: Shift+click or Shift+drag
                            // keeps this bevel and starts a fresh one (values
                            // reset) on the current polygon selection; a drag
                            // then shapes it like a plain drag anywhere. The
                            // selection stays as it is (provisional, TA-Q02).
                            self.run(Command::Continue);
                            match i2_route::capture(&mut self.editor, OFF_HANDLE, false) {
                                Ok((params, ticket)) => {
                                    self.i2_gesture_ticket = ticket;
                                    self.clear_fields();
                                    input::GestureKind::Handle(OFF_HANDLE, params, l.tool)
                                }
                                Err(e) => {
                                    self.status = e;
                                    input::GestureKind::Blocked
                                }
                            }
                        } else if matches!(l.tool, Tool::Bevel | Tool::TwistBevel | Tool::Extrude)
                            && !screen_rule
                            && modifiers.shift
                            && !modifiers.alt
                            && !modifiers.ctrl
                            && !modifiers.command
                            && !modifiers.mac_cmd
                        {
                            self.run(Command::Continue);
                            // No unreviewed off-handle displacement law is invented.
                            // Capture the press so release cannot select/repeat again.
                            input::GestureKind::Blocked
                        } else if l.tool.is_transform()
                            && (!modifiers.shift || continues)
                            && !modifiers.alt
                            && !modifiers.mac_cmd
                        // HM-10 / HM-20 / HM-32 / HM-40: Ctrl hauls are
                        // constrained (axis lock, snapped turns; Scale's
                        // haul is uniform either way).
                        {
                            // Off the handles: away from the model the center
                            // jumps to the press; on the model it stays. Both
                            // then drag freely in the view plane (Jason,
                            // 2026-11-03). HM-02: a Shift haul adds to the
                            // continued drag where it is; the center stays.
                            let picked = (!continues).then(|| self.pick_component(pos, r));
                            if !continues && self.element_falloff().is_some() {
                                // FO-45: a press picks the element under the
                                // pointer and the drag moves it.
                                self.begin_element_falloff_drag(l.tool, pos, r)
                            } else {
                                match picked {
                                    Some(None) => self.relocate_transform_center(l.tool, pos, r),
                                    // A press on the selected element puts a
                                    // moved center back on the selection (Jason,
                                    // 2026-11-15; provisional, VAC-Q02).
                                    Some(Some((id, c))) if self.is_selected_component(id, c) => {
                                        self.run(Command::ResetActionCenter);
                                        self.free_transform_drag(l.tool)
                                    }
                                    _ => self.free_transform_drag(l.tool),
                                }
                            }
                        } else if l.tool == Tool::Bend
                            && !modifiers.shift
                            && !modifiers.alt
                            && !modifiers.mac_cmd
                        {
                            // Bend v2: the ring bends, the spine's end moves
                            // the spine, any other press moves the centre.
                            self.begin_bend(pos, r)
                        } else if (modifiers.is_none() || (screen_rule && modifiers.shift_only()))
                            && off_handle_drag(l.tool)
                        {
                            // Element Push (FO-45): with an Element falloff a
                            // press first picks the element under the
                            // pointer; nothing under it, nothing moves.
                            let picked = self.element_falloff().is_none() || {
                                match self.tweak_pick(pos, r) {
                                    Some(pick) => {
                                        self.run_ok(Command::FalloffElementPick(Some(pick.grab)))
                                    }
                                    None => {
                                        self.status =
                                            "Element falloff: press on a vertex, edge or polygon"
                                                .into();
                                        false
                                    }
                                }
                            };
                            // Off the handles: a drag anywhere drives the tool
                            // (Jason, 2026-11-04; DRAG-Q01).
                            if !picked {
                                input::GestureKind::Blocked
                            } else {
                                match i2_route::capture(&mut self.editor, OFF_HANDLE, false) {
                                    Ok((params, ticket)) => {
                                        self.i2_gesture_ticket = ticket;
                                        self.clear_fields();
                                        input::GestureKind::Handle(OFF_HANDLE, params, l.tool)
                                    }
                                    Err(e) => {
                                        self.status = e;
                                        input::GestureKind::Blocked
                                    }
                                }
                            }
                        } else {
                            self.status="Drag an explicit handle. Overlapping handles require orbiting. Edit fields or Drop/Cancel to finish.".into();
                            input::GestureKind::Blocked
                        }
                    } else if let Some(kind) = self.fill_press(pos, r, modifiers) {
                        kind
                    } else if modeling_menu::wants_menu(modifiers) {
                        // Capture the press; the menu opens on release (as the
                        // selection context menu does), so the release doesn't
                        // count as a click outside it and close it again.
                        self.modeling_menu_pressed = true;
                        input::GestureKind::Blocked
                    } else if let Some(kind) = self.polygon_connected_double_click(
                        pos,
                        r,
                        modifiers,
                        press_time,
                        double_click_delay,
                    ) {
                        kind
                    } else if let Some(kind) = self.edge_loop_double_click(
                        pos,
                        r,
                        modifiers,
                        press_time,
                        double_click_delay,
                    ) {
                        kind
                    } else {
                        // A possible first press of an Edge (loop) or
                        // Polygon (connected) double-click.
                        if matches!(self.editor.scene.selection.mode, Mode::Edge | Mode::Polygon) {
                            self.last_component_press = Some((press_time, pos));
                        }
                        self.run(Command::BeginSelection);
                        self.select_at(pos, r, modifiers);
                        input::GestureKind::Select
                    };
                    ui.ctx().request_repaint();
                    self.navigation = Some(input::Navigation {
                        kind,
                        start: pos,
                        last: pos,
                        modifiers,
                        button: PointerButton::Primary,
                    });
                }
                Event::PointerMoved(pos) => {
                    if self.drop_drag.is_some() {
                        let _ = pos;
                        ui.ctx().request_repaint();
                    }
                    if let Some(mut g) = self.navigation.take() {
                        // A press that became a paint drag is no first click
                        // of a double-click, even if it comes back.
                        if matches!(g.kind, input::GestureKind::Select)
                            && pos.distance(g.start) > EDGE_DOUBLE_CLICK_DISTANCE
                        {
                            self.last_component_press = None;
                        }
                        if let input::GestureKind::Draw(d) = &mut g.kind {
                            let start = g.start;
                            self.draw_drag(d, start, pos, r);
                        }
                        if let input::GestureKind::SolidSketch(d) = &mut g.kind {
                            let start = g.start;
                            self.solid_sketch_drag(d, start, pos, r);
                        }
                        if let input::GestureKind::PenDrag { point } = g.kind
                            && pos.distance(g.start) >= 1.
                        {
                            self.pen_drag(point, pos, r);
                        }
                        if matches!(g.kind, input::GestureKind::WeldDrag) {
                            self.weld_move(pos, r);
                        }
                        if let input::GestureKind::Haul(drag) = &mut g.kind {
                            self.haul_move(drag, pos);
                        }
                        if let input::GestureKind::PenOffset {
                            start_offset,
                            metres_per_pixel,
                        } = g.kind
                        {
                            self.pen_offset_drag(start_offset, metres_per_pixel, pos.x - g.start.x);
                        }
                        if let input::GestureKind::Lasso(path, _) = &mut g.kind
                            && path.last().is_none_or(|p| p.distance(pos) >= 1.)
                        {
                            if path.len() < 4096 {
                                path.push(pos);
                            } else {
                                self.status = "Lasso path budget is 4096 points".into();
                            }
                        }
                        if let input::GestureKind::ScreenFalloffDraw { start, path } = &mut g.kind {
                            let start = *start;
                            self.screen_falloff_drag(start, path, pos, r);
                        }
                        if let input::GestureKind::SketchStroke(stroke) = &mut g.kind {
                            self.sketch_stroke_move(stroke, pos, false);
                        } else {
                            self.gesture_move(&g, pos, r);
                        }
                        ui.ctx().request_repaint();
                        g.last = pos;
                        self.navigation = Some(g);
                    }
                }
                Event::PointerButton {
                    pos,
                    button,
                    pressed: false,
                    ..
                } => {
                    if std::mem::take(&mut self.modeling_menu_pressed) {
                        self.modeling_menu_requested = true;
                    }
                    if button == PointerButton::Primary && self.drop_drag.is_some() {
                        self.drop_release(pos, r);
                        ui.ctx().request_repaint();
                        continue;
                    }
                    if let Some(mut g) = self.navigation.take() {
                        if g.button != button {
                            self.navigation = Some(g);
                            continue;
                        }
                        // NAV-03 flick and NAV-07 box zoom finish on release.
                        if let input::GestureKind::Navigate(kind) = g.kind {
                            self.navigation_release(kind, g.start, pos, r, ui.ctx());
                        }
                        if let input::GestureKind::SolidSketch(d) = &mut g.kind {
                            let start = g.start;
                            self.solid_sketch_drag(d, start, pos, r);
                            let d = d.clone();
                            self.solid_sketch_release(&d, start, pos);
                            self.release_pointer();
                            ui.ctx().request_repaint();
                            continue;
                        }
                        if let input::GestureKind::PenDrag { .. }
                        | input::GestureKind::PenOffset { .. } = g.kind
                        {
                            self.i2_gesture_ticket = None;
                            self.snap_session = Default::default();
                            self.snap_marker = None;
                            self.snap_reticle = None;
                            ui.ctx().request_repaint();
                            continue;
                        }
                        if matches!(g.kind, input::GestureKind::WeldDrag) {
                            self.weld_release(pos, g.start, r);
                            ui.ctx().request_repaint();
                            continue;
                        }
                        if matches!(g.kind, input::GestureKind::Blocked)
                            && button == PointerButton::Primary
                            && g.modifiers.is_none()
                            && g.start.distance(pos) < 3.
                            && g.start.distance(g.last) < 3.
                            && self
                                .editor
                                .reduction
                                .as_ref()
                                .is_some_and(|r| !r.initialized())
                        {
                            let succeeded = self.run_ok(Command::InitializeReduction);
                            if succeeded {
                                self.status = self.reduction_status();
                            }
                        }
                        if let input::GestureKind::Lasso(path, middle) = &mut g.kind {
                            path.push(pos);
                            if g.start.distance(pos) < 1.
                                && path.iter().all(|p| p.distance(g.start) < 1.)
                            {
                                self.run(Command::EndSelection { cancel: true });
                                if !*middle && !self.mouse_region_click(g.start, r) {
                                    self.selection_context_requested = true;
                                }
                            } else {
                                self.finish_lasso(path, *middle, g.modifiers, r);
                            }
                        }
                        if let input::GestureKind::ScreenFalloffDraw { start, path } = &mut g.kind {
                            let start = *start;
                            self.screen_falloff_drag(start, path, pos, r);
                            self.screen_falloff_release();
                        }
                        if let input::GestureKind::SketchStroke(stroke) = &mut g.kind {
                            self.sketch_stroke_move(stroke, pos, true);
                        } else {
                            self.gesture_move(&g, pos, r);
                        }
                        ui.ctx().request_repaint();
                        if let input::GestureKind::Utility(drag) = &g.kind {
                            self.utility_release(drag, pos, r);
                        }
                        if let input::GestureKind::Slice(drag) = &g.kind {
                            self.slice_release(drag);
                        }
                        if let input::GestureKind::Duplicate(drag) = &g.kind {
                            self.path_tool_release(drag, r);
                        }
                        if let input::GestureKind::Haul(drag) = &mut g.kind {
                            self.haul_move(drag, pos);
                            let drag = drag.clone();
                            self.haul_release(&drag, pos, r);
                        }
                        if let input::GestureKind::Draw(d) = &mut g.kind {
                            let start = g.start;
                            self.draw_release(d, start, pos, r);
                        }
                        if matches!(g.kind, input::GestureKind::Select) {
                            self.run(Command::EndSelection { cancel: false });
                        }
                        // Each transform drag is its own undo step; the tool
                        // stays active (Jason, 2026-11-03).
                        if let input::GestureKind::Handle(_, _, tool) = g.kind
                            && tool.is_transform()
                        {
                            self.run(Command::CommitTransformStep);
                        }
                        if matches!(g.kind, input::GestureKind::Handle(..)) {
                            self.release_drag_readout(ui.ctx().input(|i| i.time));
                        }
                        if let input::GestureKind::Handle(id, _, _) = g.kind {
                            self.finish_precision_handle(id);
                        }
                        if let input::GestureKind::Handle(_, _, tool) = g.kind
                            && tool.is_bevel()
                        {
                            self.bevel_dragged = true;
                        }
                        // TWK-24: each Tweak drag is one undo step; a click
                        // that didn't move records nothing.
                        if let input::GestureKind::Tweak { pick, .. } = g.kind {
                            // TP-30: a pen drag with a weld waiting is one
                            // move-and-weld edit instead.
                            if !(self.topology_pen_open() && self.pen_tweak_release(pick)) {
                                self.run(Command::CommitTransformStep);
                            }
                        }
                        // SC-90: one stroke, one undo step.
                        match g.kind {
                            input::GestureKind::Sculpt => self.finish_sculpt_stroke(),
                            #[cfg(feature = "painting")]
                            input::GestureKind::Paint => self.finish_paint_stroke(),
                            #[cfg(feature = "painting")]
                            input::GestureKind::PaintSize { .. } => self.paint_size_release(),
                            input::GestureKind::SculptSize { .. } => {
                                self.sculpt_adjust_release(true, g.start, pos)
                            }
                            input::GestureKind::SculptStrength { .. } => {
                                self.sculpt_adjust_release(false, g.start, pos)
                            }
                            input::GestureKind::FalloffSteps { layer, start } => {
                                self.falloff_steps_release(layer, start, g.start, pos)
                            }
                            _ => {}
                        }
                        // The press is over: the next change is a new tweak.
                        self.release_pointer();
                        self.snap_session = Default::default();
                        self.snap_marker = None;
                        self.snap_reticle = None;
                        self.i2_gesture_ticket = None;
                    }
                }
                // A press still held when the pointer leaves the window
                // keeps going until its release, as egui's own drags do:
                // orbiting, a tool drag or a stroke that runs off the edge
                // doesn't stop dead (Windows tester, 2026-11-21). The OS
                // still sends the moves and the release to this window.
                // Losing focus still ends it, below.
                Event::PointerGone if pointer_down => {}
                Event::WindowFocused(false) | Event::PointerGone => {
                    self.cancel_drop();
                    self.release_pointer();
                    // An abandoned Command-press must not open the menu later.
                    self.modeling_menu_pressed = false;
                    if self.editor.selection_start.is_some() {
                        self.run(Command::EndSelection { cancel: true });
                    }
                    self.navigation = None;
                    // A stroke the pointer left keeps what it did.
                    self.finish_sculpt_stroke();
                    #[cfg(feature = "painting")]
                    self.finish_paint_stroke();
                    self.sculpt.adjust_at = None;
                    let lost_capture = self.i2_gesture_ticket.take().is_some();
                    if lost_capture {
                        let _ = self.editor.identity_ticket();
                    }
                }
                _ => {}
            }
        }
        if response.hovered()
            && !self.editor_owned_input
            && !egui::Popup::is_any_open(ui.ctx())
            && !self.fov_wheel(ui)
        {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0. {
                self.wheel_zoom(response.hover_pos(), scroll, r);
            }
        }
    }
    /// The nearest element of any kind under `p` (Closest rollovers and
    /// Mouse Regions Trigger Selections): a point or edge within the pick
    /// radius, weighed by Point Selection Priority, else the polygon under
    /// the pointer.
    pub(super) fn pick_closest(&self, p: Pos2, r: Rect) -> Option<(u64, Component)> {
        let surface = self.pick_surface();
        self.pick_closest_on(p, r, &surface)
    }
    fn pick_closest_on(
        &self,
        p: Pos2,
        r: Rect,
        surface: &picking::Surface,
    ) -> Option<(u64, Component)> {
        let mut candidates = self.pick_candidates_in(Mode::Vertex, p, p, r);
        candidates.edges = self.pick_candidates_in(Mode::Edge, p, p, r).edges;
        self.pick_among(p, r, surface, &candidates).or_else(|| {
            surface
                .pick_in(&self.camera, p, r, crate::picking::limit(&self.pick_only()))
                .map(|(id, f, _)| (id, Component::Polygon(f)))
        })
    }
    /// The items viewport picking sees; empty means every item. With Pick
    /// Other Meshes in Component Modes off, Vertex, Edge and Polygon picks
    /// see only the active meshes (`Scene::active_meshes`). Other meshes
    /// then neither pick nor hide anything (a ray passes through them), so
    /// a click never switches the active mesh. With no shown active mesh
    /// picking sees every item, so a click can always start somewhere.
    pub(super) fn pick_only(&self) -> Vec<u64> {
        let scene = &self.editor.scene;
        if self.selection_preferences.pick_other_meshes
            || !matches!(
                scene.selection.mode,
                Mode::Vertex | Mode::Edge | Mode::Polygon
            )
        {
            return Vec::new();
        }
        scene.shown_active_meshes()
    }
    /// Transparency: Handles. Where a surface hides a handle (its middle,
    /// at the action centre's depth), it draws at this opacity. Not while
    /// a drag changes the scene: the picking surface is stale then and
    /// rebuilding it every frame costs too much on large meshes.
    pub(super) fn handle_fade(&self, a: Pos2, b: Pos2, r: Rect) -> f32 {
        let xray = self.handle_preferences.xray_opacity();
        let Some(center) = self
            .editor
            .live
            .as_ref()
            .and_then(|l| l.action_frame)
            .map(|f| f.center)
            .filter(|_| xray < 1. && self.navigation.is_none())
        else {
            return 1.;
        };
        let (o, d) = self.camera.ray(a.lerp(b, 0.5), r);
        let at = o + d * (center - o).dot(d);
        let surface = self.pick_surface();
        if surface.visible(&self.camera, at, r) {
            1.
        } else {
            xray
        }
    }
    /// Mouse Regions Trigger Selections: a right-click on a point, edge or
    /// polygon switches to that element's mode and selects it. False (and
    /// nothing happens) when the preference is off or nothing is there.
    pub(super) fn mouse_region_click(&mut self, p: Pos2, r: Rect) -> bool {
        if !self.selection_preferences.mouse_regions {
            return false;
        }
        let Some((item, hit)) = self.pick_closest(p, r) else {
            return false;
        };
        let mode = match hit {
            Component::Vertex(_) => Mode::Vertex,
            Component::Edge(..) => Mode::Edge,
            Component::Polygon(_) => Mode::Polygon,
            _ => return false,
        };
        if self.editor.scene.selection.mode != mode {
            self.run(Command::Mode(mode));
        }
        self.click_select(Click {
            hit: Some((item, hit)),
            how: ClickHow::Replace,
            over_skipped_mesh: false,
        });
        true
    }
    /// What the pointer pre-highlights at `p`: the current mode's element,
    /// or with Closest rollovers in a component mode, the nearest of any
    /// kind.
    fn rollover_component_on(
        &self,
        p: Pos2,
        r: Rect,
        surface: &picking::Surface,
    ) -> Option<(u64, Component)> {
        if self.closest_rollovers() {
            self.pick_closest_on(p, r, surface)
        } else {
            self.pick_component_on(p, r, surface)
        }
    }
    pub(super) fn closest_rollovers(&self) -> bool {
        self.selection_preferences.rollovers == crate::selection_preferences::Rollovers::Closest
            && self.editor.scene.selection.mode != Mode::Item
    }
    /// Whether a picked element is part of the selection.
    pub(super) fn is_selected_component(&self, id: u64, c: Component) -> bool {
        let s = &self.editor.scene.selection;
        match c {
            Component::Vertex(v) => s.vertices_of(id).contains(&v),
            Component::Edge(a, b) => s.edges_of(id).iter().any(|&e| e == (a, b) || e == (b, a)),
            Component::Polygon(f) => s.faces_of(id).contains(&f),
            Component::Item => s.item == Some(id) || s.also.contains(&id),
        }
    }
    pub(super) fn pick_component(&self, p: Pos2, r: Rect) -> Option<(u64, Component)> {
        self.pick_component_on(p, r, &self.pick_surface())
    }
    fn pick_component_on(
        &self,
        p: Pos2,
        r: Rect,
        surface: &picking::Surface,
    ) -> Option<(u64, Component)> {
        let mode = self.editor.scene.selection.mode;
        if matches!(mode, Mode::Polygon | Mode::Item) {
            let hit = surface.pick_in(&self.camera, p, r, crate::picking::limit(&self.pick_only()));
            // CP-A01: Select No can't be picked as an item in 3D views
            // (still selectable in the Items list).
            if mode == Mode::Item
                && hit.is_some_and(|(id, _, _)| {
                    self.editor
                        .scene
                        .items
                        .iter()
                        .any(|i| i.id == id && !i.is_viewport_selectable())
                })
            {
                return None;
            }
            return hit.map(|(id, f, _)| {
                (
                    id,
                    if mode == Mode::Item {
                        Component::Item
                    } else {
                        Component::Polygon(f)
                    },
                )
            });
        }
        self.pick_among(p, r, surface, &self.pick_candidates(p, p, r))
    }
    /// `item`'s points where they show, in its own coordinates: on the
    /// smooth surface for points whose polygons are all smooth, else on
    /// the cage (`SubdivisionCache::shown_points`). Picking, rollovers and
    /// point dots all use these, so every tool meets a subdivided mesh's
    /// points where they are drawn.
    pub(crate) fn shown_points<'a>(
        &self,
        item: &'a operandi_core::Item,
    ) -> std::borrow::Cow<'a, [DVec3]> {
        match self.subdivision_cache.shown_points(item) {
            Some(points) => std::borrow::Cow::Owned(points),
            None => std::borrow::Cow::Borrowed(&item.mesh.vertices),
        }
    }
    /// The points (Vertex mode) or edges (Edge mode) whose screen position
    /// lies within the 12-point pick radius of the screen segment `from`–`to`,
    /// projected once. A paint drag picks at every pixel along the pointer's
    /// path; gathering candidates once per move instead of projecting the
    /// whole mesh per pixel keeps fast drags on large meshes smooth.
    pub(super) fn pick_candidates(&self, from: Pos2, to: Pos2, r: Rect) -> PickCandidates {
        self.pick_candidates_in(self.editor.scene.selection.mode, from, to, r)
    }
    /// `pick_candidates` for `mode`'s elements, whatever the current mode.
    fn pick_candidates_in(&self, mode: Mode, from: Pos2, to: Pos2, r: Rect) -> PickCandidates {
        let mut out = PickCandidates::default();
        if !matches!(mode, Mode::Vertex | Mode::Edge) {
            return out;
        }
        let radius = self.pick_radius();
        let reach = Rect::from_two_pos(from, to).expand(radius);
        let near_path = |q: Pos2| input::segment_distance(q, from, to) < radius;
        // The view basis once, not per point.
        let projector = self.camera.projector(r);
        let display_hidden = self.editor.scene.display_hidden();
        let only = self.pick_only();
        for item in self.editor.scene.items.iter().filter(|i| {
            i.visible
                && !display_hidden.contains(&i.id)
                && (only.is_empty() || only.contains(&i.id))
        }) {
            let Ok(matrix) = self.editor.scene.world_matrix(item.id) else {
                continue;
            };
            self.item_pick_candidates(item, matrix, mode, reach, &near_path, &projector, &mut out);
        }
        out
    }
    /// `item`'s candidates for `pick_candidates_in` (and Tweak): its
    /// points (Vertex mode) or edges (Edge mode) in `reach` near the path,
    /// where they show (on a smooth surface, not its cage).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn item_pick_candidates(
        &self,
        item: &operandi_core::Item,
        matrix: glam::DMat4,
        mode: Mode,
        reach: Rect,
        near_path: &dyn Fn(Pos2) -> bool,
        projector: &crate::camera::Projector,
        out: &mut PickCandidates,
    ) {
        // Points pick where they show: on the smooth surface.
        let shown = self.shown_points(item);
        let point = |v: u32| matrix.transform_point3(shown[v as usize]);
        if mode == Mode::Vertex {
            for v in 0..item.mesh.vertices.len() as u32 {
                let world = point(v);
                if let Some((q, _)) = projector.project(world)
                    && reach.contains(q)
                    && near_path(q)
                {
                    out.points.push((item.id, v, q, world));
                }
            }
        } else {
            // The edge list is cached per mesh; sorting it every frame
            // cost several ms on large meshes.
            let edges = self.mesh_edges(item);
            // Each point projected once, not once per edge end.
            let projected: Vec<Option<(Pos2, f64)>> = (0..item.mesh.vertices.len() as u32)
                .map(|v| projector.project(point(v)))
                .collect();
            // Smooth polygons' edges pick where they are drawn: on the
            // smooth surface (Jason's QA, 11-05), each traced piece a
            // candidate naming its cage edge.
            let mut on_surface = std::collections::BTreeSet::new();
            if let Some(surface) = self.subdivision_cache.smooth(item) {
                for ((a, b), (wa, wb)) in surface.all_edge_paths() {
                    on_surface.insert((a, b));
                    let (wa, wb) = (matrix.transform_point3(wa), matrix.transform_point3(wb));
                    if let (Some((qa, za)), Some((qb, zb))) =
                        (projector.project(wa), projector.project(wb))
                        && Rect::from_two_pos(qa, qb).intersects(reach)
                    {
                        out.edges
                            .push((item.id, a, b, [(qa, za), (qb, zb)], [wa, wb]));
                    }
                }
            }
            for &(a, b) in edges.iter() {
                if on_surface.contains(&(a.min(b), a.max(b))) {
                    continue;
                }
                if let (Some((qa, za)), Some((qb, zb))) =
                    (projected[a as usize], projected[b as usize])
                    && Rect::from_two_pos(qa, qb).intersects(reach)
                {
                    out.edges
                        .push((item.id, a, b, [(qa, za), (qb, zb)], [point(a), point(b)]));
                }
            }
        }
    }
    /// The component nearest `p` among `candidates` (see `pick_candidates`)
    /// that isn't hidden behind a surface.
    pub(super) fn pick_among(
        &self,
        p: Pos2,
        r: Rect,
        surface: &picking::Surface,
        candidates: &PickCandidates,
    ) -> Option<(u64, Component)> {
        let radius = self.pick_radius();
        // Point Selection Priority: a point counts this much nearer when
        // weighed against edges (it only matters when both are candidates).
        let priority = self.selection_preferences.point_priority as f32;
        let mut best: Option<(f32, u64, Component)> = None;
        let mut nearest_point = radius;
        // With picking limited to the active mesh, other meshes don't hide
        // its points and edges either, as they don't block its polygons.
        let only = self.pick_only();
        let only = crate::picking::limit(&only);
        for &(id, v, q, world) in &candidates.points {
            let d = q.distance(p);
            if d < nearest_point && surface.visible_in(&self.camera, world, r, only) {
                nearest_point = d;
                best = Some((d - priority, id, Component::Vertex(v)));
            }
        }
        for &(id, a, b, [(qa, za), (qb, zb)], [wa, wb]) in &candidates.edges {
            let d = qb - qa;
            let t = ((p - qa).dot(d) / d.length_sq().max(1e-8)).clamp(0., 1.);
            let distance = input::segment_distance(p, qa, qb);
            if distance < best.as_ref().map_or(radius, |b| b.0)
                && surface.visible_in(
                    &self.camera,
                    wa.lerp(wb, picking::span_world_parameter(&self.camera, t, za, zb)),
                    r,
                    only,
                )
            {
                best = Some((distance, id, Component::Edge(a, b)));
            }
        }
        best.map(|(_, id, c)| (id, c))
    }
    /// Polygon Selection Fill: Shift+Cmd click (Shift+Ctrl off macOS) on a
    /// polygon inside a selected perimeter (Jason decision 2026-11-01,
    /// SEL-Q-M3); on any other polygon, the row between it and the last pick
    /// (SEL-PHY-03). Returns the gesture when the press belongs to Fill; other
    /// presses, including plain Shift+click, fall through to ordinary picking.
    pub(super) fn fill_press(
        &mut self,
        pos: Pos2,
        r: Rect,
        mods: Modifiers,
    ) -> Option<input::GestureKind> {
        // `command` is Cmd on macOS and Ctrl elsewhere (synthetic events may
        // carry only `mac_cmd`).
        if self.editor.scene.selection.mode != Mode::Polygon
            || !mods.shift
            || !(mods.command || mods.mac_cmd)
            || mods.alt
        {
            return None;
        }
        match self.pick_component(pos, r) {
            Some((item, Component::Polygon(face))) => {
                // The clicked mesh's polygons (any active mesh).
                let before = self.editor.scene.selection.faces_of(item).to_vec();
                if !before.contains(&face) {
                    self.run(Command::SelectionFill { item, face });
                }
                let after = self.editor.scene.selection.faces_of(item).len();
                if after > before.len() + 1 {
                    self.status = format!("Selection Fill: {after} polygons selected");
                } else if !before.is_empty() && !before.contains(&face) {
                    // SEL-PHY-03: outside a selected perimeter (Fill found
                    // nothing but the polygon itself), the click picks the
                    // polygon and selects the row between it and the last
                    // pick.
                    if after == before.len() {
                        self.click_select(Click {
                            hit: Some((item, Component::Polygon(face))),
                            how: ClickHow::Add,
                            over_skipped_mesh: false,
                        });
                    }
                    self.execute("Select Between");
                    self.status = format!(
                        "Select Between: {} polygons selected",
                        self.editor.scene.selection.faces.len()
                    );
                }
            }
            _ => {
                self.status =
                    "Selection Fill: Shift+Cmd click a polygon inside the selected perimeter".into()
            }
        }
        Some(input::GestureKind::Blocked)
    }
    /// Polygon mode: the second press of a double-click on a polygon selects
    /// every polygon connected to it (Shift adds that piece to the
    /// selection). Like the edge-loop double-click, the first press was an
    /// ordinary click and its own undo step; the piece is one more "Select
    /// Connected" step. Any other press falls through to ordinary picking.
    pub(super) fn polygon_connected_double_click(
        &mut self,
        pos: Pos2,
        r: Rect,
        mods: Modifiers,
        now: f64,
        delay: f64,
    ) -> Option<input::GestureKind> {
        if self.editor.scene.selection.mode != Mode::Polygon {
            return None;
        }
        // Taken, not peeked: after a double-click, a third press starts over.
        let (first_time, first_pos) = self.last_component_press.take()?;
        if mods.ctrl
            || mods.alt
            || mods.command
            || mods.mac_cmd
            || now - first_time > delay
            || first_pos.distance(pos) > EDGE_DOUBLE_CLICK_DISTANCE
        {
            return None;
        }
        // Picked only on the second press, so an ordinary click on a big
        // mesh still costs one pick.
        let Some((item, Component::Polygon(face))) = self.pick_component(pos, r) else {
            return None;
        };
        self.run(Command::SelectConnectedPolygons {
            item,
            face,
            add: mods.shift,
        });
        let total = self.editor.scene.selection.faces_of(item).len();
        self.status = format!("Select Connected: {total} selected Polygons");
        Some(input::GestureKind::Blocked)
    }
    /// Edge mode: the second press of a double-click on an edge selects
    /// that edge's loop (Shift adds the loop to the selection). The first
    /// press was an ordinary click, so it already picked the edge as its own
    /// undo step; the loop is one more "Select Loop" step. Returns the
    /// gesture when the press is that second press; any other press falls
    /// through to ordinary picking (and the caller remembers it as a
    /// possible first press).
    pub(super) fn edge_loop_double_click(
        &mut self,
        pos: Pos2,
        r: Rect,
        mods: Modifiers,
        now: f64,
        delay: f64,
    ) -> Option<input::GestureKind> {
        // Taken, not peeked: after a double-click, a third press starts over.
        let (first_time, first_pos) = self.last_component_press.take()?;
        if self.editor.scene.selection.mode != Mode::Edge
            || mods.ctrl
            || mods.alt
            || mods.command
            || mods.mac_cmd
            || now - first_time > delay
            || first_pos.distance(pos) > EDGE_DOUBLE_CLICK_DISTANCE
        {
            return None;
        }
        // Picked only on the second press, so an ordinary click on a big
        // mesh still costs one pick.
        let Some((item, Component::Edge(a, b))) = self.pick_component(pos, r) else {
            return None;
        };
        let before = self.editor.scene.selection.edges_of(item).len();
        self.run(Command::SelectEdgeLoop {
            item,
            a,
            b,
            add: mods.shift,
        });
        let after = self.editor.scene.selection.edges_of(item).len();
        // With Shift the first press already added the clicked edge.
        let grew = if mods.shift {
            after > before
        } else {
            after > 1
        };
        self.status = if grew {
            format!("Select Loop: {after} selected Edge")
        } else {
            format!("Edge loop stops here; {after} selected Edge")
        };
        Some(input::GestureKind::Blocked)
    }
    /// Whether painting `hit` (adding it, or removing it with `remove`)
    /// would change the selection, counting its Symmetry counterpart.
    fn paint_changes(&self, item: u64, hit: Component, remove: bool) -> bool {
        let scene = &self.editor.scene;
        let s = &scene.selection;
        // A component of any active mesh paints into that mesh's selection;
        // Shift/Ctrl item paints add and remove items.
        if hit != Component::Item
            && s.item != Some(item)
            && !scene.shown_active_meshes().contains(&item)
        {
            return true;
        }
        let selected = |c: Component| match c {
            Component::Vertex(v) => s.vertices_of(item).contains(&v),
            Component::Edge(a, b) => s.edges_of(item).contains(&(a.min(b), a.max(b))),
            Component::Polygon(f) => s.faces_of(item).contains(&f),
            Component::Item => s.item == Some(item) || s.also.contains(&item),
        };
        let mirror = scene.symmetric_counterpart(item, hit, self.editor.active_symmetry());
        [Some(hit), mirror]
            .into_iter()
            .flatten()
            .any(|c| selected(c) == remove)
    }
    pub(super) fn select_at(&mut self, p: Pos2, r: Rect, mods: Modifiers) {
        let hit = self.pick_component(p, r);
        let how = ClickHow::from_keys(mods.shift, mods.ctrl);
        // Only a plain click on nothing needs to know what it missed.
        let over_skipped_mesh =
            hit.is_none() && how == ClickHow::Replace && self.on_other_mesh(p, r);
        if !self.click_select(Click {
            hit,
            how,
            over_skipped_mesh,
        }) {
            return;
        }
        if hit.is_some() {
            // SL-03: Shortest Path Mode adds the path to a Shift+clicked element.
            self.shortest_path_after_click(mods.shift, mods.ctrl);
        }
        self.selection_status();
    }
    /// Every viewport selection click ends here (AR-23): the hit becomes
    /// a selection change in `click_select::resolve`, so a later selection
    /// style changes one place. False when only the active mesh picks
    /// here (the status line says so).
    pub(super) fn click_select(&mut self, click: Click) -> bool {
        match click_select::resolve(&self.editor.scene, click) {
            click_select::Outcome::Run(command) => self.run(*command),
            click_select::Outcome::Nothing => {}
            click_select::Outcome::OnlyActiveMeshPicks => {
                self.status =
                    "Only the active mesh picks here (Preferences > Selection: Pick Other \
                     Meshes in Component Modes)"
                        .into();
                return false;
            }
        }
        true
    }
    /// Whether `p` is over a mesh that limited picking (`pick_only`) skips.
    fn on_other_mesh(&self, p: Pos2, r: Rect) -> bool {
        let only = self.pick_only();
        !only.is_empty()
            && self
                .pick_surface()
                .pick(&self.camera, p, r)
                .is_some_and(|(id, _, _)| !only.contains(&id))
    }
    /// "N selected <mode>" in the status line.
    fn selection_status(&mut self) {
        let s = &self.editor.scene.selection;
        // Components count over every active mesh.
        self.status = format!(
            "{} selected {:?}",
            match s.mode {
                Mode::Vertex | Mode::Edge | Mode::Polygon => s.component_count(),
                Mode::Item => usize::from(s.item.is_some()),
            },
            s.mode
        );
    }
}
impl Operandi {
    pub(super) fn finish_lasso(&mut self, path: &[Pos2], middle: bool, mods: Modifiers, r: Rect) {
        // Half a point of tolerance: invisible on screen, but a hand-drawn
        // loop drops from thousands of points to a few dozen.
        let region = lasso::simplify(&lasso::outline(self.lasso_style, path), 0.5);
        let result = self.lasso_hits(&region, middle, r);
        match result {
            Ok(meshes) => {
                if !mods.shift && !mods.ctrl {
                    self.run(Command::ClearSelection);
                }
                let how = if mods.ctrl {
                    ClickHow::Remove
                } else {
                    ClickHow::Add
                };
                for (item, hits) in meshes {
                    for hit in hits {
                        self.click_select(Click {
                            hit: Some((item, hit)),
                            how,
                            over_skipped_mesh: false,
                        });
                    }
                }
                self.run(Command::EndSelection { cancel: false });
                self.status =
                    "Lasso selected enclosed components not hidden behind a surface".into();
            }
            Err(error) => {
                self.run(Command::EndSelection { cancel: true });
                self.status = error;
            }
        }
    }
    pub(super) fn lasso_hits(
        &self,
        region: &[Pos2],
        middle: bool,
        r: Rect,
    ) -> operandi_core::Result<Vec<(u64, Vec<Component>)>> {
        if !lasso::valid(region) {
            return Err("Lasso requires a nonzero-area shape".into());
        }
        // Every shown active mesh (multi_active.rs), primary first.
        let shown = self.editor.scene.shown_active_meshes();
        let meshes: Vec<&operandi_core::Item> = shown
            .iter()
            .filter_map(|id| self.editor.scene.item(*id))
            .collect();
        if meshes.is_empty() {
            return Err("Lasso requires an active visible mesh".into());
        }
        // Curves and polylines lasso like polygons: their points are mesh
        // points, their spans are edges and each line is one element
        // (Jason's QA, 2026-11-10: a mesh holding a curve refused the
        // lasso outright, so nothing on it could be lassoed).
        let corners: usize = meshes
            .iter()
            .flat_map(|i| i.mesh.faces.iter())
            .map(|f| f.vertices.len())
            .sum();
        // Each corner is tested against each outline segment a few times;
        // corners outside the outline's bounds are rejected up front.
        if corners.saturating_mul(region.len()) > 50_000_000 {
            return Err("Lasso exceeds the 50-million intersection-work budget; shorten the path or reduce geometry".into());
        }
        meshes
            .into_iter()
            .map(|item| Ok((item.id, self.lasso_mesh(item, region, middle, r)?)))
            .collect()
    }
    /// One mesh's components inside the lasso `region` (see `lasso_hits`).
    fn lasso_mesh(
        &self,
        item: &operandi_core::Item,
        region: &[Pos2],
        middle: bool,
        r: Rect,
    ) -> operandi_core::Result<Vec<Component>> {
        let partial = self.lasso_partial || self.lasso_partial_held;
        // Shaded/right culls; shaded/middle through; Wireframe reverses it.
        // Polygon Selection and Lasso: Ignore Shading Style (preferences).
        let prefs = self.selection_preferences;
        let cull = prefs
            .polygon_selection
            .lasso_culls(self.wireframe_view() && !prefs.lasso_ignore_shading, middle);
        // Where the points show (on a smooth surface, not its cage).
        let world: Vec<_> = self
            .shown_points(item)
            .iter()
            .map(|&v| self.editor.scene.world_point(item.id, v))
            .collect();
        // Edges and polygons test hiding at their cage centres, as before:
        // a chord between two surface points lies inside a convex surface.
        let cage: Vec<_> = item
            .mesh
            .vertices
            .iter()
            .map(|&v| self.editor.scene.world_point(item.id, v))
            .collect();
        let projected: Vec<_> = world
            .iter()
            .map(|&v| self.camera.project(v, r).map(|p| p.0))
            .collect();
        // Padded past the 0.001 on-the-line tolerance of `lasso::contains`.
        let bounds = Rect::from_points(region).expand(0.01);
        let accepts = |vertices: &[u32]| {
            let points: Option<Vec<_>> = vertices.iter().map(|&v| projected[v as usize]).collect();
            points.is_some_and(|points| {
                // Cheap bounds check first: a full enclosure needs every
                // corner inside the outline's bounds, a partial one needs
                // the element's bounds to touch them.
                let element = Rect::from_points(&points);
                let near = if partial {
                    element.intersects(bounds)
                } else {
                    bounds.contains_rect(element)
                };
                near && lasso::eligible(region, &points, partial)
            })
        };
        // Components use the same test as clicking (Jason, 2026-10-31 for
        // points, 2026-11-01 for edges and polygons): eligible unless a
        // surface lies in front of them. Edges are tested at their midpoint
        // and polygons at their center. This replaces a facing-only rule
        // that skipped unobstructed parts whose polygons face away.
        let surface = self.pick_surface();
        let unhidden = |at: DVec3| !cull || surface.visible(&self.camera, at, r);
        let center = |vertices: &[u32]| {
            vertices.iter().map(|&v| cage[v as usize]).sum::<DVec3>() / vertices.len() as f64
        };
        let mut hits = Vec::new();
        match self.editor.scene.selection.mode {
            Mode::Polygon => {
                for (index, face) in item.mesh.faces.iter().enumerate() {
                    if accepts(&face.vertices) && unhidden(center(&face.vertices)) {
                        hits.push(Component::Polygon(index as u32));
                    }
                }
            }
            Mode::Vertex => {
                for vertex in 0..item.mesh.vertices.len() as u32 {
                    if accepts(&[vertex]) && unhidden(world[vertex as usize]) {
                        hits.push(Component::Vertex(vertex));
                    }
                }
            }
            Mode::Edge => {
                for (a, b) in item.mesh.edges() {
                    if accepts(&[a, b]) && unhidden(center(&[a, b])) {
                        hits.push(Component::Edge(a, b));
                    }
                }
            }
            Mode::Item => {
                return Err(
                    "Item lasso is not supported by the current single-item selection state".into(),
                );
            }
        }
        Ok(hits)
    }
}

impl Operandi {
    /// Falloff handles in world space: Linear Start (0) and End (1); Radial
    /// Center (0) and one Size handle per axis (1–3). Jason-specified
    /// on-screen editing (2026-11-02); the reference's pointer behaviour
    /// wasn't measured, so the layout is provisional.
    /// A right-drag on a Selection falloff: one Step per
    /// `FALLOFF_STEP_PIXELS` moved right from the press, fewer to the left,
    /// never below 1. Vertical movement is ignored.
    pub(super) fn falloff_steps_drag(&mut self, layer: usize, start: u32, from: Pos2, to: Pos2) {
        let moved = ((to.x - from.x) / FALLOFF_STEP_PIXELS).trunc() as i64;
        self.set_falloff_steps(layer, start as i64 + moved);
    }
    /// FO-46 / FO-Q04 default: a right-click that didn't drag adds one Step.
    pub(super) fn falloff_steps_release(&mut self, layer: usize, start: u32, from: Pos2, to: Pos2) {
        if from.distance(to) < FALLOFF_CLICK_PIXELS {
            self.set_falloff_steps(layer, start as i64 + 1);
        }
    }
    /// FO-Q05: start a Soft Selection Radius drag, measured at the
    /// selection's centre (or the view's target).
    fn begin_falloff_radius(
        &mut self,
        layer: usize,
        start: f64,
        _pos: Pos2,
        r: Rect,
    ) -> Option<input::GestureKind> {
        let center = self.transform_handle_center();
        let metres_per_pixel = self.camera.metres_per_pixel(center, r)?;
        self.status = format!(
            "Soft Selection Radius: {} · drag right for more, left for less",
            crate::form::Unit::Distance.format(start)
        );
        Some(input::GestureKind::FalloffRadius {
            layer,
            start,
            metres_per_pixel,
        })
    }
    pub(super) fn falloff_radius_drag(
        &mut self,
        layer: usize,
        start: f64,
        metres_per_pixel: f64,
        from: Pos2,
        to: Pos2,
    ) {
        let radius = (start + (to.x - from.x) as f64 * metres_per_pixel).max(0.);
        falloff_panel::set_soft_selection_radius(self, layer, radius);
        self.status = format!(
            "Soft Selection Radius: {} · drag right for more, left for less",
            crate::form::Unit::Distance.format(radius)
        );
    }
    fn set_falloff_steps(&mut self, layer: usize, steps: i64) {
        let Some((at, f)) = falloff_panel::selection_layer(&self.editor) else {
            return;
        };
        let steps = steps.clamp(1, operandi_core::falloff::MAX_SELECTION_STEPS as i64) as u32;
        if at != layer || steps == f.steps {
            return;
        }
        falloff_panel::set_selection_steps(self, layer, steps);
        self.status =
            format!("Selection falloff Steps: {steps} · drag right for more, left for fewer");
    }
    /// What the views' HT-5/HT-6 tints need baked into the geometry: the
    /// selected weight map when some 3D view shows weights, and whether
    /// some view shows UV distortion.
    pub(crate) fn view_tints(&self) -> crate::gpu_scene::Tints {
        let drawings = || {
            std::iter::once(&self.display.settings.drawing)
                .chain(self.displays.values().map(|d| &d.settings.drawing))
        };
        crate::gpu_scene::Tints {
            weights: drawings()
                .any(|d| d.weights)
                .then(|| self.editor.selected_weight_map.clone())
                .flatten(),
            distortion: drawings().any(|d| d.distortion),
        }
    }
    /// The falloff on screen: its handles, the Linear line or Radial
    /// outline, and each point of the active mesh tinted by its weight.
    fn draw_falloff(&self, painter: &egui::Painter, r: Rect) {
        // CS-01: nothing is drawn for an armed tool.
        if self.editor.is_armed() {
            return;
        }
        let (Some(field), Some(stack)) =
            (self.editor.spatial_falloff, self.editor.active_falloff())
        else {
            return;
        };
        let project = |p: DVec3| self.camera.project(p, r).map(|(q, _)| q);
        // FO-06: View ▸ Show Falloffs turns the point tints off.
        if self.show_falloffs
            && let Some(item) = self.editor.scene.active()
            && item.mesh.vertices.len() <= 20_000
        {
            // The whole stack, Vertex Map included; tints show 0…1.
            // Point weights come from the tool's starting mesh while it
            // edits, as the tool reads them (worked out once per drag).
            let base = self
                .editor
                .live
                .as_ref()
                .filter(|l| l.base.item(item.id).is_some())
                .map_or(&self.editor.scene, |l| &l.base);
            let prepared = stack.prepare(base, item.id).unwrap_or_default();
            let inputs = prepared.inputs();
            // FO-47/48 Show Vertex and Show Edge on the first layer.
            let (show_vertex, show_edge) = match field {
                Falloff::SoftSelection(f) => (f.show_vertex, f.show_edge),
                Falloff::Coplanar(f) => (f.show_vertex, f.show_edge),
                _ => (true, false),
            };
            let weights: Vec<f32> = item
                .mesh
                .vertices
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let p = self.editor.scene.world_point(item.id, v);
                    (stack.weight_at_local(p, v, i as u32, inputs) as f32).clamp(0., 1.)
                })
                .collect();
            if show_edge {
                for (a, b) in item.mesh.edges() {
                    let w = weights[a as usize].min(weights[b as usize]);
                    if w > 0.
                        && let (Some(qa), Some(qb)) = (
                            project(
                                self.editor
                                    .scene
                                    .world_point(item.id, item.mesh.vertices[a as usize]),
                            ),
                            project(
                                self.editor
                                    .scene
                                    .world_point(item.id, item.mesh.vertices[b as usize]),
                            ),
                        )
                    {
                        painter.line_segment(
                            [qa, qb],
                            Stroke::new(
                                1.5_f32,
                                palette::falloff_weight(w).gamma_multiply(0.25 + 0.75 * w),
                            ),
                        );
                    }
                }
            }
            for (i, &v) in item.mesh.vertices.iter().enumerate() {
                if !show_vertex {
                    break;
                }
                let p = self.editor.scene.world_point(item.id, v);
                let w = weights[i];
                if w > 0.
                    && let Some(q) = project(p)
                {
                    painter.circle_filled(
                        q,
                        2. + 2. * w,
                        palette::falloff_weight(w).gamma_multiply(0.25 + 0.75 * w),
                    );
                }
            }
        }
        self.draw_falloff_shape(painter, r, field);
    }
    /// Edge Slide's handle frame (`HandleAnchor::EdgeRail`), once per drag;
    /// see `operandi_core::edge_slide_handle_frame`.
    fn edge_slide_frame(&self) -> Option<(DVec3, DVec3, f64)> {
        let l = self.editor.live.as_ref()?;
        let symmetry = self.editor.active_symmetry();
        self.edge_slide_frame_memo.get_with(
            &l.base,
            (symmetry, operandi_core::HandlePick::FirstPicked),
            || operandi_core::edge_slide_handle_frame(&l.base, symmetry),
        )
    }
    /// The vertex tools' handle frame (`HandleAnchor::Vertex`), once per
    /// drag; see `operandi_core::vertex_handle_frame`.
    fn vertex_frame(&self) -> Option<(DVec3, DVec3, DVec3, f64)> {
        let l = self.editor.live.as_ref()?;
        let symmetry = self.editor.active_symmetry();
        let pick = operandi_core::handle_pick(l.tool);
        self.vertex_frame_memo
            .get_with(&l.base, (symmetry, pick), || {
                operandi_core::vertex_handle_frame(&l.base, symmetry, pick)
            })
    }
    /// The edge tools' handle frame (`HandleAnchor::Edge`), once per drag;
    /// see `operandi_core::edge_handle_frame`.
    fn edge_handle_frame(&self) -> Option<(DVec3, DVec3, DVec3)> {
        let l = self.editor.live.as_ref()?;
        let symmetry = self.editor.active_symmetry();
        let pick = operandi_core::handle_pick(l.tool);
        self.edge_frame_memo
            .get_with(&l.base, (symmetry, pick), || {
                operandi_core::edge_handle_frame(&l.base, symmetry, pick)
            })
    }
}

/// A polygon's screen-depth gradient, so widened edge strokes stay on top of
/// their own surface at every pixel. Taken from the polygon's best-shaped fan
/// triangle: a sliver (nearly collinear corners, common in imported n-gons
/// and thin polygons) gives a huge, wrong gradient that pushed strokes in
/// front of other surfaces as stray segments. Below one square pixel there
/// is no usable plane; use none.
fn depth_gradient(camera: &crate::Camera, r: Rect, points: &[(Pos2, f64)]) -> Vec2 {
    let (mut best, mut j, mut k) = (0f32, 1, 2);
    for i in 1..points.len().saturating_sub(1) {
        let d1 = points[i].0 - points[0].0;
        let d2 = points[i + 1].0 - points[0].0;
        let det = (d1.x * d2.y - d1.y * d2.x).abs();
        if det > best {
            (best, j, k) = (det, i, i + 1);
        }
    }
    if best < 1.0 {
        return Vec2::ZERO;
    }
    let depth = |p: &(Pos2, f64)| outline::vertex(camera, r, *p, Color32::WHITE).position[2];
    let d1 = points[j].0 - points[0].0;
    let d2 = points[k].0 - points[0].0;
    let determinant = d1.x * d2.y - d1.y * d2.x;
    let (z0, z1, z2) = (depth(&points[0]), depth(&points[j]), depth(&points[k]));
    let (z1, z2) = (z1 - z0, z2 - z0);
    vec2(
        (z1 * d2.y - z2 * d1.y) / determinant,
        (d1.x * z2 - d2.x * z1) / determinant,
    )
}

/// Points and edges near a pointer path, already projected (see
/// `Operandi::pick_candidates`), in item and index order.
#[derive(Default)]
pub(crate) struct PickCandidates {
    /// (item, point, screen position, world position).
    points: Vec<(u64, u32, Pos2, DVec3)>,
    /// (item, ends, screen position and depth of each end, world ends).
    #[allow(clippy::type_complexity)]
    edges: Vec<(u64, u32, u32, [(Pos2, f64); 2], [DVec3; 2])>,
}

impl PickCandidates {
    /// Each edge candidate's ends, screen ends and world ends.
    #[allow(clippy::type_complexity)]
    pub(crate) fn edge_spans(
        &self,
    ) -> impl Iterator<Item = (u32, u32, [(Pos2, f64); 2], [DVec3; 2])> + '_ {
        self.edges.iter().map(|&(_, a, b, q, w)| (a, b, q, w))
    }
}

/// A value derived only from a live tool's base scene (the scene before the
/// drag), such as a handle frame. The base stays the same for the whole drag,
/// so the value is computed once instead of on every frame. Keyed on the
/// active item's mesh allocation, its transform and the selection, which is
/// everything these frames read.
pub(crate) struct BaseMemo<T>(std::cell::RefCell<Option<(BaseKey, T)>>);

/// A [`BaseMemo`] whose value also reads the Symmetry setting and the
/// tool's handle rule (ACS-Q-HANDLE1).
pub(crate) type SymmetryMemo<T> = BaseMemo<(HandleMemoKey, T)>;
pub(crate) type HandleMemoKey = (operandi_core::Symmetry, operandi_core::HandlePick);

struct BaseKey {
    mesh: Option<operandi_core::Shared<operandi_core::Mesh>>,
    world: Option<glam::DMat4>,
    selection: operandi_core::Selection,
}

impl<T> Default for BaseMemo<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T: Clone> BaseMemo<(HandleMemoKey, T)> {
    /// `get` for a value that also reads the Symmetry setting (handle
    /// frames pick the primary side, ACS-01): Symmetry switched on or off
    /// with the tool open recomputes it.
    pub(crate) fn get_with(
        &self,
        base: &Scene,
        symmetry: HandleMemoKey,
        compute: impl FnOnce() -> T,
    ) -> T {
        let stale = matches!(&*self.0.borrow(), Some((_, (at, _))) if *at != symmetry);
        if stale {
            *self.0.borrow_mut() = None;
        }
        self.get(base, || (symmetry, compute())).1
    }
}

impl Operandi {
    /// AC-75: the Local handle's frame (on the last selected element's
    /// cluster) for the live Move, Rotate, Scale or Transform tool: the
    /// pivot service's answer, once per drag base.
    pub(super) fn local_handle_frame(&self) -> Option<operandi_core::ActionFrame> {
        let l = self.editor.live.as_ref()?;
        let context = self.editor.action_context;
        if context.center != CenterMode::Local
            || !matches!(
                l.tool,
                Tool::Move | Tool::Rotate | Tool::Scale | Tool::Transform
            )
        {
            return None;
        }
        let memo = &self.local_handle_memo;
        let stale = matches!(&*memo.0.borrow(), Some((_, (at, _))) if *at != context);
        if stale {
            *memo.0.borrow_mut() = None;
        }
        memo.get(&l.base, || {
            (
                context,
                self.editor
                    .pivot_request(&l.base, l.tool)
                    .local_handle_frame(),
            )
        })
        .1
    }

    pub(super) fn local_handle_basis(&self) -> Option<glam::DMat3> {
        self.local_handle_frame().map(|f| f.basis)
    }
}

impl<T: Clone> BaseMemo<T> {
    pub(crate) fn get(&self, base: &Scene, compute: impl FnOnce() -> T) -> T {
        let item = base.active();
        let key = BaseKey {
            mesh: item.map(|i| i.mesh.shared().clone()),
            world: item.and_then(|i| base.world_matrix(i.id).ok()),
            selection: base.selection.clone(),
        };
        if let Some((k, v)) = &*self.0.borrow()
            // The same mesh allocation (a cheap check) means the same mesh.
            && match (&k.mesh, &key.mesh) {
                (Some(a), Some(b)) => operandi_core::Shared::same(a, b),
                (a, b) => a.is_none() && b.is_none(),
            }
            && k.world == key.world
            && k.selection == key.selection
        {
            return v.clone();
        }
        let value = compute();
        *self.0.borrow_mut() = Some((key, value.clone()));
        value
    }
}

#[cfg(test)]
mod falloff_wedge_tests {
    use super::{FALLOFF_WEDGE_MIN_HALF_WIDTH, linear_falloff_wedge};
    use crate::egui::pos2;
    #[test]
    fn wedge_starts_at_a_point_and_opens_a_quarter_length_across_the_end() {
        let (a, b) = (pos2(10., 20.), pos2(110., 20.));
        let [tip, left, right] = linear_falloff_wedge(a, b);
        assert_eq!(tip, a);
        // The open end crosses the End handle at right angles.
        assert_eq!((left + (right - left) / 2.), b);
        assert!((left - right).dot(b - a).abs() < 1e-4);
        assert!(((left - right).length() - 25.).abs() < 1e-4);
    }
    #[test]
    fn wedge_stays_visible_when_short_or_seen_end_on() {
        for b in [pos2(13., 20.), pos2(10., 20.)] {
            let [_, left, right] = linear_falloff_wedge(pos2(10., 20.), b);
            let width = (left - right).length();
            assert!((width - 2. * FALLOFF_WEDGE_MIN_HALF_WIDTH).abs() < 1e-4);
            assert!(left.is_finite() && right.is_finite());
        }
    }
}

#[cfg(test)]
mod pen_review_tests {
    use super::selected_vertex_mask;
    #[test]
    fn pen_rev03_point_membership_is_linear_and_preserves_mixed_selection() {
        for n in [1_000, 10_000, 100_000] {
            let selected: Vec<u32> = (0..n as u32).step_by(2).rev().collect();
            let mask = selected_vertex_mask(n, &selected);
            assert_eq!(mask.len(), n);
            assert!(
                mask.iter()
                    .enumerate()
                    .all(|(i, &value)| value == (i % 2 == 0))
            );
            assert_eq!(selected[0], n as u32 - 2);
        }
    }
}

/// The 12 edges of `mesh`'s bounding box through `matrix`, as wire strokes
/// (Bounding Box Threshold), in `color` (the item's plain wire: boxes
/// draw in the shared overlay, so the inactive colour is baked here).
/// Only heavy meshes get the dense-mesh box corners (Operandi's choice,
/// provisional): a crowd of small objects, each a few points across, would
/// otherwise all be boxed.
const DENSE_BOX_POLYGONS: usize = 10_000;

/// Whether a mesh's polygons draw smaller on screen than Fade Wireframe on
/// Small Polygons keeps full wire for: its box's screen area shared out
/// among its polygons, as squares. Not when the box reaches behind the
/// eye.
fn too_dense_for_wire(
    projector: &crate::camera::Projector,
    lo: DVec3,
    hi: DVec3,
    matrix: glam::DMat4,
    polygons: usize,
) -> bool {
    if polygons == 0 {
        return false;
    }
    let mut screen = Rect::NOTHING;
    for i in 0..8 {
        let corner = DVec3::new(
            if i & 1 == 0 { lo.x } else { hi.x },
            if i & 2 == 0 { lo.y } else { hi.y },
            if i & 4 == 0 { lo.z } else { hi.z },
        );
        let Some((p, _)) = projector.project(matrix.transform_point3(corner)) else {
            return false;
        };
        screen.extend_with(p);
    }
    let side = (f64::from(screen.area()) / polygons as f64).sqrt();
    side < f64::from(crate::gpu::WIRE_FADE_FULL)
}

/// The corners of a box (`lo`..`hi` placed by `matrix`): a fifth of each
/// edge from each end, in `color`.
fn box_corner_strokes(
    lo: DVec3,
    hi: DVec3,
    matrix: glam::DMat4,
    color: Color32,
    out: &mut Vec<crate::gpu_scene::EdgeVertex>,
) {
    let corner = |i: usize| {
        matrix.transform_point3(DVec3::new(
            if i & 1 == 0 { lo.x } else { hi.x },
            if i & 2 == 0 { lo.y } else { hi.y },
            if i & 4 == 0 { lo.z } else { hi.z },
        ))
    };
    for i in 0..8 {
        for bit in [1, 2, 4] {
            // Each box edge once, from its lower-numbered corner.
            if i & bit != 0 {
                continue;
            }
            let (a, b) = (corner(i), corner(i | bit));
            // Width 1 point (not the plain wire's 0), so the overlay keeps
            // the colour instead of taking the view's wire colour.
            crate::gpu_scene::stroke_width(out, a, a.lerp(b, 0.2), 1., color);
            crate::gpu_scene::stroke_width(out, b, b.lerp(a, 0.2), 1., color);
        }
    }
}

fn box_strokes(
    mesh: &operandi_core::Mesh,
    matrix: glam::DMat4,
    color: Color32,
    out: &mut Vec<crate::gpu_scene::EdgeVertex>,
) {
    let Some(first) = mesh.vertices.first() else {
        return;
    };
    let (lo, hi) = mesh
        .vertices
        .iter()
        .fold((*first, *first), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    let corner = |i: usize| {
        matrix.transform_point3(DVec3::new(
            if i & 1 == 0 { lo.x } else { hi.x },
            if i & 2 == 0 { lo.y } else { hi.y },
            if i & 4 == 0 { lo.z } else { hi.z },
        ))
    };
    for (a, b) in [
        (0, 1),
        (2, 3),
        (4, 5),
        (6, 7),
        (0, 2),
        (1, 3),
        (4, 6),
        (5, 7),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ] {
        crate::gpu_scene::stroke(out, corner(a), corner(b), false, color);
    }
}

/// VMT-30's colours at 0, +1 and −1 (gamma-encoded 0–255); the 3D
/// view's weight tint uses the same (gpu_scene.rs `tint_wgsl`).
pub(crate) const WEIGHT_ZERO: [f64; 3] = [128., 140., 128.];
pub(crate) const WEIGHT_POSITIVE: [f64; 3] = [255., 0., 0.];
pub(crate) const WEIGHT_NEGATIVE: [f64; 3] = [0., 0., 255.];

/// VMT-30 weight shading: grey-green at 0, blending to red at +1 and blue
/// at −1, clamped beyond ±1 (measured, VM-CASE-16).
#[cfg(test)]
pub(crate) fn weight_colour(w: f64) -> Color32 {
    let zero = WEIGHT_ZERO;
    let end = if w >= 0. {
        WEIGHT_POSITIVE
    } else {
        WEIGHT_NEGATIVE
    };
    let t = if w.is_finite() { w.abs().min(1.) } else { 0. };
    let c = |i: usize| (zero[i] + (end[i] - zero[i]) * t) as u8;
    Color32::from_rgb(c(0), c(1), c(2))
}

#[cfg(test)]
mod dense_mesh_box_tests {
    use super::*;

    /// A box's 8 corners, each with its 3 edges' first fifths: 24 strokes
    /// of 6 corners, in the given colour, none longer than a fifth.
    #[test]
    fn box_corners_are_a_fifth_of_each_edge() {
        let mut out = Vec::new();
        let colour = Color32::from_rgb(1, 2, 3);
        box_corner_strokes(
            DVec3::ZERO,
            DVec3::splat(5.),
            glam::DMat4::IDENTITY,
            colour,
            &mut out,
        );
        assert_eq!(out.len(), 24 * 6);
        assert!(out.iter().all(|v| v.color == colour && v.width == 1.));
        for v in &out {
            let len = (glam::Vec3::from(v.a) - glam::Vec3::from(v.b)).length();
            assert!((len - 1.).abs() < 1e-5, "{len}");
        }
    }

    /// Dense is about polygons per screen area, not polygon count alone.
    #[test]
    fn density_is_screen_area_per_polygon() {
        let camera = crate::camera::Camera::default();
        let r = Rect::from_min_size(Pos2::ZERO, vec2(800., 600.));
        let p = camera.projector(r);
        let (lo, hi) = (DVec3::splat(-1.), DVec3::splat(1.));
        assert!(!too_dense_for_wire(&p, lo, hi, glam::DMat4::IDENTITY, 100));
        assert!(too_dense_for_wire(
            &p,
            lo,
            hi,
            glam::DMat4::IDENTITY,
            10_000_000
        ));
        assert!(!too_dense_for_wire(&p, lo, hi, glam::DMat4::IDENTITY, 0));
    }
}

/// Paint strokes skip moves like sculpt strokes (the dabs follow the
/// straight line from the last one).
#[cfg(feature = "painting")]
fn painting_fast(kind: &input::GestureKind) -> bool {
    matches!(
        kind,
        input::GestureKind::Paint | input::GestureKind::PaintSize { .. }
    )
}
#[cfg(not(feature = "painting"))]
fn painting_fast(_: &input::GestureKind) -> bool {
    false
}
