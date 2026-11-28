//! Opt-in CPU benchmarks using original analytic geometry, never user scenes.
//! Run alone in release mode; timings include result destruction, exclude setup.
use super::*;
use std::{hint::black_box, time::Instant};

fn measure(name: &str, mut operation: impl FnMut()) {
    if std::env::var("OPERANDI_BENCH_FILTER").is_ok_and(|s| !name.contains(&s)) {
        return;
    }
    let samples = std::env::var("OPERANDI_BENCH_SAMPLES")
        .map(|s| s.parse::<usize>().expect("positive sample count"))
        .unwrap_or(7);
    assert!(samples > 0);
    operation(); // Warm caches and any lazy UI resources outside measurements.
    let mut times = Vec::with_capacity(samples);
    let mut counts = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        let mut count = 0;
        loop {
            operation();
            count += 1;
            if start.elapsed().as_millis() >= 30 {
                break;
            }
        }
        times.push(start.elapsed().as_secs_f64() * 1000. / count as f64);
        counts.push(count);
    }
    times.sort_by(f64::total_cmp);
    println!(
        "PERF {name} median_ms={:.6} min_ms={:.6} max_ms={:.6} samples={samples} iterations={counts:?}",
        times[samples / 2],
        times[0],
        times[samples - 1]
    );
}

fn scene(sides: u32, segments: u32) -> Scene {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.active_mut().unwrap().mesh = operandi_core::globe_geometry(
        DVec3::ZERO,
        RadialParameters {
            radius: DVec3::ONE,
            sides,
            segments,
        },
    )
    .unwrap()
    .into();
    // Original corner attributes make copying/serialization representative.
    for (i, face) in scene
        .active_mut()
        .unwrap()
        .mesh
        .faces
        .iter_mut()
        .enumerate()
    {
        face.uv = Some(
            (0..face.vertices.len())
                .map(|c| glam::DVec2::new(i as f64, c as f64))
                .collect(),
        );
        face.material = "benchmark material".into();
        face.part = Some("original analytic fixture".into());
    }
    scene.validate().unwrap();
    scene
}

#[test]
#[ignore = "opt-in optimized CPU benchmark"]
fn modeling_workloads() {
    for (label, sides, segments) in [("small", 24, 12), ("large", 128, 64)] {
        let scene = scene(sides, segments);
        let mesh = &scene.active().unwrap().mesh;
        let edges = mesh.edges();
        println!(
            "FIXTURE {label} sides={sides} segments={segments} vertices={} edges={} faces={} corners={}",
            mesh.vertices.len(),
            edges.len(),
            mesh.faces.len(),
            mesh.faces.iter().map(|f| f.vertices.len()).sum::<usize>()
        );
        measure(&format!("{label}/edges"), || {
            black_box(mesh.edges());
        });
        measure(&format!("{label}/mesh_validate"), || {
            black_box(mesh.validate()).unwrap();
        });
        measure(&format!("{label}/surface_build"), || {
            black_box(picking::Surface::new(&scene));
        });
        for count in [0, 32, 256] {
            let mut selected = scene.clone();
            selected.selection.mode = Mode::Edge;
            selected.selection.edges = edges.iter().copied().take(count).collect();
            measure(&format!("{label}/scene_validate_{count}"), || {
                black_box(selected.validate()).unwrap();
            });
        }
        let bytes = persistence::encode(&scene).unwrap();
        measure(&format!("{label}/encode"), || {
            black_box(persistence::encode(&scene)).unwrap();
        });
        measure(&format!("{label}/decode"), || {
            black_box(persistence::decode(&bytes)).unwrap();
        });
        let mut editor = Editor {
            scene: scene.clone(),
            ..Default::default()
        };
        editor.scene.selection.mode = Mode::Edge;
        editor.scene.selection.edges = edges.iter().copied().take(32).collect();
        editor.dispatch(Command::Activate(Tool::Scale)).unwrap();
        let p = Parameters {
            delta: DVec3::splat(0.95),
            ..Default::default()
        };
        measure(&format!("{label}/scale_preview_32_edges"), || {
            editor
                .dispatch(Command::Preview(black_box(p.clone())))
                .unwrap();
        });
        editor.dispatch(Command::Cancel).unwrap();
        editor.scene.selection.mode = Mode::Polygon;
        editor.scene.selection.edges.clear();
        editor.scene.selection.faces = vec![segments + 2];
        editor.dispatch(Command::Activate(Tool::Bevel)).unwrap();
        let p = Parameters {
            inset: 0.0001,
            shift: 0.001,
            ..Default::default()
        };
        measure(&format!("{label}/bevel_preview"), || {
            editor
                .dispatch(Command::Preview(black_box(p.clone())))
                .unwrap();
        });
        editor.dispatch(Command::Cancel).unwrap();
        editor.dispatch(Command::Activate(Tool::Extrude)).unwrap();
        let p = Parameters {
            extent: DVec3::Y * 0.001,
            ..Default::default()
        };
        measure(&format!("{label}/extrude_preview"), || {
            editor
                .dispatch(Command::Preview(black_box(p.clone())))
                .unwrap();
        });
        let ctx = Context::default();
        theme::install(&ctx);
        let mut app = Operandi::empty();
        app.editor.scene = scene.clone();
        app.camera.view = View::Front;
        for mode in [Mode::Polygon, Mode::Vertex, Mode::Edge] {
            app.editor.scene.selection.mode = mode;
            app.editor.scene.selection.faces.clear();
            app.editor.scene.selection.vertices.clear();
            app.editor.scene.selection.edges.clear();
            match mode {
                Mode::Polygon => {
                    app.editor.scene.selection.faces = (0..mesh.faces.len() as u32).collect()
                }
                Mode::Vertex => {
                    app.editor.scene.selection.vertices = (0..mesh.vertices.len() as u32).collect()
                }
                Mode::Edge => app.editor.scene.selection.edges = edges.clone(),
                Mode::Item => unreachable!(),
            }
            measure(&format!("{label}/cpu_frame_{mode:?}_all"), || {
                black_box(tests::draw(
                    &mut app,
                    &ctx,
                    vec2(1440., 900.),
                    vec![Event::PointerMoved(pos2(700., 450.))],
                ));
            });
            assert_eq!(app.editor.scene.items, scene.items);
        }
    }
}

/// STAB-00 instrumentation only: production behavior is unchanged. These CPU
/// calls exclude native input, GPU submission and compositor presentation.
#[test]
#[ignore = "opt-in STAB-00 baseline; requires isolated OPERANDI_DEV_DATA"]
fn stabilization_baseline() {
    let root = PathBuf::from(std::env::var_os("OPERANDI_DEV_DATA").expect("isolated data root"));
    assert!(root.is_dir());
    assert!(!root.join("recovery.opscene").exists());
    for (label, sides, segments) in [("small", 24, 12), ("large", 128, 64)] {
        let source = scene(sides, segments);
        let mut editor = Editor {
            scene: source.clone(),
            ..Default::default()
        };
        editor.synchronize_identities().unwrap();
        for depth in [0, 10, 100] {
            while editor.history.undo.len() < depth {
                editor
                    .dispatch(Command::Rename(format!(
                        "history-{}",
                        editor.history.undo.len()
                    )))
                    .unwrap();
            }
            println!(
                "STAB_FIXTURE {label} history={} redo={} encoded_bytes={}",
                editor.history.undo.len(),
                editor.history.redo.len(),
                persistence::encode(&editor.scene).unwrap().len()
            );
            measure(&format!("{label}/editor_clone_history_{depth}"), || {
                black_box(editor.clone());
            });
            if depth > 0 {
                let expected = editor.scene.clone();
                measure(&format!("{label}/undo_redo_pair_history_{depth}"), || {
                    editor.dispatch(Command::Undo).unwrap();
                    editor.dispatch(Command::Redo).unwrap();
                });
                assert_eq!(editor.scene, expected);
            }
        }
        let path = root.join(format!("{label}-save.opscene"));
        measure(&format!("{label}/save_sync_rename"), || {
            persistence::save(&path, &source).unwrap();
        });
        measure(&format!("{label}/load_disk_and_compare"), || {
            assert_eq!(persistence::load(&path).unwrap(), source);
        });
        let environment = crate::app_environment::AppEnvironment::from_inputs(
            Some(root.clone().into_os_string()),
            None,
            Vec::new(),
            env!("CARGO_MANIFEST_DIR"),
        );
        assert_eq!(environment.paths().data_root(), root.as_path());
        let mut app = Operandi::with_environment(environment);
        app.editor.scene = source.clone();
        app.saved = source.clone();
        measure(&format!("{label}/dirty_equal_scene"), || {
            assert!(!black_box(app.dirty()));
        });
        let ctx = Context::default();
        theme::install(&ctx);
        measure(&format!("{label}/idle_cpu_compose"), || {
            black_box(tests::draw(&mut app, &ctx, vec2(1440., 900.), vec![]));
        });
        let start = Instant::now();
        for n in 0..120 {
            app.editor.scene.active_mut().unwrap().name = format!("recovery-{n}");
            app.journal();
            assert_eq!(app.journaled.as_ref(), Some(&app.editor.scene));
        }
        let elapsed = start.elapsed().as_secs_f64() * 1000.;
        let recovery = root.join("recovery.opscene");
        assert_eq!(persistence::load(&recovery).unwrap(), app.editor.scene);
        let metadata = std::fs::metadata(&recovery).unwrap();
        println!(
            "STAB_RECOVERY {label} changed_calls=120 successful_replacements=120 elapsed_ms={elapsed:.6} final_bytes={} history=0",
            metadata.len()
        );
        measure(&format!("{label}/journal_unchanged_dirty"), || {
            app.journal();
        });
        assert_eq!(
            std::fs::metadata(&recovery).unwrap().modified().unwrap(),
            metadata.modified().unwrap()
        );
        std::fs::remove_file(recovery).unwrap();
    }
}

