//! Curve MeshOps (modelling-parity specs, CURVE-COMMANDS-2D-SHAPES v1
//! CC-70..75; CURVE-EXTRUDE-PEN-CLONE v1; MeshOps TYPE-IDS):
//!
//! - The six 2D curve shapes (Create › Curves): Circle, Ellipse,
//!   Rectangle, Diamond, N-Sided and Star, each one closed curve in a
//!   plane, with the reference's channels and defaults.
//! - The path tools as ops: Curve Extrude, Curve Clone and Pen Clone, with
//!   their path kept as the op's own points (Operandi has no curve items
//!   for the op to read yet; provisional, CX-Q7).
//!
//! Every shape offers the "Curve" previous-op group (CC-75); the path ops
//! offer the Sweep's and Clone's groups.
use crate::curves::{CurveKind, bezier_points};
use crate::meshop_types::{Field, FieldKind, OpContext, OpDef, Typed, TypedOp};
use crate::path_tools::{PathKind, PathTool};
use crate::{Face, Mesh, PolygonRepresentation, Result};
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// The Bezier circle constant: handles at this fraction of the radius make
/// a quarter-circle span (0.5 × 0.5523 = 0.2761, CC-72).
pub const KAPPA: f64 = 0.552_284_749_830_793_4;

/// Points of a Spline circle (CC-74).
pub const SPLINE_CIRCLE_POINTS: usize = 16;

/// The shapes' Type choices (the full list, CC-74).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShapeType {
    #[default]
    Bezier,
    BSpline,
    Polyline,
    Spline,
}
impl ShapeType {
    pub const NAMES: [&'static str; 4] = ["Bezier", "B-Spline", "Polyline", "Spline"];
    fn from_index(i: u32) -> Self {
        match i {
            1 => Self::BSpline,
            2 => Self::Polyline,
            3 => Self::Spline,
            _ => Self::Bezier,
        }
    }
}

/// Which shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Circle,
    Ellipse,
    Rectangle,
    Diamond,
    NSided(u32),
    /// Sides, and the inner points' radius as a fraction of the outer.
    Star(u32, f64),
}
impl Shape {
    fn round(self) -> bool {
        matches!(self, Shape::Circle | Shape::Ellipse)
    }
}

/// The 2D plane of `axis` (its normal; 0 X, 1 Y, 2 Z): world directions
/// of the shape's u (Position X) and v (Position Y). Y is observed (the
/// shape in XZ, v along +Z); X and Z are provisional (CC-Q5).
pub fn plane_axes(axis: u32) -> (DVec3, DVec3) {
    match axis {
        0 => (DVec3::Y, DVec3::Z),
        2 => (DVec3::X, DVec3::Y),
        _ => (DVec3::X, DVec3::Z),
    }
}

/// The shape's corner (knot) points in 2D, centred on the origin: they
/// start at +v and turn toward +u (CC-73).
fn knots(shape: Shape, half: DVec2) -> Vec<DVec2> {
    let on =
        |angle: f64, scale: f64| DVec2::new(angle.sin() * half.x, angle.cos() * half.y) * scale;
    let tau = std::f64::consts::TAU;
    match shape {
        Shape::Circle | Shape::Ellipse | Shape::Diamond => {
            (0..4).map(|k| on(tau * k as f64 / 4., 1.)).collect()
        }
        // Corners from top-left, turning toward +u (provisional start).
        Shape::Rectangle => vec![
            DVec2::new(-half.x, half.y),
            DVec2::new(half.x, half.y),
            DVec2::new(half.x, -half.y),
            DVec2::new(-half.x, -half.y),
        ],
        Shape::NSided(n) => (0..n).map(|k| on(tau * k as f64 / n as f64, 1.)).collect(),
        Shape::Star(n, scale) => (0..2 * n)
            .map(|k| {
                let r = if k % 2 == 0 { 1. } else { scale };
                on(tau * k as f64 / (2 * n) as f64, r)
            })
            .collect(),
    }
}

