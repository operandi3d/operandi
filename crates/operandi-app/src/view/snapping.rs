//! Snapping (specifications/snapping-2026-11-05, SN-01..SN-14): one on/off
//! switch (X; hold X to flip it for one drag), targets ticked per mode
//! (Global, Component, Items) and the Inner/Outer Range in screen pixels.
//! Vertex and Grid targets are built; the other target types are listed
//! greyed. Settings are saved per user in `snapping-preferences.json`.
//!
//! What snaps is the tool handle (SN-08): while a Move drag runs, the
//! handle's would-be screen position looks for the nearest active target.
//! Within Outer Range the target is pre-highlighted; within Inner Range the
//! handle jumps onto it in 3D and the selection moves by the same offset.
//! Typed values never snap (SN-03): only viewport drags come through here.
use crate::prefs_file::PrefsFile;
use crate::snap_kinds::Kind;
use crate::unavailable::Gate;
use eframe::egui::{self, Pos2};
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// Which target types are ticked in one mode (SN-05; the types are
/// `snap_kinds::Kind`, and the snap service looks for each one).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Targets {
    pub grid: bool,
    pub vertex: bool,
    pub edge: bool,
    pub edge_center: bool,
    pub polygon: bool,
    pub polygon_center: bool,
    pub intersection: bool,
    #[serde(rename = "box")]
    pub bounding_box: bool,
    pub pivot: bool,
    pub world_axis: bool,
    pub straight_line: bool,
    pub right_angle: bool,
}
impl Targets {
    /// Nothing ticked.
    #[cfg_attr(not(test), allow(dead_code))]
    pub const NONE: Self = Self {
        grid: false,
        vertex: false,
        edge: false,
        edge_center: false,
        polygon: false,
        polygon_center: false,
        intersection: false,
        bounding_box: false,
        pivot: false,
        world_axis: false,
        straight_line: false,
        right_angle: false,
    };
    /// Mesh points only: what Draw's join, the Ruler and Weld look for.
    pub fn vertex_only() -> Self {
        Self {
            vertex: true,
            ..Self::default()
        }
    }
    pub fn get(self, kind: Kind) -> bool {
        match kind {
            Kind::Grid => self.grid,
            Kind::Vertex => self.vertex,
            Kind::Edge => self.edge,
            Kind::EdgeCenter => self.edge_center,
            Kind::Polygon => self.polygon,
            Kind::PolygonCenter => self.polygon_center,
            Kind::Intersection => self.intersection,
            Kind::Box => self.bounding_box,
            Kind::Pivot => self.pivot,
            Kind::WorldAxis => self.world_axis,
            Kind::StraightLine => self.straight_line,
            Kind::RightAngle => self.right_angle,
        }
    }
    pub fn flag(&mut self, kind: Kind) -> &mut bool {
        match kind {
            Kind::Grid => &mut self.grid,
            Kind::Vertex => &mut self.vertex,
            Kind::Edge => &mut self.edge,
            Kind::EdgeCenter => &mut self.edge_center,
            Kind::Polygon => &mut self.polygon,
            Kind::PolygonCenter => &mut self.polygon_center,
            Kind::Intersection => &mut self.intersection,
            Kind::Box => &mut self.bounding_box,
            Kind::Pivot => &mut self.pivot,
            Kind::WorldAxis => &mut self.world_axis,
            Kind::StraightLine => &mut self.straight_line,
            Kind::RightAngle => &mut self.right_angle,
        }
    }
    pub fn any(self) -> bool {
        Kind::ALL.iter().any(|&k| self.get(k))
    }
    /// Any target the snap service looks for (all but Grid).
    pub fn any_service(self) -> bool {
        Kind::ALL.iter().any(|&k| k != Kind::Grid && self.get(k))
    }
    pub fn union(self, other: Self) -> Self {
        let mut out = self;
        for k in Kind::ALL {
            *out.flag(k) |= other.get(k);
        }
        out
    }
    /// Short list for the status overlay.
    pub fn names(self) -> String {
        let names: Vec<&str> = Kind::ALL
            .iter()
            .filter(|&&k| self.get(k))
            .map(|k| k.label())
            .collect();
        if names.is_empty() {
            "no targets".into()
        } else {
            names.join(", ")
        }
    }
}

/// The modes whose ticks the Mode dropdown edits (SN-05).
const MODES: [&str; 3] = ["Global", "Component", "Items"];

pub const RANGE_PX: std::ops::RangeInclusive<f32> = 1.0..=200.0;
pub const GRID_DISTANCE: std::ops::RangeInclusive<f32> = 0.01..=1.0;
/// Fixed grid size and Depth, metres.
pub const LENGTH: std::ops::RangeInclusive<f64> = 1e-6..=1e6;
/// Symmetry's seam snap distance (symmetry-2026-11-13 SYM3-23, Q3).
pub const SYMMETRY_SEAM_PX: std::ops::RangeInclusive<f32> = 1.0..=60.0;

/// SN2-18: which meshes give geometry candidates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layers {
    /// Selected visible meshes.
    Active,
    /// Visible meshes that aren't selected.
    Background,
    /// Every visible mesh (Operandi choice default, Q-02).
    #[default]
    Both,
}
impl Layers {
    /// SN2-18's order.
    pub const ALL: [Self; 3] = [Self::Background, Self::Active, Self::Both];
    pub fn label(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Background => "Background",
            Self::Both => "Both",
        }
    }
}

/// SN2-12: one Mode's presets. The list always starts with Default;
/// `current` is the one selected, and tick changes go into it when the
/// popover closes (Default included, Operandi choice Q-03). Presets hold
/// ticks only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModePresets {
    pub current: String,
    /// (name, ticks), Default first.
    pub list: Vec<(String, Targets)>,
}
impl ModePresets {
    fn new(default: Targets) -> Self {
        Self {
            current: DEFAULT_PRESET.into(),
            list: vec![(DEFAULT_PRESET.into(), default)],
        }
    }
}
impl Default for ModePresets {
    fn default() -> Self {
        Self::new(Targets::default())
    }
}

/// The preset every Mode starts with.
pub const DEFAULT_PRESET: &str = "Default";

/// The Preset row's two list actions while one is under way (SN2-12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetStep {
    New,
    Remove,
}

