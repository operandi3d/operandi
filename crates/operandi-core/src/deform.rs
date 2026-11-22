//! Deform tab, Tier 1B (specifications/deform-presets-2026-11-06): the
//! transform-plus-falloff presets (Soft Move, Shear, Twist, Swirl, Taper,
//! Bulge) and the two small point tools Push and Smooth.
//!
//! A preset is not a tool of its own: it is a row of the tool preset table
//! (tool_preset.rs) that switches on Move, Rotate or Scale together with a
//! falloff fitted to the selection, and turns that falloff off again when
//! the tool is dropped, giving back whatever falloff the user had before
//! (DEFORM §2, DF-Q2). Topology never changes.
use crate::{Falloff, Mesh, Result, Scene, Tool};
use glam::DVec3;
use std::collections::BTreeSet;

/// The Deform tab's transform presets, as handles onto their rows in the
/// tool preset table (tool_preset.rs). The rows hold what each one does;
/// this enum only names them for code that refers to one directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeformPreset {
    /// Move with a Radial falloff (Linear shape).
    SoftMove,
    /// Move with a Linear falloff.
    Shear,
    /// Rotate with a Linear falloff; each point turns by weight × Angle.
    Twist,
    /// Rotate with a Radial falloff; each point turns by weight × Angle.
    Swirl,
    /// Scale with a Linear falloff.
    Taper,
    /// Scale with a Radial falloff (Smooth shape).
    Bulge,
    /// Transform with a Selection falloff and the Selection Border action
    /// centre (SPEC-SELECTION-FALLOFF-FLEX §2), for bending limbs.
    Flex,
}

impl DeformPreset {
    pub const ALL: [Self; 7] = [
        Self::SoftMove,
        Self::Shear,
        Self::Twist,
        Self::Swirl,
        Self::Taper,
        Self::Bulge,
        Self::Flex,
    ];
    /// The row's id in the tool preset table.
    pub fn id(self) -> &'static str {
        match self {
            Self::SoftMove => "xfrm.magnet",
            Self::Shear => "xfrm.shear",
            Self::Twist => "xfrm.twist",
            Self::Swirl => "Swirl",
            Self::Taper => "xfrm.taper",
            Self::Bulge => "xfrm.pole",
            Self::Flex => "Flex",
        }
    }
    pub fn row(self) -> &'static crate::PresetRow {
        crate::tool_preset::row(self.id()).expect("every Deform preset has a row")
    }
    pub fn label(self) -> &'static str {
        &self.row().label
    }
    /// The transform the preset drives.
    pub fn tool(self) -> Tool {
        self.row().tool().expect("Deform presets start a tool")
    }
    pub fn tooltip(self) -> &'static str {
        &self.row().tooltip
    }
    pub fn fitted_falloff(self, scene: &Scene) -> Result<Falloff> {
        self.row()
            .fitted_falloff(scene)?
            .ok_or_else(|| format!("{} has no falloff", self.label()))
    }
    /// Command-line / shortcut name.
    pub fn command_name(self) -> &'static str {
        &self.row().names[0]
    }
    pub fn from_command_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.command_name() == name)
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.id() == id)
    }
}

/// Push's Selection Normals (DEFORM: Push).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PushNormals {
    /// Each point's normal from every polygon around it.
    #[default]
    Averaged,
    /// Only from selected polygons (Polygon mode), so a patch pushes out as
    /// a slab. Outside Polygon mode it behaves like Averaged.
    Selected,
}

