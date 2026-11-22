//! Edit tab, Subdivide group and Center (`docs/specs/edit-wave1/SUBDIVIDE.md`).
//!
//! * **Subdivide Polygons** (`poly.subdivide`): one command with four
//!   methods. Catmull-Clark (the "SDS Subdivide" button, SUB-04/SUB-10) is one
//!   standard Catmull-Clark step; SDS Subdivide (dialog only, SUB-11) creases
//!   edges sharper than Max Smoothing Angle; Faceted (SUB-20) splits without
//!   moving anything; Smooth (SUB-21) splits and pushes the new points out.
//!   A partial selection is subdivided as a patch whose outline is an open
//!   border (SUB-14): outline edge points are inserted into the unselected
//!   neighbours too.
//! * **UnSubdivide** (`poly.unsubdiv`, SUB-40..44): merges groups of four
//!   quads around a centre point back into one quad and (Flat off) undoes the
//!   Catmull-Clark move of the corners.
//! * **Center** (`vert.center`, CEN-01..04): moves the selected elements so
//!   their world bounding-box centre sits at 0 on the chosen axes.
//!
//! The geometry functions take a mesh and return a new one; the `Editor`
//! methods at the end publish them as one undo step each.
use crate::edge_commands::{CornerBlend, OutFace, finish, rebuild};
use crate::fxhash::FxHashMap;
use crate::topology::edge_key;
use crate::{Mesh, Mode, Result};
use glam::{DMat4, DVec2, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// SUB-30 Subdivision Method. The toolbox button labelled "SDS Subdivide"
/// runs `CatmullClark`; the dialog's `SdsSubdivide` is a different method
/// (SUB-04, the naming trap is the reference's own).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubdivideMethod {
    Faceted,
    Smooth,
    SdsSubdivide,
    #[default]
    CatmullClark,
}

impl SubdivideMethod {
    /// Dialog order (SUB-30).
    pub const ALL: [Self; 4] = [
        Self::Faceted,
        Self::Smooth,
        Self::SdsSubdivide,
        Self::CatmullClark,
    ];
    /// Dialog label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Faceted => "Faceted",
            Self::Smooth => "Smooth",
            Self::SdsSubdivide => "SDS Subdivide",
            Self::CatmullClark => "Catmull-Clark",
        }
    }
    /// `poly.subdivide <type>` word. `flat`, `smooth` and `ccsds` come from
    /// the reference name map's buttons; the map has no word for the
    /// dialog-only SDS Subdivide method, so `sds` is Operandi's (SUBT-Q01).
    pub fn script_name(self) -> &'static str {
        match self {
            Self::Faceted => "flat",
            Self::Smooth => "smooth",
            Self::SdsSubdivide => "sds",
            Self::CatmullClark => "ccsds",
        }
    }
    /// A `type:` value: the script word, the dialog label, or the dialog
    /// index (0 Faceted .. 3 Catmull-Clark; the map's integer values aren't
    /// published, SUBT-Q01).
    pub fn parse(text: &str) -> Option<Self> {
        let t = text.trim().trim_matches('"');
        Self::ALL.into_iter().find(|m| {
            t.eq_ignore_ascii_case(m.script_name())
                || t.eq_ignore_ascii_case(m.label())
                || t.parse::<usize>().ok() == Self::ALL.iter().position(|x| x == m)
        })
    }
}

/// SUB-12 Boundary Rules for the Catmull-Clark method.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubdivideBoundary {
    #[default]
    SmoothAll,
    CreaseAll,
    /// Measured identical to Smooth All so far (SUB-Q3).
    CreaseEdges,
}

impl SubdivideBoundary {
    pub const ALL: [Self; 3] = [Self::SmoothAll, Self::CreaseAll, Self::CreaseEdges];
    pub fn label(self) -> &'static str {
        match self {
            Self::SmoothAll => "Smooth All",
            Self::CreaseAll => "Crease All",
            Self::CreaseEdges => "Crease Edges",
        }
    }
    /// `bound:` word (Operandi's; the map only says "integer", SUBT-Q01).
    pub fn script_name(self) -> &'static str {
        match self {
            Self::SmoothAll => "smooth",
            Self::CreaseAll => "crease",
            Self::CreaseEdges => "creaseEdges",
        }
    }
    pub fn parse(text: &str) -> Option<Self> {
        let t = text.trim().trim_matches('"');
        Self::ALL.into_iter().find(|b| {
            t.eq_ignore_ascii_case(b.script_name())
                || t.eq_ignore_ascii_case(&b.label().replace(' ', ""))
                || t.eq_ignore_ascii_case(b.label())
                || t.parse::<usize>().ok() == Self::ALL.iter().position(|x| x == b)
        })
    }
}

/// SUB-30 options. Fractal is kept in percent and the angle in degrees, the
/// units the dialog shows; scripts pass a fraction and radians, converted
/// where arguments are parsed (`command_names::script_percent/script_angle`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SubdivideOptions {
    pub method: SubdivideMethod,
    pub fractal_percent: f64,
    pub max_angle_degrees: f64,
    pub linear_uvs: bool,
    pub boundary: SubdivideBoundary,
    /// Greyed until Operandi has background meshes; stored so it round-trips.
    pub background_constraint: bool,
    pub ignore_edge_weights: bool,
}

impl Default for SubdivideOptions {
    /// SUB-30 starting values (provisional, SUB-Q7).
    fn default() -> Self {
        Self {
            method: SubdivideMethod::CatmullClark,
            fractal_percent: 0.,
            max_angle_degrees: 89.5,
            linear_uvs: false,
            boundary: SubdivideBoundary::SmoothAll,
            background_constraint: false,
            ignore_edge_weights: false,
        }
    }
}

impl SubdivideOptions {
    pub fn validate(&self) -> Result<()> {
        if !(self.fractal_percent.is_finite() && (0. ..=100.).contains(&self.fractal_percent)) {
            return Err("Fractal must be between 0% and 100%".into());
        }
        if !(self.max_angle_degrees.is_finite() && (0. ..=180.).contains(&self.max_angle_degrees)) {
            return Err("Max Smoothing Angle must be between 0° and 180°".into());
        }
        Ok(())
    }
}

/// SUB-40 UnSubdivide options.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UnsubdivideOptions {
    pub iterations: u32,
    pub flat: bool,
    pub uv_hint: bool,
    pub material_hint: bool,
    pub part_hint: bool,
    pub smoothing_group_hint: bool,
    pub selection_set_hint: bool,
    pub selection_set_name: String,
}

impl Default for UnsubdivideOptions {
    /// SUB-40 starting values (Iterations 1 is provisional).
    fn default() -> Self {
        Self {
            iterations: 1,
            flat: false,
            uv_hint: true,
            material_hint: true,
            part_hint: false,
            smoothing_group_hint: false,
            selection_set_hint: false,
            selection_set_name: String::new(),
        }
    }
}

/// Most iterations one UnSubdivide accepts (an Operandi bound: each one at
/// least quarters the polygon count, so more never does anything).
pub const MAX_UNSUBDIVIDE_ITERATIONS: u32 = 16;

pub const NO_POLYGONS_SELECTED: &str = "No polygons selected";
pub const ONLY_QUADS: &str = "UnSubdivide only rebuilds quads";

/// A subdivision result and the polygons it made (SUB-15 selects them).
#[derive(Clone, Debug, PartialEq)]
pub struct SubdivideOutcome {
    pub mesh: Mesh,
    pub new_faces: Vec<u32>,
}

/// Small deterministic generator for Fractal (SUB-31); the editor seeds it
/// from the clock so repeated runs differ, tests seed it themselves.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        // xorshift64*
        let mut x = self.0.max(1);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
    /// A point in the unit ball.
    fn ball(&mut self) -> DVec3 {
        loop {
            let v = DVec3::new(self.next(), self.next(), self.next()) * 2. - DVec3::ONE;
            if v.length_squared() <= 1. {
                return v;
            }
        }
    }
}

/// The polygons a Subdivide or UnSubdivide works on (SUB-08): the selected
/// ones, or every polygon when none are; lines never take part.
fn working_faces(mesh: &Mesh, selected: &[u32]) -> Vec<usize> {
    let mut faces: Vec<usize> = if selected.is_empty() {
        (0..mesh.faces.len()).collect()
    } else {
        let mut s: Vec<usize> = selected
            .iter()
            .map(|&f| f as usize)
            .filter(|&f| f < mesh.faces.len())
            .collect();
        s.sort_unstable();
        s.dedup();
        s
    };
    faces.retain(|&f| !mesh.is_polyline(f));
    faces
}

/// Hermite midpoint of the curve from `a` to `b` whose end tangents lie in
/// the planes of normals `na` and `nb`, each as long as the chord. Flat
/// (equal normals in the chord's plane) gives the plain midpoint. With cube
/// corner normals this gives SUB-21's reference values exactly: sqrt(6)/24
/// out along each face normal for an edge, sqrt(3)/6 for a face centre.
fn hermite_mid(a: DVec3, na: DVec3, b: DVec3, nb: DVec3) -> DVec3 {
    let d = b - a;
    let len = d.length();
    let mid = (a + b) / 2.;
    if len < 1e-12 {
        return mid;
    }
    let tangent = |n: DVec3| {
        let t = d - n * d.dot(n);
        if t.length() < 1e-12 {
            d
        } else {
            t.normalize() * len
        }
    };
    mid + (tangent(na) - tangent(nb)) / 8.
}

