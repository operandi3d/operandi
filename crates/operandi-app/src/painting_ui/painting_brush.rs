//! Painting stage 2 in the app (PNT-40..87): the Airbrush, Paintbrush and
//! Eraser tools with the Smooth and Hard tips, painting live in the 3D
//! view. Painting builds only.
//!
//! - A tool is a stack (PNT-40): the tool's own rows, then the tip's. One
//!   tip at a time, shared by the three tools (PNT-41). The tips and the
//!   dab spacing are the shared brush engine's (brush_engine.rs), the
//!   same code the sculpt brushes run (PNT-42; their values stay per sculpt
//!   brush, PNT-Q10).
//! - Press lays a dab under the pointer, dragging lays dabs every
//!   Interpolation Step × size (PNT-47, 48); the image updates in every
//!   view while the stroke goes on (PNT-51); release records one undo step
//!   "<Tool> Stroke" (PNT-50). Right-drag sizes the tip (PNT-53);
//!   middle-drag does nothing.
//! - Every setting is a command line (UIC-160): the toolbox buttons run
//!   `operandi.paint.airbrush` …, Tool Properties rows run
//!   `operandi.paint.set <name>:<value>`.
use super::*;
use crate::command_handlers::Commands;
use crate::painting::word;
use crate::theme::viewport as palette;
use crate::unavailable::Gate;
use operandi_core::brush_engine::{DabPlace, Spacing, Tip, TipKind, TipShape};
use operandi_core::command_registry::{self as table, Area, ArgType, Unit};
use operandi_core::command_runtime::{Call, CommandState, HostHandler};
use operandi_core::paint::{self, PaintEdit};
use operandi_core::paint_stroke::{
    PaintBlend, PaintSettings, PaintStroke, PaintTool, PaintView, StrokeSetup,
};
use operandi_core::paint_tips::{Brush, BrushPreset, TipChoice};
use std::sync::Arc;

/// Maximum Brush Size (PNT-82; provisional start 500 px, PNT-Q18).
pub(crate) const MAX_BRUSH_SIZE: f64 = 500.;
/// Size change per pointer pixel of a right-drag (as sculpt's).
const SIZE_RATE: f64 = 0.5;
/// Most dabs one pointer move lays.
const MAX_DABS: usize = 4000;
pub(crate) const LATER: &str = "Coming in a later stage";

/// The toolbox's buttons for tools later stages bring (PNT-85).
#[cfg(test)]
pub(crate) const LATER_TOOLS: &[&str] = &["Duplicate Tools"];

/// A stroke in progress.
pub(crate) struct LivePaint {
    pub(crate) stroke: PaintStroke,
    spacing: Spacing,
    /// The paint target layer (a Photoshop image paints its PSD Layer).
    layer: u64,
    item: u64,
    /// What the scene showed for the image at the press (put back before
    /// the stroke's undo step is recorded).
    shown: Arc<operandi_core::paint::pixels::PaintImage>,
    /// The tip radius in screen points for the whole stroke.
    radius: f64,
    /// When the views last got the stroke's pixels.
    shown_at: Option<std::time::Instant>,
    /// The 3D views' texture took the stroke's changed rectangles (PNT-51):
    /// the scene keeps `shown` until the release.
    patched: bool,
    /// The texture couldn't take a rectangle: whole pictures from then on
    /// (as a UV-view stroke shows them).
    whole_pictures: bool,
    /// Changed pixels not yet in a whole picture.
    pending: bool,
    /// Line and the gradients: the press point; the shape goes down on
    /// release (PNT-100, 108).
    shape_from: Option<Pos2>,
    shape_to: Pos2,
    /// The tip stamps once (Stamp mode, Stamp, Text; PNT-132..134).
    stamp: bool,
    /// The nozzle's per-stroke seed, the dabs laid so far, and the last
    /// pointer event (for speed and direction).
    seed: u32,
    dabs: u32,
    last_event: Option<(Pos2, std::time::Instant)>,
    last_dab: Option<glam::Vec2>,
    /// The Curve Constraint's background curves (PNT-200) and, for
    /// Background 3D, how far from the curve a dab reaches (world units).
    guide: Option<(
        operandi_core::paint_inks::CurveConstraint,
        operandi_core::paint_inks::CurveGuide,
    )>,
    reach: Option<f64>,
}

/// The drawn lasso's handles: the centre moves it, the corner sizes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LassoHandle {
    Move,
    Scale,
}

/// The paint tools' state (not scene data).
pub(crate) struct BrushState {
    pub(crate) tool: Option<PaintTool>,
    /// Each tool's own rows (in `PaintTool::ALL` order); the tip is shared
    /// (`tip`).
    settings: Vec<PaintSettings>,
    /// Every tip's values and the Tablet Nozzle (paint_tips.rs): what a
    /// brush preset saves.
    pub(crate) tips: Brush,
    pub(crate) live: Option<LivePaint>,
    /// Numbers each stroke's jitter seed.
    strokes: u32,
    /// A right-drag sizing: where it started.
    pub(crate) adjust_at: Option<Pos2>,
    /// The Paint tab joins the toolbox's tab strip once opened (a paint
    /// tool turned on, or the tab picked), like UV and Workbench.
    pub(crate) tab_shown: bool,
    /// Clone (PNT-95, 96): the source point on screen, and the offset the
    /// first dab after it fixed (kept for later strokes).
    pub(crate) clone_source: Option<Pos2>,
    pub(crate) clone_offset: Option<glam::DVec2>,
    /// Lasso (PNT-115..120): drawing it is on, the lasso drawn, and the
    /// points of one being drawn.
    pub(crate) lasso_mode: bool,
    pub(crate) lasso: Option<operandi_core::paint_lasso::Lasso>,
    pub(crate) lasso_settings: operandi_core::paint_lasso::Lasso,
    lasso_draft: Option<Vec<Pos2>>,
    /// A drag on the drawn lasso's handles: which, where it started and
    /// the lasso then (stage 6, PNT-120).
    pub(crate) lasso_drag: Option<(LassoHandle, Pos2, operandi_core::paint_lasso::Lasso)>,
    lasso_last_edit: Option<(glam::DVec2, glam::DVec2, f64)>,
    /// Stage 7: the sculpt brushes' own tips (PNT-42/Q10 keeps them apart
    /// from the paint tools'), the mask brush in use and the mask's
    /// settings, and the meshes a mask stroke lent its hidden map to.
    pub(crate) sculpt_tips: Brush,
    pub(crate) mask_brush: Option<operandi_core::SculptBrush>,
    pub(crate) mask: crate::painting_sculpt::MaskSettings,
    pub(crate) mask_lent: Vec<(u64, operandi_core::MeshSlot, usize)>,
    /// The last refusal, shown over the view (`paint_notice`).
    pub(crate) notice: Option<(String, std::time::Instant)>,
    /// Stage 5's inks (painting_inks.rs).
    pub(crate) ink: crate::painting_inks::InkState,
    /// The stroke in progress started in a UV view (PNT-215).
    pub(crate) uv_live: bool,
}

impl Default for BrushState {
    fn default() -> Self {
        let s = PaintSettings::default();
        BrushState {
            tool: None,
            settings: vec![s; PaintTool::ALL.len()],
            tips: Brush::default(),
            live: None,
            strokes: 0,
            adjust_at: None,
            tab_shown: false,
            clone_source: None,
            clone_offset: None,
            lasso_mode: false,
            lasso: None,
            lasso_settings: operandi_core::paint_lasso::Lasso::default(),
            lasso_draft: None,
            notice: None,
            ink: Default::default(),
            uv_live: false,
            lasso_drag: None,
            sculpt_tips: Brush::default(),
            mask_brush: None,
            mask: Default::default(),
            mask_lent: Vec::new(),
            lasso_last_edit: None,
        }
    }
}

fn slot(tool: PaintTool) -> usize {
    PaintTool::ALL
        .iter()
        .position(|&t| t == tool)
        .expect("every tool is in ALL")
}

impl BrushState {
    /// `tool`'s settings with the shared tip.
    pub(crate) fn settings(&self, tool: PaintTool) -> PaintSettings {
        PaintSettings {
            tip: self.tips.tip(),
            ..self.settings[slot(tool)]
        }
    }
    /// The active tip's Size, Auto Scale and Bias.
    pub(crate) fn tip(&self) -> Tip {
        self.tips.tip()
    }
    /// Default Foreground Color (PNT-214): every tool's FG.
    pub(crate) fn set_every_fg(&mut self, rgb: [f64; 3]) {
        for s in &mut self.settings {
            s.fg.color = rgb;
        }
    }
    /// Maximum Brush Size went down: the tip follows.
    pub(crate) fn clamp_size(&mut self, max: f64) {
        let tip = self.tips.tip_mut();
        tip.size = tip.size.min(max);
    }
    /// The seed of the stroke about to start (Random Ink, the nozzle).
    pub(crate) fn stroke_seed(&self) -> u32 {
        self.strokes.wrapping_add(1)
    }
}

// ------------------------------------------------------------- commands

const SET_ARGS: &[table::Arg] = &[
    table::opt("step", ArgType::Float, Unit::Percent, None),
    table::opt("blend", ArgType::Text, Unit::None, None),
    table::opt("opacity", ArgType::Float, Unit::Percent, None),
    table::opt("density", ArgType::Float, Unit::Percent, None),
    table::opt("attenuation", ArgType::Int, Unit::None, None),
    table::opt("useFalloff", ArgType::Bool, Unit::None, None),
    table::opt("fg", ArgType::Text, Unit::None, None),
    table::opt("fgAlpha", ArgType::Float, Unit::Percent, None),
    table::opt("fgValue", ArgType::Float, Unit::Percent, None),
    table::opt("bg", ArgType::Text, Unit::None, None),
    table::opt("bgAlpha", ArgType::Float, Unit::Percent, None),
    table::opt("bgValue", ArgType::Float, Unit::Percent, None),
    table::opt("size", ArgType::Float, Unit::None, None),
    table::opt("autoScale", ArgType::Bool, Unit::None, None),
    table::opt("bias", ArgType::Float, Unit::Percent, None),
    table::opt("shape", ArgType::Text, Unit::None, None),
    table::opt("softBorder", ArgType::Float, Unit::Percent, None),
    table::opt("inWeight", ArgType::Float, Unit::None, None),
    table::opt("outWeight", ArgType::Float, Unit::None, None),
    table::opt("inSlope", ArgType::Float, Unit::None, None),
    table::opt("outSlope", ArgType::Float, Unit::None, None),
    table::opt("fillTolerance", ArgType::Float, Unit::Percent, None),
    table::opt(
        "lineShape",
        ArgType::Choice(&["line", "circle", "rectangle"]),
        Unit::None,
        None,
    ),
    table::opt(
        "curveConstraint",
        ArgType::Choice(&["none", "background2D", "background3D"]),
        Unit::None,
        None,
    ),
];
pub(crate) const TIPSET_ARGS: &[table::Arg] = &[
    table::opt(
        "pattern",
        ArgType::Choice(&["noise", "cellular", "dots"]),
        Unit::None,
        None,
    ),
    table::opt("invert", ArgType::Bool, Unit::None, None),
    table::opt("scaleU", ArgType::Float, Unit::Percent, None),
    table::opt("scaleV", ArgType::Float, Unit::Percent, None),
    table::opt("scaleW", ArgType::Float, Unit::Percent, None),
    table::opt("amplitude", ArgType::Float, Unit::Percent, None),
    table::opt("gain", ArgType::Float, Unit::Percent, None),
    table::opt("heightShading", ArgType::Bool, Unit::None, None),
    table::opt("flatness", ArgType::Float, Unit::Percent, None),
    table::opt("specular", ArgType::Float, Unit::Percent, None),
    table::opt("image", ArgType::Text, Unit::None, None),
    table::opt(
        "mode",
        ArgType::Choice(&["normal", "roller", "stamp"]),
        Unit::None,
        None,
    ),
    table::opt("offset", ArgType::Float, Unit::Percent, None),
    table::opt("rotation", ArgType::Float, Unit::None, None),
    table::opt("border", ArgType::Float, Unit::Percent, None),
    table::opt("useAsMask", ArgType::Bool, Unit::None, None),
    table::opt("keepAspect", ArgType::Bool, Unit::None, None),
    table::opt("text", ArgType::Text, Unit::None, None),
    table::opt("font", ArgType::Text, Unit::None, None),
    table::opt("bold", ArgType::Bool, Unit::None, None),
    table::opt("italics", ArgType::Bool, Unit::None, None),
    table::opt(
        "sensitivity",
        ArgType::Choice(&["low", "medium", "high"]),
        Unit::None,
        None,
    ),
    table::opt(
        "strengthBy",
        ArgType::Choice(&["none", "pressure", "speed", "both"]),
        Unit::None,
        None,
    ),
    table::opt(
        "sizeBy",
        ArgType::Choice(&["none", "pressure", "speed", "both"]),
        Unit::None,
        None,
    ),
    table::opt("align", ArgType::Bool, Unit::None, None),
    table::opt("jitter", ArgType::Bool, Unit::None, None),
    table::opt("jitterSpacing", ArgType::Float, Unit::Percent, None),
    table::opt("jitterPosition", ArgType::Float, Unit::Percent, None),
    table::opt("jitterSize", ArgType::Float, Unit::Percent, None),
    table::opt("jitterRotation", ArgType::Float, Unit::Percent, None),
    table::opt("jitterStrength", ArgType::Float, Unit::Percent, None),
];
const PRESET_ARGS: &[table::Arg] = &[
    table::opt("name", ArgType::Text, Unit::None, None),
    table::opt("filename", ArgType::Path, Unit::None, None),
];
const LASSO_ARGS: &[table::Arg] = &[
    table::opt(
        "style",
        ArgType::Choice(&["lasso", "rectangle", "circle", "ellipse"]),
        Unit::None,
        None,
    ),
    table::opt("softBorder", ArgType::Float, Unit::None, None),
    table::opt(
        "mix",
        ArgType::Choice(&["multiply", "add", "subtract", "max", "min"]),
        Unit::None,
        None,
    ),
    table::opt("worldTransforms", ArgType::Bool, Unit::None, None),
];
const LASSO_EDIT_ARGS: &[table::Arg] = &[
    table::opt("moveX", ArgType::Float, Unit::None, None),
    table::opt("moveY", ArgType::Float, Unit::None, None),
    table::opt("aboutX", ArgType::Float, Unit::None, None),
    table::opt("aboutY", ArgType::Float, Unit::None, None),
    table::opt("scale", ArgType::Float, Unit::None, None),
];
const TIP_ARGS: &[table::Arg] = &[table::arg(
    "kind",
    ArgType::Choice(&[
        "smooth",
        "hard",
        "procedural",
        "sphere",
        "image",
        "stamp",
        "text",
    ]),
    Unit::None,
)];
const LATER_ARGS: &[table::Arg] = &[table::opt("name", ArgType::Text, Unit::None, None)];

