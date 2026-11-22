//! Atlas Unwrap (`operandi.uv.atlas`): a quick automatic UV layout with no
//! seams to mark. Polygons are grouped into charts by flood fill across
//! shared edges while each polygon's normal stays within `angle` of the
//! chart's first (largest) polygon, so every chart projects flat onto that
//! polygon's plane without folding. Charts are turned to their long axis
//! and shelf-packed into 0..1 with one scale, so texel density is the same
//! everywhere. Good for bakes and meshes imported without UVs; UV Unwrap
//! (`uv.unwrap`) gives fewer, less stretched islands.
//!
//! Adapted from TextureCraft `crates/geom/src/unwrap.rs` at 9c1b9cbc (MIT OR
//! Apache-2.0, Copyright (c) 2026 ArtCraft Team and the TextureCraft
//! contributors; see docs/licenses/texturecraft-LICENSE-MIT.txt). Changes:
//! works on Operandi's polygons and per-corner UV maps (no vertex
//! splitting or welding), one 0..1 space for all materials, v up, the
//! chart angle is an option.
use crate::{Editor, Mesh, Result};
use glam::{DVec2, DVec3};
use std::collections::{HashMap, VecDeque};

/// Packing never goes beyond this virtual resolution (texels per side).
const MAX_PACK_RES: f64 = 65_536.0;

#[derive(Clone, Debug, PartialEq)]
pub struct AtlasOptions {
    /// The UV map written; `None` is the mesh's usual one (made when
    /// missing).
    pub uv_map: Option<String>,
    /// Most a polygon's normal may turn from its chart's first polygon, in
    /// radians.
    pub angle: f64,
    /// Space around each chart, in texels at `resolution`.
    pub gaps: f64,
    /// The texture size the gaps are measured at.
    pub resolution: u32,
}

impl Default for AtlasOptions {
    fn default() -> Self {
        AtlasOptions {
            uv_map: None,
            angle: 60f64.to_radians(),
            gaps: 4.0,
            resolution: 1024,
        }
    }
}

impl AtlasOptions {
    pub fn validate(&self) -> Result<()> {
        if !(self.angle > 0.0 && self.angle <= std::f64::consts::FRAC_PI_2) {
            return Err("angle is above 0 and at most 90° (1.5708 radians)".into());
        }
        if !(0.0..=64.0).contains(&self.gaps) {
            return Err("gaps is 0 to 64 texels".into());
        }
        if !(16..=16384).contains(&self.resolution) {
            return Err("resolution is 16 to 16384".into());
        }
        Ok(())
    }
}

struct Chart {
    t: DVec3,
    b: DVec3,
    /// Turn to the chart's long axis (cos, sin).
    rot: DVec2,
    lo: DVec2,
    hi: DVec2,
    offset: DVec2,
}

impl Chart {
    fn local(&self, p: DVec3) -> DVec2 {
        let q = DVec2::new(p.dot(self.t), p.dot(self.b));
        DVec2::new(
            q.x * self.rot.x - q.y * self.rot.y,
            q.x * self.rot.y + q.y * self.rot.x,
        )
    }
    fn size(&self) -> DVec2 {
        (self.hi - self.lo).max(DVec2::ZERO)
    }
}

