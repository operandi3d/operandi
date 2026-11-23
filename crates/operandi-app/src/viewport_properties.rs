//! The Viewport Properties panel (VP-10..46, BEHAVIOR-VIEWPORT-BUTTONS,
//! layout from LAYOUT-PROPERTIES-PANEL v1.2): O over a view (near the
//! pointer), the header gear (hanging under it) or View > Viewport
//! Properties opens it for that view. Unpinned it is a popover that closes
//! when the pointer leaves it; the circle in its title bar pins it as a
//! floating panel that stays, can be dragged and comes back after a
//! relaunch (VP-10..19, UIC-62/64).
//!
//! Its tabs, sections and rows are declared for the shared form component
//! in panel mode (form::panel, UIC-100..126): labels right-aligned on one
//! edge, checkboxes with their name beside the box, every choice a
//! dropdown, greyed rows kept in place. These are view settings, not
//! channels, so rows have no status light (VP-40..46).
//!
//! Every row edits the target view's own settings live (`display`, and
//! `camera` for the view type, VP-20..24). A row that also appears
//! elsewhere (the header's shading selector and Display menu) is the same
//! setting, never a copy. Nothing here is an undo step, and the settings
//! save with the view layout. Settings Operandi doesn't have yet are left
//! out, not greyed.
use super::*;
use crate::form::{Kind, Row, Unit, Value, Values, When, panel};
use crate::unavailable::Gate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use view_display::{
    Background, Overlay, ReplicatorDisplay, ShowVertices, Style, TransparencyMode, WorkPlaneShow,
};

/// The panel's tabs, top to bottom on its right edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tab {
    #[default]
    Drawing,
    Visibility,
    Advanced,
    Active,
    Inactive,
}
impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Drawing,
        Tab::Visibility,
        Tab::Advanced,
        Tab::Active,
        Tab::Inactive,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Tab::Drawing => "Drawing and Control",
            Tab::Visibility => "Visibility",
            Tab::Advanced => "Advanced Options",
            Tab::Active => "Active Meshes",
            Tab::Inactive => "Inactive Meshes",
        }
    }
    fn index(self) -> usize {
        Tab::ALL.iter().position(|&t| t == self).unwrap_or(0)
    }
}

/// The panel: which view it is open for (its target), and how it shows.
#[derive(Default)]
pub struct Popover {
    /// The target view (VP-10..19): the panel edits it until it closes or O
    /// over another view retargets it.
    pub view: Option<u32>,
    pub tab: Tab,
    /// Pinned: a floating panel that stays until its circle is clicked
    /// (VP-10..19).
    pub pinned: bool,
    /// The panel's top-left on screen. `None`: it hangs under the target
    /// view's gear (opened from the gear or the View menu).
    pub at: Option<Pos2>,
    /// Folded sections ("tab/section"), kept for the user across
    /// openings and launches.
    pub collapsed: BTreeSet<String>,
    /// Where each row, section and tab drew this frame, by key (tests and
    /// the layout check).
    pub rows: BTreeMap<String, Rect>,
    /// One Tab / All Tabs presets and the new-view default (UIC-103).
    pub presets: view_tab_presets::TabPresets,
    /// A preset name being typed in a header menu.
    pub naming: String,
    /// All Tabs ▾'s find field.
    pub find: String,
    /// The tabs ticked for "(new, selected tabs …)".
    pub chosen_tabs: Vec<Tab>,
    /// The pointer was over the panel (or within the roll-off margin)
    /// last frame: the panel owns the keys then.
    pub hovered: bool,
    /// The pointer has been over the panel since it opened: rolling off
    /// closes it only after that (a panel opened from the menu bar starts
    /// away from the pointer).
    entered: bool,
    /// The state last written to disk, so it is saved only on a change.
    saved: Option<SavedState>,
}
impl Popover {
    /// Whether the panel takes the keyboard this frame: while the pointer
    /// is over it (a pinned panel away from the pointer leaves the keys to
    /// the views, so O can retarget it).
    pub fn owns_input(&self) -> bool {
        self.view.is_some() && self.hovered
    }
}

/// What `viewport-properties.json` keeps: folded sections, the last tab,
/// and a pinned panel's view and place (VP-10..19). Older files held only the
/// folded sections and still load.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct SavedState {
    collapsed: BTreeSet<String>,
    tab: Tab,
    pinned: Option<Pinned>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Pinned {
    view: u32,
    at: [f32; 2],
}
impl SavedState {
    fn parse(text: &str) -> Option<Self> {
        serde_json::from_str(text).ok().or_else(|| {
            serde_json::from_str::<BTreeSet<String>>(text)
                .ok()
                .map(|collapsed| Self {
                    collapsed,
                    ..Self::default()
                })
        })
    }
}

/// Input-map action: open the panel for the view under the pointer.
pub const ACTION: &str = "Viewport Properties";

/// The panel's width (points; about 400, VP-40..46).
const POPOVER_WIDTH: f32 = 400.;
/// VP-10..19: an unpinned panel closes once the pointer is this far outside.
const ROLL_OFF: f32 = 8.;
/// Opened with O: the panel's top-left sits this far up and left of the
/// pointer, so the pointer starts inside it (VP-10..19).
const NEAR_POINTER: Vec2 = vec2(14., 12.);

/// Row keys (also what `When` conditions name).
pub(crate) mod key {
    pub const VIEW_TYPE: &str = "view_type";
    pub const CONTROLS: &str = "controls";
    pub const INFO: &str = "info";
    pub const ITEM_MODE_WIREFRAME: &str = "item_mode_wireframe";
    pub const BACKGROUND: &str = "background";
    pub const LIGHTS: &str = "lights";
    pub const CAMERAS: &str = "cameras";
    pub const MESHES: &str = "meshes";
    pub const GRID: &str = "grid";
    pub const WORK_PLANE: &str = "work_plane";
    pub const FADED_GRID: &str = "faded_grid";
    pub const GRID_AXES: &str = "grid_axes";
    pub const LOCATORS: &str = "locators";
    pub const REPLICATORS: &str = "replicators";
    pub const TEXTURE_LOCATORS: &str = "texture_locators";
    pub const SCENE_LIGHTING: &str = "scene_lighting";
    pub const SELECTIONS: &str = "selections";
    pub const SELECTION_NORMALS: &str = "selection_normals";
    pub const SELECTED_FILLING: &str = "selected_filling";
    pub const SELECTED_OUTLINE: &str = "selected_outline";
    pub const BACKDROPS: &str = "backdrops";
    pub const SYMMETRY_PLANE: &str = "symmetry_plane";
    pub const TRACKBALL: &str = "trackball";
    pub const ORBIT_SELECTED: &str = "orbit_selected";
    pub const CAMERA_ITEM: &str = "camera_item";
    pub const AMBIENT_INTENSITY: &str = "ambient_intensity";
    pub const AMBIENT_COLOR: &str = "ambient_color";
    pub const MATCAP: &str = "matcap";
    pub const MATCAP_IMAGE: &str = "matcap_image";
    pub const MATCAP_LOAD: &str = "matcap_load";
    pub const INDEPENDENT_POINT_SIZE: &str = "independent_point_size";
    pub const POINT_SIZE: &str = "point_size";
    pub const ROLLOVERS: &str = "rollovers";
    pub const SMOOTHING: &str = "smoothing";
    pub const SHADOWS: &str = "shadows";
    pub const UV_SEAMS: &str = "uv_seams";
    pub const WEIGHTS: &str = "weights";
    pub const DISTORTION: &str = "distortion";
    pub const SHOW_VERTICES: &str = "show_vertices";
    pub const SAME_AS_ACTIVE: &str = "inactive.same_as_active";
    pub const INVISIBLE: &str = "inactive.invisible";
    /// A group's rows: `group(true, STYLE)` is "active.style".
    pub const STYLE: &str = "style";
    pub const OVERLAY: &str = "overlay";
    pub const OPACITY: &str = "opacity";
    pub const WIRE_COLOR: &str = "wire_color";
    pub const SMOOTH_SHADE: &str = "smooth_shade";
    /// A group's Transparency Overrides rows (VP-80).
    pub const T_MODE: &str = "t_mode";
    pub const T_FRONT_COLOR: &str = "t_front_color";
    pub const T_FRONT_OPACITY: &str = "t_front_opacity";
    pub const T_BACK_COLOR: &str = "t_back_color";
    pub const T_BACK_OPACITY: &str = "t_back_opacity";
    pub const T_OVERLAY: &str = "t_overlay";
    pub const T_FLAT: &str = "t_flat";
    pub fn group(active: bool, name: &str) -> String {
        format!("{}.{name}", if active { "active" } else { "inactive" })
    }
}

/// Camera Item's first choice: the camera Camera view picks by itself.
const CURRENT_SELECTION: &str = "(Current Selection)";

/// The overlay choices, in dropdown order.
const OVERLAYS: [(Overlay, &str); 3] = [
    (Overlay::None, "None"),
    (Overlay::Uniform, "Uniform"),
    (Overlay::Colored, "Colored"),
];
/// Show Grid: (`grid`, `grid_follows_work_plane`) per choice.
const SHOW_GRID: [((bool, bool), &str); 3] = [
    ((true, false), "Always"),
    ((true, true), "Inactive Work Plane"),
    ((false, true), "Never"),
];
const SHOW_WORK_PLANE: [(WorkPlaneShow, &str); 3] = [
    (WorkPlaneShow::Always, "Always"),
    (WorkPlaneShow::Active, "Active"),
    (WorkPlaneShow::Never, "Never"),
];
const TRACKBALL: [(Option<bool>, &str); 3] = [
    (None, "Use Preference"),
    (Some(true), "On"),
    (Some(false), "Off"),
];
const BACKGROUNDS: [(Background, &str); 3] = [
    (Background::None, "None"),
    (Background::Gradient, "Gradient"),
    (Background::Environment, "Environment"),
];