/// The settings, saved per user (SN2-05, defaults SN2-10). Coordinate
/// Rounding and Fixed Increment are the Units preferences' and Snap Angle
/// is Tool Handles' Rotation Snap Angle: one setting each, shown here
/// too (SN2-39/40).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SnappingPreferences {
    pub(crate) version: u8,
    /// SN-01: the switch, off on a fresh install.
    pub enabled: bool,
    pub global: Targets,
    pub component: Targets,
    pub items: Targets,
    /// Snap distance in screen pixels.
    pub inner_range: f32,
    /// Pre-highlight distance in screen pixels.
    pub outer_range: f32,
    /// Grid options (SN2-24): fraction of a grid square (1 = 100 %)
    /// around a crossing that snaps.
    pub grid_distance: f32,
    /// SN2-23: crossings every `grid_size` from the world origin instead
    /// of the grid the view shows.
    pub use_fixed_grid: bool,
    pub grid_size: f64,
    /// Geometry options (SN2-30).
    pub layers: Layers,
    pub only_selected: bool,
    pub depth_limit: bool,
    /// Depth Limitation's distance from the camera, metres.
    pub depth: f64,
    /// SN2-30 Pre Highlight: show the pre-highlight marker (snapping works
    /// either way).
    pub pre_highlight: bool,
    /// SN2-30b Constraint Snapping: shown, on by default; axis-arrow drags
    /// snap along their axis either way (Operandi choice, Q-14).
    pub constraint_snapping: bool,
    /// Pivot options (SN2-33): Inner Range for pivots, screen pixels.
    pub pivot_snap_size: f32,
    /// SN2-12: each Mode's presets (Global, Component, Items).
    pub presets: [ModePresets; 3],
    /// A pinned popover's place, so it comes back next launch (UIC-64).
    pub pinned_at: Option<[f32; 2]>,
    /// SYM3-23: a point placed this close (screen pixels) to the symmetry
    /// plane lands on it. Not gated by `enabled`: it is part of Symmetry.
    pub symmetry_seam: f32,
}
impl Default for SnappingPreferences {
    fn default() -> Self {
        Self {
            version: 1,
            enabled: false,
            global: Targets::vertex_only(),
            component: Targets::default(),
            items: Targets::default(),
            inner_range: 24.,
            outer_range: 40.,
            grid_distance: 1.,
            use_fixed_grid: false,
            grid_size: 0.2,
            layers: Layers::Both,
            only_selected: false,
            depth_limit: false,
            depth: 0.,
            pre_highlight: true,
            constraint_snapping: true,
            pivot_snap_size: 15.,
            presets: [
                ModePresets::new(Targets::vertex_only()),
                ModePresets::default(),
                ModePresets::default(),
            ],
            pinned_at: None,
            symmetry_seam: 8.,
        }
    }
}
impl SnappingPreferences {
    /// SN-05: Global ticks always apply; Component ticks add to them in
    /// Vertex/Edge/Polygon modes, Items ticks in Items mode.
    pub fn active(&self, mode: operandi_core::Mode) -> Targets {
        self.global.union(if mode == operandi_core::Mode::Item {
            self.items
        } else {
            self.component
        })
    }
    /// The ticks Mode `mode` edits (0 Global, 1 Component, 2 Items).
    pub fn ticks_mut(&mut self, mode: usize) -> &mut Targets {
        match mode {
            1 => &mut self.component,
            2 => &mut self.items,
            _ => &mut self.global,
        }
    }
    /// SN2-12: switch Mode `mode`'s ticks to its preset `name` (the
    /// current ticks go into the preset left first).
    pub fn select_preset(&mut self, mode: usize, name: &str) {
        let mode = mode.min(2);
        self.store_ticks_in_preset(mode);
        let Some(ticks) = self.presets[mode]
            .list
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, t)| *t)
        else {
            return;
        };
        self.presets[mode].current = name.to_owned();
        *self.ticks_mut(mode) = ticks;
    }
    /// SN2-12 (New Preset): Mode `mode`'s current ticks under a new,
    /// unique name, selected. Err says why not.
    pub fn new_preset(&mut self, mode: usize, name: &str) -> Result<(), String> {
        let mode = mode.min(2);
        let name = name.trim();
        if name.is_empty() {
            return Err("Type a name for the preset".into());
        }
        if self.presets[mode].list.iter().any(|(n, _)| n == name) {
            return Err(format!("A preset named {name} already exists"));
        }
        // The current ticks become the new preset's (the one left keeps
        // what it had).
        let ticks = *self.ticks_mut(mode);
        let presets = &mut self.presets[mode];
        presets.list.push((name.to_owned(), ticks));
        presets.current = name.to_owned();
        Ok(())
    }
    /// SN2-12 (Remove Preset). Default can't be removed; removing the
    /// selected one selects Default and keeps the current ticks.
    pub fn remove_preset(&mut self, mode: usize, name: &str) {
        let presets = &mut self.presets[mode.min(2)];
        if name == DEFAULT_PRESET {
            return;
        }
        presets.list.retain(|(n, _)| n != name);
        if presets.current == name {
            presets.current = DEFAULT_PRESET.into();
        }
    }
    /// SN2-12: Mode `mode`'s ticks go into its selected preset (when the
    /// popover closes, and before another preset is picked).
    pub fn store_ticks_in_preset(&mut self, mode: usize) {
        let mode = mode.min(2);
        let ticks = *self.ticks_mut(mode);
        let presets = &mut self.presets[mode];
        if let Some((_, t)) = presets.list.iter_mut().find(|(n, _)| *n == presets.current) {
            *t = ticks;
        }
    }
    /// All three Modes' ticks into their selected presets.
    pub fn store_all_ticks(&mut self) {
        for mode in 0..3 {
            self.store_ticks_in_preset(mode);
        }
    }
    /// Inner and Outer Range rows, for Preferences > Tool Handles (the
    /// same values as Edit > Snapping > Snapping Options).
    pub(crate) fn range_rows(&mut self, ui: &mut egui::Ui) {
        use crate::form::{Unit, rows};
        let d = Self::default();
        rows::number_d(
            ui,
            "Inner Range",
            &mut self.inner_range,
            d.inner_range,
            Unit::Suffix(" px"),
            RANGE_PX,
            "A snapping handle jumps onto a target this close on screen",
        );
        rows::number_d(
            ui,
            "Outer Range",
            &mut self.outer_range,
            d.outer_range,
            Unit::Suffix(" px"),
            RANGE_PX,
            "Targets this close are highlighted",
        );
        rows::number_d(
            ui,
            "Symmetry Seam Snap",
            &mut self.symmetry_seam,
            d.symmetry_seam,
            Unit::Suffix(" px"),
            SYMMETRY_SEAM_PX,
            "With Symmetry on, a point you place this close to the centre line lands exactly on it",
        );
    }
    /// The outer range is never below the inner one.
    pub fn outer(&self) -> f32 {
        self.outer_range.max(self.inner_range)
    }
}
impl crate::prefs_file::PrefsFile for SnappingPreferences {
    /// Presets make the file bigger than most preference files.
    const MAX_BYTES: u64 = 1 << 20;
    const INVALID: &'static str = "Invalid snapping preference";
    fn valid(&self) -> bool {
        self.version == 1
            && RANGE_PX.contains(&self.inner_range)
            && RANGE_PX.contains(&self.outer_range)
            && RANGE_PX.contains(&self.pivot_snap_size)
            && GRID_DISTANCE.contains(&self.grid_distance)
            && LENGTH.contains(&self.grid_size)
            && (0. ..=1e6).contains(&self.depth)
            && self.presets.iter().all(|p| {
                p.list.first().is_some_and(|(n, _)| n == DEFAULT_PRESET)
                    && p.list.iter().any(|(n, _)| *n == p.current)
            })
            && SYMMETRY_SEAM_PX.contains(&self.symmetry_seam)
    }
}

/// A target found near the handle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub point: DVec3,
    pub screen: Pos2,
    pub distance: f32,
}

/// The nearest of `candidates` (world point, screen position) to `handle`
/// within `range` pixels.
#[cfg(test)]
pub fn nearest(
    candidates: impl IntoIterator<Item = (DVec3, Pos2)>,
    handle: Pos2,
    range: f32,
) -> Option<Hit> {
    candidates
        .into_iter()
        .map(|(point, screen)| Hit {
            point,
            screen,
            distance: screen.distance(handle),
        })
        .filter(|h| h.distance <= range)
        .min_by(|a, b| a.distance.total_cmp(&b.distance))
}

