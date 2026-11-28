//! Dialog mode of the form component (UIC-80..99).
//!
//! A small modal window whose title is the command's name without "…"
//! (UIC-81), sized exactly to its rows and buttons. The form sits on the
//! left; a thin divider, then the button column on the right starting at
//! the top: OK (Ret), Cancel (Esc), then any extra buttons (UIC-83/84).
//! There is no heading, description, icon, group box or bottom bar; help
//! is in each row's tooltip (UIC-82).
//!
//! Keys (UIC-85): Return is OK and Escape is Cancel wherever focus is.
//! While typing in a field, the first Return commits the field and the
//! next one is OK. The Return that opened the dialog (from the command
//! line) is ignored.
use super::layout::{self, RowSize, dialog::*};
use super::widgets::{self, Fields};
use super::{Kind, Row, Values};
use crate::theme;
use eframe::egui::{
    self, Align2, Context, CornerRadius, Frame, Id, Key, Rect, Sense, Stroke, TextStyle, UiBuilder,
    pos2, vec2,
};

/// What the dialog's buttons and keys asked for this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Still open; nothing pressed.
    Open,
    /// OK or Return. The caller runs the command once (UIC-96) and closes
    /// the dialog, or keeps it open with an error to retry.
    Ok,
    /// Cancel, Escape or a click outside: nothing changes (UIC-96).
    Cancel,
    /// One of the extra buttons, by index (Apply, Reset, UIC-84).
    Extra(usize),
    /// A question's No (its Escape); see `Dialog::question`.
    No,
}

/// Where one row's label and field were drawn (layout tests).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RowRects {
    pub(crate) label: Option<Rect>,
    pub(crate) field: Rect,
}

/// One open dialog: its rows, their current values and its field state.
#[derive(Clone)]
pub(crate) struct Dialog {
    /// The command's name without "…" (UIC-81).
    pub(crate) title: String,
    pub(crate) rows: Vec<Row>,
    pub(crate) values: Values,
    /// The last refusal (bad number, failed command); shown as the OK
    /// button's tooltip and in the status line by the caller. The dialog
    /// stays open so the user can fix it.
    pub(crate) error: Option<String>,
    /// Everything else, boxed: the app keeps several dialogs inline in its
    /// (already large) state, so each stays small.
    p: Box<Private>,
}

/// A dialog's own state.
#[derive(Clone, Default)]
struct Private {
    /// Extra buttons under Cancel (UIC-84).
    extras: Vec<String>,
    /// Buttons shown: OK and Cancel, or OK alone for a notice.
    cancel: bool,
    /// Yes / No / Cancel instead (`Dialog::question`).
    question: bool,
    fields: Fields,
    /// Frames shown so far (the opening Return is ignored, UIC-85).
    frames: u32,
    /// OK is greyed while this holds a reason (shown as its tooltip).
    ok_blocked: Option<String>,
    /// Button rows clicked since the caller last asked (UIC-116).
    clicked: Vec<String>,
    /// Each row's label and field rects, as last drawn.
    rects: Vec<RowRects>,
}

impl Dialog {
    /// A dialog for `title` with `rows` at `values`.
    pub(crate) fn new(title: &str, rows: Vec<Row>, values: Values) -> Self {
        debug_assert_eq!(super::check_rows(&rows), Ok(()), "{title}");
        Self {
            title: title.trim_end_matches('…').trim_end_matches("...").into(),
            rows,
            values,
            error: None,
            p: Box::new(Private {
                cancel: true,
                ..Private::default()
            }),
        }
    }

    /// A dialog for `title` with every row at its default.
    pub(crate) fn with_defaults(title: &str, rows: Vec<Row>) -> Self {
        let values = Values::defaults(&rows);
        Self::new(title, rows, values)
    }

    /// A confirmation: `message` as plain text, OK and Cancel (UIC-97).
    pub(crate) fn confirm(title: &str, message: &str) -> Self {
        Self::with_defaults(title, vec![Row::message(message)])
    }

    /// An acknowledgement: `message` and OK alone (Escape still closes).
    pub(crate) fn notice(title: &str, message: &str) -> Self {
        let mut dialog = Self::confirm(title, message);
        dialog.p.cancel = false;
        dialog
    }

    /// A Yes / No / Cancel question (Jason, 2026-11-08; a deviation from
    /// UIC-83/84/85/97, which give OK and Cancel only). The buttons stack
    /// in the right-hand column as usual (UIC-83), each with its key at
    /// its right: Yes = Return (Outcome::Ok), No = Escape (Outcome::No),
    /// Cancel = Shift+Escape (Outcome::Cancel, changes nothing, UIC-96).
    /// Here Escape answers No rather than cancelling. A click outside
    /// still cancels (UIC-82).
    pub(crate) fn question(title: &str, message: &str) -> Self {
        let mut dialog = Self::confirm(title, message);
        dialog.p.question = true;
        dialog
    }