#[test]
#[ignore = "opt-in bounded I2 CPU history baseline"]
fn stabilization_i2_baseline() {
    let source: Scene = serde_json::from_str(include_str!(
        "../../../../docs/i2/expectations/graphs/G0.json"
    ))
    .unwrap();
    for depth in [0, 10, 100] {
        let mut editor = Editor {
            scene: source.clone(),
            ..Default::default()
        };
        editor.synchronize_identities().unwrap();
        for n in 0..depth {
            editor
                .dispatch(Command::Rename(format!("history-{n}")))
                .unwrap();
        }
        editor.dispatch(Command::Activate(Tool::Extrude)).unwrap();
        let mut params = Parameters {
            extent: DVec3::Z * 0.5,
            extrude_mode: ExtrudeMode::Shift,
            extrude_uvs: ExtrudeUvs::None,
            ..Default::default()
        };
        editor
            .dispatch(Command::ConfigureExtrude(params.clone()))
            .unwrap();
        editor.dispatch(Command::BeginExtrude).unwrap();
        editor.dispatch(Command::Preview(params.clone())).unwrap();
        println!(
            "STAB_I2 fixture=G0 history={depth} local_cursor=2 vertices={} faces={} uv=None selected_map=Texture sides=1",
            editor.scene.active().unwrap().mesh.vertices.len(),
            editor.scene.active().unwrap().mesh.faces.len()
        );
        measure(&format!("i2_G0/preview_history_{depth}"), || {
            params.extent.z = if params.extent.z == 0.5 { 0.25 } else { 0.5 };
            editor
                .dispatch(Command::Preview(black_box(params.clone())))
                .unwrap();
        });
        let expected = editor.scene.clone();
        measure(
            &format!("i2_G0/move_undo_redo_pair_history_{depth}"),
            || {
                editor.dispatch(Command::Undo).unwrap();
                editor.dispatch(Command::Redo).unwrap();
            },
        );
        assert_eq!(editor.scene, expected);
        editor.scene.validate().unwrap();
        editor.identities.current.validate(&editor.scene).unwrap();
    }
}

#[test]
#[ignore = "opt-in PERF-BOX-100 generated plane CPU benchmark"]
fn perf_box_100_plane() {
    let mut app = Operandi::with_environment(crate::app_environment::AppEnvironment::from_inputs(
        None,
        None,
        Vec::new(),
        env!("CARGO_MANIFEST_DIR"),
    ));
    app.editor.scene.new_mesh().unwrap();
    app.editor.scene.active_mut().unwrap().mesh = operandi_core::primitive::segmented_cube(
        DVec3::ZERO,
        DVec3::new(5., 5., 0.),
        [100, 100, 100],
    )
    .unwrap()
    .into();
    app.editor.scene.selection.mode = Mode::Polygon;
    let mesh = &app.editor.scene.active().unwrap().mesh;
    println!(
        "BOX100 vertices={} faces={} edges={}",
        mesh.vertices.len(),
        mesh.faces.len(),
        mesh.edges().len()
    );
    assert_eq!(
        (mesh.vertices.len(), mesh.faces.len(), mesh.edges().len()),
        (10201, 10000, 20200)
    );
    let ctx = Context::default();
    theme::install(&ctx);
    app.camera.view = View::Front;
    measure("box100/surface_build", || {
        black_box(picking::Surface::new(&app.editor.scene));
    });
    measure("box100/cpu_frame_polygon_none", || {
        black_box(tests::draw(&mut app, &ctx, vec2(1440., 900.), vec![]));
    });
    app.camera.view = View::Perspective;
    measure("box100/cpu_frame_navigation", || {
        app.camera.yaw += 0.001;
        black_box(tests::draw(&mut app, &ctx, vec2(1440., 900.), vec![]));
    });
    measure("box100/cpu_frame_mesh_mutation", || {
        app.editor.scene.active_mut().unwrap().mesh.vertices[0].x += 0.000001;
        black_box(tests::draw(&mut app, &ctx, vec2(1440., 900.), vec![]));
    });
}

#[test]
#[ignore = "local-only PERF-BOX-100 recovery replay; path supplied explicitly"]
fn perf_box_100_local_snapshot_replay() {
    let path = std::env::var_os("OPERANDI_LOCAL_REPLAY").expect("local recovery path");
    let bytes = std::fs::read(path).unwrap();
    let original = persistence::decode(&bytes).unwrap();
    let mut app = Operandi::with_environment(crate::app_environment::AppEnvironment::from_inputs(
        None,
        None,
        Vec::new(),
        env!("CARGO_MANIFEST_DIR"),
    ));
    app.editor.scene = original.clone();
    app.editor.synchronize_identities().unwrap();
    assert_eq!(
        (
            original.items[0].mesh.vertices.len(),
            original.items[0].mesh.faces.len(),
            original.items[0].mesh.edges().len()
        ),
        (10201, 10000, 20200)
    );
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..2 {
        black_box(tests::draw(&mut app, &ctx, vec2(1440., 900.), vec![]));
    }
    assert_eq!(app.editor.scene, original);
    assert_eq!(
        persistence::decode(&persistence::encode(&app.editor.scene).unwrap()).unwrap(),
        original
    );
    let first = app
        .surface_cache
        .get(&app.editor.scene, &app.subdivision_cache);
    app.editor.scene.items[0].mesh.vertices[0].x += 0.001;
    let changed = app
        .surface_cache
        .get(&app.editor.scene, &app.subdivision_cache);
    assert!(!std::sync::Arc::ptr_eq(&first, &changed));
    app.editor.scene = original.clone();
    let restored = app
        .surface_cache
        .get(&app.editor.scene, &app.subdivision_cache);
    assert!(!std::sync::Arc::ptr_eq(&changed, &restored));
    assert_eq!(app.editor.scene, original);
}

