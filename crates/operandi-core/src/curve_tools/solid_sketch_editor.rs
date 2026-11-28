//! The Solid Sketch tool session (spec `solid-sketch` v0.3, sections 5, 7
//! and 8): clicks and drags edit the node tree, and the mesh rebuilds live.
//!
//! - The session keeps the mesh as it was at activation (minus the old
//!   sketch's polygons when it was picked up again) and appends the cage
//!   to it after every edit. The sketch only ever owns the polygons it made.
//! - Every edit is its own step inside the tool: Undo takes back the last
//!   one and keeps the tool (spec 7, [OP]); with none left, Undo closes the
//!   tool like other tools (MU-MID-02). A drag is one step: its frames send
//!   `continuing` edits that join the first.
//! - Drop records the whole session as one undo step, "Solid Sketch".
//!   Cancel puts the scene back.
//! - Node positions are kept in the mesh item's local space; commands that
//!   come from the viewport (Add, Insert, Translate, Rotate) are in world
//!   space and mapped here.
use crate::solid_sketch::{CLICK_SIZE, SketchOptions, SolidSketch};
use crate::{Command, Editor, Mesh, Result, Scene};
use glam::{DMat4, DQuat, DVec3};

/// One edit of the live sketch.
#[derive(Clone, Debug, PartialEq)]
pub enum SketchEdit {
    /// New node at a world point, under the current node (spec 5.1). Size
    /// `None` is a plain click's 100 mm. `up` is the view's work-plane
    /// normal (world); the first node takes it for the root's frame.
    Add {
        at: DVec3,
        size: Option<f64>,
        up: Option<DVec3>,
    },
    /// Ctrl-insert on the link into `child` (spec 5.1).
    Insert {
        child: usize,
        at: DVec3,
        size: Option<f64>,
    },
    /// Make a node current (`None`: nothing current).
    Select(Option<usize>),
    /// Move a node, or its branch, by a world offset.
    Translate {
        index: usize,
        delta: DVec3,
        branch: bool,
    },
    /// Uniform scale of a node's sizes, or of its branch (sizes and spacing).
    Scale {
        index: usize,
        factor: f64,
        branch: bool,
    },
    /// Turn a node and swing its branch (world rotation).
    Rotate { index: usize, turn: DQuat },
    /// Delete a node (its children rejoin its parent) or its branch.
    Delete { index: usize, branch: bool },
    /// The current node's panel values, local space; `twist` in radians.
    SetNode {
        center: DVec3,
        size: DVec3,
        twist: f64,
    },
    /// Per-tool options (Edit Mode, Subdiv Mode and the rest).
    Options(SketchOptions),
}

#[derive(Clone, Debug, PartialEq)]
struct Snapshot {
    sketch: SolidSketch,
    current: Option<usize>,
    options: SketchOptions,
}

/// The open Solid Sketch session.
#[derive(Clone, Debug, PartialEq)]
pub struct SketchSession {
    target: u64,
    /// The scene at activation, for Cancel and the Drop undo step.
    before: Scene,
    /// The target mesh without this sketch's polygons.
    base: Mesh,
    /// Item local → world, and back, fixed for the session.
    world: DMat4,
    local: DMat4,
    sketch: SolidSketch,
    current: Option<usize>,
    options: SketchOptions,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// Picked up stored sketch data at activation (phase 4).
    pub(crate) resumed: bool,
    /// New nodes get symmetry twins: Symmetry is on and matches the
    /// sketch's mirror plane (SS-SYM-08/09).
    twin_new: bool,
}
impl SketchSession {
    pub fn target(&self) -> u64 {
        self.target
    }
    pub fn sketch(&self) -> &SolidSketch {
        &self.sketch
    }
    pub fn current(&self) -> Option<usize> {
        self.current
    }
    pub fn options(&self) -> &SketchOptions {
        &self.options
    }
    /// Item local → world.
    pub fn world(&self) -> DMat4 {
        self.world
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            sketch: self.sketch.clone(),
            current: self.current,
            options: self.options.clone(),
        }
    }
    fn restore(&mut self, s: Snapshot) {
        self.sketch = s.sketch;
        self.current = s.current;
        self.options = s.options;
    }
    /// Symmetry changed with the tool open (tool chain, Jason 2026-11-14):
    /// the activation rule again, so new nodes follow the new setting.
    pub(crate) fn follow_symmetry(
        &mut self,
        mirror: Option<crate::solid_sketch_symmetry::SketchMirror>,
    ) {
        self.twin_new = twins_new_nodes(&mut self.sketch, mirror);
    }
    /// True while new nodes get symmetry twins.
    pub fn mirrors_new_nodes(&self) -> bool {
        self.twin_new
    }
    /// What the status bar says about Symmetry while the tool is open:
    /// a symmetric sketch whose plane isn't the Symmetry setting
    /// (SS-SYM-08), else a link crossing the plane (SS-SYM-04).
    pub fn symmetry_notice(&self) -> Option<String> {
        if let Some(m) = self.sketch.mirror.filter(|_| self.sketch.has_twins())
            && !self.twin_new
        {
            let axis = m.label();
            return Some(format!(
                "Sketch is symmetric about {axis}; its twins stay linked. Turn Symmetry {axis} on to mirror new nodes"
            ));
        }
        self.sketch
            .crossing_links()
            .then(|| crate::solid_sketch_symmetry::CROSSING_MESSAGE.to_string())
    }
    /// World centre of every node.
    pub fn world_centres(&self) -> Vec<DVec3> {
        self.sketch
            .nodes
            .iter()
            .map(|n| self.world.transform_point3(n.center))
            .collect()
    }
}

