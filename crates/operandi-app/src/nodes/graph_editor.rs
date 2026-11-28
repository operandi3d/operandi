//! The graph editor (BEHAVIOR-GRAPH-EDITOR GE-01..93): a channel list, the
//! curves (time across, value up) and a time strip, with a toolbar and a
//! bottom bar.
//!
//! Everything that changes keys runs a command line (UIC-160): a drag
//! previews and records one command at mouse-up (GE-45, UIC-161/224);
//! view settings (fit, zoom, normalise, stacked, isolate) are the graph's
//! own state through `operandi.graph.view`. Key selection, the clipboard,
//! the current time and key time are shared with the timeline and track
//! row (GE-90..93). Its keys over the graph come from the one routing
//! seam (anim_keys.rs, GE-21).
//!
//! [OP, provisional] It opens as a floating window (Animate > Graph
//! Editor, the timeline's Graph button), not yet a dockable panel kind
//! (GE-01, UIC-190..195; FLAGS).
use super::*;
use crate::form::status::key_red;
use crate::icons::Icon;
use crate::unavailable::Gate;
use operandi_core::anim_curve::{Behavior, Curve};
use operandi_core::animation::{self, ChannelKey};
use operandi_core::time_tick::Tick;

const LIST_W: f32 = 190.;
const STRIP_H: f32 = 22.;
/// The bottom bar's fixed row.
const BOTTOM_H: f32 = 26.;
/// The graph editor window's remembered place (palettes' places).
const GRAPH_PLACE: &str = "graphEditor";
const KEY_R: f32 = 3.5;

/// GE-51 [OP]: the broken-handles key colour, a theme token so colour
/// schemes can change it.
pub(crate) fn broken_key() -> Color32 {
    Color32::from_rgb(220, 70, 200)
}

/// What a graph drag is doing.
#[derive(Clone, Debug)]
enum Drag {
    /// Moving the selected keys; `axes`: (time, value) allowed.
    Keys {
        from: Pos2,
        to: Pos2,
        axes: (bool, bool),
    },
    Handle {
        channel: ChannelKey,
        time: Tick,
        incoming: bool,
        to: Pos2,
    },
    Box {
        from: Pos2,
        to: Pos2,
    },
    Pan {
        last: Pos2,
    },
    Zoom {
        last: Pos2,
        time: bool,
    },
    /// Shift+right / Shift+middle: scale time or value around `at`.
    Scale {
        at: Pos2,
        to: Pos2,
        time: bool,
    },
    Scrub,
    /// GE-44: a Box Scale drag of `part` of the box.
    BoxScale {
        part: BoxPart,
        bounds: [f64; 4],
        from: Pos2,
        to: Pos2,
    },
    /// GE-31: Ctrl+Shift+drag dropping copies.
    Copies {
        from: Pos2,
        to: Pos2,
    },
    /// GE-20: Ctrl+Alt+right-drag box zoom.
    BoxZoom {
        from: Pos2,
        to: Pos2,
    },
    /// GE-X5: a retime marker dragged.
    Retime {
        index: usize,
        to: f64,
    },
}

/// Which part of the Box Scale box a drag holds (GE-44).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BoxPart {
    Inside,
    Left,
    Right,
    Top,
    Bottom,
    Corner { left: bool, top: bool },
}

/// The graph editor's own state (per window).
#[derive(Clone, Debug, Default)]
pub(crate) struct GraphEditor {
    pub(crate) open: bool,
    /// Visible (frames, value) box; `None` fits the shown keys.
    view: Option<[f64; 4]>,
    pub(crate) normalised: bool,
    pub(crate) stacked: bool,
    /// GE-X6: only the hovered or selected curve.
    pub(crate) isolate: bool,
    pub(crate) velocity: bool,
    pub(crate) show_shadow: bool,
    /// GE-42: related keys move together in time.
    pub(crate) link: bool,
    /// GE-44: the Box Scale tool.
    pub(crate) box_scale: bool,
    /// GE-X5: the retime tool and its markers (frames).
    pub(crate) retime: bool,
    pub(crate) retime_marks: Vec<f64>,
    /// GE-81: Speed for the shown position channels.
    pub(crate) speed: bool,
    /// GE-82: Stacked rows in view and the first row shown.
    pub(crate) rows: usize,
    pub(crate) row_offset: usize,
    /// This frame's shown channels as `item:name`.
    shown_keys: Vec<String>,
    /// GE-11 filters.
    animated_only: bool,
    hide_transforms: bool,
    hide_item_channels: bool,
    /// GE-11 Lock: the list keeps these items.
    locked: Option<Vec<u64>>,
    drag: Option<Drag>,
    /// The toolbar's Frame and Value fields while typed in.
    frame_text: Option<String>,
    value_text: Option<String>,
    /// Where the graph drew (tests, keys).
    pub(crate) graph_rect: Option<Rect>,
    /// A fit asked for by `operandi.graph.view`, done when next drawn.
    fit: Option<String>,
}

/// A shown curve: its channel and the curve (or the flat value).
struct Shown {
    key: ChannelKey,
    curve: Option<Curve>,
    value: f64,
    colour: Color32,
}

fn colour_of(k: &ChannelKey, i: usize) -> Color32 {
    match k.channel.rsplit_once('.').map(|(_, a)| a) {
        Some("X") | Some("r") => Color32::from_rgb(225, 80, 70),
        Some("Y") | Some("g") => Color32::from_rgb(90, 200, 90),
        Some("Z") | Some("b") => Color32::from_rgb(80, 140, 235),
        _ => [
            Color32::from_rgb(230, 190, 80),
            Color32::from_rgb(170, 130, 230),
            Color32::from_rgb(90, 200, 200),
            Color32::from_rgb(220, 140, 90),
        ][i % 4],
    }
}

fn fmt(f: f64) -> String {
    if (f - f.round()).abs() < 1e-6 {
        format!("{}", f.round() as i64)
    } else {
        format!("{f:.4}")
    }
}

/// Frames and values ↔ the graph's screen.
#[derive(Clone, Copy)]
struct Map {
    rect: Rect,
    f0: f64,
    f1: f64,
    v0: f64,
    v1: f64,
}

impl Map {
    fn x(&self, f: f64) -> f32 {
        self.rect.left() + ((f - self.f0) / (self.f1 - self.f0)) as f32 * self.rect.width()
    }
    fn y(&self, v: f64) -> f32 {
        self.rect.bottom() - ((v - self.v0) / (self.v1 - self.v0)) as f32 * self.rect.height()
    }
    fn f(&self, x: f32) -> f64 {
        self.f0 + ((x - self.rect.left()) / self.rect.width()) as f64 * (self.f1 - self.f0)
    }
    fn v(&self, y: f32) -> f64 {
        self.v0 + ((self.rect.bottom() - y) / self.rect.height()) as f64 * (self.v1 - self.v0)
    }
    fn per_frame(&self) -> f64 {
        self.rect.width() as f64 / (self.f1 - self.f0)
    }
    fn per_value(&self) -> f64 {
        self.rect.height() as f64 / (self.v1 - self.v0)
    }
}

impl Operandi {
    /// The items the channel list shows (GE-10/11).
    fn graph_items(&self) -> Vec<u64> {
        match &self.graph.locked {
            Some(ids) => ids
                .iter()
                .copied()
                .filter(|id| self.editor.scene.item(*id).is_some())
                .collect(),
            None => self.editor.scene.selected_items(),
        }
    }

    /// The channel list's rows: item, channel names (GE-10/11).
    fn graph_list(&self) -> Vec<(u64, Vec<String>)> {
        let s = &self.editor.scene;
        self.graph_items()
            .into_iter()
            .map(|id| {
                let names = animation::keyable_channels(s, id)
                    .into_iter()
                    .filter(|n| {
                        let animated = s.animation.is_animated(&ChannelKey::new(id, n));
                        let transform = ChannelKey::new(id, n).transform_part().is_some();
                        (!self.graph.animated_only || animated)
                            && !(self.graph.hide_transforms && transform)
                            && !(self.graph.hide_item_channels && !transform)
                    })
                    .collect();
                (id, names)
            })
            .collect()
    }

    /// The curves drawn (GE-10: the selected channels; none selected, the
    /// animated channels of the listed items).
    fn graph_shown(&self) -> Vec<Shown> {
        let s = &self.editor.scene;
        let listed: Vec<ChannelKey> = self
            .graph_list()
            .into_iter()
            .flat_map(|(id, names)| names.into_iter().map(move |n| ChannelKey::new(id, &n)))
            .collect();
        let mut chosen: Vec<ChannelKey> = listed
            .iter()
            .filter(|k| self.editor.anim.selected_channels.contains(k))
            .cloned()
            .collect();
        if chosen.is_empty() {
            chosen = listed
                .into_iter()
                .filter(|k| s.animation.is_animated(k))
                .collect();
        }
        chosen
            .into_iter()
            .enumerate()
            .map(|(i, k)| Shown {
                colour: colour_of(&k, i),
                curve: s
                    .animation
                    .channels
                    .get(&k)
                    .filter(|c| !c.curve.is_empty())
                    .map(|c| c.curve.clone()),
                value: animation::read(s, &k).unwrap_or(0.),
                key: k,
            })
            .collect()
    }

