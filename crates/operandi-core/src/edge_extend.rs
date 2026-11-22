//! ME-T03 Edge Extend (SPEC-EDGE-EXTEND-ROTATE-SCALE v1).
//!
//! Edge Extend copies the selected edges outward and joins them to the
//! originals with new polygons. The new edges are first offset (Offset
//! X/Y/Z and the Local Inset/Shift), then rotated and scaled about the
//! action centre, which moves with the Offset (provisional, EXT-RS-Q1; it
//! is what EXT-RS-02 needs). Segments divides the new strip along its
//! length; each row takes its share of every value, so a turned extension
//! sweeps round in steps.
//!
//! The basic Operandi profile extends boundary edges, and interior edges as
//! a fin off their first neighbouring polygon (provisional, EXW-Q02; their
//! Local offsets use that neighbour too, EXT-RS-Q3). Connected selected
//! edges share their new points, so a chain extends as one connected row.
//! The new quads are wound to match their neighbour and take its tags;
//! their corner maps are unassigned unless Make UVs fills one. The outer
//! edges of the new strip are returned so the tool can select them, ready
//! for another extension.
use crate::topology::edge_key;
use crate::{ActionFrame, Face, Mesh, Result};
use glam::{DMat3, DMat4, DQuat, DVec2, DVec3};
use std::collections::{BTreeMap, BTreeSet};

/// Directed uses `(face, from, to)` of each undirected edge.
type EdgeUses = BTreeMap<(u32, u32), Vec<(usize, u32, u32)>>;

/// Make UVs: which UV direction runs out along the new polygons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExtendUvs {
    Off,
    /// The default (spec v1.1).
    #[default]
    U,
    V,
}

/// Plane: what the two-axis circles do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExtendPlane {
    #[default]
    None,
    Move,
    Scale,
}

/// Haul: what a drag off the handles changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExtendHaul {
    /// Offset X/Y/Z, in the view plane.
    #[default]
    Global,
    /// Horizontal drag = Inset, vertical = Shift.
    Local,
}

/// Which handle sets are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtendHandles {
    pub moves: bool,
    pub rotate: bool,
    pub scale: bool,
    pub local: bool,
}
impl Default for ExtendHandles {
    /// Move on, the others off (spec v1.1).
    fn default() -> Self {
        Self {
            moves: true,
            rotate: false,
            scale: false,
            local: false,
        }
    }
}

/// Edge Extend's options beyond Offset (`Parameters::delta`), Rotate
/// (`rotation_delta`), Scale (`scale_delta`), Inset, Shift and Segments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EdgeExtendOptions {
    pub handles: ExtendHandles,
    pub plane: ExtendPlane,
    pub haul: ExtendHaul,
    pub make_uvs: ExtendUvs,
}

/// One extension, in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExtendShape {
    pub offset: DVec3,
    /// Sideways in each source polygon's plane, away from it.
    pub inset: f64,
    /// Along each source polygon's normal.
    pub shift: f64,
    /// Degrees about the action axes, applied X, then Y, then Z (as the
    /// Transform tool's Rotate).
    pub rotate: DVec3,
    /// Factors along the action axes (1 = 100 %).
    pub scale: DVec3,
    /// The action centre and axes (world space).
    pub frame: ActionFrame,
    pub segments: u32,
    pub make_uvs: ExtendUvs,
}
impl ExtendShape {
    /// A plain offset about the world origin, as the tool had before.
    pub fn offset(offset: DVec3, segments: u32) -> Self {
        Self {
            offset,
            inset: 0.,
            shift: 0.,
            rotate: DVec3::ZERO,
            scale: DVec3::ONE,
            frame: ActionFrame {
                center: DVec3::ZERO,
                basis: DMat3::IDENTITY,
            },
            segments,
            make_uvs: ExtendUvs::Off,
        }
    }
    /// Nothing moves: the new edges would sit on the old ones.
    fn is_identity(&self) -> bool {
        self.offset == DVec3::ZERO
            && self.inset == 0.
            && self.shift == 0.
            && self.rotate == DVec3::ZERO
            && self.scale == DVec3::ONE
    }
    /// Where a point at world `p` (with Local offset `local`) lands in row
    /// `t` (0 at the source, 1 at the outer edge).
    pub fn place(&self, p: DVec3, local: DVec3, t: f64) -> DVec3 {
        let angles = self.rotate * t;
        let turn = DMat3::from_quat(DQuat::from_euler(
            glam::EulerRot::XYZ,
            angles.x.to_radians(),
            angles.y.to_radians(),
            angles.z.to_radians(),
        ));
        let scale = DVec3::ONE + (self.scale - DVec3::ONE) * t;
        let b = self.frame.basis;
        let linear = b * turn * DMat3::from_diagonal(scale) * b.transpose();
        // Offset first, then rotate and scale about the moved centre:
        // c + o + L(p + l − c), the same as about the original centre and
        // then offset.
        self.frame.center + self.offset * t + linear * (p + local * t - self.frame.center)
    }
}