const fn cmd(name: &'static str, help: &'static str) -> table::Entry {
    table::script(name, Area::Tools, help).operandi()
}

type State = fn(&Operandi, &Call<'_>) -> CommandState;

/// What each tool's command says it does.
fn tool_help(tool: PaintTool) -> &'static str {
    match tool {
        PaintTool::Airbrush => {
            "Airbrush: soft strokes that build up within a stroke, up to Opacity (PNT-70). Turns the tool on or off."
        }
        PaintTool::Paintbrush => {
            "Paintbrush: solid strokes, no build-up within a stroke (PNT-71). Turns the tool on or off."
        }
        PaintTool::Eraser => {
            "Eraser: lowers alpha, or paints the BG colour on images without alpha (PNT-72). Turns the tool on or off."
        }
        PaintTool::Smudge => {
            "Smudge: drags the pixels under the brush along the stroke (PNT-90). Turns the tool on or off."
        }
        PaintTool::Blur => {
            "Blur: averages the pixels under the brush (PNT-91). Turns the tool on or off."
        }
        PaintTool::Sharpen => {
            "Sharpen (the toolbox's Sharp): raises edge contrast (PNT-92). Turns the tool on or off."
        }
        PaintTool::Clone => {
            "Clone: Ctrl+click sets the source, painting copies what the view shows at the same offset (PNT-95..98). Turns the tool on or off."
        }
        PaintTool::Line => {
            "Line: drag a Line, Circle or Rectangle, laid down with the tip on release (PNT-100..103). Turns the tool on or off."
        }
        PaintTool::Fill => {
            "Fill: click a pixel; every pixel within Fill Tolerance of its colour takes the FG colour (PNT-106). Turns the tool on or off."
        }
        PaintTool::LinearGradient => {
            "Linear Gradient: drag a line; the image ramps from FG to BG across it (PNT-108). Turns the tool on or off."
        }
        PaintTool::RadialGradient => {
            "Radial Gradient: drag from the centre; FG there to BG at the drag's end (PNT-109). Turns the tool on or off."
        }
    }
}

fn tool_of(name: &str) -> Option<PaintTool> {
    PaintTool::ALL.into_iter().find(|t| t.command() == name)
}

pub(crate) fn commands() -> Vec<(table::Entry, HostHandler<Operandi>, State)> {
    let mut out: Vec<(table::Entry, HostHandler<Operandi>, State)> = PaintTool::ALL
        .into_iter()
        .map(|tool| {
            let entry = table::script(tool.command(), Area::Tools, tool_help(tool))
                .operandi()
                .tool();
            let run: HostHandler<Operandi> = |a, c| {
                if let Some(mut t) = tool_of(c.entry.name) {
                    // PNT-111: Alt+click on Linear Gradient gives Radial.
                    if t == PaintTool::LinearGradient && a.alt_held() {
                        t = PaintTool::RadialGradient;
                    }
                    a.activate_paint(t);
                }
            };
            let state: State = |a, c| {
                CommandState::lit(
                    a.painting.brush.tool.map(PaintTool::command) == Some(c.entry.name),
                )
            };
            (entry, run, state)
        })
        .collect();
    let rest: Vec<(table::Entry, HostHandler<Operandi>, State)> = vec![
        (
            cmd("operandi.paint.tip", "The paint tip: Smooth, Hard (PNT-80, 81), Procedural, Sphere, Image (the toolbox's Texture), Stamp or Text (PNT-130..134). One tip at a time.")
                .args(TIP_ARGS)
                .example("operandi.paint.tip kind:hard"),
            |a, c| match word(c.rest, "kind", 0).and_then(|k| TipChoice::named(&k)) {
                Some(choice) => {
                    a.painting.brush.tips.choice = choice;
                    a.paint_status();
                }
                None => a.status = "Usage: operandi.paint.tip kind:<smooth|hard|procedural|sphere|image|stamp|text>".into(),
            },
            |a, c| {
                let want = word(c.rest, "kind", 0).and_then(|k| TipChoice::named(&k));
                CommandState::lit(want == Some(a.painting.brush.tips.choice))
            },
        ),
        (
            cmd("operandi.paint.set", "A paint tool setting (PNT-55..63, 80, 81) on the active paint tool (Airbrush when none): step, blend, opacity, density, attenuation, useFalloff, fg, fgAlpha, fgValue, bg, bgAlpha, bgValue; tip size, autoScale, bias, shape, softBorder, inWeight, outWeight, inSlope, outSlope. Percents 0..1. Colours r,g,b.")
                .args(SET_ARGS)
                .example("operandi.paint.set opacity:0.5 blend:multiply"),
            run_set,
            |_, _| CommandState::default(),
        ),
        (
            cmd("operandi.paint.tab", "Opens the toolbox's Paint tab (PNT-85); it then stays in the tab strip.").ui(),
            |a, _| {
                a.painting.brush.tab_shown = true;
                a.show_toolbox_tab(crate::toolbox::Category::Paint);
            },
            |a, _| {
                CommandState::lit(a.shown_category() == Some(crate::toolbox::Category::Paint))
            },
        ),
        (
            cmd("operandi.paint.notYet", "A paint toolbox button a later stage brings (PNT-85).").args(LATER_ARGS),
            |a, c| a.status = format!("{}: {LATER}", word(c.rest, "name", 0).unwrap_or_default()),
            |_, _| CommandState::unless(Some(LATER)),
        ),
        (
            cmd("operandi.paint.tipSet", "The active tip's own values (PNT-130..134) and the nozzle's (PNT-141): pattern (noise|cellular|dots), invert, scaleU, scaleV, scaleW, amplitude, gain; heightShading, flatness, specular; image (a scene image name or id), mode (normal|roller|stamp), offset, rotation (degrees), border, useAsMask, keepAspect; text, font, bold, italics; sensitivity (low|medium|high), strengthBy, sizeBy (none|pressure|speed|both), align, jitter, jitterSpacing, jitterPosition, jitterSize, jitterRotation, jitterStrength. Percents 0..1.")
                .args(TIPSET_ARGS)
                .example("operandi.paint.tipSet pattern:cellular invert:true"),
            run_tip_set,
            |_, _| CommandState::default(),
        ),
        (
            cmd("operandi.paint.nozzle", "Nozzle: adds the Tablet Nozzle to the paint tool (pressure, speed, rotation, jitter; PNT-140), or takes it off.").ui(),
            |a, _| {
                let t = &mut a.painting.brush.tips;
                t.nozzle_on = !t.nozzle_on;
                a.status = if t.nozzle_on { "Tablet Nozzle on".into() } else { "Tablet Nozzle off".into() };
            },
            |a, _| CommandState::lit(a.painting.brush.tips.nozzle_on),
        ),
        (
            cmd("operandi.paint.savePreset", "Save Brush Preset: the tip, its values and the nozzle (never the tool) as a brush preset in the Presets folder's Brushes (PNT-150).")
                .args(PRESET_ARGS)
                .side_effect()
                .example("operandi.paint.savePreset name:Patchy"),
            run_save_preset,
            |_, _| CommandState::default(),
        ),
        (
            cmd("operandi.paint.applyPreset", "Applies a brush preset to the current tool: its tip, values and nozzle (PNT-150).")
                .args(PRESET_ARGS)
                .example("operandi.paint.applyPreset name:Patchy"),
            run_apply_preset,
            |_, _| CommandState::default(),
        ),
        (
            cmd("operandi.paint.lasso", "Lasso: draw a shape (left-drag) that limits painting with Use Falloff on (PNT-115..120). Turns drawing on or off; the lasso stays when a paint tool is picked.").ui(),
            |a, _| {
                let b = &mut a.painting.brush;
                b.lasso_mode = !b.lasso_mode;
                b.tab_shown = true;
                a.status = if b.lasso_mode {
                    "Lasso: left-drag to draw it, then pick a paint tool (Use Falloff on)".into()
                } else {
                    "Lasso off".into()
                };
            },
            |a, _| CommandState::lit(a.painting.brush.lasso_mode),
        ),
        (
            cmd("operandi.paint.lassoSet", "Lasso settings: style (lasso, rectangle, circle, ellipse), softBorder (screen pixels), mix (the falloff mix modes), worldTransforms.")
                .args(LASSO_ARGS)
                .example("operandi.paint.lassoSet style:rectangle softBorder:10"),
            run_lasso_set,
            |_, _| CommandState::default(),
        ),
        (
            cmd("operandi.paint.lassoEdit", "The drawn lasso's handles as a command (PNT-120): moveX, moveY (screen points), then scale about aboutX, aboutY.")
                .args(LASSO_EDIT_ARGS)
                .example("operandi.paint.lassoEdit moveX:20 moveY:0"),
            run_lasso_edit,
            |a, _| CommandState::unless(a.painting.brush.lasso.is_none().then_some("No lasso is drawn")),
        ),
        (
            cmd("operandi.paint.lassoClear", "Removes the drawn lasso."),
            |a, _| {
                a.painting.brush.lasso = None;
                a.status = "Lasso cleared".into();
            },
            |a, _| CommandState::unless(a.painting.brush.lasso.is_none().then_some("No lasso is drawn")),
        ),
    ];
    out.extend(rest);
    out
}

