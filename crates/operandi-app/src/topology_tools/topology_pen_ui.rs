//! Topology Pen v2 in the viewport (BEHAVIOR-TOPOLOGY-PEN, TP-10..TP-40,
//! 2026-11-20): the Mode drop-down's gestures, the modifier gestures, the
//! pre-highlight and the Tool Properties. Core: `operandi_core::topology_pen`.
//!
//! Every press-to-release is one undo step; a press that hits nothing
//! changes nothing. Edits that add or remove geometry drop the pen, run
//! one `Command::TopologyPen` and start the pen again (its options stick,
//! `Editor::topology_pen_memory`).
//!
//! Keys (UI bible, "Keys"): Ctrl in these click and drag gestures is the
//! real Ctrl key on the Mac too, never Cmd.
use super::*;
use crate::create_utility::UtilityDrag;
use crate::tool_form as tf;
use operandi_core::topology_pen::{self as tp, PenEdit, PenMode, TopologyPenOptions};
use operandi_core::{Mesh, TweakElement};
use theme::viewport as palette;

/// What a Topology Pen press holds until release.
#[derive(Clone, Debug)]
pub(crate) enum PenDrag {
    /// Smoothing or Slide: points move in the scene from `base` (the
    /// press-time mesh); the release makes them one edit.
    Points {
        item: u64,
        base: Mesh,
        kind: PointsKind,
        last: Pos2,
        label: &'static str,
    },
    /// Split: a rubber band from point `from` (world `at`).
    Split { item: u64, from: u32, at: DVec3 },
    /// Add Loop across edge `a`–`b` at `t` (0..1 from `a`).
    AddLoop {
        a: u32,
        b: u32,
        t: f64,
        /// The edges the loop crosses and which pairs a new edge joins.
        ring: Vec<(u32, u32)>,
        joins: Vec<[usize; 2]>,
    },
    /// Duplicate: the element pulled out from grab point `at`.
    Duplicate {
        item: u64,
        element: TweakElement,
        whole_loop: bool,
        at: DVec3,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum PointsKind {
    /// Smoothing; `along` is the loop for Shift+Ctrl+right / Edge Loop.
    Smooth { along: Option<Vec<(u32, u32)>> },
    /// Slide: each point runs along one of its own edges (`rails`: local
    /// unit direction and length), grabbed at world `at`.
    Slide {
        rails: Vec<(u32, Vec<(DVec3, f64)>)>,
        at: DVec3,
    },
}

/// Fill's last pre-highlight: pointer, scene revision, polygon (world).
type FillHover = (Pos2, u64, Option<Vec<DVec3>>);

/// The pen's viewport state.
#[derive(Default)]
pub(crate) struct PenUi {
    pub drag: Option<PenDrag>,
    /// The active item the "no background" hint was last given for.
    hinted: Option<u64>,
    /// Smart snapping: the welds a release would make (dragged → target).
    pub weld: Vec<(u32, u32)>,
    /// The world point the dragged point jumped to, for the highlight.
    pub weld_at: Option<DVec3>,
    /// The last element moved (item, its points) and its offset, for the
    /// Offset fields (TP-24).
    pub last: Option<(u64, Vec<u32>)>,
    /// TP-04: the background constraint and snapping as they were before
    /// the Topology tab's button started the pen, put back on drop.
    pub restore: Option<(bool, bool)>,
    /// Fill's pre-highlight for the last hover (pointer, scene revision).
    fill_cache: std::cell::RefCell<Option<FillHover>>,
}

/// Pixels of pointer travel between Smoothing passes.
const SMOOTH_STEP: f32 = 4.;

impl Operandi {
    /// TP-04: the Topology tab's Topology Pen button starts the pen with
    /// the background constraint on and snapping off; both go back as
    /// they were when the pen drops (`settle_topology_pen`). The Draw row's
    /// button leaves them as the user has them.
    pub(super) fn topology_tab_pen(&mut self) {
        let before = (
            self.editor.background_constraint.is_some(),
            self.snapping.enabled,
        );
        if !before.0 {
            self.const_bg_set("on");
        }
        if before.1 {
            self.toggle_snapping();
        }
        self.execute("Topology Pen");
        if self.topology_pen_open() {
            self.pen.restore = Some(before);
        } else {
            // It didn't start: put them back now.
            self.pen.restore = Some(before);
            self.settle_topology_pen();
        }
    }

    /// Every frame: once the pen is down, give back what TP-04 changed.
    pub(super) fn settle_topology_pen(&mut self) {
        if self.topology_pen_open() {
            return;
        }
        self.pen.drag = None;
        self.pen.weld.clear();
        self.pen.weld_at = None;
        let Some((constraint, snapping)) = self.pen.restore.take() else {
            return;
        };
        if self.editor.background_constraint.is_some() != constraint {
            self.const_bg_set(if constraint { "on" } else { "off" });
        }
        if self.snapping.enabled != snapping {
            self.toggle_snapping();
        }
    }

    /// The live Topology Pen options (defaults when the pen isn't up).
    pub(super) fn pen_options(&self) -> TopologyPenOptions {
        self.editor
            .session
            .live
            .as_ref()
            .filter(|_| self.topology_pen_open())
            .map(|l| {
                let mut o = l.params.topology_pen;
                // The falloff lives in Tweak's options (Move uses them).
                o.radius = l.params.tweak_options.range;
                o.shape = l.params.tweak_options.shape;
                o.use_connectivity =
                    l.params.tweak_options.connect == operandi_core::TweakConnect::Connectivity;
                o
            })
            .unwrap_or_default()
    }

    /// Drop the pen, run one edit and start the pen again; the status
    /// keeps the edit's own message.
    fn pen_commit(&mut self, edit: PenEdit, done: &str) -> bool {
        let row = self.editor.session.preset.as_ref().map(|p| p.id.clone());
        self.run(Command::Drop);
        let ok = self.run_ok(Command::TopologyPen(Box::new(edit)));
        let message = self.status.clone();
        if let Some(id) = row {
            self.activate_preset(&id);
        }
        self.status = if ok { done.to_string() } else { message };
        ok
    }

    /// The world point under the pointer on the background, else on the
    /// work plane (TP-15; hint once per item when there's no background).
    fn pen_place(&mut self, pos: Pos2, r: Rect) -> Option<DVec3> {
        let (origin, dir) = self.camera.ray(pos, r);
        if let Some(hit) = self.editor.background_hit(origin, dir) {
            return Some(hit);
        }
        let item = self.editor.scene.active().map(|i| i.id);
        if self.pen.hinted != item {
            self.pen.hinted = item;
            self.status = "Topology Pen: no background mesh, so points go on the work plane".into();
        }
        self.work_point(pos, r, self.actual_plane(), self.workplane_center())
    }

    /// The world matrix of the active mesh.
    fn pen_world(&self, item: u64) -> Option<glam::DMat4> {
        self.editor.scene.world_matrix(item).ok()
    }

    /// TP-10/20: a press with Topology Pen up. Returns the gesture it
    /// starts (`Blocked` for clicks done at the press or refused).
    pub(super) fn topology_pen_press(
        &mut self,
        button: PointerButton,
        modifiers: Modifiers,
        pos: Pos2,
        r: Rect,
    ) -> input::GestureKind {
        use input::GestureKind as G;
        if self.editor.scene.selection.mode == Mode::Item {
            self.status = "Topology Pen: pick a component mode (Vertex, Edge or Polygon)".into();
            return G::Blocked;
        }
        let o = self.pen_options();
        let (shift, ctrl) = (modifiers.shift, modifiers.ctrl);
        let pick = self.tweak_pick(pos, r);
        let element = pick.map(|p| p.grab.element);
        let on_edge = matches!(element, Some(TweakElement::Edge(..)));
        match (button, shift, ctrl) {
            (PointerButton::Primary, false, false) => match o.mode {
                PenMode::Move if o.edge_loop && on_edge => self.pen_loop_move(pos, r),
                PenMode::Move if o.edge_slide && pick.is_some() => self.pen_begin_slide(pos, r),
                PenMode::Move => self.begin_tweak(pos, r),
                PenMode::Duplicate => self.pen_begin_duplicate(pos, r, o.edge_loop),
                PenMode::Remove => {
                    self.pen_remove(pos, r, o.edge_loop, o.keep_vertices);
                    G::Blocked
                }
                PenMode::Split => self.pen_begin_split(pos, r),
                PenMode::AddLoop => self.pen_begin_add_loop(pos, r, o.split_middle),
                PenMode::Point if pick.is_some() => self.begin_tweak(pos, r),
                PenMode::Point => {
                    if let Some(at) = self.pen_place(pos, r) {
                        self.pen_commit(PenEdit::Point { world: at }, "Topology Pen: point");
                    }
                    G::Blocked
                }
                PenMode::Fill => {
                    self.pen_fill(pos, r, o);
                    G::Blocked
                }
                PenMode::Smoothing => {
                    let along = (o.edge_loop && on_edge).then_some(());
                    self.pen_begin_smooth(pos, r, along.is_some())
                }
            },
            // TP-20 modifier gestures.
            (PointerButton::Primary, true, false) => self.pen_begin_duplicate(pos, r, false),
            (PointerButton::Primary, false, true) if pick.is_some() => self.pen_begin_slide(pos, r),
            (PointerButton::Primary, false, true) => {
                self.pen_quad(pos, r);
                G::Blocked
            }
            (PointerButton::Primary, true, true) => self.pen_begin_smooth(pos, r, false),
            (PointerButton::Secondary, false, false) if on_edge => self.pen_loop_move(pos, r),
            (PointerButton::Secondary, false, false) => self.begin_tweak_range(pos, r),
            (PointerButton::Secondary, true, false) if on_edge => {
                self.pen_begin_duplicate(pos, r, true)
            }
            (PointerButton::Secondary, true, true) if on_edge => {
                self.pen_begin_smooth(pos, r, true)
            }
            (PointerButton::Middle, false, false) => self.pen_begin_split(pos, r),
            (PointerButton::Middle, true, false) | (PointerButton::Middle, false, true) => {
                self.topology_middle_press(pos, r, shift, o.edge_loop, o.keep_vertices);
                G::Blocked
            }
            _ => {
                self.status = "Topology Pen: that button and key combination does nothing".into();
                G::Blocked
            }
        }
    }

    /// TP-10 Edge Loop / right-drag on an edge: Move with the whole loop.
    fn pen_loop_move(&mut self, pos: Pos2, r: Rect) -> input::GestureKind {
        match self.begin_tweak(pos, r) {
            input::GestureKind::Tweak { pick, mut base } => {
                base.tweak_options.connect = operandi_core::TweakConnect::EdgeLoops;
                input::GestureKind::Tweak { pick, base }
            }
            other => other,
        }
    }

    /// TP-12 Remove at the press.
    fn pen_remove(&mut self, pos: Pos2, r: Rect, edge_loop: bool, keep_vertices: bool) {
        let Some(pick) = self.tweak_pick(pos, r) else {
            self.status = "Topology Pen Remove: click a point, edge or polygon".into();
            return;
        };
        self.pen_commit(
            PenEdit::Remove {
                item: pick.grab.item,
                element: pick.grab.element,
                edge_loop,
                keep_vertices,
            },
            "Topology Pen: removed",
        );
    }

    /// TC-42 Shift/Ctrl+middle-click: Add Loop at the middle / Remove.
    pub(super) fn topology_middle_press(
        &mut self,
        pos: Pos2,
        r: Rect,
        add_loop: bool,
        edge_loop: bool,
        keep_vertices: bool,
    ) {
        if !add_loop {
            return self.pen_remove(pos, r, edge_loop, keep_vertices);
        }
        match self.tweak_pick(pos, r).map(|p| p.grab.element) {
            Some(TweakElement::Edge(a, b)) => {
                let row = self.editor.session.preset.as_ref().map(|p| p.id.clone());
                self.run(Command::Drop);
                self.run(Command::AddEdgeLoop { a, b, t: 0.5 });
                if let Some(id) = row {
                    self.activate_preset(&id);
                }
            }
            _ => self.status = "Topology Pen: Shift+middle-click an edge to add a loop".into(),
        }
    }

    /// TP-16 Fill at the press.
    fn pen_fill(&mut self, pos: Pos2, r: Rect, o: TopologyPenOptions) {
        let Some(item) = self.editor.scene.active().map(|i| i.id) else {
            return;
        };
        let Some(click) = self.pen_place(pos, r) else {
            return;
        };
        let toward_viewer = self.camera.basis().0;
        self.pen_commit(
            PenEdit::Fill {
                item,
                click,
                toward_viewer,
                range: o.fill_range,
                quad_only: o.quad_only,
            },
            "Topology Pen: filled",
        );
    }

    /// TP-20 Ctrl+click empty space: a square quad facing the viewer, its
    /// side 5 % of the view's visible height.
    fn pen_quad(&mut self, pos: Pos2, r: Rect) {
        let Some(center) = self.pen_place(pos, r) else {
            return;
        };
        let Some(mpp) = self.camera.metres_per_pixel(center, r) else {
            return;
        };
        let (_, right, up, _) = self.camera.basis();
        let side = mpp * r.height() as f64 * 0.05;
        self.pen_commit(
            PenEdit::Quad {
                center,
                right,
                up,
                side,
            },
            "Topology Pen: new quad",
        );
    }

    /// TP-11 Duplicate: a press on a border edge or point.
    fn pen_begin_duplicate(&mut self, pos: Pos2, r: Rect, whole_loop: bool) -> input::GestureKind {
        let Some(pick) = self.tweak_pick(pos, r) else {
            self.status = "Topology Pen: drag a border edge or point to duplicate it".into();
            return input::GestureKind::Blocked;
        };
        let Some(mesh) = self.grabbed_mesh(pick.grab.item) else {
            return input::GestureKind::Blocked;
        };
        let border = match pick.grab.element {
            TweakElement::Edge(a, b) => tp::is_border_edge(&mesh, a, b),
            TweakElement::Vertex(v) => !tp::border_neighbours(&mesh, v).is_empty(),
            TweakElement::Polygon(_) => false,
        };
        if !border {
            self.status = "Duplicate needs a border edge".into();
            return input::GestureKind::Blocked;
        }
        self.pen.drag = Some(PenDrag::Duplicate {
            item: pick.grab.item,
            element: pick.grab.element,
            whole_loop,
            at: pick.at,
        });
        self.status = "Topology Pen: drag out the new polygon; release to make it".into();
        input::GestureKind::Utility(UtilityDrag::TopologyPen)
    }

    fn grabbed_mesh(&self, item: u64) -> Option<Mesh> {
        self.editor.scene.item(item).map(|i| (*i.mesh).clone())
    }

    /// TP-13 Split: a press on a point starts the rubber band.
    fn pen_begin_split(&mut self, pos: Pos2, r: Rect) -> input::GestureKind {
        match self.tweak_pick(pos, r) {
            Some(pick) => match pick.grab.element {
                TweakElement::Vertex(v) => {
                    self.pen.drag = Some(PenDrag::Split {
                        item: pick.grab.item,
                        from: v,
                        at: pick.at,
                    });
                    self.status =
                        "Topology Pen Split: release on another point of the same polygon".into();
                    input::GestureKind::Utility(UtilityDrag::TopologyPen)
                }
                _ => {
                    self.status = "Topology Pen Split: drag from a point".into();
                    input::GestureKind::Blocked
                }
            },
            None => {
                self.status = "Topology Pen Split: drag from a point".into();
                input::GestureKind::Blocked
            }
        }
    }

    /// TP-14 Add Loop: a press on an edge puts the loop there.
    fn pen_begin_add_loop(&mut self, pos: Pos2, r: Rect, middle: bool) -> input::GestureKind {
        let Some(pick) = self.tweak_pick(pos, r) else {
            self.status = "Topology Pen Add Loop: click an edge".into();
            return input::GestureKind::Blocked;
        };
        let TweakElement::Edge(a, b) = pick.grab.element else {
            self.status = "Topology Pen Add Loop: click an edge".into();
            return input::GestureKind::Blocked;
        };
        let Some(mesh) = self.grabbed_mesh(pick.grab.item) else {
            return input::GestureKind::Blocked;
        };
        let (ring, joins) = match operandi_core::add_loop_ring(&mesh, a, b) {
            Ok(x) => x,
            Err(e) => {
                self.status = e;
                return input::GestureKind::Blocked;
            }
        };
        let t = if middle {
            0.5
        } else {
            self.pen_edge_t(pick.grab.item, a, b, pos, r)
        };
        self.pen.drag = Some(PenDrag::AddLoop {
            a,
            b,
            t,
            ring,
            joins,
        });
        input::GestureKind::Utility(UtilityDrag::TopologyPen)
    }

    /// Where along edge `a`–`b` the pointer is, 0..1 (on screen).
    fn pen_edge_t(&self, item: u64, a: u32, b: u32, pos: Pos2, r: Rect) -> f64 {
        let (Some(it), Some(world)) = (self.editor.scene.item(item), self.pen_world(item)) else {
            return 0.5;
        };
        let at = |v: u32| {
            it.mesh
                .vertices
                .get(v as usize)
                .and_then(|&p| self.camera.project(world.transform_point3(p), r))
                .map(|(q, _)| q)
        };
        let (Some(qa), Some(qb)) = (at(a), at(b)) else {
            return 0.5;
        };
        let d = qb - qa;
        ((pos - qa).dot(d) / d.length_sq().max(1e-6)).clamp(0.02, 0.98) as f64
    }

    /// TP-17 Smoothing: one pass at the press, more as the pointer moves.
    fn pen_begin_smooth(&mut self, pos: Pos2, r: Rect, along_loop: bool) -> input::GestureKind {
        let Some(item) = self.editor.scene.active().map(|i| i.id) else {
            return input::GestureKind::Blocked;
        };
        let along = if along_loop {
            match self.tweak_pick(pos, r).map(|p| p.grab.element) {
                Some(TweakElement::Edge(a, b)) => {
                    Some(tp::pen_edge_loop(&self.editor.scene, item, a, b))
                }
                _ => {
                    self.status =
                        "Topology Pen: Shift+Ctrl+right-drag an edge to smooth its loop".into();
                    return input::GestureKind::Blocked;
                }
            }
        } else {
            None
        };
        let Some(base) = self.grabbed_mesh(item) else {
            return input::GestureKind::Blocked;
        };
        self.pen.drag = Some(PenDrag::Points {
            item,
            base,
            kind: PointsKind::Smooth { along },
            last: pos,
            label: "Topology Pen Smoothing",
        });
        self.pen_smooth_pass(pos, r);
        input::GestureKind::Utility(UtilityDrag::TopologyPen)
    }

    /// One Smoothing pass around the pointer, then the moved points back
    /// onto the background.
    fn pen_smooth_pass(&mut self, pos: Pos2, r: Rect) {
        let o = self.pen_options();
        let Some(PenDrag::Points {
            item,
            kind: PointsKind::Smooth { along },
            ..
        }) = &self.pen.drag
        else {
            return;
        };
        let (item, along) = (*item, along.clone());
        let Some(world) = self.pen_world(item) else {
            return;
        };
        let local = world.inverse();
        let Some(mesh) = self.grabbed_mesh(item) else {
            return;
        };
        let (origin, dir) = self.camera.ray(pos, r);
        let center = self
            .pick_surface()
            .pick(&self.camera, pos, r)
            .map(|(_, _, depth)| origin + dir * depth)
            .or_else(|| self.editor.background_hit(origin, dir))
            .unwrap_or(origin + dir * self.camera.distance);
        let weights = match &along {
            Some(edges) => {
                let points: std::collections::BTreeSet<u32> =
                    edges.iter().flat_map(|&(a, b)| [a, b]).collect();
                points.into_iter().map(|v| (v, 1.)).collect()
            }
            None => {
                // The radius is in world metres; measure in local space
                // through the matrix's scale (provisional: uniform scale).
                let scale = local.transform_vector3(DVec3::X).length();
                tp::falloff_weights(
                    &mesh,
                    local.transform_point3(center),
                    o.radius * scale,
                    o.shape,
                )
            }
        };
        let moved = tp::smooth_points(
            &mesh,
            &weights,
            o.strength,
            o.iterations,
            o.lock_boundary,
            o.lock_corner,
            along.as_deref(),
        );
        let landed: Vec<(u32, DVec3)> = moved
            .into_iter()
            .map(|(v, p)| {
                let on = self.editor.land_on_background(world.transform_point3(p));
                (v, local.transform_point3(on))
            })
            .collect();
        let Some(it) = self.editor.scene.item_mut(item) else {
            return;
        };
        let m = &mut *it.mesh;
        for (v, p) in landed {
            if let Some(at) = m.vertices.get_mut(v as usize) {
                *at = p;
            }
        }
    }

    /// TP-20 Slide: each point of the element runs along its own edges.
    fn pen_begin_slide(&mut self, pos: Pos2, r: Rect) -> input::GestureKind {
        let Some(pick) = self.tweak_pick(pos, r) else {
            return input::GestureKind::Blocked;
        };
        let Some(base) = self.grabbed_mesh(pick.grab.item) else {
            return input::GestureKind::Blocked;
        };
        let Ok(points) = pick.grab.element.points(&base) else {
            return input::GestureKind::Blocked;
        };
        let edges = base.edges();
        let rails = points
            .iter()
            .map(|&p| {
                let ends: Vec<(DVec3, f64)> = edges
                    .iter()
                    .filter_map(|&(a, b)| {
                        let q = if a == p {
                            b
                        } else if b == p {
                            a
                        } else {
                            return None;
                        };
                        if points.contains(&q) {
                            return None;
                        }
                        let d = base.vertices[q as usize] - base.vertices[p as usize];
                        let len = d.length();
                        (len > 1e-12).then(|| (d / len, len))
                    })
                    .collect();
                (p, ends)
            })
            .collect();
        self.pen.drag = Some(PenDrag::Points {
            item: pick.grab.item,
            base,
            kind: PointsKind::Slide { rails, at: pick.at },
            last: pos,
            label: "Topology Pen Slide",
        });
        self.status = "Topology Pen Slide: the points run along their edges".into();
        input::GestureKind::Utility(UtilityDrag::TopologyPen)
    }

    fn pen_slide_to(&mut self, pos: Pos2, r: Rect) {
        let Some(PenDrag::Points {
            item,
            base,
            kind: PointsKind::Slide { rails, at },
            ..
        }) = &self.pen.drag
        else {
            return;
        };
        let item = *item;
        let Some(world) = self.pen_world(item) else {
            return;
        };
        let (_, _, _, forward) = self.camera.basis();
        let (origin, direction) = self.camera.ray(pos, r);
        let along = direction.dot(forward);
        if along.abs() < 1e-9 {
            return;
        }
        let target = origin + direction * ((*at - origin).dot(forward) / along);
        let drag = world.inverse().transform_vector3(target - *at);
        let moves: Vec<(u32, DVec3)> = rails
            .iter()
            .filter_map(|(p, ends)| {
                let start = base.vertices[*p as usize];
                let (dir, len) = ends
                    .iter()
                    .copied()
                    .max_by(|x, y| x.0.dot(drag).total_cmp(&y.0.dot(drag)))?;
                let s = drag.dot(dir).clamp(0., len);
                Some((*p, start + dir * s))
            })
            .collect();
        if let Some(it) = self.editor.scene.item_mut(item) {
            let m = &mut *it.mesh;
            for (v, p) in moves {
                m.vertices[v as usize] = p;
            }
        }
    }

    /// A pointer move during a pen drag.
    pub(super) fn topology_pen_drag(&mut self, pos: Pos2, r: Rect) {
        match &mut self.pen.drag {
            Some(PenDrag::Points { kind, last, .. }) => match kind {
                PointsKind::Smooth { .. } => {
                    if last.distance(pos) >= SMOOTH_STEP {
                        *last = pos;
                        self.pen_smooth_pass(pos, r);
                    }
                }
                PointsKind::Slide { .. } => self.pen_slide_to(pos, r),
            },
            Some(PenDrag::AddLoop { a, b, .. }) => {
                let (a, b) = (*a, *b);
                let next = if self.pen_options().split_middle {
                    0.5
                } else {
                    let item = self.editor.scene.active().map_or(0, |i| i.id);
                    self.pen_edge_t(item, a, b, pos, r)
                };
                if let Some(PenDrag::AddLoop { t, .. }) = &mut self.pen.drag {
                    *t = next;
                }
            }
            _ => {}
        }
    }

    /// The release of a pen drag: one edit, or nothing.
    pub(super) fn topology_pen_release(&mut self, pos: Pos2, r: Rect) {
        let Some(drag) = self.pen.drag.take() else {
            return;
        };
        match drag {
            PenDrag::Points {
                item, base, label, ..
            } => {
                let Some(it) = self.editor.scene.item_mut(item) else {
                    return;
                };
                let now = it.mesh.vertices.clone();
                // Back to the press-time mesh; the edit makes the change.
                it.mesh = base.clone().into();
                let points: Vec<(u32, DVec3)> = now
                    .iter()
                    .zip(&base.vertices)
                    .enumerate()
                    .filter(|(_, (a, b))| a != b)
                    .map(|(v, (a, _))| (v as u32, *a))
                    .collect();
                if points.is_empty() {
                    return;
                }
                self.pen_commit(
                    PenEdit::MovePoints {
                        item,
                        points,
                        label: label.into(),
                    },
                    label,
                );
            }
            PenDrag::Split { item, from, .. } => match self.tweak_pick(pos, r) {
                Some(pick) => match pick.grab.element {
                    TweakElement::Vertex(to) if pick.grab.item == item => {
                        self.pen_commit(
                            PenEdit::Split {
                                item,
                                a: from,
                                b: to,
                            },
                            "Topology Pen: split",
                        );
                    }
                    _ => self.status = "Topology Pen Split: release on a point".into(),
                },
                None => self.status = "Topology Pen Split: release on a point; nothing cut".into(),
            },
            PenDrag::AddLoop { a, b, t, .. } => {
                let row = self.editor.session.preset.as_ref().map(|p| p.id.clone());
                self.run(Command::Drop);
                let ok = self.run_ok(Command::AddEdgeLoop { a, b, t });
                let message = self.status.clone();
                if let Some(id) = row {
                    self.activate_preset(&id);
                }
                // The Position field shows where the loop went.
                if let Some(l) = self.editor.session.live.as_mut() {
                    l.params.topology_pen.position = t;
                }
                self.status = if ok {
                    format!("Topology Pen: loop at {:.0} %", t * 100.)
                } else {
                    message
                };
            }
            PenDrag::Duplicate {
                item,
                element,
                whole_loop,
                at,
            } => {
                let (_, _, _, forward) = self.camera.basis();
                let (origin, direction) = self.camera.ray(pos, r);
                let along = direction.dot(forward);
                if along.abs() < 1e-9 {
                    return;
                }
                let target = origin + direction * ((at - origin).dot(forward) / along);
                let offset = target - at;
                if offset.length() < 1e-9 {
                    self.status = "Topology Pen: drag further to duplicate".into();
                    return;
                }
                let edit = match element {
                    TweakElement::Vertex(v) => PenEdit::DuplicateVertex { item, v, offset },
                    TweakElement::Edge(a, b) if whole_loop => {
                        PenEdit::DuplicateLoop { item, a, b, offset }
                    }
                    TweakElement::Edge(a, b) => {
                        // One edge: the existing strip (TC-45).
                        let row = self.editor.session.preset.as_ref().map(|p| p.id.clone());
                        self.run(Command::Drop);
                        let ok = self.run_ok(Command::TopologyDuplicate { item, a, b, offset });
                        let message = self.status.clone();
                        if let Some(id) = row {
                            self.activate_preset(&id);
                        }
                        self.status = if ok {
                            "Topology Pen: new strip on the background".into()
                        } else {
                            message
                        };
                        return;
                    }
                    TweakElement::Polygon(_) => return,
                };
                self.pen_commit(edit, "Topology Pen: duplicated");
            }
        }
    }

    // ------------------------------------------------------ smart snapping

    /// TP-30 smart snapping during a Move drag: the first dragged point
    /// within the pick radius of another point of the mesh jumps onto it;
    /// the welds wait for the release. Border → border only unless Inner
    /// Snap; the element's own points never snap to each other.
    pub(super) fn topology_pen_snap(
        &mut self,
        pick: viewport::tweak_view::TweakPick,
        delta: DVec3,
        r: Rect,
    ) -> DVec3 {
        self.pen.weld.clear();
        self.pen.weld_at = None;
        let o = self.pen_options();
        let Some(base) = self
            .editor
            .session
            .live
            .as_ref()
            .and_then(|l| l.base.item(pick.grab.item))
            .map(|i| i.mesh.clone())
        else {
            return delta;
        };
        let Some(world) = self.pen_world(pick.grab.item) else {
            return delta;
        };
        let Ok(points) = pick.grab.element.points(&base) else {
            return delta;
        };
        let border = |v: u32| !tp::border_neighbours(&base, v).is_empty();
        let pinned = self
            .editor
            .scene
            .item(pick.grab.item)
            .and_then(|i| operandi_core::Protection::of(&i.mesh, &i.elements).map(|p| p.pinned))
            .unwrap_or_default();
        let used = base.used_points();
        let radius = self.pick_radius();
        let screen = |p: DVec3| self.camera.project(p, r).map(|(q, _)| q);
        let mut best: Option<(f32, u32, u32)> = None;
        for &p in &points {
            if !o.inner_snap && !border(p) {
                continue;
            }
            let Some(q) = screen(world.transform_point3(base.vertices[p as usize]) + delta) else {
                continue;
            };
            for t in 0..base.vertices.len() as u32 {
                if points.contains(&t)
                    || !used[t as usize]
                    || pinned.get(t as usize).copied().unwrap_or(false)
                {
                    continue;
                }
                let Some(qt) = screen(world.transform_point3(base.vertices[t as usize])) else {
                    continue;
                };
                let d = q.distance(qt);
                if d <= radius && best.is_none_or(|(bd, ..)| d < bd) && (o.inner_snap || border(t))
                {
                    best = Some((d, p, t));
                }
            }
        }
        let Some((_, p, t)) = best else {
            return delta;
        };
        let target = world.transform_point3(base.vertices[t as usize]);
        let snapped = target - world.transform_point3(base.vertices[p as usize]);
        self.pen.weld.push((p, t));
        self.pen.weld_at = Some(target);
        snapped
    }

    /// The release of a pen Move drag. With a weld waiting: the move and
    /// the weld are one edit (true). Otherwise remembers the element and
    /// offset for the Offset fields and lets the move commit (false).
    pub(super) fn pen_tweak_release(&mut self, pick: viewport::tweak_view::TweakPick) -> bool {
        let welds = std::mem::take(&mut self.pen.weld);
        self.pen.weld_at = None;
        let Some(live) = self.editor.session.live.as_ref() else {
            return false;
        };
        let delta = live.params.delta;
        let item = pick.grab.item;
        if welds.is_empty() {
            let points = live
                .base
                .item(item)
                .and_then(|i| pick.grab.element.points(&i.mesh).ok())
                .unwrap_or_default();
            self.pen.last = Some((item, points));
            if let Some(l) = self.editor.session.live.as_mut() {
                l.params.topology_pen.offset = delta;
            }
            return false;
        }
        let (Some(before), Some(after)) = (live.base.item(item), self.editor.scene.item(item))
        else {
            return false;
        };
        let moves: Vec<(u32, DVec3)> = after
            .mesh
            .vertices
            .iter()
            .zip(&before.mesh.vertices)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(v, (a, _))| (v as u32, *a))
            .collect();
        // Undo the preview; the edit below makes the move and the weld.
        let mut params = live.params.clone();
        params.delta = DVec3::ZERO;
        self.run(Command::Preview(params));
        let copy_uvs = self.pen_options().copy_uvs;
        self.pen_commit(
            PenEdit::Weld {
                item,
                pairs: welds,
                moves,
                copy_uvs,
            },
            "Topology Pen: welded",
        );
        true
    }

    /// TP-24: typing an Offset moves the last moved element by the change.
    pub(super) fn pen_offset_typed(&mut self, offset: DVec3) {
        let old = self.pen_options().offset;
        let Some((item, points)) = self.pen.last.clone() else {
            self.status = "Topology Pen: drag an element first; Offset moves the last one".into();
            return;
        };
        let (Some(world), Some(mesh)) = (self.pen_world(item), self.grabbed_mesh(item)) else {
            return;
        };
        let d = world.inverse().transform_vector3(offset - old);
        let moves = points
            .iter()
            .filter_map(|&v| mesh.vertices.get(v as usize).map(|&p| (v, p + d)))
            .collect();
        if self.pen_commit(
            PenEdit::MovePoints {
                item,
                points: moves,
                label: "Topology Pen Offset".into(),
            },
            "Topology Pen: moved by Offset",
        ) && let Some(l) = self.editor.session.live.as_mut()
        {
            l.params.topology_pen.offset = offset;
        }
    }

    // ------------------------------------------------------ drawing

    /// The pen's overlays: the drag in progress, the weld highlight,
    /// Fill's would-be polygon and the falloff's points and edges.
    pub(super) fn draw_topology_pen(&self, painter: &Painter, hover: Option<Pos2>, r: Rect) {
        let stroke = Stroke::new(2_f32, palette::selection());
        let project = |p: DVec3| self.camera.project(p, r).map(|(q, _)| q);
        let o = self.pen_options();
        if let Some(at) = self.pen.weld_at.and_then(project) {
            painter.circle_stroke(at, 7., Stroke::new(2_f32, palette::snap_target()));
        }
        match &self.pen.drag {
            Some(PenDrag::Split { at, .. }) | Some(PenDrag::Duplicate { at, .. }) => {
                if let (Some(a), Some(h)) = (project(*at), hover) {
                    painter.line_segment([a, h], stroke);
                }
            }
            Some(PenDrag::AddLoop { t, ring, joins, .. }) => {
                let Some(item) = self.editor.scene.active() else {
                    return;
                };
                let Some(world) = self.pen_world(item.id) else {
                    return;
                };
                let cut: Vec<Option<Pos2>> = ring
                    .iter()
                    .map(|&(x, y)| {
                        let (px, py) = (
                            item.mesh.vertices.get(x as usize)?,
                            item.mesh.vertices.get(y as usize)?,
                        );
                        project(world.transform_point3(px.lerp(*py, *t)))
                    })
                    .collect();
                for [i, j] in joins {
                    if let (Some(Some(a)), Some(Some(b))) = (cut.get(*i), cut.get(*j)) {
                        painter.line_segment([*a, *b], stroke);
                    }
                }
            }
            _ => {}
        }
        if self.navigation.is_some() {
            return;
        }
        let Some(h) = hover else {
            return;
        };
        if o.mode == PenMode::Fill {
            let ring = self.pen_fill_preview(h, r);
            for (a, b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
                if let (Some(qa), Some(qb)) = (project(*a), project(*b)) {
                    painter.line_segment([qa, qb], Stroke::new(2_f32, palette::rollover()));
                }
            }
        }
        // TP-21 Show Vertex / Show Edge: what a Move under the pointer
        // would carry with the falloff.
        if o.radius > 0. && o.mode == PenMode::Move && (o.show_vertex || o.show_edge) {
            self.draw_pen_falloff(painter, h, r, o);
        }
    }

    /// Fill's would-be polygon under the pointer (world points).
    fn pen_fill_preview(&self, h: Pos2, r: Rect) -> Vec<DVec3> {
        let revision = self.editor.revision;
        if let Some((p, rev, ring)) = self.pen.fill_cache.borrow().as_ref()
            && *p == h
            && *rev == revision
        {
            return ring.clone().unwrap_or_default();
        }
        let o = self.pen_options();
        let ring = (|| {
            let item = self.editor.scene.active()?;
            let world = self.pen_world(item.id)?;
            let local = world.inverse();
            let (origin, dir) = self.camera.ray(h, r);
            let click = self
                .editor
                .background_hit(origin, dir)
                .or_else(|| self.work_point(h, r, self.actual_plane(), self.workplane_center()))?;
            let toward = self.camera.basis().0;
            let ring = tp::fill_ring(
                &item.mesh,
                local.transform_point3(click),
                local.transform_vector3(toward),
                o.fill_range,
                o.quad_only,
            )?;
            Some(
                ring.iter()
                    .map(|&v| world.transform_point3(item.mesh.vertices[v as usize]))
                    .collect::<Vec<_>>(),
            )
        })();
        *self.pen.fill_cache.borrow_mut() = Some((h, revision, ring.clone()));
        ring.unwrap_or_default()
    }

    fn draw_pen_falloff(&self, painter: &Painter, h: Pos2, r: Rect, o: TopologyPenOptions) {
        let Some(pick) = self.tweak_pick(h, r) else {
            return;
        };
        let Some(item) = self.editor.scene.item(pick.grab.item) else {
            return;
        };
        let Some(world) = self.pen_world(item.id) else {
            return;
        };
        let local = world.inverse();
        let scale = local.transform_vector3(DVec3::X).length();
        let weights = tp::falloff_weights(
            &item.mesh,
            local.transform_point3(pick.at),
            o.radius * scale,
            o.shape,
        );
        let color = palette::tweak_range();
        let project = |v: u32| {
            self.camera
                .project(world.transform_point3(item.mesh.vertices[v as usize]), r)
                .map(|(q, _)| q)
        };
        if o.show_edge {
            let inside: std::collections::BTreeSet<u32> = weights.iter().map(|w| w.0).collect();
            for (a, b) in item.mesh.edges() {
                if inside.contains(&a)
                    && inside.contains(&b)
                    && let (Some(qa), Some(qb)) = (project(a), project(b))
                {
                    painter.line_segment([qa, qb], Stroke::new(1_f32, color.gamma_multiply(0.6)));
                }
            }
        }
        if o.show_vertex {
            for (v, w) in weights {
                if let Some(q) = project(v) {
                    painter.circle_filled(q, 1.5 + 2. * w as f32, color);
                }
            }
        }
    }
}

// ------------------------------------------------------------ the form

/// Topology Pen's Tool Properties (TP-10, TP-21..24): Mode and Edge Loop,
/// then each mode's own values (greyed when another mode is on, UIC-92,
/// UIC-117), Falloff, Snapping and Options. Rows for options not built yet
/// are left out (UIC-117 exception): Slip UVs, Backface, Raycast Inactive
/// Surfaces.
pub(super) fn pen_form(p: &Parameters, f: &mut tf::ToolForm<Parameters>) {
    use crate::form::Unit;
    use live_fields::{check, dist, pick, plain, whole};
    let o = p.topology_pen;
    let modes: Vec<_> = PenMode::ALL.iter().map(|&m| (m, m.label())).collect();
    f.field(
        pick(
            "pen_mode",
            "Mode",
            &modes,
            |p| p.topology_pen.mode,
            |p, m| p.topology_pen.mode = m,
        )
        .tip("What a plain left-drag does; the modifier gestures work as well"),
    )
    .field(
        check(
            "pen_edge_loop",
            "Edge Loop",
            |p| p.topology_pen.edge_loop,
            |p, on| p.topology_pen.edge_loop = on,
        )
        .tip("Edges act on their whole loop (Move, Duplicate, Remove, Smoothing)"),
    );
    let is = |m: PenMode| o.mode == m;
    f.field(
        check(
            "pen_keep_vertices",
            "Keep Vertices",
            |p| p.topology_pen.keep_vertices,
            |p, on| p.topology_pen.keep_vertices = on,
        )
        .tip("Remove: keep points a removed edge leaves in only two edges")
        .enabled_if(is(PenMode::Remove), "Only Remove uses it"),
    )
    .field(
        tf::percent(
            "pen_position",
            "Position",
            " %",
            |p: &Parameters| p.topology_pen.position,
            |p, v| {
                if !(0. ..=100.).contains(&v) {
                    return Err("Position is from 0 % to 100 %".into());
                }
                p.topology_pen.position = v / 100.;
                Ok(())
            },
        )
        .tip("Add Loop: where the last loop went across its ring")
        .enabled_if(is(PenMode::AddLoop), "Only Add Loop uses it"),
    )
    .field(
        check(
            "pen_split_middle",
            "Split at the Middle",
            |p| p.topology_pen.split_middle,
            |p, on| p.topology_pen.split_middle = on,
        )
        .tip("Add Loop: always cut at 50 %")
        .enabled_if(is(PenMode::AddLoop), "Only Add Loop uses it"),
    )
    .field(
        plain(
            "pen_fill_range",
            "Range",
            |p| p.topology_pen.fill_range,
            |p, v| p.topology_pen.fill_range = v.max(0.),
        )
        .tip("Fill: how far to look for points, in average edge lengths")
        .enabled_if(is(PenMode::Fill), "Only Fill uses it"),
    )
    .field(
        check(
            "pen_quad_only",
            "Quad Only",
            |p| p.topology_pen.quad_only,
            |p, on| p.topology_pen.quad_only = on,
        )
        .tip("Fill: only quads; off, a triangle where no quad fits")
        .enabled_if(is(PenMode::Fill), "Only Fill uses it"),
    );
    let smoothing = (
        is(PenMode::Smoothing),
        "Smoothing uses it (and Shift+Ctrl drags)",
    );
    f.field(
        plain(
            "pen_strength",
            "Strength",
            |p| p.topology_pen.strength,
            |p, v| p.topology_pen.strength = v.clamp(0., 1.),
        )
        .enabled_if(smoothing.0, smoothing.1),
    )
    .field(
        whole(
            "pen_iterations",
            "Iterations",
            (1., 100.),
            "Iterations is a whole number from 1 to 100",
            |p| p.topology_pen.iterations as f64,
            |p, n| p.topology_pen.iterations = n,
        )
        .enabled_if(smoothing.0, smoothing.1),
    )
    .field(
        check(
            "pen_lock_boundary",
            "Lock Boundary",
            |p| p.topology_pen.lock_boundary,
            |p, on| p.topology_pen.lock_boundary = on,
        )
        .enabled_if(smoothing.0, smoothing.1),
    )
    .field(
        check(
            "pen_lock_corner",
            "Lock Corner",
            |p| p.topology_pen.lock_corner,
            |p, on| p.topology_pen.lock_corner = on,
        )
        .enabled_if(smoothing.0, smoothing.1),
    );
    // Falloff (TP-21): Radius, Use Connectivity and Shape drive Move
    // through Tweak's options; Smoothing reads them too.
    f.separator()
        .field(
            dist(
                "pen_radius",
                "Radius",
                |p| p.tweak_options.range,
                |p, v| p.tweak_options.range = if v.is_finite() { v.max(0.) } else { 0. },
            )
            .tip("0 moves just the element (Smoothing: the whole mesh); right-drag empty space sets it"),
        )
        .field(check(
            "pen_use_connectivity",
            "Use Connectivity",
            |p| p.tweak_options.connect == operandi_core::TweakConnect::Connectivity,
            |p, on| {
                p.tweak_options.connect = if on {
                    operandi_core::TweakConnect::Connectivity
                } else {
                    operandi_core::TweakConnect::Ignore
                }
            },
        ));
    let shapes = falloff_panel::shape_fields(
        "pen_shape",
        falloff_panel::SHAPE,
        p.tweak_options.shape,
        |p: &Parameters| p.tweak_options.shape,
        |p, s| p.tweak_options.shape = s,
    );
    f.fields_from(shapes);
    f.field(check(
        "pen_show_vertex",
        "Show Vertex",
        |p| p.topology_pen.show_vertex,
        |p, on| p.topology_pen.show_vertex = on,
    ))
    .field(check(
        "pen_show_edge",
        "Show Edge",
        |p| p.topology_pen.show_edge,
        |p, on| p.topology_pen.show_edge = on,
    ));
    // Snapping (TP-30).
    f.separator()
        .field(
            check(
                "pen_inner_snap",
                "Inner Snap",
                |p| p.topology_pen.inner_snap,
                |p, on| p.topology_pen.inner_snap = on,
            )
            .tip("Interior points snap and weld too (off: border to border only)"),
        )
        .field(
            check(
                "pen_copy_uvs",
                "Copy Discontinuous UVs",
                |p| p.topology_pen.copy_uvs,
                |p, on| p.topology_pen.copy_uvs = on,
            )
            .tip("After a weld the dragged point's UVs win (off: the target's)"),
        );
    // Options (TP-24).
    f.separator();
    let offsets = tf::vector(
        "pen_offset",
        "Offset",
        Unit::Distance,
        |p: &Parameters| p.topology_pen.offset,
        |p, a, v| {
            p.topology_pen.offset[a] = v;
            Ok(())
        },
    );
    f.fields_from(
        offsets
            .into_iter()
            .map(|x| {
                x.suffix(" m")
                    .command(|app, p: &Parameters| app.pen_offset_typed(p.topology_pen.offset))
            })
            .collect(),
    );
    f.field(
        check(
            "pen_edge_slide",
            "Edge Slide",
            |p| p.topology_pen.edge_slide,
            |p, on| p.topology_pen.edge_slide = on,
        )
        .tip("Move drags slide points along their edges (as Ctrl+drag does)"),
    )
    .note("Left-drag: the Mode. Right-drag an edge: move its loop; right-drag empty space: Radius. Middle-drag: Split. Shift: Duplicate. Ctrl: Slide, or a new quad in empty space. Shift+Ctrl: Smoothing.");
}
