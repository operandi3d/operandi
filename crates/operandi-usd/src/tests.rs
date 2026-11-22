//! Files written through OpenUSD and read back with it.
use crate::ffi::Stage;
use crate::plan_tests::{add_box, box_scene};
use glam::{DMat4, DVec3};
use operandi_core::interchange::ExportFormat;
use operandi_core::{MeshCrease, PolygonRepresentation, Scene};
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("operandi-usd-{name}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Exports `scene` and returns the reopened stage and its text.
fn round_trip(scene: &Scene, format: ExportFormat) -> (Stage, String) {
    let dir = scratch("rt");
    let path = dir.join(format!("out.{}", format.extension()));
    let report = crate::export(&path, scene, format).unwrap();
    assert!(report.polygons > 0);
    let entries: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
    assert_eq!(entries.len(), 1, "only the file, no temporaries");
    let stage = Stage::open(&path).unwrap();
    let text = stage.text().unwrap();
    (stage, text)
}

#[test]
fn a_box_round_trips_through_openusd() {
    let (_, text) = round_trip(&box_scene("Box"), ExportFormat::Usd);
    for expected in [
        "upAxis = \"Y\"",
        "metersPerUnit = 1",
        "defaultPrim = \"root\"",
        "def Xform \"root\"",
        "def Xform \"Box\"",
        "def Mesh \"Box\"",
        "int[] faceVertexCounts = [4, 4, 4, 4, 4, 4]",
        "uniform token subdivisionScheme = \"none\"",
        "normal3f[] normals = [",
        "interpolation = \"faceVarying\"",
        "float3[] extent = [(-0.5, -0.5, -0.5), (0.5, 0.5, 0.5)]",
        "uniform bool doubleSided = 1",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in\n{text}");
    }
}

#[test]
fn usda_is_text_and_usd_is_binary() {
    let dir = scratch("formats");
    let scene = box_scene("Box");
    for (name, format, text) in [
        ("a.usda", ExportFormat::Usda, true),
        ("b.usd", ExportFormat::Usd, false),
        ("c.usdc", ExportFormat::Usdc, false),
        ("d.usd", ExportFormat::Usda, true),
    ] {
        let path = dir.join(name);
        crate::export(&path, &scene, format).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.starts_with(b"#usda 1.0"), text, "{name}");
        assert_eq!(bytes.starts_with(b"PXR-USDC"), !text, "{name}");
    }
}

/// USD's row-vector matrix as glam's column-vector one.
fn usd_to_glam(m: [f64; 16]) -> DMat4 {
    DMat4::from_cols_array(&m)
}

#[test]
fn world_placement_matches_operandi_for_both_rotation_orders() {
    let mut scene = box_scene("Parent");
    let parent = scene.items[0].id;
    let child = add_box(&mut scene, "Child", Some(parent));
    {
        let t = &mut scene.items[0].transform;
        t.translation = DVec3::new(1.0, -2.0, 0.5);
        t.rotation = DVec3::new(0.3, -1.1, 2.0);
        t.scale = DVec3::new(1.5, 0.5, 2.0);
    }
    {
        let t = &mut scene.items[1].transform;
        t.translation = DVec3::new(0.0, 3.0, -1.0);
        t.rotation = DVec3::new(-0.7, 0.4, 1.3);
        t.zxy = true;
    }
    let (stage, _) = round_trip(&scene, ExportFormat::Usda);
    for (id, path) in [(parent, "/root/Parent"), (child, "/root/Parent/Child")] {
        let ours = scene.world_matrix(id).unwrap();
        let theirs = usd_to_glam(stage.world_matrix(path).unwrap());
        assert!(
            ours.abs_diff_eq(theirs, 1e-9),
            "{path}: Operandi {ours:?} vs USD {theirs:?}"
        );
    }
}

#[test]
fn hidden_items_and_hidden_hierarchies() {
    let mut scene = box_scene("Parent");
    let parent = scene.items[0].id;
    add_box(&mut scene, "Child", Some(parent));
    scene.items[0].visible = false;
    let (_, text) = round_trip(&scene, ExportFormat::Usda);
    // The item's own mesh is hidden, its child isn't.
    assert_eq!(
        text.matches("token visibility = \"invisible\"").count(),
        1,
        "{text}"
    );
    let hidden_mesh = text.find("def Mesh \"Parent\"").unwrap();
    let child = text.find("def Xform \"Child\"").unwrap();
    let invisible = text.find("invisible").unwrap();
    assert!(hidden_mesh < invisible && invisible < child);
}

#[test]
fn subdivision_cage_writes_scheme_and_creases() {
    let mut scene = box_scene("Cage");
    let mesh = &mut *scene.items[0].mesh;
    mesh.attributes.representations = vec![PolygonRepresentation::CatmullClark; 6];
    mesh.attributes.creases = vec![MeshCrease {
        a: 0,
        b: 3,
        weight: 1.0,
    }];
    let (_, text) = round_trip(&scene, ExportFormat::Usda);
    for expected in [
        "uniform token subdivisionScheme = \"catmullClark\"",
        "int[] creaseIndices = [0, 3]",
        "int[] creaseLengths = [2]",
        "float[] creaseSharpnesses = [10]",
        "token interpolateBoundary = \"edgeAndCorner\"",
        "token faceVaryingLinearInterpolation = \"cornersPlus1\"",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in\n{text}");
    }
    assert!(
        !text.contains("normals"),
        "subdivision surfaces store no normals"
    );
}

#[test]
fn uvs_are_indexed_primvars_and_tags_are_subsets() {
    let mut scene = box_scene("Box");
    let mesh = &mut *scene.items[0].mesh;
    let uvs = (0..6)
        .map(|_| {
            (0..4)
                .map(|c| Some(glam::DVec2::new(c as f64 * 0.5, 0.0)))
                .collect()
        })
        .collect();
    mesh.uv_maps.insert("Texture".into(), uvs);
    mesh.faces[2].material = "Red Paint".into();
    let (_, text) = round_trip(&scene, ExportFormat::Usda);
    for expected in [
        "texCoord2f[] primvars:st = [",
        "int[] primvars:st:indices = [",
        "def GeomSubset \"Red_Paint\"",
        "def GeomSubset \"Default\"",
        "uniform token familyName = \"materialBind\"",
        // customData "operandi:materialTag" nests by namespace.
        "string materialTag = \"Red Paint\"",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in\n{text}");
    }
}

#[test]
fn names_are_made_safe_and_unique() {
    let mut scene = box_scene("2 Boxes!");
    add_box(&mut scene, "2 Boxes!", None);
    let (_, text) = round_trip(&scene, ExportFormat::Usda);
    assert!(text.contains("def Xform \"_2_Boxes_\""), "{text}");
    assert!(text.contains("def Xform \"_2_Boxes_2\""), "{text}");
}

