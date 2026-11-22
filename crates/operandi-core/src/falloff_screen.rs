//! Screen-space falloffs (falloff spec 2026-11-15 batch 4): Screen (FO-43),
//! Lasso (FO-52) and Airbrush (FO-44). Their weights depend on the view
//! they were drawn in. FO-Q08 (default): they are worked out when drawn or
//! painted and then stick to the points, so moving the camera afterwards
//! changes nothing. The app works them out with these functions and hands
//! them to the editor as stamps (`Command::FalloffStamps`), one weight per
//! point per item and layer.
use crate::Mesh;
use crate::falloff::FalloffShape;
use glam::{DMat4, DVec2, DVec3};
use serde::{Deserialize, Serialize};

/// FO-43 Screen: a cylinder through the screen around Center.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScreenFalloff {
    /// Screen pixels from the view's top-left corner.
    pub center: DVec2,
    /// Radius in pixels.
    pub size: f64,
    /// Off: only points on polygons facing the view are affected.
    pub transparent: bool,
}
impl Default for ScreenFalloff {
    /// FO-43 (measured): Size 20 px, Transparent off; Center is where it
    /// is drawn.
    fn default() -> Self {
        Self {
            center: DVec2::ZERO,
            size: 20.,
            transparent: false,
        }
    }
}
impl ScreenFalloff {
    pub fn validate(self) -> crate::Result<()> {
        if !self.center.is_finite() || !(self.size.is_finite() && self.size >= 0.) {
            return Err("Screen falloff needs a finite Center and a Size of zero or more".into());
        }
        Ok(())
    }
    /// 1 − d / Size, linear, 0 from Size on (hands-on check 2026-11-17,
    /// Soft Drag: full at the centre, falling straight to the edge).
    pub fn weight_at(self, screen: DVec2) -> f64 {
        let d = screen.distance(self.center);
        if self.size <= 0. || d >= self.size {
            0.
        } else {
            1. - d / self.size
        }
    }
}

/// FO-52 Lasso Style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LassoStyle {
    Lasso,
    /// Factory (measured).
    #[default]
    Rectangle,
    Circle,
    Ellipse,
}
impl LassoStyle {
    pub const ALL: [Self; 4] = [Self::Lasso, Self::Rectangle, Self::Circle, Self::Ellipse];
    pub fn label(self) -> &'static str {
        match self {
            Self::Lasso => "Lasso",
            Self::Rectangle => "Rectangle",
            Self::Circle => "Circle",
            Self::Ellipse => "Ellipse",
        }
    }
}

/// FO-52 Lasso: a shape drawn on screen, projected straight through.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LassoFalloff {
    pub style: LassoStyle,
    /// Pixels over which the weight fades outside the shape.
    pub soft_border: f64,
    /// Stored; the shape's weights are fixed when drawn (FO-Q08).
    pub world: bool,
}
impl Default for LassoFalloff {
    /// FO-52 (measured): Rectangle, Soft Border 0, Use World Transforms on.
    fn default() -> Self {
        Self {
            style: LassoStyle::Rectangle,
            soft_border: 0.,
            world: true,
        }
    }
}
impl LassoFalloff {
    pub fn validate(self) -> crate::Result<()> {
        if !(self.soft_border.is_finite() && self.soft_border >= 0.) {
            return Err("Lasso Soft Border must be zero or more".into());
        }
        Ok(())
    }
}

/// FO-44 Airbrush: weights painted by dragging.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AirbrushFalloff {
    /// The last brush position, in pixels.
    pub center: DVec2,
    /// Brush radius in pixels.
    pub size: f64,
    /// Added per pass, 0..1.
    pub strength: f64,
    pub transparent: bool,
}
impl Default for AirbrushFalloff {
    /// FO-44 (measured): Size 20 px, Strength 10 %, Transparent off.
    fn default() -> Self {
        Self {
            center: DVec2::ZERO,
            size: 20.,
            strength: 0.1,
            transparent: false,
        }
    }
}
impl AirbrushFalloff {
    pub fn validate(self) -> crate::Result<()> {
        if !(self.size.is_finite() && self.size >= 0.) {
            return Err("Airbrush Size must be zero or more".into());
        }
        if !(self.strength.is_finite() && (0. ..=1.).contains(&self.strength)) {
            return Err("Airbrush Strength must be from 0 % to 100 %".into());
        }
        Ok(())
    }
}

/// Smooth(d / size), 0 from size on; a zero size reaches nothing.
fn brush(d: f64, size: f64) -> f64 {
    if size <= 0. || d >= size {
        0.
    } else {
        FalloffShape::Smooth.weight(d / size)
    }
}

/// The view a screen-space falloff is worked out in: where a world point
/// lands in pixels (`None` behind the camera), and whether a polygon with
/// world normal `n` at world point `p` faces the view.
pub struct StampView<'a> {
    pub project: &'a dyn Fn(DVec3) -> Option<DVec2>,
    pub facing: &'a dyn Fn(DVec3, DVec3) -> bool,
}

