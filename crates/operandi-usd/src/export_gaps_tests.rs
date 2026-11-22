//! What the USD export used to drop: written through OpenUSD, read back
//! with Operandi's own import, and compared.
use operandi_core::interchange::ExportFormat;
use operandi_core::{Editor, Scene};
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "operandi-usd-gaps-{name}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Exports `scene` as `format` and imports the file again.
fn export_and_import(scene: &Scene, format: ExportFormat) -> (Vec<String>, Scene, String) {
    let path = scratch("rt").join(format!("out.{}", format.extension()));
    let report = crate::export(&path, scene, format).unwrap();
    let text = if format == ExportFormat::Usda {
        std::fs::read_to_string(&path).unwrap()
    } else {
        String::new()
    };
    let back = crate::import(&path, &Default::default()).unwrap();
    (report.notes, back.scene, text)
}

/// MeshOps: File > Export, scene.saveAs and the MCP bridge hand the
/// export the evaluated scene (Purpose::Final), so the file holds what the
/// stack makes, not the Base Mesh under it.
#[test]
fn meshops_results_are_exported_not_the_base_mesh() {
    use operandi_core::evaluate::Purpose;
    operandi_core::interchange::set_usd_exporter(crate::export);
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.dispatch(operandi_core::Command::MeshOpCreate(
        "prim.cube.item".into(),
    ))
    .unwrap();
    let base = e.scene.items[0].mesh.faces.len();
    let evaluated = e.evaluated(Purpose::Final).items[0].mesh.faces.len();
    assert!(evaluated > base, "the stack adds polygons to the Base Mesh");
    let path = scratch("meshops").join("stack.usda");
    let mut args = serde_json::Map::new();
    args.insert(
        "filename".into(),
        path.to_string_lossy().into_owned().into(),
    );
    operandi_core::command_exec::run(&mut e, "scene.saveAs", &args).unwrap();
    let back = crate::import(&path, &Default::default()).unwrap();
    let polygons: usize = back.scene.items.iter().map(|i| i.mesh.faces.len()).sum();
    assert_eq!(polygons, evaluated);
}

/// A box carrying one of everything the export used to drop: weight,
/// typed and colour maps (some covering only part of the mesh), a morph,
/// parts and point, polygon and edge selection sets. Values are exact in
/// f32, so the round trip compares equal.
pub(crate) fn feature_scene() -> Scene {
    use glam::{DVec3, DVec4};
    use operandi_core::TypedMap;
    let mut scene = crate::plan_tests::box_scene("Box");
    let item = &mut scene.items[0];
    let mesh = item.mesh.shared_mut();
    let sizes: Vec<usize> = mesh.faces.iter().map(|f| f.vertices.len()).collect();
    let corners = |f: usize, value: Option<f64>| -> Vec<Vec<Option<f64>>> {
        sizes
            .iter()
            .enumerate()
            .map(|(i, &n)| vec![if i == f { value } else { None }; n])
            .collect()
    };
    let (e0, e1) = (mesh.faces[0].vertices[0], mesh.faces[0].vertices[1]);
    {
        let faces = &mut *mesh.faces;
        faces[0].part = Some("Lid".into());
        faces[1].part = Some("Lid".into());
        faces[2].part = Some("Side wall".into());
    }
    let a = &mut *mesh.attributes;
    // Point 3 has no weight; point 2 weighs 0 (not the same thing).
    a.point_weights
        .insert("Weight".into(), [(0, 0.5), (1, 1.0), (2, 0.0)].into());
    a.corner_weights
        .insert("Corner weight".into(), corners(0, Some(0.25)));
    a.typed_maps.insert(
        "Tint".into(),
        TypedMap {
            kind: "rgb3".into(),
            dimension: 3,
            zero_default: false,
            points: (0..8)
                .map(|p| (p, vec![0.5, 0.25, p as f64 / 8.0]))
                .collect(),
            corners: Vec::new(),
        },
    );
    a.typed_maps.insert(
        "Velocity".into(),
        TypedMap {
            kind: "vector3".into(),
            dimension: 3,
            zero_default: true,
            points: [(4, vec![0.0, 1.0, -0.5])].into(),
            corners: Vec::new(),
        },
    );
    a.typed_maps.insert(
        "Basis".into(),
        TypedMap {
            kind: "tangent_basis6".into(),
            dimension: 6,
            zero_default: false,
            points: [(5, vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0])].into(),
            corners: Vec::new(),
        },
    );
    a.point_colors.insert(
        "Point colour".into(),
        [(0, DVec4::new(1.0, 0.0, 0.0, 0.5)), (6, DVec4::ONE)].into(),
    );
    let mut corner_colour: Vec<Vec<Option<DVec4>>> = sizes.iter().map(|&n| vec![None; n]).collect();
    corner_colour[3][2] = Some(DVec4::new(0.0, 0.5, 1.0, 1.0));
    a.corner_colors
        .insert("Corner colour".into(), corner_colour);
    a.relative_morphs.insert(
        "Smile".into(),
        [
            (1, DVec3::new(0.0, 0.5, 0.0)),
            (7, DVec3::new(-0.25, 0.0, 0.0)),
        ]
        .into(),
    );
    a.polygon_sets.insert("Top".into(), [0, 2].into());
    a.polygon_sets.insert("Empty".into(), Default::default());
    a.vertex_sets.insert("Corners".into(), [0, 7].into());
    a.edge_sets
        .insert("Rim".into(), [(e0.min(e1), e0.max(e1))].into());
    scene
}

