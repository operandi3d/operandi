//! Layout maths for the shared form component, with no egui in it so it can
//! be tested on its own. Dialog mode follows UIC-81..94; panel mode follows
//! UIC-100..120.

/// Dialog sizes at 1x (UIC-94).
pub(crate) mod dialog {
    /// One row's height.
    pub(crate) const ROW_HEIGHT: f32 = 20.;
    /// Space between rows.
    pub(crate) const ROW_GAP: f32 = 3.;
    /// Space between the window edge and its contents.
    pub(crate) const PADDING: f32 = 8.;
    /// Space between a label's right edge and its field.
    pub(crate) const LABEL_GAP: f32 = 8.;
    /// The narrowest the field column may be.
    pub(crate) const MIN_FIELD: f32 = 200.;
    /// Space each side of the divider before the buttons.
    pub(crate) const DIVIDER_GAP: f32 = 6.;
    /// The divider itself.
    pub(crate) const DIVIDER: f32 = 1.;
    /// Width of the button column.
    pub(crate) const BUTTON_WIDTH: f32 = 70.;
    /// A Yes / No / Cancel question's button column (Jason, 2026-11-08,
    /// deviation from UIC-94): wide enough for "Cancel  Shift+Esc".
    pub(crate) const QUESTION_BUTTON_WIDTH: f32 = 112.;
    /// Height of one button.
    pub(crate) const BUTTON_HEIGHT: f32 = 20.;
    /// Space between buttons.
    pub(crate) const BUTTON_GAP: f32 = 4.;
    /// A checkbox's square.
    pub(crate) const CHECK: f32 = 14.;
    /// Space between a checkbox and its text.
    pub(crate) const CHECK_GAP: f32 = 6.;
    /// The window's title bar.
    pub(crate) const TITLE_HEIGHT: f32 = 22.;
    /// A confirmation message never grows wider than this (it wraps).
    pub(crate) const MAX_MESSAGE: f32 = 360.;
}

/// Panel sizes at 1x (UIC-120).
pub(crate) mod panel {
    pub(crate) const ROW_HEIGHT: f32 = 20.;
    pub(crate) const ROW_GAP: f32 = 2.;
    pub(crate) const SECTION_HEIGHT: f32 = 18.;
    /// Label → status circle and circle → field.
    pub(crate) const GAP: f32 = 4.;
    /// The status circle (UIC-110).
    pub(crate) const CIRCLE: f32 = 9.;
    pub(crate) const RIGHT_MARGIN: f32 = 6.;
    pub(crate) const LEFT_MARGIN: f32 = 6.;
    /// The vertical tab strip (UIC-100).
    pub(crate) const TAB_STRIP: f32 = 22.;
    /// The Presets / One Tab / All Tabs header row (UIC-103; slot only,
    /// not built yet).
    #[allow(dead_code)]
    pub(crate) const HEADER: f32 = 18.;
    /// A label takes at most this share of the panel before it is cut
    /// with "…" (UIC-111).
    pub(crate) const MAX_LABEL_SHARE: f32 = 0.45;
}

/// What a row needs in a dialog, measured by the caller.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RowSize {
    /// Width of the label text; 0 for rows with no left label
    /// (checkboxes, UIC-87; messages, UIC-97).
    pub(crate) label: f32,
    /// The width the field needs (a checkbox's box and text, a long
    /// choice name); the field column is at least this wide.
    pub(crate) field: f32,
    /// The row's height; a plain row is `dialog::ROW_HEIGHT`, a wrapped
    /// message can be taller.
    pub(crate) height: f32,
    /// The row spans the label and field columns (a message).
    pub(crate) spans: bool,
}

