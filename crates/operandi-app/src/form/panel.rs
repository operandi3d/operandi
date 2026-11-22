//! Panel mode of the form component (UIC-100..124, UIC-125).
//!
//! Tabs → sections → rows. Several tabs show as a vertical strip on the
//! right with each tab's full name rotated, the active one filled with the
//! accent (UIC-100); the scroll bar sits between the content and the strip
//! (UIC-102). Sections are darker foldable bars with a triangle, and their
//! fold state is kept across restarts by the caller (UIC-105..107). Each
//! row is label | status light | field (UIC-110..112): the light is a
//! plain ring at the default and "Constant" (filled) once changed; rows
//! that are not channels leave its space empty so every field edge lines
//! up.
//! All fields are one width (UIC-113); every choice is a dropdown here.
//!
//! The Presets / One Tab / All Tabs header row (UIC-103) is not built yet;
//! `HEADER` in `layout::panel` reserves its height for when it is.
use super::layout::{self, panel::*};
use super::status::ChannelStatus;
use super::widgets::{self, Fields};
use super::{Kind, Row, Unit, Value, Values};
use crate::theme;
use eframe::egui::{
    self, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, TextStyle, Ui, UiBuilder,
    epaint::TextShape, pos2, vec2,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A foldable group of rows; `name: None` is rows with no bar.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Section {
    pub(crate) name: Option<String>,
    pub(crate) rows: Vec<Row>,
}

impl Section {
    pub(crate) fn new(name: &str, rows: Vec<Row>) -> Self {
        Self {
            name: Some(name.into()),
            rows,
        }
    }
    #[allow(dead_code)]
    pub(crate) fn plain(rows: Vec<Row>) -> Self {
        Self { name: None, rows }
    }
}

/// One tab of a panel.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Tab {
    pub(crate) name: String,
    pub(crate) sections: Vec<Section>,
}

impl Tab {
    pub(crate) fn new(name: &str, sections: Vec<Section>) -> Self {
        Self {
            name: name.into(),
            sections,
        }
    }
    fn rows(&self) -> impl Iterator<Item = &Row> {
        self.sections.iter().flat_map(|s| &s.rows)
    }
}

/// What a panel remembers between frames and restarts: the active tab and
/// the folded sections (UIC-107). The caller saves it with its other
/// preferences.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct State {
    pub(crate) tab: usize,
    /// Folded sections, as "tab/section".
    pub(crate) folded: BTreeSet<String>,
}

/// Three number rows for a vector: "Position X", then "Y", "Z" (UIC-114).
#[allow(dead_code)]
pub(crate) fn vector_rows(key: &str, label: &str, unit: Unit, default: [f64; 3]) -> Vec<Row> {
    ["X", "Y", "Z"]
        .iter()
        .enumerate()
        .map(|(i, axis)| {
            let shown = if i == 0 {
                format!("{label} {axis}")
            } else {
                (*axis).to_owned()
            };
            Row::number(&format!("{key}.{i}"), &shown, unit, default[i])
        })
        .collect()
}

/// What a panel did this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Changes {
    /// Rows whose values changed, and buttons clicked, by key.
    pub(crate) keys: Vec<String>,
    /// The fold state or tab changed (save it).
    pub(crate) state: bool,
    /// Where things drew this frame, for callers' layout checks and tests:
    /// rows by key, section bars as "tab/section", tabs as "tab <name>".
    pub(crate) rects: BTreeMap<String, Rect>,
    /// The text editors of number and text fields drawn this frame, so the
    /// caller can keep shortcuts out of a field being typed in.
    pub(crate) text_ids: Vec<Id>,
}

/// The height `tab`'s open sections and rows take (bars, rows and gaps),
/// so a caller sizing a popover can fit it to its content.
pub(crate) fn content_height(tab: &Tab, state: &State) -> f32 {
    tab.sections
        .iter()
        .map(|section| {
            let bar = if section.name.is_some() {
                SECTION_HEIGHT
            } else {
                0.
            };
            let folded = section
                .name
                .as_ref()
                .is_some_and(|name| state.folded.contains(&format!("{}/{name}", tab.name)));
            let rows = if folded {
                0.
            } else {
                section
                    .rows
                    .iter()
                    .map(|row| ROW_HEIGHT + row.label_extra() + ROW_GAP)
                    .sum()
            };
            bar + rows
        })
        .sum()
}

