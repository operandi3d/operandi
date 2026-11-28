//! Solid Sketch in the app (spec `solid-sketch` v0.3 sections 2, 4, 5, 8):
//! starting it (button, menu, `tool.set`), the pick-up question, viewport
//! clicks and drags, the node handles, the overlay and Tool Properties.
//!
//! The tool's state lives in the core session (`ToolSession::solid_sketch`);
//! every gesture ends in `Command::SolidSketch` edits. A drag's first edit
//! is a step of its own inside the tool and its later frames join it
//! (`continuing`), so Undo takes back one drag (spec 7).
//!
//! Mouse map (spec 5.1), Edit Mode Create / Move / Scale:
//! - left click / drag in empty space: new node under the current one; the
//!   drag sizes it uniformly (Q5: half-size = pointer distance on the plane);
//! - left press on a node: make it current; dragging moves it (Q4: at once),
//!   Shift-drag moves its branch;
//! - Ctrl press on a link inserts a node there, Ctrl press on a node inserts
//!   on its first link out (a leaf: a new child); the drag sizes it;
//! - right drag: scale the current node (Shift: its branch); a right click
//!   that doesn't move opens the viewport menu (UIC-60);
//! - middle click on a node: delete it (Shift: its branch);
//! - Edit Mode Destroy: left click on a node deletes it; empty space does
//!   nothing (Q8, provisional).
//!
//! Handles on the current node (spec 5.2; #45, Jason 2026-11-27): red,
//! green and blue squares = Size X, Y and Z; a dark green dot beyond the
//! green square = Twist, dragged left and right; rings rotate the node's branch only with Rotate
//! Handles on (usual axis colours: red about X, green about Y, blue about
//! Z; Jason chose these over the documented reverse, Q12).
use super::*;
use crate::form::Unit;
use crate::tool_form::{self as tf, ToolFields, ToolForm};
use operandi_core::SketchEdit;
use operandi_core::solid_sketch::{EditMode, MIN_SIZE, SketchOptions, SubdivMode};
use theme::viewport as palette;

/// Pick distance for node dots, handle squares and links, in points.
const HIT: f32 = 8.;
/// Pointer travel below this is a click, not a drag.
const CLICK_SLOP: f32 = 3.;
/// Rings sit a little outside the node's size, so they don't cover the
/// squares.
const RING_SCALE: f64 = 1.35;
/// Right-drag scale: ×e per 100 points to the right.
const SCALE_RATE: f64 = 0.01;
/// The Twist dot sits this far (points) beyond the green square, along
/// the same screen direction (provisional, #45).
const TWIST_GAP: f32 = 18.;
/// Twist drag: radians per point dragged right (half a degree; provisional,
/// #45).
const TWIST_RATE: f64 = std::f64::consts::PI / 360.;

/// What a press on the sketch grabbed.
#[derive(Clone, Debug, PartialEq)]
pub enum SketchGrab {
    /// A node made by the press; the drag sizes it uniformly.
    Create { index: usize, center: DVec3 },
    /// Moving a node (or its branch) on the plane through it parallel to
    /// the work plane; `last` is the plane point already applied.
    Move {
        index: usize,
        branch: bool,
        plane: u8,
        last: DVec3,
        /// The node's centre less the pointer's plane point at the press:
        /// the node goes where the pointer puts it plus this, then snaps
        /// (SOLID-SKETCH-SPEC 5.3).
        offset: DVec3,
    },
    /// Right drag: uniform scale; `applied` is the factor so far.
    Scale {
        index: usize,
        branch: bool,
        applied: f64,
    },
    /// A Size square: `axis` 0 = X, 2 = Z, along `dir` (world).
    Size {
        axis: usize,
        center: DVec3,
        dir: DVec3,
    },
    /// The Twist dot: left and right drags turn the node (#45).
    Twist { start: f64, from_x: f32 },
    /// A rotate ring about world `axis` through `center`; `last` is the
    /// angle applied so far.
    Ring {
        index: usize,
        axis: DVec3,
        center: DVec3,
        from: DVec3,
        last: f64,
    },
    /// A press that only waits for its release (right click for the menu).
    Click,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SketchDrag {
    pub grab: SketchGrab,
    /// The drag has sent its first edit, so the next ones join it.
    pub started: bool,
}

/// A handle of the current node.
#[derive(Clone, Copy, Debug, PartialEq)]
enum HandleHit {
    SizeX,
    SizeY,
    SizeZ,
    Twist,
    /// Ring about frame axis 0 = X, 1 = Y, 2 = Z.
    Ring(usize),
}

/// The pick-up question waiting for an answer (spec 5.5).
pub(super) struct SketchAsk {
    preset: bool,
    form: form::Dialog,
}

/// Where the remembered answer is kept (dialog memory, UIC-95).
const ASK_MEMORY: &str = "Solid Sketch Pick Up";
fn ask_rows() -> Vec<form::Row> {
    vec![
        form::Row::check("remember", "Don't ask again", false)
            .tip("Answer the same way next time (kept with your dialog settings)"),
        form::Row::choice("answer", "Answer", &["Yes", "No"], 0),
    ]
}

/// The options a start uses: the `SS Default` preset (button and menu) or
/// the bare tool (`tool.set solid.sketch on`).
pub(super) fn start_options(preset: bool) -> SketchOptions {
    if preset {
        SketchOptions::ss_default()
    } else {
        SketchOptions::default()
    }
}

/// SNH-Q02: how near (screen points) a snap target must be to pull a new
/// node onto it.
const CREATE_SNAP_PX: f32 = 8.;

impl Operandi {
    /// Starts Solid Sketch; asks first when the mesh holds a sketch.
    pub(super) fn start_solid_sketch(&mut self, preset: bool) {
        if self.editor.session.solid_sketch.is_some() {
            // The tool's own button drops it (one rule for every tool).
            self.run(Command::Drop);
            return;
        }
        if self.has_modal() {
            self.status = "Close the current dialog before Solid Sketch".into();
            return;
        }
        if let Some(extra) = operandi_core::solid_sketch_store::resume_question(&self.editor) {
            let remembered = self.dialog_memory.values(ASK_MEMORY, &ask_rows());
            if remembered.bool("remember") {
                self.begin_solid_sketch(preset, remembered.choice("answer") == 0);
                return;
            }
            let mut message = "This mesh has Solid Sketch data. Pick it up again?".to_string();
            if extra > 0 {
                message.push_str(&format!(
                    "\n\nYes rebuilds the layer from its nodes: the other {extra} polygon{} in this layer will be removed. No starts a separate new sketch and leaves the layer as it is.",
                    if extra == 1 { "" } else { "s" }
                ));
            } else {
                message
                    .push_str("\n\nNo starts a separate new sketch and leaves the layer as it is.");
            }
            let mut rows = vec![form::Row::message(&message)];
            rows.push(ask_rows().remove(0));
            self.solid_sketch_ask = Some(SketchAsk {
                preset,
                form: form::Dialog::with_defaults("Solid Sketch", rows).asking(),
            });
            return;
        }
        self.begin_solid_sketch(preset, false);
    }

