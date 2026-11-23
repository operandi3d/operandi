//! Mesh Stack for the Deform finish tools (modelling parity batch 1):
//! Quantize, Radial Align,
//! Linear Align and Relax Mesh under Operandi's own ids (the type list
//! names none for them; DFF-Q08, provisional). Each runs the direct tool's
//! function in deform_finish.rs on the points of the chosen polygons (the
//! whole mesh with none chosen); the Align ops align the border of the
//! chosen polygons. The op has no item placement, so Quantize's grid is in
//! the mesh's own space.
use crate::deform_finish::{self as df, LinearMode, RadialMode};
use crate::meshop_types::{Field, FieldKind, OpContext, OpDef, Typed, TypedOp};
use crate::{Mesh, Mode, Result, Selection};
use glam::DVec3;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;

/// Jitter's op (`xfrm.jitter.item`) is in mesh_stack_tools.rs and runs
/// `deform_finish::jitter_geometry`.
pub const JITTER_OP: &str = "xfrm.jitter.item";
pub const QUANTIZE_OP: &str = "xfrm.quantize.item";
pub const RADIAL_ALIGN_OP: &str = "xfrm.radialAlign.item";
pub const LINEAR_ALIGN_OP: &str = "xfrm.linearAlign.item";
pub const RELAX_MESH_OP: &str = "sculpt.relaxMesh.item";

fn points(ctx: &OpContext) -> Result<BTreeSet<u32>> {
    let mesh = ctx.input;
    Ok(match ctx.selection {
        None => (0..mesh.vertices.len() as u32).collect(),
        Some(faces) => faces
            .iter()
            .filter_map(|&f| mesh.faces.get(f as usize))
            .flat_map(|f| f.vertices.iter().copied())
            .collect(),
    })
}

fn runs(ctx: &OpContext) -> Result<(Vec<df::Run>, Vec<u32>)> {
    let selection = Selection {
        mode: Mode::Polygon,
        faces: ctx.chosen_or_all()?,
        ..Default::default()
    };
    Ok(df::align_runs(ctx.input, &selection))
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct QuantizeOp {
    pub step_x: f64,
    pub step_y: f64,
    pub step_z: f64,
}
impl TypedOp for QuantizeOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let o = df::QuantizeOptions {
            step: DVec3::new(self.step_x, self.step_y, self.step_z),
        };
        df::quantize_geometry(ctx.input, &points(ctx)?, &o, glam::DMat4::IDENTITY, None)
    }
}

/// Angles in radians, as op angle fields store them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RadialAlignOp {
    /// 0 Circle, 1 Arc, 2 N-Sided (DP-10 order).
    pub mode: u32,
    pub sides: u32,
    pub rotate: f64,
    pub radius: f64,
    pub start_angle: f64,
    pub central_angle: f64,
    pub weight: f64,
    pub smooth_iter: u32,
    pub flatten: bool,
    pub exponent: f64,
}
impl Default for RadialAlignOp {
    fn default() -> Self {
        let o = df::RadialAlignOptions::default();
        Self {
            mode: 0,
            sides: o.sides,
            rotate: o.rotate.to_radians(),
            radius: o.radius,
            start_angle: o.start_angle.to_radians(),
            central_angle: o.central_angle.to_radians(),
            weight: o.weight,
            smooth_iter: o.smoothing_iterations,
            flatten: o.flatten,
            exponent: o.exponent,
        }
    }
}
impl TypedOp for RadialAlignOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let mode = *RadialMode::ALL
            .get(self.mode as usize)
            .ok_or("Radial Align Mode must be 0 (Circle), 1 (Arc) or 2 (N-Sided)")?;
        let o = df::RadialAlignOptions {
            mode,
            sides: self.sides,
            rotate: self.rotate.to_degrees(),
            radius: self.radius,
            start_angle: self.start_angle.to_degrees(),
            central_angle: self.central_angle.to_degrees(),
            weight: self.weight,
            smoothing_iterations: self.smooth_iter,
            flatten: self.flatten,
            exponent: self.exponent,
        };
        let (runs, inner) = runs(ctx)?;
        df::radial_align_geometry(ctx.input, &runs, &inner, &o, None)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LinearAlignOp {
    /// 0 Line, 1 Curve.
    pub mode: u32,
    pub uniform: bool,
    pub weight: f64,
    pub smooth_iter: u32,
    pub align_length: bool,
    pub edge_length: f64,
}
impl Default for LinearAlignOp {
    fn default() -> Self {
        let o = df::LinearAlignOptions::default();
        Self {
            mode: 0,
            uniform: o.uniform,
            weight: o.weight,
            smooth_iter: o.smoothing_iterations,
            align_length: o.align_length,
            edge_length: o.edge_length,
        }
    }
}
impl TypedOp for LinearAlignOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let mode = *LinearMode::ALL
            .get(self.mode as usize)
            .ok_or("Linear Align Mode must be 0 (Line) or 1 (Curve)")?;
        let o = df::LinearAlignOptions {
            mode,
            uniform: self.uniform,
            weight: self.weight,
            smoothing_iterations: self.smooth_iter,
            align_length: self.align_length,
            edge_length: self.edge_length,
            ..Default::default()
        };
        let (runs, _) = runs(ctx)?;
        df::linear_align_geometry(ctx.input, &runs, &o, None)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelaxMeshOp {
    pub iter: u32,
    pub constrain: bool,
}
impl Default for RelaxMeshOp {
    fn default() -> Self {
        let o = df::RelaxMeshOptions::default();
        Self {
            iter: o.iterations,
            constrain: o.constrain,
        }
    }
}
impl TypedOp for RelaxMeshOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        let o = df::RelaxMeshOptions {
            iterations: self.iter,
            constrain: self.constrain,
        };
        df::relax_mesh_geometry(ctx.input, &points(ctx)?, &o)
    }
}

