//! Live patches (G7 PC-7): the USD export writes the tessellated mesh.
use glam::DVec3;
use operandi_core::curves::CurveKind;
use operandi_core::{Editor, Face, Mesh, PolygonRepresentation};

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// Four straight curves around a square and a live patch on its corners.
fn patched_square() -> Mesh {
    let mut m = Mesh {
        vertices: vec![
            DVec3::new(-1., 0., -1.),
            DVec3::new(1., 0., -1.),
            DVec3::new(1., 0., 1.),
            DVec3::new(-1., 0., 1.),
        ],
        ..Default::default()
    };
    for k in 0..4u32 {
        m.faces.push(face(vec![k, (k + 1) % 4]));
        m.attributes
            .representations
            .push(PolygonRepresentation::Polyline {
                closed: false,
                curve: CurveKind::Line,
            });
    }
    m.faces.push(face(vec![0, 1, 2, 3]));
    m.attributes
        .representations
        .push(PolygonRepresentation::Patch);
    m.validate().unwrap();
    m
}

#[test]
fn a_live_patch_exports_as_its_tessellation() {
    operandi_core::interchange::set_usd_exporter(crate::export);
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.items[0].mesh = patched_square().into();
    let dir = std::env::temp_dir().join(format!("operandi-usd-patch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("patch.usda");
    let mut args = serde_json::Map::new();
    args.insert(
        "filename".into(),
        path.to_string_lossy().into_owned().into(),
    );
    operandi_core::command_exec::run(&mut e, "scene.saveAs", &args).unwrap();
    let back = crate::import(&path, &Default::default()).unwrap();
    let quads: usize = back
        .scene
        .items
        .iter()
        .map(|i| {
            (0..i.mesh.faces.len())
                .filter(|&f| !i.mesh.is_polyline(f) && i.mesh.faces[f].vertices.len() == 4)
                .count()
        })
        .sum();
    assert_eq!(quads, 256, "16 x 16 cells, not the one patch polygon");
}