    fn begin_solid_sketch(&mut self, preset: bool, resume: bool) {
        let started = self.run_ok(Command::SolidSketchStart {
            options: start_options(preset),
            resume,
        });
        if started {
            self.clear_fields();
            self.tool_properties_open = true;
            self.status = "Solid Sketch · click to add nodes · drag to size".into();
        }
    }

    /// One frame of the pick-up question.
    pub(super) fn solid_sketch_dialog(&mut self, ctx: &Context) {
        let Some(mut ask) = self.solid_sketch_ask.take() else {
            return;
        };
        let answer = match self.show_form(ctx, &mut ask.form) {
            form::Outcome::Ok => Some(true),
            form::Outcome::No => Some(false),
            form::Outcome::Cancel => {
                self.status = "Solid Sketch not started".into();
                return;
            }
            _ => None,
        };
        let Some(yes) = answer else {
            self.solid_sketch_ask = Some(ask);
            return;
        };
        if ask.form.values.bool("remember") {
            let rows = ask_rows();
            let mut values = form::Values::defaults(&rows);
            values.set("remember", form::Value::Bool(true));
            values.set("answer", form::Value::Choice(if yes { 0 } else { 1 }));
            self.dialog_memory.remember(ASK_MEMORY, &rows, &values);
            let _ = self
                .dialog_memory
                .save(&self.environment.paths().dialog_values());
        }
        self.begin_solid_sketch(ask.preset, yes);
    }

    /// World centres of the live sketch's nodes.
    fn sketch_centres(&self) -> Vec<DVec3> {
        self.editor
            .session
            .solid_sketch
            .as_ref()
            .map(|s| s.world_centres())
            .unwrap_or_default()
    }

    /// SOLID-SKETCH-SPEC 5.3: a node placed or moved (`moving`, the node
    /// being dragged) snapped to the active targets: other items' geometry
    /// and the sketch's other nodes. The sketch's own mesh is left out, so
    /// a node never snaps onto the skin being rebuilt around it.
    fn sketch_snap(&mut self, at: DVec3, moving: Option<usize>, r: Rect) -> Option<DVec3> {
        let session = self.editor.session.solid_sketch.as_ref()?;
        let target = session.target();
        let centres = self.sketch_centres();
        // Node centres aren't mesh points: nothing moving is a mesh target
        // (a node dragged onto a point stays on it), and the dragged node
        // is left out of `own`.
        let own: Vec<DVec3> = centres
            .iter()
            .enumerate()
            .filter(|&(i, _)| Some(i) != moving)
            .map(|(_, &c)| c)
            .collect();
        self.snap_draw_point_skipping(at, &[], &own, None, Some(target), r)
    }

    /// SNH-Q02 (answered): where a new node lands. With Symmetry on, the
    /// seam pulls it onto the plane within Symmetry Seam Snap (8 px);
    /// snap targets pull within [`CREATE_SNAP_PX`]. When both are in
    /// range the nearer on screen wins, the seam on a tie.
    fn sketch_place(&mut self, at: DVec3, r: Rect) -> DVec3 {
        let screen = |a: &Self, p: DVec3| a.camera.project(p, r).map(|q| q.0);
        let Some(here) = screen(self, at) else {
            return at;
        };
        let distance =
            |a: &Self, p: DVec3| screen(a, p).map_or(f32::INFINITY, |q| q.distance(here));
        let seam = self.symmetry_seam_snap(at, r);
        let seam = (seam != at).then_some(seam);
        let target = self
            .sketch_snap(at, None, r)
            .filter(|&t| distance(self, t) <= CREATE_SNAP_PX);
        match (seam, target) {
            (Some(s), Some(t)) => {
                if distance(self, s) <= distance(self, t) {
                    s
                } else {
                    t
                }
            }
            (Some(s), None) => s,
            (None, Some(t)) => t,
            (None, None) => at,
        }
    }

