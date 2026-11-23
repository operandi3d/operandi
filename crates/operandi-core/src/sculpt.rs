//! Sculpting foundation (spec `sculpt-foundation`, rules SC-01..SC-123).
//!
//! Every brush is the same machine with a different "what happens to a
//! vertex" rule plugged in (SC section 0). This module is that machine,
//! without a window: brush settings, the tip's falloff across the circle,
//! a spatial lookup of the points and polygons near the brush, the stroke
//! (dabs, density tracking, symmetry, locked borders) and the one command
//! that records a finished stroke as one undo step (SC-90).
//!
//! The app turns pointer moves into `Dab`s and grab deltas; a stroke edits
//! its own world-space copy of the points and writes the changed ones into
//! the scene once per frame (`Stroke::flush`), so the viewport patches only
//! what moved (SC-110/111). On release the app puts the scene back as it was
//! at the press and runs `Command::SculptStroke` with the points the stroke
//! changed, so the stroke is one ordinary, undoable, scriptable edit.
//!
//! Adding a brush: one `SculptBrush` variant with its label, command name,
//! kind and defaults, and its vertex rule in `Stroke::offset_dab` (offset
//! brushes) or a capture rule like Move's (grab brushes).
use crate::fxhash::FxHashMap;
use crate::sculpt_blender::{brushes, plane};
use crate::{Mesh, Result, Scene, Symmetry};
use glam::{DMat3, DMat4, DVec3};
use serde::{Deserialize, Serialize};

// The tips are the shared brush engine's (brush_engine.rs, PNT-42): one
// implementation for sculpt and paint.
pub use crate::brush_engine::{CustomProfile, SIZE_RANGE, Tip, TipKind, TipShape};

/// The sculpt brushes: the pilot four (SC-120..123) and the rest of the
/// toolbox (spec `sculpt-toolbox`, SCT-01). Each brush's vertex rule is in
/// `Stroke::offset_dab` (or a grab rule), built on the Blender-derived
/// maths in `crate::sculpt_blender`:
///
/// | Brush | Blender rule |
/// |---|---|
/// | Push, Carve, Fold | Draw (`brushes::draw`); Fold with a signed profile |
/// | Inflate | Inflate |
/// | Smooth | Smooth |
/// | Clay | Clay (plane from `plane::PlaneSampler`, never past it) |
/// | Smudge | Nudge (`brushes::tangent_offset`) |
/// | Scar | Crease: Draw plus a pinch toward the stroke line |
/// | Flatten | Plane (Scrape, or Fill with Inverted Flatten) |
/// | Tangent / Center Pinch | Pinch along the surface |
/// | Spin | Rotate |
/// | Move, Emboss | Grab; Emboss along each point's normal |
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SculptBrush {
    Smooth,
    Push,
    Inflate,
    Move,
    Fold,
    Clay,
    Smudge,
    Carve,
    Scar,
    Flatten,
    TangentPinch,
    CenterPinch,
    Emboss,
    Attenuate,
    Spin,
    // Vertex Map Tools (spec vertex-map-tools, VMT-01): they paint the
    // selected weight map instead of moving points (sculpt_weights.rs).
    VmPaint,
    VmSmooth,
    VmScaleDown,
    VmErase,
    VmSetValue,
}

/// Move Mode (SCT-49).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveMode {
    /// The foundation's screen-plane grab.
    #[default]
    Normal,
    /// Moved points keep to the original surface (provisional, SCT-Q14).
    Slide,
}

/// Sculpt Mode (SCT §2). Operandi sculpts meshes only, so Automatic and
/// Mesh both move the mesh's points; the image and curve modes are shown
/// greyed (SCT-07).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SculptMode {
    #[default]
    Automatic,
    Mesh,
    ImageFast,
    ImageDetails,
    Curves,
}

/// Curve Constraint (SCT §2). Background constraints are a later spec
/// (SCT-55), so only None works.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurveConstraint {
    #[default]
    None,
    Background2D,
    Background3D,
}

/// SCT-Q12 (Jason's call): whether Attenuate works on plain meshes by
/// fading toward the shape at the stroke's start. Off: the brush is greyed
/// until Operandi has sculpt layers (the spec's recommendation).
pub const ATTENUATE_ON_PLAIN_MESHES: bool = false;

/// SCT-13 Fold (provisional, SCT-Q05): where the profile crosses from in
/// to out, as a fraction of the radius.
pub const FOLD_CROSSOVER: f64 = 0.5;
/// SCT-48 Spin (provisional, SCT-Q13): radians a sweep of one radius turns
/// a full-weight point at Offset Amount 100 %.
pub const SPIN_RADIANS_PER_RADIUS: f64 = std::f64::consts::FRAC_PI_2;

/// What a brush does with a dab (SC-80, SC-81).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushKind {
    /// Each dab moves the points a little; dabs add up (Density Mode).
    Offset,
    /// The points under the brush at the press follow the pointer.
    Grab,
}

impl SculptBrush {
    /// Toolbox order, two to a row (SCT-01); Tangent and Center Pinch share
    /// one slot in the toolbox.
    pub const ALL: [SculptBrush; 15] = [
        Self::Push,
        Self::Inflate,
        Self::Fold,
        Self::Clay,
        Self::Smooth,
        Self::Smudge,
        Self::Carve,
        Self::Scar,
        Self::Flatten,
        Self::TangentPinch,
        Self::CenterPinch,
        Self::Emboss,
        Self::Attenuate,
        Self::Spin,
        Self::Move,
    ];
    /// The Vertex Map Tools section, in order (VMT-01).
    pub const VERTEX_MAP: [SculptBrush; 5] = [
        Self::VmPaint,
        Self::VmSmooth,
        Self::VmScaleDown,
        Self::VmErase,
        Self::VmSetValue,
    ];
    /// Every brush: the sculpt toolbox, then the Vertex Map Tools.
    pub fn every() -> impl Iterator<Item = SculptBrush> {
        Self::ALL.into_iter().chain(Self::VERTEX_MAP)
    }
    /// A Vertex Map Tool (paints weights, moves nothing).
    pub fn is_vertex_map(self) -> bool {
        Self::VERTEX_MAP.contains(&self)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::VmPaint => "Paint",
            Self::VmSmooth => "Smooth",
            Self::VmScaleDown => "Scale Down",
            Self::VmErase => "Erase",
            Self::VmSetValue => "Set Value",
            Self::Smooth => "Smooth",
            Self::Push => "Push",
            Self::Inflate => "Inflate",
            Self::Move => "Move",
            Self::Fold => "Fold",
            Self::Clay => "Clay",
            Self::Smudge => "Smudge",
            Self::Carve => "Carve",
            Self::Scar => "Scar",
            Self::Flatten => "Flatten",
            Self::TangentPinch => "Tangent Pinch",
            Self::CenterPinch => "Center Pinch",
            Self::Emboss => "Emboss",
            Self::Attenuate => "Attenuate",
            Self::Spin => "Spin",
        }
    }
    /// The tool's command name (spec section 14, SCT names addendum).
    pub fn command(self) -> &'static str {
        match self {
            Self::Smooth => "sculpt.smooth",
            Self::Push => "sculpt.push",
            Self::Inflate => "sculpt.inflate",
            Self::Move => "sculpt.move",
            Self::Fold => "sculpt.fold",
            Self::Clay => "sculpt.clay",
            Self::Smudge => "sculpt.smudge",
            Self::Carve => "sculpt.carve",
            Self::Scar => "sculpt.scar",
            Self::Flatten => "sculpt.flatten",
            Self::TangentPinch => "sculpt.tangentPinch",
            Self::CenterPinch => "sculpt.centerPinch",
            Self::Emboss => "sculpt.emboss",
            // SCT-04: the reference's long name is "Sculpt Erase".
            Self::Attenuate => "sculpt.shrink",
            Self::Spin => "sculpt.spin",
            // Registry names (vertex-map-tools reply, 2026-11-11).
            Self::VmPaint => "sculpt.vmapPaint",
            Self::VmSmooth => "sculpt.vmapSmooth",
            Self::VmScaleDown => "sculpt.vmapScale",
            Self::VmErase => "sculpt.vmapErase",
            Self::VmSetValue => "sculpt.vmapSetVal",
        }
    }
    /// The hover description (SCT-04, own words).
    pub fn description(self) -> &'static str {
        match self {
            Self::Smooth => "Relaxes the surface.",
            Self::Push => "Pushes the surface out along its normal.",
            Self::Inflate => "Swells the surface along each point's own normal.",
            Self::Move => "Grabs the surface under the brush and drags it.",
            Self::Fold => "Folds the surface into a hill and a valley.",
            Self::Clay => "Builds the surface up to a set level.",
            Self::Smudge => "Drags points along with the brush, smearing the surface.",
            Self::Carve => "Pushes points sharply inward.",
            Self::Scar => "Cuts a pinched gash into the surface.",
            Self::Flatten => "Flattens the surface to a plane.",
            Self::TangentPinch => {
                "Pinches points toward the stroke line, sliding along the surface."
            }
            Self::CenterPinch => {
                "Pinches points toward the brush centre, sliding along the surface."
            }
            Self::Emboss => {
                "Moves the area under the brush in or out along its normals by a set amount."
            }
            Self::Attenuate => "Fades stored sculpt detail back toward the unsculpted shape.",
            Self::Spin => "Swirls points around the brush centre.",
            Self::VmPaint => "Raises the selected weight map toward 100 %.",
            Self::VmSmooth => "Blurs the selected weight map's values.",
            Self::VmScaleDown => "Pulls the selected weight map's values toward 0.",
            Self::VmErase => "Removes the selected weight map's values.",
            Self::VmSetValue => "Writes exactly Value into the selected weight map.",
        }
    }
    /// Why the brush can't be used, when it can't (SCT-45, SCT-Q12).
    pub fn unavailable(self) -> Option<&'static str> {
        (self == Self::Attenuate && !ATTENUATE_ON_PLAIN_MESHES)
            .then_some("Needs a sculpt layer or multiresolution mesh")
    }
    pub fn from_command(name: &str) -> Option<Self> {
        Self::every().find(|b| b.command().eq_ignore_ascii_case(name))
    }
    pub fn kind(self) -> BrushKind {
        match self {
            Self::Move | Self::Emboss => BrushKind::Grab,
            _ => BrushKind::Offset,
        }
    }
    /// What Ctrl does with this brush (SC-60, tooltips SC-100, SCT §3).
    pub fn ctrl_effect(self) -> &'static str {
        match self {
            Self::Smooth => "keep volume",
            Self::Push => "push in",
            Self::Inflate => "deflate",
            Self::Move => "opposite direction",
            Self::Fold => "bump with a trough",
            Self::Clay => "cut down to the level",
            Self::Smudge => "slide the other way",
            Self::Carve => "raise a ridge",
            Self::Scar => "raised welt",
            Self::Flatten => "push up",
            Self::TangentPinch | Self::CenterPinch => "spread out",
            Self::Emboss => "reverse direction",
            Self::Attenuate => "nothing",
            Self::Spin => "spin the other way",
            Self::VmPaint => "lower toward 0",
            Self::VmSmooth => "sharpen",
            Self::VmScaleDown => "scale up",
            Self::VmErase => "erase",
            Self::VmSetValue => "erase",
        }
    }
    /// Whether the brush reads Normal Mode (SC-102: greyed on the others;
    /// Inflate always uses each point's own normal, SC-122).
    pub fn uses_normal_mode(self) -> bool {
        matches!(
            self,
            Self::Push | Self::Clay | Self::Carve | Self::Flatten | Self::Spin
        )
    }
    /// Whether the brush lays dabs whose effect adds up with travel, so
    /// each dab's share is divided by the dab spacing (SC-41). Brushes that
    /// move points toward a target (Clay, Flatten) or follow the pointer
    /// (Smudge) take their full strength per dab.
    fn spaced(self) -> bool {
        !matches!(
            self,
            Self::Smooth | Self::Clay | Self::Flatten | Self::Smudge | Self::Attenuate
        ) && !self.is_vertex_map()
    }
    /// The measured defaults (SC-25, SC-31, SC-41, SC-44, SC-50, SC-62).
    pub fn defaults(self) -> BrushSettings {
        let base = BrushSettings::default();
        match self {
            Self::Smooth => BrushSettings {
                offset_amount: 0.25,
                density_mode: DensityMode::Additive,
                normal_mode: NormalMode::Average,
                ..base
            },
            Self::Push => BrushSettings {
                offset_amount: 0.30,
                density_mode: DensityMode::Attenuate,
                normal_mode: NormalMode::Average,
                ..base
            },
            Self::Inflate => BrushSettings {
                offset_amount: 0.30,
                density_mode: DensityMode::Attenuate,
                normal_mode: NormalMode::Vertex,
                ..base
            },
            Self::Move => BrushSettings {
                offset_amount: 1.0,
                density_mode: DensityMode::Additive,
                normal_mode: NormalMode::BrushCenter,
                ..base
            },
            // SCT §3 starting values; fixed values for rows a form doesn't
            // show (SCT-06).
            Self::Fold | Self::Scar => BrushSettings {
                offset_amount: 0.25,
                density_mode: DensityMode::Additive,
                normal_mode: NormalMode::Average,
                ..base
            },
            Self::Clay | Self::Carve => BrushSettings {
                offset_amount: 0.5,
                density_mode: DensityMode::Attenuate,
                normal_mode: NormalMode::Average,
                ..base
            },
            Self::Smudge => BrushSettings {
                offset_amount: 0.5,
                density_mode: DensityMode::Additive,
                normal_mode: NormalMode::Vertex,
                ..base
            },
            Self::Flatten => BrushSettings {
                offset_amount: 0.5,
                density_mode: DensityMode::Additive,
                normal_mode: NormalMode::BrushCenter,
                ..base
            },
            Self::TangentPinch | Self::CenterPinch => BrushSettings {
                offset_amount: 0.25,
                density_mode: DensityMode::Additive,
                normal_mode: NormalMode::Vertex,
                ..base
            },
            Self::Emboss => BrushSettings {
                offset_amount: 0.25,
                density_mode: DensityMode::Additive,
                normal_mode: NormalMode::BrushCenter,
                ..base
            },
            Self::Attenuate => BrushSettings {
                offset_amount: 0.25,
                density_mode: DensityMode::Additive,
                normal_mode: NormalMode::Average,
                ..base
            },
            Self::Spin => BrushSettings {
                offset_amount: 0.25,
                density_mode: DensityMode::Additive,
                normal_mode: NormalMode::BrushCenter,
                ..base
            },
            // VMT-03..05: Paint Value, Smooth Amount and Scale Amount start
            // at 50 %, Set Value's Value at 0 %; Normalize on.
            Self::VmPaint | Self::VmScaleDown => BrushSettings {
                offset_amount: 0.5,
                density_mode: DensityMode::Attenuate,
                ..base
            },
            Self::VmSmooth | Self::VmErase => BrushSettings {
                offset_amount: 0.5,
                density_mode: DensityMode::Additive,
                ..base
            },
            Self::VmSetValue => BrushSettings {
                offset_amount: 0.,
                density_mode: DensityMode::Additive,
                ..base
            },
        }
    }
}