fn index_of<T: PartialEq>(choices: &[(T, &str)], value: &T) -> usize {
    choices.iter().position(|(v, _)| v == value).unwrap_or(0)
}
fn names<T>(choices: &[(T, &'static str)]) -> Vec<&'static str> {
    choices.iter().map(|(_, n)| *n).collect()
}

/// The display styles, in the header menu's order.
fn style_index(style: Style) -> usize {
    Style::ALL.iter().position(|&s| s == style).unwrap_or(0)
}
/// Every style but Wireframe, for "greyed when the style is Wireframe".
fn filled_styles() -> Vec<usize> {
    (0..Style::ALL.len())
        .filter(|&i| Style::ALL[i] != Style::Wireframe)
        .collect()
}

/// A colour stored as 8-bit sRGB (what the theme and `Group::wire_color`
/// hold) as the colour field's linear values, and back.
fn linear(rgb: [u8; 3]) -> [f64; 3] {
    let c = egui::Rgba::from(Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
    [c.r(), c.g(), c.b()].map(f64::from)
}
fn encoded(rgb: [f64; 3]) -> [u8; 3] {
    let [r, g, b] = rgb.map(|c| c.clamp(0., 1.) as f32);
    let c = Color32::from(egui::Rgba::from_rgb(r, g, b));
    [c.r(), c.g(), c.b()]
}
fn rgb(c: Color32) -> [u8; 3] {
    [c.r(), c.g(), c.b()]
}

/// The view type choices: every view, plus Camera when the scene has one
/// to look through (or the view already does).
fn view_kinds(camera: bool) -> Vec<(Option<View>, &'static str)> {
    let mut kinds = vec![
        (Some(View::Perspective), "Perspective"),
        (Some(View::Orthographic), "Orthographic"),
        (Some(View::Top), "Top"),
        (Some(View::Bottom), "Bottom"),
        (Some(View::Front), "Front"),
        (Some(View::Back), "Back"),
        (Some(View::Left), "Left"),
        (Some(View::Right), "Right"),
    ];
    if camera {
        kinds.push((None, "Camera"));
    }
    kinds
}

/// One mesh group's Shading rows (Active or Inactive Meshes). `enabled` is
/// the inactive tab's override condition (the overrides grey the
/// rest of the tab).
fn shading_rows(active: bool, enabled: Option<When>) -> Vec<Row> {
    let k = |name| key::group(active, name);
    // `When` names keys as `&'static str`; the two groups' keys are fixed.
    let style: &'static str = if active {
        "active.style"
    } else {
        "inactive.style"
    };
    let overlay: &'static str = if active {
        "active.overlay"
    } else {
        "inactive.overlay"
    };
    let with = |w: When| match &enabled {
        Some(e) => When::All(vec![e.clone(), w]),
        None => w,
    };
    let always = enabled.clone().unwrap_or(When::Always);
    let whose = if active {
        "this view's active (selected) meshes"
    } else {
        "this view's inactive (visible, unselected) meshes"
    };
    let styles: Vec<&str> = Style::ALL.iter().map(|s| s.label()).collect();
    vec![
        Row::choice(&k(key::STYLE), "Shading Style", &styles, 0)
            .when(always.clone())
            .tip(if active {
                "How this view fills its active meshes; the same setting as the view header's shading button".to_owned()
            } else {
                format!("How {whose} fill")
            })
            .not_channel(),
        Row::choice(&k(key::OVERLAY), "Wireframe Overlay", &names(&OVERLAYS), 0)
            .when(with(When::ChoiceIn(style, filled_styles())))
            .tip(format!(
                "The wire drawn over the fill of {whose} (none with the Wireframe style)"
            ))
            .not_channel(),
        Row::number(&k(key::OPACITY), "Wireframe Opacity", Unit::Percent, 70.)
            .range(0., 100.)
            .when(with(When::All(vec![
                When::ChoiceIn(style, filled_styles()),
                When::ChoiceIn(overlay, vec![1, 2]),
            ])))
            .tip("How strongly the wire overlay shows")
            .not_channel(),
        Row::colour(&k(key::WIRE_COLOR), "Wireframe Color", [0.; 3])
            .when(always.clone())
            .tip(if active {
                "The plain wire's colour in this view; follows the theme's Wireframe until you change it"
            } else {
                "The plain wire's colour in this view; follows the theme's Inactive Wireframe until you change it. Locked, seam, crease and selection edges keep their own colours"
            })
            .not_channel(),
        Row::check(&k(key::SMOOTH_SHADE), "Smooth Shade", true)
            .when(always)
            .tip("Off: flat polygons in this view only (renders are unchanged)")
            .not_channel(),
    ]
}

/// The Transparency Mode choices, in dropdown order.
const TRANSPARENCY_MODES: [(TransparencyMode, &str); 3] = [
    (TransparencyMode::Normal, "Normal"),
    (TransparencyMode::Ghost, "Ghost"),
    (TransparencyMode::XRay, "X-Ray"),
];

/// One mesh group's Transparency Overrides rows (VP-80). Everything greys
/// with the Wireframe style or while Transparency Mode is Normal; Front
/// rows grey in X-Ray (the front polygons are clear); Overlay is on the
/// Active Meshes tab only. `enabled` as in `shading_rows`.
fn transparency_rows(active: bool, enabled: Option<When>) -> Vec<Row> {
    let k = |name| key::group(active, name);
    let (style, mode): (&'static str, &'static str) = if active {
        ("active.style", "active.t_mode")
    } else {
        ("inactive.style", "inactive.t_mode")
    };
    let with = |w: When| match &enabled {
        Some(e) => When::All(vec![e.clone(), w]),
        None => w,
    };
    let filled = When::ChoiceIn(style, filled_styles());
    let see_through = with(When::All(vec![
        filled.clone(),
        When::ChoiceIn(mode, vec![1, 2]),
    ]));
    let ghost = with(When::All(vec![
        filled.clone(),
        When::ChoiceIn(mode, vec![1]),
    ]));
    let mut rows = vec![
        Row::choice(&k(key::T_MODE), "Transparency Mode", &names(&TRANSPARENCY_MODES), 0)
            .when(with(filled))
            .tip("Ghost: polygons facing you are see-through and tinted, those facing away solid. X-Ray: only the polygons facing away show. Not with the Wireframe style")
            .not_channel(),
        Row::colour(&k(key::T_FRONT_COLOR), "Front Polygon Color", [0.8; 3])
            .when(ghost.clone())
            .tip("The tint of polygons facing you")
            .not_channel(),
        Row::number(&k(key::T_FRONT_OPACITY), "Front Polygon Opacity", Unit::Percent, 25.)
            .range(0., 100.)
            .when(ghost)
            .tip("How solid the polygons facing you are")
            .not_channel(),
        Row::colour(&k(key::T_BACK_COLOR), "Back Polygon Color", [0.5; 3])
            .when(see_through.clone())
            .tip("The colour of polygons facing away")
            .not_channel(),
        Row::number(&k(key::T_BACK_OPACITY), "Back Polygon Opacity", Unit::Percent, 60.)
            .range(0., 100.)
            .when(see_through.clone())
            .tip("How solid the polygons facing away are")
            .not_channel(),
    ];
    if active {
        rows.push(
            Row::check(&k(key::T_OVERLAY), "Overlay", false)
                .when(see_through.clone())
                .tip("The see-through polygons draw over everything else in this view")
                .not_channel(),
        );
    }
    rows.push(
        Row::check(&k(key::T_FLAT), "Flat Shaded", false)
            .when(see_through)
            .tip("No lighting on the see-through polygons")
            .not_channel(),
    );
    rows
}

/// The panel's tabs and rows, declared for the shared form (VP-40..46).
/// `camera`: the view type list offers Camera.
/// `cameras`: the scene's cameras' names for Camera Item (none: the row
/// is left out).
fn panel_tabs(camera: bool, cameras: &[String]) -> Vec<panel::Tab> {
    use panel::{Section, Tab as PanelTab};
    let check =
        |k: &str, label: &str, tip: &str| Row::check(k, label, false).tip(tip).not_channel();
    let kinds: Vec<&str> = view_kinds(camera).iter().map(|(_, n)| *n).collect();
    let overrides = When::All(vec![
        When::Unchecked(key::SAME_AS_ACTIVE),
        When::Unchecked(key::INVISIBLE),
    ]);
    let mut active_shading = shading_rows(true, None);
    // Operandi draws UV boundaries for the whole view, so the row shows
    // once, with the active meshes.
    active_shading.push(check(
        key::UV_SEAMS,
        "Show UV Boundaries",
        "Edges where the first UV map's islands break (whole view)",
    ));
    // HT-5/HT-6: the header buttons' settings, whole view like the above.
    active_shading.push(check(
        key::WEIGHTS,
        "Show Weightmaps",
        "Colour meshes by the selected weight map (whole view)",
    ));
    active_shading.push(check(
        key::DISTORTION,
        "Show UV Distortion",
        "Colour polygons by UV stretch in the selected UV map (whole view)",
    ));
    vec![
        PanelTab::new(
            Tab::Drawing.label(),
            vec![Section::new(
                "View and Shading",
                vec![
                    Row::choice(key::VIEW_TYPE, "View Type", &kinds, 0)
                        .tip("Same as the view header's menu")
                        .not_channel(),
                    check(
                        key::CONTROLS,
                        "Show Viewport Controls and Information",
                        "Off hides the header's corner buttons and the view's information text; O still opens this",
                    ),
                    check(
                        key::INFO,
                        "Show View Information",
                        "The view and selection lines at the view's top left",
                    )
                    .when(When::Checked(key::CONTROLS)),
                    check(
                        key::ITEM_MODE_WIREFRAME,
                        "Show Wireframe in Item Mode",
                        "Item mode: draw every mesh's wire over its fill",
                    ),
                    Row::choice(key::BACKGROUND, "Background", &names(&BACKGROUNDS), 0)
                        .tip("Flat, or a gradient from Gradient Top (Preferences > Display > Colors)")
                        .not_channel(),
                ]
                .into_iter()
                .chain((!cameras.is_empty()).then(|| {
                    let mut choices = vec![CURRENT_SELECTION];
                    choices.extend(cameras.iter().map(String::as_str));
                    Row::choice(key::CAMERA_ITEM, "Camera Item", &choices, 0)
                        .tip("The camera this view looks through: the selected (or newest) camera, or one chosen here, which switches the view to Camera")
                        .not_channel()
                }))
                .collect(),
            ),
            Section::new(
                "Mouse Control",
                vec![
                    // Provisional placement until spec v2.1 (PREF-Q7):
                    // the view's own Trackball Rotation choice.
                    Row::choice(key::TRACKBALL, "Trackball Rotation", &names(&TRACKBALL), 0)
                        .tip("Orbit in this view: free trackball, two-axis, or as in Preferences")
                        .not_channel(),
                    check(
                        key::ORBIT_SELECTED,
                        "Orbit Selected",
                        "Orbit turns the view around the selection's centre instead of the view's centre",
                    ),
                ],
            )],
        ),
        PanelTab::new(
            Tab::Visibility.label(),
            vec![
                Section::new(
                    "Item Visibility",
                    vec![
                        check(key::LIGHTS, "Show Lights", "Draw lights in this view (off by default)"),
                        check(
                            key::SCENE_LIGHTING,
                            "Show Scene Lighting",
                            "On: the scene's lights light this view (Advanced style). Off: the view's own light does, which keeps it quick while placing lights",
                        ),
                        check(key::CAMERAS, "Show Cameras", "Draw cameras in this view (off by default)"),
                        check(key::LOCATORS, "Show Locators", "Draw group locators in this view"),
                        Row::choice(
                            key::REPLICATORS,
                            "Instancers",
                            &ReplicatorDisplay::ALL.map(|m| m.label()),
                            0,
                        )
                        .tip("Which Instancers' copies draw shaded in this view: None draws every copy as a box, Selected shades the selected Instancer's, All shades them all")
                        .not_channel(),
                        check(key::MESHES, "Show Meshes", "Off hides every mesh item in this view"),
                        check(
                            key::TEXTURE_LOCATORS,
                            "Show Texture Locators",
                            "Draw every texture placement gizmo; off, only the selected one and those of the textures picked in the Shader Tree",
                        ),
                    ],
                ),
                Section::new(
                    "General Visibility",
                    vec![
                        Row::choice(key::WORK_PLANE, "Show Work Plane", &names(&SHOW_WORK_PLANE), 0)
                            .tip("When this view draws the work plane, while Show Work Plane is on (Work Plane Settings or Preferences > Display). Always; Active: only once the work plane has been moved or rotated; Never")
                            .not_channel(),
                        Row::choice(key::GRID, "Show Grid", &names(&SHOW_GRID), 1)
                            .tip("The floor grid. Inactive Work Plane: only while the work plane is at rest, so only one grid shows")
                            .not_channel(),
                        check(
                            key::FADED_GRID,
                            "Draw Faded Grid",
                            "The grids fade out with distance",
                        ),
                        check(
                            key::GRID_AXES,
                            "Faded Grid Color Axes",
                            "The floor's main axis lines in the axis colours (with View > Ground Axes on)",
                        ),
                        check(
                            key::BACKDROPS,
                            "Show Backdrop",
                            "Off hides every Backdrop in this view only",
                        ),
                        check(
                            key::SYMMETRY_PLANE,
                            "Show Symmetry Plane",
                            "With Symmetry on, outline the mirror plane across the active mesh. Off hides only the outline; Symmetry stays on",
                        ),
                    ],
                ),
                Section::new(
                    "Selection Visibility",
                    vec![
                        check(
                            key::SELECTIONS,
                            "Show Selections",
                            "Off hides every selection highlight in this view",
                        ),
                        check(
                            key::SELECTION_NORMALS,
                            "Show Selection Normals",
                            "A dashed line out of each selected polygon, showing which way it faces",
                        )
                        .when(When::Checked(key::SELECTIONS)),
                        check(
                            key::SELECTED_FILLING,
                            "Show Selected Filling",
                            "The tint on selected polygons",
                        )
                        .when(When::Checked(key::SELECTIONS)),
                        check(
                            key::SELECTED_OUTLINE,
                            "Show Selected Outline",
                            "The outline around selected polygons",
                        )
                        .when(When::Checked(key::SELECTIONS)),
                        check(
                            key::ROLLOVERS,
                            "Show Selection Rollovers",
                            "Off: nothing pre-highlights under the pointer in this view",
                        )
                        .when(When::Checked(key::SELECTIONS)),
                    ],
                ),
                Section::new(
                    "Performance",
                    vec![check(
                        key::SMOOTHING,
                        "Mesh Smoothing",
                        "Off: every polygon in this view faceted",
                    )],
                ),
            ],
        ),
        PanelTab::new(
            Tab::Advanced.label(),
            vec![Section::new(
                "Effects",
                vec![
                    Row::number(
                        key::AMBIENT_INTENSITY,
                        "Ambient Intensity",
                        Unit::Percent,
                        44.,
                    )
                    .range(0., 200.)
                    .tip("Advanced style: how much light the sides facing away from the light get")
                    .not_channel(),
                    Row::colour(key::AMBIENT_COLOR, "Ambient Color", [1.; 3])
                        .tip("Advanced style: the colour of that light")
                        .not_channel(),
                    check(
                        key::SHADOWS,
                        "Show Shadows",
                        "Advanced style: shadows from the scene's lights (the view light when there are none)",
                    ),
                ],
            ),
            Section::new("Matcap", matcap_rows())],
        ),
        PanelTab::new(
            Tab::Active.label(),
            vec![
                Section::new("Shading", active_shading),
                Section::new(
                    "Display",
                    vec![
                        Row::choice(
                            key::SHOW_VERTICES,
                            "Show Vertices",
                            &ShowVertices::ALL.map(|m| m.label()),
                            0,
                        )
                        .tip("When the active meshes' points draw in this view; selected points always do")
                        .not_channel(),
                        check(
                            key::INDEPENDENT_POINT_SIZE,
                            "Independent Point Size",
                            "This view draws points at its own size instead of Preferences > Selection's",
                        ),
                        Row::number(key::POINT_SIZE, "Point Size", Unit::Suffix(" px"), 3.)
                            .range(1., 20.)
                            .when(When::Checked(key::INDEPENDENT_POINT_SIZE))
                            .tip("Point dot size in this view")
                            .not_channel(),
                    ],
                ),
                Section::new("Transparency Overrides", transparency_rows(true, None)),
            ],
        ),
        PanelTab::new(
            Tab::Inactive.label(),
            vec![
                Section::new(
                    "Overrides",
                    vec![
                        check(
                            key::SAME_AS_ACTIVE,
                            "Make Inactive Same as Active",
                            "Unselected meshes draw exactly like the active ones (their wire colour too)",
                        ),
                        check(
                            key::INVISIBLE,
                            "Make Inactive Invisible",
                            "Unselected meshes disappear from this view only; they still render and stay in the Item List",
                        ),
                    ],
                ),
                Section::new("Shading", shading_rows(false, Some(overrides.clone()))),
                Section::new(
                    "Transparency Overrides",
                    transparency_rows(false, Some(overrides)),
                ),
            ],
        ),
    ]
}

/// Every row of `tabs`.
fn all_rows(tabs: &[panel::Tab]) -> Vec<Row> {
    tabs.iter()
        .flat_map(|t| t.sections.iter().flat_map(|s| s.rows.iter().cloned()))
        .collect()
}

/// The panel's values from a view's settings (`view`: its type, `None`
/// for Camera).
fn panel_values(
    rows: &[Row],
    settings: &view_display::Settings,
    view: Option<View>,
    camera: bool,
) -> Values {
    let mut v = Values::defaults(rows);
    let d = &settings.drawing;
    let kinds = view_kinds(camera);
    v.set(key::VIEW_TYPE, Value::Choice(index_of(&kinds, &view)));
    for (k, on) in [
        (key::CONTROLS, d.controls),
        (key::INFO, d.info),
        (key::ITEM_MODE_WIREFRAME, d.item_mode_wireframe),
        (key::LIGHTS, d.lights),
        (key::CAMERAS, d.cameras),
        (key::MESHES, d.meshes),
        (key::BACKDROPS, d.backdrops),
        (key::SYMMETRY_PLANE, d.symmetry_plane),
        (key::ROLLOVERS, d.rollovers),
        (key::ORBIT_SELECTED, d.orbit_selected),
        (key::INDEPENDENT_POINT_SIZE, d.independent_point_size),
        (key::FADED_GRID, d.faded_grid),
        (key::GRID_AXES, d.grid_axes),
        (key::LOCATORS, d.locators),
        (key::TEXTURE_LOCATORS, d.texture_locators),
        (key::SCENE_LIGHTING, d.scene_lighting),
        (key::SELECTIONS, d.selections),
        (key::SELECTION_NORMALS, d.selection_normals),
        (key::SELECTED_FILLING, d.selected_filling),
        (key::SELECTED_OUTLINE, d.selected_outline),
        (key::SMOOTHING, d.smoothing),
        (key::SHADOWS, d.shadows),
        (key::UV_SEAMS, d.uv_seams),
        (key::WEIGHTS, d.weights),
        (key::DISTORTION, d.distortion),
        (key::SAME_AS_ACTIVE, d.inactive_same_as_active),
        (key::INVISIBLE, d.inactive_invisible),
    ] {
        v.set(k, Value::Bool(on));
    }
    v.set(
        key::BACKGROUND,
        Value::Choice(index_of(&BACKGROUNDS, &d.background)),
    );
    // Never shows whatever the work-plane link was.
    let grid = (d.grid, d.grid_follows_work_plane || !d.grid);
    v.set(key::GRID, Value::Choice(index_of(&SHOW_GRID, &grid)));
    v.set(
        key::WORK_PLANE,
        Value::Choice(index_of(&SHOW_WORK_PLANE, &d.work_plane)),
    );
    v.set(
        key::TRACKBALL,
        Value::Choice(index_of(&TRACKBALL, &d.trackball)),
    );
    let vertices = ShowVertices::ALL
        .iter()
        .position(|&m| m == d.show_vertices)
        .unwrap_or(0);
    v.set(key::SHOW_VERTICES, Value::Choice(vertices));
    let replicators = ReplicatorDisplay::ALL
        .iter()
        .position(|&m| m == d.replicators)
        .unwrap_or(0);
    v.set(key::REPLICATORS, Value::Choice(replicators));
    v.set(
        key::AMBIENT_INTENSITY,
        Value::Number(f64::from(d.ambient_intensity)),
    );
    v.set(key::AMBIENT_COLOR, Value::Colour(linear(d.ambient_color)));
    v.set(key::MATCAP, Value::Choice(d.matcap.menu_index()));
    v.set(
        key::MATCAP_IMAGE,
        Value::Text(
            d.matcap
                .path()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
        ),
    );
    v.set(key::POINT_SIZE, Value::Number(f64::from(d.point_size)));
    for active in [true, false] {
        let g = if active {
            settings.active
        } else {
            settings.inactive
        };
        let k = |name| key::group(active, name);
        v.set(&k(key::STYLE), Value::Choice(style_index(g.style)));
        v.set(
            &k(key::OVERLAY),
            Value::Choice(index_of(&OVERLAYS, &g.overlay)),
        );
        v.set(&k(key::OPACITY), Value::Number(f64::from(g.opacity)));
        v.set(
            &k(key::WIRE_COLOR),
            Value::Colour(linear(group_wire_color(&g, active))),
        );
        v.set(&k(key::SMOOTH_SHADE), Value::Bool(g.smoothing));
        let t = &g.transparency;
        v.set(
            &k(key::T_MODE),
            Value::Choice(index_of(&TRANSPARENCY_MODES, &t.mode)),
        );
        v.set(&k(key::T_FRONT_COLOR), Value::Colour(linear(t.front_color)));
        v.set(
            &k(key::T_FRONT_OPACITY),
            Value::Number(f64::from(t.front_opacity)),
        );
        v.set(&k(key::T_BACK_COLOR), Value::Colour(linear(t.back_color)));
        v.set(
            &k(key::T_BACK_OPACITY),
            Value::Number(f64::from(t.back_opacity)),
        );
        v.set(&k(key::T_OVERLAY), Value::Bool(t.overlay));
        v.set(&k(key::T_FLAT), Value::Bool(t.flat));
    }
    v
}

/// The colour a group's Wireframe Color row shows: its own, or the theme
/// colour it follows until edited (Wireframe for the active meshes,
/// Inactive Wireframe for the inactive ones).
fn group_wire_color(group: &view_display::Group, active: bool) -> [u8; 3] {
    group.wire_color.unwrap_or_else(|| {
        rgb(if active {
            theme::viewport::wireframe()
        } else {
            theme::viewport::inactive_wireframe()
        })
    })
}

/// Write one changed row back into a view's settings. Returns the new view
/// type when that row changed (`Some(None)`: Camera).
/// The Matcap menu's names (MC-05): the built-in matcaps, then a
/// Library submenu of every matcap under the preset browser's roots
/// (grouped by folder), then Custom Image (any other file).
fn matcap_names() -> Vec<String> {
    matcap_names_in(&crate::matcap::library())
}

fn matcap_names_in(library: &crate::matcap::Library) -> Vec<String> {
    let mut names: Vec<String> = crate::matcap::BUILT_IN
        .iter()
        .map(|(_, label, _)| (*label).to_owned())
        .collect();
    let sub = crate::form::SUBMENU;
    for m in library {
        names.push(format!("Library{sub}{}{sub}{}", m.group, m.name));
    }
    names.push("Custom Image".into());
    names
}

/// The Matcap style's matcap rows (an Operandi extra, Jason
/// 2026-11-10): a menu of the built-in and library matcaps (MC-05) and
/// Custom Image, then for Custom Image the file's path and a button to
/// choose one.
fn matcap_rows() -> Vec<Row> {
    let names = matcap_names();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let custom = names.len() - 1;
    vec![
        Row::choice(
            key::MATCAP,
            "Image",
            &names,
            crate::matcap::Choice::default().menu_index(),
        )
            .tip("Matcap style: the matcap the surfaces show (a lit ball, looked up by which way each point faces)")
            .not_channel(),
        Row::text(key::MATCAP_IMAGE, "Custom Image", "")
            .when(When::ChoiceIn(key::MATCAP, vec![custom]))
            .tip("Matcap style: a PNG, JPEG or EXR image of a lit ball to use as the matcap")
            .not_channel(),
        Row::button(key::MATCAP_LOAD, "Load Image…")
            .when(When::ChoiceIn(key::MATCAP, vec![custom]))
            .tip("Choose the matcap image file"),
    ]
}

fn apply_value(
    settings: &mut view_display::Settings,
    values: &Values,
    k: &str,
    camera: bool,
) -> Option<Option<View>> {
    let d = &mut settings.drawing;
    let flag = |v: &Values| v.bool(k);
    match k {
        key::VIEW_TYPE => {
            return view_kinds(camera)
                .get(values.choice(k))
                .map(|(view, _)| *view);
        }
        key::CONTROLS => d.controls = flag(values),
        key::INFO => d.info = flag(values),
        key::ITEM_MODE_WIREFRAME => d.item_mode_wireframe = flag(values),
        key::LIGHTS => d.lights = flag(values),
        key::CAMERAS => d.cameras = flag(values),
        key::MESHES => d.meshes = flag(values),
        key::BACKDROPS => d.backdrops = flag(values),
        key::ROLLOVERS => d.rollovers = flag(values),
        key::ORBIT_SELECTED => d.orbit_selected = flag(values),
        key::INDEPENDENT_POINT_SIZE => d.independent_point_size = flag(values),
        key::AMBIENT_INTENSITY => {
            d.ambient_intensity = values.number(k).round().clamp(0., 200.) as u8
        }
        key::AMBIENT_COLOR => d.ambient_color = encoded(values.colour(k)),
        key::POINT_SIZE => d.point_size = values.number(k).clamp(1., 20.) as f32,
        key::FADED_GRID => d.faded_grid = flag(values),
        key::GRID_AXES => d.grid_axes = flag(values),
        key::LOCATORS => d.locators = flag(values),
        key::SYMMETRY_PLANE => d.symmetry_plane = flag(values),
        key::TEXTURE_LOCATORS => d.texture_locators = flag(values),
        key::SCENE_LIGHTING => d.scene_lighting = flag(values),
        key::SELECTIONS => d.selections = flag(values),
        key::SELECTION_NORMALS => d.selection_normals = flag(values),
        key::SELECTED_FILLING => d.selected_filling = flag(values),
        key::SELECTED_OUTLINE => d.selected_outline = flag(values),
        key::WORK_PLANE => d.work_plane = SHOW_WORK_PLANE[values.choice(k).min(2)].0,
        key::SMOOTHING => d.smoothing = flag(values),
        key::SHADOWS => d.shadows = flag(values),
        key::UV_SEAMS => d.uv_seams = flag(values),
        key::WEIGHTS => d.weights = flag(values),
        key::DISTORTION => d.distortion = flag(values),
        key::SAME_AS_ACTIVE => d.inactive_same_as_active = flag(values),
        key::INVISIBLE => d.inactive_invisible = flag(values),
        key::BACKGROUND => {
            d.background = BACKGROUNDS[values.choice(k).min(BACKGROUNDS.len() - 1)].0;
        }
        key::GRID => {
            let ((grid, follows), _) = SHOW_GRID[values.choice(k).min(2)];
            d.grid = grid;
            // Never keeps the link, so turning the grid back on from the
            // menus comes back as Inactive Work Plane.
            d.grid_follows_work_plane = follows;
        }
        key::TRACKBALL => d.trackball = TRACKBALL[values.choice(k).min(2)].0,
        key::SHOW_VERTICES => {
            d.show_vertices = ShowVertices::ALL[values.choice(k).min(ShowVertices::ALL.len() - 1)]
        }
        key::REPLICATORS => {
            d.replicators =
                ReplicatorDisplay::ALL[values.choice(k).min(ReplicatorDisplay::ALL.len() - 1)]
        }
        _ => {
            let (active, name) = match k.split_once('.') {
                Some(("active", name)) => (true, name),
                Some(("inactive", name)) => (false, name),
                _ => return None,
            };
            let g = if active {
                &mut settings.active
            } else {
                &mut settings.inactive
            };
            match name {
                key::STYLE => g.style = Style::ALL[values.choice(k).min(Style::ALL.len() - 1)],
                key::OVERLAY => g.overlay = OVERLAYS[values.choice(k).min(2)].0,
                key::OPACITY => g.opacity = values.number(k).round().clamp(0., 100.) as u8,
                // Once edited the view keeps its own colour (no longer
                // following the theme).
                key::WIRE_COLOR => g.wire_color = Some(encoded(values.colour(k))),
                key::SMOOTH_SHADE => g.smoothing = values.bool(k),
                key::T_MODE => g.transparency.mode = TRANSPARENCY_MODES[values.choice(k).min(2)].0,
                key::T_FRONT_COLOR => g.transparency.front_color = encoded(values.colour(k)),
                key::T_FRONT_OPACITY => {
                    g.transparency.front_opacity = values.number(k).round().clamp(0., 100.) as u8
                }
                key::T_BACK_COLOR => g.transparency.back_color = encoded(values.colour(k)),
                key::T_BACK_OPACITY => {
                    g.transparency.back_opacity = values.number(k).round().clamp(0., 100.) as u8
                }
                key::T_OVERLAY => g.transparency.overlay = values.bool(k),
                key::T_FLAT => g.transparency.flat = values.bool(k),
                _ => {}
            }
        }
    }
    None
}

use crate::popup_title::{TitleAction, title_bar};

impl Operandi {
    /// Open the panel for `view`, retarget it there, or close it when it is
    /// already open for that view. `pointer`: opened with O at this point
    /// (the panel goes near it); `None` (the gear, the View menu) hangs it
    /// under the view's gear.
    pub(super) fn toggle_view_properties(&mut self, view: u32, pointer: Option<Pos2>) {
        // UIC-181: one unpinned popover at a time.
        if self.snapping_options.is_some() && !self.snapping_pinned {
            self.close_snapping_options();
        }
        if self.mesh_constraint_popover.is_some() && !self.mesh_constraint_pinned {
            self.close_mesh_constraint_popover();
        }
        let p = &mut self.corner.properties;
        match p.view {
            Some(open) if open == view => self.close_view_properties(),
            Some(_) => {
                // VP-10..19: one panel; O over another view retargets it. A
                // pinned panel stays where it is.
                p.view = Some(view);
                if !p.pinned {
                    p.at = pointer.map(|at| at - NEAR_POINTER);
                    p.entered = false;
                }
            }
            None => {
                p.view = Some(view);
                p.pinned = false;
                p.at = pointer.map(|at| at - NEAR_POINTER);
                p.entered = false;
                self.corner.popover_rect = None;
            }
        }
    }

    /// Close the panel (pinned or not).
    pub(super) fn close_view_properties(&mut self) {
        let p = &mut self.corner.properties;
        p.view = None;
        p.pinned = false;
        p.at = None;
        p.hovered = false;
        p.entered = false;
        self.corner.popover_rect = None;
    }

    /// The view type, as the header's menu sets it: `None` is Camera view.
    pub(super) fn choose_view_type(&mut self, view: Option<View>) {
        match view {
            Some(view) => {
                self.leave_camera_view();
                self.camera.view = view;
            }
            None => self.enter_camera_view(),
        }
    }

    /// The saved panel state at launch: folds, last tab, and a pinned
    /// panel (if its view still exists). Call after `load_views`.
    pub(super) fn load_view_properties(&mut self) {
        if let Ok(text) = std::fs::read_to_string(self.environment.paths().viewport_properties())
            && let Some(state) = SavedState::parse(&text)
        {
            let views = self.views.viewports();
            let p = &mut self.corner.properties;
            p.collapsed = state.collapsed.clone();
            p.tab = state.tab;
            if let Some(pin) = state.pinned
                && views.contains(&pin.view)
            {
                p.view = Some(pin.view);
                p.pinned = true;
                p.at = Some(pos2(pin.at[0], pin.at[1]));
            }
            p.saved = Some(state);
        }
        self.corner.properties.presets =
            view_tab_presets::TabPresets::load(&self.environment.paths().viewport_tab_presets());
    }

    fn view_properties_state(&self) -> SavedState {
        let p = &self.corner.properties;
        SavedState {
            collapsed: p.collapsed.clone(),
            tab: p.tab,
            pinned: match (p.pinned, p.view, p.at) {
                (true, Some(view), Some(at)) => Some(Pinned {
                    view,
                    at: [at.x, at.y],
                }),
                _ => None,
            },
        }
    }

    /// After every view drew: close the panel when its target view is gone
    /// (VP-10..19, even pinned), and save its state when that changed.
    pub(super) fn view_properties_frame_end(&mut self) {
        if let Some(view) = self.corner.properties.view
            && !self.views.viewports().contains(&view)
        {
            self.close_view_properties();
        }
        let state = self.view_properties_state();
        if self.corner.properties.saved.as_ref() != Some(&state) {
            if let Ok(text) = serde_json::to_string_pretty(&state) {
                let _ = std::fs::write(self.environment.paths().viewport_properties(), text);
            }
            self.corner.properties.saved = Some(state);
        }
    }

    /// Draw the panel if it is open for the view being drawn (its camera
    /// and display are `camera` and `display` now).
    pub(super) fn view_properties_popover(&mut self, ui: &Ui, view: u32) {
        if self.corner.properties.view != Some(view) {
            return;
        }
        let ctx = ui.ctx().clone();
        // VP-10..19: the target view gets a thin accent outline while open.
        ui.painter().rect_stroke(
            self.viewport_rect.shrink(0.5),
            CornerRadius::ZERO,
            Stroke::new(1.5_f32, theme::orange()),
            StrokeKind::Inside,
        );
        let anchor = self
            .corner
            .rects
            .get(&(view, viewport_buttons::Corner::Properties))
            .copied()
            .filter(|_| self.display.settings.drawing.controls)
            .or_else(|| {
                self.corner
                    .areas
                    .get(&view)
                    .map(|a| Rect::from_min_size(a.right_top() - vec2(18., 0.), vec2(18., 18.)))
            })
            .unwrap_or_else(|| Rect::from_min_size(self.viewport_rect.right_top(), Vec2::ZERO));
        let screen = ctx.content_rect();
        let at = self.corner.properties.at;
        let top = at.map_or(anchor.bottom(), |a| a.y);
        // Capped at the window's height less a margin; longer content
        // scrolls (VP-40..46).
        let room = (screen.bottom() - top - 72.).max(140.);
        let area = egui::Area::new(Id::new("viewport-properties"))
            .order(egui::Order::Foreground)
            .constrain_to(screen);
        // VP-10..19: opened with O, top-left near the pointer (the area keeps
        // it on screen); from the gear, hanging under it.
        let area = match at {
            Some(at) => area.fixed_pos(at),
            None => area
                .pivot(Align2::RIGHT_TOP)
                .fixed_pos(anchor.right_bottom()),
        };
        let shown = area.show(&ctx, |ui| {
            egui::Frame::popup(&ctx.style())
                .fill(theme::popup_fill())
                .show(ui, |ui| self.view_properties_ui(ui, room))
                .inner
        });
        let rect = shown.response.rect;
        self.corner.popover_rect = Some(rect);
        let pinned = self.corner.properties.pinned;
        match shown.inner {
            TitleAction::Circle if pinned => {
                self.close_view_properties();
                return;
            }
            TitleAction::Circle => {
                // VP-10..19: pinned where it is now, a floating panel.
                let p = &mut self.corner.properties;
                p.pinned = true;
                p.at = Some(rect.min);
            }
            TitleAction::Drag(delta) if pinned => {
                // VP-10..19: a pinned panel drags by its title bar.
                let p = &mut self.corner.properties;
                p.at = Some(p.at.unwrap_or(rect.min) + delta);
            }
            _ => {}
        }
        let hover = ctx.input(|i| i.pointer.hover_pos());
        let from_gear = self.corner.properties.at.is_none();
        let near = |p: Pos2| {
            rect.expand(ROLL_OFF).contains(p) || (from_gear && anchor.expand(4.).contains(p))
        };
        let p = &mut self.corner.properties;
        p.hovered = hover.is_some_and(near);
        if p.hovered {
            p.entered = true;
        }
        if pinned {
            return;
        }
        // A pop-up opened from one of its fields (a dropdown, the colour
        // picker) hangs outside it: that isn't leaving (VP-10..19).
        let child_open = egui::Popup::is_any_open(&ctx) || crate::colour_picker::is_open();
        if child_open {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.close_view_properties();
            return;
        }
        let clicked_outside = ctx
            .input(|i| i.pointer.any_click() && i.pointer.interact_pos().is_some_and(|q| !near(q)));
        // Popover Dismissal Behavior: Force Click-Off keeps it until a
        // click outside; otherwise rolling off closes it (VP-10..19), with no
        // click. A button held (a field drag) never rolls it off.
        let entered = self.corner.properties.entered;
        let rolled_off = !self.ui_preferences.click_off()
            && entered
            && !ctx.input(|i| i.pointer.any_down())
            && hover.is_some_and(|q| !near(q));
        let close =
            rolled_off || ((self.ui_preferences.click_off() || !entered) && clicked_outside);
        if close {
            self.close_view_properties();
        }
    }

    /// The panel's contents: title bar, header row, then the form with its
    /// tabs. `room` is the most height the form may take.
    fn view_properties_ui(&mut self, ui: &mut Ui, room: f32) -> TitleAction {
        ui.set_width(POPOVER_WIDTH);
        ui.spacing_mut().item_spacing.y = 2.;
        self.corner.properties.rows.clear();
        // VP-10..19: "Viewport Properties: <view>" (the view type; Operandi's
        // views have no custom names yet).
        let title = format!("Viewport Properties: {}", self.camera.view_label());
        let action = title_bar(ui, &title, self.corner.properties.pinned);
        self.mode_row(ui);
        let camera =
            camera_light_ui::view_camera(&self.editor.scene).is_some() || self.camera.through;
        let (camera_ids, camera_names): (Vec<u64>, Vec<String>) = self
            .editor
            .scene
            .items
            .iter()
            .filter(|i| i.camera.is_some())
            .map(|i| (i.id, i.name.clone()))
            .unzip();
        let tabs = panel_tabs(camera, &camera_names);
        let rows = all_rows(&tabs);
        let view = (!self.camera.through).then_some(self.camera.view);
        let mut values = panel_values(&rows, &self.display.settings, view, camera);
        // Camera Item: the camera this view looks through when it was
        // chosen here (or by a script), else (Current Selection).
        let chosen = self
            .camera
            .through
            .then_some(self.camera.through_item)
            .flatten()
            .and_then(|id| camera_ids.iter().position(|&c| c == id));
        values.set(key::CAMERA_ITEM, Value::Choice(chosen.map_or(0, |i| i + 1)));
        let mut state = panel::State {
            tab: self.corner.properties.tab.index(),
            folded: self.corner.properties.collapsed.clone(),
        };
        // As tall as the shown tab needs (and the tab strip), up to `room`.
        let needed = (panel::content_height(&tabs[state.tab], &state) + 6.)
            .max(panel::strip_length(ui, &tabs));
        let height = needed.min(room);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        let changes = panel::show(
            &mut child,
            "viewport-properties",
            &tabs,
            &mut state,
            &mut values,
            &BTreeSet::new(),
        );
        self.text_editors.extend(changes.text_ids.iter().copied());
        let p = &mut self.corner.properties;
        p.rows.extend(changes.rects);
        p.tab = Tab::ALL.get(state.tab).copied().unwrap_or_default();
        p.collapsed = state.folded;
        for k in &changes.keys {
            // The matcap rows run operandi.view3d.matcap (UIC-164).
            if let Some(line) = matcap_line(&values, k) {
                self.execute(&line);
                continue;
            }
            if k == key::CAMERA_ITEM {
                match values
                    .choice(k)
                    .checked_sub(1)
                    .and_then(|i| camera_ids.get(i))
                {
                    Some(&id) => {
                        if let Err(error) = self.look_through(id) {
                            self.status = error;
                        }
                    }
                    None => self.camera.through_item = None,
                }
                continue;
            }
            // Rows with a view3d.* command run it (UIC-164/165).
            if let Some(line) = self.view3d_line(&values, k) {
                self.execute(&line);
                continue;
            }
            if let Some(view) = apply_value(&mut self.display.settings, &values, k, camera) {
                self.choose_view_type(view);
            }
        }
        action
    }

    /// The header row (UIC-103), right-aligned: Presets ▾, One Tab ▾ and
    /// All Tabs ▾. One Tab and All Tabs hold presets of the panel's values
    /// (FLAGS F11); how the tabs show (one, or all in one column) is on the
    /// O key over the popover.
    fn mode_row(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let all = self.header_menu(ui, "All Tabs ⏷", |a, ui| a.all_tabs_menu(ui));
                all.on_hover_text(
                    "Presets for every tab of this view (or chosen tabs), and the \
                     default new views start from",
                );
                let one = self.header_menu(ui, "One Tab ⏷", |a, ui| a.one_tab_menu(ui));
                one.on_hover_text("Presets for the tab on show");
                let presets = self.header_menu(ui, "Presets ⏷", |a, ui| {
                    // VP-90: the saved viewport presets (the Display menu's),
                    // then saving and reverting.
                    let saved = a.view_presets.all();
                    for preset in &saved {
                        if ui.button(&preset.name).clicked() {
                            a.display.choose(preset);
                            a.status = format!("Viewport preset: {}", preset.name);
                            ui.close();
                        }
                    }
                    if !saved.is_empty() {
                        ui.separator();
                    }
                    if ui.button("Save Preset…").clicked() {
                        a.open_save_viewport_preset();
                        a.close_view_properties();
                        ui.close();
                    }
                    if ui.button("Revert to Default").clicked() {
                        a.display = view_display::ViewDisplay::default();
                        a.status = "Viewport properties reverted to defaults".into();
                        ui.close();
                    }
                });
                presets.on_hover_text("Save this view's settings as a preset, or revert them");
            });
        });
    }

    /// One small text pop-up of the header row (a word plus ▾).
    fn header_menu(
        &mut self,
        ui: &mut Ui,
        text: &str,
        menu: impl FnOnce(&mut Self, &mut Ui),
    ) -> Response {
        let button = Button::new(RichText::new(text).color(theme::text()).small()).frame(false);
        let response = egui::containers::menu::MenuButton::from_button(button)
            .ui(ui, |ui| {
                self.menu_owned_input = true;
                ui.set_min_width(190.);
                menu(self, ui);
            })
            .0;
        self.corner
            .properties
            .rows
            .insert(text.into(), response.rect);
        response
    }

    fn save_tab_presets(&mut self) {
        let path = self.environment.paths().viewport_tab_presets();
        if let Err(error) = self.corner.properties.presets.save(&path) {
            self.status = format!("Could not save the viewport presets: {error}");
        }
    }

    /// A name field inside a menu. Enter accepts the name.
    fn preset_name_field(
        &mut self,
        ui: &mut Ui,
        hint: &str,
        rename: Option<(bool, usize, &str)>,
    ) -> Option<String> {
        let draft_id = rename.map(|(one, index, _)| ui.id().with(("rename-preset", one, index)));
        let pass = ui.ctx().cumulative_pass_nr();
        // A rename draft belongs to one submenu opening. Closing the menu
        // leaves a gap in drawing it, so reopening starts from the saved name.
        let draft = draft_id.and_then(|id| {
            ui.data(|d| d.get_temp::<(String, u64)>(id))
                .filter(|(_, last)| *last + 1 >= pass)
        });
        let mut opening = draft.is_none();
        let mut draft =
            rename.map(|(_, _, name)| draft.map_or_else(|| name.to_owned(), |(text, _)| text));
        let name = draft.as_mut().unwrap_or(&mut self.corner.properties.naming);
        let response = ui.add(
            TextEdit::singleline(name)
                .hint_text(hint)
                .id_salt("viewport-preset-name")
                .desired_width(170.),
        );
        self.text_editors.insert(response.id);
        crate::numeric_tab::register(ui, &response, Some(name));
        if draft_id.is_some() && opening {
            crate::numeric_tab::focus_and_select_on_open(&response, name, &mut opening);
        } else if !response.has_focus() && !response.lost_focus() {
            response.request_focus();
        }
        let accept = response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let accepted_name = accept
            .then(|| std::mem::take(name))
            .filter(|n| !n.trim().is_empty());
        if let Some(id) = draft_id {
            if accepted_name.is_some() || ui.input(|i| i.key_pressed(Key::Escape)) {
                ui.data_mut(|d| d.remove::<(String, u64)>(id));
            } else {
                ui.data_mut(|d| d.insert_temp(id, (name.clone(), pass)));
            }
        }
        accepted_name
    }

    /// One Tab ▾: the shown tab's presets, then (new for 3D Viewport),
    /// (rename), (delete); rename and delete grey with nothing to act on.
    fn one_tab_menu(&mut self, ui: &mut Ui) {
        use view_tab_presets::TYPE_NAME;
        let tab = self.corner.properties.tab;
        let saved: Vec<(usize, String)> = self
            .corner
            .properties
            .presets
            .one_tab
            .iter()
            .enumerate()
            .filter(|(_, p)| p.holds(tab))
            .map(|(i, p)| (i, p.name.clone()))
            .collect();
        for (i, name) in &saved {
            if ui.button(name).clicked() {
                let preset = self.corner.properties.presets.one_tab[*i].clone();
                preset.apply(&mut self.display.settings);
                self.status = format!("{}: {name}", tab.label());
                ui.close();
            }
        }
        if !saved.is_empty() {
            ui.separator();
        }
        let new = format!("(new for {TYPE_NAME})");
        ui.menu_button(new, |ui| {
            if let Some(name) = self.preset_name_field(ui, "Preset name, then Enter", None) {
                let values = self.display.settings;
                self.corner
                    .properties
                    .presets
                    .add(true, &name, &[tab], values);
                self.save_tab_presets();
                ui.close();
            }
        });
        self.rename_delete_menus(ui, true, &saved);
    }

    /// (rename) and (delete) sub-menus over `saved` (index, name).
    fn rename_delete_menus(&mut self, ui: &mut Ui, one: bool, saved: &[(usize, String)]) {
        ui.gated_ui(
            crate::unavailable::unless(!saved.is_empty(), "No saved presets yet"),
            |ui| {
                ui.menu_button("(rename)", |ui| {
                    for (i, name) in saved {
                        ui.menu_button(name, |ui| {
                            if let Some(new) = self.preset_name_field(
                                ui,
                                "New name, then Enter",
                                Some((one, *i, name)),
                            ) {
                                self.corner.properties.presets.rename(one, *i, &new);
                                self.save_tab_presets();
                                ui.close();
                            }
                        });
                    }
                });
                ui.menu_button("(delete)", |ui| {
                    for (i, name) in saved {
                        if ui.button(name).clicked() {
                            self.corner.properties.presets.delete(one, *i);
                            self.save_tab_presets();
                            ui.close();
                        }
                    }
                });
            },
        );
    }

    /// All Tabs ▾: a find field, the saved presets, (new, all tabs …),
    /// (new, selected tabs …), (rename), (delete); then Load / Save
    /// Default for 3D Viewport. (The hauling defaults come with hauling.)
    fn all_tabs_menu(&mut self, ui: &mut Ui) {
        use view_tab_presets::TYPE_NAME;
        let find = ui.add(
            TextEdit::singleline(&mut self.corner.properties.find)
                .hint_text("Find")
                .id_salt("viewport-preset-find")
                .desired_width(170.),
        );
        self.text_editors.insert(find.id);
        crate::numeric_tab::register(ui, &find, Some(&self.corner.properties.find));
        let needle = self.corner.properties.find.trim().to_lowercase();
        let saved: Vec<(usize, String)> = self
            .corner
            .properties
            .presets
            .all_tabs
            .iter()
            .enumerate()
            .filter(|(_, p)| needle.is_empty() || p.name.to_lowercase().contains(&needle))
            .map(|(i, p)| (i, p.name.clone()))
            .collect();
        for (i, name) in &saved {
            if ui.button(name).clicked() {
                let preset = self.corner.properties.presets.all_tabs[*i].clone();
                preset.apply(&mut self.display.settings);
                self.status = format!("Viewport preset: {name}");
                ui.close();
            }
        }
        ui.separator();
        ui.menu_button(format!("(new, all tabs for {TYPE_NAME})"), |ui| {
            if let Some(name) = self.preset_name_field(ui, "Preset name, then Enter", None) {
                let values = self.display.settings;
                self.corner
                    .properties
                    .presets
                    .add(false, &name, &Tab::ALL, values);
                self.save_tab_presets();
                ui.close();
            }
        });
        ui.menu_button(format!("(new, selected tabs for {TYPE_NAME})"), |ui| {
            for tab in Tab::ALL {
                let mut on = self.corner.properties.chosen_tabs.contains(&tab);
                if ui.checkbox(&mut on, tab.label()).changed() {
                    let chosen = &mut self.corner.properties.chosen_tabs;
                    chosen.retain(|&t| t != tab);
                    if on {
                        chosen.push(tab);
                    }
                }
            }
            let chosen: Vec<Tab> = Tab::ALL
                .into_iter()
                .filter(|t| self.corner.properties.chosen_tabs.contains(t))
                .collect();
            ui.gated_ui(
                crate::unavailable::unless(
                    !chosen.is_empty(),
                    "Tick at least one tab to save first",
                ),
                |ui| {
                    if let Some(name) = self.preset_name_field(ui, "Preset name, then Enter", None)
                    {
                        let values = self.display.settings;
                        self.corner
                            .properties
                            .presets
                            .add(false, &name, &chosen, values);
                        self.save_tab_presets();
                        ui.close();
                    }
                },
            );
        });
        self.rename_delete_menus(ui, false, &saved);
        ui.separator();
        let has_default = self.corner.properties.presets.default.is_some();
        if ui
            .gated(
                crate::unavailable::unless(has_default, "Save a default first"),
                Button::new(format!("Load Default for {TYPE_NAME}")),
            )
            .clicked()
        {
            if let Some(d) = self.corner.properties.presets.default {
                self.display.settings = d;
            }
            ui.close();
        }
        if ui
            .button(format!("Save Default for {TYPE_NAME}"))
            .on_hover_text("New views start with this view's settings")
            .clicked()
        {
            self.corner.properties.presets.default = Some(self.display.settings);
            self.save_tab_presets();
            self.status = "New views will start with this view's settings".into();
            ui.close();
        }
    }
}

