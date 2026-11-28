//! Falloff batch 3 (falloff spec 2026-11-15): Cylinder (FO-42) and Use
//! World Transforms (FO-35). Original test data.
use glam::DVec3;
use operandi_core::*;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// A unit cube moved to x = 10, all its points selected (Vertex mode).
fn moved_cube() -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = cube_geometry(DVec3::ZERO, DVec3::ONE).unwrap().into();
    e.scene.active_mut().unwrap().transform.translation = DVec3::new(10., 0., 0.);
    e.scene.selection.mode = Mode::Vertex;
    let n = e.scene.active().unwrap().mesh.vertices.len() as u32;
    e.scene.selection.vertices = (0..n).collect();
    let id = e.scene.active().unwrap().id;
    e.history.undo.clear();
    (e, id)
}

/// FO-42: measured defaults; the Axis component is ignored, the other two
/// fall off like Radial.
#[test]
fn cylinder_ignores_its_axis() {
    let c = CylinderFalloff::default();
    assert_eq!(
        (c.center, c.size, c.axis, c.shape, c.world),
        (DVec3::ZERO, DVec3::ONE, 1, FalloffShape::Linear, false)
    );
    assert!(close(c.weight(DVec3::new(0., 50., 0.)), 1.));
    assert!(close(c.weight(DVec3::new(0.5, -7., 0.)), 0.5));
    assert!(close(c.weight(DVec3::new(0.3, 0., 0.4)), 0.5));
    assert_eq!(c.weight(DVec3::new(1., 0., 0.)), 0.);
    let along_x = CylinderFalloff { axis: 0, ..c };
    assert!(close(along_x.weight(DVec3::new(9., 0.5, 0.)), 0.5));
    assert!(CylinderFalloff { axis: 3, ..c }.validate().is_err());
}

/// FO-35: off (factory), positions are the mesh's own, so a moved mesh
/// weighs as if it weren't moved; on, they are world positions.
#[test]
fn use_world_transforms_picks_the_space() {
    let (e, id) = moved_cube();
    let linear = |world| LinearFalloff {
        start: DVec3::new(-0.5, 0., 0.),
        end: DVec3::new(0.5, 0., 0.),
        world,
        ..Default::default()
    };
    let own = ActiveFalloff::new(linear(false).into())
        .weights(&e.scene, id)
        .unwrap();
    let points = e.scene.active().unwrap().mesh.vertices.clone();
    for (w, p) in own.iter().zip(&points) {
        assert!(close(*w, 0.5 - p.x), "{w} at {p}");
    }
    let world = ActiveFalloff::new(linear(true).into())
        .weights(&e.scene, id)
        .unwrap();
    assert!(world.iter().all(|&w| w == 0.), "{world:?}");
    // Auto Size fits in the falloff's own space.
    let r = RadialFalloff::default().auto_size(&e.scene).unwrap();
    assert!(close(r.center.x, 0.), "{:?}", r.center);
    let r = RadialFalloff {
        world: true,
        ..Default::default()
    }
    .auto_size(&e.scene)
    .unwrap();
    assert!(close(r.center.x, 10.), "{:?}", r.center);
}

/// A Move under a Cylinder falloff moves each point by its weight, and
/// Cancel restores.
#[test]
fn cylinder_drives_a_move() {
    let (mut e, _) = moved_cube();
    let before = e.scene.active().unwrap().mesh.vertices.clone();
    let c = CylinderFalloff {
        size: DVec3::splat(2.),
        ..Default::default()
    };
    e.dispatch(Command::SpatialFalloff(Some(c.into()))).unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let p = e.session.live.as_ref().unwrap().params.clone();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::Y,
        ..p
    }))
    .unwrap();
    let after = e.scene.active().unwrap().mesh.vertices.clone();
    for (a, b) in after.iter().zip(&before) {
        assert!(close(a.y - b.y, c.weight(*b)), "{b}");
    }
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.vertices, before);
}
