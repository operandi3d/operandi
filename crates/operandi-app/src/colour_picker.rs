//! The Colour Picker popover (colour-nav note 07, Part B). A colour field's
//! click opens it next to the field; it is non-modal (edits apply live),
//! closes when the pointer rolls off it or on Esc, and edits whichever
//! field opened it.
//!
//! Left: the picker area (hue/saturation wheel, or a square of two
//! components) with a strip for the third below. Right: the colour model,
//! axis and Stops, the five numeric forms of the colour, and the current
//! colour swatch (with a swap arrow where the context has two colours).
//! The Options tab holds the Color Unit System (the same preference as
//! Preferences > Input), the wheel type and the dynamic strip.
//!
//! Harmony rules and presets (B6) are scaffolded only: listed, greyed.
use crate::colour_field::{Options, fill_color, next_gesture};
use crate::unavailable::Gate;
use crate::units_preferences::ColorUnits;
use crate::value_display;
use eframe::egui::{
    self, Color32, Context, Id, Mesh, Pos2, Rect, Sense, Shape, Stroke, Ui, pos2, vec2,
};
use operandi_core::colour_math::{
    self as cm, Hsv, KELVIN_DEFAULT, KELVIN_MAX, KELVIN_MIN, hsv_to_rgb, kelvin_to_rgb, rgb_to_hsv,
    stops_multiplier,
};
use std::cell::RefCell;

const AREA: f32 = 168.;
const STRIP: f32 = 14.;
/// How far the pointer may stray outside the popover and its field before
/// it rolls off.
const ROLL_OFF_MARGIN: f32 = 18.;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Model {
    #[default]
    Hsv,
    Kelvin,
    Rgb,
}
impl Model {
    const ALL: [Self; 3] = [Self::Hsv, Self::Kelvin, Self::Rgb];
    fn label(self) -> &'static str {
        match self {
            Self::Hsv => "HSV",
            Self::Kelvin => "Kelvin",
            Self::Rgb => "RGB",
        }
    }
}

/// Which component the strip controls; the square shows the other two.
/// 0/1/2 = H/S/V in HSV, R/G/B in RGB.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Axis(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WheelType {
    /// Default (OP).
    #[default]
    Rgb,
    /// The artist's red-yellow-blue wheel.
    Ryb,
}

/// B6 scaffold: harmony rules, listed for later.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HarmonyRule {
    #[default]
    Solo,
    Complementary,
    Analogous,
    Triadic,
    Tetradic,
    Compound,
    Tints,
    Shades,
}
impl HarmonyRule {
    pub const ALL: [Self; 8] = [
        Self::Solo,
        Self::Complementary,
        Self::Analogous,
        Self::Triadic,
        Self::Tetradic,
        Self::Compound,
        Self::Tints,
        Self::Shades,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Solo => "Solo",
            Self::Complementary => "Complementary",
            Self::Analogous => "Analogous",
            Self::Triadic => "Triadic",
            Self::Tetradic => "Tetradic",
            Self::Compound => "Compound",
            Self::Tints => "Tints",
            Self::Shades => "Shades",
        }
    }
}

