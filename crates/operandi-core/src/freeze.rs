//! Freeze v1 (specifications/freeze-2026-11-10, FRZ-01..10): curves become
//! plain straight-edged geometry, subdivision surfaces become plain
//! polygons (SDV-12, in `subdivision.rs`), both in one undo step.
//!
//! v2 (checked against the reference): selection limits what freezes,
//! Tessellation off freezes nothing, on-curve points are kept as samples
//! and every span gets 36 equal-parameter steps.
//!
//! Provisional choices (build guesses in the spec, FRZ-B01..B11 in
//! docs/SPECIFICATION_QUESTIONS.md):
//! - frozen curves are appended after the other polygons, not in place;
//! - Polygon mode with nothing selected, and Edge mode, act like Item mode;
//! - Fixed Spans off cuts adaptively by Curve Refinement Angle (FRZ-06
//!   Option A, Jason's choice 2026-11-11);
//! - a Faces result too short for a polygon (an open straight span cut
//!   adaptively gives two samples) becomes a two-point line instead.
use crate::curves::{self, CurveKind};
use crate::subdivision::SubdivisionSettings;
use crate::{Face, Mesh, PolygonRepresentation, Result};
use std::collections::BTreeSet;

/// FRZ-09 refusal.
pub const NOTHING_TO_FREEZE: &str = "Nothing to freeze";

/// SUB-50 "Freeze Curves as", in the drop-down order. The command form
/// numbers them differently (see [`FreezeCurves::parse`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FreezeCurves {
    TwoPointPolygons,
    #[default]
    Faces,
    Polylines,
}

impl FreezeCurves {
    pub const ALL: [FreezeCurves; 3] = [
        FreezeCurves::TwoPointPolygons,
        FreezeCurves::Faces,
        FreezeCurves::Polylines,
    ];
    pub fn label(self) -> &'static str {
        match self {
            FreezeCurves::TwoPointPolygons => "2 Point Polygons",
            FreezeCurves::Faces => "Faces",
            FreezeCurves::Polylines => "Polylines",
        }
    }
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|c| *c == self).unwrap_or(1)
    }
    /// A command value (registry, 2026-11-11): 0 = Faces and 1 = Polylines
    /// as the reference; 2 = 2 Point Polygons is Operandi-only. Readable
    /// names work too; anything else is rejected.
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text.trim().to_ascii_lowercase().as_str() {
            "0" | "faces" => FreezeCurves::Faces,
            "1" | "polylines" => FreezeCurves::Polylines,
            "2" | "twopointpolygons" => FreezeCurves::TwoPointPolygons,
            _ => return None,
        })
    }
}

/// Smallest and largest Curve Refinement Angle, degrees (FRZ-Q5 guess).
pub const MIN_REFINEMENT_DEGREES: f64 = 0.01;
pub const MAX_REFINEMENT_DEGREES: f64 = 180.;

/// The Freeze options that act in v1 (the other SUB-50 fields are greyed).
#[derive(Clone, Debug, PartialEq)]
pub struct FreezeOptions {
    pub curves_as: FreezeCurves,
    pub tessellation: bool,
    pub fixed_spans: bool,
    /// Curve Refinement Angle in degrees, used when `fixed_spans` is off.
    pub refinement_degrees: f64,
}

impl Default for FreezeOptions {
    fn default() -> Self {
        Self {
            curves_as: FreezeCurves::Faces,
            tessellation: true,
            fixed_spans: true,
            refinement_degrees: 5.,
        }
    }
}

/// Whether element `f` is a smooth curve (Curve, B-Spline or Bezier).
pub fn is_curve(mesh: &Mesh, f: usize) -> bool {
    matches!(
        mesh.attributes.representations.get(f),
        Some(PolygonRepresentation::Polyline { curve, .. }) if !curve.is_line()
    )
}

/// The sample chain a curve freezes to (FRZ-05/06) and the number of
/// segments in each span. A closed curve's chain does not repeat its first
/// point.
pub fn curve_samples(
    points: &[glam::DVec3],
    kind: CurveKind,
    closed: bool,
    options: &FreezeOptions,
) -> (Vec<glam::DVec3>, Vec<usize>) {
    let counts = if options.fixed_spans {
        vec![curves::FREEZE_STEPS; curves::span_count(points, kind, closed)]
    } else {
        let degrees = options
            .refinement_degrees
            .clamp(MIN_REFINEMENT_DEGREES, MAX_REFINEMENT_DEGREES);
        curves::adaptive_counts(points, kind, closed, degrees.to_radians())
    };
    if counts.is_empty() {
        return (
            curves::sample(points, kind, closed, curves::FREEZE_STEPS),
            counts,
        );
    }
    let samples = curves::sample_spans(points, kind, closed, &|i| {
        counts.get(i).copied().unwrap_or(1)
    });
    (samples, counts)
}

