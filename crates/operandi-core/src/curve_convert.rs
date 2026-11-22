//! The hands-on curve commands (CURVE-COMMANDS-2D-SHAPES, hands-on
//! 2026-11-17): curve type conversions (CC-50..54), curve offsets
//! (CC-40..43) and Bezier Tangent (CC-60..64).
use crate::curves::{self, CurveKind, bezier_anchors, bezier_points};
use crate::{Mesh, PolygonRepresentation, Result};
use glam::DVec3;

/// A curve element's kind and closure, or `None` for a polygon.
pub fn curve_of(mesh: &Mesh, f: usize) -> Option<(CurveKind, bool)> {
    match mesh.attributes.representations.get(f).copied() {
        Some(PolygonRepresentation::Polyline { closed, curve }) => Some((curve, closed)),
        _ if f < mesh.faces.len() && mesh.is_polyline(f) => Some((CurveKind::Line, false)),
        _ => None,
    }
}

fn points_of(mesh: &Mesh, f: usize) -> Vec<DVec3> {
    mesh.faces[f]
        .vertices
        .iter()
        .map(|&v| mesh.vertices[v as usize])
        .collect()
}

// ---- conversions (CC-50..54) ----

/// The three smooth curve kinds the conversions work between.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveType {
    Curve,
    Bezier,
    BSpline,
}
impl CurveType {
    pub fn of(kind: CurveKind) -> Option<Self> {
        match kind {
            CurveKind::Curve { .. } => Some(Self::Curve),
            CurveKind::Bezier => Some(Self::Bezier),
            CurveKind::BSpline => Some(Self::BSpline),
            CurveKind::Line => None,
        }
    }
    pub fn kind(self) -> CurveKind {
        match self {
            Self::Curve => CurveKind::Curve {
                start_control: false,
                end_control: false,
            },
            Self::Bezier => CurveKind::Bezier,
            Self::BSpline => CurveKind::BSpline,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Curve => "Curve",
            Self::Bezier => "Bezier",
            Self::BSpline => "B-Spline",
        }
    }
    /// The new item's name (CC-51).
    pub fn item_name(self) -> &'static str {
        match self {
            Self::Curve => "Converted_Curve",
            Self::Bezier => "Converted_Bezier",
            Self::BSpline => "Converted_Bspline",
        }
    }
    /// The word in `curve.convert from:<word>`.
    pub fn named(word: &str) -> Result<Self> {
        match word.to_ascii_lowercase().replace(['-', '_'], "").as_str() {
            "curve" => Ok(Self::Curve),
            "bezier" => Ok(Self::Bezier),
            "bspline" => Ok(Self::BSpline),
            other => Err(format!("No curve type called {other}")),
        }
    }
}

/// Each span of the drawn curve as one cubic (start, handle, handle,
/// end), exactly: every span of a Curve, B-Spline or Bezier is a cubic,
/// so four samples pin it down.
pub fn span_cubics(points: &[DVec3], kind: CurveKind, closed: bool) -> Vec<[DVec3; 4]> {
    let spans = curves::span_count(points, kind, closed);
    (0..spans)
        .map(|i| {
            let at =
                |t: f64| curves::point_at(points, kind, closed, i as f64 + t).unwrap_or_default();
            let (s0, s1, s2, s3) = (at(0.), at(1. / 3.), at(2. / 3.), at(1.));
            [
                s0,
                (s0 * -5. + s1 * 18. - s2 * 9. + s3 * 2.) / 6.,
                (s0 * 2. - s1 * 9. + s2 * 18. - s3 * 5.) / 6.,
                s3,
            ]
        })
        .collect()
}

