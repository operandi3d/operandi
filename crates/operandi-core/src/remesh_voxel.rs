//! Voxel Remesh: rebuilds a mesh as an even, all-quad surface on a voxel
//! grid (`sculpt.remesh`).
//!
//! Operandi's own method, written for this tool:
//!
//! 1. Every lattice line along each axis is hit-tested against the
//!    triangles with exact orientation tests, so each crossing is found
//!    once (a shared edge or point belongs to exactly one triangle). A
//!    crossing counts +1 or -1 by which way its triangle faces, so the
//!    running sum along a line is the winding number: overlapping shells
//!    join (a union), and an inside-out shell still counts as solid.
//! 2. A grid point is inside when the lines through it say so. A line
//!    that ends with a non-zero sum went through a hole and doesn't vote;
//!    the other axes decide (two of three, or the one that's left).
//! 3. Faces of the grid where inside and outside points sit diagonally
//!    ("checkerboard" squares) are filled in, so every output edge has
//!    at most two polygons.
//! 4. Surface nets: one point per grid cell the surface passes through,
//!    at the mean of the exact crossings on that cell's edges, and one quad
//!    per grid edge whose ends differ.
//!
//! The grid stores three bits per point (one per axis) and the crossings,
//! never a full distance field, so fine grids stay small.
use crate::fxhash::FxHashMap;
use glam::DVec3;

/// Most grid points one remesh may use (about 640 per side of a cube).
pub const MAX_GRID_POINTS: u64 = 1 << 28;

/// A finished voxel remesh: points and quads, outward-facing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VoxelResult {
    pub points: Vec<DVec3>,
    pub quads: Vec<[u32; 4]>,
}

/// The lattice: grid points at `origin + i * h` for i in 0..n per axis.
#[derive(Debug, Clone, Copy)]
struct Grid {
    origin: DVec3,
    h: f64,
    n: [usize; 3],
    /// Bits per x row: `n[0]` rounded up to whole 64-bit words, so each
    /// row is its own run of words and whole rows compare word by word.
    stride: usize,
}

/// The two axes across lines along `a`, so (a, u, v) is right-handed.
fn across(a: usize) -> (usize, usize) {
    ((a + 1) % 3, (a + 2) % 3)
}

impl Grid {
    /// Grid point index from per-axis indices (x fastest).
    fn index(&self, p: [usize; 3]) -> usize {
        p[0] + self.stride * (p[1] + self.n[1] * p[2])
    }
    fn total(&self) -> usize {
        self.stride * self.n[1] * self.n[2]
    }
    fn words_per_row(&self) -> usize {
        self.stride / 64
    }
    /// The first word of row (y, z).
    fn row(&self, y: usize, z: usize) -> usize {
        (y + self.n[1] * z) * self.words_per_row()
    }
    /// Line index of the line along `a` through (u, v).
    fn line(&self, a: usize, ju: usize, jv: usize) -> usize {
        let (u, _) = across(a);
        ju + self.n[u] * jv
    }
    fn lines(&self, a: usize) -> usize {
        let (u, v) = across(a);
        self.n[u] * self.n[v]
    }
    /// The grid point at step `i` along line (a, ju, jv).
    fn point_on(&self, a: usize, i: usize, ju: usize, jv: usize) -> [usize; 3] {
        let (u, v) = across(a);
        let mut p = [0; 3];
        p[a] = i;
        p[u] = ju;
        p[v] = jv;
        p
    }
    fn coord(&self, axis: usize, i: f64) -> f64 {
        self.origin[axis] + i * self.h
    }
}

/// A packed bit per grid point.
#[derive(Clone)]
struct Bits(Vec<u64>);

impl Bits {
    fn new(n: usize) -> Self {
        Bits(vec![0; n.div_ceil(64)])
    }
    fn get(&self, i: usize) -> bool {
        self.0[i >> 6] >> (i & 63) & 1 == 1
    }
    fn set(&mut self, i: usize) {
        self.0[i >> 6] |= 1 << (i & 63);
    }
}

/// One crossing of a lattice line: which line, how far along, and +1/-1
/// for which way the triangle faces along the line.
#[derive(Clone, Copy)]
struct Crossing {
    line: u32,
    /// The triangle crossed, for its normal (Sharp placement).
    tri: u32,
    depth: f64,
    sign: i8,
}

