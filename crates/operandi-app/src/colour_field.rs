//! The colour field: one shared control for every colour in Operandi
//! (colour-nav note 07, Part A), and the Colour Picker popover it opens
//! (Part B).
//!
//! The field is a row filled with the colour itself, with its R, G and B
//! drawn on top in the Color Unit System. Drag a number to change that
//! component; Shift, Ctrl and Alt drags change brightness, hue and
//! saturation of all three. A click anywhere on the field, numbers
//! included, opens the picker next to it (CN-Q02, Jason 2026-11-08): the
//! numbers change only by dragging, or by typing in the picker's fields.
//! Right-click copies, pastes or resets.
//!
//! Each change reports a gesture id; callers run their command with
//! `Editor::undo_fold` set to it, so one drag (or one picker drag) is one
//! undo step however many frames and items it touches.
use crate::colour_picker;
use crate::unavailable::Gate;
use crate::value_display;
use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, vec2,
};
use operandi_core::colour_math::{DragStart, SwatchDrag, light_text_over};
use std::cell::Cell;

/// A press released within this many points of where it started is a
/// click (A4), and opens the picker at once.
const CLICK_DISTANCE: f32 = 3.;

thread_local! {
    static NEXT_GESTURE: Cell<u64> = const { Cell::new(1) };
    /// Copy / Paste colour (right-click), shared by every field.
    static CLIPBOARD: Cell<Option<[f64; 3]>> = const { Cell::new(None) };
    /// The last field change this frame: its gesture and swatch drag.
    static FRAME_CHANGE: Cell<Option<(u64, Option<GangDrag>)>> = const { Cell::new(None) };
}

/// Start of a frame: forget the previous frame's change.
pub fn begin_frame() {
    FRAME_CHANGE.with(|c| c.set(None));
}

/// The undo gesture of a colour change made this frame, if any. The app
/// sets `Editor::undo_fold` to it while it runs commands, so a drag over
/// frames (and items) is one undo step.
pub fn frame_gesture() -> Option<u64> {
    FRAME_CHANGE.with(Cell::get).map(|(g, _)| g)
}

/// The swatch drag of the field drawn just now (taken), for gang edits.
pub fn take_gang() -> Option<GangDrag> {
    FRAME_CHANGE.with(|c| {
        let v = c.get();
        if let Some((g, Some(_))) = v {
            c.set(Some((g, None)));
        }
        v.and_then(|(_, gang)| gang)
    })
}

/// A fresh gesture id for one undoable change.
pub fn next_gesture() -> u64 {
    NEXT_GESTURE.with(|n| {
        let id = n.get();
        n.set(id.wrapping_add(1).max(1));
        id
    })
}

/// How a field behaves where it is used.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Scene colours may go above 1 (HDR); UI and theme colours clamp.
    pub hdr: bool,
    /// What Right-click > Reset to Default restores, and whether it can reset.
    default: Option<([f64; 3], bool)>,
    /// The values are already display-encoded (theme colours): the fill
    /// and numbers use them as they are.
    pub encoded: bool,
}

impl Options {
    /// A scene colour (materials, lights, environment): HDR.
    pub fn scene() -> Self {
        Self {
            hdr: true,
            default: None,
            encoded: false,
        }
    }
    /// A UI or theme colour: clamped to 0–1.
    pub fn ui() -> Self {
        Self {
            hdr: false,
            default: None,
            encoded: false,
        }
    }
    /// A theme colour: clamped and already display-encoded (sRGB).
    pub fn theme() -> Self {
        Self {
            encoded: true,
            ..Self::ui()
        }
    }
    pub fn with_default(mut self, default: [f64; 3], can_reset: bool) -> Self {
        self.default = Some((default, can_reset));
        self
    }
}

/// A swatch drag this frame, for gang edits (A5): apply it to the same
/// colour on the other selected items.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GangDrag {
    pub start: DragStart,
    /// Pixels from the press to now, and this frame's part of it.
    pub total: f64,
    pub step: f64,
}

