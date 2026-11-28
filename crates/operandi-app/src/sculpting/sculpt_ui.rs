//! Sculpting in the app (spec `sculpt-foundation`): the Sculpt toolbox tab,
//! the brush cursor, strokes in the viewport, the size and strength drags,
//! the brush's Tool Properties and the sculpt commands. The stroke machine
//! itself is `operandi_core::sculpt`.
//!
//! A sculpt brush is an app-held tool like the Create utilities: the app
//! keeps every brush's settings (user prefs, SC-95), each stroke ends in one
//! core command (one undo step, SC-90), and turning on any modeling tool
//! turns the brush off, and the other way round.
use super::*;
use crate::form::Unit;
use crate::theme::viewport as palette;
use crate::tool_form::{self as tf, ToolFields, ToolForm};
use operandi_core::sculpt::{
    BrushSettings, Dab, DensityMode, NormalMode, OffsetMode, SmoothMode, Stroke as SculptStroke,
    StrokeSetup, TipKind, TipShape,
};
use operandi_core::{SculptBrush, SculptSettings};
use serde::{Deserialize, Serialize};

/// How fast a right-drag resizes the brush: pixels of radius per pixel of
/// pointer travel to the right (SC-22; relative, SQ-06 v0).
pub(crate) const SIZE_RATE: f64 = 1.;
/// How fast a middle-drag changes Offset Amount: a fraction per pixel
/// (0.5 % per pixel, SC-23).
pub(crate) const STRENGTH_RATE: f64 = 0.005;
/// A drag shorter than this is a click (a right-click opens the menu).
const CLICK_SLOP: f32 = 3.;
/// Most dabs one pointer move may lay (a tiny step and a huge flick).
const MAX_DABS_PER_MOVE: usize = 4000;

/// The toolbox icon of `brush` (batches 12, 16 and 17).
pub(crate) fn brush_icon(brush: SculptBrush) -> icons::Icon {
    match brush {
        SculptBrush::Smooth => icons::Icon::SculptSmooth,
        SculptBrush::Push => icons::Icon::SculptPush,
        SculptBrush::Inflate => icons::Icon::SculptInflate,
        SculptBrush::Move => icons::Icon::SculptMove,
        SculptBrush::Fold => icons::Icon::SculptFold,
        SculptBrush::Clay => icons::Icon::SculptClay,
        SculptBrush::Smudge => icons::Icon::SculptSmudge,
        SculptBrush::Carve => icons::Icon::SculptCarve,
        SculptBrush::Scar => icons::Icon::SculptScar,
        SculptBrush::Flatten => icons::Icon::SculptFlatten,
        SculptBrush::TangentPinch => icons::Icon::SculptTangentPinch,
        SculptBrush::CenterPinch => icons::Icon::SculptCenterPinch,
        SculptBrush::Emboss => icons::Icon::SculptEmboss,
        SculptBrush::Attenuate => icons::Icon::SculptAttenuate,
        SculptBrush::Spin => icons::Icon::SculptSpin,
        SculptBrush::VmPaint => icons::Icon::VmapPaint,
        SculptBrush::VmSmooth => icons::Icon::VmapSmooth,
        SculptBrush::VmScaleDown => icons::Icon::VmapScaleDown,
        SculptBrush::VmErase => icons::Icon::VmapErase,
        SculptBrush::VmSetValue => icons::Icon::VmapSetValue,
    }
}

/// The brushes' input-map actions (no default keys, SQ-08, SCT-05).
pub(crate) fn action(brush: SculptBrush) -> &'static str {
    match brush {
        SculptBrush::Smooth => "Sculpt Smooth",
        SculptBrush::Push => "Sculpt Push",
        SculptBrush::Inflate => "Sculpt Inflate",
        SculptBrush::Move => "Sculpt Move",
        SculptBrush::Fold => "Sculpt Fold",
        SculptBrush::Clay => "Sculpt Clay",
        SculptBrush::Smudge => "Sculpt Smudge",
        SculptBrush::Carve => "Sculpt Carve",
        SculptBrush::Scar => "Sculpt Scar",
        SculptBrush::Flatten => "Sculpt Flatten",
        SculptBrush::TangentPinch => "Sculpt Tangent Pinch",
        SculptBrush::CenterPinch => "Sculpt Center Pinch",
        SculptBrush::Emboss => "Sculpt Emboss",
        SculptBrush::Attenuate => "Sculpt Attenuate",
        SculptBrush::Spin => "Sculpt Spin",
        SculptBrush::VmPaint => "Vertex Map Paint",
        SculptBrush::VmSmooth => "Vertex Map Smooth",
        SculptBrush::VmScaleDown => "Vertex Map Scale Down",
        SculptBrush::VmErase => "Vertex Map Erase",
        SculptBrush::VmSetValue => "Vertex Map Set Value",
    }
}

/// The Tool Properties header and the brush section's title (SC-101,
/// SCT-04).
pub(crate) fn title(brush: SculptBrush) -> &'static str {
    match brush {
        SculptBrush::Smooth => "Sculpt: Smooth",
        SculptBrush::Push => "Sculpt: Push",
        SculptBrush::Inflate => "Sculpt: Inflate",
        SculptBrush::Move => "Sculpt: Move",
        SculptBrush::Fold => "Sculpt: Fold",
        SculptBrush::Clay => "Sculpt: Clay",
        SculptBrush::Smudge => "Sculpt: Smudge",
        SculptBrush::Carve => "Sculpt: Carve",
        SculptBrush::Scar => "Sculpt: Scar",
        SculptBrush::Flatten => "Sculpt: Flatten",
        SculptBrush::TangentPinch => "Sculpt: Tangent Pinch",
        SculptBrush::CenterPinch => "Sculpt: Center Pinch",
        SculptBrush::Emboss => "Sculpt: Emboss",
        SculptBrush::Attenuate => "Sculpt: Attenuate",
        SculptBrush::Spin => "Sculpt: Spin",
        SculptBrush::VmPaint => "Vertex Map: Paint",
        SculptBrush::VmSmooth => "Vertex Map: Smooth",
        SculptBrush::VmScaleDown => "Vertex Map: Scale Down",
        SculptBrush::VmErase => "Vertex Map: Erase",
        SculptBrush::VmSetValue => "Vertex Map: Set Value",
    }
}

/// The surface under the pointer (world space), for the cursor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hover {
    pub item: u64,
    pub point: DVec3,
    pub normal: DVec3,
}

/// What the hover pick depended on.
#[derive(Clone, PartialEq)]
struct HoverKey {
    pointer: Pos2,
    rect: Rect,
    camera: [f64; 7],
    view: u8,
    meshes: Vec<(u64, operandi_core::Shared<operandi_core::Mesh>, glam::DMat4)>,
}

/// A stroke in progress.
pub(crate) struct LiveStroke {
    stroke: SculptStroke,
    /// Where the last dab went: pointer, surface point and normal. `None`
    /// while the pointer is off the mesh (the stroke pauses, SC-43).
    last: Option<(Pos2, DVec3, DVec3)>,
    /// Pointer travel since the last dab, in pixels.
    carry: f64,
    /// Move: the grab point at the press.
    anchor: DVec3,
    /// The brush radius in metres when Auto Scale fixes it.
    fixed_radius: Option<f64>,
    /// Pen pressure (0..1, after the curve) at the last dab; dabs between
    /// two pointer events blend from it to the new event's (SC-45).
    pressure: f64,
    /// Input > Pen Tablet: Pressure Sets Brush Size, read at the press.
    pressure_size: bool,
    /// Painting builds: the Tablet Nozzle's seed and the dabs laid
    /// (PNT-241).
    #[cfg(feature = "painting")]
    nozzle: (u32, u32),
}

/// Smallest share of the brush size pressure can shrink it to (SC-45
/// pressure->size; provisional, PTQ-04).
const MIN_PRESSURE_SIZE: f64 = 0.05;

impl LiveStroke {
    /// What `pressure` does to the brush radius: nothing unless Pressure
    /// Sets Brush Size is on.
    fn size_factor(&self, pressure: f64) -> f64 {
        if self.pressure_size {
            pressure.clamp(MIN_PRESSURE_SIZE, 1.)
        } else {
            1.
        }
    }
}

/// The pen pressure (0..1, curved) for a pointer event at `p`.
fn pen_pressure(p: Pos2) -> f64 {
    f64::from(crate::tablet::pressure_at([p.x, p.y])).clamp(0., 1.)
}

/// Brush settings as saved in the data folder.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Saved {
    pub sculpt: SculptSettings,
}

impl Saved {
    pub(crate) fn load(path: &std::path::Path) -> Self {
        let saved = std::fs::metadata(path)
            .ok()
            .filter(|m| m.len() <= 64 * 1024)
            .and_then(|_| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .unwrap_or_default();
        Self {
            sculpt: saved.sculpt.sanitized(),
        }
    }
    fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        crate::resource_write::write_bytes(path, &temp, &bytes)
    }
}