/// Where each cell's new point goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Placement {
    /// The cell's centre: a blocky, stepped surface.
    Blocks,
    /// The mean of the surface's crossings on the cell's edges.
    Smooth,
    /// The point that best fits the planes of the crossed triangles, so
    /// edges and corners stay sharp (dual contouring). Higher `sharpness`
    /// trusts weaker corners.
    Sharp { sharpness: f64 },
}

/// Picks the grid for voxel size `h` around points spanning `lo..hi`.
fn grid_for(lo: DVec3, hi: DVec3, h: f64) -> Result<Grid, String> {
    if !(h.is_finite() && h > 0.0) {
        return Err("Voxel Size must be above zero".into());
    }
    // Two cells of empty room on every side, and a tiny odd shift so
    // lattice lines rarely run exactly through the mesh's own points
    // (axis-aligned models line up with round numbers).
    let pad = 2.0 * h;
    let origin = lo - DVec3::splat(pad) + DVec3::new(0.000_137, 0.000_291, 0.000_193) * h;
    let span = hi - origin + DVec3::splat(pad);
    let mut n = [0usize; 3];
    let mut total: u64 = 1;
    for a in 0..3 {
        let k = (span[a] / h).ceil();
        if !k.is_finite() || k > MAX_GRID_POINTS as f64 {
            return Err(too_fine());
        }
        n[a] = k as usize + 1;
        total = total.saturating_mul(n[a] as u64);
    }
    if total > MAX_GRID_POINTS {
        return Err(too_fine());
    }
    let stride = n[0].div_ceil(64) * 64;
    if (stride * n[1] * n[2]) as u64 > MAX_GRID_POINTS * 2 {
        return Err(too_fine());
    }
    Ok(Grid {
        origin,
        h,
        n,
        stride,
    })
}

fn too_fine() -> String {
    "Voxel Size is too small for this mesh: the grid would pass 268 million voxels. Use a larger Voxel Size or a lower Density".into()
}

/// Finds every crossing of the lines along axis `a` with the triangles.
fn crossings(grid: &Grid, a: usize, pts: &[DVec3], tris: &[[u32; 3]]) -> Vec<Crossing> {
    let (u, v) = across(a);
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(16);
    let chunk = tris.len().div_ceil(threads).max(1);
    let mut parts: Vec<Vec<Crossing>> = Vec::new();
    std::thread::scope(|s| {
        let handles: Vec<_> = tris
            .chunks(chunk)
            .enumerate()
            .map(|(k, part)| {
                s.spawn(move || line_hits(grid, a, u, v, pts, part, (k * chunk) as u32))
            })
            .collect();
        for h in handles {
            parts.push(h.join().unwrap_or_default());
        }
    });
    let mut all: Vec<Crossing> = parts.concat();
    all.sort_unstable_by(|x, y| x.line.cmp(&y.line).then(x.depth.total_cmp(&y.depth)));
    all
}

/// True when the edge a -> b of a counter-clockwise triangle owns points
/// lying exactly on it ("top-left" rule). Of two triangles sharing an
/// edge in opposite directions, exactly one owns it.
fn owns_edge(a: [f64; 2], b: [f64; 2]) -> bool {
    b[1] < a[1] || (b[1] == a[1] && b[0] > a[0])
}

fn orient(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    robust::orient2d(
        robust::Coord { x: a[0], y: a[1] },
        robust::Coord { x: b[0], y: b[1] },
        robust::Coord { x: c[0], y: c[1] },
    )
}

