//! Greyed controls explain themselves: hovering a greyed button, menu row
//! or field shows why it can't be used and what to do ("Select some
//! polygons first"). No reason means the control shouldn't be greyed.
//!
//! Every greyed egui control goes through [`Gate`]: it takes the reason
//! itself (`None` = available), so a control can't be greyed without one.
//! `gate_guard_tests` below fails if code calls egui's `add_enabled`,
//! `add_enabled_ui` or `disable` directly outside this module and the
//! form rows (which derive their reasons from the controlling row).
//! Custom-painted controls (toolbox buttons, pie slots, the native menu)
//! carry an `Option` reason in place of an `enabled` flag for the same
//! effect.
use eframe::egui::{InnerResponse, Response, Sense, Ui, Widget};

/// While a tool is live, edits elsewhere wait for it to finish.
pub(crate) const TOOL_ACTIVE: &str = "Drop or Cancel the active tool first";
/// Assembly > Lock stops edits to the item.
pub(crate) const LOCKED: &str = "The item is locked (Assembly > Lock)";
/// A numeric field holds text that isn't a valid value.
pub(crate) const FIELD_ERRORS: &str = "Correct or revert the invalid numeric fields first";

/// `Some(reason)` when `ok` is false: the gate for a control usable when
/// `ok` holds.
pub(crate) fn unless(ok: bool, reason: &str) -> Option<&str> {
    (!ok).then_some(reason)
}

/// The first reason of several conditions, in order (outer blockers first).
pub(crate) fn first<'a>(gates: impl IntoIterator<Item = Option<&'a str>>) -> Option<&'a str> {
    gates.into_iter().flatten().next()
}

