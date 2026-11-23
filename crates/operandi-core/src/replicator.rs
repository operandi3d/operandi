//! Replicators (G8 Replica family, RP-1..RP-5): copies of a prototype item
//! on the points of a point cloud, without one item per copy.
//!
//! A Replica tool (the Replica buttons, `Copier::Replica`) places exactly
//! where its Instance sibling would, but instead of N instance items it
//! makes three: a group ("proto Replicator") at the root holding a hidden
//! points-only mesh ("proto (Point Cloud)") and the Replicator item
//! ("proto (Replicator)"). The original stays and counts as the first copy,
//! so the cloud has a point for each other copy.
//!
//! Each point is where that copy's pivot goes; its rotation and scale (a
//! mirror is a negative X scale) are kept in two vector maps on the cloud,
//! [`ROTATION_MAP`] (XYZ angles, radians, as item rotations) and
//! [`SCALE_MAP`]. A cloud without them (any mesh chosen as Point Source)
//! places unrotated, unscaled copies.
//!
//! The evaluated scene (`evaluate.rs`) shows every copy as a Mesh Instance
//! of the prototype (ids from [`copy_id`]), so the viewport, Cycles and the
//! USD export read ordinary instances; the document keeps the one item.
//!
//! Provisional (docs/SPECIFICATION_QUESTIONS.md):
//! - Random values follow Blender's Instance on Points + Random Value: a
//!   uniform value in [−range, +range] per point and channel from the Seed
//!   (scale: ±range % around 100 %), applied on top of the point's own
//!   transform; Snap Rotation rounds random angles to 90° steps (RP-Q1).
//! - Use Source Transforms on multiplies the prototype's own world rotation
//!   and scale in; off, copies take only the point's.
//! - Point Source reads points (Vertices); Surface mode is not built
//!   (RP-Q3).
use crate::{Item, Mesh, Result, Scene, Transform, TypedMap};
use glam::{DMat3, DMat4, DVec3};
use serde::{Deserialize, Serialize};

/// The cloud's per-point rotation map (vector3, XYZ angles in radians).
pub const ROTATION_MAP: &str = "Replica Rotation";
/// The cloud's per-point scale map (vector3; a mirror is a negative X).
pub const SCALE_MAP: &str = "Replica Scale";
/// The cloud's per-point pick among `Replicator::members` (weight1, an
/// index), for Mesh Paint Replica.
pub const MEMBER_MAP: &str = "Replica Member";
/// RP-6: what a Replica tool says with nothing to copy.
pub const NOTHING_TO_REPLICATE: &str = "Select an item to replicate";
/// Most copies one Replicator shows. Copies are drawn and rendered as
/// instances, so this is far above any one point cloud; past it the
/// Replicator panel says how many points are left out (hidden-limits A4,
/// was 1,048,576 with no note).
pub const MAX_COPIES: usize = 1 << 24;

/// How many of the Replicator `id`'s points get no copy because of
/// `MAX_COPIES` (0 when all fit, or `id` isn't a Replicator).
pub fn copies_left_out(scene: &Scene, id: u64) -> usize {
    let Some(rep) = scene.item(id).and_then(|i| i.replicator.as_ref()) else {
        return 0;
    };
    scene.item(rep.point_source).map_or(0, |cloud| {
        cloud
            .mesh
            .read()
            .as_read()
            .point_count()
            .saturating_sub(MAX_COPIES)
    })
}

/// A Replicator's settings (RP-2), shown in Properties.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Replicator {
    /// The mesh whose points place the copies (the point cloud).
    pub point_source: u64,
    /// The item copied: a mesh item, or a group whose mesh members each
    /// point picks one of at random.
    pub prototype: u64,
    /// Mesh Paint Replica (RP-7): several background meshes, one picked per
    /// point (the cloud's [`MEMBER_MAP`] says which); empty otherwise.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<u64>,
    pub use_source_transforms: bool,
    /// Copies include the prototype's children.
    pub hierarchy: bool,
    /// Metres per axis.
    pub random_position: DVec3,
    /// Degrees per axis.
    pub random_rotation: DVec3,
    /// Percent per axis.
    pub random_scale: DVec3,
    pub uniform_scale: bool,
    /// Percent.
    pub base_scale: f64,
    pub snap_rotation: bool,
    /// Percent of points that get a copy in renders and exports.
    pub density: f64,
    /// Percent of points drawn in the viewport.
    pub display_density: f64,
    pub seed: u32,
}

