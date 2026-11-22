//! Drop Action (spec drop-action-2026-11-06): what happens when an item is
//! dragged onto another item in the viewport. Each action is a pure scene
//! operation on (sources, destination); `Command::DropAction` makes one
//! undo step of it, named after the action (DA-8).

use crate::Result;
use crate::scene::{Scene, Transform};
use glam::{DMat4, DQuat, DVec3};
use serde::{Deserialize, Serialize};

/// The top-bar Drop Action choices, in menu order (DA-2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DropAction {
    #[default]
    None,
    Parent,
    ParentInPlace,
    Match,
    MatchPosition,
    MatchRotation,
    MatchScale,
    Insert,
    InsertAtParent,
    InsertInPlace,
    Place,
    PlaceAndAlign,
}

impl DropAction {
    pub const ALL: [Self; 12] = [
        Self::None,
        Self::Parent,
        Self::ParentInPlace,
        Self::Match,
        Self::MatchPosition,
        Self::MatchRotation,
        Self::MatchScale,
        Self::Insert,
        Self::InsertAtParent,
        Self::InsertInPlace,
        Self::Place,
        Self::PlaceAndAlign,
    ];
    /// The menu entry and the undo step's name.
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "(none)",
            Self::Parent => "Parent",
            Self::ParentInPlace => "Parent in Place",
            Self::Match => "Match",
            Self::MatchPosition => "Match Position",
            Self::MatchRotation => "Match Rotation",
            Self::MatchScale => "Match Scale",
            Self::Insert => "Insert",
            Self::InsertAtParent => "Insert at Parent",
            Self::InsertInPlace => "Insert in Place",
            Self::Place => "Place",
            Self::PlaceAndAlign => "Place and Align",
        }
    }
    /// The MCP / preferences name, e.g. `match_position`.
    pub fn key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Parent => "parent",
            Self::ParentInPlace => "parent_in_place",
            Self::Match => "match",
            Self::MatchPosition => "match_position",
            Self::MatchRotation => "match_rotation",
            Self::MatchScale => "match_scale",
            Self::Insert => "insert",
            Self::InsertAtParent => "insert_at_parent",
            Self::InsertInPlace => "insert_in_place",
            Self::Place => "place",
            Self::PlaceAndAlign => "place_and_align",
        }
    }
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.key() == key)
    }
    /// The Insert family uses only the pressed item as source (DA-Q4).
    pub fn single_source(self) -> bool {
        matches!(
            self,
            Self::Insert | Self::InsertAtParent | Self::InsertInPlace
        )
    }
    /// Place and Place and Align need a ray against the destination's
    /// surface (DA-29).
    pub fn needs_ray(self) -> bool {
        matches!(self, Self::Place | Self::PlaceAndAlign)
    }
}

/// A world-space ray from the eye through the pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: DVec3,
    pub direction: DVec3,
}

/// The first hit of `ray` on item `id`'s polygons, in world space: the
/// point and the unit polygon normal facing the ray's origin. Polygons are
/// fanned into triangles; `None` when it misses or `id` has no mesh.
pub fn surface_hit(scene: &Scene, id: u64, ray: Ray) -> Option<(DVec3, DVec3)> {
    let item = scene.item(id)?;
    if !item.is_mesh() {
        return None;
    }
    let world = scene.world_matrix(id).ok()?;
    let dir = ray.direction.normalize_or_zero();
    if dir == DVec3::ZERO {
        return None;
    }
    let mesh = &item.mesh;
    let mut best: Option<(f64, DVec3, DVec3)> = None;
    for face in &mesh.faces {
        let points: Vec<DVec3> = face
            .vertices
            .iter()
            .filter_map(|&v| mesh.vertices.get(v as usize))
            .map(|&p| world.transform_point3(p))
            .collect();
        if points.len() < 3 {
            continue;
        }
        for k in 1..points.len() - 1 {
            let (a, b, c) = (points[0], points[k], points[k + 1]);
            let Some(t) = ray_triangle(ray.origin, dir, a, b, c) else {
                continue;
            };
            if best.is_some_and(|(bt, _, _)| bt <= t) {
                continue;
            }
            let mut n = (b - a).cross(c - a).normalize_or_zero();
            if n == DVec3::ZERO {
                continue;
            }
            if n.dot(dir) > 0. {
                n = -n;
            }
            best = Some((t, ray.origin + dir * t, n));
        }
    }
    best.map(|(_, p, n)| (p, n))
}