/// The patch being subdivided: its polygons, their edges and, for each edge,
/// the patch polygons and every mesh polygon using it.
struct Patch {
    faces: Vec<usize>,
    in_patch: Vec<bool>,
    edge_id: FxHashMap<(u32, u32), usize>,
    edges: Vec<(u32, u32)>,
    /// Patch polygons on each edge (more than two only on bad meshes).
    edge_faces: Vec<Vec<usize>>,
    /// Every surface polygon on each edge, in the whole mesh.
    mesh_faces: Vec<Vec<usize>>,
    /// Per patch polygon (by its position in `faces`), the edge of each side
    /// i (corner i to corner i+1).
    sides: Vec<Vec<usize>>,
}

impl Patch {
    fn new(mesh: &Mesh, faces: Vec<usize>) -> Self {
        let mut in_patch = vec![false; mesh.faces.len()];
        let mut edge_id = FxHashMap::default();
        let mut edges = Vec::new();
        let mut edge_faces: Vec<Vec<usize>> = Vec::new();
        let mut sides = Vec::with_capacity(faces.len());
        for &f in &faces {
            in_patch[f] = true;
            let v = &mesh.faces[f].vertices;
            let n = v.len();
            let mut ids = Vec::with_capacity(n);
            for i in 0..n {
                let k = edge_key(v[i], v[(i + 1) % n]);
                let id = *edge_id.entry(k).or_insert_with(|| {
                    edges.push(k);
                    edge_faces.push(Vec::new());
                    edges.len() - 1
                });
                edge_faces[id].push(f);
                ids.push(id);
            }
            sides.push(ids);
        }
        let mut mesh_faces = vec![Vec::new(); edges.len()];
        for (g, face) in mesh.faces.iter().enumerate() {
            if mesh.is_polyline(g) {
                continue;
            }
            let v = &face.vertices;
            for i in 0..v.len() {
                if let Some(&id) = edge_id.get(&edge_key(v[i], v[(i + 1) % v.len()])) {
                    mesh_faces[id].push(g);
                }
            }
        }
        Self {
            faces,
            in_patch,
            edge_id,
            edges,
            edge_faces,
            mesh_faces,
            sides,
        }
    }
    /// An edge inside the patch, between exactly two patch polygons.
    fn interior(&self, e: usize) -> bool {
        self.edge_faces[e].len() == 2
    }
}

/// Edges at each point in the whole mesh ("valence", SUB-12).
fn valences(mesh: &Mesh) -> Vec<u32> {
    let mut v = vec![0u32; mesh.vertices.len()];
    for (a, b) in mesh.edges() {
        v[a as usize] += 1;
        v[b as usize] += 1;
    }
    v
}

/// SUB-11/SUB-Q4 (provisional, from the oracle's bumpy grid): the angle
/// test reads a polygon's normal at its first corner, (v1 - v0) x
/// (v_last - v0), not an averaged normal. Same as the averaged normal on a
/// planar polygon; on a non-planar one this reproduces every crease the
/// oracle shows, including the ones that aren't mirror-symmetric.
fn first_corner_normal(mesh: &Mesh, f: usize) -> DVec3 {
    let v = &mesh.faces[f].vertices;
    let p = |i: usize| mesh.vertices[v[i] as usize];
    let n = (p(1) - p(0))
        .cross(p(v.len() - 1) - p(0))
        .normalize_or_zero();
    if n == DVec3::ZERO { mesh.normal(f) } else { n }
}

/// Angle in radians between the normals of two polygons.
fn dihedral(normals: &[DVec3], f: usize, g: usize) -> f64 {
    normals[f].dot(normals[g]).clamp(-1., 1.).acos()
}

/// Corner normals for Smooth: per patch-adjacent polygon and point, the
/// average of the normals of the polygons at the point that meet this one at
/// no more than `max_angle` (the polygon's own included).
struct CornerNormals<'a> {
    normals: &'a [DVec3],
    around: FxHashMap<u32, Vec<usize>>,
    max_angle: f64,
}
impl CornerNormals<'_> {
    fn get(&self, f: usize, v: u32) -> DVec3 {
        let own = self.normals[f];
        let mut sum = DVec3::ZERO;
        for &g in self.around.get(&v).map(Vec::as_slice).unwrap_or(&[]) {
            if dihedral(self.normals, f, g) <= self.max_angle + 1e-12 {
                sum += self.normals[g];
            }
        }
        let n = sum.normalize_or_zero();
        if n == DVec3::ZERO { own } else { n }
    }
}

/// Where a new point came from, for its point attributes and Fractal size.
struct NewPoint {
    sources: Vec<u32>,
    /// The polygon it was made in (Fractal's size reference).
    face: usize,
}

/// Memory one polygon costs the app once it is in a scene: the mesh, its
/// undo copy and the viewport's geometry on both sides of the graphics
/// card. Measured at 4.6 KB on a Sphere subdivided five times (283k quads,
/// 2026-11-12), rounded up.
pub const BYTES_PER_POLYGON: u64 = 5_000;

/// Exactly how many polygons `mesh` has after `times` Subdivide passes of
/// `selected` (all when empty) with `method`: every n-sided polygon becomes
/// n quads, except that Faceted and Smooth split a triangle into four
/// triangles (SUB-20). Later passes subdivide the new polygons.
pub fn subdivided_polygon_count(
    mesh: &Mesh,
    selected: &[u32],
    method: SubdivideMethod,
    times: u32,
) -> u64 {
    let faces = working_faces(mesh, selected);
    let untouched = (mesh.faces.len() - faces.len()) as u64;
    let keeps_triangles = matches!(method, SubdivideMethod::Faceted | SubdivideMethod::Smooth);
    // The polygons being subdivided: triangles, and the corners of the rest.
    let (mut triangles, mut corners) = (0u64, 0u64);
    for &f in &faces {
        match mesh.faces[f].vertices.len() {
            3 => triangles += 1,
            n => corners += n as u64,
        }
    }
    let mut made = faces.len() as u64;
    for _ in 0..times {
        let quads = if keeps_triangles {
            corners
        } else {
            corners + 3 * triangles
        };
        if keeps_triangles {
            triangles = triangles.saturating_mul(4);
        } else {
            triangles = 0;
        }
        made = triangles.saturating_add(quads);
        corners = quads.saturating_mul(4);
    }
    untouched + made
}

/// Subdivide's result before it runs (`Editor::subdivide_estimate`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubdivideEstimate {
    /// Polygons of the changed meshes afterwards.
    pub polygons: u64,
    /// Memory those polygons need, in bytes.
    pub bytes: u64,
}

/// `n` with thousands separators: 4,521,984.
pub fn group_thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (k, c) in digits.chars().enumerate() {
        if k > 0 && (digits.len() - k).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// `bytes` as gigabytes with one decimal: "22.6 GB".
pub fn gigabytes(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1e9)
}

