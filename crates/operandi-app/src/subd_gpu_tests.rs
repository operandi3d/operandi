//! Subdivision polygons drawn as patches on the graphics card look like
//! the stored surface they replace (docs/SUBD_GPU_DESIGN.md). Same points
//! and edges; the shading normal is the patch's own (from its slopes)
//! instead of the average of the pieces around each point, so lit pixels
//! may differ slightly.
use super::*;

fn wavy(n: u32) -> operandi_core::Mesh {
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            let (x, y) = (i as f64 / n as f64 - 0.5, j as f64 / n as f64 - 0.5);
            mesh.vertices
                .push(DVec3::new(x, y, 0.15 * (x * 9.).sin() * (y * 7.).cos()));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + n + 2, a + n + 1],
                uv: None,
                material: String::new(),
                smooth: true,
                part: None,
                material_label: None,
            });
        }
    }
    mesh
}

/// The first view's picture after a few frames, as patches or stored.
fn picture(
    mesh: operandi_core::Mesh,
    setup: &dyn Fn(&mut Operandi),
    patches: bool,
) -> Option<(Vec<u8>, [u32; 2], Option<u64>)> {
    let state = super::performance_tests::headless_gpu()?;
    crate::subd_gpu::set_off(!patches);
    crate::subd_gpu::set_min_pieces(0);
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::Subdivision,
    ));
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    a.camera.yaw += 0.4;
    a.camera.pitch -= 0.3;
    setup(&mut a);
    for _ in 0..3 {
        super::tests::draw(&mut a, &ctx, vec2(900., 600.), vec![]);
    }
    crate::subd_gpu::set_off(false);
    assert_eq!(
        a.viewport_gpu.as_ref().unwrap().patched_items(),
        usize::from(patches)
    );
    let triangles = a
        .resident_cache
        .drawn_triangles(a.editor.scene.active().unwrap());
    let views = a.viewport_gpu.as_ref().unwrap().read_views();
    views
        .into_iter()
        .next()
        .map(|(_, size, rgba)| (rgba, size, triangles))
}

/// Culling the clusters that face away leaves the picture as it was: on a
/// closed globe (fill and wire culled) and an open sheet (fill only), seen
/// from outside and, for the globe, from inside.
#[test]
fn culling_keeps_the_picture() {
    type Setup = Box<dyn Fn(&mut Operandi)>;
    let cases: Vec<(&str, operandi_core::Mesh, Setup)> = vec![
        ("globe", globe(48, 24), Box::new(|_| {})),
        ("sheet", wavy(16), Box::new(|_| {})),
        (
            "inside the globe",
            globe(48, 24),
            Box::new(|a| {
                a.camera.distance = 0.05;
            }),
        ),
    ];
    for (name, mesh, setup) in &cases {
        crate::subd_gpu::set_culling(true);
        let culled = picture(mesh.clone(), setup.as_ref(), true);
        crate::subd_gpu::set_culling(false);
        let all = picture(mesh.clone(), setup.as_ref(), true);
        crate::subd_gpu::set_culling(true);
        let (Some((culled, size, _)), Some((all, _, _))) = (culled, all) else {
            println!("no graphics device");
            return;
        };
        keep(&format!("culled {name}"), &culled, size);
        keep(&format!("unculled {name}"), &all, size);
        let most = culled.iter().zip(&all).map(|(p, q)| p.abs_diff(*q)).max();
        assert!(most <= Some(2), "{name}: pictures differ by {most:?}/255");
        assert!(all.chunks(4).any(|p| p[3] > 0), "{name}: nothing drawn");
    }
}

/// Save a picture for a person to look at, when OPERANDI_CAPTURE_DIR is set.
fn keep(name: &str, rgba: &[u8], size: [u32; 2]) {
    if let Ok(dir) = std::env::var("OPERANDI_CAPTURE_DIR")
        && let Some(image) = image::RgbaImage::from_raw(size[0], size[1], rgba.to_vec())
    {
        let _ = image.save(std::path::Path::new(&dir).join(format!("{name}.png")));
    }
}