/// SN-11: the grid crossing nearest `point` on the work plane `plane`
/// (0 = XZ, 1 = YZ, 2 = XY) through `center`, with squares of `step`. The
/// height off the plane is kept. With `distance` below 1 the point must lie
/// within that fraction of half a square of the crossing on both axes
/// (Operandi choice); at 100 % every point snaps.
pub fn grid_crossing(
    point: DVec3,
    plane: u8,
    center: DVec3,
    step: f64,
    distance: f32,
) -> Option<DVec3> {
    if !(step.is_finite() && step > 0.) {
        return None;
    }
    let axes: [usize; 2] = match plane {
        1 => [1, 2],
        2 => [0, 1],
        _ => [0, 2],
    };
    let mut out = point;
    for a in axes {
        let local = point[a] - center[a];
        let snapped = (local / step).round() * step;
        if (local - snapped).abs() > distance as f64 * step / 2. + 1e-12 {
            return None;
        }
        out[a] = center[a] + snapped;
    }
    Some(out)
}

/// An axis-arrow drag (SN2-30b): which frame axis it is held to, the
/// move when it started, and how far the pointer has moved since the
/// press.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisDrag {
    pub axis: usize,
    pub base_delta: DVec3,
    pub moved: egui::Vec2,
}

/// Hold X to flip snapping for one drag (SN-02): the toggle action flips
/// the switch at once; if a drag runs while the key is still down, letting
/// the key go flips it back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hold {
    pub key: egui::Key,
    pub dragged: bool,
}

/// The vertex targets of one drag or placement: every shown mesh point
/// when it started, and the surface that hides them, pinned until the
/// gesture ends (resetting this to its default starts a new one). The
/// shared [`SnapService`](crate::snap_service::SnapService) does the
/// gathering and the screen-space search.
#[derive(Clone, Default)]
pub struct Session {
    pub targets: Option<(
        std::rc::Rc<crate::snap_service::WorldTargets>,
        std::sync::Arc<crate::picking::Surface>,
    )>,
    /// Points of the active item that the tool moves.
    pub moved: std::collections::BTreeSet<u32>,
}

impl super::Operandi {
    /// SN-01: flip the switch (button, X, Edit menu). Not an undo step.
    pub(super) fn toggle_snapping(&mut self) {
        self.snapping.enabled = !self.snapping.enabled;
        self.save_snapping();
        self.status = format!(
            "Snapping {}",
            if self.snapping.enabled { "on" } else { "off" }
        );
    }
    /// The X action: toggle now, and remember the key so holding it through
    /// a drag flips back on release.
    pub(super) fn snapping_key(&mut self) {
        self.toggle_snapping();
        let key = self
            .bindings
            .get("Snapping Toggle")
            .and_then(|b| crate::input::chord(b))
            .map(|(_, k)| k);
        self.snap_hold = key.map(|key| Hold {
            key,
            dragged: false,
        });
    }
    /// Once a frame: end a hold when its key is up.
    pub(super) fn update_snap_hold(&mut self, ctx: &egui::Context) {
        let Some(hold) = self.snap_hold else {
            return;
        };
        if ctx.input(|i| i.key_down(hold.key)) {
            return;
        }
        self.snap_hold = None;
        if hold.dragged {
            self.snapping.enabled = !self.snapping.enabled;
            self.save_snapping();
            self.status = format!(
                "Snapping {} again",
                if self.snapping.enabled { "on" } else { "off" }
            );
        }
    }
    pub(super) fn save_snapping(&mut self) {
        if let Err(error) = self
            .snapping
            .save(&self.environment.paths().snapping_preferences())
        {
            self.status = format!("Snapping changed; could not save it: {error}");
        }
    }
    /// The active targets now, or none when snapping is off.
    pub(super) fn snap_targets(&self) -> Targets {
        if self.snapping.enabled {
            self.snapping.active(self.editor.scene.selection.mode)
        } else {
            Targets::default()
        }
    }
    /// SN2-04/06/07: the Snapping popover, in the app at the pointer
    /// (F11, Edit > Snapping > Snapping Options, or the Snapping button
    /// with Option held). It closes when the pointer leaves it (UIC-62);
    /// the circle pins it (UIC-64), and a pinned one comes back next
    /// launch. Rows in the spec's order; settings, not channels, so no
    /// status lights (UIC-112).
    pub(super) fn snapping_window(&mut self, ctx: &egui::Context) {
        let Some(at) = self.snapping_options else {
            return;
        };
        let before = (
            self.snapping.clone(),
            self.units_preferences,
            self.handle_preferences,
        );
        let state = PopoverState {
            at,
            pinned: self.snapping_pinned,
            entered: self.snapping_entered,
            opening: std::mem::take(&mut self.snapping_opening),
        };
        let fallback = self.viewport_rect.left_top() + egui::vec2(40., 40.);
        let click_off = self.ui_preferences.click_off();
        let shown = show_popover(ctx, "snapping-popover", state, fallback, click_off, |ui| {
            self.snapping_popover_ui(ui)
        });
        match shown {
            None => self.close_snapping_options(),
            Some(state) => {
                self.snapping_options = Some(state.at);
                self.snapping_pinned = state.pinned;
                self.snapping_entered = state.entered;
                if state.pinned {
                    self.snapping.pinned_at = Some([state.at.x, state.at.y]);
                }
            }
        }
        self.save_snapping_popover(before);
    }

    /// Save whichever of the shown settings changed: snapping's own, and
    /// the Units (rounding) and Tool Handles (Snap Angle) settings it
    /// shares.
    fn save_snapping_popover(
        &mut self,
        before: (
            SnappingPreferences,
            crate::units_preferences::UnitsPreferences,
            crate::handle_preferences::HandlePreferences,
        ),
    ) {
        if self.snapping != before.0 {
            self.save_snapping();
        }
        self.save_units(before.1);
        if self.handle_preferences != before.2
            && let Err(error) = self
                .handle_preferences
                .save(&self.environment.paths().handle_preferences())
        {
            self.status = format!("Preference changed; could not save it: {error}");
        }
    }

    /// Close the popover; with a preset selected, the tick changes go
    /// into it (SN2-12).
    pub(super) fn close_snapping_options(&mut self) {
        self.snapping.store_all_ticks();
        self.snapping_preset_step = None;
        self.snapping_options = None;
        self.snapping_pinned = false;
        self.snapping_entered = false;
        self.snapping.pinned_at = None;
        self.snapping_mode = 0;
        self.save_snapping();
    }

