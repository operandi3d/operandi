//! UsdSkel import (BEHAVIOR-RIG-PHASE5 §7): each UsdSkelSkeleton becomes
//! joints (their setup pose = the skeleton's bind transforms), its
//! SkelAnimation becomes keys on the joints' transform channels, and each
//! bound mesh gets one weight map per joint (named after it) and a skin
//! piece; `dualQuaternion` skinning sets Dual Quaternion. The bound mesh's
//! own placement stands for geomBindTransform (provisional, SK-Q11).
//! Operandi's own code (not adapted from Blender).
use crate::stage::{SkelData, StageData};
use glam::{DMat4, DQuat, DVec3};
use operandi_core::animation::{self, AnimChannel, ChannelKey, TRANSFORM_CHANNELS};
use operandi_core::locator::LocatorKind;
use operandi_core::skin::{self, SkinValues};
use operandi_core::{Result, Scene, Transform};
use std::collections::BTreeMap;

fn m(a: &[f64; 16]) -> DMat4 {
    DMat4::from_cols_array(a)
}

/// File space to scene space: units on the translation, as the importer's
/// own `scaled` does; the axis turn applies on top-level worlds only.
#[derive(Clone, Copy)]
pub(crate) struct Conv {
    pub turn: DMat4,
    pub scale: f64,
}

impl Conv {
    fn scaled(self, mut m: DMat4) -> DMat4 {
        m.w_axis.x *= self.scale;
        m.w_axis.y *= self.scale;
        m.w_axis.z *= self.scale;
        m
    }
    fn world(self, m: DMat4) -> DMat4 {
        self.turn * self.scaled(m)
    }
}

/// Sets `id`'s local transform to `local`.
fn place(scene: &mut Scene, id: u64, local: DMat4) {
    let (s, q, t) = local.to_scale_rotation_translation();
    if let Some(item) = scene.item_mut(id) {
        let rotation = item.transform.angles_for(q);
        item.transform = Transform {
            translation: t,
            rotation,
            scale: s,
            ..item.transform.clone()
        };
    }
}

/// The skeletons' joints (by skeleton path, in joint order) after adding
/// them to `scene`. `conv` takes file space to scene space (axes and
/// units).
fn add_joints(
    scene: &mut Scene,
    stage: &StageData,
    conv: Conv,
) -> Result<BTreeMap<String, Vec<u64>>> {
    let mut out = BTreeMap::new();
    for s in &stage.skeletons {
        let world = m(&s.world);
        let mut ids: Vec<u64> = Vec::with_capacity(s.joints.len());
        let mut worlds: Vec<DMat4> = Vec::with_capacity(s.joints.len());
        for (k, path) in s.joints.iter().enumerate() {
            let name = path.rsplit('/').next().unwrap_or(path).to_owned();
            let parent = path
                .rsplit_once('/')
                .and_then(|(p, _)| s.joints.iter().position(|j| j == p))
                .filter(|&p| p < k);
            let id = scene.add_group_locator_named(Some(name))?;
            let bind = s.bind.get(k).map(m).unwrap_or(DMat4::IDENTITY);
            let w = conv.world(world * bind);
            let local = match parent {
                Some(p) => worlds[p].inverse() * w,
                None => w,
            };
            {
                let item = scene.item_mut(id).expect("made");
                item.locator = Some(LocatorKind::Joint);
                item.parent = parent.map(|p| ids[p]);
            }
            place(scene, id, local);
            ids.push(id);
            worlds.push(w);
        }
        for &j in &ids {
            skin::ensure_setup_channels(scene, j);
        }
        add_keys(scene, s, &ids, conv, world)?;
        out.insert(s.path.clone(), ids);
    }
    Ok(out)
}

