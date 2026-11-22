//! Duplicate tools (specifications/duplicate-tools, v1 2026-11-08): Mirror,
//! Clone, Array, Radial Array, and the shared parts Lathe uses
//! (`duplicate_sweep.rs`).
//!
//! Every tool is a **placer** (where copies go: a mirror plane, a line, a
//! grid, a circle) and a **copier** (DS-01/02). The placers here turn their
//! settings into world-space transforms, one per copy; the Copies copier
//! (`copy_faces`) puts a copy of the source polygons at each and welds
//! them (DS-20..23). Lathe's Sweep copier joins the positions instead.
//!
//! Provisional readings (docs/SPECIFICATION_QUESTIONS.md, DUP-Q*): new
//! points of one copy never weld to each other (as DS-23 protects the
//! source's own close points); two polygons that end up on the same points
//! after a weld (an internal wall) are both removed, for every tool, since
//! a mesh edge can't hold more than two polygons.
use crate::clipboard::{KeptFace, subset, whole_faces};
use crate::{Mesh, Mode, Result, Scene};
use glam::{DMat3, DMat4, DQuat, DVec3};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// DS-51: more new polygons than this stops the preview.
pub const NEW_POLYGON_LIMIT: usize = 1_000_000;
/// DS-22: Distance 0 welds points this close ("exactly on top").
pub const WELD_EXACT: f64 = 1e-6;
/// DS-50: what the panel says when there is nothing to copy.
pub const NOTHING_TO_COPY: &str = "Nothing to copy: select polygons or an item.";

/// The Copies copier's options (DS-20..23).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Copies {
    pub replace_source: bool,
    pub flip: bool,
    pub merge: bool,
    /// Merge Distance in metres (world).
    pub distance: f64,
    /// The Item section: what item copies are and where they go (DX-31).
    pub item: crate::duplicate_items::ItemCopies,
}
impl Default for Copies {
    fn default() -> Self {
        Self {
            replace_source: false,
            flip: false,
            merge: false,
            distance: 0.,
            item: Default::default(),
        }
    }
}

/// Mirror's plane modes (MR-23). Three Points is later (MR-24).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MirrorMode {
    #[default]
    Axis,
    FreeRotation,
}

/// Mirror: the plane placer and its copier (BEHAVIOR-MIRROR.md).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mirror {
    /// 0 X, 1 Y, 2 Z: the plane's facing direction (MR-20).
    pub axis: usize,
    /// Degrees the plane turns about its hinge (Left); 0 is the plain
    /// reflection (MR-21, probe).
    pub angle: f64,
    pub mode: MirrorMode,
    /// A point on the plane (world).
    pub center: DVec3,
    pub left: DVec3,
    pub up: DVec3,
    /// MR-33: the centre grip snaps to the world axes' zero.
    pub snap_world_axis: bool,
    pub copies: Copies,
}
impl Default for Mirror {
    fn default() -> Self {
        let (left, up) = Mirror::axis_frame(0);
        Self {
            axis: 0,
            angle: 0.,
            mode: MirrorMode::Axis,
            center: DVec3::ZERO,
            left,
            up,
            snap_world_axis: true,
            // Merge on, Distance 0 (probe): exact overlaps weld.
            copies: Copies {
                merge: true,
                ..Default::default()
            },
        }
    }
}
impl Mirror {
    /// MR-22 (probe for X, the same rotation for Y and Z): Left and Up for
    /// an Axis; the facing direction is Left × Up and the hinge runs along
    /// Up.
    pub fn axis_frame(axis: usize) -> (DVec3, DVec3) {
        match axis {
            0 => (DVec3::Y, DVec3::Z),
            1 => (DVec3::Z, DVec3::X),
            _ => (DVec3::X, DVec3::Y),
        }
    }
    /// The plane's facing direction, Left and Up, all unit length. In Axis
    /// mode they come from Axis; in Free Rotation from Left and Up, with Up
    /// corrected to be at right angles to Left (MR-23).
    pub fn frame(&self) -> Result<(DVec3, DVec3, DVec3)> {
        let (left, up) = match self.mode {
            MirrorMode::Axis => Self::axis_frame(self.axis),
            MirrorMode::FreeRotation => {
                let left = self.left.normalize_or_zero();
                let up = (self.up - left * self.up.dot(left)).normalize_or_zero();
                if left == DVec3::ZERO || up == DVec3::ZERO {
                    return Err("Mirror: Left and Up must point in different directions".into());
                }
                (left, up)
            }
        };
        Ok((left.cross(up), left, up))
    }
    /// MR-21 (probe): the reflection across the plane turned about its
    /// hinge line (through Center along Left) by Angle.
    pub fn transform(&self) -> Result<DMat4> {
        if !(-360. ..=360.).contains(&self.angle) {
            return Err("Mirror Angle must be from −360° to 360°".into());
        }
        if !self.center.is_finite() {
            return Err("Mirror Center must be a finite point".into());
        }
        let n = self.plane_normal()?.normalize();
        // I − 2nnᵀ about the centre.
        let reflect = DMat3::from_cols(
            DVec3::X - 2. * n.x * n,
            DVec3::Y - 2. * n.y * n,
            DVec3::Z - 2. * n.z * n,
        );
        Ok(DMat4::from_translation(self.center)
            * DMat4::from_mat3(reflect)
            * DMat4::from_translation(-self.center))
    }
    /// The hinge turn about Left.
    pub fn hinge(&self) -> Result<DQuat> {
        let (_, left, _) = self.frame()?;
        Ok(DQuat::from_axis_angle(left, self.angle.to_radians()))
    }
    /// The plane's normal after the hinge turn.
    pub fn plane_normal(&self) -> Result<DVec3> {
        let (normal, _, _) = self.frame()?;
        Ok(self.hinge()? * normal)
    }
}