    /// The popover's rows (SN2-07, SN2-08).
    /// Returns what the header strip asked for.
    fn snapping_popover_ui(&mut self, ui: &mut egui::Ui) -> HeaderAction {
        use crate::form::{Unit, rows};
        ui.set_width(300.);
        ui.spacing_mut().item_spacing.y = 2.;
        // UIC-181: a popover has no title bar; the Snapping button below
        // names it. Only the pin circle sits at the top right (UIC-64). The
        // generic panel Presets ▾ menu (UIC-103) isn't built for popovers
        // yet, so it is left out rather than shown with nothing behind it.
        let action = popover_header(ui, self.snapping_pinned);
        // The on/off bar: orange while on (SN2-01).
        let key = self
            .bindings
            .get("Snapping Toggle")
            .filter(|b| !b.is_empty())
            .map_or("Unbound in the Input Map".to_owned(), |b| {
                format!(
                    "{b} in the viewport; hold it while dragging to flip snapping for that drag"
                )
            });
        let s = &mut self.snapping;
        let (fill, text) = if s.enabled {
            (crate::theme::orange(), crate::theme::on_orange())
        } else {
            (crate::theme::control(), crate::theme::text())
        };
        let bar = egui::Button::new(egui::RichText::new("Snapping").color(text).strong())
            .fill(fill)
            .min_size(egui::vec2(ui.available_width(), 24.));
        if ui.add(bar).on_hover_text(key).clicked() {
            s.enabled = !s.enabled;
        }
        ui.add_space(2.);
        let d = SnappingPreferences::default();
        let mut mode = self.snapping_mode.min(2);
        let mut step = self.snapping_preset_step;
        let naming = &mut self.snapping_naming;
        let mut note: Option<String> = None;
        let units = &mut self.units_preferences;
        let handles = &mut self.handle_preferences;
        let text_editors = &mut self.text_editors;
        rows::plain(|| {
            rows::scope(ui, "snapping-options", |ui| {
                let s = &mut self.snapping;
                rows::section(ui, "Snap Types", false, |ui| {
                    let modes: Vec<(usize, &str)> =
                        MODES.iter().enumerate().map(|(k, n)| (k, *n)).collect();
                    let before = mode;
                    rows::choice_d(
                        ui,
                        "Mode",
                        &mut mode,
                        &0,
                        &modes,
                        "Global ticks apply in every selection mode; Component ticks add to \
                         them in Vertex, Edge and Polygon modes, Items ticks in Items mode",
                    );
                    if mode != before {
                        step = None;
                    }
                    // SN2-12: this Mode's presets, then the two list actions.
                    const NEW: &str = "(New Preset)";
                    const REMOVE: &str = "(Remove Preset)";
                    let names: Vec<String> = s.presets[mode]
                        .list
                        .iter()
                        .map(|(n, _)| n.clone())
                        .collect();
                    let mut options: Vec<(String, &str)> =
                        names.iter().map(|n| (n.clone(), n.as_str())).collect();
                    options.push((NEW.into(), NEW));
                    options.push((REMOVE.into(), REMOVE));
                    let mut picked = s.presets[mode].current.clone();
                    rows::choice_d(
                        ui,
                        "Preset",
                        &mut picked,
                        &DEFAULT_PRESET.to_owned(),
                        &options,
                        "This Mode's ticks, saved under a name",
                    );
                    match picked.as_str() {
                        NEW => step = Some(PresetStep::New),
                        REMOVE => step = Some(PresetStep::Remove),
                        name if name != s.presets[mode].current => {
                            step = None;
                            s.select_preset(mode, name);
                        }
                        _ => {}
                    }
                    match step {
                        Some(PresetStep::New) => {
                            let mut add = false;
                            rows::row(ui, "Name", |ui| {
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        let edit = ui.add(
                                            egui::TextEdit::singleline(naming).desired_width(120.),
                                        );
                                        text_editors.insert(edit.id);
                                        crate::numeric_tab::register(
                                            ui,
                                            &edit,
                                            Some(naming.as_str()),
                                        );
                                        add = ui.button("Add").clicked()
                                            || (edit.lost_focus()
                                                && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                                    },
                                );
                            });
                            if add {
                                match s.new_preset(mode, naming) {
                                    Ok(()) => {
                                        naming.clear();
                                        step = None;
                                    }
                                    Err(e) => note = Some(e),
                                }
                            }
                        }
                        Some(PresetStep::Remove) => {
                            let removable: Vec<&String> =
                                names.iter().filter(|n| *n != DEFAULT_PRESET).collect();
                            rows::row(ui, "Remove", |ui| {
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        if removable.is_empty() {
                                            ui.label("No presets to remove");
                                        }
                                        for name in removable {
                                            if ui.button(name.as_str()).clicked() {
                                                s.remove_preset(mode, name);
                                                step = None;
                                            }
                                        }
                                    },
                                );
                            });
                        }
                        None => {}
                    }
                    let defaults = match mode {
                        1 => d.component,
                        2 => d.items,
                        _ => d.global,
                    };
                    let targets = s.ticks_mut(mode);
                    for kind in crate::snap_kinds::Kind::ALL {
                        rows::check_d(
                            ui,
                            kind.label(),
                            targets.flag(kind),
                            defaults.get(kind),
                            kind.tip(),
                        );
                    }
                });
                // SN2-08: each type's options, while it is active (ticked in
                // Global or in the Mode on show). Groups appearing departs from
                // UIC-92; Jason approved it (2026-11-14).
                let shown = s.global.union(*s.ticks_mut(mode));
                use crate::snap_kinds::Kind;
                if shown.grid {
                    rows::section(ui, "Grid Options", false, |ui| {
                        rows::check_d(
                            ui,
                            "Use Fixed Grid",
                            &mut s.use_fixed_grid,
                            d.use_fixed_grid,
                            "Crossings every Grid Size from the world origin, instead of the \
                             grid the view shows",
                        );
                        ui.gated_ui(
                            crate::unavailable::unless(s.use_fixed_grid, "Turn on Use Fixed Grid"),
                            |ui| {
                                rows::number_d(
                                    ui,
                                    "Grid Size",
                                    &mut s.grid_size,
                                    d.grid_size,
                                    Unit::Distance,
                                    LENGTH,
                                    "The fixed grid's square size",
                                );
                            },
                        );
                        rows::number_d(
                            ui,
                            "Distance",
                            &mut s.grid_distance,
                            d.grid_distance,
                            Unit::Factor,
                            GRID_DISTANCE,
                            "100 % snaps to the nearest grid crossing everywhere; less snaps \
                             only near a crossing",
                        );
                    });
                }
                let geometry = [
                    Kind::Vertex,
                    Kind::Edge,
                    Kind::EdgeCenter,
                    Kind::Polygon,
                    Kind::PolygonCenter,
                    Kind::Intersection,
                    Kind::Box,
                ];
                if geometry.iter().any(|&k| shown.get(k)) {
                    rows::section(ui, "Geometry Options", false, |ui| {
                        let layers: Vec<(Layers, &str)> =
                            Layers::ALL.iter().map(|l| (*l, l.label())).collect();
                        rows::choice_d(
                            ui,
                            "Layers",
                            &mut s.layers,
                            &d.layers,
                            &layers,
                            "Which meshes give targets: Active (selected), Background (not \
                             selected) or Both",
                        );
                        rows::check_d(
                            ui,
                            "Pre Highlight",
                            &mut s.pre_highlight,
                            d.pre_highlight,
                            "Mark the target a handle would snap to before it snaps",
                        );
                        rows::check_d(
                            ui,
                            "Only Selected",
                            &mut s.only_selected,
                            d.only_selected,
                            "Only the selected polygons of the active mesh, with their edges \
                             and vertices, give targets",
                        );
                        rows::check_d(
                            ui,
                            "Depth Limitation",
                            &mut s.depth_limit,
                            d.depth_limit,
                            "Ignore targets farther from the camera than Depth",
                        );
                        ui.gated_ui(
                            crate::unavailable::unless(s.depth_limit, "Turn on Depth Limitation"),
                            |ui| {
                                rows::number_d(
                                    ui,
                                    "Depth",
                                    &mut s.depth,
                                    d.depth,
                                    Unit::Distance,
                                    0.0..=1e6,
                                    "How far from the camera targets still count",
                                );
                            },
                        );
                        rows::check_d(
                            ui,
                            "Constraint Snapping",
                            &mut s.constraint_snapping,
                            d.constraint_snapping,
                            "On an axis arrow, the handle takes the target's position along the \
                             axis (it works the same either way for now)",
                        );
                    });
                }
                if shown.pivot {
                    rows::section(ui, "Pivot Options", false, |ui| {
                        rows::number_d(
                            ui,
                            "Snap Size",
                            &mut s.pivot_snap_size,
                            d.pivot_snap_size,
                            Unit::Count,
                            RANGE_PX,
                            "How close the handle must be to snap to a pivot",
                        );
                        s.pivot_snap_size = s.pivot_snap_size.round();
                    });
                }
                ui.separator();
                rows::number_d(
                    ui,
                    "Inner Range (pixels)",
                    &mut s.inner_range,
                    d.inner_range,
                    Unit::Count,
                    RANGE_PX,
                    "A snapping handle jumps onto a target this close on screen",
                );
                s.inner_range = s.inner_range.round();
                rows::number_d(
                    ui,
                    "Outer Range (pixels)",
                    &mut s.outer_range,
                    d.outer_range,
                    Unit::Count,
                    RANGE_PX,
                    "Targets this close are highlighted",
                );
                s.outer_range = s.outer_range.round();
                ui.separator();
                let factory = crate::units_preferences::UnitsPreferences::default();
                let roundings: Vec<(crate::units_preferences::Rounding, &str)> =
                    crate::units_preferences::Rounding::ALL
                        .iter()
                        .map(|r| (*r, r.label()))
                        .collect();
                rows::choice_d(
                    ui,
                    "Coordinate Rounding",
                    &mut units.rounding,
                    &factory.rounding,
                    &roundings,
                    "How a pointer position on the work plane becomes a coordinate when \
                     nothing snaps (the same setting as Preferences > Units)",
                );
                use crate::units_preferences::Rounding;
                let fixed = matches!(units.rounding, Rounding::Fixed | Rounding::ForcedFixed);
                // UIC-92/93: greyed in place unless rounding is Fixed.
                ui.gated_ui(
                    crate::unavailable::unless(
                        fixed,
                        "Set Coordinate Rounding to Fixed or Force Fixed",
                    ),
                    |ui| {
                        rows::number_d(
                            ui,
                            "Fixed Increment",
                            &mut units.fixed_increment,
                            factory.fixed_increment,
                            Unit::Distance,
                            1e-6..=1e6,
                            "The rounding step with Fixed and Force Fixed rounding (the same \
                             setting as Preferences > Units)",
                        );
                    },
                );
                let hd = crate::handle_preferences::HandlePreferences::default();
                rows::number_d(
                    ui,
                    "Snap Angle",
                    &mut handles.rotation_snap,
                    hd.rotation_snap,
                    Unit::Angle,
                    crate::handle_preferences::SNAP_ANGLE_RANGE,
                    "The step Rotate's rings jump by when constrained (the same setting as \
                     Rotation Snap Angle)",
                );
            });
        });
        self.snapping_mode = mode;
        self.snapping_preset_step = step;
        if let Some(note) = note {
            self.status = note;
        }
        action
    }

    /// SN2-04: pressed again while open (and not pinned), it closes.
    pub(super) fn open_snapping_options(&mut self) {
        if self.snapping_options.is_some() && !self.snapping_pinned {
            self.close_snapping_options();
            return;
        }
        // UIC-181: one unpinned popover at a time.
        if self.corner.properties.view.is_some() && !self.corner.properties.pinned {
            self.close_view_properties();
        }
        if self.mesh_constraint_popover.is_some() && !self.mesh_constraint_pinned {
            self.close_mesh_constraint_popover();
        }
        // Placed under the pointer on the next frame (`snapping_window`).
        self.snapping_options = Some(Pos2::new(f32::NAN, f32::NAN));
        self.snapping_entered = false;
    }
    /// A popover pinned when the app last quit comes back (UIC-64).
    pub(super) fn restore_snapping_popover(&mut self) {
        if let Some([x, y]) = self.snapping.pinned_at {
            self.snapping_options = Some(Pos2::new(x, y));
            self.snapping_pinned = true;
        }
    }
    /// SN-08..SN-12: snap a Move (or Transform's move) drag. `params` is
    /// the unsnapped sample; returns it with the handle moved onto the
    /// nearest active target in range, and sets the pre-highlight marker.
    /// `line` is the action frame axis an axis-arrow drag is held to
    /// (SN2-30b): with Constraint Snapping on the handle stays on that line
    /// at the point nearest the target; off, it doesn't snap to geometry.
    pub(super) fn snap_move(
        &mut self,
        tool: operandi_core::Tool,
        mut params: operandi_core::Parameters,
        line: Option<AxisDrag>,
        r: egui::Rect,
    ) -> operandi_core::Parameters {
        // The snapping layer: only tools whose card snaps their handle.
        if tool.card().layers.snap != operandi_core::tool_card::Snap::Handle {
            self.snap_marker = None;
            self.snap_reticle = None;
            return params;
        }
        // The handle's own frame, so a unit of `delta` here is a unit of
        // travel along the drawn arrow: under Local that's the cluster's
        // frame, not the whole selection's action frame (AC-75).
        let Some(frame) = self.transform_handle_frame() else {
            self.snap_marker = None;
            self.snap_reticle = None;
            return params;
        };
        let handle = frame.center + frame.basis * params.delta;
        // SN2-30b/43b: on an axis arrow the reticle roams off the line,
        // following the pointer from where the handle started, and the
        // handle takes the target's position along the line. Constraint
        // Snapping on or off, it works the same way (Operandi choice, Q-14).
        let hunt = match line {
            Some(drag) => {
                let start = frame.center + frame.basis * drag.base_delta;
                self.camera
                    .project(start, r)
                    .and_then(|(q, _)| view_plane_point(&self.camera, start, q + drag.moved, r))
                    .unwrap_or(handle)
            }
            None => handle,
        };
        if let Some(point) = self.snap_point(hunt, None, r) {
            let local = frame.basis.inverse() * (point - handle);
            if local.is_finite() {
                match line {
                    Some(drag) => params.delta[drag.axis] += local[drag.axis],
                    None => params.delta += local,
                }
            }
        }
        params
    }
    /// Snap one dragged world point (a tool handle or Tweak's grab point):
    /// the target it lands on, if any, and the pre-highlight marker.
    /// `moved` names the active mesh's points that travel with it, when
    /// the selection doesn't say (Tweak's grabbed element).
    pub(super) fn snap_point(
        &mut self,
        handle: DVec3,
        moved: Option<std::collections::BTreeSet<u32>>,
        r: egui::Rect,
    ) -> Option<DVec3> {
        let at = self.camera.project(handle, r).map(|(at, _)| at);
        self.snap_point_at(handle, at, moved, r)
    }
    /// An Action Center being placed by a press at `at` (C-AUTO's press,
    /// GitHub #39): the target under the pointer, if any. Unlike a dragged
    /// handle, the search is where the pointer is, not where `point` (the
    /// work-plane hit at the old centre's height) shows on screen, and the
    /// selection's own points, edges and polygons are targets too: nothing
    /// moves while a centre is placed, so a pivot can go onto a corner of
    /// what it is about to turn.
    pub(super) fn snap_placed_center(
        &mut self,
        point: DVec3,
        at: egui::Pos2,
        r: egui::Rect,
    ) -> Option<DVec3> {
        // Its own search, so the drag that follows builds the usual one.
        self.snap_session = Default::default();
        let target = self.snap_point_at(point, Some(at), Some(Default::default()), r);
        self.snap_session = Default::default();
        target
    }
    /// `snap_point` with the search at screen point `at` (none: `handle`
    /// is out of view).
    fn snap_point_at(
        &mut self,
        handle: DVec3,
        at: Option<egui::Pos2>,
        moved: Option<std::collections::BTreeSet<u32>>,
        r: egui::Rect,
    ) -> Option<DVec3> {
        self.snap_marker = None;
        if let Some(hold) = &mut self.snap_hold {
            hold.dragged = true;
        }
        let targets = self.snap_targets();
        if !targets.any() {
            self.snap_reticle = None;
            return None;
        }
        self.snap_reticle = Some(handle);
        let at = at?;
        let found = if targets.any_service() {
            self.build_snap_session(moved);
            self.snap_handle_target(at, targets, r)
        } else {
            None
        };
        let grid = targets.grid.then(|| self.grid_hit(handle, at, r)).flatten();
        let target = self.pick_snap(found, grid);
        self.snap_marker = match (target, found) {
            (Some(t), _) => Some((t.point, true)),
            // SN2-30: Pre-highlight off shows no marker until it snaps.
            (None, Some(v)) if self.snapping.pre_highlight => Some((v.hit.point, false)),
            (None, _) => None,
        };
        target.map(|t| t.point)
    }
    /// SN2-15/22: a found target within its Inner Range wins; otherwise
    /// the grid crossing, which snaps wherever its rule allows.
    fn pick_snap(
        &self,
        found: Option<crate::snap_service::SnapHit>,
        grid: Option<Hit>,
    ) -> Option<Hit> {
        let inner = |kind: Kind| {
            if kind == Kind::Pivot {
                self.snapping.pivot_snap_size
            } else {
                self.snapping.inner_range
            }
        };
        match found {
            Some(f) if f.hit.distance <= inner(f.kind) => Some(f.hit),
            _ => grid,
        }
    }
    /// SN2-22/23: the grid crossing nearest `handle`, on the grid the view
    /// shows, or every Grid Size from the world origin with Use Fixed Grid.
    fn grid_hit(&self, handle: DVec3, at: Pos2, r: egui::Rect) -> Option<Hit> {
        let plane = self.actual_plane();
        let (center, step) = if self.snapping.use_fixed_grid {
            (DVec3::ZERO, self.snapping.grid_size)
        } else {
            let center = self.workplane_center();
            (center, crate::grid::spacing(&self.camera, r, plane, center))
        };
        let point = grid_crossing(handle, plane, center, step, self.snapping.grid_distance)?;
        let (screen, _) = self.camera.project(point, r)?;
        Some(Hit {
            point,
            screen,
            distance: screen.distance(at),
        })
    }
    /// The Geometry options' rules (SN2-18/19) every snap search shares:
    /// whether `owner` is left out by Layers or Only Selected, judged on
    /// `scene`'s selection.
    fn geometry_skip(
        &self,
        scene: &operandi_core::Scene,
        world: &crate::snap_service::WorldTargets,
        owner: &crate::snap_service::Owner,
    ) -> bool {
        use crate::snap_service::Owner;
        let item = match *owner {
            Owner::Point { item, .. } | Owner::Edge { item, .. } | Owner::Polygon { item, .. } => {
                item
            }
            Owner::Item(item) => item,
            Owner::None => return false,
        };
        let selection = &scene.selection;
        let selected = selection.item == Some(item) || selection.also.contains(&item);
        let by_layer = match self.snapping.layers {
            Layers::Both => false,
            Layers::Active => !selected,
            Layers::Background => selected,
        };
        if by_layer {
            return true;
        }
        if !self.snapping.only_selected || matches!(owner, Owner::Item(_)) {
            return false;
        }
        // Only the selected polygons of the active mesh, their edges and
        // their points (Operandi choice).
        if selection.item != Some(item) {
            return true;
        }
        match *owner {
            Owner::Polygon { face, .. } => !selection.faces.contains(&face),
            _ => {
                let points = world.owner_points(owner);
                !points.iter().all(|v| {
                    selection.faces.iter().any(|&f| {
                        world
                            .owner_points(&Owner::Polygon { item, face: f })
                            .contains(v)
                    })
                })
            }
        }
    }
    #[cfg(test)]
    pub(super) fn geometry_skip_for_tests(
        &self,
        scene: &operandi_core::Scene,
        world: &crate::snap_service::WorldTargets,
        owner: &crate::snap_service::Owner,
    ) -> bool {
        self.geometry_skip(scene, world, owner)
    }
    /// SN-09: every visible target of every visible mesh when the drag
    /// started, and the active mesh's points the tool moves.
    fn build_snap_session(&mut self, moved: Option<std::collections::BTreeSet<u32>>) {
        if self.snap_session.targets.is_some() {
            return;
        }
        let Some(live) = self.editor.session.live.as_ref() else {
            return;
        };
        let base = &live.base;
        let mut session = Session {
            // Occlusion uses the cage, not the smooth surface
            // (provisional; unchanged by the smooth-picking merge).
            targets: Some((self.snap_service.world(base), self.snap_service.cage(base))),
            ..Default::default()
        };
        if let Some(moved) = moved {
            session.moved = moved;
        } else if base.selection.mode != operandi_core::Mode::Item
            && let Ok(moved) = operandi_core::transform_vertices(base)
        {
            session.moved = moved;
        }
        self.snap_session = session;
    }
    /// The best target for a tool handle at `at` within the Outer Range.
    /// What the tool moves is left out (SN2-17): the selection's points,
    /// any point whose position or item changed in the preview (Symmetry,
    /// Falloff, items), and edges, polygons and boxes made from them.
    fn snap_handle_target(
        &self,
        at: Pos2,
        targets: Targets,
        r: egui::Rect,
    ) -> Option<crate::snap_service::SnapHit> {
        use crate::snap_service::Owner;
        let live = self.editor.session.live.as_ref()?;
        let (base, scene) = (&live.base, &self.editor.scene);
        let active = base.selection.item;
        let item_mode = base.selection.mode == operandi_core::Mode::Item;
        let mut moved_items = std::collections::BTreeSet::new();
        for item in &base.items {
            let same = match (base.world_matrix(item.id), scene.world_matrix(item.id)) {
                (Ok(a), Ok(b)) => a == b,
                _ => false,
            };
            if !same || (item_mode && Some(item.id) == active) {
                moved_items.insert(item.id);
            }
        }
        let now = scene.active().filter(|i| Some(i.id) == active);
        let moved = &self.snap_session.moved;
        let (world, occluder) = self.snap_session.targets.as_ref()?;
        let point_moves = |v: u32| {
            if moved.contains(&v) {
                return true;
            }
            let original = base
                .item(active.unwrap_or_default())
                .and_then(|i| i.mesh.vertices.get(v as usize));
            original.is_none() || now.and_then(|i| i.mesh.vertices.get(v as usize)) != original
        };
        let skip = |owner: &Owner| {
            let item = match *owner {
                Owner::None => return false,
                Owner::Point { item, .. }
                | Owner::Edge { item, .. }
                | Owner::Polygon { item, .. }
                | Owner::Item(item) => item,
            };
            if moved_items.contains(&item) || self.geometry_skip(base, world, owner) {
                return true;
            }
            if Some(item) != active {
                return false;
            }
            match owner {
                // The moved mesh's own box changes as it moves.
                Owner::Item(_) => !moved.is_empty() && !item_mode,
                _ => world.owner_points(owner).into_iter().any(point_moves),
            }
        };
        // Nearest first, skipping targets hidden behind a surface (Jason's
        // QA: a point on the far side overlapping on screen won).
        let role = crate::snap_service::Role::Handle;
        self.snap_service.nearest(
            world,
            &crate::snap_service::Query {
                at,
                radius: role.radius(&self.snapping),
                inner: self.snapping.inner_range,
                pivot_inner: self.snapping.pivot_snap_size,
                targets,
                skip: &skip,
                extra: &[],
                max_depth: self.snapping.depth_limit.then_some(self.snapping.depth),
                surface: occluder,
                camera: &self.camera,
                rect: r,
            },
        )
    }

    /// SPEC-DRAG-SNAP-POINTS DSN-08..13: snap a point a draw tool drags (a
    /// Curve/B-Spline/Bezier point or handle end, a Pen point, a Tube path
    /// point). `handle` is where the pointer puts it; `moving` are the
    /// world positions travelling with it, never targets (DSN-10); `own`
    /// are the shape's points that aren't mesh points yet (a Pen draft, a
    /// Tube path), which are targets like any mesh point. With `line`
    /// (origin, unit direction) the drag is Ctrl-constrained and the target
    /// is projected onto it (DSN-12). Runs the active targets with the same
    /// Inner/Outer Range as tool handles; sets the pre-highlight marker.
    /// None when snapping is off or nothing is in range.
    pub(super) fn snap_draw_point(
        &mut self,
        handle: DVec3,
        moving: &[DVec3],
        own: &[DVec3],
        line: Option<(DVec3, DVec3)>,
        r: egui::Rect,
    ) -> Option<DVec3> {
        self.snap_draw_point_skipping(handle, moving, own, line, None, r)
    }

    /// [`Self::snap_draw_point`] that also leaves out every target of item
    /// `skip_item` (Solid Sketch: the mesh it is rebuilding under the
    /// pointer).
    pub(super) fn snap_draw_point_skipping(
        &mut self,
        handle: DVec3,
        moving: &[DVec3],
        own: &[DVec3],
        line: Option<(DVec3, DVec3)>,
        skip_item: Option<u64>,
        r: egui::Rect,
    ) -> Option<DVec3> {
        self.snap_marker = None;
        self.note_snap_drag();
        let targets = self.snap_targets();
        if !targets.any() {
            self.snap_reticle = None;
            return None;
        }
        self.snap_reticle = Some(handle);
        let onto_line = |p: DVec3| match line {
            Some((o, d)) => o + d * (p - o).dot(d),
            None => p,
        };
        let (at, _) = self.camera.project(handle, r)?;
        let is_moving = |p: DVec3| moving.iter().any(|m| (*m - p).length() < 1e-9);
        let found = if targets.any_service() {
            self.build_scene_snap_session();
            self.snap_session
                .targets
                .as_ref()
                .and_then(|(world, occluder)| {
                    // Mesh targets hidden behind a surface don't count; the
                    // shape's own points are drawn on top, so they always do.
                    let own: Vec<DVec3> = own.iter().copied().filter(|&p| !is_moving(p)).collect();
                    let scene = &self.editor.scene;
                    let skip = |owner: &crate::snap_service::Owner| {
                        skip_item.is_some_and(|item| owner_item(owner) == Some(item))
                            || self.geometry_skip(scene, world, owner)
                            || is_moving_owner(world, owner, &is_moving)
                    };
                    let role = crate::snap_service::Role::DrawPoint;
                    self.snap_service.nearest(
                        world,
                        &crate::snap_service::Query {
                            at,
                            radius: role.radius(&self.snapping),
                            inner: self.snapping.inner_range,
                            pivot_inner: self.snapping.pivot_snap_size,
                            targets,
                            skip: &skip,
                            extra: &own,
                            max_depth: self.snapping.depth_limit.then_some(self.snapping.depth),
                            surface: occluder,
                            camera: &self.camera,
                            rect: r,
                        },
                    )
                })
        } else {
            None
        };
        let grid = targets.grid.then(|| self.grid_hit(handle, at, r)).flatten();
        let target = self.pick_snap(found, grid).map(|t| onto_line(t.point));
        self.snap_marker = match (target, found) {
            (Some(t), _) => Some((t, true)),
            // SN2-30: Pre-highlight off shows no marker until it snaps.
            (None, Some(v)) if self.snapping.pre_highlight => Some((v.hit.point, false)),
            (None, _) => None,
        };
        target
    }

    /// SN2-41 / VX-CR02: a point a placement tool (a primitive's corner or
    /// side, Create's point) puts down, snapped to the active targets in
    /// the scene before the tool's preview; `own` are the tool's earlier
    /// points, targets too. With `line` (origin, unit direction) the point
    /// stays on that line (a side haul). Unchanged when snapping is off or
    /// nothing is in range. Typed values never come here (SN2-03).
    pub(super) fn snap_placement(
        &mut self,
        point: DVec3,
        own: &[DVec3],
        line: Option<(DVec3, DVec3)>,
        r: egui::Rect,
    ) -> DVec3 {
        self.snap_draw_point(point, &[], own, line, r)
            .unwrap_or(point)
    }

    /// A drag ran while X is held (DSN-13): letting X go flips snapping
    /// back, whether or not this drag snapped.
    pub(super) fn note_snap_drag(&mut self) {
        if let Some(hold) = &mut self.snap_hold {
            hold.dragged = true;
        }
    }

    /// The vertex targets for a draw-tool drag: every point of every
    /// visible mesh in the scene as it is (draw tools have no live base),
    /// pinned for the drag.
    fn build_scene_snap_session(&mut self) {
        if self.snap_session.targets.is_some() {
            return;
        }
        // Hidden by the picking surface: a mesh point snaps when it could
        // be picked (it was the cage before the shared snap service).
        // A tool previewing on a live base (Pen Slice) snaps to the scene
        // as it was before the preview, not to its own cut.
        let scene = self
            .editor
            .session
            .live
            .as_ref()
            .map_or(&self.editor.scene, |l| &l.base);
        let surface = self.pick_surface();
        self.snap_session = Session {
            targets: Some((self.snap_service.world(scene), surface)),
            ..Default::default()
        };
    }
}