#[test]
fn patches_draw_the_stored_surface() {
    type Setup = Box<dyn Fn(&mut Operandi)>;
    let scenes: Vec<(&str, Setup)> = vec![
        ("plain", Box::new(|_| {})),
        (
            "polygons selected",
            Box::new(|a| {
                a.run(Command::Mode(Mode::Polygon));
                a.editor.scene.selection.faces = vec![100, 101, 117, 5];
            }),
        ),
        (
            "item selected",
            Box::new(|a| a.run(Command::Mode(Mode::Item))),
        ),
    ];
    for (name, setup) in &scenes {
        let (Some((new, size, drawn_new)), Some((old, _, drawn_old))) = (
            picture(wavy(16), setup.as_ref(), true),
            picture(wavy(16), setup.as_ref(), false),
        ) else {
            println!("no graphics device");
            return;
        };
        keep(&format!("patches {name}"), &new, size);
        keep(&format!("stored {name}"), &old, size);
        assert_eq!(drawn_new, drawn_old, "{name}: GL triangle count");
        let differ = new
            .chunks(4)
            .zip(old.chunks(4))
            .filter(|(p, q)| p.iter().zip(*q).any(|(a, b)| a.abs_diff(*b) > 24))
            .count();
        let drawn = old.chunks(4).filter(|p| p[3] > 0).count();
        println!("{name}: {differ} of {drawn} pixels differ by more than 24/255");
        assert!(drawn > 0, "{name}: nothing drawn");
        assert!(
            differ * 100 < drawn,
            "{name}: {differ} of {drawn} pixels differ by more than 24/255"
        );
    }
}

/// A globe: `segments` around × `rings` down, quads with triangle fans at
/// the poles (`segments × rings` polygons), radius 1.
fn globe(segments: u32, rings: u32) -> operandi_core::Mesh {
    let mut mesh = operandi_core::Mesh::default();
    let face = |vertices| Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: true,
        part: None,
        material_label: None,
    };
    mesh.vertices.push(DVec3::Y);
    for r in 1..rings {
        let phi = std::f64::consts::PI * r as f64 / rings as f64;
        for s in 0..segments {
            let theta = std::f64::consts::TAU * s as f64 / segments as f64;
            mesh.vertices.push(DVec3::new(
                phi.sin() * theta.cos(),
                phi.cos(),
                phi.sin() * theta.sin(),
            ));
        }
    }
    mesh.vertices.push(-DVec3::Y);
    let bottom = mesh.vertices.len() as u32 - 1;
    let at = |r: u32, s: u32| 1 + (r - 1) * segments + s % segments;
    mesh.faces.reserve((segments * rings) as usize);
    for s in 0..segments {
        mesh.faces.push(face(vec![0, at(1, s + 1), at(1, s)]));
    }
    for r in 1..rings - 1 {
        for s in 0..segments {
            mesh.faces.push(face(vec![
                at(r, s),
                at(r, s + 1),
                at(r + 1, s + 1),
                at(r + 1, s),
            ]));
        }
    }
    for s in 0..segments {
        mesh.faces
            .push(face(vec![at(rings - 1, s), at(rings - 1, s + 1), bottom]));
    }
    mesh
}

fn resident_megabytes() -> f64 {
    let Ok(mut system) = sysinfo::System::new() else {
        return 0.;
    };
    let pid = sysinfo::get_current_pid().ok();
    system.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&pid.into_iter().collect::<Vec<_>>()),
        true,
    );
    pid.and_then(|p| system.process(p))
        .map_or(0., |p| p.memory() as f64 / (1 << 20) as f64)
}