/// Clone: copies in a line (BEHAVIOR-CLONE.md).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CloneLine {
    /// Copies made, not counting the original (CL-10).
    pub count: u32,
    /// World step from one copy to the next.
    pub offset: DVec3,
    /// Size change per step, percent, compounding (CL-12: 100 = none).
    pub scale: DVec3,
    /// Turn per step, degrees, compounding (CL-13).
    pub rotate: DVec3,
    pub copies: Copies,
}
impl Default for CloneLine {
    fn default() -> Self {
        Self {
            count: 1,
            offset: DVec3::ZERO,
            scale: DVec3::splat(100.),
            rotate: DVec3::ZERO,
            copies: Copies::default(),
        }
    }
}

/// Array: copies in a 3D grid (BEHAVIOR-ARRAY.md).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Array {
    /// Cells along X, Y, Z, the original's included (AR-10).
    pub count: [u32; 3],
    pub offset: DVec3,
    pub jitter: DVec3,
    /// Percent per axis (AR-14).
    pub scale: DVec3,
    /// Offset spans first to last cell (AR-12).
    pub between: bool,
    pub copies: Copies,
}
impl Default for Array {
    fn default() -> Self {
        Self {
            count: [2, 1, 2],
            offset: DVec3::ONE,
            jitter: DVec3::ZERO,
            scale: DVec3::splat(100.),
            between: false,
            copies: Copies::default(),
        }
    }
}

/// The Circle placer Lathe and Radial Array share (LT section 3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    /// Lathe: segments; Radial Array: pieces, the original included.
    pub count: u32,
    /// The Axis dropdown, 0 X, 1 Y, 2 Z.
    pub axis: usize,
    /// A point on the spin axis (world).
    pub center: DVec3,
    /// The axis direction (Axis X/Y/Z rows; LT-32).
    pub direction: DVec3,
    /// Degrees.
    pub start: f64,
    pub end: f64,
    /// Rise along the axis per step (metres).
    pub offset: f64,
}
impl Circle {
    pub fn new(count: u32) -> Self {
        Self {
            count,
            axis: 1,
            center: DVec3::ZERO,
            direction: DVec3::Y,
            start: 0.,
            end: 360.,
            offset: 0.,
        }
    }
    /// The unit axis direction.
    pub fn axis_direction(&self) -> Result<DVec3> {
        let d = self.direction.normalize_or_zero();
        if d == DVec3::ZERO || !d.is_finite() {
            return Err("The axis direction (Axis X, Y, Z) can't be zero".into());
        }
        Ok(d)
    }
    /// End − Start is a whole, non-zero number of turns.
    pub fn whole_turns(&self) -> bool {
        let span = (self.end - self.start) / 360.;
        span.abs() > 1e-9 && (span - span.round()).abs() < 1e-9
    }
    /// Turned `degrees` about the axis and raised `rise`, as a world matrix.
    pub fn turn(&self, degrees: f64, rise: f64) -> Result<DMat4> {
        let d = self.axis_direction()?;
        Ok(DMat4::from_translation(self.center + d * rise)
            * DMat4::from_quat(DQuat::from_axis_angle(d, degrees.to_radians()))
            * DMat4::from_translation(-self.center))
    }
    /// The world point of the axis nearest `p`.
    pub fn foot(&self, p: DVec3) -> DVec3 {
        let d = self.direction.normalize_or_zero();
        self.center + d * (p - self.center).dot(d)
    }
    fn check(&self) -> Result<()> {
        if !(self.center.is_finite()
            && self.start.is_finite()
            && self.end.is_finite()
            && self.offset.is_finite())
        {
            return Err("Circle values must be finite".into());
        }
        if self.count == 0 {
            return Err("Count must be at least 1".into());
        }
        self.axis_direction().map(|_| ())
    }
}

/// Radial Array (BEHAVIOR-RADIAL-ARRAY.md).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadialArray {
    pub circle: Circle,
    pub copies: Copies,
}
impl Default for RadialArray {
    fn default() -> Self {
        Self {
            circle: Circle::new(8),
            copies: Copies::default(),
        }
    }
}

/// Which UV direction runs round the axis (LT-27).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UvsAround {
    #[default]
    U,
    V,
}

/// Lathe's Sweep copier (LT section 3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sweep {
    pub flip: bool,
    pub cap_start: bool,
    pub cap_end: bool,
    pub uvs_around: UvsAround,
}
impl Default for Sweep {
    /// Caps on and UVs Around V (probe).
    fn default() -> Self {
        Self {
            flip: false,
            cap_start: true,
            cap_end: true,
            uvs_around: UvsAround::V,
        }
    }
}

/// Lathe (BEHAVIOR-LATHE.md).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lathe {
    pub circle: Circle,
    pub sweep: Sweep,
}
impl Default for Lathe {
    fn default() -> Self {
        Self {
            circle: Circle::new(8),
            sweep: Sweep::default(),
        }
    }
}

