//! Which tools take part in Symmetry, and the "never silent" rule
//! (symmetry-2026-11-13 section 7, SYM3-40): while Symmetry is on, a tool
//! that doesn't support it yet still works one-sided, but the status bar
//! says "<Tool> ignores Symmetry for now". Each tool's class is on its card
//! (`ToolCard::layers.symmetry`, tool_card.rs), whose match has no wildcard
//! arm, so a new tool doesn't compile until someone decides its class.
use crate::Editor;
use crate::active_task::ActiveTask;

/// How a tool takes part in Symmetry (spec section 7's classes, folded).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymmetrySupport {
    /// Wired into the shared service (movement or creation path).
    Built,
    /// Class B: works on each element by itself, so a symmetric selection
    /// gives a symmetric result with no extra code.
    ViaSelection,
    /// Not wired yet: works one-sided and says so (SYM3-40).
    Ignores,
}

/// The support of any open task, and its name for messages.
pub fn task_support(task: ActiveTask) -> (SymmetrySupport, &'static str) {
    use SymmetrySupport::*;
    match task {
        ActiveTask::Live(tool) => (tool.card().layers.symmetry, tool.label()),
        ActiveTask::Cone => (Built, "Cone"),
        ActiveTask::Ellipsoid => (Built, "Ellipsoid"),
        ActiveTask::Shape(kind) => (Built, kind.label()),
        // Spikes grow from each selected polygon by itself.
        ActiveTask::Spike => (ViaSelection, "Spike"),
        // Bridges the picked pairs; symmetric when both sides are picked.
        ActiveTask::Bridge => (ViaSelection, "Bridge"),
        ActiveTask::Reduction => (ViaSelection, "Polygon Reduction"),
        ActiveTask::Pen => (Built, "Pen"),
        ActiveTask::SolidSketch => (Built, "Solid Sketch"),
    }
}

impl Editor {
    /// What the status bar should say about Symmetry for the open tool:
    /// "<Tool> ignores Symmetry for now" (SYM3-40), or the tool's own
    /// Symmetry notice (Solid Sketch, SS-SYM-04/08). `None` when there is
    /// nothing to say.
    pub fn symmetry_notice(&self) -> Option<String> {
        if let Some(session) = &self.session.solid_sketch {
            return session.symmetry_notice();
        }
        self.symmetry.axis?;
        let fixing = self
            .session
            .live
            .as_ref()
            .is_none_or(|l| l.tool == crate::Tool::SymmetryTool);
        if !fixing && !self.symmetry_unpaired.is_empty() {
            return Some(unpaired_message(self.symmetry_unpaired.len()));
        }
        let (support, name) = task_support(self.session.active_task()?);
        (support == SymmetrySupport::Ignores).then(|| crate::symmetry::ignores_message(name))
    }
}

/// Section 8's unpaired warning.
pub fn unpaired_message(count: usize) -> String {
    format!(
        "{count} point{} no mirror partner — Symmetrize or Symmetry Tool to fix",
        if count == 1 { " has" } else { "s have" }
    )
}

impl Editor {
    /// Section 8: when a tool starts with Symmetry on, the points it edits
    /// that have no mirror partner are kept in `symmetry_unpaired` (the
    /// status bar warns and the viewport marks them). The Symmetry Tool
    /// keeps its own list.
    pub(crate) fn note_unpaired_points(&mut self) {
        let Some(live) = &self.session.live else {
            self.symmetry_unpaired.clear();
            return;
        };
        if live.tool == crate::Tool::SymmetryTool {
            return;
        }
        self.symmetry_unpaired.clear();
        let symmetry = self.active_symmetry();
        if symmetry.axis.is_none() || self.scene.selection.mode == crate::Mode::Item {
            return;
        }
        let (Some(item), Ok(points)) =
            (self.scene.active(), crate::transform_vertices(&self.scene))
        else {
            return;
        };
        let points: Vec<u32> = points.into_iter().collect();
        let partners = crate::symmetry::partners_of(&item.mesh, symmetry, &points);
        self.symmetry_unpaired = points
            .into_iter()
            .zip(partners)
            .filter(|(_, p)| p.is_none())
            .map(|(v, _)| v)
            .collect();
    }
}
