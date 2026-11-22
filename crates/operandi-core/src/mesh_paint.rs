//! Mesh Paint and Mesh Paint Instance. Spec: missing-tools-g6
//! G6-MESH-PAINT v1, MP-01..18.
//!
//! A placer and a copier (the spec's TOOL-FAMILIES pairing): the app turns
//! a click or stroke into surface points on the target (the active mesh,
//! MP-02), and one [`MeshPaintStroke`] carries them here, where each point
//! becomes a placement (MP-09..13) and the copier adds a copy of a source
//! (a visible background mesh, MP-01/14) there: real geometry appended to
//! the target (Mesh Paint, as Clone's copy-geometry copier does), or a new
//! instance item (Mesh Paint Instance, the Instance copier's placer,
//! MP-15). One stroke or click is one undo step (MP-16). Every random
//! choice comes from the stroke's seed, so undo, redo and reopen agree.
use crate::{Editor, Mesh, Result, Scene};
use glam::{DMat3, DMat4, DQuat, DVec3};

/// The refusal without a source to copy (MP-01, a reading).
pub const NO_SOURCE: &str = "Mesh Paint needs a background mesh to copy.";

/// Paint Mode (MP-03..08).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PaintMode {
    #[default]
    Slide,
    Strokes,
    Drop,
    DropNoScale,
    Linear,
    Array,
    Radial,
}

/// Size Mode (MP-11).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SizeMode {
    Uniform,
    Adaptive,
    Random,
    AdaptiveRandom,
    #[default]
    Locked,
}

/// Rotation Mode (MP-10). The tablet modes take pen tilt; with no tilt
/// they fall back to Surface Align (or the view's up for Screen Align).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RotationMode {
    Uniform,
    #[default]
    SurfaceAlign,
    Random,
    SurfaceAlignRandom,
    SurfaceAlignTablet,
    SurfaceAlignTabletNoPitch,
    ScreenAlignTablet,
    ScreenAlignTabletNoPitch,
}

/// A parameter feeding an effect (MP-12).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PaintParameter {
    #[default]
    Falloff,
    Pressure,
    Slope,
    Altitude,
    InverseSlope,
    InverseAltitude,
}

/// What a parameter changes (MP-12).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PaintEffect {
    None,
    #[default]
    Density,
    Size,
    Rotation,
    SizeDensity,
}

/// Geometry Source (MP-14).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GeometrySource {
    AllLayers,
    #[default]
    RandomLayer,
}

/// The tool's settings (the spec's defaults).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshPaintSettings {
    pub mode: PaintMode,
    /// Screen radius, points.
    pub brush_size: f64,
    /// Percent.
    pub brush_density: f64,
    pub size_mode: SizeMode,
    /// Percent per axis.
    pub size: DVec3,
    /// Percent; 0 is free.
    pub scale_increment: f64,
    pub rotation_mode: RotationMode,
    /// Degrees per axis.
    pub rotation: DVec3,
    pub rotation_snap: bool,
    /// Degrees.
    pub snap_angle: f64,
    /// Metres along the surface normal.
    pub offset: f64,
    pub parameter1: (PaintParameter, PaintEffect),
    pub parameter2: (PaintParameter, PaintEffect),
    pub linear_count: u32,
    pub array_count: [u32; 2],
    pub radial_count: u32,
    pub brush_3d: bool,
    pub source: GeometrySource,
    /// Mesh Paint Instance: new instance items instead of geometry.
    pub instances: bool,
    /// Mesh Paint Replica (G8 RP-7): the stroke makes one Replicator, a
    /// point per copy, instead of geometry or instance items.
    pub replicas: bool,
    pub parent: crate::duplicate_items::ParentMode,
}
impl Default for MeshPaintSettings {
    fn default() -> Self {
        Self {
            mode: PaintMode::Slide,
            brush_size: 20.,
            brush_density: 50.,
            size_mode: SizeMode::Locked,
            size: DVec3::splat(100.),
            scale_increment: 0.,
            rotation_mode: RotationMode::SurfaceAlign,
            rotation: DVec3::ZERO,
            rotation_snap: false,
            snap_angle: 15.,
            offset: 0.,
            parameter1: (PaintParameter::Falloff, PaintEffect::Density),
            parameter2: (PaintParameter::Pressure, PaintEffect::Density),
            linear_count: 2,
            array_count: [2, 2],
            radial_count: 6,
            brush_3d: false,
            source: GeometrySource::RandomLayer,
            instances: false,
            replicas: false,
            parent: crate::duplicate_items::ParentMode::Off,
        }
    }
}