/// Every Duplicate tool's settings. Each tool keeps its own; `placed` is
/// the session's: false until the first click or Apply (DS-30/31).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DuplicateParameters {
    pub placed: bool,
    pub mirror: Mirror,
    pub clone: CloneLine,
    pub array: Array,
    pub radial: RadialArray,
    pub lathe: Lathe,
    /// Curve Extrude's path and Sweep (path_tools.rs).
    pub curve_extrude: crate::path_tools::PathTool,
    /// Curve Clone's path and Clone (path_tools.rs).
    pub curve_clone: crate::path_tools::PathTool,
    /// Scatter Clone (generator_ops.rs, GO-01..07).
    pub scatter: crate::generator_ops::ScatterTool,
    /// Symmetrize's tool form (symmetrize_tool.rs, DX-40..45).
    pub symmetrize: crate::symmetrize_tool::Symmetrize,
    /// The open button's copier (duplicate_copier.rs); `None` for the
    /// plain buttons. Set on every activation from the open preset row.
    pub copier: Option<crate::duplicate_copier::Copier>,
    /// The Replica buttons' Replica section (replicator.rs, G8 RP-6).
    pub replica: crate::replicator::ReplicaOptions,
}
impl DuplicateParameters {
    /// The settings kept for the next session (a drawn path is not).
    pub fn options_only(&self) -> Self {
        Self {
            placed: false,
            curve_extrude: self.curve_extrude.options_only(),
            curve_clone: self.curve_clone.options_only(),
            ..self.clone()
        }
    }
    /// The copier options of `tool` (Lathe has none: its seam always welds).
    pub fn copies(&self, tool: crate::editor::Tool) -> Option<&Copies> {
        use crate::editor::Tool;
        match tool {
            Tool::Mirror => Some(&self.mirror.copies),
            Tool::Clone => Some(&self.clone.copies),
            Tool::Array => Some(&self.array.copies),
            Tool::RadialArray => Some(&self.radial.copies),
            Tool::CurveClone => Some(&self.curve_clone.clone.copies),
            Tool::ScatterClone => Some(&self.scatter.copies),
            _ => None,
        }
    }
}

impl crate::editor::Tool {
    /// The five Duplicate tab tools.
    pub fn is_duplicate(self) -> bool {
        self.card().family == crate::tool_card::Family::Duplicate
    }
}

// ---- random numbers (CL-14: a fixed seed per copy) ----

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// A number in −1..1, the same every time for the same slot and channel.
pub fn jitter(salt: u64, slot: u64, channel: u64) -> f64 {
    let x = splitmix(salt ^ splitmix(slot.wrapping_mul(7919) ^ splitmix(channel)));
    (x >> 11) as f64 / (1u64 << 53) as f64 * 2. - 1.
}

// ---- placers: one world transform per copy ----

/// About `center`: `m` with `center` held still.
fn about(center: DVec3, m: DMat4) -> DMat4 {
    DMat4::from_translation(center) * m * DMat4::from_translation(-center)
}

/// Clone's copies (CL-10..15, v1.2), slot 0 included with Replace Source.
/// Copy *i* sits *i* × Offset along, sized Scale^*i* and turned *i* ×
/// Rotate (X, then Y, then Z) about its own bounding-box centre; the
/// steps compound and nothing is random.
pub fn clone_transforms(c: &CloneLine, centre: DVec3) -> Result<Vec<DMat4>> {
    if !(c.offset.is_finite() && c.scale.is_finite() && c.rotate.is_finite()) {
        return Err("Clone values must be finite".into());
    }
    if c.scale.min_element() <= 0. {
        return Err("Clone Scale must be more than 0 %".into());
    }
    let first = if c.copies.replace_source { 0 } else { 1 };
    (first..=c.count as u64)
        .map(|i| {
            let n = i as f64;
            let scale = (c.scale / 100.).powf(n);
            if !scale.is_finite() || scale.min_element() < 1e-12 {
                return Err("Clone Scale shrinks the copies to nothing; use fewer clones or a Scale nearer 100 %".to_string());
            }
            let turn = DQuat::from_rotation_z((n * c.rotate.z).to_radians())
                * DQuat::from_rotation_y((n * c.rotate.y).to_radians())
                * DQuat::from_rotation_x((n * c.rotate.x).to_radians());
            let at = c.offset * n;
            Ok(DMat4::from_translation(at)
                * about(centre, DMat4::from_quat(turn) * DMat4::from_scale(scale)))
        })
        .collect()
}

/// Array's cells (AR-10..14), the first cell only with Replace Source.
pub fn array_transforms(a: &Array, centre: DVec3) -> Result<Vec<DMat4>> {
    if !(a.offset.is_finite() && a.jitter.is_finite() && a.scale.is_finite()) {
        return Err("Array values must be finite".into());
    }
    if a.count.contains(&0) {
        return Err("Array Count must be at least 1 on each axis".into());
    }
    let step = DVec3::from_array(std::array::from_fn(|axis| {
        let n = a.count[axis];
        if a.between {
            if n > 1 {
                a.offset[axis] / (n - 1) as f64
            } else {
                0.
            }
        } else {
            a.offset[axis]
        }
    }));
    let scale = a.scale / 100.;
    if scale.abs().min_element() < 1e-6 {
        return Err("Array Scale can't be 0 %".into());
    }
    let mut out = Vec::new();
    let mut slot = 0u64;
    for k in 0..a.count[2] {
        for j in 0..a.count[1] {
            for i in 0..a.count[0] {
                let first = i == 0 && j == 0 && k == 0;
                slot += 1;
                if first && !a.copies.replace_source {
                    continue;
                }
                let shake = DVec3::new(jitter(2, slot, 0), jitter(2, slot, 1), jitter(2, slot, 2))
                    * a.jitter;
                let at = DVec3::new(i as f64, j as f64, k as f64) * step + shake;
                out.push(DMat4::from_translation(at) * about(centre, DMat4::from_scale(scale)));
            }
        }
    }
    Ok(out)
}