fn line_hits(
    grid: &Grid,
    a: usize,
    u: usize,
    v: usize,
    pts: &[DVec3],
    tris: &[[u32; 3]],
    first: u32,
) -> Vec<Crossing> {
    let mut out = Vec::new();
    let h = grid.h;
    for (ti, t) in tris.iter().enumerate() {
        let p = t.map(|i| pts[i as usize]);
        let mut q = p.map(|p| [p[u], p[v]]);
        let mut depth = p.map(|p| p[a]);
        let area = orient(q[0], q[1], q[2]);
        if area == 0.0 {
            // Edge-on to these lines: other axes see it.
            continue;
        }
        // Counter-clockwise from here on; the sign remembers the facing.
        let sign: i8 = if area > 0.0 { 1 } else { -1 };
        if area < 0.0 {
            q.swap(1, 2);
            depth.swap(1, 2);
        }
        let area = area.abs();
        let lo_u = q.iter().map(|c| c[0]).fold(f64::INFINITY, f64::min);
        let hi_u = q.iter().map(|c| c[0]).fold(f64::NEG_INFINITY, f64::max);
        let lo_v = q.iter().map(|c| c[1]).fold(f64::INFINITY, f64::min);
        let hi_v = q.iter().map(|c| c[1]).fold(f64::NEG_INFINITY, f64::max);
        let ju0 = ((lo_u - grid.origin[u]) / h).ceil().max(0.0) as usize;
        let ju1 = (((hi_u - grid.origin[u]) / h).floor() as isize).min(grid.n[u] as isize - 1);
        let jv0 = ((lo_v - grid.origin[v]) / h).ceil().max(0.0) as usize;
        let jv1 = (((hi_v - grid.origin[v]) / h).floor() as isize).min(grid.n[v] as isize - 1);
        if ju1 < ju0 as isize || jv1 < jv0 as isize {
            continue;
        }
        let own = [
            owns_edge(q[1], q[2]),
            owns_edge(q[2], q[0]),
            owns_edge(q[0], q[1]),
        ];
        for jv in jv0..=jv1 as usize {
            let cv = grid.coord(v, jv as f64);
            for ju in ju0..=ju1 as usize {
                let c = [grid.coord(u, ju as f64), cv];
                let w = [
                    orient(q[1], q[2], c),
                    orient(q[2], q[0], c),
                    orient(q[0], q[1], c),
                ];
                if (0..3).any(|k| w[k] < 0.0 || (w[k] == 0.0 && !own[k])) {
                    continue;
                }
                let d = (w[0] * depth[0] + w[1] * depth[1] + w[2] * depth[2]) / area;
                out.push(Crossing {
                    line: grid.line(a, ju, jv) as u32,
                    tri: first + ti as u32,
                    depth: d,
                    sign,
                });
            }
        }
    }
    out
}

/// Per axis: which grid points that axis's lines call inside, and the
/// lines that can't be trusted (their winding doesn't return to zero, so
/// they went through a hole).
fn axis_inside(grid: &Grid, a: usize, hits: &[Crossing]) -> (Bits, Vec<(usize, usize)>) {
    let (u, v) = across(a);
    let mut inside = Bits::new(grid.total());
    let mut unreliable = Vec::new();
    let mut k = 0;
    for jv in 0..grid.n[v] {
        for ju in 0..grid.n[u] {
            let line = grid.line(a, ju, jv) as u32;
            let start = k;
            while k < hits.len() && hits[k].line == line {
                k += 1;
            }
            let row = &hits[start..k];
            if row.is_empty() {
                continue;
            }
            if row.iter().map(|c| c.sign as i32).sum::<i32>() != 0 {
                unreliable.push((ju, jv));
                continue;
            }
            // Points after crossing `d` (strictly) take its winding: the
            // first is floor((d - o) / h) + 1.
            let first_after = |d: f64| -> usize {
                (((d - grid.origin[a]) / grid.h).floor() + 1.0).clamp(0.0, grid.n[a] as f64)
                    as usize
            };
            let mut winding = 0i32;
            for (c, next) in row.iter().zip(row.iter().skip(1).map(Some).chain([None])) {
                winding += c.sign as i32;
                if winding == 0 {
                    continue;
                }
                let from = first_after(c.depth);
                let to = next.map_or(grid.n[a], |n| first_after(n.depth));
                if a == 0 {
                    let base = grid.index(grid.point_on(a, 0, ju, jv));
                    set_range(&mut inside, base + from, base + to);
                } else {
                    for i in from..to {
                        inside.set(grid.index(grid.point_on(a, i, ju, jv)));
                    }
                }
            }
        }
    }
    (inside, unreliable)
}

/// Sets bits `from..to`.
fn set_range(bits: &mut Bits, from: usize, to: usize) {
    let mut i = from;
    while i < to {
        if i & 63 == 0 && to - i >= 64 {
            bits.0[i >> 6] = u64::MAX;
            i += 64;
        } else {
            bits.set(i);
            i += 1;
        }
    }
}

