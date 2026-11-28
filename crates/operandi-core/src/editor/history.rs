//! Undo, redo and the undo history's steps, marks and memory budget.
use super::*;

impl Editor {
    /// Undo one history step.
    pub(super) fn handle_undo(
        &mut self,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> Result<()> {
        self.history_eligibility(crate::HistoryAction::Undo)
            .permit_reported(report)?;
        // Inside a tool, Undo takes back its last tweak and keeps the tool
        // (Jason, 2026-11-04).
        if let Some(label) = self.undo_tweak()? {
            if let Some(report) = report {
                report.effect = crate::HistoryEffect::TweakUndone {
                    label: label.into(),
                };
            }
            return Ok(());
        }
        // Mid-tool undo (MTU-Q1, provisional): a numeric primitive's
        // unrecorded preview goes first, and the tool stays active.
        if let Some(label) = self.discard_primitive_preview(true)? {
            if let Some(report) = report {
                report.effect = crate::HistoryEffect::PreviewDiscarded { label };
            }
            return Ok(());
        }
        // With none of its own steps left, Undo undoes the tool's
        // activation: the tool closes, geometry stays (MU-MID-02/03), and a
        // drawn primitive's unfinished preview goes with it (U1).
        if let Some(label) = self.close_tool_by_undo()? {
            if let Some(report) = report {
                report.effect = crate::HistoryEffect::ToolClosed {
                    label: label.into(),
                };
            }
            return Ok(());
        }
        if let Some(label) = self.discard_primitive_preview(false)? {
            if let Some(report) = report {
                report.effect = crate::HistoryEffect::PreviewDiscarded { label };
            }
            return Ok(());
        }
        // A transform tool's pending change was recorded as its own step
        // before this transaction (see `dispatch_transaction`).
        let transform = self
            .live_transform_tool()
            .or_else(|| self.live_primitive_tool());
        if transform.is_some() && self.history.undo.is_empty() {
            return Ok(());
        }
        // Only supported unapplied surface tools and transform tools can
        // reach ordinary Undo with live state; the policy above is its
        // single guard.
        self.session.live = None;
        self.history.selection_run = 0;
        if let Some(c) = self.history.undo.pop() {
            if let Some(report) = report {
                report.effect = crate::HistoryEffect::Completed {
                    action: crate::HistoryAction::Undo,
                    label: c.label.clone(),
                };
            }
            let _undone = std::mem::replace(&mut self.scene, c.before.clone());
            // TK-05: undo never changes the current time.
            crate::animation::keep_time(&_undone, &mut self.scene);
            // Images aren't undone (PNT-11).
            #[cfg(feature = "painting")]
            crate::paint::carry_images(&_undone, &mut self.scene);
            if let Some(ids) = &c.identity_before {
                self.identities.current = ids.clone();
                self.identities.observe(&self.scene)?;
            }
            if let Some(n) = self.history.tool_steps.as_mut() {
                *n = n.saturating_sub(1);
            }
            self.history.redo.push(c);
        }
        if let Some(tool) = transform {
            self.resume_transform(tool);
        }
        self.rebase_numeric_primitive();
        Ok(())
    }

    /// Redo one history step.
    pub(super) fn handle_redo(
        &mut self,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> Result<()> {
        self.history_eligibility(crate::HistoryAction::Redo)
            .permit_reported(report)?;
        if let Some(label) = self.redo_tweak()? {
            if let Some(report) = report {
                report.effect = crate::HistoryEffect::TweakRedone {
                    label: label.into(),
                };
            }
            return Ok(());
        }
        // Redo of a tool's activation step opens it again (U4).
        if let Some(label) = self.reopen_tool_by_redo()? {
            if let Some(report) = report {
                report.effect = crate::HistoryEffect::ToolReopened {
                    label: label.into(),
                };
            }
            return Ok(());
        }
        self.session.spike = None;
        let transform = self
            .live_transform_tool()
            .or_else(|| self.live_primitive_tool());
        if transform.is_some() && !self.history.redo.is_empty() {
            self.session.live = None;
        }
        self.history.selection_run = 0;
        if let Some(c) = self.history.redo.pop() {
            if let Some(report) = report {
                report.effect = crate::HistoryEffect::Completed {
                    action: crate::HistoryAction::Redo,
                    label: c.label.clone(),
                };
            }
            let _redone = std::mem::replace(&mut self.scene, c.after.clone());
            crate::animation::keep_time(&_redone, &mut self.scene);
            #[cfg(feature = "painting")]
            crate::paint::carry_images(&_redone, &mut self.scene);
            if let Some(ids) = &c.identity_after {
                self.identities.current = ids.clone();
                self.identities.observe(&self.scene)?;
            }
            if let Some(n) = self.history.tool_steps.as_mut() {
                *n += 1;
            }
            self.history.undo.push(c);
            if let Some(tool) = transform {
                self.resume_transform(tool);
            }
            self.rebase_numeric_primitive();
        }
        Ok(())
    }

    pub(crate) fn record(&mut self, label: String, before: Scene) {
        // ACS-31: guide symmetry, and rigs drop deleted parts.
        crate::character_rig::sync(&mut self.scene, &before);
        // TK-30/31: auto key, in the same step as the edit.
        self.auto_key_from(&before);
        // Mesh Instances share their sources' meshes again (an edit made
        // on an instance reaches its source and the other instances).
        self.scene.sync_instances(Some(&before));
        // An edit that removed edges takes them out of saved edge sets.
        crate::selection_sets::prune_edge_sets(&mut self.scene);
        // Other active meshes keep components only while selected and
        // unedited (multi_active.rs), or as a tool that ran on them left
        // them (multi_tools.rs).
        let fresh = std::mem::take(&mut self.multi_fresh);
        self.scene
            .prune_component_selections_keeping(Some(&before), &fresh);
        self.multi_fresh = fresh;
        // One gesture, one step: a later edit of the same gesture replaces
        // the previous step's result when nothing came in between.
        if let Some(gesture) = self.history.undo_fold
            && self.history.redo.is_empty()
            && let Some(last) = self.history.undo.last_mut().map(std::sync::Arc::make_mut)
            && last.fold == Some(gesture)
            && last.after == before
        {
            last.after = self.scene.clone();
            let pending = std::mem::take(&mut self.history.pending_records.0);
            last.records
                .extend_capped(pending, self.history.settings.max_sub_commands as usize);
            self.history.history_serial = self.history.history_serial.wrapping_add(1);
            self.history.history_mark += 1;
            return;
        }
        self.i2_branch();
        self.history.history_mark += 1;
        self.history.undo.push(std::sync::Arc::new(Checkpoint {
            identity_before: self.identities.record_before.clone(),
            identity_after: None,
            label,
            before,
            after: self.scene.clone(),
            fold: self.history.undo_fold,
            records: std::mem::take(&mut self.history.pending_records),
            mark: self.history.history_mark,
        }));
        // Max Undo Level: the oldest steps go once a new one exceeds it.
        // A lowered limit trims here, on the next step (UND-Q2, provisional).
        self.history.trim_to_max_undo_level();
        self.trim_undo_memory();
        self.history.redo.clear();
        self.history.selection_run = 0;
        if let Some(n) = self.history.tool_steps.as_mut() {
            *n += 1;
        }
        self.history.history_serial = self.history.history_serial.wrapping_add(1);
    }
    /// Drops the oldest undo steps while the history's item copies pass
    /// `UNDO_MEMORY_BUDGET`, keeping at least the newest step. Each step
    /// keeps two whole copies of the item list; meshes are shared, so the
    /// items themselves are the cost that grows with the scene (100 steps
    /// of a 100,000-item scene were about 35 GB). Provisional
    /// (scene-budget, 2026-11-20).
    pub(crate) fn trim_undo_memory(&mut self) {
        if self.history.undo_trim_paused {
            return;
        }
        self.trim_undo_memory_to(
            self.history
                .undo_memory_budget
                .unwrap_or(UNDO_MEMORY_BUDGET),
        );
    }
    /// `trim_undo_memory` with another budget, in bytes.
    /// The history counts the mesh data each step alone keeps alive
    /// (undo_memory.rs, limits sweep A1), and is held to at most half of
    /// the memory free beside it, when a free-memory source is set. The
    /// newest step is always kept.
    pub fn trim_undo_memory_to(&mut self, budget: usize) {
        let costs =
            crate::undo_memory::step_costs(&self.scene, &self.history.undo, &self.history.redo);
        let mut total: usize = costs.iter().sum();
        let budget = match self.free_memory {
            crate::FreeMemory(Some(free)) => {
                let free = usize::try_from(free()).unwrap_or(usize::MAX);
                budget.min(total.saturating_add(free) / 2)
            }
            crate::FreeMemory(None) => budget,
        };
        let mut excess = 0;
        while total > budget && excess + 1 < self.history.undo.len() {
            total -= costs[excess];
            excess += 1;
        }
        self.history.undo.drain(..excess);
    }
    /// The history's mark now; pass it to `recorded_since` or
    /// `steps_since` after running something (Astra P4-003: "did that
    /// record?" was guessed from `undo.len()` growing, which stops working
    /// once Max Undo Level trims a step for every new one).
    pub fn history_mark(&self) -> u64 {
        self.history.history_mark
    }
    /// Whether recording changed the undo history since `mark`
    /// (`History::recorded_since`).
    pub fn recorded_since(&self, mark: u64) -> bool {
        self.history.recorded_since(mark)
    }
    /// How many undo steps are newer than `mark` (`History::steps_since`).
    pub fn steps_since(&self, mark: u64) -> usize {
        self.history.steps_since(mark)
    }
    /// For a command that sets its result selection after recording its
    /// step: the step recorded since `mark` ends with that selection, so
    /// Redo gives it back (Astra P4-002).
    pub(crate) fn amend_step_selection(&mut self, mark: u64) {
        if self.steps_since(mark) > 0
            && let Some(step) = self.history.undo.last_mut().map(std::sync::Arc::make_mut)
        {
            step.after.selection = self.scene.selection.clone();
        }
    }
    /// The bytes the undo history holds beyond the scene and redo list.
    pub fn undo_memory_bytes(&self) -> usize {
        crate::undo_memory::step_costs(&self.scene, &self.history.undo, &self.history.redo)
            .iter()
            .sum()
    }
}
