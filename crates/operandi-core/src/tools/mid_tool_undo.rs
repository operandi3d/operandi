//! Undo while a tool is active: the tool's activation is an undo step of its
//! own (mid-tool undo spec v1, 2026-11-04: MU-MID-02, MU-MID-03, U1, U4,
//! MU-MID-04).
//!
//! Covers the tools that keep running through Undo/Redo: Move, Rotate,
//! Scale, Transform, the drawn primitives (Cube, Sphere, Cylinder) and the
//! numeric primitive sessions (Cone, Ellipsoid, Torus/Capsule/Arc/Tube).
//!
//! - Activating a tool keeps Redo, as before (not observed; MTU-Q12): Redo
//!   then redoes the undone work with the tool still active.
//! - `History::tool_steps` counts the steps recorded since activation. With
//!   none left, the next Undo closes the tool and changes no geometry
//!   (MU-MID-02, MU-MID-03). A drawn primitive's unfinished preview is part
//!   of that activation step, so one Undo removes it and closes the tool
//!   (U1, plain-drag case).
//! - Redo straight after such an Undo opens the tool again as it was, with
//!   its preview (U4, provisional on panel values).
//! - Leaving the tool (Escape/Cancel, Q/Drop, another tool) clears Redo
//!   (MU-MID-04).
//!
//! Inside a tool session each tweak (a drag, a typed value, a scrub) is a
//! step of the tool's own (Jason, 2026-11-04): Undo takes back the last
//! tweak and keeps the tool, Redo puts it back. Once the tool is dropped the
//! whole session is one undo step. Applies to Move/Rotate/Scale/Transform,
//! the drawn primitives and the numeric primitive sessions; Tweak keeps one
//! document step per drag (TWK-24). `ToolSession::tweaks`.
use crate::{ActiveTask, Command, Editor, IdentityScene, Live, NumericCone, NumericEllipsoid};
use crate::{NumericBridge, NumericShape, NumericSpike, Result, Scene, Tool};

/// A tool's own undo history while it is active: the state before each
/// tweak. Not document state; cleared when the tool closes or records a
/// document step (Apply, Drop).
#[derive(Clone, Default, PartialEq)]
pub struct ToolTweaks {
    undo: Vec<TweakState>,
    redo: Vec<TweakState>,
    /// A tweak is in progress (a drag still held), so further changes join it.
    open: bool,
}
impl ToolTweaks {
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    /// The name of the tweak Undo (or Redo) would take back, when it has
    /// one of its own (MN-05.4's "Reset <tool>").
    pub(crate) fn top_name(&self, redo: bool) -> Option<&str> {
        let stack = if redo { &self.redo } else { &self.undo };
        stack.last().and_then(|s| s.named.as_deref())
    }
    /// How many tweaks Undo can take back.
    pub fn undo_steps(&self) -> usize {
        self.undo.len()
    }
}

#[derive(Clone, PartialEq)]
struct TweakState {
    /// The tweak began with Bevel's Continue: `scene` is where one bevel of
    /// the session ended and the next began (TA-07).
    bevel_start: bool,
    /// The step's own name when it isn't the tool's ("Reset Move",
    /// MN-05.4).
    named: Option<String>,
    session: ReopenSession,
    scene: Scene,
    identities: IdentityScene,
    /// The session's command lines so far (UIC-226): undoing a tweak takes
    /// back the `tool.attr` lines it recorded, redoing puts them back.
    records: crate::StepRecords,
}

/// Commands that adjust the active tool rather than the document.
pub(crate) fn is_tweak_command(cmd: &Command) -> bool {
    matches!(
        cmd,
        Command::Preview(_)
            // MN-05.4: a reset is one tweak, named "Reset <tool>".
            | Command::ResetToolAttributes
            // HM-02: taking up the last drag joins the Shift-drag's tweak.
            | Command::ContinueTransformStep
            | Command::ConfigureBevel(_)
            | Command::ConfigureChamfer(_)
            | Command::ConfigureSmoothShift(_)
            | Command::ConfigureExtrude(_)
            | Command::ConfigureSpike(_)
            | Command::ConfigureBridge(_)
            | Command::ConfigureCone(_)
            | Command::ConfigureEllipsoid(_)
            | Command::ConfigureShape(_)
            | Command::ToolItemChannel { .. }
    )
}

