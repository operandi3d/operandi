//! MP-C08 Polygon Align: shared panel/command/input route and its flipped-count
//! acknowledgment (ALN-B02). The acknowledgment is a plain OK notice in the
//! shared form dialog (UIC-97); no In the Future preference is created or
//! retained.
use super::*;
pub(super) struct AlignNotice {
    pub(super) flipped: usize,
}
impl Operandi {
    /// Immediate Align. The count shown comes from the same core plan the
    /// command executes, read just before it runs.
    pub(super) fn polygon_align(&mut self) {
        if self.has_modal() {
            self.status = "Close the open dialog before Align".into();
            return;
        }
        let count = self.editor.align_flip_count();
        let succeeded = self.run_ok(Command::AlignPolygons);
        if let (Ok(count), true) = (count, succeeded) {
            self.align_notice = Some(AlignNotice { flipped: count });
        }
    }
    pub(super) fn align_notice_dialog(&mut self, ctx: &Context) {
        let Some(notice) = &self.align_notice else {
            return;
        };
        let count = notice.flipped;
        let message = format!(
            "{count} polygon{} flipped.",
            if count == 1 { "" } else { "s" }
        );
        // The Return that submitted `poly.align` is ignored on the notice's
        // first frame (UIC-85).
        if self.show_notice(ctx, "Align Polygons", &message) != form::Outcome::Open {
            self.align_notice = None;
        }
    }
}