    /// The frame/value box around `keys` (or every shown key).
    fn graph_fit(&self, shown: &[Shown], only_selected: bool) -> [f64; 4] {
        let rate = self.editor.scene.time.rate;
        let selected = self.editor.live_selected_keys();
        let mut b = [f64::MAX, f64::MIN, f64::MAX, f64::MIN];
        for s in shown {
            match &s.curve {
                Some(c) => {
                    for k in &c.keys {
                        if only_selected && !selected.contains(&(s.key.clone(), k.time)) {
                            continue;
                        }
                        let f = animation::frames(rate, k.time);
                        b = [
                            b[0].min(f),
                            b[1].max(f),
                            b[2].min(k.value),
                            b[3].max(k.value),
                        ];
                    }
                }
                None if !only_selected => {
                    b[2] = b[2].min(s.value);
                    b[3] = b[3].max(s.value);
                }
                None => {}
            }
        }
        let (r0, r1) = animation::current_range(&self.editor.scene);
        if b[0] > b[1] {
            b[0] = animation::frames(rate, r0);
            b[1] = animation::frames(rate, r1);
        }
        if b[2] > b[3] {
            b[2] = -1.;
            b[3] = 1.;
        }
        let pf = ((b[1] - b[0]) * 0.08).max(1.);
        let pv = ((b[3] - b[2]) * 0.12).max(0.1);
        [b[0] - pf, b[1] + pf, b[2] - pv, b[3] + pv]
    }

    /// `operandi.graph.view`: fits and the view toggles (GE-21, GE-82,
    /// GE-X6, GE-81).
    pub(crate) fn graph_view(&mut self, rest: &str) {
        for word in rest.split_whitespace() {
            let Some((k, v)) = word.split_once(':') else {
                continue;
            };
            let toggle = |old: bool| match v {
                "toggle" => !old,
                "false" | "off" => false,
                _ => true,
            };
            match k {
                "fit" => self.graph.fit = Some(v.to_owned()),
                "normalise" | "normalize" => self.graph.normalised = toggle(self.graph.normalised),
                "stacked" => self.graph.stacked = toggle(self.graph.stacked),
                "isolate" => self.graph.isolate = toggle(self.graph.isolate),
                "velocity" => self.graph.velocity = toggle(self.graph.velocity),
                "shadow" => self.graph.show_shadow = toggle(self.graph.show_shadow),
                "link" => self.graph.link = toggle(self.graph.link),
                "boxScale" => self.graph.box_scale = toggle(self.graph.box_scale),
                "retime" => self.graph.retime = toggle(self.graph.retime),
                "speed" => self.graph.speed = toggle(self.graph.speed),
                "rows" => self.graph.rows = v.parse::<usize>().unwrap_or(6).clamp(1, 12),
                "rowOffset" => self.graph.row_offset = v.parse().unwrap_or(0),
                "animatedOnly" => self.graph.animated_only = toggle(self.graph.animated_only),
                "lock" => {
                    self.graph.locked = toggle(self.graph.locked.is_some())
                        .then(|| self.editor.scene.selected_items())
                }
                _ => self.status = format!("operandi.graph.view: unknown {k}"),
            }
        }
    }