/// A popover's place and state between frames.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PopoverState {
    /// Top left; not finite until the first frame puts it at the pointer.
    pub at: Pos2,
    /// Pinned with its circle (UIC-64).
    pub pinned: bool,
    /// The pointer has been over it since it opened.
    pub entered: bool,
    /// N06: opened by a click this frame (Option+click on the Snapping
    /// button, placed below it): that click isn't a click outside.
    pub opening: bool,
}

/// One frame of a popover (Snapping, Mesh Constraint): in the app, above
/// everything, first placed at the pointer. The circle pins it where it
/// is (UIC-64); pinned, the circle closes it and the header strip moves it
/// inside the main window only (UIC-185/187). Unpinned, leaving it by
/// more than 8 px closes it (UIC-62), as do Esc and a click outside.
/// `body` draws the rows under the header and returns what the header
/// asked for. Returns the next state, or `None` when it closes.
pub(super) fn show_popover(
    ctx: &egui::Context,
    id: &str,
    mut state: PopoverState,
    fallback: Pos2,
    click_off: bool,
    body: impl FnOnce(&mut egui::Ui) -> HeaderAction,
) -> Option<PopoverState> {
    if !state.at.is_finite() {
        state.at = ctx
            .pointer_latest_pos()
            .map(|p| p - egui::vec2(12., 12.))
            .unwrap_or(fallback);
        if !state.at.is_finite() {
            state.at = Pos2::new(40., 40.);
        }
    }
    let screen = ctx.content_rect();
    let shown = egui::Area::new(egui::Id::new(id))
        .order(egui::Order::Foreground)
        .constrain_to(screen)
        .fixed_pos(state.at)
        .show(ctx, |ui| {
            egui::Frame::popup(&ctx.style())
                .fill(crate::theme::popup_fill())
                .show(ui, body)
                .inner
        });
    let rect = shown.response.rect;
    match shown.inner {
        HeaderAction::Pin if state.pinned => return None,
        HeaderAction::Pin => state.pinned = true,
        HeaderAction::Drag(delta) => {
            // #60: a drag pins it, so it stays where it is dropped
            // (UIC-185: only a pinned popover moves).
            state.pinned = true;
            let size = rect.size();
            let max = (screen.max - size).max(screen.min);
            state.at = (state.at + delta).clamp(screen.min, max);
        }
        HeaderAction::None => {}
    }
    let hover = ctx.input(|i| i.pointer.hover_pos());
    let near = |p: Pos2| rect.expand(8.).contains(p);
    if hover.is_some_and(near) {
        state.entered = true;
    }
    let child_open = egui::Popup::is_any_open(ctx);
    if !state.pinned && !child_open {
        let escape = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        let clicked_outside = !state.opening
            && ctx.input(|i| {
                i.pointer.any_click() && i.pointer.interact_pos().is_some_and(|q| !near(q))
            });
        let rolled_off = !click_off
            && state.entered
            && !ctx.input(|i| i.pointer.any_down())
            && hover.is_some_and(|q| !near(q));
        if escape || rolled_off || ((click_off || !state.entered) && clicked_outside) {
            return None;
        }
    }
    Some(state)
}