/// The picker's settings, shared by every field (session-only for now).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickerOptions {
    pub model: Model,
    pub hue_as_wheel: bool,
    pub axis: Axis,
    pub dynamic_strip: bool,
    pub wheel: WheelType,
    /// Brightness multiplier stops² (B4). Never rescales the stored colour.
    pub stops: f64,
    pub harmony: HarmonyRule,
}
impl Default for PickerOptions {
    fn default() -> Self {
        Self {
            model: Model::Hsv,
            hue_as_wheel: true,
            axis: Axis(2),
            dynamic_strip: true,
            wheel: WheelType::Rgb,
            stops: 1.,
            harmony: HarmonyRule::Solo,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Picker,
    Options,
}

#[derive(Clone, Debug)]
struct Open {
    owner: Id,
    /// The popover's last rect, for roll-off.
    rect: Option<Rect>,
    /// The undo gesture of the press under way (one press-drag-release, or
    /// one typed commit, is one step).
    gesture: Option<u64>,
    tab: Tab,
    /// Kelvin while in the Kelvin model, so it doesn't drift through RGB.
    kelvin: Option<f64>,
    /// Hue kept through greys and black.
    hue: f64,
    hex: Option<String>,
}

thread_local! {
    /// The open field's values are display-encoded (theme colours).
    static ENCODED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static OPEN: RefCell<Option<Open>> = const { RefCell::new(None) };
    static OPTIONS: RefCell<PickerOptions> = RefCell::new(PickerOptions::default());
}

pub fn open(owner: Id, _anchor: Rect) {
    OPEN.with(|o| {
        *o.borrow_mut() = Some(Open {
            owner,
            rect: None,
            gesture: None,
            tab: Tab::Picker,
            kelvin: None,
            hue: 0.,
            hex: None,
        })
    });
}
pub fn close() {
    OPEN.with(|o| *o.borrow_mut() = None);
}
/// Whether a picker is open (Viewport Properties doesn't roll off while
/// one of its colour fields has the picker out).
pub fn is_open() -> bool {
    OPEN.with(|o| o.borrow().is_some())
}

/// Draw the popover when field `owner` opened it. Returns the gesture of a
/// change made in it this frame.
pub fn show_for(
    ctx: &Context,
    owner: Id,
    field: Rect,
    rgb: &mut [f64; 3],
    options: Options,
) -> Option<u64> {
    show_pair(ctx, owner, field, rgb, None, options)
}

/// `show_for` where the context has two colours (foreground/background):
/// the swatch area gets a swap arrow.
pub fn show_pair(
    ctx: &Context,
    owner: Id,
    field: Rect,
    rgb: &mut [f64; 3],
    other: Option<&mut [f64; 3]>,
    options: Options,
) -> Option<u64> {
    let mut open = OPEN
        .with(|o| o.borrow().clone())
        .filter(|o| o.owner == owner)?;
    ENCODED.with(|e| e.set(options.encoded));
    let mut opts = options_now();
    let before = *rgb;
    let mut swapped = false;
    // Below the field, or above when there's no room.
    let screen = ctx.content_rect();
    let height = AREA + STRIP + 120.;
    let pos = if field.bottom() + height < screen.bottom() {
        field.left_bottom() + vec2(0., 2.)
    } else {
        field.left_top() - vec2(0., height + 2.)
    };
    let area = egui::Area::new(Id::new("colour-picker-popover"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut open.tab, Tab::Picker, "Picker");
                    ui.selectable_value(&mut open.tab, Tab::Options, "Options");
                });
                match open.tab {
                    Tab::Picker => picker_tab(ui, &mut open, &mut opts, rgb, options.hdr),
                    Tab::Options => options_tab(ui, &mut opts),
                }
                ui.horizontal(|ui| {
                    // Bottom right: the current colour, with a swap arrow
                    // where the context has two colours.
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(44., 22.), Sense::hover());
                        ui.painter().rect_filled(r, 2., fill(*rgb));
                        ui.painter().rect_stroke(
                            r,
                            2.,
                            Stroke::new(1_f32, crate::theme::border()),
                            egui::StrokeKind::Inside,
                        );
                        if let Some(other) = other {
                            if ui.small_button("⇄").on_hover_text("Swap colors").clicked() {
                                std::mem::swap(rgb, other);
                                swapped = true;
                            }
                            let (r, _) = ui.allocate_exact_size(vec2(22., 22.), Sense::hover());
                            ui.painter().rect_filled(r, 2., fill(*other));
                        }
                    });
                });
            });
        });
    let rect = area.response.rect;
    open.rect = Some(rect);
    OPTIONS.with(|o| *o.borrow_mut() = opts);
    // B1: roll-off closes; Esc closes. Edits already applied stay.
    let (pointer, down, escape) = ctx.input(|i| {
        (
            i.pointer.hover_pos(),
            i.pointer.any_down(),
            i.key_pressed(egui::Key::Escape),
        )
    });
    let inside = |p: Pos2| {
        rect.expand(ROLL_OFF_MARGIN).contains(p)
            || field.expand(ROLL_OFF_MARGIN).contains(p)
            || Rect::from_two_pos(field.center(), rect.center()).contains(p)
    };
    let rolled_off = pointer.is_some_and(|p| !inside(p)) && !down;
    if !down {
        // The press (if any) ended: the next change is a new step.
        open.gesture.take();
    }
    let changed = *rgb != before || swapped;
    let gesture = changed.then(|| *open.gesture.get_or_insert_with(next_gesture));
    if rolled_off || escape {
        close();
    } else {
        OPEN.with(|o| *o.borrow_mut() = Some(open));
    }
    gesture
}

