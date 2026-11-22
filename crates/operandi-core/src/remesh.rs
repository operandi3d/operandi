//! Remesh: rebuild a mesh's whole surface as new, even polygons.
//!
//! Three commands (names from the registry thread, 2026-11-11):
//!
//! - `mesh.retopo` (Automatic Retopology): all quads that follow the
//!   surface's flow, through QuadriFlow (`operandi-remesh`).
//! - `sculpt.remesh` (Voxel Remesh): an even all-quad skin on a voxel grid
//!   (`remesh_voxel.rs`); joins overlapping pieces and closes holes, the
//!   remesh sculpting uses.
//! - `operandi.remesh` (Blocks / Smooth / Sharp): the same grid engine with
//!   octree sizing; corners stay sharp in Sharp (plane fit).
//!
//! Each works on the whole active mesh, is one undo step, and its left-out
//! arguments keep their last values. The work is a pure function of the
//! mesh and the options ([`remesh_mesh`]), so the app can run it on a
//! worker thread with progress and Cancel, then hand the finished mesh to
//! the very same command ([`RemeshPrepared`]).
use crate::{Face, Mesh, Result};
use glam::DVec3;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

/// How `mesh.retopo` picks its polygon count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CountMode {
    /// `polyTargetNum` polygons.
    #[default]
    Number,
    /// `polyTargetPct` of the mesh's polygon count.
    Percent,
}

/// `mesh.retopo` (Automatic Retopology) options.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RetopoOptions {
    pub count_mode: CountMode,
    /// `polyTargetNum`: about how many quads.
    pub target_count: u32,
    /// `polyTargetPct`: about this share (0..1) of the input's polygons.
    pub target_percent: f64,
    /// `creaseAngle`, radians: edges whose polygons meet more sharply than
    /// this are followed by the new edges; 0 is off.
    pub crease_angle: f64,
    /// `boundaryAlign`: new edges follow open borders.
    pub boundary_align: bool,
    /// `vertNormals`: the new polygons are smooth shaded.
    pub vert_normals: bool,
    /// `newMesh`: put the result in a new mesh item next to this one,
    /// which is left as it was.
    pub new_mesh: bool,
    /// Operandi-only `seed`: the same seed gives the same quads.
    pub seed: i32,
}

impl Default for RetopoOptions {
    fn default() -> Self {
        Self {
            count_mode: CountMode::Number,
            target_count: 5000,
            target_percent: 0.25,
            crease_angle: 0.0,
            boundary_align: true,
            vert_normals: true,
            new_mesh: false,
            seed: 0,
        }
    }
}

impl RetopoOptions {
    pub fn validate(&self) -> Result<()> {
        if !(1..=1_000_000).contains(&self.target_count) {
            return Err("Polygon Target must be from 1 to 1,000,000".into());
        }
        if !(self.target_percent.is_finite()
            && self.target_percent > 0.0
            && self.target_percent <= 10.0)
        {
            return Err("Relative Polygon Target must be above 0 % and at most 1000 %".into());
        }
        if !(self.crease_angle.is_finite()
            && (0.0..=std::f64::consts::PI).contains(&self.crease_angle))
        {
            return Err("Crease Angle must be from 0° to 180°".into());
        }
        Ok(())
    }

    /// The quad count to ask for, for a mesh of `polygons` polygons.
    pub fn target(&self, polygons: usize) -> u32 {
        match self.count_mode {
            CountMode::Number => self.target_count,
            CountMode::Percent => ((polygons as f64 * self.target_percent).round() as u32).max(1),
        }
    }
}

/// `sculpt.remesh` (Voxel Remesh) options.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VoxelOptions {
    /// `density`, 0..1: higher is finer (see [`VoxelOptions::voxel_size`]).
    pub density: f64,
    /// Operandi-only `voxelSize`, world units: when above 0 it sets the
    /// voxel size directly and Density is not used.
    pub voxel_size: f64,
    /// Operandi-only `keepMaps`: weight and colour maps (sculpt masks
    /// among them) carry over from the nearest old point.
    pub keep_maps: bool,
    /// `vertNormals` (Operandi-only here): smooth shaded result.
    pub vert_normals: bool,
}

impl Default for VoxelOptions {
    fn default() -> Self {
        Self {
            density: 0.5,
            voxel_size: 0.0,
            keep_maps: true,
            vert_normals: true,
        }
    }
}

/// Voxels along the longest side at Density 0 and at Density 1; between,
/// they grow geometrically (provisional, REM-Q01).
pub const DENSITY_CELLS: (f64, f64) = (16.0, 640.0);

impl VoxelOptions {
    pub fn validate(&self) -> Result<()> {
        if !(self.density.is_finite() && (0.0..=1.0).contains(&self.density)) {
            return Err("Density must be from 0 % to 100 %".into());
        }
        if !(self.voxel_size.is_finite() && self.voxel_size >= 0.0) {
            return Err("Voxel Size must be 0 (use Density) or above".into());
        }
        Ok(())
    }

    /// The voxel size in world units for a mesh whose longest world-space
    /// side is `longest`.
    pub fn voxel_size(&self, longest: f64) -> f64 {
        if self.voxel_size > 0.0 {
            return self.voxel_size;
        }
        let (lo, hi) = DENSITY_CELLS;
        longest / (lo * (hi / lo).powf(self.density))
    }
}

/// `operandi.remesh` modes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OctreeMode {
    /// Cubes on the octree grid.
    Blocks,
    /// A smooth surface.
    #[default]
    Smooth,
    /// Keeps sharp edges and corners.
    Sharp,
}

