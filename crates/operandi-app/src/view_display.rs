//! Per-view display settings (specifications/viewport-display-2026-11-03):
//! how each 3D view draws its active meshes (the selected item) and its
//! inactive ones (visible, unselected), plus the view's quick toggles and
//! the user's saved presets. None of this is scene data: changing it is
//! never an undoable edit, and presets persist with the preferences.
use operandi_core::Mode;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

/// How a group's polygons fill. Saved by variant name, so the menu order
/// (`ALL`) and the shader numbers (`code`) can change without breaking
/// saved presets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Style {
    /// Edges and points only, no fill; back edges show.
    Wireframe,
    /// Flat, unlit fill.
    Solid,
    /// Lit fill with the fixed view light; never shadowed.
    Shaded,
    /// Faces colored by the mesh's vertex map.
    VertexMap,
    /// Shadowless warm-to-cool tone shading.
    Gooch,
    /// Lighting in a few flat bands.
    Cel,
    /// Matcap (named Reflection before 2026-11-10): the view's matcap,
    /// looked up by which way each point faces (matcap.rs).
    Reflection,
    /// The material colour (and image maps), lit by the view light.
    Texture,
    /// Shaded plus layered image maps. Until Operandi has image maps it
    /// draws exactly as Shaded. New views start here.
    #[default]
    Default,
    /// Default plus a small highlight and shadows from the view light (the
    /// scene's light with Show Scene Lighting).
    Advanced,
}
impl Style {
    /// The Display menu's order.
    pub const ALL: [Style; 10] = [
        Style::Wireframe,
        Style::Solid,
        Style::VertexMap,
        Style::Shaded,
        Style::Texture,
        Style::Default,
        Style::Advanced,
        Style::Gooch,
        Style::Cel,
        Style::Reflection,
    ];
    /// The menu's name.
    pub fn label(self) -> &'static str {
        match self {
            Style::Wireframe => "Wireframe",
            Style::Solid => "Solid",
            Style::Shaded => "Shaded",
            Style::VertexMap => "Vertex Map",
            Style::Gooch => "Gooch Tone Shading",
            Style::Cel => "Cel Shading",
            // Operandi's name (Jason, 2026-11-10); "Reflection" still
            // works in commands and the MCP bridge (`old_label`).
            Style::Reflection => "Matcap",
            Style::Texture => "Texture",
            Style::Default => "Default",
            Style::Advanced => "Advanced",
        }
    }
    /// The style's earlier name, still accepted by commands and the MCP
    /// bridge so scripts keep working: Matcap was Reflection.
    pub fn old_label(self) -> Option<&'static str> {
        (self == Style::Reflection).then_some("Reflection")
    }
    /// The Display button's name: the menu's, shortened where it is long.
    pub fn short_label(self) -> &'static str {
        match self {
            Style::Gooch => "Gooch",
            Style::Cel => "Cel",
            s => s.label(),
        }
    }
    /// The shader's style number (display_shading.rs WGSL `shade`).
    pub fn code(self) -> u32 {
        match self {
            Style::Wireframe => 0,
            Style::Solid => 1,
            Style::Shaded => 2,
            Style::VertexMap => 3,
            Style::Gooch => 4,
            Style::Cel => 5,
            Style::Reflection => 6,
            Style::Texture => 7,
            Style::Default => 8,
            Style::Advanced => 9,
        }
    }
    /// Whether this style casts and receives shadows.
    pub fn shadows(self) -> bool {
        self == Style::Advanced
    }
}

/// The wire drawn over a group's fill.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Overlay {
    None,
    /// Light lines, the same for every mesh.
    Uniform,
    /// Dark lines (the theme's Dark Lines Overlay, black by default). A new
    /// view's default (VS-6).
    #[default]
    Colored,
}
/// One group's drawing: active or inactive meshes.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub style: Style,
    pub overlay: Overlay,
    /// Overlay opacity, 0–100.
    pub opacity: u8,
    pub smoothing: bool,
    /// This view's plain wire colour (RGB) for the group's meshes. `None`:
    /// the theme's Inactive Wireframe for the inactive group, the theme's
    /// Wireframe for the active one (plain_wire_color). Set by Viewport
    /// Properties' Wireframe Color rows; older saved settings
    /// and presets without it still load.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wire_color: Option<[u8; 3]>,
    /// Transparency Overrides (VP-80): see-through polygons for this group.
    #[serde(default, skip_serializing_if = "Transparency::is_normal")]
    pub transparency: Transparency,
}

