//! UV symmetry (E10, BEHAVIOR-UV-TOOLS §10, UVT-70/71; UVT-136 asked for
//! it to be built before tools in a UV view mirror their edits).
//!
//! - [`UvSymmetry`]: the UV Symmetry layer: Off, U or V, and the line's
//!   Center. In a UV view a tool's edit to a UV position is copied, mirrored
//!   across the line, to its partner: the position at the mirrored place
//!   (matched by place, as 3D symmetry matches by position). Never 3D
//!   symmetry (UVT-136).
//! - Center (button): welds UV positions lying on the line.
//! - [`symmetrize`]: copies UVs between 3D mirror polygons, mirrored across
//!   the UV line.
//!
//! Provisional (docs/SPECIFICATION_QUESTIONS.md UVY-P01..P04): Center
//! starts at 0.5; partners are found within `PARTNER_TOLERANCE`; when both
//! partners moved, the one that moved further drives; Center welds
//! positions within `WELD_TOLERANCE` of the line; Symmetrize's Align to
//! Symmetry Axis isn't built.
use crate::uv_edit::UvAxis;
use crate::{Mesh, Result};
use glam::{DVec2, DVec3};
use std::collections::HashMap;

/// How near a UV position must be to the mirrored place to be a partner.
pub const PARTNER_TOLERANCE: f64 = 1e-5;
/// How near the line a UV position must be for Center to weld it.
pub const WELD_TOLERANCE: f64 = 0.005;

/// The UV Symmetry layer (UVT-70).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct UvSymmetry {
    pub axis: Option<UvAxis>,
    /// The line's position on its axis (UVY-P01: 0.5 start).
    pub center: f64,
}
impl Default for UvSymmetry {
    fn default() -> Self {
        Self {
            axis: None,
            center: 0.5,
        }
    }
}
impl UvSymmetry {
    fn index(axis: UvAxis) -> usize {
        match axis {
            UvAxis::U => 0,
            UvAxis::V => 1,
        }
    }
    /// `p` mirrored across the line.
    pub fn mirror(&self, p: DVec2) -> DVec2 {
        let Some(axis) = self.axis else {
            return p;
        };
        let mut q = p;
        let i = Self::index(axis);
        q[i] = 2. * self.center - p[i];
        q
    }
    pub fn validate(&self) -> Result<()> {
        if !self.center.is_finite() {
            return Err("UV Symmetry's Center must be a number".into());
        }
        Ok(())
    }
}

fn key(p: DVec2) -> (i64, i64) {
    let q = |x: f64| (x / PARTNER_TOLERANCE).round() as i64;
    (q(p.x), q(p.y))
}

/// Each point's partner in `start` (UV positions as (U, V, 0)): the point
/// at its mirrored place, or `None`. A point on the line is its own.
pub fn partners(start: &[DVec3], s: &UvSymmetry) -> Vec<Option<usize>> {
    let mut at: HashMap<(i64, i64), usize> = HashMap::with_capacity(start.len());
    for (i, p) in start.iter().enumerate() {
        at.entry(key(p.truncate())).or_insert(i);
    }
    start
        .iter()
        .map(|p| {
            let m = s.mirror(p.truncate());
            // Look at the cell and its neighbours, so rounding never hides
            // a partner sitting on a cell edge.
            let (kx, ky) = key(m);
            (-1..=1)
                .flat_map(|dx| (-1..=1).map(move |dy| (kx + dx, ky + dy)))
                .filter_map(|k| at.get(&k).copied())
                .find(|&j| start[j].truncate().distance(m) <= PARTNER_TOLERANCE * 2.)
        })
        .collect()
}