/// The entries, in the Deform folder.
pub(crate) fn entries() -> Vec<Arc<dyn OpDef>> {
    const FOLDER: &str = "Deform";
    let f = Field::new;
    let d = |k, l| Field::new(k, l, FieldKind::Distance);
    let c = |k, l| Field::new(k, l, FieldKind::Check);
    let count = |k, l, max| Field::new(k, l, FieldKind::Count { min: 0, max });
    let choice = |k, l, names: &[&'static str]| {
        Field::new(
            k,
            l,
            FieldKind::Choice(names.iter().map(|n| (*n).into()).collect()),
        )
    };
    vec![
        Arc::new(Typed::<QuantizeOp>::new(
            QUANTIZE_OP,
            "Quantize",
            FOLDER,
            true,
            vec![
                d("stepX", "Step X"),
                d("stepY", "Step Y"),
                d("stepZ", "Step Z"),
            ],
        )),
        Arc::new(Typed::<RadialAlignOp>::new(
            RADIAL_ALIGN_OP,
            "Radial Align",
            FOLDER,
            true,
            vec![
                choice("mode", "Mode", &["Circle", "Arc", "N-Sided"]),
                Field::new("sides", "Sides", FieldKind::Count { min: 3, max: 1000 }),
                f("rotate", "Rotate", FieldKind::Angle),
                d("radius", "Radius"),
                f("startAngle", "Start Angle", FieldKind::Angle),
                f("centralAngle", "Central Angle", FieldKind::Angle),
                f("weight", "Weight", FieldKind::Percent),
                count("smoothIter", "Smoothing Iterations", 10_000),
                c("flatten", "Flatten"),
                f("exponent", "Exponent", FieldKind::Number),
            ],
        )),
        Arc::new(Typed::<LinearAlignOp>::new(
            LINEAR_ALIGN_OP,
            "Linear Align",
            FOLDER,
            true,
            vec![
                choice("mode", "Mode", &["Line", "Curve"]),
                c("uniform", "Uniform"),
                f("weight", "Weight", FieldKind::Percent),
                count("smoothIter", "Smoothing Iterations", 10_000),
                c("alignLength", "Align Length"),
                d("edgeLength", "Edge Length"),
            ],
        )),
        Arc::new(Typed::<RelaxMeshOp>::new(
            RELAX_MESH_OP,
            "Relax Mesh",
            FOLDER,
            true,
            vec![
                Field::new("iter", "Iterations", FieldKind::Count { min: 1, max: 1000 }),
                c("constrain", "Constrain"),
            ],
        )),
    ]
}