// ------------------------------------------------- view3d.* commands

/// The Modo `view3d.*` command for each Viewport Properties row (UIC-165,
/// specifications/command-names/MODO-NAMES-SUBSET.txt): (command, row
/// key). Group rows (`active.` / `inactive.`) take `bgmesh:1` for the
/// Inactive Meshes group (provisional: the map doesn't say what bgmesh's
/// values mean). The panel runs these commands for its row edits, and they
/// apply through `apply_value`, so the two can't disagree.
pub(crate) const VIEW3D: &[(&str, &str)] = &[
    ("view3d.controls", key::CONTROLS),
    ("view3d.showHUD", key::INFO),
    ("view3d.showWireframeItemMode", key::ITEM_MODE_WIREFRAME),
    ("view3d.bgEnvironment", key::BACKGROUND),
    ("view3d.showLights", key::LIGHTS),
    ("view3d.showCameras", key::CAMERAS),
    ("view3d.showMeshes", key::MESHES),
    ("view3d.showGrid", key::GRID),
    ("view3d.showWorkPlane", key::WORK_PLANE),
    ("view3d.fadedGrid", key::FADED_GRID),
    ("view3d.fadedGridAxes", key::GRID_AXES),
    ("view3d.showLocators", key::LOCATORS),
    ("view3d.showTextureLocators", key::TEXTURE_LOCATORS),
    ("view3d.showLightIllumination", key::SCENE_LIGHTING),
    ("view3d.showSelections", key::SELECTIONS),
    ("view3d.selectedNormals", key::SELECTION_NORMALS),
    ("view3d.fillSelected", key::SELECTED_FILLING),
    ("view3d.outlineSelected", key::SELECTED_OUTLINE),
    ("view3d.showBackdrop", key::BACKDROPS),
    // An Operandi extra (Jason, 2026-11-14): hide the symmetry plane outline.
    ("operandi.view3d.showSymmetryPlane", key::SYMMETRY_PLANE),
    ("view3d.useTrackball", key::TRACKBALL),
    ("view3d.orbitSelected", key::ORBIT_SELECTED),
    ("view3d.gnzAmbientIntensity", key::AMBIENT_INTENSITY),
    ("view3d.gnzAmbientColor", key::AMBIENT_COLOR),
    ("view3d.indPointSize", key::INDEPENDENT_POINT_SIZE),
    ("view3d.pointSize", key::POINT_SIZE),
    ("view3d.showSelectionRollover", key::ROLLOVERS),
    ("view3d.meshSmoothing", key::SMOOTHING),
    ("view3d.showGnzShadows", key::SHADOWS),
    ("view3d.showUVBounds", key::UV_SEAMS),
    ("view3d.showweights", key::WEIGHTS),
    ("view3d.showDistortion", key::DISTORTION),
    ("view3d.sameAsActive", key::SAME_AS_ACTIVE),
    ("view3d.inactiveInvisible", key::INVISIBLE),
    ("view3d.projection", key::VIEW_TYPE),
    ("view3d.shadingStyle", key::STYLE),
    ("view3d.wireframeOverlay", key::OVERLAY),
    ("view3d.wireframeAlpha", key::OPACITY),
    ("view3d.wireframeViewColor", key::WIRE_COLOR),
    ("view3d.TransparencyOverride", key::T_MODE),
    (
        "view3d.TransparencyOverrideSolidColorFront",
        key::T_FRONT_COLOR,
    ),
    (
        "view3d.TransparencyOverrideSolidAlphaFront",
        key::T_FRONT_OPACITY,
    ),
    (
        "view3d.TransparencyOverrideSolidColorBack",
        key::T_BACK_COLOR,
    ),
    (
        "view3d.TransparencyOverrideSolidAlphaBack",
        key::T_BACK_OPACITY,
    ),
    ("view3d.TransparencyOverrideOverlayMode", key::T_OVERLAY),
    ("view3d.TransparencyOverrideFlatShade", key::T_FLAT),
];

