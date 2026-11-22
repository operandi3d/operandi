//! Where a mesh preset or imported file lands (preset browser gaps
//! PG-04, PG-06). PG-03 (no item transform kept) is PB-44, in
//! `PresetSource::build`.
use crate::Scene;
use glam::DVec3;

/// The world-space box of every mesh point in `scene`, if it has any.
pub fn mesh_bounds(scene: &Scene) -> Option<(DVec3, DVec3)> {
    let mut bounds: Option<(DVec3, DVec3)> = None;
    for item in scene.items.iter().filter(|i| i.is_mesh()) {
        let Ok(world) = scene.world_matrix(item.id) else {
            continue;
        };
        for &p in &item.mesh.vertices {
            let w = world.transform_point3(p);
            bounds = Some(match bounds {
                Some((lo, hi)) => (lo.min(w), hi.max(w)),
                None => (w, w),
            });
        }
    }
    bounds
}

/// Move `scene`'s top-level items so its meshes' box has the middle of its
/// bottom (centred on X and Z, lowest Y) at `point`: PG-06's Shift-drop
/// onto a surface, and PG-04's Centre on Ground with `point` at the
/// origin. Returns whether anything moved (a scene with no mesh points
/// stays as it is).
pub fn rest_on(scene: &mut Scene, point: DVec3) -> bool {
    let Some((lo, hi)) = mesh_bounds(scene) else {
        return false;
    };
    let base = DVec3::new((lo.x + hi.x) / 2., lo.y, (lo.z + hi.z) / 2.);
    let offset = point - base;
    if offset == DVec3::ZERO {
        return false;
    }
    for item in scene.items.iter_mut().filter(|i| i.parent.is_none()) {
        item.transform.translation += offset;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_scene() -> Scene {
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        let item = scene.items.last_mut().unwrap();
        item.mesh = crate::primitive::cube_geometry(DVec3::ZERO, DVec3::ONE)
            .unwrap()
            .into();
        item.transform.translation = DVec3::new(2., 0., 0.);
        item.transform.rotation = DVec3::new(0., 0.5, 0.);
        scene
    }

    /// PG-04: at the origin, the cube sits at the origin; resting it puts its
    /// bottom centre on the given point.
    #[test]
    fn clear_and_rest_on_a_point() {
        let mut s = cube_scene();
        s.items[0].transform = Default::default();
        let (lo, hi) = mesh_bounds(&s).unwrap();
        assert!((lo + hi).length() < 1e-9, "{lo} {hi}");
        assert!(rest_on(&mut s, DVec3::ZERO));
        let (lo, hi) = mesh_bounds(&s).unwrap();
        assert!(lo.y.abs() < 1e-9 && (lo.x + hi.x).abs() < 1e-9 && (lo.z + hi.z).abs() < 1e-9);
        assert!(rest_on(&mut s, DVec3::new(0., 3., 1.)));
        let (lo, _) = mesh_bounds(&s).unwrap();
        assert!((lo.y - 3.).abs() < 1e-9);
        assert!(!rest_on(&mut Scene::default(), DVec3::ONE));
    }
}
