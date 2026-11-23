//! Application display preferences, separate from scene data and modeling history.
use crate::prefs_file::PrefsFile;
use serde::{Deserialize, Serialize};

pub const FLATNESS_RANGE: std::ops::RangeInclusive<u8> = 0..=100;
pub const MAX_GL_RANGE: std::ops::RangeInclusive<u64> = 0..=1_000_000_000;

/// Bounding Box Threshold: which items draw as boxes while the view is
/// navigated once the graphics card nears its memory budget
/// (gpu_memory.rs; it used to be Max GL Polygon Count).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoxThreshold {
    /// Provisional default: never (the drawing so far).
    #[default]
    None,
    AllSurfaceItems,
    OnlyInactiveSurfaceItems,
}
impl BoxThreshold {
    pub const ALL: [Self; 3] = [
        Self::None,
        Self::AllSurfaceItems,
        Self::OnlyInactiveSurfaceItems,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::AllSurfaceItems => "All Surface Items",
            Self::OnlyInactiveSurfaceItems => "Only Inactive Surface Items",
        }
    }
}
pub const PIXEL_SIZE_RANGE: std::ops::RangeInclusive<f64> = 1e-6..=1000.;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ViewportPreferences {
    version: u8,
    pub workplane_opacity: u8,
    /// Show Work Plane: draw the work plane's own grid, apart from the
    /// ground grid (Jason, 2026-11-23). Off by default; each view's Show
    /// Work Plane row (Viewport Properties > Visibility) then decides
    /// when it draws there.
    pub workplane_display: bool,
    /// Grid Visibility: whether new 3D views show the ground grid.
    /// Measured: on.
    pub grid_visibility: bool,
    /// Flatness of Perspective, percent: higher is flatter (a longer lens).
    /// Measured: 60, which keeps Operandi's lens so far.
    pub flatness: u8,
    /// Item Drawing: Default Size for New Scenes (the current scene's is
    /// kept in the scene). Provisional: 1×.
    pub item_draw_size: f64,
    /// Backdrop Item: Include Backdrops in Viewport Fitting. Provisional:
    /// on (BD-42, the behaviour so far).
    pub backdrop_fitting: bool,
    /// Display Backdrops in Uniform Size: new Backdrops size from their
    /// item's scale instead of Pixel Size. Provisional: off (BD-02).
    pub backdrop_uniform_size: bool,
    /// Pixel Size: metres per image pixel for new Backdrops. Measured:
    /// 10 mm.
    pub backdrop_pixel_size: f64,
    /// Retired Max GL Polygon Count (Jason, 2026-11-20): graphics memory
    /// is watched instead (gpu_memory.rs). Still read so older files load
    /// (the struct denies unknown fields); never written.
    #[serde(skip_serializing)]
    pub max_gl_polygons: u64,
    pub box_threshold: BoxThreshold,
    /// Fade Wireframe on Small Polygons: where polygons are only a few
    /// points across on screen, their plain wire fades out (gpu.rs
    /// WIRE_FADE_*). On by default (Jason, 2026-11-13).
    pub fade_small_wire: bool,
    /// Show Material Transparency: in the Advanced style (Jason,
    /// 2026-11-14), surfaces with Transparent Amount let what is behind them show
    /// through, tinted by Transparent Color (gpu.rs, viewport transparency
    /// spec VT-01..05). On by default (provisional, VT-Q05).
    pub material_transparency: bool,
    /// Show Asymmetries (N22, Jason 2026-11-17): with Symmetry on, points
    /// that have no mirror partner show as small yellow squares. On by
    /// default; also in the Symmetry menu.
    pub show_asymmetries: bool,
    /// Retired MT8 Topology Offset (Jason, 2026-11-21: dropped, no effect
    /// while the foreground draws on top). Still read so files saved with
    /// it load (the struct denies unknown fields); never written.
    #[serde(default, skip_serializing)]
    topology_offset: f64,
    /// MT9 Foreground Opacity, percent: Topology Mode's face opacity.
    #[serde(default = "topology_opacity")]
    pub topology_opacity: u8,
    /// MT7-03 Hide back-facing topology points (costs speed). Default on.
    #[serde(default = "shown")]
    pub topology_hide_back_points: bool,
}
fn topology_opacity() -> u8 {
    50
}
fn shown() -> bool {
    true
}
impl Default for ViewportPreferences {
    fn default() -> Self {
        Self {
            version: 1,
            workplane_opacity: 25,
            workplane_display: false,
            grid_visibility: true,
            flatness: 60,
            item_draw_size: 1.,
            backdrop_fitting: true,
            backdrop_uniform_size: false,
            backdrop_pixel_size: operandi_core::backdrop::DEFAULT_PIXEL_SIZE,
            max_gl_polygons: 5_000_000,
            box_threshold: BoxThreshold::None,
            fade_small_wire: true,
            material_transparency: true,
            show_asymmetries: true,
            topology_offset: 0.,
            topology_opacity: topology_opacity(),
            topology_hide_back_points: true,
        }
    }
}
impl crate::prefs_file::PrefsFile for ViewportPreferences {
    const INVALID: &'static str = "Invalid viewport display preference";
    fn valid(&self) -> bool {
        self.version == 1
            && self.workplane_opacity <= 100
            && FLATNESS_RANGE.contains(&self.flatness)
            && operandi_core::scene::ITEM_DRAW_SIZE_RANGE.contains(&self.item_draw_size)
            && PIXEL_SIZE_RANGE.contains(&self.backdrop_pixel_size)
            && MAX_GL_RANGE.contains(&self.max_gl_polygons)
            && self.topology_opacity <= 100
    }
}