fn fill(rgb: [f64; 3]) -> Color32 {
    fill_color(rgb, ENCODED.with(std::cell::Cell::get))
}

fn options_now() -> PickerOptions {
    OPTIONS.with(|o| *o.borrow())
}

fn picker_tab(
    ui: &mut Ui,
    open: &mut Open,
    opts: &mut PickerOptions,
    rgb: &mut [f64; 3],
    hdr: bool,
) {
    let top = if hdr { f64::INFINITY } else { 1. };
    let mult = stops_multiplier(opts.stops);
    let hsv = rgb_to_hsv(*rgb);
    if hsv.s > 1e-6 && hsv.v > 1e-9 {
        open.hue = hsv.h;
    }
    let hsv = Hsv { h: open.hue, ..hsv };
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            area_control(ui, open, opts, rgb, hsv, mult, top);
            strip_control(ui, open, opts, rgb, hsv, mult, top);
        });
        ui.vertical(|ui| {
            ui.spacing_mut().interact_size.x = 52.;
            egui::Grid::new("picker-settings")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Color Model");
                    egui::ComboBox::from_id_salt("picker-model")
                        .selected_text(opts.model.label())
                        .show_ui(ui, |ui| {
                            for m in Model::ALL {
                                ui.selectable_value(&mut opts.model, m, m.label());
                            }
                        });
                    ui.end_row();
                    if opts.model == Model::Hsv {
                        ui.label("Hue as Wheel");
                        ui.checkbox(&mut opts.hue_as_wheel, "");
                        ui.end_row();
                    }
                    if opts.model != Model::Kelvin
                        && !(opts.model == Model::Hsv && opts.hue_as_wheel)
                    {
                        ui.label("Axis");
                        let names = axis_names(opts.model);
                        egui::ComboBox::from_id_salt("picker-axis")
                            .selected_text(names[opts.axis.0.min(2)])
                            .show_ui(ui, |ui| {
                                for (i, n) in names.iter().enumerate() {
                                    ui.selectable_value(&mut opts.axis, Axis(i), *n);
                                }
                            });
                        ui.end_row();
                    }
                    ui.label("Stops").on_hover_text(
                        "Brightness multiplier for picking (stops²): 1.4 ≈ 2×, 2 = 4×. \
                     Doesn't rescale the color.",
                    );
                    ui.add(
                        crate::numeric_tab::DragValue::new(&mut opts.stops)
                            .range(0.1..=8.)
                            .speed(0.01)
                            .max_decimals(2),
                    );
                    ui.end_row();
                });
            ui.separator();
            numeric_fields(ui, open, rgb, top);
        });
    });
    if opts.model != Model::Kelvin {
        open.kelvin = None;
    }
}

fn axis_names(model: Model) -> [&'static str; 3] {
    match model {
        Model::Rgb => ["Red", "Green", "Blue"],
        _ => ["Hue", "Saturation", "Value"],
    }
}

