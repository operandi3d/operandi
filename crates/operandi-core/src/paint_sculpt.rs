//! Painting stage 7: the shared painting parts on sculpt brushes, and the
//! sculpt mask (PNT-151, 241..247, 260). Painting builds only.
//!
//! A sculpt stroke can carry a [`SculptPaint`] (`Stroke::set_paint`). It
//! scales each point's brush weight:
//! - **Tips (PNT-246, 247):** a Procedural, Sphere, Texture, Stamp or Text
//!   tip replaces the Smooth/Hard profile. A Texture tip used as a mask is
//!   signed around its Offset: white pushes out, black pushes in, and the
//!   zero point (Offset 50 %: mid-grey) leaves the surface alone.
//! - **Inks (PNT-243):** the inks' result at the point scales the
//!   strength: Image Ink's brightness (seen through the view), Parametric
//!   Ink's mask, Random Ink's per-dab brightness. Projection Ink is paint
//!   only and does nothing here.
//! - **Mask (PNT-260):** each mesh's per-point mask (0..1) holds points
//!   still in proportion; Invert Mask flips that.
//!
//! The mask itself is painted with the Vertex Map Tools' brush machinery
//! on a hidden weight map ([`MASK_MAP`]) that only lives during a stroke;
//! the result is kept in `Scene::paint.masks`, never in the scene file.
use crate::brush_engine::{Tip, TipKind};
use crate::paint_inks::{InkDab, InkRun, InkTexel};
use crate::paint_stroke::PaintView;
use crate::paint_tips::TipExtra;
use crate::sculpt::Dab;
use glam::{DVec2, DVec3};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The hidden weight map a mask stroke paints (never saved: the app puts
/// the mesh back after the stroke).
pub const MASK_MAP: &str = "\u{1}Sculpt Mask";

/// Every mesh's sculpt mask: one value per point, 0 (free) to 1 (held).
pub type Masks = BTreeMap<u64, Arc<Vec<f32>>>;

/// The painting parts one sculpt stroke uses.
#[derive(Clone)]
pub struct SculptPaint {
    /// The tip beyond Smooth/Hard (`None`: the brush's own profile).
    pub extra: Option<TipExtra>,
    pub tip: Tip,
    /// A Texture tip used as a mask: its zero point (PNT-246).
    pub signed_offset: Option<f64>,
    pub inks: InkRun,
    pub view: PaintView,
    pub masks: Masks,
    pub mask_enabled: bool,
    pub invert_mask: bool,
    /// The dab's turn (nozzle rotation, Stamp's dragged angle), radians.
    pub angle: f64,
    /// Constrain to Background on (PNT-264): moved points go onto the
    /// background meshes, as the vertex tools' constraint puts them.
    pub background: Option<(
        Arc<crate::background_constraint::Background>,
        crate::background_constraint::BackgroundConstraint,
        crate::background_constraint::ConstraintView,
    )>,
}

impl std::fmt::Debug for SculptPaint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SculptPaint")
            .field("extra", &self.extra.is_some())
            .field("inks", &self.inks.inks.0.len())
            .field("masks", &self.masks.len())
            .finish()
    }
}

impl SculptPaint {
    pub fn new(view: PaintView) -> SculptPaint {
        SculptPaint {
            extra: None,
            tip: Tip::default(),
            signed_offset: None,
            inks: InkRun::default(),
            view,
            masks: Masks::new(),
            mask_enabled: true,
            invert_mask: false,
            angle: 0.,
            background: None,
        }
    }

    /// Where a point moving from `start` to `moved` (world) goes with
    /// Constrain to Background on.
    pub fn constrain(&self, start: DVec3, moved: DVec3) -> DVec3 {
        match &self.background {
            Some((bg, c, view)) => bg.constrain(c, view, Some(start), moved).unwrap_or(moved),
            None => moved,
        }
    }

    /// Whether this replaces the brush's own tip profile.
    pub fn replaces_tip(&self) -> bool {
        self.extra.is_some()
    }

    /// The flat tip the brush measures with when the profile comes from
    /// here: full weight across the dab (a Texture tip's square corners
    /// included).
    pub fn flat_tip(&self, tip: &Tip) -> Tip {
        Tip {
            kind: TipKind::Hard,
            soft_border: 0.,
            bias: 0.5,
            ..*tip
        }
    }

    /// How far a dab reaches, in radii.
    pub fn reach(&self) -> f64 {
        self.extra.as_ref().map_or(1., TipExtra::reach)
    }

    /// Point `v` of mesh `item`'s share of the brush: 1 free, 0 held.
    pub fn mask_factor(&self, item: u64, v: u32) -> f64 {
        if !self.mask_enabled {
            return 1.;
        }
        let m = self
            .masks
            .get(&item)
            .and_then(|m| m.get(v as usize))
            .map_or(0., |&x| f64::from(x).clamp(0., 1.));
        if self.invert_mask { m } else { 1. - m }
    }

    /// The tip's and inks' factor at world point `p` (normal `normal`) for
    /// dab `dab`, the `index`th of the stroke. Negative pushes the other
    /// way (a signed Texture tip below its zero point).
    pub fn gain(&self, p: DVec3, normal: DVec3, dab: &Dab, index: u32) -> f64 {
        let mut g = 1.;
        if let Some(extra) = &self.extra {
            // The point across the dab in screen axes (y down), in radii.
            let d = p - dab.center;
            let q = DVec2::new(d.dot(self.view.right), -d.dot(self.view.up)) / dab.radius;
            let uv = DVec2::new(p.x + p.z * 0.37, p.y - p.z * 0.61);
            let tip_eval = match (extra, self.signed_offset) {
                (TipExtra::Image { px, tip }, Some(off)) => {
                    // Signed around the zero point (PNT-246).
                    let plain = TipExtra::Image {
                        px: px.clone(),
                        tip: crate::paint_tips::ImageTip {
                            offset: 0.,
                            use_as_mask: true,
                            ..*tip
                        },
                    };
                    plain.eval(&self.tip, q, self.angle, uv, 0.).map(|(w, _)| {
                        let v = f64::from(w);
                        if v >= off {
                            (v - off) / (1. - off).max(1e-6)
                        } else {
                            (v - off) / off.max(1e-6)
                        }
                    })
                }
                (e, _) => e
                    .eval(&self.tip, q, self.angle, uv, 0.)
                    .map(|(w, _)| f64::from(w)),
            };
            match tip_eval {
                Some(w) => g *= w,
                None => return 0.,
            }
        }
        if !self.inks.is_empty() {
            let Some((screen, _)) = self.view.project(p) else {
                return 0.;
            };
            let texel = InkTexel {
                screen,
                world: p,
                normal,
                toward_viewer: (self.view.eye - p).normalize_or(-self.view.forward),
                uv: DVec2::ZERO,
            };
            let dab_info = InkDab {
                dab: index,
                pressure: dab.pressure,
                fg: [1.; 3],
                bg: [0.; 3],
            };
            let (c, k) = self.inks.apply(&texel, &dab_info, [1.; 3], 1.);
            g *= f64::from(k) * f64::from(operandi_paint::image::luma(c));
        }
        g
    }
}