/// Combines the three axes' answers (see the module notes): two of three
/// trustworthy axes, or the only one there is. Word by word; an axis's
/// trust bits are only built when it has untrustworthy lines.
fn vote(grid: &Grid, axes: &[(Bits, Vec<(usize, usize)>); 3]) -> Bits {
    let trust: Vec<Option<Bits>> = axes
        .iter()
        .enumerate()
        .map(|(a, (_, bad))| {
            if bad.is_empty() {
                return None;
            }
            let mut r = Bits(vec![u64::MAX; grid.total().div_ceil(64)]);
            for &(ju, jv) in bad {
                for i in 0..grid.n[a] {
                    let k = grid.index(grid.point_on(a, i, ju, jv));
                    r.0[k >> 6] &= !(1 << (k & 63));
                }
            }
            Some(r)
        })
        .collect();
    let words = grid.total().div_ceil(64);
    let mut out = Bits(vec![0; words]);
    for w in 0..words {
        let i = [0, 1, 2].map(|a| axes[a].0.0[w]);
        let r = [0, 1, 2].map(|a| trust[a].as_ref().map_or(u64::MAX, |t| t.0[w]));
        out.0[w] = (i[0] & i[1])
            | (i[1] & i[2])
            | (i[0] & i[2])
            | (i[0] & !r[1] & !r[2])
            | (i[1] & !r[0] & !r[2])
            | (i[2] & !r[0] & !r[1]);
    }
    out
}

/// Bits x+1 of a row, at x: the row's words shifted down by one bit.
fn next_bits(row: &[u64], w: usize) -> u64 {
    (row[w] >> 1) | row.get(w + 1).map_or(0, |n| n << 63)
}

/// Fills grid squares whose inside corners sit diagonally, until none are
/// left (or a few passes are done; the mesh check catches the rest).
/// Word by word: a whole row of squares at a time.
fn fill_checkerboards(grid: &Grid, inside: &mut Bits) {
    let wpr = grid.words_per_row();
    let [_, ny, nz] = grid.n;
    let checker = |a: u64, b: u64, c: u64, d: u64| (a & c & !b & !d) | (b & d & !a & !c);
    for _ in 0..16 {
        let mut changed = false;
        let bits = &mut inside.0;
        for z in 0..nz {
            for y in 0..ny {
                let r0 = grid.row(y, z);
                // Squares spanning x and y, then x and z: rows (y, z) and
                // the next row along y or z.
                for r1 in [
                    (y + 1 < ny).then(|| grid.row(y + 1, z)),
                    (z + 1 < nz).then(|| grid.row(y, z + 1)),
                ]
                .into_iter()
                .flatten()
                {
                    for w in 0..wpr {
                        let (a, d) = (bits[r0 + w], bits[r1 + w]);
                        let b = next_bits(&bits[r0..r0 + wpr], w);
                        let c = next_bits(&bits[r1..r1 + wpr], w);
                        let m = checker(a, b, c, d);
                        if m != 0 {
                            changed = true;
                            let fill = |row: usize, bits: &mut Vec<u64>| {
                                bits[row + w] |= m | (m << 1);
                                if w + 1 < wpr {
                                    bits[row + w + 1] |= m >> 63;
                                }
                            };
                            fill(r0, bits);
                            fill(r1, bits);
                        }
                    }
                }
                // Squares spanning y and z, at every x of the row.
                if y + 1 < ny && z + 1 < nz {
                    let (rb, rc, rd) = (
                        grid.row(y + 1, z),
                        grid.row(y + 1, z + 1),
                        grid.row(y, z + 1),
                    );
                    for w in 0..wpr {
                        let m = checker(bits[r0 + w], bits[rb + w], bits[rc + w], bits[rd + w]);
                        if m != 0 {
                            changed = true;
                            for r in [r0, rb, rc, rd] {
                                bits[r + w] |= m;
                            }
                        }
                    }
                }
            }
        }
        if !changed {
            return;
        }
    }
}

/// Runs the voxel remesh of triangles `tris` over `pts` at voxel size `h`,
/// with smooth placement.
pub fn voxel_remesh(pts: &[DVec3], tris: &[[u32; 3]], h: f64) -> Result<VoxelResult, String> {
    voxel_remesh_with(pts, tris, h, Placement::Smooth)
}