/// How long the tab strip is for `tabs` (each name's length plus margins).
pub(crate) fn strip_length(ui: &Ui, tabs: &[Tab]) -> f32 {
    let font = TextStyle::Body.resolve(ui.style());
    tabs.iter()
        .map(|tab| {
            ui.painter()
                .layout_no_wrap(tab.name.clone(), font.clone(), theme::text())
                .size()
                .x
                + 16.
        })
        .sum()
}

/// Draws `tabs` filling `ui`'s available rect. `mixed` lists rows whose
/// selected items disagree; they show "(mixed)" (UIC-118).
pub(crate) fn show(
    ui: &mut Ui,
    id: &str,
    tabs: &[Tab],
    state: &mut State,
    values: &mut Values,
    mixed: &BTreeSet<String>,
) -> Changes {
    let mut changes = Changes::default();
    if tabs.is_empty() {
        return changes;
    }
    state.tab = state.tab.min(tabs.len() - 1);
    #[cfg(test)]
    crate::ui_inventory::push_scope(id);
    #[cfg(test)]
    let _pop = PopScope;
    let id = Id::new(("form-panel", id));
    let mut fields: Fields = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
    fields.text_ids.clear();
    let whole = ui.available_rect_before_wrap();
    let with_tabs = tabs.len() > 1;
    if with_tabs {
        let strip = Rect::from_min_max(pos2(whole.right() - TAB_STRIP, whole.top()), whole.max);
        if let Some(tab) = tab_strip(ui, strip, id, tabs, state.tab, &mut changes.rects) {
            state.tab = tab;
            changes.state = true;
        }
    }
    let content = Rect::from_min_max(
        whole.min,
        pos2(
            whole.right() - if with_tabs { TAB_STRIP } else { 0. },
            whole.bottom(),
        ),
    );
    let tab = &tabs[state.tab];
    let longest = tab
        .rows()
        .filter(|r| r.label_in_column())
        .map(|r| widgets::text_width(ui, &r.label))
        .fold(0., f32::max);
    let bar = layout::scroll_bar_room(ui);
    let mut child = ui.new_child(UiBuilder::new().max_rect(content).id_salt(id));
    egui::ScrollArea::vertical()
        .id_salt(("form-panel-scroll", state.tab))
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            let cols = layout::panel_layout(content.width(), longest, false, bar);
            for section in &tab.sections {
                let open = match &section.name {
                    Some(name) => {
                        let key = format!("{}/{name}", tab.name);
                        let folded = state.folded.contains(&key);
                        let bar = super::rows::section_bar(ui, cols.content_width, name, !folded);
                        changes.rects.insert(key.clone(), bar.rect);
                        if bar.clicked() {
                            if folded {
                                state.folded.remove(&key);
                            } else {
                                state.folded.insert(key);
                            }
                            changes.state = true;
                        }
                        !folded
                    }
                    None => true,
                };
                if !open {
                    continue;
                }
                for row in &section.rows {
                    let why = values.why_disabled(row, tab.sections.iter().flat_map(|s| &s.rows));
                    let (changed, rect) = draw_row(ui, cols, row, why, values, mixed, &mut fields);
                    changes.rects.insert(row.key.clone(), rect);
                    if changed {
                        changes.keys.push(row.key.clone());
                    }
                    ui.add_space(ROW_GAP);
                }
            }
        });
    changes.text_ids = fields.text_ids.clone();
    ui.data_mut(|d| d.insert_temp(id, fields));
    ui.advance_cursor_after_rect(whole);
    changes
}

/// A row's status light (UIC-112), or `None` for rows that aren't
/// channels. "Constant" once the value leaves the default; a "(mixed)"
/// row's values differ, so at least one is non-default and it shows
/// "Constant" too.
fn channel_status(row: &Row, value: &Value, is_mixed: bool) -> Option<ChannelStatus> {
    (row.channel && row.default != Value::None)
        .then(|| ChannelStatus::from_non_default(is_mixed || *value != row.default))
}

