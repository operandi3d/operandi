//! Lace Geometry as a Mesh Stack op (`laceGeom`, MESHOPS-TIDY TB-50..54):
//! adds a capped tube along a guide curve held by another mesh (the Guide
//! Mesh), leaving the mesh's own geometry alone.
//!
//! The tube runs along curve Guide Curve ID (counting the guide's curves
//! from 0) from Start to End of its length. Rings sit at the ends and
//! wherever the curve has turned by Refinement Angle since the last ring,
//! so a straight run has rings only at its ends. Each ring is the profile
//! (Circle, Oval, Square, Rectangle) of Radius, its height axis kept as
//! close to world +Y as the curve allows (+Z where the curve starts
//! vertical) and carried along without twisting.
//!
//! Provisional (MOQ-69): Custom Profile isn't built (a note, and the
//! circle is used); Start ≥ End adds nothing (TB-Q6: trimmed properly); a
//! closed guide used whole makes a closed tube without caps. Make UVs
//! writes the "Texture" map: along the length by Pattern Size, around the
//! ring 0..1 (U) or the other way round (V).
use crate::mesh_stack_builtin::{choice, count, distance};
use crate::meshop_curve_common::{kind_of, points_of};
use crate::meshop_types::{Field, FieldKind, OpContext, OpDef, OpNote, Typed, TypedOp};
use crate::{Face, Mesh, Result};
use glam::{DQuat, DVec2, DVec3};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const LACE_ID: &str = "laceGeom";
/// Most rings one tube may have.
const MAX_RINGS: usize = 100_000;
/// Samples per curve span while finding the rings.
const SAMPLES: usize = 64;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LaceOp {
    /// The Guide Mesh item (0 = none).
    pub guide: u64,
    /// 0 Circle, 1 Oval, 2 Square, 3 Rectangle, 4 Custom Profile.
    pub shape: u32,
    pub sides: u32,
    pub aspect: f64,
    pub radius: f64,
    /// Radians.
    pub angle: f64,
    /// Radians.
    pub curve_angle: f64,
    pub start: f64,
    pub end: f64,
    pub curve_id: u32,
    pub pattern_size: f64,
    /// 0 None, 1 U, 2 V.
    pub uvs: u32,
    /// Kept, not used (profile presets aren't built).
    pub profile_preset: String,
}
impl Default for LaceOp {
    fn default() -> Self {
        Self {
            guide: 0,
            shape: 0,
            sides: 12,
            aspect: 0.5,
            radius: 0.002,
            angle: 0.,
            curve_angle: 5f64.to_radians(),
            start: 0.,
            end: 1.,
            curve_id: 0,
            pattern_size: 0.35,
            uvs: 1,
            profile_preset: String::new(),
        }
    }
}

impl LaceOp {
    /// The profile as (across, up) pairs on a unit-free ring of Radius.
    fn profile(&self) -> Vec<DVec2> {
        let r = self.radius;
        let ring = |angles: Vec<f64>, up: f64| -> Vec<DVec2> {
            angles
                .into_iter()
                .map(|a| {
                    let a = a + self.angle;
                    DVec2::new(a.cos() * r, a.sin() * r * up)
                })
                .collect()
        };
        let even = |n: u32| -> Vec<f64> {
            (0..n)
                .map(|i| std::f64::consts::TAU * i as f64 / n as f64)
                .collect()
        };
        let quarter = std::f64::consts::FRAC_PI_4;
        match self.shape {
            1 => ring(even(self.sides), self.aspect),
            2 => ring((0..4).map(|i| quarter * (2 * i + 1) as f64).collect(), 1.),
            3 => {
                let a = quarter * self.aspect;
                let pi = std::f64::consts::PI;
                ring(vec![a, pi - a, pi + a, -a], 1.)
            }
            _ => ring(even(self.sides), 1.),
        }
    }
}

/// The guide curve as a dense polyline, and whether it is closed.
fn guide_line(guide: &Mesh, id: u32) -> Option<(Vec<DVec3>, bool)> {
    let f = (0..guide.faces.len())
        .filter(|&f| guide.is_polyline(f))
        .nth(id as usize)?;
    let (kind, closed) = kind_of(guide, f);
    let mut line = crate::curves::sample(&points_of(guide, f), kind, closed, SAMPLES);
    line.dedup_by(|a, b| a.distance(*b) < 1e-12);
    if closed && line.len() > 1 {
        line.push(line[0]);
    }
    (line.len() >= 2).then_some((line, closed))
}

