//! Original PM-UP01..06 / PM-T13..17 product acceptance fixtures.
use glam::{DVec2, DVec3};
use operandi_core::*;

fn fixture() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh =
        Mesh::box_mesh(DVec3::new(3., 4., 5.), DVec3::new(2., 3., 4.), 0)
            .unwrap()
            .into();
    let mesh = &mut e.scene.active_mut().unwrap().mesh;
    mesh.faces[0].uv = Some(vec![DVec2::new(0.125, 0.875); 4]);
    mesh.faces[0].material = "retained material".into();
    mesh.faces[0].part = Some("retained part".into());
    mesh.uv_maps.insert(
        "seam".into(),
        vec![vec![Some(DVec2::new(0.25, 0.75)); 4]; 6],
    );
    e.scene.selection.faces = vec![0, 2];
    e.scene.workplane.center = DVec3::new(21., -13., 8.);
    e
}
fn retain_resolution(e: &mut Editor, tool: Tool) {
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.position = DVec3::new(10., 12., 14.);
    p.size = DVec3::new(2., 3., 4.);
    p.segments = [2, 3, 4];
    if let Some(r) = p.radial.as_mut() {
        r.radius = DVec3::new(2., 3., 4.);
        r.sides = 8;
        r.segments = if tool == Tool::Sphere { 4 } else { 3 };
    }
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
}
/// The unit shapes are the tools' reset shapes, whatever was last used
/// (unit-primitive-2026-11-05): a 1 m cube, a 0.5 m sphere, a 1.5 m tall
/// cylinder.
fn canonical(tool: Tool, mesh: &Mesh, first: usize) {
    for v in &mesh.vertices[first..] {
        assert!(v.is_finite());
        let half_height = if tool == Tool::Cylinder { 0.75 } else { 0.5 };
        assert!(v.x.abs().max(v.z.abs()) <= 0.5 + 1e-10 && v.y.abs() <= half_height + 1e-10);
        match tool {
            Tool::Box => assert!((v.abs().max_element() - 0.5).abs() < 1e-10),
            Tool::Sphere => assert!((v.length() - 0.5).abs() < 1e-10),
            Tool::Cylinder => assert!((v.x.hypot(v.z) - 0.5).abs() < 1e-10),
            _ => unreachable!(),
        }
    }
    mesh.validate().unwrap();
}

#[test]
fn pm_t13_t15_t17_canonical_existing_origin_retained_resolution_attributes_and_history() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        let mut e = fixture();
        retain_resolution(&mut e, tool);
        let before = e.clone();
        let prior = &before.scene.active().unwrap().mesh;
        e.dispatch(Command::InsertUnit {
            tool,
            destination: UnitDestination::ExistingMesh,
        })
        .unwrap();
        assert!(e.session.live.is_none());
        assert_eq!(e.scene.items.len(), before.scene.items.len());
        assert_eq!(e.scene.selection, before.scene.selection);
        assert_eq!(e.scene.workplane, before.scene.workplane);
        assert_eq!(
            e.scene.active().unwrap().transform,
            before.scene.active().unwrap().transform
        );
        let mesh = &e.scene.active().unwrap().mesh;
        canonical(tool, mesh, prior.vertices.len());
        assert_eq!(&mesh.vertices[..prior.vertices.len()], &prior.vertices);
        assert_eq!(&mesh.faces[..prior.faces.len()], &prior.faces);
        assert_eq!(
            &mesh.uv_maps["seam"][..prior.faces.len()],
            &prior.uv_maps["seam"]
        );
        assert!(
            mesh.uv_maps["seam"][prior.faces.len()..]
                .iter()
                .flatten()
                .all(Option::is_none)
        );
        let added = mesh.faces.len() - prior.faces.len();
        assert_eq!(
            added,
            match tool {
                Tool::Box => 6,
                Tool::Sphere => 288,
                _ => 26,
            }
        );
        // Unit canonical values must not overwrite ordinary remembered fields.
        assert_eq!(
            *e.presets.get::<operandi_core::CubePreset>(),
            *before.presets.get::<operandi_core::CubePreset>()
        );
        assert_eq!(e.sphere_unit_resolution, before.sphere_unit_resolution);
        assert_eq!(e.cylinder_unit_resolution, before.cylinder_unit_resolution);
        assert_eq!(e.history.undo.len(), before.history.undo.len() + 1);
        let result = e.scene.clone();
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, before.scene);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene, result);
        assert!(e.session.live.is_none());
        let path = std::env::temp_dir().join(format!(
            "operandi-unit-{}-{tool:?}.opscene",
            std::process::id()
        ));
        e.save_committed(&path).unwrap();
        assert_eq!(persistence::load(&path).unwrap(), result);
        std::fs::remove_file(path).unwrap();
        // Repetition appends one additional unit, never changes the old one.
        let prior = e.scene.active().unwrap().mesh.clone();
        e.dispatch(Command::InsertUnit {
            tool,
            destination: UnitDestination::ExistingMesh,
        })
        .unwrap();
        assert_eq!(
            &e.scene.active().unwrap().mesh.vertices[..prior.vertices.len()],
            &prior.vertices
        );
        assert_eq!(
            e.scene.active().unwrap().mesh.faces.len() - prior.faces.len(),
            added
        );
    }
}

