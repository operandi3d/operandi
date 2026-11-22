//! Scatters as PointInstancers (USD-IQ25, provisional). When at least
//! [`threshold`] Mesh Instances of one source share a parent, have no
//! items below them and nothing of their own that would make them a
//! copy (other materials or subdivision settings), and a plain
//! translate / rotate / positive scale, they are written as one
//! UsdGeomPointInstancer under that parent instead of one instanceable
//! Xform each: positions, orientations (quath and quatf), scales and
//! protoIndices, with the prototype an instanceable reference to the
//! source's `/root/_prototypes` meshes. Their names (customData
//! `operandi:names`) and eyes (invisibleIds) go with it, so the import
//! brings the same items back. Fewer, or any that differ, stay
//! instanceable Xforms.
use crate::plan::{Notes, Plan};
use glam::{DMat3, DQuat};
use operandi_core::{Item, Scene};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

/// Fewest Mesh Instances of one source under one parent written as a
/// PointInstancer. Below this the instanceable Xforms cost little and
/// keep every instance a named prim other programs can pick.
pub const DEFAULT_THRESHOLD: usize = 16;

thread_local! {
    static THRESHOLD: Cell<usize> = const { Cell::new(DEFAULT_THRESHOLD) };
}

/// The threshold exports on this thread use.
pub fn threshold() -> usize {
    THRESHOLD.with(Cell::get)
}

/// Runs `f` with another threshold (`usize::MAX`: never), for comparing
/// the two ways of writing a scatter.
pub fn with_threshold<R>(n: usize, f: impl FnOnce() -> R) -> R {
    let old = THRESHOLD.with(|t| t.replace(n));
    let out = f();
    THRESHOLD.with(|t| t.set(old));
    out
}

/// One PointInstancer prim.
#[derive(Clone, Debug, PartialEq)]
pub struct InstancerPrim {
    pub name: String,
    /// Index into [`Plan::prototypes`].
    pub prototype: usize,
    /// Per instance, in the parent's space.
    pub names: Vec<String>,
    pub positions: Vec<[f32; 3]>,
    /// x, y, z, w.
    pub orientations: Vec<[f32; 4]>,
    pub scales: Vec<[f32; 3]>,
    /// Instances whose eye is off (their index, also their id).
    pub invisible: Vec<i64>,
}

impl InstancerPrim {
    /// The description `ou_define_point_instancer` reads, and its buffers.
    pub fn json_and_buffers(&self, prototype: &str) -> (String, Vec<Vec<u8>>) {
        fn bytes<T: Copy, const N: usize>(v: &[[T; N]], f: impl Fn(T) -> [u8; 4]) -> Vec<u8> {
            v.iter().flatten().flat_map(|&x| f(x)).collect()
        }
        let n = self.positions.len();
        let mut buffers = vec![
            (0..n as i32)
                .map(|_| 0i32)
                .flat_map(i32::to_ne_bytes)
                .collect(),
            bytes(&self.positions, f32::to_ne_bytes),
            bytes(&self.orientations, f32::to_ne_bytes),
            bytes(&self.scales, f32::to_ne_bytes),
        ];
        let mut json = serde_json::json!({
            "name": self.name,
            "prototypes": [prototype],
            "protoIndices": 0,
            "positions": 1,
            "orientations": 2,
            "scales": 3,
            "custom": { "scatter": true, "names": self.names },
        });
        if !self.invisible.is_empty() {
            buffers.push(
                self.invisible
                    .iter()
                    .flat_map(|i| i.to_ne_bytes())
                    .collect(),
            );
            json["invisibleIds"] = 4.into();
        }
        (json.to_string(), buffers)
    }
}

/// Whether `item` can be one point of its source's scatter.
fn fits(
    scene: &Scene,
    item: &Item,
    children: &BTreeMap<u64, Vec<&Item>>,
    notes: &mut Notes<'_>,
) -> Option<u64> {
    let &source = notes.instances.get(&item.id)?;
    let src = scene.item(source)?;
    let t = &item.transform;
    let plain = source != item.id
        && item.instance_of.is_some()
        && children.get(&item.id).is_none_or(Vec::is_empty)
        && item.subdivision == src.subdivision
        && t.scale.min_element() > 0.0
        && t.translation.is_finite()
        && t.rotation.is_finite()
        && notes.materials.per_face(scene, item) == notes.materials.per_face(scene, src);
    plain.then_some(source)
}

/// Splits `siblings` into scatters written as PointInstancers and the
/// rest: returns the ids now in an instancer, and the instancers.
pub(crate) fn scatters(
    scene: &Scene,
    siblings: &[&Item],
    children: &BTreeMap<u64, Vec<&Item>>,
    plan: &mut Plan,
    notes: &mut Notes<'_>,
) -> (BTreeSet<u64>, Vec<InstancerPrim>) {
    let min = threshold().max(1);
    let mut groups: BTreeMap<u64, Vec<&Item>> = BTreeMap::new();
    for &item in siblings {
        if let Some(source) = fits(scene, item, children, notes) {
            groups.entry(source).or_default().push(item);
        }
    }
    let mut taken = BTreeSet::new();
    let mut out = Vec::new();
    for (source, items) in groups {
        // G8 RP-4: a Replicator's copies always go out as an instancer.
        let replicas = items
            .iter()
            .all(|i| operandi_core::replicator::copy_replicator(i.id).is_some());
        if items.len() < min && !replicas {
            continue;
        }
        let Some(k) = crate::plan::instance_prototype(scene, items[0], plan, notes) else {
            continue;
        };
        let polygons: usize = plan.prototypes[k]
            .meshes
            .iter()
            .map(|m| m.face_counts.len())
            .sum();
        let mut prim = InstancerPrim {
            name: format!(
                "{}_instances",
                scene.item(source).map_or("source", |s| s.name.as_str())
            ),
            prototype: k,
            names: Vec::new(),
            positions: Vec::new(),
            orientations: Vec::new(),
            scales: Vec::new(),
            invisible: Vec::new(),
        };
        for (i, item) in items.iter().enumerate() {
            let t = &item.transform;
            let turn = operandi_core::scene::Transform {
                translation: glam::DVec3::ZERO,
                scale: glam::DVec3::ONE,
                ..t.clone()
            }
            .matrix();
            let q = DQuat::from_mat3(&DMat3::from_mat4(turn)).normalize();
            prim.names.push(item.name.clone());
            prim.positions.push(t.translation.as_vec3().to_array());
            prim.orientations.push(q.as_quat().to_array());
            prim.scales.push(t.scale.as_vec3().to_array());
            if !item.visible {
                prim.invisible.push(i as i64);
            }
            taken.insert(item.id);
        }
        plan.items += items.len();
        plan.polygons += polygons * items.len();
        notes.scattered += items.len();
        out.push(prim);
    }
    (taken, out)
}