    /// Any dialog as a Yes / No / Cancel question (see `question`), for one
    /// with rows besides its message.
    pub(crate) fn asking(mut self) -> Self {
        self.p.question = true;
        self
    }

    /// Adds buttons under Cancel (UIC-84).
    pub(crate) fn extras(mut self, names: &[&str]) -> Self {
        self.p.extras = names.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Puts the keyboard in `key`'s field when the dialog opens.
    pub(crate) fn focus(mut self, key: &str) -> Self {
        self.p.fields.focus = Some(key.into());
        self
    }

    /// Focus a name field and select its existing text when the dialog opens.
    pub(crate) fn focus_name(mut self, key: &str) -> Self {
        self.p.fields.focus = Some(key.into());
        self.p.fields.select_on_focus = true;
        self
    }

    /// Greys OK with `reason` as its tooltip, or ungreys it.
    pub(crate) fn block_ok(&mut self, reason: Option<String>) {
        self.p.ok_blocked = reason;
    }

    /// The labels of the buttons, top to bottom.
    fn buttons(&self) -> Vec<(&str, Option<&str>)> {
        if self.p.question {
            return vec![
                ("Yes", Some("Ret")),
                ("No", Some("Esc")),
                ("Cancel", Some("Shift+Esc")),
            ];
        }
        let mut buttons = vec![("OK", Some("Ret"))];
        if self.p.cancel {
            buttons.push(("Cancel", Some("Esc")));
        }
        buttons.extend(self.p.extras.iter().map(|e| (e.as_str(), None)));
        buttons
    }

    /// Each row's label and field rects, as last drawn.
    pub(crate) fn row_rects(&self) -> &[RowRects] {
        &self.p.rects
    }

    /// Was the button row `key` clicked (since last asked)?
    pub(crate) fn take_clicked(&mut self, key: &str) -> bool {
        let before = self.p.clicked.len();
        self.p.clicked.retain(|k| k != key);
        self.p.clicked.len() != before
    }

    /// The text fields drawn last frame (so the app keeps shortcuts out
    /// of them).
    pub(crate) fn text_ids(&self) -> &[Id] {
        &self.p.fields.text_ids
    }

    /// Shows a dialog the caller doesn't keep itself (a confirmation behind
    /// a flag): `make` builds it on the first frame and egui keeps it until
    /// OK or Cancel.
    pub(crate) fn show_kept(ctx: &Context, key: &str, make: impl FnOnce() -> Self) -> Outcome {
        let id = Id::new(("form-dialog-kept", key));
        let frame = ctx.cumulative_frame_nr();
        // A dialog not shown last frame was closed some other way: start
        // it afresh (its message may have changed).
        let mut dialog = ctx
            .data_mut(|d| d.get_temp::<(Self, u64)>(id))
            .filter(|(_, shown)| shown + 1 >= frame)
            .map(|(dialog, _)| dialog)
            .unwrap_or_else(make);
        let outcome = dialog.show(ctx);
        if outcome == Outcome::Open {
            ctx.data_mut(|d| d.insert_temp(id, (dialog, frame)));
        } else {
            ctx.data_mut(|d| d.remove::<(Self, u64)>(id));
        }
        outcome
    }

    fn id(&self) -> Id {
        Id::new(("form-dialog", &self.title))
    }

    /// Draws the dialog for one frame and reports what was pressed.
    pub(crate) fn show(&mut self, ctx: &Context) -> Outcome {
        // Was a field being typed in when this frame began? Then Return
        // only commits it (UIC-85).
        // N12: only once something was typed; a field focused on opening
        // (Set Material's Name) lets the first Return be OK.
        let focused = ctx
            .memory(|m| m.focused())
            .is_some_and(|f| self.p.fields.text_ids.contains(&f));
        if !focused {
            self.p.fields.edited = false;
        }
        let typing = focused && self.p.fields.typed();
        self.p.fields.text_ids.clear();
        let first = self.p.frames == 0;
        self.p.frames = self.p.frames.saturating_add(1);
        let frame = Frame::NONE
            .fill(theme::window())
            .stroke(Stroke::new(1.0f32, theme::border()))
            .corner_radius(CornerRadius::same(4));
        let id = self.id();
        let mut pressed = None;
        // A question's keys go before egui's modal sees Escape (which it
        // would take as a click outside): Shift+Escape cancels, Escape
        // answers No. Shift+Escape is checked first, since a plain Escape
        // pattern also matches with Shift held.
        if self.p.question {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, Key::Escape)) {
                pressed = Some(Outcome::Cancel);
            } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
                pressed = Some(Outcome::No);
            }
        }
        let modal = crate::dialog_place::modal(ctx, id)
            .frame(frame)
            .show(ctx, |ui| {
                let clicked = self.contents(ui);
                pressed = pressed.or(clicked);
            });
        let mut outcome = pressed.unwrap_or(Outcome::Open);
        if outcome == Outcome::Open && modal.should_close() {
            outcome = Outcome::Cancel;
        }
        // On its first frame egui doesn't count the dialog as the top modal
        // yet; Escape still cancels it (UIC-85), unless a list is open.
        if outcome == Outcome::Open
            && !modal.any_popup_open
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape))
        {
            outcome = Outcome::Cancel;
        }
        let enter = ctx.input(|i| i.key_pressed(Key::Enter));
        if outcome == Outcome::Open && enter && !first && !typing && !modal.any_popup_open {
            outcome = if self.p.ok_blocked.is_some() {
                Outcome::Open
            } else {
                Outcome::Ok
            };
        }
        if outcome == Outcome::Ok {
            // OK while a number is still being typed commits it first.
            if let Err(e) = self.p.fields.commit(&self.rows, &mut self.values) {
                self.error = Some(e);
                outcome = Outcome::Open;
            }
        }
        if let Some(e) = self.p.fields.error.take() {
            self.error = Some(e);
        }
        if outcome != Outcome::Open {
            // A closing dialog lets go of the keyboard at once, so the next
            // shortcut isn't taken for typing into a field that's gone.
            ctx.memory_mut(|m| {
                for id in &self.p.fields.text_ids {
                    m.surrender_focus(*id);
                }
            });
        }
        if outcome == Outcome::Cancel {
            self.p.fields = Fields::default();
        }
        outcome
    }

    /// Measures each row for the layout.
    fn measure(&self, ui: &egui::Ui) -> Vec<RowSize> {
        self.rows
            .iter()
            .map(|row| {
                let spans = row.kind == Kind::Message;
                let field = widgets::natural_width(ui, row);
                RowSize {
                    label: if row.label_in_column() {
                        widgets::text_width(ui, &row.label)
                    } else {
                        0.
                    },
                    field,
                    height: if spans {
                        widgets::message_height(ui, &row.label, field.max(1.))
                    } else {
                        // Min/max labels sit under the field.
                        ROW_HEIGHT + row.label_extra()
                    },
                    spans,
                }
            })
            .collect()
    }

    fn contents(&mut self, ui: &mut egui::Ui) -> Option<Outcome> {
        let sizes = self.measure(ui);
        let buttons: Vec<(String, Option<String>)> = self
            .buttons()
            .into_iter()
            .map(|(a, b)| (a.to_owned(), b.map(str::to_owned)))
            .collect();
        // UIC-83/84: one column, every button its width. It grows to the
        // widest label so none runs past its edge ("Skip This Build",
        // Leo 2026-11-27); the table's width stays the minimum.
        let button_width = buttons
            .iter()
            .map(|(name, hint)| widgets::button_width_for(ui, name, hint.as_deref()))
            .fold(
                if self.p.question {
                    QUESTION_BUTTON_WIDTH
                } else {
                    BUTTON_WIDTH
                },
                f32::max,
            );
        let layout = layout::dialog_layout_with(&sizes, buttons.len(), button_width);
        let (whole, _) = ui.allocate_exact_size(
            vec2(layout.width, TITLE_HEIGHT + layout.height),
            Sense::hover(),
        );
        // The title bar (UIC-81): the command's name, nothing else.
        let title = Rect::from_min_size(whole.min, vec2(whole.width(), TITLE_HEIGHT));
        let painter = ui.painter();
        painter.rect_filled(
            title,
            CornerRadius {
                nw: 4,
                ne: 4,
                sw: 0,
                se: 0,
            },
            theme::panel(),
        );
        painter.hline(
            title.x_range(),
            title.bottom() - 0.5,
            Stroke::new(1.0f32, theme::border()),
        );
        painter.text(
            title.center(),
            Align2::CENTER_CENTER,
            &self.title,
            TextStyle::Body.resolve(ui.style()),
            theme::text(),
        );
        ui.interact(title, ui.id().with("title"), Sense::hover())
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &self.title));
        let origin = pos2(whole.left(), title.bottom());
        let mut rects = Vec::with_capacity(self.rows.len());
        let at =
            |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(origin + vec2(x, y), vec2(w, h));
        for (i, row) in self.rows.iter().enumerate() {
            let top = layout.row_tops[i];
            let height = sizes[i].height.max(ROW_HEIGHT);
            // A message fills its height; other rows keep their label and
            // field in the top ROW_HEIGHT (min/max labels go below).
            let cell = if row.kind == Kind::Message {
                height
            } else {
                ROW_HEIGHT
            };
            let why = self.values.why_disabled(row, &self.rows);
            let enabled = why.is_none();
            let row_rect = at(PADDING, top, layout.divider_x - PADDING, height);
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(row_rect)
                    .id_salt(("form-row", &row.key)),
            );
            if !enabled {
                // UIC-92: greyed in place, stops responding.
                child.disable();
            }
            let mut label_rect = None;
            if row.label_in_column() {
                let label = at(PADDING, top, layout.label_column, cell);
                label_rect = Some(widgets::label(&mut child, label, &row.label, &row.tooltip).rect);
            }
            let field = if row.kind == Kind::Message {
                at(
                    PADDING,
                    top,
                    layout.field_x + layout.field_width - PADDING,
                    height,
                )
            } else {
                at(layout.field_x, top, layout.field_width, cell)
            };
            rects.push(RowRects {
                label: label_rect,
                field,
            });
            let mut value = self
                .values
                .get(&row.key)
                .cloned()
                .unwrap_or_else(|| row.default.clone());
            let field_id = child.id().with("field");
            let out = widgets::field(
                &mut child,
                field,
                field_id,
                row,
                &mut value,
                &mut self.p.fields,
                false,
            );
            if row.kind == Kind::Button && out.changed {
                self.p.clicked.push(row.key.clone());
            }
            widgets::hover(out.response, &row.tooltip);
            if let Some(why) = why {
                // Over the greyed row, the reason (unavailable.rs).
                child
                    .interact(row_rect, child.id().with("why"), Sense::hover())
                    .on_disabled_hover_text(why);
            }
            if row.default != super::Value::None {
                self.values.set(&row.key, value);
            }
        }
        self.p.rects = rects;
        // UIC-83: a thin divider, then the buttons.
        ui.painter().vline(
            origin.x + layout.divider_x + 0.5,
            (origin.y + PADDING)..=(origin.y + layout.height - PADDING),
            Stroke::new(1.0f32, theme::border()),
        );
        let mut pressed = None;
        for (i, (name, hint)) in buttons.iter().enumerate() {
            let rect = at(
                layout.button_x,
                layout.button_tops[i],
                button_width,
                BUTTON_HEIGHT,
            );
            let ok = i == 0;
            let enabled = !(ok && self.p.ok_blocked.is_some());
            let mut child = ui.new_child(UiBuilder::new().max_rect(rect).id_salt(("button", i)));
            if !enabled {
                child.disable();
            }
            let mut response = child.interact(rect, child.id().with("b"), Sense::click());
            widgets::paint_button(&child, rect, &response, name, hint.as_deref(), ok);
            let tip = if ok {
                self.p.ok_blocked.clone().or_else(|| self.error.clone())
            } else {
                None
            };
            if let Some(tip) = tip {
                response = response
                    .on_hover_text(tip.clone())
                    .on_disabled_hover_text(tip);
            }
            if enabled && response.clicked() {
                pressed = Some(match (i, self.p.cancel) {
                    (0, _) => Outcome::Ok,
                    (1, _) if self.p.question => Outcome::No,
                    (2, _) if self.p.question => Outcome::Cancel,
                    (1, true) => Outcome::Cancel,
                    (n, true) => Outcome::Extra(n - 2),
                    (n, false) => Outcome::Extra(n - 1),
                });
            }
        }
        pressed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::form::{Unit, Value, When};
    use eframe::egui::{Event, Modifiers, Pos2, RawInput, accesskit::Role};

    fn rows() -> Vec<Row> {
        vec![
            Row::number("size", "Size", Unit::Scalar, 1.),
            Row::check("fixed", "Fixed", false),
            Row::number("distance", "Distance", Unit::Scalar, 2.).when(When::Checked("fixed")),
        ]
    }

    fn key(k: Key) -> Event {
        Event::Key {
            key: k,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    /// One frame; returns the outcome and the accessibility nodes.
    fn frame(
        ctx: &Context,
        dialog: &mut Dialog,
        events: Vec<Event>,
    ) -> (Outcome, Vec<egui::accesskit::Node>) {
        let mut outcome = Outcome::Open;
        let out = ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000., 700.))),
                events,
                ..Default::default()
            },
            |ctx| outcome = dialog.show(ctx),
        );
        let nodes = out
            .platform_output
            .accesskit_update
            .map(|u| u.nodes.into_iter().map(|(_, n)| n).collect())
            .unwrap_or_default();
        (outcome, nodes)
    }

    fn context() -> Context {
        let ctx = Context::default();
        ctx.enable_accesskit();
        ctx
    }

    fn find<'a>(
        nodes: &'a [egui::accesskit::Node],
        role: Role,
        name: &str,
    ) -> &'a egui::accesskit::Node {
        nodes
            .iter()
            .find(|n| n.role() == role && n.label().or(n.value()) == Some(name))
            .unwrap_or_else(|| panic!("{role:?} {name}"))
    }

    fn rect(n: &egui::accesskit::Node) -> Rect {
        let b = n.bounds().unwrap();
        Rect::from_min_max(
            Pos2::new(b.x0 as f32, b.y0 as f32),
            Pos2::new(b.x1 as f32, b.y1 as f32),
        )
    }

    fn click(ctx: &Context, dialog: &mut Dialog, at: Pos2) -> Outcome {
        frame(ctx, dialog, vec![Event::PointerMoved(at)]);
        frame(
            ctx,
            dialog,
            vec![Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            }],
        );
        frame(
            ctx,
            dialog,
            vec![Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            }],
        )
        .0
    }

    #[test]
    fn the_opening_return_is_ignored_and_the_next_is_ok() {
        // UIC-85.
        let ctx = context();
        let mut d = Dialog::with_defaults("Test…", rows());
        assert_eq!(d.title, "Test", "UIC-81: no ellipsis in the title");
        assert_eq!(frame(&ctx, &mut d, vec![key(Key::Enter)]).0, Outcome::Open);
        assert_eq!(frame(&ctx, &mut d, vec![key(Key::Enter)]).0, Outcome::Ok);
    }

    /// N12: a text field focused on opening (Set Material's Name) but not
    /// typed in lets the first Return be OK; once typed in, Return commits
    /// first (UIC-85).
    #[test]
    fn a_focused_untouched_text_field_lets_return_be_ok() {
        let rows = || {
            vec![
                Row::text("name", "Name", "Default").suggestions(vec!["Metal".into()]),
                Row::check("smoothing", "Smoothing", true),
            ]
        };
        let ctx = context();
        let mut d = Dialog::with_defaults("Test", rows()).focus("name");
        frame(&ctx, &mut d, vec![]);
        frame(&ctx, &mut d, vec![]);
        assert_eq!(frame(&ctx, &mut d, vec![key(Key::Enter)]).0, Outcome::Ok);
        let ctx = context();
        let mut d = Dialog::with_defaults("Test", rows()).focus("name");
        frame(&ctx, &mut d, vec![]);
        frame(&ctx, &mut d, vec![Event::Text("x".into())]);
        assert_eq!(frame(&ctx, &mut d, vec![key(Key::Enter)]).0, Outcome::Open);
        assert_eq!(frame(&ctx, &mut d, vec![key(Key::Enter)]).0, Outcome::Ok);
    }

    #[test]
    fn while_typing_return_commits_the_field_then_is_ok() {
        // UIC-85.
        let ctx = context();
        let mut d = Dialog::with_defaults("Test", rows()).focus("size");
        frame(&ctx, &mut d, vec![]);
        frame(&ctx, &mut d, vec![key(Key::A)]);
        let select_all = Event::Key {
            key: Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        };
        frame(&ctx, &mut d, vec![select_all, Event::Text("7".into())]);
        assert_eq!(frame(&ctx, &mut d, vec![key(Key::Enter)]).0, Outcome::Open);
        assert_eq!(d.values.number("size"), 7.);
        assert_eq!(frame(&ctx, &mut d, vec![key(Key::Enter)]).0, Outcome::Ok);
    }

    /// A frame as the app runs it: Tab is taken before egui sees it and
    /// carried out at the start of the pass (numeric_tab, UIC-51).
    fn tab_frame(ctx: &Context, dialog: &mut Dialog, events: Vec<Event>) {
        let mut raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000., 700.))),
            events,
            ..Default::default()
        };
        crate::numeric_tab::take_tab(ctx, &mut raw);
        let _ = ctx.run(raw, |ctx| {
            crate::numeric_tab::begin_pass(ctx);
            dialog.show(ctx);
        });
    }

    #[test]
    fn tab_moves_between_text_and_number_fields_shift_tab_back() {
        // UIC-51, widened to text fields (Jason, 2026-11-10). The checkbox
        // between them is skipped; typing replaces the selected value.
        let ctx = context();
        let rows = vec![
            Row::text("name", "Name", "Part"),
            Row::check("fixed", "Fixed", false),
            Row::number("size", "Size", Unit::Scalar, 1.),
        ];
        let mut d = Dialog::with_defaults("Test", rows).focus("name");
        let tab = |shift: bool| Event::Key {
            key: Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: if shift {
                Modifiers::SHIFT
            } else {
                Modifiers::NONE
            },
        };
        tab_frame(&ctx, &mut d, vec![]);
        tab_frame(&ctx, &mut d, vec![]);
        let ids = d.text_ids().to_vec();
        assert_eq!(ids.len(), 2);
        let focused = || ctx.memory(|m| m.focused());
        assert_eq!(focused(), Some(ids[0]));
        tab_frame(&ctx, &mut d, vec![tab(false)]);
        tab_frame(&ctx, &mut d, vec![]);
        assert_eq!(focused(), Some(ids[1]), "Tab skips the checkbox");
        tab_frame(&ctx, &mut d, vec![Event::Text("7".into())]);
        tab_frame(&ctx, &mut d, vec![tab(true)]);
        tab_frame(&ctx, &mut d, vec![]);
        assert_eq!(d.values.number("size"), 7., "Tab accepted the typed value");
        assert_eq!(focused(), Some(ids[0]), "Shift+Tab goes back");
        tab_frame(&ctx, &mut d, vec![Event::Text("Lid".into())]);
        tab_frame(&ctx, &mut d, vec![tab(false)]);
        tab_frame(&ctx, &mut d, vec![]);
        assert_eq!(
            d.values.text("name"),
            "Lid",
            "typing replaced the selection"
        );
        assert_eq!(focused(), Some(ids[1]));
        // Wraps at the end.
        tab_frame(&ctx, &mut d, vec![tab(false)]);
        tab_frame(&ctx, &mut d, vec![]);
        assert_eq!(focused(), Some(ids[0]));
    }

    #[test]
    fn escape_cancels_wherever_focus_is() {
        let ctx = context();
        let mut d = Dialog::with_defaults("Test", rows()).focus("size");
        frame(&ctx, &mut d, vec![]);
        assert_eq!(
            frame(&ctx, &mut d, vec![key(Key::Escape)]).0,
            Outcome::Cancel
        );
    }

    #[test]
    fn layout_buttons_and_greyed_rows() {
        let ctx = context();
        let mut d = Dialog::with_defaults("Test", rows());
        frame(&ctx, &mut d, vec![]);
        let (_, nodes) = frame(&ctx, &mut d, vec![]);
        // UIC-83/84: OK over Cancel, right of the fields, starting at the top.
        let ok = rect(find(&nodes, Role::Button, "OK"));
        let cancel = rect(find(&nodes, Role::Button, "Cancel"));
        assert!((ok.left() - cancel.left()).abs() < 0.5 && ok.bottom() < cancel.top());
        assert_eq!(ok.width(), layout::dialog::BUTTON_WIDTH);
        let size = rect(find(&nodes, Role::Label, "Size"));
        let distance = rect(find(&nodes, Role::Label, "Distance"));
        // UIC-86: labels share one right edge.
        assert!((size.right() - distance.right()).abs() < 0.5);
        assert!((ok.top() - size.top()).abs() < 4.);
        // UIC-87: the checkbox has no left label; its box starts the field
        // column.
        let check = rect(find(&nodes, Role::CheckBox, "Fixed"));
        assert!(check.left() > size.right());
        // UIC-92: Distance is greyed until Fixed is ticked, never hidden.
        let field = |nodes: &[egui::accesskit::Node], i: usize| {
            nodes
                .iter()
                .filter(|n| n.role() == Role::TextInput)
                .nth(i)
                .cloned()
                .unwrap()
        };
        let mut texts: Vec<_> = nodes
            .iter()
            .filter(|n| n.role() == Role::TextInput)
            .cloned()
            .collect();
        texts.sort_by(|a, b| rect(a).top().total_cmp(&rect(b).top()));
        assert!(texts[1].is_disabled(), "Distance greyed");
        assert!(!field(&nodes, 0).is_disabled() || !texts[0].is_disabled());
        click(&ctx, &mut d, check.center());
        assert_eq!(d.values.get("fixed"), Some(&Value::Bool(true)));
        let (_, nodes) = frame(&ctx, &mut d, vec![]);
        let mut texts: Vec<_> = nodes
            .iter()
            .filter(|n| n.role() == Role::TextInput)
            .cloned()
            .collect();
        texts.sort_by(|a, b| rect(a).top().total_cmp(&rect(b).top()));
        assert!(!texts[1].is_disabled(), "Distance live once Fixed is on");
        // UIC-96: OK reports once; the caller runs the command.
        assert_eq!(click(&ctx, &mut d, ok.center()), Outcome::Ok);
    }

    #[test]
    fn every_number_kind_has_its_arrows_inside_the_right_end() {
        // UIC-53/90/113: the arrows sit inside the field at its right end;
        // the value reads from the left of the rest. Every unit, and the
        // X/Y/Z rows of a vector, use the same number kind.
        let ctx = context();
        let mut rows = vec![
            Row::number("dist", "Dist", Unit::Distance, 1.),
            Row::number("ang", "Ang", Unit::Angle, 0.),
            Row::number("pct", "Pct", Unit::Percent, 50.),
            Row::number("fac", "Fac", Unit::Factor, 0.5),
            Row::number("cnt", "Cnt", Unit::Count, 3.),
            Row::number("sca", "Sca", Unit::Scalar, 2.),
            Row::number("suf", "Suf", Unit::Suffix(" px"), 4.),
        ];
        rows.extend(super::super::panel::vector_rows(
            "pos",
            "Position",
            Unit::Distance,
            [0.; 3],
        ));
        let mut d = Dialog::with_defaults("Test", rows.clone());
        frame(&ctx, &mut d, vec![]);
        let (_, nodes) = frame(&ctx, &mut d, vec![]);
        for row in &rows {
            let arrows = rect(find(&nodes, Role::Slider, &format!("{} arrows", row.label)));
            let text = nodes
                .iter()
                .filter(|n| n.role() == Role::TextInput)
                .map(rect)
                .find(|t| (t.center().y - arrows.center().y).abs() < 1.)
                .unwrap_or_else(|| panic!("{}: no value box in its row", row.label));
            assert!(
                text.right() <= arrows.left() + 0.5,
                "{}: value {text:?} runs into the arrows {arrows:?}",
                row.label
            );
            assert!(
                arrows.left() > text.center().x && arrows.width() < 20.,
                "{}: arrows {arrows:?} are not at the right end of {text:?}",
                row.label
            );
        }
    }

    #[test]
    fn every_number_row_has_its_arrows_greyed_with_the_row() {
        // UIC-90/113: floats and whole numbers alike; greyed rows keep
        // their arrows, greyed. The arrows are part of the number kind,
        // so no caller can leave them out.
        let ctx = context();
        let mut rows = rows();
        rows.push(Row::number("count", "Count", Unit::Count, 3.));
        rows.push(
            Row::number("angle", "Angle", Unit::Angle, 0.)
                .when(When::Never)
                .tip("Not built yet"),
        );
        let mut d = Dialog::with_defaults("Test", rows.clone());
        frame(&ctx, &mut d, vec![]);
        let (_, nodes) = frame(&ctx, &mut d, vec![]);
        for row in rows
            .iter()
            .filter(|r| matches!(r.kind, Kind::Number { .. }))
        {
            let arrows = find(&nodes, Role::Slider, &format!("{} arrows", row.label));
            let greyed = !d.values.enabled(row);
            assert_eq!(arrows.is_disabled(), greyed, "{}", row.label);
        }
    }

    #[test]
    fn a_blocked_ok_ignores_return_and_clicks() {
        let ctx = context();
        let mut d = Dialog::with_defaults("Test", rows());
        d.block_ok(Some("Type a name".into()));
        frame(&ctx, &mut d, vec![]);
        assert_eq!(frame(&ctx, &mut d, vec![key(Key::Enter)]).0, Outcome::Open);
        let (_, nodes) = frame(&ctx, &mut d, vec![]);
        let ok = find(&nodes, Role::Button, "OK");
        assert!(ok.is_disabled());
        let at = rect(ok).center();
        assert_eq!(click(&ctx, &mut d, at), Outcome::Open);
    }

    #[test]
    fn confirmations_and_notices() {
        // UIC-97: message as plain text, OK / Cancel (+ extras).
        let ctx = context();
        let mut d = Dialog::confirm("Close All", "Close all open scenes?").extras(&["Discard"]);
        frame(&ctx, &mut d, vec![]);
        let (_, nodes) = frame(&ctx, &mut d, vec![]);
        find(&nodes, Role::Label, "Close all open scenes?");
        let discard = rect(find(&nodes, Role::Button, "Discard"));
        let cancel = rect(find(&nodes, Role::Button, "Cancel"));
        assert!(discard.top() > cancel.bottom(), "extras under Cancel");
        assert_eq!(click(&ctx, &mut d, discard.center()), Outcome::Extra(0));
        let mut n = Dialog::notice("Align Polygons", "2 polygons flipped.");
        let (_, nodes) = frame(&ctx, &mut n, vec![]);
        assert!(
            !nodes
                .iter()
                .any(|x| x.role() == Role::Button && x.label() == Some("Cancel"))
        );
    }

    #[test]
    fn a_long_extra_button_label_fits_its_button() {
        // Leo, 2026-11-27: "Skip This Build" ran past its button's edge
        // in the Update Available dialog. The column grows to the widest
        // label; 70 stays the minimum, so short labels look as before.
        let ctx = context();
        let mut d = Dialog::confirm("Update Available", "A new build is available.")
            .extras(&["Skip This Build"]);
        frame(&ctx, &mut d, vec![]);
        let (_, nodes) = frame(&ctx, &mut d, vec![]);
        let skip = rect(find(&nodes, Role::Button, "Skip This Build"));
        let ok = rect(find(&nodes, Role::Button, "OK"));
        // At the body size "Skip This Build" is about 90 pt of text.
        assert!(
            skip.width() > layout::dialog::BUTTON_WIDTH + 10.,
            "wider than the plain column: {}",
            skip.width()
        );
        assert_eq!(ok.width(), skip.width(), "one column, one width (UIC-84)");
        let mut short = Dialog::confirm("Close All", "Close all open scenes?").extras(&["Discard"]);
        frame(&ctx, &mut short, vec![]);
        let (_, nodes) = frame(&ctx, &mut short, vec![]);
        let ok = rect(find(&nodes, Role::Button, "OK"));
        assert_eq!(
            ok.width(),
            layout::dialog::BUTTON_WIDTH,
            "short labels: as before"
        );
    }

    #[test]
    fn questions_answer_yes_no_cancel_by_button_and_key() {
        // Jason, 2026-11-08 (GL-Q7): Yes / No / Cancel stacked in the button
        // column (UIC-83), Return = Yes, Escape = No, Shift+Escape = Cancel.
        let ctx = context();
        let ask = || Dialog::question("Delete Item", "Delete the child items too?");
        let mut d = ask();
        frame(&ctx, &mut d, vec![]);
        let (_, nodes) = frame(&ctx, &mut d, vec![]);
        find(&nodes, Role::Label, "Delete the child items too?");
        assert!(
            !nodes
                .iter()
                .any(|x| x.role() == Role::Button && x.label() == Some("OK"))
        );
        let yes = rect(find(&nodes, Role::Button, "Yes"));
        let no = rect(find(&nodes, Role::Button, "No"));
        let cancel = rect(find(&nodes, Role::Button, "Cancel"));
        assert!(
            yes.bottom() < no.top() && no.bottom() < cancel.top(),
            "stacked"
        );
        assert!((yes.left() - no.left()).abs() < 0.5 && (no.left() - cancel.left()).abs() < 0.5);
        assert!(
            yes.width() >= QUESTION_BUTTON_WIDTH - 0.5,
            "room for the keys"
        );
        let message = rect(find(&nodes, Role::Label, "Delete the child items too?"));
        assert!(yes.left() > message.right(), "buttons on the right");
        assert_eq!(click(&ctx, &mut d, no.center()), Outcome::No);
        let mut d = ask();
        frame(&ctx, &mut d, vec![]);
        assert_eq!(click(&ctx, &mut d, cancel.center()), Outcome::Cancel);

        let shift_escape = Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::SHIFT,
        };
        for (event, want) in [
            (key(Key::Enter), Outcome::Ok),
            (key(Key::Escape), Outcome::No),
            (shift_escape, Outcome::Cancel),
        ] {
            let mut d = ask();
            frame(&ctx, &mut d, vec![]);
            frame(&ctx, &mut d, vec![]);
            let (got, _) = frame(&ctx, &mut d, vec![event]);
            assert_eq!(got, want);
        }
        // A plain confirmation's Escape still cancels (UIC-85).
        let mut c = Dialog::confirm("Close All", "Close all open scenes?");
        frame(&ctx, &mut c, vec![]);
        let (got, _) = frame(&ctx, &mut c, vec![key(Key::Escape)]);
        assert_eq!(got, Outcome::Cancel);
    }

    #[test]
    fn a_bad_number_keeps_the_dialog_open_with_its_text() {
        let ctx = context();
        let mut d = Dialog::with_defaults("Test", rows()).focus("size");
        frame(&ctx, &mut d, vec![]);
        let select_all = Event::Key {
            key: Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        };
        frame(&ctx, &mut d, vec![select_all, Event::Text("abc".into())]);
        frame(&ctx, &mut d, vec![key(Key::Enter)]);
        assert!(d.error.is_some());
        assert_eq!(d.values.number("size"), 1.);
        // OK refuses while the text can't be read.
        assert_eq!(frame(&ctx, &mut d, vec![key(Key::Enter)]).0, Outcome::Open);
    }
}
