//! Curve Sweep as a Mesh Stack op (`curve.sweep`, TYPE-IDS.md): a tube
//! along each curve in the mesh. A round profile of diameter Size starts
//! square to the curve's start and is swept along it by the Curve Extrude
//! engine (`path_tools::sweep_geometry`, CX-10..16): Steps sections (or
//! Auto, where the curve bends more than Auto Angle), Align keeping the
//! profile square to the curve, Rotation rolling it, Flip turning the
//! polygons inside out and End Caps closing the ends. Curve Index picks
//! curves by their order among the mesh's curves (empty: all).
//!
//! Provisional (MOQ-48): the profile is an 8-sided circle (Sweep Mode
//! "shape" with the current shape; preset shapes, Shape Subdiv Tolerance
//! and the profile's centre aren't built); End Caps lists None, Start,
//! End, Both; the curves stay. Channels for drawing, caching, start/end
//! percentages, normal finding and the scale and twist gradients aren't
//! kept. With Create Polygons off the op passes its input through.
use crate::curves::CurveKind;
use crate::mesh_stack_builtin::{check, choice, count, distance};
use crate::meshop_curve_common::{chosen_curves, kind_of, points_of};
use crate::meshop_types::{Field, FieldKind, OpContext, OpDef, Typed, TypedOp};
use crate::path_tools::{PathKind, PathTool, sections, sweep_geometry};
use crate::{Face, Mesh, Result};
use glam::DVec3;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Sides of the round profile (MOQ-48).
pub const PROFILE_SIDES: usize = 8;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CurveSweepOp {
    /// Curve Index, `0-2,5` form; empty = every curve.
    pub index: String,
    pub make_polys: bool,
    pub auto_steps: bool,
    pub steps: u32,
    /// Radians.
    pub walk_angle: f64,
    pub align: bool,
    pub flip: bool,
    /// 0 None, 1 Start, 2 End, 3 Both.
    pub cap_mode: u32,
    pub size: f64,
    /// Radians.
    pub rotate: f64,
}
impl Default for CurveSweepOp {
    fn default() -> Self {
        Self {
            index: String::new(),
            make_polys: true,
            auto_steps: false,
            steps: 12,
            walk_angle: 0.087_266,
            align: true,
            flip: false,
            cap_mode: 0,
            size: 0.25,
            rotate: 0.,
        }
    }
}

/// The path tool that draws element `f` of `mesh` as its path.
fn path_of(mesh: &Mesh, f: usize, op: &CurveSweepOp) -> PathTool {
    let (kind, closed) = kind_of(mesh, f);
    let mut t = PathTool::default();
    t.curve.kind = match kind {
        CurveKind::Line => PathKind::Pen,
        CurveKind::Curve {
            start_control,
            end_control,
        } => {
            t.curve.start_control = start_control;
            t.curve.end_control = end_control;
            PathKind::Curve
        }
        CurveKind::Bezier => PathKind::Bezier,
        CurveKind::BSpline => PathKind::BSpline,
    };
    t.curve.points = points_of(mesh, f);
    t.curve.closed = closed;
    // Rotation turns the profile itself (`profile`), so every ring, the
    // first one too, rolls by it (#51: the bank left the first ring as
    // it was).
    t.curve.bank = 0.;
    t.steps.number = op.steps;
    t.steps.automatic = op.auto_steps;
    t.steps.smoothing_angle = op.walk_angle.to_degrees();
    t.steps.align_to_path = op.align;
    t.pen_align = op.align;
    t.steps.start_at_source = false;
    t.sweep.invert = op.flip;
    // A closed curve makes a closed tube: no ends to cap (#51).
    t.sweep.cap_start = !closed && op.cap_mode & 1 == 1;
    t.sweep.cap_end = !closed && op.cap_mode & 2 == 2;
    t.sweep.size = 1.;
    t
}

/// A round profile polygon of diameter `size` at `at`, facing `along`,
/// rolled by `rotate` radians about it.
fn profile(at: DVec3, along: DVec3, size: f64, rotate: f64) -> Mesh {
    let (u, v) = along.any_orthonormal_pair();
    let roll = glam::DQuat::from_axis_angle(along.normalize_or_zero(), rotate);
    let (u, v) = (roll * u, roll * v);
    let r = size / 2.;
    let vertices = (0..PROFILE_SIDES)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / PROFILE_SIDES as f64;
            at + (u * a.cos() + v * a.sin()) * r
        })
        .collect();
    Mesh {
        vertices,
        faces: vec![Face {
            vertices: (0..PROFILE_SIDES as u32).collect(),
            uv: None,
            material: "Default".into(),
            smooth: false,
            part: None,
            material_label: None,
        }]
        .into(),
        ..Default::default()
    }
}