/// SC-24: how strength is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OffsetMode {
    /// A fraction of the brush's world radius (Offset Amount).
    #[default]
    Adaptive,
    /// A fixed distance (Offset Distance).
    Absolute,
}

/// SC-44: what crossing the same spot twice in one stroke does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DensityMode {
    #[default]
    Additive,
    Attenuate,
    MaxStroke,
}

/// SC-50: which way "out" is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalMode {
    BrushCenter,
    Vertex,
    #[default]
    Average,
    Screen,
    MouseDown,
    X,
    Y,
    Z,
}

/// SC-62.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmoothMode {
    #[default]
    ShiftKey,
    Uniform,
    LowPressure,
}

/// One brush's settings (SC-95: kept per brush).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BrushSettings {
    pub offset_mode: OffsetMode,
    /// Absolute strength in metres.
    pub offset_distance: f64,
    /// Adaptive strength as a fraction of the world radius (0.3 = 30 %);
    /// Smooth's strength and Move's follow factor.
    pub offset_amount: f64,
    /// Shift-smooth strength (SC-61), 0..1.
    pub smooth_amount: f64,
    pub smooth_mode: SmoothMode,
    pub normal_mode: NormalMode,
    pub density_mode: DensityMode,
    /// SC-41 Interpolation Step: gap between dabs, a fraction of the radius.
    pub step: f64,
    /// SC-42.
    pub continuous: bool,
    pub tip: Tip,
    /// Clay's Level (SCT-14): the build-up plane's height above the brush
    /// plane, a fraction of the radius.
    pub level: f64,
    /// Flatten's Inverted Flatten (SCT-30): raise what's below the plane
    /// instead of lowering what's above.
    pub inverted_flatten: bool,
    /// Move's Move Mode (SCT-49).
    pub move_mode: MoveMode,
    pub sculpt_mode: SculptMode,
    pub curve_constraint: CurveConstraint,
    /// Vertex Map Tools' Normalize Weights (VMT-11).
    pub normalize_weights: bool,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            normalize_weights: true,
            sculpt_mode: SculptMode::Automatic,
            curve_constraint: CurveConstraint::None,
            level: 0.1,
            inverted_flatten: false,
            move_mode: MoveMode::Normal,
            offset_mode: OffsetMode::Adaptive,
            offset_distance: 0.1,
            offset_amount: 0.3,
            smooth_amount: 0.5,
            smooth_mode: SmoothMode::ShiftKey,
            normal_mode: NormalMode::Average,
            density_mode: DensityMode::Additive,
            step: 0.05,
            continuous: false,
            tip: Tip::default(),
        }
    }
}

impl BrushSettings {
    /// Refuses values a stroke can't use (field edits, scripts, prefs).
    pub fn validate(&self) -> Result<()> {
        let finite = [
            self.offset_distance,
            self.offset_amount,
            self.smooth_amount,
            self.step,
            self.tip.size,
            self.tip.soft_border,
            self.tip.bias,
            self.tip.scale_reference,
            self.level,
        ]
        .iter()
        .all(|v| v.is_finite());
        if !finite {
            return Err("Brush values must be finite numbers".into());
        }
        if !(SIZE_RANGE.0..=SIZE_RANGE.1).contains(&self.tip.size) {
            return Err("Size must be from 1 to 2000 pixels".into());
        }
        if !(0. ..=1e6).contains(&self.offset_distance) {
            return Err("Offset Distance must be from 0 to 1000 km".into());
        }
        if !(-10. ..=10.).contains(&self.offset_amount) {
            return Err("Offset Amount must be from -1000 % to 1000 %".into());
        }
        if !(0. ..=1.).contains(&self.smooth_amount) {
            return Err("Smooth Amount must be from 0 % to 100 %".into());
        }
        if !(0.01..=10.).contains(&self.step) {
            return Err("Interpolation Step must be from 1 % to 1000 %".into());
        }
        if !(0. ..=1.).contains(&self.tip.soft_border) {
            return Err("Soft Border must be from 0 % to 100 %".into());
        }
        if !(0. ..=1.).contains(&self.tip.bias) {
            return Err("Bias must be from 0 % to 100 %".into());
        }
        // SCT-08: Level may be typed past 100 % (provisional range, SCT-Q03).
        if !(0. ..=10.).contains(&self.level) {
            return Err("Level must be from 0 % to 1000 %".into());
        }
        if !matches!(self.sculpt_mode, SculptMode::Automatic | SculptMode::Mesh) {
            return Err("Operandi sculpts meshes only: use Automatic or Mesh".into());
        }
        if self.curve_constraint != CurveConstraint::None {
            return Err("Background curve constraints aren't available yet".into());
        }
        Ok(())
    }
    /// The gap between dabs as a fraction of the radius. Continuous Stroke
    /// joins far-apart dabs into a solid line by filling in at a tenth of
    /// the radius at most (SC-42, provisional reading).
    pub fn spacing(&self) -> f64 {
        let step = self.step.clamp(0.01, 10.);
        if self.continuous { step.min(0.1) } else { step }
    }
}

/// Every brush's settings plus the global sculpt options (SC-72..74, SC-95).
/// User prefs, not scene data (SC-94).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SculptSettings {
    pub smooth: BrushSettings,
    pub push: BrushSettings,
    pub inflate: BrushSettings,
    #[serde(rename = "move")]
    pub grab: BrushSettings,
    pub fold: BrushSettings,
    pub clay: BrushSettings,
    pub smudge: BrushSettings,
    pub carve: BrushSettings,
    pub scar: BrushSettings,
    pub flatten: BrushSettings,
    pub tangent_pinch: BrushSettings,
    pub center_pinch: BrushSettings,
    pub emboss: BrushSettings,
    pub attenuate: BrushSettings,
    pub spin: BrushSettings,
    pub vm_paint: BrushSettings,
    pub vm_smooth: BrushSettings,
    pub vm_scale_down: BrushSettings,
    pub vm_erase: BrushSettings,
    pub vm_set_value: BrushSettings,
    pub lock_borders: bool,
    pub lock_corners: bool,
    /// The pinch slot's face: Tangent or Center Pinch, whichever was
    /// picked last (SCT-02).
    pub pinch_face: SculptBrush,
    /// Vertex Map Tools' Live Deformers (VMT-20): on, what the map drives
    /// updates during a stroke; off, once on release.
    pub live_deformers: bool,
}

impl Default for SculptSettings {
    fn default() -> Self {
        Self {
            smooth: SculptBrush::Smooth.defaults(),
            push: SculptBrush::Push.defaults(),
            inflate: SculptBrush::Inflate.defaults(),
            grab: SculptBrush::Move.defaults(),
            fold: SculptBrush::Fold.defaults(),
            clay: SculptBrush::Clay.defaults(),
            smudge: SculptBrush::Smudge.defaults(),
            carve: SculptBrush::Carve.defaults(),
            scar: SculptBrush::Scar.defaults(),
            flatten: SculptBrush::Flatten.defaults(),
            tangent_pinch: SculptBrush::TangentPinch.defaults(),
            center_pinch: SculptBrush::CenterPinch.defaults(),
            emboss: SculptBrush::Emboss.defaults(),
            attenuate: SculptBrush::Attenuate.defaults(),
            spin: SculptBrush::Spin.defaults(),
            vm_paint: SculptBrush::VmPaint.defaults(),
            vm_smooth: SculptBrush::VmSmooth.defaults(),
            vm_scale_down: SculptBrush::VmScaleDown.defaults(),
            vm_erase: SculptBrush::VmErase.defaults(),
            vm_set_value: SculptBrush::VmSetValue.defaults(),
            lock_borders: false,
            lock_corners: true,
            pinch_face: SculptBrush::TangentPinch,
            live_deformers: true,
        }
    }
}

impl SculptSettings {
    pub fn brush(&self, brush: SculptBrush) -> &BrushSettings {
        match brush {
            SculptBrush::Smooth => &self.smooth,
            SculptBrush::Push => &self.push,
            SculptBrush::Inflate => &self.inflate,
            SculptBrush::Move => &self.grab,
            SculptBrush::Fold => &self.fold,
            SculptBrush::Clay => &self.clay,
            SculptBrush::Smudge => &self.smudge,
            SculptBrush::Carve => &self.carve,
            SculptBrush::Scar => &self.scar,
            SculptBrush::Flatten => &self.flatten,
            SculptBrush::TangentPinch => &self.tangent_pinch,
            SculptBrush::CenterPinch => &self.center_pinch,
            SculptBrush::Emboss => &self.emboss,
            SculptBrush::Attenuate => &self.attenuate,
            SculptBrush::Spin => &self.spin,
            SculptBrush::VmPaint => &self.vm_paint,
            SculptBrush::VmSmooth => &self.vm_smooth,
            SculptBrush::VmScaleDown => &self.vm_scale_down,
            SculptBrush::VmErase => &self.vm_erase,
            SculptBrush::VmSetValue => &self.vm_set_value,
        }
    }
    pub fn brush_mut(&mut self, brush: SculptBrush) -> &mut BrushSettings {
        match brush {
            SculptBrush::Smooth => &mut self.smooth,
            SculptBrush::Push => &mut self.push,
            SculptBrush::Inflate => &mut self.inflate,
            SculptBrush::Move => &mut self.grab,
            SculptBrush::Fold => &mut self.fold,
            SculptBrush::Clay => &mut self.clay,
            SculptBrush::Smudge => &mut self.smudge,
            SculptBrush::Carve => &mut self.carve,
            SculptBrush::Scar => &mut self.scar,
            SculptBrush::Flatten => &mut self.flatten,
            SculptBrush::TangentPinch => &mut self.tangent_pinch,
            SculptBrush::CenterPinch => &mut self.center_pinch,
            SculptBrush::Emboss => &mut self.emboss,
            SculptBrush::Attenuate => &mut self.attenuate,
            SculptBrush::Spin => &mut self.spin,
            SculptBrush::VmPaint => &mut self.vm_paint,
            SculptBrush::VmSmooth => &mut self.vm_smooth,
            SculptBrush::VmScaleDown => &mut self.vm_scale_down,
            SculptBrush::VmErase => &mut self.vm_erase,
            SculptBrush::VmSetValue => &mut self.vm_set_value,
        }
    }
    /// Settings read from prefs: anything a stroke can't use goes back to
    /// that brush's defaults.
    pub fn sanitized(mut self) -> Self {
        for brush in SculptBrush::every() {
            if self.brush(brush).validate().is_err() {
                *self.brush_mut(brush) = brush.defaults();
            }
        }
        if !matches!(
            self.pinch_face,
            SculptBrush::TangentPinch | SculptBrush::CenterPinch
        ) {
            self.pinch_face = SculptBrush::TangentPinch;
        }
        self
    }
}

// ---- spatial lookup ----

type Cell = (i32, i32, i32);

/// A uniform hash grid of indices (points or polygons). Cells are cubes
/// `size` wide in world space; only occupied cells are stored.
#[derive(Clone, Debug)]
struct Grid {
    inv: f64,
    cells: FxHashMap<Cell, Vec<u32>>,
}

impl Grid {
    fn new(size: f64) -> Self {
        Self {
            inv: 1. / size,
            cells: FxHashMap::default(),
        }
    }
    fn cell(&self, p: DVec3) -> Cell {
        // `as` saturates; coordinates are within ±1e9 anyway.
        let c = |x: f64| (x * self.inv).floor() as i32;
        (c(p.x), c(p.y), c(p.z))
    }
    fn insert(&mut self, c: Cell, i: u32) {
        self.cells.entry(c).or_default().push(i);
    }
    fn remove(&mut self, c: Cell, i: u32) {
        if let Some(list) = self.cells.get_mut(&c)
            && let Some(k) = list.iter().position(|&x| x == i)
        {
            list.swap_remove(k);
        }
    }
    /// Every index in the cells overlapping the box `lo..hi`.
    fn query(&self, lo: DVec3, hi: DVec3, out: &mut Vec<u32>) {
        let (a, b) = (self.cell(lo), self.cell(hi));
        let span = |p: i32, q: i32| (q as i64 - p as i64 + 1).max(0);
        let boxed = span(a.0, b.0) * span(a.1, b.1) * span(a.2, b.2);
        if boxed > self.cells.len() as i64 {
            // A brush far bigger than the cells: walk the occupied cells.
            for (c, list) in &self.cells {
                if (a.0..=b.0).contains(&c.0)
                    && (a.1..=b.1).contains(&c.1)
                    && (a.2..=b.2).contains(&c.2)
                {
                    out.extend_from_slice(list);
                }
            }
            return;
        }
        for x in a.0..=b.0 {
            for y in a.1..=b.1 {
                for z in a.2..=b.2 {
                    if let Some(list) = self.cells.get(&(x, y, z)) {
                        out.extend_from_slice(list);
                    }
                }
            }
        }
    }
}

/// Polygons wider than this many cells go in a list tested on every pick.
const OVERSIZE_CELLS: i64 = 4096;

/// The polygons of one target in a grid, for ray picking during a stroke
/// (the viewport's picking surface would rebuild every frame).
#[derive(Clone, Debug)]
struct FaceGrid {
    grid: Grid,
    /// Each polygon's cell range; `None` when not in the grid (polylines,
    /// hidden polygons, oversize ones).
    ranges: Vec<Option<(Cell, Cell)>>,
    oversize: Vec<u32>,
    /// Box of everything in the grid, for clipping rays.
    lo: DVec3,
    hi: DVec3,
    /// Ray stamp per polygon so each is tested once per ray.
    stamp: Vec<u32>,
    ray: u32,
}

