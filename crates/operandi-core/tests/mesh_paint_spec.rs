//! Mesh Paint and Mesh Paint Instance against specifications/missing-tools-g6
//! (G6-MESH-PAINT v1): setup (MP-01/02), placement (MP-09..13), the two
//! copiers (MP-14/15), one undo step per stroke (MP-16), symmetry (MP-17)
//! and the seeded random choices (MP-04).
use glam::DVec3;
use operandi_core::duplicate_items::ParentMode;
use operandi_core::mesh_paint::*;
use operandi_core::*;

/// A unit cube as the background (a new item), then a Plane Item as the
/// target, selected.
fn editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::InsertUnit {
        tool: Tool::Box,
        destination: UnitDestination::NewMesh,
    })
    .unwrap();
    e.dispatch(Command::InsertShapeUnit(
        UnitShape::Plane,
        UnitDestination::NewMesh,
    ))
    .unwrap();
    e
}

fn stroke(settings: MeshPaintSettings, points: Vec<PaintPoint>) -> Command {
    Command::MeshPaint(Box::new(MeshPaintStroke {
        settings,
        points,
        seed: 7,
        view_up: DVec3::Y,
        brush_world: 0.5,
    }))
}

fn target(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

fn bounds(points: &[DVec3]) -> (DVec3, DVec3) {
    points.iter().fold(
        (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
        |(lo, hi), v| (lo.min(*v), hi.max(*v)),
    )
}

#[test]
fn mp_01_without_a_background_mesh_it_refuses() {
    let mut e = Editor::default();
    e.dispatch(Command::InsertShapeUnit(
        UnitShape::Plane,
        UnitDestination::NewMesh,
    ))
    .unwrap();
    let err = e
        .dispatch(stroke(
            Default::default(),
            vec![PaintPoint::at(DVec3::ZERO, DVec3::Y)],
        ))
        .unwrap_err();
    assert_eq!(err, NO_SOURCE);
}

#[test]
fn mp_14_a_click_adds_the_source_as_geometry_at_the_point_one_undo_step() {
    let mut e = editor();
    let before = target(&e).clone();
    let steps = e.undo.len();
    e.dispatch(stroke(
        Default::default(),
        vec![PaintPoint::at(DVec3::new(0.2, 0., 0.1), DVec3::Y)],
    ))
    .unwrap();
    let m = target(&e);
    assert_eq!(m.faces.len(), 1 + 6, "the plane and the cube");
    // MP-13: the source's origin on the point (Locked, 100 %: the source's
    // own size, a 1 m cube about its origin).
    let (lo, hi) = bounds(&m.vertices[4..]);
    assert!(((lo + hi) / 2. - DVec3::new(0.2, 0., 0.1)).length() < 1e-9);
    assert!((hi - lo - DVec3::ONE).length() < 1e-9);
    assert_eq!(e.undo.len(), steps + 1, "MP-16");
    assert_eq!(e.undo.last().unwrap().label, "Mesh Paint");
    e.dispatch(Command::Undo).unwrap();
    assert!(target(&e) == &before);
    // The background is untouched.
    assert_eq!(e.scene.items[0].mesh.faces.len(), 6);
}

#[test]
fn mp_09_10_surface_align_turns_up_onto_the_normal_and_offset_lifts() {
    let mut e = editor();
    let settings = MeshPaintSettings {
        size: DVec3::new(100., 300., 100.),
        offset: 0.5,
        ..Default::default()
    };
    e.dispatch(stroke(
        settings,
        vec![PaintPoint::at(DVec3::ZERO, DVec3::X)],
    ))
    .unwrap();
    let (lo, hi) = bounds(&target(&e).vertices[4..]);
    // The cube stretched 3× along its +Y, which now points along +X.
    assert!((hi.x - lo.x - 3.).abs() < 1e-9, "{lo} {hi}");
    assert!((hi.y - lo.y - 1.).abs() < 1e-9);
    // Centred 0.5 m along the normal.
    assert!(((lo + hi) / 2. - DVec3::new(0.5, 0., 0.)).length() < 1e-9);
    // Uniform keeps the source's own orientation.
    let mut e = editor();
    let settings = MeshPaintSettings {
        size: DVec3::new(100., 300., 100.),
        rotation_mode: RotationMode::Uniform,
        ..Default::default()
    };
    e.dispatch(stroke(
        settings,
        vec![PaintPoint::at(DVec3::ZERO, DVec3::X)],
    ))
    .unwrap();
    let (lo, hi) = bounds(&target(&e).vertices[4..]);
    assert!((hi.y - lo.y - 3.).abs() < 1e-9);
}

#[test]
fn mp_04_the_same_seed_makes_the_same_random_copies() {
    let settings = MeshPaintSettings {
        rotation_mode: RotationMode::Random,
        rotation: DVec3::splat(90.),
        size_mode: SizeMode::Random,
        size: DVec3::splat(50.),
        ..Default::default()
    };
    let run = |seed: u64| {
        let mut e = editor();
        e.dispatch(Command::MeshPaint(Box::new(MeshPaintStroke {
            settings,
            points: (0..5)
                .map(|i| PaintPoint::at(DVec3::new(f64::from(i), 0., 0.), DVec3::Y))
                .collect(),
            seed,
            view_up: DVec3::Y,
            brush_world: 0.5,
        })))
        .unwrap();
        target(&e).vertices.clone()
    };
    assert_eq!(run(3), run(3));
    assert_ne!(run(3), run(4));
}

#[test]
fn mp_12_falloff_density_drops_points_at_the_brush_edge() {
    let mut e = editor();
    let mut edge = PaintPoint::at(DVec3::ZERO, DVec3::Y);
    edge.falloff = 0.;
    e.dispatch(stroke(Default::default(), vec![edge])).unwrap();
    assert_eq!(target(&e).faces.len(), 1, "density 0: nothing placed");
}

#[test]
fn mp_14_random_and_all_background_layers() {
    let mut e = editor();
    // A second background: a sphere.
    e.dispatch(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: UnitDestination::NewMesh,
    })
    .unwrap();
    let sphere_faces = e.scene.active().unwrap().mesh.faces.len();
    let plane = e.scene.items[1].id;
    e.dispatch(Command::SelectItem(Some(plane))).unwrap();
    let points: Vec<PaintPoint> = (0..20)
        .map(|i| PaintPoint::at(DVec3::new(f64::from(i), 0., 0.), DVec3::Y))
        .collect();
    e.dispatch(stroke(Default::default(), points)).unwrap();
    let faces = target(&e).faces.len() - 1;
    // Random BG Layer: a mix of cubes and spheres.
    assert!(faces > 20 * 6 && faces < 20 * sphere_faces, "{faces}");
    assert_eq!(
        (faces - 20 * 6) % (sphere_faces - 6),
        0,
        "whole cubes and spheres"
    );
    // All BG Layers: both in every copy.
    let mut e2 = e.clone();
    e2.dispatch(Command::Undo).unwrap();
    let settings = MeshPaintSettings {
        source: GeometrySource::AllLayers,
        ..Default::default()
    };
    e2.dispatch(stroke(
        settings,
        vec![PaintPoint::at(DVec3::ZERO, DVec3::Y)],
    ))
    .unwrap();
    assert_eq!(target(&e2).faces.len(), 1 + 6 + sphere_faces);
}

#[test]
fn mp_15_instances_are_new_items_placed_and_parented() {
    for (parent, expect_parent) in [(ParentMode::Off, false), (ParentMode::Root, true)] {
        let mut e = editor();
        let cube = e.scene.items[0].id;
        let settings = MeshPaintSettings {
            instances: true,
            parent,
            ..Default::default()
        };
        e.dispatch(stroke(
            settings,
            vec![PaintPoint::at(DVec3::new(1., 2., 3.), DVec3::Y)],
        ))
        .unwrap();
        assert_eq!(e.undo.last().unwrap().label, "Mesh Paint Instance");
        let copies: Vec<&Item> = e
            .scene
            .items
            .iter()
            .filter(|i| i.instance_of == Some(cube))
            .collect();
        assert_eq!(copies.len(), 1);
        let world = e.scene.world_matrix(copies[0].id).unwrap();
        assert!(
            world
                .transform_point3(DVec3::ZERO)
                .abs_diff_eq(DVec3::new(1., 2., 3.), 1e-9)
        );
        assert_eq!(copies[0].parent == Some(cube), expect_parent, "{parent:?}");
        // The target stays selected and unchanged.
        assert_eq!(target(&e).faces.len(), 1);
    }
}

#[test]
fn mp_17_symmetry_mirrors_each_copy() {
    let mut e = editor();
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    e.dispatch(stroke(
        Default::default(),
        vec![PaintPoint::at(DVec3::new(2., 0., 0.), DVec3::Y)],
    ))
    .unwrap();
    let m = target(&e);
    assert_eq!(m.faces.len(), 1 + 12);
    let (lo, hi) = bounds(&m.vertices[4..]);
    assert!(
        (lo.x + 2.5).abs() < 1e-9 && (hi.x - 2.5).abs() < 1e-9,
        "{lo} {hi}"
    );
}

#[test]
fn a_painted_scene_saves_and_reopens() {
    let mut e = editor();
    e.dispatch(stroke(
        Default::default(),
        vec![PaintPoint::at(DVec3::ZERO, DVec3::Y)],
    ))
    .unwrap();
    let path = std::env::temp_dir().join(format!("mesh-paint-{}.opscene", std::process::id()));
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    std::fs::remove_file(&path).ok();
    assert!(*back.active().unwrap().mesh == *target(&e));
}

#[test]
fn surface_hit_finds_the_target_plane() {
    let e = editor();
    let plane = e.scene.active().unwrap().id;
    let (at, n) = e
        .scene
        .surface_hit(plane, DVec3::new(0.1, 5., 0.2), -DVec3::Y)
        .unwrap();
    assert!(at.abs_diff_eq(DVec3::new(0.1, 0., 0.2), 1e-12));
    assert!(n.abs_diff_eq(DVec3::Y, 1e-12));
    assert!(
        e.scene
            .surface_hit(plane, DVec3::new(3., 5., 0.), -DVec3::Y)
            .is_none()
    );
}