/// RA-10: the slot angles and rises of Radial Array, slot 0 included with
/// Replace Source (where it turns by Start).
pub fn radial_transforms(r: &RadialArray) -> Result<Vec<DMat4>> {
    let c = &r.circle;
    c.check()?;
    let n = c.count;
    let span = c.end - c.start;
    let step = if c.whole_turns() {
        span / n as f64
    } else if n > 1 {
        span / (n - 1) as f64
    } else {
        0.
    };
    let replace = r.copies.replace_source;
    let first = if replace { 0 } else { 1 };
    (first..n)
        .map(|i| {
            let angle = i as f64 * step + if replace { c.start } else { 0. };
            c.turn(angle, i as f64 * c.offset)
        })
        .collect()
}

// ---- the source ----

/// The polygons a Copies tool copies from the active mesh (DS-10/11):
/// selected polygons; in Edge or Vertex mode the polygons touching the
/// selection; nothing selected (or Item mode) means every polygon.
pub fn source_faces(scene: &Scene) -> Vec<usize> {
    let Some(item) = scene.active() else {
        return Vec::new();
    };
    let mesh = &item.mesh;
    let s = &scene.selection;
    let all = || (0..mesh.faces.len()).collect::<Vec<_>>();
    match s.mode {
        Mode::Polygon if !s.faces.is_empty() => {
            let mut f: Vec<usize> = s
                .faces
                .iter()
                .map(|&f| f as usize)
                .filter(|&f| f < mesh.faces.len())
                .collect();
            f.sort_unstable();
            f.dedup();
            f
        }
        Mode::Vertex if !s.vertices.is_empty() => {
            let on: BTreeSet<u32> = s.vertices.iter().copied().collect();
            (0..mesh.faces.len())
                .filter(|&f| mesh.faces[f].vertices.iter().any(|v| on.contains(v)))
                .collect()
        }
        Mode::Edge if !s.edges.is_empty() => {
            let on: BTreeSet<(u32, u32)> =
                s.edges.iter().map(|&(a, b)| (a.min(b), a.max(b))).collect();
            (0..mesh.faces.len())
                .filter(|&f| {
                    mesh.element_spans(f)
                        .any(|(a, b)| on.contains(&(a.min(b), a.max(b))))
                })
                .collect()
        }
        _ => all(),
    }
}

/// The world-space bounding box of `faces`' points of mesh item `id`.
pub fn faces_bounds(scene: &Scene, id: u64, faces: &[usize]) -> Option<(DVec3, DVec3)> {
    let item = scene.item(id)?;
    let world = scene.world_matrix(id).ok()?;
    let mut lo = DVec3::splat(f64::INFINITY);
    let mut hi = DVec3::splat(f64::NEG_INFINITY);
    let mut any = false;
    let mut seen = vec![false; item.mesh.vertices.len()];
    for &f in faces {
        for &v in &item.mesh.faces.get(f)?.vertices {
            if !std::mem::replace(&mut seen[v as usize], true) {
                let p = world.transform_point3(item.mesh.vertices[v as usize]);
                lo = lo.min(p);
                hi = hi.max(p);
                any = true;
            }
        }
    }
    any.then_some((lo, hi))
}

// ---- the Copies copier ----

/// Which polygon came from where, after `copy_faces`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CopyOutcome {
    pub mesh: Mesh,
    /// Each input polygon's index in the result (`None`: removed).
    pub face_map: Vec<Option<u32>>,
    /// Each input point's index in the result.
    pub point_map: Vec<Option<u32>>,
    /// With Replace Source: each source point's slot-0 copy, by input point.
    pub replaced_points: BTreeMap<u32, u32>,
    /// With Replace Source: the slot-0 copies of the source polygons.
    pub replaced_faces: Vec<u32>,
    /// Polygons the copies added.
    pub new_faces: usize,
}

/// How a weld places its point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WeldAt {
    /// On the original point (Clone, Array, Radial Array).
    Original,
    /// The mean of the welded points (Mirror, MR-42: on the plane).
    Mean,
}

/// Reverse a polygon's facing: keep its first corner, reverse the rest,
/// with every per-corner record following its corner.
pub(crate) fn reverse_corners(mesh: &mut Mesh, fi: usize) {
    fn rev<T>(row: &mut [T]) {
        if row.len() > 1 {
            row[1..].reverse();
        }
    }
    rev(&mut mesh.faces[fi].vertices);
    if let Some(uv) = mesh.faces[fi].uv.as_mut() {
        rev(uv);
    }
    for rows in mesh.uv_maps.values_mut() {
        rev(&mut rows[fi]);
    }
    let a = &mut mesh.attributes;
    for rows in a.corner_weights.values_mut() {
        rev(&mut rows[fi]);
    }
    for rows in a.corner_colors.values_mut() {
        rev(&mut rows[fi]);
    }
    for rows in a.corner_normals.values_mut() {
        rev(&mut rows[fi]);
    }
    for rows in a.uv_records.values_mut() {
        rev(&mut rows[fi]);
    }
    for map in a.typed_maps.values_mut() {
        if let Some(row) = map.corners.get_mut(fi) {
            rev(row);
        }
    }
}