/// The converted point list (CC-52, CC-53; the other four by the same
/// pattern, build guess).
pub fn convert_points(
    points: &[DVec3],
    from: CurveKind,
    closed: bool,
    to: CurveType,
) -> Result<Vec<DVec3>> {
    let cubics = span_cubics(points, from, closed);
    if cubics.is_empty() {
        return Err("The curve needs more points to convert".into());
    }
    // As a Bezier: knots at the span ends with the spans' handles. For a
    // Curve this is exact, handles ± (next − previous) / 6 (CC-52).
    let bezier = || -> Vec<DVec3> {
        let n = cubics.len();
        let knots = if closed { n } else { n + 1 };
        let anchors: Vec<[DVec3; 3]> = (0..knots)
            .map(|k| {
                let out = cubics.get(k).map(|c| c[1]);
                let inn = if k > 0 {
                    Some(cubics[k - 1][2])
                } else if closed {
                    Some(cubics[n - 1][2])
                } else {
                    None
                };
                let a = cubics.get(k).map_or(cubics[n - 1][3], |c| c[0]);
                [inn.unwrap_or(a), a, out.unwrap_or(a)]
            })
            .collect();
        bezier_points(&anchors, closed)
    };
    match to {
        CurveType::Bezier => Ok(bezier()),
        // Through the knots only (build guess).
        CurveType::Curve => {
            let mut knots: Vec<DVec3> = cubics.iter().map(|c| c[0]).collect();
            if !closed {
                knots.push(cubics[cubics.len() - 1][3]);
            }
            Ok(knots)
        }
        // CC-53: the Bezier's handles plus its first knot (and, open, its
        // last) become the control points; the other knots are dropped.
        CurveType::BSpline => {
            let b = if CurveType::of(from) == Some(CurveType::Bezier) {
                points.to_vec()
            } else {
                bezier()
            };
            let anchors = bezier_anchors(&b, closed);
            let n = anchors.len();
            // First knot, then every handle in order; open curves end on
            // their last knot, closed ones on the first knot's in handle.
            let mut out = vec![anchors[0][1], anchors[0][2]];
            for a in &anchors[1..] {
                out.push(a[0]);
                if closed {
                    out.push(a[2]);
                }
            }
            if closed {
                out.push(anchors[0][0]);
            } else {
                // The open run so far: a0, out0, in1, in2... rebuild it.
                out = vec![anchors[0][1], anchors[0][2]];
                for (k, a) in anchors.iter().enumerate().skip(1) {
                    out.push(a[0]);
                    if k + 1 < n {
                        out.push(a[2]);
                    }
                }
                out.push(anchors[n - 1][1]);
            }
            Ok(out)
        }
    }
}

/// CC-50..53: the selected curves of type `from` converted into a new
/// mesh of curves of type `to` (the item is made by the editor).
pub fn convert_mesh(mesh: &Mesh, selected: &[u32], from: CurveType, to: CurveType) -> Result<Mesh> {
    let mut out = Mesh::default();
    let mut any = false;
    for &f in selected {
        let f = f as usize;
        let Some((kind, closed)) = curve_of(mesh, f) else {
            continue;
        };
        if CurveType::of(kind) != Some(from) {
            continue;
        }
        let pts = convert_points(&points_of(mesh, f), kind, closed, to)?;
        let first = out.vertices.len() as u32;
        out.vertices.extend(pts.iter().copied());
        let ids: Vec<u32> = (first..out.vertices.len() as u32).collect();
        crate::vertex_commands::append_face(&mut out, ids, None);
        out.faces.last_mut().unwrap().material = mesh.faces[f].material.clone();
        out.attributes
            .representations
            .push(PolygonRepresentation::Polyline {
                closed,
                curve: to.kind(),
            });
        any = true;
    }
    if !any {
        return Err(format!("Select a {} curve to convert", from.label()));
    }
    out.validate()?;
    Ok(out)
}

/// CC-54: the reason a conversion from `from` can't run now, if any.
pub fn convert_unavailable(mesh: &Mesh, selected: &[u32], from: CurveType) -> Option<String> {
    let has = selected
        .iter()
        .any(|&f| curve_of(mesh, f as usize).is_some_and(|(k, _)| CurveType::of(k) == Some(from)));
    (!has).then(|| format!("Select a {} curve (Polygon mode)", from.label()))
}

// ---- offsets (CC-40..43) ----

