//! The kept renderer (`Session`, oc_session_*): a render through a session
//! that has rendered something else before matches a render from scratch,
//! whatever changed in between (after a cancel, up to noise), and equal
//! meshes share their points.
#![cfg_attr(not(feature = "cycles"), allow(dead_code))]
use super::*;
use operandi_core::mesh::Mesh;

/// Two boxes side by side over a floor, lit by the studio sun.
fn two_boxes() -> Scene {
    let mut scene = Scene::default();
    for x in [-0.8, 0.8] {
        scene.new_mesh().unwrap();
        let item = scene.items.last_mut().unwrap();
        item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        item.transform.translation = DVec3::new(x, 0.0, 0.0);
    }
    scene.new_mesh().unwrap();
    let floor = scene.items.last_mut().unwrap();
    floor.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::new(6.0, 0.1, 6.0), 1)
        .unwrap()
        .into();
    floor.transform.translation = DVec3::new(0.0, -0.55, 0.0);
    scene
}

fn make_red(scene: &mut Scene) {
    for n in &mut scene.shader_tree.nodes {
        if let NodeKind::Material(m) = &mut n.kind {
            m.diffuse_color = [0.9, 0.1, 0.1];
        }
    }
}

fn red_boxes() -> Scene {
    let mut scene = two_boxes();
    make_red(&mut scene);
    scene
}

fn settings(eye: DVec3) -> Settings {
    let forward = (-eye).normalize();
    let right = forward.cross(DVec3::Y).normalize();
    let up = right.cross(forward);
    let mut s = Settings::studio(48, 32, View::perspective(eye, right, up, forward, 0.9));
    s.samples = 8;
    s.gpu = false;
    s.denoise = false;
    s.threads = 2;
    s
}

/// Mean absolute difference per channel.
fn difference(a: &Image, b: &Image) -> f32 {
    assert_eq!(a.rgba.len(), b.rgba.len());
    a.rgba
        .iter()
        .zip(&b.rgba)
        .map(|(x, y)| (x - y).abs())
        .sum::<f32>()
        / a.rgba.len() as f32
}

fn fresh(scene: &Scene, s: &Settings) -> Image {
    render(&scene_meshes(scene).unwrap(), s).unwrap()
}

fn kept(session: &mut Session, scene: &Scene, s: &Settings) -> Image {
    session
        .render(
            &scene_meshes(scene).unwrap(),
            s,
            None,
            &RenderControl::default(),
        )
        .unwrap()
}

/// Cycles' sampling is seeded the same way each render, so the same scene
/// renders the same picture: a kept session must too.
const SAME: f32 = 1e-4;

#[cfg(feature = "cycles")]
#[test]
fn kept_session_renders_what_a_fresh_one_does() {
    let mut scene = two_boxes();
    let near = settings(DVec3::new(0.0, 1.5, 4.0));
    let mut session = Session::new();
    let first = kept(&mut session, &scene, &near);
    assert!(
        difference(&first, &fresh(&scene, &near)) < SAME,
        "first render"
    );
    assert!(session.is_kept());

    // Camera only.
    let side = settings(DVec3::new(3.0, 1.0, 3.0));
    let moved_camera = kept(&mut session, &scene, &side);
    assert!(
        difference(&moved_camera, &fresh(&scene, &side)) < SAME,
        "camera"
    );
    assert!(difference(&moved_camera, &first) > 0.01, "the view changed");

    // An item moved: only its transform changes.
    scene.items[0].transform.translation.y += 0.4;
    let moved_item = kept(&mut session, &scene, &side);
    assert!(
        difference(&moved_item, &fresh(&scene, &side)) < SAME,
        "item"
    );
    assert!(
        difference(&moved_item, &moved_camera) > 1e-3,
        "the box moved"
    );

    // The material changed: a new surface on the same points.
    make_red(&mut scene);
    let red = kept(&mut session, &scene, &side);
    assert!(difference(&red, &fresh(&scene, &side)) < SAME, "material");

    // An item deleted, a light added, a bigger picture.
    scene.items.remove(1);
    let mut wide = settings(DVec3::new(0.0, 2.0, 4.0));
    wide.width = 64;
    wide.sun_strength *= 2.0;
    let fewer = kept(&mut session, &scene, &wide);
    assert!(
        difference(&fewer, &fresh(&scene, &wide)) < SAME,
        "fewer items"
    );
}