/// `src` moved by `m` (local space). A mirroring `m` turns the copy
/// inside-out, so its polygons are reversed (MR-41, AR-14); `flip`
/// reverses them again (DS-21). Lines have no facing and keep their order.
pub(crate) fn transformed_copy(src: &Mesh, m: DMat4, flip: bool) -> Mesh {
    let mut out = src.clone();
    for v in &mut out.vertices {
        *v = m.transform_point3(*v);
    }
    let linear = DMat3::from_mat4(m);
    for map in out.attributes.relative_morphs.values_mut() {
        for d in map.values_mut() {
            *d = linear * *d;
        }
    }
    let normals = linear.inverse().transpose();
    let sign = if flip { -1. } else { 1. };
    for rows in out.attributes.corner_normals.values_mut() {
        for n in rows.iter_mut().flatten().flatten() {
            *n = (normals * *n).normalize_or_zero() * sign;
        }
    }
    if (linear.determinant() < 0.) != flip {
        for fi in 0..out.faces.len() {
            if !out.is_polyline(fi) {
                reverse_corners(&mut out, fi);
            }
        }
    }
    out
}

/// Join many meshes into one, pairwise so each point is validated only a
/// few times (`append_mesh` checks both sides on every call).
fn concat(mut parts: Vec<Mesh>) -> Result<Mesh> {
    if parts.is_empty() {
        return Ok(Mesh::default());
    }
    while parts.len() > 1 {
        let mut next = Vec::with_capacity(parts.len().div_ceil(2));
        let mut it = parts.into_iter();
        while let Some(mut a) = it.next() {
            if let Some(b) = it.next() {
                crate::append_mesh(&mut a, b)?;
            }
            next.push(a);
        }
        parts = next;
    }
    Ok(parts.pop().unwrap_or_default())
}

/// Points of `mesh` that no polygon uses.
fn loose_points(mesh: &Mesh) -> BTreeSet<u32> {
    mesh.used_points()
        .iter()
        .enumerate()
        .filter(|(_, u)| !**u)
        .map(|(v, _)| v as u32)
        .collect()
}

/// The Copies copier on one mesh (local space): a copy of `source`'s
/// polygons at each of `transforms`, welded by `copies` (DS-20..23).
pub fn copy_faces(
    mesh: &Mesh,
    source: &[usize],
    transforms: &[DMat4],
    copies: &Copies,
    weld_at: WeldAt,
    distance: f64,
) -> Result<CopyOutcome> {
    let picked: BTreeSet<usize> = source.iter().copied().collect();
    let (src, src_points) = subset(
        mesh,
        &whole_faces(mesh, |f| picked.contains(&f)),
        &BTreeSet::new(),
    )?;
    let added = src.faces.len().saturating_mul(transforms.len());
    if added > NEW_POLYGON_LIMIT {
        return Err(format!("Too many copies ({added} polygons)"));
    }
    // Replace Source: the source goes first (DS-20), its points with it
    // unless another polygon uses them; loose points stay.
    let (base, face_map, point_map) = if copies.replace_source {
        let kept: Vec<KeptFace> = whole_faces(mesh, |f| !picked.contains(&f));
        let face_map = {
            let mut m = vec![None; mesh.faces.len()];
            for (i, k) in kept.iter().enumerate() {
                m[k.source] = Some(i as u32);
            }
            m
        };
        let (base, points) = subset(mesh, &kept, &loose_points(mesh))?;
        (base, face_map, points)
    } else {
        (
            mesh.clone(),
            (0..mesh.faces.len() as u32).map(Some).collect(),
            (0..mesh.vertices.len() as u32).map(Some).collect(),
        )
    };
    let start_point = base.vertices.len();
    let start_face = base.faces.len();
    let parts: Vec<Mesh> = transforms
        .iter()
        .map(|&m| transformed_copy(&src, m, copies.flip))
        .collect();
    let mut out = base;
    crate::append_mesh(&mut out, concat(parts)?)?;
    let mut outcome = CopyOutcome {
        face_map,
        point_map,
        new_faces: added,
        ..Default::default()
    };
    if copies.replace_source && !transforms.is_empty() {
        for (v, new) in src_points.iter().enumerate() {
            if let Some(new) = new {
                outcome
                    .replaced_points
                    .insert(v as u32, start_point as u32 + new);
            }
        }
        outcome.replaced_faces = (0..src.faces.len() as u32)
            .map(|f| start_face as u32 + f)
            .collect();
    }
    if copies.merge {
        let per_copy = src.vertices.len().max(1);
        let tolerance = distance.max(WELD_EXACT);
        let (welded, faces, points) = weld_new(&out, start_point, per_copy, tolerance, weld_at)?;
        out = welded;
        for slot in outcome.face_map.iter_mut() {
            *slot = slot.and_then(|f| faces[f as usize]);
        }
        for slot in outcome.point_map.iter_mut() {
            *slot = slot.and_then(|v| points[v as usize]);
        }
        outcome.replaced_faces = outcome
            .replaced_faces
            .iter()
            .filter_map(|&f| faces[f as usize])
            .collect();
        outcome.replaced_points = outcome
            .replaced_points
            .iter()
            .filter_map(|(&k, &v)| Some((k, points[v as usize]?)))
            .collect();
    }
    out.validate()?;
    outcome.mesh = out;
    Ok(outcome)
}

/// Grid cell of `p` for a weld tolerance.
fn cell(p: DVec3, size: f64) -> (i64, i64, i64) {
    let c = (p / size).floor();
    (c.x as i64, c.y as i64, c.z as i64)
}