/// The colour at a picker-area point `(x, y)` (0–1, y up), keeping the
/// strip's component from `hsv`/`rgb`.
fn area_colour(
    opts: &PickerOptions,
    open: &Open,
    rgb: [f64; 3],
    hsv: Hsv,
    mult: f64,
    x: f64,
    y: f64,
) -> [f64; 3] {
    match opts.model {
        Model::Kelvin => {
            let k = KELVIN_MIN + x * (KELVIN_MAX - KELVIN_MIN);
            kelvin_to_rgb(k).map(|c| c * y * mult)
        }
        Model::Rgb => {
            let mut out = rgb;
            let [a, b] = others(opts.axis.0);
            out[a] = x * mult;
            out[b] = y * mult;
            out
        }
        Model::Hsv => {
            let mut h = [hsv.h / 360., hsv.s, hsv.v / mult];
            let [a, b] = others(opts.axis.0);
            h[a] = x;
            h[b] = y;
            let _ = open;
            hsv_to_rgb(Hsv {
                h: h[0] * 360.,
                s: h[1],
                v: h[2] * mult,
            })
        }
    }
}

fn others(axis: usize) -> [usize; 2] {
    match axis {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    }
}

fn wheel_hue(opts: &PickerOptions, angle: f64) -> f64 {
    match opts.wheel {
        WheelType::Rgb => angle,
        WheelType::Ryb => cm::ryb_to_rgb_hue(angle),
    }
}
fn wheel_angle(opts: &PickerOptions, hue: f64) -> f64 {
    match opts.wheel {
        WheelType::Rgb => hue,
        WheelType::Ryb => cm::rgb_to_ryb_hue(hue),
    }
}

/// Shown colour of a value in the picker, for drawing.
fn shown(rgb: [f64; 3], mult: f64) -> Color32 {
    fill(rgb.map(|c| c / mult.max(1e-9)))
}

