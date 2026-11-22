//! Curve tab commands (CURVE-COMMANDS-2D-SHAPES v1): Make Open / Closed
//! Curve (CC-01..04), Make Curve Fill (CC-10..11) and Align Winding
//! (CC-20..21). Set Curves Color (CC-30) is a viewport setting and lives in
//! the app. Curve-type conversions, offsets and Bezier Tangent wait for a
//! hands-on check (CC-Q1..Q3).
use crate::curves::{self, CurveKind, bezier_anchors, bezier_points};
use crate::{Mesh, PolygonRepresentation, Result};
use glam::DVec3;

/// What Make Open / Closed Curve says with too few points (CC-03).
pub const TOO_FEW_POINTS: &str = "Select two or more vertices";

/// The representation of element `f` (legacy meshes have none: all Face).
fn representation(mesh: &Mesh, f: usize) -> PolygonRepresentation {
    mesh.attributes
        .representations
        .get(f)
        .copied()
        .unwrap_or(PolygonRepresentation::Face)
}

/// Give element `f` its representation, filling the legacy all-Face list.
pub(crate) fn set_representation(mesh: &mut Mesh, f: usize, r: PolygonRepresentation) {
    let reps = &mut mesh.attributes.representations;
    if reps.len() < mesh.faces.len() {
        reps.resize(mesh.faces.len(), PolygonRepresentation::Face);
    }
    reps[f] = r;
}

/// A closed curve element: its points and kind, or `None`.
fn closed_curve(mesh: &Mesh, f: usize) -> Option<CurveKind> {
    match representation(mesh, f) {
        PolygonRepresentation::Polyline {
            closed: true,
            curve,
        } => Some(curve),
        _ => None,
    }
}

/// CC-01/02: one new Curve through `points` in the order given; Closed
/// joins the last back to the first. Returns the mesh and the new curve.
pub fn make_curve(mesh: &Mesh, points: &[u32], closed: bool) -> Result<(Mesh, u32)> {
    let mut order: Vec<u32> = Vec::with_capacity(points.len());
    for &v in points {
        if (v as usize) < mesh.vertices.len() && !order.contains(&v) {
            order.push(v);
        }
    }
    if order.len() < 2 {
        return Err(TOO_FEW_POINTS.into());
    }
    let mut out = mesh.clone();
    crate::vertex_commands::append_face(&mut out, order, None);
    let f = out.faces.len() - 1;
    set_representation(
        &mut out,
        f,
        PolygonRepresentation::Polyline {
            closed,
            curve: CurveKind::Curve {
                start_control: false,
                end_control: false,
            },
        },
    );
    out.validate()?;
    Ok((out, f as u32))
}

/// Points per span when a Bezier or B-Spline is filled along its outline.
pub const FILL_STEPS: usize = 12;

/// CC-10: a flat Curve Fill polygon for each selected closed curve, the
/// curve kept. A Curve or polyline fills on its own points (observed); a
/// Bezier or B-Spline, which doesn't pass through all its points, fills on
/// new points along its outline. Returns the mesh and the fills. Nested
/// curves filling as one shape with holes is not built yet (CC-11).
pub fn make_curve_fill(mesh: &Mesh, selected: &[u32]) -> Result<(Mesh, Vec<u32>)> {
    let curves: Vec<usize> = selected
        .iter()
        .map(|&f| f as usize)
        .filter(|&f| f < mesh.faces.len() && mesh.is_polyline(f))
        .collect();
    if curves.is_empty() {
        return Err("Make Curve Fill: select closed curves".into());
    }
    if curves.iter().any(|&f| closed_curve(mesh, f).is_none()) {
        return Err("Make Curve Fill works on closed curves; close the open ones first".into());
    }
    let mut out = mesh.clone();
    let mut fills = Vec::new();
    for f in curves {
        let kind = closed_curve(mesh, f).unwrap_or_default();
        let ids = mesh.faces[f].vertices.clone();
        let corners: Vec<u32> = match kind {
            CurveKind::Line | CurveKind::Curve { .. } => ids,
            CurveKind::Bezier | CurveKind::BSpline => {
                let points: Vec<DVec3> = ids.iter().map(|&v| mesh.vertices[v as usize]).collect();
                let outline = curves::sample(&points, kind, true, FILL_STEPS);
                let first = out.vertices.len() as u32;
                out.vertices.extend(outline.iter().copied());
                (first..out.vertices.len() as u32).collect()
            }
        };
        if corners.len() < 3 {
            return Err("Make Curve Fill: a curve needs three or more points".into());
        }
        crate::vertex_commands::append_face(&mut out, corners, Some(f));
        let fill = out.faces.len() - 1;
        set_representation(&mut out, fill, PolygonRepresentation::Face);
        fills.push(fill as u32);
    }
    out.validate()?;
    Ok((out, fills))
}

/// Align Winding's options (CC-21; Axis choices beyond Automatic: CC-Q4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindingOptions {
    pub merge: bool,
    pub reverse: bool,
}
impl Default for WindingOptions {
    fn default() -> Self {
        Self {
            merge: true,
            reverse: false,
        }
    }
}

