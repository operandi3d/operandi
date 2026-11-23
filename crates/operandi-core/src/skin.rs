//! The skin piece (BEHAVIOR-RIG-PHASE5 §4, SK-30..36): a MeshOp in the
//! mesh's stack that moves each point by its joints, blended by the
//! joints' weight maps (F7 `point_weights`, one per joint, named after it).
//!
//! The bind pose is the joints' setup pose (SK-23, SK-51): each joint's
//! move is its world now times the inverse of its setup-pose world, both
//! in the mesh item's space (the link's `to_local` and `rest`). In Setup
//! mode the joints show their setup pose, so the skin shows the rest
//! shape, and moving a joint there moves the bind pose with it.
//!
//! Every point's weights add up to 100% before blending (SK-30); Linear is
//! classic linear-blend skinning, Dual Quaternion blends rigid moves as
//! dual quaternions so a bent elbow keeps its volume (SK-33). The mesh's
//! own points are never changed (F5).
use crate::animation::{ChannelKey, TRANSFORM_CHANNELS};
use crate::meshop_types::{Field, FieldKind, OpContext, OpDef};
use crate::{Mesh, Result, Scene};
use glam::{DMat4, DQuat, DVec3};
use serde::{Deserialize, Serialize};

/// The op's saved type id (Operandi's own, provisional: SK-Q06).
pub const SKIN_OP: &str = "operandi.skin.item";

/// SK-31 Interpolation.
pub const LOCAL: u32 = 0;
pub const LINEAR: u32 = 1;
pub const DUAL_QUATERNION: u32 = 2;

/// The skin's values (SK-31, SK-35). `joints`, `maps`, `enabled` and
/// `smooth` are parallel lists, one entry per joint influence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SkinValues {
    pub interpolation: u32,
    pub limit_sources: bool,
    pub max_sources: u32,
    pub threshold: f64,
    pub sharpness: f64,
    pub world: bool,
    pub joints: Vec<u64>,
    pub maps: Vec<String>,
    /// Each influence's Enable.
    pub enabled: Vec<bool>,
    /// Each influence's Interpolation: false Linear, true Smooth-curved.
    pub smooth: Vec<bool>,
}

impl Default for SkinValues {
    fn default() -> Self {
        SkinValues {
            interpolation: LOCAL,
            limit_sources: false,
            max_sources: 3,
            threshold: 0.,
            sharpness: 0.,
            world: true,
            joints: Vec::new(),
            maps: Vec::new(),
            enabled: Vec::new(),
            smooth: Vec::new(),
        }
    }
}

impl SkinValues {
    fn check(&self) -> Result<()> {
        let n = self.joints.len();
        if self.maps.len() != n || self.enabled.len() != n || self.smooth.len() != n {
            return Err("The skin's joint lists don't match".into());
        }
        if self.interpolation > DUAL_QUATERNION {
            return Err("Interpolation is Local, Linear or Dual Quaternion".into());
        }
        if !(1..=8).contains(&self.max_sources) {
            return Err("Max Sources is 1 to 8".into());
        }
        if !self.threshold.is_finite() || !self.sharpness.is_finite() {
            return Err("Not a number".into());
        }
        Ok(())
    }
    /// Adds a joint influence reading map `map` (Rebind, SK-25).
    pub fn add(&mut self, joint: u64, map: &str) {
        if self.joints.contains(&joint) {
            return;
        }
        self.joints.push(joint);
        self.maps.push(map.to_owned());
        self.enabled.push(true);
        self.smooth.push(false);
    }
}

/// The table entry.
pub struct SkinDef {
    fields: Vec<Field>,
}

pub(crate) fn entries() -> Vec<std::sync::Arc<dyn OpDef>> {
    vec![std::sync::Arc::new(SkinDef {
        fields: vec![
            Field::new(
                "interpolation",
                "Interpolation",
                FieldKind::Choice(vec![
                    "Local".into(),
                    "Linear".into(),
                    "Dual Quaternion".into(),
                ]),
            ),
            Field::new("limitSources", "Limit Sources", FieldKind::Check),
            Field::new(
                "maxSources",
                "Max Sources",
                FieldKind::Count { min: 1, max: 8 },
            ),
            Field::new("threshold", "Threshold", FieldKind::Percent),
            Field::new("sharpness", "Sharpness", FieldKind::Number),
            Field::new("world", "World", FieldKind::Check),
            Field::new("joints", "Joints", FieldKind::Items),
            Field::new("maps", "Weight Maps", FieldKind::Hidden),
            Field::new("enabled", "Influence Enable", FieldKind::Hidden),
            Field::new("smooth", "Influence Smooth", FieldKind::Hidden),
        ],
    })]
}

fn parse(values: &serde_json::Value) -> Result<SkinValues> {
    let v: SkinValues = serde_json::from_value(values.clone()).map_err(|e| e.to_string())?;
    v.check()?;
    Ok(v)
}