#[test]
fn z_up_scenes_say_so() {
    let options = operandi_core::interchange::IoOptions {
        up_axis: 2,
        ..Default::default()
    };
    operandi_core::interchange::set_io_options(options);
    let (_, text) = round_trip(&box_scene("Box"), ExportFormat::Usda);
    operandi_core::interchange::set_io_options(Default::default());
    assert!(text.contains("upAxis = \"Z\""), "{text}");
}

#[test]
fn a_failed_export_keeps_the_old_file() {
    let dir = scratch("keep");
    let path = dir.join("keep.usd");
    std::fs::write(&path, b"old").unwrap();
    let mut empty = Scene::default();
    empty.items.clear();
    assert!(crate::export(&path, &empty, ExportFormat::Usd).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"old");
    // A good export replaces it.
    crate::export(&path, &box_scene("Box"), ExportFormat::Usd).unwrap();
    assert_ne!(std::fs::read(&path).unwrap(), b"old");
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
}

#[test]
fn the_linked_openusd_is_the_expected_one() {
    assert!(crate::available());
    assert_eq!(crate::usd_version().as_deref(), Some("0.26.8"));
}

#[test]
fn materials_are_preview_surfaces_bound_to_meshes_and_tags() {
    let mut scene = box_scene("Box");
    scene.materials.insert(
        "Red".into(),
        operandi_core::Material {
            color: [1.0, 0.0, 0.0],
            ..Default::default()
        },
    );
    scene.items[0].mesh.faces[0].material = "Red".into();
    let (_, text) = round_trip(&scene, ExportFormat::Usda);
    for expected in [
        "def Scope \"_materials\"",
        "def Material \"Default\"",
        "def Material \"Red\"",
        "uniform token info:id = \"UsdPreviewSurface\"",
        "color3f inputs:diffuseColor = (1, 0, 0)",
        "token outputs:surface.connect = </root/_materials/Red/previewShader.outputs:surface>",
        "rel material:binding = </root/_materials/Default>",
        "rel material:binding = </root/_materials/Red>",
        "apiSchemas = [\"MaterialBindingAPI\"]",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in\n{text}");
    }
}

/// A stand-in picture file (USD only names it; nothing decodes it).
fn picture(dir: &std::path::Path) -> PathBuf {
    let path = dir.join("grid.png");
    std::fs::write(&path, b"\x89PNG\r\n\x1a\nnot really").unwrap();
    path
}

#[test]
fn image_maps_are_copied_beside_the_file_and_read_through_st() {
    let pictures = scratch("pictures");
    let mut scene = box_scene("Box");
    crate::plan_tests::add_image_layer(&mut scene, &picture(&pictures).to_string_lossy());
    let dir = scratch("textured");
    let path = dir.join("scene.usda");
    crate::export(&path, &scene, ExportFormat::Usda).unwrap();
    assert!(dir.join("scene_textures/grid.png").is_file());
    let text = std::fs::read_to_string(&path).unwrap();
    for expected in [
        "uniform token info:id = \"UsdUVTexture\"",
        "asset inputs:file = @./scene_textures/grid.png@",
        "token inputs:sourceColorSpace = \"sRGB\"",
        "token inputs:wrapS = \"repeat\"",
        "uniform token info:id = \"UsdPrimvarReader_float2\"",
        "string inputs:varname = \"st\"",
        "color3f inputs:diffuseColor.connect = </root/_materials/Default/tex_diffuseColor.outputs:rgb>",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in\n{text}");
    }
}

#[test]
fn usdz_packs_the_layer_and_its_pictures() {
    let pictures = scratch("pictures-z");
    let mut scene = box_scene("Box");
    crate::plan_tests::add_image_layer(&mut scene, &picture(&pictures).to_string_lossy());
    let dir = scratch("usdz");
    let path = dir.join("scene.usdz");
    let report = crate::export(&path, &scene, ExportFormat::Usdz).unwrap();
    assert!(
        report.notes.iter().all(|n| !n.contains("USDZ viewers")),
        "{:?}",
        report.notes
    );
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"PK\x03\x04"), "a zip archive");
    let has = |name: &[u8]| bytes.windows(name.len()).any(|w| w == name);
    // OpenUSD's packager files pictures in numbered folders ("0/grid.png")
    // and rewrites the layer's paths to match.
    assert!(has(b"scene.usdc") && has(b"/grid.png"));
    // Only the package is left: no build folder, no temporaries.
    let left: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, vec![std::ffi::OsString::from("scene.usdz")]);
    // OpenUSD opens it.
    let text = Stage::open(&path).unwrap().text().unwrap();
    assert!(text.contains("def Mesh \"Box\""));
}

#[test]
fn a_missing_picture_is_noted_and_the_export_still_works() {
    let mut scene = box_scene("Box");
    crate::plan_tests::add_image_layer(&mut scene, "/nowhere/at/all/grid.png");
    let dir = scratch("missing");
    let path = dir.join("scene.usda");
    let report = crate::export(&path, &scene, ExportFormat::Usda).unwrap();
    assert!(
        report.notes.iter().any(|n| n.contains("Missing image")),
        "{:?}",
        report.notes
    );
    assert!(!dir.join("scene_textures").exists());
}

#[test]
fn cameras_and_lights_are_usd_cameras_and_lights_in_their_items() {
    use operandi_core::camera_light::{CameraSettings, LightKind, LightSettings};
    let mut scene = box_scene("Box");
    let place = operandi_core::scene::Transform {
        translation: DVec3::new(0.0, 1.0, 5.0),
        ..Default::default()
    };
    scene
        .add_camera(CameraSettings::default(), place.clone())
        .unwrap();
    let mut spot = LightSettings::new(LightKind::Spot);
    spot.intensity = 1.0;
    spot.dissolve = 0.0;
    scene.add_light(spot, place.clone()).unwrap();
    scene
        .add_light(LightSettings::new(LightKind::Area), place)
        .unwrap();
    let names: Vec<String> = scene.items.iter().map(|i| i.name.clone()).collect();
    let (stage, text) = round_trip(&scene, ExportFormat::Usda);
    for expected in [
        "def Camera ",
        "float focalLength = 0.5",
        "float horizontalAperture = 0.36",
        "def SphereLight ",
        "float inputs:shaping:cone:angle = 22.5",
        "float inputs:intensity = 4",
        "def RectLight ",
        "bool inputs:normalize = 1",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in\n{text}");
    }
    // The camera sits where its item does.
    let camera = format!("/root/{0}/{0}", names[1].replace(' ', "_"));
    let m = stage
        .world_matrix(&camera)
        .unwrap_or_else(|| panic!("{camera}\n{text}"));
    assert_eq!([m[12], m[13], m[14]], [0.0, 1.0, 5.0]);
}

