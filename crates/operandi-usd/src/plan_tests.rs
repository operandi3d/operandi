//! What the export writes, checked without OpenUSD.
use crate::plan::*;
use glam::{DVec2, DVec3};
use operandi_core::interchange::IoOptions;
use operandi_core::{Mesh, MeshCrease, PolygonRepresentation, Scene};

/// A scene with one box item named `name`.
pub(crate) fn box_scene(name: &str) -> Scene {
    let mut scene = Scene::default();
    scene.items.clear();
    add_box(&mut scene, name, None);
    scene
}

/// Adds a unit box item and returns its id.
pub(crate) fn add_box(scene: &mut Scene, name: &str, parent: Option<u64>) -> u64 {
    scene.new_mesh().unwrap();
    let item = scene.items.last_mut().unwrap();
    item.name = name.into();
    item.parent = parent;
    item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    item.id
}

fn plan_of(scene: &Scene) -> Plan {
    plan(scene, &IoOptions::default()).unwrap()
}

#[test]
fn a_box_is_one_xform_with_one_mesh() {
    let p = plan_of(&box_scene("Box"));
    assert_eq!((p.items, p.polygons), (1, 6));
    assert_eq!(p.up_axis, 'Y');
    assert_eq!(p.root_matrix, None);
    let [item] = p.prims.as_slice() else { panic!() };
    assert_eq!(item.name, "Box");
    let [mesh] = item.meshes.as_slice() else {
        panic!()
    };
    assert_eq!(mesh.points.len(), 8);
    assert_eq!(mesh.face_counts, vec![4; 6]);
    assert_eq!(mesh.face_indices.len(), 24);
    assert!(!mesh.subdivision);
    // Flat box: one outward normal per corner.
    let normals = mesh.normals.as_ref().unwrap();
    assert_eq!(normals.len(), 24);
    assert!(
        normals
            .iter()
            .all(|n| (glam::Vec3::from(*n).length() - 1.0).abs() < 1e-5)
    );
    assert!(mesh.subsets.is_empty(), "no tags, no subsets");
}

#[test]
fn transforms_become_trs_in_degrees_with_operandis_rotation_order() {
    let mut scene = box_scene("Box");
    let t = &mut scene.items[0].transform;
    t.translation = DVec3::new(1.0, 2.0, 3.0);
    t.rotation = DVec3::new(90f64.to_radians(), 0.0, 45f64.to_radians());
    t.scale = DVec3::new(2.0, 1.0, 0.5);
    let p = plan_of(&scene);
    let item = &p.prims[0];
    assert_eq!(item.translate, [1.0, 2.0, 3.0]);
    assert!((item.rotate[0] - 90.0).abs() < 1e-9 && (item.rotate[2] - 45.0).abs() < 1e-9);
    // XYZ turns X first (native v44), as USD's rotateXYZ does.
    assert_eq!(item.order, RotateOrder::Xyz);
    assert_eq!(item.scale, [2.0, 1.0, 0.5]);
    scene.items[0].transform.zxy = true;
    assert_eq!(plan_of(&scene).prims[0].order, RotateOrder::Zxy);
}

#[test]
fn children_nest_under_their_parents() {
    let mut scene = box_scene("Parent");
    let parent = scene.items[0].id;
    let child = add_box(&mut scene, "Child", Some(parent));
    add_box(&mut scene, "Grandchild", Some(child));
    let p = plan_of(&scene);
    assert_eq!(p.prims.len(), 1);
    assert_eq!(p.prims[0].children[0].name, "Child");
    assert_eq!(p.prims[0].children[0].children[0].name, "Grandchild");
    assert_eq!(p.items, 3);
}

#[test]
fn a_hidden_item_hides_only_itself_unless_it_hides_its_children() {
    let mut scene = box_scene("Parent");
    let parent = scene.items[0].id;
    add_box(&mut scene, "Child", Some(parent));
    scene.items[0].visible = false;
    let p = plan_of(&scene);
    assert!(p.prims[0].visible, "the Xform stays visible for the child");
    assert!(!p.prims[0].meshes[0].visible);
    assert!(p.prims[0].children[0].meshes[0].visible);
    scene.items[0].hides_children = true;
    let p = plan_of(&scene);
    assert!(!p.prims[0].visible, "inherited: hides the child too");
}