/// One row: label | status light | field. Returns whether it changed,
/// and where it drew.
/// Closes the recorder scope `show` opened, on every return path.
#[cfg(test)]
struct PopScope;
#[cfg(test)]
impl Drop for PopScope {
    fn drop(&mut self) {
        crate::ui_inventory::pop_scope();
    }
}

fn draw_row(
    ui: &mut Ui,
    cols: layout::PanelLayout,
    row: &Row,
    why: Option<String>,
    values: &mut Values,
    mixed: &BTreeSet<String>,
    fields: &mut Fields,
) -> (bool, Rect) {
    #[cfg(test)]
    crate::ui_inventory::row(&row.label);
    // Min/max labels sit under the field, in the row's extra height.
    let (whole, _) = ui.allocate_exact_size(
        vec2(cols.content_width, ROW_HEIGHT + row.label_extra()),
        Sense::hover(),
    );
    let rect = Rect::from_min_size(whole.min, vec2(whole.width(), ROW_HEIGHT));
    let enabled = why.is_none();
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(whole)
            .id_salt(("form-panel-row", &row.key)),
    );
    if !enabled {
        // UIC-117: greyed, never hidden.
        child.disable();
    }
    let x = rect.left();
    if row.label_in_column() {
        let label = Rect::from_min_size(
            pos2(x + cols.label_x, rect.top()),
            vec2(cols.label_width, ROW_HEIGHT),
        );
        widgets::label(&mut child, label, &row.label, &row.tooltip);
    }
    let mut value = values
        .get(&row.key)
        .cloned()
        .unwrap_or_else(|| row.default.clone());
    let is_mixed = mixed.contains(&row.key);
    if let Some(status) = channel_status(row, &value, is_mixed) {
        let c = pos2(x + cols.circle_x, rect.center().y);
        super::status::paint(child.painter(), c, status, enabled);
    }
    // Every choice is a dropdown in panels (UIC-113).
    let dropdown;
    let row = if let Kind::Segments(options) = &row.kind {
        dropdown = Row {
            kind: Kind::Choice(options.clone()),
            ..row.clone()
        };
        &dropdown
    } else {
        row
    };
    let field = Rect::from_min_size(
        pos2(x + cols.field_x, rect.top()),
        vec2(cols.field_width, ROW_HEIGHT),
    );
    let field = if row.kind == Kind::Message {
        Rect::from_min_max(pos2(x + cols.label_x, rect.top()), field.max)
    } else {
        field
    };
    let field_id = child.id().with("field");
    let out = widgets::field(
        &mut child, field, field_id, row, &mut value, fields, is_mixed,
    );
    widgets::hover(out.response, &row.tooltip);
    if let Some(why) = why {
        // Over the greyed row, the reason (unavailable.rs).
        child
            .interact(rect, child.id().with("why"), Sense::hover())
            .on_disabled_hover_text(why);
    }
    if row.default != Value::None {
        values.set(&row.key, value);
    }
    (out.changed, rect)
}