#[test]
fn maps_morphs_parts_and_sets_round_trip() {
    let scene = feature_scene();
    let (_, back, text) = export_and_import(&scene, ExportFormat::Usda);
    for expected in [
        "color3f[] primvars:displayColor",
        "unauthoredValuesIndex = 0",
        "float[] primvars:Weight",
        "elementSize = 6",
        "def BlendShape \"Smile\"",
        "uniform token familyName = \"operandi:part\"",
        "uniform token elementType = \"point\"",
        "edgeSets",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in\n{text}");
    }
    let mesh_items: Vec<_> = back.items.iter().filter(|i| i.is_mesh()).collect();
    let [item] = mesh_items[..] else {
        panic!("one mesh item, got {}", mesh_items.len())
    };
    let (was, now) = (&scene.items[0].mesh, &item.mesh);
    assert_eq!(was.vertices.len(), now.vertices.len());
    let (a, b) = (&was.attributes, &now.attributes);
    assert_eq!(a.point_weights, b.point_weights);
    assert_eq!(a.corner_weights, b.corner_weights);
    assert_eq!(a.typed_maps, b.typed_maps);
    assert_eq!(a.point_colors, b.point_colors);
    assert_eq!(a.corner_colors, b.corner_colors);
    assert_eq!(a.relative_morphs, b.relative_morphs);
    assert_eq!(a.polygon_sets, b.polygon_sets);
    assert_eq!(a.vertex_sets, b.vertex_sets);
    assert_eq!(a.edge_sets, b.edge_sets);
    let parts = |m: &operandi_core::Mesh| -> Vec<Option<String>> {
        m.faces.iter().map(|f| f.part.clone()).collect()
    };
    assert_eq!(parts(was), parts(now));
}

/// Binary files carry the same (customData and indexed primvars survive
/// .usdc too).
#[test]
fn maps_survive_binary_usd() {
    let scene = feature_scene();
    let (_, back, _) = export_and_import(&scene, ExportFormat::Usdc);
    let item = back.items.iter().find(|i| i.is_mesh()).unwrap();
    let a = &scene.items[0].mesh.attributes;
    assert_eq!(a.typed_maps, item.mesh.attributes.typed_maps);
    assert_eq!(a.relative_morphs, item.mesh.attributes.relative_morphs);
}

