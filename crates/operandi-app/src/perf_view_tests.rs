//! Viewport drawing timings on generated plain (unsubdivided) meshes
//! (perf task 3, 2026-11-13): navigation frames and the switch into and out
//! of Item mode, waiting for the viewport's own graphics device each frame.
//!
//! `OPERANDI_PERF_DIR=<folder> OPERANDI_PERF_SCENES=edit1m,flat10m cargo
//! test --release -p operandi-app perf_view -- --ignored --nocapture`
use super::perf_drags_tests::{Bench, drag, ms, one, report, undo_redo};
use super::*;
use std::time::Instant;

fn orbit(b: &Bench, a: &mut Operandi, name: &str, frames: usize) {
    b.frame(a, vec![]);
    let mut times: Vec<f64> = (0..frames)
        .map(|_| {
            a.camera.yaw += std::f64::consts::TAU / frames as f64;
            b.frame(a, vec![])
        })
        .collect();
    report(&format!("{name}/orbit"), &mut times);
}

fn scene(b: &Bench, path: &std::path::Path, name: &str) {
    let state = super::performance_tests::headless_gpu().expect("graphics device");
    // Time against the device the viewport draws with.
    let b = &Bench {
        ctx: b.ctx.clone(),
        device: state.device.clone(),
        size: b.size,
    };
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.camera.view = crate::camera::View::Perspective;
    b.frame(&mut a, vec![]);
    let t = Instant::now();
    a.open(path.to_path_buf());
    let first = b.frame(&mut a, vec![]);
    one(&format!("{name}/open_total"), ms(t));
    one(&format!("{name}/open_first_frame"), first);
    let polygons: usize = a
        .editor
        .scene
        .items
        .iter()
        .map(|i| i.mesh.faces.len())
        .sum();
    // What the views draw (the GL triangle count overlay's number).
    let drawn = a.drawn_triangles();
    let smooth = a
        .editor
        .scene
        .items
        .iter()
        .any(|i| operandi_core::subdivision::has_smooth(&i.mesh));
    println!("PERF {name}/info polygons={polygons} drawn_triangles={drawn} smooth={smooth}");
    for item in a.editor.scene.items.iter().take(1) {
        let settings = item.subdivision.clone().unwrap_or_default();
        match operandi_core::subdivision::display_surface_edges(&item.mesh, &settings) {
            Ok((mesh, _, _)) => println!(
                "PERF {name}/surface level={} pieces={}",
                settings.catmull_clark_level,
                mesh.faces.len()
            ),
            Err(e) => println!(
                "PERF {name}/surface level={} error={e}",
                settings.catmull_clark_level
            ),
        }
    }
    a.run(Command::Mode(Mode::Polygon));
    a.frame(false);
    b.frame(&mut a, vec![]);
    orbit(b, &mut a, &format!("{name}/polygon_mode"), 90);
    for mode in [Mode::Item, Mode::Polygon, Mode::Item] {
        let t = Instant::now();
        a.run(Command::Mode(mode));
        b.frame(&mut a, vec![]);
        one(&format!("{name}/switch_to_{mode:?}"), ms(t));
    }
    orbit(b, &mut a, &format!("{name}/item_mode"), 90);
    // Deselect and select the item in Item mode.
    let id = a.editor.scene.items[0].id;
    for select in [None, Some(id)] {
        let t = Instant::now();
        a.editor.scene.select_item(select);
        b.frame(&mut a, vec![]);
        one(
            &format!(
                "{name}/item_{}",
                if select.is_some() {
                    "select"
                } else {
                    "deselect"
                }
            ),
            ms(t),
        );
    }
}

