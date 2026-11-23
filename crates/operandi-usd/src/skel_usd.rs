//! UsdSkel export of skins (BEHAVIOR-RIG-PHASE5 §7). A skinned mesh is
//! written in its rest shape (its Base Mesh) with UsdSkelBindingAPI: the
//! joint indices and weights its skin uses (effective weights, per point,
//! `elementSize` = Max Sources with Limit Sources, else the real most), the
//! mesh's setup-pose world as geomBindTransform and the skinning method
//! (Dual Quaternion → `dualQuaternion`). Each skeleton (a root joint and
//! its joints) becomes a UsdSkelSkeleton under `/root` with bind
//! transforms (setup-pose worlds) and rest transforms (setup-pose locals),
//! and a SkelAnimation with the joints' local translations, rotations and
//! scales at every sample time (or the current pose once). `/root` becomes
//! the SkelRoot. Joint items are not written as Xforms: the skeleton is
//! their place in USD. Operandi's own code (not adapted from Blender).
// Joint-indexed loops read several parallel lists at once.
#![allow(clippy::needless_range_loop)]
use glam::{DMat4, DVec3};
use operandi_core::Scene;
use operandi_core::skeleton;
use operandi_core::skin::{self, SkinValues};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// One skeleton.
#[derive(Clone, Debug, PartialEq)]
pub struct SkeletonPrim {
    /// The prim's name under `/root`.
    pub name: String,
    pub joint_ids: Vec<u64>,
    /// Joint paths (`Root`, `Root/Arm`, ...), parents first.
    pub joints: Vec<String>,
    /// Row-major, USD's row-vector convention.
    pub bind: Vec<[f64; 16]>,
    pub rest: Vec<[f64; 16]>,
    pub anim: Option<AnimPrim>,
}

/// A SkelAnimation: per time, per joint.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimPrim {
    pub times: Vec<f64>,
    pub translations: Vec<Vec<[f64; 3]>>,
    /// w, x, y, z.
    pub rotations: Vec<Vec<[f64; 4]>>,
    pub scales: Vec<Vec<[f64; 3]>>,
}

/// A mesh prim's binding to skeleton `skeleton` (index into the plan's).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkinBinding {
    pub skeleton: usize,
    pub element_size: usize,
    pub indices: Vec<i32>,
    pub weights: Vec<f32>,
    pub geom_bind: [f64; 16],
    pub dual_quaternion: bool,
}

/// What a scene's skins export as: the skeletons, each skinned item's
/// rest mesh and per-point binding (in its Base Mesh's point order), and
/// the joint items the skeletons stand for.
#[derive(Default)]
pub struct Skins {
    pub skeletons: Vec<SkeletonPrim>,
    pub items: BTreeMap<u64, (operandi_core::MeshSlot, SkinBinding)>,
    pub joints: BTreeSet<u64>,
}

fn matrix(m: DMat4) -> [f64; 16] {
    m.to_cols_array().map(operandi_core::rig::clean)
}

/// A joint path element USD accepts.
fn token(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit()) {
        s.insert(0, '_');
    }
    s
}

fn root_joint(scene: &Scene, mut j: u64) -> u64 {
    while let Some(p) = scene.item(j).and_then(|i| i.parent) {
        if !scene.item(p).is_some_and(|i| i.is_joint()) {
            break;
        }
        j = p;
    }
    j
}

/// The joints of `root`'s chain, parents first, with their paths.
fn chain(scene: &Scene, root: u64) -> Vec<(u64, String, Option<usize>)> {
    let mut out: Vec<(u64, String, Option<usize>)> = Vec::new();
    let name = |j: u64| token(&scene.item(j).map(|i| i.name.clone()).unwrap_or_default());
    out.push((root, name(root), None));
    let mut k = 0;
    while k < out.len() {
        let (j, path) = (out[k].0, out[k].1.clone());
        for c in skeleton::children(scene, j) {
            out.push((c, format!("{path}/{}", name(c)), Some(k)));
        }
        k += 1;
    }
    out
}

/// Item `id`'s local matrix relative to joint `parent` (its world when
/// `None`), in scene `s` (posed) or the setup pose.
fn local(world: &dyn Fn(u64) -> DMat4, id: u64, parent: Option<u64>) -> DMat4 {
    match parent {
        Some(p) => world(p).inverse() * world(id),
        None => world(id),
    }
}