impl FaceGrid {
    fn new(size: f64, faces: usize) -> Self {
        Self {
            grid: Grid::new(size),
            ranges: vec![None; faces],
            oversize: Vec::new(),
            lo: DVec3::splat(f64::INFINITY),
            hi: DVec3::splat(f64::NEG_INFINITY),
            stamp: vec![0; faces],
            ray: 0,
        }
    }
    fn place(&mut self, f: u32, lo: DVec3, hi: DVec3) {
        self.lo = self.lo.min(lo);
        self.hi = self.hi.max(hi);
        let range = (self.grid.cell(lo), self.grid.cell(hi));
        if self.ranges[f as usize] == Some(range) {
            return;
        }
        self.unplace(f);
        let (a, b) = range;
        let cells = (b.0 as i64 - a.0 as i64 + 1)
            * (b.1 as i64 - a.1 as i64 + 1)
            * (b.2 as i64 - a.2 as i64 + 1);
        if cells > OVERSIZE_CELLS {
            if !self.oversize.contains(&f) {
                self.oversize.push(f);
            }
            return;
        }
        self.oversize.retain(|&g| g != f);
        for x in a.0..=b.0 {
            for y in a.1..=b.1 {
                for z in a.2..=b.2 {
                    self.grid.insert((x, y, z), f);
                }
            }
        }
        self.ranges[f as usize] = Some(range);
    }
    fn unplace(&mut self, f: u32) {
        if let Some((a, b)) = self.ranges[f as usize].take() {
            for x in a.0..=b.0 {
                for y in a.1..=b.1 {
                    for z in a.2..=b.2 {
                        self.grid.remove((x, y, z), f);
                    }
                }
            }
        }
    }
}

/// Where a ray meets a triangle (two-sided), as the distance along it.
fn ray_triangle(o: DVec3, d: DVec3, a: DVec3, b: DVec3, c: DVec3) -> Option<f64> {
    let (e1, e2) = (b - a, c - a);
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-18 {
        return None;
    }
    let inv = 1. / det;
    let s = o - a;
    let u = s.dot(p) * inv;
    if !(0. ..=1.).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = d.dot(q) * inv;
    if v < 0. || u + v > 1. {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 0.).then_some(t)
}

// ---- stroke ----

/// The surface under the pointer (world space).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceHit {
    pub item: u64,
    pub face: u32,
    pub point: DVec3,
    /// The polygon's outward normal.
    pub normal: DVec3,
    /// Distance along the ray.
    pub depth: f64,
}

/// One stamp of the brush (SC-40), in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dab {
    pub center: DVec3,
    /// The brush radius in metres at `center` (pixels × metres per pixel).
    pub radius: f64,
    /// Unit vector from the surface toward the viewer.
    pub view: DVec3,
    /// The surface normal under the brush (Brush Center).
    pub normal: DVec3,
    /// Tablet pressure 0..1 (SC-45); 1 for a mouse.
    pub pressure: f64,
}

/// Everything fixed at the press (modifiers are read there, SC section 7).
#[derive(Clone, Debug)]
pub struct StrokeSetup {
    /// The active brush tool.
    pub brush: SculptBrush,
    pub settings: BrushSettings,
    /// Ctrl held at the press (SC-60, SC-63).
    pub ctrl: bool,
    /// Shift held at the press (SC-61).
    pub shift: bool,
    /// The shared Symmetry (SC-70), in each mesh's own coordinates.
    pub symmetry: Symmetry,
    /// The active falloff (with anything added on top and the selected
    /// weight map) multiplies the brush weight (SC-33, VMF-13).
    pub falloff: Option<crate::ActiveFalloff>,
    pub lock_borders: bool,
    pub lock_corners: bool,
    /// The brush radius in metres at the press, to size the lookup cells.
    pub radius: f64,
    /// The selected weight map (VM-02): what the Vertex Map Tools paint.
    pub weight_map: Option<String>,
}

/// One mesh a stroke edits.
#[derive(Clone)]
struct Target {
    item: u64,
    to_local: DMat4,
    /// World matrix's linear part, for mirroring directions.
    linear: DMat3,
    linear_inv: DMat3,
    /// −1 when the placement mirrors (world windings flip).
    orient: f64,
    /// The mesh at the press (topology and base positions).
    base: crate::Shared<Mesh>,
    point_faces: std::rc::Rc<crate::topology_cache::PointFaces>,
    /// Current world positions.
    world: Vec<DVec3>,
    /// World positions at the press.
    world_base: Vec<DVec3>,
    points: Grid,
    point_cell: Vec<Cell>,
    faces: FaceGrid,
    /// Points no brush moves: locked or hidden geometry (SC-03), and with
    /// Lock Borders / Lock Corners their points (SC-72, SC-73).
    frozen: Vec<bool>,
    /// Each point's falloff weight at the press (SC-33), clamped to 0…1;
    /// `None` with no falloff.
    falloff_w: Option<Vec<f64>>,
    /// Vertex Map Tools: the weight maps this stroke paints.
    weights: Option<weights::WeightLayer>,
    /// Points on an open border (Smooth keeps them along it).
    border: Vec<bool>,
    edge_counts: std::rc::Rc<crate::topology_cache::EdgeCounts>,
    /// SC-44: how far each point has moved in this stroke, and the cap.
    moved: Vec<f64>,
    /// Points changed since the last flush, and every point this stroke
    /// changed.
    dirty: Vec<u32>,
    is_dirty: Vec<bool>,
    touched: Vec<bool>,
    /// Move's capture: (point, main weight, mirror weight).
    grab: Vec<(u32, f64, f64)>,
    /// Point normals, reused by the dabs of one frame (`normal_frame` is
    /// the frame each was computed in; `frame` counts flushes).
    normals: Vec<DVec3>,
    normal_frame: Vec<u32>,
    frame: u32,
    /// Symmetry on: each point's mirror partner (itself on the plane,
    /// `NO_PARTNER` when it has none), found the first time a dab reaches
    /// the point (`UNKNOWN_PARTNER` until then). Empty with Symmetry off.
    partner: Vec<u32>,
    symmetry: Symmetry,
    /// World matrix (local → world).
    world_m: DMat4,
    /// The matching tolerance in world metres.
    tolerance: f64,
    /// Points moved since the last `keep_symmetric`.
    recent: Vec<u32>,
    in_recent: Vec<bool>,
    partner_scratch: Vec<u32>,
}

/// `Target::partner` values that aren't point indices.
const UNKNOWN_PARTNER: u32 = u32::MAX;
const NO_PARTNER: u32 = u32::MAX - 1;

/// A stroke in progress.
pub struct Stroke {
    setup: StrokeSetup,
    targets: Vec<Target>,
    /// The brush that acts: Smooth when Shift was held.
    acting: SculptBrush,
    /// SC-50 Mouse Down: the first dab's normal.
    mouse_down: Option<DVec3>,
    /// Move: the pointer's world offset so far.
    grab_delta: DVec3,
    grab_started: bool,
    scratch: Vec<u32>,
    /// The last dab's centre: each dab's motion (Smudge, Scar, Tangent
    /// Pinch) is the step from it.
    last_center: Option<DVec3>,
    /// Flatten with Mouse Down: the plane taken at the first dab
    /// (point, normal) (SCT-29).
    press_plane: Option<(DVec3, DVec3)>,
    /// Painting builds: tips, inks and the mask on sculpt brushes
    /// (paint_sculpt.rs, PNT-243..260), and the dabs laid so far.
    #[cfg(feature = "painting")]
    paint: Option<std::sync::Arc<crate::paint_sculpt::SculptPaint>>,
    #[cfg(feature = "painting")]
    dabs: u32,
}

/// One candidate point of a dab.
struct Touch {
    v: u32,
    weight: f64,
    normal: DVec3,
    /// Distance from the centre across the view, 0..1 of the radius.
    t: f64,
}

/// The points of one mesh a finished stroke moved, in the mesh's own
/// coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct SculptItemEdit {
    pub item: u64,
    pub points: Vec<(u32, DVec3)>,
}

/// A finished stroke: `Command::SculptStroke`'s payload (one undo step).
#[derive(Clone, Debug, PartialEq)]
pub struct SculptEdit {
    /// The undo label, e.g. "Push Stroke" (SC-92).
    pub label: String,
    pub items: Vec<SculptItemEdit>,
}

impl SculptEdit {
    pub fn is_empty(&self) -> bool {
        self.items.iter().all(|i| i.points.is_empty())
    }
}

/// Face normal (×2 area, Newell) from world positions.
fn newell(world: &[DVec3], vertices: &[u32]) -> DVec3 {
    let mut n = DVec3::ZERO;
    let k = vertices.len();
    for i in 0..k {
        let a = world[vertices[i] as usize];
        let b = world[vertices[(i + 1) % k] as usize];
        n += DVec3::new(
            (a.y - b.y) * (a.z + b.z),
            (a.z - b.z) * (a.x + b.x),
            (a.x - b.x) * (a.y + b.y),
        );
    }
    n
}