/// The active view's Viewport Properties value of check row `k` (tests).
#[cfg(test)]
pub(crate) fn row_value_for_tests(a: &Operandi, k: &str) -> bool {
    let rows = view3d_rows();
    panel_values(&rows, &a.display.settings, Some(a.camera.view), false).bool(k)
}

/// Group row names: their key is `active.<name>` or `inactive.<name>`.
const GROUPED: &[&str] = &[
    key::STYLE,
    key::OVERLAY,
    key::OPACITY,
    key::WIRE_COLOR,
    key::T_MODE,
    key::T_FRONT_COLOR,
    key::T_FRONT_OPACITY,
    key::T_BACK_COLOR,
    key::T_BACK_OPACITY,
    key::T_OVERLAY,
    key::T_FLAT,
];

/// Rows kept 0–100 in the panel that take Modo's 0–1 percent.
const PERCENT_ROWS: &[&str] = &[key::OPACITY, key::T_FRONT_OPACITY, key::T_BACK_OPACITY];

/// The command for row `k` and whether it names the inactive group.
fn view3d_command(k: &str) -> Option<(&'static str, bool)> {
    let (inactive, name) = match k.split_once('.') {
        Some(("active", n)) if GROUPED.contains(&n) => (false, n),
        Some(("inactive", n)) if GROUPED.contains(&n) => (true, n),
        _ => (false, k),
    };
    VIEW3D
        .iter()
        .find(|(_, row)| *row == name)
        .map(|(c, _)| (*c, inactive))
}

