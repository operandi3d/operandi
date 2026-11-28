//! Absolute Scaling (spec `snaps-precision-2026-11-10`, part A): resize the
//! selection to typed sizes. Grab Size reads the selection's box into the
//! fields; Uniform Scale scales every axis by one factor so the Uniform Axis
//! reaches the typed length; Explicit Scale makes the box exactly X × Y × Z;
//! Reference Scale measures the factors on the selection but applies them to
//! the whole mesh, about the whole mesh's box.
//!
//! Sizes are in each mesh's own coordinates (item transforms are ignored,
//! A4); several meshes share one combined box (A4, case 38). Only point
//! positions change, as one undo step.
//!
//! Decided (Jason, 2026-11-10, spec Q2): an axis whose current size is zero
//! is left alone by every mode. Uniform with a zero-size Uniform Axis does
//! nothing and says so.
use crate::*;
use glam::{DMat3, DVec3};
use std::collections::BTreeMap;

/// Which box side Uniform Scale reads (A3 "Uniform Axis").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UniformAxis {
    #[default]
    Longest,
    X,
    Y,
    Z,
}
impl UniformAxis {
    pub const ALL: [Self; 4] = [Self::Longest, Self::X, Self::Y, Self::Z];
    pub fn label(self) -> &'static str {
        match self {
            Self::Longest => "Longest Axis",
            Self::X => "X Axis",
            Self::Y => "Y Axis",
            Self::Z => "Z Axis",
        }
    }
    /// The script name (`absolutescale.uniformaxis`, spec D).
    pub fn script_name(self) -> &'static str {
        match self {
            Self::Longest => "longestAxis",
            Self::X => "x",
            Self::Y => "y",
            Self::Z => "z",
        }
    }
    /// The box size this axis reads; Longest is the largest side.
    pub fn of(self, size: DVec3) -> f64 {
        match self {
            Self::Longest => size.max_element(),
            Self::X => size.x,
            Self::Y => size.y,
            Self::Z => size.z,
        }
    }
}

/// The point an axis scales about (A5): the box's middle, top or bottom,
/// or (Operandi's addition, Jason 2026-11-11, ABS-Q12) the item's origin, so
/// the geometry keeps its place around the origin.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScaleCenter {
    #[default]
    Center,
    High,
    Low,
    Origin,
}
impl ScaleCenter {
    /// Menu order (A3), Origin last.
    pub const ALL: [Self; 4] = [Self::Center, Self::High, Self::Low, Self::Origin];
    pub fn label(self) -> &'static str {
        match self {
            Self::Center => "Center",
            Self::High => "High",
            Self::Low => "Low",
            Self::Origin => "Origin",
        }
    }
}

/// Scale Absolute / Size Absolute handles (spec part 1 B, C; panel Q6
/// guess): along each axis a base handle at `base` from the centre and a
/// scale handle at `base × factor`. The factor is the Scale tool's own
/// (`Parameters::delta`), so typed values, undo and repeat work as for
/// Scale. Size Absolute names them Absolute (the base) and Relative (the
/// size you want).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AbsoluteHandles {
    /// Base length per axis in metres (Scale Absolute starts at 1.2,
    /// Size Absolute at 1; spec D).
    pub base: DVec3,
    /// Size Absolute's Absolute/Relative fields instead of Scale's factor.
    pub size: bool,
}
impl AbsoluteHandles {
    pub fn scale_absolute() -> Self {
        Self {
            base: DVec3::splat(1.2),
            size: false,
        }
    }
    pub fn size_absolute() -> Self {
        Self {
            base: DVec3::ONE,
            size: true,
        }
    }
}

/// The four buttons (`scale.absolute mode`, spec D).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbsoluteMode {
    Grab,
    Uniform,
    Explicit,
    Reference,
}
impl AbsoluteMode {
    pub const ALL: [Self; 4] = [Self::Grab, Self::Uniform, Self::Explicit, Self::Reference];
    pub fn label(self) -> &'static str {
        match self {
            Self::Grab => "Grab Size",
            Self::Uniform => "Uniform Scale",
            Self::Explicit => "Explicit Scale",
            Self::Reference => "Reference Scale",
        }
    }
    pub fn script_name(self) -> &'static str {
        match self {
            Self::Grab => "grab",
            Self::Uniform => "uniform",
            Self::Explicit => "explicit",
            Self::Reference => "reference",
        }
    }
}