    /// The graph editor window (GE-01): its own OS window (Jason,
    /// 2026-11-22: a true floating window, draggable onto another screen,
    /// resized from its edges), through the app's one floating-window path
    /// (`os_window`, floating.rs), so shortcuts, undo and the MCP bridge
    /// work as in the main window. It opens as a wide, short strip over the
    /// lower part of the main window and then keeps where it was put
    /// (remembered with the palettes' places). Where the platform can't
    /// open another window (tests, some Linux setups) egui draws it inside
    /// the main one.
    pub(super) fn graph_editor_window(&mut self, ctx: &Context) {
        if !self.graph.open || !crate::anim_switch::enabled() {
            self.graph.graph_rect = None;
            return;
        }
        let place = self.palettes.places.get(GRAPH_PLACE).copied();
        let main = ctx.input(|i| i.viewport().inner_rect);
        let monitor = ctx.input(|i| i.viewport().monitor_size);
        let size = place.map_or_else(
            || {
                // ~3.5:1, clamped to the screen.
                let max = monitor.unwrap_or(vec2(1600., 1000.));
                [
                    1200f32.min(max.x - 40.).max(400.),
                    340f32.min(max.y - 80.).max(200.),
                ]
            },
            |p| p.size,
        );
        let position = place.and_then(|p| p.position).or_else(|| {
            main.map(|r| {
                [
                    r.left() + ((r.width() - size[0]) / 2.).max(0.),
                    (r.bottom() - size[1] - 40.).max(r.top()),
                ]
            })
        });
        let spec = crate::floating::OsWindow {
            id: egui::ViewportId::from_hash_of("operandi-graph-editor"),
            title: "Graph Editor".into(),
            size,
            position,
            min_size: [420., 200.],
        };
        let shown = self.os_window(ctx, spec, |app, ctx| {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::side_top_panel(&ctx.style())
                        .fill(ctx.style().visuals.panel_fill)
                        .inner_margin(6.),
                )
                .show(ctx, |ui| app.graph_editor_ui(ui));
        });
        let mut place = place.unwrap_or(crate::palettes::Place {
            size,
            position: None,
        });
        if let Some(s) = shown.size
            && s.iter().all(|v| *v >= 100.)
        {
            place.size = s;
        }
        if let Some(at) = shown.position {
            place.position = Some(at);
        }
        self.palettes.places.insert(GRAPH_PLACE.into(), place);
        if !shown.open {
            self.graph.open = false;
        }
    }

    /// The window's contents, laid out inside exactly the space it was
    /// given: the toolbar on top, the bottom bar a fixed row at the
    /// bottom, and the list, graph and strip in what's left. Nothing asks
    /// for more than it was given, so the window never grows by itself and
    /// a resize sticks.
    fn graph_editor_ui(&mut self, ui: &mut Ui) {
        // The shown channels, worked out once a frame (GE-X7).
        self.graph.shown_keys = self.graph_shown().iter().map(|s| s.key.text()).collect();
        let full = ui.available_rect_before_wrap();
        let toolbar = ui
            .scope(|ui| {
                ui.set_max_width(full.width());
                self.graph_toolbar(ui);
            })
            .response
            .rect;
        let bottom = Rect::from_min_max(
            pos2(
                full.left(),
                (full.bottom() - BOTTOM_H).max(toolbar.bottom()),
            ),
            full.max,
        );
        let body = Rect::from_min_max(
            pos2(full.left(), toolbar.bottom() + 4.),
            pos2(full.right(), (bottom.top() - 4.).max(toolbar.bottom() + 4.)),
        );
        let list = Rect::from_min_size(
            body.min,
            vec2(LIST_W.min(body.width() * 0.3), body.height()),
        );
        let graph = Rect::from_min_max(
            pos2(list.right() + 6., body.top()),
            pos2(body.right(), (body.bottom() - STRIP_H).max(body.top())),
        );
        let strip = Rect::from_min_max(
            pos2(graph.left(), graph.bottom()),
            pos2(body.right(), body.bottom()),
        );
        self.graph_channel_list(ui, list);
        let shown = self.graph_shown();
        let map = self.graph_map(graph, &shown);
        self.graph_area(ui, map, &shown);
        self.graph_strip(ui, strip, map);
        let mut row = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(bottom)
                .layout(Layout::left_to_right(Align::Center)),
        );
        row.set_clip_rect(bottom.intersect(ui.clip_rect()));
        self.graph_bottom_bar(&mut row);
        // Claim exactly the space given, no more.
        ui.allocate_rect(full, Sense::hover());
    }

    fn graph_map(&mut self, rect: Rect, shown: &[Shown]) -> Map {
        if let Some(fit) = self.graph.fit.take() {
            let all = self.graph_fit(shown, false);
            let sel = self.graph_fit(shown, true);
            let now = self.graph.view.unwrap_or(all);
            self.graph.view = Some(match fit.as_str() {
                "selected" => sel,
                "time" => [all[0], all[1], now[2], now[3]],
                "values" => [now[0], now[1], all[2], all[3]],
                _ => all,
            });
        }
        let [f0, f1, v0, v1] = self
            .graph
            .view
            .unwrap_or_else(|| self.graph_fit(shown, false));
        let (v0, v1) = if self.graph.normalised {
            (-1.2, 1.2)
        } else {
            (v0, v1)
        };
        Map {
            rect,
            f0,
            f1: f1.max(f0 + 1e-3),
            v0,
            v1: v1.max(v0 + 1e-6),
        }
    }

    fn graph_channel_list(&mut self, ui: &mut Ui, rect: Rect) {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        let ui = &mut child;
        let mut picked: Option<String> = None;
        egui::ScrollArea::vertical()
            .id_salt("graph-channels")
            .show(ui, |ui| {
                let list = self.graph_list();
                if list.is_empty() {
                    ui.weak("Select items to see their channels");
                }
                for (id, names) in list {
                    let name = self
                        .editor
                        .scene
                        .item(id)
                        .map_or_else(|| id.to_string(), |i| i.name.clone());
                    ui.strong(name);
                    for n in names {
                        let k = ChannelKey::new(id, &n);
                        let animated = self.editor.scene.animation.is_animated(&k);
                        let selected = self.editor.anim.selected_channels.contains(&k);
                        let text = if animated {
                            egui::RichText::new(format!("  ◆ {n}")).color(key_red())
                        } else {
                            egui::RichText::new(format!("    {n}"))
                        };
                        let r = ui.selectable_label(selected, text);
                        if r.clicked() {
                            let add = ui.input(|i| i.modifiers.command || i.modifiers.shift);
                            picked = Some(format!(
                                "operandi.channel.select channel:{} add:{add}",
                                k.text()
                            ));
                        }
                    }
                }
            });
        if let Some(line) = picked {
            self.execute(&line);
        }
    }

    fn graph_area(&mut self, ui: &mut Ui, map: Map, shown: &[Shown]) {
        self.graph.graph_rect = Some(map.rect);
        let rate = self.editor.scene.time.rate;
        let painter = ui.painter_at(map.rect);
        painter.rect_filled(map.rect, CornerRadius::ZERO, theme::field());
        // Grid: frames across, values up.
        let step = |span: f64, px: f64| {
            let raw = span * 60. / px.max(1.);
            let p = 10f64.powf(raw.log10().floor());
            [1., 2., 5., 10.]
                .into_iter()
                .map(|m| m * p)
                .find(|s| *s >= raw)
                .unwrap_or(raw)
        };
        let fs = step(map.f1 - map.f0, map.rect.width() as f64).max(1.);
        let mut f = (map.f0 / fs).ceil() * fs;
        while f <= map.f1 {
            let x = map.x(f);
            painter.line_segment(
                [pos2(x, map.rect.top()), pos2(x, map.rect.bottom())],
                Stroke::new(0.5f32, theme::border()),
            );
            painter.text(
                pos2(x + 2., map.rect.top() + 1.),
                egui::Align2::LEFT_TOP,
                fmt(f),
                egui::FontId::proportional(9.),
                theme::weak_text(),
            );
            f += fs;
        }
        if !self.graph.normalised && !self.graph.stacked {
            let vs = step(map.v1 - map.v0, map.rect.height() as f64);
            let mut v = (map.v0 / vs).ceil() * vs;
            while v <= map.v1 {
                let y = map.y(v);
                painter.line_segment(
                    [pos2(map.rect.left(), y), pos2(map.rect.right(), y)],
                    Stroke::new(0.5f32, theme::border()),
                );
                painter.text(
                    pos2(map.rect.left() + 2., y - 1.),
                    egui::Align2::LEFT_BOTTOM,
                    fmt((v * 1e4).round() / 1e4),
                    egui::FontId::proportional(9.),
                    theme::weak_text(),
                );
                v += vs;
            }
        }
        let selected = self.editor.live_selected_keys();
        let hover = ui
            .input(|i| i.pointer.hover_pos())
            .filter(|p| map.rect.contains(*p));
        // GE-82: per-curve value maps for Normalised and Stacked.
        // GE-82: up to `rows` rows in view (Ctrl+wheel), from `row_offset`
        // (Shift+wheel).
        let row_count = if self.graph.rows == 0 {
            6
        } else {
            self.graph.rows
        };
        let rows = shown.len().clamp(1, row_count) as f32;
        let offset = self.graph.row_offset.min(shown.len().saturating_sub(1));
        let (stacked, normalised) = (self.graph.stacked, self.graph.normalised);
        let curve_map = move |i: usize, s: &Shown| -> Map {
            let (lo, hi) = match &s.curve {
                Some(c) => c.keys.iter().fold((f64::MAX, f64::MIN), |(a, b), k| {
                    (a.min(k.value), b.max(k.value))
                }),
                None => (s.value - 1., s.value + 1.),
            };
            let (lo, hi) = if hi - lo < 1e-9 {
                (lo - 1., hi + 1.)
            } else {
                (lo, hi)
            };
            if stacked {
                let h = map.rect.height() / rows;
                let row = i as f32 - offset as f32;
                // Rows scrolled out of view sit off the graph (clipped).
                let top = map.rect.top() + row * h;
                let pad = (hi - lo) * 0.15;
                Map {
                    rect: Rect::from_min_size(
                        pos2(map.rect.left(), top),
                        vec2(map.rect.width(), h),
                    ),
                    v0: lo - pad,
                    v1: hi + pad,
                    ..map
                }
            } else if normalised {
                // lo..hi onto -1..1 of the shared band.
                let span = (hi - lo) / 2.;
                let mid = (hi + lo) / 2.;
                Map {
                    v0: mid - 1.2 * span,
                    v1: mid + 1.2 * span,
                    ..map
                }
            } else {
                map
            }
        };
        // GE-X6: isolate the hovered (else first selected) curve.
        let isolated: Option<ChannelKey> = self
            .graph
            .isolate
            .then(|| {
                selected
                    .first()
                    .map(|(k, _)| k.clone())
                    .or_else(|| shown.first().map(|s| s.key.clone()))
            })
            .flatten();
        for (i, s) in shown.iter().enumerate() {
            if isolated.as_ref().is_some_and(|k| *k != s.key) {
                continue;
            }
            let m = curve_map(i, s);
            let Some(curve) = &s.curve else {
                // GE-12: no keys, a dotted flat line at its value.
                let y = m.y(s.value);
                let mut x = m.rect.left();
                while x < m.rect.right() {
                    painter.line_segment(
                        [pos2(x, y), pos2((x + 4.).min(m.rect.right()), y)],
                        Stroke::new(1.0f32, s.colour.gamma_multiply(0.7)),
                    );
                    x += 8.;
                }
                continue;
            };
            // GE-80: the shadow, faded behind.
            if self.graph.show_shadow
                && let Some(sh) = self.editor.scene.animation.shadows.get(&s.key)
            {
                draw_curve(&painter, &m, sh, rate, s.colour.gamma_multiply(0.3), 1.0);
            }
            draw_curve(&painter, &m, curve, rate, s.colour, 1.6);
            // GE-81 Velocity: the rate of change, per frame.
            if self.graph.velocity && shown.len() == 1 {
                let fps = rate.as_f64();
                let pts: Vec<Pos2> = (0..=m.rect.width() as i32 / 2)
                    .map(|px| {
                        let f = m.f(m.rect.left() + px as f32 * 2.);
                        let d = curve.derivative(animation::at_frames(rate, f)) / fps;
                        pos2(m.x(f), m.y(d))
                    })
                    .collect();
                painter.add(egui::Shape::line(pts, Stroke::new(1.0f32, theme::cyan())));
            }
            // Keys: unified white, broken magenta, selected accent (GE-51).
            let keys = if curve.cycle {
                curve.cycled()
            } else {
                curve.clone()
            };
            let n = curve.keys.len();
            // GE-X7: only keys in view; where keys are closer than 3 px
            // the curve shows them and only selected keys draw.
            let dense = m.per_frame() < 3.;
            for (j, k) in curve.keys.iter().enumerate() {
                let f = animation::frames(rate, k.time);
                if f < m.f0 - 1. || f > m.f1 + 1. {
                    continue;
                }
                let is_sel = selected.contains(&(s.key.clone(), k.time));
                if dense && !is_sel {
                    continue;
                }
                let shown_v = keys.keys[j].value;
                let (mut x, mut y) = (m.x(f), m.y(shown_v));
                if is_sel && let Some(Drag::Keys { from, to, axes }) = &self.graph.drag {
                    if axes.0 {
                        let df = ((to.x - from.x) as f64 / m.per_frame()).round();
                        x = m.x(f + df);
                    }
                    if axes.1 {
                        y += to.y - from.y;
                    }
                }
                // GE-62: the last key's own value as a ghost, joined.
                if curve.cycle && j == n - 1 && (k.value - shown_v).abs() > 1e-9 {
                    let gy = m.y(k.value);
                    painter.line_segment(
                        [pos2(x, gy), pos2(x, y)],
                        Stroke::new(1.0f32, theme::weak_text()),
                    );
                    painter.rect_stroke(
                        Rect::from_center_size(pos2(x, gy), vec2(KEY_R * 2., KEY_R * 2.)),
                        CornerRadius::ZERO,
                        Stroke::new(1.0f32, theme::weak_text()),
                        egui::StrokeKind::Middle,
                    );
                }
                if is_sel {
                    self.graph_handles(&painter, &m, curve, j, pos2(x, y), &s.key);
                }
                let fill = if is_sel {
                    theme::orange()
                } else if k.broken {
                    broken_key()
                } else {
                    Color32::WHITE
                };
                painter.rect_filled(
                    Rect::from_center_size(pos2(x, y), vec2(KEY_R * 2., KEY_R * 2.)),
                    CornerRadius::ZERO,
                    fill,
                );
                // GE-X4: an Auto slope that overshoots a neighbour.
                if overshoots(curve, j, rate) {
                    painter.circle_stroke(
                        pos2(x, y),
                        KEY_R + 3.,
                        Stroke::new(1.2f32, theme::orange()),
                    );
                }
            }
        }
        // GE-81 Speed: distance per frame of the shown positions.
        if self.graph.speed {
            self.draw_speed(&painter, &map, shown);
        }
        // GE-44: the Box Scale box around the selected keys.
        if let Some(b) = self.box_bounds(shown) {
            let r = box_rect(&map, b);
            let r = match &self.graph.drag {
                Some(Drag::BoxScale {
                    part,
                    bounds,
                    from,
                    to,
                }) => box_rect(
                    &map,
                    scaled_box(*bounds, *part, *from, *to, &map, ui.input(|i| i.modifiers)).0,
                ),
                _ => r,
            };
            painter.rect_stroke(
                r,
                CornerRadius::ZERO,
                Stroke::new(1.0f32, theme::cyan()),
                egui::StrokeKind::Middle,
            );
            for c in [
                r.left_top(),
                r.right_top(),
                r.left_bottom(),
                r.right_bottom(),
                r.center_top(),
                r.center_bottom(),
                r.left_center(),
                r.right_center(),
            ] {
                painter.rect_filled(
                    Rect::from_center_size(c, vec2(6., 6.)),
                    CornerRadius::ZERO,
                    theme::cyan(),
                );
            }
        }
        if let Some(Drag::BoxZoom { from, to }) = &self.graph.drag {
            painter.rect_stroke(
                Rect::from_two_pos(*from, *to),
                CornerRadius::ZERO,
                Stroke::new(1.0f32, theme::text()),
                egui::StrokeKind::Middle,
            );
        }
        // The current time (and key time) lines (GE-93).
        let now = animation::frames(rate, self.editor.scene.time.current);
        painter.line_segment(
            [
                pos2(map.x(now), map.rect.top()),
                pos2(map.x(now), map.rect.bottom()),
            ],
            Stroke::new(1.5f32, theme::orange()),
        );
        if let Some(kt) = self.editor.anim.key_time {
            let x = map.x(animation::frames(rate, kt));
            painter.line_segment(
                [pos2(x, map.rect.top()), pos2(x, map.rect.bottom())],
                Stroke::new(1.5f32, theme::cyan()),
            );
        }
        if let Some(Drag::Box { from, to }) = &self.graph.drag {
            painter.rect(
                Rect::from_two_pos(*from, *to),
                CornerRadius::ZERO,
                theme::orange().gamma_multiply(0.1),
                Stroke::new(1.0f32, theme::orange()),
                egui::StrokeKind::Middle,
            );
        }
        self.graph_input(ui, map, shown, &curve_map, hover);
    }

    /// GE-50: a selected key's handles.
    fn graph_handles(
        &self,
        painter: &egui::Painter,
        m: &Map,
        curve: &Curve,
        j: usize,
        at: Pos2,
        key: &ChannelKey,
    ) {
        for incoming in [true, false] {
            let Some(end) = handle_end(m, curve, j, incoming, self.editor.scene.time.rate) else {
                continue;
            };
            let end = match &self.graph.drag {
                Some(Drag::Handle {
                    channel,
                    time,
                    incoming: side,
                    to,
                }) if channel == key && *time == curve.keys[j].time && *side == incoming => *to,
                _ => end,
            };
            painter.line_segment([at, end], Stroke::new(1.0f32, theme::text()));
            painter.circle_filled(end, 3., theme::text());
        }
    }

    #[allow(clippy::too_many_lines)]
    fn graph_input(
        &mut self,
        ui: &mut Ui,
        map: Map,
        shown: &[Shown],
        curve_map: &dyn Fn(usize, &Shown) -> Map,
        hover: Option<Pos2>,
    ) {
        let rate = self.editor.scene.time.rate;
        let response = ui.interact(map.rect, Id::new("graph-area"), Sense::click_and_drag());
        let pointer = response.interact_pointer_pos().or(hover);
        let selected = self.editor.live_selected_keys();
        // Every key on screen: (curve index, key time, position).
        let mut keys: Vec<(usize, Tick, Pos2)> = Vec::new();
        for (i, s) in shown.iter().enumerate() {
            if let Some(c) = &s.curve {
                let m = curve_map(i, s);
                let shown_c = if c.cycle { c.cycled() } else { c.clone() };
                for (j, k) in c.keys.iter().enumerate() {
                    // GE-X7: keys out of view can't be hit.
                    let f = animation::frames(rate, k.time);
                    if f < m.f0 - 1. || f > m.f1 + 1. {
                        continue;
                    }
                    keys.push((
                        i,
                        k.time,
                        pos2(
                            m.x(animation::frames(rate, k.time)),
                            m.y(shown_c.keys[j].value),
                        ),
                    ));
                }
            }
        }
        let key_under = |p: Pos2| {
            keys.iter()
                .find(|(_, _, at)| at.distance(p) <= KEY_R + 3.)
                .cloned()
        };
        let handle_under = |p: Pos2| -> Option<(ChannelKey, Tick, bool)> {
            for (i, s) in shown.iter().enumerate() {
                let Some(c) = &s.curve else { continue };
                let m = curve_map(i, s);
                for (j, k) in c.keys.iter().enumerate() {
                    if !selected.contains(&(s.key.clone(), k.time)) {
                        continue;
                    }
                    for incoming in [true, false] {
                        if handle_end(&m, c, j, incoming, rate).is_some_and(|e| e.distance(p) <= 5.)
                        {
                            return Some((s.key.clone(), k.time, incoming));
                        }
                    }
                }
            }
            None
        };
        let mods = ui.input(|i| i.modifiers);
        let mode = if mods.shift && (mods.command || mods.ctrl) {
            "toggle"
        } else if mods.shift {
            "add"
        } else if mods.command || mods.ctrl {
            "remove"
        } else {
            "replace"
        };
        let key_line = |i: usize, t: Tick| {
            format!(
                "{}@{}",
                shown[i].key.text(),
                fmt(animation::frames(rate, t))
            )
        };
        // Wheel: zoom at the pointer (GE-20); Stacked: Ctrl+wheel sets the
        // rows in view, Shift+wheel steps through them (GE-82).
        if let Some(p) = hover {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y + i.smooth_scroll_delta.x);
            let wheel_mods = ui.input(|i| i.modifiers);
            if self.graph.stacked && scroll.abs() > 0. && (wheel_mods.command || wheel_mods.ctrl) {
                let rows = if self.graph.rows == 0 {
                    6
                } else {
                    self.graph.rows
                };
                let rows = if scroll > 0. {
                    rows.saturating_sub(1)
                } else {
                    rows + 1
                };
                self.execute(&format!("operandi.graph.view rows:{}", rows.clamp(1, 12)));
            } else if self.graph.stacked && scroll.abs() > 0. && wheel_mods.shift {
                let off = if scroll > 0. {
                    self.graph.row_offset.saturating_sub(1)
                } else {
                    (self.graph.row_offset + 1).min(shown.len().saturating_sub(1))
                };
                self.execute(&format!("operandi.graph.view rowOffset:{off}"));
            } else if scroll.abs() > 0. {
                let k = (1.0 - scroll as f64 * 0.002).clamp(0.5, 2.0);
                let (cf, cv) = (map.f(p.x), map.v(p.y));
                self.graph.view = Some([
                    cf - (cf - map.f0) * k,
                    cf + (map.f1 - cf) * k,
                    cv - (cv - map.v0) * k,
                    cv + (map.v1 - cv) * k,
                ]);
            }
        }
        if response.double_clicked()
            && let Some(p) = pointer
            && let Some((i, t, _)) = key_under(p)
        {
            if mods.shift {
                // GE-51: Shift+double-click resets the handles to Auto.
                self.execute(&format!("operandi.key.select keys:{}", key_line(i, t)));
                self.execute("operandi.key.autoHandles slopes:true");
            } else if let Some(c) = &shown[i].curve {
                // GE-30: every key on its curve.
                let all: Vec<String> = c.keys.iter().map(|k| key_line(i, k.time)).collect();
                self.execute(&format!("operandi.key.select keys:{}", all.join(",")));
            }
            return;
        }
        if response.clicked()
            && let Some(p) = pointer
        {
            match key_under(p) {
                Some((i, t, _)) => {
                    let line = format!(
                        "operandi.key.select keys:{} mode:{}",
                        key_line(i, t),
                        if mode == "toggle" { "add" } else { mode }
                    );
                    self.execute(&line);
                }
                None if mode == "replace" => self.execute("operandi.key.select mode:clear"),
                None => {}
            }
        }
        // GE-31: middle-click (or Ctrl+Alt+click) adds a key at the
        // pointer, on the nearest curve; normalised, only on a curve.
        let ctrl_alt_click = response.clicked() && mods.alt && (mods.ctrl || mods.command);
        if (response.middle_clicked() || ctrl_alt_click)
            && let Some(p) = pointer
        {
            let near = shown.iter().enumerate().min_by(|(ia, a), (ib, b)| {
                let d = |i: usize, s: &Shown| {
                    let m = curve_map(i, s);
                    let v = s.curve.as_ref().map_or(s.value, |c| {
                        c.value_at(animation::at_frames(rate, m.f(p.x)))
                    });
                    (m.y(v) - p.y).abs()
                };
                d(*ia, a).total_cmp(&d(*ib, b))
            });
            if let Some((i, s)) = near {
                let m = curve_map(i, s);
                let f = m.f(p.x).round();
                let on_curve = s.curve.as_ref().is_some_and(|c| {
                    (m.y(c.value_at(animation::at_frames(rate, f))) - p.y).abs() < 8.
                });
                let value = if on_curve {
                    String::new()
                } else if self.graph.normalised && shown.len() > 1 {
                    // GE-33: no single value scale.
                    return;
                } else {
                    format!(" value:{}", m.v(p.y))
                };
                self.execute(&format!(
                    "operandi.key.add channel:{} frame:{}{value}",
                    s.key.text(),
                    fmt(f)
                ));
            }
        }
        if response.drag_started()
            && let Some(p) = pointer
        {
            let middle = response.dragged_by(PointerButton::Middle);
            let right = response.dragged_by(PointerButton::Secondary);
            let primary = response.dragged_by(PointerButton::Primary);
            let boxed = self.box_bounds(shown).filter(|_| primary);
            self.graph.drag = if right && mods.alt && (mods.ctrl || mods.command) {
                // GE-20: box zoom.
                Some(Drag::BoxZoom { from: p, to: p })
            } else if let Some(b) = boxed
                && let Some(part) = box_part(box_rect(&map, b), p)
            {
                Some(Drag::BoxScale {
                    part,
                    bounds: b,
                    from: p,
                    to: p,
                })
            } else if primary
                && mods.shift
                && (mods.ctrl || mods.command)
                && key_under(p)
                    .is_some_and(|(i, t, _)| selected.contains(&(shown[i].key.clone(), t)))
            {
                // GE-31: drop copies along the drag.
                Some(Drag::Copies { from: p, to: p })
            } else if mods.alt && middle && mods.shift {
                Some(Drag::Zoom {
                    last: p,
                    time: false,
                })
            } else if mods.alt && right && mods.shift {
                Some(Drag::Zoom {
                    last: p,
                    time: true,
                })
            } else if mods.alt && (middle || right) {
                Some(Drag::Pan { last: p })
            } else if mods.shift && (middle || right) {
                Some(Drag::Scale {
                    at: p,
                    to: p,
                    time: right,
                })
            } else if let Some((channel, time, incoming)) =
                handle_under(p).filter(|_| response.dragged_by(PointerButton::Primary))
            {
                Some(Drag::Handle {
                    channel,
                    time,
                    incoming,
                    to: p,
                })
            } else if let Some((i, t, _)) = key_under(p) {
                if !selected.contains(&(shown[i].key.clone(), t)) {
                    self.execute(&format!(
                        "operandi.key.select keys:{} mode:{mode}",
                        key_line(i, t)
                    ));
                }
                Some(Drag::Keys {
                    from: p,
                    to: p,
                    axes: (!middle, !right),
                })
            } else if middle || right {
                // GE-40: on empty graph, middle moves values, right times.
                Some(Drag::Keys {
                    from: p,
                    to: p,
                    axes: (right, middle),
                })
            } else {
                Some(Drag::Box { from: p, to: p })
            };
        }
        if response.dragged()
            && let Some(p) = pointer
        {
            match &mut self.graph.drag {
                Some(Drag::Keys { to, .. })
                | Some(Drag::BoxScale { to, .. })
                | Some(Drag::Copies { to, .. })
                | Some(Drag::BoxZoom { to, .. })
                | Some(Drag::Box { to, .. })
                | Some(Drag::Handle { to, .. })
                | Some(Drag::Scale { to, .. }) => *to = p,
                Some(Drag::Pan { last }) => {
                    let d = p - *last;
                    let (df, dv) = (
                        -(d.x as f64) / map.per_frame(),
                        d.y as f64 / map.per_value(),
                    );
                    self.graph.view = Some([map.f0 + df, map.f1 + df, map.v0 + dv, map.v1 + dv]);
                    *last = p;
                }
                Some(Drag::Zoom { last, time }) => {
                    let d = (p.x - last.x) as f64;
                    let k = (1.0 - d * 0.005).clamp(0.5, 2.0);
                    let (cf, cv) = (
                        map.f((map.rect.left() + map.rect.right()) / 2.),
                        map.v(map.rect.center().y),
                    );
                    self.graph.view = Some(if *time {
                        [
                            cf - (cf - map.f0) * k,
                            cf + (map.f1 - cf) * k,
                            map.v0,
                            map.v1,
                        ]
                    } else {
                        [
                            map.f0,
                            map.f1,
                            cv - (cv - map.v0) * k,
                            cv + (map.v1 - cv) * k,
                        ]
                    });
                    *last = p;
                }
                _ => {}
            }
        }
        if response.drag_stopped() {
            match self.graph.drag.take() {
                Some(Drag::BoxScale {
                    part,
                    bounds,
                    from,
                    to,
                }) => {
                    let (b, moved) = scaled_box(bounds, part, from, to, &map, mods);
                    if moved {
                        let tf = (b[1] - b[0]) / (bounds[1] - bounds[0]);
                        let vf = (b[3] - b[2]) / (bounds[3] - bounds[2]);
                        // Scale around the old box's low corner, then move
                        // it onto the new one's.
                        self.execute(&format!(
                            "operandi.key.transform timeFactor:{tf} timePivot:{} valueFactor:{vf} valuePivot:{} frames:{} value:{}",
                            bounds[0],
                            bounds[2],
                            fmt((b[0] - bounds[0]).round()),
                            b[2] - bounds[2]
                        ));
                    }
                }
                Some(Drag::Copies { from, to }) => {
                    let df = ((to.x - from.x) as f64 / map.per_frame()).round();
                    let dv = -((to.y - from.y) as f64) / map.per_value();
                    if df != 0. {
                        self.execute(&format!(
                            "operandi.key.dropCopies frames:{} value:{dv}",
                            fmt(df)
                        ));
                    }
                }
                Some(Drag::BoxZoom { from, to }) => {
                    let r = Rect::from_two_pos(from, to);
                    if r.width() > 4. && r.height() > 4. {
                        self.graph.view = Some([
                            map.f(r.left()),
                            map.f(r.right()),
                            map.v(r.bottom()),
                            map.v(r.top()),
                        ]);
                    }
                }
                Some(Drag::Keys { from, to, axes }) => {
                    let df = if axes.0 {
                        ((to.x - from.x) as f64 / map.per_frame()).round()
                    } else {
                        0.
                    };
                    let dv = if axes.1 && !self.graph.normalised && !self.graph.stacked {
                        -((to.y - from.y) as f64) / map.per_value()
                    } else {
                        0.
                    };
                    if df != 0. || dv.abs() > 1e-12 {
                        let link = if self.graph.link { " link:true" } else { "" };
                        self.execute(&format!(
                            "operandi.key.offset frames:{} value:{dv}{link}",
                            fmt(df)
                        ));
                    }
                }
                Some(Drag::Box { from, to }) => {
                    let r = Rect::from_two_pos(from, to);
                    let inside: Vec<String> = keys
                        .iter()
                        .filter(|(_, _, at)| r.contains(*at))
                        .map(|(i, t, _)| key_line(*i, *t))
                        .collect();
                    if inside.is_empty() {
                        if mode == "replace" {
                            self.execute("operandi.key.select mode:clear");
                        }
                    } else {
                        let m = if mode == "toggle" { "add" } else { mode };
                        self.execute(&format!(
                            "operandi.key.select keys:{} mode:{m}",
                            inside.join(",")
                        ));
                    }
                }
                Some(Drag::Handle {
                    channel,
                    time,
                    incoming,
                    to,
                }) => {
                    let i = shown.iter().position(|s| s.key == channel);
                    if let Some(i) = i
                        && let Some(c) = &shown[i].curve
                        && let Some(j) = c.key_at(time)
                    {
                        let m = curve_map(i, &shown[i]);
                        let at = pos2(m.x(animation::frames(rate, time)), m.y(c.keys[j].value));
                        let (dx, dy) = (to.x - at.x, at.y - to.y);
                        let df = dx as f64 / m.per_frame();
                        let dvv = dy as f64 / m.per_value();
                        let df = if incoming {
                            df.min(-1e-3)
                        } else {
                            df.max(1e-3)
                        };
                        let slope = dvv / df;
                        // Weight: the handle's time length against a third
                        // of the segment on that side.
                        let seg = if incoming {
                            j.checked_sub(1).map(|p| {
                                animation::frames(rate, time)
                                    - animation::frames(rate, c.keys[p].time)
                            })
                        } else {
                            c.keys.get(j + 1).map(|n| {
                                animation::frames(rate, n.time) - animation::frames(rate, time)
                            })
                        };
                        let weight = seg.map(|s| (df.abs() / (s / 3.)).clamp(0.01, 3.));
                        self.execute(&format!(
                            "operandi.key.select keys:{}@{}",
                            channel.text(),
                            fmt(animation::frames(rate, time))
                        ));
                        let mut line = format!(
                            "operandi.key.handle side:{} slope:{slope}",
                            if incoming { "in" } else { "out" }
                        );
                        if let Some(w) = weight
                            && (w - c.keys[j].weight_out.max(c.keys[j].weight_in)).abs() > 0.05
                        {
                            line.push_str(&format!(" weight:{w:.3}"));
                        }
                        self.execute(&line);
                    }
                }
                Some(Drag::Scale { at, to, time }) => {
                    if time {
                        let factor = (1. + (to.x - at.x) as f64 / 200.).max(0.05);
                        self.execute(&format!(
                            "operandi.key.scale factor:{factor:.4} frame:{}",
                            fmt(map.f(at.x).round())
                        ));
                    } else {
                        let factor = 1. - (to.y - at.y) as f64 / 200.;
                        self.execute(&format!(
                            "operandi.key.scaleValue factor:{factor:.4} pivot:{}",
                            map.v(at.y)
                        ));
                    }
                }
                _ => {}
            }
        }
        // GE-73: right-click: the key menu (a key under the pointer joins
        // the selection first).
        if response.secondary_clicked()
            && let Some(p) = pointer
            && let Some((i, t, _)) = key_under(p)
            && !selected.contains(&(shown[i].key.clone(), t))
        {
            self.execute(&format!(
                "operandi.key.select keys:{} mode:add",
                key_line(i, t)
            ));
        }
        if self.graph.drag.is_none() {
            let mut picked: Option<String> = None;
            response.context_menu(|ui| graph_menu(ui, &mut picked, !selected.is_empty()));
            if let Some(line) = picked {
                self.execute(&line);
            }
        }
        // GE-21: keys over the graph, from the routing seam.
        if hover.is_some() && !self.text_editor_focused(ui.ctx()) {
            let lines =
                ui.input_mut(|i| crate::anim_keys::take_keys(i, crate::anim_keys::GRAPH_KEYS));
            for line in lines {
                self.execute(line);
            }
        }
    }

    /// GE-02/93: the time strip under the graph; scrubbing it scrubs the
    /// scene.
    fn graph_strip(&mut self, ui: &mut Ui, rect: Rect, map: Map) {
        let rate = self.editor.scene.time.rate;
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::ZERO, theme::control());
        let now = animation::frames(rate, self.editor.scene.time.current);
        let x = map.x(now);
        painter.line_segment(
            [pos2(x, rect.top()), pos2(x, rect.bottom())],
            Stroke::new(2.0f32, theme::orange()),
        );
        painter.text(
            pos2(x + 3., rect.center().y),
            egui::Align2::LEFT_CENTER,
            animation::time_text(
                self.editor.scene.animation.settings.display,
                rate,
                self.editor.scene.time.current,
            ),
            egui::FontId::proportional(10.),
            theme::text(),
        );
        for t in self.editor.scope_key_times() {
            let x = map.x(animation::frames(rate, t));
            if rect.x_range().contains(x) {
                painter.rect_filled(
                    Rect::from_center_size(pos2(x, rect.bottom() - 5.), vec2(4., 4.)),
                    CornerRadius::ZERO,
                    key_red(),
                );
            }
        }
        let r = ui.interact(rect, Id::new("graph-strip"), Sense::click_and_drag());
        // GE-X5: with the retime tool on, the strip holds retime markers:
        // a click adds one, a drag moves one and the keys between its
        // neighbours follow, right-click removes one.
        if self.graph.retime {
            self.retime_strip(ui, rect, map, &r);
            return;
        }
        if let Some(p) = r.interact_pointer_pos() {
            if r.drag_started() {
                self.graph.drag = Some(Drag::Scrub);
            }
            if r.dragged() {
                let _ = self
                    .editor
                    .set_current_time(animation::at_frames(rate, map.f(p.x)), false);
            }
            if r.clicked() || r.drag_stopped() {
                self.graph.drag = None;
                self.execute(&format!(
                    "operandi.time.set frame:{}",
                    fmt(map.f(p.x).round())
                ));
            }
        }
    }

    fn retime_strip(&mut self, ui: &mut Ui, rect: Rect, map: Map, r: &Response) {
        let painter = ui.painter_at(rect);
        let marks = self.graph.retime_marks.clone();
        let dragging = match &self.graph.drag {
            Some(Drag::Retime { index, to }) => Some((*index, *to)),
            _ => None,
        };
        for (i, f) in marks.iter().enumerate() {
            let f = match dragging {
                Some((j, to)) if j == i => to,
                _ => *f,
            };
            let x = map.x(f);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(x - 5., rect.top()),
                    pos2(x + 5., rect.top()),
                    pos2(x, rect.bottom()),
                ],
                theme::cyan(),
                Stroke::NONE,
            ));
        }
        let Some(p) = r.interact_pointer_pos() else {
            return;
        };
        let under = marks.iter().position(|f| (map.x(*f) - p.x).abs() <= 6.);
        let f = map.f(p.x).round();
        if r.drag_started()
            && let Some(i) = under
        {
            self.graph.drag = Some(Drag::Retime { index: i, to: f });
        }
        if r.dragged()
            && let Some(Drag::Retime { to, .. }) = &mut self.graph.drag
        {
            *to = f;
        }
        if r.drag_stopped()
            && let Some(Drag::Retime { index, to }) = self.graph.drag.take()
        {
            let from = marks[index];
            let mut sorted = marks.clone();
            sorted.sort_by(f64::total_cmp);
            let rate = self.editor.scene.time.rate;
            let s = &self.editor.scene;
            let low = sorted
                .iter()
                .rev()
                .find(|m| **m < from)
                .copied()
                .unwrap_or(animation::frames(rate, s.time.start).min(from - 1.));
            let high = sorted
                .iter()
                .find(|m| **m > from)
                .copied()
                .unwrap_or(animation::frames(rate, s.time.end).max(from + 1.));
            let to = to.clamp(low + 1., high - 1.);
            let channels: Vec<String> = self.graph.shown_keys.clone();
            if to != from && !channels.is_empty() {
                self.execute(&format!(
                    "operandi.curve.retime channels:{} low:{} high:{} from:{} to:{}",
                    channels.join(","),
                    fmt(low),
                    fmt(high),
                    fmt(from),
                    fmt(to)
                ));
                self.graph.retime_marks[index] = to;
            }
            return;
        }
        if r.clicked() && under.is_none() {
            self.graph.retime_marks.push(f);
        }
        if r.secondary_clicked()
            && let Some(i) = under
        {
            self.graph.retime_marks.remove(i);
        }
    }

    fn graph_toolbar(&mut self, ui: &mut Ui) {
        let rate = self.editor.scene.time.rate;
        let sel = self.editor.live_selected_keys();
        let has = !sel.is_empty();
        let mut lines: Vec<String> = Vec::new();
        ui.horizontal_wrapped(|ui| {
            // GE-03: Frame and Value of the selected keys, "(mixed)".
            let values: Vec<f64> = sel
                .iter()
                .filter_map(|(k, t)| {
                    let c = &self.editor.scene.animation.channels.get(k)?.curve;
                    c.key_at(*t).map(|i| c.keys[i].value)
                })
                .collect();
            let frames: Vec<f64> = sel
                .iter()
                .map(|(_, t)| animation::frames(rate, *t))
                .collect();
            let shown = |v: &[f64]| match v.first() {
                None => String::new(),
                Some(a) if v.iter().all(|b| (a - b).abs() < 1e-9) => fmt(*a),
                Some(_) => crate::form::rows::MIXED.to_owned(),
            };
            for (label, list, cmd, text) in [
                ("Frame", &frames, "operandi.key.setFrame frame:", 0),
                ("Value", &values, "operandi.key.setValue value:", 1),
            ] {
                ui.label(label);
                let slot = if text == 0 {
                    &mut self.graph.frame_text
                } else {
                    &mut self.graph.value_text
                };
                let mut t = slot.clone().unwrap_or_else(|| shown(list));
                let r = ui.gated(
                    crate::unavailable::unless(has, "Select keys first"),
                    egui::TextEdit::singleline(&mut t).desired_width(64.),
                );
                self.text_editors.insert(r.id);
                crate::numeric_tab::register(ui, &r, Some(&t));
                let slot = if text == 0 {
                    &mut self.graph.frame_text
                } else {
                    &mut self.graph.value_text
                };
                if r.has_focus() {
                    *slot = Some(t.clone());
                }
                if r.lost_focus() {
                    *slot = None;
                    if ui.input(|i| i.key_pressed(Key::Enter)) && t.trim().parse::<f64>().is_ok() {
                        lines.push(format!("{cmd}{}", t.trim()));
                    }
                }
            }
            ui.separator();
            let shown_channels: Vec<String> =
                self.graph.shown_keys.clone();
            let list = shown_channels.join(",");
            let now = fmt(animation::frames(rate, self.editor.scene.time.current));
            if ui
                .button("Key")
                .on_hover_text("Key the shown channels at the current time (GE-32)")
                .clicked()
                && !list.is_empty()
            {
                lines.push(format!("operandi.key.set channels:{list}"));
            }
            if ui
                .add(crate::icons::button(ui, Icon::KeyKeepingShape, ""))
                .on_hover_text("Key Keeping Shape: key the shown channels without changing their curves (GE-32)\noperandi.key.keepShape")
                .clicked()
                && !list.is_empty()
            {
                lines.push(format!(
                    "operandi.key.keepShape channels:{list} frame:{now}"
                ));
            }
            ui.separator();
            // GE-52 presets.
            for (text, line, tip) in [
                ("Auto", "operandi.key.slope type:auto", "Slopes Auto"),
                (
                    "Manual",
                    "operandi.key.slope type:manual",
                    "Slopes Manual (keeps today's shape)",
                ),
                (
                    "Smooth Flat",
                    "operandi.key.slope type:smoothFlat",
                    "Smooth Flat (TK-12)",
                ),
                (
                    "Auto Flat",
                    "operandi.key.slope type:autoFlat",
                    "Auto, flat next to equal values",
                ),
                ("Flat", "operandi.key.slope type:flat", "Flat"),
                (
                    "Linear",
                    "operandi.key.slope type:linear",
                    "Linear (right-click: In / Out)",
                ),
                ("Stepped", "operandi.key.slope type:stepped", "Stepped"),
            ] {
                let r = ui
                    .gated(
                        crate::unavailable::unless(has, "Select keys first"),
                        egui::Button::new(text),
                    )
                    .on_hover_text(tip);
                if r.clicked() {
                    lines.push(line.into());
                }
                if text == "Linear" {
                    r.context_menu(|ui| {
                        for (t, l) in [("Linear In", "linearIn"), ("Linear Out", "linearOut")] {
                            if ui.button(t).clicked() {
                                lines.push(format!("operandi.key.slope type:{l}"));
                                ui.close();
                            }
                        }
                    });
                }
            }
            ui.separator();
            for (icon, tip, line) in [
                (Icon::BreakTangents, "Break Handles: each side moves alone (GE-51)", "operandi.key.break"),
                (Icon::UnifyTangents, "Unify Handles: one straight line (GE-51)", "operandi.key.unify"),
            ] {
                if ui
                    .gated(
                        crate::unavailable::unless(has, "Select keys first"),
                        crate::icons::button(ui, icon, ""),
                    )
                    .on_hover_text(format!("{tip}\n{line}"))
                    .clicked()
                {
                    lines.push(line.into());
                }
            }
            // GE-53: In / Out slope pop-ups; Out greys while the keys are
            // unified (it follows In).
            let unified = sel.iter().all(|(k, t)| {
                self.editor
                    .scene
                    .animation
                    .channels
                    .get(k)
                    .and_then(|c| c.curve.key_at(*t).map(|i| !c.curve.keys[i].broken))
                    .unwrap_or(true)
            });
            for (side, icon) in [("in", Icon::InSlope), ("out", Icon::OutSlope)] {
                let why = if !has {
                    Some("Select keys first")
                } else if side == "out" && unified {
                    Some("The out side follows the in side while the handles are unified; Break them first")
                } else {
                    None
                };
                let r = ui
                    .gated(why, crate::icons::button(ui, icon, ""))
                    .on_hover_text(format!("The selected keys' {side} slope (GE-53)"));
                egui::Popup::menu(&r).show(|ui| {
                    for (name, slope) in operandi_core::anim_curve::Slope::NAMES {
                        let _ = slope;
                        if ui.button(name).clicked() {
                            lines.push(format!("operandi.key.slope type:{name} side:{side}"));
                            ui.close();
                        }
                    }
                });
            }
            ui.separator();
            for (side, icon) in [("pre", Icon::PreBehaviour), ("post", Icon::PostBehaviour)] {
                let r = ui.add(crate::icons::button(ui, icon, "")).on_hover_text(format!(
                    "{} Behaviour: what the curve does {} the keys (GE-61)\noperandi.curve.behavior {side}:…",
                    if side == "pre" { "Pre" } else { "Post" },
                    if side == "pre" { "before" } else { "after" }
                ));
                egui::Popup::menu(&r).show(|ui| {
                    for (name, b) in Behavior::NAMES {
                        if ui.button(b.label()).clicked() {
                            lines.push(format!("operandi.curve.behavior {side}:{name}"));
                            ui.close();
                        }
                    }
                });
            }
            if ui
                .add(crate::icons::button(ui, Icon::CycleCurve, ""))
                .on_hover_text("Cycle: the last key matches the first (GE-62)\noperandi.curve.cycle")
                .clicked()
            {
                lines.push("operandi.curve.cycle on:true".into());
            }
            ui.separator();
            for (icon, on, line, tip) in [
                (Icon::BoxScale, self.graph.box_scale, "operandi.graph.view boxScale:toggle", "Box Scale: a box around the selected keys to move and scale them (GE-44)"),
                (Icon::Retime, self.graph.retime, "operandi.graph.view retime:toggle", "Retime: click the time strip for markers, drag one to stretch the timing (GE-X5)"),
            ] {
                if ui.add(crate::icons::button(ui, icon, "").selected(on)).on_hover_text(format!("{tip}\n{line}")).clicked() {
                    lines.push(line.into());
                }
            }
            // GE-83: envelope presets.
            let presets = ui.add(crate::icons::button(ui, Icon::CurvePresets, "")).on_hover_text("Envelope presets: save the shown channels' curves, or put one on them (GE-83)");
            let folder = self.envelope_folder();
            egui::Popup::menu(&presets).show(|ui| {
                if ui.button("Save Envelope…").clicked() {
                    lines.push("@save-envelope".into());
                    ui.close();
                }
                ui.separator();
                let mut files: Vec<PathBuf> = std::fs::read_dir(&folder)
                    .map(|d| {
                        d.filter_map(|e| e.ok().map(|e| e.path()))
                            .filter(|p| p.extension().is_some_and(|x| x == operandi_core::anim_graph::ENVELOPE_EXTENSION))
                            .collect()
                    })
                    .unwrap_or_default();
                files.sort();
                if files.is_empty() {
                    ui.weak("No envelopes saved yet");
                }
                for f in files {
                    let name = f.file_stem().map_or_else(String::new, |s| s.to_string_lossy().into_owned());
                    if ui.button(&name).clicked() {
                        lines.push(format!("operandi.curve.applyEnvelope path:{{{}}}", f.display()));
                        ui.close();
                    }
                }
            });
            ui.separator();
            if ui.button("Set Frame…").clicked() && has {
                self.timeline_value_dialog("Set Frame", "Frame", "operandi.key.setFrame frame:{}");
            }
            if ui.button("Set Value…").clicked() && has {
                self.timeline_value_dialog("Set Value", "Value", "operandi.key.setValue value:{}");
            }
            if ui.button("Scale Keys…").clicked() && has {
                self.timeline_value_dialog("Scale Keys", "Factor", "operandi.key.scale factor:{}");
            }
        });
        let shown: Vec<String> = self.graph.shown_keys.clone();
        for line in lines {
            if line == "@save-envelope" {
                let folder = self.envelope_folder();
                let _ = std::fs::create_dir_all(&folder);
                self.open_value_dialog(
                    "Save Envelope",
                    "Name",
                    &format!(
                        "operandi.curve.saveEnvelope channels:{} path:{{{}/{{}}.{}}}",
                        shown.join(","),
                        folder.display(),
                        operandi_core::anim_graph::ENVELOPE_EXTENSION
                    ),
                );
                continue;
            }
            // A preset goes on the shown channels at the current time.
            if line.starts_with("operandi.curve.applyEnvelope") && !shown.is_empty() {
                self.execute(&format!("{line} channels:{}", shown.join(",")));
                continue;
            }
            self.execute(&line);
        }
    }

    /// GE-83: where envelope presets are kept: "Envelopes" in the presets
    /// folder.
    pub(crate) fn envelope_folder(&self) -> PathBuf {
        self.environment.paths().presets().join("Envelopes")
    }

    fn graph_bottom_bar(&mut self, ui: &mut Ui) {
        let mut lines: Vec<&str> = Vec::new();
        ui.horizontal(|ui| {
            // Icon toggles (lit when on, UIC-23); tooltips name their keys.
            for (icon, on, tip, line) in [
                (
                    Icon::Normalised,
                    self.graph.normalised,
                    "Normalised: every curve in one -1..1 band (N, GE-82)",
                    "operandi.graph.view normalise:toggle",
                ),
                (
                    Icon::Stacked,
                    self.graph.stacked,
                    "Stacked: each curve in its own row (GE-82)",
                    "operandi.graph.view stacked:toggle",
                ),
                (
                    Icon::IsolateCurve,
                    self.graph.isolate,
                    "Isolate: only the selected curve (GE-X6)",
                    "operandi.graph.view isolate:toggle",
                ),
                (
                    Icon::Velocity,
                    self.graph.velocity,
                    "Velocity: the curve's rate of change (GE-81)",
                    "operandi.graph.view velocity:toggle",
                ),
                (
                    Icon::LinkRelated,
                    self.graph.link,
                    "Link Related: X/Y/Z keys move together in time (GE-42)",
                    "operandi.graph.view link:toggle",
                ),
            ] {
                if ui
                    .add(crate::icons::button(ui, icon, "").selected(on))
                    .on_hover_text(format!("{tip}\n{line}"))
                    .clicked()
                {
                    lines.push(line);
                }
            }
            if ui
                .add(crate::icons::button(ui, Icon::CurveSpeed, "").selected(self.graph.speed))
                .on_hover_text(
                    "Speed: each curve's rate of change (GE-81)\noperandi.graph.view speed:toggle",
                )
                .clicked()
            {
                lines.push("operandi.graph.view speed:toggle");
            }
            for (text, on, line) in [
                (
                    "Show Shadow",
                    self.graph.show_shadow,
                    "operandi.graph.view shadow:toggle",
                ),
                (
                    "Animated Only",
                    self.graph.animated_only,
                    "operandi.graph.view animatedOnly:toggle",
                ),
                (
                    "Lock List",
                    self.graph.locked.is_some(),
                    "operandi.graph.view lock:toggle",
                ),
            ] {
                if ui.selectable_label(on, text).clicked() {
                    lines.push(line);
                }
            }
            ui.separator();
            ui.label("Shadow:");
            for (icon, tip, line) in [
                (
                    Icon::ShadowCopy,
                    "Copy to Shadow (GE-80)",
                    "operandi.curve.shadow action:copy",
                ),
                (
                    Icon::ShadowSwap,
                    "Swap with Shadow (GE-80)",
                    "operandi.curve.shadow action:swap",
                ),
            ] {
                if ui
                    .add(crate::icons::button(ui, icon, ""))
                    .on_hover_text(format!("{tip}\n{line}"))
                    .clicked()
                {
                    lines.push(line);
                }
            }
            for (text, line) in [
                ("Replace", "operandi.curve.shadow action:replace"),
                ("Clear", "operandi.curve.shadow action:clear"),
            ] {
                if ui.button(text).clicked() {
                    lines.push(line);
                }
            }
            ui.separator();
            if ui
                .button("Fit")
                .on_hover_text("A: fit every shown key")
                .clicked()
            {
                lines.push("operandi.graph.view fit:all");
            }
        });
        for line in lines {
            self.execute(line);
        }
    }
}

