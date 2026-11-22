//! Cycles renders of imported USD files, for checking real scenes by eye
//! and against Blender (docs/BEHAVIOR-USD-IMPORT.md). Opt-in, needs the
//! Cycles and USD libraries:
//! `OPERANDI_USD_FILE=<scene.usdc> OPERANDI_USD_RENDER=<out.png>
//! [OPERANDI_USD_FRAME=<part of item names to frame>]
//! cargo test -p operandi-app --features cycles,usd render_a_named_usd_file -- --ignored --nocapture`
use glam::DVec3;
use operandi_core::interchange::{self, UsdImportOptions};

/// A view from above and to the front of the mesh items whose names
/// contain `only` (every mesh item when empty).
fn framing(scene: &operandi_core::Scene, only: &str) -> operandi_cycles::View {
    let (mut lo, mut hi) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
    for i in scene
        .items
        .iter()
        .filter(|i| i.is_mesh() && i.name.contains(only))
    {
        let w = scene.world_matrix(i.id).unwrap();
        for v in &i.mesh.vertices {
            let p = w.transform_point3(*v);
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    let centre = (lo + hi) / 2.0;
    let radius = (hi - lo).length() / 2.0;
    let eye = centre + DVec3::new(0.0, 0.9, 1.6) * radius;
    let forward = (centre - eye).normalize();
    let right = forward.cross(DVec3::Y).normalize();
    let up = right.cross(forward);
    operandi_cycles::View::perspective(eye, right, up, forward, 0.45)
}

#[test]
#[ignore = "needs OPERANDI_USD_FILE, OPERANDI_USD_RENDER and the Cycles and USD libraries"]
fn render_a_named_usd_file() {
    interchange::set_usd_importer(operandi_usd::import);
    let path = std::path::PathBuf::from(std::env::var("OPERANDI_USD_FILE").unwrap());
    let out = std::path::PathBuf::from(std::env::var("OPERANDI_USD_RENDER").unwrap());
    let mut imported = interchange::import_with(&path, &UsdImportOptions::default()).unwrap();
    imported.scene.ensure_camera_and_light().unwrap();
    let scene = &imported.scene;
    let (w, h) = (960, 540);
    let mut settings = operandi_cycles::Settings::studio(
        w,
        h,
        framing(
            scene,
            &std::env::var("OPERANDI_USD_FRAME").unwrap_or_default(),
        ),
    )
    .with_scene_lights(scene)
    .with_scene_world(scene);
    settings.samples = 64;
    let meshes = operandi_cycles::scene_meshes(scene).unwrap();
    let img = operandi_cycles::render(&meshes, &settings).unwrap();
    let encode = |v: f32| {
        let v = v.clamp(0.0, 1.0);
        let s = if v <= 0.003_130_8 {
            12.92 * v
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0).round() as u8
    };
    let mut png = image::RgbImage::new(w, h);
    for (k, p) in png.pixels_mut().enumerate() {
        let c = &img.rgba[k * 4..k * 4 + 3];
        *p = image::Rgb([encode(c[0]), encode(c[1]), encode(c[2])]);
    }
    png.save(&out).unwrap();
    println!("{} ({} on {})", out.display(), meshes.len(), img.device);
}

/// Mean Rec. 709 luminance of each cell of a `cols` × `rows` grid.
fn grid(rgba: &[f32], w: usize, h: usize, cols: usize, rows: usize) -> Vec<f64> {
    let mut out = vec![0.0; cols * rows];
    for y in 0..h {
        for x in 0..w {
            let p = &rgba[(y * w + x) * 4..];
            let l = 0.2126 * p[0] as f64 + 0.7152 * p[1] as f64 + 0.0722 * p[2] as f64;
            out[(y * rows / h) * cols + x * cols / w] += l;
        }
    }
    let per = (w * h / (cols * rows)) as f64;
    out.iter().map(|v| v / per).collect()
}

fn save_png(rgba: &[f32], w: u32, h: u32, path: &std::path::Path) {
    let encode = |v: f32| {
        let v = v.clamp(0.0, 1.0);
        let s = if v <= 0.003_130_8 {
            12.92 * v
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0).round() as u8
    };
    let mut png = image::RgbImage::new(w, h);
    for (k, p) in png.pixels_mut().enumerate() {
        let c = &rgba[k * 4..k * 4 + 3];
        *p = image::Rgb([encode(c[0]), encode(c[1]), encode(c[2])]);
    }
    png.save(path).unwrap();
}

/// Renders the imported USD-T1 through its camera at `w` × `h` (the
/// environment dark when `lights_only`) and compares a grid of cell
/// brightnesses with Blender's render `reference`.
fn compare_t1(reference: &std::path::Path, lights_only: bool) {
    let blender = image::open(reference).unwrap().into_rgba32f();
    let (w, h) = blender.dimensions();
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/usd/usd_t1/usd_t1.usdc");
    let mut imported = interchange::import_with(&file, &UsdImportOptions::default()).unwrap();
    if lights_only {
        for item in &mut imported.scene.items {
            if let Some(e) = item.environment.as_mut() {
                e.intensity = 0.0;
            }
        }
    }
    let scene = &imported.scene;
    let camera = scene.render_camera().unwrap();
    let view = operandi_cycles::camera_view(scene, camera, w as f64 / h as f64).unwrap();
    let mut settings = operandi_cycles::Settings::studio(w, h, view)
        .with_only_scene_lights(scene)
        .with_scene_world(scene);
    settings.samples = 256;
    settings.denoise = false;
    let meshes = operandi_cycles::scene_meshes(scene).unwrap();
    let ours = operandi_cycles::render(&meshes, &settings).unwrap();
    let theirs: Vec<f32> = blender.into_raw();
    let stem = reference
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    save_png(
        &ours.rgba,
        w,
        h,
        &reference.with_file_name(format!("{stem}_operandi.png")),
    );
    save_png(
        &theirs,
        w,
        h,
        &reference.with_file_name(format!("{stem}_blender.png")),
    );
    let (a, b) = (
        grid(&ours.rgba, w as usize, h as usize, 4, 3),
        grid(&theirs, w as usize, h as usize, 4, 3),
    );
    let mean = |g: &[f64]| g.iter().sum::<f64>() / g.len() as f64;
    println!(
        "{stem} overall: operandi {:.4}, blender {:.4}",
        mean(&a),
        mean(&b)
    );
    for (k, (x, y)) in a.iter().zip(&b).enumerate() {
        println!(
            "{stem} cell {k}: operandi {x:.4}, blender {y:.4}, ratio {:.3}",
            x / y
        );
    }
    assert!((mean(&a) / mean(&b) - 1.0).abs() < 0.10, "{stem}: overall");
    for (x, y) in a.iter().zip(&b) {
        // Dark cells are mostly noise; compare the lit ones.
        if *y > 0.02 {
            assert!((x / y - 1.0).abs() < 0.10, "{stem}: {x} vs {y}");
        }
    }
}

/// U3's check: the USD-T1 scene rendered through its imported camera with
/// its imported lights and dome matches Blender's own Cycles render of the
/// same scene (fixtures/usd/make_usd_t1.py with USD_T1_RENDER: 480 × 270,
/// Standard view, no denoising) within 10 % overall and per lit region,
/// once as it is and once with the world dark (the lights alone, from
/// Blender's `<name>_lights.exr`). `OPERANDI_T1_BLENDER=<blender.exr>`;
/// writes each pair as PNGs beside it.
#[test]
#[ignore = "needs OPERANDI_T1_BLENDER and the Cycles and USD libraries"]
fn usd_t1_renders_like_blender() {
    interchange::set_usd_importer(operandi_usd::import);
    let reference = std::path::PathBuf::from(std::env::var("OPERANDI_T1_BLENDER").unwrap());
    compare_t1(&reference, false);
    // Blender's second render, world dark: the lights alone.
    let lights = reference.with_file_name(format!(
        "{}_lights.exr",
        reference.file_stem().unwrap().to_string_lossy()
    ));
    compare_t1(&lights, true);
}

/// Speckle measure: the share of pixels brighter than twice their 3 × 3
/// neighbourhood's median plus 0.05, and the RMS of each pixel's luminance
/// minus that median (fireflies and grain both raise it).
fn speckle(rgba: &[f32], w: usize, h: usize) -> (f64, f64) {
    let lum = |x: usize, y: usize| {
        let p = &rgba[(y * w + x) * 4..];
        0.2126 * p[0] as f64 + 0.7152 * p[1] as f64 + 0.0722 * p[2] as f64
    };
    let (mut hot, mut sq, mut n) = (0usize, 0.0, 0usize);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let mut around: Vec<f64> = (0..9).map(|k| lum(x + k % 3 - 1, y + k / 3 - 1)).collect();
            around.sort_by(f64::total_cmp);
            let (m, v) = (around[4], lum(x, y));
            if v > 2.0 * m + 0.05 {
                hot += 1;
            }
            sq += (v.min(4.0) - m.min(4.0)).powi(2);
            n += 1;
        }
    }
    (hot as f64 / n as f64, (sq / n as f64).sqrt())
}

