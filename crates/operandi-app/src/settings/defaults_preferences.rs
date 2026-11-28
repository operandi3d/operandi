//! Tool defaults, 3D information overlays and file info (specs
//! preferences-2026-11-04/MESH-TOOL-DEFAULTS.md and OVERLAYS-UI.md).
//! Saved per user in `defaults-preferences.json`. Values the spec has not
//! measured yet are marked *provisional*.
use crate::prefs_file::PrefsFile;
use crate::unavailable::Gate;
use eframe::egui::{self, Align2, FontId, Painter, Rect, Stroke, vec2};
use operandi_core::{ActionContext, AxisOrientation, CenterMode, Mode};
use serde::{Deserialize, Serialize};

/// A question's "In the Future" choice (Jason, 2026-11-21): keep asking,
/// or answer it the same way from now on. Reset "In the Future" Messages
/// in Preferences puts every one back to Ask.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InTheFuture {
    #[default]
    Ask,
    AlwaysYes,
    AlwaysNo,
}
impl InTheFuture {
    /// The dropdown's entries, in `index` order.
    pub const LABELS: [&'static str; 3] = ["Always Ask", "Always Yes", "Always No"];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn from_index(index: usize) -> Self {
        match index {
            1 => Self::AlwaysYes,
            2 => Self::AlwaysNo,
            _ => Self::Ask,
        }
    }
}

/// Action centre choices, saved by name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Center {
    /// Provisional default: Operandi's start-up centre so far.
    #[default]
    None,
    Selection,
    Origin,
    Automatic,
    /// Whatever the mode used last, kept between launches.
    LastSelected,
}
impl Center {
    pub const ALL: [Self; 5] = [
        Self::None,
        Self::Selection,
        Self::Origin,
        Self::Automatic,
        Self::LastSelected,
    ];
    /// The action centre this choice starts with (Last Selected: `last`).
    pub fn mode_or(self, last: CenterMode) -> CenterMode {
        match self {
            Self::None => CenterMode::None,
            Self::Selection => CenterMode::Selection,
            Self::Origin => CenterMode::Origin,
            Self::Automatic => CenterMode::Automatic,
            Self::LastSelected => last,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::LastSelected => "Last Selected",
            c => c.mode_or(CenterMode::None).label(),
        }
    }
}
/// Action axis choices, saved by name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axis {
    /// Provisional default: Operandi's start-up axis so far.
    #[default]
    World,
    Selection,
    WorkPlaneXz,
    WorkPlaneYz,
    WorkPlaneXy,
    /// Whatever the mode used last, kept between launches.
    LastSelected,
}
impl Axis {
    pub const ALL: [Self; 6] = [
        Self::World,
        Self::Selection,
        Self::WorkPlaneXz,
        Self::WorkPlaneYz,
        Self::WorkPlaneXy,
        Self::LastSelected,
    ];
    /// The action axis this choice starts with (Last Selected: `last`).
    pub fn orientation_or(self, last: AxisOrientation) -> AxisOrientation {
        match self {
            Self::World => AxisOrientation::World,
            Self::Selection => AxisOrientation::Selection,
            Self::WorkPlaneXz => AxisOrientation::Workplane(0),
            Self::WorkPlaneYz => AxisOrientation::Workplane(1),
            Self::WorkPlaneXy => AxisOrientation::Workplane(2),
            Self::LastSelected => last,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::LastSelected => "Last Selected",
            a => a.orientation_or(AxisOrientation::World).label(),
        }
    }
}