/// A tool closed by Undo of its activation step, kept so Redo can open it
/// again. Not document state.
#[derive(Clone, PartialEq)]
pub struct ToolReopen {
    session: ReopenSession,
    /// What the scene showed when Undo closed the tool (a drawn primitive's
    /// preview), and the matching identities.
    shown: Scene,
    shown_identities: IdentityScene,
    /// The scene right after the close; Redo reopens only from it.
    closed: Scene,
    /// Redo reopens the tool only at the history position where it closed.
    redo_len: usize,
    /// The tool's own undone tweaks, which Redo can put back after it.
    tweaks: ToolTweaks,
}

#[derive(Clone, PartialEq)]
enum ReopenSession {
    Live(Box<Live>),
    Cone(NumericCone),
    Ellipsoid(NumericEllipsoid),
    Shape(NumericShape),
    Spike(NumericSpike),
    Bridge(NumericBridge),
}

impl Editor {
    /// The active tool if it is one that keeps running through Undo/Redo.
    pub(crate) fn mid_undo_task(&self) -> Option<ActiveTask> {
        // Every tool except Pen (its own point undo) and Polygon Reduction
        // (refused on purpose, RED-004/D1).
        self.session.active_task().filter(|task| {
            !matches!(
                task,
                ActiveTask::Pen | ActiveTask::Reduction | ActiveTask::SolidSketch
            )
        })
    }

    /// With no step of its own left, Undo closes the active tool: it would
    /// undo the tool's activation (MU-MID-02/03).
    pub(crate) fn undo_closes_tool(&self) -> bool {
        self.history.tool_steps == Some(0) && self.tweak_session().is_some()
    }

    /// Redo would open again the tool that Undo just closed.
    pub(crate) fn redo_reopens_tool(&self) -> Option<&'static str> {
        let r = self.session.tool_reopen.as_ref()?;
        (self.session.active_task().is_none()
            && self.history.redo.len() == r.redo_len
            && self.scene == r.closed)
            .then(|| r.session.label())
    }

    /// Bookkeeping after a successful command. `history` is true for Undo
    /// and Redo, which keep these fields themselves.
    pub(crate) fn track_tool_activation(&mut self, before: Option<ActiveTask>, history: bool) {
        let after = self.mid_undo_task();
        if after.is_none() {
            self.history.tool_steps = None;
        }
        if history {
            return;
        }
        self.session.tool_reopen = None;
        if after != before {
            if before.is_some() {
                // Leaving drops the steps undone while the tool was active
                // (MU-MID-04: Redo is off after Escape or Q); older undone
                // work stays redoable.
                if let Some(floor) = self.history.tool_redo_floor.take() {
                    self.history.redo.truncate(floor);
                }
            }
            if after.is_some() {
                self.history.tool_steps = Some(0);
                self.history.tool_redo_floor = Some(self.history.redo.len());
            }
        }
    }

    /// Undo of the activation step: close the tool, throwing away a drawn
    /// primitive's unfinished preview. Returns the tool's name.
    pub(crate) fn close_tool_by_undo(&mut self) -> Result<Option<&'static str>> {
        if !self.undo_closes_tool() {
            return Ok(None);
        }
        let shown = self.scene.clone();
        let shown_identities = self.identities.current.clone();
        let session = if let Some(l) = self.session.live.take() {
            self.scene = l.base.clone();
            if let Some(ids) = &l.identity_base {
                self.identities.current = ids.clone();
                self.identities.observe(&self.scene)?;
            }
            ReopenSession::Live(Box::new(l))
        } else if let Some(s) = self.session.cone.take() {
            ReopenSession::Cone(s)
        } else if let Some(s) = self.session.ellipsoid.take() {
            ReopenSession::Ellipsoid(s)
        } else if let Some(s) = self.session.shape.take() {
            ReopenSession::Shape(s)
        } else if let Some(s) = self.session.spike.take() {
            ReopenSession::Spike(s)
        } else if let Some(s) = self.session.bridge.take() {
            ReopenSession::Bridge(s)
        } else {
            return Ok(None);
        };
        let label = session.label();
        self.history.tool_steps = None;
        let tweaks = std::mem::take(&mut self.session.tweaks);
        self.session.tool_reopen = Some(ToolReopen {
            session,
            shown,
            shown_identities,
            closed: self.scene.clone(),
            redo_len: self.history.redo.len(),
            tweaks,
        });
        Ok(Some(label))
    }

    /// Redo of the activation step: open the tool again as Undo left it.
    pub(crate) fn reopen_tool_by_redo(&mut self) -> Result<Option<&'static str>> {
        if self.redo_reopens_tool().is_none() {
            return Ok(None);
        }
        let Some(r) = self.session.tool_reopen.take() else {
            return Ok(None);
        };
        let label = r.session.label();
        match r.session {
            ReopenSession::Live(l) => {
                self.scene = r.shown;
                self.identities.current = r.shown_identities;
                self.identities.observe(&self.scene)?;
                self.session.live = Some(*l);
            }
            ReopenSession::Cone(s) => self.session.cone = Some(s),
            ReopenSession::Ellipsoid(s) => self.session.ellipsoid = Some(s),
            ReopenSession::Shape(s) => self.session.shape = Some(s),
            ReopenSession::Spike(s) => self.session.spike = Some(s),
            ReopenSession::Bridge(s) => self.session.bridge = Some(s),
        }
        self.history.tool_steps = Some(0);
        self.history.tool_redo_floor = Some(self.history.redo.len());
        self.session.tweaks = r.tweaks;
        Ok(Some(label))
    }
}