impl OpDef for SkinDef {
    fn id(&self) -> &str {
        SKIN_OP
    }
    fn label(&self) -> &str {
        "Skin"
    }
    fn folder(&self) -> &str {
        "Deform"
    }
    fn takes_selection(&self) -> bool {
        false
    }
    fn fields(&self) -> &[Field] {
        &self.fields
    }
    fn defaults(&self) -> serde_json::Value {
        serde_json::to_value(SkinValues::default()).unwrap_or_default()
    }
    fn read(&self, version: u32, values: &serde_json::Value) -> Result<serde_json::Value> {
        if version != self.version() {
            return Err(format!(
                "Saved by a newer Operandi (version {version} of this operation)"
            ));
        }
        serde_json::to_value(parse(values)?).map_err(|e| e.to_string())
    }
    /// The joints, in influence order (a missing joint stays in the list
    /// so the links line up with the influences; it fails only itself).
    fn links(&self, values: &serde_json::Value, _scene: &Scene) -> Vec<u64> {
        parse(values).map(|v| v.joints).unwrap_or_default()
    }
    fn run(&self, ctx: &OpContext, values: &serde_json::Value) -> Result<Mesh> {
        let v = parse(values)?;
        // Each influence's move in the mesh's space; a missing joint or a
        // disabled influence doesn't move its points.
        let moves: Vec<Option<DMat4>> = v
            .joints
            .iter()
            .enumerate()
            .map(|(i, joint)| {
                if !v.enabled[i] {
                    return None;
                }
                let link = ctx.links.iter().find(|l| l.item == *joint)?;
                let s = link.source.as_ref().ok()?;
                (s.rest.determinant().abs() > 1e-18).then(|| s.to_local * s.rest.inverse())
            })
            .collect();
        Ok(deform(ctx.input, &v, &moves))
    }
}

/// Each point's influences: (influence index, weight) after Smooth,
/// Threshold, Sharpness and Limit Sources, normalised to add up to 1
/// (SK-30, SK-31). Points with no weight have none. `moves` names which
/// influences count (`None`: off or missing).
pub fn influences(mesh: &Mesh, v: &SkinValues, live: &[bool]) -> Vec<Vec<(u32, f64)>> {
    let n = mesh.vertices.len();
    let mut out: Vec<Vec<(u32, f64)>> = vec![Vec::new(); n];
    for (i, map) in v.maps.iter().enumerate() {
        if !live.get(i).copied().unwrap_or(false) {
            continue;
        }
        let Some(weights) = mesh.attributes.point_weights.get(map) else {
            continue;
        };
        for (&p, &w) in weights {
            let Some(slot) = out.get_mut(p as usize) else {
                continue;
            };
            let mut w = w.clamp(0., 1.);
            if v.smooth[i] {
                w = w * w * (3. - 2. * w);
            }
            if w > 0. {
                slot.push((i as u32, w));
            }
        }
    }
    let power = 2f64.powf(v.sharpness);
    for slot in &mut out {
        if slot.is_empty() {
            continue;
        }
        if v.threshold > 0. {
            slot.retain(|(_, w)| *w >= v.threshold);
        }
        if v.sharpness != 0. {
            let top = slot.iter().map(|s| s.1).fold(0., f64::max);
            if top > 0. {
                for s in slot.iter_mut() {
                    s.1 = (s.1 / top).powf(power) * top;
                }
            }
        }
        if v.limit_sources && slot.len() > v.max_sources as usize {
            slot.sort_by(|a, b| b.1.total_cmp(&a.1));
            slot.truncate(v.max_sources as usize);
        }
        let total: f64 = slot.iter().map(|s| s.1).sum();
        if total > 0. {
            for s in slot.iter_mut() {
                s.1 /= total;
            }
        } else {
            slot.clear();
        }
    }
    out
}

/// `mesh` with every weighted point moved by its influences' `moves`.
pub fn deform(mesh: &Mesh, v: &SkinValues, moves: &[Option<DMat4>]) -> Mesh {
    let live: Vec<bool> = moves.iter().map(Option::is_some).collect();
    let weights = influences(mesh, v, &live);
    let mut out = mesh.clone();
    let dual = v.interpolation == DUAL_QUATERNION;
    let duals: Vec<Option<Dual>> = if dual {
        moves.iter().map(|m| m.map(Dual::from_matrix)).collect()
    } else {
        Vec::new()
    };
    for (p, slot) in weights.iter().enumerate() {
        if slot.is_empty() {
            continue;
        }
        let rest = mesh.vertices[p];
        out.vertices[p] = if dual {
            blend_dual(rest, slot, &duals)
        } else {
            slot.iter()
                .map(|&(i, w)| moves[i as usize].expect("live").transform_point3(rest) * w)
                .sum()
        };
    }
    out
}