impl Default for Replicator {
    fn default() -> Self {
        Self {
            point_source: 0,
            prototype: 0,
            members: Vec::new(),
            use_source_transforms: false,
            hierarchy: false,
            random_position: DVec3::ZERO,
            random_rotation: DVec3::ZERO,
            random_scale: DVec3::ZERO,
            uniform_scale: true,
            base_scale: 100.,
            snap_rotation: false,
            density: 100.,
            display_density: 100.,
            seed: 401,
        }
    }
}

/// RP-Q1 (provisional): Snap Rotation's step.
pub const SNAP_STEP_DEGREES: f64 = 90.;

impl Replicator {
    pub fn validate(&self) -> Result<()> {
        let finite = [
            self.random_position,
            self.random_rotation,
            self.random_scale,
        ]
        .iter()
        .all(|v| v.is_finite())
            && self.base_scale.is_finite()
            && self.density.is_finite()
            && self.display_density.is_finite();
        if !finite {
            return Err("Replicator settings must be finite".into());
        }
        if !(0.0..=100.).contains(&self.density) || !(0.0..=100.).contains(&self.display_density) {
            return Err("Density is a percentage from 0 to 100".into());
        }
        Ok(())
    }
}

// ---------------------------------------------------------- tool options

/// RP-6: what a Replica tool replicates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReplicaSource {
    /// The selected (active) items, each its own Replicator.
    #[default]
    ActiveItem,
    SpecificItem,
    AllBackgroundMeshes,
    MeshLayerPreset,
    RandomPresets,
}
impl ReplicaSource {
    pub const ALL: [ReplicaSource; 5] = [
        ReplicaSource::ActiveItem,
        ReplicaSource::SpecificItem,
        ReplicaSource::AllBackgroundMeshes,
        ReplicaSource::MeshLayerPreset,
        ReplicaSource::RandomPresets,
    ];
    pub fn label(self) -> &'static str {
        match self {
            ReplicaSource::ActiveItem => "Active Item",
            ReplicaSource::SpecificItem => "Specific Item",
            ReplicaSource::AllBackgroundMeshes => "All Background Meshes",
            ReplicaSource::MeshLayerPreset => "Mesh Layer Preset",
            ReplicaSource::RandomPresets => "Random Presets",
        }
    }
    /// Built in the placement tools (RP-6; the others are provisional,
    /// RP-Q4).
    pub fn built(self) -> bool {
        self == ReplicaSource::ActiveItem
    }
}

/// RP-6: where a Replica tool's new group goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReplicaParent {
    /// At the root.
    #[default]
    Scene,
    /// Under the replicated item.
    Selected,
}
impl ReplicaParent {
    pub const ALL: [ReplicaParent; 2] = [ReplicaParent::Scene, ReplicaParent::Selected];
    pub fn label(self) -> &'static str {
        match self {
            ReplicaParent::Scene => "Scene",
            ReplicaParent::Selected => "Selected",
        }
    }
}

/// A Replica tool's Replica section (RP-6).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReplicaOptions {
    pub source: ReplicaSource,
    pub parent: ReplicaParent,
}

// ---------------------------------------------------------- copy ids

/// Evaluated copies get ids from here up, out of the document's reach.
const COPY_BASE: u64 = 1 << 62;
/// Copies per Replicator in the id space.
const COPY_BITS: u32 = 24;

/// The evaluated scene's id of copy `k` of Replicator `replicator`.
pub fn copy_id(replicator: u64, k: usize) -> u64 {
    COPY_BASE | (replicator << COPY_BITS) | (k as u64 & ((1 << COPY_BITS) - 1))
}

/// The Replicator a copy in the evaluated scene belongs to; `None` for a
/// document item.
pub fn copy_replicator(id: u64) -> Option<u64> {
    (id >= COPY_BASE).then_some((id & !COPY_BASE) >> COPY_BITS)
}

