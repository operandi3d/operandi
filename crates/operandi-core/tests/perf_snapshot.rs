//! Performance snapshot scenes (Jason, 2026-11-12): procedurally generated
//! test scenes for the viewport and editing targets, written as .opscene
//! files, with their save and reopen times.
//!
//! Opt-in, release mode only:
//! `OPERANDI_PERF_DIR=<folder> cargo test --release -p operandi-core --test all perf_snapshot:: -- --ignored --nocapture`
//! `OPERANDI_PERF_ONLY=sub10m,objects10k` limits which scenes are made.
use glam::DVec3;
use operandi_core::{
    Face, Mesh, RadialParameters, Scene, globe_geometry, persistence, subdivision,
};
use std::path::PathBuf;
use std::time::Instant;

/// One mesh item: a sphere of `sides` × `segments` polygons, made
/// Catmull-Clark (default display level 2) when `subdivided`.
fn sphere_scene(sides: u32, segments: u32, subdivided: bool) -> Scene {
    // Built here: the Sphere primitive stops at 250,000 polygons.
    let mut mesh = big_sphere(sides, segments);
    if subdivided {
        mesh = subdivision::toggle(&mesh, &[], subdivision::Toggle::CatmullClark).unwrap();
    }
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.items.last_mut().unwrap().name = "Sphere".into();
    scene.items.last_mut().unwrap().mesh = mesh.into();
    scene
}

/// A unit UV sphere about Y: quads, with triangle fans at the poles.
fn big_sphere(sides: u32, segments: u32) -> Mesh {
    use std::f64::consts::{PI, TAU};
    let mut vertices = vec![DVec3::Y, -DVec3::Y];
    for s in 1..segments {
        let polar = PI * f64::from(s) / f64::from(segments);
        for i in 0..sides {
            let a = TAU * f64::from(i) / f64::from(sides);
            vertices.push(DVec3::new(
                polar.sin() * a.cos(),
                polar.cos(),
                -polar.sin() * a.sin(),
            ));
        }
    }
    let ring = |s: u32, i: u32| 2 + (s - 1) * sides + i % sides;
    let face = |vertices: Vec<u32>| Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    };
    let mut faces = Vec::with_capacity((sides * segments) as usize);
    for i in 0..sides {
        faces.push(face(vec![0, ring(1, i), ring(1, i + 1)]));
    }
    for s in 1..segments - 1 {
        for i in 0..sides {
            faces.push(face(vec![
                ring(s, i),
                ring(s + 1, i),
                ring(s + 1, i + 1),
                ring(s, i + 1),
            ]));
        }
    }
    for i in 0..sides {
        faces.push(face(vec![
            1,
            ring(segments - 1, i + 1),
            ring(segments - 1, i),
        ]));
    }
    Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    }
}

/// `count` separate mesh items, each a small 16 × 8 sphere (128 polygons),
/// laid out on a square grid by their item positions.
fn objects_scene(count: usize) -> Scene {
    let mesh = globe_geometry(
        DVec3::ZERO,
        RadialParameters {
            radius: DVec3::splat(0.4),
            sides: 16,
            segments: 8,
        },
    )
    .unwrap();
    let side = (count as f64).sqrt().ceil() as usize;
    let mut scene = Scene::default();
    for n in 0..count {
        scene.new_mesh().unwrap();
        let item = scene.items.last_mut().unwrap();
        item.name = format!("Ball {n}");
        item.mesh = mesh.clone().into();
        item.transform.translation = DVec3::new(
            (n % side) as f64 - side as f64 / 2.,
            0.,
            (n / side) as f64 - side as f64 / 2.,
        );
    }
    scene
}

/// Triangles drawn: each polygon as a fan, and a Catmull-Clark cage
/// polygon of n sides as n · 4^(L-1) quads at level L.
fn drawn_triangles(scene: &Scene) -> usize {
    scene
        .items
        .iter()
        .map(|i| {
            let smooth = subdivision::has_smooth(&i.mesh);
            i.mesh
                .faces
                .iter()
                .map(|f| {
                    let n = f.vertices.len();
                    if smooth {
                        n * 4 * 2
                    } else {
                        n.saturating_sub(2)
                    }
                })
                .sum::<usize>()
        })
        .sum()
}

#[test]
#[ignore = "opt-in: writes the performance snapshot scenes (OPERANDI_PERF_DIR)"]
fn perf_snapshot_scenes() {
    let Some(dir) = std::env::var_os("OPERANDI_PERF_DIR").map(PathBuf::from) else {
        println!("OPERANDI_PERF_DIR not set");
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let only = std::env::var("OPERANDI_PERF_ONLY").ok();
    let wanted = |name: &str| {
        only.as_deref()
            .is_none_or(|o| o.split(',').any(|w| w == name))
    };
    type Make = Box<dyn Fn() -> Scene>;
    let scenes: Vec<(&str, Make)> = vec![
        // 10M triangles after subdivision: 312k-polygon cage, level 2.
        ("sub10m", Box::new(|| sphere_scene(790, 396, true))),
        // 50M triangles after subdivision: 1.56M-polygon cage, level 2.
        ("sub50m", Box::new(|| sphere_scene(1768, 884, true))),
        // 10M triangles with no subdivision: 5M plain polygons.
        ("flat10m", Box::new(|| sphere_scene(3162, 1581, false))),
        // 1M editable polygons in one mesh, no subdivision.
        ("edit1m", Box::new(|| sphere_scene(1414, 708, false))),
        // 10k separate objects.
        ("objects10k", Box::new(|| objects_scene(10_000))),
        // A small cage at a high level (Jason's reference: a sphere showing
        // ~145M triangles orbits at ~25 fps in another modeller, 2026-11-13):
        // 8.7k-polygon cage, Catmull-Clark level 6, ~71M triangles. Half the
        // reference's triangles, to keep the run's memory modest.
        (
            "subcage70m",
            Box::new(|| {
                let mut scene = sphere_scene(132, 66, true);
                let item = scene.items.last_mut().unwrap();
                let mut settings = item.subdivision.clone().unwrap_or_default();
                settings.catmull_clark_level = subdivision::MAX_LEVEL;
                item.subdivision = Some(settings);
                scene
            }),
        ),
    ];
    for (name, make) in scenes {
        if !wanted(name) {
            continue;
        }
        let t = Instant::now();
        let scene = make();
        let made = t.elapsed().as_secs_f64();
        let polygons: usize = scene.items.iter().map(|i| i.mesh.faces.len()).sum();
        let path = dir.join(format!("{name}.opscene"));
        let t = Instant::now();
        persistence::save(&path, &scene).unwrap();
        let saved = t.elapsed().as_secs_f64();
        let bytes = std::fs::metadata(&path).unwrap().len();
        drop(scene);
        let t = Instant::now();
        let back = persistence::load(&path).unwrap();
        let loaded = t.elapsed().as_secs_f64();
        println!(
            "PERF scene/{name} items={} polygons={polygons} drawn_triangles={} file_mb={:.1} make_s={made:.2} save_s={saved:.2} load_s={loaded:.2}",
            back.items.len(),
            drawn_triangles(&back),
            bytes as f64 / 1_048_576.,
        );
    }
}