impl super::Operandi {
    /// A view that has no display settings yet starts from these: Grid
    /// Visibility decides its ground grid.
    /// A saved "Default for 3D Viewport" (Viewport Properties > All Tabs)
    /// wins over both.
    pub(super) fn new_view_display(&self) -> crate::view_display::ViewDisplay {
        let mut display = crate::view_display::ViewDisplay::default();
        display.settings.drawing.grid = self.viewport_preferences.grid_visibility;
        if let Some(saved) = self.corner.properties.presets.default {
            display.settings = saved;
        }
        display
    }
    /// Hand what core reads (new names, Backdrop sizing) and the new-view
    /// lens to their owners, at launch and whenever a preference changes.
    pub(super) fn apply_user_defaults(&self) {
        let d = &self.defaults_preferences;
        let v = &self.viewport_preferences;
        let mut defaults = operandi_core::user_defaults::UserDefaults {
            index_style: d.index_style,
            material: d.default_material.clone(),
            texture_map: d.texture_map.clone(),
            backdrop_uniform_size: v.backdrop_uniform_size,
            backdrop_pixel_size: v.backdrop_pixel_size,
            ..Default::default()
        };
        self.mesh_items_preferences.fill(&mut defaults);
        operandi_core::user_defaults::set_current(defaults);
        crate::camera::set_flatness(v.flatness);
        crate::os_color::set_enabled(d.os_color_picker);
    }
    /// Flatness of Perspective changed from `before` percent: every view's
    /// lens follows (see `Camera::reflatten`).
    pub(super) fn reflatten_views(&mut self, before: u8) {
        let ratio = crate::camera::flatness_scale(self.viewport_preferences.flatness)
            / crate::camera::flatness_scale(before);
        self.camera.reflatten(ratio);
        for camera in self.cameras.values_mut() {
            camera.reflatten(ratio);
        }
        crate::camera::set_flatness(self.viewport_preferences.flatness);
    }
    /// Bounding Box Threshold: whether `item` draws as its box now (only
    /// while the view is navigated and the scene is over the cap).
    pub(super) fn draws_as_box(&self, item: u64, over_cap: bool) -> bool {
        use BoxThreshold::*;
        over_cap
            && matches!(
                self.navigation.as_ref().map(|g| &g.kind),
                Some(crate::input::GestureKind::Navigate(_))
            )
            && match self.viewport_preferences.box_threshold {
                None => false,
                AllSurfaceItems => true,
                OnlyInactiveSurfaceItems => self.editor.scene.selection.item != Some(item),
            }
    }