fn area_control(
    ui: &mut Ui,
    open: &mut Open,
    opts: &PickerOptions,
    rgb: &mut [f64; 3],
    hsv: Hsv,
    mult: f64,
    top: f64,
) {
    let (rect, response) = ui.allocate_exact_size(vec2(AREA, AREA), Sense::click_and_drag());
    let painter = ui.painter_at(rect.expand(4.));
    let wheel = opts.model == Model::Hsv && opts.hue_as_wheel;
    if wheel {
        let c = rect.center();
        let radius = rect.width() / 2.;
        let v = (hsv.v / mult).clamp(0., 1.);
        let mut mesh = Mesh::default();
        const RINGS: usize = 8;
        const SEGS: usize = 64;
        for ring in 0..=RINGS {
            let s = ring as f64 / RINGS as f64;
            for seg in 0..=SEGS {
                let a = seg as f64 / SEGS as f64 * 360.;
                let col = fill(hsv_to_rgb(Hsv {
                    h: wheel_hue(opts, a),
                    s,
                    v,
                }));
                let (sin, cos) = a.to_radians().sin_cos();
                let p = c + vec2(cos as f32, -sin as f32) * radius * s as f32;
                mesh.colored_vertex(p, col);
            }
        }
        for ring in 0..RINGS {
            for seg in 0..SEGS {
                let i = (ring * (SEGS + 1) + seg) as u32;
                let j = i + (SEGS + 1) as u32;
                mesh.add_triangle(i, i + 1, j);
                mesh.add_triangle(i + 1, j + 1, j);
            }
        }
        painter.add(Shape::mesh(mesh));
        let a = wheel_angle(opts, hsv.h).to_radians();
        let marker = c + vec2(a.cos() as f32, -a.sin() as f32) * radius * hsv.s.min(1.) as f32;
        marker_at(&painter, marker);
        if let Some(p) = response.interact_pointer_pos()
            && (response.dragged() || response.clicked() || response.is_pointer_button_down_on())
        {
            let d = p - c;
            let s = (f64::from(d.length()) / f64::from(radius)).min(1.);
            let angle = f64::from(-d.y)
                .atan2(f64::from(d.x))
                .to_degrees()
                .rem_euclid(360.);
            let h = wheel_hue(opts, angle);
            open.hue = h;
            set(
                rgb,
                hsv_to_rgb(Hsv {
                    h,
                    s,
                    v: hsv.v.max(if hsv.v <= 0. { mult } else { 0. }),
                }),
                top,
            );
        }
    } else {
        // A square: 2D gradient of the area's two components.
        const N: usize = 16;
        let mut mesh = Mesh::default();
        for yi in 0..=N {
            for xi in 0..=N {
                let (x, y) = (xi as f64 / N as f64, yi as f64 / N as f64);
                let col = shown(area_colour(opts, open, *rgb, hsv, mult, x, y), mult);
                mesh.colored_vertex(
                    pos2(
                        rect.left() + rect.width() * x as f32,
                        rect.bottom() - rect.height() * y as f32,
                    ),
                    col,
                );
            }
        }
        for yi in 0..N {
            for xi in 0..N {
                let i = (yi * (N + 1) + xi) as u32;
                let j = i + (N + 1) as u32;
                mesh.add_triangle(i, i + 1, j);
                mesh.add_triangle(i + 1, j + 1, j);
            }
        }
        painter.add(Shape::mesh(mesh));
        let (x, y) = area_position(opts, open, *rgb, hsv, mult);
        marker_at(
            &painter,
            pos2(
                rect.left() + rect.width() * x as f32,
                rect.bottom() - rect.height() * y as f32,
            ),
        );
        if let Some(p) = response.interact_pointer_pos()
            && (response.dragged() || response.clicked() || response.is_pointer_button_down_on())
        {
            let x = f64::from((p.x - rect.left()) / rect.width()).clamp(0., 1.);
            let y = f64::from((rect.bottom() - p.y) / rect.height()).clamp(0., 1.);
            if opts.model == Model::Kelvin {
                open.kelvin = Some(KELVIN_MIN + x * (KELVIN_MAX - KELVIN_MIN));
            }
            set(rgb, area_colour(opts, open, *rgb, hsv, mult, x, y), top);
        }
    }
}

fn area_position(
    opts: &PickerOptions,
    open: &Open,
    rgb: [f64; 3],
    hsv: Hsv,
    mult: f64,
) -> (f64, f64) {
    match opts.model {
        Model::Kelvin => {
            let k = open.kelvin.unwrap_or_else(|| cm::rgb_to_kelvin(rgb));
            (
                (k - KELVIN_MIN) / (KELVIN_MAX - KELVIN_MIN),
                (hsv.v / mult).clamp(0., 1.),
            )
        }
        Model::Rgb => {
            let [a, b] = others(opts.axis.0);
            ((rgb[a] / mult).clamp(0., 1.), (rgb[b] / mult).clamp(0., 1.))
        }
        Model::Hsv => {
            let h = [hsv.h / 360., hsv.s, (hsv.v / mult).clamp(0., 1.)];
            let [a, b] = others(opts.axis.0);
            (h[a], h[b])
        }
    }
}

/// The strip: value (wheel), the axis component (square), or temperature
/// (Kelvin). `t` runs 0–1 left to right.
fn strip_colour(
    opts: &PickerOptions,
    rgb: [f64; 3],
    hsv: Hsv,
    mult: f64,
    t: f64,
    dynamic: bool,
) -> [f64; 3] {
    match opts.model {
        Model::Kelvin => kelvin_to_rgb(KELVIN_MIN + t * (KELVIN_MAX - KELVIN_MIN))
            .map(|c| c * if dynamic { hsv.v.min(mult) } else { mult }),
        Model::Hsv if opts.hue_as_wheel => {
            if dynamic {
                hsv_to_rgb(Hsv { v: t * mult, ..hsv })
            } else {
                [t * mult; 3]
            }
        }
        Model::Hsv => {
            let mut h = [hsv.h / 360., hsv.s, hsv.v / mult];
            if !dynamic {
                h = [0., 1., 1.];
                if opts.axis.0 == 1 {
                    h = [0., 0., 1.];
                }
            }
            h[opts.axis.0] = t;
            hsv_to_rgb(Hsv {
                h: h[0] * 360.,
                s: h[1],
                v: h[2] * mult,
            })
        }
        Model::Rgb => {
            let mut out = if dynamic { rgb } else { [0.; 3] };
            out[opts.axis.0.min(2)] = t * mult;
            out
        }
    }
}

