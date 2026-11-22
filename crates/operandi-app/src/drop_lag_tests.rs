//! Opt-in probe: what a Transform drag, its release and the frames after
//! it cost in a heavy scene (bug report "Lag after modeling operation on
//! a large model"). Generated scene: `P_ITEMS` meshes (400) of `P_N` x
//! `P_N` quads (27); `P_SCENE=<file>` opens a local scene instead (add
//! `--features usd` for USD). `P_ROUNDS` drags (3), `P_FACE` first
//! selected polygon. Run:
//! `cargo test --release -p operandi-app drop_lag_probe -- --ignored --nocapture`
use super::*;
use std::time::Instant;

fn grid(n: u32, x0: f64, y0: f64) -> operandi_core::Mesh {
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            let (x, y) = (i as f64 / n as f64, j as f64 / n as f64);
            mesh.vertices
                .push(DVec3::new(x0 + x, y0 + y, 0.05 * (x * 9.).sin()));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + n + 2, a + n + 1],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    mesh
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.
}

#[test]
#[ignore = "opt-in timing probe; see the file comment"]
fn drop_lag_probe() {
    let items: u32 = std::env::var("P_ITEMS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(400);
    let n: u32 = std::env::var("P_N")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(27);
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = Operandi::empty();
    if let Some(state) = super::performance_tests::headless_gpu() {
        println!("gpu on");
        a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    }
    a.camera.view = crate::camera::View::Perspective;
    let t = Instant::now();
    // P_SCENE: a local scene file instead of the generated one (local
    // measurement only; never commit a user scene or its results).
    if let Some(path) = std::env::var_os("P_SCENE").map(PathBuf::from) {
        a.initialize_new_document();
        a.open(path);
        if let Some(dialog) = a.usd_import.take() {
            a.execute(&crate::usd_import_dialog::command_line(
                &dialog.path,
                &dialog.form.values,
            ));
        }
        println!("{}", a.status);
    } else {
        for k in 0..items {
            a.run(Command::NewMesh);
            let (x, y) = ((k % 20) as f64 * 1.2, (k / 20) as f64 * 1.2);
            a.editor.scene.active_mut().unwrap().mesh = grid(n, x, y).into();
        }
    }
    println!("built in {:.0} ms", ms(t));
    let first = a
        .editor
        .scene
        .items
        .iter()
        .max_by_key(|i| i.mesh.read().as_read().polygon_count())
        .unwrap()
        .id;
    a.editor.scene.selection.item = Some(first);
    println!(
        "items {}, polygons {}, active has {}",
        a.editor.scene.items.len(),
        a.editor
            .scene
            .items
            .iter()
            .map(|i| i.mesh.read().as_read().polygon_count())
            .sum::<usize>(),
        a.editor
            .scene
            .item(first)
            .unwrap()
            .mesh
            .read()
            .as_read()
            .polygon_count()
    );
    a.editor.scene.selection.also.clear();
    a.frame(false);
    for _ in 0..3 {
        super::tests::draw(&mut a, &ctx, size, vec![]);
    }
    a.run(Command::Mode(Mode::Polygon));
    let f0: u32 = std::env::var("P_FACE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            a.editor
                .scene
                .active()
                .map_or(0, |i| i.mesh.faces.len() as u32 / 2)
        });
    a.editor.scene.selection.faces = (f0..f0 + 8).collect();
    a.activate(Tool::Transform);
    for _ in 0..3 {
        super::tests::draw(&mut a, &ctx, size, vec![]);
    }
    let mut still = vec![];
    for _ in 0..4 {
        let t = Instant::now();
        super::tests::draw(&mut a, &ctx, size, vec![]);
        still.push(ms(t));
    }
    println!("still frames {still:.1?}");
    let r = a.viewport_rect;
    let start = (0..200)
        .flat_map(|y| (0..300).map(move |x| (x, y)))
        .map(|(x, y)| r.left_top() + vec2(x as f32 * 4. + 20., y as f32 * 4. + 20.))
        .find(|&p| a.handle_hit(p, r).is_some())
        .expect("handle");
    let rounds: usize = std::env::var("P_ROUNDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    for round in 0..rounds {
        super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![
                Event::PointerMoved(start),
                super::tests::pointer(start, true, Modifiers::NONE),
            ],
        );
        let mut end = start;
        let mut worst = 0f64;
        for k in 0..20 {
            end = start + vec2(k as f32 * 0.7, k as f32 * -0.5);
            let t = Instant::now();
            super::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(end)]);
            if k > 3 {
                worst = worst.max(ms(t));
            }
        }
        let before = a.rebuild_totals();
        let t = Instant::now();
        super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![super::tests::pointer(end, false, Modifiers::NONE)],
        );
        let release = ms(t);
        let mut after = vec![];
        for _ in 0..4 {
            let t = Instant::now();
            super::tests::draw(&mut a, &ctx, size, vec![]);
            after.push(ms(t));
        }
        println!(
            "round {round}: drag worst {worst:.1} ms, release {release:.1} ms, next frames {after:.1?}\n  rebuilds before {before:?}\n  after {:?}",
            a.rebuild_totals()
        );
    }
    let t = Instant::now();
    a.run(Command::Drop);
    println!("drop {:.1} ms", ms(t));
}
