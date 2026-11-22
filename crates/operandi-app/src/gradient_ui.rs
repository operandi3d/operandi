//! The Gradient layer's properties (gradient family v1 §1): Input
//! Parameter, the inline Value and Color strips and a numeric key list,
//! and the Edit Gradient window (GR-Q03 v2): a larger editor with Presets,
//! a Recent group, Reverse, Distribute Evenly and Reset.
use crate::theme::workspace as palette;
use crate::unavailable::Gate;
use eframe::egui::{self, Color32, ComboBox, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use operandi_core::gradient::{Curve, CurveOut, Gradient, GradientInput, Key, KeyShape, Unit};

/// Draws the Gradient's settings; `color` when its effect reads the Color
/// curve. Returns whether a drag continues (one drag, one undo step).
/// `input_driven`: free nodes drive the Input (NE-03), so its row is
/// greyed with the driven light (NE-04, UIC-112).
pub(crate) fn properties(ui: &mut Ui, g: &mut Gradient, color: bool, input_driven: bool) -> bool {
    if input_driven {
        crate::form::rows::set_status(crate::form::status::ChannelStatus::Driven);
    }
    let why =
        input_driven.then_some("Driven by nodes: remove the wire in the node view to edit it");
    ui.gated_ui(why, |ui| input_row(ui, g));
    let unit = g.input.unit();
    curves(ui, &mut g.value, &mut g.color, unit, color, Heights::INLINE)
}

/// The Input Parameter row.
fn input_row(ui: &mut Ui, g: &mut Gradient) {
    crate::option_row::field_row(ui, "Input Parameter", |ui| {
        ComboBox::from_id_salt("gradient-input")
            .selected_text(g.input.label())
            .height(420.)
            .show_ui(ui, |ui| {
                for group in GradientInput::GROUPS {
                    ui.weak(*group);
                    for &input in GradientInput::ALL.iter().filter(|i| i.group() == *group) {
                        let label = if input.is_stored_only() {
                            format!("{} (stored)", input.label())
                        } else {
                            input.label().to_owned()
                        };
                        ui.selectable_value(&mut g.input, input, label)
                            .on_hover_text(if input.is_stored_only() {
                                "Kept with the scene; renders as 0 until it is built"
                            } else {
                                ""
                            });
                    }
                }
            });
    });
}

/// Strip heights: the panel's inline strips, or the window's larger ones.
#[derive(Clone, Copy)]
struct Heights {
    value: f32,
    color: f32,
}
impl Heights {
    const INLINE: Heights = Heights {
        value: 46.,
        color: 26.,
    };
    const WINDOW: Heights = Heights {
        value: 180.,
        color: 56.,
    };
}

/// Most curves the window's Recent group keeps (GR-Q09).
pub(crate) const RECENT: usize = 8;

/// Remembers `g`'s curves at the front of `recent` (once; the oldest go).
pub(crate) fn remember(recent: &mut Vec<Gradient>, g: &Gradient) {
    recent.retain(|r| r.value != g.value || r.color != g.color);
    recent.insert(0, g.clone());
    recent.truncate(RECENT);
}

/// The Edit Gradient window over `g` (GR-Q03 v2), `color` when the layer's
/// effect reads the Color curve. Returns (still open, a drag continues).
/// Presets and Recent replace the curve the effect reads, over its keys'
/// span (the input's usual range when they span nothing); Reverse,
/// Distribute Evenly and Reset act on it too.
pub(crate) fn window(
    ctx: &egui::Context,
    title: &str,
    g: &mut Gradient,
    color: bool,
    recent: &mut Vec<Gradient>,
) -> (bool, bool) {
    use operandi_core::gradient::Preset;
    let mut open = true;
    let mut dragging = false;
    egui::Window::new(format!("Edit Gradient: {title}"))
        .id(egui::Id::new("edit-gradient-window"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_width(520.)
        .show(ctx, |ui| {
            input_row(ui, g);
            ui.horizontal(|ui| {
                let span = |g: &Gradient| {
                    let s = if color {
                        g.color.span()
                    } else {
                        g.value.span()
                    };
                    match s {
                        Some((lo, hi)) if hi - lo > 1e-9 => (lo, hi),
                        _ => g.input.natural_span(),
                    }
                };
                ui.menu_button("Presets ▾", |ui| {
                    let list: &[Preset] = if color {
                        &Preset::COLOR
                    } else {
                        &Preset::VALUE
                    };
                    for p in list {
                        if ui.button(p.label(color)).clicked() {
                            remember(recent, g);
                            let (lo, hi) = span(g);
                            if color {
                                g.color = p.color(lo, hi);
                            } else {
                                g.value = p.value(lo, hi);
                            }
                            ui.close();
                        }
                    }
                    ui.separator();
                    ui.weak("Recent");
                    if recent.is_empty() {
                        ui.weak("(none yet)");
                    }
                    let mut pick = None;
                    for (k, r) in recent.iter().enumerate() {
                        let keys = if color {
                            r.color.keys.len()
                        } else {
                            r.value.keys.len()
                        };
                        if ui
                            .button(format!("{}. {}, {keys} keys", k + 1, r.input.label()))
                            .on_hover_text("Use this curve again (keeps the Input Parameter)")
                            .clicked()
                        {
                            pick = Some(r.clone());
                            ui.close();
                        }
                    }
                    if let Some(r) = pick {
                        remember(recent, g);
                        if color {
                            g.color = r.color;
                        } else {
                            g.value = r.value;
                        }
                    }
                })
                .response
                .on_hover_text("Built-in curves over the keys' span, and curves used before");
                if ui
                    .button("Reverse")
                    .on_hover_text("Mirror the keys across their span")
                    .clicked()
                {
                    if color {
                        g.color = g.color.reversed();
                    } else {
                        g.value = g.value.reversed();
                    }
                }
                if ui
                    .button("Distribute Evenly")
                    .on_hover_text("Space the keys evenly across their span")
                    .clicked()
                {
                    if color {
                        g.color = g.color.distributed();
                    } else {
                        g.value = g.value.distributed();
                    }
                }
                if ui
                    .button("Reset")
                    .on_hover_text("Back to a new Gradient's single key")
                    .clicked()
                {
                    remember(recent, g);
                    let fresh = Gradient::default();
                    if color {
                        g.color = fresh.color;
                    } else {
                        g.value = fresh.value;
                    }
                }
            });
            ui.separator();
            let unit = g.input.unit();
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .max_height(520.)
                .show(ui, |ui| {
                    dragging |=
                        curves(ui, &mut g.value, &mut g.color, unit, color, Heights::WINDOW);
                });
        });
    (open, dragging)
}

/// A Variation Texture (gradient family §4): its source and the curves
/// over the element's random number (0..1).
pub(crate) fn variation(
    ui: &mut Ui,
    v: &mut operandi_core::vertex_map_texture::Variation,
    color: bool,
) -> bool {
    use operandi_core::vertex_map_texture::VariationSource;
    crate::option_row::field_row(ui, "Variation Source", |ui| {
        ComboBox::from_id_salt("variation-source")
            .selected_text(v.source.label())
            .show_ui(ui, |ui| {
                for s in VariationSource::ALL {
                    let label = if s.is_stored_only() {
                        format!("{} (stored)", s.label())
                    } else {
                        s.label().to_owned()
                    };
                    ui.selectable_value(&mut v.source, s, label);
                }
            });
    });
    if v.source.is_stored_only() {
        ui.weak("Operandi has no particles yet: every element reads 0");
    }
    curves(
        ui,
        &mut v.value,
        &mut v.color,
        Unit::Number,
        color,
        Heights::INLINE,
    )
}

/// The Value or Color strip and key list.
fn curves(
    ui: &mut Ui,
    value: &mut Curve<f64>,
    color_curve: &mut Curve<[f64; 4]>,
    unit: Unit,
    color: bool,
    heights: Heights,
) -> bool {
    let mut dragging = false;
    let g = &mut (value, color_curve);
    ui.add_space(4.);
    ui.strong(if color { "Color" } else { "Value" });
    ui.weak("Middle-click (Ctrl+Alt+click) adds a key; drag a key to move it");
    if color {
        dragging |= strip(ui, "gradient-color", g.1, true, heights.color);
        dragging |= key_list(ui, "gradient-color-keys", g.1, unit, |ui, out| {
            // The shared colour field; alpha stays a separate field (A1).
            let mut rgb = [out[0], out[1], out[2]];
            let field = ui
                .allocate_ui(egui::vec2(150., ui.spacing().interact_size.y), |ui| {
                    crate::colour_field::show(ui, &mut rgb, crate::colour_field::Options::scene())
                })
                .inner;
            if field.changed() {
                out[..3].copy_from_slice(&rgb);
            }
            let mut a = out[3] * 100.;
            let r = ui.add(
                crate::numeric_tab::DragValue::new(&mut a)
                    .range(0.0..=100.0)
                    .speed(0.5)
                    .suffix(" %"),
            );
            out[3] = a / 100.;
            r.dragged() && !r.drag_started()
        });
    } else {
        dragging |= strip(ui, "gradient-value", g.0, false, heights.value);
        dragging |= key_list(ui, "gradient-value-keys", g.0, unit, |ui, out| {
            let mut v = *out * 100.;
            let r = ui.add(
                crate::numeric_tab::DragValue::new(&mut v)
                    .range(-1e6..=1e6)
                    .speed(0.5)
                    .suffix(" %"),
            );
            *out = v / 100.;
            r.dragged() && !r.drag_started()
        });
    }
    dragging
}

/// The span a strip shows: the keys' span, widened to at least one unit.
fn view(curve: &Curve<impl CurveOut>) -> (f64, f64) {
    let (lo, hi) = curve.span().unwrap_or((0., 1.));
    if hi - lo < 1e-9 {
        (lo - 0.5, lo + 0.5)
    } else {
        let pad = (hi - lo) * 0.05;
        (lo - pad, hi + pad)
    }
}

/// A ramp colour drawn like its keys' swatches (colour_field::display_color).
fn to_color32(v: [f64; 4]) -> Color32 {
    crate::colour_field::display_color([v[0], v[1], v[2]])
}

/// One strip: the curve (Value) or its colours (Color) over the key span,
/// key handles to drag, middle-click to add.
fn strip<V: CurveOut>(
    ui: &mut Ui,
    id: &str,
    curve: &mut Curve<V>,
    color: bool,
    height: f32,
) -> bool {
    let width = ui.available_width().max(120.);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let (lo, hi) = view(curve);
    let x_of = |at: f64| rect.left() + ((at - lo) / (hi - lo)) as f32 * rect.width();
    let at_of = |x: f32| lo + f64::from((x - rect.left()) / rect.width()) * (hi - lo);
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2., palette::list());
    let steps = (rect.width() as usize).clamp(16, 400);
    // The output range a Value strip draws.
    let (vlo, vhi) = if color {
        (0., 1.)
    } else {
        let (mut a, mut b) = (0f64, 1f64);
        for k in &curve.keys {
            a = a.min(k.out.get(0));
            b = b.max(k.out.get(0));
        }
        (a, b)
    };
    let y_of = |v: f64| {
        rect.bottom() - 4. - ((v - vlo) / (vhi - vlo).max(1e-9)) as f32 * (rect.height() - 8.)
    };
    let mut line = Vec::with_capacity(steps + 1);
    for k in 0..=steps {
        let x = rect.left() + rect.width() * k as f32 / steps as f32;
        let out = curve.sample(at_of(x));
        if color {
            let c = to_color32(std::array::from_fn(|i| out.get(i.min(V::N - 1))));
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(rect.width() / steps as f32 + 0.5, c),
            );
        } else {
            line.push(Pos2::new(x, y_of(out.get(0))));
        }
    }
    if !color {
        painter.add(egui::Shape::line(
            line,
            Stroke::new(1.5_f32, palette::field_text()),
        ));
    }
    painter.rect_stroke(
        rect,
        2.,
        Stroke::new(1_f32, palette::field_text()),
        StrokeKind::Inside,
    );
    let mut dragging = false;
    // Key handles.
    let mut moved: Option<(usize, f64, Option<f64>)> = None;
    for (k, key) in curve.keys.iter().enumerate() {
        let centre = Pos2::new(
            x_of(key.at),
            if color {
                rect.bottom() - 4.
            } else {
                y_of(key.out.get(0))
            },
        );
        let handle = Rect::from_center_size(centre, Vec2::splat(9.));
        let r = ui.interact(handle, ui.id().with((id, k)), Sense::drag());
        let hot = r.hovered() || r.dragged();
        painter.circle(
            centre,
            if hot { 5. } else { 4. },
            palette::list(),
            Stroke::new(1.5_f32, palette::field_text()),
        );
        if r.dragged()
            && let Some(p) = r.interact_pointer_pos()
        {
            let out = (!color).then(|| {
                vlo + f64::from((rect.bottom() - 4. - p.y) / (rect.height() - 8.)) * (vhi - vlo)
            });
            moved = Some((k, at_of(p.x.clamp(rect.left(), rect.right())), out));
            dragging |= !r.drag_started();
        }
    }
    if let Some((k, at, out)) = moved {
        curve.keys[k].at = at;
        if let Some(v) = out {
            curve.keys[k].out = V::from_fn(|_| v);
        }
        curve.sort();
    }
    // Middle-click, or Ctrl+Alt+click on a Mac trackpad, adds a key there.
    let modifiers = ui.input(|i| i.modifiers);
    let add = response.middle_clicked() || (response.clicked() && modifiers.ctrl && modifiers.alt);
    if add
        && curve.keys.len() < operandi_core::gradient::MAX_KEYS
        && let Some(p) = response.interact_pointer_pos()
    {
        let at = at_of(p.x);
        curve.keys.push(Key {
            at,
            out: curve.sample(at),
            shape: KeyShape::Smooth,
        });
        curve.sort();
    }
    dragging
}

/// The numeric key list: position (in the input's units), output, shape,
/// remove; and Add Key.
fn key_list<V: CurveOut>(
    ui: &mut Ui,
    id: &str,
    curve: &mut Curve<V>,
    unit: Unit,
    mut out: impl FnMut(&mut Ui, &mut V) -> bool,
) -> bool {
    let mut dragging = false;
    let mut remove = None;
    let (scale, suffix, speed) = match unit {
        Unit::Degrees => (1., "°", 0.5),
        Unit::Metres => (1., " m", 0.01),
        Unit::Percent => (100., " %", 0.5),
        Unit::Number => (1., "", 0.01),
    };
    let count = curve.keys.len();
    for (k, key) in curve.keys.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let mut at = key.at * scale;
            let r = ui.add(
                crate::numeric_tab::DragValue::new(&mut at)
                    .range(-1e9..=1e9)
                    .speed(speed)
                    .max_decimals(3)
                    .suffix(suffix),
            );
            key.at = at / scale;
            dragging |= r.dragged() && !r.drag_started();
            dragging |= out(ui, &mut key.out);
            ComboBox::from_id_salt((id, k, "shape"))
                .selected_text(key.shape.label())
                .width(72.)
                .show_ui(ui, |ui| {
                    for s in KeyShape::ALL {
                        ui.selectable_value(&mut key.shape, s, s.label());
                    }
                });
            // A curve keeps at least one key.
            if ui
                .gated(
                    crate::unavailable::unless(count > 1, "A gradient keeps at least one key"),
                    egui::Button::new("−"),
                )
                .on_hover_text("Remove this key")
                .clicked()
            {
                remove = Some(k);
            }
        });
    }
    if let Some(k) = remove {
        curve.keys.remove(k);
    }
    if ui
        .gated(
            crate::unavailable::unless(
                curve.keys.len() < operandi_core::gradient::MAX_KEYS,
                "The gradient has the most keys it can hold (256)",
            ),
            egui::Button::new("Add Key"),
        )
        .on_hover_text("A key one step past the last, with its output")
        .clicked()
    {
        let last = curve.keys.last().cloned();
        if let Some(last) = last {
            curve.keys.push(Key {
                at: last.at
                    + if unit == Unit::Percent {
                        0.1
                    } else {
                        10. * speed * 2.
                    },
                ..last
            });
        }
    }
    curve.sort();
    dragging
}
