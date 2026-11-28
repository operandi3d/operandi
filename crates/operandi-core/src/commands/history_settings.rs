//! Undo preferences (Preferences > Defaults > Application): how many steps
//! are kept, and whether a run of selection changes becomes one step.
use crate::{Editor, Scene};

/// Undo limits the app sets from the user's preferences. Not scene data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistorySettings {
    /// Most undo steps kept; the oldest is dropped past it.
    pub max_undo_level: usize,
    /// Merge consecutive selection changes into a single undo step.
    pub group_selection: bool,
    /// Most selection changes merged into one step while grouping.
    pub max_sub_commands: u32,
}
impl HistorySettings {
    /// Measured: Max Undo Level 100.
    pub const MAX_UNDO_LEVEL: usize = 100;
    /// Provisional (UND default not yet measured): grouping off, so each
    /// selection change stays its own step as before.
    pub const GROUP_SELECTION: bool = false;
    /// Provisional (not yet measured).
    pub const MAX_SUB_COMMANDS: u32 = 100;
}
impl Default for HistorySettings {
    fn default() -> Self {
        Self {
            max_undo_level: Self::MAX_UNDO_LEVEL,
            group_selection: Self::GROUP_SELECTION,
            max_sub_commands: Self::MAX_SUB_COMMANDS,
        }
    }
}

impl Editor {
    /// Record a selection change. With Group Selection Commands on, it joins
    /// the newest step when that step is a selection group still under Max
    /// Sub-Commands Recorded and nothing else has changed the scene since.
    /// Provisional (UND-Q1): any other recorded step, Undo, Redo or untracked
    /// scene change ends a group; a pause does not.
    pub(crate) fn record_selection(&mut self, label: String, before: Scene) {
        let settings = self.history.settings;
        if settings.group_selection
            && self.history.selection_run > 0
            && self.history.selection_run < settings.max_sub_commands
            && self.history.redo.is_empty()
            && let Some(last) = self.history.undo.last_mut().map(std::sync::Arc::make_mut)
            && last.after == before
        {
            last.after = self.scene.clone();
            let pending = std::mem::take(&mut self.history.pending_records.0);
            last.records
                .extend_capped(pending, settings.max_sub_commands as usize);
            self.history.selection_run += 1;
            self.history.history_mark += 1;
            return;
        }
        self.record(label, before);
        self.history.selection_run = 1;
    }

    /// True when the scene holds only committed work, so an auto-save backup
    /// can be taken: no tool session or selection drag open, except a
    /// transform tool that stays armed with nothing pending.
    pub fn backup_ready(&self) -> bool {
        let armed_transform = self
            .session.live
            .as_ref()
            .is_some_and(|l| l.tool.is_transform() && !self.transform_change_pending())
            // Drags kept as tweaks aren't recorded until Drop.
            && !self.session.tweaks.can_undo();
        self.selection_start.is_none()
            && (!self.session.has_active_task() || (armed_transform && self.active_task_is_live()))
    }
    fn active_task_is_live(&self) -> bool {
        self.session.cone.is_none()
            && self.session.ellipsoid.is_none()
            && self.session.shape.is_none()
            && self.session.spike.is_none()
            && self.session.bridge.is_none()
            && self.session.reduction.is_none()
            && self.session.pen_draft.is_none()
            && self.session.solid_sketch.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Component, Mode};

    fn editor(settings: HistorySettings) -> (Editor, u64) {
        let mut e = Editor::default();
        e.history.settings = settings;
        e.dispatch(Command::NewMesh).unwrap();
        e.scene.active_mut().unwrap().mesh =
            crate::cube_geometry(glam::DVec3::ZERO, glam::DVec3::ONE)
                .unwrap()
                .into();
        e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
        e.history.undo.clear();
        let id = e.scene.selection.item.unwrap();
        (e, id)
    }
    fn click(e: &mut Editor, id: u64, face: u32) {
        e.dispatch(Command::Select {
            item: id,
            hit: Component::Polygon(face),
            replace: true,
            remove: false,
        })
        .unwrap();
    }
    fn grouped(max_sub_commands: u32) -> HistorySettings {
        HistorySettings {
            group_selection: true,
            max_sub_commands,
            ..Default::default()
        }
    }