pub(crate) fn run_tip_set(a: &mut Operandi, c: &Call<'_>) {
    use operandi_core::paint_tips::{Drive, ImageMode, PatternType, Sensitivity};
    let images: Vec<(u64, String)> = a
        .editor
        .scene
        .images
        .iter()
        .map(|i| (i.id, i.name.clone()))
        .collect();
    // Sculpt brushes keep their own tip values (PNT-42/Q10); the nozzle is
    // one for every brush (PNT-241).
    let sculpt = c.entry.name == "operandi.paint.sculptTipSet";
    let mut t = if sculpt {
        a.painting.brush.sculpt_tips.clone()
    } else {
        a.painting.brush.tips.clone()
    };
    let apply = |t: &mut Brush| -> std::result::Result<(), String> {
        let words = operandi_core::command_line::tokens(c.rest).map_err(str::to_owned)?;
        if words.is_empty() {
            return Err("Name a tip setting: operandi.paint.tipSet pattern:cellular".into());
        }
        let stamp = t.choice == TipChoice::Stamp;
        for w in words {
            let (Some(name), value) = operandi_core::command_line::named(&w) else {
                return Err(format!("{w}: use name:value"));
            };
            let flag = || operandi_core::command_args::boolean(name, value);
            let pct = || number(value, name);
            let drive = || {
                Drive::ALL
                    .into_iter()
                    .find(|d| d.label().eq_ignore_ascii_case(value))
                    .ok_or_else(|| format!("{name} is none, pressure, speed or both"))
            };
            let img = if stamp { &mut t.stamp } else { &mut t.image };
            match name {
                "pattern" => {
                    t.procedural.pattern = PatternType::ALL
                        .into_iter()
                        .find(|p| p.label().eq_ignore_ascii_case(value))
                        .ok_or("pattern is noise, cellular or dots")?
                }
                "invert" => t.procedural.invert = flag()?,
                "scaleU" => t.procedural.scale[0] = pct()?,
                "scaleV" => t.procedural.scale[1] = pct()?,
                "scaleW" => t.procedural.scale[2] = pct()?,
                "amplitude" => t.procedural.amplitude = pct()?,
                "gain" => t.procedural.gain = pct()?,
                "heightShading" => t.sphere.height_shading = flag()?,
                "flatness" => t.sphere.flatness = pct()?,
                "specular" => t.sphere.specular = pct()?,
                "image" => {
                    img.image = if value.eq_ignore_ascii_case("none") {
                        None
                    } else {
                        Some(
                            images
                                .iter()
                                .find(|(id, n)| n == value || id.to_string() == value)
                                .map(|(id, _)| *id)
                                .ok_or_else(|| format!("No image called {value}"))?,
                        )
                    }
                }
                "mode" => {
                    img.mode = ImageMode::ALL
                        .into_iter()
                        .find(|m| m.label().eq_ignore_ascii_case(value))
                        .ok_or("mode is normal, roller or stamp")?
                }
                "offset" => img.offset = pct()?,
                "rotation" => {
                    let r = number(value, name)?.to_radians();
                    if t.choice == TipChoice::Text {
                        t.text.rotation = r;
                    } else {
                        img.rotation = r;
                    }
                }
                "border" => img.soft_border = pct()?,
                "useAsMask" => img.use_as_mask = flag()?,
                "keepAspect" => img.keep_aspect = flag()?,
                "text" => t.text.text = value.to_owned(),
                "font" => {
                    let f = operandi_core::paint_tips::FONTS
                        .iter()
                        .find(|(n, _)| n.eq_ignore_ascii_case(value))
                        .ok_or_else(|| format!("No font {value}"))?;
                    t.text.font = f.0.to_owned();
                }
                "bold" => t.text.bold = flag()?,
                "italics" => t.text.italics = flag()?,
                "sensitivity" => {
                    t.nozzle.sensitivity = match value.to_ascii_lowercase().as_str() {
                        "low" => Sensitivity::Low,
                        "medium" => Sensitivity::Medium,
                        "high" => Sensitivity::High,
                        _ => return Err("sensitivity is low, medium or high".into()),
                    }
                }
                "strengthBy" => t.nozzle.strength = drive()?,
                "sizeBy" => t.nozzle.size = drive()?,
                "align" => t.nozzle.align = flag()?,
                "jitter" => t.nozzle.jitter = flag()?,
                "jitterSpacing" | "jitterPosition" | "jitterSize" | "jitterRotation"
                | "jitterStrength" => {
                    let v = pct()?;
                    if !(0. ..=5.).contains(&v) {
                        return Err("Jitter is 0 % to 500 %".into());
                    }
                    let n = &mut t.nozzle;
                    match name {
                        "jitterSpacing" => n.jitter_spacing = v,
                        "jitterPosition" => n.jitter_position = v,
                        "jitterSize" => n.jitter_size = v,
                        "jitterRotation" => n.jitter_rotation = v,
                        _ => n.jitter_strength = v,
                    }
                }
                other => return Err(format!("No tip setting {other}")),
            }
        }
        Ok(())
    };
    match apply(&mut t) {
        Ok(()) if sculpt => {
            a.painting.brush.tips.nozzle = t.nozzle;
            a.painting.brush.sculpt_tips = t;
        }
        Ok(()) => {
            a.painting.brush.sculpt_tips.nozzle = t.nozzle;
            a.painting.brush.tips = t;
        }
        Err(e) => a.status = e,
    }
}

/// Where brush presets go: Presets/Brushes in the data folder.
pub(crate) fn brush_folder(a: &Operandi) -> std::path::PathBuf {
    a.environment.paths().presets().join("Brushes")
}

fn preset_path(
    a: &Operandi,
    c: &Call<'_>,
) -> std::result::Result<(String, std::path::PathBuf), String> {
    if let Some(f) = word(c.rest, "filename", usize::MAX) {
        let path = std::path::PathBuf::from(&f);
        let name = path
            .file_stem()
            .map_or_else(|| f.clone(), |s| s.to_string_lossy().into_owned());
        return Ok((name, path));
    }
    let name = match word(c.rest, "name", 0).filter(|n| !n.trim().is_empty()) {
        Some(n) => n,
        // The button: named after the tip, numbered when taken.
        None if c.entry.name == "operandi.paint.savePreset" => {
            let base = format!("{} Brush", a.painting.brush.tips.choice.label());
            let taken = |n: &str| {
                brush_folder(a)
                    .join(format!(
                        "{n}.{}",
                        operandi_core::paint_tips::BRUSH_PRESET_EXTENSION
                    ))
                    .exists()
            };
            (1..)
                .map(|k| {
                    if k == 1 {
                        base.clone()
                    } else {
                        format!("{base} {k}")
                    }
                })
                .find(|n| !taken(n))
                .expect("some number is free")
        }
        None => return Err("Name the brush preset: name:<name>".into()),
    };
    let safe: String = name
        .chars()
        .map(|ch| if "/\\:".contains(ch) { '_' } else { ch })
        .collect();
    let file = format!(
        "{safe}.{}",
        operandi_core::paint_tips::BRUSH_PRESET_EXTENSION
    );
    Ok((name, brush_folder(a).join(file)))
}

/// The saved brush presets' names, for Texture > Paint Tools > Brush
/// Presets.
pub(crate) fn preset_names(a: &Operandi) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(brush_folder(a))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            (p.extension()
                .is_some_and(|x| x == operandi_core::paint_tips::BRUSH_PRESET_EXTENSION))
            .then(|| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .flatten()
        })
        .collect();
    names.sort();
    names
}

fn run_save_preset(a: &mut Operandi, c: &Call<'_>) {
    match preset_path(a, c).and_then(|(name, path)| {
        BrushPreset::new(&name, a.active_tips())
            .save(&path)
            .map(|()| name)
    }) {
        Ok(name) => a.status = format!("Saved brush preset {name}"),
        Err(e) => a.status = e,
    }
}

fn run_apply_preset(a: &mut Operandi, c: &Call<'_>) {
    match preset_path(a, c).and_then(|(_, path)| BrushPreset::load(&path)) {
        Ok(p) => {
            // The tool stays; its tip, values and nozzle change (PNT-150).
            // PNT-244: the tool stays; a sculpt brush takes the preset
            // into its own tips, and the nozzle is shared.
            a.apply_brush(p.brush);
            a.status = format!("Brush preset {} applied", p.name);
        }
        Err(e) => a.status = e,
    }
}

fn run_lasso_set(a: &mut Operandi, c: &Call<'_>) {
    use operandi_core::paint_lasso::LassoStyle;
    let mut l = a.painting.brush.lasso_settings.clone();
    if let Some(v) = word(c.rest, "style", usize::MAX) {
        match LassoStyle::named(&v) {
            Some(s) => l.style = s,
            None => {
                a.status = format!("No lasso style {v}");
                return;
            }
        }
    }
    if let Some(v) = word(c.rest, "softBorder", usize::MAX) {
        match v.parse::<f64>() {
            Ok(n) if (0. ..=1000.).contains(&n) => l.soft_border = n,
            _ => {
                a.status = "Soft Border is 0 to 1000 pixels".into();
                return;
            }
        }
    }
    if let Some(v) = word(c.rest, "mix", usize::MAX) {
        match operandi_core::MixMode::ALL
            .into_iter()
            .find(|m| m.label().eq_ignore_ascii_case(&v))
        {
            Some(m) => l.mix = m,
            None => {
                a.status = format!("No mix mode {v}");
                return;
            }
        }
    }
    if let Some(v) = word(c.rest, "worldTransforms", usize::MAX) {
        match operandi_core::command_args::boolean("worldTransforms", &v) {
            Ok(on) => l.world_transforms = on,
            Err(e) => {
                a.status = e;
                return;
            }
        }
    }
    let b = &mut a.painting.brush;
    if let Some(drawn) = &mut b.lasso {
        // The drawn shape keeps its points and takes the new settings.
        drawn.soft_border = l.soft_border;
        drawn.mix = l.mix;
        drawn.world_transforms = l.world_transforms;
        if drawn.style != l.style
            && drawn.style != operandi_core::paint_lasso::LassoStyle::Lasso
            && l.style != operandi_core::paint_lasso::LassoStyle::Lasso
        {
            drawn.style = l.style;
        }
    }
    b.lasso_settings = l;
}

pub(crate) fn register(r: &mut Commands) {
    for (entry, run, state) in commands() {
        let name = entry.name;
        r.add_host_command(entry, run)
            .unwrap_or_else(|e| panic!("{e}"));
        r.answer(name, state).unwrap_or_else(|e| panic!("{e}"));
    }
}

/// A handle drag from `from` to `to` on lasso `start`: the lasso, and its
/// move, scale centre and scale.
fn lasso_edit(
    start: &operandi_core::paint_lasso::Lasso,
    h: LassoHandle,
    from: Pos2,
    to: Pos2,
) -> (
    operandi_core::paint_lasso::Lasso,
    glam::DVec2,
    glam::DVec2,
    f64,
) {
    let d = glam::DVec2::new(f64::from(to.x - from.x), f64::from(to.y - from.y));
    let (lo, hi) = start.bounds().unwrap_or_default();
    let centre = (lo + hi) / 2.;
    match h {
        LassoHandle::Move => (start.moved(d, centre, 1.), d, centre, 1.),
        LassoHandle::Scale => {
            let f = glam::DVec2::new(f64::from(from.x), f64::from(from.y));
            let was = f.distance(centre).max(1.);
            let k = (f + d).distance(centre) / was;
            (
                start.moved(glam::DVec2::ZERO, centre, k),
                glam::DVec2::ZERO,
                centre,
                k,
            )
        }
    }
}

fn run_lasso_edit(a: &mut Operandi, c: &Call<'_>) {
    let Some(l) = a.painting.brush.lasso.clone() else {
        a.status = "No lasso is drawn".into();
        return;
    };
    let get = |n: &str, d: f64| -> std::result::Result<f64, String> {
        word(c.rest, n, usize::MAX).map_or(Ok(d), |v| number(&v, n))
    };
    let r = (|| -> std::result::Result<_, String> {
        Ok((
            glam::DVec2::new(get("moveX", 0.)?, get("moveY", 0.)?),
            glam::DVec2::new(get("aboutX", 0.)?, get("aboutY", 0.)?),
            get("scale", 1.)?,
        ))
    })();
    match r {
        Ok((by, about, k)) => {
            a.painting.brush.lasso = Some(l.moved(by, about, k));
            a.status = "Lasso adjusted".into();
        }
        Err(e) => a.status = e,
    }
}

fn number(v: &str, what: &str) -> std::result::Result<f64, String> {
    let v = v.trim();
    let (v, pct) = match v.strip_suffix('%') {
        Some(n) => (n, true),
        None => (v, false),
    };
    let n: f64 = v.parse().map_err(|_| format!("{what} is a number"))?;
    Ok(if pct { n / 100. } else { n })
}

fn colour(v: &str) -> std::result::Result<[f64; 3], String> {
    let parts: Vec<f64> = v
        .split(',')
        .map(|p| p.trim().parse::<f64>())
        .collect::<std::result::Result<_, _>>()
        .map_err(|_| "colours are r,g,b".to_owned())?;
    match parts[..] {
        [r, g, b] => Ok([r, g, b]),
        _ => Err("colours are r,g,b".into()),
    }
}

fn run_set(a: &mut Operandi, c: &Call<'_>) {
    let tool = a.painting.brush.tool.unwrap_or(PaintTool::Airbrush);
    let max_size = a.painting.prefs.max_brush;
    let mut s = a.painting.brush.settings(tool);
    let apply = |s: &mut PaintSettings| -> std::result::Result<(), String> {
        let words = operandi_core::command_line::tokens(c.rest).map_err(str::to_owned)?;
        if words.is_empty() {
            return Err("Name a setting: operandi.paint.set opacity:0.5".into());
        }
        for w in words {
            let (Some(name), value) = operandi_core::command_line::named(&w) else {
                return Err(format!("{w}: use name:value"));
            };
            let flag = || operandi_core::command_args::boolean(name, value);
            match name {
                "step" => s.step = number(value, name)?,
                "blend" => {
                    s.blend =
                        PaintBlend::named(value).ok_or_else(|| format!("No blend mode {value}"))?
                }
                "opacity" => s.opacity = number(value, name)?,
                "density" => s.density = number(value, name)?,
                "attenuation" => {
                    s.attenuation = value.parse().map_err(|_| "attenuation is a whole number")?
                }
                "useFalloff" => s.use_falloff = flag()?,
                "fg" => s.fg.color = colour(value)?,
                "fgAlpha" => s.fg.alpha = number(value, name)?,
                "fgValue" => s.fg.value = number(value, name)?,
                "bg" => s.bg.color = colour(value)?,
                "bgAlpha" => s.bg.alpha = number(value, name)?,
                "bgValue" => s.bg.value = number(value, name)?,
                "size" => s.tip.size = number(value, name)?.clamp(1., max_size),
                "autoScale" => {
                    s.tip.auto_scale = flag()?;
                    s.tip.scale_reference = 0.;
                }
                "bias" => s.tip.bias = number(value, name)?,
                "softBorder" => s.tip.soft_border = number(value, name)?,
                "shape" => {
                    s.tip.shape = TipShape::ALL
                        .into_iter()
                        .chain([TipShape::Custom])
                        .find(|t| {
                            t.label()
                                .replace(' ', "")
                                .eq_ignore_ascii_case(&value.replace(' ', ""))
                        })
                        .ok_or_else(|| format!("No tip shape {value}"))?
                }
                "inWeight" | "outWeight" | "inSlope" | "outSlope" => {
                    let v = number(value, name)?;
                    let p = &mut s.tip.custom;
                    match name {
                        "inWeight" => p.in_weight = v,
                        "outWeight" => p.out_weight = v,
                        "inSlope" => p.in_slope = v,
                        _ => p.out_slope = v,
                    }
                    // Editing the profile makes it Custom (PNT-80).
                    s.tip.shape = TipShape::Custom;
                }
                "fillTolerance" => s.fill_tolerance = number(value, name)?,
                "lineShape" => {
                    s.shape = operandi_core::paint_stroke::LineShape::ALL
                        .into_iter()
                        .find(|l| l.label().eq_ignore_ascii_case(value))
                        .ok_or_else(|| format!("No line shape {value}"))?
                }
                "curveConstraint" => {
                    s.curve = operandi_core::paint_inks::CurveConstraint::named(value)
                        .ok_or("curveConstraint is none, background2D or background3D")?
                }
                other => return Err(format!("No paint setting {other}")),
            }
        }
        s.validate()
    };
    match apply(&mut s) {
        Ok(()) => {
            let b = &mut a.painting.brush;
            let kind = b.tips.tip().kind;
            *b.tips.tip_mut() = Tip { kind, ..s.tip };
            b.settings[slot(tool)] = PaintSettings {
                tip: b.tips.tip(),
                ..s
            };
        }
        Err(e) => a.status = e,
    }
}