#[test]
#[ignore = "opt-in viewport frame benchmark on a large generated mesh"]
fn viewport_frame_large_mesh() {
    // A 128 x 128 quad grid, folded so every quad faces the camera a little.
    let n = 128u32;
    let mesh = wavy_grid(n);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.frame(false);
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    super::tests::draw(&mut a, &ctx, size, vec![]);
    let viewport = a.viewport_rect.center();
    let mut flip = false;
    a.force_resident = true;
    measure("viewport/frame_16k_quads_resident", || {
        flip = !flip;
        let p = viewport + vec2(if flip { 3. } else { -3. }, 0.);
        black_box(super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(p)],
        ));
    });
    a.force_resident = false;
    // Vertex mode: every point drawn as a dot (16.6k points).
    a.run(Command::Mode(Mode::Vertex));
    a.force_resident = true;
    measure("viewport/frame_16k_quads_vertex_mode_resident", || {
        flip = !flip;
        let p = viewport + vec2(if flip { 3. } else { -3. }, 0.);
        black_box(super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(p)],
        ));
    });
    // A new point selected every frame: nothing big may rebuild.
    let mut pick = 0u32;
    measure("viewport/frame_16k_quads_vertex_select_resident", || {
        pick = (pick + 1) % 100;
        a.editor.scene.selection.vertices = vec![pick];
        black_box(super::tests::draw(&mut a, &ctx, size, vec![]));
    });
    a.editor.scene.selection.vertices.clear();
    a.force_resident = false;
    measure("viewport/frame_16k_quads_vertex_mode", || {
        flip = !flip;
        let p = viewport + vec2(if flip { 3. } else { -3. }, 0.);
        black_box(super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(p)],
        ));
    });
    a.run(Command::Mode(Mode::Edge));
    a.force_resident = true;
    measure("viewport/frame_16k_quads_edge_mode_resident", || {
        flip = !flip;
        let p = viewport + vec2(if flip { 3. } else { -3. }, 0.);
        black_box(super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(p)],
        ));
    });
    a.force_resident = false;
    a.run(Command::Mode(Mode::Polygon));
    // Dragging on the subdivided mesh: the mesh changes every frame.
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    let base = a.editor.scene.active().unwrap().mesh.clone();
    let mut step = 0u32;
    a.force_resident = true;
    measure("viewport/frame_16k_quads_subdivided_drag_resident", || {
        step += 1;
        let mut mesh = (*base).clone();
        mesh.vertices[100].z += step as f64 * 1e-4;
        a.editor.scene.active_mut().unwrap().mesh = mesh.into();
        black_box(super::tests::draw(&mut a, &ctx, size, vec![]));
    });
    a.force_resident = false;
    a.editor.scene.active_mut().unwrap().mesh = base;
    measure("subdivision/display_surface_16k_l2", || {
        let item = a.editor.scene.active().unwrap();
        black_box(
            operandi_core::subdivision::display_surface(&item.mesh, &Default::default()).unwrap(),
        );
    });
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    measure("viewport/frame_16k_quads", || {
        // Move the pointer so each frame does its rollover pick too.
        flip = !flip;
        let p = viewport + vec2(if flip { 3. } else { -3. }, 0.);
        black_box(super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(p)],
        ));
    });
    // The same mesh as Catmull-Clark at its default level 2 (16 x the
    // polygons drawn), with the cage wireframe traced over the surface.
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    super::tests::draw(&mut a, &ctx, size, vec![]);
    // Geometry kept on the graphics card: the CPU work left per frame
    // (camera, rollover, overlays); uploads happen only on change.
    a.force_resident = true;
    measure("viewport/frame_16k_quads_subdivided_resident", || {
        flip = !flip;
        let p = viewport + vec2(if flip { 3. } else { -3. }, 0.);
        black_box(super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(p)],
        ));
    });
    a.force_resident = false;
    measure("viewport/frame_16k_quads_subdivided", || {
        flip = !flip;
        let p = viewport + vec2(if flip { 3. } else { -3. }, 0.);
        black_box(super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(p)],
        ));
    });
}

#[test]
#[ignore = "opt-in Move drag benchmark on a large subdivided mesh"]
fn move_drag_subdivided() {
    // 128 x 128 quad grid (16k quads) as Catmull-Clark, level 2; one point
    // selected and dragged with the Move handle, as in Jason's test.
    let n = 128u32;
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            let (x, y) = (i as f64 / n as f64 - 0.5, j as f64 / n as f64 - 0.5);
            mesh.vertices
                .push(DVec3::new(x, y, 0.1 * (x * 9.).sin() * (y * 7.).cos()));
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
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    a.force_resident = true;
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    super::tests::draw(&mut a, &ctx, size, vec![]);
    a.run(Command::Mode(Mode::Vertex));
    a.editor.scene.selection.vertices = vec![n * (n + 1) / 2 + n / 2];
    a.activate(Tool::Move);
    for _ in 0..2 {
        super::tests::draw(&mut a, &ctx, size, vec![]);
    }
    let r = a.viewport_rect;
    let start = (0..300)
        .flat_map(|y| (0..300).map(move |x| (x, y)))
        .map(|(x, y)| r.center() + vec2(x as f32 * 2. - 300., y as f32 * 2. - 300.))
        .find(|&p| a.handle_hit(p, r).is_some())
        .expect("a Move handle on screen");
    super::tests::draw(
        &mut a,
        &ctx,
        size,
        vec![
            Event::PointerMoved(start),
            super::tests::pointer(start, true, Modifiers::NONE),
        ],
    );
    let picked = a.editor.scene.selection.vertices[0] as usize;
    let before_point = a.editor.scene.active().unwrap().mesh.vertices[picked];
    let builds = a.resident_cache.builds();
    let mut step = 0f32;
    measure("drag/move_point_16k_quads_subdivided", || {
        step += 1.;
        let p = start + vec2((step % 40.) * 0.7, (step % 40.) * -0.5);
        black_box(super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(p)],
        ));
    });
    assert!(a.navigation.is_some(), "still dragging");
    println!(
        "nav {:?} live {:?} status {}",
        a.navigation
            .as_ref()
            .map(|n| matches!(n.kind, input::GestureKind::Handle(..))),
        a.editor.session.live.as_ref().map(|l| l.params.delta),
        a.status
    );
    println!(
        "moved {} builds {}",
        a.editor.scene.active().unwrap().mesh.vertices[picked] - before_point,
        a.resident_cache.builds() - builds
    );
}

#[test]
#[ignore = "opt-in Item-mode Move drag benchmark on a 65k-polygon mesh"]
fn item_move_drag_large() {
    // 256 x 256 quads (65k polygons, about a dense sculpt's size), the
    // whole item dragged with the Move handle in Item mode: only the
    // item's placement changes, never its mesh.
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = wavy_grid(256).into();
    if std::env::var_os("OPERANDI_BENCH_SUBDIVIDED").is_some() {
        // About a subdivided sculpt: 137 x 137 = 18.8k quads, Catmull-Clark.
        a.editor.scene.active_mut().unwrap().mesh = wavy_grid(137).into();
        a.run(Command::ToggleSubdivision(
            operandi_core::subdivision::Toggle::CatmullClark,
        ));
    }
    a.force_resident = true;
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    // The steps `update` runs around the viewport each frame.
    let full_frame = |a: &mut Operandi, events: Vec<Event>| {
        a.frame_stats.begin();
        a.poll_import();
        super::tests::draw(a, &ctx, size, events);
        a.keep_utility_exclusive();
        a.keep_draw_valid();
        black_box(a.menu_model(&ctx));
        a.persist_layout(&ctx);
        a.journal_when_due(&ctx);
        black_box(a.dirty());
        a.end_frame(&ctx);
    };
    full_frame(&mut a, vec![]);
    a.run(Command::Mode(Mode::Item));
    a.activate(Tool::Move);
    for _ in 0..2 {
        full_frame(&mut a, vec![]);
    }
    let r = a.viewport_rect;
    let start = (0..300)
        .flat_map(|y| (0..300).map(move |x| (x, y)))
        .map(|(x, y)| r.center() + vec2(x as f32 * 2. - 300., y as f32 * 2. - 300.))
        .find(|&p| a.handle_hit(p, r).is_some())
        .expect("a Move handle on screen");
    full_frame(
        &mut a,
        vec![
            Event::PointerMoved(start),
            super::tests::pointer(start, true, Modifiers::NONE),
        ],
    );
    let before = a.editor.scene.active().unwrap().transform.clone();
    let builds = a.resident_cache.builds();
    let mut step = 0f32;
    measure("drag/item_move_65k_quads", || {
        step += 1.;
        let p = start + vec2((step % 40.) * 0.7, (step % 40.) * -0.5);
        full_frame(&mut a, vec![Event::PointerMoved(p)]);
    });
    assert!(a.navigation.is_some(), "still dragging");
    assert_ne!(a.editor.scene.active().unwrap().transform, before, "moved");
    println!(
        "item drag: full GPU geometry builds {}",
        a.resident_cache.builds() - builds
    );
}

