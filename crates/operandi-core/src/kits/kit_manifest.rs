//! A kit's manifest, `kit.toml` (kits plan K1; docs/KITS.md).
//!
//! A kit is a folder with a `kit.toml` at its top. The manifest says who
//! the kit is (its id, which is also its command namespace), which kit
//! contract it was written for ([`KIT_API_VERSION`]) and what it adds.
//! Contract version 1 adds file importers run by a helper program
//! ([`ImporterDecl`]); later versions add more kinds of parts.
//!
//! The manifest is strict: an unknown field is refused, so a typo shows
//! up when the kit is loaded rather than as a part that silently does
//! nothing. A kit written for a newer contract is refused with a message
//! saying which Operandi it needs, never half-loaded.
use crate::Result;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Component, Path};

/// The kit contract this build speaks. A kit names the version it was
/// written for in `api_version`; one newer than this is refused.
///
/// Bump it only when a manifest field or the helper protocol changes
/// meaning (adding an optional field doesn't). Version 1: `[kit]` and
/// `[[importer]]`, helper protocol 1 (docs/KITS.md).
pub const KIT_API_VERSION: u32 = 1;

/// The manifest's file name, at the top of the kit's folder.
pub const MANIFEST_FILE: &str = "kit.toml";

/// Longest kit id: it is also a command namespace.
const MAX_ID_LEN: usize = 32;

/// Formats a helper may hand back: built-in readers whose files carry
/// their own units (OBJ and STL don't, so the user's File I/O units would
/// rescale them).
pub const HELPER_OUTPUTS: [&str; 8] = ["glb", "gltf", "ply", "fbx", "usd", "usda", "usdc", "usdz"];

/// Platform keys a `[importer.helper]` table may use, as
/// `<os>-<arch>` from Rust's `std::env::consts`, plus `any` for a helper
/// that runs everywhere (a script with a `#!` line).
pub const PLATFORMS: [&str; 7] = [
    "any",
    "linux-aarch64",
    "linux-x86_64",
    "macos-aarch64",
    "macos-x86_64",
    "windows-aarch64",
    "windows-x86_64",
];

/// This machine's platform key (`macos-aarch64`).
pub fn this_platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// A parsed, checked `kit.toml`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KitManifest {
    pub kit: KitInfo,
    /// File types the kit can import (`[[importer]]` tables).
    #[serde(default, rename = "importer")]
    pub importers: Vec<ImporterDecl>,
}

/// The `[kit]` table.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KitInfo {
    /// Lower-case letters, digits and `_`, starting with a letter
    /// (`step_import`). It is the kit's command namespace and the folder
    /// it installs into, and scenes name it in kit items.
    pub id: String,
    /// The name people see (`STEP Import`).
    pub name: String,
    /// The kit's own version, shown in the kit list (`0.3.1`).
    pub version: String,
    /// The kit contract it was written for ([`KIT_API_VERSION`]).
    pub api_version: u32,
    /// An SPDX licence expression (`GPL-3.0-or-later`). A helper program
    /// may use any licence; see docs/KITS.md.
    pub licence: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub homepage: Option<String>,
}

/// One `[[importer]]`: a file type the kit's helper program turns into a
/// file Operandi reads itself.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImporterDecl {
    /// What the file type is called in the Import dialog (`STEP files`).
    pub label: String,
    /// Extensions without the dot, lower case (`["step", "stp"]`).
    pub extensions: Vec<String>,
    /// What the helper writes: one of [`HELPER_OUTPUTS`].
    pub output: String,
    /// Seconds before a helper that hasn't finished is stopped.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// The helper program per platform ([`PLATFORMS`]), as a path inside
    /// the kit folder (`bin/macos-aarch64/step-helper`).
    pub helper: BTreeMap<String, String>,
}

fn default_timeout() -> u64 {
    600
}

/// Longest wait a manifest may ask for: a day.
const MAX_TIMEOUT: u64 = 24 * 60 * 60;

impl ImporterDecl {
    /// The helper for `platform`, else the `any` one.
    pub fn helper_for(&self, platform: &str) -> Option<&str> {
        self.helper
            .get(platform)
            .or_else(|| self.helper.get("any"))
            .map(String::as_str)
    }
}