impl Editor {
    /// The active tool keeps its own tweak history.
    pub(crate) fn tweak_session(&self) -> Option<ActiveTask> {
        // An Extrude stage keeps its own Begin/Move Extrude history.
        if self.i2_active()
            || (self.i2.selected.is_some()
                && self
                    .session
                    .live
                    .as_ref()
                    .is_some_and(|l| l.tool == Tool::Extrude))
        {
            return None;
        }
        self.mid_undo_task()
    }
    /// The tool's state, unless it holds an invalid draft (a field error),
    /// which is never a state to come back to.
    fn session_snapshot(&self) -> Option<TweakState> {
        self.tweak_session()?;
        let invalid = self
            .session
            .live
            .as_ref()
            .is_some_and(|l| l.preview_error.is_some())
            || self
                .session
                .cone
                .as_ref()
                .is_some_and(|s| s.error.is_some())
            || self
                .session
                .ellipsoid
                .as_ref()
                .is_some_and(|s| s.error.is_some())
            || self
                .session
                .shape
                .as_ref()
                .is_some_and(|s| s.error.is_some());
        if invalid {
            return None;
        }
        let session = if let Some(l) = &self.session.live {
            ReopenSession::Live(Box::new(l.clone()))
        } else if let Some(s) = &self.session.cone {
            ReopenSession::Cone(s.clone())
        } else if let Some(s) = &self.session.ellipsoid {
            ReopenSession::Ellipsoid(s.clone())
        } else if let Some(s) = &self.session.shape {
            ReopenSession::Shape(s.clone())
        } else if let Some(s) = &self.session.spike {
            ReopenSession::Spike(s.clone())
        } else {
            ReopenSession::Bridge(self.session.bridge.clone()?)
        };
        Some(TweakState {
            bevel_start: false,
            named: None,
            session,
            scene: self.scene.clone(),
            identities: self.identities.current.clone(),
            records: self.history.pending_records.clone(),
        })
    }
    fn restore_snapshot(&mut self, state: TweakState) -> Result<()> {
        self.history.pending_records = state.records;
        self.scene = state.scene;
        self.identities.current = state.identities;
        self.identities.observe(&self.scene)?;
        match state.session {
            ReopenSession::Live(l) => self.session.live = Some(*l),
            ReopenSession::Cone(s) => self.session.cone = Some(s),
            ReopenSession::Ellipsoid(s) => self.session.ellipsoid = Some(s),
            ReopenSession::Shape(s) => self.session.shape = Some(s),
            ReopenSession::Spike(s) => self.session.spike = Some(s),
            ReopenSession::Bridge(s) => self.session.bridge = Some(s),
        }
        Ok(())
    }

