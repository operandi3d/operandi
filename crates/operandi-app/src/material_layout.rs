//! Where each Material layer row goes in the Properties panel: page tabs
//! down the right edge (UIC-100) and folding sections inside them
//! (UIC-105..107), instead of one long list.
//!
//! The arrangement is Jason's brief (material editing item 4). Rows with
//! no obvious home went to the closest tab; they're marked "placed" below
//! and listed in the status note.

pub(crate) const MATERIAL_REF_TAB: &str = "Material Ref";
pub(crate) const REFLECTION_TAB: &str = "Reflection";
pub(crate) const TRANSPARENCY_TAB: &str = "Transparency";
pub(crate) const RAYS_TAB: &str = "Rays";
pub(crate) const USER_CHANNELS_TAB: &str = "User Channels";
pub(crate) const TAGS_TAB: &str = "Tags";

/// The Material layer's tabs, top to bottom.
pub(crate) const TABS: [&str; 6] = [
    MATERIAL_REF_TAB,
    REFLECTION_TAB,
    TRANSPARENCY_TAB,
    RAYS_TAB,
    USER_CHANNELS_TAB,
    TAGS_TAB,
];

/// Rows the panel draws itself (not channels of `MaterialChannels`).
pub(crate) const SHADING_MODEL: &str = "Shading Model";
pub(crate) const REFLECTION_TYPE: &str = "Reflection Type";

/// One section: its name and its rows, in order.
pub(crate) struct Section {
    pub name: &'static str,
    pub rows: &'static [&'static str],
}

/// The sections of `tab`, in order. A tab with none shows a "None yet"
/// note.
pub(crate) fn sections(tab: &str) -> &'static [Section] {
    match tab {
        MATERIAL_REF_TAB => &[
            Section {
                name: "Diffuse and Specular",
                rows: &[
                    SHADING_MODEL,
                    "Diffuse Color",
                    "Diffuse Amount",
                    "Diffuse Roughness",
                    "Metallic",
                    "Refractive Index",
                    "Use Refractive Index",
                    "Specular Color",
                    "Specular Amount",
                    "Specular Fresnel",
                    "Specular Tint",
                    "Match Specular",
                    "Roughness",
                    "Anisotropy",
                    "Sheen",
                    "Sheen Tint",
                    "Sheen Color",
                    "Sheen Roughness",
                    "Flatness",
                    REFLECTION_TYPE,
                    "Clearcoat Amount",
                    "Clearcoat Roughness",
                    "Clearcoat Bump",
                    // Placed: the coat's other OpenPBR rows sit with
                    // Clearcoat.
                    "Coat Color",
                    "Coat IOR",
                    "Coat Darkening",
                    "Coat Anisotropy",
                ],
            },
            Section {
                name: "Surface Normal",
                rows: &[
                    "Bump Amplitude",
                    "Displacement Distance",
                    "Smoothing",
                    "Smoothing Angle",
                    "Double Sided",
                    "Rounded Edge Width",
                    "Rounded Edge Angle",
                    "Round Same Surface Only",
                ],
            },
            // Placed: luminosity has no tab of its own in the brief.
            Section {
                name: "Luminosity",
                rows: &["Luminous Color", "Luminous Intensity"],
            },
            // Placed: viewport-only rows.
            Section {
                name: "Viewport Display",
                rows: &["GL Display", "GL Color Override", "GL Color"],
            },
        ],
        REFLECTION_TAB => &[Section {
            name: "Reflection",
            rows: &[
                "Reflection Color",
                "Reflection Amount",
                "Reflection Fresnel",
                "Blurry Reflection",
            ],
        }],
        TRANSPARENCY_TAB => &[
            Section {
                name: "Transparency",
                rows: &[
                    "Transparent Color",
                    "Transparent Amount",
                    "Refraction Roughness",
                    "Absorption Distance",
                    "Dispersion",
                    "Abbe Number",
                    "Transmission Scatter",
                    "Scatter Anisotropy",
                    "Thin Walled",
                    "Dissolve",
                ],
            },
            Section {
                name: "Subsurface Scattering",
                rows: &[
                    "Subsurface Color",
                    "Subsurface Amount",
                    "Subsurface Distance",
                    "Albedo from Diffuse",
                    "Subsurface Albedo",
                    "Subsurface Depth",
                    "Subsurface Phase",
                ],
            },
            Section {
                name: "Thin Film",
                rows: &["Thin Film", "Thin Film Thickness", "Thin Film IOR"],
            },
            // Placed: the volume rows are about what's inside a
            // transparent surface.
            Section {
                name: "Volume",
                rows: &[
                    "Volume Scattering",
                    "Volume Absorption",
                    "Volume Density",
                    "Volume Level",
                    "Volume Luminous",
                ],
            },
        ],
        RAYS_TAB => &[
            Section {
                name: "Rays",
                rows: &[
                    "Reflection Rays",
                    "Refraction Rays",
                    "Subsurface Samples",
                    "Same Surface",
                    "Exit Color",
                    "Importance",
                ],
            },
            // Placed: clipping and matte change what rays see.
            Section {
                name: "Clipping and Matte",
                rows: &["Clip Enable", "Clip Value", "Matte", "Matte Color"],
            },
        ],
        _ => &[],
    }
}

/// The tab to show for a Material layer: `current` if the Material has it,
/// else the first (UIC-101).
pub(crate) fn tab_for(current: &str) -> &'static str {
    TABS.into_iter()
        .find(|t| *t == current)
        .unwrap_or(MATERIAL_REF_TAB)
}

#[cfg(test)]
mod tests {
    use super::*;
    use operandi_core::shader_tree::MaterialChannels;

    fn placed() -> Vec<&'static str> {
        TABS.iter()
            .flat_map(|t| sections(t).iter())
            .flat_map(|s| s.rows.iter().copied())
            .collect()
    }

    #[test]
    fn every_material_row_has_one_home() {
        let rows = placed();
        let mut m = MaterialChannels::default();
        for (name, _) in m.channels() {
            let n = rows.iter().filter(|r| **r == name).count();
            assert_eq!(n, 1, "{name} placed {n} times");
        }
        assert!(rows.contains(&SHADING_MODEL));
        assert!(rows.contains(&REFLECTION_TYPE));
        // Nothing listed that isn't a row.
        let names: Vec<&str> = m.channels().into_iter().map(|(n, _)| n).collect();
        for r in rows {
            assert!(
                names.contains(&r) || r == SHADING_MODEL || r == REFLECTION_TYPE,
                "{r} is not a Material row"
            );
        }
    }

    #[test]
    fn brief_arrangement() {
        let first = &sections(MATERIAL_REF_TAB)[0];
        assert_eq!(first.name, "Diffuse and Specular");
        assert_eq!(first.rows[0], SHADING_MODEL);
        assert_eq!(sections(MATERIAL_REF_TAB)[1].name, "Surface Normal");
        assert!(
            sections(TRANSPARENCY_TAB)[0]
                .rows
                .contains(&"Absorption Distance")
        );
        assert!(sections(TRANSPARENCY_TAB)[0].rows.contains(&"Dispersion"));
        assert!(sections(USER_CHANNELS_TAB).is_empty());
        assert!(sections(TAGS_TAB).is_empty());
    }

    #[test]
    fn a_missing_tab_shows_the_first() {
        assert_eq!(tab_for("Rays"), RAYS_TAB);
        assert_eq!(tab_for("Surface"), MATERIAL_REF_TAB);
        assert_eq!(tab_for(""), MATERIAL_REF_TAB);
    }
}