    #[test]
    fn backups_wait_for_committed_work() {
        let (mut e, id) = editor(HistorySettings::default());
        click(&mut e, id, 0);
        assert!(e.backup_ready());
        e.dispatch(Command::BeginSelection).unwrap();
        assert!(!e.backup_ready(), "selection drag open");
        e.dispatch(Command::EndSelection { cancel: true }).unwrap();
        e.dispatch(Command::Activate(crate::Tool::Move)).unwrap();
        assert!(e.backup_ready(), "armed Move with nothing pending");
        e.dispatch(Command::Cancel).unwrap();
        e.dispatch(Command::Activate(crate::Tool::Extrude)).unwrap();
        assert!(!e.backup_ready(), "Extrude session open");
    }

    #[test]
    fn defaults_keep_100_steps_and_separate_selection_steps() {
        let d = HistorySettings::default();
        assert_eq!(d.max_undo_level, 100);
        assert!(!d.group_selection);
        let (mut e, id) = editor(d);
        let start = e.history.undo.len();
        for face in 0..3 {
            click(&mut e, id, face);
        }
        assert_eq!(e.history.undo.len(), start + 3);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene.selection.faces, vec![1]);
    }

    #[test]
    fn oldest_step_is_dropped_past_max_undo_level() {
        let (mut e, id) = editor(HistorySettings {
            max_undo_level: 3,
            ..Default::default()
        });
        for face in 0..6 {
            click(&mut e, id, face % 2);
        }
        assert_eq!(e.history.undo.len(), 3);
        for _ in 0..3 {
            e.dispatch(Command::Undo).unwrap();
        }
        // Three steps back from the last click; earlier ones are gone.
        assert_eq!(e.scene.selection.faces, vec![0]);
        assert!(e.history.undo.is_empty());
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene.selection.faces, vec![0]);
    }

    #[test]
    fn lowering_the_limit_trims_on_the_next_step() {
        let (mut e, id) = editor(HistorySettings::default());
        for face in 0..5 {
            click(&mut e, id, face);
        }
        let len = e.history.undo.len();
        e.history.settings.max_undo_level = 2;
        assert_eq!(
            e.history.undo.len(),
            len,
            "nothing is lost until a new step"
        );
        click(&mut e, id, 0);
        assert_eq!(e.history.undo.len(), 2);
    }

    #[test]
    fn grouped_selection_clicks_undo_together() {
        let (mut e, id) = editor(grouped(100));
        let before = e.scene.clone();
        let start = e.history.undo.len();
        for face in 0..4 {
            click(&mut e, id, face);
        }
        assert_eq!(e.history.undo.len(), start + 1);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, before);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene.selection.faces, vec![3]);
    }

    #[test]
    fn group_starts_a_new_step_after_max_sub_commands() {
        let (mut e, id) = editor(grouped(2));
        let start = e.history.undo.len();
        for face in 0..5 {
            click(&mut e, id, face);
        }
        // 2 + 2 + 1 changes.
        assert_eq!(e.history.undo.len(), start + 3);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene.selection.faces, vec![3]);
    }

    #[test]
    fn a_modeling_step_or_undo_ends_a_group() {
        let (mut e, id) = editor(grouped(100));
        let start = e.history.undo.len();
        click(&mut e, id, 0);
        click(&mut e, id, 1);
        e.dispatch(Command::ClearSelection).unwrap();
        e.dispatch(Command::NewMesh).unwrap();
        e.dispatch(Command::SelectItem(Some(id))).unwrap();
        e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
        let after_new = e.history.undo.len();
        assert!(after_new >= start + 2);
        click(&mut e, id, 2);
        assert_eq!(
            e.history.undo.len(),
            after_new + 1,
            "new group after New Mesh"
        );
        e.dispatch(Command::Undo).unwrap();
        let base = e.scene.clone();
        click(&mut e, id, 3);
        click(&mut e, id, 4);
        assert_eq!(e.history.undo.len(), after_new + 1, "Undo ended the group");
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, base, "both later clicks undo together");
    }

    #[test]
    fn turning_grouping_on_lets_the_last_selection_step_grow() {
        let (mut e, id) = editor(HistorySettings::default());
        click(&mut e, id, 0);
        let len = e.history.undo.len();
        e.history.settings = grouped(100);
        // The previous step was recorded as a group of one, so it may grow.
        click(&mut e, id, 1);
        assert_eq!(e.history.undo.len(), len);
        e.dispatch(Command::Undo).unwrap();
        assert!(e.scene.selection.faces.is_empty());
    }
}