    /// The node whose dot is under `pos`, nearest first.
    fn sketch_node_at(&self, pos: Pos2, r: Rect) -> Option<usize> {
        self.sketch_centres()
            .iter()
            .enumerate()
            .filter_map(|(i, &p)| self.camera.project(p, r).map(|(q, _)| (i, q.distance(pos))))
            .filter(|&(_, d)| d <= HIT)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// The link (by its child) under `pos`, and the world point on it
    /// nearest the pointer's ray.
    fn sketch_link_at(&self, pos: Pos2, r: Rect) -> Option<(usize, DVec3)> {
        let session = self.editor.session.solid_sketch.as_ref()?;
        let centres = session.world_centres();
        let (o, d) = self.camera.ray(pos, r);
        let mut best: Option<(usize, DVec3, f32)> = None;
        for (child, node) in session.sketch().nodes.iter().enumerate() {
            let Some(parent) = node.parent else { continue };
            let (a, b) = (centres[parent], centres[child]);
            let point = closest_on_segment(o, d, a, b);
            let Some((q, _)) = self.camera.project(point, r) else {
                continue;
            };
            let dist = q.distance(pos);
            if dist <= HIT && best.is_none_or(|(_, _, bd)| dist < bd) {
                best = Some((child, point, dist));
            }
        }
        best.map(|(c, p, _)| (c, p))
    }

    /// The current node's frame in world space: centre, axes (unit) and
    /// half-sizes.
    pub(super) fn sketch_current_frame(&self) -> Option<(usize, DVec3, [DVec3; 3], DVec3)> {
        let session = self.editor.session.solid_sketch.as_ref()?;
        let index = session.current()?;
        let frame = session.sketch().frames(session.options().auto_align)[index];
        let world = session.world();
        let node = &session.sketch().nodes[index];
        let axes = [frame.x, frame.y, frame.z].map(|a| world.transform_vector3(a));
        let sizes = DVec3::new(
            axes[0].length() * node.size.x,
            axes[1].length() * node.size.y,
            axes[2].length() * node.size.z,
        );
        Some((
            index,
            world.transform_point3(node.center),
            axes.map(|a| a.normalize_or_zero()),
            sizes,
        ))
    }

    /// The ring about frame axis `k`, as world points.
    fn sketch_ring(center: DVec3, axes: &[DVec3; 3], sizes: DVec3, k: usize) -> Vec<DVec3> {
        let (u, v) = ((k + 1) % 3, (k + 2) % 3);
        let (ru, rv) = (sizes[u] * RING_SCALE, sizes[v] * RING_SCALE);
        (0..=64)
            .map(|i| {
                let t = i as f64 / 64. * std::f64::consts::TAU;
                center + axes[u] * (ru * t.cos()) + axes[v] * (rv * t.sin())
            })
            .collect()
    }

    /// Where the current node's Twist dot is on screen: TWIST_GAP points
    /// beyond the green square, away from the centre (straight up when the
    /// node's Y is seen end-on).
    pub(super) fn sketch_twist_handle(&self, r: Rect) -> Option<Pos2> {
        let (_, c, axes, s) = self.sketch_current_frame()?;
        let centre = self.camera.project(c, r)?.0;
        let green = self.camera.project(c + axes[1] * s.y, r)?.0;
        let away = green - centre;
        let dir = if away.length() > 1. {
            away.normalized()
        } else {
            vec2(0., -1.)
        };
        Some(green + dir * TWIST_GAP)
    }

    /// Which handle of the current node is under `pos`.
    fn sketch_handle_at(&self, pos: Pos2, r: Rect) -> Option<HandleHit> {
        let (_, c, axes, s) = self.sketch_current_frame()?;
        // The nearest of the centre dot and the squares wins, so a small
        // node's dot stays pickable; the dot is left to the node press.
        let distance = |p: DVec3| {
            self.camera
                .project(p, r)
                .map_or(f32::INFINITY, |(q, _)| q.distance(pos))
        };
        let candidates = [
            (None, distance(c)),
            (Some(HandleHit::SizeX), distance(c + axes[0] * s.x)),
            (Some(HandleHit::SizeY), distance(c + axes[1] * s.y)),
            (Some(HandleHit::SizeZ), distance(c + axes[2] * s.z)),
            (
                Some(HandleHit::Twist),
                self.sketch_twist_handle(r)
                    .map_or(f32::INFINITY, |q| q.distance(pos)),
            ),
        ];
        if let Some((hit, _)) = candidates
            .into_iter()
            .filter(|&(_, d)| d <= HIT)
            .min_by(|a, b| a.1.total_cmp(&b.1))
        {
            return hit;
        }
        let rotate = self
            .editor
            .session
            .solid_sketch
            .as_ref()
            .is_some_and(|s| s.options().rotate_handles);
        if rotate {
            for k in 0..3 {
                let ring: Vec<Pos2> = Self::sketch_ring(c, &axes, s, k)
                    .into_iter()
                    .filter_map(|p| self.camera.project(p, r).map(|(q, _)| q))
                    .collect();
                if ring
                    .windows(2)
                    .any(|w| segment_distance(pos, w[0], w[1]) <= HIT * 0.75)
                {
                    return Some(HandleHit::Ring(k));
                }
            }
        }
        None
    }

    fn sketch_edit(&mut self, edit: SketchEdit, continuing: bool) -> bool {
        let ok = self.run_ok(Command::SolidSketch { edit, continuing });
        if ok {
            self.clear_fields();
        }
        ok
    }

    /// The work plane normal of this view, world.
    fn sketch_plane_normal(&self) -> DVec3 {
        match self.actual_plane() {
            1 => DVec3::X,
            2 => DVec3::Z,
            _ => DVec3::Y,
        }
    }

    /// A press in the viewport while Solid Sketch is on. `None` leaves the
    /// press to the rest of the viewport (navigation, menus).
    pub(super) fn solid_sketch_press(
        &mut self,
        pos: Pos2,
        r: Rect,
        button: PointerButton,
        modifiers: Modifiers,
    ) -> Option<input::GestureKind> {
        let session = self.editor.session.solid_sketch.as_ref()?;
        if input::navigation(modifiers).is_some() || modifiers.mac_cmd {
            return None;
        }
        let destroy = session.options().edit_mode == EditMode::Destroy;
        let current = session.current();
        let shift = modifiers.shift;
        let ctrl = modifiers.ctrl;
        let gesture = |grab: SketchGrab| {
            Some(input::GestureKind::SolidSketch(Box::new(SketchDrag {
                grab,
                started: false,
            })))
        };
        match button {
            PointerButton::Middle => {
                let index = self.sketch_node_at(pos, r)?;
                self.sketch_edit(
                    SketchEdit::Delete {
                        index,
                        branch: shift,
                    },
                    false,
                );
                Some(input::GestureKind::Blocked)
            }
            PointerButton::Secondary => {
                // Right drag scales the node under the pointer, else the
                // current one; a click opens the menu on release.
                let index = self.sketch_node_at(pos, r).or(current);
                match index {
                    Some(index) => gesture(SketchGrab::Scale {
                        index,
                        branch: shift,
                        applied: 1.,
                    }),
                    None => gesture(SketchGrab::Click),
                }
            }
            PointerButton::Primary => {
                if destroy {
                    if let Some(index) = self.sketch_node_at(pos, r) {
                        self.sketch_edit(
                            SketchEdit::Delete {
                                index,
                                branch: shift,
                            },
                            false,
                        );
                    }
                    return Some(input::GestureKind::Blocked);
                }
                if !ctrl && let Some(hit) = self.sketch_handle_at(pos, r) {
                    return self.sketch_handle_press(hit, pos, r).and_then(gesture);
                }
                if ctrl {
                    return self.sketch_insert_press(pos, r).and_then(gesture);
                }
                if let Some(index) = self.sketch_node_at(pos, r) {
                    if current != Some(index) {
                        self.sketch_edit(SketchEdit::Select(Some(index)), false);
                    }
                    let center = self.sketch_centres()[index];
                    let plane = self.actual_plane();
                    let last = self.work_point(pos, r, plane, center).unwrap_or(center);
                    self.snap_session = Default::default();
                    return gesture(SketchGrab::Move {
                        index,
                        branch: shift,
                        plane,
                        last,
                        offset: center - last,
                    });
                }
                // The new node extends the current one: in a view without
                // perspective it keeps that node's depth (placement_center).
                let from = current.and_then(|i| self.sketch_centres().get(i).copied());
                let through = self.placement_center(from);
                let Some(at) = self.work_point(pos, r, self.actual_plane(), through) else {
                    self.status = "Click where the pointer meets the work plane".into();
                    return Some(input::GestureKind::Blocked);
                };
                // SOLID-SKETCH-SPEC 5.3: the new node snaps, then (as for
                // the Pen) SYM3-23: a node placed near the symmetry plane
                // becomes a seam node (SS-SYM-02).
                self.snap_session = Default::default();
                let at = self.sketch_place(at, r);
                let up = self.sketch_plane_normal();
                if !self.sketch_edit(
                    SketchEdit::Add {
                        at,
                        size: None,
                        up: Some(up),
                    },
                    false,
                ) {
                    return Some(input::GestureKind::Blocked);
                }
                let index = self.editor.session.solid_sketch.as_ref()?.current()?;
                Some(input::GestureKind::SolidSketch(Box::new(SketchDrag {
                    grab: SketchGrab::Create { index, center: at },
                    started: true,
                })))
            }
            _ => None,
        }
    }

    /// Ctrl press: insert on the link under the pointer, or on the link
    /// out of the node under it (spec 5.1).
    fn sketch_insert_press(&mut self, pos: Pos2, r: Rect) -> Option<SketchGrab> {
        let session = self.editor.session.solid_sketch.as_ref()?;
        let plane = self.actual_plane();
        let edit = if let Some(node) = self.sketch_node_at(pos, r) {
            let centre = self.sketch_centres()[node];
            let at = self.work_point(pos, r, plane, centre).unwrap_or(centre);
            let child = session
                .sketch()
                .nodes
                .iter()
                .position(|n| n.parent == Some(node));
            match child {
                Some(child) => SketchEdit::Insert {
                    child,
                    at,
                    size: None,
                },
                None => {
                    // A leaf: a new child from it.
                    if session.current() != Some(node) {
                        self.sketch_edit(SketchEdit::Select(Some(node)), false);
                    }
                    SketchEdit::Add {
                        at,
                        size: None,
                        up: None,
                    }
                }
            }
        } else if let Some((child, at)) = self.sketch_link_at(pos, r) {
            SketchEdit::Insert {
                child,
                at,
                size: None,
            }
        } else {
            self.status = "Ctrl-click a link between two nodes to insert a node".into();
            return None;
        };
        let center = match &edit {
            SketchEdit::Insert { at, .. } | SketchEdit::Add { at, .. } => *at,
            _ => DVec3::ZERO,
        };
        if !self.sketch_edit(edit, false) {
            return None;
        }
        let index = self.editor.session.solid_sketch.as_ref()?.current()?;
        Some(SketchGrab::Create { index, center })
    }

    fn sketch_handle_press(&mut self, hit: HandleHit, pos: Pos2, r: Rect) -> Option<SketchGrab> {
        let (index, c, axes, _) = self.sketch_current_frame()?;
        Some(match hit {
            HandleHit::SizeX => SketchGrab::Size {
                axis: 0,
                center: c,
                dir: axes[0],
            },
            HandleHit::SizeY => SketchGrab::Size {
                axis: 1,
                center: c,
                dir: axes[1],
            },
            HandleHit::SizeZ => SketchGrab::Size {
                axis: 2,
                center: c,
                dir: axes[2],
            },
            HandleHit::Twist => {
                let session = self.editor.session.solid_sketch.as_ref()?;
                SketchGrab::Twist {
                    start: session.sketch().nodes[index].twist,
                    from_x: pos.x,
                }
            }
            HandleHit::Ring(k) => {
                let axis = axes[k];
                let from = ray_plane(self.camera.ray(pos, r), c, axis)?;
                SketchGrab::Ring {
                    index,
                    axis,
                    center: c,
                    from,
                    last: 0.,
                }
            }
        })
    }

    /// A pointer move (or the release) of a sketch drag.
    pub(super) fn solid_sketch_drag(
        &mut self,
        drag: &mut SketchDrag,
        start: Pos2,
        pos: Pos2,
        r: Rect,
    ) {
        if pos.distance(start) < CLICK_SLOP && !drag.started {
            return;
        }
        let continuing = drag.started;
        let Some(session) = self.editor.session.solid_sketch.as_ref() else {
            return;
        };
        let current_node = session.current().map(|i| session.sketch().nodes[i].clone());
        let edit = match &mut drag.grab {
            SketchGrab::Create { center, .. } => {
                if pos.distance(start) < CLICK_SLOP {
                    return;
                }
                let plane = self.actual_plane();
                let Some(p) = self.work_point(pos, r, plane, *center) else {
                    return;
                };
                let Some(node) = current_node else { return };
                // World distance to local half-size.
                let scale = operandi_core::scene::uniform_scale(session.world());
                let size = ((p - *center).length() / scale).max(MIN_SIZE);
                SketchEdit::SetNode {
                    center: node.center,
                    size: DVec3::splat(size),
                    twist: node.twist,
                }
            }
            SketchGrab::Move {
                index,
                branch,
                plane,
                last,
                offset,
            } => {
                let Some(p) = self.work_point(pos, r, *plane, *last) else {
                    return;
                };
                *last = p;
                // SOLID-SKETCH-SPEC 5.3: the moved node snaps. (The seam
                // snap stays a placing rule, SS-SYM-02.)
                let Some(&from) = self.sketch_centres().get(*index) else {
                    return;
                };
                let wanted = p + *offset;
                let at = self.sketch_snap(wanted, Some(*index), r).unwrap_or(wanted);
                let delta = at - from;
                if delta == DVec3::ZERO {
                    return;
                }
                SketchEdit::Translate {
                    index: *index,
                    delta,
                    branch: *branch,
                }
            }
            SketchGrab::Scale {
                index,
                branch,
                applied,
            } => {
                let factor = ((pos.x - start.x) as f64 * SCALE_RATE).exp();
                let step = factor / *applied;
                *applied = factor;
                SketchEdit::Scale {
                    index: *index,
                    factor: step,
                    branch: *branch,
                }
            }
            SketchGrab::Size { axis, center, dir } => {
                let Some(node) = current_node else { return };
                let (o, d) = self.camera.ray(pos, r);
                let t = closest_on_line(o, d, *center, *dir);
                let scale = session.world().transform_vector3(*dir).length();
                let local_dir = session.world().inverse().transform_vector3(*dir);
                let per_unit = local_dir.length().max(1e-12);
                let _ = scale;
                let mut size = node.size;
                size[*axis] = (t * per_unit).max(MIN_SIZE);
                SketchEdit::SetNode {
                    center: node.center,
                    size,
                    twist: node.twist,
                }
            }
            SketchGrab::Twist {
                start: twist,
                from_x,
            } => {
                let Some(node) = current_node else { return };
                // #45: right turns one way, left the other.
                SketchEdit::SetNode {
                    center: node.center,
                    size: node.size,
                    twist: *twist + f64::from(pos.x - *from_x) * TWIST_RATE,
                }
            }
            SketchGrab::Ring {
                index,
                axis,
                center,
                from,
                last,
            } => {
                let Some(p) = ray_plane(self.camera.ray(pos, r), *center, *axis) else {
                    return;
                };
                let (a, b) = (*from - *center, p - *center);
                if a.length_squared() < 1e-18 || b.length_squared() < 1e-18 {
                    return;
                }
                let angle = a.cross(b).dot(*axis).atan2(a.dot(b));
                let step = angle - *last;
                *last = angle;
                SketchEdit::Rotate {
                    index: *index,
                    turn: glam::DQuat::from_axis_angle(*axis, step),
                }
            }
            SketchGrab::Click => return,
        };
        if self.sketch_edit(edit, continuing) {
            drag.started = true;
        }
    }

    /// The end of a sketch press: a right click that didn't move opens the
    /// viewport menu (UIC-60).
    pub(super) fn solid_sketch_release(&mut self, drag: &SketchDrag, start: Pos2, end: Pos2) {
        let click = start.distance(end) < CLICK_SLOP && !drag.started;
        if click
            && matches!(drag.grab, SketchGrab::Click | SketchGrab::Scale { .. })
            && !self.mouse_region_click(start, self.viewport_rect)
        {
            self.selection_context_requested = true;
        }
        self.status = self.sketch_status();
    }

    fn sketch_status(&self) -> String {
        let Some(session) = self.editor.session.solid_sketch.as_ref() else {
            return "Ready".into();
        };
        let n = session.sketch().len();
        match session.current() {
            Some(c) => format!(
                "Solid Sketch · {n} node{} · node {c} current",
                if n == 1 { "" } else { "s" }
            ),
            None => format!("Solid Sketch · {n} nodes · click to add a node"),
        }
    }

    /// The overlay: links, node dots (the current one highlighted), and
    /// the current node's handles. Returns a tooltip for a hovered handle
    /// (UIC-05).
    pub(super) fn draw_solid_sketch(
        &self,
        painter: &Painter,
        r: Rect,
        hover: Option<Pos2>,
    ) -> Option<String> {
        let session = self.editor.session.solid_sketch.as_ref()?;
        let centres = session.world_centres();
        let project = |p: DVec3| self.camera.project(p, r).map(|(q, _)| q);
        let link = Stroke::new(1.5_f32, palette::selection().gamma_multiply(0.8));
        for (i, node) in session.sketch().nodes.iter().enumerate() {
            if let (Some(p), Some(a), Some(b)) = (
                node.parent,
                node.parent.and_then(|p| project(centres[p])),
                project(centres[i]),
            ) {
                let _ = p;
                painter.line_segment([a, b], link);
            }
        }
        let current = session.current();
        for (i, &c) in centres.iter().enumerate() {
            let Some(q) = project(c) else { continue };
            if Some(i) == current {
                // UIC-01: the tool highlight marks the current node.
                painter.circle_filled(q, 5., palette::handle_active());
                painter.circle_stroke(q, 5., Stroke::new(1_f32, Color32::BLACK));
            } else {
                painter.circle_filled(q, 4., palette::selection());
            }
        }
        let (_, c, axes, s) = self.sketch_current_frame()?;
        let [x, y, z] = palette::handles();
        // Rings: read-only size guides unless Rotate Handles is on.
        // The usual axis colours (Jason, 2026-11-13, spec Q12): red turns
        // about X, green about Y, blue about Z.
        let rotate = session.options().rotate_handles;
        let hovered = hover.and_then(|h| self.sketch_handle_at(h, r));
        for (k, colour) in [(0, x), (1, y), (2, z)] {
            let ring: Vec<Pos2> = Self::sketch_ring(c, &axes, s, k)
                .into_iter()
                .filter_map(project)
                .collect();
            let hot = hovered == Some(HandleHit::Ring(k));
            let stroke = if rotate {
                Stroke::new(if hot { 2.5_f32 } else { 1.5 }, colour)
            } else {
                Stroke::new(1_f32, colour.gamma_multiply(0.35))
            };
            painter.add(Shape::line(ring, stroke));
        }
        for (hit, at, colour) in [
            (HandleHit::SizeX, c + axes[0] * s.x, x),
            (HandleHit::SizeY, c + axes[1] * s.y, y),
            (HandleHit::SizeZ, c + axes[2] * s.z, z),
        ] {
            let Some(q) = project(at) else { continue };
            let half = if hovered == Some(hit) { 5. } else { 4. };
            painter.rect_filled(Rect::from_center_size(q, vec2(half, half) * 2.), 0., colour);
        }
        // #45: Twist's own dot, a darker green beyond the green square.
        if let Some(q) = self.sketch_twist_handle(r) {
            let dark = Color32::from_rgb(
                (f32::from(y.r()) * 0.55) as u8,
                (f32::from(y.g()) * 0.55) as u8,
                (f32::from(y.b()) * 0.55) as u8,
            );
            let radius = if hovered == Some(HandleHit::Twist) {
                6.
            } else {
                5.
            };
            painter.circle_filled(q, radius, dark);
        }
        hovered.map(|h| {
            match h {
                HandleHit::SizeX => "Size X · drag to stretch the node along its run",
                HandleHit::SizeY => "Size Y · drag to stretch the node across its run",
                HandleHit::SizeZ => "Size Z · drag to stretch the node across its run",
                HandleHit::Twist => "Twist · drag left or right to spin its cross-section",
                HandleHit::Ring(_) => "Rotate · drag to turn the node and swing its branch",
            }
            .to_string()
        })
    }

    /// `tool.attr solid.sketch <arg> <value>` for the names the panel
    /// doesn't show (hidden args) or shows by label (choice tokens).
    /// `None` leaves the attribute to the panel's fields.
    pub(super) fn solid_sketch_attr(
        &mut self,
        attr: &str,
        value: &str,
    ) -> Option<std::result::Result<Option<String>, String>> {
        let session = self.editor.session.solid_sketch.as_ref()?;
        let mut o = session.options().clone();
        let number = |v: &str| numeric::parse_quantity(v, numeric::Quantity::Scalar);
        let query = value == "?";
        let answer: Option<String> = match attr {
            "emod" if query => Some(o.edit_mode.token().into()),
            "emod" => {
                o.edit_mode = EditMode::parse(value)?;
                None
            }
            "subdiv" if query => Some(o.subdiv.token().into()),
            "subdiv" => {
                if session.sketch().is_empty() {
                    return Some(Err(format!(
                        "Subdiv Mode isn't available: {}",
                        operandi_core::solid_sketch_editor::NO_NODE
                    )));
                }
                o.subdiv = SubdivMode::parse(value)?;
                None
            }
            "tension" if query => Some(format!("{}", o.hidden.tension / 100.)),
            "tension" => {
                o.hidden.tension = match operandi_core::command_names::script_percent(value) {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e)),
                };
                None
            }
            "material" | "part" | "uvmap" if query => Some(match attr {
                "material" => o.hidden.material.clone(),
                "part" => o.hidden.part.clone(),
                _ => o.hidden.uvmap.clone(),
            }),
            "material" => {
                o.hidden.material = value.into();
                None
            }
            "part" => {
                o.hidden.part = value.into();
                None
            }
            "uvmap" => {
                o.hidden.uvmap = value.into();
                None
            }
            "uvproj" if query => Some(o.hidden.uv_projection.token()),
            "uvproj" => {
                o.hidden.uv_projection =
                    match operandi_core::solid_sketch::UvProjection::parse(value) {
                        Some(p) => p,
                        None => return Some(Err("uvProj takes default or a whole number".into())),
                    };
                None
            }
            "cenu" | "cenv" if query => Some(format!(
                "{}",
                if attr == "cenu" {
                    o.hidden.cen_u
                } else {
                    o.hidden.cen_v
                }
            )),
            "cenu" | "cenv" => {
                let v = match number(value) {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e)),
                };
                if attr == "cenu" {
                    o.hidden.cen_u = v;
                } else {
                    o.hidden.cen_v = v;
                }
                None
            }
            "rotu" if query => Some(format!("{}", o.hidden.rot_u)),
            "rotu" => {
                o.hidden.rot_u = match operandi_core::command_names::script_angle(value) {
                    Ok(v) => v.to_radians(),
                    Err(e) => return Some(Err(e)),
                };
                None
            }
            "negative" if query => Some((o.negative as u8).to_string()),
            "negative" => {
                o.negative = match operandi_core::command_args::boolean("Negative", value) {
                    Ok(b) => b,
                    Err(e) => return Some(Err(e)),
                };
                None
            }
            _ => return None,
        };
        if let Some(answer) = answer {
            return Some(Ok(Some(answer)));
        }
        if !self.sketch_edit(SketchEdit::Options(o), false) {
            return Some(Err(self.status.clone()));
        }
        Some(Ok(None))
    }
}