/// `mesh` with map `map` replaced by an atlas layout of every polygon not
/// in `hidden`, and how many charts it made. Hidden polygons and polylines
/// keep their UVs.
pub fn atlas_mesh(
    mesh: &Mesh,
    map: &str,
    hidden: &[bool],
    o: &AtlasOptions,
) -> Result<(Mesh, usize)> {
    o.validate()?;
    let n = mesh.faces.len();
    let used: Vec<bool> = (0..n)
        .map(|f| !hidden.get(f).copied().unwrap_or(false) && !mesh.is_polyline(f))
        .collect();
    if !used.iter().any(|u| *u) {
        return Err("No visible polygons to unwrap".into());
    }
    let normal: Vec<Option<DVec3>> = (0..n).map(|f| mesh.normal(f).try_normalize()).collect();
    let area: Vec<f64> = (0..n).map(|f| polygon_area(mesh, f)).collect();
    // Edge -> the used polygons on it; only edges with exactly two join.
    let mut edges: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for f in (0..n).filter(|&f| used[f]) {
        let v = &mesh.faces[f].vertices;
        for i in 0..v.len() {
            let (a, b) = (v[i], v[(i + 1) % v.len()]);
            edges.entry((a.min(b), a.max(b))).or_default().push(f);
        }
    }
    let cos = o.angle.cos() - 1e-9;
    let mut order: Vec<usize> = (0..n).filter(|&f| used[f]).collect();
    order.sort_by(|a, b| area[*b].total_cmp(&area[*a]));
    let mut chart_of = vec![usize::MAX; n];
    let mut charts: Vec<Chart> = Vec::new();
    let mut queue = VecDeque::new();
    for &seed in &order {
        if chart_of[seed] != usize::MAX {
            continue;
        }
        let id = charts.len();
        let sn = normal[seed].unwrap_or(DVec3::Z);
        let t = sn.any_orthonormal_vector();
        charts.push(Chart {
            t,
            b: sn.cross(t),
            rot: DVec2::X,
            lo: DVec2::ZERO,
            hi: DVec2::ZERO,
            offset: DVec2::ZERO,
        });
        chart_of[seed] = id;
        queue.push_back(seed);
        while let Some(f) = queue.pop_front() {
            let v = &mesh.faces[f].vertices;
            for i in 0..v.len() {
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                let Some(across) = edges.get(&(a.min(b), a.max(b))) else {
                    continue;
                };
                if across.len() != 2 {
                    continue;
                }
                let g = if across[0] == f { across[1] } else { across[0] };
                let ok = normal[g].is_none_or(|nn| nn.dot(sn) >= cos);
                if ok && chart_of[g] == usize::MAX {
                    chart_of[g] = id;
                    queue.push_back(g);
                }
            }
        }
    }
    let pos = |p: u32| mesh.vertices[p as usize];
    // Principal axes, then bounds, of every chart.
    let mut cov = vec![(DVec2::ZERO, [0.0f64; 3], 0usize); charts.len()];
    for f in (0..n).filter(|&f| used[f]) {
        let c = chart_of[f];
        for &p in &mesh.faces[f].vertices {
            let q = charts[c].local(pos(p));
            let acc = &mut cov[c];
            acc.0 += q;
            acc.1[0] += q.x * q.x;
            acc.1[1] += q.y * q.y;
            acc.1[2] += q.x * q.y;
            acc.2 += 1;
        }
    }
    for (ch, (sum, m, k)) in charts.iter_mut().zip(&cov) {
        let k = (*k).max(1) as f64;
        let (mx, my) = (sum.x / k, sum.y / k);
        let (cxx, cyy, cxy) = (m[0] / k - mx * mx, m[1] / k - my * my, m[2] / k - mx * my);
        let angle = 0.5 * (2.0 * cxy).atan2(cxx - cyy);
        ch.rot = DVec2::new((-angle).cos(), (-angle).sin());
        ch.lo = DVec2::splat(f64::MAX);
        ch.hi = DVec2::splat(f64::MIN);
    }
    for f in (0..n).filter(|&f| used[f]) {
        let ch = &mut charts[chart_of[f]];
        for &p in &mesh.faces[f].vertices {
            let q = ch.local(pos(p));
            ch.lo = ch.lo.min(q);
            ch.hi = ch.hi.max(q);
        }
    }
    for ch in &mut charts {
        if ch.size().y > ch.size().x {
            // Quarter turn (x, y) -> (y, -x) so shelves stay low.
            ch.rot = DVec2::new(ch.rot.y, -ch.rot.x);
            let (lo, hi) = (ch.lo, ch.hi);
            ch.lo = DVec2::new(lo.y, -hi.x);
            ch.hi = DVec2::new(hi.y, -lo.x);
        }
    }
    let sizes: Vec<DVec2> = charts.iter().map(Chart::size).collect();
    let pad = o.gaps.max(0.5);
    let (scale, res, offsets) = pack(&sizes, o.resolution as f64, pad)?;
    for (ch, offset) in charts.iter_mut().zip(offsets) {
        ch.offset = offset;
    }
    let mut out = mesh.clone();
    let mut rows: Vec<Vec<Option<DVec2>>> = mesh.uv_maps.get(map).cloned().unwrap_or_else(|| {
        mesh.faces
            .iter()
            .map(|f| vec![None; f.vertices.len()])
            .collect()
    });
    for f in (0..n).filter(|&f| used[f]) {
        let ch = &charts[chart_of[f]];
        rows[f] = mesh.faces[f]
            .vertices
            .iter()
            .map(|&p| {
                let q = ch.local(pos(p));
                let texel = ch.offset + DVec2::splat(pad) + (q - ch.lo) * scale;
                Some((texel / res).clamp(DVec2::ZERO, DVec2::ONE))
            })
            .collect();
    }
    out.uv_maps.insert(map.to_owned(), rows);
    Ok((out, charts.len()))
}