// ------------------------------------------------------------- app side

impl Operandi {
    /// Alt is held this frame (a button's Alt+click).
    pub(crate) fn alt_held(&self) -> bool {
        self.frame_ctx
            .as_ref()
            .is_some_and(|c| c.input(|i| i.modifiers.alt))
    }

    /// Turns a paint tool on (or off when it is on). Turning one on turns
    /// the sculpt brush and draw tools off.
    pub(crate) fn activate_paint(&mut self, tool: PaintTool) {
        if self.painting.brush.tool == Some(tool) {
            self.drop_paint();
            return;
        }
        if self.navigation.is_some() || !self.confirm_fields() {
            return;
        }
        self.drop_sculpt();
        self.drop_draw();
        self.utility = None;
        if self.editor.session.has_active_task() {
            self.run(Command::Drop);
            if self.editor.session.has_active_task() {
                return;
            }
        }
        self.finish_paint_stroke();
        // Defaults > Default Tools > Default Brush (PNT-41, 214).
        if let crate::painting_prefs::DefaultBrush::Tip(choice) = self.painting.prefs.default_brush
            && self.painting.brush.tool.is_none()
        {
            self.painting.brush.tips.choice = choice;
        }
        self.painting.brush.tool = Some(tool);
        self.painting.brush.tab_shown = true;
        self.show_toolbox_tab(crate::toolbox::Category::Paint);
        self.paint_status();
    }

    pub(crate) fn drop_paint(&mut self) {
        self.finish_paint_stroke();
        if let Some(tool) = self.painting.brush.tool.take() {
            self.status = format!("{} off", tool.label());
        }
    }

    /// PNT-73: tool, tip size, target image and resolution.
    pub(crate) fn paint_status(&mut self) {
        let Some(tool) = self.painting.brush.tool else {
            return;
        };
        let size = self.painting.brush.tip().size;
        let image = self
            .current_image()
            .and_then(|id| self.editor.scene.images.iter().find(|i| i.id == id))
            .map_or_else(
                || "no image".to_owned(),
                |i| format!("{} {} × {}", i.name, i.width, i.height),
            );
        self.status = format!("{} · {size:.0} px · {image}", tool.label());
    }

    pub(crate) fn paint_view(&self, r: Rect) -> PaintView {
        let (eye, right, up, forward) = self.camera.basis();
        PaintView {
            eye,
            right,
            up,
            forward,
            perspective: self.camera.view == View::Perspective,
            distance: self.camera.distance,
            near: self.camera.near(),
            lens: self.camera.lens,
            center: glam::DVec2::new(f64::from(r.center().x), f64::from(r.center().y)),
            size: glam::DVec2::new(f64::from(r.width()), f64::from(r.height())),
        }
    }

    /// The surface under `p` on mesh `item`.
    fn paint_hit(&self, item: u64, p: Pos2, r: Rect) -> Option<DVec3> {
        let (hit_item, _, depth) =
            self.pick_surface()
                .pick_in(&self.camera, p, r, Some(&[item]))?;
        if hit_item != item {
            return None;
        }
        let (origin, dir) = self.camera.ray(p, r);
        Some(origin + dir * depth)
    }

    /// The tip radius on screen at `at` (PNT-80): Size, or with Auto Scale
    /// the size kept on the model.
    fn paint_radius(&mut self, at: DVec3, r: Rect) -> f64 {
        let tip = self.painting.brush.tips.tip_mut();
        if !tip.auto_scale {
            return tip.size;
        }
        let Some(mpp) = self.camera.metres_per_pixel(at, r) else {
            return tip.size;
        };
        if !(tip.scale_reference.is_finite() && tip.scale_reference > 0.) {
            tip.scale_reference = mpp;
        }
        tip.size * tip.scale_reference / mpp.max(1e-12)
    }

    #[cfg(test)]
    pub(crate) fn paint_radius_for_test(&mut self, at: DVec3, r: Rect) -> f64 {
        self.paint_radius(at, r)
    }

    /// The stroke for `tool` on the paint target, or why not (sets the
    /// status line): the checks every paint tool's press makes (PNT-13..18,
    /// PNT-304).
    fn begin_paint(
        &mut self,
        tool: PaintTool,
        p: Pos2,
        r: Rect,
    ) -> Option<(
        PaintStroke,
        u64,
        u64,
        Arc<operandi_core::paint::pixels::PaintImage>,
    )> {
        let layer = self.paint_target();
        // The mesh under the pointer, else the selected one (PNT-Q40).
        let item = self
            .pick_surface()
            .pick_in(&self.camera, p, r, None)
            .map(|(id, _, _)| id)
            .filter(|&id| self.editor.scene.item(id).is_some_and(|i| i.is_mesh()))
            // A curve or an untextured mesh in front doesn't take the
            // press from the selected, textured one.
            .filter(|&id| {
                paint::paint_target_problem_on(&self.editor.scene, layer, Some(id)).is_none()
            })
            .or(self.editor.scene.selection.item);
        if let Some(problem) = paint::paint_target_problem_on(&self.editor.scene, layer, item) {
            if problem == paint::NO_TEXTURE && self.painting.prefs.auto_create {
                self.painting.ask_add = true;
            } else {
                self.paint_notice(problem);
            }
            return None;
        }
        let layer = layer.expect("checked");
        let image = paint::layer_image(&self.editor.scene, layer).expect("checked");
        let Some(item) = item else {
            self.paint_notice("Nothing to paint: point at a mesh with the texture");
            return None;
        };
        let psd = match self.editor.psd_doc(image) {
            Ok(d) => d.is_some(),
            Err(e) => {
                self.status = e;
                return None;
            }
        };
        if psd && paint::psd_layer_of(&self.editor.scene, layer).is_none() {
            self.status = paint::PSD_WHICH_LAYER.into();
            self.painting.which_layer = Some((layer, String::new()));
            return None;
        }
        let pixels = match self.editor.layer_picture(layer) {
            Ok(px) => px,
            Err(e) => {
                self.status = e;
                return None;
            }
        };
        let shown = self.editor.scene.paint.buffers[&image].clone();
        let uv_map = self.paint_uv_map(layer);
        let falloff = self.editor.active_falloff();
        let settings = self.painting.brush.settings(tool);
        let lasso = self.painting.brush.lasso.clone();
        let stroke = PaintStroke::begin(StrokeSetup {
            scene: &self.editor.scene,
            item,
            image,
            pixels,
            uv_map: uv_map.as_deref(),
            view: self.paint_view(r),
            tool,
            settings,
            symmetry: self.editor.active_symmetry(),
            falloff: falloff.as_ref(),
            lasso: lasso.as_ref(),
        });
        let mut stroke = match stroke {
            Ok(s) => s,
            Err(e) => {
                self.status = e;
                return None;
            }
        };
        // The tip beyond its profile (PNT-130..134).
        let tips = self.painting.brush.tips.clone();
        let tip_image = match tips.choice {
            TipChoice::Image | TipChoice::Stamp => tips
                .image_settings()
                .image
                .and_then(|id| self.editor.image_pixels(id).ok()),
            _ => None,
        };
        if !tool.image_space() && !tool.moves_pixels() {
            match tips.extra(|_| tip_image.clone(), settings.fg.rgb()) {
                Ok(extra) => stroke.extra = extra,
                Err(e) => {
                    self.status = e;
                    return None;
                }
            }
        }
        match self.ink_run(tool, item, r) {
            Ok(run) => stroke.inks = run,
            Err(e) => {
                self.paint_notice(&e);
                return None;
            }
        }
        Some((stroke, layer, item, shown))
    }

    /// The UV map paint target `layer` reads (its texture locator's).
    fn paint_uv_map(&self, layer: u64) -> Option<String> {
        self.editor
            .scene
            .shader_tree
            .get(layer)
            .and_then(|n| match &n.kind {
                operandi_core::shader_tree::NodeKind::Texture(t) => t.locator,
                _ => None,
            })
            .and_then(|l| self.editor.scene.texture_locator(l))
            .and_then(|l| l.uv_map.clone())
    }

    /// While a paint tool is on and no stroke runs: the selected mesh's
    /// texels for the paint target are made ahead of the press
    /// (`paint_stroke::warm_surface`; nothing when they're kept already),
    /// so a stroke starts on the frame it's pressed.
    pub(crate) fn warm_paint_surface(&mut self) {
        if self.painting.brush.tool.is_none() || self.painting.brush.live.is_some() {
            return;
        }
        let Some(item) = self.editor.scene.selection.item else {
            return;
        };
        let Some(layer) = self.paint_target() else {
            return;
        };
        let Some(image) = paint::layer_image(&self.editor.scene, layer) else {
            return;
        };
        let Some((w, h)) = self
            .editor
            .scene
            .paint
            .buffers
            .get(&image)
            .map(|p| (p.width, p.height))
        else {
            return;
        };
        let uv_map = self.paint_uv_map(layer);
        operandi_core::paint_stroke::warm_surface(
            &self.editor.scene,
            item,
            uv_map.as_deref(),
            w,
            h,
        );
    }

    /// The drawn lasso's handles on screen, when it shows as drawn: its
    /// box's centre (move) and lower-right corner (size). Hidden once the
    /// view has moved from where it was drawn (it stays on the model).
    pub(crate) fn lasso_handles(&self, r: Rect) -> Option<[(LassoHandle, Pos2); 2]> {
        let b = &self.painting.brush;
        if b.lasso_mode || b.tool.is_none() {
            return None;
        }
        let l = b.lasso.as_ref()?;
        if l.world_transforms && l.view.is_some_and(|v| v != self.paint_view(r)) {
            return None;
        }
        let (lo, hi) = l.bounds()?;
        let c = (lo + hi) / 2.;
        Some([
            (LassoHandle::Move, pos2(c.x as f32, c.y as f32)),
            (LassoHandle::Scale, pos2(hi.x as f32, hi.y as f32)),
        ])
    }

    fn lasso_handle_at(&self, p: Pos2, r: Rect) -> Option<LassoHandle> {
        self.lasso_handles(r)?
            .into_iter()
            .find(|(_, q)| q.distance(p) <= 8.)
            .map(|(h, _)| h)
    }

    /// PNT-64: the colour the view shows under `p` becomes the FG colour:
    /// the painted image's colour there (the shading's lighting isn't
    /// picked; provisional, PNT-Q55).
    fn pick_fg(&mut self, p: Pos2, r: Rect) {
        let tool = self.painting.brush.tool.unwrap_or(PaintTool::Paintbrush);
        let Some((mut stroke, _, _, _)) = self.begin_paint(tool, p, r) else {
            return;
        };
        let q = glam::DVec2::new(f64::from(p.x), f64::from(p.y));
        match stroke.pixel_at(q) {
            Some(px) => {
                let c = stroke.pixels().get(px as usize);
                self.execute(&format!("operandi.paint.set fg:{},{},{}", c[0], c[1], c[2]));
                self.status = format!("FG Color picked: {:.2}, {:.2}, {:.2}", c[0], c[1], c[2]);
            }
            None => self.paint_notice("Ctrl+Shift+click on the textured mesh to pick its colour"),
        }
    }