/// One shape as a closed curve in 2D: its points and how they are drawn
/// (CC-72..74). A Type the shape doesn't offer falls back to Bezier.
pub fn shape_points(shape: Shape, kind: ShapeType, half: DVec2) -> (Vec<DVec2>, CurveKind) {
    let kind = match (shape.round(), kind) {
        (true, ShapeType::Spline) => ShapeType::Spline,
        (false, ShapeType::Polyline) => ShapeType::Polyline,
        _ => ShapeType::Bezier,
    };
    match kind {
        ShapeType::Spline => {
            let n = SPLINE_CIRCLE_POINTS;
            let points = (0..n)
                .map(|k| {
                    let a = std::f64::consts::TAU * k as f64 / n as f64;
                    DVec2::new(a.sin() * half.x, a.cos() * half.y)
                })
                .collect();
            (
                points,
                CurveKind::Curve {
                    start_control: false,
                    end_control: false,
                },
            )
        }
        ShapeType::Polyline => (knots(shape, half), CurveKind::Line),
        _ => {
            let k = knots(shape, half);
            let n = k.len();
            let anchors: Vec<[DVec3; 3]> = (0..n)
                .map(|i| {
                    let (prev, here, next) = (k[(i + n - 1) % n], k[i], k[(i + 1) % n]);
                    let (inn, out) = if shape.round() {
                        // Handles along the tangent (perpendicular to the
                        // radius), κ of the radius on that axis.
                        let t = DVec2::new(here.y / half.y * half.x, -here.x / half.x * half.y);
                        (here - t * KAPPA, here + t * KAPPA)
                    } else {
                        // Straight sides: both handles at the side's middle.
                        ((prev + here) / 2., (here + next) / 2.)
                    };
                    let p = |q: DVec2| DVec3::new(q.x, q.y, 0.);
                    [p(inn), p(here), p(out)]
                })
                .collect();
            let points = bezier_points(&anchors, true)
                .into_iter()
                .map(|q| DVec2::new(q.x, q.y))
                .collect();
            (points, CurveKind::Bezier)
        }
    }
}

/// The shape as a mesh: one closed curve polygon at `centre` (Position X/Y)
/// in the plane of `axis`.
pub fn shape_mesh(shape: Shape, kind: ShapeType, centre: DVec2, half: DVec2, axis: u32) -> Mesh {
    let (points, curve) = shape_points(shape, kind, half);
    let (u, v) = plane_axes(axis);
    let mut m = Mesh {
        vertices: points
            .iter()
            .map(|q| u * (q.x + centre.x) + v * (q.y + centre.y))
            .collect(),
        ..Default::default()
    };
    m.faces.push(Face {
        vertices: (0..points.len() as u32).collect(),
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    });
    m.attributes.representations = vec![PolygonRepresentation::Polyline {
        closed: true,
        curve,
    }];
    m
}

/// The new geometry appended to the op's input, tagged "Curve" (CC-75).
fn appended(ctx: &OpContext, g: Mesh) -> Result<Mesh> {
    let mut out = ctx.input.clone();
    let first = out.faces.len() as u32;
    crate::append_mesh(&mut out, g)?;
    let n = out.faces.len() as u32;
    ctx.tag(&mut out, "curve", first..n);
    Ok(out)
}

fn positive(v: f64, what: &str) -> Result<f64> {
    if v.is_finite() && v > 0. {
        Ok(v)
    } else {
        Err(format!("{what} must be more than 0"))
    }
}

macro_rules! shape_op {
    ($name:ident { $($field:ident : $ty:ty = $default:expr, $key:literal;)* } => |$s:ident| $shape:expr, $half:expr) => {
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            /// 0 Bezier, 1 B-Spline, 2 Polyline, 3 Spline (CC-74).
            #[serde(rename = "type")]
            pub kind: u32,
            #[serde(rename = "pos.X")]
            pub pos_x: f64,
            #[serde(rename = "pos.Y")]
            pub pos_y: f64,
            $(#[serde(rename = $key)] pub $field: $ty,)*
            /// The plane's normal: 0 X, 1 Y, 2 Z.
            pub axis: u32,
        }
        impl Default for $name {
            fn default() -> Self {
                Self { kind: 0, pos_x: 0., pos_y: 0., $($field: $default,)* axis: 1 }
            }
        }
        impl TypedOp for $name {
            fn run(&self, ctx: &OpContext) -> Result<Mesh> {
                let $s = self;
                crate::meshops_builtin::finite(&[$s.pos_x, $s.pos_y], "Position")?;
                if $s.axis > 2 {
                    return Err("Axis must be 0 (X), 1 (Y) or 2 (Z)".into());
                }
                let shape: Shape = $shape;
                let half: DVec2 = $half;
                appended(
                    ctx,
                    shape_mesh(shape, ShapeType::from_index($s.kind), DVec2::new($s.pos_x, $s.pos_y), half, $s.axis),
                )
            }
        }
    };
}