impl OctreeMode {
    pub const ALL: [OctreeMode; 3] = [OctreeMode::Blocks, OctreeMode::Smooth, OctreeMode::Sharp];
    pub fn word(self) -> &'static str {
        match self {
            OctreeMode::Blocks => "blocks",
            OctreeMode::Smooth => "smooth",
            OctreeMode::Sharp => "sharp",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            OctreeMode::Blocks => "Blocks",
            OctreeMode::Smooth => "Smooth",
            OctreeMode::Sharp => "Sharp",
        }
    }
}

/// `operandi.remesh` (Blocks / Smooth / Sharp) options.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OctreeOptions {
    pub mode: OctreeMode,
    /// `depth`, 2..12: the grid has 2^depth cells along the longest side.
    pub depth: u32,
    /// `scale`, 0..1: how much of the grid the mesh fills; lower is
    /// coarser.
    pub scale: f64,
    /// `sharpness` (Sharp): higher keeps more edges sharp.
    pub sharpness: f64,
    /// `removeDisconnected`: drop small loose pieces.
    pub remove_disconnected: bool,
    /// `threshold`, 0..1: with Remove Disconnected, pieces smaller than
    /// this share of the biggest are dropped.
    pub threshold: f64,
    /// `vertNormals` (Operandi-only here): smooth shaded result.
    pub vert_normals: bool,
}

impl Default for OctreeOptions {
    fn default() -> Self {
        Self {
            mode: OctreeMode::Smooth,
            depth: 6,
            scale: 0.9,
            sharpness: 1.0,
            remove_disconnected: false,
            threshold: 1.0,
            vert_normals: true,
        }
    }
}

impl OctreeOptions {
    pub fn validate(&self) -> Result<()> {
        if !(2..=12).contains(&self.depth) {
            return Err("Octree Depth must be from 2 to 12".into());
        }
        if !(self.scale.is_finite() && self.scale > 0.0 && self.scale < 1.0) {
            return Err("Scale must be above 0 % and below 100 %".into());
        }
        if !(self.sharpness.is_finite() && (0.0..=1000.0).contains(&self.sharpness)) {
            return Err("Sharpness must be from 0 to 1000".into());
        }
        if !(self.threshold.is_finite() && (0.0..=1.0).contains(&self.threshold)) {
            return Err("Threshold must be from 0 % to 100 %".into());
        }
        Ok(())
    }
}

/// Which remesh, with its options.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RemeshOp {
    Retopo(RetopoOptions),
    Voxel(VoxelOptions),
    Octree(OctreeOptions),
}

impl RemeshOp {
    pub fn label(&self) -> &'static str {
        match self {
            RemeshOp::Retopo(_) => "Automatic Retopology",
            RemeshOp::Voxel(_) => "Voxel Remesh",
            RemeshOp::Octree(o) => match o.mode {
                OctreeMode::Blocks => "Remesh Blocks",
                OctreeMode::Smooth => "Remesh Smooth",
                OctreeMode::Sharp => "Remesh Sharp",
            },
        }
    }
    pub fn validate(&self) -> Result<()> {
        match self {
            RemeshOp::Retopo(o) => o.validate(),
            RemeshOp::Voxel(o) => o.validate(),
            RemeshOp::Octree(o) => o.validate(),
        }
    }
    fn smooth(&self) -> bool {
        match self {
            RemeshOp::Retopo(o) => o.vert_normals,
            RemeshOp::Voxel(o) => o.vert_normals,
            RemeshOp::Octree(o) => o.vert_normals,
        }
    }
}

/// A remesh worked out ahead (on a worker thread) for one mesh state, so
/// the command can publish it without doing the work again. The command
/// uses it only when `revision`, `item` and `op` all match; otherwise it
/// computes the result itself, so both paths give the same mesh.
#[derive(Clone, Debug, PartialEq)]
pub struct RemeshPrepared {
    pub revision: u64,
    pub item: u64,
    pub op: RemeshOp,
    pub mesh: Mesh,
}

/// The polygons of `mesh` as triangles over its points. Lines are left
/// out; a polygon that can't be ear-clipped falls back to a fan.
pub fn mesh_triangles(mesh: &Mesh) -> Vec<[u32; 3]> {
    let mut out = Vec::with_capacity(mesh.faces.len() * 2);
    for (fi, f) in mesh.faces.iter().enumerate() {
        if mesh.is_polyline(fi) || f.vertices.len() < 3 {
            continue;
        }
        let v = &f.vertices;
        if v.len() == 3 {
            out.push([v[0], v[1], v[2]]);
            continue;
        }
        match mesh.surface_triangles(fi) {
            Ok(tris) => out.extend(tris.iter().map(|t| t.map(|c| v[c]))),
            Err(_) => out.extend((1..v.len() - 1).map(|i| [v[0], v[i], v[i + 1]])),
        }
    }
    out
}

