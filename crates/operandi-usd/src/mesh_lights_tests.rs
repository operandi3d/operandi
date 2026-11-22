//! Mesh lights through USD (usd-gaps-3): MeshLightAPI on the prototype's
//! Mesh prim, both ways.
use operandi_core::Scene;
use operandi_core::camera_light::{LightKind, LightSettings};
use operandi_core::interchange::ExportFormat;
use operandi_core::mesh::Mesh;
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-usd-ml-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn panel(scene: &mut Scene, name: &str) -> u64 {
    scene.new_mesh().unwrap();
    let item = scene.items.last_mut().unwrap();
    item.mesh = Mesh::box_mesh(glam::DVec3::ZERO, glam::DVec3::ONE, 1)
        .unwrap()
        .into();
    item.name = name.into();
    item.id
}

fn mesh_light(scene: &mut Scene, proto: u64, name: &str, on_mesh: bool) {
    let mut l = LightSettings::new(LightKind::Mesh);
    l.intensity = 3.5;
    l.color = [1.0, 0.5, 0.25];
    l.affect_specular = 0.5;
    l.prototype = Some(proto);
    l.use_prototype_transform = on_mesh;
    scene.add_light(l, Default::default()).unwrap();
    scene.items.last_mut().unwrap().name = name.into();
}

#[test]
fn a_mesh_light_is_mesh_light_api_on_its_mesh_both_ways() {
    let mut scene = Scene::default();
    let glow = panel(&mut scene, "Glow");
    let other = panel(&mut scene, "Other");
    mesh_light(&mut scene, glow, "Panel Light", true);
    mesh_light(&mut scene, other, "Loose Light", false);
    let out = scratch("rt").join("lights.usda");
    let report = crate::export(&out, &scene, ExportFormat::Usda).unwrap();
    let text = std::fs::read_to_string(&out).unwrap();
    for expected in [
        "\"MeshLightAPI\"",
        "float inputs:intensity = 3.5",
        "color3f inputs:color = (1, 0.5, 0.25)",
        "bool inputs:normalize = 0",
        "float inputs:specular = 0.5",
        "token light:materialSyncMode = \"noMaterialResponse\"",
        "string meshLight = \"Panel Light\"",
    ] {
        assert!(text.contains(expected), "missing {expected}:\n{text}");
    }
    let notes = report.notes.join("\n");
    assert!(
        notes.contains("Mesh lights not exported: Loose Light (Use Prototype Transform off)"),
        "{notes}"
    );

    let back = crate::import(&out, &Default::default()).unwrap();
    let lights: Vec<_> = back
        .scene
        .items
        .iter()
        .filter(|i| i.light.as_ref().is_some_and(|l| l.kind == LightKind::Mesh))
        .collect();
    assert_eq!(lights.len(), 1, "{:#?}", back.notes);
    let item = lights[0];
    assert_eq!(item.name, "Panel Light");
    let l = item.light.as_ref().unwrap();
    assert!((l.intensity - 3.5).abs() < 1e-6);
    assert_eq!(l.color, [1.0, 0.5, 0.25]);
    assert!((l.affect_specular - 0.5).abs() < 1e-6);
    assert!(l.use_prototype_transform);
    let proto = back
        .scene
        .items
        .iter()
        .find(|i| Some(i.id) == l.prototype)
        .unwrap();
    assert_eq!(proto.name, "Glow");
    assert!(!proto.mesh.faces.is_empty());
}
