//! Live patches (G7 PC-7): renders get the tessellated surface from the
//! evaluated scene, not the one patch polygon.
use super::*;
use glam::DVec3;
use operandi_core::evaluate::Purpose;
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

#[test]
fn a_live_patch_renders_as_its_tessellation() {
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
                curve: operandi_core::curves::CurveKind::Line,
            });
    }
    m.faces.push(face(vec![0, 1, 2, 3]));
    m.attributes
        .representations
        .push(PolygonRepresentation::Patch);
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.items[0].mesh = m.into();
    let document: usize = scene_meshes(&e.scene)
        .unwrap()
        .iter()
        .map(|r| r.triangles.len())
        .sum();
    assert_eq!(document, 2, "the stored scene has one quad");
    let rendered: usize = scene_meshes(&e.evaluated(Purpose::Final))
        .unwrap()
        .iter()
        .map(|r| r.triangles.len())
        .sum();
    assert_eq!(rendered, 512, "16 x 16 cells, two triangles each");
}