#[test]
fn the_environment_becomes_a_dome_with_a_baked_picture() {
    let mut scene = box_scene("Box");
    scene.add_environment().unwrap();
    let dir = scratch("dome");
    let path = dir.join("scene.usda");
    crate::export(&path, &scene, ExportFormat::Usda).unwrap();
    let hdr = std::fs::read(dir.join("scene_textures/environment.hdr")).unwrap();
    assert!(hdr.starts_with(b"#?RADIANCE"));
    assert!(hdr.len() > 256 * 128 * 4);
    let text = std::fs::read_to_string(&path).unwrap();
    for expected in [
        "def Xform \"Environment\"",
        "def DomeLight \"Dome\"",
        "asset inputs:texture:file = @./scene_textures/environment.hdr@",
        "token inputs:texture:format = \"latlong\"",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in\n{text}");
    }
}

// --- Import (docs/BEHAVIOR-USD-IMPORT.md, U1 and U2) ---

mod import {
    use super::scratch;
    use glam::{DMat4, DQuat, DVec3};
    use operandi_core::interchange::{Imported, UsdImportOptions, UsdMaterials};
    use operandi_core::shader_tree::{NodeKind, TagMask, TextureEffect, TextureSource};
    use operandi_core::subdivision::BoundaryRule;
    use operandi_core::{Item, MeshSlot, PolygonRepresentation, Scene};
    use std::path::{Path, PathBuf};