/// The New Weight Map form (VM-03). The starting name is a guess (VM-Q01).
pub(crate) struct NewMapDraft {
    pub name: String,
    pub init: bool,
    pub value: f64,
}

impl Default for NewMapDraft {
    fn default() -> Self {
        Self {
            name: "Weight Map".into(),
            init: false,
            value: 0.,
        }
    }
}

/// The app's sculpt state (boxed in `Operandi`: the app state is large).
#[derive(Default)]
pub(crate) struct SculptState {
    /// The active brush tool, if any.
    pub brush: Option<SculptBrush>,
    pub settings: SculptSettings,
    pub stroke: Option<LiveStroke>,
    /// The resize/strength drag's centre (where it began, SC-22).
    pub adjust_at: Option<Pos2>,
    /// The last settings written to the data folder.
    saved: Option<SculptSettings>,
    /// The New Weight Map form's fields (VM-03).
    pub new_map: NewMapDraft,
    hover: std::cell::RefCell<Option<(HoverKey, Option<Hover>)>>,
}

impl SculptState {
    /// The active brush's settings.
    pub(crate) fn active(&self) -> Option<(SculptBrush, &BrushSettings)> {
        self.brush.map(|b| (b, self.settings.brush(b)))
    }
}

/// Tool Properties for the active sculpt brush (SC-101, UIC-125).
pub(super) struct SculptFields;

fn pct(
    key: &str,
    label: &str,
    get: impl Fn(&BrushSettings) -> f64 + 'static,
    set: impl Fn(&mut BrushSettings, f64) + 'static,
) -> tf::Field<BrushSettings> {
    let name = label.to_owned();
    tf::percent(key, label, " %", get, move |p, v| {
        if !v.is_finite() {
            return Err(format!("{name} must be a number"));
        }
        set(p, v / 100.);
        p.validate()
    })
}

/// A row of a brush's form (SCT §3, SI §2); `Divider` separates groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Row {
    SculptMode,
    MoveMode,
    MoveBackFaces,
    InvertedFlatten,
    OffsetMode,
    OffsetDistance,
    OffsetAmount,
    Level,
    SmoothAmount,
    SmoothMode,
    NormalMode,
    DensityMode,
    Step,
    Continuous,
    CurveConstraint,
    NormalizeWeights,
    /// Smooth's Soft Filter (SCT-50): image-only, greyed.
    SoftFilter,
    /// Move's Rigid UVs (SCT-49): UV-only, greyed.
    RigidUvs,
    /// Inflate's Paint Mode (SCT-51, SI-20): image-only, greyed.
    PaintMode,
    /// Set Color, Color, Alpha (SI-21..23): image-only, greyed.
    SetColor,
    Color,
    Alpha,
    /// Lock Borders and Lock Corners (SCT-01, SI-24): the Sculpt tab's
    /// switches, at the bottom of every sculpt brush's form.
    Locks,
    Divider,
}

/// Why the image-only rows are greyed (SCT-07; shown greyed, not hidden:
/// Jason's "match", SI-Q3).
const IMAGE_ONLY: &str = "Image sculpting isn't in Operandi yet";

/// The rows of a brush's form, in order (SCT-12..48, SI-10..24): the brush
/// section and the Stroke section (empty: the Stroke rows sit in the brush
/// section after a divider).
pub(crate) fn toolbox_rows(brush: SculptBrush) -> (Vec<Row>, Vec<Row>) {
    use Row::*;
    let stroke = vec![Step, Continuous, CurveConstraint];
    let (mut rows, stroke) = match brush {
        // SI §2: one block with dividers.
        SculptBrush::Inflate => (
            vec![
                SculptMode,
                OffsetMode,
                OffsetDistance,
                OffsetAmount,
                SmoothAmount,
                SmoothMode,
                DensityMode,
                Divider,
                Step,
                Continuous,
                CurveConstraint,
                Divider,
                PaintMode,
                Divider,
                SetColor,
                Color,
                Alpha,
            ],
            vec![],
        ),
        // SPM-10..21: Inflate's rows with Normal Mode, no Paint Mode.
        SculptBrush::Push => (
            vec![
                SculptMode,
                OffsetMode,
                OffsetDistance,
                OffsetAmount,
                SmoothAmount,
                SmoothMode,
                NormalMode,
                DensityMode,
                Divider,
                Step,
                Continuous,
                CurveConstraint,
                Divider,
                SetColor,
                Color,
                Alpha,
            ],
            vec![],
        ),
        // SPM-22..24: Smooth's short form.
        SculptBrush::Smooth => (vec![SculptMode, OffsetAmount, SoftFilter], vec![]),
        // SPM-25..30: no offset-mode, smoothing-mode, density or stroke
        // rows (Move doesn't lay dabs).
        SculptBrush::Move => (
            vec![
                SculptMode,
                MoveMode,
                MoveBackFaces,
                OffsetAmount,
                SmoothAmount,
                RigidUvs,
            ],
            vec![],
        ),
        SculptBrush::Fold => (
            vec![
                SculptMode,
                Divider,
                OffsetMode,
                OffsetDistance,
                OffsetAmount,
                SmoothAmount,
                SmoothMode,
            ],
            stroke,
        ),
        SculptBrush::Clay => (
            vec![
                OffsetAmount,
                Level,
                SmoothAmount,
                SmoothMode,
                NormalMode,
                DensityMode,
            ],
            stroke,
        ),
        SculptBrush::Smudge => (
            vec![SculptMode, Divider, OffsetAmount, SmoothAmount, SmoothMode],
            vec![],
        ),
        SculptBrush::Carve => (
            vec![
                SculptMode,
                Divider,
                OffsetMode,
                OffsetDistance,
                OffsetAmount,
                SmoothAmount,
                SmoothMode,
                NormalMode,
                DensityMode,
            ],
            vec![Step, CurveConstraint, Continuous],
        ),
        SculptBrush::Scar => (vec![OffsetAmount, SmoothAmount, SmoothMode], vec![]),
        SculptBrush::Flatten => (
            vec![
                SculptMode,
                Divider,
                InvertedFlatten,
                OffsetMode,
                OffsetDistance,
                OffsetAmount,
                SmoothAmount,
                SmoothMode,
                NormalMode,
            ],
            stroke,
        ),
        SculptBrush::TangentPinch | SculptBrush::CenterPinch => (
            vec![
                SculptMode,
                Divider,
                OffsetMode,
                OffsetDistance,
                OffsetAmount,
                SmoothAmount,
                CurveConstraint,
            ],
            vec![],
        ),
        SculptBrush::Emboss => (vec![SculptMode, Divider, OffsetAmount], vec![]),
        SculptBrush::Attenuate => (
            vec![SculptMode, Divider, OffsetAmount, SmoothAmount, SmoothMode],
            vec![],
        ),
        SculptBrush::Spin => (
            vec![
                SculptMode,
                Divider,
                OffsetMode,
                OffsetDistance,
                OffsetAmount,
                SmoothAmount,
                NormalMode,
            ],
            vec![],
        ),
        // VMT-03: the Vertex Map Tools' forms (Erase has none, and they
        // have no locks).
        SculptBrush::VmPaint
        | SculptBrush::VmSmooth
        | SculptBrush::VmScaleDown
        | SculptBrush::VmSetValue => return (vec![OffsetAmount, NormalizeWeights], vec![]),
        SculptBrush::VmErase => return (vec![], vec![]),
    };
    // SCT-01, SI-24: the locks close every sculpt brush's form.
    rows.extend([Divider, Locks]);
    (rows, stroke)
}

/// What a form row reads besides the brush's settings.
struct RowCtx {
    /// A pen has come near a tablet this session (SC-62).
    pen_seen: bool,
}

