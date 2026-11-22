//! Big scenes save and reopen (they used to stop at a fixed 64 MiB), and a
//! failed save leaves the old file alone.
use glam::{DVec2, DVec3};
use operandi_core::{Face, Mesh, Scene, persistence};
use std::path::PathBuf;
use std::time::Instant;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-big-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A `side` × `side` grid of quads with a UV map, a part on every tenth
/// polygon and two materials, as one mesh item; `items` copies of it.
fn grid_scene(side: u32, items: usize) -> Scene {
    let mut vertices = Vec::with_capacity(((side + 1) * (side + 1)) as usize);
    for y in 0..=side {
        for x in 0..=side {
            // Values that don't round-trip through short decimals.
            vertices.push(DVec3::new(
                f64::from(x) * 0.1 + 1. / 3.,
                f64::from(y) * 0.1,
                (f64::from(x * y)).sin() * 0.01,
            ));
        }
    }
    let mut faces = Vec::with_capacity((side * side) as usize);
    let mut uvs = Vec::with_capacity((side * side) as usize);
    for y in 0..side {
        for x in 0..side {
            let a = y * (side + 1) + x;
            let i = faces.len();
            faces.push(Face {
                vertices: vec![a, a + 1, a + side + 2, a + side + 1],
                uv: None,
                material: if i % 2 == 0 { "Wood" } else { "Metal" }.into(),
                smooth: i % 3 == 0,
                part: (i % 10 == 0).then(|| "Strings".into()),
                material_label: None,
            });
            let s = f64::from(side);
            let (u, v) = (f64::from(x) / s, f64::from(y) / s);
            uvs.push(vec![
                Some(DVec2::new(u, v)),
                Some(DVec2::new(u + 1. / s, v)),
                // One corner without a coordinate now and then.
                (i % 7 != 0).then(|| DVec2::new(u + 1. / s, v + 1. / s)),
                Some(DVec2::new(u, v + 1. / s)),
            ]);
        }
    }
    let mut mesh = Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    };
    mesh.uv_maps.insert("Texture".into(), uvs);
    let mut scene = Scene::default();
    for n in 0..items {
        scene.new_mesh().unwrap();
        let item = scene.items.last_mut().unwrap();
        item.name = format!("Grid {n}");
        item.mesh = mesh.clone().into();
    }
    let first = scene.items.first().map(|i| i.id);
    scene.select_item(first);
    scene
}

fn faces(scene: &Scene) -> usize {
    scene.items.iter().map(|i| i.mesh.faces.len()).sum()
}

/// Save, reopen and compare; returns (file bytes, save time, load time).
fn round_trip(scene: &Scene, tag: &str) -> (u64, f64, f64) {
    let dir = temp_dir(tag);
    let path = dir.join("Big.opscene");
    let t = Instant::now();
    persistence::save(&path, scene).unwrap();
    let saved = t.elapsed().as_secs_f64();
    let size = std::fs::metadata(&path).unwrap().len();
    let t = Instant::now();
    let back = persistence::load(&path).unwrap();
    let loaded = t.elapsed().as_secs_f64();
    assert_eq!(back.items.len(), scene.items.len());
    assert_eq!(faces(&back), faces(scene));
    assert_eq!(
        back, *scene,
        "every point, polygon and UV comes back exactly"
    );
    // Saving over the file just written (the destination check reads only
    // its header) works too.
    persistence::save(&path, &back).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    (size, saved, loaded)
}

/// 360,000 polygons in 4 items. In the old pretty-printed format this was
/// over 200 MB, past the 64 MiB that refused it.
#[test]
fn a_scene_past_the_old_64_mib_limit_saves_and_reopens() {
    let scene = grid_scene(300, 4);
    assert_eq!(faces(&scene), 360_000);
    let (size, _, _) = round_trip(&scene, "medium");
    // Packed: about 160 bytes a polygon with its points and UVs.
    assert!(size < 64 << 20, "{size} bytes");
}

/// About 5 million polygons in 50 items, like a big imported model. Slow in
/// a debug build, so opt-in:
/// `cargo test --release -p operandi-core --test all big_scene_save:: -- --ignored --nocapture`
#[test]
#[ignore]
fn five_million_polygons_save_and_reopen() {
    let scene = grid_scene(316, 50);
    let n = faces(&scene);
    assert!(n > 4_900_000, "{n}");
    let (size, saved, loaded) = round_trip(&scene, "five-million");
    eprintln!(
        "{n} polygons in {} items: {:.0} MB, save {saved:.2} s, load {loaded:.2} s",
        scene.items.len(),
        size as f64 / 1e6
    );
}