/// What Freeze acts on (FRZ-01).
#[derive(Clone, Debug, PartialEq)]
pub enum FreezeScope {
    /// Every curve and subdivision polygon of the mesh.
    All,
    /// Only these polygons (Polygon mode with a selection).
    Polygons(Vec<u32>),
}

/// FRZ-01..09 on one mesh. Errors leave the mesh to the caller unchanged.
pub fn freeze(
    mesh: &Mesh,
    settings: &SubdivisionSettings,
    options: &FreezeOptions,
    scope: &FreezeScope,
) -> Result<Mesh> {
    // FRZ-07 (v2, observed): Tessellation off leaves everything alone.
    if !options.tessellation {
        return Err(format!("{NOTHING_TO_FREEZE}: Tessellation is off"));
    }
    let mut chosen = vec![matches!(scope, FreezeScope::All); mesh.faces.len()];
    if let FreezeScope::Polygons(faces) = scope {
        for &f in faces {
            if let Some(c) = chosen.get_mut(f as usize) {
                *c = true;
            }
        }
    }
    // G7 PC-7: live patches freeze first, into cells x cells polygons
    // (the first in the patch's place), while their curves are still
    // there to read. Broken ones stay (PC-Q5).
    let patched =
        crate::patch_live::tessellate(mesh, settings.patch_subdivisions, Some(&chosen), false)?;
    let froze_patches = patched.is_some();
    let patched = patched.unwrap_or_else(|| mesh.clone());
    chosen.resize(patched.faces.len(), false);
    let mesh = &patched;
    let curves: Vec<usize> = (0..mesh.faces.len())
        .filter(|&f| chosen[f] && is_curve(mesh, f))
        .collect();
    let surfaces = (0..mesh.faces.len()).any(|f| {
        chosen[f] && crate::subdivision::is_smooth_polygon(mesh, f) && !mesh.is_polyline(f)
    });
    if curves.is_empty() && !surfaces && !froze_patches {
        return Err(NOTHING_TO_FREEZE.into());
    }
    let (mut out, chosen) = if curves.is_empty() {
        (mesh.clone(), chosen)
    } else {
        freeze_curves(mesh, &curves, options, chosen)?
    };
    if !surfaces && !froze_patches && out == *mesh {
        return Err(NOTHING_TO_FREEZE.into());
    }
    if surfaces {
        let only = matches!(scope, FreezeScope::Polygons(_)).then_some(chosen.as_slice());
        out = crate::subdivision::freeze_only(&out, settings, only)?;
    }
    out.validate()?;
    Ok(out)
}

