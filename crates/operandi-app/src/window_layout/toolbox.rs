//! Bounded toolbox amendment: persistent transforms, exclusive categories and
//! an extensible four-column primitive grid. No scene/model state is owned here.
use super::*;

/// D-08, UIC-197: the set selects toolbox forms independently of docking.
/// Picking a set never changes the layout (Jason, 2026-11-23): no view
/// opens, closes or moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToolSet {
    Model,
    Uv,
    /// Sculpting and painting (was "Paint"; the old name still reads).
    Brush,
    Setup,
    /// One form, no strip: the Animation tab (Jason, 2026-11-23).
    Animation,
    Workbench,
    Game,
}

impl ToolSet {
    /// The toolset bar's order (Jason, 2026-11-23).
    pub(super) const ALL: &[Self] = &[
        Self::Model,
        Self::Uv,
        Self::Brush,
        Self::Setup,
        Self::Animation,
        Self::Workbench,
        Self::Game,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Model => "Model",
            Self::Uv => "UV",
            Self::Brush => "Brush",
            Self::Setup => "Setup",
            Self::Animation => "Animation",
            Self::Workbench => "Workbench",
            Self::Game => "Game",
        }
    }

    /// The set a command line or saved layout names; the old "Paint" set
    /// is Brush now.
    pub(super) fn named(name: &str) -> Option<Self> {
        let name = name.trim();
        if name.eq_ignore_ascii_case("Paint") {
            return Some(Self::Brush);
        }
        Self::ALL
            .iter()
            .copied()
            .find(|set| set.label().eq_ignore_ascii_case(name))
    }

    /// The set's toolbox tabs, in strip order (Jason, 2026-11-23). A set
    /// with one tab draws no strip.
    pub(super) fn tabs(self) -> Vec<Category> {
        // The Rig tab waits for the animation switch.
        let rig = crate::anim_switch::enabled().then_some(Category::Rig);
        match self {
            Self::Model => vec![
                Category::Create,
                Category::Select,
                Category::Deform,
                Category::Duplicate,
                Category::Edit,
                Category::Vertex,
                Category::Edge,
                Category::Polygon,
                Category::Topology,
                Category::Curve,
            ],
            Self::Uv => vec![Category::Uv],
            #[cfg(feature = "painting")]
            Self::Brush => vec![
                Category::Sculpt,
                Category::Paint,
                Category::VertexMaps,
                Category::PaintUtilities,
            ],
            #[cfg(not(feature = "painting"))]
            Self::Brush => vec![Category::Sculpt],
            // Deformers, Controls and Rename Properties are left out until
            // commands exist to fill them.
            Self::Setup => [
                Category::Setup,
                Category::Constraints,
                Category::InverseKinematics,
                Category::Weighting,
            ]
            .into_iter()
            .chain(rig)
            .collect(),
            Self::Animation => vec![Category::Animation],
            Self::Workbench => vec![Category::Workbench],
            // Presets is left out: there are no game presets yet.
            Self::Game => [Category::GameVertexMap, Category::Baking]
                .into_iter()
                .chain(rig)
                .collect(),
        }
    }

    pub(super) fn first_category(self) -> Option<Category> {
        self.tabs().first().copied()
    }

    /// The first set, in bar order, with `tab` among its tabs.
    pub(super) fn with_tab(tab: Category) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|set| set.tabs().contains(&tab))
    }
}

impl Operandi {
    pub(super) fn select_tool_set(&mut self, set: ToolSet) {
        self.layouts.tool_set = set;
        if let Some(first) = set.first_category() {
            self.category = first;
        }
        self.layouts.uv_tab = set == ToolSet::Uv;
    }

    /// Open toolbox tab `tab`: in the current set when it has it, else in
    /// the first set that does. False when no set has it.
    pub(super) fn show_toolbox_tab(&mut self, tab: Category) -> bool {
        if !self.layouts.tool_set.tabs().contains(&tab) {
            let Some(set) = ToolSet::with_tab(tab) else {
                return false;
            };
            self.layouts.tool_set = set;
        }
        self.layouts.uv_tab = tab == Category::Uv;
        self.category = tab;
        true
    }

    /// A tab opened by setting `category` directly (a paint tool, a test)
    /// brings up the first set that has it. A set without tabs keeps its
    /// empty body.
    fn follow_tab_set(&mut self) {
        let tabs = self.layouts.tool_set.tabs();
        if tabs.is_empty() || tabs.contains(&self.category) {
            return;
        }
        if let Some(set) = ToolSet::with_tab(self.category) {
            self.layouts.tool_set = set;
        }
    }

    /// The tab the toolbox body draws: the current one when its set has
    /// it, else the set's first (none for a set without tabs).
    pub(super) fn shown_category(&self) -> Option<Category> {
        let tabs = self.layouts.tool_set.tabs();
        if tabs.contains(&self.category) {
            Some(self.category)
        } else {
            tabs.first().copied()
        }
    }
}