/// DS-22/23: weld every new point (index ≥ `start`) to a point within
/// `tolerance`: an original point first (nearest), else a new point of
/// another copy. Two original points never weld, nor two points of one
/// copy (`per_copy` points each). Polygons left on the same points as
/// another (an internal wall, MR-43) are removed in pairs. Returns the mesh
/// with each old polygon's and point's new index.
#[allow(clippy::type_complexity)]
pub(crate) fn weld_new(
    mesh: &Mesh,
    start: usize,
    per_copy: usize,
    tolerance: f64,
    weld_at: WeldAt,
) -> Result<(Mesh, Vec<Option<u32>>, Vec<Option<u32>>)> {
    let n = mesh.vertices.len();
    let used = mesh.used_points();
    // Only points near the copies can weld ("Fast drags": work near the
    // selection), so the grid holds the copies' box and a margin.
    let (mut lo, mut hi) = (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY));
    for p in &mesh.vertices[start.min(n)..] {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let (lo, hi) = (lo - DVec3::splat(tolerance), hi + DVec3::splat(tolerance));
    let mut grid: HashMap<(i64, i64, i64), Vec<u32>> = HashMap::new();
    for (v, p) in mesh.vertices.iter().enumerate() {
        if used[v] && p.cmpge(lo).all() && p.cmple(hi).all() {
            grid.entry(cell(*p, tolerance)).or_default().push(v as u32);
        }
    }
    let copy_of = |v: usize| (v - start) / per_copy;
    // Group of each point; each group holds at most one original point.
    let mut group: Vec<Option<usize>> = vec![None; n];
    let mut groups: Vec<Vec<u32>> = Vec::new();
    for q in start..n {
        if !used[q] {
            continue;
        }
        let p = mesh.vertices[q];
        let (cx, cy, cz) = cell(p, tolerance);
        let mut best_original: Option<(f64, u32)> = None;
        let mut best_new: Option<(f64, u32)> = None;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let Some(list) = grid.get(&(cx + dx, cy + dy, cz + dz)) else {
                        continue;
                    };
                    for &w in list {
                        let w_us = w as usize;
                        if w_us == q {
                            continue;
                        }
                        let d = mesh.vertices[w_us].distance(p);
                        if d > tolerance {
                            continue;
                        }
                        if w_us < start {
                            if best_original.is_none_or(|b| d < b.0) {
                                best_original = Some((d, w));
                            }
                        } else if w_us < q
                            && copy_of(w_us) != copy_of(q)
                            && best_new.is_none_or(|b| d < b.0)
                        {
                            best_new = Some((d, w));
                        }
                    }
                }
            }
        }
        let target = best_original.or(best_new).map(|b| b.1 as usize);
        let Some(t) = target else {
            continue;
        };
        let g = match group[t] {
            Some(g) => g,
            None => {
                groups.push(vec![t as u32]);
                group[t] = Some(groups.len() - 1);
                groups.len() - 1
            }
        };
        // A new point already grouped with another original stays there.
        if group[q].is_none() {
            groups[g].push(q as u32);
            group[q] = Some(g);
        }
    }
    let mut to: Vec<u32> = (0..n as u32).collect();
    for g in &groups {
        let keep = *g.iter().min().expect("non-empty group");
        for &m in g {
            to[m as usize] = keep;
        }
    }
    // Internal walls: two polygons on the same points after the weld.
    let mut walls: HashMap<Vec<u32>, Vec<usize>> = HashMap::new();
    for (fi, f) in mesh.faces.iter().enumerate() {
        if mesh.is_polyline(fi) {
            continue;
        }
        let mut key: Vec<u32> = f.vertices.iter().map(|&v| to[v as usize]).collect();
        key.sort_unstable();
        key.dedup();
        if key.len() >= 3 {
            walls.entry(key).or_default().push(fi);
        }
    }
    let is_new = |fi: usize| mesh.faces[fi].vertices.iter().any(|&v| v as usize >= start);
    let mut gone_faces = BTreeSet::new();
    for faces in walls.values() {
        if faces.len() == 2 && faces.iter().any(|&f| is_new(f)) {
            gone_faces.extend(faces.iter().copied());
        }
    }
    let (walled, face_step) = if gone_faces.is_empty() {
        (
            mesh.clone(),
            (0..mesh.faces.len() as u32).map(Some).collect(),
        )
    } else {
        let kept = whole_faces(mesh, |f| !gone_faces.contains(&f));
        let mut map = vec![None; mesh.faces.len()];
        for (i, k) in kept.iter().enumerate() {
            map[k.source] = Some(i as u32);
        }
        let every: BTreeSet<u32> = (0..n as u32).collect();
        (subset(mesh, &kept, &every)?.0, map)
    };
    let positions: Vec<(Vec<u32>, DVec3)> = groups
        .iter()
        .map(|g| {
            let at = match weld_at {
                WeldAt::Mean => {
                    g.iter().map(|&v| mesh.vertices[v as usize]).sum::<DVec3>() / g.len() as f64
                }
                WeldAt::Original => mesh.vertices[*g.iter().min().unwrap() as usize],
            };
            (g.clone(), at)
        })
        .collect();
    // Which polygons survive the weld (fewer than three corners go).
    let survives = |fi: usize| {
        let f = &walled.faces[fi];
        let mut vs: Vec<u32> = Vec::new();
        for &v in &f.vertices {
            let w = to[v as usize];
            if vs.last() != Some(&w) {
                vs.push(w);
            }
        }
        while vs.len() > 1 && vs.first() == vs.last() {
            vs.pop();
        }
        vs.len() >= if walled.is_polyline(fi) { 2 } else { 3 }
    };
    let alive: Vec<bool> = (0..walled.faces.len()).map(survives).collect();
    let mut after = vec![None; walled.faces.len()];
    let mut next = 0u32;
    for (f, slot) in after.iter_mut().enumerate() {
        if alive[f] {
            *slot = Some(next);
            next += 1;
        }
    }
    let (welded, remap) = if positions.is_empty() {
        (walled, (0..n as u32).map(Some).collect())
    } else {
        crate::edge_commands::weld(&walled, &positions, "Merge Vertices").map_err(|e| {
            format!(
                "{e}. Merge Vertices would join the copies badly here: lower Distance or turn Merge off"
            )
        })?
    };
    let faces = face_step
        .iter()
        .map(|f: &Option<u32>| f.and_then(|f| after[f as usize]))
        .collect();
    Ok((welded, faces, remap))
}