/// A real model: set OPERANDI_BIG_OBJ to an OBJ file. Imports it, saves,
/// reopens and compares.
#[test]
#[ignore]
fn an_imported_obj_saves_and_reopens() {
    let Ok(path) = std::env::var("OPERANDI_BIG_OBJ") else {
        eprintln!("OPERANDI_BIG_OBJ not set; skipped");
        return;
    };
    let t = Instant::now();
    let imported = operandi_core::interchange::import(std::path::Path::new(&path)).unwrap();
    let import = t.elapsed().as_secs_f64();
    let scene = imported.scene;
    let (size, saved, loaded) = round_trip(&scene, "obj");
    eprintln!(
        "{} polygons in {} items: import {import:.2} s, {:.1} MB, save {saved:.2} s, load {loaded:.2} s",
        faces(&scene),
        scene.items.len(),
        size as f64 / 1e6
    );
}

#[test]
fn a_failed_save_keeps_the_old_file_and_leaves_no_temporary() {
    let dir = temp_dir("failed");
    let path = dir.join("Keep.opscene");
    let small = grid_scene(2, 1);
    persistence::save(&path, &small).unwrap();
    let before = std::fs::read(&path).unwrap();
    // Unwritable: a folder that doesn't exist.
    let missing = dir.join("no-such-folder").join("Big.opscene");
    let error = persistence::save(&missing, &grid_scene(100, 1)).unwrap_err();
    assert!(error.contains("Cannot create save temporary"), "{error}");
    // Replacing a file whose name is taken by a folder fails at the rename.
    let blocked = dir.join("Blocked.opscene");
    std::fs::create_dir(&blocked).unwrap();
    std::fs::write(blocked.join("x"), b"x").unwrap();
    assert!(persistence::save(&blocked, &small).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let leftovers: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn small_meshes_stay_readable_arrays_and_old_versions() {
    let text = String::from_utf8(persistence::encode(&grid_scene(2, 1)).unwrap()).unwrap();
    assert!(!text.contains("\"packed\""));
    assert!(!text.contains("\"version\": 34"));
    let text = String::from_utf8(persistence::encode(&grid_scene(70, 1)).unwrap()).unwrap();
    assert!(text.contains("\"packed\": \"polygons\""));
    assert!(text.contains("\"version\": 34"));
}

#[test]
fn a_damaged_packed_array_is_refused_not_misread() {
    let text = String::from_utf8(persistence::encode(&grid_scene(70, 1)).unwrap()).unwrap();
    // Claim one more polygon than the blobs hold.
    let bad = text.replacen("\"count\": 4900", "\"count\": 4901", 1);
    assert_ne!(bad, text);
    let error = persistence::decode(bad.as_bytes()).unwrap_err();
    assert!(error.contains("wrong number of values"), "{error}");
    // A point index past the end is caught by the mesh validator.
    let mut text = text;
    // Swap in a corrupted index blob: same length, every index huge.
    let start = text.find("\"indices\": \"").unwrap() + 12;
    let end = start + text[start..].find('"').unwrap();
    let blob = "/".repeat(end - start - 4) + "/w==";
    text.replace_range(start..end, &blob[..end - start]);
    assert!(persistence::decode(text.as_bytes()).is_err());
}

/// Hidden-limits A2: a scene file over the old 4 GiB Open limit opens. The
/// file is a small scene padded with JSON whitespace to 4.5 GiB, so it
/// needs disk and memory but no big mesh. Opt-in (about 5 GB of memory and
/// disk, a minute in release):
/// `cargo test --release -p operandi-core --test all big_scene_save::a_scene_over -- --ignored --nocapture`
#[test]
#[ignore]
fn a_scene_over_four_gib_reopens() {
    use std::io::Write;
    let scene = grid_scene(10, 2);
    let bytes = persistence::encode(&scene).unwrap();
    assert_eq!(bytes[0], b'{');
    let dir = temp_dir("over-4-gib");
    let path = dir.join("padded.opscene");
    let mut file = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
    file.write_all(b"{").unwrap();
    let spaces = vec![b' '; 64 << 20];
    let target: u64 = (4 << 30) + (512 << 20);
    let mut written = 0u64;
    while written < target {
        file.write_all(&spaces).unwrap();
        written += spaces.len() as u64;
    }
    file.write_all(&bytes[1..]).unwrap();
    file.flush().unwrap();
    drop(file);
    let size = std::fs::metadata(&path).unwrap().len();
    assert!(size > 4 << 30, "{size}");
    let start = Instant::now();
    let reopened = persistence::load(&path).unwrap();
    eprintln!(
        "{size} bytes reopened in {:.1} s",
        start.elapsed().as_secs_f64()
    );
    assert_eq!(reopened.items.len(), scene.items.len());
    assert_eq!(faces(&reopened), faces(&scene));
    std::fs::remove_dir_all(&dir).unwrap();
}
