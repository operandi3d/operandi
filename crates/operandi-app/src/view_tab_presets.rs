//! Viewport Properties' One Tab ▾ and All Tabs ▾ presets (UIC-103, FLAGS
//! F11 settled 2026-11-08): saved values for one tab, or for every (or
//! some) tab, of a 3D view's properties; plus Save/Load Default, which new
//! views start from. Saved in user config; survive restarts.
//!
//! Which settings a tab holds is `copy_tab`: exactly the rows the popover
//! shows on that tab. View Type is the view's camera, not a display value,
//! so presets leave it alone.
use crate::view_display::Settings;
use crate::viewport_properties::Tab;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The item type the menus name ("(new for 3D Viewport)").
pub const TYPE_NAME: &str = "3D Viewport";

/// One saved preset: its name, the tabs it holds and their values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    /// Tab labels this preset holds.
    pub tabs: Vec<String>,
    pub values: Settings,
}

impl Preset {
    pub fn holds(&self, tab: Tab) -> bool {
        self.tabs.iter().any(|t| t == tab.label())
    }
    /// Apply the held tabs' values to `to`.
    pub fn apply(&self, to: &mut Settings) {
        for tab in Tab::ALL {
            if self.holds(tab) {
                copy_tab(tab, &self.values, to);
            }
        }
    }
}

/// Every saved viewport preset.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TabPresets {
    /// One Tab ▾ presets, each holding a single tab.
    pub one_tab: Vec<Preset>,
    /// All Tabs ▾ presets, holding all or several tabs.
    pub all_tabs: Vec<Preset>,
    /// Save Default for 3D Viewport: what new views start from.
    pub default: Option<Settings>,
}

impl TabPresets {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, text)
    }
    /// The One Tab presets for `tab`.
    #[cfg(test)]
    pub fn for_tab(&self, tab: Tab) -> Vec<&Preset> {
        self.one_tab.iter().filter(|p| p.holds(tab)).collect()
    }
    /// Save `values` of `tabs` under `name` (one_tab or all_tabs); a
    /// preset of that name in the same list is replaced.
    pub fn add(&mut self, one: bool, name: &str, tabs: &[Tab], values: Settings) -> bool {
        let name = name.trim();
        if name.is_empty() || tabs.is_empty() {
            return false;
        }
        let preset = Preset {
            name: name.to_owned(),
            tabs: tabs.iter().map(|t| t.label().to_owned()).collect(),
            values,
        };
        let list = if one {
            &mut self.one_tab
        } else {
            &mut self.all_tabs
        };
        let same = |p: &Preset| p.name == preset.name && (!one || p.tabs == preset.tabs);
        list.retain(|p| !same(p));
        list.push(preset);
        true
    }
    pub fn rename(&mut self, one: bool, index: usize, name: &str) -> bool {
        let name = name.trim();
        let list = if one {
            &mut self.one_tab
        } else {
            &mut self.all_tabs
        };
        match list.get_mut(index) {
            Some(p) if !name.is_empty() => {
                p.name = name.to_owned();
                true
            }
            _ => false,
        }
    }
    pub fn delete(&mut self, one: bool, index: usize) {
        let list = if one {
            &mut self.one_tab
        } else {
            &mut self.all_tabs
        };
        if index < list.len() {
            list.remove(index);
        }
    }
}

