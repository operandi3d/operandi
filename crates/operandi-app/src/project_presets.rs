//! Project system phase 3 (docs/PROJECT_SYSTEM.md): a project can hold
//! shared presets (panel layout, viewport display, input map) in its
//! `workspaces/` folder. Jason's rule (2026-11-01): your own settings win.
//! Nothing is applied automatically; File > Project Presets loads a project
//! preset only when asked (it then becomes your setting), or saves yours into
//! the project for others.
//!
//! Loading reads the project file through the same loader as your own file.
//! A damaged project file is refused, leaving your setting unchanged, rather
//! than silently resetting you to the defaults.
use super::*;
use crate::prefs_file::PrefsFile;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Preset {
    Layout,
    ViewportDisplay,
    InputMap,
}

impl Preset {
    pub(super) const ALL: [Preset; 3] = [Preset::Layout, Preset::ViewportDisplay, Preset::InputMap];

    pub(super) fn label(self) -> &'static str {
        match self {
            Preset::Layout => "Layout",
            Preset::ViewportDisplay => "Viewport Display",
            Preset::InputMap => "Input Map",
        }
    }

    /// Its name on a command line (`operandi.project.loadPreset`).
    pub(super) fn script_name(self) -> &'static str {
        match self {
            Preset::Layout => "layout",
            Preset::ViewportDisplay => "viewportDisplay",
            Preset::InputMap => "inputMap",
        }
    }

    /// File name inside the project's workspaces folder (same names as the
    /// per-user files, so the formats stay identical).
    fn file(self) -> &'static str {
        match self {
            Preset::Layout => "layout.json",
            Preset::ViewportDisplay => "viewport-display.json",
            Preset::InputMap => "input-map.json",
        }
    }
}

impl Operandi {
    /// The project's copy of `preset`, if a project is set.
    pub(super) fn project_preset_path(&self, preset: Preset) -> Option<PathBuf> {
        let project = self.project.as_ref()?;
        let folder = project.folder("workspaces")?;
        Some(folder.join(preset.file()))
    }

    /// True when the active project has its own copy of `preset`.
    pub(super) fn project_has_preset(&self, preset: Preset) -> bool {
        self.project_preset_path(preset)
            .is_some_and(|p| p.is_file())
    }

    /// Write your current `preset` into the project's workspaces folder.
    pub(super) fn save_project_preset(&mut self, ctx: &Context, preset: Preset) {
        let Some(path) = self.project_preset_path(preset) else {
            self.status = "Set a project first (File > Set Project…)".into();
            return;
        };
        let result = (|| -> std::io::Result<()> {
            if let Some(folder) = path.parent() {
                std::fs::create_dir_all(folder)?;
            }
            let bytes = match preset {
                Preset::Layout => self.layout_json(ctx).into_bytes(),
                Preset::ViewportDisplay => {
                    return self.viewport_preferences.save(&path);
                }
                Preset::InputMap => {
                    // The global map: a layout's own keys go with its layout file.
                    serde_json::to_vec_pretty(&self.global_bindings)
                        .map_err(std::io::Error::other)?
                }
            };
            let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
            resource_write::write_bytes(&path, &temp, &bytes)
        })();
        self.status = match result {
            Ok(()) => format!("Saved your {} to the project", preset.label()),
            Err(e) => format!("Couldn't save the {} to the project: {e}", preset.label()),
        };
    }

