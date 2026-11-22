//! The timeline strip (BEHAVIOR-TIME-KEYS TK-70/71, BEHAVIOR-TIMELINE
//! TL-01..53): ruler, current-time and key-time lines, the current-range
//! bar, the marker row, the track row (dope sheet summary) and the button
//! row, docked under the views.
//!
//! Everything it changes goes through a command line (UIC-160): scrubs set
//! the time directly while dragging and record one `operandi.time.set` at
//! mouse-up, key and range drags record one command at mouse-up
//! (UIC-161/224).
//!
//! [OP, provisional] The strip is a bottom panel the bottom bar's clock
//! button (and the Animate layout) opens, not yet a dockable panel kind
//! (UIC-190..195): see SPECIFICATION_QUESTIONS TL-Q01.
use super::*;
use crate::form::status::key_red;
use crate::icons::Icon;
use crate::unavailable::Gate;
use operandi_core::animation::{self, AutoKey, ChannelKey, EndBehavior, MarkerShape, TimeDisplay};
use operandi_core::time_tick::Tick;

const LABEL_W: f32 = 120.;
const RULER_H: f32 = 22.;
const RANGE_H: f32 = 9.;
const MARKER_H: f32 = 16.;
const BUTTONS_H: f32 = 26.;
const ROW_H: f32 = 16.;
/// Heights the strip grows through (TL-02).
const SHORT: f32 = RULER_H + RANGE_H + BUTTONS_H + 10.;
const WITH_MARKERS: f32 = SHORT + MARKER_H + 6.;
const WITH_TRACKS: f32 = WITH_MARKERS + 3. * ROW_H;

/// What a timeline drag is doing.
#[derive(Clone, Debug)]
enum Drag {
    Height {
        start_y: f32,
        start_h: f32,
    },
    Scrub,
    KeyTime,
    Pan {
        last_x: f32,
    },
    RangeStart(f64),
    RangeEnd(f64),
    RangeSlide {
        grab: f64,
        r0: f64,
        r1: f64,
        at: f64,
    },
    /// Moving the selected keys by whole frames from `from`.
    Keys {
        from: f64,
        offset: f64,
    },
    Box {
        from: Pos2,
        to: Pos2,
    },
    Scale {
        pivot: f64,
        from_x: f32,
        factor: f64,
    },
    Markers {
        ids: Vec<u64>,
        from: f64,
        offset: f64,
    },
    MarkerLength {
        id: u64,
        start: f64,
        end: f64,
    },
}

/// A small dialog (UIC-80..97): its title, the line it runs and its one
/// value.
#[derive(Clone, Debug)]
struct ValueDialog {
    title: &'static str,
    label: &'static str,
    text: String,
    /// `{}` is replaced by the typed value.
    line: String,
}

/// The timeline's state (per window; the scene's own time is per document).
#[derive(Clone, Debug)]
pub(crate) struct Timeline {
    pub(crate) open: bool,
    pub(crate) height: f32,
    /// TL-03: the Extended button row.
    pub(crate) extended: bool,
    /// TL-15: a zoomed or panned view, in frames; `None` shows the range.
    view: Option<(f64, f64)>,
    /// TL-40: items expanded into one row per channel.
    expanded: std::collections::BTreeSet<u64>,
    /// TL-40: rows grouped by channel type instead of by item.
    by_type: bool,
    /// TL-16: Range shown is the scene range instead of the current range.
    show_scene_range: bool,
    /// TL-16 Auto Pan while playing a zoomed view.
    auto_pan: bool,
    drag: Option<Drag>,
    frame_text: Option<String>,
    dialog: Option<ValueDialog>,
    /// Where the strip drew last frame (keys under the pointer, tests).
    pub(crate) rect: Rect,
}

impl Default for Timeline {
    fn default() -> Self {
        Timeline {
            open: false,
            height: SHORT,
            extended: false,
            view: None,
            expanded: Default::default(),
            by_type: false,
            show_scene_range: false,
            auto_pan: true,
            drag: None,
            frame_text: None,
            dialog: None,
            rect: Rect::NOTHING,
        }
    }
}

impl Timeline {
    /// TL-01: the Animate layout's tall strip with markers and tracks.
    pub(crate) fn open_tall(&mut self) {
        self.open = true;
        self.height = self.height.max(WITH_TRACKS + 4. * ROW_H);
    }
}

/// A track-row handle: its row, its frame and the keys under it.
type Handle = (usize, f64, Vec<(ChannelKey, Tick)>);

/// One track row (TL-40).
struct TrackRow {
    label: String,
    channels: Vec<ChannelKey>,
    indent: f32,
    /// An item row's expand triangle.
    item: Option<u64>,
}

/// Frames ↔ x in one strip.
#[derive(Clone, Copy)]
struct Scale {
    x0: f32,
    x1: f32,
    f0: f64,
    f1: f64,
}

impl Scale {
    fn x(&self, f: f64) -> f32 {
        self.x0 + ((f - self.f0) / (self.f1 - self.f0).max(1e-9)) as f32 * (self.x1 - self.x0)
    }
    fn f(&self, x: f32) -> f64 {
        self.f0 + ((x - self.x0) / (self.x1 - self.x0).max(1.)) as f64 * (self.f1 - self.f0)
    }
    fn per_frame(&self) -> f32 {
        (self.x1 - self.x0) / (self.f1 - self.f0).max(1e-9) as f32
    }
}

fn fmt(f: f64) -> String {
    if (f - f.round()).abs() < 1e-6 {
        format!("{}", f.round() as i64)
    } else {
        format!("{f:.3}")
    }
}

impl Operandi {
    /// The bottom bar's clock button (TK-70).
    pub(super) fn timeline_toggle_button(&mut self, ui: &mut Ui) {
        if !crate::anim_switch::enabled() {
            return;
        }
        let r = ui
            .add(crate::icons::button(ui, Icon::TimelineClock, "").selected(self.timeline.open))
            .on_hover_text("Timeline: show or hide the time strip (operandi.anim.timeline)");
        if r.clicked() {
            self.execute("operandi.anim.timeline");
        }
    }

    /// The strip, as a bottom panel above the bottom bar.
    pub(super) fn timeline_panel(&mut self, ctx: &Context) {
        if !self.timeline.open || !crate::anim_switch::enabled() {
            self.timeline.rect = Rect::NOTHING;
            return;
        }
        let height = self.timeline.height.clamp(SHORT, 600.);
        egui::TopBottomPanel::bottom("timeline")
            .exact_height(height)
            .resizable(false)
            .show(ctx, |ui| self.timeline_ui(ui));
    }

    fn timeline_scale(&self, x0: f32, x1: f32) -> Scale {
        let s = &self.editor.scene;
        let rate = s.time.rate;
        let (a, b) = if self.timeline.show_scene_range {
            (s.time.start, s.time.end)
        } else {
            animation::current_range(s)
        };
        let (f0, f1) = self.timeline.view.unwrap_or_else(|| {
            let (fa, fb) = (animation::frames(rate, a), animation::frames(rate, b));
            let pad = ((fb - fa) * 0.02).max(0.5);
            (fa - pad, fb + pad)
        });
        Scale {
            x0,
            x1,
            f0,
            f1: f1.max(f0 + 1.),
        }
    }