/// A rigid move as a unit dual quaternion (real `r`, dual `d`).
#[derive(Clone, Copy, Debug)]
struct Dual {
    r: DQuat,
    d: DQuat,
}

impl Dual {
    /// The rotation and translation of `m` (scale is dropped: dual
    /// quaternions blend rigid moves only).
    fn from_matrix(m: DMat4) -> Dual {
        let (_, r, t) = m.to_scale_rotation_translation();
        let r = r.normalize();
        let d = DQuat::from_xyzw(t.x, t.y, t.z, 0.) * r * 0.5;
        Dual { r, d }
    }
}

fn blend_dual(p: DVec3, slot: &[(u32, f64)], duals: &[Option<Dual>]) -> DVec3 {
    let first = duals[slot[0].0 as usize].expect("live").r;
    let mut r = DQuat::from_xyzw(0., 0., 0., 0.);
    let mut d = DQuat::from_xyzw(0., 0., 0., 0.);
    for &(i, w) in slot {
        let q = duals[i as usize].expect("live");
        // The shorter way round (antipodal quaternions are one rotation).
        let w = if q.r.dot(first) < 0. { -w } else { w };
        r += q.r * w;
        d += q.d * w;
    }
    let len = r.length();
    if len < 1e-12 {
        return p;
    }
    let (r, d) = (r / len, d / len);
    let t = (d * r.conjugate()) * 2.;
    r * p + DVec3::new(t.x, t.y, t.z)
}

/// Gives joint `id` a setup value on each transform channel that has
/// none (SK-23): an unkeyed channel then keeps its bind value as its
/// setup value while posing changes the channel. Returns whether it added
/// any.
pub fn ensure_setup_channels(scene: &mut Scene, id: u64) -> bool {
    let Some(t) = scene.item(id).map(|i| i.transform.clone()) else {
        return false;
    };
    let mut added = false;
    for (k, name) in TRANSFORM_CHANNELS.iter().enumerate() {
        let key = ChannelKey::new(id, name);
        if scene.animation.channels.contains_key(&key) {
            continue;
        }
        let axis = k % 3;
        let rest = match k / 3 {
            0 => t.translation[axis],
            1 => t.rotation[axis],
            _ => t.scale[axis],
        };
        scene.animation.channels.insert(
            key,
            crate::animation::AnimChannel {
                curve: Default::default(),
                rest,
            },
        );
        added = true;
    }
    added
}

/// Item `id`'s world matrix in the setup (rest) pose: every animated
/// transform channel at its setup value (ACS-API-31). Pivots apply;
/// targets and constraints don't (provisional, SK-Q07).
pub fn setup_world(scene: &Scene, id: u64) -> Result<DMat4> {
    let mut m = DMat4::IDENTITY;
    let mut next = Some(id);
    let mut chain = Vec::new();
    while let Some(i) = next {
        if chain.len() >= 256 || chain.contains(&i) {
            return Err("Hierarchy cycle or depth limit".into());
        }
        chain.push(i);
        next = scene.item(i).ok_or("Dangling parent reference")?.parent;
    }
    for &i in chain.iter().rev() {
        m *= setup_local(scene, i);
    }
    Ok(m)
}

/// Item `id`'s local matrix in the setup pose.
pub fn setup_local(scene: &Scene, id: u64) -> DMat4 {
    if let Some(m) = scene.animation.rig.setup_locals.get(&id) {
        return *m;
    }
    let Some(item) = scene.item(id) else {
        return DMat4::IDENTITY;
    };
    let mut t = item.transform.clone();
    let channels = &scene.animation.channels;
    if !channels.is_empty() {
        for (k, name) in TRANSFORM_CHANNELS.iter().enumerate() {
            if let Some(c) = channels.get(&ChannelKey::new(id, name)) {
                let axis = k % 3;
                match k / 3 {
                    0 => t.translation[axis] = c.rest,
                    1 => t.rotation[axis] = c.rest,
                    _ => t.scale[axis] = c.rest,
                }
            }
        }
    }
    let local = t.matrix();
    match scene.animation.rig.pivots.get(&id) {
        Some(p) => p.apply(local),
        None => local,
    }
}

/// Whether `a` and `b` differ only in their weight maps.
pub fn weights_only(a: &Mesh, b: &Mesh) -> bool {
    if a.vertices != b.vertices || a.faces != b.faces {
        return false;
    }
    let (mut a, mut b) = (a.clone(), b.clone());
    a.attributes.point_weights.clear();
    b.attributes.point_weights.clear();
    a == b
}

/// Whether every op of `stack` is a skin (it keeps every point).
pub fn only_skin(stack: &crate::mesh_stack::MeshStack) -> bool {
    !stack.ops.is_empty() && stack.ops.iter().all(|o| o.kind.type_id() == SKIN_OP)
}