/// Adds controls that can be greyed out, each with the reason it's greyed.
pub(crate) trait Gate {
    /// Adds `widget`, greyed with `why` as its hover text when `why` is
    /// set.
    fn gated(&mut self, why: Option<&str>, widget: impl Widget) -> Response;
    /// Runs `add` in a scope greyed with `why` as its hover text when `why`
    /// is set.
    fn gated_ui<R>(
        &mut self,
        why: Option<&str>,
        add: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R>;
}

impl Gate for Ui {
    fn gated(&mut self, why: Option<&str>, widget: impl Widget) -> Response {
        // Inside an already-greyed scope the outer reason is the real
        // blocker and shows from that scope; don't stack a second one.
        let outer_ok = self.is_enabled();
        let response = self.add_enabled(why.is_none(), widget);
        match why {
            Some(why) if outer_ok => response.on_disabled_hover_text(why),
            _ => response,
        }
    }
    fn gated_ui<R>(
        &mut self,
        why: Option<&str>,
        add: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let outer_ok = self.is_enabled();
        let InnerResponse { inner, response } = self.add_enabled_ui(why.is_none(), add);
        let response = match why {
            // egui gives the hover to the (greyed) child under the pointer,
            // not the scope, so the reason sits on a hover-only cover laid
            // over the scope. Nothing under it can be clicked anyway.
            Some(why) if outer_ok => {
                let cover = self.interact(response.rect, response.id.with("why"), Sense::hover());
                cover.on_hover_text(why);
                response
            }
            _ => response,
        };
        InnerResponse { inner, response }
    }
}

/// Greys `response` (a custom-painted control that already drew itself
/// greyed) with `why` as its hover text, or returns it unchanged.
pub(crate) fn explain(response: Response, why: Option<&str>) -> Response {
    match why {
        Some(why) => response.on_hover_text(why).on_disabled_hover_text(why),
        None => response,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Button, CentralPanel, Context, Event, Pos2, RawInput, Rect, vec2};

    fn run(ctx: &Context, at: Option<Pos2>, mut body: impl FnMut(&mut Ui)) {
        let _ = ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400., 300.))),
                events: at.map(Event::PointerMoved).into_iter().collect(),
                ..Default::default()
            },
            |ctx| {
                CentralPanel::default().show(ctx, |ui| body(ui));
            },
        );
    }

    #[test]
    fn a_greyed_scope_is_hovered_through_its_greyed_children() {
        // The reason's cover must see the pointer over a greyed child.
        let ctx = Context::default();
        let mut button = Rect::NOTHING;
        run(&ctx, None, |ui| {
            button = ui
                .gated_ui(Some("why"), |ui| ui.add(Button::new("Child")).rect)
                .inner;
        });
        let mut hovered = false;
        let mut child_enabled = true;
        for _ in 0..2 {
            run(&ctx, Some(button.center()), |ui| {
                let r = ui.gated_ui(Some("why"), |ui| ui.add(Button::new("Child")));
                hovered = ui
                    .ctx()
                    .read_response(r.response.id.with("why"))
                    .is_some_and(|c| c.hovered());
                child_enabled = r.inner.enabled();
            });
        }
        assert!(hovered, "the greyed scope must see the pointer");
        assert!(!child_enabled);
    }

    #[test]
    fn gates_grey_only_with_a_reason() {
        let ctx = Context::default();
        run(&ctx, None, |ui| {
            assert!(ui.gated(None, Button::new("a")).enabled());
            assert!(!ui.gated(Some("why"), Button::new("b")).enabled());
            let inner = ui
                .gated_ui(unless(false, "why"), |ui| ui.is_enabled())
                .inner;
            assert!(!inner);
            let inner = ui.gated_ui(unless(true, "why"), |ui| ui.is_enabled()).inner;
            assert!(inner);
        });
        assert_eq!(first([None, Some("a"), Some("b")]), Some("a"));
        assert_eq!(first([None, None]), None);
    }

    /// Source check: greyed controls go through [`Gate`] (or the form
    /// rows, whose reasons come from `form::Values::why_disabled`), never
    /// egui's raw enable switches, so none can be greyed without a reason.
    #[test]
    fn gate_guard_tests() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = Vec::new();
        let mut stack = vec![src.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let rel = path
                    .strip_prefix(&src)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if path.extension().is_none_or(|e| e != "rs")
                    || rel == "unavailable.rs"
                    || rel.ends_with("_tests.rs")
                    || rel == "tests.rs"
                {
                    continue;
                }
                // The form rows grey with a derived reason (form/mod.rs).
                let form_rows = rel == "form/dialog.rs" || rel == "form/panel.rs";
                let text = std::fs::read_to_string(&path).unwrap();
                for (n, line) in text.lines().enumerate() {
                    let code = line.split("//").next().unwrap_or("");
                    let raw = code.contains("add_enabled(")
                        || code.contains("add_enabled_ui(")
                        || code.contains(".set_enabled(") && !code.contains("os_color")
                        || (code.contains(".disable()") && !form_rows);
                    if raw {
                        offenders.push(format!("{rel}:{}: {}", n + 1, line.trim()));
                    }
                }
                // Tool Properties fields and buttons: `enabled_if(cond, "")`
                // would grey without a reason.
                let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
                let mut rest = compact.as_str();
                while let Some(at) = rest.find(".enabled_if(") {
                    let args = &rest[at + ".enabled_if(".len()..];
                    let mut depth = 1;
                    let close = args
                        .char_indices()
                        .find(|&(_, c)| {
                            depth += match c {
                                '(' => 1,
                                ')' => -1,
                                _ => 0,
                            };
                            depth == 0
                        })
                        .map_or(args.len(), |(i, _)| i);
                    let call = &args[..close];
                    if call.ends_with(",\"\"") || call.ends_with(",\"\",") {
                        offenders.push(format!("{rel}: enabled_if({call}) has no reason"));
                    }
                    rest = &args[close.min(args.len())..];
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "grey controls with unavailable::Gate (ui.gated / ui.gated_ui) so they \
             carry a reason:\n{}",
            offenders.join("\n")
        );
    }
}