fn strip_position(opts: &PickerOptions, open: &Open, rgb: [f64; 3], hsv: Hsv, mult: f64) -> f64 {
    match opts.model {
        Model::Kelvin => {
            let k = open.kelvin.unwrap_or_else(|| cm::rgb_to_kelvin(rgb));
            (k - KELVIN_MIN) / (KELVIN_MAX - KELVIN_MIN)
        }
        Model::Hsv if opts.hue_as_wheel => (hsv.v / mult).clamp(0., 1.),
        Model::Hsv => [hsv.h / 360., hsv.s, (hsv.v / mult).clamp(0., 1.)][opts.axis.0.min(2)],
        Model::Rgb => (rgb[opts.axis.0.min(2)] / mult).clamp(0., 1.),
    }
}

/// The colour picked at strip position `t` (0–1).
pub(crate) fn strip_pick(opts: &PickerOptions, rgb: [f64; 3], hue: f64, t: f64) -> [f64; 3] {
    let mult = stops_multiplier(opts.stops);
    let hsv = Hsv {
        h: hue,
        ..rgb_to_hsv(rgb)
    };
    match opts.model {
        Model::Kelvin => kelvin_to_rgb(KELVIN_MIN + t * (KELVIN_MAX - KELVIN_MIN))
            .map(|c| c * if hsv.v > 0. { hsv.v } else { mult }),
        _ => strip_colour(opts, rgb, hsv, mult, t, true),
    }
}

fn strip_control(
    ui: &mut Ui,
    open: &mut Open,
    opts: &PickerOptions,
    rgb: &mut [f64; 3],
    hsv: Hsv,
    mult: f64,
    top: f64,
) {
    let (rect, response) = ui.allocate_exact_size(vec2(AREA, STRIP), Sense::click_and_drag());
    let painter = ui.painter_at(rect.expand(4.));
    const N: usize = 32;
    let mut mesh = Mesh::default();
    for i in 0..=N {
        let t = i as f64 / N as f64;
        let col = shown(
            strip_colour(opts, *rgb, hsv, mult, t, opts.dynamic_strip),
            mult,
        );
        let x = rect.left() + rect.width() * t as f32;
        mesh.colored_vertex(pos2(x, rect.top()), col);
        mesh.colored_vertex(pos2(x, rect.bottom()), col);
    }
    for i in 0..N as u32 {
        let k = i * 2;
        mesh.add_triangle(k, k + 1, k + 2);
        mesh.add_triangle(k + 1, k + 3, k + 2);
    }
    painter.add(Shape::mesh(mesh));
    let t = strip_position(opts, open, *rgb, hsv, mult) as f32;
    let x = rect.left() + rect.width() * t;
    painter.line_segment(
        [pos2(x, rect.top() - 2.), pos2(x, rect.bottom() + 2.)],
        Stroke::new(2_f32, Color32::WHITE),
    );
    painter.line_segment(
        [pos2(x, rect.top() - 2.), pos2(x, rect.bottom() + 2.)],
        Stroke::new(1_f32, Color32::BLACK),
    );
    if let Some(p) = response.interact_pointer_pos()
        && (response.dragged() || response.clicked() || response.is_pointer_button_down_on())
    {
        let t = f64::from((p.x - rect.left()) / rect.width()).clamp(0., 1.);
        if opts.model == Model::Kelvin {
            open.kelvin = Some(KELVIN_MIN + t * (KELVIN_MAX - KELVIN_MIN));
        }
        set(rgb, strip_pick(opts, *rgb, open.hue, t), top);
    }
}