/// Which readouts the 3D views show (Display > 3D Information Overlays).
/// Provisional (OUI-Q1): Current Selection, OpenGL Polygon Count and Grid
/// Size on (Jason 2026-11-06: he expects the readout without hunting for
/// it); Polygon Type and Current Tool off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Overlays {
    pub current_selection: bool,
    pub polygon_count: bool,
    pub grid_size: bool,
    pub polygon_type: bool,
    pub current_tool: bool,
    /// Which snapping is on. Provisional: off.
    pub snapping_state: bool,
}
impl Default for Overlays {
    fn default() -> Self {
        Self {
            current_selection: true,
            polygon_count: true,
            grid_size: true,
            polygon_type: false,
            current_tool: false,
            snapping_state: false,
        }
    }
}
/// Bumped when the overlay defaults change, so a file saved under the old
/// all-off defaults picks up the new ones once.
const OVERLAY_REVISION: u8 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DefaultsPreferences {
    version: u8,
    pub component_center: Center,
    pub component_axis: Axis,
    pub item_center: Center,
    pub item_axis: Axis,
    pub overlays: Overlays,
    /// Written into exported files that support it (glTF). Measured: empty.
    pub author: String,
    pub copyright: String,
    /// SP-17: the name the Polygon Set Part dialog offers when the target
    /// polygons don't already share a part. Provisional (SP-Q05): `Default`.
    pub default_part: String,
    /// Which overlay defaults this file has seen (`OVERLAY_REVISION`).
    /// A file without the field predates the revisions, so it reads as 0
    /// (the struct-level default would give the current revision).
    #[serde(default = "pre_revision_overlays")]
    pub overlay_revision: u8,
    /// The action centre and axis last used in component modes and in
    /// Item mode, for the Last Selected choice.
    pub last_component_center: Center,
    pub last_component_axis: Axis,
    pub last_item_center: Center,
    pub last_item_axis: Axis,
    /// Default Material: the tag new primitives' polygons get, and Set
    /// Material's starting name. Provisional (MTD-Q1): `Default`.
    pub default_material: String,
    /// Default Texture Map: the name of new and auto-created UV maps.
    /// Provisional: `Texture`, the name primitives used so far.
    pub texture_map: String,
    /// Item Index Style. Provisional: `Item (2)`, the naming so far.
    pub index_style: operandi_core::user_defaults::IndexStyle,
    /// Default Scene: a scene file every new scene (and launch) starts
    /// from. Empty: none (measured).
    pub default_scene: String,
    /// Use OS Color Picker: colour swatches open the system picker.
    /// Measured: off.
    pub os_color_picker: bool,
    /// Delete's "only one mesh item" question: ask, or always answer Yes
    /// or No (`InTheFuture`).
    pub last_mesh_delete: InTheFuture,
}
fn pre_revision_overlays() -> u8 {
    0
}
impl Default for DefaultsPreferences {
    fn default() -> Self {
        Self {
            version: 1,
            component_center: Center::None,
            component_axis: Axis::World,
            item_center: Center::None,
            item_axis: Axis::World,
            overlays: Overlays::default(),
            author: String::new(),
            copyright: String::new(),
            default_part: "Default".into(),
            overlay_revision: OVERLAY_REVISION,
            last_component_center: Center::None,
            last_component_axis: Axis::World,
            last_item_center: Center::None,
            last_item_axis: Axis::World,
            default_material: operandi_core::material::DEFAULT_MATERIAL.into(),
            texture_map: operandi_core::PRIMITIVE_UV_MAP.into(),
            index_style: Default::default(),
            default_scene: String::new(),
            os_color_picker: false,
            last_mesh_delete: InTheFuture::Ask,
        }
    }
}
impl DefaultsPreferences {
    /// The choice a mode's last-used action centre and axis are saved as;
    /// None for those the Default lists can't name.
    pub fn remembered(context: ActionContext) -> (Option<Center>, Option<Axis>) {
        let center = Center::ALL
            .into_iter()
            .filter(|c| *c != Center::LastSelected)
            .find(|c| c.mode_or(CenterMode::None) == context.center);
        let axis = Axis::ALL
            .into_iter()
            .filter(|a| *a != Axis::LastSelected)
            .find(|a| a.orientation_or(AxisOrientation::World) == context.axis);
        (center, axis)
    }
    /// The action context a new session starts with in `mode`.
    pub fn action_context(&self, mode: Mode) -> ActionContext {
        let (center, axis, last_center, last_axis) = if mode == Mode::Item {
            (
                self.item_center,
                self.item_axis,
                self.last_item_center,
                self.last_item_axis,
            )
        } else {
            (
                self.component_center,
                self.component_axis,
                self.last_component_center,
                self.last_component_axis,
            )
        };
        ActionContext {
            center: center.mode_or(last_center.mode_or(CenterMode::None)),
            axis: axis.orientation_or(last_axis.orientation_or(AxisOrientation::World)),
        }
    }
    pub fn file_info(&self) -> operandi_core::interchange::FileInfo {
        operandi_core::interchange::FileInfo {
            author: self.author.clone(),
            copyright: self.copyright.clone(),
        }
    }
}
impl crate::prefs_file::PrefsFile for DefaultsPreferences {
    const INVALID: &'static str = "Invalid defaults preference";
    fn valid(&self) -> bool {
        self.version == 1
            && self.author.len() <= 256
            && self.copyright.len() <= 256
            && self.default_part.len() <= operandi_core::MAX_PART_NAME_BYTES
            && self.default_material.len() <= 1024
            && !self.default_material.chars().any(char::is_control)
            && self.texture_map.len() <= 1024
            && !self.texture_map.chars().any(char::is_control)
            && self.default_scene.len() <= 4096
            && !matches!(self.last_component_center, Center::LastSelected)
            && !matches!(self.last_item_center, Center::LastSelected)
            && !matches!(self.last_component_axis, Axis::LastSelected)
            && !matches!(self.last_item_axis, Axis::LastSelected)
    }
    fn upgrade(mut self) -> Self {
        if self.overlay_revision < OVERLAY_REVISION {
            self.overlays.current_selection = true;
            self.overlays.polygon_count = true;
            self.overlays.grid_size = true;
            self.overlay_revision = OVERLAY_REVISION;
        }
        self
    }
}