#[test]
fn catmull_clark_polygons_write_a_subdivision_mesh_with_creases() {
    let mut scene = box_scene("Cage");
    let mesh = &mut *scene.items[0].mesh;
    mesh.attributes.representations = vec![PolygonRepresentation::CatmullClark; 6];
    mesh.attributes.creases = vec![
        MeshCrease {
            a: 0,
            b: 1,
            weight: 0.5,
        },
        MeshCrease {
            a: 1,
            b: 2,
            weight: -0.2,
        },
    ];
    let p = plan_of(&scene);
    let [m] = p.prims[0].meshes.as_slice() else {
        panic!()
    };
    assert!(m.subdivision);
    assert_eq!(m.normals, None, "subdivision surfaces compute their own");
    assert_eq!(m.creases, vec![[0, 1]]);
    assert_eq!(m.crease_sharpness, vec![5.0]);
    assert_eq!(m.boundary, Some(Boundary::EdgeAndCorner));
    assert_eq!(m.uv_interpolation, Some(UvInterpolation::CornersPlus1));
    assert!(p.notes.iter().any(|n| n.contains("Negative edge weights")));
}

#[test]
fn a_mixed_mesh_splits_into_plain_and_subdivided_prims() {
    let mut scene = box_scene("Mixed");
    let mesh = &mut *scene.items[0].mesh;
    let mut reps = vec![PolygonRepresentation::Face; 6];
    reps[0] = PolygonRepresentation::Subdivs;
    mesh.attributes.representations = reps;
    let p = plan_of(&scene);
    let meshes = &p.prims[0].meshes;
    assert_eq!(meshes.len(), 2);
    assert_eq!(
        (meshes[0].name.as_str(), meshes[0].face_counts.len()),
        ("Mixed", 5)
    );
    assert_eq!(
        (meshes[1].name.as_str(), meshes[1].face_counts.len()),
        ("Mixed_subd", 1)
    );
    // Each prim keeps only the points it uses.
    assert_eq!(meshes[1].points.len(), 4);
    assert!(meshes[1].face_indices.iter().all(|&i| (0..4).contains(&i)));
    assert_eq!(p.polygons, 6);
    assert!(p.notes.iter().any(|n| n.contains("Catmull-Clark")));
}

#[test]
fn the_texture_map_is_st_and_other_maps_keep_their_names() {
    let mut scene = box_scene("Box");
    let mesh = &mut *scene.items[0].mesh;
    let corners = |v: f64| -> Vec<Vec<Option<DVec2>>> {
        (0..6)
            .map(|_| (0..4).map(|c| Some(DVec2::new(c as f64, v))).collect())
            .collect()
    };
    mesh.uv_maps.insert("Texture".into(), corners(0.25));
    mesh.uv_maps.insert("Atlas".into(), corners(0.75));
    let p = plan_of(&scene);
    let sets = &p.prims[0].meshes[0].uv_sets;
    assert_eq!(
        sets.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
        ["st", "Atlas"]
    );
    assert_eq!(sets[0].1.len(), 24);
    assert_eq!(sets[0].1[1], [1.0, 0.25]);
    assert_eq!(sets[1].1[1], [1.0, 0.75]);
}

#[test]
fn material_tags_become_subsets_and_an_empty_tag_is_default() {
    let mut scene = box_scene("Box");
    let mesh = &mut *scene.items[0].mesh;
    mesh.faces[0].material = "Red".into();
    mesh.faces[3].material = "Red".into();
    let p = plan_of(&scene);
    let subsets: Vec<(&str, &[i32])> = p.prims[0].meshes[0]
        .subsets
        .iter()
        .map(|s| (s.name.as_str(), s.faces.as_slice()))
        .collect();
    assert_eq!(
        subsets,
        [("Default", &[1, 2, 4, 5][..]), ("Red", &[0, 3][..])]
    );
}

#[test]
fn up_axis_follows_the_scene() {
    let scene = box_scene("Box");
    let z = IoOptions {
        up_axis: 2,
        ..Default::default()
    };
    let p = plan(&scene, &z).unwrap();
    assert_eq!((p.up_axis, p.root_matrix), ('Z', None));
    let x = IoOptions {
        up_axis: 0,
        ..Default::default()
    };
    let p = plan(&scene, &x).unwrap();
    assert_eq!(p.up_axis, 'Y');
    // The scene's up (+X) lands on the file's up (+Y): row-vector maths.
    let m = p.root_matrix.unwrap();
    let up = [m[0], m[1], m[2]];
    assert_eq!(up, [0.0, 1.0, 0.0]);
}

#[test]
fn an_empty_scene_plans_nothing() {
    let mut scene = Scene::default();
    scene.items.clear();
    let p = plan_of(&scene);
    assert_eq!((p.items, p.polygons, p.prims.len()), (0, 0, 0));
}