/// Möller–Trumbore; the distance along `dir` (unit) of a hit in front of
/// `origin`, either side of the triangle.
fn ray_triangle(origin: DVec3, dir: DVec3, a: DVec3, b: DVec3, c: DVec3) -> Option<f64> {
    let e1 = b - a;
    let e2 = c - a;
    let p = dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-14 {
        return None;
    }
    let inv = 1. / det;
    let s = origin - a;
    let u = s.dot(p) * inv;
    if !(-1e-12..=1. + 1e-12).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = dir.dot(q) * inv;
    if v < -1e-12 || u + v > 1. + 1e-12 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 1e-9).then_some(t)
}

fn exists(scene: &Scene, id: u64) -> Result<()> {
    if scene.item(id).is_some() {
        Ok(())
    } else {
        Err(format!("No item {id}"))
    }
}

/// Whether `id` is `ancestor` or below it.
fn is_within(scene: &Scene, id: u64, ancestor: u64) -> bool {
    let mut next = Some(id);
    let mut steps = 0;
    while let Some(at) = next {
        if at == ancestor {
            return true;
        }
        steps += 1;
        if steps > 4096 {
            return false;
        }
        next = scene.item(at).and_then(|i| i.parent);
    }
    false
}

fn parent_world(scene: &Scene, id: u64) -> Result<DMat4> {
    match scene.item(id).and_then(|i| i.parent) {
        Some(p) => scene.world_matrix(p),
        None => Ok(DMat4::IDENTITY),
    }
}

impl Scene {
    /// Give item `id` the world matrix `world` by solving its local
    /// transform under its current parent (rotation kept in the item's own
    /// rotation order). Refuses when that would need shear.
    pub fn set_world_matrix(&mut self, id: u64, world: DMat4) -> Result<()> {
        self.refuse_locked(&[id])?;
        let parent = parent_world(self, id)?;
        if parent.determinant().abs() < 1e-12 {
            return Err("The parent's transform can't be inverted".into());
        }
        let local = parent.inverse() * world;
        let index = self.item_index(id).ok_or_else(|| format!("No item {id}"))?;
        let old = self.items[index].transform.clone();
        let (scale, rotation, translation) = local.to_scale_rotation_translation();
        let transform = Transform {
            translation,
            rotation: old.angles_for(rotation),
            scale,
            ..old
        };
        let tolerance = 1e-9
            * local
                .abs()
                .to_cols_array()
                .iter()
                .fold(1., |a: f64, &b| a.max(b));
        if !transform.matrix().abs_diff_eq(local, tolerance) || !transform.matrix().is_finite() {
            return Err("This would need shear under the item's parent".into());
        }
        self.items[index].transform = transform;
        Ok(())
    }

    /// Apply `action` with `sources` dropped on `destination` (DA-11..31).
    /// Errors leave the scene unchanged. `ray` is the eye-to-pointer ray
    /// Place and Place and Align use; a miss is an error ("nothing
    /// happens", DA-29).
    pub fn drop_action(
        &mut self,
        action: DropAction,
        sources: &[u64],
        destination: u64,
        ray: Option<Ray>,
    ) -> Result<()> {
        exists(self, destination)?;
        let mut list: Vec<u64> = Vec::new();
        for &s in sources {
            exists(self, s)?;
            // DA-7: the destination is never a source.
            if s != destination && !list.contains(&s) {
                list.push(s);
            }
        }
        if action.single_source() {
            list.truncate(1);
        }
        if list.is_empty() {
            return Err("Drop onto another item".into());
        }
        if action == DropAction::None {
            return Err("Choose a Drop Action".into());
        }
        let before = self.clone();
        let result = (|| {
            if action.needs_ray() {
                let item = self.item(destination);
                if !item.is_some_and(|i| i.is_mesh()) {
                    return Err("Place needs a mesh to drop onto".to_owned());
                }
                let ray = ray.ok_or("Place needs a point on the surface")?;
                let (point, normal) = surface_hit(self, destination, ray)
                    .ok_or("Drop onto the destination's surface")?;
                for &s in &list {
                    self.place(
                        s,
                        point,
                        (action == DropAction::PlaceAndAlign).then_some(normal),
                    )?;
                }
                return Ok(());
            }
            for &s in &list {
                match action {
                    DropAction::Parent | DropAction::ParentInPlace => {
                        self.drop_parent(s, destination, action == DropAction::ParentInPlace)?
                    }
                    DropAction::Match
                    | DropAction::MatchPosition
                    | DropAction::MatchRotation
                    | DropAction::MatchScale => self.drop_match(action, s, destination)?,
                    DropAction::Insert | DropAction::InsertAtParent | DropAction::InsertInPlace => {
                        self.drop_insert(action, s, destination)?
                    }
                    DropAction::None | DropAction::Place | DropAction::PlaceAndAlign => {}
                }
            }
            Ok(())
        })();
        if result.is_err() {
            *self = before;
        }
        result
    }