/// A headless graphics device, as the QA renderer makes (qa.rs), so a
/// benchmark frame includes the real GPU uploads and drawing.
///
/// `WGPU_BACKEND` chooses the graphics API (`vulkan`, `dx12`, `metal`, `gl`).
/// `OPERANDI_TEST_GPU=off` hides the graphics card, so GPU tests skip as on
/// a machine without one. CI sets it on Jason's Linux runner, whose card
/// isn't a test target (docs/CI.md).
pub(crate) fn headless_gpu() -> Option<egui_wgpu::RenderState> {
    if std::env::var_os("OPERANDI_TEST_GPU").is_some_and(|v| v == "off") {
        return None;
    }
    fn wait<F: std::future::Future>(future: F) -> F::Output {
        struct Wake(std::thread::Thread);
        impl std::task::Wake for Wake {
            fn wake(self: std::sync::Arc<Self>) {
                self.0.unpark();
            }
        }
        let waker = std::task::Waker::from(std::sync::Arc::new(Wake(std::thread::current())));
        let mut cx = std::task::Context::from_waker(&waker);
        let mut f = std::pin::pin!(future);
        loop {
            match f.as_mut().poll(&mut cx) {
                std::task::Poll::Ready(v) => return v,
                std::task::Poll::Pending => std::thread::park(),
            }
        }
    }
    // WGPU_BACKEND picks the graphics API, as it does for the app (for
    // example `dx12` to run the GPU tests on DirectX 12 on Windows).
    //
    // Without it, only the primary APIs (Vulkan, Metal, DirectX 12), not
    // OpenGL. Each call makes and drops its own instance, and an OpenGL
    // instance starts a helper thread ("wgpu-hal WGL Instance Thread").
    // On Windows with an NVIDIA driver, that thread exiting (holding the
    // system's DLL loader lock while the driver's detach code waits on a
    // driver lock) deadlocks against another test creating a Vulkan device
    // (holding that driver lock while loading a DLL), and the test run
    // hangs forever. No GPU test uses OpenGL unless asked to.
    let instance = egui_wgpu::wgpu::Instance::new(&egui_wgpu::wgpu::InstanceDescriptor {
        flags: crate::gpu::instance_flags(),
        backends: egui_wgpu::wgpu::Backends::from_env()
            .unwrap_or(egui_wgpu::wgpu::Backends::PRIMARY),
        ..egui_wgpu::wgpu::InstanceDescriptor::from_env_or_default()
    });
    wait(egui_wgpu::RenderState::create(
        &crate::gpu::wgpu_configuration(),
        &instance,
        None,
        Default::default(),
    ))
    .ok()
}

#[test]
#[ignore = "opt-in Item Move drag on a real (headless) GPU, waiting for it each frame"]
fn item_move_drag_gpu() {
    // A subdivided sculpt-sized mesh: 137 x 137 = 18.8k quads,
    // Catmull-Clark level 2 (300k pieces drawn).
    let Some(state) = headless_gpu() else {
        println!("no graphics device");
        return;
    };
    let device = state.device.clone();
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = wavy_grid(137).into();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    // OPERANDI_BENCH_HISTORY=N: N mesh edits on the undo stack first.
    let history: usize = std::env::var("OPERANDI_BENCH_HISTORY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    a.run(Command::Mode(Mode::Vertex));
    for k in 0..history {
        a.editor.scene.selection.vertices = vec![(k * 37 % 18000) as u32];
        a.activate(Tool::Move);
        let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
        p.delta.z = 0.001;
        a.run(Command::Preview(p));
        a.run(Command::Drop);
    }
    a.editor.scene.selection.vertices.clear();
    println!("undo steps: {}", a.editor.history.undo.len());
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    // OPERANDI_BENCH_SCALE=2: a Retina display's pixel count.
    if let Some(scale) = std::env::var("OPERANDI_BENCH_SCALE")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
    {
        ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), scale));
    }
    let frame = |a: &mut Operandi, events: Vec<Event>| {
        black_box(super::tests::draw(a, &ctx, size, events));
        let _ = device.poll(egui_wgpu::wgpu::PollType::wait_indefinitely());
    };
    frame(&mut a, vec![]);
    measure("gpu/orbit_subdivided_19k", || {
        a.camera.yaw += 0.01;
        frame(&mut a, vec![]);
    });
    a.run(Command::Mode(Mode::Item));
    a.activate(Tool::Move);
    for _ in 0..2 {
        frame(&mut a, vec![]);
    }
    let r = a.viewport_rect;
    let start = (0..300)
        .flat_map(|y| (0..300).map(move |x| (x, y)))
        .map(|(x, y)| r.center() + vec2(x as f32 * 2. - 300., y as f32 * 2. - 300.))
        .find(|&p| a.handle_hit(p, r).is_some())
        .expect("a Move handle on screen");
    frame(
        &mut a,
        vec![
            Event::PointerMoved(start),
            super::tests::pointer(start, true, Modifiers::NONE),
        ],
    );
    let builds = a.resident_cache.builds();
    let mut step = 0f32;
    measure("gpu/item_move_subdivided_19k", || {
        step += 1.;
        let p = start + vec2((step % 40.) * 0.7, (step % 40.) * -0.5);
        frame(&mut a, vec![Event::PointerMoved(p)]);
    });
    assert!(a.navigation.is_some(), "still dragging");
    println!(
        "item drag on GPU: full geometry builds {}",
        a.resident_cache.builds() - builds
    );
    // Release, then drag anywhere off the handles (drag-anywhere rule).
    let end = start + vec2((step % 40.) * 0.7, (step % 40.) * -0.5);
    frame(
        &mut a,
        vec![super::tests::pointer(end, false, Modifiers::NONE)],
    );
    let off = (0..200)
        .map(|k| r.left_top() + vec2(40. + k as f32 * 5., 60. + k as f32 * 3.))
        .find(|&p| a.handle_hit(p, r).is_none())
        .expect("a point off the handles");
    frame(
        &mut a,
        vec![
            Event::PointerMoved(off),
            super::tests::pointer(off, true, Modifiers::NONE),
        ],
    );
    let (builds, surfaces) = (a.resident_cache.builds(), a.surface_cache.builds());
    let mut step = 0f32;
    measure("gpu/item_move_off_handle_subdivided_19k", || {
        step += 1.;
        let p = off + vec2((step % 40.) * 0.7, (step % 40.) * -0.5);
        frame(&mut a, vec![Event::PointerMoved(p)]);
    });
    println!(
        "off-handle drag: dragging {} builds {} picking surfaces {}",
        a.navigation.is_some(),
        a.resident_cache.builds() - builds,
        a.surface_cache.builds() - surfaces
    );
}