/// One surface point from the placer (world space).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintPoint {
    pub position: DVec3,
    /// The surface's unit normal there.
    pub normal: DVec3,
    /// Stroke or line direction (Strokes, Linear), for the heading.
    pub heading: Option<DVec3>,
    /// Brush falloff at the point: 1 at the centre, 0 at the edge.
    pub falloff: f64,
    /// Pen pressure (1 with a mouse).
    pub pressure: f64,
    /// Extra size from the gesture (Drop's drag, Slide's Ctrl drag).
    pub scale: f64,
    /// Extra turn about the normal from the gesture (Drop), radians.
    pub spin: f64,
}
impl PaintPoint {
    /// A plain point: no heading, full falloff and pressure, no gesture.
    pub fn at(position: DVec3, normal: DVec3) -> Self {
        Self {
            position,
            normal: normal.normalize_or(DVec3::Y),
            heading: None,
            falloff: 1.,
            pressure: 1.,
            scale: 1.,
            spin: 0.,
        }
    }
}

/// One click or stroke: the settings it was made with, its points and the
/// seed for its random choices.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshPaintStroke {
    pub settings: MeshPaintSettings,
    pub points: Vec<PaintPoint>,
    pub seed: u64,
    /// The view's up (Screen Align).
    pub view_up: DVec3,
    /// The brush's world radius under the pointer (Adaptive size).
    pub brush_world: f64,
}

/// A small seeded generator (splitmix64): the same stroke always makes
/// the same choices.
#[derive(Clone, Copy, Debug)]
pub struct Rng(pub u64);
impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// 0..1.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// -1..1.
    pub fn signed(&mut self) -> f64 {
        self.unit() * 2. - 1.
    }
}

/// A parameter's value at `pt` (MP-12): slope 0 flat to 1 vertical,
/// altitude 0 at the bottom of `bounds` (world Y) to 1 at the top.
fn parameter_value(which: PaintParameter, pt: &PaintPoint, bounds: (f64, f64)) -> f64 {
    let slope = pt.normal.dot(DVec3::Y).clamp(-1., 1.).acos() / std::f64::consts::FRAC_PI_2;
    let slope = slope.clamp(0., 1.);
    let (lo, hi) = bounds;
    let altitude = if hi > lo {
        ((pt.position.y - lo) / (hi - lo)).clamp(0., 1.)
    } else {
        0.
    };
    match which {
        PaintParameter::Falloff => pt.falloff,
        PaintParameter::Pressure => pt.pressure,
        PaintParameter::Slope => slope,
        PaintParameter::Altitude => altitude,
        PaintParameter::InverseSlope => 1. - slope,
        PaintParameter::InverseAltitude => 1. - altitude,
    }
}

/// The factors the two parameters give: (density, size, rotation).
fn effects(s: &MeshPaintSettings, pt: &PaintPoint, bounds: (f64, f64)) -> (f64, f64, f64) {
    let (mut density, mut size, mut rotation) = (1., 1., 1.);
    for (parameter, effect) in [s.parameter1, s.parameter2] {
        let v = parameter_value(parameter, pt, bounds);
        match effect {
            PaintEffect::None => {}
            PaintEffect::Density => density *= v,
            PaintEffect::Size => size *= v,
            PaintEffect::Rotation => rotation *= v,
            PaintEffect::SizeDensity => {
                size *= v;
                density *= v;
            }
        }
    }
    (density, size, rotation)
}

fn snapped(angle_degrees: f64, s: &MeshPaintSettings) -> f64 {
    if s.rotation_snap && s.snap_angle > 0. {
        (angle_degrees / s.snap_angle).round() * s.snap_angle
    } else {
        angle_degrees
    }
}