/// Jason's reference: a 4.5M-quad sphere at Subdivision level 2 (~145M
/// triangles) orbits at about 25 fps in Modo on an M1 Max. Tab, then a
/// full orbit in Polygon and Item mode, each frame waited for on the
/// graphics card. OPERANDI_GLOBE=segments×rings (default 3000×1500),
/// OPERANDI_BENCH_SIZE=w×h (default 1600×1000).
#[test]
#[ignore = "opt-in benchmark of the reference subdivided sphere"]
fn subdivided_sphere_orbit() {
    use super::perf_drags_tests::{Bench, ms, one, report};
    let pair = |key: &str, default: [u32; 2]| -> [u32; 2] {
        std::env::var(key)
            .ok()
            .and_then(|s| {
                let v: Vec<u32> = s.split('x').filter_map(|p| p.parse().ok()).collect();
                (v.len() == 2).then(|| [v[0], v[1]])
            })
            .unwrap_or(default)
    };
    let [segments, rings] = pair("OPERANDI_GLOBE", [3000, 1500]);
    let [w, h] = pair("OPERANDI_BENCH_SIZE", [1600, 1000]);
    let state = super::performance_tests::headless_gpu().expect("graphics device");
    let ctx = Context::default();
    theme::install(&ctx);
    let b = Bench {
        ctx,
        device: state.device.clone(),
        size: vec2(w as f32, h as f32),
    };
    println!(
        "PERF device max_buffer={} MB max_storage_binding={} MB",
        state.device.limits().max_buffer_size >> 20,
        state.device.limits().max_storage_buffer_binding_size >> 20
    );
    let base = resident_megabytes();
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = globe(segments, rings).into();
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    b.frame(&mut a, vec![]);
    let cage = resident_megabytes();
    let t = std::time::Instant::now();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::Subdivision,
    ));
    b.frame(&mut a, vec![]);
    one("sphere/tab_to_first_frame", ms(t));
    println!(
        "PERF sphere/info polygons={} drawn_triangles={:?} patched_items={}",
        a.editor.scene.active().unwrap().mesh.faces.len(),
        a.resident_cache
            .drawn_triangles(a.editor.scene.active().unwrap()),
        a.viewport_gpu.as_ref().unwrap().patched_items()
    );
    println!(
        "PERF sphere/memory_mb before={base:.0} cage={cage:.0} subdivided={:.0}",
        resident_megabytes()
    );
    let orbit = |a: &mut Operandi, name: &str| {
        b.frame(a, vec![]);
        let frames = std::env::var("OPERANDI_FRAMES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);
        let mut times: Vec<f64> = (0..frames)
            .map(|_| {
                a.camera.yaw += std::f64::consts::TAU / frames as f64;
                b.frame(a, vec![])
            })
            .collect();
        report(name, &mut times);
        // The same orbit split into the frame's CPU part (up to submit)
        // and the wait for the graphics card after it.
        let (mut cpu, mut gpu) = (Vec::new(), Vec::new());
        for _ in 0..frames {
            a.camera.yaw += std::f64::consts::TAU / frames as f64;
            let t = std::time::Instant::now();
            std::hint::black_box(super::tests::draw(a, &b.ctx, b.size, vec![]));
            cpu.push(ms(t));
            let t = std::time::Instant::now();
            let _ = b
                .device
                .poll(egui_wgpu::wgpu::PollType::wait_indefinitely());
            gpu.push(ms(t));
        }
        report(&format!("{name}_cpu"), &mut cpu);
        report(&format!("{name}_gpu_wait"), &mut gpu);
    };
    orbit(&mut a, "sphere/item_mode_orbit");
    a.run(Command::Mode(Mode::Polygon));
    orbit(&mut a, "sphere/polygon_mode_orbit");
    // Drags: a few points, then a patch of a thousand, moved every frame.
    for (name, count) in [("drag_1_point", 1u32), ("drag_1000_points", 1000)] {
        let mut mesh: operandi_core::Mesh = (*a.editor.scene.active().unwrap().mesh).clone();
        let (mut times, mut cpu, mut gpu) = (Vec::new(), Vec::new(), Vec::new());
        let frames: u32 = std::env::var("OPERANDI_DRAG_FRAMES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);
        for frame in 0..frames {
            for v in 0..count {
                let p = &mut mesh.vertices[(2_000_000 + v * 7) as usize];
                *p *= if frame % 2 == 0 { 1.01 } else { 1.0 / 1.01 };
            }
            a.editor.scene.active_mut().unwrap().mesh = mesh.clone().into();
            let t = std::time::Instant::now();
            std::hint::black_box(super::tests::draw(&mut a, &b.ctx, b.size, vec![]));
            cpu.push(ms(t));
            let t = std::time::Instant::now();
            let _ = b
                .device
                .poll(egui_wgpu::wgpu::PollType::wait_indefinitely());
            gpu.push(ms(t));
            times.push(cpu[cpu.len() - 1] + gpu[gpu.len() - 1]);
        }
        report(&format!("sphere/{name}"), &mut times);
        report(&format!("sphere/{name}_cpu"), &mut cpu);
        report(&format!("sphere/{name}_gpu_wait"), &mut gpu);
    }
    for (name, faces) in [("select", vec![10, 11, 2_000_000]), ("deselect", vec![])] {
        let t = std::time::Instant::now();
        a.editor.scene.selection.faces = faces;
        b.frame(&mut a, vec![]);
        one(&format!("sphere/polygons_{name}"), ms(t));
    }
}

/// Selecting polygons on a patch surface rewrites only their looks: no
/// full rebuild, and the picture matches one drawn with that selection
/// from the start.
#[test]
fn selecting_polygons_patches_only_their_looks() {
    let Some(state) = super::performance_tests::headless_gpu() else {
        println!("no graphics device");
        return;
    };
    crate::subd_gpu::set_min_pieces(0);
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = wavy(16).into();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::Subdivision,
    ));
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    a.camera.yaw += 0.4;
    a.camera.pitch -= 0.3;
    a.run(Command::Mode(Mode::Polygon));
    for _ in 0..2 {
        super::tests::draw(&mut a, &ctx, vec2(900., 600.), vec![]);
    }
    let builds = a.resident_cache.builds();
    a.editor.scene.selection.faces = vec![100, 101, 117, 5];
    for _ in 0..2 {
        super::tests::draw(&mut a, &ctx, vec2(900., 600.), vec![]);
    }
    assert_eq!(
        a.resident_cache.builds(),
        builds,
        "a selection rebuilt the item"
    );
    assert_eq!(a.viewport_gpu.as_ref().unwrap().patched_items(), 1);
    let after = a.viewport_gpu.as_ref().unwrap().read_views().remove(0).2;
    let Some((fresh, _, _)) = picture(
        wavy(16),
        &|a| {
            a.run(Command::Mode(Mode::Polygon));
            a.editor.scene.selection.faces = vec![100, 101, 117, 5];
        },
        true,
    ) else {
        return;
    };
    let most = after.iter().zip(&fresh).map(|(p, q)| p.abs_diff(*q)).max();
    assert!(most <= Some(2), "pictures differ by {most:?}/255");
}