#[test]
#[ignore = "local-only Item Move drag on a scene file; OPERANDI_BENCH_SCENE=path"]
fn item_move_drag_scene_file() {
    // Opens a local scene through Operandi's own importer (never committed)
    // and drags its largest mesh item in Item mode with Move, on a real
    // (headless) GPU when there is one, running the same per-frame app
    // steps as `update` and waiting for the GPU each frame.
    let Some(path) = std::env::var_os("OPERANDI_BENCH_SCENE").map(PathBuf::from) else {
        println!("OPERANDI_BENCH_SCENE not set");
        return;
    };
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    // OPERANDI_BENCH_SCALE=2: a Retina display's pixel count.
    if let Some(scale) = std::env::var("OPERANDI_BENCH_SCALE")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
    {
        ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), scale));
    }
    let mut a = Operandi::empty();
    let device = match headless_gpu() {
        Some(state) => {
            let device = state.device.clone();
            a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
            Some(device)
        }
        None => {
            a.force_resident = true;
            None
        }
    };
    a.open(path);
    let deadline = Instant::now() + std::time::Duration::from_secs(120);
    while a.import_job.is_some() && Instant::now() < deadline {
        a.poll_import();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    a.poll_import();
    // Answer the import's dialogs the way the user does (a partial import
    // is accepted, the result notice closed), so presses reach the viewport.
    if let Some(ready) = a.import_ready.take() {
        a.commit_import(ready);
    }
    a.import_message = None;
    a.mesh_cleanup = None;
    a.recovery = None;
    a.recovery_warning = None;
    println!("dialog open after import: {}", a.has_modal());
    let id = a
        .editor
        .scene
        .items
        .iter()
        .max_by_key(|i| i.mesh.faces.len())
        .expect("a mesh item")
        .id;
    for item in &a.editor.scene.items {
        println!(
            "item {} {:?}: {} points, {} polygons, smooth {}, subdivision {:?}, parent {:?}, materials {}",
            item.id,
            item.name,
            item.mesh.vertices.len(),
            item.mesh.faces.len(),
            operandi_core::subdivision::has_smooth(&item.mesh),
            item.subdivision,
            item.parent,
            a.editor.scene.materials.len(),
        );
    }
    println!("gpu: {}", device.is_some());
    {
        let item = a.editor.scene.item(id).unwrap();
        let mut kinds = std::collections::BTreeMap::<String, usize>::new();
        for r in &item.mesh.attributes.representations {
            *kinds.entry(format!("{r:?}")).or_default() += 1;
        }
        println!(
            "representations {kinds:?} (of {} polygons); uv maps {}; typed maps {}",
            item.mesh.faces.len(),
            item.mesh.uv_maps.len(),
            item.mesh.attributes.typed_maps.len()
        );
    }
    // Does the scene keep sharing its mesh with the saved baseline, or
    // does something replace it with an equal copy?
    let shares = |a: &Operandi| {
        let now = a.editor.scene.item(id).unwrap();
        a.saved
            .item(id)
            .is_some_and(|s| operandi_core::Shared::same(s.mesh.shared(), now.mesh.shared()))
    };
    println!("mesh shared with saved copy after import: {}", shares(&a));
    a.editor.scene.selection.item = Some(id);
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    let full_frame = |a: &mut Operandi, events: Vec<Event>| {
        a.frame_stats.begin();
        a.poll_import();
        super::tests::draw(a, &ctx, size, events);
        a.keep_utility_exclusive();
        a.keep_draw_valid();
        black_box(a.menu_model(&ctx));
        a.persist_layout(&ctx);
        a.journal_when_due(&ctx);
        black_box(a.dirty());
        a.end_frame(&ctx);
        if let Some(device) = &device {
            let _ = device.poll(egui_wgpu::wgpu::PollType::wait_indefinitely());
        }
    };
    for _ in 0..3 {
        full_frame(&mut a, vec![]);
    }
    println!("mesh shared with saved copy after 3 frames: {}", shares(&a));
    let counts = |a: &Operandi| {
        [
            a.resident_cache.builds(),
            a.surface_cache.builds(),
            a.subdivision_cache.full_builds(),
            operandi_core::topology_cache::computed(),
        ]
    };
    let report = |a: &Operandi, what: &str, before: [u64; 4]| {
        let after = counts(a);
        println!(
            "{what}: gpu geometry builds {}, picking surface builds {}, subdivision full builds {}, topology {}",
            after[0] - before[0],
            after[1] - before[1],
            after[2] - before[2],
            after[3] - before[3]
        );
    };
    // Idle with the pointer moving over the view (rollover only).
    let before = counts(&a);
    let center = a.viewport_rect.center();
    let mut flip = false;
    measure("scene/idle_hover", || {
        flip = !flip;
        let p = center + vec2(if flip { 3. } else { -3. }, 0.);
        full_frame(&mut a, vec![Event::PointerMoved(p)]);
    });
    report(&a, "idle hover", before);
    let before = counts(&a);
    measure("scene/orbit", || {
        a.camera.yaw += 0.01;
        full_frame(&mut a, vec![]);
    });
    report(&a, "orbit", before);
    a.run(Command::Mode(Mode::Item));
    a.activate(Tool::Move);
    for _ in 0..2 {
        full_frame(&mut a, vec![]);
    }
    println!(
        "Move: live {:?}, mode {:?}, modal {}, status {:?}",
        a.editor.session.live.as_ref().map(|l| l.tool),
        a.editor.scene.selection.mode,
        a.has_modal(),
        a.status
    );
    // The way a pointer drives it: press on the handle, drag.
    let r = a.viewport_rect;
    let start = (0..300)
        .flat_map(|y| (0..300).map(move |x| (x, y)))
        .map(|(x, y)| r.center() + vec2(x as f32 * 2. - 300., y as f32 * 2. - 300.))
        .find(|&p| a.handle_hit(p, r).is_some());
    if let Some(start) = start {
        full_frame(
            &mut a,
            vec![
                Event::PointerMoved(start),
                super::tests::pointer(start, true, Modifiers::NONE),
            ],
        );
        println!("press: dragging {}", a.navigation.is_some());
        let before = counts(&a);
        let mut step = 0f32;
        measure("scene/item_move_pointer_drag", || {
            step += 1.;
            let p = start + vec2((step % 40.) * 0.7, (step % 40.) * -0.5);
            full_frame(&mut a, vec![Event::PointerMoved(p)]);
        });
        println!("still dragging {}", a.navigation.is_some());
        report(&a, "pointer drag", before);
        let end = start + vec2((step % 40.) * 0.7, (step % 40.) * -0.5);
        full_frame(
            &mut a,
            vec![super::tests::pointer(end, false, Modifiers::NONE)],
        );
    } else {
        println!("no Move handle found on screen");
    }
    // The tool's value changed directly every frame (no pointer).
    if let Some(base) = a.editor.session.live.as_ref().map(|l| l.params.clone()) {
        let before = counts(&a);
        let mut k = 0.;
        measure("scene/item_move_preview", || {
            k += 1.;
            let mut p = base.clone();
            p.delta.x = 0.001 * k;
            a.run(Command::Preview(p));
            full_frame(&mut a, vec![]);
        });
        report(&a, "preview drag", before);
        // One tool update alone, as each pointer-move event in a real drag
        // runs (a mouse sends several per frame).
        measure("scene/item_move_one_update", || {
            k += 1.;
            let mut p = base.clone();
            p.delta.x = 0.001 * k;
            a.run(Command::Preview(p));
        });
        println!(
            "undo steps {}, dirty {}",
            a.editor.history.undo.len(),
            a.dirty()
        );
    }
}