/// The world matrix of a copy at `pt` (MP-09..13): the source's origin on
/// the point, moved Offset along the normal, turned and sized.
/// `source_radius` is the source's own size (Adaptive), `rotation_factor`
/// and `size_factor` the effects.
pub fn placement(
    stroke: &MeshPaintStroke,
    pt: &PaintPoint,
    rng: &mut Rng,
    source_radius: f64,
    size_factor: f64,
    rotation_factor: f64,
) -> DMat4 {
    let s = &stroke.settings;
    use RotationMode as R;
    // The direction the source's +Y turns onto.
    let up = match s.rotation_mode {
        R::Uniform | R::Random => None,
        R::ScreenAlignTablet | R::ScreenAlignTabletNoPitch => {
            Some(stroke.view_up.normalize_or(DVec3::Y))
        }
        _ => Some(pt.normal),
    };
    let mut turn = match up {
        Some(n) => DQuat::from_rotation_arc(DVec3::Y, n),
        None => DQuat::IDENTITY,
    };
    let axis = up.unwrap_or(DVec3::Y);
    // Heading: the turned +Z along the stroke, in the surface's plane.
    if let Some(h) = pt.heading {
        let flat = h - axis * h.dot(axis);
        if flat.length() > 1e-9 {
            let z = turn * DVec3::Z;
            let angle = z.cross(flat).dot(axis).atan2(z.dot(flat.normalize()));
            turn = DQuat::from_axis_angle(axis, angle) * turn;
        }
    }
    if pt.spin != 0. {
        turn = DQuat::from_axis_angle(axis, pt.spin) * turn;
    }
    // Offsets, or random turns up to them (MP-10).
    let random = matches!(s.rotation_mode, R::Random | R::SurfaceAlignRandom);
    let euler = if random {
        DVec3::new(rng.signed(), rng.signed(), rng.signed()) * s.rotation * rotation_factor
    } else {
        s.rotation
    };
    let euler = DVec3::new(
        snapped(euler.x, s),
        snapped(euler.y, s),
        snapped(euler.z, s),
    );
    let offsets = DQuat::from_euler(
        glam::EulerRot::XYZ,
        euler.x.to_radians(),
        euler.y.to_radians(),
        euler.z.to_radians(),
    );
    let rotation = turn * offsets;
    // Size (MP-11): Size X/Y/Z as multipliers of the source's own size.
    let mut scale = s.size / 100.;
    match s.size_mode {
        SizeMode::Uniform | SizeMode::Locked => {}
        SizeMode::Adaptive | SizeMode::AdaptiveRandom => {
            if source_radius > 1e-12 {
                scale *= stroke.brush_world / source_radius;
            }
        }
        SizeMode::Random => {}
    }
    if matches!(s.size_mode, SizeMode::Random | SizeMode::AdaptiveRandom) {
        // Random: a uniform factor within Size's variation of 1.
        let v = (s.size.x / 100.).clamp(0., 1.);
        scale = DVec3::splat(1. + rng.signed() * v)
            * if s.size_mode == SizeMode::AdaptiveRandom && source_radius > 1e-12 {
                stroke.brush_world / source_radius
            } else {
                1.
            };
    }
    let mut factor = pt.scale * size_factor;
    if s.scale_increment > 0. {
        let step = s.scale_increment / 100.;
        factor = ((factor / step).round() * step).max(step);
    }
    scale *= factor;
    let position = pt.position + pt.normal * s.offset;
    DMat4::from_scale_rotation_translation(scale, rotation, position)
}

impl Scene {
    /// The visible background meshes a Mesh Paint copies: shown meshes
    /// that aren't selected and have polygons.
    pub fn mesh_paint_sources(&self) -> Vec<u64> {
        self.items
            .iter()
            .filter(|i| {
                i.is_mesh()
                    && self.shows(i)
                    && Some(i.id) != self.selection.item
                    && !self.selection.also.contains(&i.id)
                    && !i.mesh.faces.is_empty()
            })
            .map(|i| i.id)
            .collect()
    }