/// SUB-10..21: subdivide `selected` polygons of `mesh` (all when empty)
/// once with `options`. `seed` drives Fractal.
pub fn subdivide_polygons(
    mesh: &Mesh,
    selected: &[u32],
    options: &SubdivideOptions,
    seed: u64,
) -> Result<SubdivideOutcome> {
    options.validate()?;
    mesh.validate()?;
    if !mesh.attributes.extrude_edge_uses.is_empty() {
        return Err("Subdivide needs consistent winding; use Align first".into());
    }
    let faces = working_faces(mesh, selected);
    if faces.is_empty() {
        return Err("No polygons to subdivide".into());
    }
    let corners: usize = faces.iter().map(|&f| mesh.faces[f].vertices.len()).sum();
    // No fixed polygon cap (Jason, 2026-11-12): the editor checks the
    // result against free memory first (`Editor::subdivide_estimate`).
    // Point and polygon numbers must still fit the mesh's 32-bit indices.
    if subdivided_polygon_count(mesh, selected, options.method, 1) > u32::MAX as u64 / 2 {
        return Err(
            "Subdivide would make more polygons than a mesh can hold; nothing changed".into(),
        );
    }
    let method = options.method;
    let patch = Patch::new(mesh, faces);
    let n_points = mesh.vertices.len();
    let p = |v: u32| mesh.vertices[v as usize];
    let normals: Vec<DVec3> = (0..mesh.faces.len()).map(|f| mesh.normal(f)).collect();
    let max_angle = options.max_angle_degrees.to_radians();
    let catmull = matches!(
        method,
        SubdivideMethod::CatmullClark | SubdivideMethod::SdsSubdivide
    );
    // Triangles split into four triangles in Faceted and Smooth (SUB-20).
    let has_centre = |f: usize| catmull || mesh.faces[f].vertices.len() != 3;

    // Polygons around each patch point (Smooth's corner normals).
    let corner_normals = (method == SubdivideMethod::Smooth).then(|| {
        let mut around: FxHashMap<u32, Vec<usize>> = FxHashMap::default();
        for &f in &patch.faces {
            for &v in &mesh.faces[f].vertices {
                around.entry(v).or_default();
            }
        }
        for (g, face) in mesh.faces.iter().enumerate() {
            if mesh.is_polyline(g) {
                continue;
            }
            for v in &face.vertices {
                if let Some(list) = around.get_mut(v) {
                    list.push(g);
                }
            }
        }
        CornerNormals {
            normals: &normals,
            around,
            max_angle,
        }
    });

    // Centre points (SUB-10: the corners' average; Smooth pushes it out).
    let centres: Vec<DVec3> = patch
        .faces
        .iter()
        .map(|&f| {
            let v = &mesh.faces[f].vertices;
            let n = v.len();
            let average = v.iter().map(|&i| p(i)).sum::<DVec3>() / n as f64;
            match &corner_normals {
                Some(cn) if n >= 4 => {
                    // SUB-21 (provisional): average of the curved diagonals
                    // from each corner to the one across.
                    (0..n)
                        .map(|i| {
                            let j = (i + n / 2) % n;
                            hermite_mid(p(v[i]), cn.get(f, v[i]), p(v[j]), cn.get(f, v[j]))
                        })
                        .sum::<DVec3>()
                        / n as f64
                }
                _ => average,
            }
        })
        .collect();
    let patch_index: FxHashMap<usize, usize> = patch
        .faces
        .iter()
        .enumerate()
        .map(|(i, &f)| (f, i))
        .collect();

    // Edge sharpness for the Catmull-Clark methods: SUB-13 weights (as
    // sharpness = 10 x weight, the subdivision spec's rule) or SUB-11's angle.
    let weights: BTreeMap<(u32, u32), f64> = mesh
        .attributes
        .creases
        .iter()
        .map(|c| (edge_key(c.a, c.b), c.weight))
        .collect();
    let sharpness: Vec<f64> = (0..patch.edges.len())
        .map(|e| {
            if !patch.interior(e) {
                return 0.;
            }
            match method {
                SubdivideMethod::CatmullClark if !options.ignore_edge_weights => {
                    10. * weights
                        .get(&patch.edges[e])
                        .copied()
                        .unwrap_or(0.)
                        .clamp(0., 1.)
                }
                SubdivideMethod::SdsSubdivide => {
                    let [f, g] = [patch.edge_faces[e][0], patch.edge_faces[e][1]];
                    let angle = first_corner_normal(mesh, f)
                        .dot(first_corner_normal(mesh, g))
                        .clamp(-1., 1.)
                        .acos();
                    if angle > max_angle + 1e-12 {
                        f64::INFINITY
                    } else {
                        0.
                    }
                }
                _ => 0.,
            }
        })
        .collect();

    // Edge points.
    let edge_points: Vec<DVec3> = (0..patch.edges.len())
        .map(|e| {
            let (a, b) = patch.edges[e];
            let mid = (p(a) + p(b)) / 2.;
            match method {
                _ if catmull => {
                    if !patch.interior(e) {
                        return mid;
                    }
                    let [f, g] = [patch.edge_faces[e][0], patch.edge_faces[e][1]];
                    let smooth =
                        (p(a) + p(b) + centres[patch_index[&f]] + centres[patch_index[&g]]) / 4.;
                    smooth + (mid - smooth) * sharpness[e].min(1.)
                }
                SubdivideMethod::Smooth => {
                    let cn = corner_normals.as_ref().expect("Smooth has corner normals");
                    let around = &patch.mesh_faces[e];
                    let sharp = match around.as_slice() {
                        // An open border: curve within its one polygon.
                        [_] => false,
                        [f, g] => dihedral(&normals, *f, *g) > max_angle + 1e-12,
                        _ => true,
                    };
                    if sharp {
                        return mid;
                    }
                    let end = |v: u32| {
                        around
                            .iter()
                            .map(|&f| cn.get(f, v))
                            .sum::<DVec3>()
                            .normalize_or_zero()
                    };
                    hermite_mid(p(a), end(a), p(b), end(b))
                }
                _ => mid,
            }
        })
        .collect();

    // Original points: only the Catmull-Clark methods move them.
    let mut vertices = mesh.vertices.clone();
    if catmull {
        let valence = valences(mesh);
        #[derive(Clone, Copy, Default)]
        struct Around {
            centres: DVec3,
            face_count: u32,
            mids: DVec3,
            edge_count: u32,
            boundary: [DVec3; 2],
            boundary_count: u32,
            sharp: [DVec3; 2],
            sharp_count: u32,
            sharp_sum: f64,
        }
        let mut around: FxHashMap<u32, Around> = FxHashMap::default();
        for (i, &f) in patch.faces.iter().enumerate() {
            for &v in &mesh.faces[f].vertices {
                let a = around.entry(v).or_default();
                a.centres += centres[i];
                a.face_count += 1;
            }
        }
        for (e, &(x, y)) in patch.edges.iter().enumerate() {
            let open = !patch.interior(e);
            let s = sharpness[e];
            for (v, other) in [(x, y), (y, x)] {
                let a = around
                    .get_mut(&v)
                    .expect("patch edge point is a patch point");
                a.mids += (p(v) + p(other)) / 2.;
                a.edge_count += 1;
                if open {
                    if a.boundary_count < 2 {
                        a.boundary[a.boundary_count as usize] = p(other);
                    }
                    a.boundary_count += 1;
                } else if s > 0. {
                    if a.sharp_count < 2 {
                        a.sharp[a.sharp_count as usize] = p(other);
                    }
                    a.sharp_count += 1;
                    a.sharp_sum += s;
                }
            }
        }
        let crease_corners = method == SubdivideMethod::CatmullClark
            && options.boundary == SubdivideBoundary::CreaseAll;
        for (&v, a) in &around {
            let old = p(v);
            vertices[v as usize] = if a.boundary_count > 0 {
                // SUB-10/12/14: patch outline and open borders.
                if a.boundary_count == 2 && !(crease_corners && valence[v as usize] == 2) {
                    (a.boundary[0] + 6. * old + a.boundary[1]) / 8.
                } else {
                    old
                }
            } else {
                let n = a.edge_count as f64;
                let f = a.centres / a.face_count as f64;
                let r = a.mids / n;
                let smooth = (f + 2. * r + (n - 3.) * old) / n;
                let rule = match a.sharp_count {
                    0 | 1 => None,
                    2 => Some((a.sharp[0] + 6. * old + a.sharp[1]) / 8.),
                    _ => Some(old),
                };
                let average = a.sharp_sum / a.sharp_count.max(1) as f64;
                match rule {
                    None => smooth,
                    Some(q) if average >= 1. => q,
                    Some(q) => smooth + (q - smooth) * average,
                }
            };
        }
    }

    // New points: centres first, then edge points (SUB-16's order).
    let mut new_points: Vec<NewPoint> = Vec::new();
    let mut centre_id = vec![u32::MAX; patch.faces.len()];
    for (i, &f) in patch.faces.iter().enumerate() {
        if has_centre(f) {
            centre_id[i] = vertices.len() as u32;
            vertices.push(centres[i]);
            new_points.push(NewPoint {
                sources: mesh.faces[f].vertices.clone(),
                face: f,
            });
        }
    }
    let edge_base = vertices.len() as u32;
    for (e, &(a, b)) in patch.edges.iter().enumerate() {
        vertices.push(edge_points[e]);
        new_points.push(NewPoint {
            sources: vec![a, b],
            face: patch.edge_faces[e][0],
        });
    }
    if vertices.len() > u32::MAX as usize / 2 {
        return Err("Mesh point budget exceeded".into());
    }
    let edge_point = |e: usize| edge_base + e as u32;

    // Polygons, in mesh order; each patch polygon is replaced by its pieces.
    let mut out_faces: Vec<OutFace> = Vec::with_capacity(mesh.faces.len() + corners);
    let mut new_faces = Vec::new();
    // Per output polygon from the patch: its patch polygon and, per corner,
    // which point of that polygon it is (for the smoothed UVs below).
    let mut recipes: Vec<(usize, Vec<Corner>)> = Vec::new();
    for (f, face) in mesh.faces.iter().enumerate() {
        let v = &face.vertices;
        let n = v.len();
        if !patch.in_patch[f] {
            // SUB-14: an unselected neighbour takes the outline edge points.
            let mut vs = Vec::with_capacity(n + 2);
            let mut donors = Vec::with_capacity(n + 2);
            let mut blends = Vec::with_capacity(n + 2);
            let mut inserted = false;
            for i in 0..n {
                vs.push(v[i]);
                donors.push(Some((f, i)));
                blends.push(None);
                let closing = i + 1 < n || !mesh.is_polyline(f);
                if !mesh.is_polyline(f)
                    && closing
                    && let Some(&e) = patch.edge_id.get(&edge_key(v[i], v[(i + 1) % n]))
                {
                    vs.push(edge_point(e));
                    donors.push(None);
                    blends.push(Some(vec![((f, i), 0.5), ((f, (i + 1) % n), 0.5)]));
                    inserted = true;
                }
            }
            out_faces.push(if inserted {
                OutFace {
                    template: f,
                    vertices: vs,
                    donors,
                    blends,
                }
            } else {
                OutFace::copy(mesh, f, v.clone())
            });
            continue;
        }
        let pi = patch_index[&f];
        let sides = &patch.sides[pi];
        let edge_blend = |i: usize| -> CornerBlend { vec![((f, i), 0.5), ((f, (i + 1) % n), 0.5)] };
        let mut push = |out_faces: &mut Vec<OutFace>, recipe: Vec<Corner>| {
            let mut vs = Vec::with_capacity(recipe.len());
            let mut donors = Vec::with_capacity(recipe.len());
            let mut blends = Vec::with_capacity(recipe.len());
            for c in &recipe {
                match *c {
                    Corner::Original(i) => {
                        vs.push(v[i]);
                        donors.push(Some((f, i)));
                        blends.push(None);
                    }
                    Corner::Edge(i) => {
                        vs.push(edge_point(sides[i]));
                        donors.push(None);
                        blends.push(Some(edge_blend(i)));
                    }
                    Corner::Centre => {
                        vs.push(centre_id[pi]);
                        donors.push(None);
                        blends.push(Some((0..n).map(|c| ((f, c), 1. / n as f64)).collect()));
                    }
                }
            }
            new_faces.push(out_faces.len() as u32);
            out_faces.push(OutFace {
                template: f,
                vertices: vs,
                donors,
                blends,
            });
            recipes.push((pi, recipe));
        };
        if has_centre(f) {
            for i in 0..n {
                push(
                    &mut out_faces,
                    vec![
                        Corner::Original(i),
                        Corner::Edge(i),
                        Corner::Centre,
                        Corner::Edge((i + n - 1) % n),
                    ],
                );
            }
        } else {
            // SUB-20: a triangle becomes four triangles.
            for i in 0..3 {
                push(
                    &mut out_faces,
                    vec![
                        Corner::Original(i),
                        Corner::Edge(i),
                        Corner::Edge((i + 2) % 3),
                    ],
                );
            }
            push(
                &mut out_faces,
                vec![Corner::Edge(0), Corner::Edge(1), Corner::Edge(2)],
            );
        }
    }
    let mut out = rebuild(mesh, vertices, out_faces, &[], "Subdivide")?;

    // Edge weights on split edges (rebuild keeps only unsplit ones). The
    // Catmull-Clark methods used them for this step, so each half carries
    // sharpness s - 1 (weight - 0.1), as a further subdivision level would
    // (provisional, SUBT-Q02); the other methods keep the weight.
    let lowers = method == SubdivideMethod::CatmullClark && !options.ignore_edge_weights;
    for c in &mesh.attributes.creases {
        if let Some(&e) = patch.edge_id.get(&edge_key(c.a, c.b)) {
            let w = if lowers && c.weight > 0. {
                c.weight - 0.1
            } else {
                c.weight
            };
            if lowers && c.weight > 0. && w <= 1e-12 {
                continue;
            }
            let m = edge_point(e);
            for (a, b) in [(c.a, m), (m, c.b)] {
                let (a, b) = (a.min(b), a.max(b));
                out.attributes
                    .creases
                    .push(crate::MeshCrease { a, b, weight: w });
            }
        }
    }
    out.attributes.creases.sort_by_key(|c| (c.a, c.b));
    out.attributes.creases.dedup_by_key(|c| (c.a, c.b));

    // SUB-32: Linear UVs off smooths the UVs with the same Catmull-Clark
    // step (provisional); Faceted and Smooth never change the UV layout.
    if catmull && !options.linear_uvs {
        smooth_uvs(mesh, &patch, &new_faces, &recipes, &mut out);
    }
    interpolate_point_data(&mut out, n_points, &new_points);

    // SUB-31 Fractal (provisional): each new point moves by a random vector
    // up to Fractal% of its polygon's mean edge length.
    if options.fractal_percent > 0. {
        let mut rng = Rng(seed ^ 0x9E37_79B9_7F4A_7C15);
        let size = |f: usize| {
            let v = &mesh.faces[f].vertices;
            (0..v.len())
                .map(|i| p(v[i]).distance(p(v[(i + 1) % v.len()])))
                .sum::<f64>()
                / v.len() as f64
        };
        for (k, np) in new_points.iter().enumerate() {
            let amount = options.fractal_percent / 100. * size(np.face);
            out.vertices[n_points + k] += rng.ball() * amount;
        }
    }
    let out = finish(out, "Subdivide")?;
    Ok(SubdivideOutcome {
        mesh: out,
        new_faces,
    })
}