    /// A primary press with a paint tool (PNT-48), or drawing the lasso
    /// (PNT-120). Ctrl+press with Clone sets its source (PNT-95).
    pub(crate) fn paint_press(&mut self, p: Pos2, r: Rect, mods: Modifiers) -> input::GestureKind {
        // The drawn lasso's handles, without leaving the tool (PNT-120).
        if let Some(h) = self.lasso_handle_at(p, r)
            && let Some(l) = self.painting.brush.lasso.clone()
        {
            self.painting.brush.lasso_drag = Some((h, p, l));
            return input::GestureKind::Paint;
        }
        // Ctrl+Shift+click picks the FG colour from the view (PNT-64).
        if self.painting.brush.tool.is_some()
            && mods.shift
            && (mods.ctrl || mods.command && !mods.mac_cmd)
        {
            self.pick_fg(p, r);
            return input::GestureKind::Blocked;
        }
        if self.painting.brush.lasso_mode {
            self.painting.brush.lasso_draft = Some(vec![p]);
            return input::GestureKind::Paint;
        }
        let Some(tool) = self.painting.brush.tool else {
            return input::GestureKind::Blocked;
        };
        if tool == PaintTool::Clone && (mods.ctrl || mods.command && !mods.mac_cmd) {
            let b = &mut self.painting.brush;
            b.clone_source = Some(p);
            b.clone_offset = None;
            self.status = "Clone source set: paint where the copy should go".into();
            return input::GestureKind::Blocked;
        }
        if let Some(kind) = self.ink_widget_press(p, r) {
            return kind;
        }
        if tool == PaintTool::Clone && self.painting.brush.clone_source.is_none() {
            self.paint_notice("Ctrl+click to set the clone source");
            return input::GestureKind::Blocked;
        }
        let Some((mut stroke, layer, item, shown)) = self.begin_paint(tool, p, r) else {
            return input::GestureKind::Blocked;
        };
        let screen = glam::DVec2::new(f64::from(p.x), f64::from(p.y));
        // Image or Projection Ink's Stamp: the whole picture in one click
        // (PNT-172, 191).
        if self.ink_stamps(tool) {
            let faces = self.ink_stamp_faces(item);
            stroke.ink_stamp(faces.as_deref());
            self.painting.brush.strokes = self.painting.brush.strokes.wrapping_add(1);
            self.commit_paint(stroke, layer, shown, false);
            return input::GestureKind::Blocked;
        }
        if tool == PaintTool::Fill {
            // PNT-106: one click, one undo step.
            match stroke.pixel_at(screen) {
                Some(seed) => {
                    stroke.fill(seed);
                    self.commit_paint(stroke, layer, shown, false);
                }
                None => self.status = "Fill: click on the painted mesh".into(),
            }
            return input::GestureKind::Blocked;
        }
        let stamp =
            !tool.image_space() && tool != PaintTool::Line && self.painting.brush.tips.stamps();
        if tool.image_space() || tool == PaintTool::Line || stamp {
            if tool.image_space() && stroke.pixel_at(screen).is_none() {
                self.status = format!("{}: start on the mesh", tool.label());
                return input::GestureKind::Blocked;
            }
            let radius = self.painting.brush.tip().size;
            self.painting.brush.live = Some(LivePaint {
                stroke,
                spacing: Spacing {
                    last: None,
                    carry: 0.,
                },
                layer,
                item,
                shown,
                radius,
                shown_at: None,
                patched: false,
                whole_pictures: false,
                pending: false,
                shape_from: Some(p),
                shape_to: p,
                stamp,
                seed: 0,
                dabs: 0,
                last_event: None,
                last_dab: None,
                guide: None,
                reach: None,
            });
            return input::GestureKind::Paint;
        }
        let guide = self.curve_guide(tool);
        let Some((p, point)) = self.constrained_hit(guide.as_ref(), item, p, r) else {
            self.paint_notice(&format!("{}: start the stroke on the mesh", tool.label()));
            return input::GestureKind::Blocked;
        };
        if tool == PaintTool::Clone {
            // PNT-96: the first dab after setting the source fixes the
            // offset; later strokes keep it.
            let b = &mut self.painting.brush;
            if b.clone_offset.is_none()
                && let Some(src) = b.clone_source
            {
                b.clone_offset = Some(glam::DVec2::new(
                    f64::from(src.x - p.x),
                    f64::from(src.y - p.y),
                ));
            }
            stroke.clone_offset = b.clone_offset;
        }
        let radius = self.paint_radius(point, r);
        // Background 3D: a dab reaches the tip's radius around the curve.
        let reach = guide
            .as_ref()
            .filter(|(c, _)| *c == operandi_core::paint_inks::CurveConstraint::Background3D)
            .and_then(|_| self.camera.metres_per_pixel(point, r))
            .map(|mpp| mpp * radius);
        let pressure = f64::from(crate::tablet::pressure_at([p.x, p.y])).clamp(0., 1.);
        let place = DabPlace {
            screen: glam::Vec2::new(p.x, p.y),
            point,
            normal: DVec3::Z,
            pressure,
        };
        let (spacing, first) = Spacing::press(place);
        self.painting.brush.strokes = self.painting.brush.strokes.wrapping_add(1);
        let mut live = LivePaint {
            stroke,
            spacing,
            layer,
            item,
            shown,
            radius,
            shown_at: None,
            patched: false,
            whole_pictures: false,
            pending: false,
            shape_from: None,
            shape_to: p,
            stamp: false,
            seed: self.painting.brush.strokes,
            dabs: 0,
            last_event: Some((p, std::time::Instant::now())),
            last_dab: None,
            guide,
            reach,
        };
        self.nozzle_dab(&mut live, first, 0.);
        self.painting.brush.live = Some(live);
        self.show_live_paint();
        self.paint_status();
        input::GestureKind::Paint
    }

    /// A pointer move during a stroke: dabs along the path (PNT-47), the
    /// shape's end (Line, gradients), or the lasso's next point.
    pub(crate) fn paint_drag(&mut self, p: Pos2, r: Rect) {
        if let Some((h, from, start)) = self.painting.brush.lasso_drag.clone() {
            let (l, by, about, k) = lasso_edit(&start, h, from, p);
            self.painting.brush.lasso = Some(l);
            self.painting.brush.lasso_last_edit = Some((by, about, k));
            return;
        }
        if let Some(draft) = &mut self.painting.brush.lasso_draft {
            if draft.last().is_none_or(|q| q.distance(p) >= 2.) {
                draft.push(p);
            }
            return;
        }
        if self.ink_widget_drag(p) {
            return;
        }
        let Some(mut live) = self.painting.brush.live.take() else {
            return;
        };
        if live.shape_from.is_some() {
            live.shape_to = p;
            self.painting.brush.live = Some(live);
            return;
        }
        match self.constrained_hit(live.guide.as_ref(), live.item, p, r) {
            None => live.spacing.lift(),
            Some((p, point)) => {
                let pressure = f64::from(crate::tablet::pressure_at([p.x, p.y])).clamp(0., 1.);
                let to = DabPlace {
                    screen: glam::Vec2::new(p.x, p.y),
                    point,
                    normal: DVec3::Z,
                    pressure,
                };
                // Pointer speed for the nozzle (PNT-141).
                let now = std::time::Instant::now();
                let speed = live.last_event.map_or(0., |(q, t)| {
                    let dt = now.duration_since(t).as_secs_f64().max(1e-3);
                    f64::from(q.distance(p)) / dt
                });
                live.last_event = Some((p, now));
                let tips = &self.painting.brush.tips;
                let gap_share = if tips.nozzle_on {
                    tips.nozzle
                        .dab(live.seed, live.dabs, pressure, speed, 0.)
                        .gap
                } else {
                    1.
                };
                let gap = (live.stroke.settings.step * live.radius * gap_share).max(1.);
                for place in live.spacing.advance(to, || gap, MAX_DABS) {
                    self.nozzle_dab(&mut live, place, speed);
                }
            }
        }
        self.painting.brush.live = Some(live);
        self.show_live_paint();
    }

    /// One dab at `place`, through the Tablet Nozzle when it is on
    /// (PNT-140, 141): its strength, size, rotation and jitter; without it,
    /// pressure drives strength as in stage 2.
    fn nozzle_dab(&self, live: &mut LivePaint, place: DabPlace, speed: f64) {
        let center = glam::DVec2::new(f64::from(place.screen.x), f64::from(place.screen.y));
        let direction = live
            .last_dab
            .map(|q| {
                let d = place.screen - q;
                f64::from(d.y).atan2(f64::from(d.x))
            })
            .unwrap_or(0.);
        live.last_dab = Some(place.screen);
        live.stroke.near = live.reach.map(|r| (place.point, r));
        let tips = &self.painting.brush.tips;
        if tips.nozzle_on {
            let m = tips
                .nozzle
                .dab(live.seed, live.dabs, place.pressure, speed, direction);
            live.stroke.dab_turned(
                center + m.offset * live.radius,
                live.radius * m.size,
                Some(place.point),
                m.strength,
                m.angle,
            );
        } else {
            live.stroke
                .dab(center, live.radius, Some(place.point), place.pressure);
        }
        live.stroke.near = None;
        live.dabs += 1;
    }

    /// The pointer `p` moved onto the Curve Constraint's curve (PNT-200),
    /// with the surface point there: Background 2D snaps on screen and
    /// then finds the surface; Background 3D finds the surface, then the
    /// curve's nearest point in space.
    fn constrained_hit(
        &self,
        guide: Option<&(
            operandi_core::paint_inks::CurveConstraint,
            operandi_core::paint_inks::CurveGuide,
        )>,
        item: u64,
        p: Pos2,
        r: Rect,
    ) -> Option<(Pos2, DVec3)> {
        use operandi_core::paint_inks::CurveConstraint;
        let at = |v: glam::DVec2| pos2(v.x as f32, v.y as f32);
        match guide {
            Some((CurveConstraint::Background2D, g)) => {
                let view = self.paint_view(r);
                let q = at(g.snap_screen(&view, glam::DVec2::new(f64::from(p.x), f64::from(p.y)))?);
                Some((q, self.paint_hit(item, q, r)?))
            }
            Some((CurveConstraint::Background3D, g)) => {
                let on = g.snap_world(self.paint_hit(item, p, r)?)?;
                let (q, _) = self.paint_view(r).project(on)?;
                Some((at(q), on))
            }
            _ => Some((p, self.paint_hit(item, p, r)?)),
        }
    }

    /// PNT-51: the stroke's pixels in every view while it goes on. The
    /// whole image goes to the GPU, at most about 30 times a second
    /// (provisional: changed tiles only is PNT-Q19).
    fn show_live_paint(&mut self) {
        let Some(live) = &mut self.painting.brush.live else {
            return;
        };
        // 3D views: the changed rectangle goes straight into the texture
        // the views already show, every frame (PNT-51): no whole picture
        // is made or uploaded during the stroke.
        if !self.painting.brush.uv_live && !live.whole_pictures {
            let Some(rect) = live.stroke.take_dirty() else {
                return;
            };
            let image = live.stroke.image;
            let rgba = crate::gpu_scene::painted_rgba(live.stroke.pixels(), rect);
            let written = self
                .resident_cache
                .painted_texture(image)
                .zip(self.viewport_gpu.as_mut())
                .is_some_and(|(key, gpu)| gpu.patch_painted(key, rect, &rgba));
            if written {
                live.patched = true;
                return;
            }
            live.whole_pictures = true;
            live.pending = true;
        }
        // A UV-view stroke (its view draws the scene's picture), or a
        // texture that can't take rectangles: whole pictures, ~30 a second.
        live.pending |= live.stroke.take_dirty().is_some();
        let due = live
            .shown_at
            .is_none_or(|t| t.elapsed() >= std::time::Duration::from_millis(33));
        if !due || !live.pending {
            return;
        }
        live.pending = false;
        live.shown_at = Some(std::time::Instant::now());
        let image = live.stroke.image;
        let px = Arc::new(live.stroke.pixels().clone());
        self.editor.scene.paint.buffers.insert(image, px);
    }

    /// Records a finished stroke as one undo step (PNT-50).
    fn commit_paint(
        &mut self,
        mut stroke: PaintStroke,
        layer: u64,
        shown: Arc<operandi_core::paint::pixels::PaintImage>,
        patched: bool,
    ) {
        let image = stroke.image;
        // The scene goes back to its pixels at the press; the command
        // records the stroke's.
        self.editor.scene.paint.buffers.insert(image, shown.clone());
        if !stroke.changed {
            // PNT-23: never silent.
            self.paint_notice(if stroke.reachable() == 0 {
                "Nothing painted: the texture's UVs don't reach the surface you can see"
            } else {
                "Nothing painted: the brush didn't touch the textured surface"
            });
            return;
        }
        let label = stroke.label();
        let pixels = Arc::new(stroke.pixels().clone());
        if patched {
            // The views' texture already shows these pixels: it keeps them
            // (no whole picture uploaded at the release).
            self.resident_cache.adopt_painted(image, &shown, &pixels);
        }
        if self.run_ok(Command::Paint(PaintEdit::Stroke {
            image,
            via: Some(layer),
            pixels,
            label: label.clone(),
        })) {
            self.status = label;
        } else if patched {
            // Not recorded: the texture took the stroke's rectangles, so
            // the press's pixels go up again as a new picture.
            self.editor
                .scene
                .paint
                .buffers
                .insert(image, Arc::new((*shown).clone()));
        }
    }

    /// A refusal shown over the view for a few seconds and in the status
    /// line, so a press that paints nothing always says why (PNT-23).
    pub(crate) fn paint_notice(&mut self, text: &str) {
        self.status = text.to_owned();
        self.painting.brush.notice = Some((text.to_owned(), std::time::Instant::now()));
    }