/// The vertical tab strip: full names rotated, the active tab filled with
/// the accent (UIC-100). Returns a newly clicked tab.
fn tab_strip(
    ui: &mut Ui,
    strip: Rect,
    id: Id,
    tabs: &[Tab],
    active: usize,
    rects: &mut BTreeMap<String, Rect>,
) -> Option<usize> {
    let font = TextStyle::Body.resolve(ui.style());
    ui.painter()
        .rect_filled(strip, CornerRadius::ZERO, theme::panel());
    let mut y = strip.top();
    let mut clicked = None;
    for (i, tab) in tabs.iter().enumerate() {
        let galley = ui
            .painter()
            .layout_no_wrap(tab.name.clone(), font.clone(), theme::text());
        let height = galley.size().x + 16.;
        let rect = Rect::from_min_size(pos2(strip.left(), y), vec2(strip.width(), height));
        let response = ui.interact(rect, id.with(("tab", i)), Sense::click());
        rects.insert(format!("tab {}", tab.name), rect);
        let on = i == active;
        // UIC-100: the active tab's name in the accent, the rest grey.
        let fill = if response.hovered() && !on {
            theme::hover()
        } else {
            theme::control()
        };
        ui.painter()
            .rect_filled(rect.shrink2(vec2(1., 0.5)), CornerRadius::same(2), fill);
        ui.painter().rect_stroke(
            rect.shrink2(vec2(1., 0.5)),
            CornerRadius::same(2),
            Stroke::new(1.0f32, theme::border()),
            StrokeKind::Inside,
        );
        let color = if on {
            theme::orange()
        } else {
            theme::weak_text()
        };
        // Rotated a quarter turn clockwise: reads top to bottom.
        let at = pos2(rect.center().x + galley.size().y / 2., rect.top() + 8.);
        ui.painter()
            .add(TextShape::new(at, galley, color).with_angle(std::f32::consts::FRAC_PI_2));
        let name = tab.name.clone();
        response
            .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, &name));
        if response.clicked() && !on {
            clicked = Some(i);
        }
        y += height;
    }
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Context, RawInput};

    fn tabs() -> Vec<Tab> {
        vec![
            Tab::new(
                "Mesh",
                vec![
                    Section::new(
                        "Items",
                        vec![
                            Row::check("masks", "Auto Masks", false),
                            Row::number("level", "Display Level", Unit::Count, 2.),
                            Row::segments("kind", "Kind", &["A", "B"], 0),
                        ],
                    ),
                    Section::new("Paste", vec![Row::check("select", "Select Pasted", true)]),
                ],
            ),
            Tab::new("Other", vec![Section::plain(vec![Row::button("go", "Go")])]),
        ]
    }

    fn frame(
        ctx: &Context,
        state: &mut State,
        values: &mut Values,
        events: Vec<egui::Event>,
    ) -> (Changes, egui::FullOutput) {
        let mut changes = Changes::default();
        let out = ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(320., 400.))),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    changes = show(ui, "test", &tabs(), state, values, &BTreeSet::new());
                });
            },
        );
        (changes, out)
    }

    fn click(at: egui::Pos2) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(at),
            egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
            egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            },
        ]
    }

    #[test]
    fn status_light_is_none_at_default_and_constant_when_changed_or_mixed() {
        let row = Row::number("n", "N", Unit::Scalar, 1.);
        let at = Value::Number(1.);
        let off = Value::Number(2.);
        assert_eq!(channel_status(&row, &at, false), Some(ChannelStatus::None));
        assert_eq!(
            channel_status(&row, &off, false),
            Some(ChannelStatus::Constant)
        );
        // UIC-112: differing selected values show Constant for now.
        assert_eq!(
            channel_status(&row, &at, true),
            Some(ChannelStatus::Constant)
        );
        let plain = Row::number("p", "P", Unit::Scalar, 1.).not_channel();
        assert_eq!(channel_status(&plain, &off, false), None);
        assert_eq!(
            channel_status(&Row::button("b", "B"), &Value::None, false),
            None
        );
    }

    #[test]
    fn sections_fold_and_tabs_switch() {
        let ctx = Context::default();
        let mut state = State::default();
        let mut values = Values::defaults(&tabs()[0].sections.concat_rows());
        frame(&ctx, &mut state, &mut values, vec![]);
        // The first section bar sits at the top of the content (CentralPanel
        // margin 8): clicking it folds "Items".
        let (changes, _) = frame(&ctx, &mut state, &mut values, click(pos2(60., 16.)));
        assert!(changes.state);
        assert!(state.folded.contains("Mesh/Items"), "{state:?}");
        // The tab strip is at the right: the second tab is below the first.
        let (changes, _) = frame(&ctx, &mut state, &mut values, click(pos2(300., 80.)));
        assert!(changes.state);
        assert_eq!(state.tab, 1);
        // The state survives a save and load (UIC-107).
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(serde_json::from_str::<State>(&json).unwrap(), state);
    }

    #[test]
    fn vector_rows_name_the_first_axis_only() {
        let rows = vector_rows("pos", "Position", Unit::Distance, [1., 2., 3.]);
        let labels: Vec<_> = rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, ["Position X", "Y", "Z"]);
        assert_eq!(rows[2].default, Value::Number(3.));
        assert_eq!(rows[1].key, "pos.1");
    }

    trait ConcatRows {
        fn concat_rows(&self) -> Vec<Row>;
    }
    impl ConcatRows for [Section] {
        fn concat_rows(&self) -> Vec<Row> {
            self.iter().flat_map(|s| s.rows.clone()).collect()
        }
    }
}
