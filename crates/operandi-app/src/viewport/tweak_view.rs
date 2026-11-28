//! Tweak (Element Move) in the viewport: what the pointer would grab, the
//! hover highlight, the drag, and the right-drag that resizes Range with
//! its sphere (spec `specifications/tweak-2026-11-03`, slices TWK-1 and
//! TWK-2; Range resize from the Tweak Range spec v1, TWR-06..14).
use super::*;
use operandi_core::{TweakElement, TweakGrab, TweakMode};

/// What a Tweak press at a point would grab, and where (world space).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TweakPick {
    pub grab: TweakGrab,
    /// The grab point: the vertex, the point on the edge nearest the
    /// pointer, or where the pointer meets the polygon.
    pub at: DVec3,
}

/// The last hover pick and what it depended on.
pub(crate) struct HoverCache {
    key: (Pos2, Rect, [f64; 7], u8, glam::DMat4, TweakMode),
    mesh: operandi_core::Shared<operandi_core::Mesh>,
    /// Any edit (of any active mesh) changes the pick.
    revision: u64,
    symmetry: operandi_core::Symmetry,
    pick: Option<(TweakPick, Option<TweakElement>)>,
}

fn component(element: TweakElement) -> Component {
    match element {
        TweakElement::Vertex(v) => Component::Vertex(v),
        TweakElement::Edge(a, b) => Component::Edge(a.min(b), a.max(b)),
        TweakElement::Polygon(f) => Component::Polygon(f),
    }
}

/// How fast a right-drag resizes Range: pixels of sphere radius on screen
/// per pixel of horizontal pointer travel, so a drag gives the same size on
/// screen at any zoom (TWR-07). At the default view in an 800-point-high
/// viewport, 100 points of travel give about 2 m (provisional, TWR-Q01).
pub(crate) const TWEAK_RANGE_RATE: f64 = 2.;

/// The status line while Tweak waits for a drag (TWR-13).
pub(crate) const TWEAK_HINT: &str =
    "Tweak · Drag: move an element · Right Click and Drag: Adjust Falloff Range";

impl Operandi {
    /// `tweak_pick` for the hover highlight, recomputed only when the
    /// pointer, the view or the active mesh changed: the viewport redraws
    /// far more often than any of those change.
    /// With Symmetry on, also the grabbed element's mirror counterpart.
    fn tweak_hover_pick(&self, p: Pos2, r: Rect) -> Option<(TweakPick, Option<TweakElement>)> {
        let symmetry = self.editor.active_symmetry();
        let item = self.editor.scene.active()?;
        let matrix = self.editor.scene.world_matrix(item.id).ok()?;
        let c = &self.camera;
        let key = (
            p,
            r,
            [
                c.target.x,
                c.target.y,
                c.target.z,
                c.distance,
                c.yaw,
                c.pitch,
                item.id as f64,
            ],
            c.view as u8,
            matrix,
            self.tweak_options().mode,
        );
        let mut cache = self.tweak_hover_cache.borrow_mut();
        if let Some(hit) = cache.as_ref().filter(|h| {
            h.key == key
                && h.symmetry == symmetry
                && h.revision == self.editor.revision
                && operandi_core::Shared::same(&h.mesh, item.mesh.shared())
        }) {
            return hit.pick;
        }
        let pick = self.tweak_pick(p, r).map(|pick| {
            let mirror = self
                .editor
                .scene
                .symmetric_counterpart(pick.grab.item, component(pick.grab.element), symmetry)
                .and_then(|c| match c {
                    Component::Vertex(v) => Some(TweakElement::Vertex(v)),
                    Component::Edge(a, b) => Some(TweakElement::Edge(a, b)),
                    Component::Polygon(f) => Some(TweakElement::Polygon(f)),
                    Component::Item => None,
                })
                .filter(|&m| m != pick.grab.element);
            (pick, mirror)
        });
        *cache = Some(HoverCache {
            key,
            mesh: item.mesh.clone().into(),
            revision: self.editor.revision,
            symmetry,
            pick,
        });
        pick
    }