/// The Viewport Properties rows (every tab, a camera listed).
fn view3d_rows() -> Vec<Row> {
    all_rows(&panel_tabs(true, &[]))
}

/// The command a Viewport Properties row label runs, for the UIC-164 test.
#[cfg(test)]
pub(crate) fn view3d_for_label(label: &str) -> Option<&'static str> {
    let row = view3d_rows().into_iter().find(|r| r.label == label)?;
    if [key::MATCAP, key::MATCAP_IMAGE, key::MATCAP_LOAD].contains(&row.key.as_str()) {
        return Some(MATCAP_COMMAND);
    }
    view3d_command(&row.key).map(|(c, _)| c)
}

/// The Matcap style's command (an Operandi extra; UIC-165 marks it
/// Operandi-only).
pub(crate) const MATCAP_COMMAND: &str = "operandi.view3d.matcap";

/// Command text for a value: braced when it holds a space.
fn braced(t: &str) -> String {
    if t.contains(' ') {
        format!("{{{t}}}")
    } else {
        t.to_owned()
    }
}

/// The `operandi.view3d.matcap` line a matcap row runs, if `k` is one:
/// Matcap with the chosen name (Custom Image with the field's image, or
/// none so the file chooser asks), Matcap Image with its path, Load
/// Image… with no image so the chooser asks (UIC-162).
fn matcap_line(values: &Values, k: &str) -> Option<String> {
    matcap_line_in(values, k, &crate::matcap::library())
}

fn matcap_line_in(values: &Values, k: &str, library: &crate::matcap::Library) -> Option<String> {
    use crate::matcap::BUILT_IN;
    let image = values.text(key::MATCAP_IMAGE).trim().to_owned();
    let custom = |image: &str| {
        if image.is_empty() {
            format!("{MATCAP_COMMAND} name:custom")
        } else {
            format!("{MATCAP_COMMAND} name:custom image:{}", braced(image))
        }
    };
    let i = values.choice(k);
    match k {
        key::MATCAP => Some(match BUILT_IN.get(i) {
            Some((name, ..)) => format!("{MATCAP_COMMAND} name:{name}"),
            // MC-05: a library matcap is a custom image by its path.
            None => match library.get(i - BUILT_IN.len()) {
                Some(m) => custom(&m.path.display().to_string()),
                None => custom(&image),
            },
        }),
        // An empty field changes nothing.
        key::MATCAP_IMAGE => (!image.is_empty()).then(|| custom(&image)),
        key::MATCAP_LOAD => Some(custom("")),
        _ => None,
    }
}

/// A row's value as command text (choices by their label).
fn value_text(row: &Row, name: &str, value: &Value) -> String {
    let brace = |t: &str| {
        if t.contains(' ') {
            format!("{{{t}}}")
        } else {
            t.to_owned()
        }
    };
    match (value, &row.kind) {
        (Value::Bool(b), _) => b.to_string(),
        (Value::Number(n), _) if PERCENT_ROWS.contains(&name) => format!("{}", n / 100.),
        (Value::Number(n), _) => format!("{n}"),
        (Value::Choice(i), Kind::Choice(options) | Kind::Segments(options)) => {
            options.get(*i).map_or_else(|| i.to_string(), |o| brace(o))
        }
        (Value::Colour(c), _) => format!("{},{},{}", c[0], c[1], c[2]),
        _ => String::new(),
    }
}