/// The field for one brush row; `None` for dividers and the locks (drawn
/// by `locks_item`).
fn row_field(
    row: Row,
    brush: SculptBrush,
    p: &BrushSettings,
    ctx: &RowCtx,
) -> Option<tf::Field<BrushSettings>> {
    use operandi_core::sculpt::{CurveConstraint, MoveMode, SculptMode};
    let adaptive = p.offset_mode == OffsetMode::Adaptive;
    // Attenuate relabels the shared fields (SCT-45).
    let attenuate = brush == SculptBrush::Attenuate;
    // Move follows the pointer instead of laying dabs (SC-123, SCQ-01).
    let grab = brush.kind() == operandi_core::sculpt::BrushKind::Grab;
    let grab_why = "Move follows the pointer; Offset Amount scales how far (provisional, SCQ-01)";
    let no_dabs = "Move doesn't lay dabs";
    // The brushes the SI / SPM specs cover.
    let foundation = matches!(
        brush,
        SculptBrush::Inflate | SculptBrush::Push | SculptBrush::Smooth | SculptBrush::Move
    );
    Some(match row {
        Row::Divider | Row::Locks => return None,
        Row::SculptMode => {
            // SI-10, SPM: the four foundation brushes list three choices;
            // the toolbox brushes five (SCT §2, SI-Q2).
            let three = foundation;
            let modes: &[(SculptMode, &str)] = if three {
                &[
                    (SculptMode::Automatic, "Automatic"),
                    (SculptMode::Mesh, "Mesh"),
                    (SculptMode::ImageDetails, "Image Map (Details)"),
                ]
            } else {
                &[
                    (SculptMode::Automatic, "Automatic"),
                    (SculptMode::Mesh, "Mesh"),
                    (SculptMode::ImageFast, "Image Map (Fast)"),
                    (SculptMode::ImageDetails, "Image Map (Details)"),
                    (SculptMode::Curves, "Curves"),
                ]
            };
            let field = tf::choice(
                "sculpt_mode",
                "Sculpt Mode",
                modes,
                |p: &BrushSettings| p.sculpt_mode,
                |p, m| p.sculpt_mode = m,
            )
            .tip("What the brush sculpts. Operandi sculpts mesh points");
            // SCT-07: image-only choices greyed with a note.
            if three {
                field.greyed_option(2, IMAGE_ONLY)
            } else {
                field
                    .greyed_option(2, IMAGE_ONLY)
                    .greyed_option(3, IMAGE_ONLY)
                    .greyed_option(4, "Curve sculpting isn't in Operandi yet")
            }
        }
        Row::MoveMode => tf::choice(
            "sculpt_move_mode",
            "Move Mode",
            &[(MoveMode::Normal, "Normal"), (MoveMode::Slide, "Slide")],
            |p: &BrushSettings| p.move_mode,
            |p, m| p.move_mode = m,
        )
        .tip("Normal: points follow the pointer. Slide: they keep to the surface"),
        Row::MoveBackFaces => tf::check(
            "sculpt_move_back_faces",
            "Move Back Faces",
            |p: &BrushSettings| p.move_back_faces,
            |p, on| p.move_back_faces = on,
        )
        .tip("Also grab the points facing away inside the brush, so the far side moves too"),
        Row::InvertedFlatten => tf::check(
            "sculpt_inverted_flatten",
            "Inverted Flatten",
            |p: &BrushSettings| p.inverted_flatten,
            |p, on| p.inverted_flatten = on,
        )
        .tip("Off: lower what's above the plane. On: raise what's below it"),
        Row::OffsetMode => tf::choice(
            "sculpt_offset_mode",
            "Offset Mode",
            &[
                (OffsetMode::Adaptive, "Adaptive"),
                (OffsetMode::Absolute, "Absolute"),
            ],
            |p: &BrushSettings| p.offset_mode,
            |p, m| p.offset_mode = m,
        )
        .tip("Adaptive: a percentage of the brush size. Absolute: a fixed distance")
        .enabled_if(!grab, grab_why),
        Row::OffsetDistance => tf::number(
            "sculpt_offset_distance",
            "Offset Distance",
            Unit::Distance,
            |p: &BrushSettings| p.offset_distance,
            |p, v| {
                p.offset_distance = v;
                p.validate()
            },
        )
        .tip("How far a dab moves the points, whatever the brush size")
        .enabled_if(
            !grab && !adaptive,
            if grab {
                grab_why
            } else {
                "Offset Mode is Adaptive: Offset Amount sets the strength"
            },
        ),
        Row::OffsetAmount => {
            let label = match brush {
                SculptBrush::Attenuate => "Amount",
                SculptBrush::VmPaint => "Paint Value",
                SculptBrush::VmSmooth => "Smooth Amount",
                SculptBrush::VmScaleDown => "Scale Amount",
                SculptBrush::VmSetValue => "Value",
                _ => "Offset Amount",
            };
            let tip = match brush {
                SculptBrush::Smooth => "How strongly each dab smooths. Ctrl: keep volume",
                SculptBrush::Move => "How closely the grabbed points follow the pointer",
                _ => {
                    "Strength as a percentage of the brush size. Middle-drag in the viewport to change it"
                }
            };
            let shows_mode = toolbox_rows(brush).0.contains(&Row::OffsetMode);
            pct(
                "sculpt_offset_amount",
                label,
                |p| p.offset_amount,
                |p, v| p.offset_amount = v,
            )
            .tip(tip)
            .enabled_if(
                grab || adaptive || !shows_mode,
                "Offset Mode is Absolute: Offset Distance sets the strength",
            )
        }
        Row::NormalizeWeights => tf::check(
            "weight_normalize",
            "Normalize Weights",
            |p: &BrushSettings| p.normalize_weights,
            |p, on| p.normalize_weights = on,
        )
        .tip("With several weight maps, lower the others so a point's total stays at 100 %"),
        Row::Level => pct("sculpt_level", "Level", |p| p.level, |p, v| p.level = v)
            .tip("How high Clay builds, as a percentage of the brush radius"),
        Row::SmoothAmount => pct(
            "sculpt_smooth_amount",
            if attenuate { "Smooth" } else { "Smooth Amount" },
            |p| p.smooth_amount,
            |p, v| p.smooth_amount = v,
        )
        .tip("How strongly Shift (or Uniform) smooths"),
        Row::SmoothMode => {
            // SI-15: Shift Key, Low Pressure, Uniform.
            let mode = tf::choice(
                "sculpt_smooth_mode",
                "Smooth Mode",
                &[
                    (SmoothMode::ShiftKey, "Shift Key"),
                    (SmoothMode::LowPressure, "Low Pressure"),
                    (SmoothMode::Uniform, "Uniform"),
                ],
                |p: &BrushSettings| p.smooth_mode,
                |p, m| p.smooth_mode = m,
            )
            .tip(
                "Shift Key: Shift smooths. Low Pressure: light pen strokes smooth, \
                 hard ones sculpt. Uniform: every stroke smooths a little too",
            );
            // SC-62, SC-102 (UIC-117): Low Pressure needs a tablet; it greys
            // until a pen has come near one this session.
            if ctx.pen_seen {
                mode
            } else {
                mode.greyed_option(
                    1,
                    "Needs a pen tablet: bring the pen near the tablet to use it",
                )
            }
        }
        Row::NormalMode => tf::choice(
            "sculpt_normal_mode",
            "Normal Mode",
            &[
                (NormalMode::BrushCenter, "Brush Center"),
                (NormalMode::Vertex, "Vertex"),
                (NormalMode::Average, "Average"),
                (NormalMode::Screen, "Screen"),
                (NormalMode::MouseDown, "Mouse Down"),
                (NormalMode::X, "X"),
                (NormalMode::Y, "Y"),
                (NormalMode::Z, "Z"),
            ],
            |p: &BrushSettings| p.normal_mode,
            |p, m| p.normal_mode = m,
        )
        .tip(match brush {
            SculptBrush::Flatten => {
                "The plane's direction. Mouse Down: one plane for the whole stroke"
            }
            SculptBrush::Spin => "The axis the brush spins around",
            SculptBrush::Clay => "The direction Clay builds along",
            _ => "Which way the brush pushes",
        })
        .enabled_if(!grab, "Move doesn't use a direction"),
        Row::DensityMode => {
            // SI-16: Maximum Map is image-only, listed greyed (SCT-07). It
            // has no engine value, so it can't be picked.
            let modes: [(Option<DensityMode>, &str); 4] = [
                (Some(DensityMode::Additive), "Additive"),
                (Some(DensityMode::Attenuate), "Attenuated"),
                (Some(DensityMode::MaxStroke), "Maximum Stroke"),
                (None, "Maximum Map"),
            ];
            tf::choice(
                "sculpt_density_mode",
                "Density Mode",
                &modes,
                |p: &BrushSettings| Some(p.density_mode),
                |p, m| {
                    if let Some(m) = m {
                        p.density_mode = m;
                    }
                },
            )
            .tip(
                "Additive: crossing a stroke adds more. Attenuated: stops at the full offset. \
                 Maximum Stroke: never more than the strongest dab",
            )
            .greyed_option(3, IMAGE_ONLY)
            .enabled_if(!grab, no_dabs)
        }
        Row::Step => pct(
            "sculpt_step",
            "Interpolation Step",
            |p| p.step,
            |p, v| p.step = v,
        )
        .tip("Gap between dabs as a percentage of the brush size. Smaller is smoother")
        .enabled_if(!grab, no_dabs),
        Row::Continuous => tf::check(
            "sculpt_continuous",
            "Continuous Stroke",
            |p: &BrushSettings| p.continuous,
            |p, on| p.continuous = on,
        )
        .tip("Join the dabs into a solid line even with a large step")
        .enabled_if(!grab, no_dabs),
        Row::CurveConstraint => tf::choice(
            "sculpt_curve_constraint",
            "Curve Constraint",
            // SI-19 / SPM: 3D before 2D on the foundation brushes; SCT §2
            // lists 2D first on the toolbox brushes.
            if foundation {
                &[
                    (CurveConstraint::None, "None"),
                    (CurveConstraint::Background3D, "Background 3D"),
                    (CurveConstraint::Background2D, "Background 2D"),
                ]
            } else {
                &[
                    (CurveConstraint::None, "None"),
                    (CurveConstraint::Background2D, "Background 2D"),
                    (CurveConstraint::Background3D, "Background 3D"),
                ]
            },
            |p: &BrushSettings| p.curve_constraint,
            |p, c| p.curve_constraint = c,
        )
        .tip("Guide strokes along a curve")
        .greyed_option(1, "Background constraints come in a later version")
        .greyed_option(2, "Background constraints come in a later version"),
        // The image- and UV-only rows: shown greyed with why (SCT-07,
        // UIC-117, SI-Q3). They hold no setting until those features exist.
        Row::SoftFilter => tf::check("sculpt_soft_filter", "Soft Filter", |_| false, |_, _| {})
            .tip("Smooths an image map's detail more gently")
            .enabled_if(false, IMAGE_ONLY),
        Row::RigidUvs => tf::check("sculpt_rigid_uvs", "Rigid UVs", |_| false, |_, _| {})
            .tip("Keeps the moved points' UVs fixed")
            .enabled_if(false, "UV sculpting isn't in Operandi yet"),
        Row::PaintMode => tf::choice(
            "sculpt_paint_mode",
            "Paint Mode",
            &[((), "Automatic")],
            |_| (),
            |_, ()| {},
        )
        .tip("How image sculpting and image painting combine")
        .enabled_if(false, IMAGE_ONLY),
        Row::SetColor => tf::check("sculpt_set_color", "Set Color", |_| false, |_, _| {})
            .tip("Paint a colour into an image map while sculpting")
            .enabled_if(false, IMAGE_ONLY),
        Row::Color => tf::value("sculpt_color", "Color", "1.0 0.0 0.0")
            .tip("The colour Set Color paints")
            .enabled_if(false, IMAGE_ONLY),
        Row::Alpha => pct("sculpt_alpha", "Alpha", |_| 1., |_, _| {})
            .tip("How strongly Set Color paints")
            .enabled_if(false, IMAGE_ONLY),
    })
}