/// The area normal (Newell) of points in order round a loop.
fn loop_normal(points: &[DVec3]) -> DVec3 {
    let n = points.len();
    (0..n).fold(DVec3::ZERO, |acc, i| {
        let (a, b) = (points[i], points[(i + 1) % n]);
        acc + DVec3::new(
            (a.y - b.y) * (a.z + b.z),
            (a.z - b.z) * (a.x + b.x),
            (a.x - b.x) * (a.y + b.y),
        )
    })
}

/// The outline a closed curve draws, for its winding.
fn outline(mesh: &Mesh, f: usize, kind: CurveKind) -> Vec<DVec3> {
    let points: Vec<DVec3> = mesh.faces[f]
        .vertices
        .iter()
        .map(|&v| mesh.vertices[v as usize])
        .collect();
    curves::sample(&points, kind, true, 8)
}

/// The point list of a closed curve run the other way round, starting at
/// the same place.
fn reversed(ids: &[u32], kind: CurveKind, mesh: &Mesh) -> Vec<u32> {
    if kind == CurveKind::Bezier {
        // Reverse the anchors and swap each one's handles, by position in
        // the stored list.
        let index: Vec<DVec3> = ids.iter().map(|&v| mesh.vertices[v as usize]).collect();
        let anchors = bezier_anchors(&index, true);
        let slots: Vec<[usize; 3]> = {
            let n = anchors.len();
            (0..n)
                .map(|i| {
                    let a = 3 * i;
                    let out = 3 * i + 1;
                    let inn = if i == 0 { 3 * n - 1 } else { 3 * i - 1 };
                    [inn, a, out]
                })
                .collect()
        };
        let n = slots.len();
        let order: Vec<[usize; 3]> = (0..n)
            .map(|k| {
                let [inn, a, out] = slots[(n - k) % n];
                [out, a, inn]
            })
            .collect();
        // Rebuild the stored order from the reordered slots.
        let as_points: Vec<[DVec3; 3]> = order
            .iter()
            .map(|s| s.map(|i| DVec3::new(i as f64, 0., 0.)))
            .collect();
        return bezier_points(&as_points, true)
            .into_iter()
            .map(|p| ids[p.x as usize])
            .collect();
    }
    let mut out = vec![ids[0]];
    out.extend(ids[1..].iter().rev());
    out
}

/// CC-20/21: every selected closed curve runs the same way round:
/// counter-clockwise seen from the Axis (automatic: the axis the curves'
/// combined facing is nearest), clockwise with Reverse. Curves that already
/// match keep their points. Merge Curves first joins selected open curves
/// that share end points into one. Returns the mesh and the curves.
pub fn align_winding(mesh: &Mesh, selected: &[u32], o: WindingOptions) -> Result<(Mesh, Vec<u32>)> {
    let mut out = mesh.clone();
    let mut picked: Vec<usize> = selected
        .iter()
        .map(|&f| f as usize)
        .filter(|&f| f < mesh.faces.len() && mesh.is_polyline(f))
        .collect();
    if picked.is_empty() {
        return Err("Align Winding: select curves".into());
    }
    if o.merge {
        picked = merge_open(&mut out, &picked)?;
    }
    let closed: Vec<(usize, CurveKind)> = picked
        .iter()
        .filter_map(|&f| closed_curve(&out, f).map(|k| (f, k)))
        .collect();
    if closed.is_empty() {
        return Err("Align Winding works on closed curves".into());
    }
    let normals: Vec<DVec3> = closed
        .iter()
        .map(|&(f, k)| loop_normal(&outline(&out, f, k)))
        .collect();
    // Automatic axis: the world axis the curves face along most.
    let total: DVec3 = normals.iter().map(|n| n.abs()).sum();
    let axis = if total.x >= total.y && total.x >= total.z && total.x > 0. {
        DVec3::X
    } else if total.z > total.y {
        DVec3::Z
    } else {
        DVec3::Y
    };
    let want = if o.reverse { -axis } else { axis };
    for (&(f, kind), n) in closed.iter().zip(&normals) {
        if n.dot(want) < 0. {
            let ids = out.faces[f].vertices.clone();
            let flipped = reversed(&ids, kind, &out);
            out.faces[f].vertices = flipped;
        }
    }
    out.validate()?;
    let mut result: Vec<u32> = picked.iter().map(|&f| f as u32).collect();
    result.sort_unstable();
    Ok((out, result))
}