/// Moving points of a patch surface updates the patches near them and
/// re-sends only what changed: no full rebuild of the item or the net, and
/// the picture matches the moved mesh drawn from scratch.
#[test]
fn moving_points_updates_the_patches_in_place() {
    let Some(state) = super::performance_tests::headless_gpu() else {
        println!("no graphics device");
        return;
    };
    crate::subd_gpu::set_min_pieces(0);
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = wavy(16).into();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::Subdivision,
    ));
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    a.camera.yaw += 0.4;
    a.camera.pitch -= 0.3;
    for _ in 0..2 {
        super::tests::draw(&mut a, &ctx, vec2(900., 600.), vec![]);
    }
    let (builds, updates) = (a.resident_cache.builds(), a.subdivision_cache.updates());
    let lift = |mesh: &mut operandi_core::Mesh, step: f64| {
        for v in [120usize, 121, 137] {
            mesh.vertices[v].z += step;
        }
    };
    let mut moved: operandi_core::Mesh = (*a.editor.scene.active().unwrap().mesh).clone();
    for _ in 0..3 {
        lift(&mut moved, 0.05);
        a.editor.scene.active_mut().unwrap().mesh = moved.clone().into();
        super::tests::draw(&mut a, &ctx, vec2(900., 600.), vec![]);
    }
    super::tests::draw(&mut a, &ctx, vec2(900., 600.), vec![]);
    assert_eq!(a.resident_cache.builds(), builds, "a drag rebuilt the item");
    // Each frame updates the patches (and the stored surface picking reads).
    assert!(
        a.subdivision_cache.updates() >= updates + 3,
        "patches weren't updated in place"
    );
    let (_, after_size, after) = a.viewport_gpu.as_ref().unwrap().read_views().remove(0);
    // The same view framed on the unmoved mesh, drawing the moved one from
    // scratch.
    let mut fresh_mesh = wavy(16);
    lift(&mut fresh_mesh, 0.15);
    let Some((fresh, _, _)) = picture(
        wavy(16),
        &move |a: &mut Operandi| {
            let mut m = fresh_mesh.clone();
            m.attributes.representations = a
                .editor
                .scene
                .active()
                .unwrap()
                .mesh
                .attributes
                .representations
                .clone();
            a.editor.scene.active_mut().unwrap().mesh = m.into();
        },
        true,
    ) else {
        return;
    };
    keep("dragged", &after, after_size);
    keep("dragged fresh", &fresh, after_size);
    let most = after.iter().zip(&fresh).map(|(p, q)| p.abs_diff(*q)).max();
    let count = after
        .chunks(4)
        .zip(fresh.chunks(4))
        .filter(|(p, q)| p != q)
        .count();
    assert!(
        most <= Some(2),
        "pictures differ by {most:?}/255 in {count} pixels"
    );
}