/// Smooth's options; starting values from STARTING-VALUES-1A-1B (1B).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmoothOptions {
    /// Share of the way to the neighbours' average per pass. Above 1 the
    /// pass is split into more, smaller passes (provisional, SM-Q1).
    pub strength: f64,
    pub iterations: u32,
    /// Points on open borders stay put.
    pub lock_boundary: bool,
    /// Border points used by only one polygon stay put.
    pub lock_corner: bool,
    /// Each pass is followed by a slight inflate pass so the shape doesn't
    /// shrink (Taubin; provisional, SM-Q1).
    pub preserve_volume: bool,
    /// Points on an edge whose polygons meet at more than `sharp_angle`
    /// stay put.
    pub lock_sharp: bool,
    /// Degrees.
    pub sharp_angle: f64,
}
impl Default for SmoothOptions {
    fn default() -> Self {
        Self {
            strength: 1.,
            iterations: 1,
            lock_boundary: false,
            lock_corner: true,
            preserve_volume: false,
            lock_sharp: false,
            sharp_angle: 60.,
        }
    }
}
impl SmoothOptions {
    pub fn validate(&self) -> Result<()> {
        if !self.strength.is_finite() || self.strength < 0. || self.strength > 100. {
            return Err("Smooth Strength must be between 0 and 100".into());
        }
        if self.iterations > 1000 {
            return Err("Smooth Iterations must be 1000 or fewer".into());
        }
        if !self.sharp_angle.is_finite() || !(0. ..=180.).contains(&self.sharp_angle) {
            return Err("Sharp Angle must be between 0° and 180°".into());
        }
        Ok(())
    }
}

/// Move each of `points` along its normal by `distance` × its weight.
/// `selected_faces` is used for [`PushNormals::Selected`]. Points with no
/// polygon (loose points) have no normal and stay put.
pub fn push_geometry(
    mesh: &Mesh,
    points: &BTreeSet<u32>,
    distance: f64,
    normals: PushNormals,
    selected_faces: Option<&[u32]>,
    weights: Option<&[f64]>,
) -> Result<Mesh> {
    if !distance.is_finite() {
        return Err("Push Distance must be finite".into());
    }
    let only: Option<BTreeSet<u32>> = match (normals, selected_faces) {
        (PushNormals::Selected, Some(faces)) if !faces.is_empty() => {
            Some(faces.iter().copied().collect())
        }
        _ => None,
    };
    let mut sums = vec![DVec3::ZERO; mesh.vertices.len()];
    for f in 0..mesh.faces.len() {
        if mesh.is_polyline(f) || only.as_ref().is_some_and(|s| !s.contains(&(f as u32))) {
            continue;
        }
        let n = mesh.normal(f);
        for &v in &mesh.faces[f].vertices {
            sums[v as usize] += n;
        }
    }
    let mut out = mesh.clone();
    for &v in points {
        let i = v as usize;
        let w = weights.map_or(1., |w| w.get(i).copied().unwrap_or(0.));
        let n = sums.get(i).copied().unwrap_or_default().normalize_or_zero();
        out.vertices[i] += n * distance * w;
    }
    Ok(out)
}

/// The normal an element is pushed along (Element Push): a polygon's own,
/// an edge's polygons' average, a point's polygons' average.
pub fn element_normal(mesh: &Mesh, element: crate::TweakElement) -> DVec3 {
    use crate::TweakElement as E;
    let around = |test: &dyn Fn(&[u32]) -> bool| {
        (0..mesh.faces.len())
            .filter(|&f| !mesh.is_polyline(f) && test(&mesh.faces[f].vertices))
            .map(|f| mesh.normal(f))
            .sum::<DVec3>()
            .normalize_or_zero()
    };
    match element {
        E::Polygon(f) if (f as usize) < mesh.faces.len() => mesh.normal(f as usize),
        E::Polygon(_) => DVec3::ZERO,
        E::Edge(a, b) => around(&|v| {
            let n = v.len();
            (0..n).any(|i| {
                let (x, y) = (v[i], v[(i + 1) % n]);
                (x, y) == (a, b) || (x, y) == (b, a)
            })
        }),
        E::Vertex(p) => around(&|v| v.contains(&p)),
    }
}

/// Move each of `points` along one direction by `distance` × its weight.
pub fn push_along(
    mesh: &Mesh,
    points: &BTreeSet<u32>,
    direction: DVec3,
    distance: f64,
    weights: Option<&[f64]>,
) -> Result<Mesh> {
    if !distance.is_finite() || !direction.is_finite() {
        return Err("Push Distance must be finite".into());
    }
    let mut out = mesh.clone();
    for &v in points {
        let i = v as usize;
        if i >= out.vertices.len() {
            continue;
        }
        let w = weights.map_or(1., |w| w.get(i).copied().unwrap_or(0.));
        out.vertices[i] += direction * distance * w;
    }
    Ok(out)
}