/// Runs `op` on `mesh` (in its own space; `world_scale` turns world
/// distances such as Voxel Size into it). `cancel` stops QuadriFlow between
/// stages; `report` gets progress from 0 to 1.
pub fn remesh_mesh(
    mesh: &Mesh,
    op: &RemeshOp,
    world_scale: f64,
    cancel: &AtomicBool,
    report: &mut dyn FnMut(f32),
) -> Result<Mesh> {
    op.validate()?;
    let tris = mesh_triangles(mesh);
    if tris.is_empty() {
        return Err("The mesh has no polygons to remesh".into());
    }
    let (points, quads): (Vec<DVec3>, Vec<Vec<u32>>) = match op {
        RemeshOp::Voxel(o) => {
            let (lo, hi) = bounds(&mesh.vertices, &tris);
            let longest = (hi - lo).max_element() * world_scale.max(1e-12);
            let h = o.voxel_size(longest) / world_scale.max(1e-12);
            report(0.1);
            let r = crate::remesh_voxel::voxel_remesh(&mesh.vertices, &tris, h)?;
            (r.points, r.quads.iter().map(|q| q.to_vec()).collect())
        }
        RemeshOp::Octree(o) => {
            // Operandi's own grid engine (remesh_voxel.rs) with the
            // octree's sizing: 2^depth cells along the longest side of a
            // cube the mesh fills to `scale`. (Blender's dualcon gave broken
            // surfaces here, REM-Q05, so it isn't used.)
            let (lo, hi) = bounds(&mesh.vertices, &tris);
            let h = (hi - lo).max_element() / o.scale / 2f64.powi(o.depth as i32);
            let placement = match o.mode {
                OctreeMode::Blocks => crate::remesh_voxel::Placement::Blocks,
                OctreeMode::Smooth => crate::remesh_voxel::Placement::Smooth,
                OctreeMode::Sharp => crate::remesh_voxel::Placement::Sharp {
                    sharpness: o.sharpness,
                },
            };
            report(0.1);
            let r = crate::remesh_voxel::voxel_remesh_with(&mesh.vertices, &tris, h, placement)?;
            let mut quads: Vec<Vec<u32>> = r.quads.iter().map(|q| q.to_vec()).collect();
            if o.remove_disconnected {
                drop_small_pieces(&mut quads, r.points.len(), o.threshold);
            }
            (r.points, quads)
        }
        RemeshOp::Retopo(o) => {
            let tris = clean_triangles(mesh, &tris);
            check_retopo_input(&tris)?;
            let (pts, tris) = f32_input(mesh, &tris);
            let polygons = mesh
                .faces
                .iter()
                .enumerate()
                .filter(|(fi, _)| !mesh.is_polyline(*fi))
                .count();
            let r = operandi_remesh::quadriflow(
                &pts,
                &tris,
                &operandi_remesh::QuadriflowOptions {
                    target_faces: o.target(polygons),
                    crease_angle: o.crease_angle as f32,
                    preserve_boundary: o.boundary_align,
                    adaptive_scale: false,
                    minimum_cost_flow: false,
                    seed: o.seed,
                },
                cancel,
                report,
            )
            .map_err(|e| e.to_string())?;
            from_f32(r)
        }
    };
    if cancel.load(Ordering::Relaxed) {
        return Err("Remesh canceled; nothing changed".into());
    }
    report(0.95);
    let mut out = build_mesh(points, quads, mesh, op.smooth())?;
    if let RemeshOp::Voxel(o) = op
        && o.keep_maps
    {
        carry_point_maps(mesh, &mut out);
    }
    out.validate()?;
    report(1.0);
    Ok(out)
}

fn bounds(points: &[DVec3], tris: &[[u32; 3]]) -> (DVec3, DVec3) {
    let mut lo = DVec3::splat(f64::INFINITY);
    let mut hi = DVec3::splat(f64::NEG_INFINITY);
    for &i in tris.iter().flatten() {
        lo = lo.min(points[i as usize]);
        hi = hi.max(points[i as usize]);
    }
    (lo, hi)
}

fn f32_input(mesh: &Mesh, tris: &[[u32; 3]]) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let pts = mesh
        .vertices
        .iter()
        .map(|p| [p.x as f32, p.y as f32, p.z as f32])
        .collect();
    (pts, tris.to_vec())
}

fn from_f32(r: operandi_remesh::QuadMesh) -> (Vec<DVec3>, Vec<Vec<u32>>) {
    (
        r.points
            .iter()
            .map(|p| DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64))
            .collect(),
        r.quads.iter().map(|q| q.to_vec()).collect(),
    )
}

/// Remove Disconnected: keeps the pieces (joined through shared points)
/// with at least `threshold` of the biggest piece's polygons.
fn drop_small_pieces(polys: &mut Vec<Vec<u32>>, points: usize, threshold: f64) {
    let mut parent: Vec<u32> = (0..points as u32).collect();
    fn find(p: &mut [u32], mut x: u32) -> u32 {
        while p[x as usize] != x {
            p[x as usize] = p[p[x as usize] as usize];
            x = p[x as usize];
        }
        x
    }
    for f in polys.iter() {
        for w in f.windows(2) {
            let (a, b) = (find(&mut parent, w[0]), find(&mut parent, w[1]));
            parent[a as usize] = b;
        }
    }
    let mut size: crate::fxhash::FxHashMap<u32, usize> = Default::default();
    let roots: Vec<u32> = polys.iter().map(|f| find(&mut parent, f[0])).collect();
    for &r in &roots {
        *size.entry(r).or_default() += 1;
    }
    let biggest = size.values().copied().max().unwrap_or(0) as f64;
    let mut k = 0;
    polys.retain(|_| {
        let keep = size[&roots[k]] as f64 >= threshold * biggest;
        k += 1;
        keep
    });
}

/// Drops triangles with no area (QuadriFlow can't give them a normal).
fn clean_triangles(mesh: &Mesh, tris: &[[u32; 3]]) -> Vec<[u32; 3]> {
    tris.iter()
        .copied()
        .filter(|t| {
            let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
            (b - a).cross(c - a).length_squared() > 0.0
        })
        .collect()
}