/// The Preview's grain (2026-11-18): a Cycles session whose first render
/// didn't denoise handed back undenoised pictures ever after, so the
/// Preview's coarse pass (once without denoising) left every full pass
/// noisy. Turning Denoise on or off between renders must denoise like a
/// fresh render does.
#[cfg(feature = "cycles")]
#[test]
fn denoise_turned_on_in_a_kept_session_denoises() {
    let scene = two_boxes();
    let raw = settings(DVec3::new(0.0, 1.5, 4.0));
    let mut denoised = raw.clone();
    denoised.denoise = true;
    let clean = fresh(&scene, &denoised);
    assert!(clean.denoised);
    assert!(
        difference(&clean, &fresh(&scene, &raw)) > 1e-3,
        "denoising changes the picture"
    );

    let mut session = Session::new();
    let mut coarse = raw.clone();
    coarse.samples = 1;
    kept(&mut session, &scene, &coarse);
    let after = kept(&mut session, &scene, &denoised);
    assert!(difference(&after, &clean) < SAME, "denoise turned on");

    // And off again.
    let noisy = kept(&mut session, &scene, &raw);
    assert!(
        difference(&noisy, &fresh(&scene, &raw)) < SAME,
        "denoise turned off"
    );
}

#[cfg(feature = "cycles")]
#[test]
fn a_cancelled_render_leaves_the_session_usable() {
    let scene = two_boxes();
    let s = settings(DVec3::new(0.0, 1.5, 4.0));
    let mut session = Session::new();
    kept(&mut session, &scene, &s);
    // A long render, cancelled while it samples (as a Preview restart does).
    let mut long = s.clone();
    long.samples = 100_000;
    let control = std::sync::Arc::new(RenderControl::default());
    let watcher = {
        let control = std::sync::Arc::clone(&control);
        std::thread::spawn(move || {
            while control.progress().samples_done == 0 {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            control.cancel();
        })
    };
    let meshes = scene_meshes(&scene).unwrap();
    assert!(session.render(&meshes, &long, None, &control).is_err());
    watcher.join().unwrap();
    assert!(
        session.is_kept(),
        "the cancel came after Cycles took the scene in"
    );
    // After a cancel Cycles carries on with other random numbers, so the
    // picture is the same render with different noise: it converges to the
    // fresh one as samples grow (noise falls like 1 / sqrt(samples)).
    let mut gap = |samples: u32| {
        let mut s = s.clone();
        s.samples = samples;
        difference(&kept(&mut session, &scene, &s), &fresh(&scene, &s))
    };
    let (rough, fine) = (gap(8), gap(128));
    assert!(fine < rough / 2.5 && fine < 0.004, "{rough} then {fine}");
}

#[cfg(feature = "cycles")]
#[test]
fn the_kept_preview_renderer_can_be_released() {
    let scene = two_boxes();
    let s = settings(DVec3::new(0.0, 1.5, 4.0));
    let meshes = scene_meshes(&scene).unwrap();
    let a = render_kept(&meshes, &s, None, &RenderControl::default()).unwrap();
    release_kept();
    let b = render_kept(&meshes, &s, None, &RenderControl::default()).unwrap();
    release_kept();
    assert!(difference(&a, &b) < SAME);
}

#[test]
fn equal_meshes_in_different_places_share_their_points() {
    let scene = two_boxes();
    let meshes = scene_meshes(&scene).unwrap();
    // The two boxes: the same points in their own space, placed apart.
    assert_eq!(meshes[0].positions, meshes[1].positions);
    assert_eq!(meshes[0].normals, meshes[1].normals);
    assert_ne!(meshes[0].transform, meshes[1].transform);
    #[cfg(feature = "cycles")]
    assert_eq!(ffi::geometry_key(&meshes[0]), ffi::geometry_key(&meshes[1]));
}

#[test]
fn transforms_round_trip() {
    let m = DMat4::from_scale_rotation_translation(
        DVec3::new(1.0, 2.0, 3.0),
        glam::DQuat::from_rotation_y(0.7),
        DVec3::new(4.0, 5.0, 6.0),
    );
    let back = rows_matrix(transform_rows(m));
    assert!(back.abs_diff_eq(m, 1e-6), "{back:?}");
    let mesh = RenderMesh {
        positions: vec![[1.0, 0.0, 0.0]],
        transform: Some(transform_rows(m)),
        ..Default::default()
    };
    let p = m.transform_point3(DVec3::X).as_vec3().to_array();
    let w = mesh.world_positions()[0];
    assert!((0..3).all(|i| (w[i] - p[i]).abs() < 1e-5));
}

#[test]
fn image_files_note_size_and_time() {
    let dir = std::env::temp_dir().join(format!("oc-session-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("a.png");
    std::fs::write(&path, b"one").unwrap();
    let name = path.to_string_lossy().into_owned();
    let mut mesh = scene_meshes(&red_boxes()).unwrap().remove(0);
    let shader = mesh.shader.as_mut().expect("a red surface");
    shader.images = vec![name.clone(), name.clone(), "/no/such/file.png".into()];
    let s = settings(DVec3::Z);
    let files = image_files(std::slice::from_ref(&mesh), &s);
    assert_eq!(files.len(), 2, "each file once");
    let mine = files.iter().find(|(p, _)| *p == name).unwrap();
    assert_eq!(mine.1.map(|(len, _)| len), Some(3));
    assert!(
        files.iter().any(|(_, stamp)| stamp.is_none()),
        "missing file"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Prints how long a restart takes from scratch and through a kept session,
/// for a camera move on a ~128k-triangle scene. Run with
/// `cargo test --release --features cycles session_speed -- --ignored --nocapture`
/// (add OPERANDI_PROBE_GPU=1 for the GPU).
#[cfg(feature = "cycles")]
#[test]
#[ignore]
fn session_speed_probe() {
    let mut scene = Scene::default();
    for k in 0..25 {
        scene.new_mesh().unwrap();
        let item = scene.items.last_mut().unwrap();
        item.mesh =
            operandi_core::radial::tessellated_sphere(DVec3::ZERO, DVec3::splat(0.4), 4, 1, false)
                .unwrap()
                .into();
        item.transform.translation = DVec3::new((k % 5) as f64 - 2.0, 0.0, (k / 5) as f64 - 2.0);
    }
    let gpu = std::env::var_os("OPERANDI_PROBE_GPU").is_some();
    let at = |eye: DVec3| {
        let mut s = settings(eye);
        s.width = 320;
        s.height = 200;
        s.samples = 1;
        s.gpu = gpu;
        s.threads = 0;
        s
    };
    let meshes = scene_meshes(&scene).unwrap();
    let triangles: usize = meshes.iter().map(|m| m.triangles.len()).sum();
    let time = |f: &mut dyn FnMut()| {
        let t = std::time::Instant::now();
        f();
        t.elapsed().as_secs_f64() * 1000.0
    };
    let control = RenderControl::default();
    let mut session = Session::new();
    let eyes = [
        DVec3::new(0.0, 4.0, 7.0),
        DVec3::new(1.0, 4.0, 7.0),
        DVec3::new(2.0, 4.0, 7.0),
    ];
    render_live(&meshes, &at(eyes[0]), None, &control).unwrap();
    let fresh_ms = time(&mut || {
        render_live(&meshes, &at(eyes[1]), None, &control).unwrap();
    });
    session
        .render(&meshes, &at(eyes[0]), None, &control)
        .unwrap();
    let kept_ms = time(&mut || {
        session
            .render(&meshes, &at(eyes[1]), None, &control)
            .unwrap();
    });
    scene.items[3].transform.translation.y += 0.5;
    let moved = scene_meshes(&scene).unwrap();
    let item_ms = time(&mut || {
        session
            .render(&moved, &at(eyes[2]), None, &control)
            .unwrap();
    });
    println!(
        "{triangles} triangles, {}: fresh {fresh_ms:.0} ms, kept camera move {kept_ms:.0} ms, \
         kept item move {item_ms:.0} ms",
        if gpu { "GPU" } else { "CPU" }
    );
}