    /// Replace your `preset` with the project's copy and keep it as yours.
    pub(super) fn load_project_preset(&mut self, ctx: &Context, preset: Preset) {
        let Some(path) = self.project_preset_path(preset).filter(|p| p.is_file()) else {
            self.status = format!("This project has no {} preset", preset.label());
            return;
        };
        let readable = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .is_some_and(|v| v.is_object());
        let saved = match preset {
            _ if !readable => Err("the file is damaged".to_string()),
            Preset::Layout => {
                self.load_layout_from(ctx, &path);
                // persist_layout writes it to your file on the next frame.
                Ok(())
            }
            Preset::ViewportDisplay => match viewport_preferences::ViewportPreferences::read(&path)
            {
                None => Err("the file is damaged".to_string()),
                Some(preferences) => {
                    self.viewport_preferences = preferences;
                    self.viewport_preferences
                        .save(&self.environment.paths().viewport_display())
                        .map_err(|e| e.to_string())
                }
            },
            Preset::InputMap => {
                let bindings = input::load_bindings(&path);
                if !input::bindings_valid(&bindings) {
                    Err("its key assignments conflict".to_string())
                } else {
                    let own = self.environment.paths().input_map();
                    let temp = own.with_extension(format!("json.tmp-{}", std::process::id()));
                    serde_json::to_vec_pretty(&bindings)
                        .map_err(|e| e.to_string())
                        .and_then(|b| {
                            resource_write::write_bytes(&own, &temp, &b).map_err(|e| e.to_string())
                        })
                        .map(|()| {
                            // The current layout's own keys stay on top.
                            self.global_bindings = bindings;
                            self.refresh_bindings();
                        })
                }
            }
        };
        self.status = match saved {
            Ok(()) => format!("Loaded the project's {}", preset.label()),
            Err(e) => format!("Couldn't load the project's {}: {e}", preset.label()),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operandi_core::Project;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "operandi-presets-{}-{}-{name}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn app_with_project(name: &str) -> (Operandi, Context, PathBuf) {
        let mut a = Operandi::empty();
        a.initialize_new_document();
        let root = scratch(name);
        a.set_project(Project::create(&root, "Presets").unwrap());
        (a, Context::default(), root)
    }

    #[test]
    fn without_a_project_nothing_is_offered() {
        let mut a = Operandi::empty();
        let ctx = Context::default();
        for p in Preset::ALL {
            assert!(!a.project_has_preset(p));
            a.save_project_preset(&ctx, p);
            assert!(a.status.contains("Set a project"), "{}", a.status);
        }
        let file = &a.menu_model(&ctx)[0];
        let leaves = file.leaves();
        for p in Preset::ALL {
            for action in [
                format!("operandi.project.loadPreset preset:{}", p.script_name()),
                format!("operandi.project.savePreset preset:{}", p.script_name()),
            ]
            .map(|line| crate::app_menu::line_action(&line))
            {
                let item = leaves
                    .iter()
                    .find(|i| i.action == action)
                    .expect("menu item");
                assert!(!item.enabled, "{action:?} disabled without a project");
            }
        }
    }

    #[test]
    fn save_then_load_round_trips_each_preset_and_yours_wins_until_loaded() {
        let (mut a, ctx, root) = app_with_project("roundtrip");
        // Your settings: a custom key and opacity. (Not the old factory
        // Shift+B, which loading migrates to the measured B, KEY-01/02.)
        a.global_bindings
            .insert("Bevel".into(), "Alt+Shift+B".into());
        a.refresh_bindings();
        a.viewport_preferences.workplane_opacity = 70;
        a.scene_fraction = 0.4;
        for p in Preset::ALL {
            a.save_project_preset(&ctx, p);
            assert!(root.join("workspaces").join(p.file()).is_file(), "{p:?}");
            assert!(a.project_has_preset(p));
        }
        // Change your own settings; the project's copies are untouched and
        // nothing is applied automatically, even when the project is set again.
        a.global_bindings.insert("Bevel".into(), "Ctrl+B".into());
        a.refresh_bindings();
        a.viewport_preferences.workplane_opacity = 20;
        a.scene_fraction = 0.6;
        a.set_project(Project::open(&root).unwrap());
        assert_eq!(a.bindings["Bevel"], "Ctrl+B");
        assert_eq!(a.viewport_preferences.workplane_opacity, 20);
        // Loading applies the project's copy and keeps it as yours.
        for p in Preset::ALL {
            a.load_project_preset(&ctx, p);
            assert!(a.status.starts_with("Loaded"), "{p:?}: {}", a.status);
        }
        assert_eq!(a.bindings["Bevel"], "Alt+Shift+B");
        assert_eq!(a.viewport_preferences.workplane_opacity, 70);
        assert!((a.scene_fraction - 0.4).abs() < 1e-6);
        let own = input::load_bindings(&a.environment.paths().input_map());
        assert_eq!(own["Bevel"], "Alt+Shift+B", "saved as your input map");
        let display = viewport_preferences::ViewportPreferences::load(
            &a.environment.paths().viewport_display(),
        );
        assert_eq!(display.workplane_opacity, 70, "saved as your display");
    }

    #[test]
    fn damaged_or_conflicting_project_presets_are_refused() {
        let (mut a, ctx, root) = app_with_project("damaged");
        let folder = root.join("workspaces");
        a.viewport_preferences.workplane_opacity = 55;
        std::fs::write(folder.join("viewport-display.json"), b"{not json").unwrap();
        a.load_project_preset(&ctx, Preset::ViewportDisplay);
        assert!(a.status.contains("damaged"), "{}", a.status);
        assert_eq!(a.viewport_preferences.workplane_opacity, 55, "yours kept");

        let before = a.bindings.clone();
        let mut clash = a.bindings.clone();
        clash.insert("Bevel".into(), "Q".into());
        clash.insert("Extrude".into(), "Q".into());
        std::fs::write(
            folder.join("input-map.json"),
            serde_json::to_vec(&clash).unwrap(),
        )
        .unwrap();
        a.load_project_preset(&ctx, Preset::InputMap);
        assert!(a.status.contains("conflict"), "{}", a.status);
        assert_eq!(a.bindings, before, "yours kept");
    }
}
