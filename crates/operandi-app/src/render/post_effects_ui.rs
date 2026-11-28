//! The Render window's Image Processing tab: Bloom and Glare
//! (operandi_core::post_effects), and the processed copy of the shown
//! picture.
//!
//! The settings are the scene's (`RenderSettings::post`, saved with it);
//! each change is one "Render Settings" undo step, a drag one step for the
//! whole drag (UIC-53), and runs the same setter as `render_settings_set`
//! (UIC-160/165). The Recent Frames cache keeps the picture as Cycles
//! rendered it, so a change re-applies to the shown picture at once,
//! without rendering again. Only Final Color gets them: Depth, Normal and
//! the other data outputs stay true numbers.
use crate::Operandi;
use crate::form::Unit;
use crate::form::rows::{self, check_d, choice_d, number_keyed_d};
use crate::unavailable::Gate;
use eframe::egui::{self, Response, Ui};
use operandi_core::Command;
use operandi_core::post_effects::{
    self, Bloom, Glare, LENGTH_RANGE, MAX_INTENSITY, MAX_THRESHOLD, PostEffects, RADIUS_RANGE,
    STREAK_CHOICES,
};
use std::time::{Duration, Instant};

/// A render in progress shows its effects only while applying them takes
/// less than this; slower, they appear when it finishes.
const LIVE_BUDGET: Duration = Duration::from_millis(60);

/// Which picture a processed copy was made from.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Source {
    /// The render in progress, at its preview generation.
    Live { slot: u64, generation: u64 },
    /// The last render's picture in memory (its slot, if it became one).
    Picture(Option<u64>),
    /// A history slot read back from the cache.
    Slot(u64),
}

#[derive(Clone, Debug, PartialEq)]
struct Key {
    source: Source,
    width: u32,
    height: u32,
    settings: PostEffects,
}

/// The shown picture with Bloom and Glare applied.
#[derive(Default)]
pub(crate) struct PostCache {
    key: Option<Key>,
    rgba: Vec<f32>,
    /// How long the last apply took.
    cost: Duration,
    /// The render in progress is shown without its effects (too slow).
    live_skipped: bool,
    /// The undo gesture of a drag in the tab.
    gesture: Option<u64>,
}

impl Operandi {
    /// The picture the window shows, as it came from the renderer or the
    /// cache, and where it came from.
    fn post_source(&self) -> Option<Source> {
        if self.render.shown.is_some() {
            return None;
        }
        let v = &self.render.viewer;
        let picture = self.render.picture.as_ref();
        let shows_picture = match (v.primary, picture) {
            (None, _) => true,
            (Some(id), Some(p)) => p.slot == Some(id),
            (Some(_), None) => false,
        };
        if shows_picture {
            let p = picture?;
            Some(match self.render.live_generation() {
                Some((slot, generation)) => Source::Live { slot, generation },
                None => Source::Picture(p.slot),
            })
        } else {
            v.loaded.as_ref().map(|l| Source::Slot(l.id))
        }
    }

    /// The processed Final Color for the shown picture, when Bloom or
    /// Glare is on and the copy is up to date.
    pub(crate) fn post_pixels(&self) -> Option<&[f32]> {
        let key = self.render.post.key.as_ref()?;
        (Some(key.source) == self.post_source() && key.settings == self.editor.scene.render.post)
            .then_some(self.render.post.rgba.as_slice())
    }

    /// Brings the processed copy up to date with the shown picture and
    /// the scene's settings. Cheap when nothing changed; called each frame
    /// the Render window draws.
    pub(crate) fn refresh_post(&mut self) {
        let settings = &self.editor.scene.render.post;
        let source = self.post_source().filter(|_| settings.is_active());
        let Some(source) = source else {
            self.render.post.live_skipped = false;
            if self.render.post.key.take().is_some() {
                self.render.texture_stale = true;
            }
            return;
        };
        let Some((width, height, raw)) = self.shown_raw_pixels() else {
            return;
        };
        let key = Key {
            source,
            width,
            height,
            settings: settings.clone(),
        };
        if self.render.post.key.as_ref() == Some(&key) {
            return;
        }
        let live = matches!(source, Source::Live { .. });
        if live && self.render.post.cost > LIVE_BUDGET {
            // Shown as it renders; the finished picture gets the effects.
            self.render.post.live_skipped = true;
            if self.render.post.key.take().is_some() {
                self.render.texture_stale = true;
            }
            return;
        }
        let mut rgba = raw.to_vec();
        let start = Instant::now();
        post_effects::apply(&key.settings, &mut rgba, width, height);
        let post = &mut self.render.post;
        post.cost = start.elapsed();
        post.live_skipped = false;
        post.rgba = rgba;
        post.key = Some(key);
        self.render.texture_stale = true;
    }