    /// The live Tweak options (defaults when Tweak isn't live).
    pub(crate) fn tweak_options(&self) -> operandi_core::TweakOptions {
        self.editor
            .live
            .as_ref()
            .filter(|l| l.tool == Tool::Tweak)
            .map(|l| l.params.tweak_options)
            // FO-45: the Element falloff picks with Tweak's code.
            .or_else(|| {
                self.element_falloff().map(|e| operandi_core::TweakOptions {
                    mode: e.mode,
                    range: e.range,
                    connect: e.connect,
                    shape: e.shape,
                })
            })
            .unwrap_or_default()
    }

    /// TWK-03/04 picking on the active mesh, limited to the element types
    /// Mode allows: the nearest point or edge within the pick radius, a
    /// point winning unless an edge is more than Point Selection Priority
    /// nearer, else the polygon under the pointer. Hidden points and edges
    /// (behind a surface) don't count. In the Center modes the grab point is
    /// the element's center. Provisional: the reference's priority and
    /// distances aren't measured (TWK-Q02); these are the selection
    /// pre-highlight's.
    pub(crate) fn tweak_pick(&self, p: Pos2, r: Rect) -> Option<TweakPick> {
        let mode = self.tweak_options().mode;
        let mut pick = self.tweak_pick_in(p, r, mode)?;
        if mode.centered() {
            let item = self.grabbed_item(pick.grab.item)?;
            let matrix = self.editor.scene.world_matrix(item.id).ok()?;
            let points = pick.grab.element.points(&item.mesh).ok()?;
            let shown = self.shown_points(item);
            pick.at = points
                .iter()
                .map(|&v| matrix.transform_point3(shown[v as usize]))
                .sum::<DVec3>()
                / points.len().max(1) as f64;
        }
        Some(pick)
    }

    /// The item a Tweak grab names (an active mesh).
    fn grabbed_item(&self, id: u64) -> Option<&operandi_core::Item> {
        self.editor.scene.item(id)
    }

    /// Picking over every active mesh (multiple active meshes, stage 3):
    /// points and edges of all of them compete, and a polygon of any of
    /// them can be grabbed. With no active mesh, the primary.
    pub(crate) fn tweak_pick_in(&self, p: Pos2, r: Rect, mode: TweakMode) -> Option<TweakPick> {
        let scene = &self.editor.scene;
        let mut ids = scene.shown_active_meshes();
        if ids.is_empty() {
            ids.extend(scene.active().filter(|i| scene.shows(i)).map(|i| i.id));
        }
        self.element_pick_among(p, r, mode, &ids, &self.pick_only())
    }

    /// C-ELEM's pick: the polygon, edge or point under the pointer on any
    /// mesh the view shows, active or not, selected or not. Every shown
    /// item can hide what is behind it.
    pub(crate) fn element_pick_any(&self, p: Pos2, r: Rect) -> Option<TweakPick> {
        let scene = &self.editor.scene;
        let ids: Vec<u64> = scene
            .items
            .iter()
            .filter(|i| i.is_mesh() && scene.shows(i))
            .map(|i| i.id)
            .collect();
        self.element_pick_among(p, r, TweakMode::AutoCenter, &ids, &[])
    }