    /// The release: the stroke (or the dragged Line or gradient) becomes
    /// one undo step; a drawn lasso is kept (PNT-120).
    pub(crate) fn finish_paint_stroke(&mut self) {
        self.painting.brush.uv_live = false;
        if let Some((_, _, start)) = self.painting.brush.lasso_drag.take() {
            // The handle drag goes down as one command line.
            self.painting.brush.lasso = Some(start);
            if let Some((by, about, k)) = self.painting.brush.lasso_last_edit.take() {
                self.execute(&format!(
                    "operandi.paint.lassoEdit moveX:{} moveY:{} aboutX:{} aboutY:{} scale:{k}",
                    by.x, by.y, about.x, about.y
                ));
            }
            return;
        }
        if self.ink_widget_release() {
            return;
        }
        if let Some(draft) = self.painting.brush.lasso_draft.take() {
            let settings = self.painting.brush.lasso_settings.clone();
            let points: Vec<glam::DVec2> = draft
                .iter()
                .map(|q| glam::DVec2::new(f64::from(q.x), f64::from(q.y)))
                .collect();
            let points = if settings.style == operandi_core::paint_lasso::LassoStyle::Lasso {
                points
            } else {
                vec![points[0], *points.last().expect("pressed")]
            };
            // Fixed to the model through the view it was drawn in (stage 6).
            let view = Some(self.paint_view(self.viewport_rect));
            self.painting.brush.lasso = Some(operandi_core::paint_lasso::Lasso {
                points,
                view,
                ..settings
            });
            self.status = "Lasso drawn: pick a paint tool and turn on Use Falloff".into();
            return;
        }
        let Some(mut live) = self.painting.brush.live.take() else {
            return;
        };
        if let Some(from) = live.shape_from {
            let to = live.shape_to;
            let (a, b) = (
                glam::DVec2::new(f64::from(from.x), f64::from(from.y)),
                glam::DVec2::new(f64::from(to.x), f64::from(to.y)),
            );
            match live.stroke.tool {
                // PNT-132..134: the press is the centre; the drag's length
                // is the size, its angle the rotation.
                _ if live.stamp => {
                    let d = b - a;
                    let radius = d.length().max(1.);
                    live.stroke.dab_turned(a, radius, None, 1., d.y.atan2(d.x));
                }
                PaintTool::Line => live.stroke.shape(a, b, live.radius),
                // A UV-view stroke's ends are UV points, on the tile or not.
                tool if live.stroke.uv_at(a).is_some() => {
                    let (ua, ub) = (
                        live.stroke.uv_at(a).expect("UV stroke"),
                        live.stroke.uv_at(b).expect("UV stroke"),
                    );
                    live.stroke
                        .gradient(ua, ub, tool == PaintTool::RadialGradient);
                }
                tool => {
                    let ends = (live.stroke.pixel_at(a), live.stroke.pixel_at(b));
                    match ends {
                        (Some(pa), Some(pb)) => {
                            let (ua, ub) = (live.stroke.pixel_uv(pa), live.stroke.pixel_uv(pb));
                            live.stroke
                                .gradient(ua, ub, tool == PaintTool::RadialGradient);
                        }
                        _ => {
                            self.status =
                                format!("{}: start and end the drag on the mesh", tool.label())
                        }
                    }
                }
            }
        }
        self.commit_paint(
            live.stroke,
            live.layer,
            live.shown,
            live.patched && !live.whole_pictures,
        );
    }

    /// A primary press in a UV view with a paint tool (PNT-215): the
    /// stroke goes straight into the paint target's image, dabs sized in
    /// the UV view's screen points.
    pub(crate) fn uv_paint_press(
        &mut self,
        p: Pos2,
        view: operandi_core::paint_stroke::UvScreen,
    ) -> bool {
        let Some(tool) = self.painting.brush.tool else {
            return false;
        };
        if tool == PaintTool::Clone && self.painting.brush.clone_source.is_none() {
            self.paint_notice("Ctrl+click to set the clone source");
            return true;
        }
        let Some(layer) = self.paint_target() else {
            self.paint_notice(paint::NO_TEXTURE);
            return true;
        };
        let Some(image) = paint::layer_image(&self.editor.scene, layer) else {
            self.paint_notice(paint::NO_TEXTURE);
            return true;
        };
        let pixels = match self.editor.layer_picture(layer) {
            Ok(px) => px,
            Err(e) => {
                self.paint_notice(&e);
                return true;
            }
        };
        let shown = self.editor.scene.paint.buffers[&image].clone();
        let settings = self.painting.brush.settings(tool);
        let mut stroke = match PaintStroke::begin_uv(operandi_core::paint_stroke::UvStrokeSetup {
            image,
            pixels,
            uv: view,
            tool,
            settings,
        }) {
            Ok(s) => s,
            Err(e) => {
                self.paint_notice(&e);
                return true;
            }
        };
        let tips = self.painting.brush.tips.clone();
        if !tool.image_space() && !tool.moves_pixels() {
            let tip_image = match tips.choice {
                TipChoice::Image | TipChoice::Stamp => tips
                    .image_settings()
                    .image
                    .and_then(|id| self.editor.image_pixels(id).ok()),
                _ => None,
            };
            match tips.extra(|_| tip_image.clone(), settings.fg.rgb()) {
                Ok(extra) => stroke.extra = extra,
                Err(e) => {
                    self.paint_notice(&e);
                    return true;
                }
            }
        }
        let screen = glam::DVec2::new(f64::from(p.x), f64::from(p.y));
        if tool == PaintTool::Fill {
            match stroke.pixel_at(screen) {
                Some(seed) => {
                    stroke.fill(seed);
                    self.commit_paint(stroke, layer, shown, false);
                }
                None => self.paint_notice("Fill: click on the image's tile"),
            }
            return true;
        }
        let item = self.editor.scene.selection.item.unwrap_or(0);
        let radius = self.painting.brush.tip().size;
        let stamp =
            !tool.image_space() && tool != PaintTool::Line && self.painting.brush.tips.stamps();
        let uv = stroke.uv_at(screen).unwrap_or_default();
        let point = DVec3::new(uv.x, uv.y, 0.);
        if tool == PaintTool::Clone {
            let b = &mut self.painting.brush;
            if b.clone_offset.is_none()
                && let Some(src) = b.clone_source
            {
                b.clone_offset = Some(glam::DVec2::new(
                    f64::from(src.x - p.x),
                    f64::from(src.y - p.y),
                ));
            }
            stroke.clone_offset = b.clone_offset;
        }
        let shape = tool.image_space() || tool == PaintTool::Line || stamp;
        let place = DabPlace {
            screen: glam::Vec2::new(p.x, p.y),
            point,
            normal: DVec3::Z,
            pressure: f64::from(crate::tablet::pressure_at([p.x, p.y])).clamp(0., 1.),
        };
        let (spacing, first) = Spacing::press(place);
        self.painting.brush.strokes = self.painting.brush.strokes.wrapping_add(1);
        let mut live = LivePaint {
            stroke,
            spacing,
            layer,
            item,
            shown,
            radius,
            shown_at: None,
            patched: false,
            whole_pictures: false,
            pending: false,
            shape_from: shape.then_some(p),
            shape_to: p,
            stamp,
            seed: self.painting.brush.strokes,
            dabs: 0,
            last_event: Some((p, std::time::Instant::now())),
            last_dab: None,
            guide: None,
            reach: None,
        };
        if !shape {
            self.nozzle_dab(&mut live, first, 0.);
        }
        self.painting.brush.live = Some(live);
        self.painting.brush.uv_live = true;
        self.show_live_paint();
        true
    }

    /// A pointer move during a UV-view stroke.
    pub(crate) fn uv_paint_drag(&mut self, p: Pos2) {
        let Some(mut live) = self.painting.brush.live.take() else {
            return;
        };
        if live.shape_from.is_some() {
            live.shape_to = p;
            self.painting.brush.live = Some(live);
            return;
        }
        let q = glam::DVec2::new(f64::from(p.x), f64::from(p.y));
        let uv = live.stroke.uv_at(q).unwrap_or_default();
        let to = DabPlace {
            screen: glam::Vec2::new(p.x, p.y),
            point: DVec3::new(uv.x, uv.y, 0.),
            normal: DVec3::Z,
            pressure: f64::from(crate::tablet::pressure_at([p.x, p.y])).clamp(0., 1.),
        };
        let gap = (live.stroke.settings.step * live.radius).max(1.);
        for place in live.spacing.advance(to, || gap, MAX_DABS) {
            self.nozzle_dab(&mut live, place, 0.);
        }
        self.painting.brush.live = Some(live);
        self.show_live_paint();
    }

    /// A right press with a paint tool: size the tip (PNT-53).
    pub(crate) fn begin_paint_size(&mut self, p: Pos2) -> input::GestureKind {
        self.painting.brush.adjust_at = Some(p);
        input::GestureKind::PaintSize {
            start: self.painting.brush.tip().size,
        }
    }

    pub(crate) fn paint_size_drag(&mut self, start: f64, from: Pos2, to: Pos2) {
        let dx = f64::from(to.x - from.x);
        let tip = self.painting.brush.tips.tip_mut();
        tip.size = (start + dx * SIZE_RATE)
            .round()
            .clamp(1., self.painting.prefs.max_brush);
        tip.scale_reference = 0.;
        self.status = format!("Size: {} px · drag right to grow, left to shrink", tip.size);
    }

    pub(crate) fn paint_size_release(&mut self) {
        self.painting.brush.adjust_at = None;
        self.paint_status();
    }

    /// PNT-52: the ring at the pointer, and where a Hard tip's Soft Border
    /// begins.
    pub(crate) fn draw_paint_cursor(&self, painter: &Painter, hover: Option<Pos2>, r: Rect) {
        // The sculpt mask in red (PNT-260).
        self.draw_sculpt_mask(painter, r);
        self.draw_inks(painter, r);
        let b = &self.painting.brush;
        if let Some((text, at)) = &b.notice
            && at.elapsed() < std::time::Duration::from_secs(4)
        {
            let pos = pos2(r.center().x, r.top() + 40.);
            let galley = painter.layout_no_wrap(
                text.clone(),
                FontId::proportional(14.),
                egui::Color32::WHITE,
            );
            let bg = egui::Rect::from_center_size(pos, galley.size() + vec2(20., 12.));
            painter.rect_filled(bg, 6., egui::Color32::from_black_alpha(200));
            painter.galley(bg.min + vec2(10., 6.), galley, egui::Color32::WHITE);
            painter
                .ctx()
                .request_repaint_after(std::time::Duration::from_millis(200));
        }
        let color = palette::tweak_range();
        let ring = Stroke::new(1.5_f32, color);
        let faint = Stroke::new(1_f32, color.gamma_multiply(0.45));
        // The lasso (PNT-115): drawn or being drawn.
        let lasso_pts: Option<Vec<Pos2>> = match (&b.lasso_draft, &b.lasso) {
            (Some(d), _)
                if b.lasso_settings.style == operandi_core::paint_lasso::LassoStyle::Lasso =>
            {
                Some(d.clone())
            }
            (Some(d), _) => {
                let l = operandi_core::paint_lasso::Lasso {
                    points: [d[0], *d.last().expect("pressed")]
                        .iter()
                        .map(|q| glam::DVec2::new(f64::from(q.x), f64::from(q.y)))
                        .collect(),
                    ..b.lasso_settings.clone()
                };
                Some(
                    l.polygon()
                        .iter()
                        .map(|q| pos2(q.x as f32, q.y as f32))
                        .collect(),
                )
            }
            (None, Some(l)) => Some(
                l.polygon()
                    .iter()
                    .map(|q| pos2(q.x as f32, q.y as f32))
                    .collect(),
            ),
            _ => None,
        };
        // A lasso fixed to the model hides its outline once the view moves
        // (it can't be drawn on the model yet); a note says it's there.
        let hidden = b.lasso_draft.is_none()
            && b.lasso.as_ref().is_some_and(|l| {
                l.world_transforms && l.view.is_some_and(|v| v != self.paint_view(r))
            });
        if hidden {
            painter.text(
                r.left_top() + vec2(10., 10.),
                egui::Align2::LEFT_TOP,
                "Lasso: fixed to the model",
                FontId::proportional(11.),
                palette::mirror_selection(),
            );
        }
        if let Some(h) = self.lasso_handles(r) {
            let s = Stroke::new(1.5_f32, palette::mirror_selection());
            painter.rect_stroke(
                Rect::from_center_size(h[0].1, vec2(10., 10.)),
                0.,
                s,
                egui::StrokeKind::Middle,
            );
            painter.circle_stroke(h[1].1, 5., s);
        }
        if let Some(pts) = lasso_pts.filter(|_| !hidden)
            && pts.len() > 1
        {
            let mut closed = pts.clone();
            closed.push(pts[0]);
            painter.add(egui::Shape::line(
                closed,
                Stroke::new(1.5_f32, palette::mirror_selection()),
            ));
        }
        let Some(tool) = b.tool else {
            return;
        };
        // Line and gradient previews (PNT-100, 108): nothing is written
        // until the release.
        if let Some(live) = &b.live
            && let Some(from) = live.shape_from
        {
            let to = live.shape_to;
            let to_d = |q: Pos2| glam::DVec2::new(f64::from(q.x), f64::from(q.y));
            if live.stamp {
                // PNT-132: the size as a ring, the angle as a handle.
                painter.circle_stroke(from, from.distance(to).max(1.), ring);
                painter.line_segment([from, to], faint);
                painter.circle_filled(to, 3., color);
                return;
            }
            let pts: Vec<Pos2> = match tool {
                PaintTool::Line => live
                    .stroke
                    .settings
                    .shape
                    .outline(to_d(from), to_d(to))
                    .iter()
                    .map(|q| pos2(q.x as f32, q.y as f32))
                    .collect(),
                PaintTool::RadialGradient => {
                    painter.circle_stroke(from, from.distance(to), faint);
                    vec![from, to]
                }
                _ => vec![from, to],
            };
            painter.add(egui::Shape::line(pts, ring));
            return;
        }
        if tool.image_space() {
            return;
        }
        let at = match (b.adjust_at, hover) {
            (Some(a), _) => a,
            (None, Some(h)) => h,
            _ => return,
        };
        let px = match &b.live {
            Some(live) => live.radius as f32,
            None => {
                let tip = b.tip();
                let tip = &tip;
                if tip.auto_scale && tip.scale_reference > 0. {
                    let item = self.editor.scene.selection.item;
                    let mpp = item
                        .and_then(|i| self.paint_hit(i, at, r))
                        .and_then(|p| self.camera.metres_per_pixel(p, r))
                        .unwrap_or(tip.scale_reference);
                    (tip.size * tip.scale_reference / mpp.max(1e-12)) as f32
                } else {
                    tip.size as f32
                }
            }
        };
        painter.circle_stroke(at, px, ring);
        if b.tip().kind == TipKind::Hard && b.tip().soft_border > 0.02 {
            painter.circle_stroke(at, px * (1. - b.tip().soft_border as f32), faint);
        }
        // PNT-97: a crosshair at the clone source.
        if tool == PaintTool::Clone {
            let src = match b.clone_offset {
                Some(o) => Some(at + vec2(o.x as f32, o.y as f32)),
                None => b.clone_source,
            };
            if let Some(c) = src {
                let k = 6.;
                painter.line_segment([c - vec2(k, 0.), c + vec2(k, 0.)], ring);
                painter.line_segment([c - vec2(0., k), c + vec2(0., k)], ring);
            }
        }
    }