/// Command text as a row value.
fn parse_value(row: &Row, name: &str, text: &str) -> std::result::Result<Value, String> {
    let label = &row.label;
    match &row.kind {
        Kind::Check => operandi_core::command_args::boolean(label, text).map(Value::Bool),
        Kind::Number { .. } if PERCENT_ROWS.contains(&name) => {
            operandi_core::command_names::script_percent(text).map(Value::Number)
        }
        Kind::Number { .. } => text
            .parse::<f64>()
            .map(Value::Number)
            .map_err(|_| format!("{label} takes a number, not \"{text}\"")),
        Kind::Choice(options) | Kind::Segments(options) => {
            let names: Vec<&str> = options.iter().map(String::as_str).collect();
            // Shading Style also takes a style's old name (Reflection is
            // Matcap now), so scripts keep working.
            let old = (name == key::STYLE)
                .then(|| {
                    Style::ALL.iter().position(|s| {
                        s.old_label()
                            .is_some_and(|o| o.eq_ignore_ascii_case(text.trim()))
                    })
                })
                .flatten();
            match old {
                Some(i) => Ok(Value::Choice(i)),
                None => operandi_core::command_args::choice(label, text, &names).map(Value::Choice),
            }
        }
        Kind::Colour => {
            let parts: Vec<f64> = text
                .trim_matches(|c| c == '[' || c == ']')
                .split(',')
                .map(|p| p.trim().parse::<f64>())
                .collect::<std::result::Result<_, _>>()
                .map_err(|_| format!("{label} takes r,g,b (0–1), not \"{text}\""))?;
            match parts.as_slice() {
                [r, g, b] => Ok(Value::Colour([*r, *g, *b])),
                _ => Err(format!("{label} takes r,g,b (0–1), not \"{text}\"")),
            }
        }
        _ => Err(format!("{label} can't be set by a command")),
    }
}

impl Operandi {
    /// The command line for a Viewport Properties row edit, if the row has
    /// a `view3d.*` command.
    fn view3d_line(&self, values: &Values, k: &str) -> Option<String> {
        let (command, inactive) = view3d_command(k)?;
        let rows = view3d_rows();
        let row = rows.iter().find(|r| r.key == k)?;
        let name = k.split_once('.').map_or(k, |(_, n)| n);
        let value = values.get(k)?;
        let text = value_text(row, name, value);
        Some(if inactive {
            format!("{command} {text} bgmesh:1")
        } else {
            format!("{command} {text}")
        })
    }

    /// `operandi.view3d.matcap name:<built-in|custom> [image:<path>]
    /// [bgmesh:0|1] [all:0|1]`, or `?`: the active view's matcap. The
    /// matcap is the view's, so bgmesh changes nothing. `all:1` (MC-06,
    /// the browser's Use in All 3D Views) sets it in every 3D view and
    /// switches each to the Matcap style. `name:custom` without an image
    /// opens the file chooser; `?` only reads (UIC-163). Like the other
    /// view3d settings it is not an undo step (MC-04). False when `text`
    /// isn't this command.
    fn view3d_matcap(&mut self, text: &str) -> bool {
        use crate::matcap::{BUILT_IN, Choice};
        let Ok(args) = command::tokens(text) else {
            return false;
        };
        if !args
            .first()
            .is_some_and(|c| c.eq_ignore_ascii_case(MATCAP_COMMAND))
        {
            return false;
        }
        let usage = format!(
            "Use {MATCAP_COMMAND} name:<name|custom> [image:<path>] [bgmesh:0|1] [all:0|1], or ?"
        );
        let (mut name, mut image, mut all) = (None::<String>, None::<String>, false);
        for arg in &args[1..] {
            match arg.split_once(':') {
                Some((k, _)) if k.eq_ignore_ascii_case("bgmesh") => {}
                Some((k, v)) if k.eq_ignore_ascii_case("all") => {
                    match operandi_core::command_args::boolean("all", v) {
                        Ok(on) => all = on,
                        Err(e) => {
                            self.status = e;
                            return true;
                        }
                    }
                }
                Some((k, v)) if k.eq_ignore_ascii_case("name") => name = Some(v.to_owned()),
                Some((k, v)) if k.eq_ignore_ascii_case("image") => image = Some(v.to_owned()),
                None if name.is_none() => name = Some(arg.clone()),
                _ => {
                    self.status = usage;
                    return true;
                }
            }
        }
        let Some(name) = name else {
            self.status = usage;
            return true;
        };
        let current = self.display.settings.drawing.matcap;
        if name == "?" {
            self.status = match current {
                Choice::BuiltIn(_) => format!("name:{}", BUILT_IN[current.menu_index()].0),
                Choice::Custom(_) => format!(
                    "name:custom image:{}",
                    braced(&current.path().unwrap_or_default().display().to_string())
                ),
            };
            return true;
        }
        if name.eq_ignore_ascii_case("custom") {
            match image {
                Some(path) => {
                    let path = std::path::PathBuf::from(path);
                    if !path.is_file() {
                        self.status = format!("No image at {}", path.display());
                        return true;
                    }
                    self.set_matcap(Choice::custom(&path), all);
                }
                None => {
                    // The missing image is asked for (UIC-162).
                    let folder = current
                        .path()
                        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                        .unwrap_or_default();
                    self.choose(
                        crate::file_chooser::Choose::Image { folder },
                        crate::file_chooser::Purpose::MatcapImage,
                    );
                }
            }
            return true;
        }
        match BUILT_IN.iter().position(|(k, label, _)| {
            k.eq_ignore_ascii_case(&name) || label.eq_ignore_ascii_case(&name)
        }) {
            Some(i) => self.set_matcap(Choice::BuiltIn(i as u8), all),
            None => {
                let names: Vec<&str> = BUILT_IN.iter().map(|(k, ..)| *k).collect();
                self.status = format!(
                    "No matcap {name}: use custom or one of {}",
                    names.join(", ")
                );
            }
        }
        true
    }

    /// Set the active view's matcap, or (`all`) every 3D view's, which
    /// also switches each to the Matcap style (MC-06).
    fn set_matcap(&mut self, choice: crate::matcap::Choice, all: bool) {
        self.display.settings.drawing.matcap = choice;
        if all {
            self.display.settings.active.style = Style::Reflection;
            for view in self.views.viewports() {
                if view == self.active_view {
                    continue;
                }
                let display = self
                    .displays
                    .entry(view)
                    .or_insert_with(|| self.display.clone());
                display.settings.drawing.matcap = choice;
                display.settings.active.style = Style::Reflection;
            }
        }
        self.status = if choice.missing() {
            format!("Matcap not found: {}", choice.label())
        } else {
            "Ready".into()
        };
    }