/// `mesh` with points that sit on one of the first ring's points (within
/// a millionth of `size`) merged into it: a closed sweep's last ring onto
/// its first.
fn join_coincident(mut mesh: Mesh, size: f64) -> Mesh {
    let tolerance = size.abs().max(1.) * 1e-6;
    let n = mesh.vertices.len();
    let mut map: Vec<u32> = (0..n as u32).collect();
    // Only the first ring (the profile's own points) can be landed on, so
    // each point is checked against those few.
    let first = PROFILE_SIDES.min(n);
    for (i, slot) in map.iter_mut().enumerate().skip(first) {
        if let Some(j) =
            (0..first).find(|&j| (mesh.vertices[i] - mesh.vertices[j]).length() <= tolerance)
        {
            *slot = j as u32;
        }
    }
    if map.iter().enumerate().all(|(i, &m)| m as usize == i) {
        return mesh;
    }
    let faces: Vec<Face> = mesh
        .faces
        .iter()
        .map(|f| {
            let mut f = f.clone();
            for v in &mut f.vertices {
                *v = map[*v as usize];
            }
            f
        })
        .collect();
    // Drop the merged-away points (the profile carries no point maps, so
    // only the polygons need renumbering).
    let mut renumber = vec![u32::MAX; n];
    let mut kept = Vec::with_capacity(n);
    for (i, &m) in map.iter().enumerate() {
        if m as usize == i {
            renumber[i] = kept.len() as u32;
            kept.push(mesh.vertices[i]);
        }
    }
    mesh.vertices = kept;
    mesh.faces = faces
        .into_iter()
        .map(|mut f| {
            for v in &mut f.vertices {
                *v = renumber[*v as usize];
            }
            f
        })
        .collect::<Vec<_>>()
        .into();
    mesh
}

impl TypedOp for CurveSweepOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        if !self.make_polys {
            return Ok(ctx.input.clone());
        }
        if self.cap_mode > 3 {
            return Err("End Caps must be 0 (None) to 3 (Both)".into());
        }
        if !(self.size.is_finite() && self.size > 0.) {
            return Err("Size must be more than 0".into());
        }
        let mut curves = chosen_curves(ctx)?;
        if !self.index.trim().is_empty() {
            let pick = crate::mesh_stack::SelectByIndex::parse(&self.index)?;
            curves = curves
                .into_iter()
                .enumerate()
                .filter(|(i, _)| pick.polygons.contains(&(*i as u32)))
                .map(|(_, f)| f)
                .collect();
        }
        let mut out = ctx.input.clone();
        let first = out.faces.len() as u32;
        for f in curves {
            let t = path_of(ctx.input, f, self);
            if !t.has_path() {
                continue;
            }
            let path = sections(&t, true)?;
            let Some(start) = path.first() else {
                continue;
            };
            let shape = profile(start.position, start.tangent, self.size, self.rotate);
            let mut swept = sweep_geometry(&shape, &[0], &t)?.mesh;
            if t.curve.closed {
                // The last ring lands on the first: join them (#51).
                swept = join_coincident(swept, self.size);
            }
            crate::append_mesh(&mut out, swept)?;
        }
        let n = out.faces.len() as u32;
        ctx.tag(&mut out, "sweep", first..n);
        out.validate()?;
        Ok(out)
    }
}

pub(crate) fn entries() -> Vec<Arc<dyn OpDef>> {
    vec![Arc::new(
        Typed::<CurveSweepOp>::new(
            "curve.sweep",
            "Curve Sweep",
            "Curves",
            true,
            vec![
                Field::new("index", "Curve Index", FieldKind::Text),
                check("makePolys", "Create Polygons"),
                check("autoSteps", "Auto"),
                count("steps", "Steps", 1, crate::path_tools::MAX_SECTIONS),
                Field::new("walkAngle", "Auto Angle", FieldKind::Angle),
                check("align", "Align"),
                check("flip", "Flip"),
                choice("capMode", "End Caps", &["None", "Start", "End", "Both"]),
                distance("size", "Size"),
                Field::new("rotate", "Rotation", FieldKind::Angle),
            ],
        )
        .with_groups(&[("sweep", "Sweep")]),
    )]
}