/// Fade Wireframe on Small Polygons on patch wire: a coarse surface (long
/// cage edges) draws as without it; a dense one loses most of its wire
/// while its outline stays.
#[test]
fn patch_wire_fades_only_where_cage_polygons_are_tiny() {
    let fade = |on: bool| move |a: &mut Operandi| a.viewport_preferences.fade_small_wire = on;
    // Seen square on (the sheet lies in X/Y), every cage edge is long.
    let front = |on: bool| {
        move |a: &mut Operandi| {
            a.viewport_preferences.fade_small_wire = on;
            a.camera.view = crate::camera::View::Front;
            a.frame(false);
        }
    };
    let (Some((coarse_on, _, _)), Some((coarse_off, _, _))) = (
        picture(wavy(8), &front(true), true),
        picture(wavy(8), &front(false), true),
    ) else {
        println!("no graphics device");
        return;
    };
    let count = coarse_on
        .chunks(4)
        .zip(coarse_off.chunks(4))
        .filter(|(p, q)| p != q)
        .count();
    assert!(
        coarse_on == coarse_off,
        "a coarse surface's wire changed in {count} pixels"
    );
    // A 200 × 100 globe: cage polygons still tiny on screen (was 400 × 200,
    // four times the work; test speed 2026-11-16).
    let (dense_on, size, _) = picture(globe(200, 100), &fade(true), true).unwrap();
    let (dense_off, _, _) = picture(globe(200, 100), &fade(false), true).unwrap();
    keep("fade on", &dense_on, size);
    keep("fade off", &dense_off, size);
    let changed = dense_on
        .chunks(4)
        .zip(dense_off.chunks(4))
        .filter(|(p, q)| p != q)
        .count();
    assert!(
        changed > 1000,
        "the dense wire didn't fade ({changed} pixels changed)"
    );
    // The outline: along each row, the leftmost and rightmost drawn sphere
    // pixels stay dark (the wire there is kept).
    let dark = |px: &[u8]| px[3] > 0 && (px[0] as u32 + px[1] as u32 + px[2] as u32) < 200;
    let w = size[0] as usize;
    let rows_with_dark_edges = (0..size[1] as usize)
        .filter(|&y| {
            let row = &dense_on[y * w * 4..(y + 1) * w * 4];
            row.chunks(4).any(&dark)
        })
        .count();
    assert!(
        rows_with_dark_edges > 20,
        "no outline left ({rows_with_dark_edges} rows)"
    );
}