// ---------------------------------------------------------- random

/// A repeatable value in [0, 1) for point `k`, channel `c` (splitmix64).
fn random(seed: u32, k: usize, c: u64) -> f64 {
    let mut z = (seed as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((k as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add(c.wrapping_mul(0x94D0_49BB_1331_11EB));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// A repeatable value in [−1, 1].
fn signed(seed: u32, k: usize, c: u64) -> f64 {
    random(seed, k, c) * 2. - 1.
}

// ---------------------------------------------------------- points

/// Each point's transform in the cloud's own space: its position, and its
/// rotation and scale from the Replica maps when the cloud has them.
pub fn point_transforms(mesh: &Mesh) -> Vec<DMat4> {
    let maps = &mesh.attributes.typed_maps;
    let read = |name: &str, v: usize| -> Option<DVec3> {
        let x = maps.get(name)?.points.get(&(v as u32))?;
        (x.len() == 3).then(|| DVec3::new(x[0], x[1], x[2]))
    };
    (0..mesh.vertices.len())
        .map(|v| {
            Transform {
                translation: mesh.vertices[v],
                rotation: read(ROTATION_MAP, v).unwrap_or(DVec3::ZERO),
                scale: read(SCALE_MAP, v).unwrap_or(DVec3::ONE),
                zxy: false,
                order: None,
            }
            .matrix()
        })
        .collect()
}

/// A points-only cloud of `places` (world matrices of the copies' pivots
/// with their rotation and scale).
pub fn cloud(places: &[Transform]) -> Mesh {
    let mut mesh = Mesh {
        vertices: places.iter().map(|t| t.translation).collect(),
        ..Default::default()
    };
    let map = |values: Vec<(u32, Vec<f64>)>| TypedMap {
        kind: "vector3".into(),
        dimension: 3,
        zero_default: false,
        points: values.into_iter().collect(),
        corners: Vec::new(),
    };
    let rotations = places
        .iter()
        .enumerate()
        .filter(|(_, t)| t.rotation != DVec3::ZERO)
        .map(|(v, t)| (v as u32, t.rotation.to_array().to_vec()))
        .collect::<Vec<_>>();
    let scales = places
        .iter()
        .enumerate()
        .filter(|(_, t)| t.scale != DVec3::ONE)
        .map(|(v, t)| (v as u32, t.scale.to_array().to_vec()))
        .collect::<Vec<_>>();
    if !rotations.is_empty() {
        mesh.attributes
            .typed_maps
            .insert(ROTATION_MAP.into(), map(rotations));
    }
    if !scales.is_empty() {
        mesh.attributes
            .typed_maps
            .insert(SCALE_MAP.into(), map(scales));
    }
    mesh
}

/// One copy a Replicator makes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Copy {
    /// The mesh item copied (the prototype, or the group member picked).
    pub source: u64,
    /// The copy's world matrix.
    pub world: DMat4,
}

/// The rotation and scale of `m` without its translation.
fn linear_only(m: DMat4) -> DMat4 {
    DMat4::from_mat3(DMat3::from_mat4(m))
}

/// Replicator `id`'s copies: for renders (`display` off, Density) or the
/// viewport (Display Density). Empty when its cloud or prototype is gone.
pub fn copies(scene: &Scene, id: u64, display: bool) -> Vec<Copy> {
    let Some(rep) = scene.item(id).and_then(|i| i.replicator.as_deref()) else {
        return Vec::new();
    };
    let Some(cloud) = scene.item(rep.point_source).filter(|i| i.is_mesh()) else {
        return Vec::new();
    };
    let Ok(cloud_world) = scene.world_matrix(cloud.id) else {
        return Vec::new();
    };
    let Some(proto) = scene.item(rep.prototype) else {
        return Vec::new();
    };
    // A group prototype: each point picks one of its mesh members.
    let members: Vec<u64> = if !rep.members.is_empty() {
        rep.members
            .iter()
            .copied()
            .filter(|m| scene.item(*m).is_some_and(|i| i.is_mesh()))
            .collect()
    } else if proto.is_mesh() {
        vec![proto.id]
    } else {
        scene
            .items
            .iter()
            .filter(|i| i.parent == Some(proto.id) && i.is_mesh())
            .map(|i| i.id)
            .collect()
    };
    if members.is_empty() {
        return Vec::new();
    }
    let share = if display {
        rep.display_density
    } else {
        rep.density
    } / 100.;
    let seed = rep.seed;
    let points = point_transforms(&cloud.mesh);
    let picked = cloud.mesh.attributes.typed_maps.get(MEMBER_MAP);
    let mut out = Vec::with_capacity(points.len().min(MAX_COPIES));
    for (k, point) in points.into_iter().enumerate() {
        if out.len() >= MAX_COPIES {
            break;
        }
        if share < 1. && random(seed, k, 0) >= share {
            continue;
        }
        let pick = picked
            .and_then(|m| m.points.get(&(k as u32)))
            .and_then(|v| v.first())
            .map(|&i| i.max(0.) as usize)
            .unwrap_or((random(seed, k, 1) * members.len() as f64) as usize);
        let member = members[pick.min(members.len() - 1)];
        let rp = rep.random_position;
        let offset = DVec3::new(
            signed(seed, k, 2) * rp.x,
            signed(seed, k, 3) * rp.y,
            signed(seed, k, 4) * rp.z,
        );
        let rr = rep.random_rotation;
        let mut angles = DVec3::new(
            signed(seed, k, 5) * rr.x,
            signed(seed, k, 6) * rr.y,
            signed(seed, k, 7) * rr.z,
        );
        if rep.snap_rotation {
            angles = (angles / SNAP_STEP_DEGREES).round() * SNAP_STEP_DEGREES;
        }
        let rs = rep.random_scale;
        let factor = if rep.uniform_scale {
            let u = signed(seed, k, 8);
            DVec3::new(u * rs.x, u * rs.x, u * rs.x)
        } else {
            DVec3::new(
                signed(seed, k, 8) * rs.x,
                signed(seed, k, 9) * rs.y,
                signed(seed, k, 10) * rs.z,
            )
        };
        let scale = (DVec3::ONE + factor / 100.) * (rep.base_scale / 100.);
        let random = Transform {
            translation: offset,
            rotation: DVec3::new(
                angles.x.to_radians(),
                angles.y.to_radians(),
                angles.z.to_radians(),
            ),
            scale,
            zxy: false,
            order: None,
        }
        .matrix();
        let own = if rep.use_source_transforms {
            scene
                .world_matrix(member)
                .map_or(DMat4::IDENTITY, linear_only)
        } else {
            DMat4::IDENTITY
        };
        let world = cloud_world * point * random * own;
        if world.is_finite() {
            out.push(Copy {
                source: member,
                world,
            });
        }
    }
    out
}

/// `scene` with every Replicator's copies added as Mesh Instance items
/// (ids from [`copy_id`]); `None` when it has no Replicator.
pub fn expand(scene: &Scene, display: bool) -> Option<Scene> {
    let reps: Vec<u64> = scene
        .items
        .iter()
        .filter(|i| i.replicator.is_some())
        .map(|i| i.id)
        .collect();
    if reps.is_empty() {
        return None;
    }
    let mut out = scene.clone();
    for rep in reps {
        // A hidden Replicator (or a hiding parent) shows no copies.
        let Some(item) = scene.item(rep) else {
            continue;
        };
        if !scene.shows(item) {
            continue;
        }
        let hierarchy = item.replicator.as_ref().is_some_and(|r| r.hierarchy);
        let mut n = 0usize;
        for copy in copies(scene, rep, display) {
            let Some(source) = scene.item(copy.source) else {
                continue;
            };
            let Ok(transform) =
                crate::duplicate_items::local_transform(DMat4::IDENTITY, copy.world)
            else {
                continue;
            };
            let id = copy_id(rep, n);
            n += 1;
            let mut c = virtual_copy(scene, source, id);
            c.transform = transform;
            c.parent = None;
            out.items.push(c);
            if hierarchy {
                // The prototype's children (all levels) under each copy.
                let mut map = vec![(source.id, id)];
                let mut i = 0;
                while i < map.len() {
                    let (old, new) = map[i];
                    i += 1;
                    for child in scene.items.iter().filter(|c| c.parent == Some(old)) {
                        let cid = copy_id(rep, n);
                        n += 1;
                        let mut c = virtual_copy(scene, child, cid);
                        c.parent = Some(new);
                        out.items.push(c);
                        map.push((child.id, cid));
                    }
                }
            }
        }
    }
    Some(out)
}

/// A copy of `item` as the evaluated scene shows it: a Mesh Instance of
/// what it shows (or a plain copy of a non-mesh item).
fn virtual_copy(scene: &Scene, item: &Item, id: u64) -> Item {
    let mut c = item.clone();
    c.id = id;
    c.visible = true;
    c.hides_children = false;
    c.source_id = None;
    c.source_display_name = None;
    c.mesh_stack = None;
    if item.is_mesh() {
        c.instance_of = Some(scene.instance_source(item.id).unwrap_or(item.id));
    }
    c
}

// ---------------------------------------------------------- boxes

/// RP-3: a wire box over `mesh`'s bounds, as twelve two-point lines; the
/// same shared box for every copy of one mesh (so the viewport keeps one
/// GPU copy).
pub fn box_lines(mesh: &crate::Shared<Mesh>) -> crate::MeshSlot {
    let slot = mesh.derived(|m| BoxLines(crate::MeshSlot::from(box_mesh(m))));
    slot.0.clone()
}
struct BoxLines(crate::MeshSlot);

fn box_mesh(m: &Mesh) -> Mesh {
    let (lo, hi) = m
        .vertices
        .iter()
        .fold(None, |b: Option<(DVec3, DVec3)>, p| {
            Some(b.map_or((*p, *p), |(lo, hi)| (lo.min(*p), hi.max(*p))))
        })
        .unwrap_or((DVec3::splat(-0.05), DVec3::splat(0.05)));
    let corner = |i: usize| {
        DVec3::new(
            if i & 1 == 0 { lo.x } else { hi.x },
            if i & 2 == 0 { lo.y } else { hi.y },
            if i & 4 == 0 { lo.z } else { hi.z },
        )
    };
    let mut out = Mesh {
        vertices: (0..8).map(corner).collect(),
        ..Default::default()
    };
    for (a, b) in [
        (0, 1),
        (2, 3),
        (4, 5),
        (6, 7),
        (0, 2),
        (1, 3),
        (4, 6),
        (5, 7),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ] {
        out.faces.push(crate::Face {
            vertices: vec![a, b],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
        out.attributes
            .representations
            .push(crate::PolygonRepresentation::Polyline {
                closed: false,
                curve: crate::curves::CurveKind::Line,
            });
    }
    out
}

/// `scene` (an evaluated scene) with the copies of every Replicator that
/// `shaded` turns down drawn as wire boxes of their prototype; `None` when
/// nothing changes.
pub fn boxed(scene: &Scene, shaded: impl Fn(u64) -> bool) -> Option<Scene> {
    let boxes: Vec<usize> = scene
        .items
        .iter()
        .enumerate()
        .filter(|(_, i)| copy_replicator(i.id).is_some_and(|r| !shaded(r)) && i.is_mesh())
        .map(|(n, _)| n)
        .collect();
    if boxes.is_empty() {
        return None;
    }
    let mut out = scene.clone();
    for n in boxes {
        let item = &mut out.items[n];
        item.mesh = box_lines(item.mesh.shared());
        item.subdivision = None;
        item.instance_of = None;
    }
    Some(out)
}

// ---------------------------------------------------------- making one

/// A new empty item named `name` (a template for the three items).
fn blank(template: &Item, id: u64, name: String) -> Item {
    let mut i = template.clone();
    i.id = id;
    i.name = name;
    i.visible = true;
    i.hides_children = false;
    i.transform = Transform::default();
    i.mesh = Mesh::default().into();
    i.parent = None;
    i.source_id = None;
    i.source_display_name = None;
    i.subdivision = None;
    i.backdrop = None;
    i.camera = None;
    i.light = None;
    i.texture = None;
    i.locator = None;
    i.environment = None;
    i.common = Default::default();
    i.elements = Default::default();
    i.mesh_stack = None;
    i.kit = None;
    i.instance_of = None;
    i.replicator = None;
    i
}

/// The three items of a new Replicator of `prototype` (RP-1): the group,
/// the hidden cloud with a point per `places` entry (world matrices of the
/// copies) and the Replicator. Added at the end of the item list; the
/// selection is left alone. Returns the Replicator's id.
pub(crate) fn add(
    scene: &mut Scene,
    prototype: u64,
    places: &[DMat4],
    options: ReplicaOptions,
    ids: &mut crate::duplicate_items::Ids,
) -> Result<u64> {
    if !options.source.built() {
        return Err(format!(
            "Replica Source {} isn't built yet; use Active Item",
            options.source.label()
        ));
    }
    let proto = scene.item(prototype).ok_or(NOTHING_TO_REPLICATE)?.clone();
    if scene.items.len() + 3 > crate::MAX_SCENE_ITEMS {
        return Err("Scene item/identity budget exhausted".into());
    }
    if places.len() > MAX_COPIES {
        return Err(format!("Too many copies ({})", places.len()));
    }
    let world = scene.world_matrix(prototype)?;
    // Each point is the copy's pivot with its own rotation and scale: the
    // placement applied to the prototype's pivot, without the prototype's
    // own rotation and scale (Use Source Transforms adds them back).
    let pivot = DMat4::from_translation(world.w_axis.truncate());
    let mut transforms = Vec::with_capacity(places.len());
    for place in places {
        let at = *place * world * linear_only(world).inverse();
        let m = if at.is_finite() { at } else { *place * pivot };
        transforms.push(crate::duplicate_items::local_transform(DMat4::IDENTITY, m)?);
    }
    let name = |scene: &Scene, base: String| scene.free_name(&base);
    let group_id = ids.take()?;
    let cloud_id = ids.take()?;
    let rep_id = ids.take()?;
    let group_name = name(scene, format!("{} Replicator", proto.name));
    let mut group = blank(&proto, group_id, group_name);
    group.locator = Some(crate::locator::LocatorKind::Group);
    if options.parent == ReplicaParent::Selected {
        // Under the replicated item, kept where it is in the world.
        group.parent = Some(prototype);
        group.transform = crate::duplicate_items::local_transform(world, DMat4::IDENTITY)?;
    }
    let cloud_name = name(scene, format!("{} (Point Cloud)", proto.name));
    let mut cloud_item = blank(&proto, cloud_id, cloud_name);
    cloud_item.mesh = cloud(&transforms).into();
    cloud_item.parent = Some(group_id);
    cloud_item.visible = false;
    let rep_name = name(scene, format!("{} (Replicator)", proto.name));
    let mut rep = blank(&proto, rep_id, rep_name);
    rep.parent = Some(group_id);
    rep.replicator = Some(Box::new(Replicator {
        point_source: cloud_id,
        prototype,
        ..Default::default()
    }));
    scene.items.extend([group, cloud_item, rep]);
    Ok(rep_id)
}

/// RP-7 Mesh Paint Replica: one Replicator for a stroke's copies
/// (`placed`: the background mesh each copy shows and its world matrix),
/// named after the first background mesh. With several, the cloud records
/// which one each point shows. Symmetry adds each copy's mirror.
pub(crate) fn add_painted(
    scene: &mut Scene,
    sources: &[u64],
    placed: &[(u64, DMat4)],
    symmetry: crate::Symmetry,
    target: u64,
    ids: &mut crate::duplicate_items::Ids,
) -> Result<u64> {
    let first = *sources.first().ok_or(NOTHING_TO_REPLICATE)?;
    let proto = scene.item(first).ok_or(NOTHING_TO_REPLICATE)?.clone();
    let target_world = scene.world_matrix(target)?;
    let mirror = symmetry.normal().map(|n| {
        let outer = DMat3::from_cols(n * n.x, n * n.y, n * n.z);
        let reflect = DMat4::from_mat3(DMat3::IDENTITY - 2. * outer);
        let shift = DMat4::from_translation(n * (2. * symmetry.offset));
        target_world * shift * reflect * target_world.inverse()
    });
    let mut places: Vec<(usize, DMat4)> = Vec::new();
    for &(source, m) in placed {
        let index = sources.iter().position(|s| *s == source).unwrap_or(0);
        places.push((index, m));
        if let Some(r) = mirror {
            places.push((index, r * m));
        }
    }
    if places.len() > MAX_COPIES {
        return Err(format!("Too many copies ({})", places.len()));
    }
    let transforms: Vec<Transform> = places
        .iter()
        .map(|(_, m)| crate::duplicate_items::local_transform(DMat4::IDENTITY, *m))
        .collect::<Result<_>>()?;
    let mut cloud_mesh = cloud(&transforms);
    if sources.len() > 1 {
        cloud_mesh.attributes.typed_maps.insert(
            MEMBER_MAP.into(),
            TypedMap {
                kind: "weight1".into(),
                dimension: 1,
                zero_default: false,
                points: places
                    .iter()
                    .enumerate()
                    .map(|(v, (i, _))| (v as u32, vec![*i as f64]))
                    .collect(),
                corners: Vec::new(),
            },
        );
    }
    let group_id = ids.take()?;
    let cloud_id = ids.take()?;
    let rep_id = ids.take()?;
    let group_name = scene.free_name(&format!("{} Replicator", proto.name));
    let mut group = blank(&proto, group_id, group_name);
    group.locator = Some(crate::locator::LocatorKind::Group);
    let cloud_name = scene.free_name(&format!("{} (Point Cloud)", proto.name));
    let mut cloud_item = blank(&proto, cloud_id, cloud_name);
    cloud_item.mesh = cloud_mesh.into();
    cloud_item.parent = Some(group_id);
    cloud_item.visible = false;
    let rep_name = scene.free_name(&format!("{} (Replicator)", proto.name));
    let mut rep = blank(&proto, rep_id, rep_name);
    rep.parent = Some(group_id);
    rep.replicator = Some(Box::new(Replicator {
        point_source: cloud_id,
        prototype: first,
        members: if sources.len() > 1 {
            sources.to_vec()
        } else {
            Vec::new()
        },
        ..Default::default()
    }));
    scene.items.extend([group, cloud_item, rep]);
    Ok(rep_id)
}

/// RP-5 Freeze Replicator (PROVISIONAL RP-Q2): Replicator `id` becomes one
/// Mesh Instance per copy (Density's share) in its group, named after the
/// prototype; the Replicator and its point cloud go (the cloud only when
/// it sits in the same group). Returns the group's id.
pub(crate) fn freeze(
    scene: &mut Scene,
    id: u64,
    ids: &mut crate::duplicate_items::Ids,
) -> Result<u64> {
    let item = scene
        .item(id)
        .filter(|i| i.replicator.is_some())
        .ok_or("Select a Replicator to freeze")?
        .clone();
    let rep = item.replicator.as_deref().cloned().unwrap_or_default();
    let made = copies(scene, id, false);
    let group = match item.parent {
        Some(g) => g,
        None => {
            let gid = ids.take()?;
            let name = scene.free_name(&format!("{} Instances", item.name));
            let mut g = blank(&item, gid, name);
            g.locator = Some(crate::locator::LocatorKind::Group);
            scene.items.push(g);
            gid
        }
    };
    let group_world = scene.world_matrix(group)?;
    if scene.items.len() + made.len() > crate::MAX_SCENE_ITEMS {
        return Err(format!("Too many copies ({} items)", made.len()));
    }
    let mut new_items = Vec::with_capacity(made.len());
    let mut taken: std::collections::BTreeSet<String> =
        scene.items.iter().map(|i| i.name.clone()).collect();
    for copy in &made {
        let source = scene.item(copy.source).ok_or("The prototype is gone")?;
        let mut c = virtual_copy(scene, source, ids.take()?);
        c.parent = Some(group);
        c.transform = crate::duplicate_items::local_transform(group_world, copy.world)?;
        // "proto (2)", "proto (3)", ... after the names already taken.
        let base = source.name.clone();
        c.name = (2..)
            .map(|n| format!("{base} ({n})"))
            .find(|n| !taken.contains(n))
            .unwrap_or(base);
        taken.insert(c.name.clone());
        new_items.push(c);
    }
    let cloud_goes = scene
        .item(rep.point_source)
        .is_some_and(|c| c.parent == item.parent && item.parent.is_some());
    scene
        .items
        .retain(|i| i.id != id && !(cloud_goes && i.id == rep.point_source));
    scene.items.extend(new_items);
    scene.sync_instances(None);
    Ok(group)
}

// ---------------------------------------------------------- editor

impl crate::Editor {
    /// The Replicator the selection names: a selected Replicator item, or
    /// the one inside a selected group.
    fn selected_replicator(&self) -> Option<u64> {
        let id = self.scene.selection.item?;
        let item = self.scene.item(id)?;
        if item.replicator.is_some() {
            return Some(id);
        }
        self.scene
            .items
            .iter()
            .find(|i| i.parent == Some(id) && i.replicator.is_some())
            .map(|i| i.id)
    }

    /// RP-5 Freeze Replicator on the selected Replicator, one undo step;
    /// the new group ends up selected.
    pub(crate) fn freeze_replicator(&mut self) -> Result<()> {
        self.require_idle()?;
        let id = self
            .selected_replicator()
            .ok_or("Select a Replicator to freeze")?;
        let before = self.scene.clone();
        let mut scene = self.scene.clone();
        let mut ids = crate::duplicate_items::Ids::starting(scene.next_id.max(self.next_item_id));
        let group = freeze(&mut scene, id, &mut ids)?;
        scene.next_id = ids.next();
        scene.select_item(Some(group));
        self.commit_replicator_edit(scene, before, "Freeze Replicator")
    }

    /// RP-2: Replicator `item`'s settings, one undo step.
    pub(crate) fn set_replicator(&mut self, item: u64, settings: Replicator) -> Result<()> {
        self.require_idle()?;
        settings.validate()?;
        let before = self.scene.clone();
        let mut scene = self.scene.clone();
        let target = scene
            .item_mut(item)
            .filter(|i| i.replicator.is_some())
            .ok_or("That item isn't a Replicator")?;
        if target.replicator.as_deref() == Some(&settings) {
            return Ok(());
        }
        target.replicator = Some(Box::new(settings));
        self.commit_replicator_edit(scene, before, "Replicator Settings")
    }

    fn commit_replicator_edit(&mut self, scene: Scene, before: Scene, label: &str) -> Result<()> {
        let ids_before = self.identities.current.clone();
        self.scene = scene;
        self.next_item_id = self.next_item_id.max(self.scene.next_id);
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.synchronize(&self.scene))
        {
            self.scene = before;
            self.identities.current = ids_before;
            return Err(e);
        }
        self.record(label.into(), before);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_ids_name_their_replicator() {
        let id = copy_id(42, 7);
        assert_eq!(copy_replicator(id), Some(42));
        assert_eq!(copy_replicator(42), None);
    }

    #[test]
    fn random_values_repeat_and_spread() {
        assert_eq!(random(401, 3, 5), random(401, 3, 5));
        assert_ne!(random(401, 3, 5), random(401, 4, 5));
        let mean = (0..1000).map(|k| random(1, k, 0)).sum::<f64>() / 1000.;
        assert!((mean - 0.5).abs() < 0.05);
    }

    #[test]
    fn a_cloud_keeps_rotation_and_scale_per_point() {
        let t = Transform {
            translation: DVec3::new(1., 2., 3.),
            rotation: DVec3::new(0., 1., 0.),
            scale: DVec3::new(-1., 1., 1.),
            zxy: false,
            order: None,
        };
        let m = cloud(&[Transform::default(), t.clone()]);
        m.validate().unwrap();
        let back = point_transforms(&m);
        assert!(back[0].abs_diff_eq(DMat4::IDENTITY, 1e-12));
        assert!(back[1].abs_diff_eq(t.matrix(), 1e-12));
    }
}
