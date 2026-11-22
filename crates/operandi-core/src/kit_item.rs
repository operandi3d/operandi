//! Items of a kind this build doesn't know (KIT-1, AR-09; kits plan K6,
//! master plan phase 0 door 1).
//!
//! A kit (a future add-on) can add new item kinds: a Gaussian splat cloud,
//! a VDB volume. Such an item is saved in [`Item::kit`](crate::Item::kit):
//! which kit and kind it is, the kind's version, its values as plain data
//! and, optionally, the box it fills. A build without that kit still opens
//! the scene: the item loads, shows in the Items list as needing its kit,
//! draws as its bounding box, and saves back exactly as it came, like an
//! unknown MeshOp (`OpKind::Placeholder`).
//!
//! No kit loader exists yet (K1), so every kit item is missing for now.
//! Its values are never changed by this build: scripts can read them
//! (`item.channel name:<value> value:?`), not set them.
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// Longest kit or kind id kept.
const MAX_ID_LEN: usize = 256;

/// An item made by a kit (KIT-1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KitItem {
    /// The kit's id, which is also its command namespace (`splat`).
    pub kit: String,
    /// The item kind within the kit (`splat.cloud`).
    pub kind: String,
    /// The kind's own data version, for the kit's upgrades.
    #[serde(default)]
    pub version: u32,
    /// The kind's values as the kit saved them. Kept as written and saved
    /// back unchanged while the kit is missing.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub values: serde_json::Value,
    /// The box the item fills, in its own space (`[min, max]`), so a
    /// build without the kit can still draw where it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<[DVec3; 2]>,
}

impl KitItem {
    /// Whether the kit that draws and edits this item is installed. Always
    /// false until the kit loader exists (kits plan K1).
    pub fn installed(&self) -> bool {
        installed(&self.kit)
    }
    /// "Needs kit splat (splat.cloud)": the Items list's tooltip and the
    /// answer scripts get when they try to change the item.
    pub fn needs_text(&self) -> String {
        format!(
            "Needs kit \"{}\" ({} v{}), which isn't installed. The item is kept and saved back unchanged.",
            self.kit, self.kind, self.version
        )
    }
    pub fn validate(&self) -> crate::Result<()> {
        let ok_id = |s: &str| !s.trim().is_empty() && s.len() <= MAX_ID_LEN;
        if !ok_id(&self.kit) || !ok_id(&self.kind) {
            return Err("A kit item needs a kit and a kind id".into());
        }
        if let Some([lo, hi]) = self.bounds
            && (!lo.is_finite()
                || !hi.is_finite()
                || lo.cmpgt(hi).any()
                || lo.abs().max_element() > 1e9
                || hi.abs().max_element() > 1e9)
        {
            return Err("A kit item's bounds must be finite, with min ≤ max".into());
        }
        Ok(())
    }
    /// The value `name` (a top-level key, or dotted for nested ones) as
    /// text, for `item.channel ... ?`.
    pub fn value_text(&self, name: &str) -> Option<String> {
        let mut at = &self.values;
        for part in name.split('.') {
            at = at.get(part)?;
        }
        Some(match at {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        })
    }
    /// Every readable value name (leaves, dotted), for error messages.
    pub fn value_names(&self) -> Vec<String> {
        let mut out = Vec::new();
        crate::channels::json_leaves("", &self.values, &mut out);
        out.into_iter()
            .map(|(name, _)| name)
            .filter(|name| !name.is_empty())
            .collect()
    }
}

/// Whether kit `id` is installed. No kit loader yet (K1): none is.
pub fn installed(_id: &str) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn splat() -> KitItem {
        KitItem {
            kit: "splat".into(),
            kind: "splat.cloud".into(),
            version: 2,
            values: serde_json::json!({"count": 1200, "file": {"name": "a.ply"}}),
            bounds: Some([DVec3::splat(-1.), DVec3::splat(1.)]),
        }
    }

    #[test]
    fn reads_values_by_dotted_name() {
        let k = splat();
        assert_eq!(k.value_text("count").as_deref(), Some("1200"));
        assert_eq!(k.value_text("file.name").as_deref(), Some("a.ply"));
        assert!(k.value_text("nope").is_none());
        assert!(k.value_names().contains(&"file.name".to_string()));
    }

    #[test]
    fn refuses_bad_ids_and_bounds() {
        let mut k = splat();
        assert!(k.validate().is_ok());
        k.kind = " ".into();
        assert!(k.validate().is_err());
        let mut k = splat();
        k.bounds = Some([DVec3::ONE, DVec3::ZERO]);
        assert!(k.validate().is_err());
        k.bounds = Some([DVec3::ZERO, DVec3::new(f64::NAN, 0., 0.)]);
        assert!(k.validate().is_err());
    }

    #[test]
    fn no_kit_is_installed_yet() {
        assert!(!splat().installed());
        assert!(splat().needs_text().contains("splat.cloud"));
    }
}