    /// `view3d.<name> <value|?> [bgmesh:0|1]`: one Viewport Properties
    /// setting of the active view (UIC-163/165). False when `text` isn't
    /// one. `view3d.showVertices` is the Vertex panel's (vertex_panel.rs).
    pub(super) fn view3d_canonical(&mut self, text: &str) -> bool {
        // The Matcap style's command (an Operandi extra).
        if self.view3d_matcap(text) {
            return true;
        }
        let Ok(args) = command::tokens(text) else {
            return false;
        };
        let Some(first) = args.first() else {
            return false;
        };
        let Some(&(command, row_name)) = VIEW3D.iter().find(|(c, _)| c.eq_ignore_ascii_case(first))
        else {
            return false;
        };
        let mut value: Option<&str> = None;
        let mut inactive = false;
        for arg in &args[1..] {
            match arg.split_once(':') {
                Some((k, v)) if k.eq_ignore_ascii_case("bgmesh") => {
                    inactive = matches!(v, "1" | "true" | "on")
                }
                Some((_, v)) if value.is_none() => value = Some(v),
                None if value.is_none() => value = Some(arg.as_str()),
                _ => {
                    self.status = format!("Use {command} <value|?> [bgmesh:0|1]");
                    return true;
                }
            }
        }
        let Some(value) = value else {
            self.status = format!("Use {command} <value|?> [bgmesh:0|1]");
            return true;
        };
        let k = if GROUPED.contains(&row_name) {
            key::group(!inactive, row_name)
        } else {
            row_name.to_owned()
        };
        let camera =
            camera_light_ui::view_camera(&self.editor.scene).is_some() || self.camera.through;
        let rows = view3d_rows();
        let Some(row) = rows.iter().find(|r| r.key == k) else {
            self.status = format!("{command} has no such setting here");
            return true;
        };
        let view = (!self.camera.through).then_some(self.camera.view);
        let mut values = panel_values(&rows, &self.display.settings, view, camera);
        if value == "?" {
            self.status = values
                .get(&k)
                .map(|v| value_text(row, row_name, v))
                .unwrap_or_default();
            return true;
        }
        match parse_value(row, row_name, value) {
            Ok(v) => {
                values.set(&k, v);
                if let Some(view) = apply_value(&mut self.display.settings, &values, &k, camera) {
                    self.choose_view_type(view);
                }
                self.status = "Ready".into();
            }
            Err(e) => self.status = e,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{box_app, draw, key, pointer};
    use viewport_buttons::Corner;

    const SIZE: Vec2 = vec2(1440., 900.);

    fn click(a: &mut Operandi, ctx: &Context, at: Pos2) {
        draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
        draw(a, ctx, SIZE, vec![pointer(at, true, Modifiers::NONE)]);
        draw(a, ctx, SIZE, vec![pointer(at, false, Modifiers::NONE)]);
        draw(a, ctx, SIZE, vec![]);
    }

    fn settle(a: &mut Operandi, ctx: &Context) {
        for _ in 0..3 {
            draw(a, ctx, SIZE, vec![]);
        }
    }

    /// operandi.view3d.matcap sets the active view's matcap by
    /// built-in name or image file, `?` reads it back, and the Matcap rows
    /// run it (UIC-164/165).
    #[test]
    fn matcap_command_sets_and_reads_the_views_matcap() {
        use crate::matcap::Choice;
        let mut a = box_app();
        assert_eq!(a.display.settings.drawing.matcap, Choice::default());
        a.execute("operandi.view3d.matcap name:fullmetal");
        assert_eq!(a.display.settings.drawing.matcap, Choice::BuiltIn(6));
        a.execute("operandi.view3d.matcap ?");
        assert_eq!(a.status, "name:fullmetal");
        a.execute("operandi.view3d.matcap name:{Ceramic Lightbulb} bgmesh:1");
        assert_eq!(a.display.settings.drawing.matcap, Choice::BuiltIn(4));
        a.execute("operandi.view3d.matcap name:nope");
        assert!(a.status.starts_with("No matcap nope"), "{}", a.status);
        assert_eq!(a.display.settings.drawing.matcap, Choice::BuiltIn(4));
        // A missing file changes nothing; a real one becomes the matcap.
        a.execute("operandi.view3d.matcap name:custom image:/no/such/matcap.png");
        assert_eq!(a.display.settings.drawing.matcap, Choice::BuiltIn(4));
        let dir = std::env::temp_dir().join(format!("operandi matcap {}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("ball.png");
        image::RgbImage::from_pixel(8, 8, image::Rgb([10, 200, 10]))
            .save(&file)
            .unwrap();
        a.execute(&format!(
            "operandi.view3d.matcap name:custom image:{{{}}}",
            file.display()
        ));
        assert_eq!(a.display.settings.drawing.matcap, Choice::custom(&file));
        a.execute("operandi.view3d.matcap ?");
        assert_eq!(
            a.status,
            format!("name:custom image:{{{}}}", file.display())
        );
        std::fs::remove_dir_all(dir).ok();
        // The rows' lines.
        let mut values = Values::default();
        values.set(key::MATCAP, Value::Choice(2));
        values.set(key::MATCAP_IMAGE, Value::Text(String::new()));
        assert_eq!(
            matcap_line(&values, key::MATCAP).as_deref(),
            Some("operandi.view3d.matcap name:pearl")
        );
        assert_eq!(matcap_line(&values, key::MATCAP_IMAGE), None);
        assert_eq!(
            matcap_line(&values, key::MATCAP_LOAD).as_deref(),
            Some("operandi.view3d.matcap name:custom")
        );
        assert!(operandi_core::command_registry::find_line("operandi.view3d.matcap ?").is_some());
        // MC-05: a library matcap sits in a Library submenu by its folder
        // and runs as a custom image by its path.
        let library = vec![operandi_core::preset::library::FoundMatcap {
            group: "Metal".into(),
            name: "Gold".into(),
            path: "/lib/Matcaps/Metal/Gold.png".into(),
        }];
        let names = matcap_names_in(&library);
        let n = crate::matcap::BUILT_IN.len();
        assert_eq!(names[n], "Library ▸ Metal ▸ Gold");
        assert_eq!(names[n + 1], "Custom Image");
        values.set(key::MATCAP, Value::Choice(n));
        assert_eq!(
            matcap_line_in(&values, key::MATCAP, &library).as_deref(),
            Some("operandi.view3d.matcap name:custom image:/lib/Matcaps/Metal/Gold.png")
        );
        // MC-06: all:1 sets every 3D view (a bad value is refused).
        a.execute("operandi.view3d.matcap name:pearl all:maybe");
        assert!(a.status.contains("all"), "{}", a.status);
    }

    /// Reflection is called Matcap now (Jason, 2026-11-10); scripts that
    /// say Reflection keep working.
    #[test]
    fn matcap_style_answers_to_its_old_name() {
        let mut a = box_app();
        a.execute("view3d.shadingStyle Reflection");
        assert_eq!(
            a.display.settings.active.style,
            Style::Reflection,
            "{}",
            a.status
        );
        a.execute("view3d.shadingStyle ?");
        assert_eq!(a.status, "Matcap");
        a.execute("view3d.shadingStyle Shaded");
        a.execute("view3d.shadingStyle Matcap");
        assert_eq!(a.display.settings.active.style, Style::Reflection);
        assert_eq!(Style::Reflection.label(), "Matcap");
    }

    fn two_views() -> (Operandi, Context) {
        let mut a = box_app();
        a.camera.view = View::Perspective;
        let ctx = Context::default();
        a.split_view();
        // Split View makes the new (right) view active; these checks start
        // from the first.
        a.activate_view(0);
        a.camera.view = View::Top;
        settle(&mut a, &ctx);
        (a, ctx)
    }

    fn press_o(a: &mut Operandi, ctx: &Context, over: Pos2) {
        let o = |pressed| Event::Key {
            key: Key::O,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        draw(
            a,
            ctx,
            SIZE,
            vec![Event::PointerMoved(over), o(true), o(false)],
        );
        draw(a, ctx, SIZE, vec![]);
    }

    fn title(a: &Operandi, out: &egui::FullOutput) -> Option<String> {
        let _ = a;
        out.shapes.iter().find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text.starts_with("Viewport Properties:") => {
                Some(t.galley.job.text.clone())
            }
            _ => None,
        })
    }

    #[test]
    fn o_opens_near_the_pointer_titled_for_its_view_and_leaving_closes_it() {
        // VP-10..19, VP-10..19, VP-10..19.
        let (mut a, ctx) = two_views();
        let over = a.corner.areas[&0].center();
        press_o(&mut a, &ctx, over);
        assert_eq!(a.corner.properties.view, Some(0));
        let out = draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(title(&a, &out).as_deref(), Some("Viewport Properties: Top"));
        let panel = a.corner.popover_rect.expect("shown");
        assert!(panel.contains(over), "the pointer starts inside: {panel:?}");
        assert!((panel.left() - (over.x - NEAR_POINTER.x)).abs() < 1.);
        // Within the 8-point margin it stays; further out it closes, no click.
        let edge = pos2(panel.right() + 5., panel.center().y);
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(edge)]);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(a.corner.properties.view, Some(0));
        let away = pos2(panel.right() + 40., panel.center().y);
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(away)]);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(a.corner.properties.view, None);
    }

    #[test]
    fn o_near_the_bottom_right_corner_nudges_the_panel_on_screen() {
        let (mut a, ctx) = two_views();
        let corner = a.corner.areas[&1].right_bottom() - vec2(20., 20.);
        press_o(&mut a, &ctx, corner);
        draw(&mut a, &ctx, SIZE, vec![]);
        let panel = a.corner.popover_rect.expect("shown");
        assert!(
            panel.right() <= SIZE.x + 0.5 && panel.bottom() <= SIZE.y + 0.5,
            "{panel:?}"
        );
    }

    #[test]
    fn the_gear_hangs_it_under_itself_and_esc_closes_it() {
        let (mut a, ctx) = two_views();
        let gear = a.corner.rects[&(0, Corner::Properties)];
        click(&mut a, &ctx, gear.center());
        assert_eq!(a.corner.properties.view, Some(0));
        let panel = a.corner.popover_rect.expect("shown");
        assert!(panel.top() >= gear.bottom() - 1., "under the gear");
        assert!((panel.right() - gear.right()).abs() < 12.);
        draw(&mut a, &ctx, SIZE, vec![key(Key::Escape)]);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(a.corner.properties.view, None);
        // The gear again (twice) opens and closes it.
        click(&mut a, &ctx, gear.center());
        assert_eq!(a.corner.properties.view, Some(0));
        click(&mut a, &ctx, gear.center());
        assert_eq!(a.corner.properties.view, None);
    }

    #[test]
    fn pinned_it_stays_drags_retargets_and_comes_back_after_relaunch() {
        // VP-10..19.
        let (mut a, ctx) = two_views();
        let over = a.corner.areas[&0].center();
        press_o(&mut a, &ctx, over);
        a.corner.properties.tab = Tab::Inactive;
        settle(&mut a, &ctx);
        let panel = a.corner.popover_rect.expect("shown");
        let pin = pos2(panel.right() - 8. - 9., panel.top() + 8. + 10.);
        click(&mut a, &ctx, pin);
        assert!(a.corner.properties.pinned, "the circle pins it");
        // Far away, it stays, and the view gets the keys again.
        let away = a.corner.areas[&1].center();
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(away)]);
        draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(a.corner.properties.view, Some(0));
        assert!(!a.corner.properties.owns_input());
        // Drag the title bar.
        let grab = pos2(panel.center().x - 60., panel.top() + 8. + 10.);
        let before = a.corner.popover_rect.unwrap().min;
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(grab)]);
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![pointer(grab, true, Modifiers::NONE)],
        );
        for dy in [10., 30., 60.] {
            draw(
                &mut a,
                &ctx,
                SIZE,
                vec![Event::PointerMoved(grab + vec2(-20., dy))],
            );
        }
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![pointer(grab + vec2(-20., 60.), false, Modifiers::NONE)],
        );
        draw(&mut a, &ctx, SIZE, vec![]);
        let moved = a.corner.popover_rect.unwrap().min;
        assert!(
            (moved - before - vec2(-20., 60.)).length() < 2.,
            "{before:?} → {moved:?}"
        );
        // O over the other view retargets the one panel, which stays put.
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(away)]);
        press_o(&mut a, &ctx, away);
        let out = draw(&mut a, &ctx, SIZE, vec![]);
        assert_eq!(a.corner.properties.view, Some(1));
        assert_eq!(
            title(&a, &out).as_deref(),
            Some("Viewport Properties: Perspective")
        );
        assert!((a.corner.popover_rect.unwrap().min - moved).length() < 1.);
        // Relaunch: pinned, same place, same tab, same view.
        a.persist_layout(&ctx);
        let mut b = box_app();
        b.environment = a.environment.clone();
        b.start_layouts();
        b.load_view_properties();
        assert!(b.corner.properties.pinned);
        assert_eq!(b.corner.properties.view, Some(1));
        assert_eq!(b.corner.properties.tab, Tab::Inactive);
        let at = b.corner.properties.at.unwrap();
        assert!((at - moved).length() < 1., "{at:?} {moved:?}");
        // The circle on a pinned panel closes it.
        let panel = a.corner.popover_rect.unwrap();
        let pin = pos2(panel.right() - 8. - 9., panel.top() + 8. + 10.);
        click(&mut a, &ctx, pin);
        assert_eq!(a.corner.properties.view, None);
        assert!(!a.corner.properties.pinned);
    }

    #[test]
    fn closing_the_target_view_closes_even_a_pinned_panel() {
        let (mut a, ctx) = two_views();
        a.toggle_view_properties(1, None);
        a.corner.properties.pinned = true;
        a.corner.properties.at = Some(pos2(300., 200.));
        settle(&mut a, &ctx);
        assert_eq!(a.corner.properties.view, Some(1));
        a.close_view(1);
        settle(&mut a, &ctx);
        assert_eq!(a.corner.properties.view, None);
        assert!(!a.corner.properties.pinned);
    }

    #[test]
    fn saved_folds_from_before_the_pin_load() {
        let old = r#"["Visibility/Item Visibility"]"#;
        let state = SavedState::parse(old).unwrap();
        assert!(state.collapsed.contains("Visibility/Item Visibility"));
        assert_eq!((state.tab, state.pinned), (Tab::Drawing, None));
        let new = SavedState {
            tab: Tab::Active,
            pinned: Some(Pinned {
                view: 2,
                at: [10., 20.],
            }),
            ..state
        };
        let text = serde_json::to_string(&new).unwrap();
        assert_eq!(SavedState::parse(&text), Some(new));
    }

    /// Settings of the view, through the panel's rows, as a user edits them.
    fn edit(settings: &mut view_display::Settings, k: &str, value: Value) {
        let tabs = panel_tabs(false, &[]);
        let rows = all_rows(&tabs);
        let mut values = panel_values(&rows, settings, Some(View::Top), false);
        values.set(k, value);
        apply_value(settings, &values, k, false);
    }

    /// Whether row `k` responds for these settings.
    fn enabled(settings: &view_display::Settings, k: &str) -> bool {
        let tabs = panel_tabs(false, &[]);
        let rows = all_rows(&tabs);
        let values = panel_values(&rows, settings, Some(View::Top), false);
        values.enabled(rows.iter().find(|r| r.key == k).unwrap())
    }

    #[test]
    fn the_header_style_is_active_shading_style() {
        // VP-30..34: the same setting both ways.
        let mut d = view_display::ViewDisplay::default();
        d.set_style(Style::Cel, false);
        let tabs = panel_tabs(false, &[]);
        let values = panel_values(&all_rows(&tabs), &d.settings, None, false);
        assert_eq!(values.choice("active.style"), style_index(Style::Cel));
        edit(
            &mut d.settings,
            "active.style",
            Value::Choice(style_index(Style::Gooch)),
        );
        assert_eq!(d.menu_label(), "Gooch");
        assert_eq!(
            d.settings.inactive.style,
            Style::Default,
            "inactive untouched"
        );
    }

    #[test]
    fn overrides_grey_the_inactive_tab_and_invisible_wins() {
        let mut s = view_display::Settings::default();
        for k in [
            "inactive.style",
            "inactive.overlay",
            "inactive.wire_color",
            "inactive.smooth_shade",
        ] {
            assert!(enabled(&s, k), "{k}");
        }
        edit(&mut s, key::SAME_AS_ACTIVE, Value::Bool(true));
        for k in [
            "inactive.style",
            "inactive.overlay",
            "inactive.opacity",
            "inactive.wire_color",
            "inactive.smooth_shade",
        ] {
            assert!(!enabled(&s, k), "{k} greyed");
        }
        assert!(enabled(&s, key::INVISIBLE), "the other override stays live");
        edit(&mut s, key::INVISIBLE, Value::Bool(true));
        let d = view_display::ViewDisplay {
            settings: s,
            applied: None,
        };
        assert_eq!(d.group(false), None, "Invisible wins over Same as Active");
        // Wireframe greys the overlay rows.
        let mut s = view_display::Settings::default();
        assert!(enabled(&s, "active.overlay") && enabled(&s, "active.opacity"));
        edit(
            &mut s,
            "active.style",
            Value::Choice(style_index(Style::Wireframe)),
        );
        assert!(!enabled(&s, "active.overlay") && !enabled(&s, "active.opacity"));
        assert!(enabled(&s, "active.wire_color"));
    }

    #[test]
    fn wire_colours_follow_the_theme_until_edited() {
        let mut s = view_display::Settings::default();
        let tabs = panel_tabs(false, &[]);
        let values = panel_values(&all_rows(&tabs), &s, None, false);
        assert_eq!(
            encoded(values.colour("inactive.wire_color")),
            rgb(theme::viewport::inactive_wireframe())
        );
        assert_eq!(
            encoded(values.colour("active.wire_color")),
            rgb(theme::viewport::wireframe())
        );
        assert_eq!(s.inactive.wire_color, None);
        edit(
            &mut s,
            "inactive.wire_color",
            Value::Colour(linear([255, 0, 0])),
        );
        assert_eq!(s.inactive.wire_color, Some([255, 0, 0]));
        assert_eq!(s.active.wire_color, None);
        for c in [[0, 0, 0], [1, 2, 3], [128, 64, 200], [255, 255, 255]] {
            assert_eq!(encoded(linear(c)), c, "round trip");
        }
    }

    #[test]
    fn panel_rows_edit_only_the_target_views_settings_and_never_undo() {
        // VP-20..24.
        let (mut a, ctx) = two_views();
        let other = a.corner.areas[&1].center();
        press_o(&mut a, &ctx, other);
        a.corner.properties.tab = Tab::Visibility;
        settle(&mut a, &ctx);
        let history = a.editor.undo.len();
        let cameras = a.displays[&1].settings.drawing.cameras;
        let row = a.corner.properties.rows[key::CAMERAS];
        click(&mut a, &ctx, row.center());
        assert_eq!(a.corner.properties.view, Some(1), "stays open");
        assert_eq!(a.displays[&1].settings.drawing.cameras, !cameras);
        assert_eq!(
            a.display.settings.drawing.cameras, cameras,
            "view 0 kept its own"
        );
        assert_eq!(a.editor.undo.len(), history, "never an undo step");
        assert_eq!(a.active_view, 0, "the panel's view isn't made active");
    }

    #[test]
    fn last_selected_mesh_stays_active_and_every_selected_one_is_active() {
        // VP-04/05, VP-T8.
        let mut a = box_app();
        a.display.settings.inactive.style = Style::Wireframe;
        let first = a.editor.scene.items[0].id;
        // A scene where no mesh was ever selected: every mesh inactive.
        a.editor.scene.selection.item = None;
        a.editor.scene.selection.also.clear();
        a.editor.scene.last_mesh = Default::default();
        assert_eq!(a.item_look(first).map(|g| g.style), Some(Style::Wireframe));
        a.editor.scene.selection.item = Some(first);
        assert_eq!(a.item_look(first).map(|g| g.style), Some(Style::Default));
        // Deselect everything: the last selected mesh keeps the active look.
        a.editor.dispatch(Command::SelectItem(Some(first))).unwrap();
        a.editor.dispatch(Command::SelectItem(None)).unwrap();
        assert_eq!(a.editor.scene.selection.item, None);
        assert_eq!(a.item_look(first).map(|g| g.style), Some(Style::Default));
        // An item selected after the first one is active too.
        a.editor.scene.selection.item = Some(first + 1000);
        a.editor.scene.selection.also = vec![first];
        assert_eq!(a.item_look(first).map(|g| g.style), Some(Style::Default));
        // Whatever the selection mode.
        a.editor.scene.selection.mode = operandi_core::Mode::Vertex;
        assert_eq!(a.item_look(first).map(|g| g.style), Some(Style::Default));
    }

    #[test]
    fn controls_off_still_lets_o_open_it() {
        let (mut a, ctx) = two_views();
        a.displays.get_mut(&1).unwrap().settings.drawing.controls = false;
        settle(&mut a, &ctx);
        assert!(!a.corner.rects.keys().any(|(v, _)| *v == 1), "no buttons");
        let other = a.corner.areas[&1].center();
        press_o(&mut a, &ctx, other);
        assert_eq!(a.corner.properties.view, Some(1));
    }

    #[test]
    fn one_column_two_tabs_built_and_nothing_overlaps() {
        // VP-40..46.
        let (mut a, ctx) = two_views();
        a.toggle_view_properties(0, None);
        for tab in Tab::ALL {
            a.corner.properties.tab = tab;
            settle(&mut a, &ctx);
            let panel = a.corner.popover_rect.expect("shown");
            assert!((panel.width() - POPOVER_WIDTH).abs() < 30., "{panel:?}");
            let rows = &a.corner.properties.rows;
            let strip = rows[&format!("tab {}", Tab::Active.label())];
            let form: Vec<(&String, &Rect)> = rows
                .iter()
                .filter(|(k, _)| !k.starts_with("tab ") && !k.contains('/') && !k.ends_with('⏷'))
                .collect();
            assert!(!form.is_empty(), "{tab:?}");
            for (i, (name, rect)) in form.iter().enumerate() {
                assert!(panel.contains_rect(**rect), "{name} {rect:?} in {panel:?}");
                assert!(
                    rect.right() <= strip.left() + 0.5,
                    "{name} clear of the tabs"
                );
                for (other, r) in &form[i + 1..] {
                    assert!(rect.intersect(**r).area() < 1., "{name} overlaps {other}");
                }
            }
            let tabs: Vec<Rect> = Tab::ALL
                .iter()
                .map(|t| rows[&format!("tab {}", t.label())])
                .collect();
            for pair in tabs.windows(2) {
                assert!(pair[1].top() >= pair[0].bottom() - 0.5);
            }
        }
    }

    #[test]
    fn folded_sections_and_the_last_tab_are_remembered() {
        let (mut a, ctx) = two_views();
        a.toggle_view_properties(0, None);
        a.corner.properties.tab = Tab::Visibility;
        settle(&mut a, &ctx);
        let key = "Visibility/Item Visibility";
        let bar = a.corner.properties.rows[key];
        click(&mut a, &ctx, bar.center());
        assert!(a.corner.properties.collapsed.contains(key));
        assert!(
            !a.corner.properties.rows.contains_key(key::LIGHTS),
            "folded"
        );
        a.close_view_properties();
        settle(&mut a, &ctx);
        let mut b = box_app();
        b.environment = a.environment.clone();
        b.load_view_properties();
        assert!(b.corner.properties.collapsed.contains(key));
        assert_eq!(b.corner.properties.tab, Tab::Visibility);
    }

    #[test]
    fn split_copies_the_views_panel_settings() {
        let mut a = box_app();
        let ctx = Context::default();
        settle(&mut a, &ctx);
        a.display.settings.inactive.style = Style::Wireframe;
        a.display.settings.inactive.wire_color = Some([200, 0, 0]);
        a.display.settings.drawing.inactive_invisible = true;
        a.split_view();
        settle(&mut a, &ctx);
        assert_eq!(a.displays[&1].settings, a.displays[&0].settings);
        assert_eq!(
            a.displays[&1].settings.inactive.wire_color,
            Some([200, 0, 0])
        );
    }

    #[test]
    fn the_overlay_actions_set_the_active_meshes_overlay() {
        let mut a = box_app();
        for (name, overlay) in [
            ("wireframe overlay none", Overlay::None),
            ("wireframe overlay colored", Overlay::Colored),
            ("wireframe overlay uniform", Overlay::Uniform),
        ] {
            a.execute(name);
            assert_eq!(a.display.settings.active.overlay, overlay);
        }
        // The inactive group keeps its own (VS-6 default: Colored).
        assert_eq!(a.display.settings.inactive.overlay, Overlay::Colored);
    }

    #[test]
    fn a_saved_default_starts_new_views() {
        let mut a = box_app();
        let mut settings = view_display::Settings::default();
        settings.drawing.background = view_display::Background::Gradient;
        a.corner.properties.presets.default = Some(settings);
        assert_eq!(
            a.new_view_display().settings.drawing.background,
            view_display::Background::Gradient
        );
    }

    #[test]
    fn settings_saved_before_the_new_rows_load_with_their_defaults() {
        let old = r#"{"grid":true,"item_mode_wireframe":false,"show_verts":false,
            "show_vertices":"Never","uv_seams":false,"smoothing":true,
            "inactive_same_as_active":false,"inactive_invisible":false}"#;
        let d: view_display::Drawing = serde_json::from_str(old).unwrap();
        assert!(d.controls && d.meshes && d.rollovers);
        assert_eq!(d.background, view_display::Background::None);
        // A whole layout's settings from before the wire colours.
        let view = r#"{"0":{"drawing":{"grid":true,"item_mode_wireframe":false,"show_verts":false,
            "show_vertices":"Never","uv_seams":false,"smoothing":true,
            "inactive_same_as_active":false,"inactive_invisible":false},
            "active":{"style":"Shaded","overlay":"Uniform","opacity":100,"smoothing":true},
            "inactive":{"style":"Default","overlay":"None","opacity":100,"smoothing":true}}}"#;
        let s: BTreeMap<u32, view_display::Settings> = serde_json::from_str(view).unwrap();
        assert_eq!(s[&0].active.opacity, 100, "saved values kept");
        assert_eq!(s[&0].inactive.wire_color, None);
    }

    #[test]
    fn visibility_rows_edit_the_views_settings() {
        // Spec v2.1 section 8 (P2).
        let mut s = view_display::Settings::default();
        let d = s.drawing;
        assert!(d.locators && d.faded_grid && d.grid_axes && d.selections);
        assert!(d.selected_filling && d.selected_outline && d.rollovers);
        assert!(!d.texture_locators && !d.scene_lighting && !d.selection_normals);
        for k in [
            key::LOCATORS,
            key::TEXTURE_LOCATORS,
            key::SCENE_LIGHTING,
            key::FADED_GRID,
            key::GRID_AXES,
            key::SELECTIONS,
            key::SELECTION_NORMALS,
            key::SELECTED_FILLING,
            key::SELECTED_OUTLINE,
        ] {
            let before = s;
            let on = {
                let tabs = panel_tabs(false, &[]);
                panel_values(&all_rows(&tabs), &s, Some(View::Top), false).bool(k)
            };
            edit(&mut s, k, Value::Bool(!on));
            assert_ne!(s, before, "{k} changed the settings");
            let tabs = panel_tabs(false, &[]);
            let values = panel_values(&all_rows(&tabs), &s, Some(View::Top), false);
            assert_eq!(values.bool(k), !on, "{k} reads back");
        }
    }

    #[test]
    fn show_grid_and_show_work_plane_draw_one_grid_by_default() {
        let mut s = view_display::Settings::default();
        let d = s.drawing;
        // Show Work Plane defaults to Always: the app-wide Show Work Plane
        // switch (off by default) decides whether it draws at all.
        assert!(d.work_plane_grid(false) && d.work_plane_grid(true));
        // At rest: the floor grid. Moved: the work plane's grid takes over,
        // but only when it is actually drawn.
        assert!(d.floor_grid(false, true) && d.floor_grid(false, false));
        assert!(
            !d.floor_grid(true, true),
            "the drawn work plane replaces it"
        );
        assert!(d.floor_grid(true, false), "work plane off keeps the floor");
        let tabs = panel_tabs(false, &[]);
        let values = panel_values(&all_rows(&tabs), &s, Some(View::Top), false);
        assert_eq!(
            names(&SHOW_GRID)[values.choice(key::GRID)],
            "Inactive Work Plane"
        );
        assert_eq!(
            names(&SHOW_WORK_PLANE)[values.choice(key::WORK_PLANE)],
            "Always"
        );
        // Active: only once moved.
        edit(&mut s, key::WORK_PLANE, Value::Choice(1));
        assert!(!s.drawing.work_plane_grid(false) && s.drawing.work_plane_grid(true));
        // Show Grid Always: the floor grid whatever the work plane does.
        edit(&mut s, key::GRID, Value::Choice(0));
        edit(&mut s, key::WORK_PLANE, Value::Choice(0));
        assert!(s.drawing.floor_grid(true, true) && s.drawing.work_plane_grid(false));
        // Never / Never: none.
        edit(&mut s, key::GRID, Value::Choice(2));
        edit(&mut s, key::WORK_PLANE, Value::Choice(2));
        assert!(!s.drawing.grid && !s.drawing.floor_grid(false, false));
        assert!(!s.drawing.work_plane_grid(true));
        // The Display menu's grid toggle is the same setting: back on, it
        // follows the work plane again.
        s.drawing.grid = true;
        let values = panel_values(&all_rows(&tabs), &s, Some(View::Top), false);
        assert_eq!(
            names(&SHOW_GRID)[values.choice(key::GRID)],
            "Inactive Work Plane"
        );
    }

    #[test]
    fn show_selections_off_greys_the_four_rows_under_it() {
        // VP-T10.
        let mut s = view_display::Settings::default();
        let under = [
            key::SELECTION_NORMALS,
            key::SELECTED_FILLING,
            key::SELECTED_OUTLINE,
            key::ROLLOVERS,
        ];
        assert!(under.iter().all(|k| enabled(&s, k)));
        edit(&mut s, key::SELECTIONS, Value::Bool(false));
        assert!(under.iter().all(|k| !enabled(&s, k)));
        assert!(!s.drawing.shows_rollovers(), "no pre-highlight either");
    }

    #[test]
    fn faded_grid_reaches_the_grid_shader() {
        let a = box_app();
        let r = Rect::from_min_size(Pos2::ZERO, vec2(400., 300.));
        let make = |fade| {
            crate::grid::PlaneGrid::new(
                &a.camera,
                r,
                crate::grid::Display {
                    plane: 0,
                    center: DVec3::ZERO,
                    axes: false,
                    workplane: false,
                    visible: true,
                    opacity: 1.,
                    zero_axes: false,
                    fade,
                },
                1.,
            )
            .data
        };
        assert_eq!(make(true)[41], 1.);
        assert_eq!(make(false)[41], 0.);
    }

    #[test]
    fn advanced_ambient_reaches_the_look() {
        // P5: Advanced Options > Ambient Intensity and Ambient Color.
        let mut d = view_display::ViewDisplay::default();
        let [look, _] = crate::gpu::Look::pair(&d, operandi_core::Mode::Polygon);
        assert_eq!(look.ambient, [0.44; 3], "44 % white by default (VS-3)");
        edit(&mut d.settings, key::AMBIENT_INTENSITY, Value::Number(100.));
        edit(
            &mut d.settings,
            key::AMBIENT_COLOR,
            Value::Colour([1., 0., 0.]),
        );
        assert_eq!(d.settings.drawing.ambient_intensity, 100);
        assert_eq!(d.settings.drawing.ambient_color, [255, 0, 0]);
        let [look, inactive] = crate::gpu::Look::pair(&d, operandi_core::Mode::Polygon);
        assert_eq!(look.ambient, [1., 0., 0.]);
        assert_eq!(inactive.ambient, [1., 0., 0.], "the whole view");
    }

    #[test]
    fn independent_point_size_sets_this_views_dots() {
        let mut a = box_app();
        let prefs = [
            a.selection_preferences.point_radius(),
            a.selection_preferences.selected_point_radius(),
        ];
        assert_eq!(a.point_radii(), prefs);
        assert!(!enabled(&a.display.settings, key::POINT_SIZE), "greyed");
        edit(
            &mut a.display.settings,
            key::INDEPENDENT_POINT_SIZE,
            Value::Bool(true),
        );
        assert!(enabled(&a.display.settings, key::POINT_SIZE));
        edit(&mut a.display.settings, key::POINT_SIZE, Value::Number(12.));
        let [r, selected] = a.point_radii();
        assert_eq!(r, 12. * 2.5 / 3.);
        assert_eq!(selected, r * a.selection_preferences.selected_point_scale);
    }

    #[test]
    fn transparency_overrides_rows_and_greying() {
        // VP-80.
        let mut s = view_display::Settings::default();
        assert!(!s.active.transparency.on());
        assert!(enabled(&s, "active.t_mode"));
        for k in [
            "active.t_front_color",
            "active.t_back_opacity",
            "active.t_overlay",
        ] {
            assert!(!enabled(&s, k), "{k} greyed while Normal");
        }
        edit(&mut s, "active.t_mode", Value::Choice(1));
        assert_eq!(s.active.transparency.mode, TransparencyMode::Ghost);
        for k in [
            "active.t_front_color",
            "active.t_front_opacity",
            "active.t_back_color",
            "active.t_flat",
        ] {
            assert!(enabled(&s, k), "{k}");
        }
        // X-Ray: the front polygons are clear, so their rows grey.
        edit(&mut s, "active.t_mode", Value::Choice(2));
        assert!(!enabled(&s, "active.t_front_opacity"));
        assert!(enabled(&s, "active.t_back_opacity"));
        edit(&mut s, "active.t_back_opacity", Value::Number(40.));
        edit(&mut s, "active.t_overlay", Value::Bool(true));
        assert_eq!(s.active.transparency.back_opacity, 40);
        assert!(s.active.transparency.overlay);
        assert!(!s.inactive.transparency.on(), "the inactive group's own");
        // Not with the Wireframe style.
        edit(
            &mut s,
            "active.style",
            Value::Choice(style_index(Style::Wireframe)),
        );
        assert!(!enabled(&s, "active.t_mode"));
        let d = view_display::ViewDisplay {
            settings: s,
            ..Default::default()
        };
        let [look, _] = crate::gpu::Look::pair(&d, operandi_core::Mode::Polygon);
        assert!(!look.see_through());
        // Overlay is on the Active Meshes tab only.
        let tabs = panel_tabs(false, &[]);
        assert!(
            all_rows(&tabs)
                .iter()
                .all(|r| r.key != "inactive.t_overlay")
        );
    }

    #[test]
    fn a_see_through_look_carries_its_rows() {
        let mut d = view_display::ViewDisplay::default();
        d.settings.active.transparency.mode = TransparencyMode::Ghost;
        d.settings.active.transparency.flat = true;
        let [look, inactive] = crate::gpu::Look::pair(&d, operandi_core::Mode::Polygon);
        assert!(look.see_through() && !inactive.see_through());
        let rows = crate::gpu::transparency_rows(&look.transparency);
        assert_eq!(rows[0][3], 0.25, "front opacity");
        assert_eq!(rows[1][3], 0.6, "back opacity");
        assert_eq!(rows[2], [1., 1., 0., 0.], "Ghost, flat, no overlay");
        // Saved settings without the field load as Normal, and Normal isn't
        // written.
        let text = serde_json::to_string(&view_display::Group::default()).unwrap();
        assert!(!text.contains("transparency"));
    }
}