/// Mesh Instances are USD native instances of one prototype (a class
/// under /root/_prototypes), and come back as Mesh Instances of the source,
/// keeping their own transforms and children.
#[test]
fn mesh_instances_are_usd_instances() {
    use crate::plan_tests::add_box;
    let mut scene = crate::plan_tests::box_scene("Source");
    let source = scene.items[0].id;
    let shared = scene.items[0].mesh.clone();
    let mut instances = Vec::new();
    for (name, x) in [("CopyA", 3.0), ("CopyB", -3.0)] {
        let id = add_box(&mut scene, name, None);
        let item = scene.item_mut(id).unwrap();
        item.instance_of = Some(source);
        item.mesh = shared.clone();
        item.transform.translation.x = x;
        instances.push(id);
    }
    // An instance with an item of its own below it.
    add_box(&mut scene, "Child", Some(instances[0]));
    let (notes, back, text) = export_and_import(&scene, ExportFormat::Usda);
    for expected in [
        "class Xform \"_prototypes\"",
        "instanceable = true",
        "prepend references = </root/_prototypes/Source>",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in\n{text}");
    }
    assert!(
        !notes.iter().any(|n| n.contains("copies")),
        "nothing written as a copy: {notes:?}"
    );
    let by_name = |n: &str| {
        back.items.iter().find(|i| i.name == n).unwrap_or_else(|| {
            let names: Vec<&str> = back.items.iter().map(|i| i.name.as_str()).collect();
            panic!("no {n:?} in {names:?}\n{text}")
        })
    };
    let src = by_name("Source");
    assert_eq!(src.instance_of, None);
    assert_eq!(src.mesh.faces.len(), 6);
    for name in ["CopyA", "CopyB"] {
        let item = by_name(name);
        assert_eq!(item.instance_of, Some(src.id), "{name}");
        assert_eq!(item.mesh.faces.len(), 6);
    }
    assert_eq!(by_name("CopyA").transform.translation.x, 3.0);
    assert_eq!(by_name("Child").parent, Some(by_name("CopyA").id));
    // Nothing from the prototypes class comes in as an item.
    assert_eq!(back.items.iter().filter(|i| i.is_mesh()).count(), 4);
}

/// An instance shading differently from its source can't share the
/// prototype: it's written as a copy, and the report says so.
#[test]
fn instances_that_differ_are_copies_and_reported() {
    let mut scene = crate::plan_tests::box_scene("Source");
    let source = scene.items[0].id;
    let id = crate::plan_tests::add_box(&mut scene, "Odd", None);
    let item = scene.item_mut(id).unwrap();
    item.instance_of = Some(source);
    item.subdivision = Some(Default::default());
    let plan = crate::plan(&scene, &Default::default()).unwrap();
    assert!(
        plan.notes.iter().any(|n| n.contains("written as copies")),
        "{:?}",
        plan.notes
    );
}

/// What USD still gets nothing of is a line in the report, never a silent
/// drop.
#[test]
fn what_stays_behind_is_reported() {
    let mut scene = crate::plan_tests::box_scene("Box");
    {
        let mesh = scene.items[0].mesh.shared_mut();
        let a = &mut *mesh.attributes;
        a.polygon_sets.insert("Top".into(), [0].into());
        a.set_groups.insert(
            "Group".into(),
            vec![operandi_core::SetRef {
                kind: operandi_core::SetKind::Polygon,
                name: "Top".into(),
            }],
        );
        a.smoothing_labels = vec![Some("hard".into()); 6];
    }
    let plan = crate::plan(&scene, &Default::default()).unwrap();
    for expected in ["Selection set groups", "Smoothing group names"] {
        assert!(
            plan.notes.iter().any(|n| n.contains(expected)),
            "no {expected:?} line in {:?}",
            plan.notes
        );
    }
}

/// Writes the feature scene (maps, morph, sets, an instance) to the file
/// `OPERANDI_USD_OUT` names, for checking it in Blender:
/// `OPERANDI_USD_OUT=/tmp/f.usdc cargo test --workspace --features operandi-app/usd write_feature_scene -- --ignored`
#[test]
#[ignore = "needs OPERANDI_USD_OUT"]
fn write_feature_scene() {
    let path = PathBuf::from(std::env::var("OPERANDI_USD_OUT").unwrap());
    let mut scene = feature_scene();
    let source = scene.items[0].id;
    let id = crate::plan_tests::add_box(&mut scene, "Copy", None);
    let shared = scene.items[0].mesh.clone();
    let item = scene.item_mut(id).unwrap();
    item.instance_of = Some(source);
    item.mesh = shared;
    item.transform.translation.x = 3.0;
    let report = crate::export(
        &path,
        &scene,
        operandi_core::interchange::export_format_of_path(&path.to_string_lossy()).unwrap(),
    )
    .unwrap();
    println!("notes: {:?}", report.notes);
}