impl KitManifest {
    /// Read and check a manifest's text.
    pub fn parse(text: &str) -> Result<Self> {
        // Look at api_version first, so a kit for a newer contract gets
        // "needs a newer Operandi" rather than an unknown-field error.
        if let Ok(table) = text.parse::<toml::Table>()
            && let Some(v) = table
                .get("kit")
                .and_then(|k| k.get("api_version"))
                .and_then(toml::Value::as_integer)
            && v > i64::from(KIT_API_VERSION)
        {
            return Err(format!(
                "This kit needs kit API {v}; this Operandi has API {KIT_API_VERSION}. Update Operandi to use it."
            ));
        }
        let manifest: KitManifest =
            toml::from_str(text).map_err(|e| format!("{MANIFEST_FILE}: {}", e.message()))?;
        manifest.check()?;
        Ok(manifest)
    }

    /// Read and check `<dir>/kit.toml`.
    pub fn read(dir: &Path) -> Result<Self> {
        let path = dir.join(MANIFEST_FILE);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("Can't read {}: {e}", path.display()))?;
        Self::parse(&text)
    }

    fn check(&self) -> Result<()> {
        let k = &self.kit;
        check_id(&k.id)?;
        if k.api_version == 0 {
            return Err("api_version starts at 1".into());
        }
        for (field, value) in [
            ("name", &k.name),
            ("version", &k.version),
            ("licence", &k.licence),
        ] {
            if value.trim().is_empty() {
                return Err(format!("[kit] needs a {field}"));
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        for imp in &self.importers {
            if imp.label.trim().is_empty() {
                return Err("An [[importer]] needs a label".into());
            }
            if imp.extensions.is_empty() {
                return Err(format!("Importer \"{}\" lists no extensions", imp.label));
            }
            for ext in &imp.extensions {
                check_extension(ext)?;
                if !seen.insert(ext.as_str()) {
                    return Err(format!(".{ext} is listed by two importers"));
                }
            }
            if !HELPER_OUTPUTS.contains(&imp.output.as_str()) {
                return Err(format!(
                    "Importer \"{}\": output must be one of {}",
                    imp.label,
                    HELPER_OUTPUTS.join(", ")
                ));
            }
            if imp.timeout_seconds == 0 || imp.timeout_seconds > MAX_TIMEOUT {
                return Err(format!(
                    "Importer \"{}\": timeout_seconds must be 1 to {MAX_TIMEOUT}",
                    imp.label
                ));
            }
            if imp.helper.is_empty() {
                return Err(format!("Importer \"{}\" names no helper", imp.label));
            }
            for (platform, path) in &imp.helper {
                if !PLATFORMS.contains(&platform.as_str()) {
                    return Err(format!(
                        "Unknown helper platform \"{platform}\" (use {})",
                        PLATFORMS.join(", ")
                    ));
                }
                check_inside(path)?;
            }
        }
        Ok(())
    }
}

/// A kit id: lower-case letters, digits and `_`, starting with a letter,
/// not one of Operandi's own command namespaces.
pub fn check_id(id: &str) -> Result<()> {
    let shaped = !id.is_empty()
        && id.len() <= MAX_ID_LEN
        && id.starts_with(|c: char| c.is_ascii_lowercase())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !shaped {
        return Err(format!(
            "Kit id \"{id}\" must be 1 to {MAX_ID_LEN} lower-case letters, digits or _, starting with a letter"
        ));
    }
    let built_in = crate::command_runtime::ALWAYS_RESERVED.contains(&id)
        || crate::command_registry::COMMANDS.iter().any(|e| {
            e.name
                .split_once('.')
                .is_some_and(|(head, _)| head.eq_ignore_ascii_case(id))
        });
    if built_in {
        return Err(format!(
            "Kit id \"{id}\" is one of Operandi's own command namespaces"
        ));
    }
    Ok(())
}

fn check_extension(ext: &str) -> Result<()> {
    let shaped = !ext.is_empty()
        && ext.len() <= 16
        && ext
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    if !shaped {
        return Err(format!(
            "Extension \"{ext}\" must be lower-case letters and digits, without the dot"
        ));
    }
    // KIT-1: one table built-ins and kits share; in contract 1 a kit adds
    // new file types and can't take over a built-in one.
    if crate::interchange::IMPORT_EXTENSIONS.contains(&ext) || ext == "opscene" || ext == "lxo" {
        return Err(format!(".{ext} files are read by Operandi itself"));
    }
    Ok(())
}

/// A relative path that stays inside the kit's folder.
pub(crate) fn check_inside(path: &str) -> Result<()> {
    let p = Path::new(path);
    let inside = !path.is_empty()
        && !path.contains('\\')
        && p.components().all(|c| matches!(c, Component::Normal(_)));
    if inside {
        Ok(())
    } else {
        Err(format!(
            "\"{path}\" must be a path inside the kit folder, with / between folders"
        ))
    }
}
