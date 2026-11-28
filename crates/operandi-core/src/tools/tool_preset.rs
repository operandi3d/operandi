//! Tool presets, stage 1 (specifications/tool-presets-2026-11-15): one
//! table of recipe rows. A row says which pipe layers to put in place
//! (action centre, action axis, falloff), then which tool to start with
//! which values. A new preset button is a new row in
//! `presets/factory-tool-presets.toml`, not new code.
//!
//! TP-10..13: starting a row remembers what the user had in every slot the
//! row names and puts the row's layer there; slots it doesn't name keep the
//! user's. When the row's tool is no longer live (Drop, Cancel, another
//! tool, Undo past its start) every remembered slot goes back, "empty"
//! included. Going from one row straight to another gives back the first
//! row's slots before the second takes them, so rows never stack.
//!
//! TP-14 (TP-Q1, proposed reading): a slot the user changes while a row is
//! open still goes back to the pre-row layer on drop.
//!
//! TP-Q2: Operandi tools already start from the values the user last
//! finished with (Lock off), and the factory rows keep that.
//!
//! TP-Q4: the rows are a readable TOML file compiled into the app and read
//! once at start, the format stage 2's user presets will use.
use crate::{
    ActionContext, AxisOrientation, CenterMode, Falloff, FalloffShape, LinearFalloff,
    RadialFalloff, Result, Scene, SelectionFalloff, SmoothShiftParameters, Tool,
};
use serde::Deserialize;
use std::sync::OnceLock;

const FACTORY_TOML: &str = include_str!("../../presets/factory-tool-presets.toml");

/// Bend's row (bend-handles-2026-11-15 BN-01): `center.auto + axis.auto +
/// xfrm.bend`. Starting Bend any way goes through it.
pub const BEND_ROW: &str = "xfrm.bend";

/// One recipe (TP-01..07).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PresetRow {
    /// The script name: `tool.set <id> on` starts it.
    pub id: String,
    pub label: String,
    /// More command-line phrases that start it (Operandi's older names).
    #[serde(default)]
    pub names: Vec<String>,
    #[serde(default)]
    pub tooltip: String,
    /// Icon slug from tools/operandi-icons.
    #[serde(default)]
    pub icon: String,
    /// Where the button sits, `tab.group`.
    #[serde(default)]
    pub group: String,
    /// In pipe order (TP-03).
    #[serde(default)]
    pub layers: Vec<Layer>,
    pub actor: Actor,
    /// CS-01/CS-05: whether picking the row makes its tool live at once
    /// (`true`) or only arms it until the first click in a view
    /// (`false`). Unset: the tool's own `[[tool]]` row, else on.
    #[serde(default)]
    pub auto_activate: Option<bool>,
}

/// CS-05: the Auto-Activate flag of a plain tool (one picked without a
/// preset row), by its enum name.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ToolFlag {
    pub tool: String,
    pub auto_activate: bool,
}