/// An image layer on a patch surface draws per pixel with the surface's
/// UVs (SDV-15), as on the same surface frozen into polygons.
#[test]
fn image_layers_draw_on_patches_as_on_the_frozen_surface() {
    image_layers_match_the_frozen_surface(true);
}

/// The same on the stored CPU surface (a small mesh, or patches off): the
/// surface carries the cage's UV maps, so the image maps there too.
#[test]
fn image_layers_draw_on_the_stored_surface_as_on_the_frozen_surface() {
    image_layers_match_the_frozen_surface(false);
}

fn image_layers_match_the_frozen_surface(patches: bool) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("fixtures/viewport-image-textures/image-layers.opscene");
    let picture_of = |frozen: bool| -> Option<(Vec<u8>, [u32; 2])> {
        let state = super::performance_tests::headless_gpu()?;
        crate::subd_gpu::set_min_pieces(0);
        crate::subd_gpu::set_off(!patches);
        let ctx = Context::default();
        theme::install(&ctx);
        let mut a = Operandi::empty();
        a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
        let mut scene = operandi_core::persistence::load(&path).unwrap();
        for image in &mut scene.images {
            if std::path::Path::new(&image.path).is_relative() {
                image.path = root.join(&image.path).to_string_lossy().into_owned();
            }
        }
        let id = scene
            .items
            .iter()
            .find(|i| i.name == "UV Image")
            .unwrap()
            .id;
        a.editor.scene = scene;
        a.editor.scene.select_item(Some(id));
        // The plane's own UVs as a UV map, which Freeze carries over.
        let mut mesh: operandi_core::Mesh = (*a.editor.scene.active().unwrap().mesh).clone();
        let rows: Vec<Vec<Option<glam::DVec2>>> = mesh
            .faces
            .iter()
            .map(|f| {
                f.uv.clone()
                    .unwrap_or_default()
                    .into_iter()
                    .map(Some)
                    .collect()
            })
            .collect();
        let mut faces = (*mesh.faces).clone();
        for f in &mut faces {
            f.uv = None;
        }
        mesh.faces = faces.into();
        let mut maps = std::collections::BTreeMap::new();
        maps.insert("Texture".to_string(), rows);
        mesh.uv_maps = maps.into();
        mesh.attributes.representations =
            vec![operandi_core::PolygonRepresentation::Subdivs; mesh.faces.len()];
        a.editor.scene.active_mut().unwrap().mesh = mesh.clone().into();
        a.camera.view = crate::camera::View::Front;
        a.frame(false);
        // Fill only: the frozen surface's wire has every piece's edges.
        for group in [
            &mut a.display.settings.active,
            &mut a.display.settings.inactive,
        ] {
            group.overlay = crate::view_display::Overlay::None;
        }
        if frozen {
            a.editor.scene.active_mut().unwrap().mesh =
                operandi_core::subdivision::freeze(&mesh, &Default::default())
                    .unwrap()
                    .into();
        }
        for _ in 0..3 {
            super::tests::draw(&mut a, &ctx, vec2(900., 600.), vec![]);
        }
        crate::subd_gpu::set_off(false);
        if !frozen {
            assert_eq!(
                a.viewport_gpu.as_ref().unwrap().patched_items(),
                usize::from(patches)
            );
        }
        let (_, size, rgba) = a.viewport_gpu.as_ref().unwrap().read_views().remove(0);
        Some((rgba, size))
    };
    let (Some((patched, size)), Some((frozen, _))) = (picture_of(false), picture_of(true)) else {
        println!("no graphics device");
        return;
    };
    keep(
        if patches {
            "image patches"
        } else {
            "image stored"
        },
        &patched,
        size,
    );
    keep("image frozen", &frozen, size);
    let differ = patched
        .chunks(4)
        .zip(frozen.chunks(4))
        .filter(|(p, q)| p.iter().zip(*q).any(|(a, b)| a.abs_diff(*b) > 24))
        .count();
    let drawn = frozen.chunks(4).filter(|p| p[3] > 0).count();
    assert!(
        differ * 100 < drawn,
        "{differ} of {drawn} pixels differ by more than 24/255"
    );
}