/// Per point: whether a polygon around it faces the view (points on no
/// polygon count as facing).
pub fn facing_points(mesh: &Mesh, world: DMat4, view: &StampView<'_>) -> Vec<bool> {
    let mut on = vec![false; mesh.vertices.len()];
    let mut used = vec![false; mesh.vertices.len()];
    for f in mesh.faces.iter() {
        let corners = &f.vertices;
        if corners.len() < 3 {
            continue;
        }
        let pts: Vec<DVec3> = corners
            .iter()
            .filter_map(|&v| mesh.vertices.get(v as usize))
            .map(|&p| world.transform_point3(p))
            .collect();
        let mut n = DVec3::ZERO;
        for i in 0..pts.len() {
            n += pts[i].cross(pts[(i + 1) % pts.len()]);
        }
        let center = pts.iter().sum::<DVec3>() / pts.len().max(1) as f64;
        let faces = n != DVec3::ZERO && (view.facing)(center, n.normalize());
        for &v in corners {
            if let Some(u) = used.get_mut(v as usize) {
                *u = true;
            }
            if faces && let Some(o) = on.get_mut(v as usize) {
                *o = true;
            }
        }
    }
    for (o, u) in on.iter_mut().zip(used) {
        if !u {
            *o = true;
        }
    }
    on
}

/// `weight` of each point's screen position; points behind the camera
/// weigh 0, and so do points facing away unless `transparent`.
fn stamp(
    mesh: &Mesh,
    world: DMat4,
    view: &StampView<'_>,
    transparent: bool,
    weight: impl Fn(DVec2) -> f64,
) -> Vec<f64> {
    let facing = (!transparent).then(|| facing_points(mesh, world, view));
    mesh.vertices
        .iter()
        .enumerate()
        .map(|(v, &p)| {
            if facing.as_ref().is_some_and(|f| !f[v]) {
                return 0.;
            }
            (view.project)(world.transform_point3(p)).map_or(0., &weight)
        })
        .collect()
}

/// FO-43: every point's Screen weight in `view`.
pub fn screen_weights(
    mesh: &Mesh,
    world: DMat4,
    view: &StampView<'_>,
    f: ScreenFalloff,
) -> Vec<f64> {
    stamp(mesh, world, view, f.transparent, |q| f.weight_at(q))
}

/// FO-52: 1 inside `outline` (a closed screen polygon), fading to 0 over
/// `soft_border` pixels outside it (Smooth, proposed), 0 beyond. Lasso
/// projects straight through, so hidden points count.
pub fn lasso_weights(
    mesh: &Mesh,
    world: DMat4,
    view: &StampView<'_>,
    outline: &[DVec2],
    soft_border: f64,
) -> Vec<f64> {
    if outline.len() < 3 {
        return vec![0.; mesh.vertices.len()];
    }
    stamp(mesh, world, view, true, |q| {
        if inside(outline, q) {
            1.
        } else {
            brush(border_distance(outline, q), soft_border)
        }
    })
}

/// FO-44: one airbrush pass at `at`: each point under the brush gains
/// Strength × Smooth(d / Size), clamped to 1.
pub fn airbrush_dab(
    weights: &mut [f64],
    mesh: &Mesh,
    world: DMat4,
    view: &StampView<'_>,
    f: AirbrushFalloff,
    at: DVec2,
) {
    let add = stamp(mesh, world, view, f.transparent, |q| {
        f.strength * brush(q.distance(at), f.size)
    });
    for (w, a) in weights.iter_mut().zip(add) {
        *w = (*w + a).min(1.);
    }
}

/// FO-52: the screen outline a drag from `a` to `b` draws in `style`
/// (Lasso uses `path`, the pointer's trail). Rectangle runs corner to
/// corner, Circle is centred on `a` through `b`, Ellipse fills the box
/// from corner `a` to corner `b`.
pub fn lasso_outline(style: LassoStyle, a: DVec2, b: DVec2, path: &[DVec2]) -> Vec<DVec2> {
    let ring = |center: DVec2, radii: DVec2| -> Vec<DVec2> {
        (0..64)
            .map(|i| {
                let t = std::f64::consts::TAU * i as f64 / 64.;
                center + DVec2::new(radii.x * t.cos(), radii.y * t.sin())
            })
            .collect()
    };
    match style {
        LassoStyle::Lasso => path.to_vec(),
        LassoStyle::Rectangle => vec![a, DVec2::new(b.x, a.y), b, DVec2::new(a.x, b.y)],
        LassoStyle::Circle => ring(a, DVec2::splat(a.distance(b))),
        LassoStyle::Ellipse => ring((a + b) / 2., ((b - a) / 2.).abs()),
    }
}

/// Even-odd point-in-polygon.
fn inside(poly: &[DVec2], q: DVec2) -> bool {
    let mut on = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > q.y) != (b.y > q.y) && q.x < (b.x - a.x) * (q.y - a.y) / (b.y - a.y) + a.x {
            on = !on;
        }
        j = i;
    }
    on
}

/// Distance from `q` to the closed outline.
fn border_distance(poly: &[DVec2], q: DVec2) -> f64 {
    (0..poly.len())
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            let d = b - a;
            let t = ((q - a).dot(d) / d.length_squared().max(1e-12)).clamp(0., 1.);
            q.distance(a + d * t)
        })
        .fold(f64::INFINITY, f64::min)
}
