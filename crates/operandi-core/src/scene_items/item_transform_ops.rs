//! Item transform Reset and Freeze (spec freeze-transforms-2026-11-14,
//! BEHAVIOR-ITEM-TRANSFORM-OPS sections 3, 4, 7 and 8).
//!
//! Reset puts part of each selected item's transform back to identity: the
//! item moves, its geometry and children's channels don't. Freeze bakes
//! that part into the mesh's points so nothing moves on screen: with the
//! item's local matrix `P = P' * B`, the points take `B` and the channels
//! become `P'`. Each direct child's channels become `B * C` so it keeps its
//! world placement. Both are one undo step for the whole selection, and
//! none when nothing would change.
use crate::camera_props::TransformReset;
use crate::*;
use glam::{DMat3, DMat4, DQuat, DVec3};

impl TransformReset {
    /// The script word (`transform.reset rotation`).
    pub fn script_word(self) -> &'static str {
        match self {
            TransformReset::Position => "translation",
            TransformReset::Rotation => "rotation",
            TransformReset::Scale => "scale",
            TransformReset::All => "all",
        }
    }
    /// A script word, case-insensitive; `position` and `pos` are taken for
    /// translation too. Shear is reserved for stage 2.
    pub fn parse(word: &str) -> Option<Self> {
        match word.trim().to_ascii_lowercase().as_str() {
            "translation" | "position" | "pos" => Some(TransformReset::Position),
            "rotation" | "rot" => Some(TransformReset::Rotation),
            "scale" | "scl" => Some(TransformReset::Scale),
            "all" => Some(TransformReset::All),
            _ => None,
        }
    }
    /// The undo step's name (section 7: "Freeze Rotation").
    pub fn step_label(self, action: &str) -> String {
        match self {
            TransformReset::All => format!("{action} Transform"),
            part => format!("{action} {}", part.label()),
        }
    }
}

/// Freeze's split of transform `t`: the channels left (`P'`) and the matrix
/// the points take (`B`), with `t.matrix() == P' * B`.
pub fn freeze_split(t: &Transform, part: TransformReset) -> (Transform, DMat4) {
    let kept = part.apply(t);
    // P' = T R S with one factor dropped; B = P'^-1 P, written out per part
    // so it stays exact instead of going through a 4x4 inverse.
    let rotation = t.rotation_quat();
    let b = match part {
        // B = (R S)^-1 T R S: a move by the position in the item's own axes.
        TransformReset::Position => {
            DMat4::from_translation((rotation.inverse() * t.translation) / t.scale)
        }
        // B = S^-1 R S: exact for the points even under non-uniform scale.
        TransformReset::Rotation => {
            DMat4::from_scale(t.scale.recip())
                * DMat4::from_quat(rotation)
                * DMat4::from_scale(t.scale)
        }
        TransformReset::Scale => DMat4::from_scale(t.scale),
        TransformReset::All => t.matrix(),
    };
    (kept, b)
}

/// `m` as channels in `order`, if it has no shear (otherwise `None`).
/// Negative scale goes on the axis `like` had negative, else on X.
pub fn decompose(m: DMat4, like: &Transform) -> Option<Transform> {
    let linear = DMat3::from_mat4(m);
    let mut scale = DVec3::new(
        linear.x_axis.length(),
        linear.y_axis.length(),
        linear.z_axis.length(),
    );
    if scale.min_element() <= 1e-300 || !m.is_finite() {
        return None;
    }
    if linear.determinant() < 0. {
        let axis = (0..3).find(|&i| like.scale[i] < 0.).unwrap_or(0);
        scale[axis] = -scale[axis];
    }
    let rot = DMat3::from_cols(
        linear.x_axis / scale.x,
        linear.y_axis / scale.y,
        linear.z_axis / scale.z,
    );
    // Shear shows as axes that aren't at right angles.
    let tol = 1e-9;
    if rot.x_axis.dot(rot.y_axis).abs() > tol
        || rot.y_axis.dot(rot.z_axis).abs() > tol
        || rot.z_axis.dot(rot.x_axis).abs() > tol
    {
        return None;
    }
    let mut out = like.clone();
    out.translation = m.w_axis.truncate();
    out.scale = scale;
    out.rotation = like.angles_for(DQuat::from_mat3(&rot).normalize());
    // Keep the old angles when the turn is the same (no 180-degree
    // relabelling of an unchanged rotation).
    if like.rotation_quat().dot(out.rotation_quat()).abs() > 1. - 1e-15 {
        out.rotation = like.rotation;
    }
    Some(out)
}