    /// The Blender-made USD-T1 scene (fixtures/usd/make_usd_t1.py).
    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/usd/usd_t1")
            .join(name)
    }

    fn import(path: &Path, options: &UsdImportOptions) -> Imported {
        crate::import(path, options).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    /// Options that unpack .usdz pictures into a scratch folder.
    fn options(name: &str) -> UsdImportOptions {
        UsdImportOptions {
            texture_folder: Some(scratch(name).join("textures")),
            ..Default::default()
        }
    }

    fn item<'a>(scene: &'a Scene, name: &str) -> &'a Item {
        scene
            .items
            .iter()
            .find(|i| i.name == name)
            .unwrap_or_else(|| panic!("no item {name}"))
    }

    fn parent_name(scene: &Scene, item: &Item) -> Option<String> {
        item.parent
            .and_then(|p| scene.items.iter().find(|i| i.id == p))
            .map(|i| i.name.clone())
    }

    /// Blender's Z-up world point as Operandi's Y-up one.
    fn y_up(p: DVec3) -> DVec3 {
        DVec3::new(p.x, p.z, -p.y)
    }

    /// Everything USD-T2 and USD-T3 compare: per item its name, parent,
    /// world placement, polygon sizes and tags; and the Material Groups.
    fn summary(scene: &Scene) -> (Vec<String>, Vec<String>) {
        let mut items: Vec<String> = scene
            .items
            .iter()
            .filter(|i| !i.is_texture_item())
            .map(|i| {
                let w = scene.world_matrix(i.id).unwrap();
                let p = w.transform_point3(DVec3::ZERO);
                let mut sizes: Vec<usize> = i.mesh.faces.iter().map(|f| f.vertices.len()).collect();
                sizes.sort();
                let tags: std::collections::BTreeSet<&str> =
                    i.mesh.faces.iter().map(|f| f.material.as_str()).collect();
                format!(
                    "{} < {:?} at ({:.4}, {:.4}, {:.4}) polygons {sizes:?} tags {tags:?}",
                    i.name,
                    parent_name(scene, i),
                    p.x,
                    p.y,
                    p.z
                )
            })
            .collect();
        items.sort();
        let mut groups: Vec<String> = scene
            .shader_tree
            .nodes
            .iter()
            .filter_map(|n| match &n.kind {
                NodeKind::Group(g) => match &g.tag {
                    TagMask::Name(t) => Some(format!("{} for {t}", n.name)),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        groups.sort();
        (items, groups)
    }

    /// The texture layers (effect, picture file name, gamma) in the group
    /// for `tag`.
    fn layers(scene: &Scene, tag: &str) -> Vec<(TextureEffect, String, f64)> {
        let group = scene
            .shader_tree
            .nodes
            .iter()
            .find(|n| matches!(&n.kind, NodeKind::Group(g) if g.tag == TagMask::Name(tag.into())))
            .unwrap_or_else(|| panic!("no group for {tag}"));
        let mut out: Vec<_> = scene
            .shader_tree
            .nodes
            .iter()
            .filter(|n| n.parent == Some(group.id))
            .filter_map(|n| match &n.kind {
                NodeKind::Texture(t) => match &t.source {
                    TextureSource::ImageMap {
                        image: Some(id),
                        settings,
                    } => {
                        let image = scene.images.iter().find(|i| i.id == *id).unwrap();
                        let file = Path::new(&image.path)
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .into_owned();
                        Some((t.effect, file, settings.gamma))
                    }
                    _ => None,
                },
                _ => None,
            })
            .collect();
        out.sort_by_key(|(e, f, _)| (format!("{e:?}"), f.clone()));
        out
    }

    fn material<'a>(
        scene: &'a Scene,
        tag: &str,
    ) -> &'a operandi_core::shader_tree::MaterialChannels {
        let group = scene
            .shader_tree
            .nodes
            .iter()
            .find(|n| matches!(&n.kind, NodeKind::Group(g) if g.tag == TagMask::Name(tag.into())))
            .unwrap();
        scene
            .shader_tree
            .nodes
            .iter()
            .filter(|n| n.parent == Some(group.id))
            .find_map(|n| match &n.kind {
                NodeKind::Material(m) => Some(&**m),
                _ => None,
            })
            .unwrap()
    }

    /// USD-T1: hierarchy, local transforms, quads, two UV maps, colours,
    /// Sub-D with creases, ten chair copies and the three materials with
    /// their pictures in the right colour space.
    #[test]
    fn usd_t1_the_blender_scene_imports() {
        let imported = import(&fixture("usd_t1.usdc"), &UsdImportOptions::default());
        let scene = &imported.scene;
        let notes = imported.notes.join("\n");
        assert!(
            notes
                .starts_with("31 item(s), 512 polygon(s): 480 quad(s), 0 n-gon(s), 32 triangle(s)"),
            "{notes}"
        );
        assert!(
            notes.contains("Materials: 3 MaterialX, 0 Preview Surface, 0 as plain polygon tags"),
            "{notes}"
        );
        assert!(
            notes.contains(
                "Instances: 10 instance(s) in the file; 9 item(s) came in as Mesh Instances"
            ),
            "{notes}"
        );
        assert!(
            notes.contains(
                "Cameras: 1 of 1 imported; lights: 3 of 3; dome lights: 1 of 1 (as the environment)"
            ),
            "{notes}"
        );
        // Three levels, local transforms kept (Z-up turned to Y-up).
        let (base, arm, hand) = (item(scene, "Base"), item(scene, "Arm"), item(scene, "Hand"));
        assert_eq!(parent_name(scene, arm).as_deref(), Some("Base"));
        assert_eq!(parent_name(scene, hand).as_deref(), Some("Arm"));
        assert!(base.parent.is_none() && base.is_group_locator());
        assert!(
            base.transform
                .translation
                .abs_diff_eq(DVec3::new(3., 0., 0.), 1e-6)
        );
        // Blender's 30° about Z (up) is 30° about the scene's Y.
        assert!(
            base.transform
                .rotation_quat()
                .abs_diff_eq(DQuat::from_rotation_y(30f64.to_radians()), 1e-6),
            "{:?}",
            base.transform
        );
        assert!(
            arm.transform
                .translation
                .abs_diff_eq(DVec3::new(0., 0., -2.), 1e-6)
        );
        assert!(
            hand.transform
                .scale
                .abs_diff_eq(DVec3::new(1., 1., 2.), 1e-6)
        );
        let blender_hand = DMat4::from_translation(DVec3::new(3., 0., 0.))
            * DMat4::from_rotation_z(30f64.to_radians())
            * DMat4::from_translation(DVec3::new(0., 2., 0.))
            * DMat4::from_rotation_x(45f64.to_radians())
            * DMat4::from_translation(DVec3::new(0., 0., 1.))
            * DMat4::from_scale(DVec3::new(1., 2., 1.));
        // A corner of the Hand's mesh lands where Blender puts it.
        let corner = DVec3::new(0.25, 0.25, 0.25);
        let world = scene.world_matrix(hand.id).unwrap();
        assert!(
            world
                .transform_point3(y_up(corner))
                .abs_diff_eq(y_up(blender_hand.transform_point3(corner)), 1e-5)
        );
        // Quads, two UV maps (st as Texture), the colour map.
        let cube = item(scene, "UVCube");
        assert!(cube.mesh.faces.iter().all(|f| f.vertices.len() == 4));
        let uv: Vec<&String> = cube.mesh.uv_maps.keys().collect();
        assert_eq!(uv, ["Texture", "UVMap2"]);
        let col = &cube.mesh.attributes.corner_colors["Col"];
        assert_eq!(col[0][0], Some(glam::DVec4::new(1., 0., 0., 1.)));
        assert!(cube.mesh.faces.iter().all(|f| f.material == "Painted"));
        assert!(cube.mesh.faces.iter().all(|f| !f.smooth), "flat normals");
        // Sub-D: Catmull-Clark polygons, four creases at full weight.
        let subd = item(scene, "SubD");
        assert!(
            subd.mesh
                .attributes
                .representations
                .iter()
                .all(|r| *r == PolygonRepresentation::CatmullClark)
        );
        assert_eq!(subd.mesh.attributes.representations.len(), 6);
        assert_eq!(subd.mesh.attributes.creases.len(), 4);
        assert!(subd.mesh.attributes.creases.iter().all(|c| c.weight == 1.0));
        let settings = subd.subdivision.as_ref().unwrap();
        assert_eq!(settings.boundary_rule, BoundaryRule::Smooth);
        assert_eq!(settings.uv_boundary_rule, BoundaryRule::CreaseEdges);
        // Ten chairs, each under its own instance item, sharing one mesh.
        let chairs: Vec<&Item> = scene.items.iter().filter(|i| i.name == "Chair_0").collect();
        assert_eq!(chairs.len(), 10);
        assert!(
            chairs
                .iter()
                .all(|c| MeshSlot::same(&c.mesh, &chairs[0].mesh))
        );
        assert!(chairs.iter().all(|c| c.mesh.faces.len() == 36));
        // U5: the first chair is the source, the other nine its instances.
        let sources: Vec<Option<u64>> = chairs.iter().map(|c| c.instance_of).collect();
        assert_eq!(sources[0], None);
        assert!(sources[1..].iter().all(|s| *s == Some(chairs[0].id)));
        let first = item(scene, "ChairInstance0");
        assert!(
            first
                .transform
                .translation
                .abs_diff_eq(DVec3::new(-6., 0., 4.), 1e-6)
        );
        // Materials: the textured one has its three pictures, colour
        // maps read as sRGB and the data maps as numbers (gamma 1, no
        // approximation notes).
        assert_eq!(
            layers(scene, "Painted"),
            [
                (TextureEffect::DiffuseColor, "base_color.png".into(), 1.0),
                (TextureEffect::Normal, "normal.png".into(), 1.0),
                (TextureEffect::Roughness, "roughness.png".into(), 1.0),
            ]
        );
        assert!(
            !notes.contains("approximated") && !notes.contains("read as is"),
            "{notes}"
        );
        let glow = material(scene, "Glow");
        assert!(glow.luminous_intensity > 0.0, "{glow:?}");
        assert!(glow.luminous_color[0] > glow.luminous_color[2]);
        let glass = material(scene, "Glass");
        assert!((glass.transparent_amount - 1.0).abs() < 1e-6, "{glass:?}");
        assert!((glass.refractive_index - 1.45).abs() < 1e-3, "{glass:?}");
        assert_eq!(
            summary(scene).1,
            ["Glass for Glass", "Glow for Glow", "Painted for Painted"]
        );
    }

    /// USD-T1, U3: the 35 mm camera, the three lights with the power
    /// Blender gave Cycles, and the HDRI dome as the environment, pole up.
    #[test]
    fn usd_t1_cameras_lights_and_the_dome() {
        use operandi_core::camera_light::LightKind;
        use std::f64::consts::PI;
        let imported = import(&fixture("usd_t1.usdc"), &UsdImportOptions::default());
        let scene = &imported.scene;
        let camera = item(scene, "Camera");
        let c = camera.camera.as_ref().unwrap();
        assert!((c.focal_length - 0.035).abs() < 1e-6);
        assert!((c.film_width - 0.036).abs() < 1e-6);
        assert!(c.clipping && (c.clip_near - 0.1).abs() < 1e-6);
        // Blender's camera at (8, -8, 5), Z up, is at (8, 5, 8) Y up.
        assert!(
            camera
                .transform
                .translation
                .abs_diff_eq(DVec3::new(8., 5., 8.), 1e-5)
        );
        // It looks where Blender's does: from (8, -8, 5) toward the
        // origin (rotateXYZ 65, 0, 45 turns its -Z down and round).
        let blender = glam::DQuat::from_rotation_z(45f64.to_radians())
            * glam::DQuat::from_rotation_x(65f64.to_radians())
            * -DVec3::Z;
        let forward = scene
            .world_matrix(camera.id)
            .unwrap()
            .transform_vector3(-DVec3::Z);
        assert!(forward.abs_diff_eq(y_up(blender), 1e-5), "{forward}");
        let light = |name: &str| item(scene, name).light.clone().unwrap();
        let point = light("PointLight");
        assert_eq!(point.kind, LightKind::Point);
        assert!((4.0 * PI * point.intensity - 100.0).abs() < 1e-3, "100 W");
        let area = light("AreaLight");
        assert_eq!(area.kind, LightKind::Area);
        assert!((area.width - 2.0).abs() < 1e-9);
        assert!(
            (area.intensity * area.width * area.height * PI - 200.0).abs() < 1e-3,
            "200 W"
        );
        let sun = light("SunLight");
        assert_eq!(sun.kind, LightKind::Directional);
        let strength =
            sun.intensity * operandi_core::camera_light_defaults::CYCLES_SUN_STRENGTH_PER_INTENSITY;
        assert!((strength - 3.0).abs() < 1e-5);
        // The dome: sky.hdr in the environment, its +Y pole (the file's
        // +Z after Blender's rotateXYZ 90, 0, 90) along the scene's +Y.
        let env = scene.environments().next().unwrap();
        let layer = scene.environment_image_layer(env.id).unwrap();
        let NodeKind::Texture(t) = &layer.kind else {
            panic!()
        };
        let TextureSource::ImageMap {
            image: Some(id), ..
        } = &t.source
        else {
            panic!()
        };
        let image = scene.images.iter().find(|i| i.id == *id).unwrap();
        assert!(image.path.ends_with("sky.hdr"), "{}", image.path);
        let locator = scene
            .items
            .iter()
            .find(|i| Some(i.id) == t.locator)
            .unwrap();
        let pole = locator.transform.rotation_quat() * DVec3::Y;
        assert!(pole.abs_diff_eq(DVec3::Y, 1e-6), "{pole}");
    }

    /// USD-T4: the Noise → Color Ramp material only the graphs sidecar
    /// carries becomes a Node Material layer in its Material Group; the
    /// plain one keeps its MaterialX import; off, or with no sidecar,
    /// nothing changes.
    #[test]
    fn usd_t4_the_graphs_sidecar_brings_node_materials() {
        let t4 = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/usd/usd_t4");
        let imported = import(&t4.join("usd_t4.usdc"), &UsdImportOptions::default());
        let scene = &imported.scene;
        let notes = imported.notes.join("\n");
        assert!(
            notes.contains("Node graphs from usd_t4_graphs.json: Marble"),
            "{notes}"
        );
        assert!(
            notes.contains(
                "Node graphs in usd_t4_graphs.json that match no material of the file: Paint"
            ) || notes.contains("Node graphs from usd_t4_graphs.json: Marble, Paint")
                || notes.contains("Node graphs from usd_t4_graphs.json: Paint, Marble"),
            "{notes}"
        );
        let group_children = |tag: &str| -> Vec<String> {
            let group = scene
                .shader_tree
                .nodes
                .iter()
                .find(
                    |n| matches!(&n.kind, NodeKind::Group(g) if g.tag == TagMask::Name(tag.into())),
                )
                .unwrap_or_else(|| panic!("no group for {tag}"));
            scene
                .shader_tree
                .nodes
                .iter()
                .filter(|n| n.parent == Some(group.id))
                .map(|n| match &n.kind {
                    NodeKind::NodeMaterial(_) => format!("node:{}", n.name),
                    NodeKind::Material(_) => "material".into(),
                    _ => "other".into(),
                })
                .collect()
        };
        assert_eq!(group_children("Marble"), ["node:Marble Node Material"]);
        let marble = scene
            .shader_tree
            .nodes
            .iter()
            .find_map(|n| match &n.kind {
                NodeKind::NodeMaterial(m) if n.name == "Marble Node Material" => m.graph,
                _ => None,
            })
            .and_then(|g| scene.shader_tree.node_graphs.get(g))
            .unwrap();
        let types: Vec<&str> = marble
            .material
            .nodes
            .iter()
            .map(|n| n.idname.as_str())
            .collect();
        assert!(
            types.contains(&"ShaderNodeTexNoise") && types.contains(&"ShaderNodeValToRGB"),
            "{types:?}"
        );
        // Off: the flattened materials stay.
        let off = import(
            &t4.join("usd_t4.usdc"),
            &UsdImportOptions {
                graphs_sidecar: false,
                ..Default::default()
            },
        );
        assert!(off.scene.shader_tree.node_graphs.is_empty());
        // A picked sidecar that's missing is reported, the import goes on.
        let picked = import(
            &t4.join("usd_t4.usdc"),
            &UsdImportOptions {
                graphs_file: Some(t4.join("nowhere.json")),
                ..Default::default()
            },
        );
        assert!(
            picked
                .notes
                .iter()
                .any(|n| n.starts_with("Graphs sidecar nowhere.json not read"))
        );
    }

    /// The Preview Surface option reads the other network.
    #[test]
    fn the_preview_surface_option_reads_usd_preview_surface() {
        let imported = import(
            &fixture("usd_t1.usdc"),
            &UsdImportOptions {
                materials: UsdMaterials::PreviewSurface,
                ..Default::default()
            },
        );
        let notes = imported.notes.join("\n");
        assert!(
            notes.contains("Materials: 0 MaterialX, 3 Preview Surface"),
            "{notes}"
        );
        assert_eq!(layers(&imported.scene, "Painted").len(), 3);
        // None: tags stay, no groups.
        let none = import(
            &fixture("usd_t1.usdc"),
            &UsdImportOptions {
                materials: UsdMaterials::None,
                ..Default::default()
            },
        );
        assert!(summary(&none.scene).1.is_empty());
        assert!(
            item(&none.scene, "UVCube")
                .mesh
                .faces
                .iter()
                .all(|f| f.material == "Painted")
        );
    }

    /// USD-T2: the .usdz gives the same scene, its pictures unpacked.
    #[test]
    fn usd_t2_the_usdz_gives_the_same_scene() {
        let usdc = import(&fixture("usd_t1.usdc"), &UsdImportOptions::default());
        let opts = options("t2");
        let usdz = import(&fixture("usd_t1.usdz"), &opts);
        assert_eq!(summary(&usdz.scene), summary(&usdc.scene));
        assert_eq!(
            layers(&usdz.scene, "Painted"),
            layers(&usdc.scene, "Painted")
        );
        let folder = opts.texture_folder.unwrap();
        for image in &usdz.scene.images {
            assert!(
                Path::new(&image.path).starts_with(&folder),
                "{}",
                image.path
            );
            assert!(Path::new(&image.path).is_file(), "{}", image.path);
        }
        assert_eq!(
            usdz.scene.images.len(),
            4,
            "3 material pictures and the dome's"
        );
        assert!(
            usdz.notes
                .iter()
                .any(|n| n.contains("unpacked from the .usdz"))
        );
    }

    /// USD-T3: Operandi's own USD export reads back with the same items,
    /// tags and materials.
    #[test]
    fn usd_t3_an_operandi_export_reads_back_the_same() {
        let first = import(&fixture("usd_t1.usdc"), &UsdImportOptions::default());
        let dir = scratch("t3");
        let out = dir.join("again.usdc");
        crate::export(
            &out,
            &first.scene,
            operandi_core::interchange::ExportFormat::Usdc,
        )
        .unwrap();
        let again = import(&out, &UsdImportOptions::default());
        let (items, groups) = summary(&first.scene);
        let (items2, mut groups2) = summary(&again.scene);
        assert_eq!(items2, items);
        // The export also writes the scene's Default material, which comes
        // back as a group for the default tag.
        groups2.retain(|g| g != "Default for Default");
        assert_eq!(groups2, groups);
        // The export writes Preview Surface with all three pictures (the
        // roughness map used to be flattened, USD-IQ14).
        let back: Vec<TextureEffect> = layers(&again.scene, "Painted")
            .into_iter()
            .map(|(e, _, _)| e)
            .collect();
        assert_eq!(
            back,
            [
                TextureEffect::DiffuseColor,
                TextureEffect::Normal,
                TextureEffect::Roughness
            ]
        );
    }

    /// U6: the export carries each material's MaterialX network beside
    /// its Preview Surface, and the round trip reads it back as MaterialX
    /// with the same items, polygons, cameras, lights and bounds.
    #[test]
    fn u6_exports_materialx_and_round_trips() {
        let first = import(&fixture("usd_t1.usdc"), &UsdImportOptions::default());
        let dir = scratch("u6");
        let out = dir.join("again.usda");
        crate::export(
            &out,
            &first.scene,
            operandi_core::interchange::ExportFormat::Usda,
        )
        .unwrap();
        let text = std::fs::read_to_string(&out).unwrap();
        for expected in [
            "token outputs:mtlx:surface.connect",
            "uniform token info:id = \"ND_open_pbr_surface_surfaceshader\"",
            "uniform token info:id = \"ND_image_color3\"",
            "token outputs:surface.connect",
            "uniform token info:id = \"UsdPreviewSurface\"",
        ] {
            assert!(text.contains(expected), "missing {expected}");
        }
        let again = import(&out, &UsdImportOptions::default());
        let notes = again.notes.join("\n");
        // The three groups come back through MaterialX; the scene's base
        // material (no group) is written as Preview Surface only.
        assert!(
            notes.contains("Materials: 3 MaterialX, 1 Preview Surface"),
            "{notes}"
        );
        let names = |scene: &Scene| -> Vec<(TextureEffect, String)> {
            layers(scene, "Painted")
                .into_iter()
                .map(|(e, f, _)| (e, f))
                .collect()
        };
        assert_eq!(names(&again.scene), names(&first.scene));
        assert_eq!(
            super::round_trip_summary(&again.scene),
            super::round_trip_summary(&first.scene)
        );
    }

    /// USD-T5: a missing picture still imports, keeps its layer (the
    /// missing-file look) and is reported.
    #[test]
    fn usd_t5_a_missing_picture_is_reported_and_kept() {
        let dir = scratch("t5");
        let copy = dir.join("alone.usdc");
        std::fs::copy(fixture("usd_t1.usdc"), &copy).unwrap();
        let imported = import(&copy, &UsdImportOptions::default());
        let notes = imported.notes.join("\n");
        assert!(
            notes.contains(
                "Missing picture(s), shown with the missing-file look: base_color.png, normal.png, roughness.png"
            ),
            "{notes}"
        );
        assert_eq!(layers(&imported.scene, "Painted").len(), 3);
        for image in &imported.scene.images {
            assert!(
                image
                    .path
                    .starts_with(&dir.join("textures").to_string_lossy().into_owned())
            );
            assert!(!Path::new(&image.path).exists());
        }
    }

    fn import_text(name: &str, text: &str) -> Imported {
        let dir = scratch(name);
        let path = dir.join(format!("{name}.usda"));
        std::fs::write(&path, text).unwrap();
        import(&path, &UsdImportOptions::default())
    }

    /// n-gons and left-handed winding, centimetres, hidden prims, guide
    /// meshes, corner sharpness and a PointInstancer, through OpenUSD.
    #[test]
    fn usda_details_come_across() {
        let imported = import_text(
            "details",
            r#"#usda 1.0
(
    upAxis = "Y"
    metersPerUnit = 0.01
)
def Xform "World"
{
    def Mesh "Pentagon"
    {
        int[] faceVertexCounts = [5]
        int[] faceVertexIndices = [0, 1, 2, 3, 4]
        point3f[] points = [(0, 0, 0), (100, 0, 0), (150, 100, 0), (50, 150, 0), (-50, 100, 0)]
        uniform token orientation = "leftHanded"
        uniform token subdivisionScheme = "none"
        int[] cornerIndices = [0]
        float[] cornerSharpnesses = [2]
    }
    def Xform "Hidden"
    {
        token visibility = "invisible"
        double3 xformOp:translate = (200, 0, 0)
        uniform token[] xformOpOrder = ["xformOp:translate"]
        def Mesh "Tri"
        {
            int[] faceVertexCounts = [3]
            int[] faceVertexIndices = [0, 1, 2]
            point3f[] points = [(0, 0, 0), (1, 0, 0), (0, 1, 0)]
            uniform token subdivisionScheme = "none"
        }
    }
    def Mesh "Guide"
    {
        uniform token purpose = "guide"
        int[] faceVertexCounts = [3]
        int[] faceVertexIndices = [0, 1, 2]
        point3f[] points = [(0, 0, 0), (1, 0, 0), (0, 1, 0)]
    }
    def PointInstancer "Scatter"
    {
        rel prototypes = </World/Scatter/Protos/Tri>
        int[] protoIndices = [0, 0, 0]
        point3f[] positions = [(0, 0, 0), (100, 0, 0), (200, 0, 0)]
        def Scope "Protos"
        {
            def Mesh "Tri"
            {
                int[] faceVertexCounts = [3]
                int[] faceVertexIndices = [0, 1, 2]
                point3f[] points = [(0, 0, 0), (10, 0, 0), (0, 10, 0)]
                uniform token subdivisionScheme = "none"
            }
        }
    }
}
"#,
        );
        let scene = &imported.scene;
        let notes = imported.notes.join("\n");
        // "World" has a transform of its own? No: it is the one top prim,
        // so its children come in at the top.
        let pentagon = item(scene, "Pentagon");
        assert!(pentagon.parent.is_none());
        assert_eq!(pentagon.mesh.faces.len(), 1);
        // Left-handed: the winding reverses, keeping the first corner.
        assert_eq!(pentagon.mesh.faces[0].vertices, [0, 4, 3, 2, 1]);
        // Centimetres: 100 cm is 1 m.
        assert!(pentagon.mesh.vertices[1].abs_diff_eq(DVec3::new(1., 0., 0.), 1e-9));
        assert!(notes.contains("1 corner sharpness(es) left out"), "{notes}");
        // Hidden: the item and what's below it.
        let hidden = item(scene, "Hidden");
        assert!(!hidden.visible && hidden.hides_children);
        assert!(
            hidden
                .transform
                .translation
                .abs_diff_eq(DVec3::new(2., 0., 0.), 1e-9)
        );
        assert!(scene.items.iter().all(|i| i.name != "Guide"));
        assert!(
            notes.contains("Guide and proxy meshes left out: Guide (guide)"),
            "{notes}"
        );
        // The PointInstancer: three copies sharing one mesh.
        let scatter = item(scene, "Scatter");
        let copies: Vec<&Item> = scene
            .items
            .iter()
            .filter(|i| i.parent == Some(scatter.id))
            .collect();
        assert_eq!(copies.len(), 3);
        assert!(
            copies
                .iter()
                .all(|c| MeshSlot::same(&c.mesh, &copies[0].mesh))
        );
        assert!(
            copies[2]
                .transform
                .translation
                .abs_diff_eq(DVec3::new(2., 0., 0.), 1e-9)
        );
        assert!(
            notes.contains(
                "Instances: 3 instance(s) in the file; 2 item(s) came in as Mesh Instances"
            ),
            "{notes}"
        );
    }

    /// The chess-set regression (fixtures/usd/crease_ring.usda): creases
    /// listed last point to first used to fail the mesh rules, and the
    /// piece arrived with no polygons.
    #[test]
    fn a_crease_ring_running_backwards_keeps_its_polygons() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/usd/crease_ring.usda");
        let imported = import(&path, &UsdImportOptions::default());
        let piece = item(&imported.scene, "Piece");
        assert_eq!(piece.mesh.faces.len(), 5, "{:?}", imported.notes);
        assert_eq!(piece.mesh.attributes.creases.len(), 8);
        assert!(piece.mesh.attributes.creases.iter().all(|c| c.a < c.b));
        // 0-1 is listed twice (sharpness 2 and 4): the sharper wins.
        let first = &piece.mesh.attributes.creases[0];
        assert_eq!((first.a, first.b, first.weight), (0, 1, 0.4));
        // Unapplied Z scale arrives as Y scale; the piece stands 5 cm tall.
        assert!(
            piece
                .transform
                .scale
                .abs_diff_eq(DVec3::new(1., 0.005, 1.), 1e-6)
        );
        let world = imported.scene.world_matrix(piece.id).unwrap();
        let top = world.transform_point3(piece.mesh.vertices[4]);
        assert!((top.y - 0.05).abs() < 1e-6, "{top}");
        assert!(
            !imported.notes.iter().any(|n| n.contains("crease")),
            "{:?}",
            imported.notes
        );
    }

    /// USD-IQ21: the camera the stage's RenderSettings names
    /// (fixtures/usd/render_camera.usda) becomes the render camera; a
    /// stage that names none leaves it unchosen.
    #[test]
    fn the_render_settings_camera_becomes_the_render_camera() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/usd/render_camera.usda");
        let imported = import(&path, &UsdImportOptions::default());
        let side = item(&imported.scene, "Side");
        assert!(side.camera.is_some());
        assert_eq!(imported.scene.render_camera, Some(side.id));
        assert_eq!(imported.scene.render_camera(), Some(side.id));

        let dir = scratch("no-render-camera");
        let plain = dir.join("plain.usda");
        let text = std::fs::read_to_string(&path).unwrap();
        let cut = text.find("def Scope \"Render\"").unwrap();
        std::fs::write(
            &plain,
            text[..cut].replace("    renderSettingsPrimPath = \"/Render/Settings\"\n", ""),
        )
        .unwrap();
        let imported = import(&plain, &UsdImportOptions::default());
        assert_eq!(imported.scene.render_camera, None);
        // The first camera renders, as before.
        assert_eq!(
            imported.scene.render_camera(),
            Some(item(&imported.scene, "Front").id)
        );
    }

    #[test]
    fn a_file_without_polygons_is_refused() {
        let dir = scratch("empty");
        let path = dir.join("empty.usda");
        std::fs::write(&path, "#usda 1.0\ndef Xform \"Nothing\"\n{\n}\n").unwrap();
        let e = crate::import(&path, &UsdImportOptions::default()).unwrap_err();
        assert!(e.contains("no polygons"), "{e}");
        std::fs::write(&path, "not usd at all").unwrap();
        assert!(crate::import(&path, &UsdImportOptions::default()).is_err());
    }
}