impl GangDrag {
    /// The drag applied to another item's current colour: a component drag
    /// sets the same value; modifier drags move each colour by the same
    /// amount from where it is.
    pub fn apply(&self, current: [f64; 3]) -> [f64; 3] {
        match self.start.drag {
            SwatchDrag::Component(_) => self.start.apply_to(current, self.total),
            _ => self.start.apply_to(current, self.step),
        }
    }
}

/// What the field did this frame.
pub struct FieldResponse {
    pub response: Response,
    /// The colour changed this frame.
    pub changed: bool,
    /// Reset to Default was chosen, even if the RGB stayed the same.
    pub reset: bool,
    /// The undo gesture the change belongs to (set with `changed`).
    pub gesture: Option<u64>,
    /// A swatch drag, for applying to other selected items.
    pub gang: Option<GangDrag>,
}

impl FieldResponse {
    pub fn changed(&self) -> bool {
        self.changed
    }
}

#[derive(Clone, Copy, Debug)]
struct Press {
    start: DragStart,
    dx: f64,
    gesture: u64,
    /// Where the press started, and whether the pointer has since gone
    /// further than a click allows.
    origin: Pos2,
    far: bool,
}

#[derive(Clone, Debug, Default)]
struct State {
    press: Option<Press>,
}

/// The three number zones of a field.
fn zones(rect: Rect) -> [Rect; 3] {
    let w = rect.width() / 3.;
    [0, 1, 2]
        .map(|i| Rect::from_min_size(rect.min + vec2(w * i as f32, 0.), vec2(w, rect.height())))
}

fn zone_at(rect: Rect, x: f32) -> usize {
    (((x - rect.left()) / (rect.width() / 3.)).floor().max(0.) as usize).min(2)
}

/// The swatch colour for stored `rgb`: the stored linear colour through
/// the display transform, so a swatch is the same colour whatever the
/// number display (Display Color Values in Display Space on or off, any
/// Color Unit System). Every colour swatch (fields, picker, ramps) goes
/// through here.
pub fn display_color(rgb: [f64; 3]) -> Color32 {
    let [r, g, b] = rgb.map(|c| c.clamp(0., 1.) as f32);
    Color32::from(egui::Rgba::from_rgb(r, g, b))
}

/// The fill for a field's values (`encoded`: already sRGB).
pub fn fill_color(rgb: [f64; 3], encoded: bool) -> Color32 {
    if encoded {
        let [r, g, b] = rgb.map(|c| (c.clamp(0., 1.) * 255.).round() as u8);
        Color32::from_rgb(r, g, b)
    } else {
        display_color(rgb)
    }
}

fn format(value: f64, encoded: bool) -> String {
    let units = value_display::color_units();
    if encoded {
        value_display::format_color_raw(value, units)
    } else {
        value_display::format_color(value, units)
    }
}