/// How a group's polygons are see-through (Viewport Properties > Active /
/// Inactive Meshes > Transparency Overrides, VP-80).
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum TransparencyMode {
    /// Solid, as usual.
    #[default]
    Normal,
    /// Polygons facing the eye are see-through, tinted with the front
    /// colour; those facing away are drawn with the back colour.
    Ghost,
    /// Polygons facing the eye are clear; only those facing away show.
    XRay,
}
/// A group's Transparency Overrides. Colours are sRGB; opacities percent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Transparency {
    pub mode: TransparencyMode,
    pub front_color: [u8; 3],
    pub front_opacity: u8,
    pub back_color: [u8; 3],
    pub back_opacity: u8,
    /// Overlay: the see-through polygons draw over everything else in the
    /// view (the Active Meshes tab only).
    pub overlay: bool,
    /// Flat Shaded: no lighting on the see-through polygons.
    pub flat: bool,
}
impl Default for Transparency {
    fn default() -> Self {
        // Provisional values (VP-Q09): a light, faint front and a mid-grey
        // back.
        Self {
            mode: TransparencyMode::Normal,
            front_color: [200, 215, 235],
            front_opacity: 25,
            back_color: [140, 140, 140],
            back_opacity: 60,
            overlay: false,
            flat: false,
        }
    }
}
impl TransparencyMode {
    /// The header buttons' order (XR-1).
    pub const ALL: [TransparencyMode; 3] = [
        TransparencyMode::Normal,
        TransparencyMode::Ghost,
        TransparencyMode::XRay,
    ];
    /// The name Viewport Properties and `view3d.TransparencyOverride` use.
    pub fn label(self) -> &'static str {
        match self {
            TransparencyMode::Normal => "Normal",
            TransparencyMode::Ghost => "Ghost",
            TransparencyMode::XRay => "X-Ray",
        }
    }
}
impl Transparency {
    pub fn is_normal(&self) -> bool {
        *self == Self::default()
    }
    /// Whether the group draws see-through.
    pub fn on(&self) -> bool {
        self.mode != TransparencyMode::Normal
    }
}
impl Group {
    /// Whether the group's meshes draw their plain wire (the overlay is on
    /// and not fully faded). Selection, rollover and crease strokes draw
    /// either way.
    pub fn draws_wire(&self) -> bool {
        self.overlay != Overlay::None && self.opacity > 0
    }
}
impl Default for Group {
    fn default() -> Self {
        Self {
            style: Style::default(),
            // VS-6: a new view's wire is Colored (black).
            overlay: Overlay::Colored,
            // Viewport Properties: Wireframe Opacity starts at 70 % (views saved before
            // keep the value they were saved with).
            opacity: 70,
            smoothing: true,
            wire_color: None,
            transparency: Transparency::default(),
        }
    }
}

/// When points draw while the Show Verts toggle is off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShowVertices {
    Always,
    InComponentModes,
    InVertexMode,
    /// Only selected and pre-highlighted points (and points no polygon
    /// uses). Operandi's default (Jason, 2026-11-03).
    #[default]
    Never,
}
impl ShowVertices {
    pub const ALL: [ShowVertices; 4] = [
        ShowVertices::Always,
        ShowVertices::InComponentModes,
        ShowVertices::InVertexMode,
        ShowVertices::Never,
    ];
    pub fn label(self) -> &'static str {
        match self {
            ShowVertices::Always => "Always",
            ShowVertices::InComponentModes => "In Component Modes",
            ShowVertices::InVertexMode => "In Vertex Mode",
            ShowVertices::Never => "Never",
        }
    }
}

/// G8 RP-3: how Replicators' copies draw in a view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicatorDisplay {
    /// Every copy as a wire box of its prototype (the default).
    #[default]
    None,
    /// The selected Replicator's copies shaded, the others as boxes.
    Selected,
    /// Every copy shaded.
    All,
}
impl ReplicatorDisplay {
    pub const ALL: [ReplicatorDisplay; 3] = [
        ReplicatorDisplay::None,
        ReplicatorDisplay::Selected,
        ReplicatorDisplay::All,
    ];
    pub fn label(self) -> &'static str {
        match self {
            ReplicatorDisplay::None => "None",
            ReplicatorDisplay::Selected => "Selected",
            ReplicatorDisplay::All => "All",
        }
    }
}