    /// The Image Processing tab.
    pub(crate) fn image_processing_tab(&mut self, ui: &mut Ui) {
        let idle = self.editor.session.live.is_none();
        let before = self.editor.scene.render.clone();
        let mut s = before.clone();
        let defaults = PostEffects::default();
        let mut changed = None::<Response>;
        let mut note = |r: Response| {
            if r.changed() {
                changed = Some(match changed.take() {
                    Some(c) => c | r,
                    None => r,
                });
            }
        };
        let other_output = self.render.shown.is_some();
        let live_skipped = self.render.post.live_skipped;
        egui::ScrollArea::vertical()
            .id_salt("iv-processing")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.;
                rows::scope(ui, "iv-processing", |ui| {
                    // UIC-117: greyed with the reason while a tool is live.
                    ui.gated_ui(
                        crate::unavailable::unless(idle, crate::unavailable::TOOL_ACTIVE),
                        |ui| {
                            bloom_section(ui, &mut s.post.bloom, &defaults.bloom, &mut note);
                            glare_section(ui, &mut s.post.glare, &defaults.glare, &mut note);
                        },
                    );
                    if other_output {
                        rows::note(
                            ui,
                            "Bloom and Glare apply to Final Color. Other outputs keep \
                             their own numbers.",
                        );
                    } else if live_skipped {
                        rows::note(
                            ui,
                            "Shown without Bloom and Glare while it renders: they \
                             appear when the render finishes.",
                        );
                    }
                });
            });
        let gesture_state = &mut self.render.post.gesture;
        let Some(response) = changed else {
            if !ui.input(|i| i.pointer.any_down()) {
                *gesture_state = None;
            }
            return;
        };
        if s == before {
            return;
        }
        // One undo step per drag (UIC-53); a typed or picked value is one.
        let dragging = response.dragged() || ui.input(|i| i.pointer.any_down());
        let gesture = match *gesture_state {
            Some(g) if dragging => g,
            _ => crate::colour_field::next_gesture(),
        };
        *gesture_state = dragging.then_some(gesture);
        let previous = self.editor.history.undo_fold.replace(gesture);
        self.run(Command::SetRenderSettings(s));
        self.editor.history.undo_fold = previous;
        // The picture (drawn after this panel) shows the change this
        // frame, not on the next pointer move.
        self.refresh_post();
    }
}

/// A colour row (UIC-58, UIC-110) with its status circle against
/// `default`. The field shows and takes sRGB numbers; the value is linear.
fn tint_row(
    ui: &mut Ui,
    key: &str,
    value: &mut [f64; 3],
    default: [f64; 3],
    tip: &str,
) -> Response {
    rows::status(*value != default);
    let (rects, _) = rows::frame(ui, Some("Tint"), tip);
    let row = crate::form::Row::colour(key, "Tint", default).tip(tip);
    let mut v = crate::form::Value::Colour(*value);
    let mut response = rows::field(ui, rects.field, &row, &mut v, false);
    if let crate::form::Value::Colour(rgb) = v {
        *value = rgb;
    }
    if rows::take_reset() {
        *value = default;
        response.mark_changed();
    }
    response
}

/// A fraction shown as a percentage row.
#[allow(clippy::too_many_arguments)]
fn percent_row(
    ui: &mut Ui,
    key: &str,
    label: &str,
    value: &mut f64,
    default: f64,
    range: std::ops::RangeInclusive<f64>,
    tip: &str,
    note: &mut impl FnMut(Response),
) {
    let mut shown = *value * 100.;
    let r = number_keyed_d(
        ui,
        key,
        label,
        &mut shown,
        default * 100.,
        Unit::Percent,
        range.start() * 100.0..=range.end() * 100.,
        tip,
    );
    if r.changed() {
        *value = (shown / 100.).clamp(*range.start(), *range.end());
    }
    note(r);
}

