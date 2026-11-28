//! Click to start: the shared first-click layer
//! (specifications/tool-click-start-2026-11-16).
//!
//! A tool whose Auto-Activate flag is off (CS-01, the flag table in
//! `presets/factory-tool-presets.toml`, CS-05) is only *armed* when picked:
//! the Tool Properties panel shows and edits its values, but nothing is
//! drawn and the mesh stays as it was. Apply runs it once with those values
//! and leaves it armed (CS-21); Drop or Cancel of a never-clicked tool
//! changes nothing (CS-22). The first left click in a 3D view
//! (`Command::ToolClick`) makes it live, through the pipe in order (CS-02):
//! the Automatic action centre goes to the click (CS-10), a tool's own
//! Auto axis row goes to the work plane's normal (CS-11), other centres
//! stay (CS-12), self-fitting falloffs keep their fit to the target
//! (CS-13), the tool's own click setup runs (CS-14; Bend fits its spine),
//! then the tool shows its result for the panel's values.
//!
//! The editor only arms when `auto_activate_flags` is on (the app turns it
//! on) and only for top-level commands, so staged one-shots and scripts
//! that never set it behave as before.
use crate::editor::{Command, Editor, Tool};
use crate::{CenterMode, Result};
use glam::DVec3;

/// CS-06: the status bar text while `tool` is armed.
pub fn armed_status(label: &str) -> String {
    format!("Click in a view to start {label}, or set values and press Apply.")
}

/// Tools that already wait for a click (or Apply) by their own press code:
/// primitives draw from the press, Duplicate tools place on the first click
/// (DS-30/31), the slices cut where clicked. Their flag is off in the
/// table, but the editor doesn't arm them on top of their own waiting.
pub fn waits_on_its_own(tool: Tool) -> bool {
    tool.is_primitive() || tool.is_duplicate() || tool.is_slice()
}

/// Whether `cmd` is one that picks a tool (and so may arm it).
pub(crate) fn may_arm(cmd: &Command) -> bool {
    matches!(
        cmd,
        Command::Activate(_) | Command::ActivateDeform(_) | Command::ActivatePreset(_)
    )
}

impl Editor {
    /// The armed tool, if the live tool is armed (CS-01).
    pub fn armed_tool(&self) -> Option<Tool> {
        let tool = self.session.armed?;
        self.session
            .live
            .as_ref()
            .filter(|l| l.tool == tool)
            .map(|l| l.tool)
    }
    pub fn is_armed(&self) -> bool {
        self.armed_tool().is_some()
    }

    /// The status text while armed (CS-06), named as the button is.
    pub fn armed_status(&self) -> Option<String> {
        self.armed_tool()?;
        Some(armed_status(self.live_tool_label().unwrap_or("the tool")))
    }

    /// After a tool was picked: arm it when its flag says so (CS-01). The
    /// scene goes back to the tool's start, so nothing the start showed
    /// (a remembered Angle, a fitted preview) stays on the mesh.
    pub(crate) fn arm_if_flagged(&mut self) {
        self.session.armed = None;
        self.session.extend_from_click = false;
        if !self.auto_activate_flags {
            return;
        }
        let Some(live) = self.session.live.as_mut() else {
            return;
        };
        let tool = live.tool;
        let preset = self
            .session
            .preset
            .as_ref()
            .filter(|p| p.tool == tool)
            .map(|p| p.id.clone());
        if waits_on_its_own(tool) || crate::tool_preset::auto_activate(tool, preset.as_deref()) {
            return;
        }
        self.scene = live.base.clone();
        live.applied = false;
        live.identity_preview = None;
        live.preview_error = None;
        self.session.armed = Some(tool);
    }