/// Closest point on segment `a`–`b` to the ray `o + t d`.
fn closest_on_segment(o: DVec3, d: DVec3, a: DVec3, b: DVec3) -> DVec3 {
    let u = b - a;
    let w = a - o;
    let (uu, ud, dd) = (u.dot(u), u.dot(d), d.dot(d));
    let denom = uu * dd - ud * ud;
    if uu < 1e-18 {
        return a;
    }
    let s = if denom.abs() < 1e-18 {
        0.
    } else {
        ((ud * d.dot(w)) - dd * u.dot(w)) / denom
    };
    a + u * s.clamp(0., 1.)
}

/// Parameter along the line `c + t dir` (unit `dir`) closest to the ray.
fn closest_on_line(o: DVec3, d: DVec3, c: DVec3, dir: DVec3) -> f64 {
    let w = c - o;
    let (ud, dd) = (dir.dot(d), d.dot(d));
    let denom = dd - ud * ud;
    if denom.abs() < 1e-12 {
        return 0.;
    }
    (ud * d.dot(w) - dd * dir.dot(w)) / denom
}

/// Where the ray meets the plane through `c` with normal `n`.
fn ray_plane((o, d): (DVec3, DVec3), c: DVec3, n: DVec3) -> Option<DVec3> {
    let denom = d.dot(n);
    if denom.abs() < 1e-9 {
        return None;
    }
    let t = (c - o).dot(n) / denom;
    Some(o + d * t)
}