    /// DA-12/13: `source` becomes `destination`'s last child, keeping its
    /// local fields (Parent) or its world (Parent in Place).
    fn drop_parent(&mut self, source: u64, destination: u64, in_place: bool) -> Result<()> {
        if is_within(self, destination, source) {
            return Err("An item can't be parented to itself or its own child".into());
        }
        self.set_parent(source, Some(destination), in_place)?;
        // Sibling order is the order in `items`: move it after the others.
        let index = self.item_index(source).unwrap_or(0);
        let item = self.items.remove(index);
        self.items.push(item);
        Ok(())
    }

    /// DA-14..17, DA-26..28: world-space match; the source's parent stays.
    fn drop_match(&mut self, action: DropAction, source: u64, destination: u64) -> Result<()> {
        let (s_scale, s_rot, s_pos) = self.world_matrix(source)?.to_scale_rotation_translation();
        let (d_scale, d_rot, d_pos) = self
            .world_matrix(destination)?
            .to_scale_rotation_translation();
        let (scale, rotation, position) = match action {
            DropAction::Match => (d_scale, d_rot, d_pos),
            DropAction::MatchPosition => (s_scale, s_rot, d_pos),
            DropAction::MatchRotation => (s_scale, d_rot, s_pos),
            _ => (d_scale, s_rot, s_pos),
        };
        self.set_world_matrix(
            source,
            DMat4::from_scale_rotation_translation(scale, rotation, position),
        )
    }

    /// DA-18..20, DA-23..25: splice `source` between `destination` and its
    /// parent. The source takes the destination's old slot in the sibling
    /// list (DA-Q5 pick); the destination keeps its world.
    fn drop_insert(&mut self, action: DropAction, source: u64, destination: u64) -> Result<()> {
        if is_within(self, destination, source) {
            return Err("An item can't be inserted above itself or its own child".into());
        }
        let d_world = self.world_matrix(destination)?;
        let s_world = self.world_matrix(source)?;
        let new_parent = self.item(destination).and_then(|i| i.parent);
        let (s_scale, _, _) = s_world.to_scale_rotation_translation();
        let frame = match (action, new_parent) {
            (DropAction::InsertAtParent, Some(p)) => self.world_matrix(p)?,
            _ => d_world,
        };
        let (_, f_rot, f_pos) = frame.to_scale_rotation_translation();
        let source_world = match action {
            DropAction::InsertInPlace => s_world,
            _ => DMat4::from_scale_rotation_translation(s_scale, f_rot, f_pos),
        };
        // Hierarchy: source into the destination's old slot under its old
        // parent, destination under the source.
        let from = self.item_index(source).unwrap_or(0);
        let item = self.items.remove(from);
        let slot = self.item_index(destination).unwrap_or(self.items.len());
        self.items.insert(slot, item);
        self.items[slot].parent = new_parent;
        self.set_world_matrix(source, source_world)?;
        if let Some(d) = self.item_mut(destination) {
            d.parent = Some(source);
        }
        self.set_world_matrix(destination, d_world)
    }

    /// DA-21/22: move `source`'s pivot to `point`; with `normal`, also turn
    /// it by the smallest rotation taking its world +Y onto the normal
    /// (DA-Q6).
    fn place(&mut self, source: u64, point: DVec3, normal: Option<DVec3>) -> Result<()> {
        let (scale, mut rotation, _) = self.world_matrix(source)?.to_scale_rotation_translation();
        if let Some(n) = normal {
            let up = rotation * DVec3::Y;
            rotation =
                (DQuat::from_rotation_arc(up.normalize(), n.normalize()) * rotation).normalize();
        }
        self.set_world_matrix(
            source,
            DMat4::from_scale_rotation_translation(scale, rotation, point),
        )
    }
}
