//! Preferences > Defaults > Mesh Items (spec preferences-2026-11-06, Tier
//! 2): what new mesh items and primitives start with, Copy and Paste
//! selection rules, Auto Hide Child Items and Defer Smoothing During
//! Interaction. Saved per user in `mesh-items-preferences.json`. Values
//! the spec has not measured are marked *provisional*; each keeps
//! Operandi's behaviour so far.
use crate::prefs_file::PrefsFile;
use eframe::egui;
use operandi_core::PolygonRepresentation;
use serde::{Deserialize, Serialize};

/// Default Primitive Polygon choices.
pub const POLYGON_TYPES: [(PolygonRepresentation, &str); 3] = [
    (PolygonRepresentation::Face, "Face"),
    (PolygonRepresentation::Subdivs, "Subdivision"),
    (PolygonRepresentation::CatmullClark, "Catmull-Clark"),
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshItemsPreferences {
    version: u8,
    /// Provisional: off.
    pub auto_item_masks: bool,
    /// Provisional: off (Ctrl-click in the item list hides the children).
    pub auto_hide_children: bool,
    /// Provisional: off.
    pub auto_uv_map: bool,
    /// New Patch Display Level. Provisional: 2, a new mesh's so far.
    pub subdivision_level: u8,
    /// New Catmull-Clark Display Level. Provisional: 2.
    pub catmull_clark_level: u8,
    /// Provisional: Face.
    pub primitive_polygon: PolygonRepresentation,
    /// Provisional: on (SYM-PR-01/02, primitives mirror with Symmetry).
    pub symmetric_primitives: bool,
    /// Provisional: off.
    pub deselect_after_copy: bool,
    /// Provisional: on, as Paste has done so far (the spec reads the
    /// reference's default as off; PREF-Q20).
    pub select_pasted: bool,
    /// Provisional: on, as Paste has done so far (PREF-Q20).
    pub deselect_before_paste: bool,
    /// Defer Smoothing During Interaction: while a drag changes a smooth
    /// mesh, its subdivision surface waits for the mouse-up. Provisional:
    /// off.
    pub defer_smoothing: bool,
}
impl Default for MeshItemsPreferences {
    fn default() -> Self {
        let d = operandi_core::user_defaults::UserDefaults::default();
        Self {
            version: 1,
            auto_item_masks: d.auto_item_masks,
            // GV-01: on by default (2026-11-10).
            auto_hide_children: true,
            auto_uv_map: d.auto_uv_map,
            subdivision_level: d.subdivision_level,
            catmull_clark_level: d.catmull_clark_level,
            primitive_polygon: d.primitive_polygon,
            symmetric_primitives: d.symmetric_primitives,
            deselect_after_copy: d.deselect_after_copy,
            select_pasted: d.select_pasted,
            deselect_before_paste: d.deselect_before_paste,
            defer_smoothing: false,
        }
    }
}
impl MeshItemsPreferences {
    /// Copy these into what core reads.
    pub fn fill(&self, d: &mut operandi_core::user_defaults::UserDefaults) {
        d.auto_item_masks = self.auto_item_masks;
        d.auto_uv_map = self.auto_uv_map;
        d.subdivision_level = self.subdivision_level;
        d.catmull_clark_level = self.catmull_clark_level;
        d.primitive_polygon = self.primitive_polygon;
        d.symmetric_primitives = self.symmetric_primitives;
        d.deselect_after_copy = self.deselect_after_copy;
        d.select_pasted = self.select_pasted;
        d.deselect_before_paste = self.deselect_before_paste;
    }
}
impl crate::prefs_file::PrefsFile for MeshItemsPreferences {
    const INVALID: &'static str = "Invalid mesh items preference";
    fn valid(&self) -> bool {
        self.version == 1
            && self.subdivision_level <= operandi_core::subdivision::MAX_LEVEL
            && self.catmull_clark_level <= operandi_core::subdivision::MAX_LEVEL
            && POLYGON_TYPES
                .iter()
                .any(|(p, _)| *p == self.primitive_polygon)
    }
}

/// The page's rows (UIC-98), in panel mode of the shared form
/// (UIC-125): the proof that property pages and dialogs share one
/// component.
fn tabs() -> Vec<crate::form::panel::Tab> {
    use crate::form::panel::{Section, Tab};
    use crate::form::{Row, Unit, When};
    let d = MeshItemsPreferences::default();
    let max = f64::from(operandi_core::subdivision::MAX_LEVEL);
    let polygon = POLYGON_TYPES
        .iter()
        .position(|(p, _)| *p == d.primitive_polygon)
        .unwrap_or(0);
    let polygons: Vec<&str> = POLYGON_TYPES.iter().map(|(_, l)| *l).collect();
    vec![Tab::new(
        "Mesh Items",
        vec![
            Section::new(
                "Mesh Items",
                vec![
                    Row::check("auto_item_masks", "Auto Create Item Masks", d.auto_item_masks)
                        .tip("New mesh items get their own shader tree group"),
                    Row::check("auto_hide_children", "Auto Hide Child Items", d.auto_hide_children)
                        .tip("Hiding or showing an item does the same to its children"),
                    Row::check("auto_uv_map", "Auto Create UV Map", d.auto_uv_map)
                        .tip("New mesh items start with an empty UV map"),
                    // UIC-117: not built, so greyed, never hidden.
                    Row::check("auto_transforms", "Auto Create Item Transforms", true)
                        .when(When::Never)
                        .tip("Not built: every Operandi item has its transform"),
                    Row::number(
                        "subdivision_level",
                        "New Patch Display Level",
                        Unit::Count,
                        f64::from(d.subdivision_level),
                    )
                    .range(0., max)
                    .tip("A new mesh's subdivision display level"),
                    Row::number(
                        "catmull_clark_level",
                        "New Catmull-Clark Display Level",
                        Unit::Count,
                        f64::from(d.catmull_clark_level),
                    )
                    .range(0., max)
                    .tip("A new Catmull-Clark mesh's display level"),
                    Row::choice("primitive_polygon", "Default Primitive Polygon", &polygons, polygon)
                        .tip("Cone and Ellipsoid's starting polygon type"),
                    Row::check(
                        "symmetric_primitives",
                        "Symmetric Primitive Creation",
                        d.symmetric_primitives,
                    )
                    .tip("With Symmetry on, primitives are made mirrored too"),
                ],
            ),
            Section::new(
                "Copy and Paste",
                vec![
                    Row::check(
                        "deselect_after_copy",
                        "Deselect Elements after Copying",
                        d.deselect_after_copy,
                    )
                    .tip("Copy leaves nothing selected"),
                    Row::check("select_pasted", "Select Pasted Elements", d.select_pasted)
                        .tip("Paste selects what it pasted"),
                    Row::check(
                        "deselect_before_paste",
                        "Deselect Elements before Pasting",
                        d.deselect_before_paste,
                    )
                    .tip("Paste clears the selection first"),
                ],
            ),
            Section::new(
                "Interaction",
                vec![
                    Row::check(
                        "defer_smoothing",
                        "Defer Smoothing During Interaction",
                        d.defer_smoothing,
                    )
                    .tip("While a drag changes a smooth mesh, its smooth surface waits for the mouse-up"),
                ],
            ),
        ],
    )]
}

impl MeshItemsPreferences {
    /// The page's values from these preferences.
    fn values(&self) -> crate::form::Values {
        use crate::form::Value;
        let mut v = crate::form::Values::default();
        for (key, on) in [
            ("auto_item_masks", self.auto_item_masks),
            ("auto_hide_children", self.auto_hide_children),
            ("auto_uv_map", self.auto_uv_map),
            ("auto_transforms", true),
            ("symmetric_primitives", self.symmetric_primitives),
            ("deselect_after_copy", self.deselect_after_copy),
            ("select_pasted", self.select_pasted),
            ("deselect_before_paste", self.deselect_before_paste),
            ("defer_smoothing", self.defer_smoothing),
        ] {
            v.set(key, Value::Bool(on));
        }
        v.set(
            "subdivision_level",
            Value::Number(f64::from(self.subdivision_level)),
        );
        v.set(
            "catmull_clark_level",
            Value::Number(f64::from(self.catmull_clark_level)),
        );
        let polygon = POLYGON_TYPES
            .iter()
            .position(|(p, _)| *p == self.primitive_polygon)
            .unwrap_or(0);
        v.set("primitive_polygon", Value::Choice(polygon));
        v
    }
    /// These preferences from the page's values.
    fn read(&mut self, v: &crate::form::Values) {
        self.auto_item_masks = v.bool("auto_item_masks");
        self.auto_hide_children = v.bool("auto_hide_children");
        self.auto_uv_map = v.bool("auto_uv_map");
        self.symmetric_primitives = v.bool("symmetric_primitives");
        self.deselect_after_copy = v.bool("deselect_after_copy");
        self.select_pasted = v.bool("select_pasted");
        self.deselect_before_paste = v.bool("deselect_before_paste");
        self.defer_smoothing = v.bool("defer_smoothing");
        self.subdivision_level = v.number("subdivision_level").round() as u8;
        self.catmull_clark_level = v.number("catmull_clark_level").round() as u8;
        self.primitive_polygon =
            POLYGON_TYPES[v.choice("primitive_polygon").min(POLYGON_TYPES.len() - 1)].0;
    }
}

impl super::Operandi {
    /// Preferences > Defaults > Mesh Items, drawn by the shared form's
    /// panel mode (UIC-100..125). The other rows the reference has
    /// (instance, curve, spline patch, vertex map and Slip UV) come with
    /// those features.
    pub(super) fn mesh_items_page(&mut self, ui: &mut egui::Ui) {
        const PANEL: &str = "Preferences: Mesh Items";
        let before = self.mesh_items_preferences.clone();
        let mut values = before.values();
        let mut state = self.dialog_memory.panel(PANEL);
        let changes = crate::form::panel::show(
            ui,
            PANEL,
            &tabs(),
            &mut state,
            &mut values,
            &Default::default(),
        );
        if changes.state {
            self.dialog_memory.set_panel(PANEL, state);
            if let Err(error) = self
                .dialog_memory
                .save(&self.environment.paths().dialog_values())
            {
                self.status = format!("Could not keep the panel's folds: {error}");
            }
        }
        self.mesh_items_preferences.read(&values);
        if self.mesh_items_preferences == before {
            return;
        }
        self.apply_user_defaults();
        if let Err(error) = self
            .mesh_items_preferences
            .save(&self.environment.paths().mesh_items_preferences())
        {
            self.status = format!("Preference changed; could not save it: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_rows_round_trip_the_preferences() {
        let rows: Vec<_> = tabs()
            .into_iter()
            .flat_map(|t| t.sections)
            .flat_map(|s| s.rows)
            .collect();
        assert_eq!(crate::form::check_rows(&rows), Ok(()));
        let mut p = MeshItemsPreferences {
            auto_hide_children: true,
            subdivision_level: 3,
            primitive_polygon: PolygonRepresentation::CatmullClark,
            select_pasted: false,
            ..Default::default()
        };
        let values = p.values();
        let mut back = MeshItemsPreferences::default();
        back.read(&values);
        assert_eq!(back, p);
        // Every row has a value and the unbuilt one is greyed (UIC-117).
        for row in &rows {
            assert!(values.get(&row.key).is_some(), "{}", row.key);
        }
        let unbuilt = rows.iter().find(|r| r.key == "auto_transforms").unwrap();
        assert!(!values.enabled(unbuilt));
        p.read(&MeshItemsPreferences::default().values());
        assert_eq!(p, MeshItemsPreferences::default());
    }
}