/// The window's fields (A3). They live in the app for the session (not
/// saved, A3); a script passes them whole.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AbsoluteFields {
    pub uniform: f64,
    pub axis: UniformAxis,
    pub orient: bool,
    /// X, Y, Z.
    pub size: DVec3,
    pub center: [ScaleCenter; 3],
}
impl AbsoluteFields {
    /// The fields after Grab Size, or after any scale (A6): X Y Z are the
    /// box's sides and Uniform its side along the Uniform Axis.
    pub fn grabbed(mut self, measured: &AbsoluteBox) -> Self {
        let size = measured.size();
        self.size = size;
        self.uniform = self.axis.of(size);
        self
    }
}

/// A measured box: `axes` are its X, Y, Z directions as columns (the
/// identity unless Orient To Selection fitted it), `min`/`max` its extent
/// along them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AbsoluteBox {
    pub axes: DMat3,
    pub min: DVec3,
    pub max: DVec3,
}
impl AbsoluteBox {
    pub fn size(&self) -> DVec3 {
        self.max - self.min
    }
    /// The scaling centre per axis, in box coordinates (A5).
    fn pivot(&self, center: [ScaleCenter; 3]) -> DVec3 {
        let mut p = DVec3::ZERO;
        for (i, c) in center.iter().enumerate() {
            p[i] = match c {
                ScaleCenter::Center => (self.min[i] + self.max[i]) / 2.,
                ScaleCenter::High => self.max[i],
                ScaleCenter::Low => self.min[i],
                // Sizes are in each mesh's own coordinates (A4), so its
                // origin is 0 there, and 0 in a box frame through it too.
                ScaleCenter::Origin => 0.,
            };
        }
        p
    }
    /// A size this small counts as zero (flat): relative to how far the box
    /// sits from the origin, so float noise on a big flat face isn't a side.
    fn flat(&self, i: usize) -> bool {
        let reach = self.min.abs().max(self.max.abs()).max_element();
        self.size()[i].abs() <= 1e-9 * (1. + reach)
    }
}

/// One Absolute Scaling press (`Command::AbsoluteScale`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AbsoluteScale {
    pub mode: AbsoluteMode,
    pub fields: AbsoluteFields,
}

/// What a press will do: the factor per box axis and which axes it leaves
/// alone because they have no size (Q2), for the status line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AbsolutePlan {
    pub factors: DVec3,
    pub flat: [bool; 3],
}

/// The meshes and points a press measures, by item id.
type Working = BTreeMap<u64, Vec<u32>>;

const AXIS_NAMES: [&str; 3] = ["X", "Y", "Z"];

