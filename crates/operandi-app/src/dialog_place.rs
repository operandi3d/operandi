//! Where dialogs open (Preferences > Input > Remapping > Open Dialogs under
//! Mouse). Off, a dialog opens in the middle of the window, as egui's
//! modals do. On, it opens with its top centre at the pointer, kept inside
//! the window, and stays there while it is open.
use eframe::egui::{self, Align2, Context, Id, Order, Pos2, Sense, UiKind};
use std::cell::Cell;

thread_local! {
    static UNDER_MOUSE: Cell<bool> = const { Cell::new(false) };
}

/// Set from the preference at launch and when it changes.
pub fn set_under_mouse(on: bool) {
    UNDER_MOUSE.with(|c| c.set(on));
}

/// A modal dialog with id `id`, placed by the preference.
pub fn modal(ctx: &Context, id: Id) -> egui::Modal {
    // Dialogs stay opaque whatever Popup Transparency says.
    let modal =
        egui::Modal::new(id).frame(egui::Frame::popup(&ctx.style()).fill(crate::theme::window()));
    if !UNDER_MOUSE.with(Cell::get) {
        return modal;
    }
    modal.area(area(ctx, id))
}

/// The dialog's area, pinned where the pointer was on its first frame.
fn area(ctx: &Context, id: Id) -> egui::Area {
    let key = id.with("dialog-opened-at");
    let frame = ctx.cumulative_frame_nr();
    let kept = ctx
        .data(|d| d.get_temp::<(Pos2, u64)>(key))
        .filter(|&(_, shown)| shown + 1 >= frame)
        .map(|(at, _)| at);
    let at = kept.unwrap_or_else(|| {
        ctx.input(|i| i.pointer.latest_pos())
            .unwrap_or_else(|| ctx.content_rect().center())
    });
    ctx.data_mut(|d| d.insert_temp(key, (at, frame)));
    egui::Area::new(id)
        .kind(UiKind::Modal)
        .sense(Sense::hover())
        .order(Order::Foreground)
        .interactable(true)
        .pivot(Align2::CENTER_TOP)
        .fixed_pos(at)
        .constrain(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Event, RawInput, Rect, pos2, vec2};

    fn show(ctx: &Context, pointer: Option<Pos2>) -> Rect {
        let mut rect = Rect::NOTHING;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.))),
            events: pointer.map(Event::PointerMoved).into_iter().collect(),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            rect = modal(ctx, Id::new("test dialog"))
                .show(ctx, |ui| {
                    ui.set_min_size(vec2(200., 100.));
                    ui.label("Dialog");
                })
                .response
                .rect;
        });
        rect
    }

    #[test]
    fn dialogs_open_mid_window_or_under_the_pointer() {
        set_under_mouse(false);
        let ctx = Context::default();
        let mut rect = Rect::NOTHING;
        for _ in 0..3 {
            rect = show(&ctx, Some(pos2(100., 100.)));
        }
        assert!((rect.center().x - 600.).abs() < 2., "centred: {rect:?}");

        set_under_mouse(true);
        let ctx = Context::default();
        for _ in 0..3 {
            rect = show(&ctx, Some(pos2(300., 200.)));
        }
        assert!((rect.center().x - 300.).abs() < 2., "{rect:?}");
        assert!((rect.top() - 200.).abs() < 2., "{rect:?}");
        // It stays put while open, whatever the pointer does.
        rect = show(&ctx, Some(pos2(900., 500.)));
        assert!((rect.center().x - 300.).abs() < 2., "{rect:?}");
        // Near the corner it is kept inside the window.
        let ctx = Context::default();
        for _ in 0..3 {
            rect = show(&ctx, Some(pos2(1190., 790.)));
        }
        assert!(rect.right() <= 1200.5 && rect.bottom() <= 800.5, "{rect:?}");
        set_under_mouse(false);
    }
}
