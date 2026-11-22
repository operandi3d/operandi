//! Value presets (UIC-55): a small downward arrow at the far right of a
//! numeric field opens a list of saved values for that property only.
//! Click a value to use it; (new) saves the current value under a name;
//! (delete) removes one. The list sorts by value or by name, and the
//! choice is remembered. Saved values live in `value-presets.json` in the
//! data folder and survive restarts.
//!
//! Any field can have presets: give it a key and (optionally) built-in
//! values in [`builtin`], then call [`arrow`] in its row. A property whose
//! list was never edited shows its built-ins; once edited, the stored list
//! replaces them (built-ins can be deleted too). Restore Defaults puts the
//! built-ins back.
use eframe::egui::{
    self, Align, Id, Key, Layout, PopupCloseBehavior, Response, RichText, Sense, TextEdit, Ui, vec2,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Refractive Index on a Material layer.
pub(crate) const REFRACTIVE_INDEX: &str = "material.refractiveIndex";
/// Dispersion on a Material layer.
pub(crate) const DISPERSION: &str = "material.dispersion";

/// The preset key of a Material channel row, if it has presets.
pub(crate) fn material_key(channel: &str) -> Option<&'static str> {
    match channel {
        "Refractive Index" => Some(REFRACTIVE_INDEX),
        "Dispersion" => Some(DISPERSION),
        _ => None,
    }
}