/// [`voxel_remesh`] with the given point placement.
pub fn voxel_remesh_with(
    pts: &[DVec3],
    tris: &[[u32; 3]],
    h: f64,
    placement: Placement,
) -> Result<VoxelResult, String> {
    if pts.is_empty() || tris.is_empty() {
        return Err("Nothing to remesh".into());
    }
    if tris.iter().flatten().any(|&i| i as usize >= pts.len()) {
        return Err("A triangle points past the mesh".into());
    }
    let mut lo = DVec3::splat(f64::INFINITY);
    let mut hi = DVec3::splat(f64::NEG_INFINITY);
    for t in tris {
        for &i in t {
            lo = lo.min(pts[i as usize]);
            hi = hi.max(pts[i as usize]);
        }
    }
    let grid = grid_for(lo, hi, h)?;
    let hits: [Vec<Crossing>; 3] = std::array::from_fn(|a| crossings(&grid, a, pts, tris));
    let axes: [(Bits, Vec<(usize, usize)>); 3] =
        std::array::from_fn(|a| axis_inside(&grid, a, &hits[a]));
    let mut inside = vote(&grid, &axes);
    drop(axes);
    // The border of the grid is always outside, so every surface closes.
    for a in 0..3 {
        let (u, v) = across(a);
        for jv in 0..grid.n[v] {
            for ju in 0..grid.n[u] {
                for i in [0, grid.n[a] - 1] {
                    let k = grid.index(grid.point_on(a, i, ju, jv));
                    inside.0[k >> 6] &= !(1 << (k & 63));
                }
            }
        }
    }
    fill_checkerboards(&grid, &mut inside);
    let normals: Vec<DVec3> = match placement {
        Placement::Sharp { .. } => tris
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| pts[i as usize]);
                (b - a).cross(c - a).normalize_or_zero()
            })
            .collect(),
        _ => Vec::new(),
    };
    Ok(surface_nets(&grid, &inside, &hits, placement, &normals))
}

/// Builds the quads (step 4 of the module notes). Edges whose ends differ
/// are found a row of 64 at a time by comparing neighbouring rows' words.
fn surface_nets(
    grid: &Grid,
    inside: &Bits,
    hits: &[Vec<Crossing>; 3],
    placement: Placement,
    normals: &[DVec3],
) -> VoxelResult {
    // Where each line's crossings start in its axis's sorted list.
    let starts: Vec<Vec<u32>> = (0..3)
        .map(|a| {
            let mut s = vec![0u32; grid.lines(a) + 1];
            for c in &hits[a] {
                s[c.line as usize + 1] += 1;
            }
            for l in 1..s.len() {
                s[l] += s[l - 1];
            }
            s
        })
        .collect();
    let mut nets = Nets {
        grid,
        inside,
        hits,
        starts: &starts,
        normals,
        cells: FxHashMap::default(),
        sums: Vec::new(),
        quads: Vec::new(),
    };
    let [nx, ny, nz] = grid.n;
    let wpr = grid.words_per_row();
    let bits = &inside.0;
    // Bits of x below `limit` in word w.
    let below = |w: usize, limit: usize| -> u64 {
        let lo = w * 64;
        if limit <= lo {
            0
        } else if limit - lo >= 64 {
            u64::MAX
        } else {
            (1u64 << (limit - lo)) - 1
        }
    };
    for z in 0..nz {
        for y in 0..ny {
            let r0 = grid.row(y, z);
            let row0 = &bits[r0..r0 + wpr];
            // Along x: lines with cells all round need 1 <= y, z < n - 1.
            if (1..ny - 1).contains(&y) && (1..nz - 1).contains(&z) {
                for w in 0..wpr {
                    let t = (row0[w] ^ next_bits(row0, w)) & below(w, nx - 1);
                    for x in set_bits(t, w) {
                        nets.edge(0, x, y, z);
                    }
                }
            }
            // Along y (u = z, v = x) and z (u = x, v = y): the next row.
            for (a, next) in [
                (1, (y + 1 < ny).then(|| grid.row(y + 1, z))),
                (2, (z + 1 < nz).then(|| grid.row(y, z + 1))),
            ] {
                let Some(r1) = next else { continue };
                let ok = match a {
                    1 => (1..nz - 1).contains(&z),
                    _ => (1..ny - 1).contains(&y),
                };
                if !ok {
                    continue;
                }
                for w in 0..wpr {
                    // Cells all round also need 1 <= x < nx - 1.
                    let t = (row0[w] ^ bits[r1 + w]) & below(w, nx - 1) & !below(w, 1);
                    for x in set_bits(t, w) {
                        match a {
                            1 => nets.edge(1, y, z, x),
                            _ => nets.edge(2, z, x, y),
                        }
                    }
                }
            }
        }
    }
    let points = nets.sums.iter().map(|c| c.place(grid, placement)).collect();
    VoxelResult {
        points,
        quads: nets.quads,
    }
}