/// A key's handle end on screen (GE-50): along its slope, `weight` × a
/// third of the segment on that side long in time.
fn handle_end(
    m: &Map,
    curve: &Curve,
    j: usize,
    incoming: bool,
    rate: operandi_core::time_tick::FrameRate,
) -> Option<Pos2> {
    let k = &curve.keys[j];
    let neighbour = if incoming {
        j.checked_sub(1).map(|p| &curve.keys[p])
    } else {
        curve.keys.get(j + 1)
    }?;
    let fk = animation::frames(rate, k.time);
    let seg = (animation::frames(rate, neighbour.time) - fk).abs();
    let w = if incoming { k.weight_in } else { k.weight_out };
    let len = seg / 3. * w;
    let slope = curve.slope(j, incoming) / rate.as_f64();
    let dir = if incoming { -1. } else { 1. };
    let (f, v) = (fk + dir * len, k.value + dir * len * slope);
    Some(pos2(m.x(f), m.y(v)))
}

fn draw_curve(
    painter: &egui::Painter,
    m: &Map,
    curve: &Curve,
    rate: operandi_core::time_tick::FrameRate,
    colour: Color32,
    width: f32,
) {
    // GE-X7: one sample per two pixels, one polyline per curve.
    let n = (m.rect.width() / 2.).max(2.) as usize;
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| {
            let x = m.rect.left() + i as f32 * m.rect.width() / n as f32;
            let v = curve.value_at(animation::at_frames(rate, m.f(x)));
            pos2(
                x,
                m.y(v).clamp(m.rect.top() - 2000., m.rect.bottom() + 2000.),
            )
        })
        .collect();
    painter.add(egui::Shape::line(pts, Stroke::new(width, colour)));
}