/// Imports the USD file `OPERANDI_USD_FILE` names and prints the report
/// and each mesh item's polygon count, for checking a real scene:
/// `OPERANDI_USD_FILE=/path/scene.usdc cargo test -p operandi-usd --features usd import_a_named_file -- --ignored --nocapture`
#[test]
#[ignore = "needs OPERANDI_USD_FILE"]
fn import_a_named_file() {
    let path = std::path::PathBuf::from(std::env::var("OPERANDI_USD_FILE").unwrap());
    let imported = crate::import(&path, &Default::default()).unwrap();
    for n in &imported.notes {
        println!("note: {n}");
    }
    for i in imported.scene.items.iter().filter(|i| i.is_mesh()) {
        // World bounds, to compare with OpenUSD's (Y up).
        let w = imported.scene.world_matrix(i.id).unwrap();
        let (mut lo, mut hi) = (glam::DVec3::splat(f64::MAX), glam::DVec3::splat(f64::MIN));
        for v in &i.mesh.vertices {
            let p = w.transform_point3(*v);
            lo = lo.min(p);
            hi = hi.max(p);
        }
        println!(
            "{}: {} polygons, {} creases, scale {:?}, world box {lo:.4} .. {hi:.4}",
            i.name,
            i.mesh.faces.len(),
            i.mesh.attributes.creases.len(),
            i.transform.scale
        );
    }
}

