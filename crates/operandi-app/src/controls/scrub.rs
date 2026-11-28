//! Drag-to-scrub for Tool Properties numeric fields.
//!
//! Every numeric field drawn by `Operandi::number` gets a narrow handle on
//! its left edge. Dragging the handle left or right changes the value live:
//! each frame writes the new value into the field's text and confirms it
//! through the same route as typing, so every tool keeps its own parsing,
//! limits and preview. Operandi's own design (provisional until Jason signs
//! off the feel):
//!
//! - plain drag: the field's base step per point dragged (1 cm, 0.5°,
//!   0.5 %, 0.01, or one whole step per 5 points for counts);
//! - Shift: ten times coarser; Option: ten times finer;
//! - Escape while dragging puts the value back and ends the scrub;
//! - a value a tool refuses stops the scrub at the last accepted value.
//!
//! The drag records no undo step of its own while it runs. A live tool's
//! session is already one undo step; for Move/Rotate/Scale/Transform (whose
//! viewport drags are each an undo step) the release records one step,
//! exactly like a handle drag.
use eframe::egui::{
    Color32, CursorIcon, Modifiers, Painter, Rect, Response, Sense, Shape, Stroke, Ui, pos2,
};
use operandi_core::Tool;

/// Width of the handle at the left of each numeric field.
pub(crate) const HANDLE_WIDTH: f32 = 12.;

/// What a field holds, which sets its scrub step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Distance,
    Angle,
    Percent,
    /// Whole numbers only (segments, sides, gaps, twist, levels, indices).
    Count,
    /// Plain numbers: factors, weights, bulges, strengths.
    Scalar,
}

impl Kind {
    /// The kind of the field `key`, shown with `unit`, while `tool` is live.
    pub(crate) fn for_field(key: &str, unit: &str, tool: Option<Tool>) -> Self {
        let unit = unit.trim();
        let count_keys = [
            "segments",
            "sides",
            "gaps",
            "twist",
            "round_level",
            "sphere_level",
            "pen_current",
            // Slice tools' whole-number fields (Axis Slice Number X/Y/Z,
            // Curve Slice Steps, clones, Loop Slice Count/Current).
            "slice_number",
            "slice_steps",
            "slice_clone",
            "slice_count",
            "slice_current",
            // Duplicate tools: Count, Number of Clones, Count X/Y/Z.
            "circle_count",
            "clone_count",
            "array_count",
            // Whole-number fields whose keys say no "count": scrubbing them
            // in 0.01 steps gave fractions the field refused, so the drag
            // did nothing (Jason, 2026-11-17: Jitter Seed).
            "jitter_seed",
            "radial_smoothing",
            "linear_smoothing",
            "smooth_iterations",
            "t_count",
            // Mesh Paint's Linear Count, Array Count X/Y and Radial Count
            // (#46: the arrows did nothing on them).
            "mp_linear",
            "mp_array_",
            "mp_radial",
        ];
        if count_keys.iter().any(|k| key.contains(k)) {
            Self::Count
        } else if unit == "°"
            || key == "bevel_angle"
            || key.starts_with("rotation_delta")
            || (key.starts_with("delta") && tool == Some(Tool::Rotate))
        {
            Self::Angle
        } else if unit == "%"
            || key == "vertex_slide_percent"
            || key == "edge_weight"
            || key == "chamfer_depth"
        {
            Self::Percent
        } else if unit == "m" {
            Self::Distance
        } else {
            Self::Scalar
        }
    }

    /// How the field's text is read back.
    pub(crate) fn quantity(self) -> operandi_core::numeric::Quantity {
        use operandi_core::numeric::Quantity;
        match self {
            Self::Distance => Quantity::Distance,
            Self::Angle => Quantity::Angle,
            Self::Percent => Quantity::Percent,
            Self::Count | Self::Scalar => Quantity::Scalar,
        }
    }

    /// Change per point dragged with no modifier.
    fn base_step(self) -> f64 {
        match self {
            Self::Distance => 0.01,
            Self::Angle | Self::Percent => 0.5,
            Self::Count => 0.2,
            Self::Scalar => 0.01,
        }
    }

    /// Change per point dragged with `modifiers` held.
    pub(crate) fn step(self, modifiers: Modifiers) -> f64 {
        let mut step = self.base_step();
        if modifiers.shift {
            step *= 10.;
        }
        if modifiers.alt {
            step *= 0.1;
        }
        step
    }

    /// The grid a scrubbed change snaps to, so the field shows clean numbers.
    fn grid(self, modifiers: Modifiers) -> f64 {
        if self == Self::Count {
            1.
        } else {
            self.step(modifiers)
        }
    }
}