/// The curve's plane normal (Newell), or Y when its points are on a line.
fn plane_normal(points: &[DVec3]) -> DVec3 {
    let n = points.len();
    let mut v = DVec3::ZERO;
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        v += DVec3::new(
            (a.y - b.y) * (a.z + b.z),
            (a.z - b.z) * (a.x + b.x),
            (a.x - b.x) * (a.y + b.y),
        );
    }
    v.try_normalize().unwrap_or(DVec3::Y)
}

/// CC-41/43: each point moved `d` along its corner's mitre (positive:
/// toward the inside of a closed curve, whichever way it winds; for an
/// open curve, to the left of travel seen from the plane's positive axis).
/// An open curve's ends sit beside the original ends, square to the end
/// legs (CC-43's build rule; the observed 0.1 × (2 + √5) slide along the
/// end legs isn't copied, CC-Q2).
pub fn offset_points(points: &[DVec3], closed: bool, d: f64) -> Vec<DVec3> {
    let n = points.len();
    // Closed: the loop's own facing, so positive is inward whichever way
    // it winds (CC-41). Open (CC-43): the plane's axis pointing up, so
    // positive is to the left of travel seen from that side (+Z for a
    // first leg along +X in the XZ plane, seen from +Y).
    let normal = if closed {
        plane_normal(points)
    } else {
        let a = plane_normal(points).abs();
        let up = if a.x >= a.y && a.x >= a.z {
            DVec3::X
        } else if a.y >= a.z {
            DVec3::Y
        } else {
            DVec3::Z
        };
        -up
    };
    // The segment from point i to i + 1's side normal.
    let side = |i: usize| -> Option<DVec3> {
        let e = points[(i + 1) % n] - points[i];
        normal.cross(e).try_normalize()
    };
    (0..n)
        .map(|i| {
            let before = if i > 0 || closed {
                side((i + n - 1) % n)
            } else {
                None
            };
            let after = if i + 1 < n || closed { side(i) } else { None };
            let m = match (before, after) {
                (Some(a), Some(b)) => {
                    let m = (a + b).try_normalize().unwrap_or(a);
                    // Mitre: each segment keeps the distance d.
                    m / m.dot(a).max(1e-6)
                }
                (Some(a), None) | (None, Some(a)) => a,
                (None, None) => DVec3::ZERO,
            };
            points[i] + m * d
        })
        .collect()
}

/// CC-40..43: an offset copy of each selected curve that is `closed` (or
/// open), added to the same mesh; Trio adds both sides. Returns the mesh
/// and the new curves.
pub fn offset_curves(
    mesh: &Mesh,
    selected: &[u32],
    closed: bool,
    d: f64,
    trio: bool,
) -> Result<(Mesh, Vec<u32>)> {
    if !d.is_finite() {
        return Err("Offset must be a number".into());
    }
    let mut out = mesh.clone();
    let mut new = Vec::new();
    for &f in selected {
        let f = f as usize;
        let Some((kind, is_closed)) = curve_of(mesh, f) else {
            continue;
        };
        if is_closed != closed || mesh.faces[f].vertices.len() < 2 {
            continue;
        }
        let pts = points_of(mesh, f);
        let sides: &[f64] = if trio { &[1., -1.] } else { &[1.] };
        for &s in sides {
            let moved = offset_points(&pts, closed, d * s);
            let first = out.vertices.len() as u32;
            out.vertices.extend(moved);
            let ids: Vec<u32> = (first..out.vertices.len() as u32).collect();
            crate::vertex_commands::append_face(&mut out, ids, Some(f));
            let g = out.faces.len() - 1;
            let reps = &mut out.attributes.representations;
            reps.resize(out.faces.len() - 1, PolygonRepresentation::Face);
            reps.push(PolygonRepresentation::Polyline {
                closed,
                curve: kind,
            });
            new.push(g as u32);
        }
    }
    if new.is_empty() {
        return Err(format!(
            "Select {} curves to offset",
            if closed { "closed" } else { "open" }
        ));
    }
    out.validate()?;
    Ok((out, new))
}

