//! Dicer (toolbox-panels-2026-11-06 EDIT.md v0, provisional): cut a closed
//! mesh into separate closed pieces with planes, the inside faces tagged
//! with an inner material. Seeded random planes split every piece they
//! cross. Fragment Scale stretches the planes' directions.
//!
//! Each cut intersects the mesh with half-space boxes using Operandi's
//! Boolean engine (boolean.rs). The pieces stay closed and keep the
//! source polygons' tags; the new inner
//! faces come from the cutting boxes and take Inner Material. Gap leaves
//! that much space between neighbouring pieces. A piece that would be empty
//! is left out. Built as commands with a dialog (the click-and-drag
//! placement is EDIT.md's and not built yet; JD-Q1).
use crate::boolean::{BooleanOp, Gathered, Input, boolean_gathered};
use crate::{Mesh, Result};
use glam::{DMat4, DQuat, DVec3};

/// Most pieces a cut may make (an allocation guard).
pub const MAX_PIECES: usize = 512;

/// Dicer's options (EDIT.md starting values; Randomness not built).
#[derive(Clone, Debug, PartialEq)]
pub struct DicerOptions {
    pub seed: u32,
    /// Max. Cutting Planes.
    pub planes: u32,
    pub gap: f64,
    pub inner_material: String,
    /// Fragment Scale: plane directions are drawn in a box this shape.
    pub fragment_scale: DVec3,
}
impl Default for DicerOptions {
    fn default() -> Self {
        Self {
            seed: 40,
            planes: 6,
            gap: 0.001,
            inner_material: String::new(),
            fragment_scale: DVec3::ONE,
        }
    }
}

/// A big box with its +X face on the plane `n·x = d`, on the side
/// `n·x <= d`, reaching `reach` past every point of the mesh.
fn half_space(n: DVec3, d: DVec3, reach: f64, material: &str) -> Result<Mesh> {
    slab(n, d - n * reach * 2., d, reach, material)
}

/// A box between the planes through `lo` and `hi` (along `n`), wide
/// enough to hold the mesh.
fn slab(n: DVec3, lo: DVec3, hi: DVec3, reach: f64, material: &str) -> Result<Mesh> {
    let depth = (hi - lo).dot(n);
    if depth <= 0. {
        return Err("Dicer: Invalid cutting box depth".into());
    }
    let mut m = Mesh::box_mesh(DVec3::ZERO, DVec3::new(depth, 4. * reach, 4. * reach), 0)?;
    let turn = DQuat::from_rotation_arc(DVec3::X, n);
    let at = (lo + hi) / 2.;
    let place = DMat4::from_translation(at) * DMat4::from_quat(turn);
    for v in &mut m.vertices {
        *v = place.transform_point3(*v);
    }
    for f in m.faces.iter_mut() {
        f.material = material.to_owned();
    }
    Ok(m)
}

/// `mesh` ∩ `cutter`, or `None` when nothing is left.
fn intersect(mesh: &Mesh, cutter: Mesh, what: &str) -> Result<Option<Mesh>> {
    let mut combined = mesh.clone();
    let target = Input {
        item: 0,
        name: "mesh".into(),
        faces: (0..mesh.faces.len()).collect(),
        point_offset: 0,
    };
    let point_offset = combined.vertices.len() as u32;
    let first = combined.faces.len();
    crate::append_mesh(&mut combined, cutter)?;
    let driver = Input {
        item: 1,
        name: "cutter".into(),
        faces: (first..combined.faces.len()).collect(),
        point_offset,
    };
    let g = Gathered {
        combined,
        target,
        drivers: vec![driver],
        to_world: DMat4::IDENTITY,
    };
    match boolean_gathered(&g, BooleanOp::Intersect, what) {
        Ok((m, _)) if m.faces.is_empty() => Ok(None),
        Ok((m, _)) => Ok(Some(m)),
        // An empty intersection can be refused by the result checks.
        Err(e) if e.contains("empty") || e.contains("nothing left") => Ok(None),
        Err(e) => Err(e),
    }
}

/// The mesh's centre and how far its points reach from it.
fn extent(mesh: &Mesh) -> Result<(DVec3, f64)> {
    let (lo, hi) = mesh.vertices.iter().fold(
        (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)),
        |a, v| (a.0.min(*v), a.1.max(*v)),
    );
    if !lo.is_finite() {
        return Err("Nothing to cut".into());
    }
    Ok(((lo + hi) / 2., ((hi - lo).length() / 2.).max(1e-6)))
}

/// The inner material, or the mesh's own first one.
fn inner(mesh: &Mesh, material: &str) -> String {
    if material.is_empty() {
        mesh.faces
            .first()
            .map(|f| f.material.clone())
            .unwrap_or_default()
    } else {
        material.to_owned()
    }
}

fn join(pieces: Vec<Mesh>) -> Result<Mesh> {
    let mut out = Mesh::default();
    for p in pieces {
        crate::append_mesh(&mut out, p)?;
    }
    out.validate()?;
    Ok(out)
}

/// Dicer: random planes through the mesh, each splitting every piece it
/// crosses (at most [`MAX_PIECES`] pieces).
pub fn dicer(mesh: &Mesh, o: &DicerOptions) -> Result<Mesh> {
    if o.planes == 0 || o.planes > 16 {
        return Err("Dicer: Max. Cutting Planes must be 1 to 16".into());
    }
    if !(o.gap.is_finite() && o.gap >= 0.) || !o.fragment_scale.is_finite() {
        return Err("Dicer values must be numbers, Gap 0 or more".into());
    }
    let (centre, reach) = extent(mesh)?;
    let material = inner(mesh, &o.inner_material);
    let mut pieces = vec![mesh.clone()];
    for k in 0..o.planes as u64 {
        let r = |c: u64| crate::duplicate::jitter(o.seed as u64, k, c);
        // A direction drawn in a box shaped by Fragment Scale: a large
        // scale on an axis cuts across it more often (shorter chunks there).
        let Some(n) = (DVec3::new(r(0), r(1), r(2)) * o.fragment_scale).try_normalize() else {
            continue;
        };
        let at = centre + DVec3::new(r(3), r(4), r(5)) * reach * 0.5;
        let far = reach * 4.;
        let mut next = Vec::new();
        for p in &pieces {
            let below = intersect(
                p,
                half_space(n, at - n * (o.gap / 2.), far, &material)?,
                "Dicer",
            )?;
            let above = intersect(
                p,
                half_space(-n, at + n * (o.gap / 2.), far, &material)?,
                "Dicer",
            )?;
            next.extend(below);
            next.extend(above);
            if next.len() > MAX_PIECES {
                return Err(format!(
                    "Dicer: more than {MAX_PIECES} pieces; use fewer planes"
                ));
            }
        }
        if !next.is_empty() {
            pieces = next;
        }
    }
    join(pieces)
}

impl crate::Editor {
    /// Dicer on the active mesh, one undo step; the pieces are
    /// selected after (Polygon mode).
    pub(crate) fn cut_pieces(
        &mut self,
        label: &str,
        run: impl FnOnce(&Mesh) -> Result<Mesh>,
    ) -> Result<()> {
        let item = self.edge_insert_target(label)?;
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let out = run(mesh)?;
        self.publish_polygon_edit(item, out, label, Vec::new())
    }
}