/// What the popover's header strip asked for this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) enum HeaderAction {
    #[default]
    None,
    /// The pin circle was clicked.
    Pin,
    /// A pinned popover's strip was dragged by this much.
    Drag(egui::Vec2),
}

/// A popover's header strip: no title (UIC-181) and the pin circle at the
/// top right (UIC-64). Once pinned, the strip itself is a plain draggable
/// header that moves the popover inside the main window (UIC-185); it has
/// no grip, because popovers never tear off (UIC-187).
pub(super) fn popover_header(ui: &mut egui::Ui, pinned: bool) -> HeaderAction {
    let (row, strip) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 16.), egui::Sense::drag());
    let circle = egui::Rect::from_center_size(
        egui::pos2(row.right() - 8., row.center().y),
        egui::Vec2::splat(14.),
    );
    let response = ui
        .interact(circle, ui.id().with("popover-pin"), egui::Sense::click())
        .on_hover_text(if pinned {
            "Close"
        } else {
            "Pin: keep this popover open; then drag this strip to move it"
        });
    let painter = ui.painter();
    if pinned {
        painter.circle_filled(circle.center(), 5.5, crate::theme::orange());
        // A move cursor says the strip drags (no grip, UIC-187).
        if strip.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if strip.hovered() && !response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
    } else {
        let color = if response.hovered() {
            crate::theme::text()
        } else {
            crate::theme::weak_text()
        };
        painter.circle_stroke(circle.center(), 5.5, egui::Stroke::new(1.2_f32, color));
    }
    if response.clicked() {
        HeaderAction::Pin
    } else if pinned && strip.dragged() && strip.drag_delta() != egui::Vec2::ZERO {
        HeaderAction::Drag(strip.drag_delta())
    } else {
        HeaderAction::None
    }
}