/// Smooth `points`: each pass pulls every free point toward the average of
/// the points it shares an edge with, by Strength × its weight.
pub fn smooth_geometry(
    mesh: &Mesh,
    points: &BTreeSet<u32>,
    options: &SmoothOptions,
    weights: Option<&[f64]>,
) -> Result<Mesh> {
    options.validate()?;
    let n = mesh.vertices.len();
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (a, b) in mesh.edges() {
        neighbours[a as usize].push(b);
        neighbours[b as usize].push(a);
    }
    // Edge use counts and the polygons on each edge (for borders and sharp
    // edges).
    let edge_faces = mesh.edge_faces();
    let mut point_faces = vec![0usize; n];
    for f in 0..mesh.faces.len() {
        if mesh.is_polyline(f) {
            continue;
        }
        for &v in &mesh.faces[f].vertices {
            point_faces[v as usize] += 1;
        }
    }
    let mut locked = vec![false; n];
    let sharp_cos = options.sharp_angle.to_radians().cos();
    for (&(a, b), faces) in edge_faces.iter() {
        let border = faces.len() == 1;
        let mut lock = border && options.lock_boundary;
        if border && options.lock_corner {
            for v in [a, b] {
                if point_faces[v as usize] == 1 {
                    locked[v as usize] = true;
                }
            }
        }
        if options.lock_sharp && faces.len() == 2 {
            let (n0, n1) = (mesh.normal(faces[0]), mesh.normal(faces[1]));
            // The angle between the normals is how far the faces bend away
            // from flat.
            if n0 != DVec3::ZERO && n1 != DVec3::ZERO && n0.dot(n1) < sharp_cos {
                lock = true;
            }
        }
        if lock {
            locked[a as usize] = true;
            locked[b as usize] = true;
        }
    }
    let free: Vec<usize> = points
        .iter()
        .map(|&v| v as usize)
        .filter(|&v| v < n && !locked[v] && !neighbours[v].is_empty())
        .collect();
    let weight = |v: usize| weights.map_or(1., |w| w.get(v).copied().unwrap_or(0.));
    let splits = options.strength.ceil().max(1.);
    let factor = options.strength / splits;
    let passes = options.iterations as usize * splits as usize;
    let mut out = mesh.clone();
    let step = |out: &mut Mesh, amount: f64| {
        let before = out.vertices.clone();
        for &v in &free {
            let ring = &neighbours[v];
            let average =
                ring.iter().map(|&u| before[u as usize]).sum::<DVec3>() / ring.len() as f64;
            out.vertices[v] = before[v] + (average - before[v]) * amount * weight(v);
        }
    };
    for _ in 0..passes {
        step(&mut out, factor);
        if options.preserve_volume && factor > 0. {
            // Taubin's inflate step: mu = -(lambda / (1 - 0.1·lambda)).
            step(&mut out, -factor / (1. - 0.1 * factor));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FalloffShape;

    fn grid(n: usize) -> Mesh {
        let mut vertices = Vec::new();
        for z in 0..=n {
            for x in 0..=n {
                vertices.push(DVec3::new(x as f64, 0., z as f64));
            }
        }
        let mut faces = Vec::new();
        let at = |x: usize, z: usize| (z * (n + 1) + x) as u32;
        for z in 0..n {
            for x in 0..n {
                faces.push(crate::Face {
                    vertices: vec![at(x, z), at(x, z + 1), at(x + 1, z + 1), at(x + 1, z)],
                    uv: None,
                    material: "Default".into(),
                    smooth: false,
                    part: None,
                    material_label: None,
                });
            }
        }
        Mesh {
            vertices,
            faces: faces.into(),
            ..Default::default()
        }
    }

    #[test]
    fn push_moves_points_along_their_normals_scaled_by_weight() {
        let mesh = grid(2);
        let all: BTreeSet<u32> = (0..9).collect();
        let pushed = push_geometry(&mesh, &all, 0.5, PushNormals::Averaged, None, None).unwrap();
        for (a, b) in mesh.vertices.iter().zip(&pushed.vertices) {
            assert!(((*b - *a) - DVec3::Y * 0.5).length() < 1e-12);
        }
        let weights = vec![0.5; 9];
        let half = push_geometry(
            &mesh,
            &all,
            0.5,
            PushNormals::Averaged,
            None,
            Some(&weights),
        )
        .unwrap();
        assert!((half.vertices[4].y - 0.25).abs() < 1e-12);
    }

    #[test]
    fn smooth_pulls_a_bump_down_and_locks_corners_by_default() {
        let mut mesh = grid(2);
        mesh.vertices[4].y = 1.;
        let all: BTreeSet<u32> = (0..9).collect();
        let smoothed = smooth_geometry(&mesh, &all, &SmoothOptions::default(), None).unwrap();
        assert!(smoothed.vertices[4].y < 1.);
        // Corner 0 is used by one polygon: Lock Corner keeps it.
        assert_eq!(smoothed.vertices[0], mesh.vertices[0]);
        // Topology unchanged.
        assert_eq!(smoothed.faces, mesh.faces);
    }

    #[test]
    fn lock_boundary_keeps_every_border_point() {
        let mut mesh = grid(2);
        mesh.vertices[1].y = 1.;
        mesh.vertices[4].y = 1.;
        let all: BTreeSet<u32> = (0..9).collect();
        let options = SmoothOptions {
            lock_boundary: true,
            ..Default::default()
        };
        let smoothed = smooth_geometry(&mesh, &all, &options, None).unwrap();
        for v in [0, 1, 2, 3, 5, 6, 7, 8] {
            assert_eq!(smoothed.vertices[v], mesh.vertices[v], "border point {v}");
        }
        assert!(smoothed.vertices[4].y < 1.);
    }

    #[test]
    fn smooth_respects_weights_and_zero_strength() {
        let mut mesh = grid(2);
        mesh.vertices[4].y = 1.;
        let all: BTreeSet<u32> = (0..9).collect();
        let zero = SmoothOptions {
            strength: 0.,
            ..Default::default()
        };
        assert_eq!(
            smooth_geometry(&mesh, &all, &zero, None).unwrap().vertices,
            mesh.vertices
        );
        let weights = vec![0.; 9];
        let held = smooth_geometry(&mesh, &all, &SmoothOptions::default(), Some(&weights)).unwrap();
        assert_eq!(held.vertices, mesh.vertices);
    }

    #[test]
    fn lock_sharp_edges_keeps_a_crease() {
        // A strip folded 90° along its middle row.
        let mut mesh = grid(2);
        for v in &mut mesh.vertices {
            if v.z > 1. {
                *v = DVec3::new(v.x, v.z - 1., 1.);
            }
        }
        let all: BTreeSet<u32> = (0..9).collect();
        let options = SmoothOptions {
            lock_sharp: true,
            ..Default::default()
        };
        let smoothed = smooth_geometry(&mesh, &all, &options, None).unwrap();
        for v in 3..6 {
            assert_eq!(smoothed.vertices[v], mesh.vertices[v], "crease point {v}");
        }
    }

    #[test]
    fn presets_fit_their_falloff_to_the_selection() {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        // A column: tall along Y.
        let mut mesh = grid(2);
        for v in &mut mesh.vertices {
            *v = DVec3::new(v.x, v.z * 2., 0.);
        }
        scene.items.last_mut().unwrap().mesh = mesh.into();
        scene.selection.mode = crate::Mode::Vertex;
        let Falloff::Linear(f) = DeformPreset::Twist.fitted_falloff(&scene).unwrap() else {
            panic!("Twist uses a Linear falloff");
        };
        assert_eq!(f.start, DVec3::new(1., 4., 0.));
        assert_eq!(f.end, DVec3::new(1., 0., 0.));
        let Falloff::Radial(r) = DeformPreset::Bulge.fitted_falloff(&scene).unwrap() else {
            panic!("Bulge uses a Radial falloff");
        };
        assert_eq!(r.center, DVec3::new(1., 2., 0.));
        assert_eq!(r.size, DVec3::new(1., 2., 0.));
        assert_eq!(r.shape, FalloffShape::Smooth);
    }
}