/// Lock Borders and Lock Corners (SC-72..74, SI-24): the Sculpt tab's
/// switches, so ticking one here ticks it there. Side by side on one row,
/// Borders left (SPM-01): a departure from UIC-115's stacked checkboxes,
/// approved by Jason 2026-11-28.
fn locks_item() -> tf::Item<BrushSettings> {
    tf::Item::Custom(std::rc::Rc::new(|app: &mut Operandi, ui: &mut Ui| {
        let mut changes = Vec::new();
        crate::option_row::row(ui, "", |ui| {
            for (corners, label, tip) in [
                (false, "Lock Borders", "Points on open edges never move"),
                (
                    true,
                    "Lock Corners",
                    "Border points used by one polygon never move",
                ),
            ] {
                let s = &app.sculpt.settings;
                let mut on = if corners {
                    s.lock_corners
                } else {
                    s.lock_borders
                };
                if ui.checkbox(&mut on, label).on_hover_text(tip).changed() {
                    changes.push((corners, on));
                }
            }
        });
        for (corners, on) in changes {
            app.set_sculpt_lock(corners, on);
        }
    }))
}

/// A brush's rows as form items, with dividers.
fn toolbox_items(
    rows: &[Row],
    brush: SculptBrush,
    p: &BrushSettings,
    ctx: &RowCtx,
) -> Vec<tf::Item<BrushSettings>> {
    let mut f = ToolForm::<BrushSettings>::new();
    for &row in rows {
        match (row, row_field(row, brush, p, ctx)) {
            (_, Some(field)) => {
                f.field(field);
            }
            (Row::Locks, None) => {
                f.push(locks_item());
            }
            (_, None) => {
                f.separator();
            }
        }
    }
    f.items
}

/// The tip's section title (SI §2: "Smooth Brush", or "Hard Brush" with
/// the hard tip).
pub(crate) fn tip_title(kind: TipKind) -> &'static str {
    match kind {
        TipKind::Smooth => "Smooth Brush",
        TipKind::Hard => "Hard Brush",
    }
}

/// The Brush Settings pop-up button (SI-34): In/Out Weight and Slope, the
/// tip's own falloff. Editing one makes the shape Custom. A popover under
/// the button (UIC-181) whose changes apply live (UIC-182); no undo step,
/// like every brush setting (SC-91).
fn brush_settings_item() -> tf::Item<BrushSettings> {
    tf::Item::Custom(std::rc::Rc::new(|app: &mut Operandi, ui: &mut Ui| {
        let Some(brush) = app.sculpt.brush else {
            return;
        };
        let response = crate::option_row::button(ui, "Brush Settings")
            .on_hover_text("Edit the falloff's In/Out Weight and Slope (makes the shape Custom)");
        // UIC-116: a button that opens a panel shows ▾ at its right end.
        ui.painter().text(
            response.rect.right_center() - egui::vec2(8., 0.),
            egui::Align2::RIGHT_CENTER,
            "▾",
            egui::TextStyle::Button.resolve(ui.style()),
            ui.visuals().text_color(),
        );
        egui::Popup::from_toggle_button_response(&response)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_min_width(220.);
                let c = app.sculpt.settings.brush(brush).tip.custom;
                // Weights are shares of full strength; slopes plain numbers
                // (the brush engine's CustomProfile, PNT-80).
                let rows: [(&str, f64, bool); 4] = [
                    ("In Weight", c.in_weight, true),
                    ("Out Weight", c.out_weight, true),
                    ("In Slope", c.in_slope, false),
                    ("Out Slope", c.out_slope, false),
                ];
                for (k, (label, v, weight)) in rows.into_iter().enumerate() {
                    let mut x = if weight { v * 100. } else { v };
                    let changed = crate::option_row::field_row(ui, label, |ui| {
                        let drag = if weight {
                            crate::numeric_tab::DragValue::new(&mut x)
                                .speed(0.5)
                                .range(0. ..=100.)
                                .suffix(" %")
                        } else {
                            crate::numeric_tab::DragValue::new(&mut x)
                                .speed(0.01)
                                .range(-4. ..=4.)
                        };
                        ui.add(drag).changed()
                    })
                    .inner;
                    if changed {
                        let tip = &mut app.sculpt.settings.brush_mut(brush).tip;
                        let v = if weight { x / 100. } else { x };
                        match k {
                            0 => tip.custom.in_weight = v,
                            1 => tip.custom.out_weight = v,
                            2 => tip.custom.in_slope = v,
                            _ => tip.custom.out_slope = v,
                        }
                        tip.shape = TipShape::Custom;
                    }
                }
            });
    }))
}

fn tip_section(p: &BrushSettings) -> Vec<tf::Item<BrushSettings>> {
    let mut f = ToolForm::<BrushSettings>::new();
    f.field(
        tf::number(
            "sculpt_size",
            "Size",
            Unit::Suffix(" px"),
            |p: &BrushSettings| p.tip.size,
            |p, v| {
                p.tip.size = v;
                p.validate()
            },
        )
        .tip("Brush radius in screen pixels. Right-drag in the viewport to change it"),
    )
    .field(
        tf::check(
            "sculpt_auto_scale",
            "Auto Scale",
            |p: &BrushSettings| p.tip.auto_scale,
            |p, on| {
                p.tip.auto_scale = on;
                // Measured again at the next stroke.
                p.tip.scale_reference = 0.;
            },
        )
        .tip("Keep the brush the same size on the model when zooming"),
    );
    let bias = pct("sculpt_bias", "Bias", |p| p.tip.bias, |p, v| p.tip.bias = v)
        .tip("Bends the falloff: 50 % leaves it as it is, higher makes it fuller");
    match p.tip.kind {
        TipKind::Smooth => {
            // SI-30..34: Bias, then Shape Preset and Brush Settings.
            let shapes: Vec<(TipShape, &str)> =
                TipShape::ALL.iter().map(|s| (*s, s.label())).collect();
            f.field(bias).field(
                tf::choice(
                    "sculpt_shape",
                    "Shape Preset",
                    &shapes,
                    |p: &BrushSettings| p.tip.shape,
                    |p, s| p.tip.shape = s,
                )
                // Brush Settings edits make the shape Custom (not a preset).
                .otherwise(TipShape::Custom.label())
                .tip("The falloff from the brush centre to its rim"),
            );
            f.push(brush_settings_item());
        }
        TipKind::Hard => {
            f.field(
                pct(
                    "sculpt_soft_border",
                    "Soft Border",
                    |p| p.tip.soft_border,
                    |p, v| p.tip.soft_border = v,
                )
                .tip("How much of the radius ramps down at the edge"),
            )
            .field(bias);
        }
    }
    f.items
}