/// The x of each set bit of word `w`.
fn set_bits(mut t: u64, w: usize) -> impl Iterator<Item = usize> {
    std::iter::from_fn(move || {
        if t == 0 {
            return None;
        }
        let b = t.trailing_zeros() as usize;
        t &= t - 1;
        Some(w * 64 + b)
    })
}

struct Nets<'a> {
    grid: &'a Grid,
    inside: &'a Bits,
    hits: &'a [Vec<Crossing>; 3],
    starts: &'a [Vec<u32>],
    /// Per triangle, for Sharp placement (else empty).
    normals: &'a [DVec3],
    /// Cell (by its lowest corner's index) -> output point.
    cells: FxHashMap<usize, u32>,
    sums: Vec<Cell>,
    quads: Vec<[u32; 4]>,
}

impl Nets<'_> {
    /// The edge from step `i` to `i + 1` of line (a, ju, jv), whose ends
    /// differ: its crossing and its quad.
    fn edge(&mut self, a: usize, i: usize, ju: usize, jv: usize) {
        let grid = self.grid;
        let (u, v) = across(a);
        let line = grid.line(a, ju, jv);
        let row = &self.hits[a][self.starts[a][line] as usize..self.starts[a][line + 1] as usize];
        let x0 = grid.coord(a, i as f64);
        let x1 = grid.coord(a, (i + 1) as f64);
        let lo = row.partition_point(|c| c.depth < x0);
        let hi = row.partition_point(|c| c.depth < x1);
        // The mean of the line's hits inside the edge, or its middle when
        // the vote overruled this axis.
        let depth = if hi > lo {
            row[lo..hi].iter().map(|c| c.depth).sum::<f64>() / (hi - lo) as f64
        } else {
            0.5 * (x0 + x1)
        };
        let normal = (!self.normals.is_empty() && hi > lo)
            .then(|| {
                row[lo..hi]
                    .iter()
                    .map(|c| self.normals[c.tri as usize])
                    .sum::<DVec3>()
                    .normalize_or_zero()
            })
            .filter(|n| *n != DVec3::ZERO);
        let mut at = DVec3::ZERO;
        at[a] = depth;
        at[u] = grid.coord(u, ju as f64);
        at[v] = grid.coord(v, jv as f64);
        // The four cells around the edge, counter-clockwise in (u, v), so
        // the quad faces +a.
        let around = [(ju - 1, jv - 1), (ju, jv - 1), (ju, jv), (ju - 1, jv)];
        let ids = around.map(|(cu, cv)| {
            let corner = grid.point_on(a, i, cu, cv);
            let cell = grid.index(corner);
            let sums = &mut self.sums;
            let id = *self.cells.entry(cell).or_insert_with(|| {
                sums.push(Cell::new(corner));
                (sums.len() - 1) as u32
            });
            self.sums[id as usize].add(at, normal);
            id
        });
        // Outward is the way from inside to outside.
        let from_inside = self.inside.get(grid.index(grid.point_on(a, i, ju, jv)));
        if from_inside {
            self.quads.push(ids);
        } else {
            self.quads.push([ids[0], ids[3], ids[2], ids[1]]);
        }
    }
}

/// What one cell gathers for its point.
#[derive(Clone, Copy)]
struct Cell {
    corner: [usize; 3],
    sum: DVec3,
    count: u32,
    /// The plane fit: sum of n nᵀ (upper triangle xx, xy, xz, yy, yz,
    /// zz) and of n (n · p).
    ata: [f64; 6],
    atb: DVec3,
    planes: u32,
}

