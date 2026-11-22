//! Bevel presentation and durable recipes; source scenes never enter this file.
use super::*;
use crate::tool_form::{self as tf, Item, MenuEntry, ToolForm};
#[derive(Default)]
pub(super) struct BevelUi {}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipes {
    version: u8,
    entries: BTreeMap<String, BevelRecipe>,
    selected: Option<String>,
}
impl Operandi {
    pub(super) fn load_bevel_recipes(&mut self) {
        self.load_bevel_recipes_at(&self.environment.paths().bevel_recipes());
    }
    fn load_bevel_recipes_at(&mut self, path: &std::path::Path) {
        if !path.exists() {
            return;
        }
        let result = (|| -> Result<Recipes> {
            if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 4_194_304 {
                return Err("Bevel recipe file exceeds size limit".into());
            }
            let r: Recipes =
                serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            if r.version != 1
                || r.entries.len() > 1024
                || r.entries.keys().any(|n| {
                    n.trim().is_empty() || n.len() > 256 || n.chars().any(char::is_control)
                })
                || r.selected
                    .as_ref()
                    .is_some_and(|n| !r.entries.contains_key(n))
            {
                return Err("Invalid Bevel recipe library".into());
            }
            for recipe in r.entries.values() {
                recipe.validate()?;
            }
            Ok(r)
        })();
        match result {
            Ok(r) => {
                self.editor.bevel_states = r.entries;
                self.editor.bevel_state = r.selected;
            }
            Err(e) => self.status = format!("Bevel recipes retained on disk: {e}"),
        }
    }
    pub(super) fn save_bevel_recipes(&self) -> Result<()> {
        self.save_bevel_recipes_at(&self.environment.paths().bevel_recipes())
    }
    fn save_bevel_recipes_at(&self, path: &std::path::Path) -> Result<()> {
        let data = serde_json::to_vec(&Recipes {
            version: 1,
            entries: self.editor.bevel_states.clone(),
            selected: self.editor.bevel_state.clone(),
        })
        .map_err(|e| e.to_string())?;
        if data.len() > 4_194_304 || self.editor.bevel_states.len() > 1024 {
            return Err("Bevel recipe library exceeds size limit".into());
        }
        let temp = path.with_extension(format!("tmp-{}", std::process::id()));
        super::resource_write::write_bytes(path, &temp, &data).map_err(|e| e.to_string())
    }
    /// Bevel's Apply, Profiles, Content Browse and Tool States rows, under
    /// its options.
    pub(super) fn bevel_extras(&self, l: &Live, f: &mut ToolForm<Parameters>) {
        let profile = self.editor.bevel_profile;
        // A pick that changes the stage confirms pending text first and
        // then shows the values it brought in.
        fn confirmed(run: impl Fn(&mut Operandi) + 'static) -> impl Fn(&mut Operandi) + 'static {
            move |app| {
                if app.confirm_fields() {
                    run(app);
                    app.clear_fields();
                }
            }
        }
        let profiles: Vec<MenuEntry> = [
            (None, "None"),
            (Some(BevelProfile::Linear), "Original Linear"),
            (Some(BevelProfile::Smooth), "Original Smooth"),
        ]
        .into_iter()
        .map(|(shape, name)| {
            MenuEntry::new(
                name,
                profile == shape,
                confirmed(move |app| app.run(Command::BevelProfile(shape))),
            )
        })
        .collect();
        let title = match profile {
            None => "None",
            Some(BevelProfile::Linear) => "Original Linear",
            Some(BevelProfile::Smooth) => "Original Smooth",
        };
        let current = self.editor.bevel_state.clone();
        let mut states = vec![MenuEntry::new("None", current.is_none(), |app| {
            app.run(Command::BevelStateRecall(None))
        })];
        for name in self.editor.bevel_states.keys() {
            let picked = name.clone();
            states.push(MenuEntry::new(
                name,
                current.as_ref() == Some(name),
                confirmed(move |app| app.run(Command::BevelStateRecall(Some(picked.clone())))),
            ));
        }
        let some = current.is_some();
        let name = |app: &Operandi| app.draft_text(STATE_NAME, "");
        f.wide(
            tf::Button::command("Apply", Command::Apply)
                .enabled_if(!l.applied && self.field_errors.is_empty(), if l.applied { "Applied; change a value to apply again" } else { crate::unavailable::FIELD_ERRORS }),
        )
        .section(
            "Profiles",
            true,
            vec![
                Item::Menu {
                    key: "bevel_profile".into(),
                    label: "Profile".into(),
                    shown: title.into(),
                    entries: profiles,
                    disabled: None,
                },
                // Spec: BV-Q02
                Item::Note("Built-in profile shapes: counts 2, 3, 4 and 8. Count 1 uses the sampled original shape.".into()),
            ],
        )
        .section(
            "Content Browse",
            true,
            vec![Item::Custom(std::rc::Rc::new(|app: &mut Operandi, ui: &mut Ui| {
                app.profile_browser_panel(ui)
            }))],
        )
        .section(
            "Tool States",
            true,
            vec![
                Item::Menu {
                    key: "bevel_recipe".into(),
                    label: "Tool State".into(),
                    shown: current.clone().unwrap_or_else(|| "None".into()),
                    entries: states,
                    disabled: None,
                },
                Item::Field(Box::new(tf::draft(STATE_NAME, "Name", ""))),
                Item::Buttons(vec![
                    tf::Button::run("Save", move |app| {
                        let n = name(app);
                        app.run(Command::BevelStateSave(n))
                    }),
                    tf::Button::run("Rename", move |app| {
                        let n = name(app);
                        app.run(Command::BevelStateRename(n))
                    })
                    .enabled_if(some, "Choose a saved state first"),
                    tf::Button::command("Delete", Command::BevelStateDelete).enabled_if(some, "Choose a saved state first"),
                    tf::Button::run(
                        "Reset Tool State",
                        confirmed(|app| app.run(Command::ResetBevelState)),
                    )
                    .enabled_if(some, "Choose a saved state first"),
                ]),
            ],
        )
        .buttons(vec![tf::Button::command(
            "Reset Tool Attributes",
            Command::ResetBevel,
        )]);
    }
}

/// The Tool State name field's key.
const STATE_NAME: &str = "bevel_state_name";

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recipes_restart_validate_versions_and_preserve_existing_files_on_failure() {
        let directory = std::env::temp_dir().join(format!(
            "operandi-bevel-recipes-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("bevel-recipes.json");
        let mut app = Operandi::empty();
        app.editor.bevel_states.insert(
            "Original".into(),
            BevelRecipe {
                shift: 0.2,
                inset: 0.1,
                grouped: false,
                options: BevelOptions {
                    segments: 3,
                    uvs: BevelUvs::V,
                    shift_range: 0.03,
                    inset_range: -0.02,
                    falloff: BevelFalloff::Vertex,
                    keep_aspect: false,
                    ..Default::default()
                },
            },
        );
        app.editor.bevel_state = Some("Original".into());
        app.editor.bevel_profile = Some(BevelProfile::Smooth);
        app.save_bevel_recipes_at(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let mut restarted = Operandi::empty();
        let scene = restarted.editor.scene.clone();
        restarted.load_bevel_recipes_at(&path);
        assert_eq!(restarted.editor.bevel_states, app.editor.bevel_states);
        assert_eq!(restarted.editor.bevel_state, app.editor.bevel_state);
        assert_eq!(restarted.editor.bevel_profile, None);
        assert_eq!(restarted.editor.scene, scene);
        assert!(restarted.editor.undo.is_empty());
        // A stale staging file fails safely; it cannot truncate a valid library.
        let temp = path.with_extension(format!("tmp-{}", std::process::id()));
        std::fs::write(&temp, b"occupied").unwrap();
        assert!(app.save_bevel_recipes_at(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(std::fs::read(&temp).unwrap(), b"occupied");
        let recipe = app.editor.bevel_states["Original"].clone();
        for index in 0..1024 {
            app.editor
                .bevel_states
                .insert(format!("Extra {index}"), recipe.clone());
        }
        assert_eq!(
            app.save_bevel_recipes_at(&path).unwrap_err(),
            "Bevel recipe library exceeds size limit"
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(std::fs::read(&temp).unwrap(), b"occupied");
        let mut invalid: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        invalid["version"] = 2.into();
        std::fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
        let before = restarted.editor.clone();
        restarted.load_bevel_recipes_at(&path);
        assert!(restarted.editor == before);
        assert!(restarted.status.contains("retained on disk"));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