/// U6: imports `OPERANDI_USD_FILE`, exports it to USD, imports that, and
/// compares item and polygon counts and world bounds:
/// `OPERANDI_USD_FILE=<scene.usdc> cargo test -p operandi-usd --features usd round_trip_a_named_file -- --ignored --nocapture`
#[test]
#[ignore = "needs OPERANDI_USD_FILE"]
fn round_trip_a_named_file() {
    let path = std::path::PathBuf::from(std::env::var("OPERANDI_USD_FILE").unwrap());
    let t = std::time::Instant::now();
    let first = crate::import(&path, &Default::default()).unwrap();
    let imported = t.elapsed().as_secs_f64();
    for n in &first.notes {
        println!("note: {n}");
    }
    let dir = scratch("named-rt");
    let out = dir.join("round_trip.usdc");
    let t = std::time::Instant::now();
    let report = crate::export(&out, &first.scene, ExportFormat::Usdc).unwrap();
    let exported = t.elapsed().as_secs_f64();
    println!("export notes: {:?}", report.notes);
    let t = std::time::Instant::now();
    let again = crate::import(&out, &Default::default()).unwrap();
    let reimported = t.elapsed().as_secs_f64();
    println!(
        "import {imported:.1} s, export {exported:.1} s ({:.0} MB), re-import {reimported:.1} s",
        std::fs::metadata(&out).map_or(0., |m| m.len() as f64 / 1e6)
    );
    let (a, b) = (
        round_trip_summary(&first.scene),
        round_trip_summary(&again.scene),
    );
    println!("first: {a:?}\nagain: {b:?}");
    assert_eq!(a, b);
}