impl Cell {
    fn new(corner: [usize; 3]) -> Self {
        Cell {
            corner,
            sum: DVec3::ZERO,
            count: 0,
            ata: [0.0; 6],
            atb: DVec3::ZERO,
            planes: 0,
        }
    }
    fn add(&mut self, p: DVec3, n: Option<DVec3>) {
        self.sum += p;
        self.count += 1;
        if let Some(n) = n {
            let a = &mut self.ata;
            a[0] += n.x * n.x;
            a[1] += n.x * n.y;
            a[2] += n.x * n.z;
            a[3] += n.y * n.y;
            a[4] += n.y * n.z;
            a[5] += n.z * n.z;
            self.atb += n * n.dot(p);
            self.planes += 1;
        }
    }
    fn place(&self, grid: &Grid, placement: Placement) -> DVec3 {
        let lo = DVec3::new(
            grid.coord(0, self.corner[0] as f64),
            grid.coord(1, self.corner[1] as f64),
            grid.coord(2, self.corner[2] as f64),
        );
        let mass = self.sum / self.count.max(1) as f64;
        match placement {
            Placement::Blocks => lo + DVec3::splat(0.5 * grid.h),
            Placement::Smooth => mass,
            Placement::Sharp { sharpness } => {
                if self.planes < 2 {
                    return mass;
                }
                // Least squares about the mass point, ignoring directions
                // the planes barely constrain (a flat or creased cell
                // stays put along its free directions).
                let tol = (0.1 / sharpness.max(1e-3)).clamp(1e-4, 0.9);
                let a = self.ata;
                let m = [[a[0], a[1], a[2]], [a[1], a[3], a[4]], [a[2], a[4], a[5]]];
                let rhs = self.atb - mul(&m, mass);
                let x = mass + pseudo_solve(m, rhs, tol);
                // Stay in (or just around) the cell, else keep it smooth.
                let slack = DVec3::splat(0.5 * grid.h);
                let hi = lo + DVec3::splat(grid.h);
                if x.is_finite() && x.cmpge(lo - slack).all() && x.cmple(hi + slack).all() {
                    x
                } else {
                    mass
                }
            }
        }
    }
}

fn mul(m: &[[f64; 3]; 3], v: DVec3) -> DVec3 {
    DVec3::new(
        m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z,
        m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z,
        m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z,
    )
}