/// Automatic Retopology needs a clean surface: every edge on at most two
/// triangles, facing one way, and no point where separate fans meet.
fn check_retopo_input(tris: &[[u32; 3]]) -> Result<()> {
    if tris.is_empty() {
        return Err("The mesh has no polygons with area to retopologize".into());
    }
    let mut uses: crate::fxhash::FxHashMap<(u32, u32), (u32, i32)> = Default::default();
    for t in tris {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            let e = uses.entry((a.min(b), a.max(b))).or_default();
            e.0 += 1;
            e.1 += if a < b { 1 } else { -1 };
        }
    }
    if uses.values().any(|&(n, s)| n > 2 || (n == 2 && s != 0)) {
        return Err("Automatic Retopology needs a clean surface: an edge has more than two polygons or they face opposite ways. Try Mesh Cleanup or Voxel Remesh first".into());
    }
    // Each point's triangles must form one fan: corners of a point joined
    // across shared edges all end up in one group.
    let mut parent: Vec<u32> = (0..tris.len() as u32 * 3).collect();
    fn find(p: &mut [u32], mut x: u32) -> u32 {
        while p[x as usize] != x {
            p[x as usize] = p[p[x as usize] as usize];
            x = p[x as usize];
        }
        x
    }
    // Corner (t, k) of point p links with the corner of p in the triangle
    // across each of its two edges.
    let mut edge_corner: crate::fxhash::FxHashMap<(u32, u32, u32), u32> = Default::default();
    for (ti, t) in tris.iter().enumerate() {
        for k in 0..3 {
            let p = t[k];
            let corner = ti as u32 * 3 + k as u32;
            for other in [t[(k + 1) % 3], t[(k + 2) % 3]] {
                let key = (p, p.min(other), p.max(other));
                if let Some(&c) = edge_corner.get(&key) {
                    let (ra, rb) = (find(&mut parent, c), find(&mut parent, corner));
                    parent[ra as usize] = rb;
                } else {
                    edge_corner.insert(key, corner);
                }
            }
        }
    }
    let mut roots: crate::fxhash::FxHashMap<u32, u32> = Default::default();
    for (ti, t) in tris.iter().enumerate() {
        for (k, &p) in t.iter().enumerate() {
            let r = find(&mut parent, ti as u32 * 3 + k as u32);
            let first = *roots.entry(p).or_insert(r);
            if first != r {
                return Err("Automatic Retopology needs a clean surface: separate parts touch at a single point. Try Voxel Remesh first".into());
            }
        }
    }
    Ok(())
}

/// Turns back-end polygons into an Operandi mesh: repeated corners
/// removed, polygons with no area dropped, and any edge with more than two
/// polygons (or two facing the same way) relieved by dropping the later
/// ones. Points no polygon uses are left out. Material and UV maps follow
/// `template` (its most used material; UV maps kept by name, empty).
fn build_mesh(
    points: Vec<DVec3>,
    polys: Vec<Vec<u32>>,
    template: &Mesh,
    smooth: bool,
) -> Result<Mesh> {
    let material = most_used_material(template);
    let mut faces: Vec<Vec<u32>> = Vec::with_capacity(polys.len());
    for mut p in polys {
        p.dedup();
        while p.len() > 1 && p.first() == p.last() {
            p.pop();
        }
        if p.len() < 3 || p.iter().any(|&i| i as usize >= points.len()) {
            continue;
        }
        let mut seen = p.clone();
        seen.sort_unstable();
        seen.dedup();
        if seen.len() != p.len() {
            continue;
        }
        faces.push(p);
    }
    // Area check with the same normal the mesh uses.
    let area_ok = |f: &[u32]| {
        let o = points[f[0] as usize];
        let mut n = DVec3::ZERO;
        for i in 0..f.len() {
            n += (points[f[i] as usize] - o).cross(points[f[(i + 1) % f.len()] as usize] - o);
        }
        n.normalize_or_zero() != DVec3::ZERO
    };
    faces.retain(|f| area_ok(f));
    // Edges: keep a polygon only while each of its edges has at most one
    // earlier polygon, running the other way.
    let mut used: crate::fxhash::FxHashMap<(u32, u32), (u8, bool)> = Default::default();
    faces.retain(|f| {
        let ok = (0..f.len()).all(|i| {
            let (a, b) = (f[i], f[(i + 1) % f.len()]);
            match used.get(&(a.min(b), a.max(b))) {
                None => true,
                Some(&(n, forward)) => n == 1 && forward != (a < b),
            }
        });
        if ok {
            for i in 0..f.len() {
                let (a, b) = (f[i], f[(i + 1) % f.len()]);
                let e = used.entry((a.min(b), a.max(b))).or_insert((0, a < b));
                e.0 += 1;
            }
        }
        ok
    });
    if faces.is_empty() {
        return Err("Remesh made no polygons; try finer settings".into());
    }
    if faces.len() > 5_000_000 {
        return Err("Remesh would make more than 5 million polygons; use coarser settings".into());
    }
    let mut remap = vec![u32::MAX; points.len()];
    let mut vertices = Vec::new();
    for f in &mut faces {
        for i in f.iter_mut() {
            if remap[*i as usize] == u32::MAX {
                remap[*i as usize] = vertices.len() as u32;
                vertices.push(points[*i as usize]);
            }
            *i = remap[*i as usize];
        }
    }
    let uv_maps: BTreeMap<String, Vec<Vec<Option<glam::DVec2>>>> = template
        .uv_maps
        .keys()
        .map(|k| {
            (
                k.clone(),
                faces.iter().map(|f| vec![None; f.len()]).collect(),
            )
        })
        .collect();
    Ok(Mesh {
        attributes: Default::default(),
        vertices,
        faces: faces
            .into_iter()
            .map(|vertices| Face {
                vertices,
                uv: None,
                material: material.clone(),
                smooth,
                part: None,
                material_label: None,
            })
            .collect(),
        uv_maps: uv_maps.into(),
    })
}