/// FRZ-04: replace `curves` with their frozen form. On-curve points (Bezier
/// anchors, Curve points) are reused as samples; handles and B-Spline
/// points go unless another polygon uses them. Returns the mesh and
/// `chosen` carried to its new polygon order.
fn freeze_curves(
    mesh: &Mesh,
    curves: &[usize],
    options: &FreezeOptions,
    chosen: Vec<bool>,
) -> Result<(Mesh, Vec<bool>)> {
    let line = |closed| PolygonRepresentation::Polyline {
        closed,
        curve: CurveKind::Line,
    };
    let mut generated = Mesh::default();
    // Generated point -> the source point it stands for.
    let mut reuse: Vec<Option<u32>> = Vec::new();
    let mut frozen = BTreeSet::new();
    for &f in curves {
        let Some(PolygonRepresentation::Polyline { closed, curve }) =
            mesh.attributes.representations.get(f).copied()
        else {
            continue;
        };
        let source = &mesh.faces[f];
        let control: Vec<_> = source
            .vertices
            .iter()
            .map(|&v| mesh.vertices[v as usize])
            .collect();
        let (samples, counts) = curve_samples(&control, curve, closed, options);
        if samples.len() < 2 {
            // Too few points to draw a curve: leave it as it is.
            continue;
        }
        if samples.len() > 4096 && options.curves_as != FreezeCurves::TwoPointPolygons {
            return Err(format!(
                "A frozen curve would have {} points; one polygon holds at most 4096. Use 2 Point Polygons or turn Fixed Spans off",
                samples.len()
            ));
        }
        frozen.insert(f);
        let base = generated.vertices.len() as u32;
        let n = samples.len() as u32;
        let mut stands_for = vec![None; samples.len()];
        if !counts.is_empty() {
            let on = curves::on_curve_points(control.len(), curve, closed, counts.len());
            let mut at = 0usize;
            for (j, point) in on.iter().enumerate() {
                if let (Some(p), Some(slot)) = (point, stands_for.get_mut(at)) {
                    *slot = Some(source.vertices[*p]);
                }
                at += counts.get(j).copied().unwrap_or(0);
            }
        }
        reuse.extend(stands_for);
        generated.vertices.extend(samples);
        let face = |vertices: Vec<u32>| Face {
            vertices,
            uv: None,
            ..source.clone()
        };
        let reps = &mut generated.attributes.representations;
        match options.curves_as {
            FreezeCurves::TwoPointPolygons => {
                let segments = if closed && n > 2 { n } else { n - 1 };
                for i in 0..segments {
                    generated
                        .faces
                        .push(face(vec![base + i, base + (i + 1) % n]));
                    reps.push(line(false));
                }
            }
            FreezeCurves::Faces if n >= 3 => {
                generated.faces.push(face((base..base + n).collect()));
                reps.push(PolygonRepresentation::Face);
                let at = generated.faces.len() - 1;
                // FRZ-04: a straight or S-shaped curve still gives a Face,
                // one with no overall facing; keep it as an explicit
                // degenerate face.
                if generated.normal(at) == glam::DVec3::ZERO {
                    generated.attributes.degenerate_faces.insert(at as u32);
                }
            }
            FreezeCurves::Faces => {
                generated.faces.push(face(vec![base, base + 1]));
                reps.push(line(false));
            }
            FreezeCurves::Polylines => {
                generated.faces.push(face((base..base + n).collect()));
                reps.push(line(closed && n > 2));
            }
        }
    }
    if frozen.is_empty() {
        return Ok((mesh.clone(), chosen));
    }
    let mut out = mesh.clone();
    let (point_start, face_start) = (out.vertices.len() as u32, out.faces.len());
    let new_faces = generated.faces.len();
    crate::mesh_attributes::append_mesh(&mut out, generated)?;
    // Reused samples point at the source's own points; their copies are
    // then unused and go away with the handles below.
    for face in &mut out.faces[face_start..] {
        for v in &mut face.vertices {
            if let Some(Some(original)) = reuse.get((*v - point_start) as usize) {
                *v = *original;
            }
        }
    }
    let mut chosen = chosen;
    // Frozen results are not frozen again as surfaces.
    chosen.extend(std::iter::repeat_n(false, new_faces));
    let kept: Vec<bool> = chosen
        .iter()
        .enumerate()
        .filter(|(f, _)| !frozen.contains(f))
        .map(|(_, &c)| c)
        .collect();
    out.remove_faces(&frozen)?;
    // Points no polygon uses now go away (loose points of the source
    // stay); so do weights on spans that lost a point.
    let mut keep = out.used_points();
    for (v, used) in mesh.used_points().into_iter().enumerate() {
        if !used {
            keep[v] = true;
        }
    }
    out.attributes
        .creases
        .retain(|c| keep[c.a as usize] && keep[c.b as usize]);
    crate::smooth_shift::compact(mesh, &mut out);
    out.attributes.extrude_edge_uses = crate::interchange::edge_exceptions(&out);
    Ok((out, kept))
}

impl crate::Editor {
    /// Freeze on the active mesh (FRZ-01..10, SDV-12): one undo step.
    /// Polygon mode with polygons selected freezes just those; Vertex mode
    /// with points selected freezes nothing; otherwise the whole mesh.
    pub(crate) fn freeze_mesh(&mut self, options: &FreezeOptions) -> Result<()> {
        self.subdivision_ready("Freeze")?;
        let item = self
            .scene
            .active()
            .filter(|i| self.scene.shows(i))
            .ok_or("Select a visible mesh to freeze")?;
        let selection = &self.scene.selection;
        let scope = match selection.mode {
            crate::Mode::Polygon if !selection.faces.is_empty() => {
                FreezeScope::Polygons(selection.faces.clone())
            }
            crate::Mode::Vertex if !selection.vertices.is_empty() => {
                return Err(format!(
                    "{NOTHING_TO_FREEZE}: Freeze doesn't act on selected points"
                ));
            }
            _ => FreezeScope::All,
        };
        let frozen = freeze(
            &item.mesh,
            &item.subdivision.clone().unwrap_or_default(),
            options,
            &scope,
        )?;
        let mut candidate = self.scene.clone();
        candidate.active_mut().ok_or("Missing mesh")?.mesh = frozen.into();
        // Item mode keeps the item selected (clear_components would drop it).
        if candidate.selection.mode != crate::Mode::Item {
            candidate.clear_components();
        }
        // The frozen mesh is new topology: selections remembered from the
        // other component modes no longer name anything (an edge selected
        // before switching to Polygon mode made Freeze fail validation).
        candidate.selection.vertices.clear();
        candidate.selection.edges.clear();
        candidate.selection.faces.clear();
        candidate.validate()?;
        let before = std::mem::replace(&mut self.scene, candidate);
        self.record("Freeze".into(), before);
        // A script's bare poly.freeze reuses these (command_exec).
        self.freeze_preset = options.clone();
        Ok(())
    }
}