/// UVT-70: after a tool moved `moved` from `start`, each moved point's
/// partner follows, mirrored; when both moved, the one that moved further
/// drives (UVY-P02). Points on the line stay on it.
pub fn mirror_edit(start: &[DVec3], moved: &mut [DVec3], s: &UvSymmetry) {
    let Some(axis) = s.axis else {
        return;
    };
    let i_axis = UvSymmetry::index(axis);
    let pairs = partners(start, s);
    let shift: Vec<f64> = start
        .iter()
        .zip(moved.iter())
        .map(|(a, b)| a.distance_squared(*b))
        .collect();
    let before: Vec<DVec3> = moved.to_vec();
    for (i, partner) in pairs.iter().enumerate() {
        let Some(j) = *partner else {
            continue;
        };
        if i == j {
            // On the line: it may slide along it, not off it.
            moved[i][i_axis] = s.center;
            continue;
        }
        if shift[i] > shift[j] {
            let m = s.mirror(before[i].truncate());
            moved[j] = DVec3::new(m.x, m.y, before[j].z);
        }
    }
}

impl crate::Editor {
    /// Center (UVT-70): UV positions within `WELD_TOLERANCE` of the line
    /// land on it, and positions of one mesh point that meet there become
    /// one (their average along the line). One undo step.
    pub(crate) fn uv_symmetry_center(&mut self) -> Result<()> {
        self.require_idle()?;
        let s = self.uv_symmetry;
        let axis = s.axis.ok_or("Turn UV Symmetry on (U or V) first")?;
        let i_axis = UvSymmetry::index(axis);
        let other = 1 - i_axis;
        let before = self.scene.clone();
        let item = self.scene.active().ok_or("Select a mesh")?;
        let id = item.id;
        let mut mesh: Mesh = (*item.mesh).clone();
        let map = crate::uv_maps::current_map(&mesh).ok_or("The mesh has no UV map yet")?;
        let rows = mesh.uv_maps.get_mut(&map).expect("current map");
        // Mesh point -> the corners on the line, and their mean along it.
        let mut on_line: HashMap<u32, Vec<(usize, usize)>> = HashMap::new();
        for (f, row) in rows.iter().enumerate() {
            for (c, uv) in row.iter().enumerate() {
                if let Some(p) = uv
                    && (p[i_axis] - s.center).abs() <= WELD_TOLERANCE
                {
                    on_line
                        .entry(mesh.faces[f].vertices[c])
                        .or_default()
                        .push((f, c));
                }
            }
        }
        if on_line.is_empty() {
            return Err("Center: no UVs on the symmetry line".into());
        }
        for corners in on_line.values() {
            let mean = corners
                .iter()
                .map(|&(f, c)| rows[f][c].unwrap()[other])
                .sum::<f64>()
                / corners.len() as f64;
            for &(f, c) in corners {
                let mut p = DVec2::ZERO;
                p[i_axis] = s.center;
                p[other] = mean;
                rows[f][c] = Some(p);
            }
        }
        mesh.attributes.point_uvs.remove(&map);
        mesh.attributes.uv_records.remove(&map);
        if mesh == *self.scene.active().unwrap().mesh {
            return Err("Center: the UVs on the line are already joined".into());
        }
        self.scene.item_mut(id).ok_or("Missing mesh")?.mesh = mesh.into();
        if let Err(e) = self.scene.validate() {
            self.scene = before;
            return Err(e);
        }
        self.record("UV Symmetry Center".into(), before);
        Ok(())
    }

    /// `Command::UvSymmetrize`: one undo step.
    pub(crate) fn uv_symmetrize(&mut self, o: SymmetrizeOptions) -> Result<()> {
        self.require_idle()?;
        let before = self.scene.clone();
        let item = self.scene.active().ok_or("Select a mesh")?;
        let id = item.id;
        let mut mesh: Mesh = (*item.mesh).clone();
        let map = crate::uv_maps::current_map(&mesh).ok_or("The mesh has no UV map yet")?;
        let line = UvSymmetry {
            axis: Some(o.mirror_axis),
            center: self.uv_symmetry.center,
        };
        let changed = symmetrize(&mut mesh, &map, &o, &line)?;
        if changed == 0 {
            return Err("Symmetrize: no polygons have a mirror partner across that axis".into());
        }
        self.scene.item_mut(id).ok_or("Missing mesh")?.mesh = mesh.into();
        if let Err(e) = self.scene.validate() {
            self.scene = before;
            return Err(e);
        }
        self.record("Symmetrize UVs".into(), before);
        Ok(())
    }
}