fn most_used_material(mesh: &Mesh) -> String {
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    for f in &mesh.faces {
        *count.entry(f.material.as_str()).or_default() += 1;
    }
    count
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map(|(m, _)| m.to_owned())
        .unwrap_or_default()
}

/// Copies point weight and colour maps from `from` to `to` by nearest
/// point (sculpt masks are weight maps, so they survive a remesh).
fn carry_point_maps(from: &Mesh, to: &mut Mesh) {
    let a = &from.attributes;
    if a.point_weights.is_empty() && a.point_colors.is_empty() {
        return;
    }
    let near = Nearest::new(&from.vertices);
    let nearest: Vec<u32> = to.vertices.iter().map(|&p| near.find(p)).collect();
    for (name, map) in &a.point_weights {
        let out = nearest
            .iter()
            .enumerate()
            .filter_map(|(i, &n)| map.get(&n).map(|&w| (i as u32, w)))
            .collect();
        to.attributes.point_weights.insert(name.clone(), out);
    }
    for (name, map) in &a.point_colors {
        let out = nearest
            .iter()
            .enumerate()
            .filter_map(|(i, &n)| map.get(&n).map(|&c| (i as u32, c)))
            .collect();
        to.attributes.point_colors.insert(name.clone(), out);
    }
}

/// Nearest-point lookup on a uniform grid of buckets.
struct Nearest<'a> {
    points: &'a [DVec3],
    lo: DVec3,
    cell: f64,
    n: [i64; 3],
    buckets: crate::fxhash::FxHashMap<(i64, i64, i64), Vec<u32>>,
}

impl<'a> Nearest<'a> {
    fn new(points: &'a [DVec3]) -> Self {
        let mut lo = DVec3::splat(f64::INFINITY);
        let mut hi = DVec3::splat(f64::NEG_INFINITY);
        for &p in points {
            lo = lo.min(p);
            hi = hi.max(p);
        }
        let span = (hi - lo).max(DVec3::splat(1e-9));
        // About two points per bucket on a surface.
        let cell = (span.x * span.y + span.y * span.z + span.z * span.x)
            .max(1e-18)
            .sqrt()
            / (points.len() as f64 / 2.0).sqrt().max(1.0);
        let cell = cell.max(span.max_element() / 4096.0);
        let n = [0, 1, 2].map(|a| (span[a] / cell).floor() as i64 + 1);
        let mut buckets: crate::fxhash::FxHashMap<(i64, i64, i64), Vec<u32>> = Default::default();
        let mut near = Nearest {
            points,
            lo,
            cell,
            n,
            buckets: Default::default(),
        };
        for (i, &p) in points.iter().enumerate() {
            buckets.entry(near.key(p)).or_default().push(i as u32);
        }
        near.buckets = buckets;
        near
    }
    fn key(&self, p: DVec3) -> (i64, i64, i64) {
        let k =
            |a: usize| (((p[a] - self.lo[a]) / self.cell).floor() as i64).clamp(0, self.n[a] - 1);
        (k(0), k(1), k(2))
    }
    fn find(&self, p: DVec3) -> u32 {
        let (x, y, z) = self.key(p);
        let mut best = (f64::INFINITY, 0u32);
        let max_ring = self.n.iter().copied().max().unwrap_or(1);
        for r in 0..=max_ring {
            // Once a point is found, one more ring settles it.
            if best.0.is_finite() && (r as f64 - 1.0) * self.cell > best.0.sqrt() {
                break;
            }
            for i in -r..=r {
                for j in -r..=r {
                    for k in -r..=r {
                        if i.abs().max(j.abs()).max(k.abs()) != r {
                            continue;
                        }
                        if let Some(b) = self.buckets.get(&(x + i, y + j, z + k)) {
                            for &q in b {
                                let d = self.points[q as usize].distance_squared(p);
                                if d < best.0 {
                                    best = (d, q);
                                }
                            }
                        }
                    }
                }
            }
        }
        best.1
    }
}

/// Command arguments, over `base` (the last-used values).
pub mod args {
    use super::*;
    type Args = serde_json::Map<String, serde_json::Value>;