    fn timeline_ui(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        self.timeline.rect = full;
        let rate = self.editor.scene.time.rate;
        // TL-02: drag the top edge for the height.
        let edge = Rect::from_min_max(full.min, pos2(full.max.x, full.min.y + 4.));
        let edge_r = ui.interact(edge, Id::new("timeline-edge"), Sense::drag());
        if edge_r.hovered() || edge_r.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        if edge_r.drag_started() {
            let y = edge_r.interact_pointer_pos().map_or(0., |p| p.y);
            self.timeline.drag = Some(Drag::Height {
                start_y: y,
                start_h: self.timeline.height,
            });
        }
        if let (Some(Drag::Height { start_y, start_h }), Some(p)) =
            (&self.timeline.drag, edge_r.interact_pointer_pos())
            && edge_r.dragged()
        {
            self.timeline.height = (start_h + (start_y - p.y)).clamp(SHORT, 600.);
        }
        if edge_r.drag_stopped() {
            self.timeline.drag = None;
        }
        let h = full.height();
        let markers = h >= WITH_MARKERS - 1.;
        let tracks = h >= WITH_TRACKS - 1.;
        let mut y = full.min.y + 6.;
        let marker_rect = markers.then(|| {
            let r = Rect::from_min_max(pos2(full.min.x, y), pos2(full.max.x, y + MARKER_H));
            y += MARKER_H;
            r
        });
        let ruler = Rect::from_min_max(pos2(full.min.x, y), pos2(full.max.x, y + RULER_H));
        y += RULER_H;
        let range = Rect::from_min_max(pos2(full.min.x, y), pos2(full.max.x, y + RANGE_H));
        y += RANGE_H + 1.;
        let buttons = Rect::from_min_max(pos2(full.min.x, full.max.y - BUTTONS_H), full.max);
        let track_rect = tracks
            .then(|| Rect::from_min_max(pos2(full.min.x, y), pos2(full.max.x, buttons.min.y - 2.)));
        let scale = self.timeline_scale(full.min.x + LABEL_W, full.max.x - 10.);
        let painter = ui.painter_at(full);
        painter.rect_filled(full, CornerRadius::ZERO, theme::panel());
        if let Some(r) = marker_rect {
            self.marker_row(ui, r, scale);
        }
        self.ruler(ui, ruler, scale);
        self.range_bar(ui, range, scale);
        if let Some(r) = track_rect {
            self.track_rows(ui, r, scale);
        }
        // The current-time and key-time lines over everything above the
        // buttons.
        let top = marker_rect.map_or(ruler.min.y, |r| r.min.y);
        let bottom = track_rect.map_or(range.max.y, |r| r.max.y);
        let now = animation::frames(rate, self.editor.scene.time.current);
        let x = scale.x(now);
        if (scale.x0..=scale.x1).contains(&x) {
            painter.line_segment(
                [pos2(x, top), pos2(x, bottom)],
                Stroke::new(1.5f32, theme::orange()),
            );
            let text = animation::time_text(
                self.editor.scene.animation.settings.display,
                rate,
                self.editor.scene.time.current,
            );
            let galley =
                painter.layout_no_wrap(text, egui::FontId::proportional(10.), theme::on_orange());
            let tag = Rect::from_min_size(
                pos2(x - galley.size().x / 2. - 3., ruler.min.y),
                galley.size() + vec2(6., 2.),
            );
            painter.rect_filled(tag, CornerRadius::same(2), theme::orange());
            painter.galley(tag.min + vec2(3., 1.), galley, theme::on_orange());
        }
        if let Some(kt) = self.editor.anim.key_time {
            let x = scale.x(animation::frames(rate, kt));
            if (scale.x0..=scale.x1).contains(&x) {
                painter.line_segment(
                    [pos2(x, top), pos2(x, bottom)],
                    Stroke::new(1.5f32, theme::cyan()),
                );
            }
        }
        self.timeline_buttons(ui, buttons);
        self.timeline_keys(ui, full);
        // TL-16 Auto Pan: keep the line in a zoomed view while playing.
        if self.timeline.auto_pan
            && self.editor.anim.playing.is_some()
            && let Some((f0, f1)) = self.timeline.view
            && !(f0..=f1).contains(&now)
        {
            let span = f1 - f0;
            self.timeline.view = Some((now - span / 2., now + span / 2.));
        }
    }

    /// TL-14 A / Shift+A, TL-50 M, Delete for the selected keys, while the
    /// pointer is over the strip and no text field has the keys.
    fn timeline_keys(&mut self, ui: &mut Ui, rect: Rect) {
        let hovered = ui.rect_contains_pointer(rect);
        if !hovered || self.text_editor_focused(ui.ctx()) {
            return;
        }
        // The keys come from the one routing seam (anim_keys.rs).
        let lines = ui.input_mut(crate::anim_keys::take_timeline_keys);
        for line in lines {
            if line == "operandi.time.fit" || line == "operandi.time.fit all:true" {
                self.timeline.view = None;
            }
            self.execute(line);
        }
    }

    // ------------------------------------------------------------ ruler