/// The view's quick toggles and overrides.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Drawing {
    pub grid: bool,
    pub item_mode_wireframe: bool,
    /// Show Verts: on draws every point in every mode; off follows
    /// `show_vertices`.
    pub show_verts: bool,
    pub show_vertices: ShowVertices,
    pub uv_seams: bool,
    /// UVT-114 Checker: fills show a numbered grey and white check through
    /// the selected UV map (an approved Operandi extra, Jason 2026-11-16;
    /// uv_checker.rs). Off by default.
    #[serde(default)]
    pub checker: bool,
    /// HT-5 Show Weightmaps (`view3d.showweights`): meshes with the
    /// selected weight map tinted by its value (VMT-30 colours) over their
    /// shading. Off by default.
    #[serde(default)]
    pub weights: bool,
    /// HT-6 Show UV Distortion (`view3d.showDistortion`): polygons tinted
    /// by UV stretch in the mesh's current UV map, the UV view's colours
    /// (UVV-03). Off by default.
    #[serde(default)]
    pub distortion: bool,
    /// MT7 Topology Mode (`view.topologyMode`, missing-tools-g3): the
    /// active meshes draw over the background meshes, see-through and
    /// flat, in the Topo colours (topology_mode.rs). Off by default.
    #[serde(default)]
    pub topology: bool,
    /// Off: every polygon faceted, whatever its group says.
    pub smoothing: bool,
    pub inactive_same_as_active: bool,
    pub inactive_invisible: bool,
    /// BD-13 Show Backdrop: off hides every Backdrop in this view.
    #[serde(default = "shown")]
    pub backdrops: bool,
    /// Show Symmetry Plane: with Symmetry on, the plane's outline across
    /// the active mesh (symmetry section 8's seam guide). Off hides only
    /// the outline; Symmetry stays on (Jason, 2026-11-14).
    #[serde(default = "shown")]
    pub symmetry_plane: bool,
    /// CL-1 Show Cameras: off (the default) hides every camera's shape in
    /// this view, selected or not.
    #[serde(default = "show_cameras")]
    pub cameras: bool,
    /// CL-1 Show Lights: off (the default) hides every light's shape in
    /// this view, selected or not.
    #[serde(default = "show_lights")]
    pub lights: bool,
    /// GL-11 Show Locators: off hides every Group Locator's shape in this
    /// view (on by default). A later Viewport Properties Visibility tab
    /// takes this field over.
    #[serde(default = "shown")]
    pub locators: bool,
    /// G8 RP-3 Replicators: which copies draw shaded rather than as boxes.
    #[serde(default)]
    pub replicators: ReplicatorDisplay,
    /// Advanced: Show Shadows (on by default). Off draws Advanced as
    /// Default.
    #[serde(default = "shown")]
    pub shadows: bool,
    /// Show Viewport Controls and Information (Viewport Properties): off
    /// hides the header's corner buttons and the view's information text.
    /// O still opens Viewport Properties.
    #[serde(default = "shown")]
    pub controls: bool,
    /// Show View Information: the two lines of view and selection
    /// information at the view's top left. Off by default (Jason,
    /// 2026-11-06); settings saved before it existed load with it off.
    #[serde(default)]
    pub info: bool,
    /// The view's background: flat, or a gradient.
    #[serde(default)]
    pub background: Background,
    /// Show Meshes: off hides every mesh item in this view.
    #[serde(default = "shown")]
    pub meshes: bool,
    /// Show Selection Rollovers: off turns pre-highlighting off in this
    /// view (Preferences > Selection still decides where it is on).
    #[serde(default = "shown")]
    pub rollovers: bool,
    /// This view's Trackball Rotation: None follows Preferences > Input >
    /// Remapping, which can also override every view's choice.
    #[serde(default)]
    pub trackball: Option<bool>,
    /// Show Grid's Inactive Work Plane choice (VP spec v2.1 section 8): with
    /// `grid` on, the floor grid shows only while the work plane is at rest,
    /// so only one grid is drawn. Off is Show Grid: Always.
    #[serde(default = "shown")]
    pub grid_follows_work_plane: bool,
    /// Show Work Plane: when this view draws the work plane's grid.
    #[serde(default)]
    pub work_plane: WorkPlaneShow,
    /// Draw Faded Grid: the grids fade out with distance.
    #[serde(default = "shown")]
    pub faded_grid: bool,
    /// Faded Grid Color Axes: the floor's main axis lines in the axis
    /// colours (with View > Ground Axes on).
    #[serde(default = "shown")]
    pub grid_axes: bool,
    /// Show Texture Locators: every texture locator's gizmo draws, not only
    /// the selected one and those of the textures picked in the Shader Tree.
    #[serde(default)]
    pub texture_locators: bool,
    /// Show Scene Lighting: the scene's lights light this view (the
    /// Advanced style); off, the view's own light does.
    #[serde(default)]
    pub scene_lighting: bool,
    /// Show Selections: off hides every selection highlight in this view.
    #[serde(default = "shown")]
    pub selections: bool,
    /// Show Selection Normals: a dashed line out of each selected polygon.
    #[serde(default)]
    pub selection_normals: bool,
    /// Show Selected Outline: the selected polygons' outline.
    #[serde(default = "shown")]
    pub selected_outline: bool,
    /// Show Selected Filling: the tint on selected polygons.
    #[serde(default = "shown")]
    pub selected_filling: bool,
    /// Orbit Selected (Drawing and Control > Mouse Control): orbit turns the
    /// view around the selection's centre.
    #[serde(default)]
    pub orbit_selected: bool,
    /// Advanced Options > Ambient Intensity, in percent: how much light the
    /// Advanced style's shadowed sides get. 44 puts the default grey's
    /// unlit side at 90 (VS-3).
    #[serde(default = "ambient_intensity")]
    pub ambient_intensity: u8,
    /// Advanced Options > Ambient Color (sRGB).
    #[serde(default = "white")]
    pub ambient_color: [u8; 3],
    /// Active Meshes > Independent Point Size: this view draws points at
    /// its own size instead of Preferences > Selection's.
    #[serde(default)]
    pub independent_point_size: bool,
    /// Active Meshes > Point Size, in screen points (with Independent Point
    /// Size on).
    #[serde(default = "point_size")]
    pub point_size: f32,
    /// Advanced Options > Matcap: the Matcap style's matcap, built in
    /// or an image file (matcap.rs; an Operandi extra).
    #[serde(default)]
    pub matcap: crate::matcap::Choice,
}
fn ambient_intensity() -> u8 {
    44
}
fn white() -> [u8; 3] {
    [255; 3]
}
fn point_size() -> f32 {
    3.
}

