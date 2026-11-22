//! The viewport's pictures for a fixed set of scenes, written to a folder
//! and compared with another build's, so a drawing speed-up can show it
//! draws exactly what it did (perf task 3, 2026-11-13).
//!
//! `OPERANDI_CAPTURE_DIR=<new> [OPERANDI_CAPTURE_COMPARE=<old>] cargo test
//! --release -p operandi-app picture_capture -- --ignored --nocapture`
use super::*;

/// `n` x `n` quads on a wave, smooth shaded.
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

/// What a scene sets up.
type Setup = fn(&mut Operandi);

/// Each scene: (name, what to set up).
fn scenes() -> Vec<(&'static str, Setup)> {
    vec![
        ("polygon_selected", |a| {
            a.run(Command::Mode(Mode::Polygon));
            let mid = (24 / 2) * 24 + 24 / 2;
            a.editor.scene.selection.faces = vec![mid, mid + 1, mid + 24, 30, 31];
        }),
        ("polygon_none", |a| a.run(Command::Mode(Mode::Polygon))),
        ("item_selected", |a| a.run(Command::Mode(Mode::Item))),
        ("item_deselected", |a| {
            a.run(Command::Mode(Mode::Item));
            a.editor.scene.select_item(None);
        }),
        ("vertex_selected", |a| {
            a.run(Command::Mode(Mode::Vertex));
            let p = (24 / 2) * 25 + 24 / 2;
            a.editor.scene.selection.vertices = vec![p, p + 1, p + 25, 3];
        }),
        ("edge_selected", |a| {
            a.run(Command::Mode(Mode::Edge));
            let p = (24 / 2) * 25 + 24 / 2;
            a.editor.scene.selection.edges = vec![(p, p + 1), (p, p + 25)];
        }),
        ("subdivided_polygon", |a| {
            a.run(Command::ToggleSubdivision(
                operandi_core::subdivision::Toggle::CatmullClark,
            ));
            a.run(Command::Mode(Mode::Polygon));
            let mid = (24 / 2) * 24 + 24 / 2;
            a.editor.scene.selection.faces = vec![mid, mid + 1];
        }),
        ("many_items", |a| {
            // 48 small grids around the first, every third selected.
            a.run(Command::Mode(Mode::Item));
            let mut ids = Vec::new();
            for k in 0..48 {
                a.editor.scene.new_mesh().unwrap();
                let item = a.editor.scene.items.last_mut().unwrap();
                item.mesh = wavy(3).into();
                item.transform.translation =
                    DVec3::new((k % 8) as f64 * 0.3 - 1., (k / 8) as f64 * 0.3 - 0.8, 0.2);
                item.transform.scale = DVec3::splat(0.25);
                ids.push(item.id);
            }
            let chosen: Vec<u64> = ids.iter().copied().step_by(3).collect();
            a.editor.scene.selection.item = Some(chosen[0]);
            a.editor.scene.selection.also = chosen[1..].to_vec();
        }),
        ("subdivided_item", |a| {
            a.run(Command::ToggleSubdivision(
                operandi_core::subdivision::Toggle::CatmullClark,
            ));
            a.run(Command::Mode(Mode::Item));
        }),
    ]
}

#[test]
#[ignore = "opt-in picture capture and comparison (OPERANDI_CAPTURE_DIR)"]
fn picture_capture() {
    let Some(dir) = std::env::var_os("OPERANDI_CAPTURE_DIR").map(PathBuf::from) else {
        println!("OPERANDI_CAPTURE_DIR not set");
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let compare = std::env::var_os("OPERANDI_CAPTURE_COMPARE").map(PathBuf::from);
    // OPERANDI_CAPTURE_TOLERANCE: the largest channel difference allowed
    // (0: exactly the same pictures).
    let tolerance: u8 = std::env::var("OPERANDI_CAPTURE_TOLERANCE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), 2f32));
    let mut differ = Vec::new();
    for (name, setup) in scenes() {
        for (view_name, view) in [
            ("persp", crate::camera::View::Perspective),
            ("top", crate::camera::View::Top),
        ] {
            let state = super::performance_tests::headless_gpu().expect("graphics device");
            let mut a = Operandi::empty();
            a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
            // The wire fade (2026-11-13) is a new look; compare without it.
            a.viewport_preferences.fade_small_wire = false;
            a.run(Command::NewMesh);
            a.editor.scene.active_mut().unwrap().mesh = wavy(24).into();
            a.camera.view = view;
            a.frame(false);
            setup(&mut a);
            if view == crate::camera::View::Perspective {
                a.camera.yaw += 0.4;
                a.camera.pitch -= 0.3;
            }
            for _ in 0..3 {
                super::tests::draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
            }
            for (v, size, rgba) in a.viewport_gpu.as_ref().unwrap().read_views() {
                let file = format!("{name}_{view_name}_{v}.rgba");
                std::fs::write(dir.join(&file), &rgba).unwrap();
                let lit = rgba.chunks(4).filter(|p| p[3] > 0).count();
                println!("CAPTURE {file} {}x{} lit_pixels={lit}", size[0], size[1]);
                if let Some(old) = &compare {
                    match std::fs::read(old.join(&file)) {
                        Ok(before) if before == rgba => {}
                        Ok(before) if before.len() != rgba.len() => {
                            differ.push(format!("{file}: a different size"))
                        }
                        Ok(before) => {
                            let changed = before
                                .chunks(4)
                                .zip(rgba.chunks(4))
                                .filter(|(p, q)| p != q)
                                .count();
                            let most = before
                                .iter()
                                .zip(&rgba)
                                .map(|(p, q)| p.abs_diff(*q))
                                .max()
                                .unwrap_or(0);
                            println!(
                                "CAPTURE {file}: {changed} pixels differ, by at most {most}/255"
                            );
                            if most > tolerance {
                                differ.push(format!(
                                    "{file}: {changed} pixels differ, by at most {most}/255"
                                ));
                            }
                        }
                        Err(e) => differ.push(format!("{file}: no earlier picture ({e})")),
                    }
                }
            }
        }
    }
    assert!(
        differ.is_empty(),
        "pictures changed:\n{}",
        differ.join("\n")
    );
}