/// The part of `line` from `a` to `b` metres along it.
fn trimmed(line: &[DVec3], a: f64, b: f64) -> Vec<DVec3> {
    let mut out = Vec::new();
    let mut at = 0.;
    for w in line.windows(2) {
        let len = w[0].distance(w[1]);
        let next = at + len;
        let point = |s: f64| w[0].lerp(w[1], ((s - at) / len).clamp(0., 1.));
        if out.is_empty() && next >= a {
            out.push(point(a));
        }
        if !out.is_empty() {
            if next >= b {
                out.push(point(b));
                break;
            }
            out.push(w[1]);
        }
        at = next;
    }
    out.dedup_by(|p, q| p.distance(*q) < 1e-12);
    out
}

/// The ring places along `line`: its ends, and each point where the
/// direction has turned by `limit` since the last ring (TB-50).
fn rings(line: &[DVec3], limit: f64) -> Vec<usize> {
    let dir = |i: usize| (line[i + 1] - line[i]).normalize_or_zero();
    let mut out = vec![0];
    let mut last = dir(0);
    for i in 1..line.len() - 1 {
        let next = dir(i);
        // Turned past the limit by the next piece: a ring here.
        if last.angle_between(next) > limit + 1e-9 || last.angle_between(next).is_nan() {
            out.push(i);
            last = next;
        }
    }
    out.push(line.len() - 1);
    out
}