/// A filled pipe slot (spec section 3).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(tag = "slot", rename_all = "snake_case", deny_unknown_fields)]
pub enum Layer {
    ActionCenter {
        center: CenterName,
    },
    ActionAxis {
        axis: AxisName,
    },
    Falloff {
        kind: FalloffKind,
        #[serde(default)]
        shape: ShapeName,
        /// TP-19: the falloff starts where the tool is placed. A Radial or
        /// Cylinder falloff keeps its fitted size but its centre goes to the
        /// tool's centre on its first click (CS-02, after the click has
        /// placed the tool's centre), so the falloff and the transform handle begin at one
        /// spot and are then edited apart. Other kinds ignore it.
        #[serde(default)]
        at_tool: bool,
    },
    /// The row's own snapping (Weld: snap to points within `radius_px`
    /// screen points). It is the tool's own snap role, separate from the
    /// user's Snapping settings, so nothing is taken from the user or
    /// given back.
    Snapping {
        target: SnapTarget,
        radius_px: f32,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapTarget {
    Points,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CenterName {
    Automatic,
    Selection,
    SelectionBorder,
    Origin,
}
impl CenterName {
    pub fn mode(self) -> CenterMode {
        match self {
            Self::Automatic => CenterMode::Automatic,
            Self::Selection => CenterMode::Selection,
            Self::SelectionBorder => CenterMode::SelectionBorder,
            Self::Origin => CenterMode::Origin,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AxisName {
    World,
    /// Axes from the selected polygons. With no polygons selected the
    /// user's axis stays (SF-Q6, provisional).
    Selection,
    /// Selection axes in any component mode with something selected
    /// (Soft Selection family, hands-on 2026-11-17: the handle's Z runs
    /// along the selection's normal). Nothing selected: the user's axis.
    SelectionAll,
    /// Action Axis Auto (Bend, BN-01): the work plane's axes, with the
    /// tool's own X / Y / Z row choosing which one it uses. Provisional
    /// (BH-B01): Operandi has no separate Auto axis yet, so the layer sets
    /// World axes.
    Auto,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FalloffKind {
    Linear,
    Radial,
    Selection,
    /// Vortex (FO-42): fitted to the selection like Radial.
    Cylinder,
    /// Soft Selection family (FO-47): its starting values.
    SoftSelection,
    /// Element Push (FO-45): picks the element under the pointer.
    Element,
    /// Soft Drag (FO-43): the app centres the circle on each press.
    Screen,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShapeName {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    Smooth,
}
impl ShapeName {
    pub fn shape(self) -> FalloffShape {
        match self {
            Self::Linear => FalloffShape::Linear,
            Self::EaseIn => FalloffShape::EaseIn,
            Self::EaseOut => FalloffShape::EaseOut,
            Self::Smooth => FalloffShape::Smooth,
        }
    }
}

/// The tool a row starts, or the one-shot command it runs, and the values
/// that differ from the tool's own (TP-04). Values a tool doesn't have are
/// refused when the table is read.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    /// An Operandi tool, by its enum name (`Move`, `Rotate`, `Push` ...).
    #[serde(default)]
    pub tool: Option<String>,
    /// A one-shot command (`flip_it`): runs and finishes, no handles.
    #[serde(default)]
    pub operation: Option<String>,
    /// A mode or tool that lives in the app rather than the editor, by the
    /// command-line phrase that starts it (Weld, Quick Align).
    #[serde(default)]
    pub app: Option<String>,
    /// Rotate: each point turns by weight × Angle instead of blending
    /// positions (TW-Q1).
    #[serde(default)]
    pub angle_by_weight: bool,
    /// Axis Rotate (TC-30): Rotate with one ring, on the work plane's
    /// normal; the app shows only that ring.
    #[serde(default)]
    pub single_ring: bool,
    /// Topology Pen (TC-45): Element Move whose Shift+drag on an edge
    /// pulls out a new strip onto the background.
    #[serde(default)]
    pub topology_pen: bool,
    /// A Duplicate button's copier (DX-20 Transform, DX-30 Instance);
    /// unset makes ordinary copies.
    #[serde(default)]
    pub copier: Option<crate::duplicate_copier::Copier>,
    /// Smooth Shift / Thicken values.
    #[serde(default)]
    pub thicken: Option<bool>,
    #[serde(default)]
    pub shift: Option<f64>,
    #[serde(default)]
    pub scale: Option<f64>,
    /// Degrees.
    #[serde(default)]
    pub max_smoothing_angle: Option<f64>,
    /// An E7 UV button (operation `uv_island`, uv_edit.rs): which rule,
    /// and its axis and Align To where it has them.
    #[serde(default)]
    pub uv: Option<crate::uv_edit::IslandPreset>,
    /// A path tool button (CX-01): its curve kind and the values it
    /// switches (path_tools.rs).
    #[serde(default)]
    pub path: Option<crate::path_tools::PathPreset>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    preset: Vec<PresetRow>,
    #[serde(default)]
    tool: Vec<ToolFlag>,
}

impl PresetRow {
    /// The tool the row starts; `None` for a one-shot operation.
    pub fn tool(&self) -> Option<Tool> {
        let name = self.actor.tool.as_deref()?;
        Tool::ALL.into_iter().find(|t| format!("{t:?}") == name)
    }
    /// Every name that starts the row on the command line: its label, its
    /// older names and its id.
    pub fn phrases(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.label.as_str())
            .chain(self.names.iter().map(String::as_str))
            .chain(std::iter::once(self.id.as_str()))
    }
    /// The row's falloff layer fitted to the selection's bounding box (the
    /// whole mesh with nothing selected). Linear: along the box's longest
    /// axis, Start (full effect) at its high end (DF-Q3, provisional).
    /// Radial: centred on the box, half its extents. Selection: the
    /// registry's `falloff.selection` defaults.
    pub fn fitted_falloff(&self, scene: &Scene) -> Result<Option<Falloff>> {
        let Some((kind, shape)) = self.layers.iter().find_map(|l| match *l {
            Layer::Falloff { kind, shape, .. } => Some((kind, shape.shape())),
            _ => None,
        }) else {
            return Ok(None);
        };
        match kind {
            FalloffKind::Selection => {
                return Ok(Some(Falloff::Selection(SelectionFalloff::default())));
            }
            FalloffKind::SoftSelection => {
                return Ok(Some(Falloff::SoftSelection(Default::default())));
            }
            FalloffKind::Element => return Ok(Some(Falloff::Element(Default::default()))),
            FalloffKind::Screen => return Ok(Some(Falloff::Screen(Default::default()))),
            FalloffKind::Cylinder => {
                // FO-04 Auto Size, Axis Y (DP starting values).
                let c = crate::CylinderFalloff {
                    shape,
                    ..Default::default()
                };
                return Ok(Some(Falloff::Cylinder(c.auto_size(scene)?)));
            }
            FalloffKind::Linear | FalloffKind::Radial => {}
        }
        // FO-35: the fitted falloff has Use World Transforms off (factory),
        // so it fits in the active mesh's own space.
        let (lo, hi) = crate::falloff::selection_bounds_in(scene, false)?;
        let extent = hi - lo;
        let too_small = || {
            format!(
                "{} needs a selection with some size; select more points",
                self.label
            )
        };
        if kind == FalloffKind::Linear {
            let axis = if extent.x >= extent.y && extent.x >= extent.z {
                0
            } else if extent.y >= extent.z {
                1
            } else {
                2
            };
            if extent[axis] <= 0. {
                return Err(too_small());
            }
            let center = (lo + hi) / 2.;
            let (mut start, mut end) = (center, center);
            start[axis] = hi[axis];
            end[axis] = lo[axis];
            Ok(Some(Falloff::Linear(LinearFalloff {
                start,
                end,
                shape,
                ..Default::default()
            })))
        } else {
            if extent.max_element() <= 0. {
                return Err(too_small());
            }
            Ok(Some(Falloff::Radial(RadialFalloff {
                center: (lo + hi) / 2.,
                size: extent / 2.,
                shape,
                world: false,
            })))
        }
    }
    /// The action centre and axis the row puts in place over `current`;
    /// `None` when it names neither slot.
    pub fn action_context(&self, scene: &Scene, current: ActionContext) -> Option<ActionContext> {
        let mut out = current;
        let mut named = false;
        for layer in &self.layers {
            match *layer {
                Layer::ActionCenter { center } => {
                    out.center = center.mode();
                    named = true;
                }
                Layer::ActionAxis { axis } => {
                    named = true;
                    out.axis = match axis {
                        AxisName::World => AxisOrientation::World,
                        AxisName::Selection
                            if scene.selection.mode == crate::Mode::Polygon
                                && !scene.selection.faces.is_empty() =>
                        {
                            AxisOrientation::Selection
                        }
                        AxisName::Selection => current.axis,
                        AxisName::SelectionAll
                            if scene.selection.mode != crate::Mode::Item
                                && scene.selection.component_count() > 0 =>
                        {
                            AxisOrientation::Selection
                        }
                        AxisName::SelectionAll => current.axis,
                        AxisName::Auto => AxisOrientation::World,
                    };
                }
                Layer::Falloff { .. } | Layer::Snapping { .. } => {}
            }
        }
        named.then_some(out)
    }
    /// The kind of falloff the row brings, if any.
    pub fn falloff_kind(&self) -> Option<FalloffKind> {
        self.layers.iter().find_map(|l| match *l {
            Layer::Falloff { kind, .. } => Some(kind),
            _ => None,
        })
    }
    /// TP-19: whether the row's falloff starts at the tool's centre.
    pub fn falloff_at_tool(&self) -> bool {
        self.layers
            .iter()
            .any(|l| matches!(l, Layer::Falloff { at_tool: true, .. }))
    }
    pub fn names_slot(&self, slot: Slot) -> bool {
        self.layers.iter().any(|l| l.slot() == slot)
    }
    /// The row's Smooth Shift values over the tool's own (Thicken).
    pub fn smooth_shift(&self, base: SmoothShiftParameters) -> SmoothShiftParameters {
        let a = &self.actor;
        SmoothShiftParameters {
            thicken: a.thicken.unwrap_or(base.thicken),
            offset: a.shift.unwrap_or(base.offset),
            scale: a.scale.unwrap_or(base.scale),
            angle: a.max_smoothing_angle.unwrap_or(base.angle),
            ..base
        }
    }
}

/// The pipe slots a stage 1 row can fill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    ActionCenter,
    ActionAxis,
    Falloff,
    Snapping,
}
impl Layer {
    pub fn slot(&self) -> Slot {
        match self {
            Self::ActionCenter { .. } => Slot::ActionCenter,
            Self::ActionAxis { .. } => Slot::ActionAxis,
            Self::Falloff { .. } => Slot::Falloff,
            Self::Snapping { .. } => Slot::Snapping,
        }
    }
}

#[cfg(test)]
fn parse(text: &str) -> std::result::Result<Vec<PresetRow>, String> {
    parse_file(text).map(|f| f.preset)
}

fn parse_file(text: &str) -> std::result::Result<File, String> {
    let file: File = toml::from_str(text).map_err(|e| e.to_string())?;
    let mut tools = std::collections::BTreeSet::new();
    for flag in &file.tool {
        if !Tool::ALL.iter().any(|t| format!("{t:?}") == flag.tool) {
            return Err(format!("tool row {}: unknown tool", flag.tool));
        }
        if !tools.insert(flag.tool.clone()) {
            return Err(format!("tool row {} is listed twice", flag.tool));
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for row in &file.preset {
        if !seen.insert(row.id.to_ascii_lowercase()) {
            return Err(format!("preset id {} is listed twice", row.id));
        }
        match (&row.actor.tool, &row.actor.operation, &row.actor.app) {
            (Some(_), None, None) if row.tool().is_none() => {
                return Err(format!("preset {}: unknown tool", row.id));
            }
            (Some(_), None, None) | (None, None, Some(_)) => {}
            (None, Some(op), None) if op == "flip_it" => {}
            (None, Some(op), None) if op == "uv_island" => {
                if row.actor.uv.is_none() {
                    return Err(format!(
                        "preset {}: a uv_island row names its uv rule",
                        row.id
                    ));
                }
            }
            (None, Some(op), None) => {
                return Err(format!("preset {}: unknown operation {op}", row.id));
            }
            _ => {
                return Err(format!(
                    "preset {}: name one tool, operation or app mode",
                    row.id
                ));
            }
        }
        let mut slots = Vec::new();
        for layer in &row.layers {
            if slots.contains(&layer.slot()) {
                return Err(format!("preset {}: one layer per slot", row.id));
            }
            slots.push(layer.slot());
        }
    }
    Ok(file)
}

/// Every factory row, in file order.
pub fn factory() -> &'static [PresetRow] {
    &factory_file().preset
}

fn factory_file() -> &'static File {
    static FILE: OnceLock<File> = OnceLock::new();
    FILE.get_or_init(|| parse_file(FACTORY_TOML).expect("presets/factory-tool-presets.toml"))
}

/// CS-05: the factory `[[tool]]` rows (plain tools' Auto-Activate flags).
pub fn tool_flags() -> &'static [ToolFlag] {
    &factory_file().tool
}

/// CS-01: whether picking `tool` (through preset row `preset`, if any)
/// makes it live at once. The row's own column wins, then the tool's
/// `[[tool]]` row; tools in neither start at once.
pub fn auto_activate(tool: Tool, preset: Option<&str>) -> bool {
    if let Some(flag) = preset.and_then(row).and_then(|r| r.auto_activate) {
        return flag;
    }
    let name = format!("{tool:?}");
    tool_flags()
        .iter()
        .find(|f| f.tool == name)
        .is_none_or(|f| f.auto_activate)
}

/// The row with this id or phrase (ignoring case), if any.
pub fn row(name: &str) -> Option<&'static PresetRow> {
    let name = name.trim();
    factory()
        .iter()
        .find(|r| r.id.eq_ignore_ascii_case(name))
        .or_else(|| {
            factory()
                .iter()
                .find(|r| r.phrases().any(|p| p.eq_ignore_ascii_case(name)))
        })
}

/// Thicken's starting values: Smooth Shift's defaults with the Thicken
/// row's values (TP-15: also what Reset gives under Thicken).
pub fn thicken_values() -> SmoothShiftParameters {
    row("Thicken")
        .expect("the Thicken row")
        .smooth_shift(SmoothShiftParameters::default())
}

/// An open row: which one, and what to give back on drop (TP-10/12).
#[derive(Clone, Debug, PartialEq)]
pub struct PresetSession {
    pub id: String,
    pub tool: Tool,
    /// Rotate turns each point by weight × Angle (TW-Q1).
    pub angle_by_weight: bool,
    /// The user's falloff, and the Linear / Radial values the falloff
    /// panel remembers, when the row names the falloff slot.
    pub previous_falloff: Option<(Option<Falloff>, LinearFalloff, RadialFalloff)>,
    pub previous_center: Option<CenterMode>,
    pub previous_axis: Option<AxisOrientation>,
    /// Set once the row's tool has started, so a later plain pick of the
    /// same tool can tell itself apart from the row's own start.
    pub started: bool,
}
impl PresetSession {
    pub fn row(&self) -> &'static PresetRow {
        row(&self.id).expect("an open preset is a factory row")
    }
    pub fn label(&self) -> &'static str {
        &self.row().label
    }
}

impl crate::Editor {
    /// Start the row called `name`: give back an open row's slots (TP-13),
    /// remember the user's layers in every slot this row names, put its
    /// layers there and start its tool. Atomic: a refusal changes nothing.
    /// A row with no layers just starts its tool.
    pub(crate) fn activate_preset(&mut self, name: &str) -> Result<()> {
        let row = row(name).ok_or_else(|| format!("No tool preset called {name}"))?;
        let Some(tool) = row.tool() else {
            return Err(format!(
                "{} isn't an editor tool; use its button or command",
                row.label
            ));
        };
        if row.layers.is_empty()
            && !row.actor.single_ring
            && !row.actor.topology_pen
            && row.actor.path.is_none()
            && row.actor.copier.is_none()
        {
            return self.dispatch_inner(crate::Command::Activate(tool), &mut None);
        }
        // Axis Rotate is a transform: it turns items too.
        if self.scene.selection.mode == crate::Mode::Item
            && !row.layers.is_empty()
            && !row.actor.single_ring
        {
            return Err(format!(
                "{} works on points, edges or polygons; pick a component mode first",
                row.label
            ));
        }
        let mut staged = self.clone();
        staged.restore_preset_slots();
        // A path button sets its curve kind and switches (CX-01) before its
        // tool starts from the kept values.
        if let Some(path) = row.actor.path {
            let d = staged
                .presets
                .get_mut::<crate::duplicate::DuplicatePreset>();
            match tool {
                Tool::CurveExtrude => path.apply(&mut d.curve_extrude),
                Tool::CurveClone => path.apply(&mut d.curve_clone),
                _ => return Err(format!("preset {}: path values on a non-path tool", row.id)),
            }
        }
        let falloff = row.fitted_falloff(&staged.scene)?;
        // Axis Rotate in Item mode keeps the user's centre: items don't take
        // an Automatic centre yet.
        let action = if staged.scene.selection.mode == crate::Mode::Item && row.actor.single_ring {
            None
        } else {
            row.action_context(&staged.scene, staged.action_context)
        };
        let session = PresetSession {
            id: row.id.clone(),
            tool,
            angle_by_weight: row.actor.angle_by_weight,
            previous_falloff: falloff.is_some().then_some((
                staged.spatial_falloff,
                staged.linear_falloff_preset,
                staged.radial_falloff_preset,
            )),
            previous_center: (action.is_some() && row.names_slot(Slot::ActionCenter))
                .then_some(staged.action_context.center),
            previous_axis: row
                .names_slot(Slot::ActionAxis)
                .then_some(staged.action_context.axis),
            started: false,
        };
        if let Some(falloff) = falloff {
            staged.spatial_falloff = Some(falloff);
        }
        if let Some(action) = action {
            staged.action_context = action;
        }
        staged.session.preset = Some(session);
        staged.dispatch_inner(crate::Command::Activate(tool), &mut None)?;
        if let Some(session) = staged.session.preset.as_mut() {
            session.started = true;
        }
        // Topology Pen's options stick for the session (TP-10): a new
        // activation starts from the last ones, falloff included.
        if row.actor.topology_pen
            && let Some(live) = staged.session.live.as_mut()
        {
            let pen = staged.topology_pen_memory;
            live.params.topology_pen = pen;
            live.params.tweak_options = pen.tweak_options(false);
        }
        *self = staged;
        Ok(())
    }