/// Extend the selected edges of `mesh` by `offset` (mesh coordinates) in
/// `segments` rows. Returns the mesh and the outer edges of the new strip.
pub fn edge_extend_geometry(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    offset: DVec3,
    segments: u32,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    edge_extend_shaped(
        mesh,
        edges,
        DMat4::IDENTITY,
        &ExtendShape::offset(offset, segments),
    )
}

/// Newell normal of polygon `fi` with its points mapped by `world`.
fn world_normal(mesh: &Mesh, fi: usize, world: &DMat4) -> DVec3 {
    let f = &mesh.faces[fi];
    let p =
        |i: usize| world.transform_point3(mesh.vertices[f.vertices[i % f.vertices.len()] as usize]);
    let mut n = DVec3::ZERO;
    for i in 0..f.vertices.len() {
        let (a, b) = (p(i), p(i + 1));
        n += DVec3::new(
            (a.y - b.y) * (a.z + b.z),
            (a.z - b.z) * (a.x + b.x),
            (a.x - b.x) * (a.y + b.y),
        );
    }
    n.normalize_or_zero()
}

/// The Local offset directions at each source point: `(inset, shift)` per
/// unit of Inset and Shift, in world space. Inset is mitred where two
/// edges meet so both new edges stay their Inset away from the old ones.
fn local_directions(mesh: &Mesh, uses: &EdgeUses, world: &DMat4) -> BTreeMap<u32, (DVec3, DVec3)> {
    let mut outs: BTreeMap<u32, (Vec<DVec3>, DVec3)> = BTreeMap::new();
    for list in uses.values() {
        let Some(&(fi, x, y)) = list.first() else {
            continue;
        };
        let n = world_normal(mesh, fi, world);
        let (px, py) = (
            world.transform_point3(mesh.vertices[x as usize]),
            world.transform_point3(mesh.vertices[y as usize]),
        );
        let centre = world.transform_point3(mesh.center(fi));
        let mut out = (py - px).cross(n).normalize_or_zero();
        if out.dot((px + py) * 0.5 - centre) < 0. {
            out = -out;
        }
        for v in [x, y] {
            let e = outs.entry(v).or_default();
            e.0.push(out);
            e.1 += n;
        }
    }
    outs.into_iter()
        .map(|(v, (dirs, normals))| {
            let inset = match dirs[..] {
                [a] => a,
                [a, b] => {
                    // Mitre: (a + b) / (1 + a·b) is 1 away from both lines;
                    // near-opposite edges fall back to their average.
                    let d = 1. + a.dot(b);
                    if d > 0.25 {
                        (a + b) / d
                    } else {
                        (a + b).normalize_or(a)
                    }
                }
                _ => {
                    let sum: DVec3 = dirs.iter().copied().sum();
                    sum.normalize_or_zero()
                }
            };
            (v, (inset, normals.normalize_or_zero()))
        })
        .collect()
}

/// Make UVs: each source point's place along its connected run of
/// selected edges (0..1 by length), per directed edge `(from, to)` so a
/// closed loop's last edge ends at 1, not back at 0 (provisional,
/// EXT-RS-Q4).
fn along_values(mesh: &Mesh, selected: &BTreeSet<(u32, u32)>) -> BTreeMap<(u32, u32), (f64, f64)> {
    let mut next: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &(a, b) in selected {
        next.entry(a).or_default().push(b);
        next.entry(b).or_default().push(a);
    }
    let length = |a: u32, b: u32| mesh.vertices[a as usize].distance(mesh.vertices[b as usize]);
    let mut done: BTreeSet<(u32, u32)> = BTreeSet::new();
    let mut out = BTreeMap::new();
    // Start runs at ends (one neighbour) first, then anywhere on loops.
    let mut starts: Vec<u32> = next
        .iter()
        .filter(|(_, n)| n.len() == 1)
        .map(|(&v, _)| v)
        .collect();
    starts.extend(next.keys().copied());
    for start in starts {
        let mut run: Vec<(u32, u32)> = Vec::new();
        let mut at = start;
        while let Some(&to) = next[&at]
            .iter()
            .find(|&&to| !done.contains(&edge_key(at, to)))
        {
            done.insert(edge_key(at, to));
            run.push((at, to));
            at = to;
        }
        let total: f64 = run.iter().map(|&(a, b)| length(a, b)).sum();
        let total = if total > 0. { total } else { 1. };
        let mut s = 0.;
        for (a, b) in run {
            let e = s + length(a, b);
            out.insert((a, b), (s / total, e / total));
            out.insert((b, a), (e / total, s / total));
            s = e;
        }
    }
    out
}