/// When a view draws the work plane's grid (Viewport Properties >
/// Visibility > Show Work Plane).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkPlaneShow {
    Always,
    /// Only once the work plane has been moved or rotated away from its
    /// resting place (provisional: fixed, or centred off the origin).
    #[default]
    Active,
    Never,
}
impl Drawing {
    /// Whether the floor grid draws, given whether the work plane is moved.
    pub fn floor_grid(&self, work_plane_moved: bool) -> bool {
        self.grid && !(self.grid_follows_work_plane && work_plane_moved)
    }
    /// Whether the work plane's grid draws, given whether it is moved.
    pub fn work_plane_grid(&self, work_plane_moved: bool) -> bool {
        match self.work_plane {
            WorkPlaneShow::Always => true,
            WorkPlaneShow::Active => work_plane_moved,
            WorkPlaneShow::Never => false,
        }
    }
    /// The Advanced style's ambient light: intensity × colour, 0–1 linear
    /// per channel (the shader works in the same space as the colours).
    pub fn ambient(&self) -> [f32; 3] {
        let k = f32::from(self.ambient_intensity.min(200)) / 100.;
        self.ambient_color.map(|c| f32::from(c) / 255. * k)
    }
    /// Pre-highlighting under the pointer (Show Selections and Show
    /// Selection Rollovers).
    pub fn shows_rollovers(&self) -> bool {
        self.selections && self.rollovers
    }
}

/// A view's background (Viewport Properties > Drawing and Control).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Background {
    /// Flat: the Background colour.
    #[default]
    None,
    /// From Gradient Top at the top to the Background colour at the bottom.
    Gradient,
    /// The scene's environment as the camera sees it (environments
    /// 2026-11-09 §9).
    Environment,
}
fn shown() -> bool {
    true
}
fn show_cameras() -> bool {
    operandi_core::camera_light_defaults::SHOW_CAMERAS
}
fn show_lights() -> bool {
    operandi_core::camera_light_defaults::SHOW_LIGHTS
}
impl Default for Drawing {
    fn default() -> Self {
        Self {
            grid: true,
            // VS-6: Show Wireframe in Item Mode starts on.
            item_mode_wireframe: true,
            show_verts: false,
            show_vertices: ShowVertices::default(),
            uv_seams: false,
            checker: false,
            weights: false,
            distortion: false,
            topology: false,
            smoothing: true,
            inactive_same_as_active: false,
            inactive_invisible: false,
            backdrops: true,
            symmetry_plane: true,
            cameras: show_cameras(),
            lights: show_lights(),
            locators: true,
            replicators: ReplicatorDisplay::None,
            shadows: true,
            controls: true,
            info: false,
            background: Background::None,
            meshes: true,
            rollovers: true,
            trackball: None,
            grid_follows_work_plane: true,
            work_plane: WorkPlaneShow::default(),
            faded_grid: true,
            grid_axes: true,
            texture_locators: false,
            scene_lighting: false,
            selections: true,
            selection_normals: false,
            selected_outline: true,
            selected_filling: true,
            orbit_selected: false,
            ambient_intensity: ambient_intensity(),
            ambient_color: white(),
            independent_point_size: false,
            point_size: point_size(),
            matcap: crate::matcap::Choice::default(),
        }
    }
}

/// Everything a view's Display menu sets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub drawing: Drawing,
    pub active: Group,
    pub inactive: Group,
}

/// A saved preset: the groups it was saved with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    pub name: String,
    pub drawing: Option<Drawing>,
    pub active: Option<Group>,
    pub inactive: Option<Group>,
}
impl Preset {
    /// `settings` with this preset's groups applied.
    fn apply_to(&self, mut settings: Settings) -> Settings {
        if let Some(d) = self.drawing {
            settings.drawing = d;
        }
        if let Some(g) = self.active {
            settings.active = g;
        }
        if let Some(g) = self.inactive {
            settings.inactive = g;
        }
        settings
    }
    /// Whether `settings` still match what this preset sets.
    fn matches(&self, settings: &Settings) -> bool {
        self.apply_to(*settings) == *settings
    }
}