/// GE-73: the graph's right-click menu, never empty (UIC-07).
fn graph_menu(ui: &mut Ui, picked: &mut Option<String>, has: bool) {
    fn item(ui: &mut Ui, picked: &mut Option<String>, ok: bool, text: &str, line: &str) {
        if ui
            .gated(
                crate::unavailable::unless(ok, "Select keys first"),
                egui::Button::new(text),
            )
            .clicked()
        {
            *picked = Some(line.into());
            ui.close();
        }
    }
    item(ui, picked, has, "Cut", "operandi.key.cut");
    item(ui, picked, true, "Copy", "operandi.key.copy");
    item(ui, picked, true, "Paste", "operandi.key.paste");
    item(
        ui,
        picked,
        true,
        "Paste Insert",
        "operandi.key.paste mode:insert",
    );
    item(
        ui,
        picked,
        true,
        "Paste Replace",
        "operandi.key.paste mode:replace",
    );
    ui.separator();
    ui.menu_button("Slope", |ui| {
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
            if ui.button(text).clicked() {
                *picked = Some(line.into());
                ui.close();
            }
        }
    });
    item(ui, picked, has, "Break Handles", "operandi.key.break");
    item(ui, picked, has, "Unify Handles", "operandi.key.unify");
    item(ui, picked, has, "Auto Weights", "operandi.key.autoHandles");
    ui.separator();
    item(ui, picked, has, "Invert", "operandi.key.invert");
    item(ui, picked, has, "Delete", "operandi.key.delete");
    item(
        ui,
        picked,
        has,
        "Delete Keys (and related)",
        "operandi.key.delete related:true",
    );
    ui.separator();
    ui.menu_button("Interpolation", |ui| {
        for (text, line) in [
            ("Curve", "operandi.curve.interpolation type:curve"),
            ("Linear", "operandi.curve.interpolation type:linear"),
            ("Stepped", "operandi.curve.interpolation type:stepped"),
        ] {
            if ui.button(text).clicked() {
                *picked = Some(line.into());
                ui.close();
            }
        }
    });
    item(
        ui,
        picked,
        true,
        "Euler Filter",
        "operandi.curve.eulerFilter",
    );
    item(ui, picked, true, "Simplify", "operandi.curve.simplify");
    item(ui, picked, true, "Bake", "operandi.key.bake");
}