impl Target {
    /// `allowed`: with a component selection in force (SCS-01), the points
    /// a brush may move; every other point is frozen.
    fn new(
        scene: &Scene,
        id: u64,
        setup: &StrokeSetup,
        allowed: Option<&[bool]>,
    ) -> Result<Option<Self>> {
        let Some(item) = scene.items.iter().find(|i| i.id == id && i.is_mesh()) else {
            return Ok(None);
        };
        let mesh = &item.mesh;
        if mesh.vertices.is_empty() {
            return Ok(None);
        }
        let world_m = scene.world_matrix(id)?;
        let to_local = world_m.inverse();
        if !to_local.is_finite() {
            return Ok(None);
        }
        let linear = DMat3::from_mat4(world_m);
        let world: Vec<DVec3> = mesh
            .vertices
            .iter()
            .map(|&p| world_m.transform_point3(p))
            .collect();
        let (mut lo, mut hi) = (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY));
        for &p in &world {
            lo = lo.min(p);
            hi = hi.max(p);
        }
        let diag = (hi - lo).length().max(1e-9);
        // Point cells half the brush radius wide: a dab reads 5 × 5 cells
        // around its circle, about twice the points it touches.
        let size = (setup.radius * 0.5).max(diag * 1e-4).max(1e-9);
        let mut points = Grid::new(size);
        let point_cell: Vec<Cell> = world.iter().map(|&p| points.cell(p)).collect();
        for (v, &c) in point_cell.iter().enumerate() {
            points.insert(c, v as u32);
        }
        let protection = crate::Protection::of(mesh, &item.elements);
        let mut faces = FaceGrid::new(size.max(1e-9), mesh.faces.len());
        for f in 0..mesh.faces.len() {
            let hidden = protection
                .as_ref()
                .is_some_and(|p| p.face_hidden.get(f).copied().unwrap_or(false));
            if hidden || mesh.is_polyline(f) || mesh.faces[f].vertices.len() < 3 {
                continue;
            }
            let (flo, fhi) = face_box(&world, &mesh.faces[f].vertices);
            faces.place(f as u32, flo, fhi);
        }
        let edge_counts = mesh.edge_counts();
        let mut border = vec![false; world.len()];
        for (&(a, b), &n) in edge_counts.iter() {
            if n == 1 {
                for v in [a, b] {
                    if let Some(x) = border.get_mut(v as usize) {
                        *x = true;
                    }
                }
            }
        }
        let point_faces = mesh.point_faces();
        let used = mesh.used_points();
        let mut frozen: Vec<bool> = match &protection {
            Some(p) => p.pinned.clone(),
            None => vec![false; world.len()],
        };
        for v in 0..world.len() {
            // Loose points (no polygon) have no surface to sculpt.
            frozen[v] |= !used[v];
            // SCS-01/04/08: unselected points join what is frozen.
            if let Some(a) = allowed {
                frozen[v] |= !a.get(v).copied().unwrap_or(false);
            }
            if border[v] {
                let corner = point_faces.of(v as u32).len() <= 1;
                frozen[v] |= setup.lock_borders || (setup.lock_corners && corner);
            }
        }
        let n = world.len();
        let symmetry = setup.symmetry;
        let partner = match symmetry.axis {
            None => Vec::new(),
            // Topology pairs come from connectivity, so they're found for
            // the whole mesh at the press.
            Some(_) if symmetry.topology => crate::symmetry::topology_counterparts(mesh, symmetry)
                .into_iter()
                .map(|m| m.unwrap_or(NO_PARTNER))
                .collect(),
            Some(_) => vec![UNKNOWN_PARTNER; n],
        };
        let scale = linear
            .x_axis
            .length()
            .max(linear.y_axis.length())
            .max(linear.z_axis.length());
        let tolerance = crate::symmetry::match_tolerance(mesh) * scale;
        Ok(Some(Self {
            item: id,
            partner,
            symmetry,
            world_m,
            tolerance,
            recent: Vec::new(),
            partner_scratch: Vec::new(),
            in_recent: if symmetry.axis.is_some() {
                vec![false; n]
            } else {
                Vec::new()
            },
            to_local,
            linear,
            linear_inv: linear.inverse(),
            orient: if world_m.determinant() < 0. { -1. } else { 1. },
            base: mesh.clone().into(),
            point_faces,
            world_base: world.clone(),
            world,
            points,
            point_cell,
            faces,
            frozen,
            weights: setup
                .brush
                .is_vertex_map()
                .then_some(setup.weight_map.as_deref())
                .flatten()
                .map(|name| {
                    let mut layer = weights::WeightLayer::new(mesh, name);
                    // SK-43: a joint map normalises across its skin only.
                    if let Some(maps) = crate::skin_weights::skin_maps(scene, id)
                        && maps.iter().any(|m| m == name)
                    {
                        layer.others.retain(|(n, _, _)| maps.contains(n));
                    }
                    layer
                }),
            falloff_w: match &setup.falloff {
                Some(f) => Some(
                    f.weights(scene, id)?
                        .into_iter()
                        .map(|w| if w.is_finite() { w.clamp(0., 1.) } else { 0. })
                        .collect(),
                ),
                None => None,
            },
            border,
            edge_counts,
            moved: vec![0.; n],
            dirty: Vec::new(),
            is_dirty: vec![false; n],
            touched: vec![false; n],
            grab: Vec::new(),
            normals: vec![DVec3::ZERO; n],
            normal_frame: vec![0; n],
            frame: 1,
        }))
    }

    /// `vertex_normal`, computed at most once a frame: the dabs of one
    /// frame are a few pixels apart, so the points barely turn between
    /// them, and the next frame reads them fresh (SC-51).
    fn frame_normal(&mut self, v: u32) -> DVec3 {
        let i = v as usize;
        if self.normal_frame[i] != self.frame {
            self.normals[i] = self.vertex_normal(v);
            self.normal_frame[i] = self.frame;
        }
        self.normals[i]
    }

    /// The point's outward normal from the current positions.
    fn vertex_normal(&self, v: u32) -> DVec3 {
        let mut n = DVec3::ZERO;
        for &f in self.point_faces.of(v) {
            n += newell(&self.world, &self.base.faces[f as usize].vertices);
        }
        (n * self.orient).normalize_or_zero()
    }

    /// The point's outward normal at the press (Emboss, Move Slide, Clay's
    /// plane).
    fn base_normal(&self, v: u32) -> DVec3 {
        let mut n = DVec3::ZERO;
        for &f in self.point_faces.of(v) {
            n += newell(&self.world_base, &self.base.faces[f as usize].vertices);
        }
        (n * self.orient).normalize_or_zero()
    }

    /// A direction (not normalised) mirrored through the symmetry plane.
    fn mirror_vec(&self, s: Symmetry, d: DVec3) -> DVec3 {
        self.linear * s.mirror_vector(self.linear_inv * d)
    }

    /// Mirror of a world point through this mesh's symmetry plane.
    fn mirror_point(&self, s: Symmetry, p: DVec3) -> DVec3 {
        self.world_m
            .transform_point3(s.mirror(self.to_local.transform_point3(p)))
    }
    fn mirror_dir(&self, s: Symmetry, d: DVec3) -> DVec3 {
        (self.linear * s.mirror_vector(self.linear_inv * d)).normalize_or_zero()
    }

    /// Moves point `v` to world `p`.
    fn set(&mut self, v: u32, p: DVec3) {
        if !p.is_finite() {
            return;
        }
        let i = v as usize;
        self.world[i] = p;
        let c = self.points.cell(p);
        if c != self.point_cell[i] {
            self.points.remove(self.point_cell[i], v);
            self.points.insert(c, v);
            self.point_cell[i] = c;
        }
        if !self.is_dirty[i] {
            self.is_dirty[i] = true;
            self.dirty.push(v);
        }
        self.touched[i] = true;
        if let Some(r) = self.in_recent.get_mut(i)
            && !*r
        {
            *r = true;
            self.recent.push(v);
        }
    }

    /// Finds point `v`'s mirror partner the first time a dab reaches it.
    /// A point that hasn't moved yet has a partner that hasn't either (a
    /// point is paired before anything moves it, and `keep_symmetric` only
    /// moves pairs), so the partner sits at the mirror of `v`'s position
    /// at the press (SYM3-04: nearest within the matching tolerance, lowest
    /// index on a tie; a point on the plane is its own).
    fn resolve_partner(&mut self, v: u32, scratch: &mut Vec<u32>) {
        let i = v as usize;
        if self.partner.get(i) != Some(&UNKNOWN_PARTNER) {
            return;
        }
        let local = self.base.vertices[i];
        let local_tolerance = crate::symmetry::match_tolerance_shared(&self.base);
        if self.symmetry.side(local, local_tolerance) == Some(crate::symmetry::Side::Plane) {
            self.partner[i] = v;
            return;
        }
        let target = self.world_m.transform_point3(self.symmetry.mirror(local));
        let reach = DVec3::splat(self.tolerance);
        scratch.clear();
        self.points.query(target - reach, target + reach, scratch);
        let mut best: Option<(f64, u32)> = None;
        for &j in scratch.iter() {
            let ju = j as usize;
            if j == v
                || self.partner[ju] != UNKNOWN_PARTNER
                || self.world[ju] != self.world_base[ju]
            {
                continue;
            }
            let d = self.world_base[ju].distance(target);
            if d < self.tolerance && best.is_none_or(|(bd, bj)| d < bd || d == bd && j < bj) {
                best = Some((d, j));
            }
        }
        match best {
            Some((_, j)) => {
                self.partner[i] = j;
                self.partner[j as usize] = v;
            }
            None => self.partner[i] = NO_PARTNER,
        }
    }

    /// SC-70: after a dab and its mirror, makes every pair they moved an
    /// exact mirror image again. The two dabs run one after the other, and
    /// where they overlap (any stroke near the centre line) the second one
    /// reads what the first did, so the sides drift apart a little with
    /// each dab. Each pair takes the average of its two moves (one of them
    /// mirrored), and points on the plane stay on it. A pair with a locked
    /// or hidden point keeps the brush's result.
    fn keep_symmetric(&mut self) {
        if self.partner.is_empty() {
            return;
        }
        let recent = std::mem::take(&mut self.recent);
        for &a in &recent {
            let ai = a as usize;
            let b = self.partner[ai];
            if b == a {
                let p = self.world[ai];
                self.set(a, (p + self.mirror_point(self.symmetry, p)) * 0.5);
                continue;
            }
            // A pair both dabs moved is done once, from its lower index.
            if b >= NO_PARTNER || (b < a && self.in_recent[b as usize]) {
                continue;
            }
            let bi = b as usize;
            if self.frozen[ai] || self.frozen[bi] {
                continue;
            }
            let da = self.world[ai] - self.world_base[ai];
            let db = self.world[bi] - self.world_base[bi];
            let avg = (da + self.mirror_vec(self.symmetry, db)) * 0.5;
            let mirrored = self.mirror_vec(self.symmetry, avg);
            self.set(a, self.world_base[ai] + avg);
            self.set(b, self.world_base[bi] + mirrored);
            let m = self.moved[ai].max(self.moved[bi]);
            self.moved[ai] = m;
            self.moved[bi] = m;
        }
        // The moves above are already symmetric.
        for &a in recent.iter().chain(&self.recent) {
            self.in_recent[a as usize] = false;
        }
        self.recent.clear();
    }

    /// The points a dab reaches (SC-30, SC-32): within the world radius of
    /// the centre, facing the viewer, not frozen; weighted by their
    /// distance from the centre across the view (screen space at the
    /// centre's depth) and the falloff.
    fn touches(&mut self, dab: &Dab, tip: &Tip, scratch: &mut Vec<u32>) -> Vec<Touch> {
        scratch.clear();
        let r = dab.radius;
        self.points.query(
            dab.center - DVec3::splat(r),
            dab.center + DVec3::splat(r),
            scratch,
        );
        let mut out = Vec::with_capacity(scratch.len());
        for &v in scratch.iter() {
            let i = v as usize;
            if self.frozen[i] {
                continue;
            }
            let d = self.world[i] - dab.center;
            if d.length_squared() > r * r {
                continue;
            }
            let across = d - dab.view * d.dot(dab.view);
            let t = across.length() / r;
            let mut weight = tip.weight(t);
            if weight <= 0. {
                continue;
            }
            if let Some(f) = &self.falloff_w {
                weight *= f[i];
                if weight <= 0. {
                    continue;
                }
            }
            if !self.partner.is_empty() {
                let mut found = std::mem::take(&mut self.partner_scratch);
                self.resolve_partner(v, &mut found);
                self.partner_scratch = found;
            }
            let normal = self.frame_normal(v);
            if normal.dot(dab.view) <= 0. {
                continue;
            }
            out.push(Touch {
                v,
                weight: weight * dab.pressure.clamp(0., 1.),
                normal,
                t,
            });
        }
        out
    }

    /// The point's neighbours along the polygons' sides; a border point's
    /// neighbours along the border only, so Smooth slides it along the edge
    /// instead of pulling it in.
    fn neighbour_average(&self, v: u32) -> Option<DVec3> {
        let mut sum = DVec3::ZERO;
        let mut n = 0usize;
        let on_border = self.border[v as usize];
        for &f in self.point_faces.of(v) {
            let face = &self.base.faces[f as usize].vertices;
            let k = face.len();
            for (i, &u) in face.iter().enumerate() {
                if u != v {
                    continue;
                }
                for w in [face[(i + k - 1) % k], face[(i + 1) % k]] {
                    if on_border {
                        let key = (v.min(w), v.max(w));
                        if self.edge_counts.get(&key).copied() != Some(1) {
                            continue;
                        }
                    }
                    sum += self.world[w as usize];
                    n += 1;
                }
            }
        }
        (n > 0).then(|| sum / n as f64)
    }

    /// One Jacobi smoothing pass over `touches` with factor `lambda` per
    /// unit weight (negative inflates back, Taubin).
    fn smooth_pass(&mut self, touches: &[Touch], lambda: f64) {
        let moves: Vec<(u32, DVec3)> = touches
            .iter()
            .filter_map(|t| {
                let avg = self.neighbour_average(t.v)?;
                let p = self.world[t.v as usize];
                let k = (lambda * t.weight).clamp(-1., 1.);
                Some((t.v, p + brushes::smooth(p, avg, k)))
            })
            .collect();
        for (v, p) in moves {
            self.set(v, p);
        }
    }

    /// Writes the changed points into the scene's mesh; false when the
    /// item is gone.
    fn flush(&mut self, scene: &mut Scene) -> bool {
        // A new frame: point normals are read fresh.
        self.frame = self.frame.wrapping_add(1).max(1);
        let weights_dirty = self.weights.as_ref().is_some_and(|w| !w.dirty.is_empty());
        if self.dirty.is_empty() && !weights_dirty {
            return true;
        }
        let Some(item) = scene.item_mut(self.item) else {
            return false;
        };
        if item.mesh.vertices.len() != self.world.len() {
            return false;
        }
        let mesh = &mut *item.mesh;
        if weights_dirty {
            self.flush_weights(mesh);
        }
        let mut faces: Vec<u32> = Vec::new();
        for &v in &self.dirty {
            mesh.vertices[v as usize] = self.to_local.transform_point3(self.world[v as usize]);
            self.is_dirty[v as usize] = false;
            faces.extend_from_slice(self.point_faces.of(v));
        }
        self.dirty.clear();
        faces.sort_unstable();
        faces.dedup();
        for f in faces {
            if self.faces.ranges[f as usize].is_some() || self.faces.oversize.contains(&f) {
                let (lo, hi) = face_box(&self.world, &self.base.faces[f as usize].vertices);
                self.faces.place(f, lo, hi);
            }
        }
        true
    }

    /// The nearest polygon hit along a world ray.
    fn pick(&mut self, origin: DVec3, dir: DVec3) -> Option<(u32, f64)> {
        let dir = dir.normalize_or_zero();
        if dir == DVec3::ZERO || !self.faces.lo.is_finite() {
            return None;
        }
        self.faces.ray = self.faces.ray.wrapping_add(1);
        if self.faces.ray == 0 {
            self.faces.stamp.fill(0);
            self.faces.ray = 1;
        }
        let mut best: Option<(u32, f64)> = None;
        let world = &self.world;
        let faces = &self.base.faces;
        let test = |f: u32, stamp: &mut Vec<u32>, ray: u32, best: &mut Option<(u32, f64)>| {
            if stamp[f as usize] == ray {
                return;
            }
            stamp[f as usize] = ray;
            let vs = &faces[f as usize].vertices;
            let a = world[vs[0] as usize];
            for i in 1..vs.len() - 1 {
                let (b, c) = (world[vs[i] as usize], world[vs[i + 1] as usize]);
                if let Some(t) = ray_triangle(origin, dir, a, b, c)
                    && best.is_none_or(|(_, bt)| t < bt)
                {
                    *best = Some((f, t));
                }
            }
        };
        for &f in &self.faces.oversize.clone() {
            test(f, &mut self.faces.stamp, self.faces.ray, &mut best);
        }
        // Clip the ray to the grid's box, then walk its cells in order.
        let pad = 1. / self.faces.grid.inv;
        let (lo, hi) = (self.faces.lo - pad, self.faces.hi + pad);
        let (mut t0, mut t1) = (0_f64, f64::INFINITY);
        for a in 0..3 {
            if dir[a].abs() < 1e-300 {
                if origin[a] < lo[a] || origin[a] > hi[a] {
                    return best;
                }
                continue;
            }
            let (mut ta, mut tb) = ((lo[a] - origin[a]) / dir[a], (hi[a] - origin[a]) / dir[a]);
            if ta > tb {
                std::mem::swap(&mut ta, &mut tb);
            }
            t0 = t0.max(ta);
            t1 = t1.min(tb);
        }
        if t0 > t1 {
            return best;
        }
        let size = pad;
        let start = origin + dir * t0;
        let mut cell = self.faces.grid.cell(start);
        let step = [dir.x, dir.y, dir.z].map(|d| if d > 0. { 1 } else { -1 });
        let next_boundary = |c: i32, a: usize| -> f64 {
            let edge = if step[a] > 0 { c + 1 } else { c } as f64 * size;
            if dir[a].abs() < 1e-300 {
                f64::INFINITY
            } else {
                (edge - origin[a]) / dir[a]
            }
        };
        let mut t_max = [
            next_boundary(cell.0, 0),
            next_boundary(cell.1, 1),
            next_boundary(cell.2, 2),
        ];
        let t_delta = [0, 1, 2].map(|a| {
            if dir[a].abs() < 1e-300 {
                f64::INFINITY
            } else {
                size / dir[a].abs()
            }
        });
        // Enough steps to cross the box corner to corner.
        let limit = (((hi - lo) / size).abs().element_sum() as i64 + 8).min(1 << 20);
        for _ in 0..limit {
            if let Some(list) = self.faces.grid.cells.get(&cell) {
                for &f in list {
                    test(f, &mut self.faces.stamp, self.faces.ray, &mut best);
                }
            }
            let exit = t_max[0].min(t_max[1]).min(t_max[2]);
            if best.is_some_and(|(_, t)| t <= exit) || exit > t1 {
                break;
            }
            let a = if t_max[0] <= t_max[1] && t_max[0] <= t_max[2] {
                0
            } else if t_max[1] <= t_max[2] {
                1
            } else {
                2
            };
            match a {
                0 => cell.0 += step[0],
                1 => cell.1 += step[1],
                _ => cell.2 += step[2],
            }
            t_max[a] += t_delta[a];
        }
        best
    }

    fn face_normal(&self, f: u32) -> DVec3 {
        (newell(&self.world, &self.base.faces[f as usize].vertices) * self.orient)
            .normalize_or_zero()
    }
}