    /// Where a ray from `origin` along `dir` first meets item `id`'s
    /// polygons (front or back), world space: the point and the polygon's
    /// unit normal facing the ray's start.
    pub fn surface_hit(&self, id: u64, origin: DVec3, dir: DVec3) -> Option<(DVec3, DVec3)> {
        let item = self.item(id)?;
        let world = self.world_matrix(id).ok()?;
        let normal_matrix = DMat3::from_mat4(world).inverse().transpose();
        let mesh = &item.mesh;
        let mut best: Option<(f64, DVec3, DVec3)> = None;
        for (f, face) in mesh.faces.iter().enumerate() {
            if mesh.is_polyline(f) || face.vertices.len() < 3 {
                continue;
            }
            let p: Vec<DVec3> = face
                .vertices
                .iter()
                .map(|&v| world.transform_point3(mesh.vertices[v as usize]))
                .collect();
            for i in 1..p.len() - 1 {
                if let Some(t) = ray_triangle(origin, dir, p[0], p[i], p[i + 1])
                    && best.is_none_or(|b| t < b.0)
                {
                    let mut n = (normal_matrix * mesh.normal(f)).normalize_or(DVec3::Y);
                    if n.dot(dir) > 0. {
                        n = -n;
                    }
                    best = Some((t, origin + dir * t, n));
                }
            }
        }
        best.map(|(_, at, n)| (at, n))
    }
}

/// Möller–Trumbore: the distance along `dir` to the triangle, if hit.
fn ray_triangle(o: DVec3, d: DVec3, a: DVec3, b: DVec3, c: DVec3) -> Option<f64> {
    let (e1, e2) = (b - a, c - a);
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-14 {
        return None;
    }
    let inv = 1. / det;
    let s = o - a;
    let u = s.dot(p) * inv;
    if !(-1e-9..=1. + 1e-9).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = d.dot(q) * inv;
    if v < -1e-9 || u + v > 1. + 1e-9 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 1e-9).then_some(t)
}

/// A mesh's own radius about its origin (for Adaptive size).
fn radius(mesh: &Mesh) -> f64 {
    mesh.vertices.iter().map(|v| v.length()).fold(0., f64::max)
}

impl Editor {
    /// One Mesh Paint click or stroke (MP-16): its copies, one undo step.
    pub(crate) fn mesh_paint(&mut self, stroke: MeshPaintStroke) -> Result<()> {
        self.require_idle()?;
        let s = stroke.settings;
        let target = self
            .scene
            .active()
            .filter(|i| i.is_mesh())
            .ok_or("Mesh Paint: select the mesh to paint on")?
            .id;
        let sources = self.scene.mesh_paint_sources();
        if sources.is_empty() {
            return Err(NO_SOURCE.into());
        }
        if stroke.points.is_empty() {
            return Ok(());
        }
        let bounds = {
            let item = self.scene.item(target).ok_or("Mesh Paint: select a mesh")?;
            let world = self.scene.world_matrix(target)?;
            item.mesh
                .vertices
                .iter()
                .map(|&v| world.transform_point3(v).y)
                .fold((f64::MAX, f64::MIN), |(lo, hi), y| (lo.min(y), hi.max(y)))
        };
        // All BG Layers: every source as one mesh; Random: one per copy.
        let merged = (s.source == GeometrySource::AllLayers).then(|| {
            let mut all = Mesh::default();
            for &id in &sources {
                if let Some(item) = self.scene.item(id) {
                    let _ = crate::append_mesh(&mut all, (*item.mesh).clone());
                }
            }
            all
        });
        let mut rng = Rng(stroke.seed);
        let mut placed: Vec<(u64, DMat4)> = Vec::new();
        for pt in &stroke.points {
            let (density, size, rotation) = effects(&s, pt, bounds);
            // Density (MP-04, MP-12): a point stays with that chance.
            if rng.unit() > density {
                continue;
            }
            let source = if merged.is_some() {
                sources[0]
            } else {
                sources[(rng.next_u64() % sources.len() as u64) as usize]
            };
            let source_radius = match &merged {
                Some(m) => radius(m),
                None => self.scene.item(source).map_or(0., |i| radius(&i.mesh)),
            };
            let m = placement(&stroke, pt, &mut rng, source_radius, size, rotation);
            placed.push((source, m));
        }
        if placed.is_empty() {
            return Ok(());
        }
        let before = self.scene.clone();
        let mut scene = self.scene.clone();
        let symmetry = self.active_symmetry();
        if s.replicas {
            let mut ids =
                crate::duplicate_items::Ids::starting(scene.next_id.max(self.next_item_id));
            crate::replicator::add_painted(
                &mut scene, &sources, &placed, symmetry, target, &mut ids,
            )?;
            scene.next_id = ids.next();
        } else if s.instances {
            self.paint_instances(&mut scene, target, &placed, symmetry, s.parent)?;
        } else {
            let to_local = scene.world_matrix(target)?.inverse();
            let mut copies = Mesh::default();
            for (source, m) in &placed {
                let mut copy = match &merged {
                    Some(all) => all.clone(),
                    None => (*scene.item(*source).ok_or(NO_SOURCE)?.mesh).clone(),
                };
                let into = to_local * *m;
                for v in &mut copy.vertices {
                    *v = into.transform_point3(*v);
                }
                if into.determinant() < 0. {
                    copy = turned_round(copy)?;
                }
                crate::append_mesh(&mut copies, copy)?;
            }
            // MP-17: with Symmetry on, each copy is mirrored too.
            let copies = symmetry.doubled(copies)?;
            let item = scene.item_mut(target).ok_or("Mesh Paint: select a mesh")?;
            crate::append_mesh(&mut item.mesh, copies)?;
        }
        scene.validate()?;
        self.scene = scene;
        self.next_item_id = self.next_item_id.max(self.scene.next_id);
        if let Err(e) = self.identities.synchronize(&self.scene) {
            self.scene = before;
            return Err(e);
        }
        let label = if s.replicas {
            "Mesh Paint Replica"
        } else if s.instances {
            "Mesh Paint Instance"
        } else {
            "Mesh Paint"
        };
        self.record(label.into(), before);
        Ok(())
    }