fn segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = if ab.length_sq() > 0. {
        ((p - a).dot(ab) / ab.length_sq()).clamp(0., 1.)
    } else {
        0.
    };
    p.distance(a + ab * t)
}

// ---- Tool Properties (spec 4) ----

/// Solid Sketch's Tool Properties.
pub(super) struct SolidSketchFields;

#[derive(Clone, PartialEq)]
pub(super) struct SketchP {
    options: SketchOptions,
    count: usize,
    current: Option<usize>,
    center: DVec3,
    size: DVec3,
    /// Degrees in the panel; radians in the core and scripts.
    twist: f64,
}

impl ToolFields for SolidSketchFields {
    type P = SketchP;
    fn params(app: &Operandi) -> Option<SketchP> {
        let s = app.editor.session.solid_sketch.as_ref()?;
        let node = s.current().map(|i| s.sketch().nodes[i].clone());
        Some(SketchP {
            options: s.options().clone(),
            count: s.sketch().len(),
            current: s.current(),
            center: node.as_ref().map_or(DVec3::ZERO, |n| n.center),
            size: node.as_ref().map_or(DVec3::ZERO, |n| n.size),
            twist: node.as_ref().map_or(0., |n| n.twist.to_degrees()),
        })
    }
    fn form(_: &Operandi, p: &SketchP) -> ToolForm<SketchP> {
        let mut f = ToolForm::new();
        let nodes = p.count > 0;
        let node = p.current.is_some();
        let no_node = operandi_core::solid_sketch_editor::NO_NODE;
        f.title("Solid Sketch")
            .field(
                tf::choice(
                    "emod",
                    "Edit Mode",
                    &[
                        (EditMode::CreateMoveScale, EditMode::CreateMoveScale.label()),
                        (EditMode::Destroy, EditMode::Destroy.label()),
                    ],
                    |p: &SketchP| p.options.edit_mode,
                    |p, m| p.options.edit_mode = m,
                )
                .tip("Create / Move / Scale: click to add nodes, drag to move and size them · Destroy: click a node to delete it"),
            )
            .field(
                tf::choice(
                    "subdiv",
                    "Subdiv Mode",
                    &SubdivMode::ALL.map(|m| (m, m.label())),
                    |p: &SketchP| p.options.subdiv,
                    |p, m| p.options.subdiv = m,
                )
                .tip("none: plain polygons · Level 1: the cage as subdivision polygons · Level 2: the cage smoothed once (4× the polygons), as subdivision polygons")
                // Spec 4 [seen]: greyed until the first node exists.
                .enabled_if(nodes, no_node),
            )
            .field(
                tf::check(
                    "rotate",
                    "Rotate Handles",
                    |p: &SketchP| p.options.rotate_handles,
                    |p, on| p.options.rotate_handles = on,
                )
                .tip("Rings round the current node turn it and swing its branch"),
            )
            .separator()
            .field(
                tf::number(
                    "current",
                    "Current Node",
                    Unit::Count,
                    |p: &SketchP| p.current.unwrap_or(0) as f64,
                    |p, v| {
                        if !v.is_finite() || v < 0. || v.fract() != 0. {
                            return Err("Current Node must be a whole number 0 or more".into());
                        }
                        p.current = Some(v as usize);
                        Ok(())
                    },
                )
                .tip("The node being edited; nodes are numbered in creation order")
                .enabled_if(nodes, no_node),
            )
            .separator();
        for (axis, key, label) in [(0, "cenX", "Center X"), (1, "cenY", "Y"), (2, "cenZ", "Z")] {
            f.field(
                tf::number(
                    key,
                    label,
                    Unit::Distance,
                    move |p: &SketchP| p.center[axis],
                    move |p, v| {
                        p.center[axis] = v;
                        Ok(())
                    },
                )
                .suffix("m")
                .enabled_if(node, no_node),
            );
        }
        f.separator();
        for (axis, key, label) in [(0, "sizX", "Size X"), (1, "sizY", "Y"), (2, "sizZ", "Z")] {
            f.field(
                tf::number(
                    key,
                    label,
                    Unit::Distance,
                    move |p: &SketchP| p.size[axis],
                    move |p, v| {
                        if v < 0. {
                            return Err("Size must be 0 or more".into());
                        }
                        p.size[axis] = v;
                        Ok(())
                    },
                )
                .suffix("m")
                .tip("Half-size of the node along its own axis: 100 mm makes a 200 mm box")
                .enabled_if(node, no_node),
            );
        }
        f.separator()
            .field(
                tf::number(
                    "bank",
                    "Twist",
                    Unit::Angle,
                    |p: &SketchP| p.twist,
                    |p, v| {
                        p.twist = v;
                        Ok(())
                    },
                )
                .suffix("°")
                .tip("Spins the node's cross-section round its run; neighbours stay put")
                .enabled_if(node, no_node),
            )
            .separator()
            .field(
                tf::check(
                    "align",
                    "Auto Align",
                    |p: &SketchP| p.options.auto_align,
                    |p, on| p.options.auto_align = on,
                )
                .tip("Each node turns to blend with the links in and out of it"),
            )
            .field(
                tf::check(
                    "merge",
                    "Auto Merge",
                    |p: &SketchP| p.options.auto_merge,
                    |p, on| p.options.auto_merge = on,
                )
                .tip("Nodes that overlap their neighbour merge into it instead of pinching"),
            )
            .field(
                tf::check(
                    "negative",
                    "Negative",
                    |p: &SketchP| p.options.negative,
                    |p, on| p.options.negative = on,
                )
                // Spec Q9: what Negative builds isn't known yet.
                .enabled_if(
                    false,
                    "Negative (carving dents) isn't built yet; the value is kept for scripts",
                ),
            )
            .buttons(vec![
                tf::Button::command("Drop", Command::Drop),
                tf::Button::run("Cancel", |app| {
                    app.run(Command::Cancel);
                    app.clear_fields();
                }),
            ]);
        f
    }
    fn publish(app: &mut Operandi, p: SketchP) -> Result<()> {
        let Some(now) = Self::params(app) else {
            return Err("Solid Sketch ended".into());
        };
        let mut candidate = app.editor.clone();
        let mut dispatch = |edit| {
            candidate.dispatch(Command::SolidSketch {
                edit,
                continuing: false,
            })
        };
        if p.options != now.options {
            dispatch(SketchEdit::Options(p.options.clone()))?;
        }
        if p.current != now.current {
            dispatch(SketchEdit::Select(p.current))?;
        } else if p.center != now.center || p.size != now.size || p.twist != now.twist {
            dispatch(SketchEdit::SetNode {
                center: p.center,
                size: p.size,
                twist: p.twist.to_radians(),
            })?;
        }
        app.editor = candidate;
        Ok(())
    }
    fn published(app: &mut Operandi) {
        app.status = app.sketch_status();
    }
}