// ---- the scene preview ----

/// One world transform per copy for `tool` (not Lathe), with the source's
/// world bounding-box centre for Clone and Array's pivots.
pub fn transforms(
    tool: crate::editor::Tool,
    p: &DuplicateParameters,
    centre: DVec3,
) -> Result<Vec<DMat4>> {
    use crate::editor::Tool;
    match tool {
        // One copy; with Replace Source it takes the source's place.
        Tool::Mirror => Ok(vec![p.mirror.transform()?]),
        Tool::Clone => clone_transforms(&p.clone, centre),
        Tool::Array => array_transforms(&p.array, centre),
        Tool::RadialArray => radial_transforms(&p.radial),
        Tool::ScatterClone => p
            .scatter
            .generator
            .transforms(centre, p.scatter.copies.replace_source),
        _ => Err("Not a Copies tool".into()),
    }
}

/// The live preview of a Duplicate tool on the base scene (`scene`).
/// Before the first click or Apply nothing changes (DS-30).
pub(crate) fn preview(
    tool: crate::editor::Tool,
    scene: &mut Scene,
    p: &DuplicateParameters,
    next_item_id: u64,
    symmetry: crate::Symmetry,
) -> Result<()> {
    if p.copier == Some(crate::duplicate_copier::Copier::Transform) {
        return crate::duplicate_transform::preview(tool, scene, p);
    }
    // The path tools show their result as soon as the path is long enough.
    match tool {
        crate::editor::Tool::CurveExtrude => {
            return crate::path_tools::preview(true, scene, &p.curve_extrude, symmetry);
        }
        crate::editor::Tool::CurveClone => {
            // Curve / Bezier / B-Spline / Pen Instance (DX-30).
            if crate::duplicate_items::instancing(p) || crate::duplicate_items::replicating(p) {
                return crate::duplicate_items::path_items(scene, p, next_item_id);
            }
            return crate::path_tools::preview(false, scene, &p.curve_clone, symmetry);
        }
        // DX-44: live from the start; the plane is its own, so the
        // Symmetry setting isn't used.
        crate::editor::Tool::Symmetrize => {
            return crate::symmetrize_tool::preview(scene, &p.symmetrize);
        }
        _ => {}
    }
    if !p.placed {
        return Ok(());
    }
    if tool == crate::editor::Tool::Lathe {
        return crate::duplicate_sweep::preview(scene, &p.lathe, symmetry);
    }
    // Mirror is Symmetry's own kind of tool and doesn't use the setting
    // (spec section 7, class G).
    let symmetry = if tool == crate::editor::Tool::Mirror {
        crate::Symmetry::OFF
    } else {
        symmetry
    };
    let copies = *p.copies(tool).ok_or("Not a Copies tool")?;
    // DX-11: Instance copies are items in every selection mode.
    if scene.selection.mode == Mode::Item
        || crate::duplicate_items::instancing(p)
        || crate::duplicate_items::replicating(p)
    {
        return crate::duplicate_items::item_copies(tool, scene, p, next_item_id);
    }
    let ids = scene.active_meshes();
    let ids = if ids.is_empty() {
        scene.active().map(|i| i.id).into_iter().collect()
    } else {
        ids
    };
    for id in ids {
        scene.as_sole_mesh(id, |s| mesh_copies(tool, s, p, &copies, symmetry))?;
    }
    Ok(())
}

/// DS-50: whether `tool` would find nothing to copy in `scene`.
pub fn nothing_to_copy(scene: &Scene) -> bool {
    if scene.selection.mode == Mode::Item {
        return !scene
            .selected_items()
            .iter()
            .any(|id| scene.items.iter().any(|i| i.id == *id && i.is_mesh()));
    }
    scene.active().is_none_or(|i| i.mesh.faces.is_empty())
}