/// Sweeps a folder of sample USD files (default
/// ~/Developer/Operandi-Content/usd-samples, or `OPERANDI_USD_SAMPLES`):
/// imports each, exports it, imports that and compares item, polygon,
/// camera and light counts and the world box, then prints a pass/fail
/// list. Samples are other people's files and stay outside the repo:
/// `cargo test --locked --workspace --features operandi-app/usd round_trip_the_samples_folder -- --ignored --nocapture`
#[test]
#[ignore = "needs a folder of sample USD files"]
fn round_trip_the_samples_folder() {
    let root = std::env::var_os("OPERANDI_USD_SAMPLES")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap())
                .join("Developer/Operandi-Content/usd-samples")
        });
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("usd" | "usda" | "usdc" | "usdz")
            ) {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(&root, &mut files);
    files.sort();
    assert!(!files.is_empty(), "no USD files under {}", root.display());
    let dir = scratch("samples-rt");
    let mut failed = Vec::new();
    let mut skipped = 0;
    for (k, path) in files.iter().enumerate() {
        let name = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .display()
            .to_string();
        let started = std::time::Instant::now();
        let result = (|| -> Result<Option<String>, String> {
            let first = match crate::import(path, &Default::default()) {
                Ok(first) => first,
                // Layer, material and stage-setting test files hold no
                // geometry: nothing to round-trip.
                Err(e) if e.contains("no polygons to import") => return Ok(None),
                Err(e) => return Err(format!("import: {e}")),
            };
            let out = dir.join(format!("{k}.usdc"));
            crate::export(&out, &first.scene, ExportFormat::Usdc)
                .map_err(|e| format!("export: {e}"))?;
            let again =
                crate::import(&out, &Default::default()).map_err(|e| format!("re-import: {e}"))?;
            let (a, b) = (
                round_trip_counts(&first.scene),
                round_trip_counts(&again.scene),
            );
            // Counts exactly; the world box to a ten-thousandth of its size
            // (points are floats, and -0 used to print unlike 0).
            let tolerance = 1e-4 * (a.2 - a.1).length().max(1.0);
            if a.0 != b.0 || !a.1.abs_diff_eq(b.1, tolerance) || !a.2.abs_diff_eq(b.2, tolerance) {
                return Err(format!("differs: {a:?} vs {b:?}"));
            }
            Ok(Some(format!("{} items, {} polygons", a.0.0, a.0.1)))
        })();
        let ms = started.elapsed().as_millis();
        match result {
            Ok(None) => {
                println!("SKIP {name} (no geometry)");
                skipped += 1;
            }
            Ok(Some(what)) => println!("PASS {name} ({what}, {ms} ms)"),
            Err(why) => {
                println!("FAIL {name}: {why}");
                failed.push(name);
            }
        }
    }
    println!(
        "{} of {} passed, {skipped} skipped (no geometry), {} failed",
        files.len() - failed.len() - skipped,
        files.len(),
        failed.len()
    );
    assert!(failed.is_empty(), "failed: {failed:?}");
}