/// One scrub in progress.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Scrub {
    /// The field being scrubbed.
    pub(crate) key: String,
    pub(crate) kind: Kind,
    /// The field's value and text when the drag began.
    pub(crate) start: f64,
    pub(crate) start_text: String,
    /// Raw change accumulated from the pointer, before snapping.
    raw: f64,
    /// The last change the tool accepted.
    pub(crate) accepted: f64,
    /// Escape put the value back; ignore the rest of this drag.
    pub(crate) cancelled: bool,
    /// A detent holding the value (UIC-54, `detents.rs`).
    pub(crate) held: Option<crate::detents::Held>,
}

impl Scrub {
    pub(crate) fn new(key: &str, kind: Kind, start: f64, start_text: String) -> Self {
        Self {
            key: key.into(),
            kind,
            start,
            start_text,
            raw: 0.,
            accepted: 0.,
            cancelled: false,
            held: None,
        }
    }

    /// Add `dx` points of horizontal drag and return the change from the
    /// start value to try now, snapped to the step grid.
    pub(crate) fn advance(&mut self, dx: f32, modifiers: Modifiers) -> f64 {
        let step = self.kind.step(modifiers);
        let grid = self.kind.grid(modifiers);
        // Keep the decimals the grid allows, so 0.1 + 0.2 shows as 0.3.
        let decimals = (-grid.log10()).ceil().max(0.) as i32 + 1;
        let scale = 10f64.powi(decimals);
        let tidy = |v: f64| (v * scale).round() / scale;
        // UIC-54: a detent holds the value until its travel is used up;
        // the travel past it carries on from the stop.
        if let Some(held) = self.held.as_mut() {
            held.left -= dx.abs();
            if held.left > 0. {
                return tidy(held.stop - self.start);
            }
            let over = f64::from(-held.left) * f64::from(dx.signum());
            let stop = held.stop;
            self.held = None;
            self.raw = stop - self.start + over * step;
            return tidy((self.raw / grid).round() * grid);
        }
        let before = self.start + self.raw;
        self.raw += f64::from(dx) * step;
        let after = self.start + self.raw;
        if let Some((stop, strength)) =
            crate::detents::first_reached(self.kind, before, after, step)
        {
            let past = ((after - stop).abs() / step) as f32;
            if past < strength.hold() {
                self.held = Some(crate::detents::Held {
                    stop,
                    left: strength.hold() - past,
                });
                self.raw = stop - self.start;
                return tidy(self.raw);
            }
        }
        tidy((self.raw / grid).round() * grid)
    }

    /// The tool took `tried` but shows `shown` (they differ when the tool
    /// clamps, as Bevel Segments does at 1): carry on from what it shows.
    pub(crate) fn accept(&mut self, tried: f64, shown: f64) {
        if (shown - tried).abs() > 1e-9 {
            self.raw = shown;
            self.held = None;
        }
        self.accepted = shown;
    }

    /// The tool refused the last change: drop back to the accepted one so
    /// dragging the other way responds at once.
    pub(crate) fn refuse(&mut self) {
        self.raw = self.accepted;
        self.held = None;
    }
}

/// Splits a numeric field's cell into the handle and the text field. The
/// handle is the mini-slider arrows at the field's right end (UIC-53,
/// UIC-113), as in every form field.
pub(crate) fn split(cell: Rect) -> (Rect, Rect) {
    let handle = Rect::from_min_max(pos2(cell.max.x - HANDLE_WIDTH, cell.min.y), cell.max);
    let mut field = cell;
    field.max.x = handle.min.x - GAP;
    (handle, field)
}

/// Space between the handle and its text field.
const GAP: f32 = 2.;

/// Draws the handle for field `key` in `rect` and returns its response
/// (drag- and click-sensed, with a horizontal-resize cursor).
pub(crate) fn handle(ui: &mut Ui, rect: Rect, key: &str, active: bool) -> Response {
    let response = ui
        .interact(rect, ui.id().with(("scrub", key)), Sense::click_and_drag())
        .on_hover_cursor(CursorIcon::ResizeHorizontal)
        .on_hover_text(
            "Drag to change · Shift: coarser · Option: finer · Escape: put back · click the right half to step up, the left half to step down",
        );
    if ui.is_rect_visible(rect) {
        let visuals = ui.visuals();
        let color = if active || response.dragged() {
            visuals.widgets.active.fg_stroke.color
        } else if response.hovered() {
            visuals.widgets.hovered.fg_stroke.color
        } else {
            visuals.weak_text_color()
        };
        paint_arrows(ui.painter(), rect, color);
    }
    response
}

/// A click on a field's handle (UIC-53): the step to add, up on the right
/// half and down on the left, ten times with Shift. Whole numbers step by
/// one; every other kind by ten of its drag steps (MN-06.4: 10 cm, 5°,
/// 5 %, 0.1).
pub(crate) fn click_step(handle: &Response, kind: Kind, modifiers: Modifiers) -> Option<f64> {
    if !handle.clicked() {
        return None;
    }
    let x = handle.interact_pointer_pos()?.x;
    Some(click_amount(kind, modifiers, x >= handle.rect.center().x))
}