    /// Tool Properties for the active paint tool (PNT-40, 70..72, 80, 81,
    /// 90..110), or the Lasso's rows while drawing it (PNT-116..119): the
    /// tool's rows in the spec's order, then the tip's. Each change runs a
    /// command line.
    pub(crate) fn paint_properties(&mut self, ui: &mut Ui) {
        let mut lines: Vec<String> = Vec::new();
        if self.painting.brush.lasso_mode {
            self.lasso_properties(ui, &mut lines);
        }
        if let Some(tool) = self.painting.brush.tool {
            if self.painting.brush.lasso_mode {
                ui.separator();
            }
            self.tool_rows(ui, tool, &mut lines);
        }
        for line in lines {
            self.execute(&line);
        }
    }

    fn lasso_properties(&self, ui: &mut Ui, lines: &mut Vec<String>) {
        use operandi_core::paint_lasso::LassoStyle;
        let l = &self.painting.brush.lasso_settings;
        ui.strong("Lasso");
        crate::option_row::field_row(ui, "Style", |ui| {
            egui::ComboBox::from_id_salt("paint-lasso-style")
                .selected_text(l.style.label())
                .show_ui(ui, |ui| {
                    for st in LassoStyle::ALL {
                        if ui.selectable_label(l.style == st, st.label()).clicked() {
                            lines.push(format!(
                                "operandi.paint.lassoSet style:{}",
                                st.label().to_ascii_lowercase()
                            ));
                        }
                    }
                });
        });
        crate::option_row::field_row(ui, "Soft Border", |ui| {
            let mut v = l.soft_border;
            if ui
                .add(
                    crate::numeric_tab::DragValue::new(&mut v)
                        .range(0. ..=1000.)
                        .suffix(" px"),
                )
                .changed()
            {
                lines.push(format!("operandi.paint.lassoSet softBorder:{v}"));
            }
        });
        crate::option_row::field_row(ui, "Mix Mode", |ui| {
            egui::ComboBox::from_id_salt("paint-lasso-mix")
                .selected_text(l.mix.label())
                .show_ui(ui, |ui| {
                    for m in operandi_core::MixMode::ALL {
                        if ui.selectable_label(l.mix == m, m.label()).clicked() {
                            lines.push(format!(
                                "operandi.paint.lassoSet mix:{}",
                                m.label().to_ascii_lowercase()
                            ));
                        }
                    }
                });
        });
        crate::option_row::field_row(ui, "Use World Transforms", |ui| {
            let mut on = l.world_transforms;
            if ui.checkbox(&mut on, "").changed() {
                lines.push(format!("operandi.paint.lassoSet worldTransforms:{on}"));
            }
        });
        if self.painting.brush.lasso.is_some() && ui.button("Clear Lasso").clicked() {
            lines.push("operandi.paint.lassoClear".into());
        }
    }

    fn tool_rows(&self, ui: &mut Ui, tool: PaintTool, lines: &mut Vec<String>) {
        let s = self.painting.brush.settings(tool);
        let pct = |ui: &mut Ui, label: &str, v: f64, name: &str, lines: &mut Vec<String>| {
            crate::option_row::field_row(ui, label, |ui| {
                let mut x = v * 100.;
                if ui
                    .add(
                        crate::numeric_tab::DragValue::new(&mut x)
                            .range(0. ..=100.)
                            .suffix(" %"),
                    )
                    .changed()
                {
                    lines.push(format!("operandi.paint.set {name}:{}", x / 100.));
                }
            });
        };
        let value = |ui: &mut Ui, label: &str, v: f64, name: &str, lines: &mut Vec<String>| {
            crate::option_row::field_row(ui, label, |ui| {
                let mut x = v * 100.;
                if ui
                    .add(
                        crate::numeric_tab::DragValue::new(&mut x)
                            .range(0. ..=10_000.)
                            .suffix(" %"),
                    )
                    .changed()
                {
                    lines.push(format!("operandi.paint.set {name}:{}", x / 100.));
                }
            });
        };
        let colour_row =
            |ui: &mut Ui, label: &str, c: [f64; 3], name: &str, lines: &mut Vec<String>| {
                crate::option_row::field_row(ui, label, |ui| {
                    let mut rgb = c.map(|v| v as f32);
                    if ui.color_edit_button_rgb(&mut rgb).changed() {
                        lines.push(format!(
                            "operandi.paint.set {name}:{},{},{}",
                            rgb[0], rgb[1], rgb[2]
                        ));
                    }
                });
            };
        let blend = |ui: &mut Ui, lines: &mut Vec<String>| {
            crate::option_row::field_row(ui, "Blend Mode", |ui| {
                egui::ComboBox::from_id_salt("paint-blend")
                    .selected_text(s.blend.label())
                    .show_ui(ui, |ui| {
                        for b in PaintBlend::ALL {
                            if ui.selectable_label(s.blend == b, b.label()).clicked() {
                                lines.push(format!(
                                    "operandi.paint.set blend:{}",
                                    b.label().replace(' ', "")
                                ));
                            }
                        }
                    });
            });
        };
        let falloff = |ui: &mut Ui, lines: &mut Vec<String>| {
            crate::option_row::field_row(ui, "Use Falloff", |ui| {
                let mut on = s.use_falloff;
                if ui.checkbox(&mut on, "").changed() {
                    lines.push(format!("operandi.paint.set useFalloff:{on}"));
                }
            });
        };
        let attenuation = |ui: &mut Ui, lines: &mut Vec<String>| {
            crate::option_row::field_row(ui, "Attenuation Steps", |ui| {
                let mut n = s.attenuation;
                if ui
                    .add(crate::numeric_tab::DragValue::new(&mut n).range(0..=10_000))
                    .changed()
                {
                    lines.push(format!("operandi.paint.set attenuation:{n}"));
                }
            });
        };
        let curve = |ui: &mut Ui, lines: &mut Vec<String>| {
            use operandi_core::paint_inks::CurveConstraint;
            crate::option_row::field_row(ui, "Curve Constraint", |ui| {
                // PNT-56, 200: snaps to a background curve.
                egui::ComboBox::from_id_salt("paint-curve-constraint")
                    .selected_text(s.curve.label())
                    .show_ui(ui, |ui| {
                        for c in CurveConstraint::ALL {
                            if ui.selectable_label(s.curve == c, c.label()).clicked() {
                                lines.push(format!(
                                    "operandi.paint.set curveConstraint:{}",
                                    c.label().replace(' ', "")
                                ));
                            }
                        }
                    });
            });
        };
        let fg = |ui: &mut Ui, lines: &mut Vec<String>| {
            colour_row(ui, "FG Color", s.fg.color, "fg", lines);
            pct(ui, "FG Alpha", s.fg.alpha, "fgAlpha", lines);
            value(ui, "FG Value", s.fg.value, "fgValue", lines);
        };
        let bg = |ui: &mut Ui, lines: &mut Vec<String>| {
            colour_row(ui, "BG Color", s.bg.color, "bg", lines);
            pct(ui, "BG Alpha", s.bg.alpha, "bgAlpha", lines);
            value(ui, "BG Value", s.bg.value, "bgValue", lines);
        };
        ui.strong(tool.label());
        match tool {
            PaintTool::Fill => {
                // PNT-107.
                blend(ui, lines);
                crate::option_row::field_row(ui, "Paint Mode", |ui| {
                    let _ = ui.gated(
                        Some("Fill always works on the image itself"),
                        egui::Button::new("Projection"),
                    );
                });
                ui.separator();
                falloff(ui, lines);
                ui.separator();
                pct(ui, "Opacity", s.opacity, "opacity", lines);
                pct(ui, "Density", s.density, "density", lines);
                pct(
                    ui,
                    "Fill Tolerance",
                    s.fill_tolerance,
                    "fillTolerance",
                    lines,
                );
                ui.separator();
                fg(ui, lines);
                return;
            }
            PaintTool::LinearGradient | PaintTool::RadialGradient => {
                // PNT-110.
                blend(ui, lines);
                ui.separator();
                falloff(ui, lines);
                ui.separator();
                pct(ui, "Opacity", s.opacity, "opacity", lines);
                ui.separator();
                fg(ui, lines);
                ui.separator();
                bg(ui, lines);
                return;
            }
            PaintTool::Line => {
                // PNT-103.
                pct(ui, "Interpolation Step", s.step, "step", lines);
                ui.separator();
                crate::option_row::field_row(ui, "Shape", |ui| {
                    egui::ComboBox::from_id_salt("paint-line-shape")
                        .selected_text(s.shape.label())
                        .show_ui(ui, |ui| {
                            for sh in operandi_core::paint_stroke::LineShape::ALL {
                                if ui.selectable_label(s.shape == sh, sh.label()).clicked() {
                                    lines.push(format!(
                                        "operandi.paint.set lineShape:{}",
                                        sh.label().to_ascii_lowercase()
                                    ));
                                }
                            }
                        });
                });
                ui.separator();
            }
            _ => {
                pct(ui, "Interpolation Step", s.step, "step", lines);
                curve(ui, lines);
                ui.separator();
            }
        }
        blend(ui, lines);
        pct(ui, "Opacity", s.opacity, "opacity", lines);
        pct(ui, "Density", s.density, "density", lines);
        attenuation(ui, lines);
        falloff(ui, lines);
        match tool {
            PaintTool::Airbrush | PaintTool::Paintbrush => {
                ui.separator();
                fg(ui, lines);
                ui.separator();
                bg(ui, lines);
            }
            PaintTool::Line => {
                ui.separator();
                fg(ui, lines);
            }
            PaintTool::Eraser => {
                ui.separator();
                bg(ui, lines);
            }
            PaintTool::Clone => {
                // PNT-97: only the screen-matched copy is built.
                crate::option_row::field_row(ui, "Use View Projection", |ui| {
                    let _ = ui.gated(
                        Some("Copying through the UVs comes later"),
                        egui::Button::new("On"),
                    );
                });
            }
            _ => {}
        }
        self.tip_rows(ui, lines);
        if self.painting.brush.tips.nozzle_on {
            self.nozzle_rows(ui, lines);
        }
        if crate::painting_inks::takes_inks(tool) {
            self.ink_rows(ui, lines);
        }
    }

    /// The active tip's rows (PNT-80, 81, 130..134).
    fn tip_rows(&self, ui: &mut Ui, lines: &mut Vec<String>) {
        let tips = self.painting.brush.tips.clone();
        self.tip_rows_for(ui, lines, &tips);
    }