/// The Preview's own render of a USD file through a named camera, at its
/// default settings, with its speckle measure. `OPERANDI_USD_FILE`,
/// `OPERANDI_USD_CAMERA` (part of the camera's name), optional
/// `OPERANDI_USD_RENDER=<out.png>`.
#[test]
#[ignore = "needs OPERANDI_USD_FILE, OPERANDI_USD_CAMERA and the Cycles and USD libraries"]
fn preview_of_a_named_usd_file() {
    interchange::set_usd_importer(operandi_usd::import);
    let path = std::path::PathBuf::from(std::env::var("OPERANDI_USD_FILE").unwrap());
    let wanted = std::env::var("OPERANDI_USD_CAMERA").unwrap();
    let mut imported = interchange::import_with(&path, &UsdImportOptions::default()).unwrap();
    imported.scene.ensure_camera_and_light().unwrap();
    let scene = &imported.scene;
    let camera = scene
        .items
        .iter()
        .find(|i| i.camera.is_some() && i.name.contains(&wanted))
        .unwrap()
        .id;
    let (w, h) = (640u32, 400u32);
    let view = operandi_cycles::camera_view(scene, camera, w as f64 / h as f64).unwrap();
    let saved = crate::preview::Saved::default();
    let request = crate::preview::Request {
        width: w,
        height: h,
        view,
        samples: scene.render.preview_samples(saved.samples),
        threshold: saved.threshold,
        denoise: std::env::var("OPERANDI_USD_NO_DENOISE").is_err() && saved.denoise,
        time_limit: scene.render.preview_seconds() as f32,
        light_paths: Default::default(),
        threads: 0,
        transparent: false,
        render_subdivision: false,
        only_selected: false,
        only_visible: true,
        scene_serial: 0,
    };
    let mut settings = request.settings(scene);
    if let Ok(t) = std::env::var("OPERANDI_USD_THRESHOLD") {
        settings.adaptive_threshold = t.parse().unwrap();
    }
    if let Ok(n) = std::env::var("OPERANDI_USD_SAMPLES") {
        settings.samples = n.parse().unwrap();
    }
    println!(
        "render settings: samples {} threshold {} clamp {} preview convergence {} time {}",
        scene.render.samples,
        scene.render.adaptive_threshold,
        scene.render.max_radiance,
        scene.render.preview_convergence,
        scene.render.preview_time_limit
    );
    let meshes = operandi_cycles::scene_meshes(scene).unwrap();
    let start = std::time::Instant::now();
    // The app's way: the coarse pass, then the full one, through the kept
    // session.
    let img = if std::env::var("OPERANDI_USD_KEPT").is_ok() {
        let control = operandi_cycles::RenderControl::default();
        let mut coarse = request.coarse(scene);
        if std::env::var("OPERANDI_USD_COARSE_DENOISE").is_ok() {
            coarse.denoise = true;
        }
        if std::env::var("OPERANDI_USD_COARSE_FULL").is_ok() {
            coarse = settings.clone();
        }
        operandi_cycles::render_kept(&meshes, &coarse, None, &control).unwrap();
        operandi_cycles::render_kept(&meshes, &settings, None, &control).unwrap()
    } else {
        operandi_cycles::render(&meshes, &settings).unwrap()
    };
    let (hot, rms) = speckle(&img.rgba, w as usize, h as usize);
    println!(
        "preview: {} samples asked, {} done, threshold {}, denoised {}, {:.1} s on {}; speckle {:.5}, rms {:.5}",
        settings.samples,
        img.samples,
        settings.adaptive_threshold,
        img.denoised,
        start.elapsed().as_secs_f64(),
        img.device,
        hot,
        rms
    );
    if let Ok(out) = std::env::var("OPERANDI_USD_RENDER") {
        save_png(&img.rgba, w, h, std::path::Path::new(&out));
    }
}