#[test]
#[ignore = "opt-in viewport timings on generated scenes (OPERANDI_PERF_DIR)"]
fn perf_view() {
    let Some(dir) = std::env::var_os("OPERANDI_PERF_DIR").map(PathBuf::from) else {
        println!("OPERANDI_PERF_DIR not set");
        return;
    };
    let scenes = std::env::var("OPERANDI_PERF_SCENES").unwrap_or_else(|_| "edit1m,flat10m".into());
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), 2f32));
    let state = super::performance_tests::headless_gpu().expect("graphics device");
    println!("PERF adapter {:?}", state.adapter.get_info());
    let b = Bench {
        ctx,
        device: state.device.clone(),
        size: vec2(1440., 900.),
    };
    for name in scenes.split(',') {
        let path = dir.join(format!("{name}.opscene"));
        if path.exists() {
            scene(&b, &path, name);
        } else {
            println!("PERF {name} missing {}", path.display());
        }
    }
}

/// Many separate objects (perf task 2): orbit in Item mode, click-select,
/// select one, drag one, select all, drag all, undo and redo.
/// `OPERANDI_PERF_DIR=<folder> cargo test --release -p operandi-app
/// perf_objects -- --ignored --nocapture` (scene `objects10k`).
#[test]
#[ignore = "opt-in viewport timings on generated scenes (OPERANDI_PERF_DIR)"]
fn perf_objects() {
    let Some(dir) = std::env::var_os("OPERANDI_PERF_DIR").map(PathBuf::from) else {
        println!("OPERANDI_PERF_DIR not set");
        return;
    };
    let name = std::env::var("OPERANDI_PERF_SCENES").unwrap_or_else(|_| "objects10k".into());
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), 2f32));
    let state = super::performance_tests::headless_gpu().expect("graphics device");
    let b = &Bench {
        ctx,
        device: state.device.clone(),
        size: vec2(1440., 900.),
    };
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.camera.view = crate::camera::View::Perspective;
    b.frame(&mut a, vec![]);
    let t = Instant::now();
    a.open(dir.join(format!("{name}.opscene")));
    let first = b.frame(&mut a, vec![]);
    one(&format!("{name}/open_total"), ms(t));
    one(&format!("{name}/open_first_frame"), first);
    println!("PERF {name}/info items={}", a.editor.scene.items.len());
    a.run(Command::Mode(Mode::Item));
    a.frame(false);
    b.frame(&mut a, vec![]);
    // OPERANDI_PERF_STILL_FRAMES: a longer still phase, for a profiler.
    let still_frames = std::env::var("OPERANDI_PERF_STILL_FRAMES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);
    let mut still: Vec<f64> = (0..still_frames).map(|_| b.frame(&mut a, vec![])).collect();
    report(&format!("{name}/still"), &mut still);
    orbit(b, &mut a, &format!("{name}/item_mode"), 60);
    let c = a.viewport_rect.center();
    let t = Instant::now();
    b.frame(
        &mut a,
        vec![
            Event::PointerMoved(c),
            super::tests::pointer(c, true, Modifiers::NONE),
        ],
    );
    b.frame(
        &mut a,
        vec![super::tests::pointer(c, false, Modifiers::NONE)],
    );
    one(&format!("{name}/click_select"), ms(t));
    let ids: Vec<u64> = a.editor.scene.items.iter().map(|i| i.id).collect();
    let t = Instant::now();
    a.editor.scene.select_item(Some(ids[ids.len() / 2]));
    b.frame(&mut a, vec![]);
    one(&format!("{name}/select_one"), ms(t));
    a.activate(Tool::Move);
    let (mut times, release) = drag(b, &mut a, 40);
    report(&format!("{name}/drag_one"), &mut times);
    one(&format!("{name}/drag_one_release"), release);
    undo_redo(b, &mut a, &format!("{name}/drag_one"));
    a.run(Command::Drop);
    let t = Instant::now();
    a.editor.scene.selection.item = Some(ids[0]);
    a.editor.scene.selection.also = ids[1..].to_vec();
    b.frame(&mut a, vec![]);
    one(&format!("{name}/select_all"), ms(t));
    a.activate(Tool::Move);
    let (mut times, release) = drag(b, &mut a, 40);
    report(&format!("{name}/drag_all"), &mut times);
    one(&format!("{name}/drag_all_release"), release);
    undo_redo(b, &mut a, &format!("{name}/drag_all"));
    a.run(Command::Drop);
}