    fn num(a: &Args, k: &str, v: &mut f64) -> Result<()> {
        if let Some(x) = a.get(k).filter(|x| !x.is_null()) {
            *v = crate::command_exec::num(x)?;
        }
        Ok(())
    }
    fn uint(a: &Args, k: &str, v: &mut u32) -> Result<()> {
        if let Some(x) = a.get(k).filter(|x| !x.is_null()) {
            *v = crate::command_exec::as_u32(x)?;
        }
        Ok(())
    }
    fn flag(a: &Args, k: &str, v: &mut bool) -> Result<()> {
        if let Some(b) = crate::command_exec::opt_bool(a, k)? {
            *v = b;
        }
        Ok(())
    }
    fn choice(a: &Args, k: &str, words: &[&str]) -> Result<Option<usize>> {
        match a.get(k) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(serde_json::Value::Number(n)) => n
                .as_u64()
                .map(|i| i as usize)
                .filter(|&i| i < words.len())
                .map(Some)
                .ok_or_else(|| format!("{k} must be 0 to {}", words.len() - 1)),
            Some(serde_json::Value::String(w)) => words
                .iter()
                .position(|x| x.eq_ignore_ascii_case(w))
                .or_else(|| w.parse::<usize>().ok().filter(|&i| i < words.len()))
                .map(Some)
                .ok_or_else(|| format!("{k} must be one of {}", words.join(", "))),
            Some(_) => Err(format!("{k} must be a word or a number")),
        }
    }
    /// The reference arguments `mesh.retopo` has but the back end can't do:
    /// refused when turned on rather than quietly ignored.
    fn refuse(a: &Args, k: &str, what: &str) -> Result<()> {
        if crate::command_exec::opt_bool(a, k)? == Some(true) {
            return Err(format!("Automatic Retopology can't do {what} yet"));
        }
        Ok(())
    }

    pub fn retopo(a: &Args, base: &RetopoOptions) -> Result<RetopoOptions> {
        let mut o = base.clone();
        if let Some(i) = choice(a, "countMode", &["number", "percent"])? {
            o.count_mode = [CountMode::Number, CountMode::Percent][i];
        }
        uint(a, "polyTargetNum", &mut o.target_count)?;
        num(a, "polyTargetPct", &mut o.target_percent)?;
        num(a, "creaseAngle", &mut o.crease_angle)?;
        flag(a, "boundaryAlign", &mut o.boundary_align)?;
        flag(a, "vertNormals", &mut o.vert_normals)?;
        flag(a, "newMesh", &mut o.new_mesh)?;
        if let Some(x) = a.get("seed").filter(|x| !x.is_null()) {
            let s = crate::command_exec::num(x)?;
            if s.fract() != 0.0 || s.abs() > i32::MAX as f64 {
                return Err("seed must be a whole number".into());
            }
            o.seed = s as i32;
        }
        refuse(a, "enableSymmetry", "symmetry")?;
        // QuadriFlow always makes quads only.
        if crate::command_exec::opt_bool(a, "quadsOnly")? == Some(false) {
            return Err("Automatic Retopology always makes quads".into());
        }
        o.validate()?;
        Ok(o)
    }

    pub fn voxel(a: &Args, base: &VoxelOptions) -> Result<VoxelOptions> {
        let mut o = base.clone();
        num(a, "density", &mut o.density)?;
        num(a, "voxelSize", &mut o.voxel_size)?;
        flag(a, "keepMaps", &mut o.keep_maps)?;
        flag(a, "vertNormals", &mut o.vert_normals)?;
        o.validate()?;
        Ok(o)
    }

    pub fn octree(a: &Args, base: &OctreeOptions) -> Result<OctreeOptions> {
        let mut o = base.clone();
        if let Some(i) = choice(a, "mode", &["blocks", "smooth", "sharp"])? {
            o.mode = OctreeMode::ALL[i];
        }
        uint(a, "depth", &mut o.depth)?;
        num(a, "scale", &mut o.scale)?;
        num(a, "sharpness", &mut o.sharpness)?;
        flag(a, "removeDisconnected", &mut o.remove_disconnected)?;
        num(a, "threshold", &mut o.threshold)?;
        flag(a, "vertNormals", &mut o.vert_normals)?;
        o.validate()?;
        Ok(o)
    }
}

impl crate::Editor {
    /// The item a remesh works on, refusing what it can't change.
    fn remesh_target(&self, what: &str) -> Result<u64> {
        let item = self.edge_insert_target(what)?;
        let it = self.scene.active().ok_or("Select a mesh")?;
        if it.backdrop.is_some() || it.camera.is_some() || it.light.is_some() {
            return Err(format!("{what} works on mesh items"));
        }
        if !it.elements.is_empty() {
            return Err(format!(
                "Unlock and unhide the mesh's points, edges and polygons before {what}"
            ));
        }
        self.scene.refuse_locked(&[item])?;
        Ok(item)
    }

    /// World units per unit of the active item's own space (uniform scale
    /// assumed, as for the other distance options).
    pub fn remesh_world_scale(&self, item: u64) -> f64 {
        self.scene
            .world_matrix(item)
            .map(crate::scene::uniform_scale)
            .unwrap_or(1.0)
    }

    /// `mesh.retopo` / `sculpt.remesh` / `operandi.remesh`: rebuilds the
    /// active mesh, one undo step. Remembers the options.
    pub(crate) fn remesh(&mut self, op: RemeshOp) -> Result<()> {
        let label = op.label();
        let item = self.remesh_target(label)?;
        op.validate()?;
        match &op {
            RemeshOp::Retopo(o) => self.retopo_preset = o.clone(),
            RemeshOp::Voxel(o) => self.voxel_remesh_preset = o.clone(),
            RemeshOp::Octree(o) => self.octree_remesh_preset = o.clone(),
        }
        let prepared = self
            .remesh_prepared
            .take()
            .filter(|p| p.revision == self.revision && p.item == item && p.op == op);
        let out = match prepared {
            Some(p) => p.mesh,
            None => {
                let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
                let scale = self.remesh_world_scale(item);
                remesh_mesh(mesh, &op, scale, &AtomicBool::new(false), &mut |_| {})?
            }
        };
        let before = self.scene.clone();
        let new_mesh = matches!(&op, RemeshOp::Retopo(o) if o.new_mesh);
        if new_mesh {
            return self.remesh_into_new_item(item, out, label, before);
        }
        self.scene.selection.vertices.clear();
        self.scene.selection.edges.clear();
        self.publish_polygon_edit_from(item, out, label, Vec::new(), before)
    }