    /// TP-19: an open row whose falloff starts at the tool puts a Radial or
    /// Cylinder falloff's centre on the live tool's centre (its action
    /// frame, in world space), keeping the fitted size. The falloff's own
    /// space applies: with Use World Transforms off (FO-35) the centre is
    /// taken into the active mesh's space. Anything missing leaves the
    /// falloff as it is.
    pub(crate) fn place_row_falloff_at_tool(&mut self) {
        let Some(live) = self.session.live.as_ref() else {
            return;
        };
        if !self
            .session
            .preset
            .as_ref()
            .is_some_and(|p| p.tool == live.tool && p.row().falloff_at_tool())
        {
            return;
        }
        let Some(world) = live.action_frame.map(|f| f.center) else {
            return;
        };
        let Some(falloff) = self.spatial_falloff else {
            return;
        };
        let center = if falloff.uses_world() {
            world
        } else {
            match self.scene.active().map(|i| self.scene.world_to_local(i.id)) {
                Some(Ok(m)) => m.transform_point3(world),
                None => world,
                Some(Err(_)) => return,
            }
        };
        if !center.is_finite() {
            return;
        }
        self.spatial_falloff = Some(match falloff {
            Falloff::Radial(f) => Falloff::Radial(RadialFalloff { center, ..f }),
            Falloff::Cylinder(f) => Falloff::Cylinder(crate::CylinderFalloff { center, ..f }),
            other => other,
        });
    }