/// Symmetrize's dialog (UVT-71).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SymmetrizeOptions {
    /// The 3D mirror axis: 0 X, 1 Y, 2 Z.
    pub axis: usize,
    /// Positive to Negative copies from the + side.
    pub positive_to_negative: bool,
    pub mirror_axis: UvAxis,
}
impl Default for SymmetrizeOptions {
    fn default() -> Self {
        Self {
            axis: 0,
            positive_to_negative: true,
            mirror_axis: UvAxis::U,
        }
    }
}

/// UVT-71: each polygon on the target side gets its 3D mirror partner's
/// UVs, mirrored across `line`. Returns how many polygons changed.
pub fn symmetrize(
    mesh: &mut Mesh,
    map: &str,
    o: &SymmetrizeOptions,
    line: &UvSymmetry,
) -> Result<usize> {
    if o.axis > 2 {
        return Err("Symmetry Axis is X, Y or Z".into());
    }
    let q = |p: DVec3| {
        let r = |x: f64| (x / 1e-5).round() as i64;
        (r(p.x), r(p.y), r(p.z))
    };
    let mut point_at: HashMap<(i64, i64, i64), u32> = HashMap::new();
    for (i, p) in mesh.vertices.iter().enumerate() {
        point_at.entry(q(*p)).or_insert(i as u32);
    }
    let mirror_point = |v: u32| -> Option<u32> {
        let mut p = mesh.vertices[v as usize];
        p[o.axis] = -p[o.axis];
        point_at.get(&q(p)).copied()
    };
    // Polygons by their sorted point set.
    let mut by_points: HashMap<Vec<u32>, usize> = HashMap::new();
    for (f, face) in mesh.faces.iter().enumerate() {
        let mut k = face.vertices.clone();
        k.sort_unstable();
        by_points.insert(k, f);
    }
    let rows = mesh
        .uv_maps
        .get(map)
        .ok_or("The mesh has no UV map yet")?
        .clone();
    let mut out = rows.clone();
    let mut changed = 0;
    for (f, face) in mesh.faces.iter().enumerate() {
        let centre = face
            .vertices
            .iter()
            .map(|&v| mesh.vertices[v as usize][o.axis])
            .sum::<f64>()
            / face.vertices.len().max(1) as f64;
        let target = if o.positive_to_negative {
            centre < -1e-9
        } else {
            centre > 1e-9
        };
        if !target {
            continue;
        }
        let Some(mirrored): Option<Vec<u32>> =
            face.vertices.iter().map(|&v| mirror_point(v)).collect()
        else {
            continue;
        };
        let mut k = mirrored.clone();
        k.sort_unstable();
        let Some(&g) = by_points.get(&k) else {
            continue;
        };
        let partner = &mesh.faces[g].vertices;
        let Some(src) = rows.get(g).filter(|r| r.iter().all(Option::is_some)) else {
            continue;
        };
        let row: Option<Vec<Option<DVec2>>> = mirrored
            .iter()
            .map(|m| {
                let c = partner.iter().position(|x| x == m)?;
                Some(Some(line.mirror(src[c].unwrap())))
            })
            .collect();
        if let Some(row) = row
            && out[f] != row
        {
            out[f] = row;
            changed += 1;
        }
    }
    mesh.uv_maps.insert(map.to_owned(), out);
    mesh.attributes.point_uvs.remove(map);
    mesh.attributes.uv_records.remove(map);
    Ok(changed)
}