/// One corner of a piece of a patch polygon: an original corner i, the edge
/// point of side i, or the polygon's centre point.
#[derive(Clone, Copy, Debug)]
enum Corner {
    Original(usize),
    Edge(usize),
    Centre,
}

/// SUB-32 (provisional): the Catmull-Clark step applied to each UV map.
/// UV points are corners sharing a point and a UV value; a UV edge used by
/// one patch polygon is a border. Points and edges on the patch outline that
/// an unselected polygon also uses stay put, so neighbours' UVs still meet.
fn smooth_uvs(
    mesh: &Mesh,
    patch: &Patch,
    new_faces: &[u32],
    recipes: &[(usize, Vec<Corner>)],
    out: &mut Mesh,
) {
    for (name, rows) in &mesh.uv_maps {
        // UV point per (patch polygon position, corner).
        let mut id_of: FxHashMap<(u32, u64, u64), usize> = FxHashMap::default();
        let mut uv_points: Vec<DVec2> = Vec::new();
        let mut uv_vertex: Vec<u32> = Vec::new();
        let mut face_uv: Vec<Vec<Option<usize>>> = Vec::with_capacity(patch.faces.len());
        for &f in &patch.faces {
            let v = &mesh.faces[f].vertices;
            let row = rows.get(f);
            face_uv.push(
                (0..v.len())
                    .map(|c| {
                        let uv = row.and_then(|r| r.get(c)).copied().flatten()?;
                        let k = (v[c], uv.x.to_bits(), uv.y.to_bits());
                        Some(*id_of.entry(k).or_insert_with(|| {
                            uv_points.push(uv);
                            uv_vertex.push(v[c]);
                            uv_points.len() - 1
                        }))
                    })
                    .collect(),
            );
        }
        if uv_points.is_empty() {
            continue;
        }
        // Face points and UV edges.
        let face_point: Vec<Option<DVec2>> = face_uv
            .iter()
            .map(|ids| {
                let pts: Option<Vec<DVec2>> = ids.iter().map(|i| i.map(|i| uv_points[i])).collect();
                pts.map(|p| p.iter().copied().sum::<DVec2>() / p.len() as f64)
            })
            .collect();
        let mut uv_edges: FxHashMap<(usize, usize), Vec<usize>> = FxHashMap::default();
        for (pi, ids) in face_uv.iter().enumerate() {
            let n = ids.len();
            for i in 0..n {
                if let (Some(a), Some(b)) = (ids[i], ids[(i + 1) % n]) {
                    uv_edges.entry((a.min(b), a.max(b))).or_default().push(pi);
                }
            }
        }
        // A UV point whose mesh edge is shared with a polygon outside the
        // patch is pinned.
        let mut pinned = vec![false; uv_points.len()];
        let outline_shared = |a: u32, b: u32| {
            patch
                .edge_id
                .get(&edge_key(a, b))
                .is_some_and(|&e| patch.mesh_faces[e].iter().any(|&g| !patch.in_patch[g]))
        };
        for &(a, b) in uv_edges.keys() {
            if outline_shared(uv_vertex[a], uv_vertex[b]) {
                pinned[a] = true;
                pinned[b] = true;
            }
        }
        let edge_point = |a: usize, b: usize| -> DVec2 {
            let k = (a.min(b), a.max(b));
            let mid = (uv_points[a] + uv_points[b]) / 2.;
            match uv_edges.get(&k).map(Vec::as_slice) {
                Some([f, g]) if !outline_shared(uv_vertex[a], uv_vertex[b]) => {
                    match (face_point[*f], face_point[*g]) {
                        (Some(x), Some(y)) => (uv_points[a] + uv_points[b] + x + y) / 4.,
                        _ => mid,
                    }
                }
                _ => mid,
            }
        };
        #[derive(Clone, Copy, Default)]
        struct Around {
            faces: DVec2,
            face_count: u32,
            mids: DVec2,
            edge_count: u32,
            boundary: [DVec2; 2],
            boundary_count: u32,
        }
        let mut around = vec![Around::default(); uv_points.len()];
        for (pi, ids) in face_uv.iter().enumerate() {
            if let Some(fp) = face_point[pi] {
                for i in ids.iter().flatten() {
                    around[*i].faces += fp;
                    around[*i].face_count += 1;
                }
            }
        }
        for (&(a, b), faces) in &uv_edges {
            for (v, o) in [(a, b), (b, a)] {
                let s = &mut around[v];
                s.mids += (uv_points[v] + uv_points[o]) / 2.;
                s.edge_count += 1;
                if faces.len() != 2 {
                    if s.boundary_count < 2 {
                        s.boundary[s.boundary_count as usize] = uv_points[o];
                    }
                    s.boundary_count += 1;
                }
            }
        }
        let moved: Vec<DVec2> = (0..uv_points.len())
            .map(|i| {
                let s = around[i];
                let p = uv_points[i];
                if pinned[i] || s.edge_count == 0 || s.face_count == 0 {
                    p
                } else if s.boundary_count > 0 {
                    if s.boundary_count == 2 {
                        (s.boundary[0] + 6. * p + s.boundary[1]) / 8.
                    } else {
                        p
                    }
                } else {
                    let n = s.edge_count as f64;
                    let f = s.faces / s.face_count as f64;
                    let r = s.mids / n;
                    (f + 2. * r + (n - 3.) * p) / n
                }
            })
            .collect();
        let Some(out_rows) = out.uv_maps.get_mut(name) else {
            continue;
        };
        for (k, &face) in new_faces.iter().enumerate() {
            let (pi, recipe) = &recipes[k];
            let ids = &face_uv[*pi];
            let n = ids.len();
            let Some(row) = out_rows.get_mut(face as usize) else {
                continue;
            };
            for (c, corner) in recipe.iter().enumerate() {
                let uv = match *corner {
                    Corner::Original(i) => ids[i].map(|i| moved[i]),
                    Corner::Edge(i) => ids[i].zip(ids[(i + 1) % n]).map(|(a, b)| edge_point(a, b)),
                    Corner::Centre => face_point[*pi],
                };
                if let (Some(uv), Some(slot)) = (uv, row.get_mut(c)) {
                    *slot = Some(uv);
                }
            }
        }
    }
}