shape_op!(CircleOp { radius: f64 = 0.5, "radius"; } => |s| Shape::Circle,
    DVec2::splat(positive(s.radius, "Radius")?));
shape_op!(EllipseOp { size_x: f64 = 1., "size.X"; size_y: f64 = 0.5, "size.Y"; } => |s| Shape::Ellipse,
    DVec2::new(positive(s.size_x, "Size X")?, positive(s.size_y, "Size Y")?) / 2.);
shape_op!(RectangleOp { size_x: f64 = 1., "size.X"; size_y: f64 = 1., "size.Y"; } => |s| Shape::Rectangle,
    DVec2::new(positive(s.size_x, "Size X")?, positive(s.size_y, "Size Y")?) / 2.);
shape_op!(DiamondOp { size_x: f64 = 1., "size.X"; size_y: f64 = 0.5, "size.Y"; } => |s| Shape::Diamond,
    DVec2::new(positive(s.size_x, "Size X")?, positive(s.size_y, "Size Y")?) / 2.);
shape_op!(NSidedOp { size_x: f64 = 1., "size.X"; size_y: f64 = 1., "size.Y"; sides: u32 = 8, "sides"; } => |s| {
        if !(3..=1000).contains(&s.sides) {
            return Err("Sides must be 3 to 1000".into());
        }
        Shape::NSided(s.sides)
    },
    DVec2::new(positive(s.size_x, "Size X")?, positive(s.size_y, "Size Y")?) / 2.);
shape_op!(StarOp { size_x: f64 = 1., "size.X"; size_y: f64 = 1., "size.Y"; scale: f64 = 0.5, "scale"; sides: u32 = 5, "sides"; } => |s| {
        if !(2..=1000).contains(&s.sides) {
            return Err("Sides must be 2 to 1000".into());
        }
        Shape::Star(s.sides, positive(s.scale, "Scale")?)
    },
    DVec2::new(positive(s.size_x, "Size X")?, positive(s.size_y, "Size Y")?) / 2.);

// ---- the path tools as ops ----