fn face_box(world: &[DVec3], vertices: &[u32]) -> (DVec3, DVec3) {
    let (mut lo, mut hi) = (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY));
    for &v in vertices {
        let p = world[v as usize];
        lo = lo.min(p);
        hi = hi.max(p);
    }
    (lo, hi)
}

/// SCT-13 Fold's signed profile at `t` (0 centre, 1 rim), multiplied into
/// the tip weight: −1 (in) at the centre rising through 0 at
/// `FOLD_CROSSOVER` to +1 (out) at the rim, where the tip fades it out.
fn fold_profile(t: f64) -> f64 {
    let c = FOLD_CROSSOVER;
    if t < c {
        -(1. - t / c)
    } else {
        ((t - c) / (1. - c)).min(1.)
    }
}

/// How much of Smooth Amount a Uniform smooth applies after each dab
/// (SC-62 "always smooth a bit"; provisional factor).
const UNIFORM_SMOOTH: f64 = 0.2;
/// Taubin's inflate-back factor for volume-keeping smoothing (SC-63).
const TAUBIN_MU: f64 = 1.04;
/// SC-62 Low Pressure: a dab lighter than this smooths (at Smooth Amount,
/// its pressure stretched over 0..1 below the line) instead of sculpting
/// (provisional, PTQ-03).
pub const LOW_PRESSURE: f64 = 0.35;

/// SCS-01..03: in Vertex, Edge or Polygon mode, when any of the meshes
/// `items` has components of that mode selected, each mesh's points the
/// brush may move (one flag per point, by `items` index): the selected
/// vertices, both ends of each selected edge, every corner of each
/// selected polygon. A mesh with nothing selected gets none. `None` (the
/// whole mesh is sculpted) in Item mode or with nothing selected.
fn selection_masks(scene: &Scene, items: &[u64]) -> Option<Vec<Vec<bool>>> {
    use crate::Mode;
    let sel = &scene.selection;
    let mode = sel.mode;
    let selected = |id: u64| crate::ComponentSelection {
        vertices: match mode {
            Mode::Vertex => sel.vertices_of(id).to_vec(),
            _ => Vec::new(),
        },
        edges: match mode {
            Mode::Edge => sel.edges_of(id).to_vec(),
            _ => Vec::new(),
        },
        faces: match mode {
            Mode::Polygon => sel.faces_of(id).to_vec(),
            _ => Vec::new(),
        },
    };
    if mode == Mode::Item || items.iter().all(|&id| selected(id).count(mode) == 0) {
        return None;
    }
    Some(
        items
            .iter()
            .map(|&id| {
                let Some(mesh) = scene.item(id).map(|i| &i.mesh) else {
                    return Vec::new();
                };
                let mut mask = vec![false; mesh.vertices.len()];
                let points =
                    crate::selection_inference::points_from(mesh, mode, &selected(id), |_| true);
                for v in points {
                    if let Some(m) = mask.get_mut(v as usize) {
                        *m = true;
                    }
                }
                mask
            })
            .collect(),
    )
}

impl Stroke {
    /// Starts a stroke on the active meshes `items` (SC-03). `None` when
    /// none of them has anything to sculpt.
    pub fn begin(scene: &Scene, items: &[u64], setup: StrokeSetup) -> Result<Option<Self>> {
        setup.settings.validate()?;
        if !setup.shift
            && let Some(why) = setup.brush.unavailable()
        {
            return Err(why.into());
        }
        if !(setup.radius.is_finite() && setup.radius > 0.) {
            return Err("The brush has no size here".into());
        }
        let allowed = selection_masks(scene, items);
        let mut targets = Vec::new();
        for (k, &id) in items.iter().enumerate() {
            let mask = allowed.as_ref().map(|m| m[k].as_slice());
            if let Some(t) = Target::new(scene, id, &setup, mask)? {
                targets.push(t);
            }
        }
        if targets.is_empty() {
            return Ok(None);
        }
        // A selection whose points are all locked, hidden or loose leaves
        // nothing to sculpt.
        if allowed.is_some() && targets.iter().all(|t| t.frozen.iter().all(|&f| f)) {
            return Ok(None);
        }
        if setup.brush.is_vertex_map() && setup.weight_map.is_none() {
            return Err("Select or create a weight map first".into());
        }
        // Shift smooths: the surface, or with a Vertex Map Tool the map.
        let acting = match (setup.shift, setup.brush.is_vertex_map()) {
            (true, true) => SculptBrush::VmSmooth,
            (true, false) => SculptBrush::Smooth,
            (false, _) => setup.brush,
        };
        Ok(Some(Self {
            setup,
            targets,
            acting,
            mouse_down: None,
            grab_delta: DVec3::ZERO,
            grab_started: false,
            scratch: Vec::new(),
            last_center: None,
            press_plane: None,
            #[cfg(feature = "painting")]
            paint: None,
            #[cfg(feature = "painting")]
            dabs: 0,
        }))
    }

    /// The brush that acts (Smooth while Shift-smoothing).
    /// Painting builds: the stroke's tips, inks and mask (PNT-243..260).
    #[cfg(feature = "painting")]
    pub fn set_paint(&mut self, paint: std::sync::Arc<crate::paint_sculpt::SculptPaint>) {
        self.paint = Some(paint);
    }

    /// The tip a dab measures with: a painting tip replaces the profile.
    fn dab_tip(&self, tip: Tip) -> Tip {
        #[cfg(feature = "painting")]
        if let Some(p) = &self.paint
            && p.replaces_tip()
        {
            return p.flat_tip(&tip);
        }
        tip
    }

    /// Painting builds: each touched point's weight scaled by the tips,
    /// inks and mask; the mask leaves mask strokes alone (`vertex_map`).
    #[cfg_attr(not(feature = "painting"), allow(unused_variables))]
    fn painted(&self, i: usize, dab: &Dab, touches: Vec<Touch>, vertex_map: bool) -> Vec<Touch> {
        #[cfg(feature = "painting")]
        if let Some(p) = &self.paint {
            let t = &self.targets[i];
            let index = self.dabs;
            return touches
                .into_iter()
                .filter_map(|mut touch| {
                    let mut g = p.gain(t.world[touch.v as usize], touch.normal, dab, index);
                    if vertex_map {
                        g = g.max(0.);
                    } else {
                        g *= p.mask_factor(t.item, touch.v);
                    }
                    touch.weight *= g;
                    (touch.weight != 0.).then_some(touch)
                })
                .collect();
        }
        touches
    }

    pub fn acting(&self) -> SculptBrush {
        self.acting
    }
    pub fn setup(&self) -> &StrokeSetup {
        &self.setup
    }
    /// Whether the stroke is a grab (Move without Shift).
    pub fn is_grab(&self) -> bool {
        self.acting.kind() == BrushKind::Grab
    }
    /// The undo label (SC-92).
    pub fn label(&self) -> String {
        if self.acting.is_vertex_map() {
            format!("Vertex Map {} Stroke", self.acting.label())
        } else {
            format!("{} Stroke", self.acting.label())
        }
    }

    /// The nearest surface of the stroke's meshes along a world ray, from
    /// the stroke's own lookup (the meshes change every frame).
    pub fn pick(&mut self, origin: DVec3, dir: DVec3) -> Option<SurfaceHit> {
        let mut best: Option<SurfaceHit> = None;
        for t in &mut self.targets {
            if let Some((f, depth)) = t.pick(origin, dir)
                && best.is_none_or(|b| depth < b.depth)
            {
                best = Some(SurfaceHit {
                    item: t.item,
                    face: f,
                    point: origin + dir.normalize_or_zero() * depth,
                    normal: t.face_normal(f),
                    depth,
                });
            }
        }
        best
    }

    /// The full move of an offset dab at weight 1, in metres (SC-24).
    fn full_offset(&self, dab: &Dab) -> f64 {
        match self.setup.settings.offset_mode {
            OffsetMode::Adaptive => dab.radius * self.setup.settings.offset_amount,
            OffsetMode::Absolute => self.setup.settings.offset_distance,
        }
    }

    /// Applies one dab, and its mirror with Symmetry on (SC-70). Grab
    /// strokes capture at their first dab and ignore the rest.
    pub fn dab(&mut self, dab: Dab) {
        if !(dab.center.is_finite() && dab.radius.is_finite() && dab.radius > 0.) {
            return;
        }
        let dab = Dab {
            view: dab.view.normalize_or_zero(),
            normal: dab.normal.normalize_or_zero(),
            ..dab
        };
        if self.mouse_down.is_none() {
            self.mouse_down = Some(dab.normal);
        }
        if self.is_grab() {
            if !self.grab_started {
                self.capture(&dab);
            }
            return;
        }
        #[cfg(feature = "painting")]
        {
            self.dabs = self.dabs.wrapping_add(1);
        }
        // The step since the last dab: the stroke's direction.
        let motion = self.last_center.map_or(DVec3::ZERO, |c| dab.center - c);
        self.last_center = Some(dab.center);
        // SC-62 Low Pressure: light dabs smooth, hard ones sculpt.
        let low = self.setup.settings.smooth_mode == SmoothMode::LowPressure
            && self.acting != SculptBrush::Smooth
            && !self.acting.is_vertex_map()
            && dab.pressure < LOW_PRESSURE;
        let (acting, dab) = if low {
            let pressure = (dab.pressure / LOW_PRESSURE).clamp(0., 1.);
            (SculptBrush::Smooth, Dab { pressure, ..dab })
        } else {
            (self.acting, dab)
        };
        let symmetry = self.setup.symmetry;
        for i in 0..self.targets.len() {
            self.offset_dab(i, &dab, motion, false, acting);
            if symmetry.axis.is_some() {
                let t = &self.targets[i];
                let mirrored = Dab {
                    center: t.mirror_point(symmetry, dab.center),
                    view: t.mirror_dir(symmetry, dab.view),
                    normal: t.mirror_dir(symmetry, dab.normal),
                    ..dab
                };
                let motion = t.mirror_vec(symmetry, motion);
                self.offset_dab(i, &mirrored, motion, true, acting);
                self.targets[i].keep_symmetric();
            }
        }
    }

    /// The direction a point moves for the normal modes (SC-50).
    fn direction(
        &self,
        t: &Target,
        mode: NormalMode,
        dab: &Dab,
        touch: &Touch,
        average: DVec3,
        mirrored: bool,
    ) -> DVec3 {
        match mode {
            NormalMode::Vertex => touch.normal,
            _ => self.brush_normal(t, mode, dab, average, mirrored),
        }
    }

    /// One direction for the whole dab (a plane's normal, a spin axis):
    /// the normal modes with Vertex read as Average.
    fn brush_normal(
        &self,
        t: &Target,
        mode: NormalMode,
        dab: &Dab,
        average: DVec3,
        mirrored: bool,
    ) -> DVec3 {
        let axis = |a: DVec3| {
            if mirrored {
                t.mirror_dir(self.setup.symmetry, a)
            } else {
                a
            }
        };
        match mode {
            NormalMode::BrushCenter => dab.normal,
            NormalMode::Vertex | NormalMode::Average => average,
            NormalMode::Screen => dab.view,
            NormalMode::MouseDown => axis(self.mouse_down.unwrap_or(dab.normal)),
            NormalMode::X => axis(DVec3::X),
            NormalMode::Y => axis(DVec3::Y),
            NormalMode::Z => axis(DVec3::Z),
        }
    }