impl ToolFields for SculptFields {
    type P = BrushSettings;
    fn params(app: &Operandi) -> Option<BrushSettings> {
        app.sculpt.active().map(|(_, s)| *s)
    }
    fn form(app: &Operandi, p: &BrushSettings) -> ToolForm<BrushSettings> {
        let mut f = ToolForm::new();
        let Some(brush) = app.sculpt.brush else {
            return f;
        };
        let ctx = RowCtx {
            pen_seen: crate::tablet::seen(),
        };
        let (rows, stroke) = toolbox_rows(brush);
        f.section(title(brush), false, toolbox_items(&rows, brush, p, &ctx));
        if !stroke.is_empty() {
            f.section("Stroke", false, toolbox_items(&stroke, brush, p, &ctx));
        }
        f.section(tip_title(p.tip.kind), false, tip_section(p));
        f
    }
    fn publish(app: &mut Operandi, p: BrushSettings) -> std::result::Result<(), String> {
        // SC-91: settings affect the next stroke; no mesh change, no undo step.
        p.validate()?;
        let brush = app.sculpt.brush.ok_or("No sculpt brush is active")?;
        *app.sculpt.settings.brush_mut(brush) = p;
        Ok(())
    }
}

impl Operandi {
    // ------------------------------------------------------------ tool life

    /// Restore the brush settings (at launch).
    pub(super) fn load_sculpt_settings(&mut self) {
        let saved = Saved::load(&self.environment.paths().sculpt_settings());
        self.sculpt.settings = saved.sculpt.clone();
        self.sculpt.saved = Some(saved.sculpt);
    }

    /// Write the brush settings when they changed (SC-95).
    pub(super) fn save_sculpt_settings(&mut self) {
        let Some(saved) = &self.sculpt.saved else {
            return;
        };
        if *saved == self.sculpt.settings || self.sculpt.stroke.is_some() {
            return;
        }
        let now = Saved {
            sculpt: self.sculpt.settings.clone(),
        };
        if let Err(e) = now.save(&self.environment.paths().sculpt_settings()) {
            self.status = format!("Couldn't save the sculpt brush settings: {e}");
        }
        self.sculpt.saved = Some(now.sculpt);
    }

    /// A brush button or command: turn the brush on, or off when it is
    /// already on (SC-100, SC-93).
    pub(super) fn activate_sculpt(&mut self, brush: SculptBrush) {
        if self.sculpt.brush == Some(brush) {
            self.drop_sculpt();
            return;
        }
        if let Some(why) = brush.unavailable_on(&self.editor.scene, &self.sculpt_targets()) {
            // SCT-45: Attenuate needs a multiresolution level.
            self.status = format!("{}: {why}", brush.label());
            return;
        }
        if self.navigation.is_some() || !self.confirm_fields() {
            return;
        }
        self.finish_sculpt_stroke();
        #[cfg(feature = "painting")]
        self.drop_paint();
        self.drop_draw();
        self.utility = None;
        self.edge_click = None;
        self.vertex_weld = false;
        if self.editor.session.has_active_task() {
            self.run(Command::Drop);
            if self.editor.session.has_active_task() {
                return;
            }
        }
        self.clear_fields();
        self.sculpt.brush = Some(brush);
        if matches!(brush, SculptBrush::TangentPinch | SculptBrush::CenterPinch) {
            // SCT-02: the pinch slot shows the last-picked pinch.
            self.sculpt.settings.pinch_face = brush;
        }
        self.status = self.sculpt_hint(brush);
    }

    /// Space or picking another tool drops the brush (SC-93).
    pub(super) fn drop_sculpt(&mut self) {
        self.finish_sculpt_stroke();
        if let Some(brush) = self.sculpt.brush.take() {
            self.clear_fields();
            self.status = format!("{} off", brush.label());
        }
    }

    /// Called once a frame: a modeling or utility tool that came on turns
    /// the brush off.
    pub(super) fn keep_sculpt_exclusive(&mut self) {
        if self.sculpt.brush.is_some()
            && self.navigation.is_none()
            && (self.editor.session.has_active_task()
                || self.utility.is_some()
                || self.draw.is_some()
                || self.edge_click.is_some()
                || self.vertex_weld)
        {
            self.finish_sculpt_stroke();
            self.sculpt.brush = None;
        }
    }

    fn sculpt_hint(&self, brush: SculptBrush) -> String {
        format!(
            "{} · Drag on a mesh: sculpt · Ctrl: {} · Shift: smooth · Right-drag: size · Middle-drag: strength",
            title(brush),
            brush.ctrl_effect()
        )
    }

    // ------------------------------------------------------------ toolbox

    /// The Add Falloff route (VMF-10): `falloff` on top of the active
    /// falloff with `mix` (it becomes the active one when there is none);
    /// `on` false removes every falloff of that kind.
    pub(super) fn add_falloff(
        &mut self,
        falloff: operandi_core::Falloff,
        mix: operandi_core::MixMode,
        on: bool,
    ) {
        let same = |f: &operandi_core::Falloff| {
            std::mem::discriminant(f) == std::mem::discriminant(&falloff)
        };
        if on {
            if self.editor.spatial_falloff.is_none() {
                self.run(Command::SpatialFalloff(Some(falloff)));
            } else {
                let mut added = self.editor.added_falloffs.clone();
                added.push((falloff, mix));
                self.run(Command::AddedFalloffs(added));
            }
            return;
        }
        let added: Vec<_> = self
            .editor
            .added_falloffs
            .iter()
            .copied()
            .filter(|(f, _)| !same(f))
            .collect();
        if added != self.editor.added_falloffs {
            self.run(Command::AddedFalloffs(added));
        }
        if self.editor.spatial_falloff.as_ref().is_some_and(same) {
            self.run(Command::SpatialFalloff(None));
        }
    }

    /// VM-03 Create New Vertex Map, for weight maps: Name, Init Value and
    /// Value.
    fn new_weight_map_form(&mut self, ui: &mut Ui) {
        self.menu_owned_input = true;
        let draft = &mut self.sculpt.new_map;
        ui.horizontal(|ui| {
            crate::option_row::label(ui, "Name");
            ui.text_edit_singleline(&mut draft.name);
        });
        ui.checkbox(&mut draft.init, "Init Value");
        use crate::unavailable::Gate;
        ui.gated_ui(
            crate::unavailable::unless(
                draft.init,
                "Turn on Init Value to give every point a value",
            ),
            |ui| {
                ui.horizontal(|ui| {
                    crate::option_row::label(ui, "Value");
                    ui.add(crate::numeric_tab::DragValue::new(&mut draft.value).speed(0.01));
                });
            },
        );
        if ui.button("OK").clicked() {
            let cmd = Command::NewWeightMap {
                name: draft.name.clone(),
                value: draft.init.then_some(draft.value),
            };
            if self.run_ok(cmd) {
                ui.close();
            }
        }
    }

    /// One brush button (UIC-22..25). The pinch slot (`pinch`) is a pop-up
    /// button (UIC-25, SCT-02): a click runs the pinch on its face, and
    /// right-click or press-and-hold lists both.
    /// The Sculpt form's Pinch slot (escape hatch `sculpt.pinchSlot`): the
    /// remembered pinch brush, both on hold or right-click.
    pub(super) fn sculpt_pinch_slot(&mut self, ui: &mut Ui, width: f32) {
        let brush = self.sculpt.settings.pinch_face;
        self.sculpt_brush_button(ui, brush, width, true);
    }

    /// The Sculpt form's New Weight Map… button (escape hatch
    /// `sculpt.newWeightMap`): a menu holding its Name / Value form.
    pub(super) fn new_weight_map_button(&mut self, ui: &mut Ui) {
        let r = ui
            .menu_button("New Weight Map…", |ui| self.new_weight_map_form(ui))
            .response
            .on_hover_text("vertMap.new · add a weight map to the active meshes and select it");
        self.toolbox_rects.insert("New Weight Map".into(), r.rect);
    }