/// The presets that ship with Operandi; they can't be deleted.
pub fn built_in() -> Vec<Preset> {
    let group = |style, overlay| Group {
        style,
        overlay,
        ..Group::default()
    };
    let both = |name: &str, g: Group| Preset {
        name: name.into(),
        drawing: None,
        active: Some(g),
        inactive: Some(g),
    };
    vec![
        both("Shaded Wire", group(Style::Shaded, Overlay::Uniform)),
        both("Solid Wire", group(Style::Solid, Overlay::Colored)),
        both("Wireframe", group(Style::Wireframe, Overlay::Uniform)),
    ]
}

/// A preset applied to a view, and the settings it replaced.
#[derive(Clone, Debug, PartialEq)]
pub struct Applied {
    pub name: String,
    pub before: Settings,
}

/// One view's display state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ViewDisplay {
    pub settings: Settings,
    pub applied: Option<Applied>,
}
impl ViewDisplay {
    /// The view header's Display button text: the active meshes' style.
    pub fn menu_label(&self) -> &'static str {
        self.settings.active.style.short_label()
    }
    /// The group an item draws with: active for the selected item.
    /// `None`: the item is hidden in this view.
    pub fn group(&self, active: bool) -> Option<Group> {
        let s = &self.settings;
        if active {
            Some(s.active)
        } else if s.drawing.inactive_invisible {
            None
        } else if s.drawing.inactive_same_as_active {
            Some(s.active)
        } else {
            Some(s.inactive)
        }
    }
    /// The colour the active (or inactive) meshes' plain wire takes in this
    /// view instead of the theme's Wireframe: the drawn group's own
    /// `wire_color` if set; otherwise the theme's Inactive Wireframe for
    /// inactive meshes (unless Inactive Same as Active makes them match
    /// the active look); otherwise `None`, Wireframe as baked.
    pub fn plain_wire_color(&self, active: bool) -> Option<eframe::egui::Color32> {
        let group = self.group(active).unwrap_or(self.settings.inactive);
        group
            .wire_color
            .map(|[r, g, b]| eframe::egui::Color32::from_rgb(r, g, b))
            .or_else(|| {
                (!active && !self.settings.drawing.inactive_same_as_active)
                    .then(crate::theme::viewport::inactive_wireframe)
            })
            // VS-1: over a filled style the plain wire is the theme's
            // Shaded Wireframe (black); Wireframe keeps its own light wire,
            // and Vertex Map (not in VS-1's list) keeps it too.
            .or_else(|| {
                (!matches!(group.style, Style::Wireframe | Style::VertexMap))
                    .then(crate::theme::viewport::shaded_wireframe)
            })
    }

    /// What actually draws for a group in selection mode `mode`: the Show
    /// Wireframe in Item Mode toggle and the view's Smoothing switch
    /// override the group's own overlay and smoothing.
    pub fn effective(&self, group: Group, mode: Mode) -> Group {
        let d = &self.settings.drawing;
        let mut g = group;
        // Show Wireframe in Item Mode decides Item mode's plain wire
        // (VS-6): on, every shown mesh draws it at full opacity in the
        // group's overlay (None turns it on as the default Colored); off,
        // none does, whatever the overlay, and the group's Wireframe Color
        // goes with it. Selection highlights aren't plain wire and still
        // draw. Component modes keep the group's overlay; Wireframe style
        // is its own wire.
        if mode == Mode::Item && g.style != Style::Wireframe {
            if d.item_mode_wireframe {
                if g.overlay == Overlay::None {
                    g.overlay = Overlay::default();
                }
                g.opacity = 100;
            } else {
                g.overlay = Overlay::None;
            }
        }
        if g.style == Style::Wireframe {
            // Wireframe is its own wire; it has no separate overlay, and
            // nothing to see through (VP-80).
            g.overlay = Overlay::Uniform;
            g.opacity = 100;
            g.transparency.mode = TransparencyMode::Normal;
        }
        g.smoothing &= d.smoothing;
        g
    }

    /// Whether every point of the active item draws in `mode` (selected,
    /// hovered and loose points always do).
    pub fn draws_points(&self, mode: Mode) -> bool {
        let d = &self.settings.drawing;
        if d.show_verts {
            return true;
        }
        match d.show_vertices {
            ShowVertices::Always => true,
            ShowVertices::InComponentModes => mode != Mode::Item,
            ShowVertices::InVertexMode => mode == Mode::Vertex,
            ShowVertices::Never => false,
        }
    }

    /// Set a style: Alt-click sets the inactive group's.
    pub fn set_style(&mut self, style: Style, inactive: bool) {
        if inactive {
            self.settings.inactive.style = style;
        } else {
            self.settings.active.style = style;
        }
    }

    /// Choose a preset: apply it, or, when it is the one already applied,
    /// take it off and restore the settings from before it.
    pub fn choose(&mut self, preset: &Preset) {
        if let Some(applied) = self.applied.take() {
            if applied.name == preset.name {
                self.settings = applied.before;
                return;
            }
            // Switching presets: the restore point stays the settings from
            // before the first one.
            self.applied = Some(applied);
        }
        let before = self.applied.as_ref().map_or(self.settings, |a| a.before);
        self.settings = preset.apply_to(self.settings);
        self.applied = Some(Applied {
            name: preset.name.clone(),
            before,
        });
    }

    /// Whether the applied preset's settings have changed since (the
    /// menu's asterisk). False when no preset is applied or it is gone.
    pub fn modified(&self, presets: &[Preset]) -> bool {
        self.applied.as_ref().is_some_and(|a| {
            presets
                .iter()
                .find(|p| p.name == a.name)
                .is_some_and(|p| !p.matches(&self.settings))
        })
    }

    /// The name the menu checks.
    pub fn applied_name(&self) -> Option<&str> {
        self.applied.as_ref().map(|a| a.name.as_str())
    }
}