fn marker_at(painter: &egui::Painter, p: Pos2) {
    painter.circle_stroke(p, 5., Stroke::new(2_f32, Color32::WHITE));
    painter.circle_stroke(p, 5., Stroke::new(1_f32, Color32::BLACK));
}

fn set(rgb: &mut [f64; 3], value: [f64; 3], top: f64) {
    *rgb = value.map(|c| if c.is_finite() { c.clamp(0., top) } else { 0. });
}

/// A component field in the Color Unit System, in display space when that
/// preference is on. True when it changed.
fn component(ui: &mut Ui, value: &mut f64, max: f64) -> bool {
    let encoded = ENCODED.with(std::cell::Cell::get);
    let mut shown = if encoded {
        *value
    } else {
        value_display::to_shown(*value)
    };
    let before = shown;
    value_display::color_component(ui, &mut shown, max);
    if shown != before {
        *value = if encoded {
            shown
        } else {
            value_display::from_shown(shown)
        }
        .min(max);
        true
    } else {
        false
    }
}

/// B5: the colour in five forms, all editable and live.
fn numeric_fields(ui: &mut Ui, open: &mut Open, rgb: &mut [f64; 3], top: f64) {
    egui::Grid::new("picker-numbers")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("RGB (HDR)");
            ui.horizontal(|ui| {
                for c in rgb.iter_mut() {
                    component(ui, c, top);
                }
            });
            ui.end_row();
            ui.label("RGB");
            ui.horizontal(|ui| {
                for c in rgb.iter_mut() {
                    let mut clipped = c.min(1.);
                    if component(ui, &mut clipped, 1.) {
                        *c = clipped;
                    }
                }
            });
            ui.end_row();
            ui.label("Kelvin");
            let mut k = open.kelvin.unwrap_or_else(|| cm::rgb_to_kelvin(*rgb));
            if ui
                .add(
                    crate::numeric_tab::DragValue::new(&mut k)
                        .range(KELVIN_MIN..=KELVIN_MAX)
                        .speed(10.)
                        .max_decimals(0)
                        .suffix(" K"),
                )
                .changed()
            {
                open.kelvin = Some(k);
                let v = rgb_to_hsv(*rgb).v.min(1.);
                let v = if v > 0. { v } else { 1. };
                *rgb = kelvin_to_rgb(k).map(|c| c * v);
            }
            ui.end_row();
            ui.label("HSV");
            ui.horizontal(|ui| {
                let mut hsv = rgb_to_hsv(*rgb);
                if hsv.s <= 1e-6 || hsv.v <= 1e-9 {
                    hsv.h = open.hue;
                }
                let mut v = hsv.v.min(1.);
                let mut changed = ui
                    .add(
                        crate::numeric_tab::DragValue::new(&mut hsv.h)
                            .range(0.0..=360.)
                            .speed(1.)
                            .max_decimals(1)
                            .suffix("°"),
                    )
                    .changed();
                changed |= ui
                    .add(
                        crate::numeric_tab::DragValue::new(&mut hsv.s)
                            .range(0.0..=1.)
                            .speed(0.005)
                            .max_decimals(3),
                    )
                    .changed();
                changed |= component(ui, &mut v, 1.);
                if changed {
                    open.hue = hsv.h;
                    *rgb = hsv_to_rgb(Hsv { v, ..hsv });
                }
            });
            ui.end_row();
            ui.label("Hex RGB");
            let display = value_display::display_space();
            let text = open.hex.get_or_insert_with(|| cm::hex_rgb(*rgb, display));
            let response = ui.add(egui::TextEdit::singleline(text).desired_width(80.));
            crate::numeric_tab::register(ui, &response, Some(text.as_str()));
            if response.lost_focus() {
                if let Some(c) = cm::parse_hex_rgb(text, display) {
                    *rgb = c;
                }
                open.hex = None;
            } else if !response.has_focus() {
                open.hex = None;
            }
            ui.end_row();
        });
    if !open
        .kelvin
        .is_some_and(|k| (KELVIN_MIN..=KELVIN_MAX).contains(&k))
    {
        open.kelvin = None;
    }
}

