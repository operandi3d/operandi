//! Materials: the surface settings a polygon material tag stands for.
//!
//! Kept deliberately small. A polygon names its material by tag
//! (`Face::material`); the scene maps each tag name to one `Material`. The
//! planned shader tree will layer on top of these tags, so a material here
//! is just the base surface: a type, a color, diffuse and specular amounts,
//! smoothing, and two flags the dialog stores but nothing acts on yet.
//!
//! A polygon with an empty tag uses the material named `Default`. A scene
//! without a stored `Default` uses factory settings, and its polygons keep
//! the theme's plain viewport color.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The tag name of the default material.
pub const DEFAULT_MATERIAL: &str = "Default";

/// Material types. There is one for now; the enum keeps room for more.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterialKind {
    #[default]
    Default,
}
impl MaterialKind {
    pub const ALL: [MaterialKind; 1] = [MaterialKind::Default];
    pub fn label(self) -> &'static str {
        match self {
            MaterialKind::Default => "Default Material",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Material {
    #[serde(default)]
    pub kind: MaterialKind,
    /// Linear RGB, each 0 to 1.
    pub color: [f64; 3],
    /// Fractions 0 to 1 (the dialog shows percent).
    pub diffuse: f64,
    /// Specular Amount. Since native version 15 this is an amount, not the
    /// reflectance: [`specular_reflectance`] turns it into one (50 % is an
    /// ordinary non-metal). Older files stored the reflectance itself and
    /// are converted when read.
    pub specular: f64,
    /// Off shades the material's polygons flat.
    pub smoothing: bool,
    /// Stored only; see SPECIFICATION_QUESTIONS.md (MAT-Q01).
    #[serde(default)]
    pub make_default: bool,
    /// Stored only; see SPECIFICATION_QUESTIONS.md (MAT-Q01).
    #[serde(default)]
    pub add_to_library: bool,
}
impl Default for Material {
    fn default() -> Self {
        Self {
            kind: MaterialKind::Default,
            // Shader tree Tier 1, G-03 (decided 2026-11-06): the reference
            // Base Material's starting values, shared with the tree.
            color: [0.6, 0.6, 0.6],
            diffuse: 0.8,
            specular: 0.5,
            smoothing: true,
            make_default: false,
            add_to_library: false,
        }
    }
}
impl Material {
    pub fn validate(&self) -> crate::Result<()> {
        let unit = |v: f64| v.is_finite() && (0.0..=1.0).contains(&v);
        if !self.color.iter().all(|&c| unit(c)) {
            return Err("Material color values must be between 0 and 1".into());
        }
        if !unit(self.diffuse) || !unit(self.specular) {
            return Err("Material Diffuse and Specular must be between 0% and 100%".into());
        }
        Ok(())
    }
    /// The color a renderer reflects diffusely: color scaled by Diffuse.
    pub fn diffuse_color(&self) -> [f64; 3] {
        self.color.map(|c| c * self.diffuse)
    }
}

/// Specular Amount (0 to 1) as the reflectance straight on (F0), the curve
/// renders and exports use (provisional, ST-Q05; our own choice). The lower
/// half is linear and covers ordinary non-metals: 50 % gives F0 0.04, plain
/// plastic or glass. The upper half rises exponentially to F0 0.9 at 100 %,
/// the strongest the earlier mapping allowed, so a mirror-like highlight
/// stays reachable.
pub fn specular_reflectance(amount: f64) -> f64 {
    let s = if amount.is_finite() {
        amount.clamp(0., 1.)
    } else {
        0.
    };
    if s <= 0.5 {
        SPECULAR_F0_AT_HALF * 2. * s
    } else {
        SPECULAR_F0_AT_HALF * (SPECULAR_F0_AT_FULL / SPECULAR_F0_AT_HALF).powf((s - 0.5) * 2.)
    }
}

/// The inverse of [`specular_reflectance`]: the amount that gives `f0`
/// (clamped to what the curve reaches). Used to read older files, which
/// stored the reflectance itself.
pub fn specular_amount_for_reflectance(f0: f64) -> f64 {
    if !f0.is_finite() || f0 <= 0. {
        return 0.;
    }
    if f0 <= SPECULAR_F0_AT_HALF {
        return f0 / (2. * SPECULAR_F0_AT_HALF);
    }
    let t = (f0 / SPECULAR_F0_AT_HALF).ln() / (SPECULAR_F0_AT_FULL / SPECULAR_F0_AT_HALF).ln();
    (0.5 + 0.5 * t).min(1.)
}

/// Reflectance at Specular Amount 50 % (an ordinary dielectric, IOR 1.5).
pub const SPECULAR_F0_AT_HALF: f64 = 0.04;
/// Reflectance at Specular Amount 100 %.
pub const SPECULAR_F0_AT_FULL: f64 = 0.9;

/// True for the names that mean the default material.
pub fn is_default_name(name: &str) -> bool {
    name.is_empty() || name == DEFAULT_MATERIAL
}

/// Validates the scene's material table: sane names, sane values.
pub(crate) fn validate_materials(materials: &BTreeMap<String, Material>) -> crate::Result<()> {
    if materials.len() > 65_536 {
        return Err("Too many materials".into());
    }
    for (name, m) in materials {
        if name.trim().is_empty() || name.len() > 1024 || name.chars().any(char::is_control) {
            return Err("Invalid material name".into());
        }
        m.validate().map_err(|e| format!("{name}: {e}"))?;
    }
    Ok(())
}

impl crate::Scene {
    /// The stored material a polygon tag refers to, if any. Empty and
    /// `Default` both mean the default material.
    pub fn material(&self, tag: &str) -> Option<&Material> {
        let key = if is_default_name(tag) {
            DEFAULT_MATERIAL
        } else {
            tag
        };
        self.materials.get(key)
    }
    /// The settings a polygon tag renders with: its stored material, or
    /// factory settings.
    pub fn material_or_default(&self, tag: &str) -> Material {
        self.material(tag).cloned().unwrap_or_default()
    }
    /// Every material name in the scene, for the dialog's Name list:
    /// `Default`, stored materials, then tags used on polygons.
    pub fn material_names(&self) -> Vec<String> {
        let mut names = std::collections::BTreeSet::new();
        names.extend(self.materials.keys().cloned());
        for item in &self.items {
            for material in &item.mesh.face_summary().materials {
                if !is_default_name(material) {
                    names.insert(material.clone());
                }
            }
        }
        names.remove(DEFAULT_MATERIAL);
        std::iter::once(DEFAULT_MATERIAL.to_owned())
            .chain(names)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_settings_match_the_dialog_defaults() {
        let m = Material::default();
        // G-03: the reference Base Material's starting values.
        assert_eq!(m.color, [0.6; 3]);
        assert_eq!(m.diffuse, 0.8);
        assert_eq!(m.specular, 0.5);
        assert!(m.smoothing && !m.make_default && !m.add_to_library);
        assert_eq!(m.kind.label(), "Default Material");
        m.validate().unwrap();
    }

    #[test]
    fn specular_curve_makes_half_a_plain_dielectric() {
        let near = |a: f64, b: f64| (a - b).abs() < 1e-12;
        assert!(near(specular_reflectance(0.), 0.));
        assert!(near(specular_reflectance(0.25), 0.02));
        assert!(near(specular_reflectance(0.5), 0.04));
        assert!(near(specular_reflectance(1.), 0.9));
        // Rising throughout, and the inverse gives the amount back.
        let mut last = -1.;
        for k in 0..=100 {
            let s = k as f64 / 100.;
            let f0 = specular_reflectance(s);
            assert!(f0 > last);
            last = f0;
            assert!(near(specular_amount_for_reflectance(f0), s), "{s}");
        }
        // The old default 4 % reflectance is the new 50 %.
        assert!(near(specular_amount_for_reflectance(0.04), 0.5));
        assert_eq!(specular_amount_for_reflectance(1.), 1.);
        assert_eq!(specular_reflectance(f64::NAN), 0.);
    }

    #[test]
    fn out_of_range_values_are_refused() {
        for bad in [
            Material {
                color: [1.5, 0., 0.],
                ..Default::default()
            },
            Material {
                diffuse: -0.1,
                ..Default::default()
            },
            Material {
                specular: f64::NAN,
                ..Default::default()
            },
        ] {
            assert!(bad.validate().is_err());
        }
    }

    #[test]
    fn empty_tag_and_default_name_share_the_default_material() {
        let mut scene = crate::Scene::default();
        assert_eq!(scene.material(""), None);
        let red = Material {
            color: [1., 0., 0.],
            ..Default::default()
        };
        scene.materials.insert(DEFAULT_MATERIAL.into(), red.clone());
        assert_eq!(scene.material(""), Some(&red));
        assert_eq!(scene.material("Default"), Some(&red));
        assert_eq!(scene.material_or_default("Other"), Material::default());
        assert_eq!(scene.material_names(), vec!["Default".to_owned()]);
    }
}