    /// Whether a Duplicate copier row (DX-20/30) for `tool` is open.
    pub(crate) fn copier_row_open(&self, tool: Tool) -> bool {
        self.session
            .preset
            .as_ref()
            .is_some_and(|p| p.tool == tool && p.row().actor.copier.is_some())
    }

    /// A plain pick of `tool` while a Duplicate copier row (DX-20/30) for
    /// the same tool is open closes that row, so the plain button makes
    /// ordinary copies.
    pub(crate) fn close_copier_row_for(&mut self, tool: Tool) {
        if self
            .session
            .preset
            .as_ref()
            .is_some_and(|p| p.started && p.tool == tool && p.row().actor.copier.is_some())
        {
            self.restore_preset_slots();
        }
    }

    /// Give back every slot the open row took (TP-12) and close it.
    fn restore_preset_slots(&mut self) {
        let Some(session) = self.session.preset.take() else {
            return;
        };
        if let Some((falloff, linear, radial)) = session.previous_falloff {
            self.spatial_falloff = falloff;
            self.linear_falloff_preset = linear;
            self.radial_falloff_preset = radial;
        }
        if let Some(center) = session.previous_center {
            self.action_context.center = center;
        }
        if let Some(axis) = session.previous_axis {
            self.action_context.axis = axis;
        }
    }