/// Copies of the active mesh's source polygons into the same mesh (DS-13).
/// With Symmetry (class C), the other side's polygons are copied with the
/// mirror image of each placement, so a Clone offset or an Array grid goes
/// outward on both sides (SYM3-12). Nothing selected copies everything with
/// the placement as entered (provisional).
fn mesh_copies(
    tool: crate::editor::Tool,
    scene: &mut Scene,
    p: &DuplicateParameters,
    copies: &Copies,
    symmetry: crate::Symmetry,
) -> Result<()> {
    let Some(item) = scene.active() else {
        return Ok(());
    };
    let id = item.id;
    if source_faces(scene).is_empty() {
        return Ok(());
    }
    let world = scene.world_matrix(id)?;
    if world.determinant() == 0. {
        return Err("Duplicate tools need an invertible item transform".into());
    }
    let local = world.inverse();
    let mirror = crate::symmetry_sides::WorldMirror::new(symmetry, world);
    crate::symmetry_sides::two_pass(scene, symmetry, |scene, mirrored| {
        let source = source_faces(scene);
        if source.is_empty() {
            return Ok(());
        }
        let centre =
            faces_bounds(scene, id, &source).map_or(DVec3::ZERO, |(lo, hi)| (lo + hi) / 2.);
        let m = mirror.filter(|_| mirrored);
        // The other side's placement is the primary one seen in the mirror,
        // pivoting on the mirrored centre.
        let centre = m.map_or(centre, |m| m.point(centre));
        let placed: Vec<DMat4> = transforms(tool, p, centre)?
            .into_iter()
            .map(|t| m.map_or(t, |m| m.conjugate(t)))
            .map(|t| local * t * world)
            .collect();
        // Distances are world lengths; the mesh is local (uniform scale
        // assumed, as for the other tools).
        let scale = crate::scene::uniform_scale(world);
        let weld_at = if tool == crate::editor::Tool::Mirror {
            WeldAt::Mean
        } else {
            WeldAt::Original
        };
        let item = scene.active().ok_or("Select a mesh")?;
        let outcome = copy_faces(
            &item.mesh,
            &source,
            &placed,
            copies,
            weld_at,
            copies.distance / scale,
        )?;
        // MR-42: a Mirror weld lands on the mean of the pair, which is on
        // the (turned) mirror plane.
        scene.active_mut().ok_or("Select a mesh")?.mesh = outcome.mesh.clone().into();
        select_after(scene, &outcome, copies.replace_source);
        Ok(())
    })?;
    Ok(())
}

/// DS-40: the original selection is kept and the copies aren't added; with
/// Replace Source the first copy takes the source's place.
pub(crate) fn select_after(scene: &mut Scene, o: &CopyOutcome, replaced: bool) {
    let s = &mut scene.selection;
    let point = |v: u32| -> Option<u32> {
        if replaced && let Some(&c) = o.replaced_points.get(&v) {
            return Some(c);
        }
        o.point_map.get(v as usize).copied().flatten()
    };
    s.vertices = s.vertices.iter().filter_map(|&v| point(v)).collect();
    s.edges = s
        .edges
        .iter()
        .filter_map(|&(a, b)| Some((point(a)?, point(b)?)))
        .collect();
    let mut faces: Vec<u32> = s
        .faces
        .iter()
        .filter_map(|&f| o.face_map.get(f as usize).copied().flatten())
        .collect();
    if replaced && s.mode == Mode::Polygon && !s.faces.is_empty() {
        faces.extend(&o.replaced_faces);
    }
    faces.sort_unstable();
    faces.dedup();
    s.faces = faces;
    s.vertices.sort_unstable();
    s.vertices.dedup();
    s.edges.dedup();
}

/// A copy of item `item` moved by world transform `t`: its new local
/// transform, and its mesh. A mirroring `t` mirrors the mesh instead of
/// giving the item a negative scale (MR-47).
pub(crate) fn item_placement(
    item: &crate::Item,
    parent: DMat4,
    world: DMat4,
    t: DMat4,
) -> Result<(crate::Transform, Mesh)> {
    let mut mesh: Mesh = (*item.mesh).clone();
    let mut new_world = t * world;
    if DMat3::from_mat4(t).determinant() < 0. {
        // Mirror the mesh across its own X, and undo that in the transform.
        let flip = DMat4::from_scale(DVec3::new(-1., 1., 1.));
        mesh = transformed_copy(&mesh, flip, false);
        new_world *= flip;
    }
    let local = parent.inverse() * new_world;
    let (scale, rotation, translation) = local.to_scale_rotation_translation();
    if !(scale.is_finite() && translation.is_finite()) {
        return Err("The copy's transform isn't finite".into());
    }
    let [x, y, z] = crate::scene::RotationOrder::Xyz.angles(rotation).to_array();
    Ok((
        crate::Transform {
            translation,
            rotation: DVec3::new(x, y, z),
            scale,
            zxy: false,
            order: None,
        },
        mesh,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jitter_is_fixed_and_in_range() {
        for slot in 0..50 {
            let a = jitter(1, slot, 2);
            assert_eq!(a, jitter(1, slot, 2));
            assert!((-1. ..=1.).contains(&a));
        }
        assert_ne!(jitter(1, 1, 0), jitter(1, 2, 0));
    }

    #[test]
    fn mirror_plain_reflection_flips_x() {
        let m = Mirror::default().transform().unwrap();
        assert!(
            m.transform_point3(DVec3::new(1., 2., 3.))
                .abs_diff_eq(DVec3::new(-1., 2., 3.), 1e-12)
        );
    }

    #[test]
    fn mirror_angle_90_turns_the_plane_about_left() {
        // MR-53 probe: Axis X, Angle 90°: the plane becomes X-Y, so the copy
        // flips in Z.
        let mirror = Mirror {
            angle: 90.,
            ..Default::default()
        };
        let m = mirror.transform().unwrap();
        let q = m.transform_point3(DVec3::new(0.5, 0., 0.5));
        assert!(q.abs_diff_eq(DVec3::new(0.5, 0., -0.5), 1e-9), "{q}");
    }

    #[test]
    fn radial_full_and_part_turns() {
        let mut r = RadialArray::default();
        r.circle.count = 4;
        let t = radial_transforms(&r).unwrap();
        assert_eq!(t.len(), 3);
        let p = t[0].transform_point3(DVec3::new(2., 0., 0.));
        assert!(p.abs_diff_eq(DVec3::new(0., 0., -2.), 1e-9));
        r.circle.count = 3;
        r.circle.end = 90.;
        let t = radial_transforms(&r).unwrap();
        let p = t[1].transform_point3(DVec3::X);
        assert!(p.abs_diff_eq(DVec3::new(0., 0., -1.), 1e-9));
    }
}
