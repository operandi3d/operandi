use super::*;
use std::collections::BTreeMap;

fn shape(name: &str) -> Mesh {
    ALL.iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no starter mesh {name}"))
        .mesh()
        .unwrap()
}

fn bounds(mesh: &Mesh) -> (DVec3, DVec3) {
    let mut min = DVec3::splat(f64::INFINITY);
    let mut max = DVec3::splat(f64::NEG_INFINITY);
    for v in &mesh.vertices {
        min = min.min(*v);
        max = max.max(*v);
    }
    (min, max)
}

/// How many faces use each edge.
fn edge_uses(mesh: &Mesh) -> BTreeMap<(u32, u32), u32> {
    let mut uses = BTreeMap::new();
    for f in &mesh.faces {
        for (k, &a) in f.vertices.iter().enumerate() {
            let b = f.vertices[(k + 1) % f.vertices.len()];
            *uses.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
    }
    uses
}

/// Signed volume (positive when faces point outward).
fn volume(mesh: &Mesh) -> f64 {
    let mut v = 0.;
    for f in &mesh.faces {
        let p: Vec<DVec3> = f
            .vertices
            .iter()
            .map(|&i| mesh.vertices[i as usize])
            .collect();
        for k in 1..p.len() - 1 {
            v += p[0].dot(p[k].cross(p[k + 1])) / 6.;
        }
    }
    v
}

fn area(mesh: &Mesh, face: &Face) -> f64 {
    let p: Vec<DVec3> = face
        .vertices
        .iter()
        .map(|&i| mesh.vertices[i as usize])
        .collect();
    (1..p.len() - 1)
        .map(|k| (p[k] - p[0]).cross(p[k + 1] - p[0]).length() / 2.)
        .sum()
}

#[test]
fn every_shape_is_clean_quads_on_the_ground_with_uvs_and_the_default_tag() {
    for s in &ALL {
        let mesh = s.mesh().unwrap();
        assert!(!mesh.faces.is_empty(), "{}", s.name);
        assert!(
            mesh.faces.iter().all(|f| f.vertices.len() == 4),
            "{} has a polygon that isn't a quad",
            s.name
        );
        assert!(
            mesh.faces
                .iter()
                .all(|f| f.material == crate::material::DEFAULT_MATERIAL),
            "{}: one default material tag",
            s.name
        );
        let (min, _) = bounds(&mesh);
        assert!(
            min.y.abs() < 1e-9,
            "{} rests on the ground: {}",
            s.name,
            min.y
        );
        let uvs = &mesh.uv_maps[PRIMITIVE_UV_MAP];
        assert_eq!(uvs.len(), mesh.faces.len(), "{}", s.name);
        assert!(
            uvs.iter()
                .zip(&mesh.faces)
                .all(|(uv, f)| uv.len() == f.vertices.len()
                    && uv.iter().all(|p| p.is_some_and(|p| p.is_finite()))),
            "{}: a UV for every corner",
            s.name
        );
        // No slivers or collapsed quads (the cone's 1 mm tip cap is the
        // smallest on purpose).
        for f in &mesh.faces {
            assert!(area(&mesh, f) > 1e-9, "{}: a degenerate quad", s.name);
            let mut seen = f.vertices.clone();
            seen.sort();
            seen.dedup();
            assert_eq!(seen.len(), 4, "{}: a quad repeats a point", s.name);
        }
        // Nothing left unused.
        let mut used = vec![false; mesh.vertices.len()];
        for f in &mesh.faces {
            for &v in &f.vertices {
                used[v as usize] = true;
            }
        }
        assert!(used.iter().all(|&u| u), "{}: stray points", s.name);
    }
}

#[test]
fn solid_shapes_are_closed_and_face_outward() {
    for name in [
        "Rounded Cube",
        "Sphere",
        "Cylinder",
        "Cone",
        "Torus",
        "Capsule",
        "Tube",
        "Table",
        "Stool",
        "Stepped Plinth",
    ] {
        let mesh = shape(name);
        let open = edge_uses(&mesh).values().filter(|&&n| n != 2).count();
        assert_eq!(open, 0, "{name}: every edge joins two quads");
        assert!(volume(&mesh) > 0., "{name} faces outward");
    }
    // The mug is closed but for the handle's two ends, hidden in the wall.
    let mug = shape("Mug");
    let uses = edge_uses(&mug);
    assert_eq!(uses.values().filter(|&&n| n == 1).count(), 2 * 16);
    assert!(uses.values().all(|&n| n <= 2));
    assert!(volume(&mug) > 0.);
}

#[test]
fn shapes_have_their_real_world_sizes() {
    let size = |name: &str| {
        let (min, max) = bounds(&shape(name));
        max - min
    };
    let near = |a: DVec3, b: DVec3| (a - b).abs().max_element() < 1e-3;
    assert!(near(size("Rounded Cube"), DVec3::splat(0.5)));
    assert!(near(size("Sphere"), DVec3::splat(0.5)));
    assert!(near(size("Cylinder"), DVec3::splat(0.5)));
    assert!(near(size("Capsule"), DVec3::new(0.25, 0.75, 0.25)));
    assert!(near(size("Torus"), DVec3::new(0.5, 0.12, 0.5)));
    assert!(near(size("Tube"), DVec3::new(0.2, 0.6, 0.2)));
    assert!(near(size("Subdivided Plane"), DVec3::new(2., 0., 2.)));
    assert!(near(size("Table"), DVec3::new(1.2, 0.75, 0.75)));
    assert!(near(size("Stepped Plinth"), DVec3::new(1.2, 0.3, 1.2)));
    assert!(near(size("Backdrop Sweep"), DVec3::new(3., 2.5, 2.3)));
    let stool = size("Stool");
    assert!((stool.y - 0.455).abs() < 0.005 && (stool.x - 0.36).abs() < 0.005);
    let mug = size("Mug");
    assert!((mug.y - 0.095).abs() < 1e-3, "mug height {}", mug.y);
    assert!((mug.z - 0.084).abs() < 1e-3);
    // The handle sticks out on +X only; the body stays on the Y axis.
    let (min, max) = bounds(&shape("Mug"));
    assert!((min.x + 0.042).abs() < 1e-3 && max.x > 0.07);
}

#[test]
fn the_sphere_is_round_and_the_rounded_cube_is_mostly_flat() {
    let sphere = shape("Sphere");
    let center = DVec3::new(0., 0.25, 0.);
    for v in &sphere.vertices {
        assert!(((*v - center).length() - 0.25).abs() < 1e-9);
    }
    let exact = 4. / 3. * PI * 0.25f64.powi(3);
    assert!((volume(&sphere) - exact).abs() / exact < 0.01);
    let cube = shape("Rounded Cube");
    let v = volume(&cube);
    assert!(v < 0.125 && v > 0.124, "rounded cube volume {v}");
}

#[test]
fn caps_join_their_walls_without_poles() {
    // Quad-gridded caps: no point is shared by more than five quads (a
    // triangle-fan pole would gather every side).
    for name in [
        "Cylinder",
        "Cone",
        "Sphere",
        "Capsule",
        "Stepped Plinth",
        "Mug",
    ] {
        let mesh = shape(name);
        let mut valence = vec![0; mesh.vertices.len()];
        for f in &mesh.faces {
            for &v in &f.vertices {
                valence[v as usize] += 1;
            }
        }
        assert!(valence.iter().all(|&n| n <= 5), "{name}: a pole");
    }
}

#[test]
fn presets_are_cc0_item_presets_that_save_the_same_every_time() {
    for s in &ALL {
        let preset = s.preset().unwrap();
        assert_eq!(preset.header.kind, PresetKind::Item);
        let source = preset.header.source.as_ref().unwrap();
        assert_eq!(source.licence, "CC0-1.0");
        assert_eq!(source.credit, CREDIT);
        assert!(source.url.starts_with("https://"));
        assert_eq!(preset.scene.items.len(), 1);
        assert_eq!(preset.scene.items[0].name, s.name);
        let a = crate::persistence::encode_preset(&preset).unwrap();
        let b = crate::persistence::encode_preset(&s.preset().unwrap()).unwrap();
        assert_eq!(a, b, "{} is reproducible", s.name);
        // And it passes the pack's licence gate where the pack puts it.
        let item = crate::preset::pack::ManifestItem {
            path: format!("{}/{}", crate::preset::pack::MESHES, s.relative_path()),
            category: s.category.into(),
            kind: "item".into(),
            licence: source.licence.clone(),
            source_url: source.url.clone(),
            credit: source.credit.clone(),
            checked: String::new(),
        };
        crate::preset::pack::check_item(&item).unwrap();
    }
}

#[test]
fn a_starter_mesh_drops_into_a_scene() {
    let dir = std::env::temp_dir().join(format!("operandi-starter-mesh-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("Mug.oppreset");
    let s = ALL.iter().find(|s| s.name == "Mug").unwrap();
    crate::persistence::save_preset(&path, &s.preset().unwrap()).unwrap();
    let mut editor = crate::Editor::default();
    let loaded = crate::persistence::load_preset(&path).unwrap();
    editor.load_preset(loaded, None).unwrap();
    let mug = editor
        .scene
        .items
        .iter()
        .find(|i| i.name == "Mug")
        .expect("the mug is in the scene");
    assert_eq!(mug.mesh.faces.len(), s.mesh().unwrap().faces.len());
    std::fs::remove_dir_all(dir).ok();
}