fn options_tab(ui: &mut Ui, opts: &mut PickerOptions) {
    egui::Grid::new("picker-options")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Color Unit System");
            let mut units = value_display::color_units();
            egui::ComboBox::from_id_salt("picker-units")
                .selected_text(units.label())
                .show_ui(ui, |ui| {
                    for u in ColorUnits::ALL {
                        ui.selectable_value(&mut units, u, u.label());
                    }
                });
            if units != value_display::color_units() {
                value_display::pick_color_units(units);
            }
            ui.end_row();
            ui.label("Wheel");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut opts.wheel, WheelType::Rgb, "RGB");
                ui.selectable_value(&mut opts.wheel, WheelType::Ryb, "RYB (artist)");
            });
            ui.end_row();
            ui.label("Dynamic Color Strip");
            ui.checkbox(&mut opts.dynamic_strip, "");
            ui.end_row();
            // B6 scaffold: listed, not built yet.
            ui.label("Harmony");
            ui.gated_ui(Some("Harmony rules come later"), |ui| {
                egui::ComboBox::from_id_salt("picker-harmony")
                    .selected_text(opts.harmony.label())
                    .show_ui(ui, |ui| {
                        for r in HarmonyRule::ALL {
                            ui.selectable_value(&mut opts.harmony, r, r.label());
                        }
                    });
            });
            ui.end_row();
            ui.label("Presets");
            ui.gated(
                Some("Color presets come later"),
                egui::Button::new("Save Current Color"),
            );
            ui.end_row();
        });
    let _ = KELVIN_DEFAULT;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CN-07: with Stops 2, the brightest point of the value strip (and so
    /// of the wheel) picks value 4.
    #[test]
    fn cn07_stops_two_picks_value_four() {
        let opts = PickerOptions {
            stops: 2.,
            ..Default::default()
        };
        let picked = strip_pick(&opts, [1., 0.5, 0.25], 20., 1.);
        assert!((rgb_to_hsv(picked).v - 4.).abs() < 1e-9, "{picked:?}");
        let one = PickerOptions::default();
        assert!((rgb_to_hsv(strip_pick(&one, [1., 0.5, 0.25], 20., 1.)).v - 1.).abs() < 1e-9);
    }

    #[test]
    fn kelvin_strip_runs_warm_to_cool() {
        let opts = PickerOptions {
            model: Model::Kelvin,
            ..Default::default()
        };
        let warm = strip_pick(&opts, [1.; 3], 0., 0.);
        let cool = strip_pick(&opts, [1.; 3], 0., 1.);
        assert!(warm[0] > warm[2] && cool[2] > cool[0]);
    }

    #[test]
    fn the_popover_draws_and_edits_live() {
        let ctx = Context::default();
        let owner = Id::new("field");
        open(owner, Rect::from_min_size(pos2(10., 10.), vec2(200., 20.)));
        let mut rgb = [0.5, 0.25, 0.1];
        for tab in [Tab::Picker, Tab::Options] {
            OPEN.with(|o| o.borrow_mut().as_mut().unwrap().tab = tab);
            let _ = ctx.run(Default::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let field = ui.allocate_exact_size(vec2(200., 20.), Sense::hover()).0;
                    show_for(ui.ctx(), owner, field, &mut rgb, Options::scene());
                });
            });
        }
        // Drawing alone changes nothing; no pointer, so it stays open.
        assert_eq!(rgb, [0.5, 0.25, 0.1]);
        assert!(is_open());
        close();
    }
}