    /// `cmd` adjusts the active tool. Bevel's Continue (Shift+click/drag
    /// starts a fresh bevel) counts too, so Undo walks back through stacked
    /// bevels one edit at a time with the tool on, and a Shift+drag is one
    /// step with the drag it starts (Jason, 2026-11-09).
    pub(crate) fn is_tweak(&self, cmd: &Command) -> bool {
        is_tweak_command(cmd)
            || (matches!(cmd, Command::Continue)
                && self.session.live.as_ref().is_some_and(|l| {
                    matches!(l.tool, Tool::Bevel | Tool::TwistBevel | Tool::EdgeExtend)
                }))
    }

    /// Before a command: the state to keep if it starts a new tweak.
    pub(crate) fn tweak_before(&self, cmd: &Command) -> Option<Box<TweakStart>> {
        if !self.is_tweak(cmd) || self.session.tweaks.open {
            return None;
        }
        let mut state = self.session_snapshot()?;
        state.bevel_start = matches!(cmd, Command::Continue);
        if matches!(cmd, Command::ResetToolAttributes) {
            state.named = self.reset_step_label();
        }
        Some(Box::new(TweakStart { state }))
    }

    /// After a successful command: keep a new tweak, join or close an open
    /// one, and forget the tool's tweaks once it closes or records a step.
    pub(crate) fn track_tweak(
        &mut self,
        cmd_was_tweak: bool,
        start: Option<Box<TweakStart>>,
        before_task: Option<ActiveTask>,
        serial_before: u64,
        history_command: bool,
    ) {
        if history_command {
            return;
        }
        let task = self.tweak_session();
        if task.is_none() || task != before_task || self.history.history_serial != serial_before {
            // Closed, switched, or recorded a step (Apply): the tweaks are
            // in that step now.
            self.session.tweaks = ToolTweaks::default();
            return;
        }
        if let Some(start) = start {
            let changed = start.state.scene != self.scene
                || self
                    .session_snapshot()
                    .is_none_or(|s| s.session != start.state.session);
            // A configure command that re-dispatches `Preview` (Bevel's panel
            // edits) arrives here twice with the same start; keep it once so
            // one edit is one Undo.
            // A named step whose inner commands kept the same start under
            // the tool's name (a reset re-dispatching Preview) takes the
            // name instead of becoming a second step.
            if let Some(last) = self.session.tweaks.undo.last_mut()
                && start.state.named.is_some()
                && last.named.is_none()
                && (TweakState {
                    named: None,
                    ..start.state.clone()
                }) == *last
            {
                last.named = start.state.named.clone();
            } else if changed && self.session.tweaks.undo.last() != Some(&start.state) {
                self.session.tweaks.undo.push(start.state);
                self.session.tweaks.redo.clear();
                self.session.tweaks.open = true;
            }
        }
        if !cmd_was_tweak || !self.session.tweak_held {
            self.session.tweaks.open = false;
        }
    }

    /// A gesture ended: the next change starts a new tweak.
    pub fn end_tweak(&mut self) {
        self.session.tweak_held = false;
        self.session.tweaks.open = false;
    }