impl Editor {
    fn absolute_preflight(&self) -> Result<()> {
        if self.i2_active() {
            return Err("Finish or Cancel the Extrude stage before Absolute Scaling".into());
        }
        if self.session.pen_draft.is_some() {
            return Err("Drop or Cancel Pen before Absolute Scaling".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish the selection gesture before Absolute Scaling".into());
        }
        Ok(())
    }

    /// The working set (A4): the points the selection touches in the
    /// current mode on each target mesh; every point of a mesh with
    /// nothing selected, and in Item mode every point of each selected
    /// mesh.
    fn absolute_working(&self) -> Result<Working> {
        let scene = &self.scene;
        let mode = scene.selection.mode;
        let targets = if mode == Mode::Item {
            scene.active_meshes()
        } else {
            match scene.component_transform_meshes() {
                Some(ids) => ids,
                None => scene
                    .active()
                    .filter(|i| i.is_mesh() && scene.shows(i))
                    .map(|i| i.id)
                    .into_iter()
                    .collect(),
            }
        };
        if targets.is_empty() {
            return Err("Absolute Scaling: select a visible mesh".into());
        }
        let mut working = Working::new();
        for id in targets {
            let item = scene.item(id).ok_or("Absolute Scaling: missing mesh")?;
            let mesh = &item.mesh;
            let selected = match mode {
                Mode::Vertex => !scene.selection.vertices_of(id).is_empty(),
                Mode::Edge => !scene.selection.edges_of(id).is_empty(),
                Mode::Polygon => !scene.selection.faces_of(id).is_empty(),
                Mode::Item => false,
            };
            let points: Vec<u32> = if selected {
                scene.mesh_transform_vertices(id)?.into_iter().collect()
            } else {
                // Nothing selected, or Item mode: the whole mesh (A4).
                let used = mesh.used_points();
                (0..mesh.vertices.len() as u32)
                    .filter(|&v| used[v as usize])
                    .collect()
            };
            if !points.is_empty() {
                working.insert(id, points);
            }
        }
        if working.is_empty() {
            return Err("Absolute Scaling: the selected meshes have no points".into());
        }
        Ok(working)
    }

    fn absolute_points<'a>(&'a self, working: &'a Working) -> impl Iterator<Item = DVec3> + 'a {
        working.iter().flat_map(move |(id, points)| {
            let mesh = self.scene.item(*id).map(|i| &i.mesh);
            points
                .iter()
                .filter_map(move |&v| mesh.and_then(|m| m.vertices.get(v as usize)).copied())
        })
    }

    /// The selection's box (A4, A8), as Grab Size reads it.
    pub fn absolute_box(&self, orient: bool) -> Result<AbsoluteBox> {
        self.absolute_preflight()?;
        let working = self.absolute_working()?;
        Ok(self.box_of(&working, orient))
    }

    fn box_of(&self, working: &Working, orient: bool) -> AbsoluteBox {
        let points: Vec<DVec3> = self.absolute_points(working).collect();
        let axes = if orient {
            fit_frame(&points, &self.frame_candidates(working))
        } else {
            DMat3::IDENTITY
        };
        extent(&points, axes)
    }

    /// The fields after a press, read from the scene as it is now (A6).
    pub fn absolute_fields_after(&self, fields: AbsoluteFields) -> Result<AbsoluteFields> {
        Ok(fields.grabbed(&self.absolute_box(fields.orient)?))
    }

    /// What `scale` would do, without doing it. Grab Size changes nothing.
    pub fn absolute_plan(&self, scale: AbsoluteScale) -> Result<AbsolutePlan> {
        let measured = self.absolute_box(scale.fields.orient)?;
        plan(&measured, scale)
    }

    /// One Absolute Scaling press: one undo step, or nothing when no point
    /// would move.
    pub(crate) fn absolute_scale(&mut self, scale: AbsoluteScale) -> Result<()> {
        let fields = scale.fields;
        let finite = fields.uniform.is_finite()
            && fields.size.is_finite()
            && fields.uniform.abs() <= 1e9
            && fields.size.abs().max_element() <= 1e9;
        if !finite {
            return Err("Absolute Scaling needs finite sizes within the scene range".into());
        }
        self.absolute_preflight()?;
        let working = self.absolute_working()?;
        let measured = self.box_of(&working, fields.orient);
        let plan = plan(&measured, scale)?;
        if scale.mode == AbsoluteMode::Grab || plan.factors == DVec3::ONE {
            return Ok(());
        }
        // Reference moves every point of the meshes, about their whole box
        // in the selection's frame (A6.4).
        let (moving, about) = if scale.mode == AbsoluteMode::Reference {
            let mut whole = Working::new();
            for &id in working.keys() {
                let item = self.scene.item(id);
                if let Some(mesh) = item.map(|i| &i.mesh) {
                    let used = mesh.used_points();
                    whole.insert(
                        id,
                        (0..mesh.vertices.len() as u32)
                            .filter(|&v| used[v as usize])
                            .collect(),
                    );
                }
            }
            let points: Vec<DVec3> = self.absolute_points(&whole).collect();
            let about = extent(&points, measured.axes);
            (whole, about)
        } else {
            (working, measured)
        };
        let axes = measured.axes;
        let to_box = axes.transpose();
        let pivot = about.pivot(fields.center);
        let mut candidate = self.scene.clone();
        let mut moved_any = false;
        for (id, points) in &moving {
            let item = candidate
                .item_mut(*id)
                .ok_or("Absolute Scaling: missing mesh")?;
            let mesh = &mut item.mesh;
            let mut touched = vec![false; mesh.vertices.len()];
            for &v in points {
                let old = mesh.vertices[v as usize];
                let q = to_box * old;
                let new = axes * (pivot + (q - pivot) * plan.factors);
                if !new.is_finite() {
                    return Err("Absolute Scaling: a point would not be finite".into());
                }
                if new != old {
                    mesh.vertices[v as usize] = new;
                    touched[v as usize] = true;
                    moved_any = true;
                }
            }
            // Uniform 0 collapses polygons (A6.2, case 22): keep them, marked
            // as the deliberate degenerate polygons they are (as Set Position).
            for f in 0..mesh.faces.len() {
                if !mesh.is_polyline(f)
                    && mesh.faces[f].vertices.iter().any(|&v| touched[v as usize])
                    && mesh.normal(f) == DVec3::ZERO
                {
                    mesh.attributes.degenerate_faces.insert(f as u32);
                }
            }
        }
        if !moved_any {
            return Ok(());
        }
        candidate.validate()?;
        // Only coordinates changed: component IDs survive (as Set Position).
        let before = std::mem::replace(&mut self.scene, candidate);
        if let Err(error) = self.identities.observe(&self.scene) {
            self.scene = before;
            return Err(error);
        }
        self.record(scale.mode.label().into(), before);
        Ok(())
    }

    /// Directions Orient To Selection tries (A8, provisional, Q4): the
    /// normals of polygons wholly in the working set and the directions of
    /// edges between working-set points, the most common first.
    fn frame_candidates(&self, working: &Working) -> Vec<DVec3> {
        let mut votes: BTreeMap<[i64; 3], (DVec3, f64)> = BTreeMap::new();
        let mut vote = |d: DVec3, weight: f64| {
            let Some(mut d) = d.try_normalize() else {
                return;
            };
            // One sign per line, so d and -d are one direction.
            let lead = if d.x.abs() > 1e-9 {
                d.x
            } else if d.y.abs() > 1e-9 {
                d.y
            } else {
                d.z
            };
            if lead < 0. {
                d = -d;
            }
            let key = (d * 1e6).round().as_i64vec3().to_array();
            votes.entry(key).or_insert((d, 0.)).1 += weight;
        };
        for (id, points) in working {
            let Some(item) = self.scene.item(*id) else {
                continue;
            };
            let mesh = &item.mesh;
            let mut inside = vec![false; mesh.vertices.len()];
            for &v in points {
                inside[v as usize] = true;
            }
            for f in 0..mesh.faces.len() {
                let face = &mesh.faces[f];
                if !face.vertices.iter().all(|&v| inside[v as usize]) {
                    continue;
                }
                vote(mesh.normal(f), 1.);
            }
            for (a, b) in mesh.edges() {
                if inside[a as usize] && inside[b as usize] {
                    let d = mesh.vertices[b as usize] - mesh.vertices[a as usize];
                    vote(d, d.length());
                }
            }
        }
        let mut ranked: Vec<(DVec3, f64)> = votes.into_values().collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
        ranked.truncate(12);
        ranked.into_iter().map(|(d, _)| d).collect()
    }
}

/// Factors per box axis (A6.2..A6.4, Q2 decided).
fn plan(measured: &AbsoluteBox, scale: AbsoluteScale) -> Result<AbsolutePlan> {
    let size = measured.size();
    let flat = [0, 1, 2].map(|i| measured.flat(i));
    let fields = scale.fields;
    let mut factors = DVec3::ONE;
    match scale.mode {
        AbsoluteMode::Grab => {}
        AbsoluteMode::Uniform => {
            let along = fields.axis.of(size);
            let along_flat = match fields.axis {
                UniformAxis::Longest => flat.iter().all(|&f| f),
                UniformAxis::X => flat[0],
                UniformAxis::Y => flat[1],
                UniformAxis::Z => flat[2],
            };
            if along_flat {
                return Err(format!(
                    "Absolute Scaling: the selection has no size along {}; nothing changed",
                    fields.axis.label()
                ));
            }
            let f = fields.uniform / along;
            for i in 0..3 {
                if !flat[i] {
                    factors[i] = f;
                }
            }
        }
        AbsoluteMode::Explicit | AbsoluteMode::Reference => {
            // Target 0 leaves the axis alone (cases 15, 28, 33); so does a
            // flat axis (case 12; Reference per Q2). Negative mirrors (21).
            for i in 0..3 {
                if fields.size[i] != 0. && !flat[i] {
                    factors[i] = fields.size[i] / size[i];
                }
            }
        }
    }
    Ok(AbsolutePlan { factors, flat })
}

impl AbsolutePlan {
    /// The status line's note about axes left alone because they're flat.
    pub fn flat_note(&self, scale: AbsoluteScale) -> Option<String> {
        let skipped: Vec<&str> = (0..3)
            .filter(|&i| {
                self.flat[i]
                    && match scale.mode {
                        AbsoluteMode::Grab => false,
                        AbsoluteMode::Uniform => true,
                        // A flat axis only matters when a size was typed.
                        _ => scale.fields.size[i] != 0.,
                    }
            })
            .map(|i| AXIS_NAMES[i])
            .collect();
        (!skipped.is_empty()).then(|| {
            format!(
                "{}: the selection is flat on {}; left unchanged there",
                scale.mode.label(),
                skipped.join(", ")
            )
        })
    }
}

/// The box of `points` along `axes` (columns).
fn extent(points: &[DVec3], axes: DMat3) -> AbsoluteBox {
    let to_box = axes.transpose();
    let mut min = DVec3::splat(f64::INFINITY);
    let mut max = DVec3::splat(f64::NEG_INFINITY);
    for &p in points {
        let q = to_box * p;
        min = min.min(q);
        max = max.max(q);
    }
    if points.is_empty() {
        min = DVec3::ZERO;
        max = DVec3::ZERO;
    }
    AbsoluteBox { axes, min, max }
}

/// Orient To Selection's frame (A8; how the reference picks it is open,
/// Q4, so this is provisional): among the world axes and frames built from
/// pairs of the selection's own directions, the box with the least volume,
/// then the least surface (so flat selections still orient), the world
/// axes winning ties. Its sides are then named X, Y, Z after the world axis
/// each lies closest to; a side halfway between two (a box turned 45°)
/// goes to the earlier axis if it's the longer (case 37: X is the long
/// side).
fn fit_frame(points: &[DVec3], directions: &[DVec3]) -> DMat3 {
    let measure = |axes: DMat3| {
        let s = extent(points, axes).size();
        (s.x * s.y * s.z, s.x * s.y + s.y * s.z + s.z * s.x)
    };
    let mut best = DMat3::IDENTITY;
    let (mut volume, mut area) = measure(best);
    let scale = extent(points, best).size().max_element().max(1e-12);
    let (vol_eps, area_eps) = (1e-9 * scale.powi(3), 1e-9 * scale.powi(2));
    for (i, &a) in directions.iter().enumerate() {
        for &b in &directions[i + 1..] {
            let Some(z) = a.cross(b).try_normalize() else {
                continue;
            };
            let y = z.cross(a);
            let axes = DMat3::from_cols(a, y, z);
            let (v, s) = measure(axes);
            if v < volume - vol_eps || (v <= volume + vol_eps && s < area - area_eps) {
                best = axes;
                volume = v;
                area = s;
            }
        }
    }
    if best == DMat3::IDENTITY {
        return best;
    }
    name_sides(best, extent(points, best).size())
}

/// Reorders and flips a frame's columns so column i is the side closest to
/// world axis i, pointing its way (see `fit_frame`).
fn name_sides(axes: DMat3, size: DVec3) -> DMat3 {
    let cols = [axes.x_axis, axes.y_axis, axes.z_axis];
    let mut pairs: Vec<(usize, usize, f64)> = Vec::new();
    for (side, c) in cols.iter().enumerate() {
        for world in 0..3 {
            pairs.push((side, world, c[world].abs()));
        }
    }
    // Closest first; a tie (within float noise) goes to the longer side,
    // then the earlier world axis.
    pairs.sort_by(|a, b| {
        let (ka, kb) = ((a.2 * 1e6).round(), (b.2 * 1e6).round());
        kb.total_cmp(&ka)
            .then(size[b.0].total_cmp(&size[a.0]))
            .then(a.1.cmp(&b.1))
    });
    let mut out = [DVec3::ZERO; 3];
    let (mut side_used, mut world_used) = ([false; 3], [false; 3]);
    for (side, world, _) in pairs {
        if side_used[side] || world_used[world] {
            continue;
        }
        side_used[side] = true;
        world_used[world] = true;
        let c = cols[side];
        out[world] = if c[world] < 0. { -c } else { c };
    }
    // Keep it right-handed so a box doesn't turn inside out.
    if out[0].cross(out[1]).dot(out[2]) < 0. {
        out[2] = -out[2];
    }
    DMat3::from_cols(out[0], out[1], out[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turned_box_names_its_long_side_x() {
        let r = DMat3::from_rotation_z(std::f64::consts::FRAC_PI_4);
        let corners: Vec<DVec3> = (0..8)
            .map(|i| {
                r * DVec3::new(
                    if i & 1 == 0 { -1. } else { 1. },
                    if i & 2 == 0 { -0.5 } else { 0.5 },
                    if i & 4 == 0 { -0.5 } else { 0.5 },
                )
            })
            .collect();
        let dirs = [r * DVec3::X, r * DVec3::Y, DVec3::Z];
        let axes = fit_frame(&corners, &dirs);
        let s = extent(&corners, axes).size();
        assert!(
            (s - DVec3::new(2., 1., 1.)).abs().max_element() < 1e-9,
            "{s}"
        );
    }

    #[test]
    fn axis_aligned_selection_keeps_world_axes() {
        let pts = [DVec3::ZERO, DVec3::new(2., 1., 0.5)];
        assert_eq!(fit_frame(&pts, &[DVec3::X, DVec3::Y]), DMat3::IDENTITY);
    }
}