    /// Preferences > Display > OpenGL (and Work Plane Opacity).
    pub(super) fn display_page(&mut self, ui: &mut eframe::egui::Ui) {
        let before = self.viewport_preferences;
        let draw_size_before = self.editor.scene.item_draw_size;
        let v = &mut self.viewport_preferences;
        let d = ViewportPreferences::default();
        use crate::form::{Unit, rows};
        let scene_size = &mut self.editor.scene.item_draw_size;
        rows::section(ui, "Display", false, |ui| {
            rows::check_d(
                ui,
                "Show Work Plane",
                &mut v.workplane_display,
                d.workplane_display,
                "Draw the work plane's own grid, apart from the ground grid",
            );
            rows::number_d(
                ui,
                "Work Plane Opacity",
                &mut v.workplane_opacity,
                d.workplane_opacity,
                Unit::Percent,
                0..=100,
                "How strongly the work plane grid draws",
            );
            rows::check_d(
                ui,
                "Grid Visibility",
                &mut v.grid_visibility,
                d.grid_visibility,
                "Whether new 3D views show the ground grid",
            );
            rows::number_d(
                ui,
                "Flatness of Perspective",
                &mut v.flatness,
                d.flatness,
                Unit::Percent,
                FLATNESS_RANGE,
                "Higher is flatter, like a longer lens",
            );
            rows::number_d(
                ui,
                "Item Drawing: Default Size for New Scenes",
                &mut v.item_draw_size,
                d.item_draw_size,
                Unit::Suffix("×"),
                operandi_core::scene::ITEM_DRAW_SIZE_RANGE,
                "How large cameras and lights draw in new scenes",
            );
            rows::number_d(
                ui,
                "Item Drawing: Current Scene",
                scene_size,
                v.item_draw_size,
                Unit::Suffix("×"),
                operandi_core::scene::ITEM_DRAW_SIZE_RANGE,
                "How large cameras and lights draw in this scene",
            );
            rows::check_d(
                ui,
                "Fade Wireframe on Small Polygons",
                &mut v.fade_small_wire,
                d.fade_small_wire,
                "Where polygons are only a few points across, their wire fades out, \
                 so dense meshes stay readable and draw faster",
            );
            rows::check_d(
                ui,
                "Show Asymmetries",
                &mut v.show_asymmetries,
                d.show_asymmetries,
                "With Symmetry on, points with no mirror partner show as small yellow squares",
            );
            rows::check_d(
                ui,
                "Show Material Transparency",
                &mut v.material_transparency,
                d.material_transparency,
                "Glass, liquids and other transparent materials let what is behind them \
                 show through in the Advanced style",
            );
            let thresholds: Vec<_> = BoxThreshold::ALL.iter().map(|&t| (t, t.label())).collect();
            rows::choice_d(
                ui,
                "Bounding Box Threshold",
                &mut v.box_threshold,
                &d.box_threshold,
                &thresholds,
                "Near the graphics card's memory budget, these items draw as boxes while navigating",
            );
            rows::check_d(
                ui,
                "Include Backdrops in Viewport Fitting",
                &mut v.backdrop_fitting,
                d.backdrop_fitting,
                "Fit and Frame also take Backdrop items into account",
            );
            rows::check_d(
                ui,
                "Display Backdrops in Uniform Size",
                &mut v.backdrop_uniform_size,
                d.backdrop_uniform_size,
                "New Backdrops size from their item's scale",
            );
            // Topology Mode (missing-tools-g3 MT7-03, MT9).
            rows::number_d(
                ui,
                "Foreground Opacity",
                &mut v.topology_opacity,
                d.topology_opacity,
                Unit::Percent,
                0..=100,
                "How solid the active meshes' faces draw in Topology Mode",
            );
            rows::check_d(
                ui,
                "Hide Back-Facing Topology Points",
                &mut v.topology_hide_back_points,
                d.topology_hide_back_points,
                "In Topology Mode, points on polygons facing away don't draw (costs speed)",
            );
            rows::number_d(
                ui,
                "Backdrop Pixel Size",
                &mut v.backdrop_pixel_size,
                d.backdrop_pixel_size,
                Unit::Distance,
                0.000_001..=1_000.,
                "Real size of one image pixel for new Backdrops",
            );
        });
        if self.editor.scene.item_draw_size != draw_size_before {
            ui.ctx().request_repaint();
        }
        if self.viewport_preferences == before {
            return;
        }
        if self.viewport_preferences.flatness != before.flatness {
            self.reflatten_views(before.flatness);
        }
        self.apply_user_defaults();
        if let Err(error) = self
            .viewport_preferences
            .save(&self.environment.paths().viewport_display())
        {
            self.status = format!("Preference changed; could not save it: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// N22: Show Asymmetries is on by default and set or read with
    /// `pref.value` like any preference.
    #[test]
    fn show_asymmetries_is_on_by_default_and_scriptable() {
        let mut a = crate::Operandi::empty();
        assert!(a.viewport_preferences.show_asymmetries);
        a.execute("pref.value display.show_asymmetries false");
        assert!(!a.viewport_preferences.show_asymmetries, "{}", a.status);
        a.execute("pref.value display.show_asymmetries ?");
        assert_eq!(a.status, "false");
    }
    #[test]
    fn work_plane_display_is_off_by_default_and_saved() {
        // Jason, 2026-11-23: the work plane has its own display switch,
        // apart from the ground grid, and it starts off.
        assert!(!ViewportPreferences::default().workplane_display);
        let root =
            std::env::temp_dir().join(format!("operandi-workplane-display-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("viewport-display.json");
        // A file saved before the switch existed loads with it off.
        fs::write(&path, r#"{"version":1,"workplane_opacity":40}"#).unwrap();
        let old = ViewportPreferences::load(&path);
        assert_eq!(old.workplane_opacity, 40);
        assert!(!old.workplane_display);
        let on = ViewportPreferences {
            workplane_display: true,
            ..Default::default()
        };
        on.save(&path).unwrap();
        assert!(ViewportPreferences::load(&path).workplane_display);
        fs::remove_dir_all(root).unwrap();
    }
    use std::fs;
    #[test]
    fn opacity_roundtrip_rejects_invalid_preferences_and_preserves_files_on_failure() {
        let root = std::env::temp_dir().join(format!(
            "operandi-opacity-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let path = root.join("viewport-display.json");
        assert_eq!(ViewportPreferences::load(&path).workplane_opacity, 25);
        for opacity in [0, 25, 50, 100] {
            let p = ViewportPreferences {
                workplane_opacity: opacity,
                ..Default::default()
            };
            p.save(&path).unwrap();
            assert_eq!(ViewportPreferences::load(&path), p);
        }
        let original = fs::read(&path).unwrap();
        assert!(
            ViewportPreferences {
                workplane_opacity: 101,
                ..Default::default()
            }
            .save(&path)
            .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), original);
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        fs::write(&temp, b"preexisting").unwrap();
        assert!(ViewportPreferences::default().save(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read(&temp).unwrap(), b"preexisting");
        for invalid in [
            r#"{"version":2,"workplane_opacity":25}"#,
            r#"{"version":1,"workplane_opacity":200}"#,
            "invalid",
        ] {
            fs::write(&path, invalid).unwrap();
            assert_eq!(
                ViewportPreferences::load(&path),
                ViewportPreferences::default()
            );
        }
        fs::remove_dir_all(root).unwrap();
    }
}