    fn sculpt_brush_button(&mut self, ui: &mut Ui, brush: SculptBrush, width: f32, pinch: bool) {
        let key = action(brush);
        let button = toolbox::CommandButton {
            key,
            label: brush.label(),
            style: toolbox::ButtonStyle::IconRow(brush_icon(brush)),
            size: vec2(width, toolbox::row_height()),
            active: self.sculpt.brush == Some(brush),
            unavailable: brush.unavailable_on(&self.editor.scene, &self.sculpt_targets()),
            shortcut: Some(key),
        };
        let response = self.command_button(ui, button);
        let mut chosen = None;
        if pinch {
            // UIC-25: the pop-up cue in the lower-right corner.
            let c = response.rect.right_bottom() - vec2(2., 2.);
            ui.painter().add(Shape::convex_polygon(
                vec![c, c - vec2(0., 4.), c - vec2(4., 0.)],
                theme::text(),
                Stroke::NONE,
            ));
            const HOLD_SECS: f64 = 0.8;
            let pressed_for = ui.input(|i| i.pointer.press_start_time().map(|t| i.time - t));
            let holding = response.is_pointer_button_down_on() && response.contains_pointer();
            let held = holding && pressed_for.is_some_and(|t| t > HOLD_SECS);
            if holding && !held {
                let left = HOLD_SECS - pressed_for.unwrap_or(0.) + 0.01;
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs_f64(left.max(0.01)));
            }
            let open =
                (response.secondary_clicked() || held).then_some(egui::SetOpenCommand::Bool(true));
            chosen = egui::Popup::menu(&response)
                .id(Id::new("sculpt-pinch-popup"))
                .open_memory(open)
                .show(|ui| {
                    self.menu_owned_input = true;
                    let mut picked = None;
                    for b in [SculptBrush::TangentPinch, SculptBrush::CenterPinch] {
                        if ui
                            .selectable_label(self.sculpt.brush == Some(b), b.label())
                            .clicked()
                        {
                            picked = Some(b);
                        }
                    }
                    if picked.is_some() {
                        ui.close();
                    }
                    picked
                })
                .and_then(|r| r.inner);
        }
        if response.clicked() {
            self.activate_sculpt(brush);
        }
        if let Some(b) = chosen
            && self.sculpt.brush != Some(b)
        {
            self.activate_sculpt(b);
        }
        let binding = self
            .bindings
            .get(key)
            .filter(|b| !b.is_empty())
            .map(|b| input::display_chord(b))
            .unwrap_or_else(|| "Unassigned".into());
        // UIC-04, UIC-27: name, shortcut, what it does and the modifiers.
        let more = if pinch {
            "\nRight-click or hold: Tangent Pinch or Center Pinch"
        } else {
            ""
        };
        response.on_hover_text(format!(
            "{} · {} · {binding}\n{}\nCtrl: {} · Shift: smooth{more}",
            brush.label(),
            brush.command(),
            brush.description(),
            brush.ctrl_effect()
        ));
    }

    /// The Tips row / `brush.smooth` / `brush.hard`: the active brush's tip.
    pub(super) fn set_sculpt_tip(&mut self, kind: TipKind) {
        let Some(brush) = self.sculpt.brush else {
            self.status = format!("{}: pick a sculpt brush first", kind.command());
            return;
        };
        if !self.confirm_fields() {
            return;
        }
        self.sculpt.settings.brush_mut(brush).tip.kind = kind;
        // Painting builds: Smooth or Hard is the sculpt brushes' tip again
        // (no Procedural, Texture… on top, PNT-151).
        #[cfg(feature = "painting")]
        {
            self.painting.brush.sculpt_tips.choice = match kind {
                TipKind::Hard => operandi_core::paint_tips::TipChoice::Hard,
                _ => operandi_core::paint_tips::TipChoice::Smooth,
            };
        }
        self.clear_fields();
        self.status = format!("{}: {} tip", brush.label(), kind.label());
    }

    /// Lock Borders (`corners` false) or Lock Corners.
    pub(super) fn set_sculpt_lock(&mut self, corners: bool, on: bool) {
        let s = &mut self.sculpt.settings;
        if corners {
            s.lock_corners = on;
        } else {
            s.lock_borders = on;
        }
        self.status = format!(
            "{} {}",
            if corners {
                "Lock Corners"
            } else {
                "Lock Borders"
            },
            if on { "on" } else { "off" }
        );
    }

    // ------------------------------------------------------------ commands

    /// Sculpt commands and action names; true when `text` was one.
    /// `tool.set sculpt.<brush> on|off|?`, `sculpt.<brush>` (toggle),
    /// `brush.smooth|hard`, `sculpt.borderLock|cornerLock [on|off]`.
    pub(super) fn sculpt_execute(&mut self, text: &str) -> bool {
        use operandi_core::command_line::{SculptLine, Switch, parse_sculpt};
        // Parsing is core's (command_line::parse_sculpt); the app runs it.
        let Some(line) = parse_sculpt(text) else {
            return false;
        };
        match line {
            SculptLine::Start(brush) => self.activate_sculpt(brush),
            SculptLine::ToolSet(brush, switch) => {
                let active = self.sculpt.brush == Some(brush);
                match switch {
                    Switch::On if !active => self.activate_sculpt(brush),
                    Switch::Off if active => self.drop_sculpt(),
                    Switch::Query => self.status = if active { "1" } else { "0" }.into(),
                    _ => {}
                }
            }
            SculptLine::Tip(kind) => self.set_sculpt_tip(kind),
            SculptLine::Lock { corners, switch } => {
                let now = if corners {
                    self.sculpt.settings.lock_corners
                } else {
                    self.sculpt.settings.lock_borders
                };
                match switch {
                    Switch::Toggle => self.set_sculpt_lock(corners, !now),
                    Switch::On => self.set_sculpt_lock(corners, true),
                    Switch::Off => self.set_sculpt_lock(corners, false),
                    Switch::Query => self.status = if now { "1" } else { "0" }.into(),
                }
            }
            SculptLine::NewWeightMap { name, value } => {
                self.run(Command::NewWeightMap { name, value });
            }
            SculptLine::SelectWeightMap(name) => self.run(Command::SelectWeightMap(name)),
            SculptLine::LiveDeformers(switch) => {
                let s = &mut self.sculpt.settings.live_deformers;
                match switch {
                    Switch::Toggle => *s = !*s,
                    Switch::On => *s = true,
                    Switch::Off => *s = false,
                    Switch::Query => {}
                }
                self.status = if *s { "1" } else { "0" }.into();
            }
            SculptLine::Falloff { falloff, mix, on } => match on {
                Some(on) => self.add_falloff(falloff, mix, on),
                None => {
                    let same = |f: &operandi_core::Falloff| {
                        std::mem::discriminant(f) == std::mem::discriminant(&falloff)
                    };
                    let present = self.editor.spatial_falloff.as_ref().is_some_and(same)
                        || self.editor.added_falloffs.iter().any(|(f, _)| same(f));
                    self.status = if present { "1" } else { "0" }.into();
                }
            },
            SculptLine::Usage(text) => self.status = text,
        }
        true
    }

    // ------------------------------------------------------------ viewport

    /// The world radius of a dab at `p` (SC-20, SC-21): Size pixels at
    /// that depth, or with Auto Scale the size fixed on the model.
    fn sculpt_radius(&mut self, p: DVec3, r: Rect) -> Option<f64> {
        let brush = self.sculpt.brush?;
        let mpp = self.camera.metres_per_pixel(p, r)?;
        let tip = &mut self.sculpt.settings.brush_mut(brush).tip;
        if tip.auto_scale {
            if !(tip.scale_reference.is_finite() && tip.scale_reference > 0.) {
                tip.scale_reference = mpp;
            }
            Some(tip.size * tip.scale_reference)
        } else {
            Some(tip.size * mpp)
        }
    }

    /// The radius in pixels the cursor shows at world point `p`.
    fn sculpt_radius_px(&self, p: DVec3, r: Rect) -> f32 {
        let Some((_, s)) = self.sculpt.active() else {
            return 0.;
        };
        let tip = &s.tip;
        if tip.auto_scale && tip.scale_reference > 0. {
            let mpp = self
                .camera
                .metres_per_pixel(p, r)
                .unwrap_or(tip.scale_reference);
            (tip.size * tip.scale_reference / mpp.max(1e-12)) as f32
        } else {
            tip.size as f32
        }
    }

    /// The unit vector from `p` toward the viewer.
    fn sculpt_view(&self, p: DVec3) -> DVec3 {
        let (eye, _, _, forward) = self.camera.basis();
        if self.camera.view == View::Perspective {
            (eye - p).normalize_or(-forward)
        } else {
            -forward
        }
    }

    /// The active meshes a brush works on (SC-03).
    pub(super) fn sculpt_targets(&self) -> Vec<u64> {
        self.editor.scene.active_meshes()
    }

    /// The surface of the active meshes under `p` (cached per pointer,
    /// view and mesh: the viewport redraws far more often).
    pub(super) fn sculpt_hover(&self, p: Pos2, r: Rect) -> Option<Hover> {
        let scene = &self.editor.scene;
        let targets = self.sculpt_targets();
        let c = &self.camera;
        let key = HoverKey {
            pointer: p,
            rect: r,
            camera: [
                c.target.x, c.target.y, c.target.z, c.distance, c.yaw, c.pitch, c.lens,
            ],
            view: c.view as u8,
            meshes: targets
                .iter()
                .filter_map(|&id| {
                    let item = scene.item(id)?;
                    Some((id, item.mesh.shared().clone(), scene.world_matrix(id).ok()?))
                })
                .collect(),
        };
        if let Some((k, hit)) = self.sculpt.hover.borrow().as_ref()
            && *k == key
        {
            return *hit;
        }
        let hit = (|| {
            if targets.is_empty() {
                return None;
            }
            let surface = self.pick_surface();
            let (item, face, depth) = surface.pick_in(&self.camera, p, r, Some(&targets))?;
            let mesh = &scene.item(item)?.mesh;
            if face as usize >= mesh.faces.len() {
                return None;
            }
            let world = scene.world_matrix(item).ok()?;
            let normal_matrix = glam::DMat3::from_mat4(world).inverse().transpose();
            let (origin, dir) = self.camera.ray(p, r);
            Some(Hover {
                item,
                point: origin + dir * depth,
                normal: (normal_matrix * mesh.normal(face as usize)).normalize_or_zero(),
            })
        })();
        *self.sculpt.hover.borrow_mut() = Some((key, hit));
        hit
    }

    /// A primary press with a brush active: start a stroke on the surface
    /// under the pointer (SC-40); off the mesh nothing happens (SC-43).
    /// Modifiers are read here and held for the stroke (SC section 7).
    pub(super) fn sculpt_press(&mut self, p: Pos2, r: Rect, mods: Modifiers) -> input::GestureKind {
        // A stroke still held (its release never came) is finished as its
        // own step before a new one starts (Astra P5-001).
        self.finish_sculpt_stroke();
        let Some(brush) = self.sculpt.brush else {
            return input::GestureKind::Blocked;
        };
        if self.editor.selection_start.is_some() {
            return input::GestureKind::Blocked;
        }
        let Some(hit) = self.sculpt_hover(p, r) else {
            self.status = format!("{}: press on an active mesh to sculpt", title(brush));
            return input::GestureKind::Blocked;
        };
        let Some(radius) = self.sculpt_radius(hit.point, r) else {
            return input::GestureKind::Blocked;
        };
        let settings = *self.sculpt.settings.brush(brush);
        // Painting builds: a mask brush paints the hidden mask map the
        // meshes borrow for the stroke (PNT-260).
        #[cfg(feature = "painting")]
        let mask = self.mask_brush_active().is_some();
        #[cfg(not(feature = "painting"))]
        let mask = false;
        // VMT-18 (provisional): with no weight map selected, the first
        // stroke makes one named "Weight" and paints it.
        if !mask && brush.is_vertex_map() && self.editor.selected_weight_map.is_none() {
            let name = self
                .editor
                .scene
                .active()
                .filter(|i| i.mesh.attributes.point_weights.contains_key("Weight"))
                .map(|_| Command::SelectWeightMap(Some("Weight".into())))
                .unwrap_or(Command::NewWeightMap {
                    name: "Weight".into(),
                    value: None,
                });
            if !self.run_ok(name) {
                return input::GestureKind::Blocked;
            }
        }
        // The pen sample for this press: its pressure (SC-45) and which
        // end touched. The eraser end acts like Ctrl (Invert) or Shift
        // (Smooth), as Input > Pen Tablet > Eraser End says.
        let pen = crate::tablet::sample_at([p.x, p.y]);
        let pressure = f64::from(pen.pressure).clamp(0., 1.);
        let eraser = pen.tool == crate::tablet::PenTool::Eraser;
        let eraser_action = self.input_preferences.pen_eraser_action;
        let setup = StrokeSetup {
            brush,
            settings,
            ctrl: mods.ctrl || (eraser && eraser_action == crate::tablet::EraserAction::Invert),
            shift: mods.shift || (eraser && eraser_action == crate::tablet::EraserAction::Smooth),
            symmetry: self.editor.active_symmetry(),
            falloff: self.editor.active_falloff(),
            lock_borders: self.sculpt.settings.lock_borders,
            lock_corners: self.sculpt.settings.lock_corners,
            radius,
            weight_map: self.editor.selected_weight_map.clone(),
        };
        #[cfg(feature = "painting")]
        let setup = if mask {
            let targets = self.sculpt_targets();
            self.lend_mask_map(&targets);
            StrokeSetup {
                weight_map: Some(operandi_core::paint_sculpt::MASK_MAP.into()),
                ..setup
            }
        } else {
            setup
        };
        #[cfg_attr(not(feature = "painting"), allow(unused_mut))]
        let mut stroke =
            match SculptStroke::begin(&self.editor.scene, &self.sculpt_targets(), setup) {
                Ok(Some(s)) => s,
                Ok(None) => {
                    self.status =
                        format!("{}: nothing to sculpt on the active meshes", title(brush));
                    return input::GestureKind::Blocked;
                }
                Err(e) => {
                    self.status = e;
                    return input::GestureKind::Blocked;
                }
            };
        // Painting builds: tips, inks and the mask (PNT-243..260).
        #[cfg(feature = "painting")]
        match self.sculpt_paint(hit.item, r) {
            Ok(p) => stroke.set_paint(std::sync::Arc::new(p)),
            Err(e) => {
                self.paint_notice(&e);
                self.finish_mask_stroke(None);
                return input::GestureKind::Blocked;
            }
        }
        let fixed_radius = settings.tip.auto_scale.then_some(radius);
        let mut live = LiveStroke {
            stroke,
            last: Some((p, hit.point, hit.normal)),
            carry: 0.,
            anchor: hit.point,
            fixed_radius,
            pressure,
            pressure_size: self.input_preferences.pen_pressure_size,
            #[cfg(feature = "painting")]
            nozzle: (self.painting.brush.stroke_seed(), 0),
        };
        let first = Dab {
            center: hit.point,
            radius: radius * live.size_factor(pressure),
            view: self.sculpt_view(hit.point),
            normal: hit.normal,
            pressure,
        };
        #[cfg(feature = "painting")]
        let first = {
            live.nozzle.1 += 1;
            self.sculpt_nozzle(first, live.nozzle.0, 0, 0.)
        };
        live.stroke.dab(first);
        if live.stroke.is_grab() && !live.stroke.grabbed() {
            self.status = format!("{}: nothing under the brush to grab", title(brush));
        } else {
            self.status = format!("{} · {}", title(brush), live.stroke.label());
        }
        live.stroke.flush(&mut self.editor.scene);
        self.sculpt.stroke = Some(live);
        input::GestureKind::Sculpt
    }

    /// A pointer move during a stroke: dabs along the path at Interpolation
    /// Step spacing (SC-41), or Move's grab following the pointer (SC-81).
    pub(super) fn sculpt_drag(&mut self, p: Pos2, r: Rect) {
        let Some(mut live) = self.sculpt.stroke.take() else {
            return;
        };
        if live.stroke.is_grab() {
            // In the plane facing the camera through the grab point.
            let (_, _, _, forward) = self.camera.basis();
            let (origin, dir) = self.camera.ray(p, r);
            let along = dir.dot(forward);
            if along.abs() > 1e-9 {
                let target = origin + dir * ((live.anchor - origin).dot(forward) / along);
                if live.stroke.acting() == SculptBrush::Emboss {
                    // SCT-39: only the horizontal travel counts.
                    let (_, right, _, _) = self.camera.basis();
                    live.stroke.emboss_to((target - live.anchor).dot(right));
                } else {
                    live.stroke.grab_to(target - live.anchor);
                }
            }
        } else {
            self.lay_dabs(&mut live, p, r);
        }
        // VMT-20 Live Deformers off: weights reach the scene on release.
        let held = live.stroke.acting().is_vertex_map() && !self.sculpt.settings.live_deformers;
        if !held && !live.stroke.flush(&mut self.editor.scene) {
            // The mesh went away under the stroke: keep what was done.
            self.sculpt.stroke = Some(live);
            self.finish_sculpt_stroke();
            return;
        }
        self.sculpt.stroke = Some(live);
    }

    fn lay_dabs(&mut self, live: &mut LiveStroke, p: Pos2, r: Rect) {
        let (origin, dir) = self.camera.ray(p, r);
        let Some(hit) = live.stroke.pick(origin, dir) else {
            // SC-43: off the surface the stroke pauses.
            live.last = None;
            return;
        };
        let spacing = live.stroke.setup().settings.spacing();
        // This pointer event's pen pressure; dabs blend to it from the last
        // dab's (SC-45).
        let to_pressure = pen_pressure(p);
        let radius_at = |app: &mut Self, at: DVec3| -> Option<f64> {
            match live.fixed_radius {
                Some(r) => Some(r),
                None => app.sculpt_radius(at, r),
            }
        };
        // The shared engine's spacing (brush_engine.rs, PNT-47).
        use operandi_core::brush_engine::{DabPlace, Spacing};
        let screen = |q: Pos2| glam::Vec2::new(q.x, q.y);
        let old_last = live.last;
        let mut spacer = Spacing {
            last: live.last.map(|(q, point, normal)| DabPlace {
                screen: screen(q),
                point,
                normal,
                pressure: live.pressure,
            }),
            carry: live.carry,
        };
        let to = DabPlace {
            screen: screen(p),
            point: hit.point,
            normal: hit.normal,
            pressure: to_pressure,
        };
        let size_factor = live.size_factor(to_pressure);
        let places = spacer.advance(
            to,
            || {
                let radius_px = self.sculpt_radius_px(hit.point, r).max(1.) as f64;
                // Dabs shrunk by pressure lie closer together (SC-41 is a
                // share of the dab's own size).
                (spacing * radius_px * size_factor).max(1.)
            },
            MAX_DABS_PER_MOVE,
        );
        for place in places {
            if let Some(radius) = radius_at(self, place.point) {
                let dab = Dab {
                    center: place.point,
                    radius: radius * live.size_factor(place.pressure),
                    view: self.sculpt_view(place.point),
                    normal: place.normal,
                    pressure: place.pressure,
                };
                #[cfg(feature = "painting")]
                let dab = {
                    let n = live.nozzle.1;
                    live.nozzle.1 += 1;
                    self.sculpt_nozzle(dab, live.nozzle.0, n, 0.)
                };
                live.stroke.dab(dab);
            }
        }
        live.carry = spacer.carry;
        live.last = spacer.last.map(|d| (p, d.point, d.normal));
        if live.last != old_last {
            live.pressure = to_pressure;
        }
    }

    /// The release (or a lost pointer): the stroke becomes one undo step
    /// named after the brush (SC-90, SC-92). The scene goes back to how it
    /// was at the press and the command sets the stroke's points, so the
    /// stroke is an ordinary edit.
    pub(super) fn finish_sculpt_stroke(&mut self) {
        let Some(live) = self.sculpt.stroke.take() else {
            return;
        };
        // Painting builds: a mask stroke becomes the mesh's mask and the
        // borrowed meshes go back (PNT-260).
        #[cfg(feature = "painting")]
        if !self.painting.brush.mask_lent.is_empty() {
            let weights = live.stroke.weight_edit();
            live.stroke.restore(&mut self.editor.scene);
            self.finish_mask_stroke(weights);
            return;
        }
        if let Some(weights) = live.stroke.weight_edit() {
            // VMT-19: a Vertex Map Tools stroke is one weight edit.
            live.stroke.restore(&mut self.editor.scene);
            if weights.is_empty() {
                return;
            }
            let label = weights.label.clone();
            if self.run_ok(Command::WeightStroke(Box::new(weights))) {
                self.status = label;
            }
            return;
        }
        let edit = live.stroke.edit();
        live.stroke.restore(&mut self.editor.scene);
        if edit.is_empty() {
            return;
        }
        let label = edit.label.clone();
        if self.run_ok(Command::SculptStroke(Box::new(edit))) {
            self.status = label;
        }
    }

    /// A right press with a brush active: resize it (SC-22), or a plain
    /// right-click for the context menu (UIC-60).
    pub(super) fn begin_sculpt_size(&mut self, p: Pos2) -> input::GestureKind {
        let Some(start) = self.sculpt.active().map(|(_, s)| s.tip.size) else {
            return input::GestureKind::Blocked;
        };
        self.sculpt.adjust_at = Some(p);
        input::GestureKind::SculptSize { start }
    }

    /// A middle press with a brush active: change its strength (SC-23).
    pub(super) fn begin_sculpt_strength(&mut self, p: Pos2) -> input::GestureKind {
        let Some(start) = self.sculpt.active().map(|(_, s)| s.offset_amount) else {
            return input::GestureKind::Blocked;
        };
        self.sculpt.adjust_at = Some(p);
        input::GestureKind::SculptStrength { start }
    }

    /// SC-22/23: drag right = more, left = less; the panel follows (SC-104).
    pub(super) fn sculpt_adjust_drag(
        &mut self,
        size: bool,
        start_value: f64,
        from: Pos2,
        to: Pos2,
    ) {
        let Some(brush) = self.sculpt.brush else {
            return;
        };
        let dx = (to.x - from.x) as f64;
        let s = self.sculpt.settings.brush_mut(brush);
        if size {
            s.tip.size = (start_value + dx * SIZE_RATE).round().clamp(
                operandi_core::sculpt::SIZE_RANGE.0,
                operandi_core::sculpt::SIZE_RANGE.1,
            );
            self.status = format!(
                "{} Size: {} px · drag right to grow, left to shrink",
                brush.label(),
                s.tip.size
            );
        } else {
            s.offset_amount = (start_value + dx * STRENGTH_RATE).clamp(0., 10.);
            self.status = format!(
                "{} Offset Amount: {:.1} % · drag right for more, left for less",
                brush.label(),
                s.offset_amount * 100.
            );
        }
        self.clear_fields();
    }

    /// The end of a size or strength drag; a right-click that didn't move
    /// opens the viewport menu (SC-22 note, UIC-60).
    pub(super) fn sculpt_adjust_release(&mut self, size: bool, start: Pos2, end: Pos2) {
        self.sculpt.adjust_at = None;
        if size
            && start.distance(end) < CLICK_SLOP
            && !self.mouse_region_click(start, self.viewport_rect)
        {
            self.selection_context_requested = true;
        }
    }

    /// SC-10..13, SC-71: the brush circle at the pointer on a mesh, a short
    /// line along the surface normal, and with Symmetry a second circle at
    /// the mirrored point. During a size or strength drag the circle stays
    /// where the drag began (SC-22).
    pub(super) fn draw_sculpt_cursor(&self, painter: &Painter, hover: Option<Pos2>, r: Rect) {
        let Some((_, settings)) = self.sculpt.active() else {
            return;
        };
        let color = palette::tweak_range();
        let ring = Stroke::new(1.5_f32, color);
        let faint = Stroke::new(1_f32, color.gamma_multiply(0.45));
        if let Some(at) = self.sculpt.adjust_at {
            let px = settings.tip.size as f32;
            painter.circle_stroke(at, px, ring);
            if matches!(
                self.navigation.as_ref().map(|g| &g.kind),
                Some(input::GestureKind::SculptStrength { .. })
            ) {
                painter.text(
                    at + vec2(px + 6., 0.),
                    Align2::LEFT_CENTER,
                    format!("{:.0} %", settings.offset_amount * 100.),
                    FontId::proportional(12.),
                    color,
                );
            }
            return;
        }
        let Some(p) = hover else {
            return;
        };
        let hit = match &self.sculpt.stroke {
            Some(live) => live.last.map(|(_, point, normal)| Hover {
                item: 0,
                point,
                normal,
            }),
            None => self.sculpt_hover(p, r),
        };
        let Some(hit) = hit else {
            return;
        };
        let mut px = self.sculpt_radius_px(hit.point, r);
        // SC-45: with Pressure Sets Brush Size the ring follows the pen
        // while drawing.
        if let Some(live) = &self.sculpt.stroke {
            px *= live.size_factor(live.pressure) as f32;
        }
        painter.circle_stroke(p, px, ring);
        // SC-12: where a Hard tip's full strength ends.
        if settings.tip.kind == TipKind::Hard && settings.tip.soft_border > 0.02 {
            painter.circle_stroke(p, px * (1. - settings.tip.soft_border as f32), faint);
        }
        // SC-11: the normal at the hit point, half a radius long.
        let length = self.camera.metres_per_pixel(hit.point, r).unwrap_or(0.) * px as f64 * 0.5;
        if let (Some((a, _)), Some((b, _))) = (
            self.camera.project(hit.point, r),
            self.camera.project(hit.point + hit.normal * length, r),
        ) {
            painter.line_segment([a, b], ring);
        }
        // SC-71: the mirrored circle, through the hit mesh's own plane.
        let symmetry = self.editor.active_symmetry();
        let item = if hit.item != 0 {
            Some(hit.item)
        } else {
            self.editor.scene.selection.item
        };
        if symmetry.axis.is_some()
            && let Some(item) = item
            && let Ok(world) = self.editor.scene.world_matrix(item)
        {
            let mirrored = world
                .transform_point3(symmetry.mirror(world.inverse().transform_point3(hit.point)));
            if let Some((q, _)) = self.camera.project(mirrored, r)
                && q.distance(p) > 1.
            {
                let mpx = self.sculpt_radius_px(mirrored, r);
                let mpx = if settings.tip.auto_scale {
                    mpx
                } else {
                    // Same world size as the main dab, seen at its depth.
                    let main = self.camera.metres_per_pixel(hit.point, r).unwrap_or(1.);
                    let here = self.camera.metres_per_pixel(mirrored, r).unwrap_or(main);
                    px * (main / here.max(1e-12)) as f32
                };
                painter.circle_stroke(q, mpx, Stroke::new(1.5_f32, palette::mirror_selection()));
            }
        }
    }
}
