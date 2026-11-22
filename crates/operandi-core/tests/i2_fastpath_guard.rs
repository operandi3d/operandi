//! FP-01: fast Move must preserve the authoritative extent-dependent refusal.
use glam::DVec3;
use operandi_core::*;

const REFUSAL: &str = "This tool cannot yet remap typed maps/named sets; source retained";

fn params(z: f64) -> Parameters {
    Parameters {
        extent: DVec3::new(0., 0., z),
        extrude_mode: ExtrudeMode::Shift,
        extrude_uvs: ExtrudeUvs::None,
        ..Default::default()
    }
}

fn armed_oblique_p0() -> Editor {
    // Original admitted P0; rotate positions only, preserving its authored data.
    let mut scene: Scene = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../docs/i2/expectations/graphs/P0.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for v in &mut scene.items[0].mesh.vertices {
        *v = DVec3::new(v.z, v.x, v.y);
    }
    scene.validate().unwrap();
    assert_eq!(scene.items[0].mesh.normal(0), DVec3::X);
    let mut e = Editor {
        scene,
        ..Default::default()
    };
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(params(0.))).unwrap();
    e
}

#[test]
fn oblique_named_set_move_preserves_full_refusal_and_accepted_state() {
    for configure in [false, true] {
        let armed = armed_oblique_p0();
        assert_eq!(
            extrude_geometry(
                &armed.scene.items[0].mesh,
                &armed.scene.selection.faces,
                params(0.5).extent,
                1,
                ExtrudeMode::Shift,
                ExtrudeUvs::None,
                Some("Texture")
            )
            .unwrap_err(),
            REFUSAL
        );
        let mut legacy = armed.clone();
        assert_eq!(
            legacy.dispatch(Command::Preview(params(0.5))),
            Err(REFUSAL.into())
        );
        assert_eq!(legacy.scene, armed.scene);
        assert_eq!(legacy.identities.current, armed.identities.current);

        let mut e = armed;
        assert!(e.i2_begin_source_supported());
        e.dispatch(Command::BeginExtrude).unwrap();
        // Check refusal from both the begun state and an accepted, nonzero
        // oblique sample inside the kernel's tolerance. Recovery returns zero.
        for accepted_z in [0., 5e-11, 0.] {
            let accepted = if configure {
                Command::ConfigureExtrude(params(accepted_z))
            } else {
                Command::Preview(params(accepted_z))
            };
            e.dispatch(accepted).unwrap();
            Mesh::validate(&e.scene.items[0].mesh).unwrap();
            e.identities.current.validate(&e.scene).unwrap();
            let before = e.clone();
            let marks = (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap());
            let command = if configure {
                Command::ConfigureExtrude(params(0.5))
            } else {
                Command::Preview(params(0.5))
            };
            // A panic is a test failure in both debug and release.
            assert_eq!(e.dispatch(command), Err(REFUSAL.into()));
            // Includes scene, selection, stage cursor/images, live/preset state,
            // histories, identity/publication/request state and diagnostics.
            assert!(e == before, "refused Move changed accepted editor state");
            assert_eq!(
                (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap()),
                marks
            );
        }
    }
}