impl Operandi {
    /// A one-value dialog for the graph editor's Set Frame / Set Value /
    /// Scale Keys (UIC-80; the timeline's dialog, one undo step on OK).
    fn timeline_value_dialog(&mut self, title: &'static str, label: &'static str, line: &str) {
        self.open_value_dialog(title, label, line);
    }
}

/// GE-X4: key `j` has an Auto side and the curve next to it leaves the
/// range between the key and that neighbour (an overshoot).
fn overshoots(curve: &Curve, j: usize, rate: operandi_core::time_tick::FrameRate) -> bool {
    use operandi_core::anim_curve::Slope;
    let k = &curve.keys[j];
    let auto = |s: Slope| s == Slope::Auto;
    let mut sides = Vec::new();
    if auto(k.slope_in) && j > 0 {
        sides.push(j - 1);
    }
    if auto(k.slope_out) && j + 1 < curve.keys.len() {
        sides.push(j + 1);
    }
    sides.into_iter().any(|n| {
        let other = &curve.keys[n];
        let (lo, hi) = (k.value.min(other.value), k.value.max(other.value));
        let slack = (hi - lo).max(1e-9) * 1e-3;
        let (a, b) = (
            animation::frames(rate, k.time.min(other.time)),
            animation::frames(rate, k.time.max(other.time)),
        );
        (1..16).any(|s| {
            let f = a + (b - a) * s as f64 / 16.;
            let v = curve.value_at(animation::at_frames(rate, f));
            v < lo - slack || v > hi + slack
        })
    })
}