/// Curve Extrude / Curve Clone / Pen Clone as an op: the tool's values,
/// with its path in the mesh's own space.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathOp {
    /// 0 Curve, 1 Bezier, 2 B-Spline, 3 Pen.
    #[serde(rename = "type")]
    pub kind: u32,
    /// The path's points (Hidden: set by the tool).
    pub points: Vec<[f64; 3]>,
    pub closed: bool,
    pub number: u32,
    pub auto: bool,
    /// Radians.
    pub smooth: f64,
    pub align: bool,
    pub local: bool,
    #[serde(rename = "byLength")]
    pub by_length: bool,
    /// Degrees.
    pub bank: f64,
    pub twist: f64,
    /// Sweep: Invert, Cap Start, Cap End; Clone: Invert Polygons.
    pub flip: bool,
    pub cap0: bool,
    pub cap1: bool,
    pub replace: bool,
    pub merge: bool,
    pub dist: f64,
    pub size: f64,
}
impl PathOp {
    fn defaults(sweep: bool, kind: PathKind) -> Self {
        let t = PathTool::default();
        Self {
            kind: PathKind::ALL.iter().position(|&k| k == kind).unwrap_or(0) as u32,
            points: Vec::new(),
            closed: false,
            number: t.steps.number,
            auto: t.steps.automatic,
            smooth: t.steps.smoothing_angle.to_radians(),
            align: t.steps.align_to_path,
            // The extrude buttons' values (CX-01).
            local: sweep,
            by_length: false,
            bank: 0.,
            twist: 0.,
            flip: sweep,
            cap0: !sweep,
            cap1: true,
            replace: false,
            merge: false,
            dist: 0.,
            size: 1.,
        }
    }
    /// The tool settings these values describe.
    pub fn tool(&self) -> PathTool {
        let mut t = PathTool::default();
        t.curve.kind = PathKind::ALL
            .get(self.kind as usize)
            .copied()
            .unwrap_or_default();
        t.curve.points = self.points.iter().map(|&p| DVec3::from_array(p)).collect();
        t.curve.closed = self.closed;
        t.curve.by_length = self.by_length;
        t.curve.bank = self.bank;
        t.curve.twist = self.twist;
        t.steps.number = self.number;
        t.steps.automatic = self.auto;
        t.steps.smoothing_angle = self.smooth.to_degrees();
        t.steps.align_to_path = self.align;
        t.pen_align = self.align;
        t.steps.start_at_source = self.local;
        t.sweep.invert = self.flip;
        t.sweep.cap_start = self.cap0;
        t.sweep.cap_end = self.cap1;
        t.sweep.size = self.size;
        t.clone.copies.replace_source = self.replace;
        t.clone.copies.flip = self.flip;
        t.clone.copies.merge = self.merge;
        t.clone.copies.distance = self.dist;
        t.clone.size = self.size;
        t
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CurveExtrudeOp(pub PathOp);
impl Default for CurveExtrudeOp {
    fn default() -> Self {
        Self(PathOp::defaults(true, PathKind::Curve))
    }
}
impl TypedOp for CurveExtrudeOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let t = self.0.tool();
        if !t.has_path() {
            return Ok(ctx.input.clone());
        }
        let faces = ctx.chosen_or_all()?;
        let r = crate::path_tools::sweep_geometry(ctx.input, &faces, &t)?;
        let mut out = r.mesh;
        let n = ctx.input.faces.len() as u32;
        let caps: Vec<u32> = r.end_caps.clone();
        // New polygons follow the input's (less the originals without Cap Start).
        let removed = if t.sweep.cap_start {
            0
        } else {
            faces.len() as u32
        };
        let sides: Vec<u32> = (n.saturating_sub(removed)..out.faces.len() as u32)
            .filter(|f| !caps.contains(f))
            .filter(|&f| !out.is_polyline(f as usize))
            .collect();
        ctx.tag(&mut out, "capEnd", caps.iter().copied());
        ctx.tag(&mut out, "caps", caps.iter().copied());
        ctx.tag(&mut out, "sides", sides);
        Ok(out)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CurveCloneOp(pub PathOp);
impl Default for CurveCloneOp {
    fn default() -> Self {
        Self(PathOp::defaults(false, PathKind::Curve))
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PenCloneOp(pub PathOp);
impl Default for PenCloneOp {
    fn default() -> Self {
        Self(PathOp::defaults(false, PathKind::Pen))
    }
}
fn clone_run(op: &PathOp, ctx: &OpContext) -> Result<Mesh> {
    let t = op.tool();
    if !t.has_path() {
        return Ok(ctx.input.clone());
    }
    let source: Vec<usize> = ctx
        .chosen_or_all()?
        .into_iter()
        .map(|f| f as usize)
        .collect();
    let placed = crate::path_tools::clone_transforms(ctx.input, &source, &t)?;
    let r = crate::duplicate::copy_faces(
        ctx.input,
        &source,
        &placed,
        &t.clone.copies,
        crate::duplicate::WeldAt::Original,
        t.clone.copies.distance,
    )?;
    let mut out = r.mesh;
    let end = out.faces.len() as u32;
    let start = end.saturating_sub(r.new_faces as u32);
    ctx.tag(&mut out, "clone", start..end);
    Ok(out)
}
impl TypedOp for CurveCloneOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        clone_run(&self.0, ctx)
    }
}
impl TypedOp for PenCloneOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let mut op = self.0.clone();
        op.kind = 3;
        clone_run(&op, ctx)
    }
}

fn field(key: &'static str, label: &'static str, kind: FieldKind) -> Field {
    Field::new(key, label, kind)
}

fn shape_fields(extra: Vec<Field>) -> Vec<Field> {
    let mut f = vec![
        field(
            "type",
            "Type",
            FieldKind::Choice(ShapeType::NAMES.iter().map(|n| (*n).into()).collect()),
        ),
        field("pos.X", "Position X", FieldKind::Distance),
        field("pos.Y", "Position Y", FieldKind::Distance),
    ];
    f.extend(extra);
    f.push(field(
        "axis",
        "Axis",
        FieldKind::Choice(vec!["X".into(), "Y".into(), "Z".into()]),
    ));
    f
}