    /// After every command: once the row's tool is no longer live (Drop,
    /// Cancel, another tool, Undo past its start), its slots go back
    /// (TP-12).
    pub(crate) fn settle_preset(&mut self) {
        let Some(session) = &self.session.preset else {
            return;
        };
        if let Some(live) = self
            .session
            .live
            .as_ref()
            .filter(|l| l.tool == session.tool)
        {
            if session.row().actor.topology_pen {
                // The falloff is edited in Tweak's options (Move uses them).
                let t = live.params.tweak_options;
                let mut pen = live.params.topology_pen;
                pen.radius = t.range;
                pen.shape = t.shape;
                pen.use_connectivity = t.connect == crate::TweakConnect::Connectivity;
                self.topology_pen_memory = pen;
            }
            return;
        }
        self.restore_preset_slots();
    }

    /// The name of the live tool as the user picked it: the row's label
    /// while one is open.
    pub fn live_tool_label(&self) -> Option<&'static str> {
        let live = self.session.live.as_ref()?;
        Some(
            self.session
                .preset
                .as_ref()
                .filter(|p| p.tool == live.tool)
                .map_or(live.tool.label(), PresetSession::label),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_factory_table_reads() {
        assert!(factory().len() >= 15);
        for id in [
            "xfrm.magnet",
            "Flex",
            "Flare",
            "FlipIt",
            "FlipItOrigin",
            "Thicken",
        ] {
            assert!(row(id).is_some(), "{id}");
        }
        // Old names and labels still find their row (TP-01 aliases).
        assert_eq!(row("soft move").unwrap().id, "xfrm.magnet");
        assert_eq!(row("Bulge").unwrap().id, "xfrm.pole");
    }

    #[test]
    fn bad_rows_are_refused_when_read() {
        let base = "[[preset]]\nid = \"a\"\nlabel = \"A\"\n";
        assert!(parse(&format!("{base}actor = {{ tool = \"Nope\" }}")).is_err());
        assert!(parse(&format!("{base}actor = {{ tool = \"Move\", wobble = 1 }}")).is_err());
        assert!(parse(&format!("{base}actor = {{}}")).is_err());
        assert!(
            parse(&format!(
                "{base}actor = {{ tool = \"Move\" }}\n{}",
                "layers = [{ slot = \"falloff\", kind = \"linear\" }, { slot = \"falloff\", kind = \"radial\" }]"
            ))
            .is_err()
        );
        assert!(
            parse(&format!(
                "{base}actor = {{ tool = \"Move\" }}\n{base}actor = {{ tool = \"Move\" }}"
            ))
            .is_err()
        );
        assert!(parse(&format!("{base}actor = {{ tool = \"Move\" }}")).is_ok());
    }

    #[test]
    fn thicken_values_match_the_old_hard_coded_ones() {
        let old = SmoothShiftParameters {
            angle: 90.1,
            thicken: true,
            ..Default::default()
        };
        assert_eq!(thicken_values(), old);
    }
}