/// The step a click adds (`up` on the right half), MN-06.4.
pub(crate) fn click_amount(kind: Kind, modifiers: Modifiers, up: bool) -> f64 {
    let one = if kind == Kind::Count {
        1.
    } else {
        kind.base_step() * 10.
    };
    let step = if modifiers.shift { one * 10. } else { one };
    if up { step } else { -step }
}

/// Two small opposed triangles, one above the other half of the cell: a
/// left-pointing one and a right-pointing one, centred in `rect`.
fn paint_arrows(painter: &Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let (w, h, gap) = (3.5, 3.5, 1.);
    let left = vec![
        pos2(c.x - gap - w, c.y),
        pos2(c.x - gap, c.y - h),
        pos2(c.x - gap, c.y + h),
    ];
    let right = vec![
        pos2(c.x + gap + w, c.y),
        pos2(c.x + gap, c.y + h),
        pos2(c.x + gap, c.y - h),
    ];
    for points in [left, right] {
        painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn split_puts_the_arrows_inside_the_right_end() {
        // UIC-53/113: Tool Properties, Item Transform and every
        // numeric_tab::DragValue cut their cell with `split`.
        let cell = Rect::from_min_size(pos2(10., 5.), eframe::egui::vec2(120., 20.));
        let (handle, field) = split(cell);
        assert_eq!(handle.max, cell.max);
        assert_eq!(handle.width(), HANDLE_WIDTH);
        assert_eq!(field.min, cell.min);
        assert!(field.max.x < handle.min.x);
    }

    use super::*;

    #[test]
    fn kinds_follow_key_unit_and_tool() {
        assert_eq!(Kind::for_field("shift", " m", None), Kind::Distance);
        assert_eq!(Kind::for_field("bevel_segments", "", None), Kind::Count);
        assert_eq!(Kind::for_field("radial_sides", "", None), Kind::Count);
        assert_eq!(Kind::for_field("bri_twist", "", None), Kind::Count);
        for key in [
            "jitter_seed",
            "radial_smoothing",
            "linear_smoothing",
            "smooth_iterations",
        ] {
            assert_eq!(Kind::for_field(key, "", None), Kind::Count, "{key}");
        }
        for key in [
            "circle_count",
            "clone_count",
            "array_count0",
            "array_count2",
        ] {
            assert_eq!(Kind::for_field(key, "", None), Kind::Count, "{key}");
        }
        assert_eq!(Kind::for_field("sst_angle", " °", None), Kind::Angle);
        assert_eq!(
            Kind::for_field("delta1", "", Some(Tool::Rotate)),
            Kind::Angle
        );
        assert_eq!(
            Kind::for_field("delta1", "", Some(Tool::Scale)),
            Kind::Scalar
        );
        assert_eq!(
            Kind::for_field("delta1", " m", Some(Tool::Move)),
            Kind::Distance
        );
        assert_eq!(
            Kind::for_field("vertex_slide_percent", "%", None),
            Kind::Percent
        );
        assert_eq!(Kind::for_field("sst_scale", " %", None), Kind::Percent);
        assert_eq!(Kind::for_field("ell_top", "", None), Kind::Scalar);
    }

    #[test]
    fn modifiers_scale_the_step_and_changes_snap_to_it() {
        let none = Modifiers::NONE;
        let shift = Modifiers::SHIFT;
        let alt = Modifiers::ALT;
        let mut s = Scrub::new("shift", Kind::Distance, 0.123, "0.123 m".into());
        // 10 points at 1 cm each.
        assert_eq!(s.advance(10., none), 0.1);
        // 2 more points with Shift: 10 cm each, snapped to the 10 cm grid.
        assert_eq!(s.advance(2., shift), 0.3);
        // Back 25 points with Option: 1 mm each, but 0.4 is a light
        // detent at that speed (UIC-54): it holds for 4 points first.
        assert_eq!(s.advance(-25., alt), 0.277);
        assert_eq!(s.advance(-2., alt), 0.277);
        // The 4 points of hold are travel spent; the rest carries on.
        assert_eq!(s.advance(-4., alt), 0.273);
        // Counts move one whole step per 5 points and never show fractions.
        let mut c = Scrub::new("bevel_segments", Kind::Count, 3., "3".into());
        assert_eq!(c.advance(4., none), 1.);
        assert_eq!(c.advance(6., none), 2.);
        assert_eq!(c.advance(-30., none), -4.);
    }

    #[test]
    fn a_refusal_drops_back_to_the_accepted_change() {
        let mut s = Scrub::new("bevel_segments", Kind::Count, 1., "1".into());
        s.accepted = s.advance(5., Modifiers::NONE);
        assert_eq!(s.accepted, 1.);
        assert_eq!(s.advance(-50., Modifiers::NONE), -9.);
        s.refuse();
        // One point back the other way is felt straight away.
        assert_eq!(s.advance(5., Modifiers::NONE), 2.);
    }
}
