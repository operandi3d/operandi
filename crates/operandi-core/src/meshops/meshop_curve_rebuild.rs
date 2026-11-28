//! Curve Rebuild as a Mesh Stack op (`cRebuild.mOp`, TYPE-IDS.md): each
//! chosen curve is rebuilt through evenly spaced points along the shape it
//! draws: Point Count points, or one every Point Spacing.
//!
//! Provisional (MOQ-46): the rebuilt curve is a Curve through the new
//! points (a line stays a line); an open curve keeps both its end points,
//! a closed one starts where it started. Points are spaced by length along
//! the drawn shape. Curve Rebuild Spans (`curve.rebuildSpan`) is its own
//! op (meshop_curve_rebuild_spans.rs).
use crate::curves::{self, CurveKind};
use crate::mesh_stack_builtin::{choice, count, distance};
use crate::meshop_curve_common::{chosen_curves, drop_faces, kind_of, points_of, push_curve};
use crate::meshop_types::{OpContext, OpDef, Typed, TypedOp};
use crate::{Mesh, Result};
use glam::DVec3;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Most points a rebuilt curve may have.
const MAX_POINTS: usize = 100_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CurveRebuildOp {
    /// 0 Point Count, 1 Point Spacing.
    pub pt_mode: u32,
    pub pt_count: u32,
    pub pt_distance: f64,
}
impl Default for CurveRebuildOp {
    fn default() -> Self {
        Self {
            pt_mode: 0,
            pt_count: 20,
            pt_distance: 0.05,
        }
    }
}

/// `count` points spaced evenly by length along the polyline `line`
/// (which, for a closed curve, ends back at its start).
fn spaced(line: &[DVec3], count: usize, closed: bool) -> Vec<DVec3> {
    let mut at = vec![0.];
    for w in line.windows(2) {
        at.push(at.last().unwrap() + w[0].distance(w[1]));
    }
    let total = *at.last().unwrap();
    let gaps = if closed { count } else { count - 1 };
    let mut out = Vec::with_capacity(count);
    let mut j = 0;
    for k in 0..count {
        let s = total * k as f64 / gaps as f64;
        while j + 2 < at.len() && at[j + 1] < s {
            j += 1;
        }
        let span = at[j + 1] - at[j];
        let t = if span > 0. { (s - at[j]) / span } else { 0. };
        out.push(line[j].lerp(line[j + 1], t.clamp(0., 1.)));
    }
    out
}

impl TypedOp for CurveRebuildOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        if self.pt_mode > 1 {
            return Err("Mode must be 0 (Point Count) or 1 (Point Spacing)".into());
        }
        if self.pt_mode == 1 && !(self.pt_distance.is_finite() && self.pt_distance > 0.) {
            return Err("Point Spacing must be more than 0".into());
        }
        let mesh = ctx.input;
        let mut out = mesh.clone();
        let mut gone = std::collections::BTreeSet::new();
        for f in chosen_curves(ctx)? {
            let (kind, closed) = kind_of(mesh, f);
            let mut line = curves::sample(&points_of(mesh, f), kind, closed, 32);
            if line.len() < 2 {
                continue;
            }
            if closed {
                line.push(line[0]);
            }
            let length: f64 = line.windows(2).map(|w| w[0].distance(w[1])).sum();
            let least = if closed { 3 } else { 2 };
            let n = if self.pt_mode == 0 {
                self.pt_count as usize
            } else {
                let gaps = (length / self.pt_distance).round() as usize;
                if closed { gaps } else { gaps + 1 }
            };
            if n > MAX_POINTS {
                return Err(format!(
                    "A rebuilt curve can have at most {MAX_POINTS} points"
                ));
            }
            let n = n.max(least);
            let points = spaced(&line, n, closed);
            let made = if kind == CurveKind::Line {
                CurveKind::Line
            } else {
                CurveKind::Curve {
                    start_control: false,
                    end_control: false,
                }
            };
            push_curve(&mut out, &points, made, closed, Some(f));
            gone.insert(f);
        }
        let out = drop_faces(&out, &gone)?;
        out.validate()?;
        Ok(out)
    }
}

pub(crate) fn entries() -> Vec<Arc<dyn OpDef>> {
    vec![Arc::new(Typed::<CurveRebuildOp>::new(
        "cRebuild.mOp",
        "Curve Rebuild",
        "Curves",
        true,
        vec![
            choice("ptMode", "Mode", &["Point Count", "Point Spacing"]),
            count("ptCount", "Point Count", 2, MAX_POINTS as u32),
            distance("ptDistance", "Point Spacing"),
        ],
    ))]
}