fn path_fields(sweep: bool, pen: bool) -> Vec<Field> {
    // Values a kind doesn't use are kept but not shown (Hidden), so every
    // saved value has a field.
    let shown = |on: bool, f: Field| {
        if on {
            f
        } else {
            Field {
                kind: FieldKind::Hidden,
                ..f
            }
        }
    };
    vec![
        shown(
            !pen,
            field(
                "type",
                "Path Type",
                FieldKind::Choice(PathKind::ALL.iter().map(|k| k.label().into()).collect()),
            ),
        ),
        field("points", "Path", FieldKind::Hidden),
        field("closed", "Closed", FieldKind::Check),
        shown(
            !pen,
            field(
                "number",
                "Steps",
                FieldKind::Count {
                    min: 1,
                    max: 10_000,
                },
            ),
        ),
        shown(!pen, field("auto", "Automatic", FieldKind::Check)),
        shown(!pen, field("smooth", "Smoothing Angle", FieldKind::Angle)),
        shown(!pen, field("byLength", "By Length", FieldKind::Check)),
        shown(!pen, field("local", "Start at Source", FieldKind::Check)),
        field(
            "align",
            if pen { "Align" } else { "Align to Path" },
            FieldKind::Check,
        ),
        field("bank", "Bank", FieldKind::Number),
        field("twist", "Twist", FieldKind::Number),
        field(
            "flip",
            if sweep { "Invert" } else { "Invert Polygons" },
            FieldKind::Check,
        ),
        shown(sweep, field("cap0", "Cap Start", FieldKind::Check)),
        shown(sweep, field("cap1", "Cap End", FieldKind::Check)),
        shown(!sweep, field("replace", "Replace Source", FieldKind::Check)),
        shown(!sweep, field("merge", "Merge Vertices", FieldKind::Check)),
        shown(!sweep, field("dist", "Distance", FieldKind::Distance)),
        field("size", "Size", FieldKind::Percent),
    ]
}

const CURVE_GROUP: &[(&str, &str)] = &[("curve", "Curve")];
const SWEEP_GROUPS: &[(&str, &str)] =
    &[("caps", "Caps"), ("capEnd", "Cap End"), ("sides", "Sides")];
const CLONE_GROUP: &[(&str, &str)] = &[("clone", "Cloned Polygons")];

/// The curve ops, for the MeshOp type table.
pub(crate) fn entries() -> Vec<Arc<dyn OpDef>> {
    const CURVES: &str = "Create › Curves";
    let size = |_: f64| {
        vec![
            field("size.X", "Size X", FieldKind::Distance),
            field("size.Y", "Size Y", FieldKind::Distance),
        ]
    };
    let mut nsided = size(1.);
    nsided.push(field(
        "sides",
        "Sides",
        FieldKind::Count { min: 3, max: 1000 },
    ));
    let mut star = size(1.);
    star.push(field("scale", "Scale", FieldKind::Percent));
    star.push(field(
        "sides",
        "Sides",
        FieldKind::Count { min: 2, max: 1000 },
    ));
    vec![
        Arc::new(
            Typed::<CircleOp>::new(
                "curve.circle.item",
                "Circle",
                CURVES,
                false,
                shape_fields(vec![field("radius", "Radius", FieldKind::Distance)]),
            )
            .with_groups(CURVE_GROUP),
        ),
        Arc::new(
            Typed::<EllipseOp>::new(
                "curve.ellipse.item",
                "Ellipse",
                CURVES,
                false,
                shape_fields(size(0.5)),
            )
            .with_groups(CURVE_GROUP),
        ),
        Arc::new(
            Typed::<RectangleOp>::new(
                "curve.rectangle.item",
                "Rectangle",
                CURVES,
                false,
                shape_fields(size(1.)),
            )
            .with_groups(CURVE_GROUP),
        ),
        Arc::new(
            Typed::<DiamondOp>::new(
                "curve.diamond.item",
                "Diamond",
                CURVES,
                false,
                shape_fields(size(0.5)),
            )
            .with_groups(CURVE_GROUP),
        ),
        Arc::new(
            Typed::<NSidedOp>::new(
                "curve.nsided.item",
                "N-Sided",
                CURVES,
                false,
                shape_fields(nsided),
            )
            .with_groups(CURVE_GROUP),
        ),
        Arc::new(
            Typed::<StarOp>::new("curve.star.item", "Star", CURVES, false, shape_fields(star))
                .with_groups(CURVE_GROUP),
        ),
        Arc::new(
            Typed::<CurveExtrudeOp>::new(
                "curve.extrude.item",
                "Curve Extrude",
                "Duplicate",
                true,
                path_fields(true, false),
            )
            .with_groups(SWEEP_GROUPS),
        ),
        Arc::new(
            Typed::<CurveCloneOp>::new(
                "curve.clone.item",
                "Curve Clone",
                "Duplicate",
                true,
                path_fields(false, false),
            )
            .with_groups(CLONE_GROUP),
        ),
        Arc::new(
            Typed::<PenCloneOp>::new(
                "pen.clone.item",
                "Pen Clone",
                "Duplicate",
                true,
                path_fields(false, true),
            )
            .with_groups(CLONE_GROUP),
        ),
    ]
}