    /// One offset dab on target `i` (SC-80): `acting`'s vertex rule (the
    /// stroke's acting brush, or Smooth for a Low Pressure dab). `motion`
    /// is the step since the last dab (zero for the first).
    fn offset_dab(
        &mut self,
        i: usize,
        dab: &Dab,
        motion: DVec3,
        mirrored: bool,
        acting: SculptBrush,
    ) {
        let settings = self.setup.settings;
        // VMT-15: Set Value has no tip; every point under it gets Value.
        let tip = if acting == SculptBrush::VmSetValue {
            Tip {
                kind: TipKind::Hard,
                soft_border: 0.,
                bias: 0.5,
                ..settings.tip
            }
        } else {
            self.dab_tip(settings.tip)
        };
        let touches = self.targets[i].touches(dab, &tip, &mut self.scratch);
        let touches = self.painted(i, dab, touches, acting.is_vertex_map());
        if touches.is_empty() {
            return;
        }
        if acting.is_vertex_map() {
            self.weight_dab(i, &touches, acting);
            return;
        }
        if acting == SculptBrush::Smooth {
            // Shift- and Low Pressure smoothing use the brush's Smooth
            // Amount; the Smooth brush its own Offset Amount (SC-61,
            // SC-62, SC-120).
            let strength = if self.setup.shift || acting != self.acting {
                settings.smooth_amount
            } else {
                settings.offset_amount
            }
            .clamp(0., 1.);
            let t = &mut self.targets[i];
            t.smooth_pass(&touches, strength);
            if self.setup.ctrl {
                // SC-63: inflate back so the shape keeps its volume.
                t.smooth_pass(&touches, -strength * TAUBIN_MU);
            }
            return;
        }
        let r = dab.radius;
        let ctrl = if self.setup.ctrl { -1. } else { 1. };
        // Offset Amount as a fraction of the radius (or Offset Distance
        // over the radius), signed by Ctrl.
        let strength = self.full_offset(dab) / r;
        // A dab's share of the full move: one radius of travel lays
        // 1/spacing dabs, so a sweep moves points about the full offset at
        // any Interpolation Step (provisional, SC-41).
        let share = if acting.spaced() {
            settings.spacing().min(1.)
        } else {
            1.
        };
        let average = {
            let s: DVec3 = touches.iter().map(|t| t.normal * t.weight).sum();
            let s = s.normalize_or_zero();
            if s == DVec3::ZERO { dab.normal } else { s }
        };
        let moves: Vec<(u32, DVec3)> = match acting {
            SculptBrush::Push | SculptBrush::Inflate | SculptBrush::Carve | SculptBrush::Fold => {
                self.draw_moves(i, dab, &touches, average, mirrored, acting, strength * r)
            }
            SculptBrush::Clay => {
                self.clay_moves(i, dab, &touches, average, mirrored, strength.abs())
            }
            SculptBrush::Smudge => {
                // SCT-19 (Nudge): carried along the pointer's step, along
                // the surface.
                let t = &self.targets[i];
                let s = strength * ctrl;
                touches
                    .iter()
                    .map(|touch| {
                        let p = t.world[touch.v as usize];
                        let offset = brushes::tangent_offset(touch.normal, motion);
                        (
                            touch.v,
                            p + brushes::grab(offset, (touch.weight * s).clamp(-1., 1.)),
                        )
                    })
                    .collect()
            }
            SculptBrush::Scar => {
                // SCT-25 (Crease): pressed in along the normal, and pinched
                // toward the stroke line in equal shares (SCT-Q09).
                let t = &self.targets[i];
                let across = average.cross(motion).normalize_or_zero();
                let s = strength * share;
                touches
                    .iter()
                    .map(|touch| {
                        let p = t.world[touch.v as usize];
                        let w = touch.weight;
                        let mut m = brushes::draw(-average * ctrl, r, s, w);
                        if across != DVec3::ZERO {
                            m += brushes::pinch(p, dab.center, &[across], (w * s).clamp(0., 1.));
                        }
                        (touch.v, p + m)
                    })
                    .collect()
            }
            SculptBrush::Flatten => {
                self.flatten_moves(i, dab, &touches, average, mirrored, strength, share)
            }
            SculptBrush::TangentPinch | SculptBrush::CenterPinch => {
                // SCT-33, SCT-36 (Pinch): toward the stroke line or the
                // centre, sliding in each point's tangent plane.
                let t = &self.targets[i];
                let across = average.cross(motion).normalize_or_zero();
                if acting == SculptBrush::TangentPinch && across == DVec3::ZERO {
                    return;
                }
                let s = (strength * share * ctrl).clamp(-1., 1.);
                touches
                    .iter()
                    .map(|touch| {
                        let p = t.world[touch.v as usize];
                        let n = touch.normal;
                        let m = if acting == SculptBrush::TangentPinch {
                            brushes::pinch(p, dab.center, &[across], touch.weight * s)
                        } else {
                            let (a, b) = n.any_orthonormal_pair();
                            brushes::pinch(p, dab.center, &[a, b], touch.weight * s)
                        };
                        (touch.v, p + (m - n * m.dot(n)))
                    })
                    .collect()
            }
            SculptBrush::Spin => {
                // SCT-46 (Rotate): about the Normal Mode axis through the
                // centre; clockwise looking down the axis (SCT-Q13).
                let t = &self.targets[i];
                let axis = self.brush_normal(t, settings.normal_mode, dab, average, mirrored);
                let hand = if mirrored { -1. } else { 1. };
                let turn = -strength * share * SPIN_RADIANS_PER_RADIUS * ctrl * hand;
                touches
                    .iter()
                    .map(|touch| {
                        let p = t.world[touch.v as usize];
                        let m = brushes::rotate(p, dab.center, axis, turn * touch.weight);
                        (touch.v, p + m)
                    })
                    .collect()
            }
            SculptBrush::Attenuate => {
                if !ATTENUATE_ON_PLAIN_MESHES {
                    return;
                }
                // SCT-Q12 alternative: fade toward the shape at the press.
                let t = &self.targets[i];
                touches
                    .iter()
                    .map(|touch| {
                        let v = touch.v as usize;
                        let k = (touch.weight * strength.abs()).clamp(0., 1.);
                        (
                            touch.v,
                            t.world[v] + brushes::smooth(t.world[v], t.world_base[v], k),
                        )
                    })
                    .collect()
            }
            // Grab brushes, Smooth and the Vertex Map Tools are handled
            // elsewhere.
            SculptBrush::Smooth
            | SculptBrush::Move
            | SculptBrush::Emboss
            | SculptBrush::VmPaint
            | SculptBrush::VmSmooth
            | SculptBrush::VmScaleDown
            | SculptBrush::VmErase
            | SculptBrush::VmSetValue => return,
        };
        // Painting builds: Constrain to Background (PNT-264).
        #[cfg(feature = "painting")]
        let moves = match &self.paint {
            Some(p) if p.background.is_some() => {
                let t = &self.targets[i];
                moves
                    .into_iter()
                    .map(|(v, q)| (v, p.constrain(t.world_base[v as usize], q)))
                    .collect()
            }
            _ => moves,
        };
        let t = &mut self.targets[i];
        for (v, p) in moves {
            t.set(v, p);
        }
        if settings.smooth_mode == SmoothMode::Uniform {
            t.smooth_pass(&touches, settings.smooth_amount * UNIFORM_SMOOTH);
        }
    }

    /// Push, Inflate, Carve and Fold (Draw / Inflate): along the normal by
    /// the full offset (`full`, metres) × weight × the dab's share, with the
    /// Density Mode cap (SC-44).
    #[allow(clippy::too_many_arguments)]
    fn draw_moves(
        &mut self,
        i: usize,
        dab: &Dab,
        touches: &[Touch],
        average: DVec3,
        mirrored: bool,
        acting: SculptBrush,
        full: f64,
    ) -> Vec<(u32, DVec3)> {
        let settings = self.setup.settings;
        let mode = match acting {
            SculptBrush::Inflate => NormalMode::Vertex,
            SculptBrush::Fold => NormalMode::Average,
            _ => settings.normal_mode,
        };
        // Carve digs by default (SCT-22); Ctrl flips every brush.
        let mut sign = if self.setup.ctrl { -1. } else { 1. };
        if acting == SculptBrush::Carve {
            sign = -sign;
        }
        let share = settings.spacing().min(1.);
        let mut out = Vec::with_capacity(touches.len());
        let t = &self.targets[i];
        let mut moved = Vec::with_capacity(touches.len());
        for touch in touches {
            let dir = self.direction(t, mode, dab, touch, average, mirrored);
            if dir == DVec3::ZERO {
                continue;
            }
            // SCT-13 Fold: in at the centre, out toward the rim.
            let profile = if acting == SculptBrush::Fold {
                fold_profile(touch.t)
            } else {
                1.
            };
            let cap = full.abs() * touch.weight * profile.abs();
            let done = t.moved[touch.v as usize];
            let amount = match settings.density_mode {
                DensityMode::Additive => cap * share,
                DensityMode::Attenuate => (cap * share).min(cap - done),
                DensityMode::MaxStroke => cap - done,
            };
            if amount > 0. {
                let s = sign * full.signum() * profile.signum();
                let p = t.world[touch.v as usize];
                out.push((touch.v, p + brushes::draw(dir, amount, s, 1.)));
                moved.push((touch.v, amount));
            }
        }
        let t = &mut self.targets[i];
        for (v, amount) in moved {
            t.moved[v as usize] += amount;
        }
        out
    }

    /// SCT-14 Clay: points rise toward a plane Level × radius above the
    /// brush plane (the surface at the press, so brushing longer doesn't
    /// build higher), never past it; Ctrl cuts down to a plane Level below
    /// (SCT-Q06).
    fn clay_moves(
        &self,
        i: usize,
        dab: &Dab,
        touches: &[Touch],
        average: DVec3,
        mirrored: bool,
        strength: f64,
    ) -> Vec<(u32, DVec3)> {
        let settings = self.setup.settings;
        let t = &self.targets[i];
        let n = self.brush_normal(t, settings.normal_mode, dab, average, mirrored);
        if n == DVec3::ZERO {
            return Vec::new();
        }
        let r = dab.radius;
        let mut sampler = plane::PlaneSampler::new(dab.center, dab.view, 0., r);
        for touch in touches {
            let base = t.world_base[touch.v as usize];
            sampler.add(base, touch.normal, base.distance(dab.center));
        }
        let center = sampler.finish().center;
        // The brush plane through the press surface, then Level along it.
        let down = self.setup.ctrl;
        let level = settings.level * r * if down { -1. } else { 1. };
        let plane_point = center + n * level;
        touches
            .iter()
            .filter_map(|touch| {
                let p = t.world[touch.v as usize];
                let side = n.dot(p - plane_point);
                let toward = if down { side > 0. } else { side < 0. };
                toward.then(|| {
                    let k = (touch.weight * strength).clamp(0., 1.);
                    (touch.v, p + brushes::clay(plane_point, n, p, 1., k))
                })
            })
            .collect()
    }

    /// SCT-28..31 Flatten (Plane): points above the plane come down to it
    /// (Scrape), or with Inverted Flatten points below rise to it (Fill).
    /// The plane is the brush centre's each dab, or with Mouse Down the
    /// press's for the whole stroke. Ctrl pushes up instead.
    #[allow(clippy::too_many_arguments)]
    fn flatten_moves(
        &mut self,
        i: usize,
        dab: &Dab,
        touches: &[Touch],
        average: DVec3,
        mirrored: bool,
        strength: f64,
        share: f64,
    ) -> Vec<(u32, DVec3)> {
        let settings = self.setup.settings;
        let r = dab.radius;
        let t = &self.targets[i];
        if self.setup.ctrl {
            return touches
                .iter()
                .map(|touch| {
                    let p = t.world[touch.v as usize];
                    (
                        touch.v,
                        p + brushes::draw(average, r, strength.abs() * share, touch.weight),
                    )
                })
                .collect();
        }
        let (point, n) = if settings.normal_mode == NormalMode::MouseDown {
            let (p, n) = *self
                .press_plane
                .get_or_insert((dab.center, self.mouse_down.unwrap_or(dab.normal)));
            if mirrored {
                (
                    t.mirror_point(self.setup.symmetry, p),
                    t.mirror_dir(self.setup.symmetry, n),
                )
            } else {
                (p, n)
            }
        } else {
            (
                dab.center,
                self.brush_normal(t, settings.normal_mode, dab, average, mirrored),
            )
        };
        if n == DVec3::ZERO {
            return Vec::new();
        }
        let t = &self.targets[i];
        touches
            .iter()
            .filter_map(|touch| {
                let p = t.world[touch.v as usize];
                // Height above the plane in radii (the plane frame's z).
                let z = n.dot(p - point) / r;
                let side = if settings.inverted_flatten {
                    z < 0.
                } else {
                    z > 0.
                };
                side.then(|| {
                    let k = (touch.weight * strength.abs()).clamp(0., 1.);
                    (touch.v, p + brushes::plane(n, r, 1., z, k))
                })
            })
            .collect()
    }

    /// SC-81: Move and Emboss capture the points under the brush (and under
    /// its mirror) with their weights at the press.
    fn capture(&mut self, dab: &Dab) {
        self.grab_started = true;
        let tip = self.dab_tip(self.setup.settings.tip);
        let symmetry = self.setup.symmetry;
        for i in 0..self.targets.len() {
            let mut scratch = std::mem::take(&mut self.scratch);
            let touches = self.targets[i].touches(dab, &tip, &mut scratch);
            let touches = self.painted(i, dab, touches, false);
            let t = &mut self.targets[i];
            let mut weights: FxHashMap<u32, (f64, f64)> = FxHashMap::default();
            for touch in touches {
                weights.entry(touch.v).or_default().0 = touch.weight;
            }
            if symmetry.axis.is_some() {
                let mirrored = Dab {
                    center: t.mirror_point(symmetry, dab.center),
                    view: t.mirror_dir(symmetry, dab.view),
                    normal: t.mirror_dir(symmetry, dab.normal),
                    ..*dab
                };
                for touch in t.touches(&mirrored, &tip, &mut scratch) {
                    weights.entry(touch.v).or_default().1 = touch.weight;
                }
            }
            let mut grab: Vec<(u32, f64, f64)> =
                weights.into_iter().map(|(v, (a, b))| (v, a, b)).collect();
            grab.sort_unstable_by_key(|g| g.0);
            t.grab = grab;
            self.scratch = scratch;
        }
    }

    /// Move: the pointer has moved `delta` (world, in the screen plane at
    /// the grab's depth) since the press. Captured points follow it scaled
    /// by their weight and Offset Amount; Ctrl reverses it (SC-123). Move
    /// Mode Slide keeps each point's move in its tangent plane at the press
    /// (provisional, SCT-Q14).
    pub fn grab_to(&mut self, delta: DVec3) {
        if self.acting != SculptBrush::Move || !self.grab_started || !delta.is_finite() {
            return;
        }
        self.grab_delta = delta;
        let factor = self.setup.settings.offset_amount * if self.setup.ctrl { -1. } else { 1. };
        let slide = self.setup.settings.move_mode == MoveMode::Slide;
        let symmetry = self.setup.symmetry;
        for t in &mut self.targets {
            let mirrored = if symmetry.axis.is_some() {
                t.mirror_dir(symmetry, delta) * delta.length()
            } else {
                DVec3::ZERO
            };
            let grab = std::mem::take(&mut t.grab);
            for &(v, a, b) in &grab {
                let mut d = brushes::grab(delta, a) + brushes::grab(mirrored, b);
                if slide {
                    let n = t.base_normal(v);
                    d -= n * d.dot(n);
                }
                let p = t.world_base[v as usize] + d * factor;
                t.set(v, p);
            }
            t.grab = grab;
            t.keep_symmetric();
        }
    }

    /// Emboss (SCT-39): the pointer has moved `distance` metres to the
    /// right of the press (negative: left). The captured patch moves along
    /// each point's own normal by distance × Offset Amount × weight; Ctrl
    /// reverses it (SCT-40, SCT-42 provisional scale).
    pub fn emboss_to(&mut self, distance: f64) {
        if self.acting != SculptBrush::Emboss || !self.grab_started || !distance.is_finite() {
            return;
        }
        let amount =
            distance * self.setup.settings.offset_amount * if self.setup.ctrl { -1. } else { 1. };
        for t in &mut self.targets {
            let grab = std::mem::take(&mut t.grab);
            for &(v, a, b) in &grab {
                let n = t.base_normal(v);
                let p = t.world_base[v as usize] + brushes::inflate(n, amount, 1., a.max(b));
                t.set(v, p);
            }
            t.grab = grab;
            t.keep_symmetric();
        }
    }