/// Works out the skins of `scene` (an evaluated copy, or a document
/// scene). `frames` are the animation's posed scenes, if any.
pub(crate) fn skins(scene: &Scene, frames: Option<&crate::anim_usd::Frames>) -> Skins {
    let mut out = Skins::default();
    let mut by_root: BTreeMap<u64, usize> = BTreeMap::new();
    for item in &scene.items {
        let Some(stack) = item.mesh_stack.as_ref() else {
            continue;
        };
        if !skin::only_skin(stack) {
            continue;
        }
        let Ok((_, v)) = operandi_core::skin_commands::skin_op(scene, item.id) else {
            continue;
        };
        let Some(first) = v
            .joints
            .iter()
            .find(|j| scene.item(**j).is_some_and(|i| i.is_joint()))
        else {
            continue;
        };
        let root = root_joint(scene, *first);
        let k = *by_root.entry(root).or_insert_with(|| {
            out.skeletons.push(skeleton_prim(scene, root, frames));
            out.skeletons.len() - 1
        });
        let base = scene
            .animation
            .rig
            .skin_bases
            .get(&item.id)
            .cloned()
            .unwrap_or_else(|| item.mesh.clone());
        let binding = binding(
            &base,
            &v,
            &out.skeletons[k],
            k,
            skin::setup_world(scene, item.id).unwrap_or(DMat4::IDENTITY),
        );
        out.joints
            .extend(out.skeletons[k].joint_ids.iter().copied());
        out.items.insert(item.id, (base, binding));
    }
    out
}

fn skeleton_prim(
    scene: &Scene,
    root: u64,
    frames: Option<&crate::anim_usd::Frames>,
) -> SkeletonPrim {
    let chain = chain(scene, root);
    let ids: Vec<u64> = chain.iter().map(|c| c.0).collect();
    let parent_of = |k: usize| chain[k].2.map(|p| ids[p]);
    let setup = |j: u64| skin::setup_world(scene, j).unwrap_or(DMat4::IDENTITY);
    let bind = ids.iter().map(|&j| matrix(setup(j))).collect();
    let rest = (0..ids.len())
        .map(|k| matrix(local(&setup, ids[k], parent_of(k))))
        .collect();
    let pose = |s: &Scene| {
        let world = |j: u64| s.world_matrix(j).unwrap_or(DMat4::IDENTITY);
        let mut t = Vec::new();
        let mut r = Vec::new();
        let mut sc = Vec::new();
        for k in 0..ids.len() {
            let (scale, q, tr) =
                local(&world, ids[k], parent_of(k)).to_scale_rotation_translation();
            t.push(tr.to_array());
            r.push([q.w, q.x, q.y, q.z]);
            sc.push(scale.to_array());
        }
        (t, r, sc)
    };
    let mut anim = AnimPrim {
        times: Vec::new(),
        translations: Vec::new(),
        rotations: Vec::new(),
        scales: Vec::new(),
    };
    match frames {
        Some(f) => {
            for (time, s) in f.times.iter().zip(&f.posed) {
                let (t, r, sc) = pose(s);
                anim.times.push(*time);
                anim.translations.push(t);
                anim.rotations.push(r);
                anim.scales.push(sc);
            }
        }
        None => {
            // The pose shown now, once.
            let (t, r, sc) = pose(scene);
            anim.times.push(operandi_core::animation::frames(
                scene.time.rate,
                scene.time.current,
            ));
            anim.translations.push(t);
            anim.rotations.push(r);
            anim.scales.push(sc);
        }
    }
    SkeletonPrim {
        name: format!(
            "{}_Skeleton",
            token(&scene.item(root).map(|i| i.name.clone()).unwrap_or_default())
        ),
        joints: chain.iter().map(|c| c.1.clone()).collect(),
        joint_ids: ids,
        bind,
        rest,
        anim: Some(anim),
    }
}