/// `mesh` with every point taken through `b` (item local space): normals
/// by the inverse transpose, relative morph offsets by the linear part,
/// and polygons reversed when `b` mirrors (rule 4.2.4). UVs, weights,
/// colours, sets and materials are left alone (rule 4.2.5).
pub fn bake_mesh(mesh: &Mesh, b: DMat4) -> Mesh {
    let mut out = crate::duplicate::transformed_copy(mesh, b, false);
    let linear = DMat3::from_mat4(b);
    let normals = linear.inverse().transpose();
    for map in out.attributes.typed_maps.values_mut() {
        let apply: Box<dyn Fn(DVec3) -> DVec3> = match map.kind.as_str() {
            "normal3" => Box::new(move |n| (normals * n).normalize_or_zero()),
            "relative_morph3" => Box::new(move |d| linear * d),
            "absolute_position3" => Box::new(move |p| b.transform_point3(p)),
            _ => continue,
        };
        let fix = |v: &mut Vec<f64>| {
            if v.len() == 3 {
                let r = apply(DVec3::new(v[0], v[1], v[2]));
                *v = vec![r.x, r.y, r.z];
            }
        };
        for v in map.points.values_mut() {
            fix(v);
        }
        for v in map.corners.iter_mut().flatten().flatten() {
            fix(v);
        }
    }
    // A stored Solid Sketch is in the old local space; picking it up again
    // would put its nodes back where they were (FRZT-Q04, provisional).
    out.attributes.solid_sketch = None;
    out
}

/// How deep item `id` is in the hierarchy (0 at the top).
fn depth(scene: &Scene, id: u64) -> usize {
    let mut d = 0;
    let mut next = scene.item(id).and_then(|i| i.parent);
    while let Some(p) = next {
        d += 1;
        if d > 256 {
            break;
        }
        next = scene.item(p).and_then(|i| i.parent);
    }
    d
}

/// "1 locked item skipped" / "3 locked items skipped".
fn locked_note(n: usize) -> String {
    format!("{n} locked item{} skipped", if n == 1 { "" } else { "s" })
}

impl Editor {
    /// Section 3 Reset on `items`: locked ones skipped with one note.
    pub(crate) fn reset_transform(&mut self, items: Vec<u64>, part: TransformReset) -> Result<()> {
        if items.is_empty() {
            return Err("Select an item".into());
        }
        if items.iter().any(|id| self.scene.item(*id).is_none()) {
            return Err("Missing item".into());
        }
        let (locked, open): (Vec<u64>, Vec<u64>) = items
            .iter()
            .partition(|id| self.scene.item(**id).is_some_and(|i| i.is_locked()));
        if open.is_empty() {
            return Err(if locked.len() == 1 {
                self.scene.refuse_locked(&locked).err().unwrap_or_default()
            } else {
                "The selected items are locked (Assembly > Lock)".into()
            });
        }
        self.transform_note = (!locked.is_empty()).then(|| locked_note(locked.len()));
        let before = self.scene.clone();
        for id in &open {
            if let Some(item) = self.scene.item_mut(*id) {
                item.transform = part.apply(&item.transform);
            }
        }
        // Rule 4.2.11 / section 7: no change, no undo step.
        if self.scene == before {
            return Ok(());
        }
        self.record(part.step_label("Reset"), before);
        Ok(())
    }

    /// Section 4 Freeze on `items`: parents first, every child kept in
    /// place. Items with nothing to bake, locked items and items whose
    /// children can't keep their place are skipped; the reasons go to
    /// `transform_note`.
    pub(crate) fn freeze_transform(&mut self, items: Vec<u64>, part: TransformReset) -> Result<()> {
        if self.session.live.is_some() {
            return Err("Drop the active tool before Freeze".into());
        }
        if items.is_empty() {
            return Err("Select an item".into());
        }
        if items.iter().any(|id| self.scene.item(*id).is_none()) {
            return Err("Missing item".into());
        }
        // Section 7: items that can't take Freeze are skipped silently
        // when mixed with mesh items.
        let mut meshes: Vec<u64> = items
            .iter()
            .copied()
            .filter(|id| self.scene.item(*id).is_some_and(|i| i.is_mesh()))
            .collect();
        if meshes.is_empty() {
            return Err("Freeze bakes a mesh item's transform; select a mesh item".into());
        }
        meshes.dedup();
        let locked = meshes
            .iter()
            .filter(|id| self.scene.item(**id).is_some_and(|i| i.is_locked()))
            .count();
        meshes.retain(|id| self.scene.item(*id).is_some_and(|i| !i.is_locked()));
        // Rule 4.2.3: parents first, so a selected child freezes from its
        // compensated channels.
        meshes.sort_by_key(|id| depth(&self.scene, *id));
        let before = self.scene.clone();
        let mut notes = Vec::new();
        if locked > 0 {
            notes.push(locked_note(locked));
        }
        for id in meshes {
            if let Err(note) = self.freeze_one(id, part) {
                notes.push(note);
            }
        }
        let note = (!notes.is_empty()).then(|| notes.join("; "));
        if self.scene == before {
            self.transform_note = None;
            // Nothing could be done and something was refused: say why.
            return note.map_or(Ok(()), Err);
        }
        if let Err(e) = self.scene.validate() {
            self.scene = before;
            return Err(e);
        }
        self.transform_note = note;
        self.record(part.step_label("Freeze"), before);
        Ok(())
    }