/// Built-in values for `key`, in the order Jason gave them.
pub(crate) fn builtin(key: &str) -> &'static [(&'static str, f64)] {
    match key {
        // Jason's list (2026-11-12); "Flourite" spelt as given.
        REFRACTIVE_INDEX => &[
            ("Air", 1.0),
            ("Carbon Dioxide (Liquid)", 1.2),
            ("Water", 1.33),
            ("Flourite", 1.434),
            ("Calspar2", 1.486),
            ("Zinc Crown Glass", 1.517),
            ("Zinc Crown Glass (High Dispersion)", 1.52),
            ("Sodium Chloride (Salt)", 1.544),
            ("Quartz", 1.553),
            ("Emerald", 1.57),
            ("Flint Glass (Light)", 1.575),
            ("Lapis Lazuli", 1.61),
            ("Topaz", 1.61),
            ("Flint Glass (Heavy)", 1.65),
            ("Calspar1", 1.66),
            ("Ruby/Sapphire", 1.77),
            ("Flint Glass (Heaviest)", 1.89),
            ("Crystal", 2.0),
            ("Diamond", 2.42),
            ("Chromium Oxide", 2.5),
            ("Copper Oxide", 2.705),
            ("Amorphous Selenium", 2.92),
            ("Iodine Crystal", 3.34),
        ],
        // Physical B−G dispersion figures (MAT-ED-Q01, Jason 2026-11-12):
        // picking one sets Dispersion to 100 % and Abbe Number from the
        // Refractive Index (operandi_core::dispersion).
        DISPERSION => &[
            ("Flourite", 0.007),
            ("Silica Glass", 0.01),
            ("Quartz", 0.013),
            ("Zircon", 0.038),
            ("Diamond", 0.044),
        ],
        _ => &[],
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Preset {
    pub name: String,
    pub value: f64,
}

/// How a list is ordered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SortBy {
    #[default]
    Value,
    Name,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct List {
    /// None: the built-ins (never edited).
    entries: Option<Vec<Preset>>,
    sort: SortBy,
}

/// Every property's saved values.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ValuePresets {
    lists: BTreeMap<String, List>,
}

impl ValuePresets {
    pub fn load(path: &Path) -> Self {
        let mut presets: Self = std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        for list in presets.lists.values_mut() {
            if let Some(entries) = &mut list.entries {
                entries.retain(|p| p.value.is_finite() && !p.name.trim().is_empty());
            }
        }
        presets
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        crate::resource_write::write_bytes(path, &temp, &bytes)
    }

    /// `key`'s values, in its sort order (ties keep their saved order).
    pub fn entries(&self, key: &str) -> Vec<Preset> {
        let list = self.lists.get(key);
        let mut entries: Vec<Preset> = match list.and_then(|l| l.entries.clone()) {
            Some(e) => e,
            None => builtin(key)
                .iter()
                .map(|(name, value)| Preset {
                    name: (*name).to_owned(),
                    value: *value,
                })
                .collect(),
        };
        match self.sort(key) {
            SortBy::Value => entries.sort_by(|a, b| a.value.total_cmp(&b.value)),
            SortBy::Name => entries.sort_by_key(|p| p.name.to_lowercase()),
        }
        entries
    }
    pub fn sort(&self, key: &str) -> SortBy {
        self.lists.get(key).map(|l| l.sort).unwrap_or_default()
    }
    pub fn set_sort(&mut self, key: &str, sort: SortBy) {
        self.lists.entry(key.to_owned()).or_default().sort = sort;
    }
    /// The editable list, starting from the built-ins.
    fn edit(&mut self, key: &str) -> &mut Vec<Preset> {
        let builtins = self.entries_unsorted_builtin(key);
        self.lists
            .entry(key.to_owned())
            .or_default()
            .entries
            .get_or_insert(builtins)
    }
    fn entries_unsorted_builtin(&self, key: &str) -> Vec<Preset> {
        builtin(key)
            .iter()
            .map(|(name, value)| Preset {
                name: (*name).to_owned(),
                value: *value,
            })
            .collect()
    }
    /// Save `value` as `name`; a preset of that name is replaced. False for
    /// an empty name or a value that isn't a number.
    pub fn add(&mut self, key: &str, name: &str, value: f64) -> bool {
        let name = name.trim();
        if name.is_empty() || !value.is_finite() {
            return false;
        }
        let list = self.edit(key);
        list.retain(|p| p.name != name);
        list.push(Preset {
            name: name.to_owned(),
            value,
        });
        true
    }
    /// Remove the preset called `name` (a built-in too).
    pub fn remove(&mut self, key: &str, name: &str) -> bool {
        let list = self.edit(key);
        let before = list.len();
        list.retain(|p| p.name != name);
        list.len() != before
    }
    /// Put the built-ins back (the sort choice stays).
    pub fn restore_defaults(&mut self, key: &str) {
        if let Some(list) = self.lists.get_mut(key) {
            list.entries = None;
        }
    }
    /// Whether `key` shows its built-ins unedited.
    pub fn is_default(&self, key: &str) -> bool {
        self.lists.get(key).is_none_or(|l| l.entries.is_none())
    }
}

/// What the user did in a preset list this frame.
#[derive(Default)]
pub(crate) struct Outcome {
    /// A value to put in the field.
    pub picked: Option<f64>,
    /// The saved lists changed: write them to disk.
    pub changed: bool,
}

/// Width of the arrow beside a field.
pub(crate) const ARROW_WIDTH: f32 = 14.;

/// The preset arrow for property `key` (UIC-55), drawn at the current
/// place in `ui` (in a right-to-left row, the far right). `current` is the
/// field's value, saved by (new); `format` shows a value as the field does.
pub(crate) fn arrow(
    ui: &mut Ui,
    key: &str,
    presets: &mut ValuePresets,
    current: f64,
    format: &dyn Fn(f64) -> String,
) -> Outcome {
    let mut out = Outcome::default();
    let height = ui.spacing().interact_size.y;
    let (rect, response) = ui.allocate_exact_size(vec2(ARROW_WIDTH, height), Sense::click());
    let response = response.on_hover_text("Presets: saved values for this property");
    let color = if response.hovered() {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    let c = rect.center();
    ui.painter().add(egui::Shape::convex_polygon(
        vec![c + vec2(-3.5, -1.5), c + vec2(3.5, -1.5), c + vec2(0., 2.5)],
        color,
        egui::Stroke::NONE,
    ));
    let naming_id = Id::new(("value-preset-naming", key));
    let deleting_id = Id::new(("value-preset-deleting", key));
    egui::Popup::menu(&response)
        .id(Id::new(("value-presets", key)))
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(220.);
            list_ui(
                ui,
                key,
                presets,
                current,
                format,
                naming_id,
                deleting_id,
                &mut out,
            );
        });
    if !egui::Popup::is_id_open(ui.ctx(), Id::new(("value-presets", key))) {
        ui.data_mut(|d| {
            d.remove::<String>(naming_id);
            d.remove::<bool>(deleting_id);
        });
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn list_ui(
    ui: &mut Ui,
    key: &str,
    presets: &mut ValuePresets,
    current: f64,
    format: &dyn Fn(f64) -> String,
    naming_id: Id,
    deleting_id: Id,
    out: &mut Outcome,
) {
    // (new): name the current value.
    if let Some(mut name) = ui.data(|d| d.get_temp::<String>(naming_id)) {
        ui.label(format!("Save {} as:", format(current)));
        let r = ui.add(TextEdit::singleline(&mut name).desired_width(200.));
        crate::numeric_tab::register(ui, &r, Some(&name));
        r.request_focus();
        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let mut done = false;
        ui.horizontal(|ui| {
            if (ui.button("Save").clicked() || enter) && presets.add(key, &name, current) {
                out.changed = true;
                done = true;
            }
            if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                done = true;
            }
        });
        ui.data_mut(|d| {
            if done {
                d.remove::<String>(naming_id);
            } else {
                d.insert_temp(naming_id, name);
            }
        });
        return;
    }
    let deleting = ui
        .data(|d| d.get_temp::<bool>(deleting_id))
        .unwrap_or(false);
    if deleting {
        ui.weak("Click a preset to delete it");
    }
    let entries = presets.entries(key);
    egui::ScrollArea::vertical()
        .max_height(320.)
        .show(ui, |ui| {
            for p in &entries {
                if entry_row(ui, &p.name, &format(p.value), deleting).clicked() {
                    if deleting {
                        out.changed |= presets.remove(key, &p.name);
                    } else {
                        out.picked = Some(p.value);
                        ui.close();
                    }
                }
            }
            if entries.is_empty() {
                ui.weak("No presets yet");
            }
        });
    ui.separator();
    if deleting {
        if ui.button("Done").clicked() {
            ui.data_mut(|d| d.remove::<bool>(deleting_id));
        }
        return;
    }
    if ui.selectable_label(false, "(new)").clicked() {
        ui.data_mut(|d| d.insert_temp(naming_id, String::new()));
    }
    use crate::unavailable::Gate;
    if ui
        .gated(
            crate::unavailable::unless(!entries.is_empty(), "No presets to delete"),
            egui::Button::selectable(false, "(delete)"),
        )
        .clicked()
    {
        ui.data_mut(|d| d.insert_temp(deleting_id, true));
    }
    ui.separator();
    let sort = presets.sort(key);
    for (label, s) in [
        ("Sort by Value", SortBy::Value),
        ("Sort by Name", SortBy::Name),
    ] {
        if ui.selectable_label(sort == s, label).clicked() && sort != s {
            presets.set_sort(key, s);
            out.changed = true;
        }
    }
    if !builtin(key).is_empty() && !presets.is_default(key) {
        ui.separator();
        if ui.selectable_label(false, "Restore Defaults").clicked() {
            presets.restore_defaults(key);
            out.changed = true;
        }
    }
}

/// One preset: its name on the left, its value on the right.
fn entry_row(ui: &mut Ui, name: &str, value: &str, deleting: bool) -> Response {
    let width = ui.available_width().max(200.);
    let height = ui.spacing().interact_size.y;
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, 2., ui.visuals().widgets.hovered.weak_bg_fill);
    }
    let inner = rect.shrink2(vec2(6., 0.));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let name = if deleting {
        RichText::new(format!("✕ {name}"))
    } else {
        RichText::new(name)
    };
    child.label(name);
    child.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.weak(value);
    });
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unedited_lists_show_the_builtins_sorted_by_value() {
        let p = ValuePresets::default();
        let e = p.entries(REFRACTIVE_INDEX);
        assert_eq!(e.len(), 23);
        assert_eq!(e[0].name, "Air");
        assert_eq!(e.last().unwrap().name, "Iodine Crystal");
        assert!(e.windows(2).all(|w| w[0].value <= w[1].value));
        assert!(e.iter().any(|x| x.name == "Flourite" && x.value == 1.434));
        let d = p.entries(DISPERSION);
        assert_eq!(
            d.iter().map(|x| x.value).collect::<Vec<_>>(),
            [0.007, 0.01, 0.013, 0.038, 0.044]
        );
        assert!(p.is_default(REFRACTIVE_INDEX));
    }

    #[test]
    fn sort_by_name_is_remembered() {
        let mut p = ValuePresets::default();
        p.set_sort(REFRACTIVE_INDEX, SortBy::Name);
        let e = p.entries(REFRACTIVE_INDEX);
        assert_eq!(e[0].name, "Air");
        assert_eq!(e[1].name, "Amorphous Selenium");
        // Sorting alone doesn't edit the list.
        assert!(p.is_default(REFRACTIVE_INDEX));
        assert_eq!(p.sort(DISPERSION), SortBy::Value);
    }

    #[test]
    fn new_saves_and_replaces_by_name() {
        let mut p = ValuePresets::default();
        assert!(p.add(REFRACTIVE_INDEX, " Glass ", 1.5));
        assert!(!p.add(REFRACTIVE_INDEX, "  ", 1.5));
        assert!(!p.add(REFRACTIVE_INDEX, "Bad", f64::NAN));
        assert!(p.add(REFRACTIVE_INDEX, "Glass", 1.52));
        let e = p.entries(REFRACTIVE_INDEX);
        assert_eq!(e.len(), 24);
        assert_eq!(e.iter().filter(|x| x.name == "Glass").count(), 1);
        assert!(e.iter().any(|x| x.name == "Glass" && x.value == 1.52));
        assert!(!p.is_default(REFRACTIVE_INDEX));
    }

    #[test]
    fn delete_removes_builtins_and_restore_brings_them_back() {
        let mut p = ValuePresets::default();
        p.set_sort(DISPERSION, SortBy::Name);
        assert!(p.remove(DISPERSION, "Diamond"));
        assert!(!p.remove(DISPERSION, "Diamond"));
        assert_eq!(p.entries(DISPERSION).len(), 4);
        p.restore_defaults(DISPERSION);
        assert_eq!(p.entries(DISPERSION).len(), 5);
        assert_eq!(p.sort(DISPERSION), SortBy::Name);
    }

    #[test]
    fn presets_survive_a_restart() {
        let dir = std::env::temp_dir().join(format!("value-presets-{}", std::process::id()));
        let path = dir.join("value-presets.json");
        let mut p = ValuePresets::default();
        p.add(REFRACTIVE_INDEX, "Mine", 1.45);
        p.remove(REFRACTIVE_INDEX, "Air");
        p.set_sort(REFRACTIVE_INDEX, SortBy::Name);
        p.save(&path).unwrap();
        let back = ValuePresets::load(&path);
        assert_eq!(back, p);
        assert!(
            back.entries(REFRACTIVE_INDEX)
                .iter()
                .all(|x| x.name != "Air")
        );
        std::fs::remove_dir_all(&dir).ok();
        // A missing or broken file is the built-ins.
        assert_eq!(ValuePresets::load(&path), ValuePresets::default());
    }

    #[test]
    fn only_listed_material_rows_have_presets() {
        assert_eq!(material_key("Refractive Index"), Some(REFRACTIVE_INDEX));
        assert_eq!(material_key("Dispersion"), Some(DISPERSION));
        assert_eq!(material_key("Roughness"), None);
        assert!(builtin("other").is_empty());
    }
}