/// Where everything in a dialog goes, relative to its content's top-left
/// (below the title bar).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DialogLayout {
    /// Width of the shared label column: the longest label (UIC-86).
    pub(crate) label_column: f32,
    /// Left edge and width of the field column; every field is this wide.
    pub(crate) field_x: f32,
    pub(crate) field_width: f32,
    /// Each row's top.
    pub(crate) row_tops: Vec<f32>,
    /// The divider's x (UIC-83).
    pub(crate) divider_x: f32,
    /// The button column's left edge.
    pub(crate) button_x: f32,
    /// Each button's top.
    pub(crate) button_tops: Vec<f32>,
    /// The whole content area (excluding the title bar), exactly the
    /// contents plus padding (UIC-81).
    pub(crate) width: f32,
    pub(crate) height: f32,
}

/// Lays out a dialog with `rows` and `buttons` buttons (OK, Cancel and any
/// extras, UIC-84).
#[cfg(test)]
pub(crate) fn dialog_layout(rows: &[RowSize], buttons: usize) -> DialogLayout {
    dialog_layout_with(rows, buttons, dialog::BUTTON_WIDTH)
}

/// `dialog_layout` with a button column `button_width` wide.
pub(crate) fn dialog_layout_with(
    rows: &[RowSize],
    buttons: usize,
    button_width: f32,
) -> DialogLayout {
    use dialog::*;
    let label_column = rows
        .iter()
        .filter(|r| !r.spans)
        .map(|r| r.label)
        .fold(0., f32::max)
        .ceil();
    // With no labels at all (a confirmation) there is no label column and
    // no gap before the fields.
    let gap = if label_column > 0. { LABEL_GAP } else { 0. };
    let spanning = rows
        .iter()
        .filter(|r| r.spans)
        .map(|r| r.field - label_column - gap)
        .fold(0., f32::max);
    let field_width = rows
        .iter()
        .filter(|r| !r.spans)
        .map(|r| r.field)
        .fold(MIN_FIELD, f32::max)
        .max(spanning)
        .ceil();
    let field_x = PADDING + label_column + gap;
    let mut row_tops = Vec::with_capacity(rows.len());
    let mut y = PADDING;
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            y += ROW_GAP;
        }
        row_tops.push(y);
        y += row.height.max(ROW_HEIGHT);
    }
    let form_bottom = if rows.is_empty() { PADDING } else { y };
    let divider_x = field_x + field_width + DIVIDER_GAP;
    let button_x = divider_x + DIVIDER + DIVIDER_GAP;
    let mut button_tops = Vec::with_capacity(buttons);
    let mut y = PADDING;
    for i in 0..buttons {
        if i > 0 {
            y += BUTTON_GAP;
        }
        button_tops.push(y);
        y += BUTTON_HEIGHT;
    }
    let buttons_bottom = if buttons == 0 { PADDING } else { y };
    DialogLayout {
        label_column,
        field_x,
        field_width,
        row_tops,
        divider_x,
        button_x,
        button_tops,
        width: button_x + button_width + PADDING,
        height: form_bottom.max(buttons_bottom) + PADDING,
    }
}

/// Where the columns of a property panel go, relative to its left edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PanelLayout {
    /// The label column's left edge and width (labels right-aligned in it).
    pub(crate) label_x: f32,
    pub(crate) label_width: f32,
    /// The status circle's centre x (UIC-110).
    pub(crate) circle_x: f32,
    /// The field column; every field is this wide (UIC-113).
    pub(crate) field_x: f32,
    pub(crate) field_width: f32,
    /// The content's width (left of the scroll bar and tab strip).
    pub(crate) content_width: f32,
}

/// The room a panel's vertical scroll bar needs at the right edge. egui's
/// floating bar allocates no width (it draws over the content), so the
/// rows went under it and its bar covered fields and buttons at the right
/// edge. Panels keep this room always, so nothing shifts when the bar
/// appears.
pub(crate) fn scroll_bar_room(ui: &eframe::egui::Ui) -> f32 {
    let scroll = &ui.spacing().scroll;
    scroll.bar_inner_margin + scroll.bar_width + scroll.bar_outer_margin
}

