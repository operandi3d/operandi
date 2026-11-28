use glam::{DVec2, DVec3};
use operandi_core::*;

fn fixture() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut mesh = Mesh::box_mesh(DVec3::splat(-4.), DVec3::ONE, 0).unwrap();
    for (i, face) in mesh.faces.iter_mut().enumerate() {
        face.uv = Some(vec![DVec2::splat(i as f64); 4]);
        face.part = Some(format!("part-{i}"));
        face.material_label = Some(format!("mat-{i}"));
    }
    mesh.uv_maps.insert(
        "original-seam".into(),
        (0..6)
            .map(|i| vec![Some(DVec2::splat(i as f64)); 4])
            .collect(),
    );
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![5, 2];
    e
}

#[test]
fn m16_replacement_history_attributes_cancel_and_committed_save_reopen() {
    for tool in [Tool::Sphere, Tool::Cylinder] {
        let mut e = fixture();
        let base = e.scene.clone();
        e.dispatch(Command::Activate(tool)).unwrap();
        assert_eq!(e.scene, base, "activation creates no geometry");
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.position = DVec3::new(1., 2., -3.);
        p.radial = Some(RadialParameters {
            radius: DVec3::new(1., 1.5, 2.),
            sides: 8,
            segments: if tool == Tool::Sphere { 4 } else { 3 },
        });
        for x in [1., 2., 1.] {
            p.position.x = x;
            e.dispatch(Command::Preview(p.clone())).unwrap();
            let mesh = &e.scene.active().unwrap().mesh;
            let old = &base.active().unwrap().mesh;
            assert_eq!(&mesh.vertices[..8], &old.vertices);
            assert_eq!(&mesh.faces[..6], &old.faces);
            assert_eq!(
                &mesh.uv_maps["original-seam"][..6],
                &old.uv_maps["original-seam"]
            );
            assert!(
                mesh.uv_maps["original-seam"][6..]
                    .iter()
                    .flatten()
                    .all(Option::is_none)
            );
            assert_eq!(e.scene.selection, base.selection);
            assert_eq!(mesh.faces.len(), if tool == Tool::Sphere { 38 } else { 32 });
        }
        // A preview that isn't a step yet: Redo would replace it.
        assert!(e.dispatch(Command::Redo).is_err());
        let valid = e.clone();
        for invalid in [f64::NAN, f64::INFINITY, -1., 1e12] {
            let mut bad = p.clone();
            bad.radial.as_mut().unwrap().radius.y = invalid;
            assert!(e.dispatch(Command::Preview(bad)).is_err());
            assert!(e == valid);
        }
        for (sides, segments) in [(0, 3), (2, 3), (8, 0), (u32::MAX, u32::MAX), (4097, 1)] {
            let mut bad = p.clone();
            let r = bad.radial.as_mut().unwrap();
            r.sides = sides;
            r.segments = segments;
            assert!(e.dispatch(Command::Preview(bad)).is_err());
            assert!(e == valid);
        }
        let bad_path = std::env::temp_dir().join("operandi-m16-no-parent/scene.opscene");
        assert!(e.save_committed(&bad_path).is_err());
        assert!(e == valid);
        e.dispatch(Command::Cancel).unwrap();
        assert_eq!(e.scene, base);
        e.dispatch(Command::Activate(tool)).unwrap();
        e.dispatch(Command::Preview(p)).unwrap();
        let preview = e.scene.clone();
        let path = std::env::temp_dir().join(format!(
            "operandi-m16-{}-{}.opscene",
            tool.label(),
            std::process::id()
        ));
        e.save_committed(&path).unwrap();
        assert!(e.session.live.is_none());
        assert_eq!(e.history.undo.last().unwrap().label, tool.label());
        assert_eq!(persistence::load(&path).unwrap(), preview);
        std::fs::remove_file(path).unwrap();
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, base);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene, preview);
    }
}

#[test]
fn m16_flat_base_is_kept_and_target_refusals_are_atomic() {
    for tool in [Tool::Sphere, Tool::Cylinder] {
        let mut e = Editor::default();
        assert!(e.dispatch(Command::Activate(tool)).is_err());
        e = fixture();
        e.scene.active_mut().unwrap().visible = false;
        assert!(e.dispatch(Command::Activate(tool)).is_err());
        assert!(e.session.live.is_none());
        e.scene.active_mut().unwrap().visible = true;
        // A moved mesh takes primitives now (primitive_moved_mesh.rs).
        e.scene.active_mut().unwrap().transform.translation.x = 1.;
        e.dispatch(Command::Activate(tool)).unwrap();
        e.dispatch(Command::Cancel).unwrap();
        assert!(e.session.live.is_none());
        e.scene.active_mut().unwrap().transform = Default::default();
        let base = e.scene.clone();
        for normal in 0..3 {
            e.dispatch(Command::Activate(tool)).unwrap();
            let mut p = e.session.live.as_ref().unwrap().params.clone();
            p.radial.as_mut().unwrap().radius[normal] = 0.;
            e.dispatch(Command::Preview(p.clone())).unwrap();
            // drag-creation-2026-11-06: the flat base is a real disc.
            let faces = base.active().unwrap().mesh.faces.len();
            assert_eq!(e.scene.active().unwrap().mesh.faces.len(), faces + 1);
            let mut bad = p.clone();
            bad.radial.as_mut().unwrap().radius[(normal + 1) % 3] = f64::NAN;
            assert!(e.dispatch(Command::Preview(bad)).is_err());
            assert_eq!(e.session.live.as_ref().unwrap().params, p);
            e.dispatch(Command::Drop).unwrap();
            // Drop keeps it (rule 4); undo takes it back.
            assert_eq!(e.scene.active().unwrap().mesh.faces.len(), faces + 1);
            e.dispatch(Command::Undo).unwrap();
            assert_eq!(e.scene, base);
        }
    }
}