fn bloom_section(ui: &mut Ui, b: &mut Bloom, d: &Bloom, note: &mut impl FnMut(Response)) {
    rows::section(ui, "Bloom", false, |ui| {
        // UIC-115: a checkbox row has no left label.
        note(check_d(
            ui,
            "Bloom",
            &mut b.enabled,
            d.enabled,
            "A soft glow around the brightest parts of the render, like light \
             spreading in a lens. Changes the picture, not the render",
        ));
        // UIC-117: greyed, not hidden, while Bloom is off.
        ui.gated_ui(
            crate::unavailable::unless(b.enabled, "Bloom is off"),
            |ui| {
                note(number_keyed_d(
                    ui,
                    "Bloom Threshold",
                    "Threshold",
                    &mut b.threshold,
                    d.threshold,
                    Unit::Scalar,
                    0.0..=MAX_THRESHOLD,
                    "Brightness where the glow starts. 1 is white; lights and \
                     sun glints are often far brighter",
                ));
                percent_row(
                    ui,
                    "Bloom Intensity",
                    "Intensity",
                    &mut b.intensity,
                    d.intensity,
                    0.0..=MAX_INTENSITY,
                    "How much of the light over the threshold spreads into the glow",
                    note,
                );
                percent_row(
                    ui,
                    "Bloom Radius",
                    "Radius",
                    &mut b.radius,
                    d.radius,
                    RADIUS_RANGE,
                    "How far the glow reaches, as a share of the picture's width, so \
                     it looks the same at any resolution",
                    note,
                );
                note(tint_row(
                    ui,
                    "Bloom Tint",
                    &mut b.tint,
                    d.tint,
                    "The glow's colour. White keeps the light's own colour",
                ));
            },
        );
    });
}

fn glare_section(ui: &mut Ui, g: &mut Glare, d: &Glare, note: &mut impl FnMut(Response)) {
    rows::section(ui, "Glare", false, |ui| {
        note(check_d(
            ui,
            "Glare",
            &mut g.enabled,
            d.enabled,
            "Straight streaks out of the brightest points, like a star filter. \
             Changes the picture, not the render",
        ));
        ui.gated_ui(
            crate::unavailable::unless(g.enabled, "Glare is off"),
            |ui| {
                note(number_keyed_d(
                    ui,
                    "Glare Threshold",
                    "Threshold",
                    &mut g.threshold,
                    d.threshold,
                    Unit::Scalar,
                    0.0..=MAX_THRESHOLD,
                    "Brightness where streaks start. 1 is white",
                ));
                percent_row(
                    ui,
                    "Glare Intensity",
                    "Intensity",
                    &mut g.intensity,
                    d.intensity,
                    0.0..=MAX_INTENSITY,
                    "How much of the light over the threshold goes into the streaks",
                    note,
                );
                let labels: Vec<String> = STREAK_CHOICES.iter().map(|n| n.to_string()).collect();
                let options: Vec<(u32, &str)> = STREAK_CHOICES
                    .iter()
                    .zip(&labels)
                    .map(|(n, l)| (*n, l.as_str()))
                    .collect();
                note(choice_d(
                    ui,
                    "Streaks",
                    &mut g.streaks,
                    &d.streaks,
                    &options,
                    "Rays around each bright point: 2 is one line through it, 4 a cross, \
                     6 and 8 stars",
                ));
                note(number_keyed_d(
                    ui,
                    "Glare Angle",
                    "Angle",
                    &mut g.angle,
                    d.angle,
                    Unit::Angle,
                    -360.0..=360.0,
                    "Turns the streaks. 0 ° puts the first ray to the right",
                ));
                percent_row(
                    ui,
                    "Glare Length",
                    "Length",
                    &mut g.length,
                    d.length,
                    LENGTH_RANGE,
                    "How far the streaks reach, as a share of the picture's width",
                    note,
                );
                note(tint_row(
                    ui,
                    "Glare Tint",
                    &mut g.tint,
                    d.tint,
                    "The streaks' colour. White keeps the light's own colour",
                ));
            },
        );
    });
}