#[test]
fn export_refuses_scene_files_and_empty_scenes_without_writing() {
    let dir = std::env::temp_dir().join(format!("operandi-usd-refuse-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let scene = box_scene("Box");
    let opscene = dir.join("scene.opscene");
    assert!(
        crate::export(
            &opscene,
            &scene,
            operandi_core::interchange::ExportFormat::Usd
        )
        .is_err()
    );
    let mut empty = Scene::default();
    empty.items.clear();
    let usd = dir.join("empty.usd");
    let err =
        crate::export(&usd, &empty, operandi_core::interchange::ExportFormat::Usd).unwrap_err();
    assert!(err.contains("Nothing to export"), "{err}");
    assert!(!usd.exists() && !opscene.exists());
    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        0,
        "no temporaries left"
    );
}

/// Adds an Image Map layer reading scene image 7 at `path` (the default
/// layer drives Diffuse Color through the mesh UVs).
pub(crate) fn add_image_layer(scene: &mut Scene, path: &str) {
    use operandi_core::shader_tree::{NewLayer, NodeKind, TextureSource};
    scene.images.push(operandi_core::backdrop::SceneImage {
        id: 7,
        name: "grid".into(),
        path: path.into(),
        width: 4,
        height: 4,
    });
    let tex = scene.shader_tree.add(NewLayer::ImageMap, Some(4)).unwrap();
    let mut t = scene.shader_tree.get(tex).unwrap().clone();
    if let NodeKind::Texture(x) = &mut t.kind
        && let TextureSource::ImageMap { image, .. } = &mut x.source
    {
        *image = Some(7);
    }
    scene.shader_tree.replace(t).unwrap();
}

fn input<'p>(
    m: &'p crate::materials::MaterialPrim,
    name: &str,
) -> Option<&'p crate::materials::SurfaceInput> {
    m.inputs.iter().find(|i| i.name == name)
}

#[test]
fn every_mesh_gets_the_default_material() {
    let p = plan_of(&box_scene("Box"));
    assert_eq!(p.materials.len(), 1);
    let m = &p.materials[0];
    assert_eq!(m.name, "Default");
    let base = input(m, "diffuseColor").unwrap();
    assert!(base.texture.is_none());
    assert!((base.value[0] - 0.6).abs() < 1e-6, "{base:?}");
    assert!(input(m, "roughness").is_some());
    assert!(input(m, "emissiveColor").is_none(), "no glow, no emission");
    assert!(input(m, "opacity").is_none(), "opaque: left at USD's 1");
    assert_eq!(p.prims[0].meshes[0].material, Some(0));
    assert!(p.prims[0].meshes[0].subsets.is_empty());
}

#[test]
fn stored_materials_bind_to_their_tags() {
    let mut scene = box_scene("Box");
    scene.materials.insert(
        "Red".into(),
        operandi_core::Material {
            color: [1.0, 0.0, 0.0],
            ..Default::default()
        },
    );
    let mesh = &mut *scene.items[0].mesh;
    mesh.faces[0].material = "Red".into();
    let p = plan_of(&scene);
    assert_eq!(p.materials.len(), 2);
    let red = p.materials.iter().position(|m| m.name == "Red").unwrap();
    assert_eq!(
        input(&p.materials[red], "diffuseColor").unwrap().value,
        [1.0, 0.0, 0.0]
    );
    let subsets = &p.prims[0].meshes[0].subsets;
    let red_subset = subsets.iter().find(|s| s.name == "Red").unwrap();
    assert_eq!(red_subset.material, Some(red));
    let default_subset = subsets.iter().find(|s| s.name == "Default").unwrap();
    assert_ne!(default_subset.material, Some(red));
}

#[test]
fn an_image_map_becomes_a_uv_texture_on_diffuse() {
    let mut scene = box_scene("Box");
    add_image_layer(&mut scene, "/nowhere/grid.png");
    let p = plan_of(&scene);
    let base = input(&p.materials[0], "diffuseColor").unwrap();
    let tex = base.texture.as_ref().expect("a texture");
    assert_eq!(tex.image, 7);
    assert_eq!(tex.uv, "st");
    assert_eq!(tex.channel, crate::materials::Channel::Rgb);
    assert!(!tex.raw, "colour is read as sRGB");
    assert!(
        p.notes.iter().all(|n| !n.contains("plain values")),
        "{:?}",
        p.notes
    );
}