/// Toolbox sizes (Jason, 2026-11-01: icons were far too small next to the
/// reference). Rows are tall enough for a near-full-height icon; grid cells
/// keep their height and draw the icon larger.
/// Default Form Icon Size (Preferences > Defaults > UI) scales these:
/// Small 0.8×, Normal (as sized above), Large 1.25×.
pub(super) fn row_height() -> f32 {
    sized(30.)
}
/// Text-only rows (commands without an icon) stay compact.
pub(super) const TEXT_ROW_HEIGHT: f32 = 22.;
pub(super) fn row_icon() -> f32 {
    sized(28.)
}
pub(super) fn cell_height() -> f32 {
    sized(34.)
}
pub(super) fn cell_icon() -> f32 {
    sized(32.)
}
/// Where a row's text starts, just right of its icon.
fn row_text() -> f32 {
    row_icon() + 6.
}
thread_local! {
    static ICON_SCALE: std::cell::Cell<f32> = const { std::cell::Cell::new(1.) };
}
pub(super) fn set_icon_scale(scale: f32) {
    ICON_SCALE.with(|c| c.set(scale.clamp(0.5, 2.)));
}
fn sized(points: f32) -> f32 {
    (points * ICON_SCALE.with(std::cell::Cell::get)).round()
}
use theme::chrome;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Category {
    #[default]
    Create,
    /// Jason, 2026-11-04: selection commands, just below Create.
    Select,
    /// Deform presets, Push and Smooth (deform_panel.rs).
    Deform,
    /// Duplicate tab (DS-60): Lathe, Clone, Array, Radial Array, Mirror
    /// (duplicate_panel.rs).
    Duplicate,
    Polygon,
    Edge,
    Vertex,
    /// Edit tab (toolbox-panels-2026-11-06): the Slice tools for now.
    Edit,
    /// Topology tab (TOOLBOX-TOPOLOGY-CURVE TB-10, topology_tab.rs).
    Topology,
    /// Curve tab (TB-20): the curve tools and commands (curve_tab.rs,
    /// `assets/forms/toolbox.curve.json`).
    Curve,
    /// Sculpt tab (SC-100): brushes, tips, Lock Borders/Corners.
    Sculpt,
    /// UV tab (UVT-201, uv_editor.rs): Map, the transform tools, Unwrap,
    /// Pack and the island buttons; the UV layout opens at it.
    Uv,
    /// Rig tab (UIC-190..201; phases 4-6): setup, skeleton and pose,
    /// constraints, links, skin and IK (`assets/forms/toolbox.rig.json`).
    /// Shows in the Animate layout, with the animation switch on.
    Rig,
    /// The user's own workbench (layouts stage 4, LAY-82): an empty form
    /// they fill with Edit Workbench (workbench.rs).
    Workbench,
    /// The Setup set's tabs (Jason, 2026-11-23): item setup, constraints,
    /// inverse kinematics and weighting, from existing commands
    /// (`toolbox.setup*.json`).
    Setup,
    Constraints,
    InverseKinematics,
    Weighting,
    /// The Game set's Vertex Map and Baking tabs (`toolbox.game.*.json`).
    GameVertexMap,
    Baking,
    /// The Animation set's one form (`toolbox.animation.json`).
    Animation,
    /// Paint Tools (painting builds only, PNT-85).
    #[cfg(feature = "painting")]
    Paint,
    /// The Sculpt/Paint set's Vertex Map Tools tab (PNT-211).
    #[cfg(feature = "painting")]
    VertexMaps,
    /// The Sculpt/Paint set's Utilities tab (PNT-212).
    #[cfg(feature = "painting")]
    PaintUtilities,
}
impl Category {
    #[cfg(not(feature = "painting"))]
    pub(super) const ALL: [Self; 21] = [
        Self::Create,
        Self::Select,
        Self::Deform,
        Self::Duplicate,
        Self::Polygon,
        Self::Edge,
        Self::Vertex,
        Self::Edit,
        Self::Topology,
        Self::Curve,
        Self::Sculpt,
        Self::Uv,
        Self::Rig,
        Self::Workbench,
        Self::Setup,
        Self::Constraints,
        Self::InverseKinematics,
        Self::Weighting,
        Self::GameVertexMap,
        Self::Baking,
        Self::Animation,
    ];
    #[cfg(feature = "painting")]
    pub(super) const ALL: [Self; 24] = [
        Self::Create,
        Self::Select,
        Self::Deform,
        Self::Duplicate,
        Self::Polygon,
        Self::Edge,
        Self::Vertex,
        Self::Edit,
        Self::Topology,
        Self::Curve,
        Self::Sculpt,
        Self::Paint,
        Self::Uv,
        Self::Rig,
        Self::Workbench,
        Self::VertexMaps,
        Self::PaintUtilities,
        Self::Setup,
        Self::Constraints,
        Self::InverseKinematics,
        Self::Weighting,
        Self::GameVertexMap,
        Self::Baking,
        Self::Animation,
    ];
    /// The tab a command line names (`operandi.toolbox.tab uv`).
    pub(super) fn named(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|c| c.label().eq_ignore_ascii_case(name.trim()))
    }
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Create => "Create",
            Self::Select => "Select",
            Self::Deform => "Deform",
            Self::Duplicate => "Duplicate",
            Self::Polygon => "Polygon",
            Self::Edge => "Edge",
            Self::Vertex => "Vertex",
            Self::Edit => "Edit",
            Self::Topology => "Topology",
            Self::Curve => "Curve",
            Self::Sculpt => "Sculpt",
            Self::Uv => "UV",
            Self::Rig => "Rig",
            Self::Workbench => "Workbench",
            Self::Setup => "Setup",
            Self::Constraints => "Constraints",
            Self::InverseKinematics => "Inverse Kinematics",
            Self::Weighting => "Weighting",
            Self::GameVertexMap => "Vertex Map",
            Self::Baking => "Baking",
            Self::Animation => "Animation",
            #[cfg(feature = "painting")]
            Self::Paint => "Paint",
            #[cfg(feature = "painting")]
            Self::VertexMaps => "Vertex Maps",
            #[cfg(feature = "painting")]
            Self::PaintUtilities => "Utilities",
        }
    }
}
// PAR-014a–d: appearance choices calibrated at actual scale, not a conversion
// of the handoff's approximate screenshot pixels. Preserve the old grid band.
const CATEGORY_HIT_WIDTH: f32 = 22.;
/// The message line's strip under Tool Properties.
pub(crate) const MESSAGE_LINE_HEIGHT: f32 = 18.;
pub(super) const DOCK_PANEL_INSET: f32 = 4.;
pub(super) const HORIZONTAL_TOOL_TABS_HEIGHT: f32 = CATEGORY_HIT_WIDTH + 2. * DOCK_PANEL_INSET;
/// A docked vertical strip is as wide as its tabs plus padding, like the
/// horizontal one is one row high (Jason, 2026-11-19).
pub(super) const VERTICAL_TOOL_TABS_WIDTH: f32 = CATEGORY_HIT_WIDTH + 2. * DOCK_PANEL_INSET;
const CATEGORY_BAND: f32 = 22.;
const CATEGORY_FONT_SIZE: f32 = crate::typography::STANDARD_SIZE;

struct CategoryTabs {
    entries: Vec<(Category, f32)>,
    gaps: f32,
    labels: f32,
    compact: f32,
    natural: f32,
}

fn category_color(active: bool) -> Color32 {
    if active {
        chrome::text()
    } else {
        chrome::disabled_text()
    }
}

/// Tabs on the right: each tab rounds its two outer (right) corners.
pub(super) fn category_corners(_index: usize, _count: usize) -> CornerRadius {
    CornerRadius {
        ne: 3,
        se: 3,
        ..CornerRadius::ZERO
    }
}

