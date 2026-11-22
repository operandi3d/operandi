//! Local original profile library. Navigation is presentation state; only an
//! explicit shape click returns a modeling selection to the caller. No scene
//! conversion, external preset decoder or proprietary library is implied.
use super::*;
use crate::unavailable::Gate;
#[derive(Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum Folder {
    #[default]
    Library,
    Original,
}
#[derive(Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum Presentation {
    #[default]
    List,
    Grid,
}
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct ProfileBrowser {
    folder: Folder,
    view: Presentation,
    search: String,
    #[serde(skip)]
    history: Vec<Folder>,
    #[serde(skip)]
    cursor: usize,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredBrowser {
    version: u8,
    browser: ProfileBrowser,
}
impl ProfileBrowser {
    fn navigate(&mut self, folder: Folder) {
        if self.folder != folder {
            self.history.truncate(self.cursor + 1);
            self.history.push(folder);
            self.cursor = self.history.len() - 1;
            self.folder = folder;
        }
    }
    pub(super) fn load(path: &std::path::Path) -> Self {
        std::fs::metadata(path)
            .ok()
            .filter(|m| m.len() <= 8192)
            .and_then(|_| std::fs::read(path).ok())
            .and_then(|b| serde_json::from_slice::<StoredBrowser>(&b).ok())
            .filter(|b| b.version == 1 && b.browser.search.len() <= 1024)
            .map(|b| b.browser)
            .unwrap_or_default()
    }
    pub(super) fn save(&self, path: &std::path::Path) -> Result<()> {
        if self.search.len() > 1024 {
            return Err("Profile search exceeds 1024 bytes".into());
        }
        // History is deliberately session-local; only folder/view/filter persist.
        let data = serde_json::to_vec(&StoredBrowser {
            version: 1,
            browser: Self {
                folder: self.folder,
                view: self.view,
                search: self.search.clone(),
                ..Default::default()
            },
        })
        .map_err(|e| e.to_string())?;
        let temp = path.with_extension(format!("tmp-{}", std::process::id()));
        super::resource_write::write_bytes(path, &temp, &data).map_err(|e| e.to_string())
    }
    pub(super) fn show(
        &mut self,
        ui: &mut Ui,
        editors: &mut std::collections::HashSet<Id>,
        selected: Option<BevelProfile>,
    ) -> Option<BevelProfile> {
        if self.history.is_empty() {
            self.history.push(self.folder);
        }
        ui.horizontal(|ui| {
            if ui
                .gated(
                    crate::unavailable::unless(self.cursor > 0, "Nothing to go back to"),
                    Button::new("Back"),
                )
                .clicked()
            {
                self.cursor -= 1;
                self.folder = self.history[self.cursor];
            }
            if ui
                .gated(
                    crate::unavailable::unless(
                        self.cursor + 1 < self.history.len(),
                        "Nothing to go forward to",
                    ),
                    Button::new("Forward"),
                )
                .clicked()
            {
                self.cursor += 1;
                self.folder = self.history[self.cursor];
            }
            if ui
                .gated(
                    crate::unavailable::unless(
                        self.folder != Folder::Library,
                        "Already at the top folder",
                    ),
                    Button::new("Parent"),
                )
                .clicked()
            {
                self.navigate(Folder::Library);
            }
        });
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.view, Presentation::List, "List");
            ui.selectable_value(&mut self.view, Presentation::Grid, "Grid");
        });
        let response = ui.add(
            TextEdit::singleline(&mut self.search)
                .hint_text("Find original profiles")
                .char_limit(256),
        );
        editors.insert(response.id);
        crate::numeric_tab::register(ui, &response, Some(&self.search));
        let mut choice = None;
        match self.folder {
            Folder::Library => {
                ui.weak("Local Profile Library");
                if ui.button("Original Shapes").clicked() {
                    self.navigate(Folder::Original);
                }
            }
            Folder::Original => {
                ui.weak("Local Profile Library / Original Shapes");
                let filter = self.search.to_lowercase();
                let entries = [
                    (BevelProfile::Linear, "Original Linear"),
                    (BevelProfile::Smooth, "Original Smooth"),
                ];
                let grid = self.view == Presentation::Grid;
                let mut cards = |ui: &mut Ui| {
                    for (shape, name) in entries {
                        if !name.to_lowercase().contains(&filter) {
                            continue;
                        }
                        ui.vertical(|ui| {
                            let size = if grid {
                                vec2(92., 60.)
                            } else {
                                vec2(ui.available_width(), 34.)
                            };
                            let (rect, response) = ui.allocate_exact_size(size, Sense::click());
                            let visuals = ui
                                .style()
                                .interact_selectable(&response, selected == Some(shape));
                            ui.painter().rect_filled(rect, 2., visuals.bg_fill);
                            let points = shape
                                .samples(&BevelOptions::default())
                                .expect("Library shape has a bounded default sample count");
                            let plot = rect.shrink(5.);
                            let positions: Vec<_> = points
                                .iter()
                                .map(|p| {
                                    pos2(
                                        plot.left() + p.x as f32 * plot.width(),
                                        plot.bottom() - p.y as f32 * plot.height(),
                                    )
                                })
                                .collect();
                            ui.painter().add(Shape::line(
                                positions,
                                Stroke::new(1.5_f32, visuals.fg_stroke.color),
                            ));
                            let click = response.on_hover_text(name).clicked();
                            let label = ui.selectable_label(selected == Some(shape), name);
                            if click || label.clicked() {
                                choice = Some(shape);
                            }
                        });
                    }
                };
                if grid {
                    ui.horizontal_wrapped(&mut cards);
                } else {
                    cards(ui);
                }
            }
        }
        ui.weak("Original local shapes only. Scene Item conversion and external preset decoding are unavailable.");
        choice
    }
}
impl Operandi {
    pub(super) fn profile_browser_panel(&mut self, ui: &mut Ui) {
        let before = serde_json::to_vec(&self.profile_browser).ok();
        let choice =
            self.profile_browser
                .show(ui, &mut self.text_editors, self.editor.bevel_profile);
        if let Some(shape) = choice
            && self.confirm_fields()
        {
            self.run(Command::BevelProfile(Some(shape)));
            self.clear_fields();
        }
        if before != serde_json::to_vec(&self.profile_browser).ok()
            && let Err(e) = self
                .profile_browser
                .save(&self.environment.paths().profile_browser())
        {
            self.status = format!("Profile browser preferences: {e}");
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disk_roundtrip_defaults_byte_limits_and_occupied_temp() {
        let root = std::env::temp_dir().join(format!(
            "operandi-browser-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("profile-browser.json");
        let temp = path.with_extension(format!("tmp-{}", std::process::id()));
        let mut browser = ProfileBrowser {
            folder: Folder::Original,
            view: Presentation::Grid,
            search: "é".repeat(512),
            history: vec![Folder::Library, Folder::Original],
            cursor: 1,
        };
        browser.save(&path).unwrap();
        let restored = ProfileBrowser::load(&path);
        assert!(restored.folder == Folder::Original && restored.view == Presentation::Grid);
        assert_eq!(restored.search, browser.search);
        assert!(restored.history.is_empty());
        assert_eq!(restored.cursor, 0);
        assert!(!temp.exists());
        let original = std::fs::read(&path).unwrap();
        std::fs::write(&temp, b"occupied").unwrap();
        browser.search.push('é');
        assert_eq!(
            browser.save(&path).unwrap_err(),
            "Profile search exceeds 1024 bytes"
        );
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(std::fs::read(&temp).unwrap(), b"occupied");
        browser.search = "valid".into();
        assert!(browser.save(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(std::fs::read(&temp).unwrap(), b"occupied");
        std::fs::remove_file(&temp).unwrap();
        browser.save(&path).unwrap();
        assert_eq!(ProfileBrowser::load(&path).search, "valid");
        assert!(!temp.exists());
        for invalid in [
            "invalid".to_owned(),
            r#"{"version":2,"browser":{}}"#.to_owned(),
            r#"{"version":1,"browser":{"unknown":true}}"#.to_owned(),
            " ".repeat(8193),
            serde_json::json!({"version":1,"browser":{"search":"é".repeat(513)}}).to_string(),
        ] {
            std::fs::write(&path, invalid).unwrap();
            let loaded = ProfileBrowser::load(&path);
            assert!(loaded.folder == Folder::Library && loaded.view == Presentation::List);
            assert!(loaded.search.is_empty() && loaded.history.is_empty());
            assert_eq!(loaded.cursor, 0);
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn navigation_branches_without_modeling_state() {
        let mut b = ProfileBrowser::default();
        b.history.push(Folder::Library);
        b.navigate(Folder::Original);
        assert_eq!(b.cursor, 1);
        b.cursor = 0;
        b.folder = b.history[0];
        b.navigate(Folder::Original);
        assert_eq!(b.history.len(), 2);
        b.navigate(Folder::Library);
        assert_eq!(b.history.len(), 3);
        let bytes = serde_json::to_vec(&b).unwrap();
        let restored: ProfileBrowser = serde_json::from_slice(&bytes).unwrap();
        assert!(restored.history.is_empty());
        assert!(restored.folder == Folder::Library);
    }
}