/// `12345` as `12,345`.
pub(crate) fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

impl super::Operandi {
    /// Start-up action centre and axis, at launch and for each new scene.
    pub(super) fn apply_tool_defaults(&mut self) {
        self.editor.action_context = self
            .defaults_preferences
            .action_context(self.editor.scene.selection.mode);
    }
    /// Keep the action centre and axis in use for Last Selected (saved
    /// only when they change; called once per frame).
    pub(super) fn remember_action_context(&mut self) {
        let item = self.editor.scene.selection.mode == Mode::Item;
        let (center, axis) = DefaultsPreferences::remembered(self.editor.action_context);
        let d = &mut self.defaults_preferences;
        let (last_center, last_axis) = if item {
            (&mut d.last_item_center, &mut d.last_item_axis)
        } else {
            (&mut d.last_component_center, &mut d.last_component_axis)
        };
        let mut changed = false;
        if let Some(c) = center
            && *last_center != c
        {
            *last_center = c;
            changed = true;
        }
        if let Some(a) = axis
            && *last_axis != a
        {
            *last_axis = a;
            changed = true;
        }
        if changed {
            let _ = self
                .defaults_preferences
                .save(&self.environment.paths().defaults_preferences());
        }
    }
    /// The Current Selection readout: what the active mode's tools will work
    /// on. Nothing selected in a component mode means the tools act on the
    /// whole mesh, so it reads "All 482 Polygons"; otherwise "3 of 482
    /// Polygons".
    pub(super) fn selection_summary(&self) -> String {
        let scene = &self.editor.scene;
        let s = &scene.selection;
        // Just the selected count ("3 Polygons"), or "All Polygons" when
        // nothing is selected: "n of N" got unwieldy on large meshes
        // (Jason, 2026-11-06).
        let counted = |selected: usize, one: &str, many: &str| match selected {
            0 => format!("All {many}"),
            1 => format!("1 {one}"),
            n => format!("{} {many}", thousands(n as u64)),
        };
        if s.mode == Mode::Item {
            let selected = scene.selected_items();
            return match selected.as_slice() {
                [] => "No Items".into(),
                [one] => scene
                    .item(*one)
                    .map_or_else(|| "1 Item".into(), |i| i.name.clone()),
                many => format!("{} Items", thousands(many.len() as u64)),
            };
        }
        if scene.active().is_none() {
            return "No active mesh".into();
        }
        match s.mode {
            Mode::Vertex => counted(s.vertices.len(), "Vertex", "Vertices"),
            Mode::Edge => counted(s.edges.len(), "Edge", "Edges"),
            _ => counted(s.faces.len(), "Polygon", "Polygons"),
        }
    }
    /// `item`'s edges, cached per mesh: listing them costs several ms on a
    /// large mesh, too much to repeat every frame.
    pub(super) fn mesh_edges(
        &self,
        item: &operandi_core::scene::Item,
    ) -> std::sync::Arc<Vec<(u32, u32)>> {
        let mut cache = self.edge_cache.borrow_mut();
        match cache.get(&item.id) {
            Some((mesh, edges)) if *mesh == item.mesh => edges.clone(),
            _ => {
                let edges = std::sync::Arc::new(item.mesh.edges());
                cache.insert(item.id, (item.mesh.clone().into(), edges.clone()));
                edges
            }
        }
    }
    /// Triangles the views draw for the visible items (the "GL:" readout).
    /// Items held on the graphics card count their cached fill, so a frame
    /// costs a lookup per item even at 10M+ triangles; others (no GPU, an
    /// item a primitive tool is drawing, a shape preview) are counted from
    /// their polygon list, which for a mesh with no SubD or polylines is a
    /// remembered number, not a pass. Items with a Mesh Stack count the result
    /// the views draw (MO-01). The graphics memory watch counts the same way
    /// (PREF-Q21, Jason 2026-11-13: triangles).
    pub(super) fn drawn_triangles(&self) -> u64 {
        self.drawn_triangles_where(|_| true)
    }
    /// [`Self::drawn_triangles`] for the items `count` picks: the
    /// Statistics panel's Drawn (GPU) Active and Background rows read the
    /// same count (statistics_ui.rs).
    pub(super) fn drawn_triangles_where(&self, count: impl Fn(u64) -> bool) -> u64 {
        // Items with a Mesh Stack draw their result; that scene is only built
        // when some item has a stack.
        let shown = self
            .editor
            .scene
            .items
            .iter()
            .any(|i| i.mesh_stack.is_some())
            .then(|| self.editor.evaluated_display())
            .flatten();
        let scene = shown.as_ref().unwrap_or(&self.editor.scene);
        // The views' own test (viewport.rs): the eye, and Display Visible
        // No or a hiding parent (CP-D01; empty in the common case).
        let display_hidden = scene.display_hidden();
        scene
            .items
            .iter()
            .filter(|i| i.visible && !display_hidden.contains(&i.id) && count(i.id))
            .map(|i| {
                self.resident_cache.drawn_triangles(i).unwrap_or_else(|| {
                    operandi_core::subdivision::drawn_triangles(
                        &i.mesh,
                        &i.subdivision.clone().unwrap_or_default(),
                    )
                })
            })
            .sum()
    }
    /// The readout lines the 3D views show, top to bottom.
    pub(super) fn overlay_lines(&self, r: Rect) -> Vec<String> {
        let o = self.defaults_preferences.overlays;
        let scene = &self.editor.scene;
        let mut lines = Vec::new();
        if o.current_selection {
            lines.push(self.selection_summary());
        }
        if o.polygon_count {
            lines.push(format!("GL: {}", thousands(self.drawn_triangles())));
        }
        if o.grid_size {
            let step = crate::grid::spacing(
                &self.camera,
                r,
                self.actual_plane(),
                self.workplane_center(),
            );
            lines.push(format!(
                "Grid: {}",
                operandi_core::units::format_distance(step)
            ));
        }
        if o.polygon_type {
            let mut kinds = std::collections::BTreeSet::new();
            for i in scene.items.iter().filter(|i| scene.shows(i)) {
                let summary = i.mesh.face_summary();
                if summary.smooth < i.mesh.faces.len() {
                    kinds.insert(false);
                }
                if summary.smooth > 0 {
                    kinds.insert(true);
                }
            }
            lines.push(
                match (kinds.contains(&false), kinds.contains(&true)) {
                    (true, true) => "Polygons: mixed",
                    (false, true) => "Polygons: Catmull-Clark",
                    _ => "Polygons: face",
                }
                .into(),
            );
        }
        if o.current_tool {
            let tool = self
                .editor
                .session
                .live
                .as_ref()
                .map_or("No tool", |l| l.tool.label());
            let falloff = self.editor.spatial_falloff.map_or("None", |f| f.label());
            let context = self.editor.action_context;
            // SN-13: snapping state and its active targets.
            let snapping = if self.snapping.enabled {
                format!(
                    " · Snapping {}",
                    self.snapping
                        .active(self.editor.scene.selection.mode)
                        .names()
                )
            } else {
                String::new()
            };
            lines.push(format!(
                "{tool} · Falloff {falloff} · Center {} · Axis {}{snapping}",
                context.center.label(),
                context.axis.label()
            ));
        }
        if o.snapping_state {
            lines.push(if self.snapping.enabled {
                format!(
                    "Snapping: {}",
                    self.snapping
                        .active(self.editor.scene.selection.mode)
                        .names()
                )
            } else {
                "Snapping: Off".into()
            });
        }
        // Always shown: some items are graphics memory stand-ins.
        lines.extend(self.gpu_memory.note());
        lines
    }
    pub(super) fn draw_overlays(&self, painter: &Painter, r: Rect) {
        // SN2-43b: the unsnapped reticle, where the point would be without
        // snapping, while a snapping drag runs.
        if let Some(point) = self.snap_reticle
            && let Some((at, _)) = self.camera.project(point, r)
        {
            let color = crate::theme::viewport::snap_unsnapped();
            let stroke = Stroke::new(1.5_f32, color);
            painter.circle_stroke(at, 4., stroke);
            for d in [egui::vec2(1., 0.), egui::vec2(0., 1.)] {
                painter.line_segment([at + d * 6., at + d * 10.], stroke);
                painter.line_segment([at - d * 6., at - d * 10.], stroke);
            }
        }
        // SN-08: the pre-highlighted snap target, filled once snapped.
        if let Some((point, snapped)) = self.snap_marker
            && let Some((at, _)) = self.camera.project(point, r)
        {
            // SN2-43: Unsnapped while pre-highlighted, Snapped once on it.
            let color = if snapped {
                crate::theme::viewport::snap_target()
            } else {
                crate::theme::viewport::snap_unsnapped()
            };
            painter.circle_stroke(at, 7., Stroke::new(2.0_f32, color));
            if snapped {
                painter.circle_filled(at, 3.5, color);
            }
        }
        self.draw_fov_readout(painter, r);
        self.draw_drag_readout(painter, r);
        // Lower right, right-aligned (the axis gizmo and view labels own the
        // lower left). The selection line, on top, uses the selection colour.
        let lines = self.overlay_lines(r);
        let first_is_selection = self.defaults_preferences.overlays.current_selection;
        let rows = lines.len();
        for (row, line) in lines.iter().rev().enumerate() {
            let color = if first_is_selection && row + 1 == rows {
                crate::theme::viewport::selection()
            } else {
                crate::theme::text()
            };
            painter.text(
                r.right_bottom() + vec2(-10., -10. - row as f32 * 16.),
                Align2::RIGHT_BOTTOM,
                line,
                FontId::proportional(12.),
                color,
            );
        }
    }