/// Which groups Save Viewport Preset includes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Include {
    pub drawing: bool,
    pub active: bool,
    pub inactive: bool,
}

/// The user's presets, kept with the preferences.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Presets {
    version: u8,
    pub user: Vec<Preset>,
}
impl Presets {
    /// Built-ins first, then the user's (a user preset with a built-in's
    /// name replaces it).
    pub fn all(&self) -> Vec<Preset> {
        let mut out: Vec<Preset> = built_in()
            .into_iter()
            .filter(|b| self.user.iter().all(|u| u.name != b.name))
            .collect();
        out.extend(self.user.iter().cloned());
        out
    }

    /// Save `settings` as preset `name` (replacing a user preset of that
    /// name). Errors on an empty name or no groups.
    pub fn save(
        &mut self,
        name: &str,
        settings: &Settings,
        include: Include,
    ) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("A preset needs a name".into());
        }
        if !(include.drawing || include.active || include.inactive) {
            return Err("Choose at least one group to save".into());
        }
        let preset = Preset {
            name: name.into(),
            drawing: include.drawing.then_some(settings.drawing),
            active: include.active.then_some(settings.active),
            inactive: include.inactive.then_some(settings.inactive),
        };
        match self.user.iter_mut().find(|p| p.name == name) {
            Some(existing) => *existing = preset,
            None => self.user.push(preset),
        }
        Ok(())
    }

    /// Delete a user preset. Built-ins can't be deleted.
    pub fn delete(&mut self, name: &str) -> bool {
        let before = self.user.len();
        self.user.retain(|p| p.name != name);
        self.user.len() != before
    }

    pub fn load(path: &Path) -> Self {
        fs::metadata(path)
            .ok()
            .filter(|m| m.len() <= 1 << 20)
            .and_then(|_| fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .filter(|p| p.version == 1)
            .unwrap_or(Self {
                version: 1,
                user: Vec::new(),
            })
    }

    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        let file = Self {
            version: 1,
            user: self.user.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&file).map_err(io::Error::other)?;
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        super::resource_write::write_bytes(path, &temp, &bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_label_names_the_active_style() {
        let mut d = ViewDisplay::default();
        assert_eq!(d.menu_label(), "Default");
        d.set_style(Style::Wireframe, false);
        assert_eq!(d.menu_label(), "Wireframe");
        // Alt-click changes only the inactive meshes; the label stays.
        d.set_style(Style::Gooch, true);
        assert_eq!(d.menu_label(), "Wireframe");
        d.set_style(Style::VertexMap, false);
        assert_eq!(d.menu_label(), "Vertex Map");
        // The button keeps the short names; the menu has the long ones.
        d.set_style(Style::Gooch, false);
        assert_eq!(d.menu_label(), "Gooch");
        assert_eq!(Style::Gooch.label(), "Gooch Tone Shading");
        d.set_style(Style::Cel, false);
        assert_eq!(d.menu_label(), "Cel");
        assert_eq!(Style::Cel.label(), "Cel Shading");
    }

    /// DM-T1: the menu lists exactly these ten, in this order.
    #[test]
    fn menu_order() {
        let labels: Vec<_> = Style::ALL.iter().map(|s| s.label()).collect();
        assert_eq!(
            labels,
            [
                "Wireframe",
                "Solid",
                "Vertex Map",
                "Shaded",
                "Texture",
                "Default",
                "Advanced",
                "Gooch Tone Shading",
                "Cel Shading",
                "Matcap",
            ]
        );
    }

    /// Reordering the menu doesn't renumber the shader's styles, and saved
    /// styles load by name.
    #[test]
    fn style_codes_and_saved_names_are_stable() {
        let codes: Vec<_> = [
            Style::Wireframe,
            Style::Solid,
            Style::Shaded,
            Style::VertexMap,
            Style::Gooch,
            Style::Cel,
            Style::Reflection,
            Style::Texture,
            Style::Default,
            Style::Advanced,
        ]
        .iter()
        .map(|s| s.code())
        .collect();
        assert_eq!(codes, (0..10).collect::<Vec<u32>>());
        for (name, style) in [
            ("\"Shaded\"", Style::Shaded),
            ("\"Gooch\"", Style::Gooch),
            ("\"VertexMap\"", Style::VertexMap),
        ] {
            assert_eq!(serde_json::from_str::<Style>(name).unwrap(), style);
        }
        // A preset saved before Show Shadows existed still loads, with
        // shadows on.
        let mut old = serde_json::to_value(Drawing::default()).unwrap();
        old.as_object_mut().unwrap().remove("shadows");
        let back: Drawing = serde_json::from_value(old).unwrap();
        assert!(back.shadows);
    }

    #[test]
    fn defaults_match_the_spec() {
        // DM-T8: a new view draws in Default, with Show Shadows on.
        let d = ViewDisplay::default();
        assert_eq!(d.settings.active.style, Style::Default);
        assert_eq!(d.settings.inactive.style, Style::Default);
        assert!(d.settings.drawing.shadows);
        // VS-6: Colored wire, Show Wireframe in Item Mode on.
        assert_eq!(d.settings.active.overlay, Overlay::Colored);
        assert_eq!(d.settings.inactive.overlay, Overlay::Colored);
        assert!(d.settings.drawing.grid && d.settings.drawing.smoothing);
        assert!(!d.settings.drawing.show_verts && !d.settings.drawing.uv_seams);
        assert!(d.settings.drawing.item_mode_wireframe);
    }

    #[test]
    fn alt_sets_the_inactive_style() {
        let mut d = ViewDisplay::default();
        d.set_style(Style::Gooch, false);
        d.set_style(Style::Wireframe, true);
        assert_eq!(d.group(true).unwrap().style, Style::Gooch);
        assert_eq!(d.group(false).unwrap().style, Style::Wireframe);
    }

    /// Groups saved before the per-view inactive wire colour load with
    /// `None` (follow the theme); `None` isn't written, a set colour is.
    #[test]
    fn groups_saved_without_the_wire_colour_still_load() {
        let old = r#"{"style":"Shaded","overlay":"Uniform","opacity":80,"smoothing":true}"#;
        let g: Group = serde_json::from_str(old).unwrap();
        assert_eq!(g.wire_color, None);
        assert_eq!(g.style, Style::Shaded);
        assert!(!serde_json::to_string(&g).unwrap().contains("wire_color"));
        let set = Group {
            wire_color: Some([1, 2, 3]),
            ..g
        };
        let text = serde_json::to_string(&set).unwrap();
        assert_eq!(serde_json::from_str::<Group>(&text).unwrap(), set);
        // A whole preset from before the field.
        let preset = r#"{"name":"Old","drawing":null,"active":null,
            "inactive":{"style":"Wireframe","overlay":"Uniform","opacity":100,"smoothing":false}}"#;
        let p: Preset = serde_json::from_str(preset).unwrap();
        assert_eq!(p.inactive.unwrap().wire_color, None);
    }

    #[test]
    fn inactive_overrides() {
        let mut d = ViewDisplay::default();
        d.set_style(Style::Cel, false);
        d.settings.drawing.inactive_same_as_active = true;
        assert_eq!(d.group(false).unwrap().style, Style::Cel);
        d.settings.drawing.inactive_invisible = true;
        assert_eq!(d.group(false), None);
        assert!(d.group(true).is_some());
    }

    #[test]
    fn item_mode_wireframe_overrides_the_overlay_only_in_item_mode() {
        let mut d = ViewDisplay::default();
        d.settings.active.overlay = Overlay::None;
        d.settings.drawing.item_mode_wireframe = true;
        let g = d.settings.active;
        assert_eq!(d.effective(g, Mode::Item).overlay, Overlay::Colored);
        assert_eq!(d.effective(g, Mode::Polygon).overlay, Overlay::None);
        d.settings.drawing.item_mode_wireframe = false;
        assert_eq!(d.effective(g, Mode::Item).overlay, Overlay::None);
        // On keeps a group's own overlay choice.
        let uniform = Group {
            overlay: Overlay::Uniform,
            ..g
        };
        d.settings.drawing.item_mode_wireframe = true;
        assert_eq!(d.effective(uniform, Mode::Item).overlay, Overlay::Uniform);
    }

    /// VS-6: off hides Item mode's plain wire though the overlay is still
    /// Colored, a Wireframe Color override included; component modes keep
    /// it, and Wireframe style is its own wire.
    #[test]
    fn item_mode_wireframe_off_hides_the_wire_in_item_mode() {
        let mut d = ViewDisplay::default();
        d.settings.drawing.item_mode_wireframe = false;
        d.settings.active.wire_color = Some([200, 40, 40]);
        let g = d.settings.active;
        assert_eq!(g.overlay, Overlay::Colored);
        assert_eq!(d.effective(g, Mode::Item).overlay, Overlay::None);
        for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon] {
            assert_eq!(d.effective(g, mode).overlay, Overlay::Colored, "{mode:?}");
        }
        let inactive = d.settings.inactive;
        assert_eq!(d.effective(inactive, Mode::Item).overlay, Overlay::None);
        let wire = Group {
            style: Style::Wireframe,
            ..g
        };
        assert_eq!(d.effective(wire, Mode::Item).overlay, Overlay::Uniform);
    }

    #[test]
    fn smoothing_switch_turns_every_group_faceted() {
        let mut d = ViewDisplay::default();
        let g = d.settings.active;
        assert!(d.effective(g, Mode::Polygon).smoothing);
        d.settings.drawing.smoothing = false;
        assert!(!d.effective(g, Mode::Polygon).smoothing);
    }

    #[test]
    fn show_verts_mode_resolution() {
        let mut d = ViewDisplay::default();
        let modes = [Mode::Item, Mode::Vertex, Mode::Edge, Mode::Polygon];
        let draws = |d: &ViewDisplay| modes.map(|m| d.draws_points(m));
        assert_eq!(draws(&d), [false; 4]);
        d.settings.drawing.show_vertices = ShowVertices::InVertexMode;
        assert_eq!(draws(&d), [false, true, false, false]);
        d.settings.drawing.show_vertices = ShowVertices::InComponentModes;
        assert_eq!(draws(&d), [false, true, true, true]);
        d.settings.drawing.show_vertices = ShowVertices::Always;
        assert_eq!(draws(&d), [true; 4]);
        // The toggle draws every point whatever the mode setting.
        d.settings.drawing.show_vertices = ShowVertices::Never;
        d.settings.drawing.show_verts = true;
        assert_eq!(draws(&d), [true; 4]);
    }

    #[test]
    fn preset_apply_modified_revert() {
        let presets = built_in();
        let solid = presets.iter().find(|p| p.name == "Solid Wire").unwrap();
        let wire = presets.iter().find(|p| p.name == "Wireframe").unwrap();
        let mut d = ViewDisplay::default();
        d.settings.active.style = Style::Gooch;
        let original = d.settings;
        d.choose(solid);
        assert_eq!(d.applied_name(), Some("Solid Wire"));
        assert_eq!(d.settings.active.style, Style::Solid);
        assert!(!d.modified(&presets));
        d.settings.active.opacity = 50;
        assert!(d.modified(&presets));
        // A setting the preset doesn't hold isn't a modification.
        d.settings.active.opacity = Group::default().opacity;
        d.settings.drawing.grid = false;
        assert!(!d.modified(&presets));
        // Switching keeps the first restore point.
        d.choose(wire);
        assert_eq!(d.applied_name(), Some("Wireframe"));
        d.choose(wire);
        assert_eq!(d.applied, None);
        assert_eq!(d.settings, original);
    }

    #[test]
    fn save_and_delete_presets() {
        let mut presets = Presets::default();
        let mut s = Settings::default();
        s.active.style = Style::Reflection;
        let none = Include {
            drawing: false,
            active: false,
            inactive: false,
        };
        assert!(presets.save("x", &s, none).is_err());
        let active_only = Include {
            active: true,
            ..none
        };
        assert!(presets.save("  ", &s, active_only).is_err());
        presets.save("Chrome", &s, active_only).unwrap();
        let chrome = presets
            .all()
            .into_iter()
            .find(|p| p.name == "Chrome")
            .unwrap();
        assert_eq!(chrome.active.unwrap().style, Style::Reflection);
        assert_eq!((chrome.drawing, chrome.inactive), (None, None));
        // Same name replaces.
        s.active.style = Style::Cel;
        presets.save("Chrome", &s, active_only).unwrap();
        assert_eq!(presets.user.len(), 1);
        assert_eq!(presets.user[0].active.unwrap().style, Style::Cel);
        assert!(!presets.delete("Wireframe"), "built-ins stay");
        assert!(presets.delete("Chrome"));
        assert!(presets.user.is_empty());
        assert_eq!(presets.all().len(), built_in().len());
    }

    #[test]
    fn presets_persist() {
        let root = std::env::temp_dir().join(format!(
            "operandi-view-presets-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let path = root.join("viewport-presets.json");
        assert!(Presets::load(&path).user.is_empty());
        let mut presets = Presets::load(&path);
        let mut s = Settings::default();
        s.drawing.show_vertices = ShowVertices::InComponentModes;
        s.inactive.overlay = Overlay::Colored;
        let all = Include {
            drawing: true,
            active: true,
            inactive: true,
        };
        presets.save("Mine", &s, all).unwrap();
        presets.save_to(&path).unwrap();
        let back = Presets::load(&path);
        assert_eq!(back.user, presets.user);
        fs::write(&path, b"not json").unwrap();
        assert!(Presets::load(&path).user.is_empty());
        let _ = fs::remove_dir_all(root);
    }
}
