//! A click applies a tool that waits for Apply (UIC-78, provisional until
//! the maintainer OKs the wording): a plain left click in empty space in a
//! 3D view (off every handle), without a drag, does what Apply does. The tool stays live on
//! the result, so changing a value reshapes it (still one undo step) until
//! a new selection or another edit settles it. Drags still drive the tool
//! (DRAG-Q01, UIC-76). When
//! the tool can't apply, nothing changes and the status line says why.
//! Clicks never select while the tool is on (the maintainer, 2026-11-25):
//! select first, then pick the tool.
use super::*;
use input::NumericDrag;

/// How far (in pixels) a press may wander and still count as a click, as
/// Polygon Reduction's stationary click does.
pub(crate) const CLICK_SLOP: f32 = 3.;

impl Operandi {
    /// The release of a plain left click in a 3D view during a numeric
    /// session's press (`drag` is the gesture the press began).
    pub(crate) fn click_finish_numeric(&mut self, drag: &NumericDrag) {
        match drag {
            NumericDrag::Bridge(_) => self.click_apply_bridge(),
            NumericDrag::Spike(_) => self.click_apply_spike(),
            NumericDrag::Reduction(..) => self.click_apply_reduction(),
            _ => {}
        }
    }

    /// Bridge: Apply with the panel's values; the tool stays on.
    fn click_apply_bridge(&mut self) {
        // The panel greys Apply for the same reasons (UIC-92).
        if !self.field_errors.is_empty() {
            self.status = format!("Bridge: {}", crate::unavailable::FIELD_ERRORS);
            return;
        }
        if let Err(e) = self.editor.bridge_eligibility() {
            self.status = format!("Bridge: {e}");
            return;
        }
        if self.run_ok(Command::Apply) {
            self.status = "Bridge applied · change a value to reshape it".into();
        }
    }

    /// Spike: Apply with the panel's Strength; the tool stays on.
    fn click_apply_spike(&mut self) {
        if !self.field_errors.is_empty() {
            self.status = format!("Spike: {}", crate::unavailable::FIELD_ERRORS);
            return;
        }
        if self.run_ok(Command::Apply) {
            self.status = "Spike applied · change Strength to reshape it".into();
        }
    }

    /// Polygon Reduction: its preview is live before Apply (RED-004), so a
    /// click keeps a changed preview as Apply does and the tool stays
    /// armed for the next reduction. A click with nothing changed yet (the
    /// first one initializes the mesh) keeps nothing.
    fn click_apply_reduction(&mut self) {
        let changed = self
            .editor
            .session
            .reduction
            .as_ref()
            .and_then(|r| r.source.as_ref())
            .is_some_and(|source| source.scene != self.editor.scene);
        if changed && self.run_ok(Command::Apply) {
            self.status =
                "Polygon Reduction applied · drag or type a Number to reduce again".into();
        }
    }
}