/// The message for a script or field edit with no node yet (spec 8.15).
pub const NO_NODE: &str = "Solid Sketch has no nodes yet; click in the viewport to make one";

impl Editor {
    pub(crate) fn solid_sketch_command(
        &mut self,
        cmd: Command,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> Result<()> {
        match cmd {
            Command::SolidSketchStart { options, resume } => {
                if self.session.solid_sketch.is_some() {
                    return Ok(());
                }
                self.start_solid_sketch(options, resume)
            }
            Command::SolidSketch { edit, continuing } => self.solid_sketch_edit(edit, continuing),
            Command::Undo | Command::Redo => {
                let undo = matches!(cmd, Command::Undo);
                let session = self
                    .session
                    .solid_sketch
                    .as_mut()
                    .ok_or("Solid Sketch ended")?;
                let label = "Solid Sketch".to_string();
                let effect = if undo {
                    match session.undo.pop() {
                        Some(s) => {
                            let now = session.snapshot();
                            session.redo.push(now);
                            session.restore(s);
                            crate::HistoryEffect::TweakUndone { label }
                        }
                        // Nothing of its own left: Undo closes the tool
                        // (MU-MID-02); it changed nothing to keep.
                        None => {
                            self.cancel_solid_sketch()?;
                            if let Some(report) = report {
                                report.effect = crate::HistoryEffect::ToolClosed { label };
                            }
                            return Ok(());
                        }
                    }
                } else {
                    let s = session
                        .redo
                        .pop()
                        .ok_or("Nothing to redo in Solid Sketch")?;
                    let now = session.snapshot();
                    session.undo.push(now);
                    session.restore(s);
                    crate::HistoryEffect::TweakRedone { label }
                };
                self.rebuild_solid_sketch()?;
                if let Some(report) = report {
                    report.effect = effect;
                }
                Ok(())
            }
            Command::Cancel => {
                self.cancel_solid_sketch()?;
                if let Some(report) = report {
                    report.effect = crate::HistoryEffect::Cancel;
                }
                Ok(())
            }
            Command::Drop => self.drop_solid_sketch(),
            _ => Err("Drop or Cancel Solid Sketch first".into()),
        }
    }

    fn start_solid_sketch(&mut self, options: SketchOptions, resume: bool) -> Result<()> {
        if self.selection_start.is_some() {
            return Err("Finish selection before Solid Sketch".into());
        }
        if self.session.has_active_task() || self.i2_active() {
            return Err("Finish the current task before Solid Sketch".into());
        }
        self.scene.validate()?;
        let item = self
            .scene
            .active()
            .filter(|i| self.scene.shows(i) && i.backdrop.is_none() && i.is_mesh())
            .ok_or("Solid Sketch needs a mesh layer. Add or select a mesh first.")?;
        let world = self.scene.world_matrix(item.id)?;
        if world.determinant().abs() < 1e-18 {
            return Err("Solid Sketch needs an invertible mesh transform".into());
        }
        let local = world.inverse();
        // Remembered per-tool settings (Q18): keep the last values used.
        let mut options = SketchOptions {
            rotate_handles: self.solid_sketch_options.rotate_handles,
            auto_align: self.solid_sketch_options.auto_align,
            auto_merge: self.solid_sketch_options.auto_merge,
            negative: self.solid_sketch_options.negative,
            ..options
        };
        let stored = crate::solid_sketch_store::stored(&item.mesh);
        let mirror = crate::solid_sketch_symmetry::SketchMirror::of(self.active_symmetry());
        let (base, mut sketch, current, resumed) = match stored.filter(|_| resume) {
            // Picking it up again rebuilds the whole layer from the nodes
            // (spec 5.5): the layer's other polygons go.
            Some(data) => {
                options.subdiv = data.options.subdiv;
                options.auto_align = data.options.auto_align;
                options.auto_merge = data.options.auto_merge;
                options.negative = data.options.negative;
                options.hidden = data.options.hidden.clone();
                let mut base = Mesh::default();
                base.attributes.solid_sketch = None;
                let current = data.current.filter(|&c| c < data.sketch.len());
                (base, data.sketch, current, true)
            }
            None => {
                let mut base = Mesh::clone(&item.mesh);
                // A new sketch leaves the layer's geometry alone; a
                // stored sketch it didn't pick up stays as plain polygons.
                base.attributes.solid_sketch = None;
                let up = local
                    .transform_vector3(self.workplane_normal())
                    .try_normalize()
                    .unwrap_or(glam::DVec3::Y);
                (base, SolidSketch::new(up), None, false)
            }
        };
        // SS-SYM-08/09: a sketch with twins keeps its plane; one without
        // takes the current Symmetry setting's.
        let twin_new = twins_new_nodes(&mut sketch, mirror);
        self.session.solid_sketch = Some(SketchSession {
            twin_new,
            target: item.id,
            before: self.scene.clone(),
            base,
            world,
            local,
            sketch,
            current,
            options,
            undo: Vec::new(),
            redo: Vec::new(),
            resumed,
        });
        if resumed {
            self.rebuild_solid_sketch()?;
        }
        Ok(())
    }

    /// The scene's work-plane normal in world space.
    fn workplane_normal(&self) -> DVec3 {
        match self.scene.workplane.plane {
            1 => DVec3::X,
            2 => DVec3::Z,
            _ => DVec3::Y,
        }
    }

    fn solid_sketch_edit(&mut self, edit: SketchEdit, continuing: bool) -> Result<()> {
        let session = self
            .session
            .solid_sketch
            .as_ref()
            .ok_or("Activate Solid Sketch first")?;
        let mut next = session.clone();
        let local = next.local;
        let local_point = |p: DVec3| local.transform_point3(p);
        let size = |s: Option<f64>| -> Result<f64> {
            let s = s.unwrap_or(CLICK_SIZE);
            if !s.is_finite() || s < 0. {
                return Err("Size must be 0 or more".into());
            }
            Ok(s)
        };
        let node = |next: &SketchSession, index: usize| -> Result<usize> {
            if next.sketch.is_empty() {
                return Err(NO_NODE.into());
            }
            (index < next.sketch.len())
                .then_some(index)
                .ok_or_else(|| "That Solid Sketch node doesn't exist".into())
        };
        match edit {
            SketchEdit::Add { at, size: s, up } => {
                if next.sketch.is_empty()
                    && let Some(up) = up.and_then(|u| local.transform_vector3(u).try_normalize())
                {
                    next.sketch.up = up;
                }
                let parent = next.current;
                let twin = next.twin_new;
                let index = next
                    .sketch
                    .add_symmetric(local_point(at), size(s)?, parent, twin)?;
                next.current = Some(index);
            }
            SketchEdit::Insert { child, at, size: s } => {
                node(&next, child)?;
                let twin = next.twin_new;
                let index = next
                    .sketch
                    .insert_symmetric(child, local_point(at), size(s)?, twin)?;
                next.current = Some(index);
            }
            SketchEdit::Select(index) => {
                // Typed out of range clamps to the last node (spec 8.5).
                next.current = match index {
                    Some(_) if next.sketch.is_empty() => return Err(NO_NODE.into()),
                    Some(i) => Some(i.min(next.sketch.len() - 1)),
                    None => None,
                };
            }
            SketchEdit::Translate {
                index,
                delta,
                branch,
            } => {
                node(&next, index)?;
                let d = next
                    .sketch
                    .seam_delta(index, local.transform_vector3(delta));
                next.sketch.translate(index, d, branch)?;
                next.sketch.sync_twins(&session.sketch, index);
            }
            SketchEdit::Scale {
                index,
                factor,
                branch,
            } => {
                node(&next, index)?;
                next.sketch.scale(index, factor, branch)?;
                next.sketch.sync_twins(&session.sketch, index);
            }
            SketchEdit::Rotate { index, turn } => {
                node(&next, index)?;
                // A world turn seen in local space.
                let (_, r, _) = local.to_scale_rotation_translation();
                let turn = next.sketch.seam_turn(index, r * turn * r.inverse());
                next.sketch.rotate(index, turn)?;
                next.sketch.sync_twins(&session.sketch, index);
            }
            SketchEdit::Delete { index, branch } => {
                node(&next, index)?;
                // SS-SYM-06: a node's twin goes with it.
                next.current = if branch {
                    next.sketch.delete_branch_symmetric(index)?
                } else {
                    next.sketch.delete_symmetric(index)?
                };
            }
            SketchEdit::SetNode {
                center,
                size,
                twist,
            } => {
                let index = next.current.ok_or(NO_NODE)?;
                let n = &mut next.sketch.nodes[index];
                if !center.is_finite() || !size.is_finite() || !twist.is_finite() {
                    return Err("Solid Sketch values must be finite".into());
                }
                if size.min_element() < 0. {
                    return Err("Size must be 0 or more".into());
                }
                n.center = center;
                // Typed 0 clamps to a small minimum (spec 8.6).
                n.size = size.max(DVec3::splat(crate::solid_sketch::MIN_SIZE));
                n.twist = twist;
                next.sketch.sync_twins(&session.sketch, index);
            }
            SketchEdit::Options(options) => {
                next.options = options;
            }
        }
        next.sketch.validate()?;
        let changed = next.sketch != session.sketch
            || next.current != session.current
            || next.options != session.options;
        if !changed {
            return Ok(());
        }
        // A drag's later frames join its first step.
        if !(continuing && !next.undo.is_empty()) {
            next.undo.push(session.snapshot());
        }
        next.redo.clear();
        let previous = self.session.solid_sketch.replace(next);
        if let Err(e) = self.rebuild_solid_sketch() {
            self.session.solid_sketch = previous;
            return Err(e);
        }
        let options = self
            .session
            .solid_sketch
            .as_ref()
            .map(|s| s.options.clone());
        if let Some(o) = options {
            self.solid_sketch_options = o;
        }
        Ok(())
    }

    /// Puts base + cage into the target mesh of `self.scene`.
    fn rebuild_solid_sketch(&mut self) -> Result<()> {
        let session = self
            .session
            .solid_sketch
            .as_ref()
            .ok_or("Solid Sketch ended")?;
        let cage = session.sketch.cage(&session.options)?;
        let mut mesh = session.base.clone();
        crate::append_geometry(&mut mesh, cage)?;
        let target = session.target;
        let item = self
            .scene
            .item_mut(target)
            .ok_or("The Solid Sketch mesh is gone")?;
        item.mesh = mesh.into();
        Ok(())
    }

    fn cancel_solid_sketch(&mut self) -> Result<()> {
        let session = self
            .session
            .solid_sketch
            .take()
            .ok_or("Solid Sketch ended")?;
        self.scene = session.before;
        Ok(())
    }

    fn drop_solid_sketch(&mut self) -> Result<()> {
        let session = self
            .session
            .solid_sketch
            .as_ref()
            .ok_or("Solid Sketch ended")?;
        let target = session.target;
        let data = crate::solid_sketch_store::StoredSketch {
            sketch: session.sketch.clone(),
            options: session.options.clone(),
            current: session.current,
        };
        let keep = !session.sketch.is_empty();
        let resumed = session.resumed;
        let mut candidate = self.scene.clone();
        if let Some(item) = candidate.item_mut(target) {
            let mut mesh = Mesh::clone(&item.mesh);
            if keep {
                mesh.attributes.solid_sketch = Some(Box::new(data));
            } else if resumed {
                // The picked-up sketch lost all its nodes.
                mesh.attributes.solid_sketch = None;
            }
            item.mesh = mesh.into();
        }
        candidate.validate()?;
        let session = self
            .session
            .solid_sketch
            .take()
            .ok_or("Solid Sketch ended")?;
        if candidate == session.before {
            self.scene = candidate;
            return Ok(());
        }
        self.scene = candidate;
        self.record("Solid Sketch".into(), session.before);
        Ok(())
    }
}

/// SS-SYM-08/09: a sketch with twins keeps its plane, and new nodes get
/// twins only while Symmetry matches it; one without twins takes the
/// Symmetry setting's plane.
fn twins_new_nodes(
    sketch: &mut SolidSketch,
    mirror: Option<crate::solid_sketch_symmetry::SketchMirror>,
) -> bool {
    match sketch.mirror.filter(|_| sketch.has_twins()) {
        Some(stored) => mirror.is_some_and(|m| m.same_as(stored)),
        None => {
            sketch.mirror = mirror;
            mirror.is_some()
        }
    }
}