    /// The commands an armed tool answers itself; `None` lets the command
    /// run as usual (and a tool that is no longer live forgets it was
    /// armed).
    pub(crate) fn armed_command(&mut self, cmd: &Command) -> Option<Result<()>> {
        if self.session.armed.is_some() && !self.is_armed() {
            self.session.armed = None;
        }
        self.session.armed?;
        match cmd {
            // CS-01: panel edits change the values, not the mesh.
            Command::Preview(p) => {
                let live = self.session.live.as_mut()?;
                live.params = p.clone();
                Some(Ok(()))
            }
            Command::Apply => Some(self.apply_armed()),
            Command::ToolClick { point, plane } => Some(self.first_click(*point, *plane)),
            // CS-22: nothing was shown, so Drop and Cancel change nothing;
            // the tool's own drop still keeps its values for next time.
            Command::Drop | Command::Cancel => {
                self.session.armed = None;
                None
            }
            _ => None,
        }
    }

    /// CS-21: Apply on an armed tool runs it once with the panel's values
    /// as one undo step and leaves it armed on the result, values kept.
    fn apply_armed(&mut self) -> Result<()> {
        let live = self.session.live.as_ref().ok_or("No active tool")?;
        let (tool, params) = (live.tool, live.params.clone());
        let mut staged = self.clone();
        staged.session.armed = None;
        staged.session.dispatch_depth += 1;
        let before = staged.scene.clone();
        staged.preview(params.clone(), false)?;
        let live = staged.session.live.as_ref().ok_or("No active tool")?;
        if staged.scene != before {
            staged.identities.record_before = live.identity_base.clone();
            let label = staged
                .session
                .preset
                .as_ref()
                .filter(|p| p.tool == tool)
                .map_or(tool.label(), crate::PresetSession::label)
                .to_string();
            staged.record(label, before);
        }
        let base = staged.scene.clone();
        let current = staged.identities.current.clone();
        let live = staged.session.live.as_mut().ok_or("No active tool")?;
        live.base = base;
        live.identity_base = Some(current);
        live.identity_preview = None;
        live.applied = false;
        live.params = params;
        staged.session.armed = Some(tool);
        staged.session.dispatch_depth -= 1;
        *self = staged;
        Ok(())
    }

    /// CS-02: the first click makes the armed tool live at `point` on work
    /// plane `plane`. Refused input leaves it armed.
    fn first_click(&mut self, point: DVec3, plane: u8) -> Result<()> {
        if !point.is_finite() || point.abs().max_element() > 1e9 {
            return Err("The click must be a finite point".into());
        }
        if plane > 2 {
            return Err("The work plane must be 0, 1 or 2".into());
        }
        let mut staged = self.clone();
        staged.session.armed = None;
        staged.session.dispatch_depth += 1;
        let tool = staged.session.live.as_ref().ok_or("No active tool")?.tool;
        if tool == Tool::Bend {
            // CS-10/11/14 for Bend: the centre (its own copy of the action
            // centre), Axis on the plane's normal, the spine fit.
            staged.dispatch(Command::ToolClick { point, plane })?;
        } else if matches!(
            staged.action_context.center,
            CenterMode::Automatic | CenterMode::None
        ) && let Some(frame) = staged
            .session
            .live
            .as_mut()
            .and_then(|l| l.action_frame.as_mut())
        {
            // CS-10: Automatic puts the centre at the click, and so does
            // None, which is Automatic without an axis choice (AC-12,
            // ACS-07). CS-12: the other centres stay where they are.
            frame.center = point;
        }
        // CS-14 for Edge Extend (N25, Jason 2026-11-17): the click makes
        // the extension, rows on the source edges until dragged, so the
        // next tool can scale them in at once.
        if tool == Tool::EdgeExtend {
            staged.session.extend_from_click = true;
        }
        // CS-13: Linear and Radial falloffs were fitted to the target when
        // the tool was picked (TP-17); the target hasn't changed since.
        // TP-19: a row whose falloff starts at the tool (Soft Move) puts
        // the falloff's centre where the click put the tool's centre.
        staged.place_row_falloff_at_tool();
        // Then the tool shows its result for the panel's values.
        let params = staged
            .session
            .live
            .as_ref()
            .ok_or("No active tool")?
            .params
            .clone();
        staged.preview(params, false)?;
        staged.session.dispatch_depth -= 1;
        *self = staged;
        Ok(())
    }
}