    /// Preferences > Defaults > Tool Settings.
    pub(super) fn tool_settings_page(&mut self, ui: &mut egui::Ui) {
        let before = self.defaults_preferences.clone();
        let d = &mut self.defaults_preferences;
        use crate::form::rows;
        let dd = DefaultsPreferences::default();
        let centers: Vec<_> = Center::ALL.iter().map(|&c| (c, c.label())).collect();
        let axes: Vec<_> = Axis::ALL.iter().map(|&a| (a, a.label())).collect();
        rows::section(ui, "Tool Settings", false, |ui| {
            rows::choice_d(
                ui,
                "Default Action Center (Component modes)",
                &mut d.component_center,
                &dd.component_center,
                &centers,
                "The action center component modes start with",
            );
            rows::choice_d(
                ui,
                "Default Action Axis (Component modes)",
                &mut d.component_axis,
                &dd.component_axis,
                &axes,
                "The action axis component modes start with",
            );
            rows::choice_d(
                ui,
                "Default Action Center (Item mode)",
                &mut d.item_center,
                &dd.item_center,
                &centers,
                "The action center Item mode starts with",
            );
            rows::choice_d(
                ui,
                "Default Action Axis (Item mode)",
                &mut d.item_axis,
                &dd.item_axis,
                &axes,
                "The action axis Item mode starts with",
            );
        });
        rows::note(
            ui,
            "Used at launch and for each new scene; Last Selected starts with what that \
             mode used last. Not built yet: Auto Activate and Select Through (tools have \
             no per-tool switch for them).",
        );
        rows::section(ui, "Polygon Tags", false, |ui| {
            rows::text_d(
                ui,
                "Default Part",
                &mut d.default_part,
                &dd.default_part,
                "Name given to new parts",
            );
            truncate_chars(&mut d.default_part, 256);
        });
        self.save_defaults(before);
    }
    /// Preferences > Display > 3D Information Overlays.
    pub(super) fn overlays_page(&mut self, ui: &mut egui::Ui) {
        let before = self.defaults_preferences.clone();
        let o = &mut self.defaults_preferences.overlays;
        use crate::form::rows;
        let d = DefaultsPreferences::default().overlays;
        rows::section(ui, "3D Information Overlays", false, |ui| {
            rows::check_d(
                ui,
                "Current Selection",
                &mut o.current_selection,
                d.current_selection,
                "What is selected, in the selection colour",
            );
            rows::check_d(
                ui,
                "OpenGL Polygon Count",
                &mut o.polygon_count,
                d.polygon_count,
                "How many triangles the views draw (GL: n)",
            );
            rows::check_d(
                ui,
                "Grid Size",
                &mut o.grid_size,
                d.grid_size,
                "The grid spacing at the current zoom",
            );
            rows::check_d(
                ui,
                "Polygon Type",
                &mut o.polygon_type,
                d.polygon_type,
                "The active mesh's polygon type",
            );
            rows::check_d(
                ui,
                "Current Tool",
                &mut o.current_tool,
                d.current_tool,
                "The active tool's name",
            );
            rows::check_d(
                ui,
                "Snapping State",
                &mut o.snapping_state,
                d.snapping_state,
                "Whether snapping is on, and to what",
            );
        });
        rows::note(ui, "Shown at the lower right of each 3D view.");
        self.save_defaults(before);
    }
    /// Preferences > Defaults > Scene Save/Export.
    pub(super) fn file_info_page(&mut self, ui: &mut egui::Ui) {
        let before = self.defaults_preferences.clone();
        let d = &mut self.defaults_preferences;
        use crate::form::rows;
        let dd = DefaultsPreferences::default();
        rows::section(ui, "Scene Save/Export", false, |ui| {
            rows::text_d(
                ui,
                "Author",
                &mut d.author,
                &dd.author,
                "Written into glTF/GLB exports",
            );
            truncate_chars(&mut d.author, 256);
            rows::text_d(
                ui,
                "Copyright",
                &mut d.copyright,
                &dd.copyright,
                "Written into glTF/GLB exports",
            );
            truncate_chars(&mut d.copyright, 256);
        });
        rows::note(
            ui,
            "Written into glTF/GLB exports. Files are always saved as UTF-8.",
        );
        self.save_defaults(before);
    }
    /// Preferences > Defaults > Application, below Undo: names new things
    /// get, the Default Scene and the colour picker.
    pub(super) fn application_defaults_ui(&mut self, ui: &mut egui::Ui) {
        let before = self.defaults_preferences.clone();
        let mut choose = false;
        let d = &mut self.defaults_preferences;
        use crate::form::rows;
        let dd = DefaultsPreferences::default();
        let styles: Vec<_> = operandi_core::user_defaults::IndexStyle::ALL
            .iter()
            .map(|&x| (x, x.label()))
            .collect();
        rows::section(ui, "Names and Files", false, |ui| {
            rows::text_d(
                ui,
                "Default Material",
                &mut d.default_material,
                &dd.default_material,
                "Material tag of new primitives' polygons; Set Material starts with it",
            );
            truncate_chars(&mut d.default_material, 256);
            rows::text_d(
                ui,
                "Default Texture Map",
                &mut d.texture_map,
                &dd.texture_map,
                "Name of the UV map new primitives get",
            );
            truncate_chars(&mut d.texture_map, 256);
            rows::choice_d(
                ui,
                "Item Index Style",
                &mut d.index_style,
                &dd.index_style,
                &styles,
                "How a number is added when a name is taken",
            );
            rows::text_d(
                ui,
                "Default Scene",
                &mut d.default_scene,
                &dd.default_scene,
                "A scene file each new scene starts from (empty: none)",
            );
            truncate_chars(&mut d.default_scene, 4096);
            // UIC-116: the field's buttons on the rows under it.
            choose = rows::button(ui, "Choose…")
                .on_hover_text("Pick the default scene file")
                .clicked();
            ui.gated_ui(
                crate::unavailable::unless(!d.default_scene.is_empty(), "No default scene is set"),
                |ui| {
                    if rows::button(ui, "Clear")
                        .on_hover_text("No default scene")
                        .clicked()
                    {
                        d.default_scene.clear();
                    }
                },
            );
            rows::check_d(
                ui,
                "Use OS Color Picker",
                &mut d.os_color_picker,
                dd.os_color_picker,
                "Colour swatches open the system colour panel",
            );
        });
        // UIC-116: a panel button on its own row. Greyed, never hidden
        // (UIC-117), while every question still asks.
        ui.gated_ui(
            crate::unavailable::unless(
                self.defaults_preferences.last_mesh_delete != InTheFuture::Ask,
                "Every question already asks",
            ),
            |ui| {
                if rows::button(ui, "Reset \"In the Future\" Messages")
                    .on_hover_text("Questions you told to stop asking ask again")
                    .clicked()
                {
                    self.defaults_preferences.last_mesh_delete = InTheFuture::Ask;
                }
            },
        );
        let names_changed = self.defaults_preferences.default_material != before.default_material
            || self.defaults_preferences.texture_map != before.texture_map
            || self.defaults_preferences.index_style != before.index_style
            || self.defaults_preferences.os_color_picker != before.os_color_picker;
        self.save_defaults(before);
        if names_changed {
            self.apply_user_defaults();
        }
        if choose {
            let folder = std::path::Path::new(&self.defaults_preferences.default_scene)
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_else(|| crate::file_chooser::split(&self.open_dialog_default()).0);
            if !self.choose(
                crate::file_chooser::Choose::Open { folder },
                crate::file_chooser::Purpose::DefaultScene,
            ) {
                self.status = "File panel unavailable".into();
            }
        }
    }
    pub(super) fn set_default_scene(&mut self, path: std::path::PathBuf) {
        let before = self.defaults_preferences.clone();
        self.defaults_preferences.default_scene = path.display().to_string();
        self.save_defaults(before);
    }
    pub(super) fn save_defaults(&mut self, before: DefaultsPreferences) {
        if self.defaults_preferences == before {
            return;
        }
        if let Err(error) = self
            .defaults_preferences
            .save(&self.environment.paths().defaults_preferences())
        {
            self.status = format!("Preference changed; could not save it: {error}");
        }
    }
}