/// The SkelAnimation as keys on each joint's changing channels.
fn add_keys(scene: &mut Scene, s: &SkelData, ids: &[u64], conv: Conv, world: DMat4) -> Result<()> {
    let Some(a) = &s.anim else {
        return Ok(());
    };
    let rate = scene.time.rate;
    let slot: Vec<Option<usize>> = s
        .joints
        .iter()
        .map(|j| a.joints.iter().position(|x| x == j))
        .collect();
    let mut first = f64::INFINITY;
    let mut last = f64::NEG_INFINITY;
    for (k, &id) in ids.iter().enumerate() {
        let Some(i) = slot[k] else {
            continue;
        };
        let parent_is_joint = s.joints[k].contains('/');
        let mut values: Vec<(f64, [f64; 9])> = Vec::new();
        for (n, &time) in a.times.iter().enumerate() {
            let (Some(t), Some(r), Some(sc)) = (
                a.translations.get(n).and_then(|v| v.get(i)),
                a.rotations.get(n).and_then(|v| v.get(i)),
                a.scales.get(n).and_then(|v| v.get(i)),
            ) else {
                continue;
            };
            let local = DMat4::from_scale_rotation_translation(
                DVec3::from_array(*sc),
                DQuat::from_xyzw(r[1], r[2], r[3], r[0]).normalize(),
                DVec3::from_array(*t),
            );
            // Roots sit in the skeleton's space; children in their parent's.
            let scene_local = if parent_is_joint {
                conv.scaled(local)
            } else {
                conv.world(world * local)
            };
            let (scl, q, tr) = scene_local.to_scale_rotation_translation();
            let angles = scene
                .item(id)
                .map(|i| i.transform.angles_for(q))
                .unwrap_or_default();
            values.push((
                time,
                [
                    tr.x, tr.y, tr.z, angles.x, angles.y, angles.z, scl.x, scl.y, scl.z,
                ],
            ));
            first = first.min(time);
            last = last.max(time);
        }
        if values.len() < 2 {
            continue;
        }
        for (c, name) in TRANSFORM_CHANNELS.iter().enumerate() {
            let v0 = values[0].1[c];
            if values.iter().all(|(_, v)| (v[c] - v0).abs() < 1e-9) {
                continue;
            }
            let key = ChannelKey::new(id, name);
            let rest = scene.animation.channels.get(&key).map_or(v0, |ch| ch.rest);
            let mut curve = operandi_core::anim_curve::Curve::default();
            for (time, v) in &values {
                curve
                    .keys
                    .push(animation::new_key(animation::at_frames(rate, *time), v[c]));
            }
            scene
                .animation
                .channels
                .insert(key, AnimChannel { curve, rest });
        }
    }
    if first.is_finite() && last > first {
        scene.time.start = animation::at_frames(rate, first);
        scene.time.end = animation::at_frames(rate, last);
    }
    let now = scene.time.current;
    animation::pose(scene, now);
    Ok(())
}

/// Adds the skeletons and binds the skinned meshes (`skinned`: item and
/// mesh data index). Returns report lines.
pub(crate) fn add(
    scene: &mut Scene,
    stage: &StageData,
    conv: Conv,
    skinned: &[(u64, usize)],
) -> Result<Vec<String>> {
    if stage.skeletons.is_empty() {
        return Ok(Vec::new());
    }
    let joints = add_joints(scene, stage, conv)?;
    let mut bound = 0;
    let mut notes = Vec::new();
    for &(item, mi) in skinned {
        let Some(data) = stage.meshes.get(mi) else {
            continue;
        };
        let Some(sk) = &data.skin else {
            continue;
        };
        let (Some(ids), Some(skel)) = (
            joints.get(&sk.skeleton),
            stage.skeletons.iter().find(|s| s.path == sk.skeleton),
        ) else {
            notes.push(format!(
                "A skinned mesh names a skeleton that wasn't read ({})",
                sk.skeleton
            ));
            continue;
        };
        let indices = stage.i32s(&sk.indices)?;
        let weights = stage.f32s(&sk.weights)?;
        let size = sk.element_size.max(1);
        // The mesh's joint order, mapped to the skeleton's.
        let order: Vec<Option<usize>> = if sk.joints.is_empty() {
            (0..skel.joints.len()).map(Some).collect()
        } else {
            sk.joints
                .iter()
                .map(|j| skel.joints.iter().position(|x| x == j))
                .collect()
        };
        let names: Vec<String> = ids
            .iter()
            .map(|j| scene.item(*j).map(|i| i.name.clone()).unwrap_or_default())
            .collect();
        let mut maps: Vec<BTreeMap<u32, f64>> = vec![BTreeMap::new(); ids.len()];
        let n_points = scene.item(item).map_or(0, |i| i.mesh.vertices.len());
        for p in 0..n_points.min(indices.len() / size) {
            for e in 0..size {
                let (j, w) = (indices[p * size + e], weights[p * size + e]);
                if w <= 0. || j < 0 {
                    continue;
                }
                if let Some(Some(k)) = order.get(j as usize) {
                    *maps[*k].entry(p as u32).or_insert(0.) += f64::from(w);
                }
            }
        }
        let mut values = SkinValues {
            interpolation: if sk.method == "dualQuaternion" {
                skin::DUAL_QUATERNION
            } else {
                skin::LOCAL
            },
            ..Default::default()
        };
        {
            let it = scene.item_mut(item).expect("built");
            let mut mesh = it.mesh.open().clone();
            for (k, map) in maps.into_iter().enumerate() {
                mesh.attributes.point_weights.insert(names[k].clone(), map);
                values.add(ids[k], &names[k]);
            }
            it.mesh = mesh.into();
        }
        operandi_core::skin_bind::insert_skin(scene, item, &values)?;
        bound += 1;
    }
    let count: usize = joints.values().map(Vec::len).sum();
    notes.push(format!(
        "Skeletons: {} ({count} joints), {bound} skinned mesh{}",
        stage.skeletons.len(),
        if bound == 1 { "" } else { "es" }
    ));
    Ok(notes)
}