/// Where the pointer at `pos` meets the plane facing the view through
/// `through`.
fn view_plane_point(
    camera: &crate::camera::Camera,
    through: DVec3,
    pos: Pos2,
    r: egui::Rect,
) -> Option<DVec3> {
    let (_, _, _, forward) = camera.basis();
    let (o, d) = camera.ray(pos, r);
    let denom = d.dot(forward);
    if denom.abs() < 1e-12 {
        return None;
    }
    let t = (through - o).dot(forward) / denom;
    (t > 0.).then(|| o + d * t).filter(|p| p.is_finite())
}

/// Whether a mesh point target sits where one of the dragged points is
/// (DSN-10: what travels with the drag is never a target).
/// The item a snap target belongs to.
fn owner_item(owner: &crate::snap_service::Owner) -> Option<u64> {
    use crate::snap_service::Owner;
    match *owner {
        Owner::Point { item, .. }
        | Owner::Edge { item, .. }
        | Owner::Polygon { item, .. }
        | Owner::Item(item) => Some(item),
        Owner::None => None,
    }
}

fn is_moving_owner(
    world: &crate::snap_service::WorldTargets,
    owner: &crate::snap_service::Owner,
    is_moving: &dyn Fn(DVec3) -> bool,
) -> bool {
    match *owner {
        crate::snap_service::Owner::Point { item, v } => {
            world.point_of(item, v).is_some_and(is_moving)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operandi_core::Mode;

    #[test]
    fn defaults_are_off_global_vertex_24_and_40_px() {
        let s = SnappingPreferences::default();
        assert!(!s.enabled);
        assert_eq!(
            s.global,
            Targets {
                vertex: true,
                grid: false,
                ..Targets::NONE
            }
        );
        assert!(!s.component.any() && !s.items.any());
        assert_eq!(
            (s.inner_range, s.outer_range, s.grid_distance),
            (24., 40., 1.)
        );
    }

    #[test]
    fn modes_add_their_own_ticks_to_global() {
        let s = SnappingPreferences {
            global: Targets {
                vertex: false,
                grid: true,
                ..Targets::NONE
            },
            component: Targets {
                vertex: true,
                grid: false,
                ..Targets::NONE
            },
            ..Default::default()
        };
        assert_eq!(
            s.active(Mode::Vertex),
            Targets {
                vertex: true,
                grid: true,
                ..Targets::NONE
            }
        );
        assert_eq!(
            s.active(Mode::Item),
            Targets {
                vertex: false,
                grid: true,
                ..Targets::NONE
            }
        );
    }

    #[test]
    fn nearest_picks_the_closest_within_range() {
        let a = (DVec3::X, Pos2::new(10., 0.));
        let b = (DVec3::Y, Pos2::new(5., 0.));
        let c = (DVec3::Z, Pos2::new(50., 0.));
        let hit = nearest([a, b, c], Pos2::ZERO, 24.).unwrap();
        assert_eq!(hit.point, DVec3::Y);
        assert!(nearest([c], Pos2::ZERO, 24.).is_none());
    }

    #[test]
    fn grid_snaps_in_plane_and_keeps_height() {
        let p = DVec3::new(0.26, 0.7, -0.74);
        // XZ plane, 0.5 m squares: X and Z round, Y stays.
        assert_eq!(
            grid_crossing(p, 0, DVec3::ZERO, 0.5, 1.),
            Some(DVec3::new(0.5, 0.7, -0.5))
        );
        // XY plane through z = 3.
        let q = grid_crossing(p, 2, DVec3::new(0., 0., 3.), 0.5, 1.).unwrap();
        assert_eq!(q, DVec3::new(0.5, 0.5, -0.74));
        // At 20 %, only within a tenth of a square of a crossing.
        assert!(grid_crossing(p, 0, DVec3::ZERO, 0.5, 0.2).is_none());
        let near = DVec3::new(0.52, 0., 0.49);
        assert_eq!(
            grid_crossing(near, 0, DVec3::ZERO, 0.5, 0.2),
            Some(DVec3::new(0.5, 0., 0.5))
        );
    }

    #[test]
    fn preferences_round_trip_and_reject_bad_files() {
        let dir = std::env::temp_dir().join(format!("operandi-snap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("snapping-preferences.json");
        let s = SnappingPreferences {
            enabled: true,
            inner_range: 10.,
            ..Default::default()
        };
        s.save(&path).unwrap();
        assert_eq!(SnappingPreferences::load(&path), s);
        std::fs::write(&path, b"{\"version\":1,\"inner_range\":-3}").unwrap();
        assert_eq!(
            SnappingPreferences::load(&path),
            SnappingPreferences::default()
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