    /// Undo the active tool's last tweak; the tool stays.
    pub(crate) fn undo_tweak(&mut self) -> Result<Option<&'static str>> {
        if self.tweak_session().is_none() {
            return Ok(None);
        }
        let Some(state) = self.session.tweaks.undo.pop() else {
            return Ok(None);
        };
        let mut now = self.session_snapshot().ok_or("No active tool")?;
        // The step keeps its kind across Undo/Redo (a Continue stays one).
        now.bevel_start = state.bevel_start;
        now.named = state.named.clone();
        let label = now.session.label();
        self.restore_snapshot(state)?;
        self.session.tweaks.redo.push(now);
        self.session.tweaks.open = false;
        Ok(Some(label))
    }

    /// Redo the active tool's last undone tweak.
    pub(crate) fn redo_tweak(&mut self) -> Result<Option<&'static str>> {
        if self.tweak_session().is_none() {
            return Ok(None);
        }
        let Some(state) = self.session.tweaks.redo.pop() else {
            return Ok(None);
        };
        let mut now = self.session_snapshot().ok_or("No active tool")?;
        now.bevel_start = state.bevel_start;
        now.named = state.named.clone();
        let label = now.session.label();
        self.restore_snapshot(state)?;
        self.session.tweaks.undo.push(now);
        self.session.tweaks.open = false;
        Ok(Some(label))
    }

    /// The scene (and identities) before the session's first kept tweak:
    /// what Drop records from and Cancel goes back to.
    pub(crate) fn tweak_session_base(&self) -> Option<(Scene, IdentityScene)> {
        // Only the transform tools move their base along with each drag, and
        // Bevel and Edge Extend with each fresh one they stack; a primitive's base is the
        // session's start already.
        if !self.session.live.as_ref().is_some_and(|l| {
            l.tool.is_transform()
                || matches!(l.tool, Tool::Bevel | Tool::TwistBevel | Tool::EdgeExtend)
        }) {
            return None;
        }
        self.session
            .tweaks
            .undo
            .first()
            .map(|s| (s.scene.clone(), s.identities.clone()))
    }
}

impl Editor {
    /// TA-07 (provisional): a Bevel session leaves one undo step per bevel.
    /// Records each bevel closed by Continue (Shift+click / Shift+drag) as
    /// its own step, then the open one up to the current scene. Called by
    /// Drop and Cancel with the live tool already taken.
    /// Edge Extend's Shift+click stacks extensions the same way.
    pub(crate) fn record_bevel_session(
        &mut self,
        tool: Tool,
        base: Scene,
        base_ids: Option<IdentityScene>,
    ) {
        let mut starts = vec![(base, base_ids)];
        for s in self.session.tweaks.undo.iter().filter(|s| s.bevel_start) {
            if starts.last().is_some_and(|(scene, _)| *scene != s.scene) {
                starts.push((s.scene.clone(), Some(s.identities.clone())));
            }
        }
        let end = self.scene.clone();
        let end_ids = self.identities.current.clone();
        let label = tool.label().to_string();
        let mut starts = starts.into_iter().peekable();
        while let Some((before, ids)) = starts.next() {
            // A closed bevel ends where the next begins; the last ends now.
            let (after, after_ids) = match starts.peek() {
                Some((scene, ids)) => (scene.clone(), ids.clone()),
                None => (end.clone(), None),
            };
            if before == after {
                continue;
            }
            self.scene = after;
            self.identities.record_before = ids;
            self.record(label.clone(), before);
            if after_ids.is_some()
                && let Some(step) = self.history.undo.last_mut().map(std::sync::Arc::make_mut)
            {
                step.identity_after = after_ids;
            }
        }
        self.scene = end;
        self.identities.current = end_ids;
    }
}

/// The tool state before a command that may become a tweak.
pub(crate) struct TweakStart {
    state: TweakState,
}

impl ReopenSession {
    fn label(&self) -> &'static str {
        match self {
            Self::Live(l) => l.tool.label(),
            Self::Cone(_) => "Cone",
            Self::Ellipsoid(_) => "Ellipsoid",
            Self::Shape(s) => s.kind().label(),
            Self::Spike(_) => "Spike",
            Self::Bridge(_) => "Bridge",
        }
    }
}