    /// Whether a grab stroke has captured anything.
    pub fn grabbed(&self) -> bool {
        self.targets.iter().any(|t| !t.grab.is_empty())
    }

    /// Writes the points changed since the last flush into `scene` (once a
    /// frame). False when a mesh the stroke edits is gone or changed size.
    pub fn flush(&mut self, scene: &mut Scene) -> bool {
        let mut ok = true;
        for t in &mut self.targets {
            ok &= t.flush(scene);
        }
        ok
    }

    /// The finished stroke: every point it moved, in its mesh's own
    /// coordinates.
    pub fn edit(&self) -> SculptEdit {
        let items = self
            .targets
            .iter()
            .map(|t| SculptItemEdit {
                item: t.item,
                points: t
                    .touched
                    .iter()
                    .enumerate()
                    .filter(|&(_, &x)| x)
                    .filter_map(|(v, _)| {
                        let p = t.to_local.transform_point3(t.world[v]);
                        (p != t.base.vertices[v]).then_some((v as u32, p))
                    })
                    .collect(),
            })
            .filter(|i: &SculptItemEdit| !i.points.is_empty())
            .collect();
        SculptEdit {
            label: self.label(),
            items,
        }
    }

    /// The meshes as they were at the press, for putting the scene back.
    pub fn restore(&self, scene: &mut Scene) {
        for t in &self.targets {
            if let Some(item) = scene.item_mut(t.item) {
                item.mesh = t.base.clone().into();
            }
        }
    }
}

impl crate::editor::Editor {
    /// `Command::SculptStroke`: set the points a stroke moved, as one undo
    /// step named after the brush (SC-90..94). Locked and hidden points are
    /// left where they are; nothing moved records nothing.
    pub(crate) fn sculpt_stroke(&mut self, edit: SculptEdit) -> Result<()> {
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish the selection gesture before sculpting".into());
        }
        let before = self.scene.clone();
        let mut changed = false;
        for e in &edit.items {
            let item = self
                .scene
                .items
                .iter()
                .find(|i| i.id == e.item && i.is_mesh())
                .ok_or("A sculpted mesh no longer exists")?;
            let n = item.mesh.vertices.len();
            if e.points
                .iter()
                .any(|&(v, p)| v as usize >= n || !p.is_finite() || p.abs().max_element() > 1e9)
            {
                self.scene = before;
                return Err("Sculpted points must exist and stay within the scene range".into());
            }
            let pinned = crate::Protection::of(&item.mesh, &item.elements).map(|p| p.pinned);
            let moves: Vec<(u32, DVec3)> = e
                .points
                .iter()
                .copied()
                .filter(|&(v, p)| {
                    !pinned.as_ref().is_some_and(|f| f[v as usize])
                        && item.mesh.vertices[v as usize] != p
                })
                .collect();
            if moves.is_empty() {
                continue;
            }
            let item = self
                .scene
                .item_mut(e.item)
                .ok_or("A sculpted mesh no longer exists")?;
            let mesh = &mut *item.mesh;
            for (v, p) in moves {
                mesh.vertices[v as usize] = p;
            }
            changed = true;
        }
        if !changed {
            return Ok(());
        }
        let label = if edit.label.trim().is_empty() {
            "Sculpt Stroke".to_string()
        } else {
            edit.label
        };
        self.record(label, before);
        Ok(())
    }
}

#[path = "sculpt_weights.rs"]
mod weights;
pub use weights::{SCALE_DOWN_RATE, WeightEdit, WeightItemEdit};