/// Keep the native-scale glyph atlas and clockwise quarter turn. Center the ink
/// bounds (rather than ascent/descent whitespace), then snap the final origin to
/// physical pixels. No scaled galley or label texture.
pub(super) fn category_text(
    ui: &Ui,
    label: &str,
    rect: Rect,
    color: Color32,
) -> egui::epaint::TextShape {
    let galley = ui.painter().layout_no_wrap(
        label.into(),
        FontId::proportional(CATEGORY_FONT_SIZE),
        color,
    );
    let ink = galley.mesh_bounds;
    let rotated = Rect::from_min_max(pos2(-ink.max.y, ink.min.x), pos2(-ink.min.y, ink.max.x));
    let origin = rect.center() - rotated.center().to_vec2();
    let scale = ui.ctx().pixels_per_point();
    let origin = pos2(
        (origin.x * scale).round() / scale,
        (origin.y * scale).round() / scale,
    );
    egui::epaint::TextShape::new(origin, galley, color).with_angle(std::f32::consts::FRAC_PI_2)
}

/// What a form's tool cell shows ([`Operandi::tool_cell`]).
pub(super) struct ToolFace<'a> {
    pub active: bool,
    /// Accessibility name, and the drawn text of a row.
    pub label: &'a str,
    /// A corner tag ("Item", "Unit") while a variant shows.
    pub tag: Option<&'a str>,
    /// Input-map action whose binding is drawn.
    pub binding: Option<&'a str>,
}

/// A small tag in a cell's top-left corner (the primitives' "Item" and
/// "Unit" while Shift or Ctrl shows that variant).
pub(super) fn paint_corner_tag(ui: &Ui, rect: Rect, tag: &str, active: bool) {
    ui.painter().text(
        rect.left_top() + vec2(2., 1.),
        Align2::LEFT_TOP,
        tag,
        FontId::proportional(9.),
        if active {
            chrome::active_text()
        } else {
            chrome::text()
        },
    );
}

/// How a [`CommandButton`] arranges its content.
#[derive(Clone, Copy)]
pub(super) enum ButtonStyle {
    /// Grid cell: centred 24-point icon; shortcut in the bottom-right corner.
    Cell(icons::Icon),
    /// Row: 20-point icon at the left, then the label; shortcut at the right.
    IconRow(icons::Icon),
    /// Row with the label only; shortcut at the right.
    TextRow,
    /// Label centred; no shortcut.
    Centered,
    /// Row with a picture from disk at the left (a workbench button's
    /// custom icon, LAY-82), then the label; shortcut at the right.
    Picture(TextureId),
    /// Grid cell with a picture from disk centred (an icon-only workbench
    /// button, provisional LAY-Q54); shortcut in the bottom-right corner.
    PictureCell(TextureId),
}

/// A toolbox button for anything that is not a [`Tool`] (commands, dialogs,
/// click modes and the numeric Cone/Ellipsoid/Spike/Bridge/Reduction
/// sessions). [`Operandi::toolbox_button`] draws the `Tool` buttons.
pub(super) struct CommandButton<'a> {
    /// Key in `toolbox_rects`; tests find the button by it.
    pub key: &'a str,
    /// Drawn text (not drawn for `Cell`) and accessibility name.
    pub label: &'a str,
    pub style: ButtonStyle,
    pub size: Vec2,
    /// Lit amber while its session, dialog or click mode is open.
    pub active: bool,
    /// Greyed out and unclickable, with this reason on hover, when set.
    pub unavailable: Option<&'a str>,
    /// Input-map action whose binding is drawn on the button.
    pub shortcut: Option<&'a str>,
}

impl<'a> CommandButton<'a> {
    /// A full-width row (taller with an icon), enabled and inactive, showing the binding of
    /// the action named `key`. Adjust fields with struct update syntax.
    pub(super) fn row(ui: &Ui, key: &'a str, label: &'a str, style: ButtonStyle) -> Self {
        Self {
            key,
            label,
            style,
            // Only rows with an icon need the taller height.
            size: vec2(
                ui.available_width(),
                if matches!(style, ButtonStyle::IconRow(_)) {
                    row_height()
                } else {
                    TEXT_ROW_HEIGHT
                },
            ),
            active: false,
            unavailable: None,
            shortcut: Some(key),
        }
    }
}