// ---- Bezier Tangent (CC-60..64) ----

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TangentOperation {
    #[default]
    Sharp,
    Smooth,
    Uniform,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TangentSide {
    #[default]
    Both,
    Out,
    In,
}

/// Bezier Tangent's dialog values (CC-60 starting values).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TangentOptions {
    pub operation: TangentOperation,
    pub tangent: TangentSide,
    pub set_length: bool,
    pub length: f64,
}
impl Default for TangentOptions {
    fn default() -> Self {
        Self {
            operation: TangentOperation::Sharp,
            tangent: TangentSide::Both,
            set_length: false,
            length: 1.,
        }
    }
}

/// New (in, out) handle positions for a knot at `a` with neighbours
/// `prev` / `next` (CC-61..64).
pub fn retangent(
    a: DVec3,
    inn: DVec3,
    out: DVec3,
    prev: DVec3,
    next: DVec3,
    o: &TangentOptions,
) -> (DVec3, DVec3) {
    let (li, lo) = ((inn - a).length(), (out - a).length());
    let do_in = o.tangent != TangentSide::Out;
    let do_out = o.tangent != TangentSide::In;
    let (mut di, mut dout) = (
        (inn - a)
            .try_normalize()
            .unwrap_or((prev - a).normalize_or_zero()),
        (out - a)
            .try_normalize()
            .unwrap_or((next - a).normalize_or_zero()),
    );
    let (mut ni, mut no) = (li, lo);
    match o.operation {
        // CC-61: each handle aims along its own segment, so the knot is a
        // clean corner (CC-Q3: the observed In direction is not copied).
        TangentOperation::Sharp => {
            if do_out {
                dout = (next - a).try_normalize().unwrap_or(dout);
            }
            if do_in {
                di = (prev - a).try_normalize().unwrap_or(di);
            }
        }
        // CC-62/63: both on one line through the knot.
        TangentOperation::Smooth | TangentOperation::Uniform => {
            let line = match o.tangent {
                TangentSide::Both => (dout - di).try_normalize().unwrap_or(dout),
                TangentSide::Out => -di,
                TangentSide::In => dout,
            };
            if do_out {
                dout = line;
            }
            if do_in {
                di = -line;
            }
            if o.operation == TangentOperation::Uniform {
                let avg = (li + lo) / 2.;
                if do_out {
                    no = avg;
                }
                if do_in {
                    ni = avg;
                }
            }
        }
    }
    if o.set_length {
        if do_out {
            no = o.length;
        }
        if do_in {
            ni = o.length;
        }
    }
    (a + di * ni, a + dout * no)
}

/// CC-60..64 on every Bezier knot among `knots` (point ids). Returns the
/// mesh and how many knots changed.
pub fn bezier_tangent(mesh: &Mesh, knots: &[u32], o: &TangentOptions) -> Result<(Mesh, usize)> {
    if o.set_length && !(o.length.is_finite() && o.length >= 0.) {
        return Err("Length must be 0 or more".into());
    }
    let mut out = mesh.clone();
    let mut changed = 0;
    for f in 0..mesh.faces.len() {
        let Some((CurveKind::Bezier, closed)) = curve_of(mesh, f) else {
            continue;
        };
        let ids = &mesh.faces[f].vertices;
        let pts = points_of(mesh, f);
        let anchors = bezier_anchors(&pts, closed);
        let n = anchors.len();
        if n < 2 {
            continue;
        }
        // Stored index of each anchor's in / knot / out.
        let slot = |k: usize| -> (Option<usize>, usize, Option<usize>) {
            let a = 3 * k;
            let inn = if k > 0 {
                Some(a - 1)
            } else if closed {
                Some(ids.len() - 1)
            } else {
                None
            };
            let outp = if k + 1 < n || closed {
                Some(a + 1)
            } else {
                None
            };
            (inn, a, outp)
        };
        for k in 0..n {
            let (si, sa, so) = slot(k);
            if !knots.contains(&ids[sa]) {
                continue;
            }
            let a = anchors[k][1];
            let prev = if k > 0 || closed {
                anchors[(k + n - 1) % n][1]
            } else {
                a
            };
            let next = if k + 1 < n || closed {
                anchors[(k + 1) % n][1]
            } else {
                a
            };
            let (ni, no) = retangent(a, anchors[k][0], anchors[k][2], prev, next, o);
            if let Some(si) = si {
                out.vertices[ids[si] as usize] = ni;
            }
            if let Some(so) = so {
                out.vertices[ids[so] as usize] = no;
            }
            changed += 1;
        }
    }
    if changed == 0 {
        return Err("Bezier Tangent: select knots of a Bezier curve (Vertex mode)".into());
    }
    out.validate()?;
    Ok((out, changed))
}