impl TypedOp for LaceOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let numbers = [
            self.aspect,
            self.radius,
            self.angle,
            self.curve_angle,
            self.start,
            self.end,
            self.pattern_size,
        ];
        if numbers.iter().any(|v| !v.is_finite()) {
            return Err("Lace Geometry values must be numbers".into());
        }
        if self.shape > 4 {
            return Err("Shape must be 0 (Circle) to 4 (Custom Profile)".into());
        }
        if self.uvs > 2 {
            return Err("Make UVs must be 0 (None), 1 (U) or 2 (V)".into());
        }
        if self.sides < 2 {
            return Err("Lace Geometry needs at least 2 Sides".into());
        }
        if self.radius <= 0. {
            return Err("Lace Geometry needs a Radius above 0".into());
        }
        if self.shape == 4 {
            ctx.note(OpNote {
                item: 0,
                text: "Custom Profile isn't built yet: the tube uses a Circle".into(),
                problem: false,
            });
        }
        // TB-54: no guide or no such curve leaves the mesh alone.
        let Some(source) = ctx.sources()?.into_iter().next() else {
            return Ok(ctx.input.clone());
        };
        let guide = source.local_mesh();
        let Some((line, closed)) = guide_line(&guide, self.curve_id) else {
            return Ok(ctx.input.clone());
        };
        let total: f64 = line.windows(2).map(|w| w[0].distance(w[1])).sum();
        let (s, e) = (self.start.clamp(0., 1.), self.end.clamp(0., 1.));
        if s >= e || total <= 0. {
            return Ok(ctx.input.clone());
        }
        let whole_loop = closed && s <= 0. && e >= 1.;
        let line = trimmed(&line, s * total, e * total);
        if line.len() < 2 {
            return Ok(ctx.input.clone());
        }
        let mut at = rings(&line, self.curve_angle.clamp(0., 89f64.to_radians()));
        if whole_loop {
            at.pop(); // the last ring is the first
        }
        if at.len() > MAX_RINGS {
            return Err(format!("A lace tube can have at most {MAX_RINGS} rings"));
        }
        // Each ring's tangent: the average of the pieces either side.
        let piece = |i: usize| (line[i + 1] - line[i]).normalize_or_zero();
        let n = line.len();
        let tangent = |i: usize| -> DVec3 {
            let (before, after) = match (i, whole_loop) {
                (0, true) => (piece(n - 2), piece(0)),
                (0, false) => (piece(0), piece(0)),
                (i, _) if i == n - 1 => (piece(n - 2), piece(n - 2)),
                (i, _) => (piece(i - 1), piece(i)),
            };
            (before + after).try_normalize().unwrap_or(after)
        };
        // Frames without twist: the height axis starts nearest world +Y
        // (+Z for a vertical start) and is carried from ring to ring.
        let t0 = tangent(at[0]);
        let pick = if t0.cross(DVec3::Y).length() < 1e-6 {
            DVec3::Z
        } else {
            DVec3::Y
        };
        let mut up = (pick - t0 * pick.dot(t0)).normalize();
        let mut prev = t0;
        let profile = self.profile();
        let k = profile.len();
        let mut tube = Mesh::default();
        let mut lengths = Vec::with_capacity(at.len());
        let mut along = 0.;
        let mut last_i = at[0];
        for &i in &at {
            for j in last_i..i {
                along += line[j].distance(line[j + 1]);
            }
            last_i = i;
            lengths.push(along);
            let t = tangent(i);
            up = (DQuat::from_rotation_arc(prev, t) * up).normalize();
            up = (up - t * up.dot(t)).try_normalize().unwrap_or(up);
            prev = t;
            let across = up.cross(t);
            for p in &profile {
                tube.vertices.push(line[i] + across * p.x + up * p.y);
            }
        }
        let face = |vertices: Vec<u32>| Face {
            vertices,
            uv: None,
            material: "Default".into(),
            smooth: false,
            part: None,
            material_label: None,
        };
        let rings_n = at.len();
        let spans = if whole_loop { rings_n } else { rings_n - 1 };
        let mut faces = Vec::new();
        let mut uv_rows: Vec<Vec<Option<DVec2>>> = Vec::new();
        let uv = |side: usize, ring_len: f64| -> Option<DVec2> {
            let length = ring_len / self.pattern_size.max(1e-12);
            let around = side as f64 / k as f64;
            Some(match self.uvs {
                2 => DVec2::new(length, around),
                _ => DVec2::new(around, length),
            })
        };
        let loop_len = lengths.last().copied().unwrap_or(0.)
            + if whole_loop {
                line[*at.last().unwrap()].distance(line[at[0]])
            } else {
                0.
            };
        for r in 0..spans {
            let r2 = (r + 1) % rings_n;
            let (l1, l2) = (lengths[r], if r2 == 0 { loop_len } else { lengths[r2] });
            for j in 0..k {
                let j2 = (j + 1) % k;
                let id = |ring: usize, side: usize| (ring * k + side) as u32;
                // Outward facing with the ring order above.
                faces.push(face(vec![id(r, j), id(r2, j), id(r2, j2), id(r, j2)]));
                uv_rows.push(vec![uv(j, l1), uv(j, l2), uv(j + 1, l2), uv(j + 1, l1)]);
            }
        }
        if !whole_loop {
            let last = rings_n - 1;
            // Each cap runs its ring the other way from the sides.
            faces.push(face((0..k as u32).collect()));
            faces.push(face(
                ((last * k) as u32..((last + 1) * k) as u32).rev().collect(),
            ));
            for _ in 0..2 {
                uv_rows.push(vec![None; k]);
            }
        }
        // The winding has to face away from the tube's middle.
        let flip = {
            let mut probe = tube.clone();
            probe.faces = faces[..1].to_vec().into();
            let n = probe.normal(0);
            let centre = line[at[0]].lerp(line[at[1 % rings_n]], 0.5);
            let mid: DVec3 = probe.faces[0]
                .vertices
                .iter()
                .map(|&v| probe.vertices[v as usize])
                .sum::<DVec3>()
                / 4.;
            n.dot(mid - centre) < 0.
        };
        if flip {
            for (f, row) in faces.iter_mut().zip(uv_rows.iter_mut()) {
                f.vertices.reverse();
                row.reverse();
            }
        }
        tube.faces = faces.into();
        if self.uvs > 0 {
            let mut maps = (*tube.uv_maps).clone();
            maps.insert("Texture".into(), uv_rows);
            tube.uv_maps = maps.into();
        }
        let mut out = ctx.input.clone();
        crate::append_mesh(&mut out, tube)?;
        out.validate()?;
        Ok(out)
    }
}

pub(crate) fn entries() -> Vec<Arc<dyn OpDef>> {
    vec![Arc::new(Typed::<LaceOp>::new(
        LACE_ID,
        "Lace Geometry",
        "Duplicate",
        false,
        vec![
            Field::new("guide", "Guide Mesh", FieldKind::Item),
            choice(
                "shape",
                "Shape",
                &["Circle", "Oval", "Square", "Rectangle", "Custom Profile"],
            ),
            count("sides", "Sides", 2, 1024),
            Field::new("aspect", "Aspect Ratio", FieldKind::Percent),
            distance("radius", "Radius"),
            Field::new("angle", "Shape Angle", FieldKind::Angle),
            Field::new("curveAngle", "Refinement Angle", FieldKind::Angle),
            Field::new("start", "Start", FieldKind::Percent),
            Field::new("end", "End", FieldKind::Percent),
            count("curveId", "Guide Curve ID", 0, u32::MAX),
            distance("patternSize", "Pattern Size"),
            choice("uvs", "Make UVs", &["None", "U", "V"]),
            Field::new("profilePreset", "Profile Preset", FieldKind::Hidden),
        ],
    ))]
}