    /// `newMesh`: the result as a new mesh item right after `source`, with
    /// its parent and transform; the source stays as it was.
    fn remesh_into_new_item(
        &mut self,
        source: u64,
        mesh: Mesh,
        label: &str,
        before: crate::Scene,
    ) -> Result<()> {
        let next = self.scene.next_id.max(self.next_item_id);
        self.scene.next_id = next;
        let (name, transform, subdivision) = {
            let it = self.scene.item(source).ok_or("Select a mesh")?;
            (
                it.name.clone(),
                it.transform.clone(),
                it.subdivision.clone(),
            )
        };
        let render_camera = self.scene.render_camera();
        if let Err(e) = self.scene.new_mesh() {
            self.scene = before;
            return Err(e);
        }
        let new_name = crate::user_defaults::free_name(&format!("{name} Retopo"), |n| {
            self.scene.items.iter().any(|i| i.name == n)
        });
        if let Some(it) = self.scene.items.last_mut() {
            it.name = new_name;
            it.transform = transform;
            it.subdivision = subdivision;
            it.mesh = mesh.into();
        }
        let id = self.scene.items.last().map(|i| i.id);
        self.scene.place_new_item(Some(source), render_camera);
        self.scene.select_item(id);
        self.next_item_id = self.scene.next_id;
        if let Err(e) = self.scene.validate() {
            self.scene = before;
            return Err(e);
        }
        self.record(label.into(), before);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube() -> Mesh {
        Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap()
    }

    fn run(m: &Mesh, op: RemeshOp) -> Result<Mesh> {
        remesh_mesh(m, &op, 1.0, &AtomicBool::new(false), &mut |_| {})
    }

    #[test]
    fn every_mode_gives_a_valid_closed_mesh() {
        let m = cube();
        for op in [
            RemeshOp::Voxel(VoxelOptions {
                density: 0.3,
                ..Default::default()
            }),
            RemeshOp::Retopo(RetopoOptions {
                target_count: 300,
                ..Default::default()
            }),
            RemeshOp::Octree(OctreeOptions {
                mode: OctreeMode::Blocks,
                depth: 4,
                ..Default::default()
            }),
            RemeshOp::Octree(OctreeOptions {
                depth: 4,
                ..Default::default()
            }),
            RemeshOp::Octree(OctreeOptions {
                mode: OctreeMode::Sharp,
                depth: 4,
                ..Default::default()
            }),
        ] {
            let out = run(&m, op.clone()).unwrap_or_else(|e| panic!("{op:?}: {e}"));
            out.validate().unwrap();
            assert!(out.faces.len() > 20, "{op:?}: {}", out.faces.len());
            assert!(out.faces.iter().all(|f| f.smooth));
            let edges = out.edge_counts();
            assert!(
                edges.values().all(|&n| n == 2),
                "{op:?}: open or nonmanifold"
            );
        }
    }

    #[test]
    fn remove_disconnected_keeps_the_big_piece() {
        let mut m = cube();
        let small = Mesh::box_mesh(DVec3::new(3.0, 0.0, 0.0), DVec3::splat(0.5), 0).unwrap();
        let off = m.vertices.len() as u32;
        m.vertices.extend(small.vertices.iter().copied());
        for f in &small.faces {
            let mut f = f.clone();
            f.vertices.iter_mut().for_each(|v| *v += off);
            m.faces.push(f);
        }
        let o = OctreeOptions {
            depth: 5,
            ..Default::default()
        };
        let both = run(&m, RemeshOp::Octree(o.clone())).unwrap();
        let one = run(
            &m,
            RemeshOp::Octree(OctreeOptions {
                remove_disconnected: true,
                threshold: 0.5,
                ..o
            }),
        )
        .unwrap();
        assert!(one.faces.len() < both.faces.len());
        assert!(one.vertices.iter().all(|p| p.x < 1.0));
    }

    #[test]
    fn density_sets_the_voxel_size() {
        let o = VoxelOptions::default();
        let fine = VoxelOptions {
            density: 1.0,
            ..o.clone()
        };
        let coarse = VoxelOptions {
            density: 0.0,
            ..o.clone()
        };
        assert!((coarse.voxel_size(16.0) - 1.0).abs() < 1e-9);
        assert!((fine.voxel_size(640.0) - 1.0).abs() < 1e-9);
        let fixed = VoxelOptions {
            voxel_size: 0.25,
            ..o
        };
        assert_eq!(fixed.voxel_size(100.0), 0.25);
    }

    #[test]
    fn retopo_refuses_a_nonmanifold_point() {
        // Two cubes sharing one corner point.
        let mut m = cube();
        let b = Mesh::box_mesh(DVec3::ONE, DVec3::ONE, 0).unwrap();
        let off = m.vertices.len() as u32;
        let shared = 6; // (0.5, 0.5, 0.5) in box_mesh's order
        assert_eq!(m.vertices[shared], DVec3::splat(0.5));
        let corner = b
            .vertices
            .iter()
            .position(|&p| p == DVec3::splat(0.5))
            .unwrap() as u32;
        m.vertices.extend(b.vertices.iter().copied());
        for f in &b.faces {
            let mut f = f.clone();
            for v in &mut f.vertices {
                *v = if *v == corner {
                    shared as u32
                } else {
                    *v + off
                };
            }
            m.faces.push(f);
        }
        let e = run(&m, RemeshOp::Retopo(RetopoOptions::default())).unwrap_err();
        assert!(e.contains("single point"), "{e}");
    }

    #[test]
    fn bad_options_are_refused() {
        let m = cube();
        for op in [
            RemeshOp::Voxel(VoxelOptions {
                density: 2.0,
                ..Default::default()
            }),
            RemeshOp::Octree(OctreeOptions {
                depth: 20,
                ..Default::default()
            }),
            RemeshOp::Retopo(RetopoOptions {
                target_count: 0,
                ..Default::default()
            }),
        ] {
            assert!(run(&m, op).is_err());
        }
        assert!(run(&Mesh::default(), RemeshOp::Voxel(Default::default())).is_err());
    }

    #[test]
    fn weight_maps_carry_over() {
        let mut m = cube();
        let w: BTreeMap<u32, f64> = (0..8)
            .map(|i| {
                (
                    i,
                    if m.vertices[i as usize].y > 0.0 {
                        1.0
                    } else {
                        0.0
                    },
                )
            })
            .collect();
        m.attributes.point_weights.insert("Mask".into(), w);
        let out = run(
            &m,
            RemeshOp::Voxel(VoxelOptions {
                density: 0.3,
                ..Default::default()
            }),
        )
        .unwrap();
        let map = &out.attributes.point_weights["Mask"];
        assert_eq!(map.len(), out.vertices.len());
        for (i, p) in out.vertices.iter().enumerate() {
            if p.y > 0.3 {
                assert_eq!(map[&(i as u32)], 1.0);
            } else if p.y < -0.3 {
                assert_eq!(map[&(i as u32)], 0.0);
            }
        }
    }

    #[test]
    fn cancel_stops_retopo() {
        let m = cube();
        let e = remesh_mesh(
            &m,
            &RemeshOp::Retopo(RetopoOptions::default()),
            1.0,
            &AtomicBool::new(true),
            &mut |_| {},
        )
        .unwrap_err();
        assert!(e.contains("canceled"), "{e}");
    }

    #[test]
    fn args_keep_last_values_and_parse_words() {
        let a: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(r#"{"mode":"sharp","depth":5}"#).unwrap();
        let base = OctreeOptions {
            scale: 0.5,
            ..Default::default()
        };
        let o = args::octree(&a, &base).unwrap();
        assert_eq!((o.mode, o.depth, o.scale), (OctreeMode::Sharp, 5, 0.5));
        let a: serde_json::Map<String, serde_json::Value> = serde_json::from_str(
            r#"{"countMode":"percent","polyTargetPct":0.5,"creaseAngle":0.5}"#,
        )
        .unwrap();
        let o = args::retopo(&a, &RetopoOptions::default()).unwrap();
        assert_eq!(o.count_mode, CountMode::Percent);
        assert_eq!(o.target(1000), 500);
        let a: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(r#"{"enableSymmetry":true}"#).unwrap();
        assert!(args::retopo(&a, &RetopoOptions::default()).is_err());
    }
}

/// Speed on a big mesh: `cargo test --release -p operandi-core --lib
/// remesh_speed -- --ignored --nocapture`.
#[cfg(test)]
mod speed {
    use super::*;

    /// A bumpy sphere of about `2 * n * n` quads (a sculpt stand-in).
    fn bumpy(n: u32) -> Mesh {
        let mut m = Mesh::default();
        for i in 0..=n {
            for j in 0..2 * n {
                let th = std::f64::consts::PI * i as f64 / n as f64;
                let ph = std::f64::consts::PI * j as f64 / n as f64;
                let r = 1.0 + 0.05 * (7.0 * th).sin() * (5.0 * ph).cos();
                m.vertices
                    .push(DVec3::new(th.sin() * ph.cos(), th.cos(), th.sin() * ph.sin()) * r);
            }
        }
        // Poles collapse to one point each.
        let w = 2 * n;
        let id = |i: u32, j: u32| -> u32 {
            if i == 0 {
                0
            } else if i == n {
                n * w
            } else {
                i * w + j % w
            }
        };
        for i in 0..n {
            for j in 0..w {
                let mut v = vec![id(i, j), id(i, j + 1), id(i + 1, j + 1), id(i + 1, j)];
                v.dedup();
                if v.first() == v.last() {
                    v.pop();
                }
                if v.len() >= 3 {
                    m.faces.push(Face {
                        vertices: v,
                        uv: None,
                        material: String::new(),
                        smooth: true,
                        part: None,
                        material_label: None,
                    });
                }
            }
        }
        // Drop the unused pole copies by rebuilding.
        build_mesh(
            m.vertices.clone(),
            m.faces.iter().map(|f| f.vertices.clone()).collect(),
            &m,
            true,
        )
        .unwrap()
    }

    #[test]
    #[ignore]
    fn remesh_speed() {
        let m = bumpy(230);
        eprintln!("input: {} polygons", m.faces.len());
        for op in [
            RemeshOp::Voxel(VoxelOptions::default()),
            RemeshOp::Voxel(VoxelOptions {
                density: 0.8,
                ..Default::default()
            }),
            RemeshOp::Retopo(RetopoOptions::default()),
            RemeshOp::Octree(OctreeOptions {
                depth: 7,
                ..Default::default()
            }),
        ] {
            let t = std::time::Instant::now();
            let out = remesh_mesh(&m, &op, 1.0, &AtomicBool::new(false), &mut |_| {}).unwrap();
            eprintln!(
                "{}: {} polygons in {:.2} s",
                op.label(),
                out.faces.len(),
                t.elapsed().as_secs_f64()
            );
        }
    }
}