    /// The element under the pointer among the meshes `ids`: a point or an
    /// edge within reach first, else the polygon. `only` names the items
    /// picking sees and that can hide an element (empty: every item).
    fn element_pick_among(
        &self,
        p: Pos2,
        r: Rect,
        mode: TweakMode,
        ids: &[u64],
        only: &[u64],
    ) -> Option<TweakPick> {
        let scene = &self.editor.scene;
        let surface = self.pick_surface();
        let projector = self.camera.projector(r);
        let reach = Rect::from_center_size(p, Vec2::splat(self.pick_radius() * 2.));
        // Points and edges where they show: on a smooth surface, not its
        // cage, as selection picks them (`item_pick_candidates`).
        let mut points = PickCandidates::default();
        let everywhere = |_: Pos2| true;
        let mut meshes = Vec::new();
        for &id in ids {
            let Some(item) = self.grabbed_item(id) else {
                continue;
            };
            let Ok(matrix) = scene.world_matrix(id) else {
                continue;
            };
            for (wanted, kind) in [
                (mode.grabs_vertices(), Mode::Vertex),
                (mode.grabs_edges(), Mode::Edge),
            ] {
                if wanted {
                    self.item_pick_candidates(
                        item,
                        matrix,
                        kind,
                        reach,
                        &everywhere,
                        &projector,
                        &mut points,
                    );
                }
            }
            meshes.push(id);
        }
        // Points and edges weighed together: Point Selection Priority
        // decides how much nearer an edge must be to beat a point.
        let hit = self.pick_among_items(p, r, &surface, &points, only);
        if let Some((id, Component::Vertex(v))) = hit {
            let &(_, _, _, at) = points.points.iter().find(|c| c.0 == id && c.1 == v)?;
            return Some(TweakPick {
                grab: TweakGrab {
                    item: id,
                    element: TweakElement::Vertex(v),
                },
                at,
            });
        }
        if let Some((id, Component::Edge(a, b))) = hit {
            // The grab point on the nearest drawn piece of the edge (a
            // smooth surface traces each cage edge in several pieces).
            let (_, [(qa, za), (qb, zb)], [wa, wb]) = points
                .edges
                .iter()
                .filter(|c| c.0 == id && c.1 == a && c.2 == b)
                .map(|c| {
                    let [(qa, _), (qb, _)] = c.3;
                    (input::segment_distance(p, qa, qb), c.3, c.4)
                })
                .min_by(|x, y| x.0.total_cmp(&y.0))?;
            let d = qb - qa;
            let t = ((p - qa).dot(d) / d.length_sq().max(1e-8)).clamp(0., 1.);
            let t = picking::span_world_parameter(&self.camera, t, za, zb);
            return Some(TweakPick {
                grab: TweakGrab {
                    item: id,
                    element: TweakElement::Edge(a, b),
                },
                at: wa.lerp(wb, t),
            });
        }
        if !mode.grabs_polygons() {
            return None;
        }
        let (hit_item, face, depth) =
            surface.pick_in(&self.camera, p, r, crate::picking::limit(only))?;
        if !meshes.contains(&hit_item) {
            return None;
        }
        let item = self.grabbed_item(hit_item)?;
        if (face as usize) >= item.mesh.faces.len() {
            return None;
        }
        let (origin, direction) = self.camera.ray(p, r);
        Some(TweakPick {
            grab: TweakGrab {
                item: hit_item,
                element: TweakElement::Polygon(face),
            },
            at: origin + direction * depth,
        })
    }

    /// A primary press with Tweak live: grab what is under the pointer and
    /// start dragging it. Over empty space nothing happens (TWK-23,
    /// provisional).
    pub(crate) fn begin_tweak(&mut self, p: Pos2, r: Rect) -> input::GestureKind {
        let Some(pick) = self.tweak_pick(p, r) else {
            self.status = "Tweak: press on a vertex, edge or polygon of the active mesh".into();
            return input::GestureKind::Blocked;
        };
        let Some(base) = self
            .editor
            .live
            .as_ref()
            .filter(|l| l.tool == Tool::Tweak)
            .map(|l| l.params.clone())
        else {
            return input::GestureKind::Blocked;
        };
        self.clear_fields();
        input::GestureKind::Tweak { pick, base }
    }