/// Item, polygon, camera and light counts and the world box, for the
/// samples sweep.
fn round_trip_counts(scene: &Scene) -> ((usize, usize, usize, usize), DVec3, DVec3) {
    let (items, polygons, cameras, lights, _) = round_trip_summary(scene);
    let (mut lo, mut hi) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
    for i in scene.items.iter().filter(|i| i.is_mesh()) {
        let w = scene.world_matrix(i.id).unwrap();
        for v in &i.mesh.vertices {
            let p = w.transform_point3(*v);
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    ((items, polygons, cameras, lights), lo, hi)
}

/// Items, polygons and the world box of everything, for round trips.
fn round_trip_summary(scene: &Scene) -> (usize, usize, usize, usize, [String; 2]) {
    let items = scene
        .items
        .iter()
        .filter(|i| !i.is_texture_item() && i.environment.is_none())
        .count();
    let polygons = scene.items.iter().map(|i| i.mesh.faces.len()).sum();
    let cameras = scene.items.iter().filter(|i| i.camera.is_some()).count();
    let lights = scene.items.iter().filter(|i| i.light.is_some()).count();
    let (mut lo, mut hi) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
    for i in scene.items.iter().filter(|i| i.is_mesh()) {
        let w = scene.world_matrix(i.id).unwrap();
        for v in &i.mesh.vertices {
            let p = w.transform_point3(*v);
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    (
        items,
        polygons,
        cameras,
        lights,
        [format!("{lo:.4}"), format!("{hi:.4}")],
    )
}

/// U6: creases on plain (unsubdivided) polygons are written too, as
/// Blender writes and reads them; they used to be dropped.
#[test]
fn creases_on_plain_meshes_are_written() {
    let mut scene = box_scene("Box");
    let item = scene.items.iter_mut().find(|i| i.name == "Box").unwrap();
    let (a, b) = {
        let f = &item.mesh.faces[0];
        (
            f.vertices[0].min(f.vertices[1]),
            f.vertices[0].max(f.vertices[1]),
        )
    };
    item.mesh
        .shared_mut()
        .attributes
        .creases
        .push(MeshCrease { a, b, weight: 0.5 });
    let (_, text) = round_trip(&scene, ExportFormat::Usda);
    assert!(text.contains("float[] creaseSharpnesses = [5]"), "{text}");
    assert!(text.contains("uniform token subdivisionScheme = \"none\""));
}