    /// Mesh Paint Instance (MP-15): an instance item of each copy's
    /// source, placed at its world matrix, parented per Parent.
    fn paint_instances(
        &self,
        scene: &mut Scene,
        target: u64,
        placed: &[(u64, DMat4)],
        symmetry: crate::Symmetry,
        parent: crate::duplicate_items::ParentMode,
    ) -> Result<()> {
        let target_world = scene.world_matrix(target)?;
        // MP-17: the symmetry plane (in the target's own space) as a world
        // reflection.
        let mirror = symmetry.normal().map(|n| {
            let reflect = DMat4::from_mat3(DMat3::IDENTITY - 2. * outer(n));
            let shift = DMat4::from_translation(n * (2. * symmetry.offset));
            target_world * shift * reflect * target_world.inverse()
        });
        let mut ids = crate::duplicate_items::Ids::starting(scene.next_id.max(self.next_item_id));
        for &(source, m) in placed {
            // place_items puts each placement on top of the source's own
            // world matrix; the copy's world is to be `m` (MP-13).
            let source_world = scene.world_matrix(source)?;
            let mut at = vec![m * source_world.inverse()];
            if let Some(r) = mirror {
                at.push(r * m * source_world.inverse());
            }
            crate::duplicate_items::place_items(
                scene,
                source,
                &at,
                &crate::Copies::default(),
                crate::duplicate_items::ItemCopies {
                    parent,
                    hierarchy: false,
                },
                true,
                &mut ids,
            )?;
        }
        scene.next_id = ids.next();
        Ok(())
    }
}

/// `n nᵀ`.
fn outer(n: DVec3) -> DMat3 {
    DMat3::from_cols(n * n.x, n * n.y, n * n.z)
}

/// Every polygon of `mesh` turned to face the other way (Flip's rule).
fn turned_round(mesh: Mesh) -> Result<Mesh> {
    let mut scene = Scene::default();
    scene.new_mesh()?;
    scene.active_mut().ok_or("Missing mesh")?.mesh = mesh.into();
    scene.selection.mode = crate::Mode::Item;
    let flipped = crate::flip::flip_winding(&scene)?;
    Ok((*flipped.active().ok_or("Missing mesh")?.mesh).clone())
}
