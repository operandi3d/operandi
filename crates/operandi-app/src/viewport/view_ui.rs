//! Drawing one 3D view: the scene, wireframes and overlays, the live tool's
//! handles and falloff, rollovers, and the view's own menu and messages.
use super::*;

impl Operandi {
    /// One view's toolbar, drawing and input. The view being drawn has its
    /// camera in `self.camera` (views.rs swaps the others in). A passive
    /// view (not the active one) draws but takes no input.
    pub(crate) fn view_ui(&mut self, ui: &mut Ui) {
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
            let mut grids = Vec::new();
            // Show Grid / Show Work Plane (Viewport Properties > Visibility):
            // the work plane counts as moved once it is fixed or centred
            // off the origin. The work plane draws only while its own
            // display is on (Show Work Plane, off by default); it is never
            // folded into the ground grid.
            let drawing = self.display.settings.drawing;
            let work_plane_moved = {
                let w = &self.editor.scene.workplane;
                w.fixed || w.center != DVec3::ZERO
            };
            let work_plane_grid = self.viewport_preferences.workplane_display
                && drawing.work_plane_grid(work_plane_moved);
            let floor_grid = drawing.floor_grid(work_plane_moved, work_plane_grid);
            let grid_step = grid::spacing(
                &self.camera,
                r,
                if work_plane_grid { plane } else { 0 },
                if work_plane_grid {
                    plane_center
                } else {
                    DVec3::ZERO
                },
            );
            let grid_label = if work_plane_grid {
                format!("Work plane: {grid_step} m minor")
            } else if floor_grid {
                format!("Ground XZ: {grid_step} m minor")
            } else {
                "Grid hidden".into()
            };
            let fade = drawing.faded_grid;
            // Separate layers preserve ground visibility even when the
            // coincident workplane grid is made fully transparent.
            if floor_grid {
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
            let zero_plane = if floor_grid {
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
            // in for the scene; otherwise the Mesh Stack (MO-01): items with a
            // stack show its result, the mesh in Edit Base Mesh its Base
            // Mesh (MO-27). Frame All reads the same (`shown_display`).
            let preview_snapshot = self.editor.tool_preview_display();
            let mesh_stack_snapshot = preview_snapshot
                .is_none()
                .then(|| self.editor.evaluated_display())
                .flatten();
            let display_scene = preview_snapshot
                .as_ref()
                .or(mesh_stack_snapshot.as_ref())
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
                .session
                .live
                .as_ref()
                .filter(|l| l.tool.is_primitive())
                .and_then(|l| l.base.active().map(|b| b.id));
            // CP-D01: items whose Display Visible is No (or under No (Hide
            // Children)) leave the view; empty in the common case.
            let display_hidden = display_scene.display_hidden();
            let shown = |i: &operandi_core::Item| {
                i.visible && !display_hidden.contains(&i.id) && !self.show_meshes_hides(i)
            };
            // Which items draw with the active look, looked up once for the
            // frame: asking the selection per item cost the square of the
            // item count with thousands selected (panel_speed_tests.rs).
            let active_now = self.editor.scene.active_item_set();
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
                let Some(look) = self.item_look_as(item.id, active_now.contains(&item.id)) else {
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
                    let flat_preview = self.editor.session.live.as_ref().is_some_and(|live| {
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
                        // HID-03: picking's corner list only includes shown polygons.
                        let Some(tessellation) = surface_cell
                            .get_or_init(|| surface_cache.get(display_scene, subdivision_cache))
                            .visible_corners(item.id, fi)
                        else {
                            continue;
                        };
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
                    if self.drop_drag.is_some() {
                        return surface().pick_in(&self.camera, p, r, crate::picking::limit(&only));
                    }
                    // A rollover shows only what a click would pick (Astra
                    // P6-005; Jason 2026-11-22): the click's own surface,
                    // and only where the drawn mesh is that same mesh, so
                    // faces that exist only on screen (Replicator copies,
                    // Mesh Stack output) don't pre-highlight.
                    let pick = self.pick_surface();
                    let pick_scene = self
                        .editor
                        .evaluated(operandi_core::evaluate::Purpose::Pick);
                    let hit = pick
                        .pick_in(&self.camera, p, r, crate::picking::limit(&only))
                        .filter(|(id, _, _)| {
                            match (display_scene.item(*id), pick_scene.item(*id)) {
                                (Some(d), Some(k)) => {
                                    operandi_core::MeshSlot::same(&d.mesh, &k.mesh)
                                }
                                _ => false,
                            }
                        });
                    // Closest rollovers: the polygon pre-highlights only
                    // when no point or edge is nearer.
                    if self.closest_rollovers() {
                        return hit.filter(|_| {
                            matches!(
                                self.pick_closest_on(p, r, &pick),
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
            // Item mode pre-highlights the pointer's whole item, not the
            // polygon under it; a selected item keeps showing selected.
            let whole_rollover = hover
                .and_then(|(i, _, _)| self.whole_item_rollover(i))
                .filter(|i| !item_picked.contains(i));
            let polygon_rollovers =
                polygon_rollovers && self.editor.scene.selection.mode != Mode::Item;
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
                    && self.editor.session.live.is_none()
                    && polygon_rollovers)
                    || whole_rollover == Some(*id)
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
                let color = if self.editor.session.live.as_ref().is_some_and(|l| {
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
                    self.plain_wire_as(active_now.contains(id))
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
                    let Some(look) = self.item_look_as(item.id, active_now.contains(&item.id))
                    else {
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
                    let visibility = (!item.elements.hidden.is_empty()).then(|| {
                        surface_cell
                            .get_or_init(|| surface_cache.get(display_scene, subdivision_cache))
                    });
                    for (sf, face) in mesh.faces.iter().enumerate() {
                        let source = sources[sf];
                        if !operandi_core::subdivision::is_smooth_polygon(&item.mesh, source)
                            || visibility.is_some_and(|surface| {
                                surface.visible_corners(item.id, source).is_none()
                            })
                        {
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
                        let pre = (hover
                            .is_some_and(|(i, f, _)| i == item.id && f == source as u32)
                            && self.editor.session.live.is_none()
                            && polygon_rollovers)
                            || whole_rollover == Some(item.id);
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
                            self.plain_wire_as(active_now.contains(&item.id))
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
            for item in display_scene.items.iter().filter(|i| {
                shown(i)
                    && self
                        .item_look_as(i.id, active_now.contains(&i.id))
                        .is_some()
            }) {
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
                            .unwrap_or_else(|| self.plain_wire_as(active_now.contains(&item.id)))
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
            if let Some(live) = &self.editor.session.live
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
                && let Some(result) = self.editor.mesh_stack.result(&self.editor.scene, item)
            {
                const GHOST_EDGE_BUDGET: usize = 200_000;
                let ghost = u64::MAX - base;
                let color = crate::mesh_stack_ui::ghost_color();
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
            #[cfg(test)]
            if self.drawing_view == self.active_view && PROJECTED_VERTEX_COUNTS.get().is_some() {
                PROJECTED_VERTEX_COUNTS.set(Some([triangles.len(), edges.len()]));
            }
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
            let box_wire: std::collections::BTreeMap<u64, Color32> = boxed
                .iter()
                .map(|&id| (id, self.plain_wire_as(active_now.contains(&id))))
                .collect();
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
                    whole_rollover: Default::default(),
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
                    && self.editor.session.live.is_some();
                self.resident_cache.sync_materials(display_scene, all_flat);
                // While a tool or drag is live, an item whose placement alone
                // changed reuses its cached geometry through a matrix on the
                // graphics card (gpu_scene.rs `delta`); it is rebuilt exactly
                // once the tool is dropped.
                // Playback and scrubbing move items every frame (an imported
                // shot moves hundreds of meshes under one locator): those
                // reuse their geometry through a matrix too, and settle at
                // their places a few at a time once time stops
                // (`Resident::begin_frame`).
                let live = self.editor.session.live.is_some()
                    || self.navigation.is_some()
                    || self.editor.anim.playing.is_some()
                    || self.anim_ui.time_moved;
                self.resident_cache.begin_frame(ctx.cumulative_pass_nr());
                let placed = |item: &'_ operandi_core::Item, matrix: glam::DMat4| match &whole {
                    Some((base, map)) if base.id == item.id => (base.clone(), matrix * *map),
                    _ => (item.clone(), matrix),
                };
                // Every selected mesh draws as active (multi_active.rs).
                // A set: with thousands of items selected, a list search per
                // item made this frame cost grow with the square of the count.
                let active_ids: std::collections::HashSet<u64> = self
                    .editor
                    .scene
                    .shown_active_meshes()
                    .into_iter()
                    .collect();
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
                    // VP-04/05: every selected mesh item is active, and
                    // the last selected one while nothing is.
                    let group = if selection.has_item(item.id) || remembered == Some(item.id) {
                        crate::gpu::LookGroup::Active
                    } else if self.display.group(false).is_none() {
                        crate::gpu::LookGroup::Hidden
                    } else {
                        crate::gpu::LookGroup::Inactive
                    };
                    // Subdivision polygons draw as patches on the graphics
                    // card when they can (subd_gpu.rs); the CPU surface is
                    // then only for the rollover, picking and overlays.
                    let patches = self
                        .viewport_gpu
                        .as_ref()
                        .filter(|_| crate::subd_gpu::on() && !draw.needs_stored_fill(group))
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
                    if whole_rollover == Some(item.id) {
                        draw.whole_rollover.insert(item.id);
                    }
                    if let Some((id, f, _)) = hover
                        && id == item.id
                        && self.editor.session.live.is_none()
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
                                // No dark backing under selected edges
                                // (Jason 2026-11-22: it read as a wireframe
                                // outline around the selection). The 2 pt
                                // line (2 device px standard, 4 on Retina)
                                // fully covers the 1-device-pixel plain wire
                                // (`plain_wire_width`).
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
                        self.editor.history.history_serial,
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
                if self.resident_cache.deferred() {
                    ctx.request_repaint();
                }
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
            let hovered_component = if self.editor.session.live.is_none() && self.rollovers_on() {
                response
                    .hover_pos()
                    .and_then(|p| self.rollover_component_on(p, r, &surface()))
            } else {
                None
            };
            // Every shown active mesh draws its points and edges
            // (multi_active.rs).
            // A set, for the same reason as `active_ids` above.
            let mut shown_active: std::collections::HashSet<u64> = self
                .editor
                .scene
                .shown_active_meshes()
                .into_iter()
                .collect();
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
                                    segment(va.lerp(vb, t), va.lerp(vb, u), color, 2.);
                                }
                            }
                        } else if shown(*edge) {
                            for &(va, vb) in segments {
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
                    .session
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
                    .session
                    .live
                    .as_ref()
                    .is_some_and(|l| l.tool == Tool::Tweak)
                && self.element_falloff().is_some()
                && self
                    .editor
                    .session
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
                        self.editor
                            .session
                            .live
                            .as_ref()
                            .map(|l| l.tool.label())
                            .unwrap_or(if self.editor.session.reduction.is_some() {
                                "Polygon Reduction · numeric"
                            } else if self.editor.session.bridge.is_some() {
                                "Bridge · numeric"
                            } else if self.editor.session.spike.is_some() {
                                "Spike · numeric"
                            } else if let Some(s) = &self.editor.session.shape {
                                match s.kind() {
                                    ShapeKind::Torus => "Torus · numeric",
                                    ShapeKind::Capsule => "Capsule · numeric",
                                    ShapeKind::Arc => "Arc · numeric",
                                    ShapeKind::Tube => "Tube · numeric",
                                    ShapeKind::Text => "Text · numeric",
                                    ShapeKind::Teapot => "Teapot · numeric",
                                }
                            } else if self.editor.session.ellipsoid.is_some() {
                                "Ellipsoid · numeric"
                            } else if self.editor.session.cone.is_some() {
                                "Cone · numeric"
                            } else {
                                "Selection"
                            })
                    ),
                    FontId::proportional(11.),
                    palette::label(),
                );
            }
            // How to finish or leave the active tool, with the user's own
            // keys (tool_exit_hint.rs); live tools lead with what they do.
            let exit = self.tool_exit_hint();
            let lead = self.editor.session.live.as_ref().map(|l| {
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
            self.draw_meshop_handles(&painter, r);
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
            self.draw_utility(&painter, r, response.hover_pos());
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
                            .session
                            .live
                            .as_ref()
                            .is_some_and(|l| l.tool.is_primitive());
                    // What Ctrl does on this handle; it is read at the press
                    // (HM-01/12a).
                    let tool = self.editor.session.live.as_ref().map(|l| l.tool);
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
                    input::GestureKind::SessionHandle { id, .. } => *id == axis,
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
                    .session
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
                        .session
                        .live
                        .as_ref()
                        .is_some_and(|l| l.tool == Tool::Scale);
                // A blue scale handle (square end) instead of an arrow.
                let scale_look = self
                    .editor
                    .session
                    .live
                    .as_ref()
                    .is_some_and(|l| scale_look_handle(l.tool, axis));
                let color = if hovered || dragging {
                    palette::handle_active()
                } else if uniform || scale_look {
                    palette::handles()[2]
                } else {
                    palette::handles()[if self.editor.session.live.as_ref().is_some_and(|l| {
                        // Handle 0 runs along the normal: blue.
                        matches!(
                            l.tool.card().handles.kind,
                            HandleKind::Pair | HandleKind::Normal
                        )
                    }) {
                        if axis == 0 { 2 } else { 0 }
                    } else if self
                        .editor
                        .session
                        .live
                        .as_ref()
                        .is_some_and(|l| l.tool == Tool::Transform)
                    {
                        axis % 3
                    } else if self
                        .editor
                        .session
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
                if self.editor.session.live.as_ref().is_some_and(|l| {
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
                        || scale_look
                        || self.editor.session.live.as_ref().is_some_and(|l| {
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
            if self.editor.session.live.is_some()
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
            if let Some(draft) = &self.editor.session.pen_draft {
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
    /// Transparency: Handles. Where a surface hides a handle (its middle,
    /// at the action centre's depth), it draws at this opacity. Not while
    /// a drag changes the scene: the picking surface is stale then and
    /// rebuilding it every frame costs too much on large meshes.
    pub(crate) fn handle_fade(&self, a: Pos2, b: Pos2, r: Rect) -> f32 {
        let xray = self.handle_preferences.xray_opacity();
        let Some(center) = self
            .editor
            .session
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
}

/// A polygon's screen-depth gradient, so widened edge strokes stay on top of
/// their own surface at every pixel. Taken from the polygon's best-shaped fan
/// triangle: a sliver (nearly collinear corners, common in imported n-gons
/// and thin polygons) gives a huge, wrong gradient that pushed strokes in
/// front of other surfaces as stray segments. Below one square pixel there
/// is no usable plane; use none.
pub(super) fn depth_gradient(camera: &crate::Camera, r: Rect, points: &[(Pos2, f64)]) -> Vec2 {
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

/// Whether `tool`'s handle `axis` is drawn as a blue scale handle (a line
/// with a square end) rather than an arrow: Vertex Bevel's Inset, which
/// grows the cap the way a scale does (the maintainer, 2026-11-26).
pub(crate) fn scale_look_handle(tool: Tool, axis: usize) -> bool {
    tool == Tool::VertexBevel && axis == 0
}