/// Copy one tab's settings from `from` to `to`: the rows that tab shows.
pub fn copy_tab(tab: Tab, from: &Settings, to: &mut Settings) {
    let (f, t) = (&from.drawing, &mut to.drawing);
    match tab {
        Tab::Drawing => {
            t.controls = f.controls;
            t.info = f.info;
            t.item_mode_wireframe = f.item_mode_wireframe;
            t.background = f.background;
            t.trackball = f.trackball;
            t.orbit_selected = f.orbit_selected;
        }
        Tab::Visibility => {
            t.lights = f.lights;
            t.cameras = f.cameras;
            t.locators = f.locators;
            t.meshes = f.meshes;
            t.grid = f.grid;
            t.backdrops = f.backdrops;
            t.symmetry_plane = f.symmetry_plane;
            t.rollovers = f.rollovers;
            t.smoothing = f.smoothing;
            t.grid_follows_work_plane = f.grid_follows_work_plane;
            t.work_plane = f.work_plane;
            t.faded_grid = f.faded_grid;
            t.grid_axes = f.grid_axes;
            t.texture_locators = f.texture_locators;
            t.scene_lighting = f.scene_lighting;
            t.selections = f.selections;
            t.selection_normals = f.selection_normals;
            t.selected_outline = f.selected_outline;
            t.selected_filling = f.selected_filling;
        }
        Tab::Advanced => {
            t.shadows = f.shadows;
            t.ambient_intensity = f.ambient_intensity;
            t.ambient_color = f.ambient_color;
        }
        Tab::Active => {
            to.active = from.active;
            let (f, t) = (&from.drawing, &mut to.drawing);
            t.uv_seams = f.uv_seams;
            t.weights = f.weights;
            t.distortion = f.distortion;
            t.show_vertices = f.show_vertices;
            t.independent_point_size = f.independent_point_size;
            t.point_size = f.point_size;
        }
        Tab::Inactive => {
            t.inactive_same_as_active = f.inactive_same_as_active;
            t.inactive_invisible = f.inactive_invisible;
            to.inactive = from.inactive;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_display::{Background, Style};

    fn changed() -> Settings {
        let mut s = Settings::default();
        s.drawing.background = Background::Gradient;
        s.drawing.grid = !s.drawing.grid;
        s.drawing.shadows = !s.drawing.shadows;
        s.active.style = Style::Wireframe;
        s.inactive.opacity = 7;
        s
    }

    #[test]
    fn a_one_tab_preset_changes_only_its_tab() {
        let mut presets = TabPresets::default();
        assert!(presets.add(true, "Wire", &[Tab::Active], changed()));
        assert_eq!(presets.for_tab(Tab::Active).len(), 1);
        assert!(presets.for_tab(Tab::Drawing).is_empty());
        let mut target = Settings::default();
        presets.one_tab[0].apply(&mut target);
        assert_eq!(target.active.style, Style::Wireframe);
        assert_eq!(
            target.drawing.background,
            Background::None,
            "Drawing untouched"
        );
        assert_eq!(target.drawing.grid, Settings::default().drawing.grid);
        assert_eq!(target.inactive, Settings::default().inactive);
    }

    #[test]
    fn all_tabs_and_selected_tabs_presets() {
        let mut presets = TabPresets::default();
        presets.add(false, "Everything", &Tab::ALL, changed());
        presets.add(false, "Two", &[Tab::Drawing, Tab::Inactive], changed());
        let mut all = Settings::default();
        presets.all_tabs[0].apply(&mut all);
        assert_eq!(all, changed(), "every tab's rows together are all settings");
        let mut two = Settings::default();
        presets.all_tabs[1].apply(&mut two);
        assert_eq!(two.drawing.background, Background::Gradient);
        assert_eq!(two.inactive.opacity, 7);
        assert_eq!(two.active, Settings::default().active, "Active not held");
        assert_eq!(two.drawing.shadows, Settings::default().drawing.shadows);
    }

    #[test]
    fn rename_delete_replace_and_survive_a_restart() {
        let dir = std::env::temp_dir().join(format!("operandi-tab-presets-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("viewport-tab-presets.json");
        let mut presets = TabPresets::default();
        presets.add(true, "A", &[Tab::Drawing], changed());
        presets.add(true, "A", &[Tab::Drawing], Settings::default());
        assert_eq!(presets.one_tab.len(), 1, "same name, same tab: replaced");
        presets.add(true, "A", &[Tab::Visibility], changed());
        assert_eq!(presets.one_tab.len(), 2, "same name, other tab: both kept");
        assert!(presets.rename(true, 0, "B"));
        assert!(!presets.rename(true, 0, "  "), "blank names refused");
        presets.default = Some(changed());
        presets.save(&path).unwrap();
        let back = TabPresets::load(&path);
        assert_eq!(back, presets);
        let mut back = back;
        back.delete(true, 0);
        assert_eq!(back.one_tab[0].tabs, ["Visibility"]);
        assert_eq!(
            TabPresets::load(&dir.join("missing.json")),
            TabPresets::default()
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