#[test]
#[ignore = "opt-in whole-frame benchmark of a Tweak pointer drag"]
fn tweak_drag_subdivided() {
    // The same mesh and camera as `move_drag_subdivided`, driven by real
    // pointer events: hover, press on a point, drag.
    let n = 128u32;
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = wavy_grid(n).into();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    a.force_resident = true;
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    super::tests::draw(&mut a, &ctx, size, vec![]);
    a.run(Command::Mode(Mode::Vertex));
    a.activate(Tool::Tweak);
    for _ in 0..2 {
        super::tests::draw(&mut a, &ctx, size, vec![]);
    }
    let r = a.viewport_rect;
    let mid = (n / 2) * (n + 1) + n / 2;
    let item = a.editor.scene.active().unwrap();
    let start = a
        .camera
        .project(
            a.editor
                .scene
                .world_point(item.id, item.mesh.vertices[mid as usize]),
            r,
        )
        .unwrap()
        .0;
    // Pointer resting on the mesh: redraws reuse the hover pick.
    super::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(start)]);
    measure("idle/tweak_16k_quads_subdivided", || {
        black_box(super::tests::draw(&mut a, &ctx, size, vec![]));
    });
    let mut hover_step = 0f32;
    measure("hover/tweak_16k_quads_subdivided", || {
        hover_step += 1.;
        let p = start + vec2((hover_step % 40.) * 0.7, (hover_step % 40.) * -0.5);
        black_box(super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(p)],
        ));
    });
    super::tests::draw(
        &mut a,
        &ctx,
        size,
        vec![
            Event::PointerMoved(start),
            super::tests::pointer(start, true, Modifiers::NONE),
        ],
    );
    let builds = a.resident_cache.builds();
    let surfaces = a.surface_cache.builds();
    let full = a.subdivision_cache.full_builds();
    let mut step = 0f32;
    measure("drag/tweak_point_16k_quads_subdivided", || {
        step += 1.;
        let p = start + vec2((step % 40.) * 0.7, (step % 40.) * -0.5);
        black_box(super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(p)],
        ));
    });
    assert!(a.navigation.is_some(), "still dragging");
    println!(
        "PERF tweak drag: gpu builds {} surface builds {} subdivision full builds {} status {}",
        a.resident_cache.builds() - builds,
        a.surface_cache.builds() - surfaces,
        a.subdivision_cache.full_builds() - full,
        a.status
    );
    // The same drag with a falloff (TWK-12): Range 0.05 m reaches about
    // six cells around the point, limited to its connected piece.
    let end = start + vec2((step % 40.) * 0.7, (step % 40.) * -0.5);
    super::tests::draw(
        &mut a,
        &ctx,
        size,
        vec![super::tests::pointer(end, false, Modifiers::NONE)],
    );
    let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
    p.tweak_options.range = 0.05;
    p.tweak_options.connect = operandi_core::TweakConnect::Connectivity;
    a.run(Command::Preview(p));
    let start = end;
    super::tests::draw(
        &mut a,
        &ctx,
        size,
        vec![
            Event::PointerMoved(start),
            super::tests::pointer(start, true, Modifiers::NONE),
        ],
    );
    let full = a.subdivision_cache.full_builds();
    let mut step = 0f32;
    measure("drag/tweak_range_16k_quads_subdivided", || {
        step += 1.;
        let p = start + vec2((step % 40.) * 0.7, (step % 40.) * -0.5);
        black_box(super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![Event::PointerMoved(p)],
        ));
    });
    assert!(a.navigation.is_some(), "still dragging");
    println!(
        "PERF tweak range drag: moved {} points, subdivision full builds {} status {}",
        a.editor
            .session
            .live
            .as_ref()
            .and_then(|l| l.params.tweak_plan.as_ref())
            .map_or(0, |plan| plan.moves.len()),
        a.subdivision_cache.full_builds() - full,
        a.status
    );
}