/// The shared colour field over linear `rgb`, filling the row's width.
pub fn show(ui: &mut Ui, rgb: &mut [f64; 3], options: Options) -> FieldResponse {
    let height = ui.spacing().interact_size.y;
    let width = ui.available_width().max(96.);
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click_and_drag());
    let id = response.id;
    let mut state: State = ui.data(|d| d.get_temp(id)).unwrap_or_default();
    let mut out = FieldResponse {
        response: response.clone(),
        changed: false,
        reset: false,
        gesture: None,
        gang: None,
    };
    let top = if options.hdr { f64::INFINITY } else { 1. };

    // A2/A3: the drag kind is read at press time and kept for the drag.
    if response.is_pointer_button_down_on()
        && state.press.is_none()
        && ui.input(|i| i.pointer.primary_down())
        && let Some(origin) = ui.input(|i| i.pointer.press_origin())
    {
        let m = ui.input(|i| i.modifiers);
        let drag = SwatchDrag::from_modifiers(zone_at(rect, origin.x), m.shift, m.ctrl, m.alt);
        state.press = Some(Press {
            start: DragStart::new(*rgb, drag, options.hdr),
            dx: 0.,
            gesture: next_gesture(),
            origin,
            far: false,
        });
    }
    if let Some(press) = state.press.as_mut()
        && response.dragged()
    {
        let step = f64::from(response.drag_delta().x);
        if step != 0. {
            press.dx += step;
            let next = press.start.at(press.dx);
            if next != *rgb {
                *rgb = next;
                out.changed = true;
                out.gesture = Some(press.gesture);
                out.gang = Some(GangDrag {
                    start: press.start,
                    total: press.dx,
                    step,
                });
            }
        }
        // The pointer hides while it drags a value (A2).
        ui.ctx().set_cursor_icon(CursorIcon::None);
    }
    if let Some(press) = state.press.as_mut()
        && let Some(p) = ui.input(|i| i.pointer.latest_pos())
        && p.distance(press.origin) > CLICK_DISTANCE
    {
        press.far = true;
    }
    // A4: a click (press and release within 3 points), on a number or
    // anywhere else on the field, opens the picker on release.
    let released = ui.input(|i| i.pointer.primary_released());
    let click = match state.press {
        Some(press) => released && !press.far,
        // Press and release in one frame: egui's own click.
        None => response.clicked(),
    };
    if !response.is_pointer_button_down_on() && !response.dragged() {
        state.press = None;
    }
    if click && !crate::os_color::open_system_for(id, *rgb) {
        colour_picker::open(id, rect);
    }
    // The system colour panel (Preferences > Defaults > Use OS Color
    // Picker) follows its owner while it stays open.
    if let Some(picked) = crate::os_color::follow_system(id, *rgb) {
        *rgb = picked.map(|c| c.clamp(0., top));
        out.changed = true;
        out.gesture = Some(next_gesture());
    }

    // Draw: fill, then the numbers.
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2., fill_color(*rgb, options.encoded));
    painter.rect_stroke(
        rect,
        2.,
        Stroke::new(1_f32, crate::theme::border()),
        egui::StrokeKind::Inside,
    );
    // Text contrast follows the fill as drawn.
    let fill = fill_color(*rgb, options.encoded);
    let drawn = [fill.r(), fill.g(), fill.b()]
        .map(|c| operandi_core::colour_math::srgb_to_linear(f64::from(c) / 255.));
    let ink = if light_text_over(drawn) {
        Color32::from_gray(235)
    } else {
        Color32::from_gray(20)
    };
    let font = FontId::proportional(11.5);
    for (i, zone) in zones(rect).into_iter().enumerate() {
        painter.text(
            zone.center(),
            Align2::CENTER_CENTER,
            format(rgb[i], options.encoded),
            font.clone(),
            ink,
        );
    }
    if response.hovered() && state.press.is_none() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }

    if crate::os_color::system_open_for(id) {
        ui.ctx().request_repaint();
    }

    // A5: Copy / Paste colour, Reset to default.
    response.context_menu(|ui| {
        if ui.button("Copy Color").clicked() {
            CLIPBOARD.with(|c| c.set(Some(*rgb)));
            ui.ctx()
                .copy_text(operandi_core::colour_math::hex_rgb(*rgb, !options.encoded));
            ui.close();
        }
        let copied = CLIPBOARD.with(Cell::get);
        if ui
            .gated(
                crate::unavailable::unless(copied.is_some(), "Copy a colour first"),
                egui::Button::new("Paste Color"),
            )
            .clicked()
            && let Some(c) = copied
        {
            *rgb = c.map(|v| v.clamp(0., top));
            out.changed = true;
            out.gesture = Some(next_gesture());
            ui.close();
        }
        if let Some((default, can_reset)) = options.default
            && ui
                .gated(
                    crate::unavailable::unless(can_reset, "Already the default colour"),
                    egui::Button::new("Reset to Default"),
                )
                .clicked()
        {
            *rgb = default;
            out.changed = true;
            out.reset = true;
            out.gesture = Some(next_gesture());
            ui.close();
        }
    });

    // Part B: the popover, when this field owns it.
    if let Some(gesture) = colour_picker::show_for(ui.ctx(), id, rect, rgb, options) {
        out.changed = true;
        out.gesture = Some(gesture);
    }

    ui.data_mut(|d| d.insert_temp(id, state));
    if out.changed {
        out.response.mark_changed();
        if let Some(g) = out.gesture {
            FRAME_CHANGE.with(|c| c.set(Some((g, out.gang))));
        }
    }
    out.response = out.response.on_hover_text(
        "Drag a number to change it · Shift: brightness · Ctrl: hue · Alt: saturation\n\
         Click: color picker",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units_preferences::ColorUnits;
    use eframe::egui::Modifiers;
    use eframe::egui::{Context, Event, PointerButton, RawInput};

    fn frame(ctx: &Context, input: RawInput, rgb: &mut [f64; 3]) -> (Rect, FieldResponse) {
        let mut out = None;
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.set_width(240.);
                let r = show(ui, rgb, Options::scene());
                out = Some((r.response.rect, r));
            });
        });
        out.unwrap()
    }

    fn pointer(events: Vec<Event>, modifiers: Modifiers) -> RawInput {
        RawInput {
            events,
            modifiers,
            ..Default::default()
        }
    }

    /// The swatch draws the numbers it shows: 0.45 fills at 45 % grey
    /// (115), not the display-transformed ~70 %. With Display Space on,
    /// the shown numbers are encoded and the swatch is the same colour as
    /// those numbers too.
    #[test]
    fn swatch_is_the_stored_colour_through_the_display_transform() {
        assert!(crate::units_preferences::UnitsPreferences::default().color_display_space);
        crate::value_display::set_display_space(true);
        let stored = operandi_core::colour_math::srgb_to_linear(0.45);
        assert_eq!(
            crate::value_display::format_color(stored, ColorUnits::FloatingPoint),
            "0.45"
        );
        let fill = display_color([stored; 3]);
        for c in [fill.r(), fill.g(), fill.b()] {
            assert!(c.abs_diff(115) <= 1, "{fill:?}");
        }
        // The fill doesn't follow the number display.
        crate::value_display::set_display_space(false);
        assert_eq!(display_color([stored; 3]), fill);
        crate::value_display::set_display_space(true);
    }

    /// Press on the G number and drag right 100 px: G rises 0.5, R and B
    /// stay, and every frame of the drag reports the same gesture (one undo
    /// step).
    #[test]
    fn cn01_dragging_a_number_changes_that_component() {
        let ctx = Context::default();
        let mut rgb = [0.2, 0.3, 0.4];
        let (rect, _) = frame(&ctx, RawInput::default(), &mut rgb);
        let start = Pos2::new(rect.center().x, rect.center().y);
        let mut gestures = Vec::new();
        let press = Event::PointerButton {
            pos: start,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        };
        frame(
            &ctx,
            pointer(vec![Event::PointerMoved(start), press], Modifiers::NONE),
            &mut rgb,
        );
        for step in 1..=10 {
            let p = start + vec2(10. * step as f32, 0.);
            let (_, r) = frame(
                &ctx,
                pointer(vec![Event::PointerMoved(p)], Modifiers::NONE),
                &mut rgb,
            );
            gestures.extend(r.gesture);
        }
        frame(
            &ctx,
            pointer(
                vec![Event::PointerButton {
                    pos: start + vec2(100., 0.),
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::NONE,
                }],
                Modifiers::NONE,
            ),
            &mut rgb,
        );
        // egui starts a drag after a few pixels; the drag counts from there.
        assert!((rgb[0] - 0.2).abs() < 1e-12 && (rgb[2] - 0.4).abs() < 1e-12);
        assert!((rgb[1] - 0.8).abs() < 0.05, "{rgb:?}");
        assert!(!gestures.is_empty());
        assert!(gestures.iter().all(|&g| g == gestures[0]), "{gestures:?}");
    }

    /// CN-08's widget side: a Ctrl-drag reports the gang drag, which turns
    /// another colour's hue by the same angle.
    #[test]
    fn a_ctrl_drag_reports_a_relative_gang_edit() {
        let ctx = Context::default();
        let mut rgb = [1., 0., 0.];
        let (rect, _) = frame(&ctx, RawInput::default(), &mut rgb);
        let start = rect.center();
        frame(
            &ctx,
            pointer(
                vec![
                    Event::PointerMoved(start),
                    Event::PointerButton {
                        pos: start,
                        button: PointerButton::Primary,
                        pressed: true,
                        modifiers: Modifiers::CTRL,
                    },
                ],
                Modifiers::CTRL,
            ),
            &mut rgb,
        );
        let mut other = [0., 0., 1.];
        for step in 1..=6 {
            let (_, r) = frame(
                &ctx,
                pointer(
                    vec![Event::PointerMoved(start + vec2(10. * step as f32, 0.))],
                    Modifiers::CTRL,
                ),
                &mut rgb,
            );
            if let Some(g) = r.gang {
                other = g.apply(other);
            }
        }
        use operandi_core::colour_math::rgb_to_hsv;
        let turned = (rgb_to_hsv(rgb).h - 0.).rem_euclid(360.);
        let other_turned = (rgb_to_hsv(other).h - 240.).rem_euclid(360.);
        assert!(turned > 1., "the drag turned the hue: {rgb:?}");
        assert!(
            (turned - other_turned).abs() < 1e-6,
            "{turned} vs {other_turned}"
        );
    }

    /// Press and release at `p`, then at `q` (a pointer move between).
    fn press_release(ctx: &Context, rgb: &mut [f64; 3], p: Pos2, q: Pos2) {
        for (at, pressed) in [(p, true), (q, false)] {
            frame(
                ctx,
                pointer(
                    vec![
                        Event::PointerMoved(at),
                        Event::PointerButton {
                            pos: at,
                            button: PointerButton::Primary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                    Modifiers::NONE,
                ),
                rgb,
            );
        }
    }

    /// CN-05 / CN-Q01: a click on a number opens the picker on release,
    /// with no wait, and leaves the colour as it was.
    #[test]
    fn cn05_a_click_on_a_number_opens_the_picker_at_once() {
        let ctx = Context::default();
        let mut rgb = [0.5; 3];
        let (rect, _) = frame(&ctx, RawInput::default(), &mut rgb);
        // The R number, in the left third.
        let p = rect.left_center() + vec2(rect.width() / 6., 0.);
        press_release(&ctx, &mut rgb, p, p + vec2(2., 0.));
        assert!(colour_picker::is_open());
        assert_eq!(rgb, [0.5; 3]);
        colour_picker::close();
        // A second click (a double-click) only opens the picker again: no
        // number turns into a text field.
        press_release(&ctx, &mut rgb, p, p);
        press_release(&ctx, &mut rgb, p, p);
        assert!(colour_picker::is_open());
        assert_eq!(rgb, [0.5; 3]);
        colour_picker::close();
    }

    /// A press that moves further than 3 points before release is not a
    /// click.
    #[test]
    fn moving_more_than_three_points_is_not_a_click() {
        let ctx = Context::default();
        let mut rgb = [0.5; 3];
        let (rect, _) = frame(&ctx, RawInput::default(), &mut rgb);
        let p = rect.center();
        colour_picker::close();
        press_release(&ctx, &mut rgb, p, p + vec2(5., 0.));
        assert!(!colour_picker::is_open());
    }

    #[test]
    fn units_change_only_the_numbers() {
        let ctx = Context::default();
        for units in ColorUnits::ALL {
            value_display::set(units, crate::units_preferences::FactorDisplay::Percentage);
            let mut rgb = [0.25, 0.5, 2.];
            frame(&ctx, RawInput::default(), &mut rgb);
            assert_eq!(rgb, [0.25, 0.5, 2.]);
        }
        value_display::set(
            ColorUnits::FloatingPoint,
            crate::units_preferences::FactorDisplay::Percentage,
        );
    }
}