/// A polygon's area (Newell's method; exact for planar polygons).
fn polygon_area(mesh: &Mesh, f: usize) -> f64 {
    let v = &mesh.faces[f].vertices;
    let mut sum = DVec3::ZERO;
    for i in 0..v.len() {
        let (a, b) = (
            mesh.vertices[v[i] as usize],
            mesh.vertices[v[(i + 1) % v.len()] as usize],
        );
        sum += a.cross(b);
    }
    sum.length() * 0.5
}

/// Shelf-packs rectangles of world `sizes` (each grown by `pad` texels a
/// side) into a square of `res` texels at the largest scale that fits:
/// (texels per unit, resolution used, texel offset of each). The
/// resolution doubles when the padding alone can't fit.
fn pack(sizes: &[DVec2], mut res: f64, pad: f64) -> Result<(f64, f64, Vec<DVec2>)> {
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by(|a, b| sizes[*b].y.total_cmp(&sizes[*a].y));
    let place = |s: f64, res: f64| -> Option<Vec<DVec2>> {
        let mut out = vec![DVec2::ZERO; sizes.len()];
        let (mut x, mut y, mut shelf) = (0.0f64, 0.0f64, 0.0f64);
        for &i in &order {
            let r = sizes[i] * s + DVec2::splat(2.0 * pad);
            if r.x > res {
                return None;
            }
            if x + r.x > res {
                y += shelf;
                x = 0.0;
                shelf = 0.0;
            }
            if y + r.y > res {
                return None;
            }
            out[i] = DVec2::new(x, y);
            x += r.x;
            shelf = shelf.max(r.y);
        }
        Some(out)
    };
    while place(0.0, res).is_none() {
        res *= 2.0;
        if res > MAX_PACK_RES {
            return Err("Too many UV charts to pack".into());
        }
    }
    let largest = sizes.iter().map(|s| s.max_element()).fold(0.0f64, f64::max);
    if largest <= 0.0 {
        return Ok((0.0, res, place(0.0, res).unwrap_or_default()));
    }
    let (mut lo, mut hi) = (0.0f64, (res - 2.0 * pad).max(0.0) / largest);
    if place(hi, res).is_some() {
        lo = hi;
    }
    for _ in 0..60 {
        if hi - lo <= lo * 1e-6 {
            break;
        }
        let mid = 0.5 * (lo + hi);
        if place(mid, res).is_some() {
            lo = mid
        } else {
            hi = mid
        }
    }
    Ok((lo, res, place(lo, res).unwrap_or_default()))
}

/// `operandi.uv.atlas` arguments; unnamed ones keep `base`'s values.
pub fn atlas_args(
    a: &serde_json::Map<String, serde_json::Value>,
    base: &AtlasOptions,
) -> Result<AtlasOptions> {
    use serde_json::Value;
    let mut o = base.clone();
    let number = |k: &str| -> Result<Option<f64>> {
        match a.get(k).filter(|v| !v.is_null()) {
            None => Ok(None),
            Some(Value::Number(n)) => n.as_f64().map(Some).ok_or(format!("{k} is a number")),
            Some(Value::String(s)) => s
                .trim()
                .parse()
                .map(Some)
                .map_err(|_| format!("{k} is a number")),
            _ => Err(format!("{k} is a number")),
        }
    };
    if let Some(v) = a.get("name").filter(|v| !v.is_null()) {
        let name = v.as_str().ok_or("name is a text")?.trim();
        o.uv_map = (!name.is_empty()).then(|| name.to_owned());
    }
    if let Some(v) = number("angle")? {
        o.angle = v;
    }
    if let Some(v) = number("gaps")? {
        o.gaps = v;
    }
    if let Some(v) = number("resolution")? {
        if v.fract() != 0.0 || v < 0.0 {
            return Err("resolution is a whole number".into());
        }
        o.resolution = v as u32;
    }
    o.validate()?;
    Ok(o)
}