/// `n` x `n` quads on a gentle wave, as used by the subdivided drag benchmarks.
fn wavy_grid(n: u32) -> operandi_core::Mesh {
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            let (x, y) = (i as f64 / n as f64 - 0.5, j as f64 / n as f64 - 0.5);
            mesh.vertices
                .push(DVec3::new(x, y, 0.1 * (x * 9.).sin() * (y * 7.).cos()));
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

#[test]
#[ignore = "opt-in whole-frame benchmark of every interactive tool on a subdivided mesh"]
fn tool_frames_subdivided() {
    // Each frame changes the tool's main value and draws: core preview,
    // subdivision update and GPU geometry, as during a drag.
    let n = 128u32;
    let mid_face = (n / 2) * n + n / 2;
    let mid_point = (n / 2) * (n + 1) + n / 2;
    type Step = fn(&mut operandi_core::Parameters, f64);
    let tools: Vec<(Tool, Mode, Step)> = vec![
        (Tool::Move, Mode::Polygon, |p, k| p.delta.z = 0.001 * k),
        (Tool::Rotate, Mode::Polygon, |p, k| {
            p.rotation_delta.z = 0.01 * k
        }),
        (Tool::Scale, Mode::Polygon, |p, k| {
            p.scale_delta = DVec3::splat(1. + 0.01 * k)
        }),
        (Tool::Extrude, Mode::Polygon, |p, k| p.shift = 0.001 * k),
        (Tool::SmoothShift, Mode::Polygon, |p, k| {
            p.smooth_shift.offset = 0.001 * k
        }),
        (Tool::Thicken, Mode::Polygon, |p, k| {
            p.smooth_shift.offset = 0.001 * k
        }),
        (Tool::Bevel, Mode::Polygon, |p, k| {
            p.inset = 0.0002 * k;
            p.shift = 0.001 * k
        }),
        (Tool::TwistBevel, Mode::Polygon, |p, k| {
            p.twist_bevel.shift = 0.05 + 0.001 * k;
            p.twist_bevel.bend = 30.;
            p.twist_bevel.rotation = 10.
        }),
        (Tool::PolyHaul, Mode::Polygon, |p, k| {
            p.poly_haul_offset = 0.001 * k
        }),
        (Tool::EdgeExtrude, Mode::Edge, |p, k| {
            p.shift = 0.001 * k;
            p.inset = 0.001
        }),
        (Tool::EdgeBevel, Mode::Edge, |p, k| {
            p.chamfer.offset = 0.0001 * k
        }),
        (Tool::EdgeChamfer, Mode::Edge, |p, k| {
            p.chamfer.offset = 0.0001 * k
        }),
        (Tool::EdgeSlide, Mode::Edge, |p, k| p.shift = 0.02 * k),
        (Tool::EdgeCrease, Mode::Edge, |p, k| p.shift = 0.05 * k),
        (Tool::VertexBevel, Mode::Vertex, |p, k| p.inset = 0.0002 * k),
        (Tool::VertexExtrude, Mode::Vertex, |p, k| {
            p.shift = 0.001 * k;
            p.inset = 0.001
        }),
        (Tool::VertexMerge, Mode::Vertex, |p, k| p.shift = 0.0001 * k),
        (Tool::VertexSlide, Mode::Vertex, |p, k| {
            p.shift = 0.05 * k;
            p.delta = DVec3::X
        }),
        // The grabbed polygon is set after activation, below.
        (Tool::Tweak, Mode::Polygon, |p, k| p.delta.z = 0.001 * k),
        // Slice tools (128 x 128 grid: point (64, 64) is 8320).
        (Tool::LoopSlice, Mode::Edge, |p, k| {
            p.slice.loop_slice.positions = vec![0.3 + 0.01 * k]
        }),
        (Tool::EdgeSlice, Mode::Polygon, |p, k| {
            let t = 0.2 + 0.01 * k;
            p.slice.edge_slice.points = vec![
                operandi_core::SlicePoint::Edge {
                    a: 8320,
                    b: 8321,
                    t,
                },
                operandi_core::SlicePoint::Edge {
                    a: 8449,
                    b: 8450,
                    t,
                },
            ]
        }),
        (Tool::PolygonSlice, Mode::Polygon, |p, k| {
            let y = 0.001 + 0.0001 * k;
            p.slice.polygon_slice.cuts = vec![operandi_core::PlaneCut {
                start: DVec3::new(-0.1, y, 0.),
                end: DVec3::new(0.1, y, 0.),
                up: DVec3::Z,
            }]
        }),
        // Waves 2 and 3 across the selected middle polygons.
        (Tool::PenSlice, Mode::Polygon, |p, k| {
            let o = &mut p.slice.pen_slice;
            o.cutter.axis = operandi_core::SliceAxis::Z;
            o.closed = false;
            let y = 0.00390625 + 7.8125e-05 * k;
            o.points = vec![
                DVec3::new(-0.00390625, y, 1.),
                DVec3::new(0.01953125, y, 1.),
            ];
        }),
        (Tool::CurveSlice, Mode::Polygon, |p, k| {
            let o = &mut p.slice.curve_slice;
            o.cutter.axis = operandi_core::SliceAxis::Z;
            // Small moves: the curve's steps stay in the same polygons.
            let y = 0.00390625 + 1e-5 * k;
            o.points = vec![
                DVec3::new(-0.00390625, y, 1.),
                DVec3::new(0.0078125, y + 0.001, 1.),
                DVec3::new(0.01953125, y, 1.),
            ];
        }),
        (Tool::PrimitiveSlice, Mode::Polygon, |p, k| {
            let o = &mut p.slice.primitive_slice;
            o.place = Some(operandi_core::ShapePlace::on(
                DVec3::new(0.0078125 + 7.8125e-05 * k, 0.00390625, 0.),
                DVec3::Z,
            ));
            o.half_x = 0.0078125;
            o.half_y = 0.001953125;
        }),
        (Tool::AxisSlice, Mode::Polygon, |p, k| {
            p.slice.axis_slice.offset.x = 7.8125e-05 * k
        }),
        // Duplicate tools: the drag moves the placer; only copy points move.
        (Tool::Mirror, Mode::Polygon, |p, k| {
            p.duplicate.placed = true;
            p.duplicate.mirror.center.x = 0.6 + 0.001 * k
        }),
        (Tool::Lathe, Mode::Polygon, |p, k| {
            p.duplicate.placed = true;
            p.duplicate.lathe.circle.center.x = 2. + 0.001 * k;
            p.duplicate.lathe.circle.end = 180.
        }),
        (Tool::Clone, Mode::Polygon, |p, k| {
            p.duplicate.placed = true;
            p.duplicate.clone.offset.x = 1. + 0.001 * k
        }),
        (Tool::Array, Mode::Polygon, |p, k| {
            p.duplicate.placed = true;
            p.duplicate.array.offset.x = 1. + 0.001 * k
        }),
        (Tool::RadialArray, Mode::Polygon, |p, k| {
            p.duplicate.placed = true;
            p.duplicate.radial.circle.center.x = 2. + 0.001 * k
        }),
    ];
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    for (tool, mode, step) in tools {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = wavy_grid(n).into();
        a.run(Command::ToggleSubdivision(
            operandi_core::subdivision::Toggle::CatmullClark,
        ));
        a.force_resident = true;
        a.camera.view = crate::camera::View::Perspective;
        a.frame(false);
        a.run(Command::Mode(mode));
        let selection = &mut a.editor.scene.selection;
        match mode {
            Mode::Polygon => selection.faces = vec![mid_face, mid_face + 1, mid_face + n],
            Mode::Edge => selection.edges = vec![(mid_point, mid_point + 1)],
            _ => selection.vertices = vec![mid_point],
        }
        super::tests::draw(&mut a, &ctx, size, vec![]);
        a.activate(tool);
        let Some(mut base) = a.editor.session.live.as_ref().map(|l| l.params.clone()) else {
            println!("PERF tool/{tool:?} not active: {}", a.status);
            continue;
        };
        if tool == Tool::Tweak {
            base.tweak = Some(operandi_core::TweakGrab {
                item: a.editor.scene.active().unwrap().id,
                element: operandi_core::TweakElement::Polygon(mid_face),
            });
        }
        super::tests::draw(&mut a, &ctx, size, vec![]);
        let builds = a.resident_cache.builds();
        let mut k = 0.;
        measure(&format!("tool/{tool:?}_16k_quads_subdivided"), || {
            k += 1.;
            let mut p = base.clone();
            step(&mut p, k % 40. + 1.);
            a.run(Command::Preview(p));
            black_box(super::tests::draw(&mut a, &ctx, size, vec![]));
        });
        println!(
            "PERF tool/{tool:?} full_builds={} status={:?}",
            a.resident_cache.builds() - builds,
            a.status
        );
    }
}

#[test]
#[ignore = "opt-in paint-selection drag benchmark on a large subdivided mesh"]
fn paint_select_subdivided() {
    // Left-drag across the grid in each component mode: each frame paints
    // the component under the pointer into the selection, as in Jason's test.
    for mode in [Mode::Polygon, Mode::Edge, Mode::Vertex] {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = wavy_grid(128).into();
        a.run(Command::ToggleSubdivision(
            operandi_core::subdivision::Toggle::CatmullClark,
        ));
        a.force_resident = true;
        a.camera.view = crate::camera::View::Perspective;
        a.frame(false);
        a.run(Command::Mode(mode));
        a.editor.scene.selection.faces.clear();
        let ctx = Context::default();
        theme::install(&ctx);
        let size = vec2(1440., 900.);
        for _ in 0..2 {
            super::tests::draw(&mut a, &ctx, size, vec![]);
        }
        let r = a.viewport_rect;
        let start = r.center() - vec2(300., 0.);
        super::tests::draw(
            &mut a,
            &ctx,
            size,
            vec![
                Event::PointerMoved(start),
                super::tests::pointer(start, true, Modifiers::NONE),
            ],
        );
        let builds = a.resident_cache.builds();
        let mut step = 0f32;
        measure(
            &format!("select/paint_{mode:?}_16k_quads_subdivided"),
            || {
                step += 1.;
                // A diagonal sweep, so most frames reach a new component.
                let p = start + vec2((step * 40.) % 600., (step * 17.) % 300.);
                black_box(super::tests::draw(
                    &mut a,
                    &ctx,
                    size,
                    vec![Event::PointerMoved(p)],
                ));
            },
        );
        let s = &a.editor.scene.selection;
        println!(
            "PERF select/{mode:?} painted {} polygons, {} edges, {} points; full GPU builds {}",
            s.faces.len(),
            s.edges.len(),
            s.vertices.len(),
            a.resident_cache.builds() - builds
        );
    }
}

#[test]
#[ignore = "writes a generated subdivided 19k-quad scene for the in-app drag benchmark"]
fn write_drag_bench_scene() {
    // OPERANDI_BENCH_WRITE=path.opscene; open it with OPERANDI_BENCH_DRAG.
    let Some(path) = std::env::var_os("OPERANDI_BENCH_WRITE").map(PathBuf::from) else {
        println!("OPERANDI_BENCH_WRITE not set");
        return;
    };
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = wavy_grid(137).into();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    if path.extension().is_some_and(|e| e == "lxo") {
        // Through Operandi's own LXO exporter, to test the import path.
        let bytes = operandi_core::lxo::export::write(&a.editor.scene, &mut Vec::new()).unwrap();
        std::fs::write(&path, bytes).unwrap();
    } else {
        operandi_core::persistence::save(&path, &a.editor.scene).unwrap();
    }
}

#[test]
#[ignore = "opt-in whole-mesh Polygon-mode Move drag on a real (headless) GPU"]
fn whole_mesh_move_drag_gpu() {
    // Polygon mode, nothing selected (how an imported scene started): the
    // Move handle drags every point of a subdivided 19k-quad mesh.
    let Some(state) = headless_gpu() else {
        println!("no graphics device");
        return;
    };
    let device = state.device.clone();
    let mut a = Operandi::empty();
    a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = wavy_grid(137).into();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces.clear();
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let frame = |a: &mut Operandi, events: Vec<Event>| {
        black_box(super::tests::draw(a, &ctx, size, events));
        let _ = device.poll(egui_wgpu::wgpu::PollType::wait_indefinitely());
    };
    frame(&mut a, vec![]);
    a.activate(Tool::Move);
    for _ in 0..2 {
        frame(&mut a, vec![]);
    }
    let r = a.viewport_rect;
    let start = (0..300)
        .flat_map(|y| (0..300).map(move |x| (x, y)))
        .map(|(x, y)| r.center() + vec2(x as f32 * 2. - 300., y as f32 * 2. - 300.))
        .find(|&p| a.handle_hit(p, r).is_some())
        .expect("a Move handle on screen");
    frame(
        &mut a,
        vec![
            Event::PointerMoved(start),
            super::tests::pointer(start, true, Modifiers::NONE),
        ],
    );
    let (builds, patches) = (a.resident_cache.builds(), a.resident_cache.patches());
    let mut step = 0f32;
    measure("gpu/whole_mesh_move_subdivided_19k", || {
        step += 1.;
        let p = start + vec2((step % 40.) * 0.7, (step % 40.) * -0.5);
        frame(&mut a, vec![Event::PointerMoved(p)]);
    });
    assert!(a.navigation.is_some(), "still dragging");
    println!(
        "whole-mesh drag: GPU builds {} patches {}",
        a.resident_cache.builds() - builds,
        a.resident_cache.patches() - patches
    );
}

#[test]
#[ignore = "opt-in whole-frame benchmark of UV tools in a UV view on a subdivided mesh"]
fn uv_tool_frames_subdivided() {
    // As `tool_frames_subdivided`, with the tool in UV space: each frame
    // changes the UVs of three polygons and draws the UV view (which also
    // feeds the 3D surface's UVs).
    let n = 128u32;
    let mid_face = (n / 2) * n + n / 2;
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    type Step = fn(&mut operandi_core::Parameters, f64);
    let tools: Vec<(Tool, Step)> = vec![
        (Tool::Move, |p, k| p.delta.x = 0.001 * k),
        (Tool::Rotate, |p, k| p.delta.z = 0.5 * k),
        (Tool::Scale, |p, k| p.delta.x = 1. + 0.01 * k),
        (Tool::Smooth, |p, k| p.smooth.strength = 0.01 * k),
        (Tool::Jitter, |p, k| p.jitter.range.x = 0.0001 * k),
    ];
    for (layout, (tool, step)) in tools.iter().flat_map(|t| [(false, *t), (true, *t)]) {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        let mut mesh = wavy_grid(n);
        let rows = mesh
            .faces
            .iter()
            .map(|f| {
                f.vertices
                    .iter()
                    .map(|&v| {
                        let q = mesh.vertices[v as usize];
                        Some(glam::DVec2::new(q.x + 0.5, q.y + 0.5))
                    })
                    .collect()
            })
            .collect();
        mesh.uv_maps.insert("Texture".into(), rows);
        a.editor.scene.active_mut().unwrap().mesh = mesh.into();
        a.run(Command::ToggleSubdivision(
            operandi_core::subdivision::Toggle::CatmullClark,
        ));
        a.force_resident = true;
        a.run(Command::Mode(Mode::Polygon));
        a.editor.scene.selection.faces = vec![mid_face, mid_face + 1, mid_face + n];
        // The UV layout: a UV view beside a Perspective view showing the
        // Checker, so the 3D surface's UVs change every frame too.
        if layout {
            a.execute("layout.restore UV");
            a.display.settings.drawing.checker = true;
            for _ in 0..2 {
                super::tests::draw(&mut a, &ctx, size, vec![]);
            }
        } else {
            a.toggle_uv_view();
        }
        let tag = if layout { "uv_layout" } else { "uv" };
        super::tests::draw(&mut a, &ctx, size, vec![]);
        a.activate(tool);
        a.run(Command::UvToolSpace(Some(None)));
        let Some(base) = a.editor.session.live.as_ref().map(|l| l.params.clone()) else {
            println!("PERF {tag}/{tool:?} not active: {}", a.status);
            continue;
        };
        super::tests::draw(&mut a, &ctx, size, vec![]);
        let builds = a.resident_cache.builds();
        let mut k = 0.;
        measure(&format!("{tag}/{tool:?}_16k_quads_subdivided"), || {
            k += 1.;
            let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
            let space = p.uv.clone();
            p = base.clone();
            p.uv = space;
            step(&mut p, k % 40. + 1.);
            a.run(Command::Preview(p));
            black_box(super::tests::draw(&mut a, &ctx, size, vec![]));
        });
        println!(
            "PERF {tag}/{tool:?} full_builds={} status={:?}",
            a.resident_cache.builds() - builds,
            a.status
        );
    }
}

#[test]
#[ignore = "local-only navigation in Perspective vs Camera view on a scene file; OPERANDI_BENCH_SCENE=path, or a generated mesh"]
fn camera_view_navigation() {
    // An Alt-drag orbit and wheel zoom, in Perspective and then looking
    // through a camera, full frames on a real (headless) GPU when there is
    // one. Without OPERANDI_BENCH_SCENE it uses a generated 1M-quad mesh.
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut a = Operandi::empty();
    let device = match headless_gpu() {
        Some(state) => {
            let device = state.device.clone();
            a.viewport_gpu = Some(gpu::ViewportGpu::new(state));
            Some(device)
        }
        None => {
            a.force_resident = true;
            None
        }
    };
    if let Some(path) = std::env::var_os("OPERANDI_BENCH_SCENE").map(PathBuf::from) {
        let usd = path
            .extension()
            .is_some_and(|e| e.to_string_lossy().starts_with("usd"));
        if usd {
            // The USD import dialog's OK with its defaults.
            a.execute(&format!(
                "operandi.usd.import filename:{} upAxis:file units:file materials:materialx cameras:true lights:true graphs:true instances:instances",
                operandi_core::command_record::quote(&path.to_string_lossy())
            ));
        } else {
            a.open(path);
        }
        println!("status after open: {:?}", a.status);
        let deadline = Instant::now() + std::time::Duration::from_secs(300);
        while a.import_job.is_some() && Instant::now() < deadline {
            a.poll_import();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        a.poll_import();
        if let Some(ready) = a.import_ready.take() {
            a.commit_import(ready);
        }
        a.import_message = None;
        a.mesh_cleanup = None;
        a.recovery = None;
        a.recovery_warning = None;
    } else {
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = wavy_grid(1000).into();
    }
    let polygons: usize = a
        .editor
        .scene
        .items
        .iter()
        .map(|i| i.mesh.faces.len())
        .sum();
    println!(
        "items {}, polygons {polygons}, gpu {}",
        a.editor.scene.items.len(),
        device.is_some()
    );
    a.editor.scene.selection.item = None;
    a.camera.view = crate::camera::View::Perspective;
    a.frame(false);
    let full_frame = |a: &mut Operandi, events: Vec<Event>, mods: Modifiers| {
        a.frame_stats.begin();
        super::tests::draw_held(a, &ctx, size, events, mods);
        a.keep_utility_exclusive();
        a.keep_draw_valid();
        black_box(a.menu_model(&ctx));
        a.persist_layout(&ctx);
        a.journal_when_due(&ctx);
        black_box(a.dirty());
        a.end_frame(&ctx);
        if let Some(device) = &device {
            let _ = device.poll(egui_wgpu::wgpu::PollType::wait_indefinitely());
        }
    };
    for _ in 0..3 {
        full_frame(&mut a, vec![], Modifiers::NONE);
    }
    let counts = |a: &Operandi| {
        [
            a.resident_cache.builds(),
            a.surface_cache.builds(),
            a.subdivision_cache.full_builds(),
            a.editor.history.undo.len() as u64,
        ]
    };
    let run = |a: &mut Operandi, label: &str| {
        let c = a.viewport_rect.center();
        full_frame(
            a,
            vec![
                Event::PointerMoved(c),
                super::tests::pointer(c, true, Modifiers::ALT),
            ],
            Modifiers::ALT,
        );
        let before = counts(a);
        let mut step = 0f32;
        measure(&format!("nav/{label}_orbit_drag"), || {
            step += 1.;
            let p = c + vec2((step % 40.) * 2., -(step % 40.));
            full_frame(a, vec![Event::PointerMoved(p)], Modifiers::ALT);
        });
        let after = counts(a);
        println!(
            "{label} drag: navigating {}, geometry builds {}, picking {}, subdivision {}, undo steps {}",
            a.navigation.is_some(),
            after[0] - before[0],
            after[1] - before[1],
            after[2] - before[2],
            after[3] - before[3]
        );
        full_frame(
            a,
            vec![super::tests::pointer(c, false, Modifiers::ALT)],
            Modifiers::NONE,
        );
        full_frame(a, vec![], Modifiers::NONE);
        let before = counts(a);
        let mut flip = false;
        measure(&format!("nav/{label}_wheel"), || {
            flip = !flip;
            let d = if flip { 4. } else { -4. };
            full_frame(
                a,
                vec![
                    Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: vec2(0., d),
                        modifiers: Modifiers::NONE,
                    },
                    Event::PointerMoved(c),
                ],
                Modifiers::NONE,
            );
        });
        let after = counts(a);
        println!(
            "{label} wheel: geometry builds {}, picking {}, subdivision {}, undo steps {}",
            after[0] - before[0],
            after[1] - before[1],
            after[2] - before[2],
            after[3] - before[3]
        );
    };
    run(&mut a, "perspective");
    if crate::camera_light_ui::view_camera(&a.editor.scene).is_none() {
        a.add_item(operandi_core::item_list::AddItemKind::Camera);
    }
    a.editor.scene.selection.item = None;
    a.enter_camera_view();
    for _ in 0..3 {
        full_frame(&mut a, vec![], Modifiers::NONE);
    }
    println!("camera view: {}", a.camera.through);
    run(&mut a, "camera");
}