fn box_rect(m: &Map, b: [f64; 4]) -> Rect {
    Rect::from_min_max(pos2(m.x(b[0]), m.y(b[3])), pos2(m.x(b[1]), m.y(b[2])))
}

/// Which part of the box `p` is on (GE-44), handles first.
fn box_part(r: Rect, p: Pos2) -> Option<BoxPart> {
    let near = |a: f32, b: f32| (a - b).abs() <= 6.;
    let inside_x = (r.left() - 6.0..=r.right() + 6.).contains(&p.x);
    let inside_y = (r.top() - 6.0..=r.bottom() + 6.).contains(&p.y);
    if !(inside_x && inside_y) {
        return None;
    }
    let (l, rr, t, b) = (
        near(p.x, r.left()),
        near(p.x, r.right()),
        near(p.y, r.top()),
        near(p.y, r.bottom()),
    );
    Some(match (l, rr, t, b) {
        (true, _, true, _) => BoxPart::Corner {
            left: true,
            top: true,
        },
        (true, _, _, true) => BoxPart::Corner {
            left: true,
            top: false,
        },
        (_, true, true, _) => BoxPart::Corner {
            left: false,
            top: true,
        },
        (_, true, _, true) => BoxPart::Corner {
            left: false,
            top: false,
        },
        (true, ..) => BoxPart::Left,
        (_, true, ..) => BoxPart::Right,
        (_, _, true, _) => BoxPart::Top,
        (_, _, _, true) => BoxPart::Bottom,
        _ => BoxPart::Inside,
    })
}