impl Editor {
    /// Atlas Unwrap of the active mesh's visible polygons, one undo step.
    pub(crate) fn uv_atlas(&mut self, options: AtlasOptions) -> Result<()> {
        self.edge_insert_target("Atlas Unwrap")?;
        options.validate()?;
        self.uv_atlas_preset = AtlasOptions {
            uv_map: None,
            ..options.clone()
        };
        let hidden = self.uv_hidden_faces();
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let map = match &options.uv_map {
            Some(name) if !name.trim().is_empty() => name.trim().to_owned(),
            _ => crate::uv_tools::target_uv_map(mesh, None),
        };
        let (out, _) = atlas_mesh(mesh, &map, &hidden, &options)?;
        self.publish_uv_edit(out, "Atlas Unwrap")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No two UV triangles of different polygons overlap (checked by
    /// sampling a grid), and every UV is in 0..1.
    fn check(mesh: &Mesh, map: &str) {
        let rows = &mesh.uv_maps[map];
        let mut tris = Vec::new();
        for (f, row) in rows.iter().enumerate() {
            let uv: Vec<DVec2> = row.iter().map(|u| u.unwrap()).collect();
            assert!(
                uv.iter()
                    .all(|u| (0.0..=1.0).contains(&u.x) && (0.0..=1.0).contains(&u.y))
            );
            for t in mesh.surface_triangles(f).unwrap() {
                tris.push((f, t.map(|c| uv[c])));
            }
        }
        let inside = |p: DVec2, t: &[DVec2; 3]| {
            let d = |a: DVec2, b: DVec2| (b - a).perp_dot(p - a);
            let (x, y, z) = (d(t[0], t[1]), d(t[1], t[2]), d(t[2], t[0]));
            (x > 1e-9 && y > 1e-9 && z > 1e-9) || (x < -1e-9 && y < -1e-9 && z < -1e-9)
        };
        let mut covered = 0;
        for i in 0..200 {
            for j in 0..200 {
                let p = DVec2::new((i as f64 + 0.5) / 200.0, (j as f64 + 0.5) / 200.0);
                let hits: Vec<usize> = tris
                    .iter()
                    .filter(|(_, t)| inside(p, t))
                    .map(|(f, _)| *f)
                    .collect();
                assert!(hits.len() <= 1, "overlap at {p}: {hits:?}");
                covered += hits.len();
            }
        }
        assert!(covered > 200 * 200 / 5, "coverage {covered}");
    }

    #[test]
    fn a_box_unwraps_into_six_charts_without_overlaps() {
        let mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::new(2.0, 1.0, 0.5), 1).unwrap();
        let (out, charts) = atlas_mesh(&mesh, "Texture", &[], &AtlasOptions::default()).unwrap();
        assert_eq!(charts, 6);
        check(&out, "Texture");
        // Same density everywhere: each face's UV area over its real area.
        let rows = &out.uv_maps["Texture"];
        let ratio: Vec<f64> = (0..6)
            .map(|f| {
                let u: Vec<DVec2> = rows[f].iter().map(|u| u.unwrap()).collect();
                let a = (0..u.len())
                    .map(|i| u[i].perp_dot(u[(i + 1) % u.len()]))
                    .sum::<f64>()
                    .abs()
                    * 0.5;
                a / polygon_area(&mesh, f)
            })
            .collect();
        assert!(
            ratio.iter().all(|r| (r / ratio[0] - 1.0).abs() < 1e-6),
            "{ratio:?}"
        );
    }

    #[test]
    fn a_sphere_keeps_faces_within_the_angle_and_hidden_polygons_alone() {
        let mesh = crate::globe_geometry(
            DVec3::ZERO,
            crate::RadialParameters {
                radius: DVec3::ONE,
                sides: 24,
                segments: 12,
            },
        )
        .unwrap();
        let mut hidden = vec![false; mesh.faces.len()];
        hidden[0] = true;
        let mut with_map = mesh.clone();
        let original: Vec<Vec<Option<DVec2>>> = mesh
            .faces
            .iter()
            .map(|f| vec![Some(DVec2::new(0.5, 0.5)); f.vertices.len()])
            .collect();
        with_map.uv_maps.insert("Texture".into(), original.clone());
        let (out, charts) =
            atlas_mesh(&with_map, "Texture", &hidden, &AtlasOptions::default()).unwrap();
        assert!(charts > 2);
        assert_eq!(out.uv_maps["Texture"][0], original[0]);
        let mut visible = out.clone();
        let keep: Vec<usize> = (1..mesh.faces.len()).collect();
        let rows = &out.uv_maps["Texture"];
        visible.uv_maps.insert(
            "Texture".into(),
            keep.iter().map(|&f| rows[f].clone()).collect(),
        );
        visible.faces = keep
            .iter()
            .map(|&f| mesh.faces[f].clone())
            .collect::<Vec<_>>()
            .into();
        check(&visible, "Texture");
    }

    #[test]
    fn bad_options_and_nothing_visible_are_refused() {
        let mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        let o = AtlasOptions {
            angle: 0.0,
            ..Default::default()
        };
        assert!(atlas_mesh(&mesh, "Texture", &[], &o).is_err());
        assert!(atlas_mesh(&mesh, "Texture", &[true; 6], &AtlasOptions::default()).is_err());
        let mut a = serde_json::Map::new();
        a.insert("gaps".into(), serde_json::Value::from(100));
        assert!(atlas_args(&a, &AtlasOptions::default()).is_err());
    }
}