#[cfg(test)]
#[path = "sculpt_toolbox_tests.rs"]
mod toolbox_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Editor, Face};

    /// `n` × `n` quads on the XY plane, 1 m wide, centred, facing +Z.
    fn grid(n: u32) -> Mesh {
        let mut mesh = Mesh::default();
        for j in 0..=n {
            for i in 0..=n {
                mesh.vertices.push(DVec3::new(
                    i as f64 / n as f64 - 0.5,
                    j as f64 / n as f64 - 0.5,
                    0.,
                ));
            }
        }
        for j in 0..n {
            for i in 0..n {
                let a = j * (n + 1) + i;
                mesh.faces.push(Face {
                    vertices: vec![a, a + 1, a + n + 2, a + n + 1],
                    uv: None,
                    material: String::new(),
                    smooth: false,
                    part: None,
                    material_label: None,
                });
            }
        }
        mesh
    }

    fn editor_with_grid(n: u32) -> (Editor, u64) {
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        let id = e.scene.active().unwrap().id;
        e.scene.active_mut().unwrap().mesh = grid(n).into();
        (e, id)
    }

    fn setup(brush: SculptBrush) -> StrokeSetup {
        StrokeSetup {
            brush,
            settings: brush.defaults(),
            ctrl: false,
            shift: false,
            symmetry: Symmetry::OFF,
            falloff: None,
            lock_borders: false,
            lock_corners: true,
            radius: 0.2,
            weight_map: None,
        }
    }

    fn dab_at(x: f64, y: f64) -> Dab {
        Dab {
            center: DVec3::new(x, y, 0.),
            radius: 0.2,
            view: DVec3::Z,
            normal: DVec3::Z,
            pressure: 1.,
        }
    }

    fn z_at(scene: &Scene, id: u64, v: usize) -> f64 {
        scene.item(id).unwrap().mesh.vertices[v].z
    }

    #[test]
    fn tip_weights_fall_from_centre_to_rim() {
        let mut tip = Tip::default();
        for shape in TipShape::ALL {
            tip.shape = shape;
            assert!((tip.weight(0.) - 1.).abs() < 1e-9, "{shape:?} centre");
            assert!(tip.weight(1.).abs() < 1e-9, "{shape:?} rim");
            assert_eq!(tip.weight(1.01), 0., "{shape:?} outside");
            assert!(tip.weight(0.3) >= tip.weight(0.6), "{shape:?} falls");
        }
        tip.kind = TipKind::Hard;
        assert_eq!(tip.weight(0.79), 1.);
        assert!(tip.weight(0.9) < 1. && tip.weight(0.9) > 0.);
        // Bias 50 % bends nothing; higher lifts the curve.
        tip.kind = TipKind::Smooth;
        tip.shape = TipShape::Linear;
        assert!((tip.weight(0.5) - 0.5).abs() < 1e-12);
        tip.bias = 0.8;
        assert!(tip.weight(0.5) > 0.5);
    }

    #[test]
    fn defaults_match_the_measured_values() {
        let s = SculptSettings::default();
        assert_eq!(s.push.offset_amount, 0.30);
        assert_eq!(s.inflate.offset_amount, 0.30);
        assert_eq!(s.smooth.offset_amount, 0.25);
        assert_eq!(s.grab.offset_amount, 1.0);
        assert_eq!(s.push.tip.size, 10.);
        assert_eq!(s.push.tip.shape, TipShape::Fat);
        assert_eq!(s.push.step, 0.05);
        assert_eq!(s.push.density_mode, DensityMode::Attenuate);
        assert_eq!(s.smooth.density_mode, DensityMode::Additive);
        assert_eq!(s.push.normal_mode, NormalMode::Average);
        assert_eq!(s.inflate.normal_mode, NormalMode::Vertex);
        assert_eq!(s.grab.normal_mode, NormalMode::BrushCenter);
        assert_eq!(s.push.smooth_amount, 0.5);
        assert_eq!(s.push.offset_distance, 0.1);
        assert!(!s.lock_borders && s.lock_corners);
        for b in SculptBrush::ALL {
            b.defaults().validate().unwrap();
        }
    }

    #[test]
    fn push_raises_the_middle_and_ctrl_digs() {
        for (ctrl, sign) in [(false, 1.), (true, -1.)] {
            let (e, id) = editor_with_grid(20);
            let mut scene = e.scene.clone();
            let mid = 10 * 21 + 10;
            let mut s = Stroke::begin(
                &scene,
                &[id],
                StrokeSetup {
                    ctrl,
                    ..setup(SculptBrush::Push)
                },
            )
            .unwrap()
            .unwrap();
            for k in 0..20 {
                s.dab(dab_at(0.002 * k as f64, 0.));
            }
            assert!(s.flush(&mut scene));
            let z = z_at(&scene, id, mid);
            assert!(z * sign > 0.001, "ctrl {ctrl}: z {z}");
            // Corners far outside the brush don't move.
            assert_eq!(z_at(&scene, id, 0), 0.);
            assert_eq!(s.label(), "Push Stroke");
        }
    }

    /// SC-45: pressure scales strength; full pressure is the mouse.
    #[test]
    fn pressure_scales_strength() {
        let run = |pressure: f64| {
            let (e, id) = editor_with_grid(20);
            let mut scene = e.scene.clone();
            let mut st = setup(SculptBrush::Push);
            st.settings.density_mode = DensityMode::Additive;
            let mut s = Stroke::begin(&scene, &[id], st).unwrap().unwrap();
            for _ in 0..10 {
                s.dab(Dab {
                    pressure,
                    ..dab_at(0., 0.)
                });
            }
            s.flush(&mut scene);
            z_at(&scene, id, 10 * 21 + 10)
        };
        let full = run(1.);
        let half = run(0.5);
        assert!(full > 0.001, "{full}");
        assert!((half - full * 0.5).abs() < 1e-9, "{half} vs {full}");
        assert_eq!(run(0.), 0.);
    }

    /// SC-62 Low Pressure: light dabs smooth, hard ones sculpt; other
    /// Smooth Modes sculpt at any pressure.
    #[test]
    fn low_pressure_smooths_light_dabs() {
        let run = |mode: SmoothMode, pressure: f64| {
            let (mut e, id) = editor_with_grid(20);
            let mid = 10 * 21 + 10;
            e.scene.active_mut().unwrap().mesh.vertices[mid].z = 0.05;
            let mut scene = e.scene.clone();
            let mut st = setup(SculptBrush::Push);
            st.settings.smooth_mode = mode;
            let mut s = Stroke::begin(&scene, &[id], st).unwrap().unwrap();
            for _ in 0..5 {
                s.dab(Dab {
                    pressure,
                    ..dab_at(0., 0.)
                });
            }
            s.flush(&mut scene);
            assert_eq!(s.label(), "Push Stroke");
            z_at(&scene, id, mid)
        };
        let light = LOW_PRESSURE * 0.5;
        assert!(run(SmoothMode::LowPressure, light) < 0.05, "light smooths");
        assert!(run(SmoothMode::LowPressure, 0.9) > 0.05, "hard sculpts");
        assert!(run(SmoothMode::ShiftKey, light) > 0.05, "Shift Key sculpts");
        // A mouse is full pressure: Low Pressure sculpts as before.
        assert_eq!(
            run(SmoothMode::LowPressure, 1.),
            run(SmoothMode::ShiftKey, 1.)
        );
    }

    #[test]
    fn attenuate_caps_and_additive_keeps_adding() {
        let run = |mode: DensityMode| {
            let (e, id) = editor_with_grid(20);
            let mut scene = e.scene.clone();
            let mut st = setup(SculptBrush::Push);
            st.settings.density_mode = mode;
            let mut s = Stroke::begin(&scene, &[id], st).unwrap().unwrap();
            for _ in 0..200 {
                s.dab(dab_at(0., 0.));
            }
            s.flush(&mut scene);
            z_at(&scene, id, 10 * 21 + 10)
        };
        // Full offset at the centre = radius × 30 % = 0.06 m.
        let attenuate = run(DensityMode::Attenuate);
        assert!((attenuate - 0.06).abs() < 1e-6, "{attenuate}");
        let max = run(DensityMode::MaxStroke);
        assert!((max - 0.06).abs() < 1e-6, "{max}");
        let additive = run(DensityMode::Additive);
        assert!(additive > 0.1, "{additive}");
    }

    #[test]
    fn smooth_flattens_a_spike_and_shift_smooths_with_any_brush() {
        for shift in [false, true] {
            let (mut e, id) = editor_with_grid(20);
            let mid = 10 * 21 + 10;
            e.scene.active_mut().unwrap().mesh.vertices[mid].z = 0.2;
            let mut scene = e.scene.clone();
            let brush = if shift {
                SculptBrush::Push
            } else {
                SculptBrush::Smooth
            };
            let mut s = Stroke::begin(
                &scene,
                &[id],
                StrokeSetup {
                    shift,
                    ..setup(brush)
                },
            )
            .unwrap()
            .unwrap();
            assert_eq!(s.acting(), SculptBrush::Smooth);
            for _ in 0..10 {
                s.dab(dab_at(0., 0.));
            }
            s.flush(&mut scene);
            assert!(z_at(&scene, id, mid) < 0.1);
            assert_eq!(s.label(), "Smooth Stroke");
        }
    }

    #[test]
    fn lock_corners_and_borders_hold_points() {
        let (e, id) = editor_with_grid(4);
        let mut scene = e.scene.clone();
        // Pull the corner point up, then smooth over it with corners locked.
        scene.item_mut(id).unwrap().mesh.vertices[0].z = 0.3;
        let corner = DVec3::new(-0.5, -0.5, 0.);
        let mut st = setup(SculptBrush::Smooth);
        st.radius = 0.6;
        st.settings.tip.kind = TipKind::Hard;
        let mut s = Stroke::begin(&scene, &[id], st.clone()).unwrap().unwrap();
        s.dab(Dab {
            center: corner,
            radius: 0.6,
            ..dab_at(0., 0.)
        });
        s.flush(&mut scene);
        assert_eq!(z_at(&scene, id, 0), 0.3, "corner locked");
        // Lock Borders: a border point (not a corner) stays too.
        scene.item_mut(id).unwrap().mesh.vertices[2].z = 0.3;
        st.lock_borders = true;
        let mut s = Stroke::begin(&scene, &[id], st).unwrap().unwrap();
        s.dab(Dab {
            center: DVec3::new(0., -0.5, 0.),
            radius: 0.6,
            ..dab_at(0., 0.)
        });
        s.flush(&mut scene);
        assert_eq!(z_at(&scene, id, 2), 0.3, "border locked");
    }

    #[test]
    fn symmetry_mirrors_dabs() {
        let (e, id) = editor_with_grid(20);
        let mut scene = e.scene.clone();
        let mut st = setup(SculptBrush::Push);
        st.symmetry = Symmetry::x();
        let mut s = Stroke::begin(&scene, &[id], st).unwrap().unwrap();
        for _ in 0..20 {
            s.dab(dab_at(0.3, 0.));
        }
        s.flush(&mut scene);
        // Points at x = +0.3 and x = -0.3 (columns 16 and 4, row 10).
        let (right, left) = (
            z_at(&scene, id, 10 * 21 + 16),
            z_at(&scene, id, 10 * 21 + 4),
        );
        assert!(right > 0.001);
        assert!((right - left).abs() < 1e-9, "{right} vs {left}");
    }

    #[test]
    fn falloff_multiplies_the_weight() {
        let (e, id) = editor_with_grid(20);
        let mut scene = e.scene.clone();
        let mut st = setup(SculptBrush::Push);
        // A linear falloff that is 0 at x ≤ 0 and 1 at x ≥ 0.2.
        st.falloff = Some(crate::ActiveFalloff {
            first: crate::Falloff::Linear(crate::LinearFalloff {
                start: DVec3::new(0.2, 0., 0.),
                end: DVec3::new(0., 0., 0.),
                ..Default::default()
            }),
            added: Vec::new(),
            weight_map: None,
            element: None,
            symmetry: Default::default(),
            stamps: Default::default(),
            path: Vec::new(),
        });
        let mut s = Stroke::begin(&scene, &[id], st).unwrap().unwrap();
        for _ in 0..20 {
            s.dab(dab_at(0., 0.));
        }
        s.flush(&mut scene);
        let (pos, neg) = (
            z_at(&scene, id, 10 * 21 + 12),
            z_at(&scene, id, 10 * 21 + 8),
        );
        assert!(pos > neg, "{pos} {neg}");
    }

    #[test]
    fn move_grabs_and_follows() {
        let (e, id) = editor_with_grid(20);
        let mut scene = e.scene.clone();
        let mut s = Stroke::begin(&scene, &[id], setup(SculptBrush::Move))
            .unwrap()
            .unwrap();
        s.dab(dab_at(0., 0.));
        assert!(s.grabbed());
        s.grab_to(DVec3::new(0., 0., 0.1));
        s.grab_to(DVec3::new(0., 0., 0.2));
        s.flush(&mut scene);
        let mid = 10 * 21 + 10;
        assert!((z_at(&scene, id, mid) - 0.2).abs() < 1e-9);
        // Further dabs don't capture more.
        s.dab(dab_at(0.4, 0.4));
        assert_eq!(z_at(&scene, id, 18 * 21 + 18), 0.);
        assert_eq!(s.label(), "Move Stroke");
    }

    #[test]
    fn pick_finds_the_surface_and_follows_edits() {
        let (e, id) = editor_with_grid(20);
        let mut scene = e.scene.clone();
        let mut s = Stroke::begin(&scene, &[id], setup(SculptBrush::Move))
            .unwrap()
            .unwrap();
        let hit = s.pick(DVec3::new(0.01, 0.01, 5.), -DVec3::Z).unwrap();
        assert!((hit.point.z).abs() < 1e-9 && (hit.depth - 5.).abs() < 1e-9);
        assert!(hit.normal.dot(DVec3::Z) > 0.99);
        s.dab(dab_at(0., 0.));
        s.grab_to(DVec3::new(0., 0., 0.5));
        s.flush(&mut scene);
        let hit = s.pick(DVec3::new(0.0, 0.0, 5.), -DVec3::Z).unwrap();
        assert!((hit.point.z - 0.5).abs() < 0.05, "{:?}", hit.point);
        assert!(s.pick(DVec3::new(3., 3., 5.), -DVec3::Z).is_none());
    }

    #[test]
    fn back_facing_points_are_left_alone() {
        let (e, id) = editor_with_grid(10);
        let mut scene = e.scene.clone();
        let mut s = Stroke::begin(&scene, &[id], setup(SculptBrush::Push))
            .unwrap()
            .unwrap();
        // Seen from below: the grid faces away.
        s.dab(Dab {
            view: -DVec3::Z,
            ..dab_at(0., 0.)
        });
        s.flush(&mut scene);
        assert!(s.edit().is_empty());
    }

    #[test]
    fn a_stroke_is_one_undo_step() {
        let (mut e, id) = editor_with_grid(20);
        let steps = e.undo.len();
        let mut scene = e.scene.clone();
        let mut s = Stroke::begin(&scene, &[id], setup(SculptBrush::Push))
            .unwrap()
            .unwrap();
        for k in 0..30 {
            s.dab(dab_at(-0.2 + 0.01 * k as f64, 0.));
        }
        s.flush(&mut scene);
        let edit = s.edit();
        assert!(!edit.is_empty());
        e.dispatch(Command::SculptStroke(Box::new(edit))).unwrap();
        assert_eq!(e.undo.len(), steps + 1);
        assert_eq!(e.undo.last().unwrap().label, "Push Stroke");
        let mid = 10 * 21 + 10;
        let raised = z_at(&e.scene, id, mid);
        assert!(raised > 0.);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(z_at(&e.scene, id, mid), 0.);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(z_at(&e.scene, id, mid), raised);
        // Nothing moved: no step.
        e.dispatch(Command::SculptStroke(Box::new(SculptEdit {
            label: "Push Stroke".into(),
            items: vec![],
        })))
        .unwrap();
        assert_eq!(e.undo.len(), steps + 1);
        // Bad points are refused and change nothing.
        let bad = SculptEdit {
            label: "Push Stroke".into(),
            items: vec![SculptItemEdit {
                item: id,
                points: vec![(0, DVec3::splat(f64::NAN))],
            }],
        };
        assert!(e.dispatch(Command::SculptStroke(Box::new(bad))).is_err());
        assert_eq!(e.undo.len(), steps + 1);
    }

    #[test]
    fn locked_points_never_move() {
        let (mut e, id) = editor_with_grid(20);
        let mid = 10 * 21 + 10;
        e.scene
            .active_mut()
            .unwrap()
            .elements
            .locked
            .vertices
            .insert(mid as u32);
        let mut scene = e.scene.clone();
        let mut s = Stroke::begin(&scene, &[id], setup(SculptBrush::Push))
            .unwrap()
            .unwrap();
        for _ in 0..20 {
            s.dab(dab_at(0., 0.));
        }
        s.flush(&mut scene);
        assert_eq!(z_at(&scene, id, mid), 0.);
        assert!(z_at(&scene, id, mid + 1) > 0.);
        // The command refuses to move it too.
        let edit = SculptEdit {
            label: "Push Stroke".into(),
            items: vec![SculptItemEdit {
                item: id,
                points: vec![(mid as u32, DVec3::new(0., 0., 1.))],
            }],
        };
        e.dispatch(Command::SculptStroke(Box::new(edit))).unwrap();
        assert_eq!(z_at(&e.scene, id, mid), 0.);
    }

    /// Opt-in cost split of a stroke frame on 100k points (release):
    /// `cargo test --release -p operandi-core sculpt_core_probe -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn sculpt_core_probe() {
        let (e, id) = editor_with_grid(316);
        let mut scene = e.scene.clone();
        let t = std::time::Instant::now();
        let mut st = setup(SculptBrush::Push);
        st.radius = 0.21; // 100 px at 240 px per 0.5 m wide view
        let mut s = Stroke::begin(&scene, &[id], st).unwrap().unwrap();
        println!("begin {:.1} ms", t.elapsed().as_secs_f64() * 1e3);
        let (mut dab_ms, mut flush_ms, mut pick_ms) = (0., 0., 0.);
        let keep = scene.items[0].mesh.clone();
        for k in 0..50 {
            let t = std::time::Instant::now();
            for j in 0..3 {
                let x = -0.3 + 0.012 * (3 * k + j) as f64;
                s.dab(Dab {
                    radius: 0.21,
                    ..dab_at(x, 0.)
                });
            }
            dab_ms += t.elapsed().as_secs_f64() * 1e3;
            let t = std::time::Instant::now();
            // The viewport holds last frame's mesh, so flush copies.
            let _held = scene.item(id).unwrap().mesh.clone();
            s.flush(&mut scene);
            flush_ms += t.elapsed().as_secs_f64() * 1e3;
            let t = std::time::Instant::now();
            s.pick(DVec3::new(0.0, 0.0, 3.), -DVec3::Z);
            pick_ms += t.elapsed().as_secs_f64() * 1e3;
        }
        drop(keep);
        println!(
            "per frame: 3 dabs {:.2} ms, flush {:.2} ms, pick {:.3} ms",
            dab_ms / 50.,
            flush_ms / 50.,
            pick_ms / 50.
        );
        let t = std::time::Instant::now();
        let edit = s.edit();
        println!(
            "edit {:.1} ms ({} points)",
            t.elapsed().as_secs_f64() * 1e3,
            edit.items[0].points.len()
        );
    }

    #[test]
    fn saved_settings_round_trip_and_bad_ones_reset() {
        let mut s = SculptSettings::default();
        s.push.tip.size = 42.;
        s.lock_borders = true;
        let text = serde_json::to_string(&s).unwrap();
        let back: SculptSettings = serde_json::from_str(&text).unwrap();
        assert_eq!(back, s);
        let mut bad = s.clone();
        bad.push.tip.size = -5.;
        assert_eq!(bad.sanitized().push, SculptBrush::Push.defaults());
    }

    /// A bumpy grid symmetric across x = 0 (column i mirrors column n - i).
    fn bumpy(n: u32) -> Mesh {
        let mut m = grid(n);
        for p in &mut m.vertices {
            p.z = 0.05 * (3. * p.x).cos() * (5. * p.y).sin() + 0.03 * (7. * p.y).cos();
        }
        m
    }

    /// Largest gap between a point and its partner's mirror image.
    fn asymmetry(scene: &Scene, id: u64, n: u32) -> f64 {
        let v = &scene.item(id).unwrap().mesh.vertices;
        let mut worst = 0_f64;
        for j in 0..=n {
            for i in 0..=n {
                let a = v[(j * (n + 1) + i) as usize];
                let b = v[(j * (n + 1) + (n - i)) as usize];
                worst = worst.max(a.distance(DVec3::new(-b.x, b.y, b.z)));
            }
        }
        worst
    }

    /// SC-70: every brush keeps a symmetric mesh symmetric, even with
    /// strokes over the centre line where the dab and its mirror overlap,
    /// on a moved, turned and scaled item, and with Topology pairing.
    #[test]
    fn symmetric_strokes_keep_the_mesh_symmetric() {
        let n = 40;
        let height =
            |x: f64, y: f64| 0.05 * (3. * x).cos() * (5. * y).sin() + 0.03 * (7. * y).cos();
        let mut bad = Vec::new();
        for (sweep, placed, topology) in [
            (-0.08, false, false),
            (-0.12, true, false),
            (-0.1, false, true),
        ] {
            for brush in SculptBrush::ALL {
                for (shift, ctrl) in [(false, false), (false, true), (true, false)] {
                    let mut e = Editor::default();
                    e.dispatch(Command::NewMesh).unwrap();
                    let id = e.scene.active().unwrap().id;
                    let item = e.scene.active_mut().unwrap();
                    item.mesh = bumpy(n).into();
                    if placed {
                        item.transform.translation = DVec3::new(0.4, -1., 2.);
                        item.transform.rotation = DVec3::new(0.3, 0.7, -0.2);
                        item.transform.scale = DVec3::new(1.5, 0.8, 1.2);
                    }
                    let mut scene = e.scene.clone();
                    let world = scene.world_matrix(id).unwrap();
                    let linear = DMat3::from_mat4(world);
                    for stroke in 0..4 {
                        let mut st = setup(brush);
                        st.symmetry = Symmetry {
                            topology,
                            ..Symmetry::x()
                        };
                        st.shift = shift;
                        st.ctrl = ctrl;
                        let Ok(Some(mut s)) = Stroke::begin(&scene, &[id], st) else {
                            continue;
                        };
                        // Sweeps across the centre line.
                        for k in 0..60 {
                            let t = k as f64 / 59.;
                            let x = sweep + 0.3 * t + 0.02 * stroke as f64;
                            let y = -0.2 + 0.4 * t;
                            s.dab(Dab {
                                center: world.transform_point3(DVec3::new(x, y, height(x, y))),
                                radius: 0.15,
                                view: (linear * DVec3::new(0.3, -0.2, 1.)).normalize(),
                                normal: (linear * DVec3::Z).normalize(),
                                pressure: 1.,
                            });
                            if k % 5 == 0 {
                                s.flush(&mut scene);
                            }
                        }
                        if s.grabbed() {
                            for k in 1..10 {
                                s.grab_to(linear * DVec3::new(0.01, 0.005, 0.02) * k as f64);
                                s.emboss_to(0.01 * k as f64);
                                s.flush(&mut scene);
                            }
                        }
                        s.flush(&mut scene);
                    }
                    let gap = asymmetry(&scene, id, n);
                    if gap.is_nan() || gap >= 1e-9 {
                        bad.push(format!(
                            "{brush:?} shift={shift} ctrl={ctrl} placed={placed} \
                             topology={topology}: {gap:.3e}"
                        ));
                    }
                }
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    /// A point with no mirror partner (the mesh was already lopsided there)
    /// still takes the brush; only pairs are kept exact.
    #[test]
    fn unpaired_points_still_sculpt() {
        let (mut e, id) = editor_with_grid(20);
        // Nudge one point on the left so its right-hand twin has no match.
        let lone = 10 * 21 + 18;
        let mut mesh = grid(20);
        mesh.vertices[10 * 21 + 2].y += 0.01;
        e.scene.active_mut().unwrap().mesh = mesh.into();
        let mut scene = e.scene.clone();
        let mut st = setup(SculptBrush::Push);
        st.symmetry = Symmetry::x();
        let mut s = Stroke::begin(&scene, &[id], st).unwrap().unwrap();
        for _ in 0..10 {
            s.dab(dab_at(0.4, 0.));
        }
        s.flush(&mut scene);
        assert!(z_at(&scene, id, lone) > 0.001);
    }
}