/// The box `bounds` (frames, values) after dragging `part` from `from` to
/// `to`; whether it changed. Inside moves (Shift: values only, Ctrl: time
/// only); a side scales from the opposite side; a corner scales both
/// (Ctrl+corner warp is provisional: the same as a corner, GE-Q09).
fn scaled_box(
    bounds: [f64; 4],
    part: BoxPart,
    from: Pos2,
    to: Pos2,
    m: &Map,
    mods: Modifiers,
) -> ([f64; 4], bool) {
    let df = ((to.x - from.x) as f64 / m.per_frame()).round();
    let dv = -((to.y - from.y) as f64) / m.per_value();
    let [mut f0, mut f1, mut v0, mut v1] = bounds;
    match part {
        BoxPart::Inside => {
            let (df, dv) = if mods.shift {
                (0., dv)
            } else if mods.ctrl || mods.command {
                (df, 0.)
            } else {
                (df, dv)
            };
            f0 += df;
            f1 += df;
            v0 += dv;
            v1 += dv;
        }
        BoxPart::Left => f0 = (f0 + df).min(f1 - 1.),
        BoxPart::Right => f1 = (f1 + df).max(f0 + 1.),
        BoxPart::Top => v1 = (v1 + dv).max(v0 + 1e-6),
        BoxPart::Bottom => v0 = (v0 + dv).min(v1 - 1e-6),
        BoxPart::Corner { left, top } => {
            if left {
                f0 = (f0 + df).min(f1 - 1.);
            } else {
                f1 = (f1 + df).max(f0 + 1.);
            }
            if top {
                v1 = (v1 + dv).max(v0 + 1e-6);
            } else {
                v0 = (v0 + dv).min(v1 - 1e-6);
            }
        }
    }
    let b = [f0, f1, v0, v1];
    (b, b.iter().zip(bounds).any(|(a, c)| (a - c).abs() > 1e-9))
}

impl Operandi {
    /// GE-44: the box around the selected keys (frames, values), while the
    /// Box Scale tool is on and two or more keys differ in time and value.
    fn box_bounds(&self, shown: &[Shown]) -> Option<[f64; 4]> {
        if !self.graph.box_scale || self.graph.normalised || self.graph.stacked {
            return None;
        }
        let rate = self.editor.scene.time.rate;
        let selected = self.editor.live_selected_keys();
        let mut b = [f64::MAX, f64::MIN, f64::MAX, f64::MIN];
        let mut n = 0;
        for (k, t) in &selected {
            if !shown.iter().any(|s| s.key == *k) {
                continue;
            }
            let Some(c) = self.editor.scene.animation.channels.get(k) else {
                continue;
            };
            let Some(i) = c.curve.key_at(*t) else {
                continue;
            };
            let (f, v) = (animation::frames(rate, *t), c.curve.keys[i].value);
            b = [b[0].min(f), b[1].max(f), b[2].min(v), b[3].max(v)];
            n += 1;
        }
        (n >= 2 && b[1] - b[0] > 1e-9 && b[3] - b[2] > 1e-9).then_some(b)
    }

    /// GE-81 Speed: for each item with a shown position channel, its
    /// distance moved per frame (local space), drawn in cyan.
    fn draw_speed(&self, painter: &egui::Painter, m: &Map, shown: &[Shown]) {
        let rate = self.editor.scene.time.rate;
        let mut items: Vec<u64> = shown
            .iter()
            .filter(|s| s.key.transform_part().is_some_and(|(g, _)| g == "pos"))
            .map(|s| s.key.item)
            .collect();
        items.dedup();
        let anim = &self.editor.scene.animation;
        for id in items {
            let at = |f: f64| -> [f64; 3] {
                ["X", "Y", "Z"].map(|a| {
                    let k = ChannelKey::new(id, &format!("pos.{a}"));
                    match anim.channels.get(&k).filter(|c| !c.curve.is_empty()) {
                        Some(c) => c.curve.value_at(animation::at_frames(rate, f)),
                        None => animation::read(&self.editor.scene, &k).unwrap_or(0.),
                    }
                })
            };
            let pts: Vec<Pos2> = (0..=(m.rect.width() / 2.) as i32)
                .map(|px| {
                    let f = m.f(m.rect.left() + px as f32 * 2.);
                    let (a, b) = (at(f - 1.), at(f));
                    let d = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2))
                        .sqrt();
                    pos2(m.x(f), m.y(d))
                })
                .collect();
            painter.add(egui::Shape::line(pts, Stroke::new(1.2f32, theme::cyan())));
        }
    }
}