    /// Freeze one mesh item; `Err` is the status note when it's skipped
    /// (the scene is then unchanged for it).
    fn freeze_one(&mut self, id: u64, part: TransformReset) -> std::result::Result<(), String> {
        let item = self.scene.item(id).ok_or("Missing item")?;
        let name = item.name.clone();
        let t = item.transform.clone();
        let (kept, b) = freeze_split(&t, part);
        if kept == t {
            return Ok(()); // already at identity
        }
        if t.scale.abs().min_element() < 1e-12 || !b.is_finite() {
            return Err(format!("{name} has a zero Scale, Freeze skipped"));
        }
        // Rule 4.2.2: each direct child's channels become B * C.
        let mut children = Vec::new();
        for child in self.scene.items.iter().filter(|c| c.parent == Some(id)) {
            let next = if DMat3::from_mat4(b) == DMat3::IDENTITY {
                // A pure move: only the child's position changes, exactly.
                let mut c = child.transform.clone();
                c.translation = b.transform_point3(c.translation);
                c
            } else {
                decompose(b * child.transform.matrix(), &child.transform).ok_or_else(|| {
                    format!(
                        "{name}: {} would shear under its non-uniform scale, Freeze {} skipped; try Freeze All on both",
                        child.name,
                        part.label()
                    )
                })?
            };
            children.push((child.id, next));
        }
        let baked = bake_mesh(&item.mesh, b);
        let item = self.scene.item_mut(id).ok_or("Missing item")?;
        item.mesh = baked.into();
        item.transform = kept;
        for (child, next) in children {
            if let Some(c) = self.scene.item_mut(child) {
                c.transform = next;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(pos: [f64; 3], rot_deg: [f64; 3], scale: [f64; 3]) -> Transform {
        Transform {
            translation: DVec3::from(pos),
            rotation: DVec3::from(rot_deg) * std::f64::consts::PI / 180.,
            scale: DVec3::from(scale),
            ..Default::default()
        }
    }

    #[test]
    fn split_recomposes_for_every_part() {
        let mut tr = t([1., -2., 3.], [30., 40., -50.], [2., 0.5, -3.]);
        tr.set_rotation_order(crate::scene::RotationOrder::Yzx);
        for part in TransformReset::ALL {
            let (kept, b) = freeze_split(&tr, part);
            let m = kept.matrix() * b;
            assert!(m.abs_diff_eq(tr.matrix(), 1e-12), "{part:?}");
            assert_eq!(kept.rotation_order(), tr.rotation_order());
        }
    }

    #[test]
    fn decompose_round_trips_and_refuses_shear() {
        let c = t([1., 2., 3.], [10., 20., 30.], [1., -2., 3.]);
        let back = decompose(c.matrix(), &c).unwrap();
        assert!(back.matrix().abs_diff_eq(c.matrix(), 1e-12));
        assert_eq!(back.scale.y.signum(), -1.);
        let shear =
            DMat4::from_scale(DVec3::new(1., 3., 1.)) * t([0.; 3], [0., 0., 45.], [1.; 3]).matrix();
        assert!(decompose(shear, &c).is_none());
    }

    #[test]
    fn script_words() {
        for part in TransformReset::ALL {
            assert_eq!(TransformReset::parse(part.script_word()), Some(part));
        }
        assert_eq!(TransformReset::parse("shear"), None);
        assert_eq!(
            TransformReset::Rotation.step_label("Freeze"),
            "Freeze Rotation"
        );
    }
}