    /// TWK-09 (provisional, TWK-Q04): the grab point follows the pointer in
    /// the plane facing the camera through the grab point, so it stays
    /// under the pointer at its own depth in both perspective and
    /// orthographic views.
    pub(crate) fn tweak_drag(
        &mut self,
        pick: TweakPick,
        base: &Parameters,
        start: Pos2,
        p: Pos2,
        r: Rect,
    ) {
        let (_, _, _, forward) = self.camera.basis();
        let (origin, direction) = self.camera.ray(p, r);
        let along = direction.dot(forward);
        if along.abs() < 1e-9 {
            return;
        }
        let mut target = origin + direction * ((pick.at - origin).dot(forward) / along);
        // SN-12: the grab point snaps like a handle; the grabbed element's
        // own points aren't targets.
        let grabbed: std::collections::BTreeSet<u32> = match pick.grab.element {
            TweakElement::Vertex(v) => [v].into(),
            TweakElement::Edge(a, b) => [a, b].into(),
            TweakElement::Polygon(f) => self
                .grabbed_item(pick.grab.item)
                .and_then(|i| i.mesh.faces.get(f as usize))
                .map(|face| face.vertices.iter().copied().collect())
                .unwrap_or_default(),
        };
        if let Some(snapped) = self.snap_point(target, Some(grabbed), r) {
            target = snapped;
        }
        let mut params = base.clone();
        params.tweak = Some(pick.grab);
        params.delta = target - pick.at;
        // TP-30: Topology Pen's smart snapping onto other points.
        if self.topology_pen_open() {
            params.delta = self.topology_pen_snap(pick, params.delta, r);
        }
        // HM-61/71: Ctrl (held at the press) keeps the drag on the first
        // work-plane axis it heads toward, like a Move haul; nothing moves
        // until that axis is picked.
        if self.handle_constrain {
            let Some(axis) = self.haul_lock_axis(p - start) else {
                return;
            };
            params.delta = axis * params.delta.dot(axis);
        }
        // What moves was worked out at the first frame; keep it.
        params.tweak_plan = self
            .editor
            .live
            .as_ref()
            .and_then(|l| l.params.tweak_plan.clone());
        if self
            .editor
            .live
            .as_ref()
            .is_some_and(|l| l.params == params)
        {
            return;
        }
        self.run(Command::Preview(params));
        self.clear_fields();
    }

    /// TWK-02: outline what a press would grab, in the rollover colour.
    pub(crate) fn draw_tweak_hover(&self, painter: &Painter, hover: Option<Pos2>, r: Rect) {
        let Some((pick, mirror)) = hover.and_then(|p| self.tweak_hover_pick(p, r)) else {
            return;
        };
        let Some(item) = self.grabbed_item(pick.grab.item) else {
            return;
        };
        let Ok(matrix) = self.editor.scene.world_matrix(item.id) else {
            return;
        };
        // Where the element shows: points and edges on a smooth surface,
        // not its cage.
        let shown = self.shown_points(item);
        let project = |v: u32| {
            shown
                .get(v as usize)
                .and_then(|&p| self.camera.project(matrix.transform_point3(p), r))
                .map(|(q, _)| q)
        };
        let lines = |edges: std::collections::BTreeSet<(u32, u32)>, color: Color32| {
            let paths = crate::subdivision_view::edge_segments(
                &self.subdivision_cache,
                item,
                matrix,
                &edges,
            );
            for (_, segments) in &paths {
                for &(wa, wb) in segments {
                    if let (Some((qa, _)), Some((qb, _))) =
                        (self.camera.project(wa, r), self.camera.project(wb, r))
                    {
                        painter.line_segment([qa, qb], Stroke::new(4_f32, palette::edge_backing()));
                        painter.line_segment([qa, qb], Stroke::new(2_f32, color));
                    }
                }
            }
        };
        // The grabbed element, and with Symmetry its counterpart in the
        // mirror colour (it moves too, TWK-20).
        let outline = |element: TweakElement, color: Color32| match element {
            TweakElement::Vertex(v) => {
                if let Some(q) = project(v) {
                    painter.circle_filled(q, 4.5, palette::edge_backing());
                    painter.circle_filled(q, 3.5, color);
                }
            }
            TweakElement::Edge(a, b) => lines([(a.min(b), a.max(b))].into(), color),
            TweakElement::Polygon(f) => {
                if let Some(face) = item.mesh.faces.get(f as usize) {
                    let v = &face.vertices;
                    let n = v.len();
                    lines(
                        (0..n)
                            .map(|i| {
                                let (a, b) = (v[i], v[(i + 1) % n]);
                                (a.min(b), a.max(b))
                            })
                            .collect(),
                        color,
                    );
                }
            }
        };
        outline(pick.grab.element, palette::rollover());
        if let Some(mirror) = mirror {
            outline(mirror, palette::mirror_selection());
        }
    }