/// Extend the selected edges of `mesh`, whose world matrix is `world`, by
/// `shape`. Returns the mesh and the outer edges of the new strip.
pub fn edge_extend_shaped(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    world: DMat4,
    shape: &ExtendShape,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    extend(mesh, edges, world, shape, false)
}

/// N25 (Jason, 2026-11-17): the extension made by the click that starts
/// Edge Extend, before any drag. With an identity `shape` the new rows sit
/// on the source edges (zero-area polygons, allowed as degenerate) so the
/// next tool, such as Scale, works on the new edges at once.
pub fn edge_extend_at_zero(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    world: DMat4,
    shape: &ExtendShape,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    extend(mesh, edges, world, shape, true)
}

fn extend(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    world: DMat4,
    shape: &ExtendShape,
    at_zero: bool,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    mesh.validate()?;
    let segments = shape.segments;
    let finite = shape.offset.is_finite()
        && shape.inset.is_finite()
        && shape.shift.is_finite()
        && shape.rotate.is_finite()
        && shape.scale.is_finite();
    if !finite {
        return Err("Extend values must be finite".into());
    }
    if !(1..=256).contains(&segments) {
        return Err("Extend Segments must be 1–256".into());
    }
    let local = world.inverse();
    if !world.determinant().is_normal() || !local.is_finite() {
        return Err("Extend needs an invertible item transform".into());
    }
    let selected: BTreeSet<(u32, u32)> = edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
    if selected.is_empty() {
        return Err("Select boundary edges to Extend".into());
    }
    if shape.is_identity() && !at_zero {
        return Ok((mesh.clone(), selected.into_iter().collect()));
    }
    // Directed use of each selected edge by its neighbouring polygons.
    let mut uses: EdgeUses = BTreeMap::new();
    for (fi, f) in mesh.faces.iter().enumerate() {
        if mesh.is_polyline(fi) {
            continue;
        }
        let n = f.vertices.len();
        for i in 0..n {
            let (x, y) = (f.vertices[i], f.vertices[(i + 1) % n]);
            if selected.contains(&edge_key(x, y)) {
                uses.entry(edge_key(x, y)).or_default().push((fi, x, y));
            }
        }
    }
    if uses.len() != selected.len() {
        return Err("Extend needs edges of polygons".into());
    }
    let directions = if shape.inset != 0. || shape.shift != 0. {
        local_directions(mesh, &uses, &world)
    } else {
        BTreeMap::new()
    };
    let mut out = mesh.clone();
    // rows[v][k-1] = new point for source point v at segment k.
    let mut rows: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut point = |out: &mut Mesh, v: u32, k: u32| -> Result<u32> {
        let row = rows.entry(v).or_default();
        while row.len() < k as usize {
            let t = (row.len() + 1) as f64 / segments as f64;
            let (inset, normal) = directions.get(&v).copied().unwrap_or_default();
            let offset = inset * shape.inset + normal * shape.shift;
            let p = world.transform_point3(mesh.vertices[v as usize]);
            let p = local.transform_point3(shape.place(p, offset, t));
            row.push(u32::try_from(out.vertices.len()).map_err(|_| "Mesh point budget exceeded")?);
            out.vertices.push(p);
        }
        Ok(row[k as usize - 1])
    };
    let along = if shape.make_uvs == ExtendUvs::Off {
        BTreeMap::new()
    } else {
        along_values(mesh, &selected)
    };
    // Make UVs: per new quad its four corners (corner order as pushed).
    let mut uvs: Vec<Vec<Option<DVec2>>> = Vec::new();
    let mut outer = Vec::new();
    for list in uses.values() {
        // An interior edge (two or more neighbours) extends as a fin off the
        // first neighbour, leaving the edge shared by three polygons (Jason,
        // 2026-11-04; provisional, EXW-Q02).
        let Some(&(fi, x, y)) = list.first() else {
            return Err("Selected edge does not exist".into());
        };
        let template = mesh.faces[fi].clone();
        // The new strip runs y→x along the source edge (opposite the
        // neighbour), then outward.
        let (mut prev_x, mut prev_y) = (x, y);
        let (ax, ay) = along.get(&(x, y)).copied().unwrap_or((0., 1.));
        for k in 1..=segments {
            let (nx, ny) = (point(&mut out, x, k)?, point(&mut out, y, k)?);
            out.faces.push(Face {
                vertices: vec![prev_y, prev_x, nx, ny],
                uv: None,
                ..template.clone()
            });
            if shape.make_uvs != ExtendUvs::Off {
                let (t0, t1) = ((k - 1) as f64 / segments as f64, k as f64 / segments as f64);
                let uv = |s: f64, t: f64| {
                    Some(if shape.make_uvs == ExtendUvs::U {
                        DVec2::new(t, s)
                    } else {
                        DVec2::new(s, t)
                    })
                };
                uvs.push(vec![uv(ay, t0), uv(ax, t0), uv(ax, t1), uv(ay, t1)]);
            }
            if !mesh.attributes.smoothing_labels.is_empty() {
                let label = mesh.attributes.smoothing_labels[fi].clone();
                out.attributes.smoothing_labels.push(label);
            }
            if !mesh.attributes.representations.is_empty() {
                let r = mesh.attributes.representations[fi];
                out.attributes.representations.push(r);
            }
            (prev_x, prev_y) = (nx, ny);
        }
        outer.push(edge_key(prev_x, prev_y));
    }
    // Unassigned corner records for the new quads.
    let added = out.faces.len() - mesh.faces.len();
    for map in out.uv_maps.values_mut() {
        map.extend(std::iter::repeat_n(vec![None; 4], added));
    }
    let at = &mut out.attributes;
    for map in at.uv_records.values_mut() {
        map.extend(std::iter::repeat_n(vec![None; 4], added));
    }
    for map in at.corner_weights.values_mut() {
        map.extend(std::iter::repeat_n(vec![None; 4], added));
    }
    for map in at.corner_colors.values_mut() {
        map.extend(std::iter::repeat_n(vec![None; 4], added));
    }
    for map in at.corner_normals.values_mut() {
        map.extend(std::iter::repeat_n(vec![None; 4], added));
    }
    for map in at.typed_maps.values_mut() {
        if !map.corners.is_empty() {
            map.corners
                .extend(std::iter::repeat_n(vec![None; 4], added));
        }
    }
    if shape.make_uvs != ExtendUvs::Off {
        write_uvs(&mut out, mesh.faces.len(), uvs);
    }
    // Fins off interior edges make edges with three polygons; record them as
    // the mesh's known exceptions.
    out.attributes.extrude_edge_uses = crate::interchange::edge_exceptions(&out);
    if at_zero {
        // Rows with no length yet: flat polygons the next edit opens up.
        for f in mesh.faces.len()..out.faces.len() {
            if out.normal(f) == DVec3::ZERO {
                out.attributes.degenerate_faces.insert(f as u32);
            }
        }
    }
    out.validate()
        .map_err(|e| format!("Extend produced an invalid mesh ({e}); nothing changed"))?;
    Ok((out, outer))
}

/// The UV map Make UVs fills: "Texture" when the mesh has it or has no
/// map, else its first map (provisional, EXT-RS-Q4).
pub const EXTEND_UV_MAP: &str = "Texture";

fn write_uvs(out: &mut Mesh, first_new: usize, uvs: Vec<Vec<Option<DVec2>>>) {
    let name = if out.uv_maps.contains_key(EXTEND_UV_MAP) || out.uv_maps.is_empty() {
        EXTEND_UV_MAP.to_string()
    } else {
        out.uv_maps.keys().next().cloned().unwrap_or_default()
    };
    if !out.uv_maps.contains_key(&name) {
        let empty: Vec<Vec<Option<DVec2>>> = out
            .faces
            .iter()
            .map(|f| vec![None; f.vertices.len()])
            .collect();
        out.uv_maps.insert(name.clone(), empty);
    }
    let map = out.uv_maps.get_mut(&name).expect("map just made");
    for (i, corners) in uvs.into_iter().enumerate() {
        map[first_new + i] = corners;
    }
}