/// Join selected open curves (and polylines) that share end points, end
/// to start, turning a piece round where needed; a chain whose ends meet
/// closes. Returns the curves left, as indices in `mesh` after the joins.
fn merge_open(mesh: &mut Mesh, picked: &[usize]) -> Result<Vec<usize>> {
    let is_open = |m: &Mesh, f: usize| {
        matches!(
            representation(m, f),
            PolygonRepresentation::Polyline {
                closed: false,
                curve: CurveKind::Line | CurveKind::Curve { .. }
            }
        )
    };
    let mut pieces: Vec<usize> = picked
        .iter()
        .copied()
        .filter(|&f| is_open(mesh, f))
        .collect();
    if pieces.len() < 2 {
        return Ok(picked.to_vec());
    }
    let mut chains: Vec<(Vec<u32>, PolygonRepresentation, usize)> = Vec::new();
    while let Some(first) = pieces.first().copied() {
        pieces.remove(0);
        let mut chain = mesh.faces[first].vertices.clone();
        loop {
            let (head, tail) = (chain[0], *chain.last().unwrap());
            let next = pieces.iter().position(|&g| {
                let v = &mesh.faces[g].vertices;
                v[0] == tail
                    || *v.last().unwrap() == tail
                    || v[0] == head
                    || *v.last().unwrap() == head
            });
            let Some(k) = next else {
                break;
            };
            let g = pieces.remove(k);
            let mut v = mesh.faces[g].vertices.clone();
            if v[0] == tail {
                chain.extend(&v[1..]);
            } else if *v.last().unwrap() == tail {
                v.reverse();
                chain.extend(&v[1..]);
            } else if *v.last().unwrap() == head {
                v.pop();
                v.extend(&chain);
                chain = v;
            } else {
                v.reverse();
                v.pop();
                v.extend(&chain);
                chain = v;
            }
        }
        chains.push((chain, representation(mesh, first), first));
    }
    let joined: Vec<usize> = picked
        .iter()
        .copied()
        .filter(|&f| is_open(mesh, f))
        .collect();
    if chains.len() == joined.len() {
        return Ok(picked.to_vec());
    }
    // Replace the pieces with the chains.
    let mut keep: Vec<usize> = picked
        .iter()
        .copied()
        .filter(|f| !joined.contains(f))
        .collect();
    let mut new_faces = Vec::new();
    for (mut chain, rep, template) in chains {
        let closes = chain.len() > 2 && chain[0] == *chain.last().unwrap();
        if closes {
            chain.pop();
        }
        crate::vertex_commands::append_face(mesh, chain, Some(template));
        let f = mesh.faces.len() - 1;
        let curve = match rep {
            PolygonRepresentation::Polyline { curve, .. } => curve,
            _ => CurveKind::Line,
        };
        set_representation(
            mesh,
            f,
            PolygonRepresentation::Polyline {
                closed: closes,
                curve,
            },
        );
        new_faces.push(f);
    }
    let remove: std::collections::BTreeSet<usize> = joined.iter().copied().collect();
    let shift = |f: usize| f - remove.iter().filter(|&&r| r < f).count();
    mesh.remove_faces(&remove)?;
    keep = keep.into_iter().map(shift).collect();
    keep.extend(new_faces.into_iter().map(shift));
    Ok(keep)
}

impl crate::Editor {
    /// CC-01..03: Make Open / Closed Curve; afterwards Polygon mode with the
    /// new curve selected (CC-02). One undo step.
    pub(crate) fn make_curve_command(&mut self, closed: bool) -> Result<()> {
        let label = if closed {
            "Make Closed Curve"
        } else {
            "Make Open Curve"
        };
        let item = self.edge_insert_target(label)?;
        if self.scene.selection.mode != crate::Mode::Vertex {
            return Err(TOO_FEW_POINTS.into());
        }
        let points = self.scene.selection.vertices.clone();
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let (out, curve) = make_curve(mesh, &points, closed)?;
        let before = self.scene.clone();
        self.scene.selection.mode = crate::Mode::Polygon;
        self.scene.selection.vertices.clear();
        self.scene.selection.edges.clear();
        self.publish_polygon_edit_from(item, out, label, vec![curve], before)
    }

    /// CC-10: Make Curve Fill on the selected closed curves; the fills are
    /// selected after.
    pub(crate) fn make_curve_fill_command(&mut self) -> Result<()> {
        let item = self.edge_insert_target("Make Curve Fill")?;
        if self.scene.selection.mode != crate::Mode::Polygon {
            return Err("Make Curve Fill: select closed curves in Polygon mode".into());
        }
        let selected = self.scene.selection.faces.clone();
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let (out, fills) = make_curve_fill(mesh, &selected)?;
        self.publish_polygon_edit(item, out, "Make Curve Fill", fills)
    }

    /// CC-20: Align Winding on the selected curves (all curves with none
    /// selected).
    pub(crate) fn align_winding_command(&mut self, options: WindingOptions) -> Result<()> {
        let item = self.edge_insert_target("Align Winding")?;
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let mut selected = if self.scene.selection.mode == crate::Mode::Polygon {
            self.scene.selection.faces.clone()
        } else {
            Vec::new()
        };
        if selected.is_empty() {
            selected = (0..mesh.faces.len() as u32)
                .filter(|&f| mesh.is_polyline(f as usize))
                .collect();
        }
        let (out, curves) = align_winding(mesh, &selected, options)?;
        let faces = if self.scene.selection.mode == crate::Mode::Polygon {
            curves
        } else {
            self.scene.selection.faces.clone()
        };
        self.publish_polygon_edit(item, out, "Align Winding", faces)
    }
}