    /// A right press with Tweak live: start resizing Range (TWR-06). The
    /// sphere centres on the surface under the pointer, else on the work
    /// plane (provisional, TWR-Q02). Modifier keys change nothing (TWR-10).
    pub(crate) fn begin_tweak_range(&mut self, p: Pos2, r: Rect) -> input::GestureKind {
        if !self.tweak_options().connect.uses_range() {
            self.status = "Tweak: Rigid doesn't use Range".into();
            return input::GestureKind::Blocked;
        }
        let (origin, direction) = self.camera.ray(p, r);
        let center = self
            .pick_surface()
            .pick(&self.camera, p, r)
            .map(|(_, _, depth)| origin + direction * depth)
            .or_else(|| self.work_point(p, r, self.actual_plane(), self.workplane_center()))
            .unwrap_or(origin + direction * self.camera.distance);
        let Some(metres_per_pixel) = self.camera.metres_per_pixel(center, r) else {
            return input::GestureKind::Blocked;
        };
        self.clear_fields();
        self.range_status();
        input::GestureKind::TweakRange {
            center,
            metres_per_pixel,
        }
    }

    /// TWR-06/07/08: each pixel moved right grows Range, left shrinks it,
    /// by `TWEAK_RANGE_RATE` pixels of sphere radius on screen; vertical
    /// movement is ignored and Range stops at 0. The change adds up from
    /// move to move, so reversing after hitting 0 grows it straight away.
    pub(crate) fn tweak_range_drag(&mut self, metres_per_pixel: f64, from: Pos2, to: Pos2) {
        let dx = (to.x - from.x) as f64;
        if dx == 0. {
            return;
        }
        let Some(mut params) = self
            .editor
            .live
            .as_ref()
            .filter(|l| l.tool == Tool::Tweak)
            .map(|l| l.params.clone())
        else {
            return;
        };
        let range = params.tweak_options.range;
        let next = (range + dx * TWEAK_RANGE_RATE * metres_per_pixel).max(0.);
        if next == range {
            return;
        }
        params.tweak_options.range = next;
        self.run(Command::Preview(params));
        // TWR-09: the field shows the new value as it changes.
        self.clear_fields();
        self.range_status();
    }

    fn range_status(&mut self) {
        let range = self.tweak_options().range;
        self.status = format!(
            "Tweak Range: {} · drag right to grow, left to shrink",
            workspace::number_text(range, " m")
        );
    }

    /// TWK-14/TWR-11: the falloff range as a wireframe sphere of four
    /// circles around `center` (world): its outline facing the camera plus
    /// its three axis circles. Drawn only during a right-drag (TWR-12).
    /// Nothing at Range 0 or with Rigid (which doesn't use Range).
    pub(crate) fn draw_tweak_range(&self, painter: &Painter, center: DVec3, r: Rect) {
        let options = self.tweak_options();
        if !(options.range > 0. && options.connect.uses_range()) {
            return;
        }
        let (_, right, up, _) = self.camera.basis();
        let radius = options.range;
        let color = palette::tweak_range();
        let circle = |a: DVec3, b: DVec3, stroke: Stroke| {
            const STEPS: usize = 64;
            let points: Vec<Option<Pos2>> = (0..=STEPS)
                .map(|i| {
                    let t = i as f64 / STEPS as f64 * std::f64::consts::TAU;
                    let p = center + (a * t.cos() + b * t.sin()) * radius;
                    self.camera.project(p, r).map(|(q, _)| q)
                })
                .collect();
            for pair in points.windows(2) {
                if let [Some(qa), Some(qb)] = pair {
                    painter.line_segment([*qa, *qb], stroke);
                }
            }
        };
        let faint = Stroke::new(1_f32, color.gamma_multiply(0.45));
        circle(DVec3::X, DVec3::Y, faint);
        circle(DVec3::Y, DVec3::Z, faint);
        circle(DVec3::Z, DVec3::X, faint);
        circle(right, up, Stroke::new(1.5_f32, color));
    }
}