fn binding(
    base: &operandi_core::Mesh,
    v: &SkinValues,
    skel: &SkeletonPrim,
    k: usize,
    mesh_world: DMat4,
) -> SkinBinding {
    let live = vec![true; v.joints.len()];
    let weights = skin::influences(base, v, &live);
    let index_of: Vec<i32> = v
        .joints
        .iter()
        .map(|j| {
            skel.joint_ids
                .iter()
                .position(|x| x == j)
                .map_or(-1, |i| i as i32)
        })
        .collect();
    let most = weights.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let size = if v.limit_sources {
        (v.max_sources as usize).max(1)
    } else {
        most
    };
    let mut indices = Vec::with_capacity(weights.len() * size);
    let mut ws = Vec::with_capacity(weights.len() * size);
    for slot in &weights {
        let mut slot: Vec<(i32, f64)> = slot
            .iter()
            .map(|&(i, w)| (index_of[i as usize], w))
            .filter(|(i, _)| *i >= 0)
            .collect();
        slot.sort_by(|a, b| b.1.total_cmp(&a.1));
        slot.truncate(size);
        for n in 0..size {
            let (i, w) = slot.get(n).copied().unwrap_or((0, 0.));
            indices.push(i);
            ws.push(w as f32);
        }
    }
    SkinBinding {
        skeleton: k,
        element_size: size,
        indices,
        weights: ws,
        geom_bind: matrix(mesh_world),
        dual_quaternion: v.interpolation == skin::DUAL_QUATERNION,
    }
}

/// A mesh prim's binding for its points, which come from `base` (matched
/// by position when the prim renumbered them).
pub(crate) fn prim_binding(
    base: &operandi_core::Mesh,
    all: &SkinBinding,
    points: &[[f32; 3]],
) -> SkinBinding {
    let size = all.element_size;
    let same = points.len() == base.vertices.len()
        && points
            .iter()
            .zip(&base.vertices)
            .all(|(p, v)| *p == v.as_vec3().to_array());
    let map: Vec<usize> = if same {
        (0..points.len()).collect()
    } else {
        let mut at: BTreeMap<[u32; 3], usize> = BTreeMap::new();
        for (i, v) in base.vertices.iter().enumerate() {
            at.entry(v.as_vec3().to_array().map(f32::to_bits))
                .or_insert(i);
        }
        points
            .iter()
            .map(|p| at.get(&p.map(f32::to_bits)).copied().unwrap_or(usize::MAX))
            .collect()
    };
    let mut out = SkinBinding {
        indices: Vec::with_capacity(points.len() * size),
        weights: Vec::with_capacity(points.len() * size),
        ..all.clone()
    };
    for &i in &map {
        for n in 0..size {
            let (j, w) = if i == usize::MAX {
                (0, 0.)
            } else {
                (all.indices[i * size + n], all.weights[i * size + n])
            };
            out.indices.push(j);
            out.weights.push(w);
        }
    }
    out
}

/// The `ou_define_skel` description: `root` is the SkelRoot's path,
/// `bindings` each bound mesh prim's path and binding.
pub fn json(root: &str, skeletons: &[SkeletonPrim], bindings: &[(String, SkinBinding)]) -> String {
    let path = |s: &SkeletonPrim| format!("{root}/{}", s.name);
    let skel: Vec<Value> = skeletons
        .iter()
        .map(|s| {
            json!({
                "path": path(s),
                "joints": s.joints,
                "bind": s.bind,
                "rest": s.rest,
                "anim": s.anim.as_ref().map(|a| json!({
                    "path": format!("{}/Animation", path(s)),
                    "times": a.times,
                    "translations": a.translations,
                    "rotations": a.rotations,
                    "scales": a.scales,
                })),
            })
        })
        .collect();
    let binds: Vec<Value> = bindings
        .iter()
        .filter_map(|(mesh, b)| {
            Some(json!({
                "mesh": mesh,
                "skeleton": path(skeletons.get(b.skeleton)?),
                "elementSize": b.element_size,
                "indices": b.indices,
                "weights": b.weights,
                "geomBind": b.geom_bind,
                "method": if b.dual_quaternion { "dualQuaternion" } else { "classicLinear" },
            }))
        })
        .collect();
    json!({ "skelRoot": root, "skeletons": skel, "bindings": binds }).to_string()
}

/// The world position a skeleton's bind puts joint `k` at (tests).
pub fn bind_position(s: &SkeletonPrim, k: usize) -> DVec3 {
    DMat4::from_cols_array(&s.bind[k]).w_axis.truncate()
}