#[test]
fn pm_t14_t15_new_mesh_and_geometry_are_one_action_with_existing_naming_selection() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        let mut e = fixture();
        retain_resolution(&mut e, tool);
        // Shift creates a new eligible target, without editing the old hidden,
        // transformed mesh. This is the existing New Mesh operation's policy.
        e.scene.active_mut().unwrap().visible = false;
        e.scene.active_mut().unwrap().transform.translation = DVec3::new(3., 2., 1.);
        let before = e.clone();
        e.dispatch(Command::InsertUnit {
            tool,
            destination: UnitDestination::NewMesh,
        })
        .unwrap();
        assert_eq!(
            &e.scene.items[..before.scene.items.len()],
            &before.scene.items
        );
        let item = e.scene.active().unwrap();
        assert_eq!(item.transform, Default::default());
        assert_eq!(item.parent, None);
        // Named after the shape (rule 10).
        assert_eq!(item.name, tool.label());
        assert_eq!(e.scene.selection.mode, before.scene.selection.mode);
        assert!(e.scene.selection.faces.is_empty());
        canonical(tool, &item.mesh, 0);
        assert!(e.session.live.is_none());
        assert_eq!(e.history.undo.len(), before.history.undo.len() + 1);
        let id = item.id;
        let result = e.scene.clone();
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, before.scene);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene, result);
        assert!(e.session.live.is_none());
        e.dispatch(Command::Undo).unwrap();
        e.dispatch(Command::InsertUnit {
            tool,
            destination: UnitDestination::NewMesh,
        })
        .unwrap();
        assert!(
            e.scene.active().unwrap().id > id,
            "existing monotonic identity policy"
        );
    }
}

#[test]
fn pm_t10_t15_failure_is_exactly_atomic_for_targets_tasks_profiles_and_budgets() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        for destination in [UnitDestination::ExistingMesh, UnitDestination::NewMesh] {
            // An open tool no longer refuses: it is dropped first
            // (2026-11-05; tool_switch_drop.rs).
            // Unit shapes ignore remembered resolutions, so a bad one can't
            // make them fail (unit-primitive-2026-11-05 rule 9).
            for invalid in ["selection", "identity budget"] {
                let mut e = fixture();
                match invalid {
                    "selection" => e.dispatch(Command::BeginSelection).unwrap(),
                    "resolution" => match tool {
                        Tool::Box => {
                            e.presets.get_mut::<operandi_core::CubePreset>().segments = [0, 1, 1]
                        }
                        Tool::Sphere => e.sphere_unit_resolution = Some([8, 1]),
                        _ => e.cylinder_unit_resolution = Some([0, 1]),
                    },
                    "item budget" => {
                        e.presets.get_mut::<operandi_core::CubePreset>().segments = [u32::MAX; 3];
                        e.sphere_unit_resolution = Some([u32::MAX; 2]);
                        e.cylinder_unit_resolution = Some([u32::MAX; 2]);
                    }
                    _ => {
                        if destination == UnitDestination::NewMesh {
                            e.next_item_id = u64::MAX - 1;
                        } else {
                            e.scene.selection.item = None;
                            // No mesh selected since the scene opened (VP-04).
                            e.scene.last_mesh = Default::default();
                        }
                    }
                }
                let before = e.clone();
                assert!(
                    e.dispatch(Command::InsertUnit { tool, destination })
                        .is_err(),
                    "{tool:?}/{destination:?}/{invalid}"
                );
                assert!(
                    e == before,
                    "failed edit changed state: {tool:?}/{destination:?}/{invalid}"
                );
            }
        }
        // A moved or parented mesh takes unit shapes now
        // (primitive_moved_mesh.rs).
        for invalid in ["empty", "hidden"] {
            let mut e = fixture();
            if invalid == "empty" {
                e.scene.select_item(None);
                // No mesh selected since the scene opened (VP-04).
                e.scene.last_mesh = Default::default();
            } else {
                e.scene.active_mut().unwrap().visible = false;
            }
            let before = e.clone();
            assert!(
                e.dispatch(Command::InsertUnit {
                    tool,
                    destination: UnitDestination::ExistingMesh
                })
                .is_err()
            );
            assert!(e == before);
        }
    }
    let mut e = Editor::default();
    e.dispatch(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: UnitDestination::NewMesh,
    })
    .unwrap();
    assert_eq!(e.scene.items.len(), 1);
    assert!(e.session.live.is_none());
}

#[test]
fn pm_t16_metre_storage_and_faceted_envelope_do_not_depend_on_display_conversion() {
    for display in ["1 m", "100 cm", "1000 mm", "39.37007874015748 in"] {
        assert!((numeric::parse(display).unwrap() - 1.).abs() < 1e-12);
    }
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        let mut e = fixture();
        // Odd-side faceting need not attain all nominal envelope extrema.
        e.sphere_unit_resolution = Some([7, 3]);
        e.cylinder_unit_resolution = Some([7, 2]);
        e.dispatch(Command::InsertUnit {
            tool,
            destination: UnitDestination::NewMesh,
        })
        .unwrap();
        canonical(tool, &e.scene.active().unwrap().mesh, 0);
        let before = e.scene.clone();
        for unit in ["100 cm", "1000 mm", "39.37007874015748 in"] {
            numeric::parse(unit).unwrap();
            assert_eq!(e.scene, before);
        }
        // Jason decision 2026-11-01: the next ordinary use starts with the
        // kept Sides/Segments too; radii stay the starting sample values.
        e.dispatch(Command::Activate(tool)).unwrap();
        if tool == Tool::Sphere {
            assert_eq!(
                e.session
                    .live
                    .as_ref()
                    .unwrap()
                    .params
                    .radial
                    .unwrap()
                    .segments,
                3
            );
        }
        if tool == Tool::Cylinder {
            assert_eq!(
                e.session
                    .live
                    .as_ref()
                    .unwrap()
                    .params
                    .radial
                    .unwrap()
                    .radius
                    .y,
                0.75
            );
        }
    }
}