/// Cuts `text` to at most `limit` characters (the old fields' limit).
fn truncate_chars(text: &mut String, limit: usize) {
    if let Some((at, _)) = text.char_indices().nth(limit) {
        text.truncate(at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Operandi};
    use std::fs;

    #[test]
    fn action_defaults_follow_the_starting_mode() {
        let mut a = Operandi::empty();
        a.defaults_preferences.component_center = Center::Selection;
        a.defaults_preferences.component_axis = Axis::WorkPlaneXy;
        a.defaults_preferences.item_center = Center::Origin;
        a.apply_tool_defaults();
        assert_eq!(a.editor.action_context.center, CenterMode::Selection);
        assert_eq!(a.editor.action_context.axis, AxisOrientation::Workplane(2));
        a.editor.scene.selection.mode = Mode::Item;
        a.apply_tool_defaults();
        assert_eq!(a.editor.action_context.center, CenterMode::Origin);
        assert_eq!(a.editor.action_context.axis, AxisOrientation::World);
    }

    #[test]
    fn overlays_report_selection_count_and_tool() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::cube_geometry(glam::DVec3::ZERO, glam::DVec3::ONE)
                .unwrap()
                .into();
        let r = Rect::from_min_size(egui::Pos2::ZERO, vec2(800., 600.));
        assert_eq!(
            a.overlay_lines(r).len(),
            3,
            "provisional default: selection, polygon count and grid on"
        );
        a.defaults_preferences.overlays = Overlays {
            current_selection: true,
            polygon_count: true,
            grid_size: true,
            polygon_type: true,
            current_tool: true,
            snapping_state: true,
        };
        a.editor.scene.selection.mode = Mode::Polygon;
        a.editor.scene.selection.faces = vec![0, 1];
        let lines = a.overlay_lines(r);
        assert_eq!(lines[0], "2 Polygons");
        assert_eq!(lines[1], "GL: 12");
        assert!(lines[2].starts_with("Grid: ") && lines[2].ends_with(" m"));
        assert_eq!(lines[3], "Polygons: face");
        assert!(lines[4].starts_with("No tool · Falloff None"));
        assert_eq!(lines[5], "Snapping: Off");
    }

    #[test]
    fn selection_readout_names_mode_counts_and_all() {
        let mut a = Operandi::empty();
        assert_eq!(a.selection_summary(), "No active mesh");
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::cube_geometry(glam::DVec3::ZERO, glam::DVec3::ONE)
                .unwrap()
                .into();
        let s = &mut a.editor.scene.selection;
        s.mode = Mode::Polygon;
        assert_eq!(a.selection_summary(), "All Polygons", "nothing selected");
        a.editor.scene.selection.faces = vec![0];
        assert_eq!(a.selection_summary(), "1 Polygon");
        a.editor.scene.selection.mode = Mode::Vertex;
        a.editor.scene.selection.vertices = vec![0, 1, 2];
        assert_eq!(a.selection_summary(), "3 Vertices");
        a.editor.scene.selection.mode = Mode::Edge;
        assert_eq!(a.selection_summary(), "All Edges");
        a.editor.scene.selection.edges = vec![(0, 1)];
        assert_eq!(a.selection_summary(), "1 Edge");
        a.editor.scene.selection.mode = Mode::Item;
        let name = a.editor.scene.active().unwrap().name.clone();
        assert_eq!(a.selection_summary(), name);
        a.editor.scene.select_item(None);
        assert_eq!(a.selection_summary(), "No Items");
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1,000");
        assert_eq!(thousands(1234567), "1,234,567");
    }

    #[test]
    fn old_all_off_overlays_file_gets_the_new_defaults_once() {
        let dir = std::env::temp_dir().join(format!("operandi-overlays-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("defaults-preferences.json");
        let off = r#"{"version":1,"overlays":{"current_selection":false,"polygon_count":false,"grid_size":false,"polygon_type":true,"current_tool":false}}"#;
        fs::write(&path, off).unwrap();
        let p = DefaultsPreferences::load(&path);
        assert!(p.overlays.current_selection && p.overlays.polygon_count && p.overlays.grid_size);
        assert!(p.overlays.polygon_type, "other choices kept");
        // Turned off after the upgrade, it stays off.
        let mut p = p;
        p.overlays.grid_size = false;
        p.save(&path).unwrap();
        assert!(!DefaultsPreferences::load(&path).overlays.grid_size);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn subdivided_triangle_count() {
        use operandi_core::PolygonRepresentation::{CatmullClark, Face, Subdivs};
        use operandi_core::subdivision::{SubdivisionSettings, drawn_triangles};
        let at = |cc: u8, sub: u8| SubdivisionSettings {
            catmull_clark_level: cc,
            subdivision_level: sub,
            ..Default::default()
        };
        let mut cube = operandi_core::cube_geometry(glam::DVec3::ZERO, glam::DVec3::ONE).unwrap();
        assert_eq!(
            drawn_triangles(&cube, &at(2, 2)),
            12,
            "plain quads draw as two triangles"
        );
        // Smooth shading alone is not subdivision.
        for f in &mut cube.faces {
            f.smooth = true;
        }
        assert_eq!(drawn_triangles(&cube, &at(2, 2)), 12);
        cube.attributes.representations = vec![CatmullClark; 6];
        assert_eq!(drawn_triangles(&cube, &at(0, 2)), 12);
        assert_eq!(drawn_triangles(&cube, &at(1, 2)), 24 * 2);
        assert_eq!(drawn_triangles(&cube, &at(2, 2)), 96 * 2);
        cube.attributes.representations[0] = Face;
        assert_eq!(drawn_triangles(&cube, &at(1, 2)), 20 * 2 + 2);
        // Subdivision polygons: L² quads per corner.
        cube.attributes.representations = vec![Subdivs; 6];
        assert_eq!(drawn_triangles(&cube, &at(2, 3)), 6 * 4 * 9 * 2);
    }

    #[test]
    fn gltf_export_carries_author_and_copyright() {
        let dir = std::env::temp_dir().join(format!("operandi-gltf-info-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh =
            operandi_core::cube_geometry(glam::DVec3::ZERO, glam::DVec3::ONE)
                .unwrap()
                .into();
        a.defaults_preferences.author = "Jason".into();
        a.defaults_preferences.copyright = "© 2026".into();
        let path = dir.join("cube.gltf");
        a.export_to(path.clone(), operandi_core::interchange::ExportFormat::Gltf);
        let json: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(json["asset"]["copyright"], "© 2026");
        assert_eq!(json["asset"]["extras"]["author"], "Jason");
        // Empty fields write nothing.
        a.defaults_preferences.author.clear();
        a.defaults_preferences.copyright.clear();
        a.export_to(path.clone(), operandi_core::interchange::ExportFormat::Gltf);
        let json: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert!(json["asset"].get("copyright").is_none());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn preferences_roundtrip() {
        let dir = std::env::temp_dir().join(format!("operandi-defaults-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("defaults-preferences.json");
        assert_eq!(
            DefaultsPreferences::load(&path),
            DefaultsPreferences::default()
        );
        // A file saved before Default Part existed still loads, with `Default`.
        fs::write(&path, br#"{"version":1,"author":"Jason"}"#).unwrap();
        let old = DefaultsPreferences::load(&path);
        assert_eq!(old.author, "Jason");
        assert_eq!(old.default_part, "Default");
        let p = DefaultsPreferences {
            item_axis: Axis::Selection,
            author: "Jason".into(),
            default_part: "Panel".into(),
            overlays: Overlays {
                grid_size: true,
                ..Default::default()
            },
            ..Default::default()
        };
        p.save(&path).unwrap();
        assert_eq!(DefaultsPreferences::load(&path), p);
        fs::remove_dir_all(dir).unwrap();
    }
}