impl crate::Editor {
    /// CC-50..53: the selected curves of `from` become a new mesh item of
    /// `to` curves, named Converted_<type>, placed like the source; the
    /// source stays. The new curves are selected (Polygon mode).
    pub(crate) fn convert_curves_command(&mut self, from: CurveType, to: CurveType) -> Result<()> {
        let source = self.edge_insert_target("Convert Curves")?;
        let faces = if self.scene.selection.mode == crate::Mode::Polygon {
            self.scene.selection.faces.clone()
        } else {
            Vec::new()
        };
        let item = self.scene.item(source).ok_or("Missing item")?;
        let converted = convert_mesh(&item.mesh, &faces, from, to)?;
        let (transform, parent) = (item.transform.clone(), item.parent);
        let before = self.scene.clone();
        let next = self.scene.next_id.max(self.next_item_id);
        self.scene.next_id = next;
        self.scene.new_mesh()?;
        self.next_item_id = self.scene.next_id;
        let name = self.scene.free_name(to.item_name());
        let id = {
            let new = self.scene.items.last_mut().ok_or("Missing item")?;
            new.name = name;
            new.transform = transform;
            new.parent = parent;
            let count = converted.faces.len() as u32;
            new.mesh = converted.into();
            (new.id, count)
        };
        let sel = &mut self.scene.selection;
        sel.item = Some(id.0);
        sel.mode = crate::Mode::Polygon;
        sel.vertices.clear();
        sel.edges.clear();
        sel.faces = (0..id.1).collect();
        if let Err(e) = self.scene.validate() {
            self.scene = before;
            return Err(e);
        }
        self.record(format!("{} to {}", from.label(), to.label()), before);
        Ok(())
    }

    /// CC-40..43 on the selected curves; the new curves are selected.
    pub(crate) fn offset_curves_command(
        &mut self,
        closed: bool,
        offset: f64,
        trio: bool,
    ) -> Result<()> {
        let label = match (closed, trio) {
            (true, false) => "Closed Curve Offset",
            (true, true) => "Closed Curve Offset Trio",
            (false, false) => "Open Curve Offset",
            (false, true) => "Open Curve Offset Trio",
        };
        let item = self.edge_insert_target(label)?;
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let selected = if self.scene.selection.mode == crate::Mode::Polygon
            && !self.scene.selection.faces.is_empty()
        {
            self.scene.selection.faces.clone()
        } else {
            (0..mesh.faces.len() as u32).collect()
        };
        let (out, new) = offset_curves(mesh, &selected, closed, offset, trio)?;
        let faces = if self.scene.selection.mode == crate::Mode::Polygon {
            new
        } else {
            self.scene.selection.faces.clone()
        };
        self.publish_polygon_edit(item, out, label, faces)
    }

    /// CC-60..64 on the selected Bezier knots (Vertex mode).
    pub(crate) fn bezier_tangent_command(&mut self, o: TangentOptions) -> Result<()> {
        let item = self.edge_insert_target("Bezier Tangent")?;
        if self.scene.selection.mode != crate::Mode::Vertex {
            return Err("Bezier Tangent: select knots of a Bezier curve (Vertex mode)".into());
        }
        let knots = self.scene.selection.vertices.clone();
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let (out, _) = bezier_tangent(mesh, &knots, &o)?;
        let faces = self.scene.selection.faces.clone();
        self.publish_polygon_edit(item, out, "Bezier Tangent", faces)
    }
}