impl Operandi {
    /// Draw a [`CommandButton`] and return its response. The caller handles
    /// clicks and sets the tooltip.
    pub(super) fn command_button(&mut self, ui: &mut Ui, button: CommandButton) -> Response {
        let enabled = button.unavailable.is_none();
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(button.size, sense);
        let response = crate::unavailable::explain(response, button.unavailable);
        #[cfg(test)]
        crate::ui_inventory::button("toolbox", button.key);
        self.toolbox_rects.insert(button.key.into(), rect);
        chrome::own_activation(ui, &response);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, button.label));
        let (fill, text) = if enabled {
            (
                chrome::button_fill(&response, button.active),
                chrome::text(),
            )
        } else {
            (chrome::control(), chrome::disabled_text())
        };
        let painter = ui.painter();
        painter.rect_filled(rect, 2., fill);
        let font = FontId::proportional(crate::typography::STANDARD_SIZE);
        let row_label = |x: f32| {
            // A longer variant label is cut, never widening the row (MB).
            let label =
                crate::button_variants::fit_label(ui, button.label, &font, rect.width() - x - 4.);
            painter.text(
                pos2(rect.left() + x, rect.center().y),
                Align2::LEFT_CENTER,
                label,
                font.clone(),
                text,
            );
        };
        // A disabled row's icon fades with its label.
        let paint_icon = if enabled {
            icons::paint
        } else {
            icons::paint_faded
        };
        let row_icon = |icon| {
            paint_icon(
                ui,
                Rect::from_center_size(
                    pos2(rect.left() + 2. + row_icon() / 2., rect.center().y),
                    vec2(row_icon(), row_icon()),
                ),
                icon,
            );
        };
        match button.style {
            ButtonStyle::Cell(icon) => {
                paint_icon(
                    ui,
                    Rect::from_center_size(rect.center(), vec2(cell_icon(), cell_icon())),
                    icon,
                );
            }
            ButtonStyle::IconRow(icon) => {
                row_icon(icon);
                row_label(row_text());
            }
            ButtonStyle::TextRow => row_label(8.),
            ButtonStyle::Picture(texture) | ButtonStyle::PictureCell(texture) => {
                let cell = matches!(button.style, ButtonStyle::PictureCell(_));
                let at = if cell {
                    let side = cell_icon().min(rect.height() - 4.).min(rect.width() - 4.);
                    Rect::from_center_size(rect.center(), vec2(side, side))
                } else {
                    Rect::from_center_size(
                        pos2(
                            rect.left() + 2. + crate::toolbox::row_icon() / 2.,
                            rect.center().y,
                        ),
                        vec2(crate::toolbox::row_icon(), crate::toolbox::row_icon()),
                    )
                };
                let tint = if enabled {
                    Color32::WHITE
                } else {
                    Color32::from_white_alpha(90)
                };
                egui::Image::new((texture, at.size()))
                    .tint(tint)
                    .paint_at(ui, at);
                if !cell {
                    row_label(row_text());
                }
            }
            ButtonStyle::Centered => {
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    button.label,
                    font,
                    text,
                );
            }
        }
        let binding = button
            .shortcut
            .and_then(|action| self.bindings.get(action))
            .filter(|b| !b.is_empty());
        if let Some(binding) = binding {
            let (at, align) = match button.style {
                ButtonStyle::Cell(_) | ButtonStyle::PictureCell(_) => {
                    (rect.right_bottom() - vec2(3., 2.), Align2::RIGHT_BOTTOM)
                }
                ButtonStyle::IconRow(_) | ButtonStyle::TextRow | ButtonStyle::Picture(_) => {
                    (rect.right_center() - vec2(4., 0.), Align2::RIGHT_CENTER)
                }
                ButtonStyle::Centered => return response,
            };
            painter.text(
                at,
                align,
                crate::input::display_chord(binding),
                FontId::proportional(9.),
                text,
            );
        }
        response
    }

    pub(super) fn toolbox_button(&mut self, ui: &mut Ui, tool: Tool, size: Vec2, icon_only: bool) {
        if self.tool_button(ui, tool, size, icon_only, None) {
            self.activate(tool);
        }
    }

    /// Draws a tool's button; true when it was clicked (the caller starts
    /// the tool). `lit` is the tool command's own answer (UIC-223) when a
    /// form draws it; `None` reads the live tool here.
    pub(super) fn tool_button(
        &mut self,
        ui: &mut Ui,
        tool: Tool,
        size: Vec2,
        icon_only: bool,
        lit: Option<bool>,
    ) -> bool {
        let active = lit.unwrap_or_else(|| {
            self.editor
                .session
                .live
                .as_ref()
                .is_some_and(|l| l.tool == tool)
        });
        let face = ToolFace {
            active,
            label: tool.label(),
            tag: None,
            binding: Some(tool.label()),
        };
        let response = self.tool_cell(ui, tool, size, icon_only, face);
        let clicked = response.clicked();
        response.on_hover_text(crate::ui_text::hover_text(&format!(
            "{} · {}. Click again to drop. Shortcut: {}",
            tool.label(),
            tool_info::info(tool).tooltip,
            self.bindings
                .get(tool.label())
                .filter(|s| !s.is_empty())
                .map(String::as_str)
                .unwrap_or("unassigned"),
        )));
        clicked
    }

    /// A tool's cell: the bordered tool button showing `face`. The caller
    /// runs the click and sets the tooltip (batch 7: a form's Cube, Sphere
    /// and Cylinder wear their held variant).
    pub(super) fn tool_cell(
        &mut self,
        ui: &mut Ui,
        tool: Tool,
        size: Vec2,
        icon_only: bool,
        face: ToolFace,
    ) -> Response {
        let active = face.active;
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        #[cfg(test)]
        crate::ui_inventory::button("toolbox", tool.label());
        self.toolbox_rects.insert(tool.label().into(), rect);
        chrome::own_activation(ui, &response);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, face.label));
        let fill = chrome::button_fill(&response, active);
        ui.painter().rect_filled(rect, 2., fill);
        ui.painter().rect_stroke(
            rect,
            2.,
            Stroke::new(1.0f32, chrome::border()),
            StrokeKind::Inside,
        );
        let icon_rect = if icon_only {
            Rect::from_center_size(rect.center(), vec2(cell_icon(), cell_icon()))
        } else {
            Rect::from_center_size(
                pos2(rect.left() + 2. + row_icon() / 2., rect.center().y),
                vec2(row_icon(), row_icon()),
            )
        };
        icons::paint(ui, icon_rect, tool_info::info(tool).icon);
        if let Some(tag) = face.tag {
            paint_corner_tag(ui, rect, tag, active);
        }
        let text = if active && !response.is_pointer_button_down_on() {
            chrome::active_text()
        } else {
            chrome::text()
        };
        if !icon_only {
            ui.painter().text(
                rect.left_center() + vec2(row_text(), 0.),
                Align2::LEFT_CENTER,
                face.label,
                crate::typography::standard_font(),
                text,
            );
        }
        if let Some(binding) = face
            .binding
            .and_then(|action| self.bindings.get(action))
            .filter(|s| !s.is_empty())
        {
            ui.painter().text(
                rect.right_bottom() - vec2(3., 2.),
                Align2::RIGHT_BOTTOM,
                crate::input::display_chord(binding),
                FontId::proportional(if icon_only {
                    9.
                } else {
                    crate::typography::STANDARD_SIZE
                }),
                text,
            );
        }
        chrome::focus(ui, rect, &response);
        response
    }

    pub(super) fn toolbox(&mut self, ui: &mut Ui) {
        chrome::apply(ui);
        self.help_region(ui.max_rect(), crate::help_mode::TOOLBOX);
        self.follow_tab_set();
        let mut panel = ui.max_rect().expand(4.);
        let combined = !self.layouts.mixed_dock;
        let area = ui.available_rect_before_wrap();
        // Jason, 2026-11-01: a deliberate step away from the reference layout
        // (PAR-014). The rows shared by every tab (transforms, Mesh Cleanup,
        // New Mesh) span the full width at the top; the tabs sit on the LEFT,
        // spanning only the part they switch, and look like tabs: the active
        // one joins the content, the others are darker and set back.
        let top = ui
            .scope_builder(UiBuilder::new().max_rect(area), |ui| {
                // Shared by every tab, so it spans the full panel width.
                let width = (ui.available_width() - 6.) / 4.;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.;
                    for tool in [Tool::Transform, Tool::Move, Tool::Rotate, Tool::Scale] {
                        self.toolbox_button(ui, tool, vec2(width, cell_height()), true);
                    }
                });
                // Persistent text-only common modeling actions.
                ui.horizontal(|ui| {
                    let spacing = 2.;
                    ui.spacing_mut().item_spacing.x = spacing;
                    let half = (ui.available_width() - spacing) / 2.;
                    let cleanup = CommandButton {
                        size: vec2(half, TEXT_ROW_HEIGHT),
                        shortcut: None,
                        ..CommandButton::row(
                            ui,
                            "Mesh Cleanup",
                            "Mesh Cleanup...",
                            ButtonStyle::Centered,
                        )
                    };
                    let cleanup = self.command_button(ui, cleanup);
                    if cleanup.clicked() && !self.has_modal() {
                        self.open_mesh_cleanup();
                    }
                    cleanup.on_hover_text("Mesh Cleanup... · remove supported floating vertices");
                    let new_mesh = CommandButton {
                        size: vec2(half, TEXT_ROW_HEIGHT),
                        shortcut: None,
                        ..CommandButton::row(ui, "New Mesh", "New Mesh", ButtonStyle::Centered)
                    };
                    let new_mesh = self.command_button(ui, new_mesh);
                    if new_mesh.clicked() && !self.has_modal() {
                        self.run(Command::NewMesh);
                    }
                    let binding = self
                        .bindings
                        .get("New Mesh")
                        .filter(|b| !b.is_empty())
                        .map(|b| format!(" · {b} (Scenes focus)"))
                        .unwrap_or_default();
                    new_mesh.on_hover_text(format!("New Mesh{binding}"));
                });
            })
            .response
            .rect;
        let body = Rect::from_min_max(pos2(area.left(), top.bottom() + 4.), area.max);
        // A set with one tab (or none) draws no strip (Jason, 2026-11-23).
        let categories = (self.layouts.tool_tabs_horizontal.is_none())
            .then(|| self.tool_categories(ui))
            .filter(|tabs| tabs.entries.len() > 1);
        self.tool_list_min = categories
            .as_ref()
            .map_or(tool_split::LIST_MIN_HEIGHT, |tabs| {
                tabs.compact.max(tool_split::LIST_MIN_HEIGHT)
            });
        // The tool drawer on top and Tool Properties below, each scrolling
        // on its own, split by a divider the user drags (Jason, 2026-11-08).
        // The split doesn't depend on the tab, so every tab behaves alike.
        let active = Some(self.tool_properties_title()).filter(|t| *t != "Tool Properties");
        if tool_split::activated(&mut self.tool_split_seen, active) {
            self.tool_split = tool_split::take_over(self.tool_split);
        }
        // MN-08: a maximised Tool Properties fills the column; a maximised
        // tool drawer hides it.
        use operandi_layout::Pane;
        let props_height = if !combined || self.panel_maximized == Some(Pane::Toolbox) {
            0.
        } else if self.tool_properties_open && self.panel_maximized == Some(Pane::ToolProperties) {
            body.height()
        } else if self.tool_properties_open {
            let total = (body.height() - tool_split::DIVIDER_HEIGHT).max(0.);
            self.tool_split_total = total.max(1.);
            tool_split::props_height(total, self.tool_split, self.tool_list_min)
        } else {
            // Folded: just the header, at the bottom.
            self.toolbox_rects
                .get("Tool Properties")
                .map_or(row_height(), |r| r.height())
                + ui.spacing().item_spacing.y
        };
        let divider = if combined && self.tool_properties_open {
            tool_split::DIVIDER_HEIGHT
        } else {
            0.
        };
        let drawer_bottom = (body.bottom() - props_height - divider).max(body.top());
        if combined && !self.is_floating(floating::Panel::Toolbox) {
            panel.max.y = drawer_bottom;
        }
        self.panel_dot(
            ui,
            crate::panel_dot::Dot::Side(floating::Panel::Toolbox),
            panel,
            Some(crate::panel_dot::header_edge(panel, 16.)),
        );
        let mut list_rect = Rect::from_min_max(body.min, pos2(body.right(), drawer_bottom));
        let props_rect =
            Rect::from_min_max(pos2(body.left(), body.bottom() - props_height), body.max);
        if combined {
            for (pane, rect) in [
                (Pane::Toolbox, list_rect),
                (Pane::ToolProperties, props_rect),
            ] {
                if rect.height() > 1. {
                    self.side_panel_rects.insert(pane, rect);
                } else {
                    self.side_panel_rects.remove(&pane);
                }
            }
        }
        if let Some(categories) = &categories {
            // UIC-196: the stock projection keeps the original band and
            // tool positions. The projected tabs have their own hover grip.
            let tabs = Rect::from_min_max(
                pos2(body.right() - CATEGORY_HIT_WIDTH, body.top()),
                pos2(body.right(), drawer_bottom),
            );
            if !self.is_floating(floating::Panel::Toolbox) {
                self.panel_dot(
                    ui,
                    crate::panel_dot::Dot::ToolTabs,
                    tabs,
                    Some(crate::panel_dot::header_edge(tabs, 16.)),
                );
            }
            // Paint before the form so selection replaces the tools in this frame.
            self.vertical_tool_tabs(ui, tabs, categories);
            let scale = ui.ctx().pixels_per_point();
            list_rect.max.x = ((body.right() - CATEGORY_BAND) * scale).round() / scale;
            let edge = list_rect.right() + 0.5;
            let shown = self.shown_category().unwrap_or(self.category);
            let open = self
                .toolbox_rects
                .get(&format!(
                    "Category {} surface",
                    self.category_tab_label(shown)
                ))
                .copied()
                .unwrap_or(Rect::from_min_max(tabs.min, tabs.min));
            for (from, to) in [(body.top(), open.top()), (open.bottom(), tabs.bottom())] {
                if to > from {
                    ui.painter().line_segment(
                        [pos2(edge, from), pos2(edge, to)],
                        Stroke::new(1.0f32, chrome::border()),
                    );
                }
            }
        }
        if combined && self.tool_properties_open {
            // Full width, like Tool Properties below it.
            let line = Rect::from_min_max(
                pos2(body.left(), list_rect.bottom()),
                pos2(body.right(), props_rect.top()),
            );
            ui.painter().hline(
                line.x_range(),
                line.center().y,
                Stroke::new(1.0f32, theme::border()),
            );
            // The grab reaches EDGE_GRAB either side of the line, like the
            // side panels' edges.
            self.tool_divider_rect =
                Rect::from_center_size(line.center(), vec2(line.width(), 2. * layout::EDGE_GRAB));
            self.tool_divider_layer = Some(ui.layer_id());
        }
        // A child, not a scope: a row wider than the drawer (the UV form's
        // in the column layouts, no longer masked by the tab strip's band)
        // must not widen the column frame after frame.
        self.toolbox_rects
            .insert(crate::tool_tab_cycle::DRAWER.into(), list_rect);
        let mut list = ui.new_child(UiBuilder::new().id_salt("toolbox-list").max_rect(list_rect));
        list.set_clip_rect(list.clip_rect().intersect(list_rect));
        {
            let ui = &mut list;
            ScrollArea::vertical()
                .id_salt("toolbox-body")
                .auto_shrink([false, false])
                .max_height(list_rect.height())
                .show(ui, |ui| {
                    // Every tab is a form (layouts stage 3, toolbox_form.rs):
                    // each form draws its own section headers.
                    use crate::toolbox_form as forms;
                    // Every set has at least one tab now.
                    let Some(shown) = self.shown_category() else {
                        return;
                    };
                    let form = match shown {
                        Category::Create => forms::CREATE_FORM,
                        Category::Select => forms::SELECT_FORM,
                        Category::Deform => forms::DEFORM_FORM,
                        Category::Sculpt => forms::SCULPT_FORM,
                        Category::Duplicate => forms::DUPLICATE_FORM,
                        Category::Edit => forms::EDIT_FORM,
                        Category::Curve => forms::CURVE_FORM,
                        Category::Topology => forms::TOPOLOGY_FORM,
                        Category::Uv => forms::UV_FORM,
                        Category::Rig => forms::RIG_FORM,
                        Category::Vertex => forms::VERTEX_FORM,
                        Category::Edge => forms::EDGE_FORM,
                        Category::Polygon => forms::POLYGON_FORM,
                        Category::Workbench => crate::workbench::TOOLBOX_WORKBENCH,
                        Category::Setup => forms::SETUP_FORM,
                        Category::Constraints => forms::CONSTRAINTS_FORM,
                        Category::InverseKinematics => forms::IK_FORM,
                        Category::Weighting => forms::WEIGHTING_FORM,
                        Category::GameVertexMap => forms::GAME_VERTEX_MAP_FORM,
                        Category::Baking => forms::BAKING_FORM,
                        Category::Animation => forms::ANIMATION_FORM,
                        #[cfg(feature = "painting")]
                        Category::Paint => forms::PAINT_FORM,
                        #[cfg(feature = "painting")]
                        Category::VertexMaps => forms::VERTEX_MAPS_FORM,
                        #[cfg(feature = "painting")]
                        Category::PaintUtilities => forms::PAINT_UTILITIES_FORM,
                    };
                    self.draw_form(ui, form);
                });
        }
        if combined {
            self.tool_properties_panel(ui, props_rect, !self.is_floating(floating::Panel::Toolbox));
        }
    }

    fn tool_categories(&self, ui: &Ui) -> CategoryTabs {
        // The set decides the tabs (Jason, 2026-11-23).
        let categories = self.layouts.tool_set.tabs();
        let entries: Vec<_> = categories
            .into_iter()
            .map(|category| {
                let label = ui.painter().layout_no_wrap(
                    self.category_tab_label(category).into(),
                    FontId::proportional(CATEGORY_FONT_SIZE),
                    chrome::text(),
                );
                (category, label.size().x)
            })
            .collect();
        let gaps = 2. * entries.len().saturating_sub(1) as f32;
        let pixels = ui.ctx().pixels_per_point();
        let labels = entries.iter().map(|&(_, length)| length).sum();
        let compact = entries
            .iter()
            .map(|&(_, length)| ((length + 4.) * pixels).ceil() / pixels)
            .sum::<f32>()
            + gaps;
        let natural = entries
            .iter()
            .map(|&(_, length)| (length + 10.).max(46.))
            .sum::<f32>()
            + gaps;
        CategoryTabs {
            entries,
            gaps,
            labels,
            compact,
            natural,
        }
    }

    // UIC-100: retain the original rotated type, outline and active edge.
    fn vertical_tool_tabs(&mut self, ui: &mut Ui, tabs: Rect, categories: &CategoryTabs) {
        let pixels = ui.ctx().pixels_per_point();
        ui.scope_builder(UiBuilder::new().max_rect(tabs), |ui| {
            ui.spacing_mut().item_spacing.y = 2.;
            // Tabs are at least 46 long; when that doesn't fit the strip
            // (a short tool drawer) each is its label plus a little room.
            // At the smallest window the panel can be shorter than even
            // that (Linux's larger UI font, 2026-11-14): share whatever room
            // is left between the labels, down to none.
            let (pad, floor) = if categories.natural <= tabs.height() {
                (10., 46.)
            } else if categories.compact <= tabs.height() {
                (4., 0.)
            } else {
                let spare = tabs.height() - categories.gaps - categories.labels;
                // Less a physical pixel per tab for the rounding up below.
                let rounding = categories.entries.len() as f32 / pixels;
                (
                    ((spare - rounding) / categories.entries.len() as f32).clamp(0., 4.),
                    0.,
                )
            };
            for (index, (category, length)) in categories.entries.iter().copied().enumerate() {
                let label = self.category_tab_label(category);
                let active = self.shown_category() == Some(category);
                let height = (length + pad).max(floor);
                let scale = ui.ctx().pixels_per_point();
                let height = (height * scale).ceil() / scale;
                let (hit, response) =
                    ui.allocate_exact_size(vec2(CATEGORY_HIT_WIDTH, height), Sense::click());
                // Flush with the panel's outer edge. The active tab reaches the
                // content edge; the others are set back from it.
                let left = tabs.right() - CATEGORY_BAND + if active { 0. } else { 3. };
                let rect =
                    Rect::from_min_max(pos2(left, hit.top()), pos2(tabs.right(), hit.bottom()));
                let corners = category_corners(index, categories.entries.len());
                let fill = if active {
                    chrome::background()
                } else if response.hovered() {
                    chrome::hover()
                } else {
                    chrome::header()
                };
                ui.painter().rect_filled(rect, corners, fill);
                // Outline the outer three sides only, so the active tab opens
                // into the content; an orange edge marks it.
                let stroke = Stroke::new(1.0f32, chrome::border());
                let (ll, t, b, r) = (
                    rect.left(),
                    rect.top() + 0.5,
                    rect.bottom() - 0.5,
                    rect.right() - 0.5,
                );
                ui.painter().line_segment([pos2(ll, t), pos2(r, t)], stroke);
                ui.painter().line_segment([pos2(r, t), pos2(r, b)], stroke);
                ui.painter().line_segment([pos2(r, b), pos2(ll, b)], stroke);
                if active {
                    ui.painter().rect_filled(
                        Rect::from_min_max(pos2(rect.right() - 2., rect.top()), rect.max),
                        CornerRadius {
                            ne: 3,
                            se: 3,
                            ..CornerRadius::ZERO
                        },
                        chrome::active(),
                    );
                }
                let text = category_text(ui, label, rect, category_color(active));
                let ink = text.visual_bounding_rect();
                ui.painter().add(text);
                self.category_response(ui, category, response, hit, rect, ink);
            }
        });
    }

    // D-07 assigns orientation by dock side. UIC-71 and UIC-193 give the
    // standalone strip the same grip and floating behaviour as other panels.
    pub(super) fn tool_tabs_panel(&mut self, ui: &mut Ui, panel: Rect) {
        chrome::apply(ui);
        let area = ui.max_rect();
        self.help_region(area, crate::help_mode::TOOLBOX);
        let horizontal = self.layouts.tool_tabs_horizontal == Some(true);
        self.panel_dot(
            ui,
            crate::panel_dot::Dot::ToolTabs,
            panel,
            Some(crate::panel_dot::header_edge(
                panel,
                CATEGORY_HIT_WIDTH + DOCK_PANEL_INSET,
            )),
        );
        self.follow_tab_set();
        let categories = self.tool_categories(ui);
        if categories.entries.len() <= 1 {
            return;
        }
        if horizontal {
            let row = Rect::from_min_max(
                area.min,
                pos2(area.right(), area.top() + CATEGORY_HIT_WIDTH),
            );
            self.horizontal_tool_tabs(ui, row, &categories);
        } else {
            let strip = Rect::from_min_max(
                area.min,
                pos2(
                    (area.left() + CATEGORY_HIT_WIDTH).min(area.right()),
                    area.bottom(),
                ),
            );
            if strip.height() < categories.compact {
                let mut child = ui.new_child(UiBuilder::new().max_rect(strip));
                child.set_clip_rect(child.clip_rect().intersect(strip));
                ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(&mut child, |ui| {
                        let tabs = Rect::from_min_size(
                            ui.cursor().min,
                            vec2(strip.width(), categories.compact),
                        );
                        self.vertical_tool_tabs(ui, tabs, &categories);
                    });
            } else {
                self.vertical_tool_tabs(ui, strip, &categories);
            }
        }
    }

    fn horizontal_tool_tabs(&mut self, ui: &mut Ui, row: Rect, categories: &CategoryTabs) {
        let mut visible = row;
        let mut offset = 0.;
        if categories.natural > row.width() {
            let arrow_width = 16_f32.min(row.width() / 3.);
            visible.min.x += arrow_width;
            visible.max.x -= arrow_width;
            let limit = categories.natural - visible.width();
            let id = ui.id().with("tool-tabs-scroll");
            offset = ui
                .data(|data| data.get_temp::<f32>(id))
                .unwrap_or(0.)
                .clamp(0., limit);
            // A tab opened from the keyboard scrolls into sight
            // (tool_tab_cycle.rs).
            let reveal = ui.data_mut(|d| d.remove_temp::<bool>(crate::tool_tab_cycle::reveal_id()));
            if reveal == Some(true) {
                let mut start = 0.;
                for &(category, length) in &categories.entries {
                    let width = (length + 10.).max(46.);
                    if self.shown_category() == Some(category) {
                        offset = offset
                            .max(start + width - visible.width())
                            .min(start)
                            .clamp(0., limit);
                        break;
                    }
                    start += width + 2.;
                }
            }
            for (direction, glyph, label, rect) in [
                (
                    -1.,
                    "‹",
                    "Scroll tool tabs left",
                    Rect::from_min_max(row.min, pos2(visible.left(), row.bottom())),
                ),
                (
                    1.,
                    "›",
                    "Scroll tool tabs right",
                    Rect::from_min_max(pos2(visible.right(), row.top()), row.max),
                ),
            ] {
                let can_scroll = if direction < 0. {
                    offset > 0.
                } else {
                    offset < limit
                };
                let response = ui.put(
                    rect,
                    Button::new(glyph).sense(if can_scroll {
                        Sense::click()
                    } else {
                        Sense::hover()
                    }),
                );
                self.toolbox_rects.insert(label.into(), rect);
                if response.on_hover_text(label).clicked() {
                    offset = (offset + direction * visible.width() * 0.8).clamp(0., limit);
                }
            }
            ui.data_mut(|data| data.insert_temp(id, offset));
        }
        let mut child = ui.new_child(UiBuilder::new().id_salt("tool-tabs-row").max_rect(visible));
        child.set_clip_rect(child.clip_rect().intersect(visible));
        let mut left = visible.left() - offset;
        // D-07: explicit positions keep a single row. The clipped viewport
        // controls both painting and hit testing; only the arrows move it.
        for &(category, length) in &categories.entries {
            let width = (length + 10.).max(46.);
            let rect = Rect::from_min_size(pos2(left, row.top()), vec2(width, row.height()));
            left = rect.right() + 2.;
            let hit = rect.intersect(visible);
            if !hit.is_positive() {
                continue;
            }
            let label = self.category_tab_label(category);
            let response = child.interact(hit, child.id().with(label), Sense::click());
            let active = self.shown_category() == Some(category);
            let fill = if active {
                chrome::background()
            } else if response.hovered() {
                chrome::hover()
            } else {
                chrome::header()
            };
            child.painter().rect(
                rect,
                3.,
                fill,
                Stroke::new(1_f32, chrome::border()),
                StrokeKind::Inside,
            );
            // UIC-01/02: the active edge and neutral tab text match the strip.
            if active {
                child.painter().hline(
                    rect.x_range(),
                    rect.top() + 1.,
                    Stroke::new(2_f32, chrome::active()),
                );
            }
            let ink = child.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(CATEGORY_FONT_SIZE),
                category_color(active),
            );
            self.category_response(&child, category, response, hit, rect, ink);
        }
    }

    fn category_response(
        &mut self,
        ui: &Ui,
        category: Category,
        response: Response,
        hit: Rect,
        surface: Rect,
        ink: Rect,
    ) {
        let label = self.category_tab_label(category);
        let active = self.shown_category() == Some(category);
        self.toolbox_rects.insert(format!("Category {label}"), hit);
        self.toolbox_rects
            .insert(format!("Category {label} surface"), surface);
        self.toolbox_rects
            .insert(format!("Category {label} ink"), ink);
        chrome::own_activation(ui, &response);
        response
            .widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, active, label));
        chrome::focus(ui, surface, &response);
        if response.clicked() && self.confirm_fields() {
            self.category = category;
        }
        let keys = self.tool_tab_keys_hint();
        response.on_hover_text(format!("{label} category · changes the toolbox only{keys}"));
    }

    /// Whether the message line (the app's status text) sits in a thin
    /// strip at the bottom of Tool Properties (Jason, 2026-11-19) rather
    /// than under each view: whenever Tool Properties is docked in the main
    /// window and showing. Help text, which needs room, stays in the view.
    /// Provisional: only dock-tree layouts (every shipped layout now); an
    /// older fixed-column layout keeps the line under its views.
    pub(crate) fn message_line_in_tool_properties(&self) -> bool {
        if !self.layouts.mixed_dock || self.status.starts_with("Help:") {
            return false;
        }
        // The bottom mode bar's row always carries the message line, so the
        // views never show a second one (Jason, 2026-11-19: "Ready" twice
        // with the left column collapsed).
        if self.layouts.mode_bar_bottom {
            return true;
        }
        if self.layouts.left_collapsed {
            return false;
        }
        self.views
            .main
            .tiles
            .find_pane(&operandi_layout::Pane::ToolProperties)
            .is_some_and(|id| self.views.main.tiles.is_visible(id))
    }

    /// The thin message strip at the bottom of Tool Properties.
    pub(super) fn message_line(&mut self, ui: &mut Ui, strip: Rect) {
        ui.painter().hline(
            strip.x_range(),
            strip.top(),
            Stroke::new(1.0f32, theme::border()),
        );
        let mut child = ui.new_child(
            UiBuilder::new()
                .id_salt("message-line")
                .max_rect(strip.shrink2(vec2(4., 1.)))
                .layout(Layout::left_to_right(Align::Center)),
        );
        child.set_clip_rect(child.clip_rect().intersect(strip));
        let response = child.add(
            egui::Label::new(
                RichText::new(&self.status)
                    .size(11.)
                    .color(theme::weak_text()),
            )
            .truncate(),
        );
        self.layouts.message_line_rect = Some(response.rect);
    }

    pub(super) fn tool_properties_panel(&mut self, parent: &mut Ui, rect: Rect, docked: bool) {
        // With the mode bar at the bottom of a dock-tree layout, the
        // message line sits in that bottom row instead (views.rs).
        let in_band = self.layouts.mixed_dock && self.layouts.mode_bar_bottom;
        let rect = if docked
            && !in_band
            && self.layouts.message_line_here
            && !self.status.starts_with("Help:")
        {
            let strip = Rect::from_min_max(
                pos2(
                    rect.left(),
                    (rect.bottom() - MESSAGE_LINE_HEIGHT).max(rect.top()),
                ),
                rect.max,
            );
            self.message_line(parent, strip);
            Rect::from_min_max(rect.min, pos2(rect.right(), strip.top()))
        } else {
            rect
        };
        let parent_clip = parent.clip_rect();
        let mut child = parent.new_child(
            UiBuilder::new()
                .id_salt("tool-properties-panel")
                .max_rect(rect),
        );
        child.set_clip_rect(child.clip_rect().intersect(rect));
        let ui = &mut child;
        chrome::apply(ui);
        // UIC-71: Tool Properties has its own corner grip even in the stock
        // arrangement, where it still shares the toolbox's outer chrome.
        let title = self.tool_properties_title();
        let header = ui
            .scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                // Egui rounds the header button to its pixel grid.
                ui.set_clip_rect(parent_clip.intersect(rect.expand(1.)));
                chrome::disclosure(ui, "tool-properties", title, self.tool_properties_open)
            })
            .inner;
        if docked {
            let panel = if self.layouts.mixed_dock {
                rect.expand(DOCK_PANEL_INSET)
            } else {
                rect.expand2(vec2(DOCK_PANEL_INSET, 0.))
            };
            self.panel_dot(
                parent,
                crate::panel_dot::Dot::ToolProperties,
                panel,
                Some(crate::panel_dot::header_edge(
                    panel,
                    header.rect.bottom() - panel.top(),
                )),
            );
        }
        self.toolbox_rects
            .insert("Tool Properties".into(), header.rect);
        // Help mode (help_mode.rs): the header and an empty panel name
        // Tool Properties; the rows under it, the tool that fills them.
        self.help_region(rect, crate::help_mode::TOOL_PROPERTIES);
        let rows = Rect::from_min_max(pos2(rect.left(), header.rect.bottom()), rect.max);
        self.help_region(rows, crate::help_mode::tool_help_id(title));
        if header.clicked() && self.confirm_fields() {
            self.tool_properties_open = !self.tool_properties_open;
        }
        if self.tool_properties_open {
            ScrollArea::vertical()
                .id_salt("tool-property-body")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // GitHub #31: rows stop short of the scroll bar, as in
                    // Item Properties. The bar widens over the content
                    // when the pointer nears it and covered the fields'
                    // slide arrows on the right.
                    let room = crate::form::layout::scroll_bar_room(ui);
                    ui.set_max_width((ui.available_width() - room).max(0.));
                    crate::form::rows::scope(ui, "tool-properties", |ui| self.parameters(ui))
                });
        }
    }
}

impl Operandi {
    /// A tab's name in the strip: a toolbox set names its own (PNT-211).
    pub(super) fn category_tab_label(&self, c: Category) -> &'static str {
        #[cfg(feature = "painting")]
        if let Some(label) = self.paint_set_label(c) {
            return label;
        }
        c.label()
    }
}