    /// The rows of tips `tips` (the paint tools' or the sculpt brushes'),
    /// as `operandi.paint.tipSet` lines.
    pub(crate) fn tip_rows_for(&self, ui: &mut Ui, lines: &mut Vec<String>, tips: &Brush) {
        use operandi_core::paint_tips::{ImageMode, PatternType};
        let tip = tips.tip();
        let pct = |ui: &mut Ui, label: &str, v: f64, line: &str, lines: &mut Vec<String>| {
            crate::option_row::field_row(ui, label, |ui| {
                let mut x = v * 100.;
                if ui
                    .add(
                        crate::numeric_tab::DragValue::new(&mut x)
                            .range(0. ..=500.)
                            .suffix(" %"),
                    )
                    .changed()
                {
                    lines.push(format!("{line}:{}", x / 100.));
                }
            });
        };
        let check = |ui: &mut Ui, label: &str, on: bool, line: &str, lines: &mut Vec<String>| {
            crate::option_row::field_row(ui, label, |ui| {
                let mut v = on;
                if ui.checkbox(&mut v, "").changed() {
                    lines.push(format!("{line}:{v}"));
                }
            });
        };
        let degrees = |ui: &mut Ui, v: f64, lines: &mut Vec<String>| {
            crate::option_row::field_row(ui, "Rotation", |ui| {
                let mut d = v.to_degrees();
                if ui
                    .add(
                        crate::numeric_tab::DragValue::new(&mut d)
                            .range(-360. ..=360.)
                            .suffix("°"),
                    )
                    .changed()
                {
                    lines.push(format!("operandi.paint.tipSet rotation:{d}"));
                }
            });
        };
        ui.separator();
        ui.strong(format!("{} Tip", tips.choice.label()));
        crate::option_row::field_row(ui, "Size", |ui| {
            let mut v = tip.size;
            if ui
                .add(
                    crate::numeric_tab::DragValue::new(&mut v)
                        .range(1. ..=self.painting.prefs.max_brush)
                        .suffix(" px"),
                )
                .changed()
            {
                lines.push(format!("operandi.paint.set size:{v}"));
            }
        });
        check(
            ui,
            "Auto Scale",
            tip.auto_scale,
            "operandi.paint.set autoScale",
            lines,
        );
        match tips.choice {
            TipChoice::Hard => {
                pct(
                    ui,
                    "Soft Border",
                    tip.soft_border,
                    "operandi.paint.set softBorder",
                    lines,
                );
                pct(ui, "Bias", tip.bias, "operandi.paint.set bias", lines);
            }
            TipChoice::Smooth => {
                pct(ui, "Bias", tip.bias, "operandi.paint.set bias", lines);
                crate::option_row::field_row(ui, "Shape Preset", |ui| {
                    egui::ComboBox::from_id_salt("paint-tip-shape")
                        .selected_text(tip.shape.label())
                        .show_ui(ui, |ui| {
                            for shape in TipShape::ALL.into_iter().chain([TipShape::Custom]) {
                                if ui
                                    .selectable_label(tip.shape == shape, shape.label())
                                    .clicked()
                                {
                                    lines.push(format!(
                                        "operandi.paint.set shape:{}",
                                        shape.label().replace(' ', "")
                                    ));
                                }
                            }
                        });
                });
                let c = tip.custom;
                ui.collapsing("Edit Brush Settings", |ui| {
                    for (label, name, v) in [
                        ("In Weight", "inWeight", c.in_weight),
                        ("Out Weight", "outWeight", c.out_weight),
                        ("In Slope", "inSlope", c.in_slope),
                        ("Out Slope", "outSlope", c.out_slope),
                    ] {
                        crate::option_row::field_row(ui, label, |ui| {
                            let mut x = v;
                            if ui
                                .add(
                                    crate::numeric_tab::DragValue::new(&mut x)
                                        .speed(0.01)
                                        .range(-4. ..=4.),
                                )
                                .changed()
                            {
                                lines.push(format!("operandi.paint.set {name}:{x}"));
                            }
                        });
                    }
                });
            }
            TipChoice::Procedural => {
                let p = tips.procedural;
                crate::option_row::field_row(ui, "Type", |ui| {
                    egui::ComboBox::from_id_salt("paint-tip-pattern")
                        .selected_text(p.pattern.label())
                        .show_ui(ui, |ui| {
                            for t in PatternType::ALL {
                                if ui.selectable_label(p.pattern == t, t.label()).clicked() {
                                    lines.push(format!(
                                        "operandi.paint.tipSet pattern:{}",
                                        t.label().to_ascii_lowercase()
                                    ));
                                }
                            }
                        });
                });
                check(
                    ui,
                    "Invert",
                    p.invert,
                    "operandi.paint.tipSet invert",
                    lines,
                );
                pct(
                    ui,
                    "Scale U",
                    p.scale[0],
                    "operandi.paint.tipSet scaleU",
                    lines,
                );
                pct(
                    ui,
                    "Scale V",
                    p.scale[1],
                    "operandi.paint.tipSet scaleV",
                    lines,
                );
                pct(
                    ui,
                    "Scale W",
                    p.scale[2],
                    "operandi.paint.tipSet scaleW",
                    lines,
                );
                pct(
                    ui,
                    "Amplitude",
                    p.amplitude,
                    "operandi.paint.tipSet amplitude",
                    lines,
                );
                pct(ui, "Bias", tip.bias, "operandi.paint.set bias", lines);
                pct(ui, "Gain", p.gain, "operandi.paint.tipSet gain", lines);
            }
            TipChoice::Sphere => {
                let sp = tips.sphere;
                pct(ui, "Bias", tip.bias, "operandi.paint.set bias", lines);
                check(
                    ui,
                    "Height Shading",
                    sp.height_shading,
                    "operandi.paint.tipSet heightShading",
                    lines,
                );
                pct(
                    ui,
                    "Flatness",
                    sp.flatness,
                    "operandi.paint.tipSet flatness",
                    lines,
                );
                pct(
                    ui,
                    "Specular",
                    sp.specular,
                    "operandi.paint.tipSet specular",
                    lines,
                );
            }
            TipChoice::Image | TipChoice::Stamp => {
                let im = *tips.image_settings();
                let images: Vec<(u64, String)> = self
                    .editor
                    .scene
                    .images
                    .iter()
                    .map(|i| (i.id, i.name.clone()))
                    .collect();
                crate::option_row::field_row(ui, "Image", |ui| {
                    let shown = im
                        .image
                        .and_then(|id| images.iter().find(|(i, _)| *i == id))
                        .map_or("(none)".to_owned(), |(_, n)| n.clone());
                    egui::ComboBox::from_id_salt("paint-tip-image")
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(im.image.is_none(), "(none)").clicked() {
                                lines.push("operandi.paint.tipSet image:none".into());
                            }
                            for (id, name) in &images {
                                if ui.selectable_label(im.image == Some(*id), name).clicked() {
                                    lines.push(format!("operandi.paint.tipSet image:{id}"));
                                }
                            }
                        });
                });
                if tips.choice == TipChoice::Image {
                    crate::option_row::field_row(ui, "Mode", |ui| {
                        egui::ComboBox::from_id_salt("paint-tip-mode")
                            .selected_text(im.mode.label())
                            .show_ui(ui, |ui| {
                                for m in ImageMode::ALL {
                                    if ui.selectable_label(im.mode == m, m.label()).clicked() {
                                        lines.push(format!(
                                            "operandi.paint.tipSet mode:{}",
                                            m.label().to_ascii_lowercase()
                                        ));
                                    }
                                }
                            });
                    });
                }
                pct(ui, "Bias", tip.bias, "operandi.paint.set bias", lines);
                pct(
                    ui,
                    "Offset",
                    im.offset,
                    "operandi.paint.tipSet offset",
                    lines,
                );
                degrees(ui, im.rotation, lines);
                pct(
                    ui,
                    "Soft Border",
                    im.soft_border,
                    "operandi.paint.tipSet border",
                    lines,
                );
                check(
                    ui,
                    "Use as Mask",
                    im.use_as_mask,
                    "operandi.paint.tipSet useAsMask",
                    lines,
                );
                check(
                    ui,
                    "Keep Aspect",
                    im.keep_aspect,
                    "operandi.paint.tipSet keepAspect",
                    lines,
                );
            }
            TipChoice::Text => {
                let t = &tips.text;
                crate::option_row::field_row(ui, "Text", |ui| {
                    // The typing is kept as a draft while the field has
                    // focus and committed when it loses it (Enter, Tab or a
                    // click away); rebuilding it from the tip every frame
                    // threw each keystroke away.
                    let draft = egui::Id::new("paint-tip-text-draft");
                    let mut text = ui
                        .data(|d| d.get_temp::<String>(draft))
                        .unwrap_or_else(|| t.text.clone());
                    let edit =
                        ui.add(egui::TextEdit::singleline(&mut text).id(draft.with("field")));
                    crate::numeric_tab::register(ui, &edit, Some(&text));
                    if edit.lost_focus() {
                        ui.data_mut(|d| d.remove::<String>(draft));
                        if text != t.text {
                            lines.push(format!(
                                "operandi.paint.tipSet text:\"{}\"",
                                text.replace('"', "'")
                            ));
                        }
                    } else if edit.has_focus() {
                        ui.data_mut(|d| d.insert_temp(draft, text));
                    } else {
                        ui.data_mut(|d| d.remove::<String>(draft));
                    }
                });
                crate::option_row::field_row(ui, "Font", |ui| {
                    egui::ComboBox::from_id_salt("paint-tip-font")
                        .selected_text(t.font.clone())
                        .show_ui(ui, |ui| {
                            for (name, _) in operandi_core::paint_tips::FONTS {
                                if ui.selectable_label(t.font == *name, *name).clicked() {
                                    lines.push(format!("operandi.paint.tipSet font:\"{name}\""));
                                }
                            }
                        });
                });
                check(ui, "Bold", t.bold, "operandi.paint.tipSet bold", lines);
                check(
                    ui,
                    "Italics",
                    t.italics,
                    "operandi.paint.tipSet italics",
                    lines,
                );
                degrees(ui, t.rotation, lines);
            }
        }
    }

    /// The Tablet Nozzle's rows (PNT-141); jitter rows greyed until Enable
    /// Jitter.
    pub(crate) fn nozzle_rows_for_sculpt(&self, ui: &mut Ui, lines: &mut Vec<String>) {
        self.nozzle_rows(ui, lines);
    }

    fn nozzle_rows(&self, ui: &mut Ui, lines: &mut Vec<String>) {
        use operandi_core::paint_tips::{Drive, Sensitivity};
        let n = self.painting.brush.tips.nozzle;
        ui.separator();
        ui.strong("Tablet Nozzle");
        crate::option_row::field_row(ui, "Pressure Sensitivity", |ui| {
            let label = |s: Sensitivity| match s {
                Sensitivity::Low => "Low",
                Sensitivity::Medium => "Medium",
                Sensitivity::High => "High",
            };
            egui::ComboBox::from_id_salt("paint-nozzle-sensitivity")
                .selected_text(label(n.sensitivity))
                .show_ui(ui, |ui| {
                    for s in [Sensitivity::Low, Sensitivity::Medium, Sensitivity::High] {
                        if ui.selectable_label(n.sensitivity == s, label(s)).clicked() {
                            lines.push(format!(
                                "operandi.paint.tipSet sensitivity:{}",
                                label(s).to_ascii_lowercase()
                            ));
                        }
                    }
                });
        });
        for (label, now, name) in [
            ("Strength", n.strength, "strengthBy"),
            ("Size", n.size, "sizeBy"),
        ] {
            crate::option_row::field_row(ui, label, |ui| {
                egui::ComboBox::from_id_salt(format!("paint-nozzle-{name}"))
                    .selected_text(now.label())
                    .show_ui(ui, |ui| {
                        for d in Drive::ALL {
                            if ui.selectable_label(now == d, d.label()).clicked() {
                                lines.push(format!(
                                    "operandi.paint.tipSet {name}:{}",
                                    d.label().to_ascii_lowercase()
                                ));
                            }
                        }
                    });
            });
        }
        crate::option_row::field_row(ui, "Rotation", |ui| {
            egui::ComboBox::from_id_salt("paint-nozzle-rotation")
                .selected_text(if n.align { "Align" } else { "None" })
                .show_ui(ui, |ui| {
                    if ui.selectable_label(!n.align, "None").clicked() {
                        lines.push("operandi.paint.tipSet align:false".into());
                    }
                    if ui.selectable_label(n.align, "Align").clicked() {
                        lines.push("operandi.paint.tipSet align:true".into());
                    }
                });
        });
        crate::option_row::field_row(ui, "Enable Jitter", |ui| {
            let mut on = n.jitter;
            if ui.checkbox(&mut on, "").changed() {
                lines.push(format!("operandi.paint.tipSet jitter:{on}"));
            }
        });
        let why = (!n.jitter).then_some("Turn on Enable Jitter first");
        for (label, v, name) in [
            ("Jitter Spacing", n.jitter_spacing, "jitterSpacing"),
            ("Position", n.jitter_position, "jitterPosition"),
            ("Size", n.jitter_size, "jitterSize"),
            ("Rotation", n.jitter_rotation, "jitterRotation"),
            ("Strength", n.jitter_strength, "jitterStrength"),
        ] {
            crate::option_row::field_row(ui, label, |ui| {
                ui.gated_ui(why, |ui| {
                    let mut x = v * 100.;
                    if ui
                        .add(
                            crate::numeric_tab::DragValue::new(&mut x)
                                .range(0. ..=500.)
                                .suffix(" %"),
                        )
                        .changed()
                    {
                        lines.push(format!("operandi.paint.tipSet {name}:{}", x / 100.));
                    }
                });
            });
        }
    }
}
