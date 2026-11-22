//! User colour theme (Preferences > Display > Colors).
//!
//! The built-in dark theme (theme.rs) is the default. The user's file holds
//! only the colours they changed, by stable key, so every colour they never
//! touched keeps following the default when it changes in a later version.
//! Saved schemes use the same format, one file per scheme. Schemes saved
//! before Steel became the default fill omitted colours from Classic Grey.
use crate::theme::presets::Preset;
use crate::theme::{self, COLORS, ColorId};
use eframe::egui::Color32;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

/// The largest preferences or scheme file read; far above a full theme.
const MAX_BYTES: u64 = 64 * 1024;
const MAX_NAME: usize = 64;

/// The colours the user changed; anything missing uses the default.
pub type Overrides = BTreeMap<ColorId, Color32>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scheme {
    BuiltIn(Preset),
    Saved(String),
}

impl Scheme {
    pub fn name(&self) -> &str {
        match self {
            Self::BuiltIn(preset) => preset.name(),
            Self::Saved(name) => name,
        }
    }

    pub fn colors(&self, dir: &Path) -> Option<Overrides> {
        match self {
            Self::BuiltIn(preset) => Some(
                preset
                    .colors()
                    .filter(|(id, color)| {
                        id.follows_accent_by_default() || *color != id.entry().default
                    })
                    .collect(),
            ),
            Self::Saved(name) => Some(ColorFile::read(&scheme_path(dir, name))?.saved_overrides()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorFile {
    version: u8,
    /// The scheme last loaded or saved, shown in the Scheme dropdown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scheme: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    built_in: bool,
    #[serde(default, deserialize_with = "read_colors")]
    colors: BTreeMap<String, String>,
    /// In new files a missing swatch follows Accent, and a present RGB is
    /// explicit even if it matches the old default. Older readers ignore
    /// this flag and can still load the version 1 file.
    #[serde(default)]
    accent_links: bool,
    /// Selection swatches gained Accent links after the original four.
    #[serde(default)]
    selection_accent_links: bool,
    /// New version-1 files use Steel as the base for omitted colours.
    #[serde(default)]
    steel_defaults: bool,
}

impl ColorFile {
    pub fn new(overrides: &Overrides, scheme: Option<String>) -> Self {
        Self {
            version: 1,
            scheme,
            built_in: false,
            accent_links: true,
            selection_accent_links: true,
            steel_defaults: true,
            colors: overrides
                .iter()
                .map(|(id, c)| (id.entry().key.to_owned(), hex(*c)))
                .collect(),
        }
    }
    pub fn for_preferences(scheme: Option<&Scheme>) -> Self {
        let mut file = Self::new(&current(), scheme.map(|s| s.name().to_owned()));
        file.built_in = matches!(scheme, Some(Scheme::BuiltIn(_)));
        file
    }

    fn selected_scheme(&self) -> Option<Scheme> {
        let name = self.scheme.as_ref()?;
        if self.built_in {
            Preset::ALL
                .into_iter()
                .find(|p| p.name() == name)
                .map(Scheme::BuiltIn)
        } else {
            clean_name(name)
                .filter(|clean| clean == name)
                .map(Scheme::Saved)
        }
    }

    fn valid_colors(&self) -> Overrides {
        self.colors
            .iter()
            .filter_map(|(key, value)| Some((ColorId::from_key(key)?, parse_hex(value)?)))
            .collect()
    }
    /// Explicit RGB values, with the legacy Accent-link migration applied.
    fn explicit_colors(&self) -> Overrides {
        let mut colors = self.valid_colors();
        colors.retain(|id, color| {
            if matches!(id, ColorId::Selection | ColorId::SelectionFill) {
                let was_default = id.legacy_accent_default() == Some(*color)
                    || (!self.steel_defaults && *color == id.entry().classic_default);
                self.selection_accent_links || self.built_in || !was_default
            } else {
                self.accent_links || id.legacy_accent_default() != Some(*color)
            }
        });
        colors
    }

    /// The colours this file changes. Unknown keys and unreadable values
    /// are skipped.
    pub fn overrides(&self) -> Overrides {
        let mut colors = self.explicit_colors();
        discard_defaults(&mut colors);
        colors
    }

    fn saved_overrides(&self) -> Overrides {
        let mut colors = self.explicit_colors();
        if !self.selection_accent_links
            && self.selected_scheme() == Some(Scheme::BuiltIn(Preset::Steel))
        {
            // Old preferences omitted Steel's explicit selection colours
            // because they matched the factory defaults at the time.
            for id in [ColorId::Selection, ColorId::SelectionFill] {
                colors.entry(id).or_insert(id.entry().default);
            }
        }
        if !self.steel_defaults {
            for (id, color) in Preset::ClassicGrey.colors() {
                if !matches!(id, ColorId::Selection | ColorId::SelectionFill) {
                    colors.entry(id).or_insert(color);
                }
            }
        }
        discard_defaults(&mut colors);
        colors
    }
    /// The file at `path`, or None when it is missing or invalid.
    pub fn read(path: &Path) -> Option<Self> {
        fs::metadata(path)
            .ok()
            .filter(|m| m.len() <= MAX_BYTES)
            .and_then(|_| fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .filter(|f| f.version == 1)
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        crate::resource_write::write_bytes(path, &temp, &bytes)
    }
}

fn discard_defaults(colors: &mut Overrides) {
    colors.retain(|id, color| id.follows_accent_by_default() || *color != id.entry().default);
}

fn read_colors<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    let values = BTreeMap::<String, serde_json::Value>::deserialize(deserializer)?;
    Ok(values
        .into_iter()
        .filter_map(|(key, value)| match value {
            serde_json::Value::String(value) => Some((key, value)),
            _ => None,
        })
        .collect())
}

pub fn export_scheme(path: &Path) -> io::Result<()> {
    let colors = COLORS.iter().map(|e| (e.id, theme::color(e.id))).collect();
    ColorFile::new(&colors, None).save(path)
}

/// Import as a new saved scheme, leaving any same-named saved scheme intact.
pub fn import_scheme(source: &Path, dir: &Path) -> io::Result<(Scheme, Overrides, usize)> {
    let invalid = |message| io::Error::new(io::ErrorKind::InvalidData, message);
    let file = ColorFile::read(source).ok_or_else(|| invalid("Invalid colour scheme JSON"))?;
    let colors = file.valid_colors();
    if colors.is_empty() {
        return Err(invalid("No valid colours found"));
    }
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .and_then(clean_name)
        .ok_or_else(|| invalid("The filename needs a usable scheme name"))?;
    let mut name = stem.clone();
    let mut suffix = 2;
    while scheme_path(dir, &name).try_exists()? {
        let tail = format!(" ({suffix})");
        let prefix: String = stem.chars().take(MAX_NAME - tail.len()).collect();
        name = format!("{}{tail}", prefix.trim_end());
        suffix += 1;
    }
    let count = colors.len();
    // Keep the file's Accent-link semantics and discard unrelated metadata.
    let file = ColorFile::new(&file.explicit_colors(), None);
    file.save(&scheme_path(dir, &name))?;
    Ok((Scheme::Saved(name), file.overrides(), count))
}

/// The user's current overrides (what theme.rs draws with).
pub fn current() -> Overrides {
    COLORS
        .iter()
        .filter_map(|e| Some((e.id, theme::override_of(e.id)?)))
        .collect()
}

/// Loads the user's colours at launch and applies them. Returns the scheme
/// name the file remembers.
pub fn load(path: &Path) -> Option<Scheme> {
    let file = ColorFile::read(path).unwrap_or_default();
    let scheme = file.selected_scheme();
    let colors = if scheme.is_some() {
        file.saved_overrides()
    } else {
        file.overrides()
    };
    theme::set_overrides(&colors);
    scheme
}

pub fn hex(c: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}
pub fn parse_hex(s: &str) -> Option<Color32> {
    let s = s.strip_prefix('#')?;
    if s.len() != 6 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
    Some(Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?))
}

/// A scheme name made safe as a file name, or None when nothing usable is left.
pub fn clean_name(name: &str) -> Option<String> {
    let name: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '(' | ')'))
        .take(MAX_NAME)
        .collect();
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

pub fn scheme_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.json"))
}

/// Saved scheme names, sorted.
pub fn schemes(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            (path.extension()? == "json")
                .then(|| path.file_stem()?.to_str().map(str::to_owned))
                .flatten()
        })
        .filter(|n| clean_name(n).as_deref() == Some(n.as_str()))
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCENT_SWATCHES: [(ColorId, fn() -> Color32); 6] = [
        (ColorId::ChannelChanged, theme::channel_changed),
        (ColorId::ItemsActiveMesh, theme::workspace::active_mesh),
        (ColorId::ItemsSelectedText, theme::workspace::selected_text),
        (ColorId::PanelDotActive, theme::panel_dot_active),
        (ColorId::Selection, theme::viewport::selection),
        (ColorId::SelectionFill, theme::viewport::selection_fill),
    ];

    fn temp(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "operandi-colors-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    // WCAG relative luminance for sRGB colors.
    fn contrast(a: Color32, b: Color32) -> f64 {
        fn luminance(c: Color32) -> f64 {
            let lin = |v: u8| {
                let v = f64::from(v) / 255.;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
        }
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn presets_are_embedded_and_their_keys_parse() {
        assert_eq!(
            Preset::ALL.map(Preset::name),
            [
                "Steel",
                "Slate Blue",
                "Fog",
                "Classic Grey",
                "Gradient Sky",
                "Charcoal",
                "Midnight",
                "None More Black",
            ]
        );
        for preset in Preset::ALL {
            let colors: Overrides = preset.colors().collect();
            assert_eq!(
                colors.len(),
                match preset {
                    Preset::ClassicGrey => COLORS.len() - 4,
                    Preset::Steel => 41,
                    _ => 42,
                },
                "{}",
                preset.name()
            );
            let file = ColorFile::new(&colors, None);
            let json = serde_json::to_string(&file).unwrap();
            let parsed: ColorFile = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.valid_colors(), colors, "{}", preset.name());
        }
    }

    #[test]
    fn presets_meet_their_text_contrast_targets() {
        use ColorId::*;
        for preset in Preset::ALL {
            let scheme = Scheme::BuiltIn(preset);
            theme::set_overrides(&scheme.colors(Path::new("")).unwrap());
            for (text, ground, minimum) in [
                (Text, Panel, 7.),
                (
                    ItemsText,
                    ItemsList,
                    if preset == Preset::ClassicGrey {
                        6.38
                    } else {
                        7.
                    },
                ),
                (WeakText, Panel, 4.5),
                (ItemsSelectedText, ItemsSelected, 4.5),
                (OnAccent, Accent, 4.5),
            ] {
                let ratio = contrast(theme::color(text), theme::color(ground));
                assert!(
                    ratio >= minimum,
                    "{}: {text:?} on {ground:?}: {ratio:.2}:1, expected {minimum}:1",
                    preset.name()
                );
            }
        }
        theme::set_overrides(&Overrides::new());
    }

    #[test]
    fn exporting_and_importing_preserves_every_resolved_color() {
        let root = temp("export-import");
        let source = root.join("Steel.json");
        let dir = root.join("schemes");
        let builtin = Scheme::BuiltIn(Preset::Steel);
        theme::set_overrides(&builtin.colors(&dir).unwrap());
        theme::set_override(ColorId::Accent, Some(Color32::from_rgb(50, 100, 150)));
        let expected: Overrides = COLORS.iter().map(|e| (e.id, theme::color(e.id))).collect();
        export_scheme(&source).unwrap();
        assert_eq!(ColorFile::read(&source).unwrap().valid_colors(), expected);
        theme::set_overrides(&Overrides::new());
        let (saved, overrides, count) = import_scheme(&source, &dir).unwrap();
        assert_eq!(saved, Scheme::Saved("Steel".into()));
        assert_eq!(count, COLORS.len());
        assert_eq!(schemes(&dir), ["Steel"]);
        assert_eq!(saved.colors(&dir).unwrap(), overrides);
        theme::set_overrides(&overrides);
        for (id, color) in expected {
            assert_eq!(theme::color(id), color, "{id:?}");
        }

        let config = root.join("color-theme.json");
        for scheme in [saved, builtin] {
            ColorFile::for_preferences(Some(&scheme))
                .save(&config)
                .unwrap();
            assert_eq!(load(&config), Some(scheme));
        }
        theme::set_overrides(&Overrides::new());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn import_cleans_names_and_ignores_bad_entries_without_replacing_saved_schemes() {
        let root = temp("import");
        let source = root.join(" Night!.json");
        let dir = root.join("schemes");
        ColorFile::new(&Overrides::new(), None)
            .save(&scheme_path(&dir, "Night"))
            .unwrap();
        fs::write(
            &source,
            r##"{"version":1,"scheme":"Other","built_in":true,"colors":{
            "viewport.grid":"#091827","interface.panel":"#+10000",
            "viewport.solid":false,"gone.key":"#ffffff","text.normal":"xyz"
        }}"##,
        )
        .unwrap();
        let (scheme, colors, count) = import_scheme(&source, &dir).unwrap();
        assert_eq!(scheme, Scheme::Saved("Night (2)".into()));
        assert_eq!(
            colors,
            Overrides::from([(ColorId::Grid, Color32::from_rgb(9, 24, 39))])
        );
        assert_eq!(count, 1);
        assert!(
            ColorFile::read(&scheme_path(&dir, "Night"))
                .unwrap()
                .colors
                .is_empty()
        );
        let saved = ColorFile::read(&scheme_path(&dir, scheme.name())).unwrap();
        assert_eq!(saved.colors.len(), 1);
        assert_eq!(saved.selected_scheme(), None);
        for bad in [
            "nope",
            r#"{"version":2}"#,
            r#"{"version":1,"colors":{"viewport.grid":42}}"#,
        ] {
            fs::write(&source, bad).unwrap();
            assert!(import_scheme(&source, &dir).is_err());
        }
        assert_eq!(schemes(&dir), ["Night", "Night (2)"]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn steel_is_the_factory_default_for_missing_and_untouched_preferences() {
        use ColorId::*;
        let root = temp("steel-default");
        let path = root.join("color-theme.json");
        assert_eq!(load(&path), None);
        assert!(current().is_empty());
        assert_eq!(theme::color(Panel), parse_hex("#2e333b").unwrap());
        assert_eq!(theme::color(Accent), parse_hex("#7fb6e6").unwrap());
        assert_eq!(theme::color(ItemsSelected), parse_hex("#2b4662").unwrap());
        assert_eq!(theme::color(ItemsSelectedText), theme::color(Accent));
        assert_eq!(theme::viewport::background(), parse_hex("#2c3138").unwrap());
        {
            let _ortho = theme::viewport::use_scheme(theme::viewport::ViewScheme::Orthographic);
            assert_eq!(theme::viewport::background(), parse_hex("#2c3138").unwrap());
            assert_eq!(theme::viewport::grid(), parse_hex("#66707c").unwrap());
        }
        let steel = Scheme::BuiltIn(Preset::Steel);
        let selection = parse_hex("#ffc24a").unwrap();
        assert_eq!(
            steel.colors(&root).unwrap(),
            Overrides::from([(Selection, selection), (SelectionFill, selection)])
        );
        for preset in [Preset::ClassicGrey, Preset::Fog, Preset::Steel] {
            let selected = Scheme::BuiltIn(preset);
            theme::set_overrides(&selected.colors(&root).unwrap());
            let before = current();
            ColorFile::for_preferences(Some(&selected))
                .save(&path)
                .unwrap();
            theme::set_overrides(&Overrides::new());
            assert_eq!(load(&path), Some(selected));
            assert_eq!(current(), before);
        }
        fs::write(&path, r#"{"version":1,"colors":{}}"#).unwrap();
        assert_eq!(load(&path), None);
        assert!(current().is_empty());
        assert_eq!(theme::color(Panel), parse_hex("#2e333b").unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_custom_colors_and_selected_schemes_survive_the_default_change() {
        use ColorId::*;
        let root = temp("legacy-scheme");
        let config = root.join("color-theme.json");
        let custom = r##"{"version":1,"colors":{
            "interface.panel":"#102030",
            "interface.accent":"#f49c1c"
        }}"##;
        fs::write(&config, custom).unwrap();
        assert_eq!(load(&config), None);
        assert_eq!(theme::color(Panel), parse_hex("#102030").unwrap());
        assert_eq!(theme::color(Accent), parse_hex("#f49c1c").unwrap());
        assert_eq!(theme::color(Field), parse_hex("#252a31").unwrap());

        // A saved scheme named Steel is still a user scheme. Missing values
        // in its old file and in the active selection both use Classic Grey.
        let saved = Scheme::Saved("Steel".into());
        let path = scheme_path(&root, saved.name());
        fs::write(&path, custom).unwrap();
        let colors = saved.colors(&root).unwrap();
        theme::set_overrides(&colors);
        for entry in COLORS {
            let expected = match entry.id {
                Panel => parse_hex("#102030").unwrap(),
                id if id.follows_accent_by_default() => parse_hex("#f49c1c").unwrap(),
                _ => entry.classic_default,
            };
            assert_eq!(theme::color(entry.id), expected, "{}", entry.key);
        }
        let selected = custom.replace(r#""version":1"#, r#""version":1,"scheme":"Steel""#);
        fs::write(&config, selected).unwrap();
        assert_eq!(load(&config), Some(saved.clone()));
        assert_eq!(current(), colors);

        ColorFile::for_preferences(Some(&saved))
            .save(&config)
            .unwrap();
        theme::set_overrides(&Overrides::new());
        assert_eq!(load(&config), Some(saved));
        assert_eq!(current(), colors);
        assert_eq!(fs::read_to_string(path).unwrap(), custom);
        theme::set_overrides(&Overrides::new());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn every_colour_has_a_unique_key_label_and_group() {
        let mut keys = std::collections::BTreeSet::new();
        for (i, e) in COLORS.iter().enumerate() {
            assert_eq!(e.id as usize, i, "{}", e.key);
            assert!(keys.insert(e.key), "duplicate key {}", e.key);
            assert!(!e.label.is_empty() && !e.group.is_empty());
            assert_eq!(ColorId::from_key(e.key), Some(e.id));
        }
    }

    #[test]
    fn untouched_colours_follow_the_default_and_overrides_win() {
        theme::set_overrides(&Overrides::new());
        assert_eq!(theme::viewport::selection(), theme::orange());
        let g = theme::generation();
        theme::set_override(ColorId::Selection, Some(Color32::from_rgb(1, 2, 3)));
        assert_eq!(theme::viewport::selection(), Color32::from_rgb(1, 2, 3));
        assert_ne!(theme::generation(), g, "a change invalidates caches");
        assert_eq!(
            theme::viewport::wireframe(),
            ColorId::Wireframe.entry().default
        );
        theme::set_override(ColorId::Selection, None);
        assert_eq!(theme::override_of(ColorId::Selection), None);
        assert!(current().is_empty());
    }

    #[test]
    fn accent_followers_use_the_live_accent_by_default() {
        theme::set_overrides(&Overrides::new());
        for accent in [theme::ORANGE, Color32::from_rgb(10, 90, 200)] {
            theme::set_override(ColorId::Accent, Some(accent));
            for (id, draw_color) in ACCENT_SWATCHES {
                assert_eq!(draw_color(), accent, "{id:?}");
                assert_eq!(theme::override_of(id), None);
            }
        }
    }

    #[test]
    fn accent_followers_can_be_overridden_and_reset_individually() {
        theme::set_overrides(&Overrides::new());
        let accent = Color32::from_rgb(10, 90, 200);
        for (id, draw_color) in ACCENT_SWATCHES {
            theme::set_override(ColorId::Accent, None);
            // Even a colour matching Accent can be pinned.
            theme::set_override(id, Some(theme::ORANGE));
            theme::set_override(ColorId::Accent, Some(accent));
            assert_eq!(draw_color(), theme::ORANGE, "{id:?}");
            for (other, other_draw_color) in ACCENT_SWATCHES {
                // The polygon fill follows Selection, so pinning
                // Selection pins the fill's colour too.
                if other == ColorId::SelectionFill && id == ColorId::Selection {
                    assert_eq!(other_draw_color(), theme::ORANGE, "{other:?}");
                } else if other != id {
                    assert_eq!(other_draw_color(), accent, "{other:?}");
                }
            }
            theme::set_override(id, None);
            assert_eq!(draw_color(), accent, "{id:?}");
        }
        theme::set_override(ColorId::Accent, None);
        assert!(current().is_empty());
    }

    #[test]
    fn accent_followers_migrate_legacy_files_and_roundtrip() {
        let root = temp("accent-migration");
        let path = root.join("color-theme.json");
        fs::write(
            &path,
            r##"{"version":1,"scheme":"Mine","colors":{
                "interface.accent":"#0a5ac8",
                "interface.channel_changed":"#f49c1c",
                "items.active_mesh":"#f49c1c",
                "text.items_selected":"#f49c1c",
                "interface.panel_dot_active":"#ffb828",
                "viewport.selection":"#ffd470",
                "viewport.selection_fill":"#ffd470"
            }}"##,
        )
        .unwrap();
        let old = ColorFile::read(&path).unwrap();
        let accent = Color32::from_rgb(10, 90, 200);
        assert_eq!(
            old.overrides(),
            Overrides::from([(ColorId::Accent, accent)])
        );
        // A customised legacy RGB is explicit, even if it matches the saved Accent.
        let mut customized = old.clone();
        for (id, _) in ACCENT_SWATCHES {
            customized.colors.insert(id.entry().key.into(), hex(accent));
        }
        let overrides = customized.overrides();
        assert_eq!(overrides.len(), ACCENT_SWATCHES.len() + 1);
        for (id, _) in ACCENT_SWATCHES {
            assert_eq!(overrides.get(&id), Some(&accent));
        }

        assert_eq!(load(&path).as_ref().map(Scheme::name), Some("Mine"));
        // Pin the old orange in the new format and save a scheme.
        theme::set_override(ColorId::ChannelChanged, Some(theme::ORANGE));
        let expected = current();
        let scheme = scheme_path(&root, "Mine");
        ColorFile::new(&expected, None).save(&scheme).unwrap();
        let saved = ColorFile::read(&scheme).unwrap();
        assert_eq!(saved.version, 1);
        assert_eq!(saved.colors.len(), expected.len());
        assert_eq!(
            saved
                .colors
                .keys()
                .filter(|key| {
                    ColorId::from_key(key).is_some_and(ColorId::follows_accent_by_default)
                })
                .count(),
            1,
            "only the pinned follower is saved",
        );
        assert_eq!(saved.overrides(), expected);
        theme::set_overrides(&saved.overrides());
        let new_accent = Color32::from_rgb(200, 90, 10);
        theme::set_override(ColorId::Accent, Some(new_accent));
        assert_eq!(theme::channel_changed(), theme::ORANGE);
        assert_eq!(theme::workspace::active_mesh(), new_accent);
        assert_eq!(theme::workspace::selected_text(), new_accent);
        assert_eq!(theme::panel_dot_active(), new_accent);
        theme::set_overrides(&Overrides::new());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn viewport_selection_migrates_steel_defaults_and_preserves_pins() {
        use ColorId::{Accent, Selection, SelectionFill};
        let former = Selection.entry().default;
        let mut file = ColorFile::new(&Overrides::from([(Selection, former)]), None);
        file.selection_accent_links = false;
        assert!(file.overrides().is_empty());
        file.steel_defaults = false;
        assert!(file.overrides().is_empty());
        let old_steel: ColorFile = serde_json::from_str(
            r#"{"version":1,"scheme":"Steel","built_in":true,"steel_defaults":true}"#,
        )
        .unwrap();
        assert_eq!(
            old_steel.saved_overrides(),
            Overrides::from([(Selection, former), (SelectionFill, former)]),
        );
        file.selection_accent_links = true;
        assert_eq!(file.overrides().get(&Selection), Some(&former));
        for preset in Preset::ALL {
            let colors = Scheme::BuiltIn(preset).colors(Path::new("")).unwrap();
            theme::set_overrides(&colors);
            theme::set_override(Accent, Some(Color32::BLACK));
            for id in [Selection, SelectionFill] {
                assert_eq!(Some(theme::color(id)), colors.get(&id).copied());
            }
        }
        theme::set_overrides(&Overrides::new());
    }

    #[test]
    fn the_file_holds_only_changed_colours_and_roundtrips() {
        let root = temp("roundtrip");
        let path = root.join("color-theme.json");
        theme::set_overrides(&Overrides::new());
        assert_eq!(load(&path), None, "no file: all defaults");
        assert!(current().is_empty());
        let mut changed = Overrides::new();
        changed.insert(ColorId::ViewportBackground, Color32::from_rgb(10, 20, 30));
        changed.insert(ColorId::HandleX, Color32::from_rgb(255, 0, 0));
        ColorFile::new(&changed, Some("Mine".into()))
            .save(&path)
            .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("\"viewport.background\": \"#0a141e\""),
            "{text}"
        );
        assert!(
            !text.contains("viewport.grid\""),
            "untouched colours are not written"
        );
        assert_eq!(load(&path).as_ref().map(Scheme::name), Some("Mine"));
        assert_eq!(current(), changed);
        assert_eq!(theme::viewport::handles()[0], Color32::from_rgb(255, 0, 0));
        assert_eq!(theme::viewport::handles()[1], theme::viewport::HANDLES[1]);
        // Unknown keys and bad values are skipped, not fatal.
        fs::write(
            &path,
            r##"{"version":1,"colors":{"viewport.grid":"#010203","gone.key":"#ffffff","viewport.solid":"red"}}"##,
        )
        .unwrap();
        load(&path);
        assert_eq!(current().len(), 1);
        assert_eq!(theme::viewport::grid(), Color32::from_rgb(1, 2, 3));
        // A broken or future file falls back to the defaults.
        for bad in ["nope", r#"{"version":2,"colors":{}}"#] {
            fs::write(&path, bad).unwrap();
            load(&path);
            assert!(current().is_empty());
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn schemes_list_by_name_and_names_are_cleaned() {
        let root = temp("schemes");
        assert!(schemes(&root.join("missing")).is_empty());
        assert_eq!(clean_name("  Night/../Owl  ").as_deref(), Some("NightOwl"));
        assert_eq!(clean_name("///"), None);
        let mut one = Overrides::new();
        one.insert(ColorId::Grid, Color32::from_rgb(9, 9, 9));
        for name in ["beta", "Alpha"] {
            ColorFile::new(&one, None)
                .save(&scheme_path(&root, name))
                .unwrap();
        }
        fs::write(root.join("notes.txt"), "x").unwrap();
        assert_eq!(schemes(&root), ["Alpha", "beta"]);
        let read = ColorFile::read(&scheme_path(&root, "Alpha")).unwrap();
        assert_eq!(read.overrides(), one);
        fs::remove_dir_all(root).unwrap();
    }

    /// Inactive meshes' plain wire: its own colour, pure black by default,
    /// listed in Display > Colors under Geometry right after Wireframe.
    #[test]
    fn inactive_wireframe_defaults_to_black_and_is_listed() {
        assert_eq!(theme::viewport::INACTIVE_WIREFRAME, Color32::BLACK);
        let at = |id| COLORS.iter().position(|e| e.id == id).unwrap();
        let e = &COLORS[at(ColorId::InactiveWireframe)];
        assert_eq!(e.key, "viewport.inactive_wireframe");
        assert_eq!(e.label, "Inactive Wireframe");
        assert_eq!(e.group, "Geometry");
        assert_eq!(e.default, Color32::BLACK);
        assert_eq!(at(ColorId::InactiveWireframe), at(ColorId::Wireframe) + 1);
        assert_eq!(
            ColorId::from_key("viewport.inactive_wireframe"),
            Some(ColorId::InactiveWireframe)
        );
    }

    /// colors-2026-11-16 section 2: the classic default look, and the
    /// text on it readable (about 6:1 on list rows and panels).
    #[test]
    fn classic_grey_look_and_contrast() {
        use crate::theme::ColorId::*;
        let d = |id: ColorId| id.entry().classic_default;
        let g = Color32::from_gray;
        assert_eq!(d(Panel), g(0x3E));
        assert_eq!(d(Text), g(0xDE));
        assert_eq!(d(ItemsList), g(0x4C));
        assert_eq!(d(ItemsListStripe), g(0x50));
        assert_eq!(d(ItemsText), g(0xDE));
        assert_eq!(d(ItemsSelected), g(0x3B));
        assert_eq!(d(ItemsSelectedText), Color32::from_rgb(0xF4, 0x9C, 0x1C));
        assert_eq!(d(Control), g(0x4C));
        assert_eq!(d(Field), g(0x4C));
        for (text, ground) in [
            (Text, Panel),
            (ItemsText, ItemsList),
            (ItemsText, ItemsListStripe),
            (ToolboxText, ToolboxBackground),
            (Text, Control),
        ] {
            let c = contrast(d(text), d(ground));
            assert!(c >= 5.95, "{text:?} on {ground:?}: {c:.2}:1");
        }
        assert!(contrast(d(ItemsSelectedText), d(ItemsSelected)) >= 5.);
    }

    /// Section 3: each view kind draws with its stock scheme's colours;
    /// the scheme comes back when the view is done.
    #[test]
    fn view_schemes_pick_their_own_colours() {
        use crate::theme::viewport::{self as v, ViewScheme};
        let g = Color32::from_gray;
        theme::set_overrides(
            &Scheme::BuiltIn(Preset::ClassicGrey)
                .colors(Path::new(""))
                .unwrap(),
        );
        assert_eq!(v::background(), g(0x38));
        assert_eq!(v::grid(), g(0x85));
        assert_eq!(v::work_plane(), Color32::from_rgb(0x5C, 0x8A, 0xED));
        {
            let _ortho = v::use_scheme(ViewScheme::Orthographic);
            assert_eq!(v::background(), g(0x39));
            assert_eq!(v::grid(), Color32::BLACK);
            assert_eq!(v::work_plane(), Color32::from_rgb(0x8F, 0x99, 0xA6));
            {
                let _uv = v::use_scheme(ViewScheme::Uv);
                assert_eq!(v::background(), g(0x43));
                assert_eq!(v::wireframe(), Color32::from_rgb(0xB0, 0xB2, 0xB2));
            }
            assert_eq!(v::background(), g(0x39));
        }
        assert_eq!(v::background(), g(0x38));
        theme::set_overrides(&Overrides::new());
    }

    /// "Report Bug Icon" (interface.report_bug) was removed 2026-11-16 once
    /// Report Bug moved to the Help menu. Old files that still carry it load,
    /// and the key is ignored.
    #[test]
    fn removed_report_bug_colour_is_ignored_on_load() {
        assert_eq!(ColorId::from_key("interface.report_bug"), None);
        let json = r##"{"version":1,"colors":{"interface.report_bug":"#f08474","interface.panel":"#102030"}}"##;
        let file: ColorFile = serde_json::from_str(json).unwrap();
        let overrides = file.overrides();
        assert_eq!(overrides.len(), 1);
        assert_eq!(
            overrides.get(&ColorId::Panel),
            Some(&Color32::from_rgb(0x10, 0x20, 0x30))
        );
    }

    #[test]
    fn hex_roundtrips() {
        for c in [
            Color32::BLACK,
            Color32::WHITE,
            Color32::from_rgb(1, 128, 254),
        ] {
            assert_eq!(parse_hex(&hex(c)), Some(c));
        }
        for bad in ["", "#12345", "123456", "#gg0000", "#1234567"] {
            assert_eq!(parse_hex(bad), None, "{bad}");
        }
    }
}

#[cfg(test)]
mod guard {
    /// Drawing code must use the live colour functions (theme::text(),
    /// palette::selection(), ...), never the default constants, or the
    /// user's colours silently stop applying there.
    #[test]
    fn drawing_code_reads_live_colours_not_the_defaults() {
        let names = [
            "ORANGE",
            "CYAN",
            "ERROR",
            "TEXT",
            "BORDER",
            "CONTROL",
            "HOVER",
            "PANEL",
            "WINDOW",
            "FIELD",
            "PRESSED",
            "WEAK_TEXT",
            "HOVER_OUTLINE",
            "DISABLED_TEXT",
            "ON_ORANGE",
            "ACTIVE",
            "ACTIVE_TEXT",
            "BACKGROUND",
            "HEADER",
            "GRID",
            "WORK_PLANE",
            "GRID_AXES",
            "ZERO_AXES",
            "WIREFRAME",
            "INACTIVE_WIREFRAME",
            "SHADED_WIREFRAME",
            "SOLID",
            "SELECTION",
            "MIRROR_SELECTION",
            "HANDLES",
            "ROLLOVER",
            "EDGE_BACKING",
            "LABEL",
            "VERTEX_MAP_NONE",
            "COLORED_WIRE",
            "UV_SEAM",
            "LIST",
            "FIELD_TEXT",
            "TAB",
            "SELECTED",
            "PRIMARY",
            "INACTIVE",
            "SEPARATOR",
            "GOOCH_COOL",
            "GOOCH_WARM",
        ];
        let prefixes = ["theme::", "chrome::", "palette::", "workspace::"];
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut bad = Vec::new();
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().is_none_or(|e| e != "rs")
                    || path.ends_with("theme.rs")
                    || path.ends_with("color_theme.rs")
                {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap();
                for (n, line) in text.lines().enumerate() {
                    // Most lines name no palette at all (test speed: the
                    // name-by-name search below took ~10 s in debug).
                    if !line.contains("::") {
                        continue;
                    }
                    for prefix in prefixes {
                        if !line.contains(prefix) {
                            continue;
                        }
                        for name in names {
                            let pat = format!("{prefix}{name}");
                            if let Some(i) = line.find(&pat) {
                                let next = line[i + pat.len()..].chars().next();
                                let before = line[..i].chars().last();
                                let whole = !next.is_some_and(|c| c.is_alphanumeric() || c == '_')
                                    && !before.is_some_and(|c| c.is_alphanumeric() || c == '_');
                                // crate::viewport (the view module) is not the palette.
                                let view_module = prefix == "palette::"
                                    && !text.contains("viewport as palette")
                                    && !text.contains("workspace as palette")
                                    && !path.starts_with(
                                        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                                            .join("src/viewport"),
                                    );
                                if whole && !view_module {
                                    bad.push(format!("{}:{}: {pat}", path.display(), n + 1));
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(
            bad.is_empty(),
            "use the live colour function instead:\n{}",
            bad.join("\n")
        );
    }
}