/// Lays out a panel `width` wide whose longest label is `longest_label`.
/// `tabs` reserves the tab strip; `scroll_bar` the scroll bar between
/// content and tabs (UIC-102).
pub(crate) fn panel_layout(
    width: f32,
    longest_label: f32,
    tabs: bool,
    scroll_bar: f32,
) -> PanelLayout {
    use panel::*;
    let strip = if tabs { TAB_STRIP } else { 0. };
    let content_width = (width - strip - scroll_bar).max(0.);
    let inner = (content_width - LEFT_MARGIN - RIGHT_MARGIN).max(0.);
    let label_width = longest_label.min(inner * MAX_LABEL_SHARE).max(0.).ceil();
    let label_x = LEFT_MARGIN;
    let circle_x = label_x + label_width + GAP + CIRCLE / 2.;
    let field_x = label_x + label_width + GAP + CIRCLE + GAP;
    PanelLayout {
        label_x,
        label_width,
        circle_x,
        field_x,
        field_width: (content_width - RIGHT_MARGIN - field_x).max(0.),
        content_width,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(label: f32) -> RowSize {
        RowSize {
            label,
            field: 0.,
            height: dialog::ROW_HEIGHT,
            spans: false,
        }
    }

    #[test]
    fn dialog_label_column_is_the_longest_label_and_fields_share_one_width() {
        // UIC-86: labels share one right edge; fields one width.
        let l = dialog_layout(&[row(40.), row(120.5), row(0.)], 2);
        assert_eq!(l.label_column, 121.);
        assert_eq!(l.field_x, 8. + 121. + 8.);
        assert_eq!(l.field_width, 200.);
        // UIC-83/94: divider gap 6 each side, then a 70 wide column.
        assert_eq!(l.divider_x, l.field_x + 200. + 6.);
        assert_eq!(l.button_x, l.divider_x + 1. + 6.);
        assert_eq!(l.width, l.button_x + 70. + 8.);
    }

    #[test]
    fn dialog_rows_and_buttons_stack_from_the_top() {
        let l = dialog_layout(&[row(10.), row(10.), row(10.)], 2);
        assert_eq!(l.row_tops, vec![8., 31., 54.]);
        assert_eq!(l.button_tops, vec![8., 32.]);
        // Fits the content exactly: three rows plus padding (UIC-81).
        assert_eq!(l.height, 8. + 3. * 20. + 2. * 3. + 8.);
    }

    #[test]
    fn short_forms_are_as_tall_as_their_buttons() {
        // One row; OK, Cancel and Reset stack taller than it.
        let l = dialog_layout(&[row(10.)], 3);
        assert_eq!(l.height, 8. + 3. * 20. + 2. * 4. + 8.);
    }

    #[test]
    fn wide_fields_widen_the_column() {
        let mut wide = row(10.);
        wide.field = 260.;
        let l = dialog_layout(&[row(10.), wide], 2);
        assert_eq!(l.field_width, 260.);
    }

    #[test]
    fn a_confirmation_has_no_label_column() {
        // UIC-97: the message sits in the form area as plain text.
        let message = RowSize {
            label: 0.,
            field: 300.,
            height: 34.,
            spans: true,
        };
        let l = dialog_layout(&[message], 2);
        assert_eq!(l.label_column, 0.);
        assert_eq!(l.field_x, 8.);
        assert_eq!(l.field_width, 300.);
        assert_eq!(l.height, 8. + 44f32.max(34.) + 8.);
    }

    #[test]
    fn panel_columns_follow_uic_110_to_113() {
        let l = panel_layout(300., 90., true, 0.);
        assert_eq!(l.content_width, 278.);
        assert_eq!(l.label_width, 90.);
        assert_eq!(l.field_x, 6. + 90. + 4. + 9. + 4.);
        assert_eq!(l.circle_x, 6. + 90. + 4. + 4.5);
        assert_eq!(l.field_width, 278. - 6. - l.field_x);
    }

    #[test]
    fn long_panel_labels_are_capped_at_45_percent() {
        // UIC-111: the rest is cut with "…".
        let l = panel_layout(222., 400., false, 0.);
        assert_eq!(l.label_width, (210. * 0.45f32).ceil());
    }
}