/// Solves `m x = b` for a symmetric 3x3 `m` through its eigenvectors
/// (Jacobi rotations), dropping eigenvalues below `tol` times the largest.
fn pseudo_solve(mut m: [[f64; 3]; 3], b: DVec3, tol: f64) -> DVec3 {
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..16 {
        let off = m[0][1].abs() + m[0][2].abs() + m[1][2].abs();
        if off < 1e-15 {
            break;
        }
        for (p, q) in [(0, 1), (0, 2), (1, 2)] {
            if m[p][q].abs() < 1e-18 {
                continue;
            }
            let theta = (m[q][q] - m[p][p]) / (2.0 * m[p][q]);
            let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
            let t = if theta == 0.0 { 1.0 } else { t };
            let c = 1.0 / (t * t + 1.0).sqrt();
            let s = t * c;
            // m <- Jᵀ m J, v <- v J for the rotation in plane (p, q).
            for row in &mut m {
                let (mkp, mkq) = (row[p], row[q]);
                row[p] = c * mkp - s * mkq;
                row[q] = s * mkp + c * mkq;
            }
            let (rp, rq) = (m[p], m[q]);
            for k in 0..3 {
                m[p][k] = c * rp[k] - s * rq[k];
                m[q][k] = s * rp[k] + c * rq[k];
            }
            for row in &mut v {
                let (vp, vq) = (row[p], row[q]);
                row[p] = c * vp - s * vq;
                row[q] = s * vp + c * vq;
            }
        }
    }
    let eig = [m[0][0], m[1][1], m[2][2]];
    let top = eig.iter().fold(0.0f64, |a, &e| a.max(e.abs()));
    let mut x = DVec3::ZERO;
    for k in 0..3 {
        if top == 0.0 || eig[k].abs() < tol * top {
            continue;
        }
        let col = DVec3::new(v[0][k], v[1][k], v[2][k]);
        x += col * (col.dot(b) / eig[k]);
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sharp_keeps_a_cube_corner_and_blocks_sit_on_cell_centres() {
        let (p, t) = cube(DVec3::ZERO, 1.0);
        let sharp = voxel_remesh_with(&p, &t, 0.1, Placement::Sharp { sharpness: 1.0 }).unwrap();
        assert!(closed_manifold(&sharp));
        // A point lands on (close to) the cube's corner.
        let best = sharp
            .points
            .iter()
            .map(|q| q.distance(DVec3::splat(0.5)))
            .fold(f64::INFINITY, f64::min);
        assert!(best < 1e-6, "nearest to the corner: {best}");
        let smooth = voxel_remesh(&p, &t, 0.1).unwrap();
        let soft = smooth
            .points
            .iter()
            .map(|q| q.distance(DVec3::splat(0.5)))
            .fold(f64::INFINITY, f64::min);
        assert!(soft > best);
        let blocks = voxel_remesh_with(&p, &t, 0.1, Placement::Blocks).unwrap();
        assert!(closed_manifold(&blocks));
        assert_eq!(blocks.quads.len(), smooth.quads.len());
    }

    #[test]
    fn pseudo_solve_drops_free_directions() {
        // Two planes x = 1 and y = 2: z stays at the start (0).
        let m = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]];
        let x = pseudo_solve(m, DVec3::new(1.0, 2.0, 0.0), 0.1);
        assert!((x - DVec3::new(1.0, 2.0, 0.0)).length() < 1e-12);
        // A rotated full-rank system.
        let m = [[2.0, 1.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 3.0]];
        let want = DVec3::new(0.5, -1.0, 2.0);
        let x = pseudo_solve(m, mul(&m, want), 0.01);
        assert!((x - want).length() < 1e-9, "{x}");
    }

    fn cube(c: DVec3, s: f64) -> (Vec<DVec3>, Vec<[u32; 3]>) {
        let p: Vec<DVec3> = (0..8)
            .map(|i| {
                c + DVec3::new(
                    (i & 1) as f64 - 0.5,
                    ((i >> 1) & 1) as f64 - 0.5,
                    ((i >> 2) & 1) as f64 - 0.5,
                ) * s
            })
            .collect();
        let quads: [[u32; 4]; 6] = [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ];
        let t = quads
            .iter()
            .flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]])
            .collect();
        (p, t)
    }

    /// Every edge used exactly twice, in opposite directions.
    fn closed_manifold(r: &VoxelResult) -> bool {
        let mut uses: FxHashMap<(u32, u32), i32> = FxHashMap::default();
        for q in &r.quads {
            for k in 0..4 {
                let (a, b) = (q[k], q[(k + 1) % 4]);
                *uses.entry((a.min(b), a.max(b))).or_default() += if a < b { 1 } else { -1 };
            }
        }
        let mut count: FxHashMap<(u32, u32), i32> = FxHashMap::default();
        for q in &r.quads {
            for k in 0..4 {
                let (a, b) = (q[k], q[(k + 1) % 4]);
                *count.entry((a.min(b), a.max(b))).or_default() += 1;
            }
        }
        uses.values().all(|&s| s == 0) && count.values().all(|&c| c == 2)
    }

    fn volume(r: &VoxelResult) -> f64 {
        r.quads
            .iter()
            .flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]])
            .map(|t| {
                let [a, b, c] = t.map(|i| r.points[i as usize]);
                a.dot(b.cross(c)) / 6.0
            })
            .sum()
    }

    #[test]
    fn cube_becomes_closed_quads_of_the_same_size() {
        let (p, t) = cube(DVec3::ZERO, 1.0);
        let r = voxel_remesh(&p, &t, 0.05).unwrap();
        assert!(r.quads.len() > 1000);
        assert!(closed_manifold(&r));
        let v = volume(&r);
        assert!((v - 1.0).abs() < 0.05, "volume {v}");
        for q in &r.points {
            assert!(q.abs().max_element() < 0.5 + 0.05, "{q}");
        }
    }

    #[test]
    fn inside_out_cube_still_faces_out() {
        let (p, mut t) = cube(DVec3::ZERO, 1.0);
        for tri in &mut t {
            tri.swap(1, 2);
        }
        let r = voxel_remesh(&p, &t, 0.1).unwrap();
        assert!(closed_manifold(&r));
        assert!(volume(&r) > 0.8);
    }

    #[test]
    fn overlapping_cubes_join() {
        let (mut p, mut t) = cube(DVec3::ZERO, 1.0);
        let (p2, t2) = cube(DVec3::new(0.5, 0.0, 0.0), 1.0);
        let off = p.len() as u32;
        p.extend(p2);
        t.extend(t2.iter().map(|tri| tri.map(|i| i + off)));
        let r = voxel_remesh(&p, &t, 0.05).unwrap();
        assert!(closed_manifold(&r));
        // The union is 1.5 x 1 x 1.
        let v = volume(&r);
        assert!((v - 1.5).abs() < 0.08, "volume {v}");
    }

    #[test]
    fn a_hole_is_closed() {
        let (p, mut t) = cube(DVec3::ZERO, 1.0);
        t.truncate(10); // drop one side
        let r = voxel_remesh(&p, &t, 0.1).unwrap();
        assert!(closed_manifold(&r));
    }

    #[test]
    fn too_fine_is_an_error() {
        let (p, t) = cube(DVec3::ZERO, 1.0);
        assert!(voxel_remesh(&p, &t, 1e-4).is_err());
        assert!(voxel_remesh(&p, &t, 0.0).is_err());
        assert!(voxel_remesh(&[], &[], 0.1).is_err());
    }
}