/// New points take the average of their source points' point data (weights,
/// colours, morph deltas, continuous UVs, typed point maps) when every
/// source has a value, and join a point set when every source is in it.
fn interpolate_point_data(out: &mut Mesh, first_new: usize, new_points: &[NewPoint]) {
    fn blend<T: Copy + std::ops::Add<Output = T> + std::ops::Mul<f64, Output = T>>(
        map: &mut BTreeMap<u32, T>,
        first_new: usize,
        new_points: &[NewPoint],
    ) {
        for (k, np) in new_points.iter().enumerate() {
            let values: Option<Vec<T>> = np.sources.iter().map(|s| map.get(s).copied()).collect();
            if let Some(values) = values
                && let Some((first, rest)) = values.split_first()
            {
                let w = 1. / values.len() as f64;
                let sum = rest.iter().fold(*first * w, |acc, v| acc + *v * w);
                map.insert((first_new + k) as u32, sum);
            }
        }
    }
    let a = &mut out.attributes;
    for m in a.point_weights.values_mut() {
        blend(m, first_new, new_points);
    }
    for m in a.point_colors.values_mut() {
        blend(m, first_new, new_points);
    }
    for m in a.relative_morphs.values_mut() {
        blend(m, first_new, new_points);
    }
    for m in a.point_uvs.values_mut() {
        blend(m, first_new, new_points);
    }
    for map in a.typed_maps.values_mut() {
        for (k, np) in new_points.iter().enumerate() {
            let values: Option<Vec<&Vec<f64>>> =
                np.sources.iter().map(|s| map.points.get(s)).collect();
            if let Some(values) = values
                && let Some(first) = values.first()
                && values.iter().all(|v| v.len() == first.len())
            {
                let mut sum = vec![0.; first.len()];
                for v in &values {
                    for (s, x) in sum.iter_mut().zip(v.iter()) {
                        *s += x / values.len() as f64;
                    }
                }
                map.points.insert((first_new + k) as u32, sum);
            }
        }
    }
    for set in a.vertex_sets.values_mut() {
        for (k, np) in new_points.iter().enumerate() {
            if np.sources.iter().all(|s| set.contains(s)) {
                set.insert((first_new + k) as u32);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// UnSubdivide
// ---------------------------------------------------------------------------

/// What a point was in the Catmull-Clark step being undone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// An original point (it stays, moved back unless Flat).
    Original,
    /// An edge point (removed).
    Edge,
    /// A centre point (removed; its four quads become one).
    Centre,
}

/// SUB-41..44: undo `options.iterations` Catmull-Clark steps on `selected`
/// polygons (all when empty). Returns the mesh and the rebuilt polygons.
pub fn unsubdivide_polygons(
    mesh: &Mesh,
    selected: &[u32],
    options: &UnsubdivideOptions,
) -> Result<(Mesh, Vec<u32>)> {
    if options.iterations > MAX_UNSUBDIVIDE_ITERATIONS {
        return Err(format!(
            "Iterations must be between 0 and {MAX_UNSUBDIVIDE_ITERATIONS}"
        ));
    }
    mesh.validate()?;
    if !mesh.attributes.extrude_edge_uses.is_empty() {
        return Err("UnSubdivide needs consistent winding; use Align first".into());
    }
    let mut faces = working_faces(mesh, selected);
    if faces.is_empty() {
        return Err("No polygons to UnSubdivide".into());
    }
    if faces.iter().any(|&f| mesh.faces[f].vertices.len() != 4) {
        return Err(ONLY_QUADS.into());
    }
    let mut current = mesh.clone();
    for _ in 0..options.iterations {
        // A partial merge can leave n-gons on its outline (SUB-40b); a
        // further iteration needs quads again (SUB-44).
        if faces.iter().any(|&f| current.faces[f].vertices.len() != 4) {
            return Err(ONLY_QUADS.into());
        }
        let (next, merged) = unsubdivide_once(&current, &faces, options)?;
        current = next;
        faces = merged.iter().map(|&f| f as usize).collect();
    }
    Ok((current, faces.into_iter().map(|f| f as u32).collect()))
}

fn unsubdivide_once(
    mesh: &Mesh,
    work: &[usize],
    options: &UnsubdivideOptions,
) -> Result<(Mesh, Vec<u32>)> {
    const NOT_FOUND: &str =
        "UnSubdivide couldn't find groups of four quads around a centre point; nothing changed";
    let n_points = mesh.vertices.len();
    let mut in_work = vec![false; mesh.faces.len()];
    for &f in work {
        in_work[f] = true;
    }
    // Surface polygons at each point, whole mesh; and edges of work quads.
    let mut around: Vec<Vec<usize>> = vec![Vec::new(); n_points];
    for (g, face) in mesh.faces.iter().enumerate() {
        if !mesh.is_polyline(g) {
            for &v in &face.vertices {
                around[v as usize].push(g);
            }
        }
    }
    let counts = mesh.edge_counts();
    let valence = valences(mesh);
    let mut edge_work: FxHashMap<(u32, u32), Vec<usize>> = FxHashMap::default();
    for &f in work {
        let v = &mesh.faces[f].vertices;
        for i in 0..4 {
            edge_work
                .entry(edge_key(v[i], v[(i + 1) % 4]))
                .or_default()
                .push(f);
        }
    }
    // centre[f]: the corner position (0..4) of f's centre point.
    let mut centre = vec![usize::MAX; mesh.faces.len()];
    let mut role: Vec<Option<Role>> = vec![None; n_points];
    let mut seen = vec![false; mesh.faces.len()];
    for &seed in work {
        if seen[seed] {
            continue;
        }
        // The connected piece of work quads around `seed`.
        let mut component = Vec::new();
        let mut queue = VecDeque::from([seed]);
        seen[seed] = true;
        while let Some(f) = queue.pop_front() {
            component.push(f);
            let v = &mesh.faces[f].vertices;
            for i in 0..4 {
                for &g in &edge_work[&edge_key(v[i], v[(i + 1) % 4])] {
                    if !seen[g] {
                        seen[g] = true;
                        queue.push_back(g);
                    }
                }
            }
        }
        // Try each choice of the seed's centre corner; keep the first one
        // that labels the whole piece consistently and passes the hints,
        // preferring one that puts odd-valence points at original corners.
        let mut best: Option<((std::cmp::Reverse<usize>, usize), Labels)> = None;
        for c0 in 0..4 {
            let Some(labels) = label_component(mesh, &component, seed, c0, &edge_work) else {
                continue;
            };
            let (centres, roles) = &labels;
            let Some(hint_breaks) =
                groups_valid(mesh, centres, roles, &around, &in_work, &valence, options)
            else {
                continue;
            };
            let score = roles
                .iter()
                .filter(|&(&v, &r)| {
                    let boundary = around[v as usize].len() != valence[v as usize] as usize;
                    let irregular = if boundary {
                        valence[v as usize] != 3
                    } else {
                        valence[v as usize] != 4
                    };
                    irregular && r == Role::Original
                })
                .count();
            // Fewest hint breaks first, then odd-valence points at
            // original corners.
            let rank = (std::cmp::Reverse(hint_breaks), score);
            if best.as_ref().is_none_or(|(r, _)| rank > *r) {
                best = Some((rank, labels));
            }
        }
        let (_, (centres, roles)) = best.ok_or(NOT_FOUND)?;
        for (f, c) in centres {
            centre[f] = c;
        }
        for (v, r) in roles {
            role[v as usize] = Some(r);
        }
    }

    // New positions for the original points (SUB-41; Flat keeps them).
    let p = |v: u32| mesh.vertices[v as usize];
    let mut vertices = mesh.vertices.clone();
    if !options.flat {
        let originals: Vec<u32> = (0..n_points as u32)
            .filter(|&v| role[v as usize] == Some(Role::Original))
            .collect();
        // Per original point: its edge-point and centre-point neighbours.
        let neighbours = |v: u32| -> (Vec<u32>, Vec<u32>) {
            let mut edges = BTreeSet::new();
            let mut centres = BTreeSet::new();
            for &f in &around[v as usize] {
                if !in_work[f] {
                    continue;
                }
                let q = &mesh.faces[f].vertices;
                let at = q.iter().position(|&x| x == v).expect("corner");
                edges.insert(q[(at + 1) % 4]);
                edges.insert(q[(at + 3) % 4]);
                centres.insert(q[(at + 2) % 4]);
            }
            (edges.into_iter().collect(), centres.into_iter().collect())
        };
        let interior = |v: u32| {
            around[v as usize].iter().all(|&f| in_work[f])
                && neighbours(v)
                    .0
                    .iter()
                    .all(|&e| counts.get(&edge_key(v, e)).copied().unwrap_or(0) == 2)
        };
        let mut solved: Vec<Option<DVec3>> = vec![None; n_points];
        let mut pending = Vec::new();
        for &v in &originals {
            let (edges, centres) = neighbours(v);
            if interior(v) {
                let n = around[v as usize].len() as f64;
                if n as usize == 3 {
                    pending.push(v);
                    continue;
                }
                let e = edges.iter().map(|&x| p(x)).sum::<DVec3>() / edges.len() as f64;
                let f = centres.iter().map(|&x| p(x)).sum::<DVec3>() / centres.len() as f64;
                // Exact inverse of (F + 2R + (n - 3)P) / n: the original edge
                // midpoints average to R = 2E - F.
                solved[v as usize] = Some((n * p(v) - 4. * e + f) / (n - 3.));
            } else {
                // Border: P' = 3/4 P + 1/8 (P_l + P_r) and border edge
                // points are midpoints, so P = 2P' - (E_l + E_r) / 2.
                let border: Vec<u32> = edges
                    .iter()
                    .copied()
                    .filter(|&e| {
                        edge_work.get(&edge_key(v, e)).map_or(0, Vec::len) == 1
                            || counts.get(&edge_key(v, e)).copied().unwrap_or(0) == 1
                    })
                    .collect();
                solved[v as usize] = Some(if border.len() == 2 {
                    2. * p(v) - (p(border[0]) + p(border[1])) / 2.
                } else {
                    p(v)
                });
            }
        }
        // Valence 3: the step can't be inverted from the point alone. Use
        // P_a + P_b = 4E - C1 - C2 from a solved neighbour across each edge
        // point; with none (a cube), the parallelogram estimate 3P' - 2F
        // (exact when the original quads are parallelograms; SUBT-Q04).
        let across = |v: u32, e: u32| -> Option<(u32, DVec3)> {
            let mut other = None;
            let mut cs = Vec::new();
            for &f in &around[e as usize] {
                if !in_work[f] {
                    continue;
                }
                let q = &mesh.faces[f].vertices;
                let at = q.iter().position(|&x| x == e).expect("corner");
                for o in [q[(at + 1) % 4], q[(at + 3) % 4]] {
                    if role[o as usize] == Some(Role::Original) && o != v {
                        other = Some(o);
                    }
                    if role[o as usize] == Some(Role::Centre) && !cs.contains(&o) {
                        cs.push(o);
                    }
                }
            }
            let o = other?;
            (cs.len() == 2).then(|| (o, 4. * p(e) - p(cs[0]) - p(cs[1])))
        };
        loop {
            let mut progress = false;
            pending.retain(|&v| {
                let (edges, _) = neighbours(v);
                let known: Vec<DVec3> = edges
                    .iter()
                    .filter_map(|&e| across(v, e))
                    .filter_map(|(o, s)| solved[o as usize].map(|po| s - po))
                    .collect();
                if known.is_empty() {
                    return true;
                }
                solved[v as usize] =
                    Some(known.iter().copied().sum::<DVec3>() / known.len() as f64);
                progress = true;
                false
            });
            if !progress || pending.is_empty() {
                break;
            }
        }
        for v in pending {
            let (_, centres) = neighbours(v);
            let f = centres.iter().map(|&x| p(x)).sum::<DVec3>() / centres.len() as f64;
            solved[v as usize] = Some(3. * p(v) - 2. * f);
        }
        for v in originals {
            if let Some(x) = solved[v as usize] {
                vertices[v as usize] = x;
            }
        }
    }

    // Merge each group: the four quads around a centre point become one quad
    // through their original corners, in winding order.
    let mut group_of: FxHashMap<u32, Vec<usize>> = FxHashMap::default();
    for &f in work {
        let c = mesh.faces[f].vertices[centre[f]];
        group_of.entry(c).or_default().push(f);
    }
    let mut first_of: FxHashMap<usize, OutFace> = FxHashMap::default();
    let mut dropped = vec![false; mesh.faces.len()];
    for children in group_of.values() {
        let start = *children.iter().min().expect("group has quads");
        let at = |f: usize, k: usize| mesh.faces[f].vertices[(centre[f] + k) % 4];
        let mut vs = Vec::with_capacity(4);
        let mut donors = Vec::with_capacity(4);
        let mut f = start;
        // SUB-40b: outline edge points an unselected neighbour still uses
        // stay, as corners of the merged polygon (an n-gon there), so the
        // two polygons keep sharing their edge: (point, its two corners).
        let mut kept: Vec<(u32, u32, u32)> = Vec::new();
        for _ in 0..4 {
            let corner = at(f, 2);
            vs.push(corner);
            donors.push(Some((f, (centre[f] + 2) % 4)));
            // The next quad round the centre shares this one's next edge
            // point as its previous one.
            let next_edge = at(f, 3);
            let next = *children
                .iter()
                .find(|&&g| at(g, 1) == next_edge)
                .ok_or(NOT_FOUND)?;
            if around[next_edge as usize].iter().any(|&g| !in_work[g]) {
                vs.push(next_edge);
                donors.push(Some((f, (centre[f] + 3) % 4)));
                kept.push((next_edge, corner, at(next, 2)));
            }
            f = next;
        }
        if f != start {
            return Err(NOT_FOUND.into());
        }
        // Flat off: a kept point goes to the middle of its two restored
        // corners (the inverse of the boundary edge rule), keeping the
        // shared edge straight; the neighbour moves with it.
        if !options.flat {
            for &(point, a, b) in &kept {
                vertices[point as usize] = (vertices[a as usize] + vertices[b as usize]) / 2.;
            }
        }
        for &c in children {
            dropped[c] = true;
        }
        first_of.insert(
            start,
            OutFace {
                template: start,
                vertices: vs,
                donors,
                blends: Vec::new(),
            },
        );
    }
    let mut out_faces = Vec::with_capacity(mesh.faces.len() / 4 + 1);
    let mut merged = Vec::new();
    let mut gone = BTreeSet::new();
    for (v, r) in role.iter().enumerate() {
        if matches!(r, Some(Role::Edge | Role::Centre)) {
            gone.insert(v as u32);
        }
    }
    for (f, face) in mesh.faces.iter().enumerate() {
        if let Some(o) = first_of.remove(&f) {
            merged.push(out_faces.len() as u32);
            out_faces.push(o);
        } else if dropped[f] {
            continue;
        } else {
            // SUB-40a: unselected polygons keep their topology (outline
            // edge points they share stay); shared original points move
            // with the merged quads because they are the same points.
            out_faces.push(OutFace::copy(mesh, f, face.vertices.clone()));
        }
    }
    let mut out = rebuild(mesh, vertices, out_faces, &[], "UnSubdivide")?;
    // Edge weights on merged edges: the larger half's weight, plus the 0.1
    // a Catmull-Clark Subdivide took off (provisional, SUBT-Q02).
    let weight: BTreeMap<(u32, u32), f64> = mesh
        .attributes
        .creases
        .iter()
        .map(|c| (edge_key(c.a, c.b), c.weight))
        .collect();
    let mut restored = Vec::new();
    for &f in &merged {
        let q = out.faces[f as usize].vertices.clone();
        for i in 0..q.len() {
            let (a, b) = (q[i], q[(i + 1) % q.len()]);
            // A side running to a kept outline point is an old edge; it
            // keeps its own weight.
            if role[a as usize] == Some(Role::Edge) || role[b as usize] == Some(Role::Edge) {
                continue;
            }
            // The edge point between a and b in the old mesh.
            let mid = around[a as usize].iter().find_map(|&g| {
                let v = &mesh.faces[g].vertices;
                let at = v.iter().position(|&x| x == a)?;
                [v[(at + 1) % v.len()], v[(at + v.len() - 1) % v.len()]]
                    .into_iter()
                    .find(|&m| {
                        role[m as usize] == Some(Role::Edge)
                            && around[m as usize]
                                .iter()
                                .any(|&h| mesh.faces[h].vertices.contains(&b))
                    })
            });
            if let Some(m) = mid {
                let w = [edge_key(a, m), edge_key(m, b)]
                    .iter()
                    .filter_map(|k| weight.get(k))
                    .copied()
                    .fold(None, |acc: Option<f64>, w| {
                        Some(acc.map_or(w, |x| x.max(w)))
                    });
                if let Some(w) = w {
                    let w = if w > 0. { (w + 0.1).min(1.) } else { w };
                    restored.push(crate::MeshCrease {
                        a: a.min(b),
                        b: a.max(b),
                        weight: w,
                    });
                }
            }
        }
    }
    out.attributes.creases.extend(restored);
    out.attributes.creases.sort_by_key(|c| (c.a, c.b));
    out.attributes.creases.dedup_by_key(|c| (c.a, c.b));
    out.validate()
        .map_err(|e| format!("UnSubdivide produced an invalid mesh ({e}); nothing changed"))?;
    let cleaned = crate::mesh_cleanup::remove_unreferenced(&out, Some(&gone))?;
    let out = finish(cleaned.mesh, "UnSubdivide")?;
    Ok((out, merged))
}

/// Per quad its centre corner, and per point its role.
type Labels = (FxHashMap<usize, usize>, FxHashMap<u32, Role>);

/// Label one connected piece of work quads with the seed's centre at corner
/// `c0`, propagating across shared edges. `None` when the labels clash.
fn label_component(
    mesh: &Mesh,
    component: &[usize],
    seed: usize,
    c0: usize,
    edge_work: &FxHashMap<(u32, u32), Vec<usize>>,
) -> Option<Labels> {
    let mut centres: FxHashMap<usize, usize> = FxHashMap::default();
    let mut roles: FxHashMap<u32, Role> = FxHashMap::default();
    let assign = |f: usize,
                  c: usize,
                  centres: &mut FxHashMap<usize, usize>,
                  roles: &mut FxHashMap<u32, Role>|
     -> bool {
        centres.insert(f, c);
        let v = &mesh.faces[f].vertices;
        for (k, &point) in v.iter().enumerate() {
            let r = match (k + 4 - c) % 4 {
                0 => Role::Centre,
                2 => Role::Original,
                _ => Role::Edge,
            };
            match roles.get(&point) {
                Some(&old) if old != r => return false,
                _ => {
                    roles.insert(point, r);
                }
            }
        }
        true
    };
    if !assign(seed, c0, &mut centres, &mut roles) {
        return None;
    }
    let mut queue = VecDeque::from([seed]);
    while let Some(f) = queue.pop_front() {
        let v = mesh.faces[f].vertices.clone();
        for i in 0..4 {
            let (a, b) = (v[i], v[(i + 1) % 4]);
            for &g in &edge_work[&edge_key(a, b)] {
                if g == f {
                    continue;
                }
                let q = &mesh.faces[g].vertices;
                let pa = q.iter().position(|&x| x == a)?;
                let pb = q.iter().position(|&x| x == b)?;
                // The centre corner of g follows from the shared edge: the
                // centre point itself, or two steps from the original point.
                let c = match (roles[&a], roles[&b]) {
                    (Role::Centre, _) => pa,
                    (_, Role::Centre) => pb,
                    (Role::Original, _) => (pa + 2) % 4,
                    (_, Role::Original) => (pb + 2) % 4,
                    _ => return None,
                };
                match centres.get(&g) {
                    Some(&old) if old != c => return None,
                    Some(_) => {}
                    None => {
                        if !assign(g, c, &mut centres, &mut roles) {
                            return None;
                        }
                        queue.push_back(g);
                    }
                }
            }
        }
    }
    (centres.len() == component.len()).then_some((centres, roles))
}

/// `None` unless every centre point has exactly four quads round it, all
/// in the piece and no other polygon, and edge points sit on two or four of
/// them. Otherwise how many groups break a corner hint: hints only break
/// ties between valid groupings (SUB-40 corrected), they never refuse one.
fn groups_valid(
    mesh: &Mesh,
    centres: &FxHashMap<usize, usize>,
    roles: &FxHashMap<u32, Role>,
    around: &[Vec<usize>],
    in_work: &[bool],
    valence: &[u32],
    options: &UnsubdivideOptions,
) -> Option<usize> {
    let mut groups: FxHashMap<u32, Vec<usize>> = FxHashMap::default();
    for (&f, &c) in centres {
        groups.entry(mesh.faces[f].vertices[c]).or_default().push(f);
    }
    for (&v, &r) in roles {
        let at = &around[v as usize];
        let ok = match r {
            Role::Centre => {
                at.len() == 4
                    && valence[v as usize] == 4
                    && groups.get(&v).is_some_and(|g| g.len() == 4)
            }
            Role::Edge => {
                let work = at.iter().filter(|&&f| in_work[f]).count();
                (work == 2 || work == 4) && (valence[v as usize] == 3 || valence[v as usize] == 4)
            }
            Role::Original => true,
        };
        if !ok {
            return None;
        }
    }
    let same = |faces: &[usize], value: &dyn Fn(usize) -> Option<String>| {
        faces.windows(2).all(|w| value(w[0]) == value(w[1]))
    };
    let breaks = |faces: &[usize]| {
        (options.material_hint && !same(faces, &|f| Some(mesh.faces[f].material.clone())))
            || (options.part_hint && !same(faces, &|f| mesh.faces[f].part.clone()))
            || (options.smoothing_group_hint
                && !same(faces, &|f| {
                    mesh.attributes.smoothing_labels.get(f).cloned().flatten()
                }))
            || (options.selection_set_hint
                && mesh
                    .attributes
                    .polygon_sets
                    .get(&options.selection_set_name)
                    .is_some_and(|set| {
                        !same(faces, &|f| set.contains(&(f as u32)).then(String::new))
                    }))
            || (options.uv_hint && !uv_continuous(mesh, faces))
    };
    Some(groups.values().filter(|faces| breaks(faces)).count())
}

/// The four quads of a group carry one continuous UV patch: at every point
/// two of them share, their UVs agree in every map.
fn uv_continuous(mesh: &Mesh, faces: &[usize]) -> bool {
    for rows in mesh.uv_maps.values() {
        let mut at: BTreeMap<u32, Option<DVec2>> = BTreeMap::new();
        for &f in faces {
            for (c, &v) in mesh.faces[f].vertices.iter().enumerate() {
                let uv = rows.get(f).and_then(|r| r.get(c)).copied().flatten();
                match at.get(&v) {
                    Some(old) => match (old, uv) {
                        (Some(a), Some(b)) if a.distance(b) > 1e-9 => return false,
                        (Some(_), None) | (None, Some(_)) => return false,
                        _ => {}
                    },
                    None => {
                        at.insert(v, uv);
                    }
                }
            }
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Center
// ---------------------------------------------------------------------------

/// CEN-01..03: move `points` (every used point when empty) so their world
/// bounding-box centre is 0 on the `axes` that are on. `world` is the mesh
/// item's world matrix.
pub fn center_points(mesh: &Mesh, world: DMat4, points: &[u32], axes: [bool; 3]) -> Result<Mesh> {
    let points: Vec<u32> = if points.is_empty() {
        let used = mesh.used_points();
        (0..mesh.vertices.len() as u32)
            .filter(|&v| used[v as usize])
            .collect()
    } else {
        points.to_vec()
    };
    if points.is_empty() {
        return Err("Center: the mesh has no points".into());
    }
    let mut lo = DVec3::INFINITY;
    let mut hi = DVec3::NEG_INFINITY;
    for &v in &points {
        let w = world.transform_point3(
            *mesh
                .vertices
                .get(v as usize)
                .ok_or("A selected point no longer exists")?,
        );
        lo = lo.min(w);
        hi = hi.max(w);
    }
    let centre = (lo + hi) / 2.;
    let mask = DVec3::new(
        f64::from(u8::from(axes[0])),
        f64::from(u8::from(axes[1])),
        f64::from(u8::from(axes[2])),
    );
    let world_delta = -centre * mask;
    let inverse = world.inverse();
    if !inverse.is_finite() {
        return Err("Center: the item's transform can't be inverted".into());
    }
    let local = inverse.transform_vector3(world_delta);
    let mut out = mesh.clone();
    let mut moved = BTreeSet::new();
    for &v in &points {
        if moved.insert(v) {
            out.vertices[v as usize] += local;
        }
    }
    Ok(out)
}

/// `vert.center` axis words (the reference name map): x, y, z, xy, yz, zx,
/// all.
pub fn center_axes(word: &str) -> Option<[bool; 3]> {
    let w = word.trim().trim_matches('"').to_ascii_lowercase();
    Some(match w.as_str() {
        "x" | "0" => [true, false, false],
        "y" | "1" => [false, true, false],
        "z" | "2" => [false, false, true],
        "xy" | "yx" => [true, true, false],
        "yz" | "zy" => [false, true, true],
        "zx" | "xz" => [true, false, true],
        "all" | "xyz" => [true, true, true],
        _ => return None,
    })
}

/// The axis word for `axes` (inverse of [`center_axes`]).
pub fn center_axes_word(axes: [bool; 3]) -> &'static str {
    match axes {
        [true, false, false] => "x",
        [false, true, false] => "y",
        [false, false, true] => "z",
        [true, true, false] => "xy",
        [false, true, true] => "yz",
        [true, false, true] => "zx",
        _ => "all",
    }
}

// ---------------------------------------------------------------------------
// Editor commands
// ---------------------------------------------------------------------------

impl crate::Editor {
    /// Polygons a Subdivide or UnSubdivide works on (SUB-08/09): the
    /// selected ones in Polygon mode (all when none), every polygon of the
    /// active mesh in Item mode (UnSubdivide; Subdivide covers every selected
    /// mesh item, `subdivide_items`); refused in Vertex and Edge mode.
    fn subdivide_target(&self, what: &str) -> Result<(u64, Vec<u32>)> {
        let item = self.edge_insert_target(what)?;
        match self.scene.selection.mode {
            Mode::Polygon => Ok((item, self.scene.selection.faces.clone())),
            Mode::Item => Ok((item, Vec::new())),
            Mode::Vertex | Mode::Edge => Err(NO_POLYGONS_SELECTED.into()),
        }
    }

    /// Publish a topology change of the active mesh as one step, selecting
    /// `faces` (SUB-15).
    pub(crate) fn publish_polygon_edit(
        &mut self,
        item: u64,
        mesh: Mesh,
        label: &str,
        faces: Vec<u32>,
    ) -> Result<()> {
        let before = self.scene.clone();
        self.publish_polygon_edit_from(item, mesh, label, faces, before)
    }

    /// [`Self::publish_polygon_edit`] with the step's "before" scene given,
    /// for a command that changed the scene first in the same step (Stencil
    /// adding its material). On error the scene is `before` again.
    pub(crate) fn publish_polygon_edit_from(
        &mut self,
        item: u64,
        mesh: Mesh,
        label: &str,
        faces: Vec<u32>,
        before: crate::Scene,
    ) -> Result<()> {
        let ids_before = self.identities.current.clone();
        self.scene.active_mut().ok_or("Select a mesh")?.mesh = mesh.into();
        let (edges, points) = self
            .scene
            .active()
            .map(|i| {
                (
                    i.mesh.edges().into_iter().collect::<BTreeSet<_>>(),
                    i.mesh.vertices.len() as u32,
                )
            })
            .unwrap_or_default();
        let selection = &mut self.scene.selection;
        selection
            .edges
            .retain(|&(x, y)| edges.contains(&edge_key(x, y)));
        selection.vertices.retain(|&v| v < points);
        selection.faces = faces;
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.invalidate_mesh(&self.scene, item))
        {
            self.scene = before;
            self.identities.current = ids_before;
            return Err(e);
        }
        self.record(label.into(), before);
        Ok(())
    }

    /// SUB-05..15: Subdivide Polygons, `times` passes as one undo step
    /// (Alt+click SDS Subdivide runs two, SUB-06). The options become the
    /// remembered ones (SUB-05).
    pub(crate) fn subdivide_polygons(
        &mut self,
        options: SubdivideOptions,
        times: u32,
    ) -> Result<()> {
        options.validate()?;
        if !(1..=4).contains(&times) {
            return Err("Subdivide runs 1 to 4 times at once".into());
        }
        if self.scene.selection.mode == Mode::Item {
            return self.subdivide_items(options, times);
        }
        self.check_subdivide_memory(&options, times)?;
        let (item, mut selected) = self.subdivide_target("Subdivide")?;
        let had_selection = !selected.is_empty();
        let mut mesh: Mesh = Mesh::clone(&self.scene.active().ok_or("Select a mesh")?.mesh);
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64)
            ^ self.history_serial.rotate_left(17);
        for pass in 0..times {
            let outcome =
                subdivide_polygons(&mesh, &selected, &options, seed.wrapping_add(pass as u64))?;
            mesh = outcome.mesh;
            selected = if had_selection {
                outcome.new_faces
            } else {
                Vec::new()
            };
        }
        self.subdivide_preset = options;
        self.publish_polygon_edit(item, mesh, "Subdivide Polygons", selected)
    }

    /// Item mode's Subdivide targets: every selected, visible mesh item
    /// with polygons (cameras, lights, backdrops are skipped).
    fn subdivide_item_targets(&self) -> Result<Vec<u64>> {
        self.edge_insert_target("Subdivide")?;
        let targets: Vec<u64> = self
            .scene
            .selected_items()
            .into_iter()
            .filter(|id| {
                self.scene.items.iter().any(|i| {
                    i.id == *id
                        && self.scene.shows(i)
                        && i.backdrop.is_none()
                        && (0..i.mesh.faces.len()).any(|f| !i.mesh.is_polyline(f))
                })
            })
            .collect();
        if targets.is_empty() {
            return Err("Select a mesh item with polygons to subdivide".into());
        }
        Ok(targets)
    }

    /// What Subdivide with `options`, `times` passes, would make now: the
    /// polygons of the meshes it changes afterwards, exactly, and the memory
    /// those polygons need ([`BYTES_PER_POLYGON`] each).
    pub fn subdivide_estimate(
        &self,
        options: &SubdivideOptions,
        times: u32,
    ) -> Result<SubdivideEstimate> {
        let polygons = if self.scene.selection.mode == Mode::Item {
            self.subdivide_item_targets()?
                .iter()
                .filter_map(|id| self.scene.item(*id))
                .map(|i| subdivided_polygon_count(&i.mesh, &[], options.method, times))
                .sum()
        } else {
            let (_, selected) = self.subdivide_target("Subdivide")?;
            let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
            subdivided_polygon_count(mesh, &selected, options.method, times)
        };
        Ok(SubdivideEstimate {
            polygons,
            bytes: polygons.saturating_mul(BYTES_PER_POLYGON),
        })
    }

    /// Refuse a Subdivide whose result needs more memory than is free
    /// (Jason, 2026-11-12, in place of the fixed 8 million cap). Without a
    /// free-memory source (`Editor::free_memory`) nothing is checked.
    fn check_subdivide_memory(&self, options: &SubdivideOptions, times: u32) -> Result<()> {
        let crate::FreeMemory(Some(free_memory)) = self.free_memory else {
            return Ok(());
        };
        let estimate = self.subdivide_estimate(options, times)?;
        let free = free_memory();
        if estimate.bytes > free {
            return Err(format!(
                "Subdivide would make {} polygons and needs about {} of memory, but only {} is free; nothing changed",
                group_thousands(estimate.polygons),
                gigabytes(estimate.bytes),
                gigabytes(free)
            ));
        }
        Ok(())
    }

    /// SUB-03 (corrected 2026-11-08): in Item mode, every polygon of every
    /// selected, visible mesh item, as one undo step. Items without
    /// polygons (cameras, lights, backdrops) are skipped.
    fn subdivide_items(&mut self, options: SubdivideOptions, times: u32) -> Result<()> {
        let targets = self.subdivide_item_targets()?;
        self.check_subdivide_memory(&options, times)?;
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64)
            ^ self.history_serial.rotate_left(17);
        let before = self.scene.clone();
        let ids_before = self.identities.current.clone();
        let result = (|| -> Result<()> {
            // Remembered component selections name the old topology.
            let selection = &mut self.scene.selection;
            selection.others.clear();
            selection.vertices.clear();
            selection.edges.clear();
            selection.faces.clear();
            // One item at a time, its identities renewed before the next,
            // so every step is a consistent scene.
            for (k, &id) in targets.iter().enumerate() {
                let item = self.scene.item_mut(id).ok_or("Missing mesh")?;
                let mut mesh = Mesh::clone(&item.mesh);
                for pass in 0..times {
                    let salt = (k as u64) << 8 | pass as u64;
                    mesh = subdivide_polygons(&mesh, &[], &options, seed.wrapping_add(salt))?.mesh;
                }
                item.mesh = mesh.into();
                self.scene.validate()?;
                self.identities.invalidate_mesh(&self.scene, id)?;
            }
            Ok(())
        })();
        if let Err(e) = result {
            self.scene = before;
            self.identities.current = ids_before;
            return Err(e);
        }
        self.subdivide_preset = options;
        self.record("Subdivide Polygons".into(), before);
        Ok(())
    }

    /// SUB-40..44 UnSubdivide as one undo step. Iterations 0 does nothing.
    pub(crate) fn unsubdivide(&mut self, options: UnsubdivideOptions) -> Result<()> {
        let (item, selected) = self.subdivide_target("UnSubdivide")?;
        self.unsubdivide_preset = options.clone();
        if options.iterations == 0 {
            return Ok(());
        }
        let had_selection = !selected.is_empty();
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let (out, merged) = unsubdivide_polygons(mesh, &selected, &options)?;
        self.publish_polygon_edit(
            item,
            out,
            "UnSubdivide",
            if had_selection { merged } else { Vec::new() },
        )
    }

    /// CEN-01..04 Center on the chosen world axes, one undo step.
    pub(crate) fn center_selection(&mut self, axes: [bool; 3]) -> Result<()> {
        let item = self.edge_insert_target("Center")?;
        let selection = &self.scene.selection;
        let points: Vec<u32> = match selection.mode {
            Mode::Item => {
                return Err(
                    "Center works on vertices, edges or polygons; switch to a component mode"
                        .into(),
                );
            }
            Mode::Vertex => selection.vertices.clone(),
            Mode::Edge => selection.edges.iter().flat_map(|&(a, b)| [a, b]).collect(),
            Mode::Polygon => {
                let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
                selection
                    .faces
                    .iter()
                    .filter_map(|&f| mesh.faces.get(f as usize))
                    .flat_map(|f| f.vertices.iter().copied())
                    .collect()
            }
        };
        let world = self.scene.world_matrix(item)?;
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let out = center_points(mesh, world, &points, axes)?;
        if out == **mesh {
            return Ok(());
        }
        let before = self.scene.clone();
        self.scene.active_mut().ok_or("Select a mesh")?.mesh = out.into();
        if let Err(e) = self.scene.validate() {
            self.scene = before;
            return Err(e);
        }
        self.record("Center".into(), before);
        Ok(())
    }
}