    fn ruler(&mut self, ui: &mut Ui, rect: Rect, scale: Scale) {
        let s = &self.editor.scene;
        let rate = s.time.rate;
        let display = s.animation.settings.display;
        let painter = ui.painter_at(rect);
        let band = Rect::from_min_max(pos2(scale.x0, rect.min.y), pos2(scale.x1, rect.max.y));
        painter.rect_filled(band, CornerRadius::ZERO, theme::field());
        // Ticks: about one label per 50 px.
        let ppf = scale.per_frame().max(1e-3);
        let step = [
            1., 2., 5., 10., 20., 50., 100., 200., 500., 1000., 2000., 5000.,
        ]
        .into_iter()
        .find(|s| s * ppf as f64 >= 50.)
        .unwrap_or(10000.);
        let minor = if step >= 5. { step / 5. } else { 1. };
        let mut f = (scale.f0 / minor).floor() * minor;
        while f <= scale.f1 {
            let x = scale.x(f);
            if x >= scale.x0 {
                let major = (f / step).round() * step == f;
                let h = if major { 8. } else { 4. };
                painter.line_segment(
                    [pos2(x, rect.max.y - h), pos2(x, rect.max.y)],
                    Stroke::new(1.0f32, theme::weak_text()),
                );
                if major {
                    let t = animation::at_frames(rate, f);
                    painter.text(
                        pos2(x + 2., rect.min.y + 1.),
                        egui::Align2::LEFT_TOP,
                        animation::time_text(display, rate, t),
                        egui::FontId::proportional(10.),
                        theme::weak_text(),
                    );
                }
            }
            f += minor;
        }
        // TL-12: keys of the selected items (or channels), with a band
        // over the animated span.
        let times = self.editor.scope_key_times();
        if let (Some(first), Some(last)) = (times.first(), times.last()) {
            let (xa, xb) = (
                scale.x(animation::frames(rate, *first)).max(scale.x0),
                scale.x(animation::frames(rate, *last)).min(scale.x1),
            );
            if xb > xa {
                painter.rect_filled(
                    Rect::from_min_max(pos2(xa, rect.max.y - 3.), pos2(xb, rect.max.y - 1.)),
                    CornerRadius::ZERO,
                    key_red().gamma_multiply(0.4),
                );
            }
        }
        for t in &times {
            let x = scale.x(animation::frames(rate, *t));
            if (scale.x0..=scale.x1).contains(&x) {
                diamond(
                    &painter,
                    pos2(x, rect.max.y - 6.),
                    3.5,
                    key_red(),
                    Stroke::NONE,
                );
            }
        }
        // TL-11 click/drag: time; TL-20 middle: key time; Shift+middle
        // pan [OP]; wheel zoom around the pointer.
        let response = ui.interact(band, Id::new("timeline-ruler"), Sense::click_and_drag());
        let pointer = response.interact_pointer_pos().or(response.hover_pos());
        let frame_at = |x: f32| scale.f(x);
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.
                && let Some(p) = pointer
            {
                let k = (1.0 - scroll as f64 * 0.002).clamp(0.5, 2.0);
                let c = frame_at(p.x);
                self.timeline.view = Some((c - (c - scale.f0) * k, c + (scale.f1 - c) * k));
            }
        }
        let shift = ui.input(|i| i.modifiers.shift);
        if response.drag_started() {
            self.timeline.drag = if response.dragged_by(PointerButton::Middle) && shift {
                Some(Drag::Pan {
                    last_x: pointer.map_or(0., |p| p.x),
                })
            } else if response.dragged_by(PointerButton::Middle) {
                Some(Drag::KeyTime)
            } else if response.dragged_by(PointerButton::Primary) {
                Some(Drag::Scrub)
            } else {
                None
            };
        }
        if let Some(p) = pointer
            && (response.dragged() || response.clicked() || response.middle_clicked())
        {
            let t = animation::at_frames(rate, frame_at(p.x));
            match self.timeline.drag.clone() {
                Some(Drag::Scrub) => {
                    let _ = self.editor.set_current_time(t, false);
                }
                Some(Drag::KeyTime) => self.editor.set_key_time(Some(t)),
                Some(Drag::Pan { last_x }) => {
                    let df = (last_x - p.x) as f64 / scale.per_frame().max(1e-3) as f64;
                    self.timeline.view = Some((scale.f0 + df, scale.f1 + df));
                    self.timeline.drag = Some(Drag::Pan { last_x: p.x });
                }
                _ if response.clicked() => {
                    let f = frame_at(p.x);
                    self.execute(&format!("operandi.time.set frame:{}", fmt(f.round())));
                }
                _ if response.middle_clicked() => {
                    let f = frame_at(p.x).round();
                    self.execute(&format!("operandi.time.keyTime frame:{}", fmt(f)));
                }
                _ => {}
            }
        }
        if response.drag_stopped() {
            match self.timeline.drag.take() {
                Some(Drag::Scrub) => {
                    let f = animation::frames(rate, self.editor.scene.time.current);
                    self.execute(&format!("operandi.time.set frame:{}", fmt(f)));
                }
                Some(Drag::KeyTime) => {
                    if let Some(kt) = self.editor.anim.key_time {
                        let f = animation::frames(rate, kt);
                        self.execute(&format!("operandi.time.keyTime frame:{}", fmt(f)));
                    }
                }
                _ => {}
            }
        }
        // UIC-60/07: the ruler's right-click menu.
        let at = pointer.map(|p| frame_at(p.x).round());
        let mut picked: Option<String> = None;
        let mut whole = false;
        let (mut scene_range, mut auto_pan) =
            (self.timeline.show_scene_range, self.timeline.auto_pan);
        response.context_menu(|ui| {
            let f = at.unwrap_or(0.);
            let entries = [
                (
                    "Add Frame Marker",
                    format!("operandi.marker.add frame:{}", fmt(f)),
                ),
                (
                    "Set Key Time Here",
                    format!("operandi.time.keyTime frame:{}", fmt(f)),
                ),
                ("Reset Key Time", "operandi.time.keyTime reset:true".into()),
                ("Insert Frame", "operandi.time.insertFrame".into()),
                ("Remove Frame", "operandi.time.removeFrame".into()),
                ("Fit to Keys (A)", "operandi.time.fit".into()),
                (
                    "Fit to All Keys (Shift+A)",
                    "operandi.time.fit all:true".into(),
                ),
            ];
            for (i, (text, line)) in entries.into_iter().enumerate() {
                if matches!(i, 3 | 5) {
                    ui.separator();
                }
                if ui.button(text).clicked() {
                    picked = Some(line);
                    ui.close();
                }
            }
            if ui.button("Show the Whole Range").clicked() {
                whole = true;
                ui.close();
            }
            ui.separator();
            ui.checkbox(&mut scene_range, "Show Scene Range");
            ui.checkbox(&mut auto_pan, "Auto Pan");
        });
        self.timeline.show_scene_range = scene_range;
        self.timeline.auto_pan = auto_pan;
        if whole {
            self.timeline.view = None;
        }
        if let Some(line) = picked {
            self.execute(&line);
        }
    }

    /// TL-13: the current range inside the scene range.
    fn range_bar(&mut self, ui: &mut Ui, rect: Rect, scale: Scale) {
        let s = &self.editor.scene;
        let rate = s.time.rate;
        let (r0, r1) = animation::current_range(s);
        let (mut f0, mut f1) = (animation::frames(rate, r0), animation::frames(rate, r1));
        match &self.timeline.drag {
            Some(Drag::RangeStart(f)) => f0 = f.min(f1),
            Some(Drag::RangeEnd(f)) => f1 = f.max(f0),
            Some(Drag::RangeSlide { r0, r1, at, grab }) => {
                f0 = r0 + (at - grab);
                f1 = r1 + (at - grab);
            }
            _ => {}
        }
        let painter = ui.painter_at(rect);
        let (sx0, sx1) = (
            scale.x(animation::frames(rate, s.time.start)).max(scale.x0),
            scale.x(animation::frames(rate, s.time.end)).min(scale.x1),
        );
        painter.rect_filled(
            Rect::from_min_max(pos2(sx0, rect.min.y + 2.), pos2(sx1, rect.max.y - 2.)),
            CornerRadius::same(2),
            theme::control(),
        );
        let (xa, xb) = (scale.x(f0).max(scale.x0), scale.x(f1).min(scale.x1));
        let bar = Rect::from_min_max(
            pos2(xa, rect.min.y + 1.),
            pos2(xb.max(xa + 2.), rect.max.y - 1.),
        );
        painter.rect_filled(
            bar,
            CornerRadius::same(2),
            theme::weak_text().gamma_multiply(0.7),
        );
        for x in [xa, xb] {
            painter.rect_filled(
                Rect::from_center_size(pos2(x, rect.center().y), vec2(5., RANGE_H)),
                CornerRadius::same(1),
                theme::text(),
            );
        }
        let response = ui.interact(rect, Id::new("timeline-range"), Sense::click_and_drag());
        if response.double_clicked() {
            self.execute("operandi.time.range reset:true");
            return;
        }
        let p = response.interact_pointer_pos();
        if response.drag_started()
            && let Some(p) = p
        {
            let f = scale.f(p.x);
            self.timeline.drag = Some(if (p.x - xa).abs() < 6. {
                Drag::RangeStart(f)
            } else if (p.x - xb).abs() < 6. {
                Drag::RangeEnd(f)
            } else {
                Drag::RangeSlide {
                    grab: f,
                    r0: f0,
                    r1: f1,
                    at: f,
                }
            });
        }
        if response.dragged()
            && let Some(p) = p
        {
            let f = scale.f(p.x).round();
            match &mut self.timeline.drag {
                Some(Drag::RangeStart(v)) | Some(Drag::RangeEnd(v)) => *v = f,
                Some(Drag::RangeSlide { at, grab, .. }) => *at = f - (*grab - grab.round()),
                _ => {}
            }
        }
        if response.drag_stopped()
            && matches!(
                self.timeline.drag,
                Some(Drag::RangeStart(_) | Drag::RangeEnd(_) | Drag::RangeSlide { .. })
            )
        {
            self.timeline.drag = None;
            self.execute(&format!(
                "operandi.time.range start:{} end:{}",
                fmt(f0.round()),
                fmt(f1.round())
            ));
        }
        response.on_hover_text(
            "Current range: drag an end or the middle; double-click for the whole scene range",
        );
    }

    // ------------------------------------------------------------ markers

    fn marker_row(&mut self, ui: &mut Ui, rect: Rect, scale: Scale) {
        let rate = self.editor.scene.time.rate;
        let painter = ui.painter_at(rect);
        // TL-51: where the next marker goes.
        let owner = if self.editor.scene.selected_items().is_empty() {
            "Scene markers"
        } else {
            "Item markers"
        };
        painter.text(
            pos2(rect.min.x + 6., rect.center().y),
            egui::Align2::LEFT_CENTER,
            owner,
            egui::FontId::proportional(10.),
            theme::weak_text(),
        );
        let offset_of = |id: u64, drag: &Option<Drag>| match drag {
            Some(Drag::Markers { ids, offset, .. }) if ids.contains(&id) => *offset,
            _ => 0.,
        };
        let mut hit: Option<(u64, Rect)> = None;
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let shown: Vec<animation::Marker> =
            self.editor.visible_markers().into_iter().cloned().collect();
        for m in &shown {
            let off = offset_of(m.id, &self.timeline.drag);
            let mut f0 = animation::frames(rate, m.time) + off;
            let mut f1 = animation::frames(rate, m.end()) + off;
            if let Some(Drag::MarkerLength { id, start, end }) = &self.timeline.drag
                && *id == m.id
            {
                f0 = *start;
                f1 = *end;
            }
            let (xa, xb) = (scale.x(f0), scale.x(f1));
            if xb < scale.x0 || xa > scale.x1 {
                continue;
            }
            let selected = self.editor.anim.selected_markers.contains(&m.id);
            let colour = egui::Color32::from_rgb(
                (m.color[0].clamp(0., 1.) * 255.) as u8,
                (m.color[1].clamp(0., 1.) * 255.) as u8,
                (m.color[2].clamp(0., 1.) * 255.) as u8,
            );
            let stroke = if selected {
                Stroke::new(1.5f32, theme::orange())
            } else {
                Stroke::NONE
            };
            let c = pos2(xa, rect.center().y);
            if f1 > f0 {
                painter.rect_filled(
                    Rect::from_min_max(pos2(xa, rect.min.y + 4.), pos2(xb, rect.max.y - 4.)),
                    CornerRadius::same(2),
                    colour.gamma_multiply(if m.frame_limits { 0.8 } else { 0.45 }),
                );
            }
            match m.shape {
                MarkerShape::Triangle => {
                    painter.add(egui::Shape::convex_polygon(
                        vec![c + vec2(-5., -6.), c + vec2(5., -6.), c + vec2(0., 6.)],
                        colour,
                        stroke,
                    ));
                }
                MarkerShape::Square => {
                    painter.rect(
                        Rect::from_center_size(c, vec2(9., 9.)),
                        CornerRadius::ZERO,
                        colour,
                        stroke,
                        egui::StrokeKind::Middle,
                    );
                }
                MarkerShape::Diamond => diamond(&painter, c, 5.5, colour, stroke),
                MarkerShape::Bar => {
                    painter.rect(
                        Rect::from_center_size(c, vec2(3., MARKER_H)),
                        CornerRadius::ZERO,
                        colour,
                        stroke,
                        egui::StrokeKind::Middle,
                    );
                }
            }
            let r =
                Rect::from_min_max(pos2(xa - 6., rect.min.y), pos2(xb.max(xa) + 6., rect.max.y));
            if m.show_label && !m.label.is_empty() {
                painter.text(
                    c + vec2(7., 0.),
                    egui::Align2::LEFT_CENTER,
                    &m.label,
                    egui::FontId::proportional(10.),
                    theme::text(),
                );
            }
            if pointer.is_some_and(|p| r.contains(p)) {
                hit = Some((m.id, r));
            }
        }
        let band = Rect::from_min_max(pos2(scale.x0, rect.min.y), pos2(scale.x1, rect.max.y));
        let response = ui.interact(band, Id::new("timeline-markers"), Sense::click_and_drag());
        if let Some((id, _)) = hit
            && let Some(m) = shown.iter().find(|m| m.id == id)
            && !m.label.is_empty()
        {
            response.clone().on_hover_text(m.label.clone());
        }
        let p = response.interact_pointer_pos();
        let shift = ui.input(|i| i.modifiers.shift);
        if response.clicked() {
            match hit {
                Some((id, _)) => {
                    self.execute(&format!("operandi.marker.select ids:{id} add:{shift}"))
                }
                None => self.execute("operandi.marker.select ids:"),
            }
        }
        if response.middle_clicked()
            && let Some(p) = p
        {
            self.execute(&format!(
                "operandi.marker.add frame:{}",
                fmt(scale.f(p.x).round())
            ));
        }
        if response.drag_started()
            && let (Some(p), Some((id, _))) = (p, hit)
        {
            let f = scale.f(p.x);
            if response.dragged_by(PointerButton::Secondary) {
                // TL-53: right-drag gives a marker a length.
                if let Some(m) = shown.iter().find(|m| m.id == id) {
                    let start = animation::frames(rate, m.time);
                    self.timeline.drag = Some(Drag::MarkerLength {
                        id,
                        start,
                        end: start,
                    });
                }
            } else {
                if !self.editor.anim.selected_markers.contains(&id) {
                    self.execute(&format!("operandi.marker.select ids:{id} add:{shift}"));
                }
                self.timeline.drag = Some(Drag::Markers {
                    ids: self.editor.anim.selected_markers.iter().copied().collect(),
                    from: f,
                    offset: 0.,
                });
            }
        }
        if response.dragged()
            && let Some(p) = p
        {
            let f = scale.f(p.x);
            match &mut self.timeline.drag {
                Some(Drag::Markers { from, offset, .. }) => *offset = (f - *from).round(),
                Some(Drag::MarkerLength { start, end, .. }) => *end = f.round().max(*start),
                _ => {}
            }
        }
        if response.drag_stopped() {
            match self.timeline.drag.take() {
                Some(Drag::Markers { ids, offset, .. }) if offset != 0. => {
                    let ids: Vec<String> = ids.iter().map(u64::to_string).collect();
                    self.execute(&format!(
                        "operandi.marker.edit ids:{} shift:{}",
                        ids.join(","),
                        fmt(offset)
                    ));
                }
                Some(Drag::MarkerLength { id, start, end }) => {
                    self.execute(&format!(
                        "operandi.marker.edit ids:{id} length:{}",
                        fmt(end - start)
                    ));
                }
                _ => {}
            }
        }
        let at = p.or(pointer).map(|p| scale.f(p.x).round()).unwrap_or(0.);
        let target = hit
            .map(|(id, _)| id)
            .or_else(|| self.editor.anim.selected_markers.iter().next().copied())
            .and_then(|id| shown.iter().find(|m| m.id == id).cloned());
        let mut picked: Option<String> = None;
        let mut asked: Option<ValueDialog> = None;
        response.context_menu(|ui| {
            let Some(m) = &target else {
                if ui.button("Add Frame Marker").clicked() {
                    picked = Some(format!("operandi.marker.add frame:{}", fmt(at)));
                    ui.close();
                }
                return;
            };
            let id = m.id;
            let start = animation::frames(rate, m.time);
            let dialogs: [(&str, &'static str, &'static str, String, String); 4] = [
                (
                    "Set Time…",
                    "Set Marker Time",
                    "Frame",
                    fmt(start),
                    format!("operandi.marker.edit ids:{id} frame:{{}}"),
                ),
                (
                    "Shift…",
                    "Shift Marker",
                    "Frames",
                    "0".into(),
                    format!("operandi.marker.edit ids:{id} shift:{{}}"),
                ),
                (
                    "Set Range…",
                    "Set Marker Range",
                    "Length (frames)",
                    fmt(animation::frames(rate, m.length)),
                    format!("operandi.marker.edit ids:{id} length:{{}}"),
                ),
                (
                    "Label…",
                    "Marker Label",
                    "Label",
                    m.label.clone(),
                    format!("operandi.marker.edit ids:{id} label:{{}}"),
                ),
            ];
            for (text, title, label, value, line) in dialogs {
                if ui.button(text).clicked() {
                    asked = Some(ValueDialog {
                        title,
                        label,
                        text: value,
                        line,
                    });
                    ui.close();
                }
            }
            if ui.button("Snap to Frames").clicked() {
                picked = Some(format!("operandi.marker.edit ids:{id} snapToFrames:true"));
                ui.close();
            }
            ui.menu_button("Shape", |ui| {
                for s in MarkerShape::NAMES {
                    if ui.radio(m.shape.name() == s, s).clicked() {
                        picked = Some(format!("operandi.marker.edit ids:{id} shape:{s}"));
                        ui.close();
                    }
                }
            });
            ui.menu_button("Colour", |ui| {
                for (name, c) in [
                    ("Yellow", "0.95,0.75,0.2"),
                    ("Red", "0.9,0.25,0.2"),
                    ("Green", "0.3,0.8,0.35"),
                    ("Blue", "0.3,0.55,0.95"),
                    ("Purple", "0.7,0.4,0.9"),
                    ("Grey", "0.6,0.6,0.6"),
                ] {
                    if ui.button(name).clicked() {
                        picked = Some(format!("operandi.marker.edit ids:{id} color:{c}"));
                        ui.close();
                    }
                }
            });
            let mut show = m.show_label;
            if ui.checkbox(&mut show, "Show Label").clicked() {
                picked = Some(format!("operandi.marker.edit ids:{id} showLabel:{show}"));
                ui.close();
            }
            let mut limits = m.frame_limits;
            if ui
                .gated(
                    crate::unavailable::unless(
                        m.is_range(),
                        "Give the marker a range first (right-drag it, or Set Range…)",
                    ),
                    egui::Checkbox::new(&mut limits, "Frame Limits"),
                )
                .clicked()
            {
                picked = Some(format!(
                    "operandi.marker.edit ids:{id} frameLimits:{limits}"
                ));
                ui.close();
            }
            ui.separator();
            if ui.button("Delete").clicked() {
                picked = Some(format!("operandi.marker.delete ids:{id}"));
                ui.close();
            }
        });
        if let Some(line) = picked {
            self.execute(&line);
        }
        if asked.is_some() {
            self.timeline.dialog = asked;
        }
    }

    // ------------------------------------------------------------ tracks

    fn track_list(&self) -> Vec<TrackRow> {
        let e = &self.editor;
        let scope = e.key_scope();
        let mut rows = vec![TrackRow {
            label: "Summary".into(),
            channels: scope.clone(),
            indent: 0.,
            item: None,
        }];
        let items: Vec<u64> = {
            let mut v: Vec<u64> = scope.iter().map(|k| k.item).collect();
            v.dedup();
            v
        };
        if self.timeline.by_type {
            for (group, label) in [("pos", "Position"), ("rot", "Rotation"), ("scl", "Scale")] {
                let ch: Vec<ChannelKey> = scope
                    .iter()
                    .filter(|k| k.channel.starts_with(group))
                    .cloned()
                    .collect();
                if !ch.is_empty() {
                    rows.push(TrackRow {
                        label: label.into(),
                        channels: ch,
                        indent: 8.,
                        item: None,
                    });
                }
            }
            let other: Vec<ChannelKey> = scope
                .iter()
                .filter(|k| k.transform_part().is_none())
                .cloned()
                .collect();
            if !other.is_empty() {
                rows.push(TrackRow {
                    label: "Other".into(),
                    channels: other,
                    indent: 8.,
                    item: None,
                });
            }
            return rows;
        }
        for id in items {
            let name = e
                .scene
                .item(id)
                .map_or_else(|| id.to_string(), |i| i.name.clone());
            let ch: Vec<ChannelKey> = scope.iter().filter(|k| k.item == id).cloned().collect();
            rows.push(TrackRow {
                label: name,
                channels: ch.clone(),
                indent: 8.,
                item: Some(id),
            });
            if self.timeline.expanded.contains(&id) {
                for k in ch {
                    rows.push(TrackRow {
                        label: k.channel.clone(),
                        channels: vec![k],
                        indent: 20.,
                        item: None,
                    });
                }
            }
        }
        rows
    }

    fn track_rows(&mut self, ui: &mut Ui, rect: Rect, scale: Scale) {
        let rate = self.editor.scene.time.rate;
        let rows = self.track_list();
        let painter = ui.painter_at(rect);
        painter.rect_filled(
            Rect::from_min_max(pos2(scale.x0, rect.min.y), pos2(scale.x1, rect.max.y)),
            CornerRadius::ZERO,
            theme::field().gamma_multiply(0.8),
        );
        let selected = self.editor.live_selected_keys();
        let move_by = match &self.timeline.drag {
            Some(Drag::Keys { offset, .. }) => *offset,
            _ => 0.,
        };
        let scale_by = match &self.timeline.drag {
            Some(Drag::Scale { pivot, factor, .. }) => Some((*pivot, *factor)),
            _ => None,
        };
        // Handles: (row, frame, keys under it).
        let mut handles: Vec<Handle> = Vec::new();
        for (i, row) in rows.iter().enumerate() {
            let y = rect.min.y + i as f32 * ROW_H;
            if y + ROW_H > rect.max.y {
                break;
            }
            let label_rect =
                Rect::from_min_max(pos2(rect.min.x, y), pos2(scale.x0 - 4., y + ROW_H));
            let mut text_x = rect.min.x + 6. + row.indent;
            if let Some(id) = row.item {
                let open = self.timeline.expanded.contains(&id);
                let tri = ui.interact(
                    Rect::from_min_size(pos2(text_x - 2., y), vec2(12., ROW_H)),
                    Id::new(("timeline-expand", id)),
                    Sense::click(),
                );
                painter.text(
                    pos2(text_x, y + ROW_H / 2.),
                    egui::Align2::LEFT_CENTER,
                    if open { "⏷" } else { "⏵" },
                    egui::FontId::proportional(10.),
                    theme::weak_text(),
                );
                if tri.clicked() {
                    if open {
                        self.timeline.expanded.remove(&id);
                    } else {
                        self.timeline.expanded.insert(id);
                    }
                }
                text_x += 12.;
            }
            painter.text(
                pos2(text_x, y + ROW_H / 2.),
                egui::Align2::LEFT_CENTER,
                &row.label,
                egui::FontId::proportional(10.5),
                if i == 0 {
                    theme::text()
                } else {
                    theme::weak_text()
                },
            );
            if i == 0 {
                let toggle = ui
                    .interact(label_rect, Id::new("timeline-by-type"), Sense::click())
                    .on_hover_text(
                        "Click to group the rows by channel type instead of by item (TL-40)",
                    );
                if toggle.clicked() {
                    self.timeline.by_type = !self.timeline.by_type;
                }
            }
            painter.line_segment(
                [pos2(scale.x0, y + ROW_H), pos2(scale.x1, y + ROW_H)],
                Stroke::new(0.5f32, theme::border()),
            );
            let mut by_frame: std::collections::BTreeMap<i64, Vec<(ChannelKey, Tick)>> =
                Default::default();
            for k in &row.channels {
                if let Some(c) = self.editor.scene.animation.channels.get(k) {
                    for t in c.curve.times() {
                        let f = (animation::frames(rate, t) * 1000.).round() as i64;
                        by_frame.entry(f).or_default().push((k.clone(), t));
                    }
                }
            }
            for (f, keys) in by_frame {
                let f = f as f64 / 1000.;
                let any_selected = keys.iter().any(|k| selected.contains(k));
                let shown_f = if any_selected {
                    match scale_by {
                        Some((pivot, factor)) => (pivot + (f - pivot) * factor).round(),
                        None => f + move_by,
                    }
                } else {
                    f
                };
                let x = scale.x(shown_f);
                if (scale.x0 - 4.0..=scale.x1 + 4.).contains(&x) {
                    let colour = if any_selected {
                        theme::orange()
                    } else {
                        key_red()
                    };
                    diamond(
                        &painter,
                        pos2(x, y + ROW_H / 2.),
                        if i == 0 { 5. } else { 4. },
                        colour,
                        Stroke::new(0.5f32, theme::border()),
                    );
                }
                handles.push((i, f, keys));
            }
        }
        // Box select rectangle.
        if let Some(Drag::Box { from, to }) = &self.timeline.drag {
            let r = Rect::from_two_pos(*from, *to);
            painter.rect(
                r,
                CornerRadius::ZERO,
                theme::orange().gamma_multiply(0.1),
                Stroke::new(1.0f32, theme::orange()),
                egui::StrokeKind::Middle,
            );
        }
        // The drag's offset or factor beside the pointer (TL-42/43).
        if let Some(p) = ui.input(|i| i.pointer.hover_pos()) {
            let label = match &self.timeline.drag {
                Some(Drag::Keys { offset, .. }) => Some(format!("{:+}", *offset as i64)),
                Some(Drag::Scale { factor, .. }) => Some(format!("×{factor:.2}")),
                _ => None,
            };
            if let Some(l) = label {
                painter.text(
                    p + vec2(12., -12.),
                    egui::Align2::LEFT_BOTTOM,
                    l,
                    egui::FontId::proportional(11.),
                    theme::text(),
                );
            }
        }
        let band = Rect::from_min_max(
            pos2(scale.x0 - 6., rect.min.y),
            pos2(scale.x1 + 6., rect.max.y),
        );
        let response = ui.interact(band, Id::new("timeline-tracks"), Sense::click_and_drag());
        let pointer = response.interact_pointer_pos().or(response.hover_pos());
        let under = |p: Pos2| -> Option<usize> {
            handles.iter().position(|(row, f, _)| {
                let y = rect.min.y + *row as f32 * ROW_H + ROW_H / 2.;
                (p.y - y).abs() <= ROW_H / 2. && (scale.x(*f) - p.x).abs() <= 6.
            })
        };
        let (shift, ctrl) =
            ui.input(|i| (i.modifiers.shift, i.modifiers.command || i.modifiers.ctrl));
        let mode = if shift {
            "add"
        } else if ctrl {
            "remove"
        } else {
            "replace"
        };
        let key_list = |keys: &[(ChannelKey, Tick)]| -> String {
            keys.iter()
                .map(|(k, t)| format!("{}@{}", k.text(), fmt(animation::frames(rate, *t))))
                .collect::<Vec<_>>()
                .join(",")
        };
        if response.clicked()
            && let Some(p) = pointer
        {
            match under(p) {
                Some(h) => {
                    let line = format!(
                        "operandi.key.select keys:{} mode:{mode}",
                        key_list(&handles[h].2)
                    );
                    self.execute(&line);
                }
                None if !shift && !ctrl => self.execute("operandi.key.select mode:clear"),
                None => {}
            }
        }
        // TL-44: middle-click keys the selected channels or items there.
        if response.middle_clicked()
            && let Some(p) = pointer
        {
            let f = scale.f(p.x).round();
            self.execute(&format!("operandi.time.keyTime frame:{}", fmt(f)));
            let line = if self.editor.anim.selected_channels.is_empty() {
                "operandi.key.items"
            } else {
                "operandi.key.set"
            };
            self.execute(line);
            self.execute("operandi.time.keyTime reset:true");
        }
        if response.drag_started()
            && let Some(p) = pointer
        {
            if response.dragged_by(PointerButton::Secondary) && shift {
                self.timeline.drag = Some(Drag::Scale {
                    pivot: scale.f(p.x).round(),
                    from_x: p.x,
                    factor: 1.,
                });
            } else if response.dragged_by(PointerButton::Primary) {
                match under(p) {
                    Some(h) => {
                        let keys = &handles[h].2;
                        if !keys.iter().any(|k| selected.contains(k)) {
                            let line =
                                format!("operandi.key.select keys:{} mode:{mode}", key_list(keys));
                            self.execute(&line);
                        }
                        self.timeline.drag = Some(Drag::Keys {
                            from: scale.f(p.x),
                            offset: 0.,
                        });
                    }
                    None => self.timeline.drag = Some(Drag::Box { from: p, to: p }),
                }
            }
        }
        if response.dragged()
            && let Some(p) = pointer
        {
            match &mut self.timeline.drag {
                Some(Drag::Keys { from, offset }) => *offset = (scale.f(p.x) - *from).round(),
                Some(Drag::Box { to, .. }) => *to = p,
                Some(Drag::Scale {
                    pivot,
                    from_x,
                    factor,
                }) => {
                    let px = scale.x(*pivot);
                    let d0 = (*from_x - px).abs().max(4.);
                    *factor = ((p.x - px).abs() / d0).clamp(0.05, 20.) as f64;
                }
                _ => {}
            }
        }
        if response.drag_stopped() {
            match self.timeline.drag.take() {
                Some(Drag::Keys { offset, .. }) if offset != 0. => {
                    self.execute(&format!("operandi.key.move frames:{}", fmt(offset)));
                }
                Some(Drag::Box { from, to }) => {
                    let r = Rect::from_two_pos(from, to);
                    let keys: Vec<(ChannelKey, Tick)> = handles
                        .iter()
                        .filter(|(row, f, _)| {
                            let y = rect.min.y + *row as f32 * ROW_H + ROW_H / 2.;
                            r.contains(pos2(scale.x(*f), y))
                        })
                        .flat_map(|(_, _, k)| k.clone())
                        .collect();
                    if keys.is_empty() {
                        if mode == "replace" {
                            self.execute("operandi.key.select mode:clear");
                        }
                    } else {
                        let line =
                            format!("operandi.key.select keys:{} mode:{mode}", key_list(&keys));
                        self.execute(&line);
                    }
                }
                Some(Drag::Scale { pivot, factor, .. }) if (factor - 1.).abs() > 1e-3 => {
                    self.execute(&format!(
                        "operandi.key.scale factor:{factor:.4} frame:{}",
                        fmt(pivot)
                    ));
                }
                _ => {}
            }
        }
        // TL-45: the track row's right-click menu.
        if self.timeline.drag.is_none() {
            response.context_menu(|ui| self.track_menu(ui));
        }
    }

    fn track_menu(&mut self, ui: &mut Ui) {
        let has = !self.editor.live_selected_keys().is_empty();
        let clip = operandi_core::anim_edit::clipboard().is_some();
        let run = |a: &mut Self, ui: &mut Ui, text: &str, line: &str, enabled: bool, why: &str| {
            if ui
                .gated(
                    crate::unavailable::unless(enabled, why),
                    egui::Button::new(text),
                )
                .clicked()
            {
                a.execute(line);
                ui.close();
            }
        };
        const SEL: &str = "Select keys first";
        run(self, ui, "Cut", "operandi.key.cut", has, SEL);
        run(self, ui, "Copy", "operandi.key.copy", true, SEL);
        run(
            self,
            ui,
            "Paste",
            "operandi.key.paste",
            clip,
            "Copy keys first",
        );
        run(
            self,
            ui,
            "Paste Insert",
            "operandi.key.paste mode:insert",
            clip,
            "Copy keys first",
        );
        run(
            self,
            ui,
            "Paste Replace",
            "operandi.key.paste mode:replace",
            clip,
            "Copy keys first",
        );
        ui.separator();
        for (text, title, label, value, line) in [
            (
                "Shift…",
                "Shift Keys",
                "Frames",
                "1",
                "operandi.key.move frames:{}",
            ),
            (
                "Scale…",
                "Scale Keys",
                "Factor",
                "1",
                "operandi.key.scale factor:{}",
            ),
        ] {
            if ui
                .gated(
                    crate::unavailable::unless(has, SEL),
                    egui::Button::new(text),
                )
                .clicked()
            {
                self.timeline.dialog = Some(ValueDialog {
                    title,
                    label,
                    text: value.into(),
                    line: line.into(),
                });
                ui.close();
            }
        }
        run(self, ui, "Invert", "operandi.key.invert", has, SEL);
        ui.separator();
        for (text, line) in [
            ("Auto", "operandi.key.slope type:auto"),
            ("Smooth Flat", "operandi.key.slope type:smoothFlat"),
            ("Auto Flat", "operandi.key.slope type:autoFlat"),
            ("Flat", "operandi.key.slope type:flat"),
            ("Linear In", "operandi.key.slope type:linearIn"),
            ("Linear Out", "operandi.key.slope type:linearOut"),
            ("Linear", "operandi.key.slope type:linear"),
            ("Stepped", "operandi.key.slope type:stepped"),
            ("Manual", "operandi.key.slope type:manual"),
        ] {
            run(self, ui, text, line, has, SEL);
        }
        ui.separator();
        run(self, ui, "Delete", "operandi.key.delete", has, SEL);
        run(
            self,
            ui,
            "Delete Related",
            "operandi.key.delete related:true",
            has,
            SEL,
        );
        ui.separator();
        run(
            self,
            ui,
            "Select All",
            "operandi.key.select mode:all",
            true,
            "",
        );
        ui.checkbox(&mut self.timeline.by_type, "Group by Channel Type");
    }

    pub(super) fn timeline_dialog(&mut self, ctx: &Context) {
        let Some(mut d) = self.timeline.dialog.take() else {
            return;
        };
        let mut done: Option<bool> = None;
        egui::Window::new(d.title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, vec2(0., 0.))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(d.label);
                    let r = ui.text_edit_singleline(&mut d.text);
                    self.text_editors.insert(r.id);
                    r.request_focus();
                    if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        done = Some(true);
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        done = Some(true);
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                        done = Some(false);
                    }
                });
            });
        match done {
            Some(true) => {
                let value = d.text.trim();
                let value = if value.contains(' ') {
                    format!("{{{value}}}")
                } else {
                    value.to_owned()
                };
                self.execute(&d.line.replace("{}", &value));
            }
            Some(false) => {}
            None => self.timeline.dialog = Some(d),
        }
    }

    /// A one-value dialog (UIC-80): OK runs `line` with `{}` replaced.
    pub(super) fn open_value_dialog(
        &mut self,
        title: &'static str,
        label: &'static str,
        line: &str,
    ) {
        self.timeline.dialog = Some(ValueDialog {
            title,
            label,
            text: String::new(),
            line: line.to_owned(),
        });
    }

    // ------------------------------------------------------------ buttons

    fn tb(&mut self, ui: &mut Ui, text: &str, name: &str, action: &str, line: &str) -> Response {
        let tip = format!("{name}{}\n{line}", self.shortcut(action));
        let r = ui
            .add(egui::Button::new(text).min_size(vec2(22., 20.)))
            .on_hover_text(tip);
        if r.clicked() {
            self.execute(line);
        }
        r
    }

    /// [`Self::tb`] wearing a rendered icon instead of a text glyph.
    fn tbi(&mut self, ui: &mut Ui, icon: Icon, name: &str, action: &str, line: &str) -> Response {
        let tip = format!("{name}{}\n{line}", self.shortcut(action));
        let button = crate::icons::button(ui, icon, "").min_size(vec2(22., 20.));
        let r = ui.add(button).on_hover_text(tip);
        if r.clicked() {
            self.execute(line);
        }
        r
    }

    fn timeline_buttons(&mut self, ui: &mut Ui, rect: Rect) {
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink2(vec2(6., 2.)))
                .layout(Layout::left_to_right(Align::Center)),
        );
        let ui = &mut child;
        ui.spacing_mut().item_spacing.x = 3.;
        let rate = self.editor.scene.time.rate;
        let current = self.editor.scene.time.current;
        let settings = self.editor.scene.animation.settings.clone();
        // TL-03: Standard / Extended.
        let ext = ui
            .selectable_label(self.timeline.extended, if self.timeline.extended { "«" } else { "»" })
            .on_hover_text("Extended buttons: first/last key, insert/remove frame, key position/rotation/scale (TL-03)");
        if ext.clicked() {
            self.timeline.extended = !self.timeline.extended;
        }
        // TK-30 / TL-30: Auto Key, lit when not Off (UIC-23).
        let auto = settings.auto_key;
        let label = egui::RichText::new(format!("Auto Key: {} ⏷", auto.label()));
        let label = if auto == AutoKey::Off {
            label
        } else {
            label.color(theme::on_orange())
        };
        let button = crate::icons::button_with(ui, Icon::AutoKey, label.into()).fill(
            if auto == AutoKey::Off {
                theme::control()
            } else {
                theme::orange()
            },
        );
        let r = ui.add(button).on_hover_text("Auto Key: Off, Animated (edits key channels that have keys) or All (operandi.key.autoKey)");
        let mut picked = None;
        egui::Popup::menu(&r).show(|ui| {
            for mode in [AutoKey::Off, AutoKey::Animated, AutoKey::All] {
                if ui.radio(auto == mode, mode.label()).clicked() {
                    picked = Some(mode);
                    ui.close();
                }
            }
        });
        if let Some(mode) = picked {
            self.execute(&format!("operandi.key.autoKey mode:{}", mode.name()));
        }
        ui.separator();
        if self.timeline.extended {
            self.tbi(
                ui,
                Icon::FirstKey,
                "First Key",
                "First Key",
                "operandi.time.go to:firstKey",
            );
        }
        self.tbi(
            ui,
            Icon::FirstFrame,
            "First Frame",
            "First Frame",
            "operandi.time.go to:first",
        );
        self.tbi(
            ui,
            Icon::PreviousFrame,
            "Previous Frame",
            "Back One Frame",
            "operandi.time.step frames:-1",
        );
        // The frame field (UIC-51/52).
        let shown = animation::time_text(settings.display, rate, current);
        let mut text = self.timeline.frame_text.clone().unwrap_or(shown);
        let r = ui.add(
            egui::TextEdit::singleline(&mut text)
                .desired_width(64.)
                .horizontal_align(Align::Center),
        );
        self.text_editors.insert(r.id);
        crate::numeric_tab::register(ui, &r, Some(&text));
        if r.has_focus() {
            self.timeline.frame_text = Some(text.clone());
        }
        if r.lost_focus() {
            self.timeline.frame_text = None;
            if ui.input(|i| i.key_pressed(Key::Enter)) {
                match animation::parse_time(settings.display, rate, &text) {
                    Ok(t) => {
                        let f = animation::frames(rate, t);
                        self.execute(&format!("operandi.time.set frame:{}", fmt(f)));
                    }
                    Err(e) => self.status = e,
                }
            }
        }
        r.on_hover_text("Current time: type a frame, seconds (1.5s) or SMPTE (00:00:01:00)");
        self.tbi(
            ui,
            Icon::NextFrame,
            "Next Frame",
            "Forward One Frame",
            "operandi.time.step frames:1",
        );
        self.tbi(
            ui,
            Icon::LastFrame,
            "Last Frame",
            "Last Frame",
            "operandi.time.go to:last",
        );
        let playing = self.editor.anim.playing.is_some();
        let play = self.tbi(
            ui,
            if playing { Icon::Stop } else { Icon::Play },
            "Play / Stop (right-click: Play Backwards)",
            "Play/Stop",
            "operandi.time.play",
        );
        if play.secondary_clicked() {
            self.execute("operandi.time.play backwards:true");
        }
        if self.timeline.extended {
            self.tbi(
                ui,
                Icon::LastKey,
                "Last Key",
                "Last Key",
                "operandi.time.go to:lastKey",
            );
        }
        ui.separator();
        self.tbi(
            ui,
            Icon::PreviousKey,
            "Previous Key",
            "Previous Key",
            "operandi.time.go to:prevKey",
        );
        self.tbi(
            ui,
            Icon::KeyChannels,
            "Key Selected Channels",
            "Key Selected Channels",
            "operandi.key.set",
        );
        self.tbi(
            ui,
            Icon::KeyItems,
            "Key Selected Items",
            "Key Selected Items",
            "operandi.key.items",
        );
        self.tbi(
            ui,
            Icon::NextKey,
            "Next Key",
            "Next Key",
            "operandi.time.go to:nextKey",
        );
        if self.timeline.extended {
            ui.separator();
            self.tbi(
                ui,
                Icon::RemoveFrame,
                "Remove Frame",
                "",
                "operandi.time.removeFrame",
            );
            self.tbi(
                ui,
                Icon::InsertFrame,
                "Insert Frame",
                "",
                "operandi.time.insertFrame",
            );
            self.tb(
                ui,
                "Pos",
                "Key Position",
                "Key Position",
                "operandi.key.transforms type:position",
            );
            self.tb(
                ui,
                "Rot",
                "Key Rotation",
                "Key Rotation",
                "operandi.key.transforms type:rotation",
            );
            self.tb(
                ui,
                "Scl",
                "Key Scale",
                "Key Scale",
                "operandi.key.transforms type:scale",
            );
            self.tbi(
                ui,
                Icon::GraphEditor,
                "Graph Editor",
                "",
                "operandi.graph.open",
            );
            self.tbi(
                ui,
                Icon::AnimationEditors,
                "Animation Editors: the tall timeline and the Graph Editor together",
                "",
                "operandi.anim.editors",
            );
        }
        // TK-41: Apply / Discard while edits are pending.
        let pending = animation::pending(&self.editor.scene).len();
        if pending > 0 {
            ui.separator();
            ui.colored_label(
                crate::form::status::edited_yellow(),
                format!("{pending} edited, not keyed"),
            );
            self.tbi(
                ui,
                Icon::ApplyKeys,
                "Apply: key the edited channels",
                "",
                "operandi.key.apply",
            );
            self.tbi(
                ui,
                Icon::DiscardKeys,
                "Discard the edits",
                "",
                "operandi.key.discard",
            );
        }
        if let Some((done, all, _)) = self.editor.anim.progress.clone() {
            ui.separator();
            ui.label(format!("Frame {done} of {all}"));
            self.tb(
                ui,
                "Cancel",
                "Stop the frame render or playblast",
                "",
                "operandi.render.cancelRange",
            );
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let r = ui.button("⚙").on_hover_text("Time settings (TK-03)");
            egui::Popup::menu(&r)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| self.time_settings(ui));
        });
    }

    /// TK-03: the settings popover; changes apply live (UIC-182).
    pub(super) fn time_settings(&mut self, ui: &mut Ui) {
        let s = &self.editor.scene;
        let rate = s.time.rate;
        let settings = s.animation.settings.clone();
        let (r0, r1) = animation::current_range(s);
        let f = |t: Tick| animation::frames(rate, t);
        let mut lines: Vec<String> = Vec::new();
        egui::Grid::new("time-settings")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Frame Rate");
                let current = rate.label();
                egui::ComboBox::from_id_salt("fps")
                    .selected_text(current.clone())
                    .show_ui(ui, |ui| {
                        for (label, fps) in [
                            ("Film 24", "24"),
                            ("PAL 25", "25"),
                            ("NTSC 29.97", "29.97"),
                            ("30", "30"),
                            ("48", "48"),
                            ("50", "50"),
                            ("60", "60"),
                        ] {
                            if ui.selectable_label(current == fps, label).clicked() {
                                lines.push(format!("operandi.time.settings fps:{fps}"));
                            }
                        }
                    });
                ui.end_row();
                ui.label("Rate");
                let mut fps = rate.as_f64();
                if ui
                    .add(
                        crate::numeric_tab::DragValue::new(&mut fps)
                            .range(0.001..=1000.)
                            .speed(0.1),
                    )
                    .on_hover_text("Any frame rate (Custom)")
                    .changed()
                {
                    lines.push(format!("operandi.time.settings fps:{fps}"));
                }
                ui.end_row();
                ui.label("Time Display");
                egui::ComboBox::from_id_salt("display")
                    .selected_text(settings.display.label())
                    .show_ui(ui, |ui| {
                        for d in [
                            TimeDisplay::Frames,
                            TimeDisplay::Seconds,
                            TimeDisplay::Smpte,
                            TimeDisplay::FilmCode,
                        ] {
                            if ui
                                .selectable_label(settings.display == d, d.label())
                                .clicked()
                            {
                                lines.push(format!("operandi.time.settings display:{}", d.name()));
                            }
                        }
                    });
                ui.end_row();
                for (label, key, value) in [
                    ("Scene Start", "start", f(s.time.start)),
                    ("Scene End", "end", f(s.time.end)),
                    ("Current Start", "rangeStart", f(r0)),
                    ("Current End", "rangeEnd", f(r1)),
                ] {
                    ui.label(label);
                    let mut v = value;
                    let r = ui.add(
                        crate::numeric_tab::DragValue::new(&mut v)
                            .speed(0.5)
                            .max_decimals(0),
                    );
                    if r.drag_stopped() || (r.changed() && !r.dragged()) {
                        lines.push(format!("operandi.time.settings {key}:{}", fmt(v.round())));
                    }
                    ui.end_row();
                }
                ui.label("Auto Key");
                egui::ComboBox::from_id_salt("autokey")
                    .selected_text(settings.auto_key.label())
                    .show_ui(ui, |ui| {
                        for m in [AutoKey::Off, AutoKey::Animated, AutoKey::All] {
                            if ui
                                .selectable_label(settings.auto_key == m, m.label())
                                .clicked()
                            {
                                lines.push(format!("operandi.key.autoKey mode:{}", m.name()));
                            }
                        }
                    });
                ui.end_row();
                ui.label("End Behavior");
                egui::ComboBox::from_id_salt("endb")
                    .selected_text(settings.end.label())
                    .show_ui(ui, |ui| {
                        for b in [EndBehavior::Hold, EndBehavior::Repeat, EndBehavior::Wrap] {
                            if ui.selectable_label(settings.end == b, b.label()).clicked() {
                                lines.push(format!(
                                    "operandi.time.settings endBehavior:{}",
                                    b.name()
                                ));
                            }
                        }
                    });
                ui.end_row();
            });
        for (label, key, value, enabled) in [
            ("Time Snap", "snap", settings.snap, true),
            ("Play Real Time", "realTime", settings.real_time, true),
            (
                "Auto Update Existing Keys",
                "autoUpdate",
                settings.auto_update_keys,
                settings.auto_key == AutoKey::Off,
            ),
        ] {
            let mut v = value;
            // UIC-117: greyed, never hidden.
            if ui
                .gated(
                    crate::unavailable::unless(enabled, "Only while Auto Key is Off"),
                    egui::Checkbox::new(&mut v, label),
                )
                .changed()
            {
                lines.push(format!("operandi.time.settings {key}:{v}"));
            }
        }
        for line in lines {
            self.execute(&line);
        }
    }
}

fn diamond(painter: &egui::Painter, c: Pos2, r: f32, fill: egui::Color32, stroke: Stroke) {
    painter.add(egui::Shape::convex_polygon(
        vec![
            c + vec2(0., -r),
            c + vec2(r, 0.),
            c + vec2(0., r),
            c + vec2(-r, 0.),
        ],
        fill,
        stroke,
    ));
}
