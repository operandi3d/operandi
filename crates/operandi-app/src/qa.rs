//! Explicit opt-in application render harness. Never enabled in the delivered
//! bundle. Uses eframe's own capture, not a screenshot of any other application.
use super::*;
pub fn offscreen_requested() -> bool {
    std::env::var_os("OPERANDI_QA_OFFSCREEN").is_some()
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
pub fn offscreen(environment: app_environment::AppEnvironment) -> Result<()> {
    use egui_wgpu::wgpu;
    let prefix = std::env::var("OPERANDI_QA_CAPTURE").map_err(|e| e.to_string())?;
    let primitive_drag = std::env::var("OPERANDI_QA_PRIMITIVE_DRAG").ok();
    let draw_modifiers = match primitive_drag.as_deref() {
        Some("Ctrl") => Modifiers::CTRL,
        Some("Ordinary") | None => Modifiers::NONE,
        _ => return Err("QA primitive drag must be Ctrl or Ordinary".into()),
    };
    let size: Vec<u32> = std::env::var("OPERANDI_QA_SIZE")
        .map_err(|e| e.to_string())?
        .split('x')
        .filter_map(|s| s.parse().ok())
        .collect();
    if size.len() != 2 || size.iter().any(|&v| v == 0 || v > 8192) {
        return Err("QA size must be 1–8192 pixels per axis".into());
    }
    let scale: f32 = std::env::var("OPERANDI_QA_SCALE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1.);
    if !(1. ..=2.).contains(&scale) {
        return Err("QA scale must be 1–2".into());
    }
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let state = wait(egui_wgpu::RenderState::create(
        &crate::gpu::wgpu_configuration(),
        &instance,
        None,
        Default::default(),
    ))
    .map_err(|e| e.to_string())?;
    let mut app = Operandi::with_environment(environment);
    // The startup steps Operandi::new runs, in its order, so captures show
    // the shipped layout (Model) and the user's colours rather than the
    // legacy built-in panels. Before the fixture and the QA overrides below,
    // so those still win.
    let ctx = Context::default();
    theme::install(&ctx);
    app.preferences_ui.scheme = color_theme::load(&app.environment.paths().color_theme());
    app.dots.locks = panel_dot::LayoutLocks::load(&app.environment.paths().layout_locks());
    app.start_layouts();
    app.load_palettes();
    app.load_form_groups();
    app.finish_layout_migration(&ctx);
    // On macOS Operandi::new puts File…Help in the system menu bar
    // (native_menu::install) and the window's top row leaves them out.
    // Offscreen there is no AppKit menu to install, so follow the same rule
    // directly and the capture matches the real Mac window.
    app.menus_in_system_bar = cfg!(target_os = "macos");
    app.gpu = format!(
        "{:?} · {}",
        state.adapter.get_info().backend,
        state.adapter.get_info().name
    );
    app.viewport_gpu = Some(gpu::ViewportGpu::new(state.clone()));
    let fixture = app
        .environment
        .document_argument()
        .ok_or("QA needs original native fixture path")?;
    app.open(fixture);
    app.camera.view = match std::env::var("OPERANDI_QA_VIEW").as_deref() {
        Ok("Perspective") => View::Perspective,
        Ok("Front") => View::Front,
        Ok("Right") => View::Right,
        Ok("Left") => View::Left,
        Ok("Back") => View::Back,
        Ok("Bottom") => View::Bottom,
        Ok("Orthographic") => View::Orthographic,
        _ => View::Top,
    };
    for (name, value) in [
        ("OPERANDI_QA_YAW", &mut app.camera.yaw),
        ("OPERANDI_QA_PITCH", &mut app.camera.pitch),
        ("OPERANDI_QA_DISTANCE", &mut app.camera.distance),
    ] {
        if let Ok(text) = std::env::var(name) {
            *value = text.parse().map_err(|_| "Invalid QA camera number")?;
        }
    }
    // The orbit centre as "x,y,z" (with DISTANCE, puts the eye anywhere).
    if let Ok(text) = std::env::var("OPERANDI_QA_TARGET") {
        let v: Vec<f64> = text
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        let [x, y, z] = v[..] else {
            return Err("QA target must be x,y,z".into());
        };
        app.camera.target = DVec3::new(x, y, z);
    }
    if let Ok(plane) = std::env::var("OPERANDI_QA_FIXED_PLANE") {
        app.editor.scene.workplane.fixed = true;
        app.editor.scene.workplane.plane = plane.parse().map_err(|_| "Invalid QA plane")?;
    }
    match std::env::var("OPERANDI_QA_GRID").as_deref() {
        Ok("Off") => app.display.settings.drawing.grid = false,
        Ok("On") => app.display.settings.drawing.grid = true,
        _ => {}
    }
    // Display styles (view_display.rs), by label: "Gooch", "Vertex Map"…
    for (name, inactive) in [
        ("OPERANDI_QA_STYLE", false),
        ("OPERANDI_QA_INACTIVE_STYLE", true),
    ] {
        if let Ok(label) = std::env::var(name) {
            let style = view_display::Style::ALL
                .into_iter()
                .find(|s| s.label() == label || s.short_label() == label)
                .ok_or("Unknown QA display style")?;
            app.display.set_style(style, inactive);
        }
    }
    // Transparency Overrides (VP-80) on the active meshes: Ghost or XRay.
    match std::env::var("OPERANDI_QA_TRANSPARENCY").as_deref() {
        Ok("Ghost") => {
            app.display.settings.active.transparency.mode = view_display::TransparencyMode::Ghost
        }
        Ok("XRay") => {
            app.display.settings.active.transparency.mode = view_display::TransparencyMode::XRay
        }
        _ => {}
    }
    if std::env::var("OPERANDI_QA_LIGHTS").as_deref() == Ok("On") {
        app.display.settings.drawing.lights = true;
    }
    if std::env::var("OPERANDI_QA_UV_SEAMS").as_deref() == Ok("On") {
        app.display.settings.drawing.uv_seams = true;
    }
    if std::env::var("OPERANDI_QA_GROUND_AXES").as_deref() == Ok("Off") {
        app.ground_axes = false;
    }
    if std::env::var("OPERANDI_QA_PLANE_DISPLAY").as_deref() == Ok("Off") {
        app.plane_display = false;
    }
    if let Ok(opacity) = std::env::var("OPERANDI_QA_WORKPLANE_OPACITY") {
        let opacity: u8 = opacity.parse().map_err(|_| "Invalid QA opacity")?;
        if opacity > 100 {
            return Err("QA opacity must be 0–100".into());
        }
        app.viewport_preferences.workplane_opacity = opacity;
    }
    match std::env::var("OPERANDI_QA_FONT").as_deref() {
        Ok("missing") => typography::install_from(
            &ctx,
            std::path::Path::new("/operandi-font-fixture-missing"),
            "Lucida Grande",
        ),
        Ok("SF") => typography::install_from(
            &ctx,
            std::path::Path::new("/System/Library/Fonts/SFNS.ttf"),
            "System Font",
        ),
        _ => {}
    }
    let extent = wgpu::Extent3d {
        width: size[0],
        height: size[1],
        depth_or_array_layers: 1,
    };
    let target = state.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Operandi offscreen UI acceptance"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: state.target_format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    // Viewport input follows drawing. Allow a settled frame after the final
    // drag release so the captured fields/mesh match the recorded parameters.
    // OPERANDI_QA_COMMAND: commands run before the frames, separated by
    // ";" (e.g. "polygons; poly.unsubdiv" to capture a dialog).
    let commands = std::env::var("OPERANDI_QA_COMMAND").ok();
    if let Some(commands) = &commands {
        for command in commands.split(';').map(str::trim).filter(|c| !c.is_empty()) {
            app.execute(command);
        }
    }
    // OPERANDI_QA_BACKGROUND=Environment: the view shows the environment
    // (HDRI stage 2); its picture is decoded before the first frame.
    if std::env::var("OPERANDI_QA_BACKGROUND").as_deref() == Ok("Environment") {
        app.display.settings.drawing.background = view_display::Background::Environment;
    }
    if let Some(sky) = app
        .editor
        .scene
        .environment_sky(operandi_core::environment::RayKind::Camera)
    {
        app.sky_cache
            .wait(&crate::environment_gpu::sky_key(&sky.path, sky.light_probe));
    }
    // OPERANDI_QA_CLEAR_SELECTION: nothing selected (with KEEP_SELECTION,
    // so the frame-3 click doesn't pick the cap again).
    if std::env::var_os("OPERANDI_QA_CLEAR_SELECTION").is_some() {
        let mode = app.editor.scene.selection.mode;
        app.editor.scene.selection = operandi_core::Selection {
            mode,
            ..Default::default()
        };
    }
    let frames = if primitive_drag.is_some() { 9 } else { 8 };
    // Dialogs settle their label column after a frame and fade in.
    let frames = if commands.is_some() {
        frames + 8
    } else {
        frames
    };
    // The Viewport Properties popover fades in over a few frames.
    let frames = if std::env::var_os("OPERANDI_QA_VIEW_PROPERTIES").is_some()
        || std::env::var_os("OPERANDI_QA_QUAD").is_some()
    {
        frames + 12
    } else {
        frames
    };
    for frame in 0..frames {
        let mut raw = RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                vec2(size[0] as f32 / scale, size[1] as f32 / scale),
            )),
            focused: true,
            ..Default::default()
        };
        raw.viewports
            .entry(ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(scale);
        if frame >= 5 {
            raw.modifiers = match std::env::var("OPERANDI_QA_PRIMITIVE_MODIFIER").as_deref() {
                Ok("Ctrl") => Modifiers::CTRL,
                Ok("Shift") => Modifiers::SHIFT,
                _ => Modifiers::NONE,
            };
        }
        if frame == 3 && std::env::var_os("OPERANDI_QA_KEEP_SELECTION").is_none() {
            let p = app
                .camera
                .project(DVec3::new(0.3, 1.5, 0.3), app.viewport_rect)
                .ok_or("QA projection failed")?
                .0;
            raw.events = vec![
                Event::PointerButton {
                    pos: p,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
                Event::PointerButton {
                    pos: p,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::NONE,
                },
            ];
        }
        if frame == 4 {
            if let Ok(strength) = std::env::var("OPERANDI_QA_SPIKE") {
                app.category = toolbox::Category::Polygon;
                app.editor.scene.selection.mode = Mode::Polygon;
                app.execute("Spike");
                app.fields.insert("spi_strength".into(), strength);
                if !app.confirm_fields() {
                    return Err(app.status.clone());
                }
                let succeeded = app.run_ok(Command::Apply);
                if !succeeded {
                    return Err(app.status.clone());
                }
                app.frame(false);
            }
            if app.editor.scene.selection.faces.len() != 1
                && std::env::var_os("OPERANDI_QA_KEEP_SELECTION").is_none()
                && std::env::var("OPERANDI_QA_TOOL").as_deref() != Ok("None")
                && std::env::var_os("OPERANDI_QA_CUBE_STAGE").is_none()
                && primitive_drag.is_none()
            {
                return Err("Actual UI pick did not select the authored cap".into());
            }
            if let Ok(profile) = std::env::var("OPERANDI_QA_ELLIPSOID") {
                app.execute("Ellipsoid");
                let mut p = EllipsoidParameters {
                    position: DVec3::new(1., -2., 3.),
                    radius: DVec3::new(2., 3., 4.),
                    sides: 9,
                    segments: 5,
                    bulge_top: 0.37,
                    bulge_side: 0.81,
                    compensation: true,
                    gaps: 7.,
                    ..Default::default()
                };
                match profile.as_str() {
                    "zero" => {
                        p.radius.x = 0.;
                        p.compensation = false;
                    }
                    "extreme" => {
                        p.sides = 8;
                        p.segments = 4;
                        p.bulge_top = 100.;
                        p.bulge_side = 100.;
                    }
                    "subdiv" => p.polygon = PolygonRepresentation::Subdivs,
                    "catmull" => p.polygon = PolygonRepresentation::CatmullClark,
                    "bulged" | "invalid" => {}
                    _ => return Err("Unknown Ellipsoid QA profile".into()),
                }
                app.editor
                    .dispatch(Command::ConfigureEllipsoid(p.clone()))?;
                app.frame(false);
                if profile == "invalid" {
                    p.radius.x = 0.;
                    if app.editor.dispatch(Command::ConfigureEllipsoid(p)).is_ok() {
                        return Err("Signed compensation was not refused".into());
                    }
                }
            }
            if std::env::var("OPERANDI_QA_TOOL").as_deref() != Ok("None") {
                let tool = match std::env::var("OPERANDI_QA_TOOL").as_deref() {
                    Ok("Cube") => Tool::Box,
                    Ok("Sphere") => Tool::Sphere,
                    Ok("Cylinder") => Tool::Cylinder,
                    Ok("Rotate") => Tool::Rotate,
                    Ok("Scale") => Tool::Scale,
                    Ok("Transform") => Tool::Transform,
                    Ok("Bevel") => Tool::Bevel,
                    Ok("Twist Bevel") => Tool::TwistBevel,
                    Ok("Extrude") => Tool::Extrude,
                    Ok("Smooth Shift") => Tool::SmoothShift,
                    Ok("Thicken") => Tool::Thicken,
                    _ => Tool::Move,
                };
                if std::env::var_os("OPERANDI_QA_SELECTION_FRAME").is_some() {
                    app.editor.dispatch(Command::ActionContext(ActionContext {
                        center: CenterMode::Selection,
                        axis: AxisOrientation::Selection,
                    }))?;
                }
                app.activate(tool);
                if let Ok(stage) = std::env::var("OPERANDI_QA_POLYGON_STAGE") {
                    let mut p = app
                        .editor
                        .live
                        .as_ref()
                        .ok_or("QA polygon activation failed")?
                        .params
                        .clone();
                    if tool.is_smooth_shift() {
                        p.smooth_shift.offset = 0.2;
                    }
                    p.shift = 0.2;
                    p.inset = 0.1;
                    p.extent = DVec3::Z * 0.5;
                    p.extrude_sides = 3;
                    if std::env::var_os("OPERANDI_QA_SELECTION_FRAME").is_some() {
                        p.inset = 0.;
                        p.extent = app
                            .editor
                            .live
                            .as_ref()
                            .unwrap()
                            .action_frame
                            .unwrap()
                            .basis
                            .y_axis
                            * 0.2;
                    }
                    if let Ok(inset) = std::env::var("OPERANDI_QA_CURVED_INSET") {
                        p.inset = inset.parse().map_err(|_| "Invalid QA Inset")?;
                    }
                    app.editor.dispatch(Command::Preview(p))?;
                    if stage != "A" {
                        app.editor.dispatch(Command::Continue)?;
                        if stage == "B" {
                            let mut p = app.editor.live.as_ref().unwrap().params.clone();
                            p.shift = 0.15;
                            p.inset = 0.05;
                            p.extent = DVec3::new(0.25, 0., 0.2);
                            if std::env::var_os("OPERANDI_QA_SELECTION_FRAME").is_some() {
                                p.inset = 0.;
                                p.extent = app
                                    .editor
                                    .live
                                    .as_ref()
                                    .unwrap()
                                    .action_frame
                                    .unwrap()
                                    .basis
                                    .y_axis
                                    * 0.15;
                            }
                            if let Ok(inset) = std::env::var("OPERANDI_QA_CURVED_INSET") {
                                p.inset = inset.parse().map_err(|_| "Invalid QA Inset")?;
                            }
                            app.editor.dispatch(Command::Preview(p))?;
                        }
                    }
                    app.camera.target = DVec3::new(
                        if app.editor.scene.active().unwrap().name == "F17-L" {
                            1.5
                        } else {
                            0.
                        },
                        1.,
                        0.,
                    );
                    app.camera.distance = 9.;
                }
                if matches!(tool, Tool::Sphere | Tool::Cylinder) && primitive_drag.is_none() {
                    let mut p = app
                        .editor
                        .live
                        .as_ref()
                        .ok_or("QA primitive did not activate")?
                        .params
                        .clone();
                    p.position = DVec3::new(1., 2., -3.);
                    p.radial = Some(RadialParameters {
                        radius: DVec3::new(
                            1.,
                            if std::env::var("OPERANDI_QA_RADIAL_STATE").as_deref() == Ok("draft") {
                                0.
                            } else {
                                1.5
                            },
                            2.,
                        ),
                        sides: 8,
                        segments: if tool == Tool::Sphere { 4 } else { 3 },
                    });
                    app.editor.dispatch(Command::Preview(p))?;
                    app.frame(false);
                }
            }
            // M3 render check: select OPERANDI_QA_POINTS (comma list) in
            // Vertex mode on the Vertex tab, then run one panel action.
            if let Ok(action) = std::env::var("OPERANDI_QA_VERTEX") {
                app.category = toolbox::Category::Vertex;
                app.editor.scene.selection.mode = Mode::Vertex;
                if let Ok(points) = std::env::var("OPERANDI_QA_POINTS") {
                    app.editor.scene.selection.vertices = points
                        .split(',')
                        .map(|p| p.trim().parse().map_err(|_| "Invalid QA point".to_string()))
                        .collect::<Result<_>>()?;
                }
                if action == "Show Vertices" {
                    app.editor.scene.selection.mode = Mode::Polygon;
                }
                if action != "Panel" {
                    app.execute(&action);
                }
            }
            if std::env::var("OPERANDI_QA_CATEGORY").as_deref() == Ok("Polygon") {
                app.category = toolbox::Category::Polygon;
            }
            match std::env::var("OPERANDI_QA_DIALOG").as_deref() {
                Ok("unsaved") => app.request_file(FileAction::Close),
                Ok("map") => {
                    app.open_input_map();
                }
                Ok("save") => app.request_file(FileAction::SaveAs),
                _ => {}
            }
        }
        // Solid Sketch render check: a small figure drawn with the tool,
        // left on with its last node current (OPERANDI_QA_SOLID_SKETCH=on),
        // or dropped (=drop).
        if frame == 5
            && let Ok(mode) = std::env::var("OPERANDI_QA_SOLID_SKETCH")
        {
            use operandi_core::SketchEdit;
            // A fresh mesh of its own; the fixture's items are hidden.
            app.editor.dispatch(Command::NewMesh)?;
            let fresh = app.editor.scene.selection.item;
            for item in &mut app.editor.scene.items {
                if Some(item.id) != fresh {
                    item.visible = false;
                }
            }
            app.editor.dispatch(Command::SolidSketchStart {
                options: operandi_core::solid_sketch::SketchOptions {
                    rotate_handles: mode == "on",
                    auto_align: true,
                    ..operandi_core::solid_sketch::SketchOptions::ss_default()
                },
                resume: false,
            })?;
            let mut add = |at: DVec3, size: f64, parent: Option<usize>| -> Result<()> {
                if let Some(p) = parent {
                    app.editor.dispatch(Command::SolidSketch {
                        edit: SketchEdit::Select(Some(p)),
                        continuing: false,
                    })?;
                }
                app.editor.dispatch(Command::SolidSketch {
                    edit: SketchEdit::Add {
                        at,
                        size: Some(size),
                        up: Some(DVec3::Y),
                    },
                    continuing: false,
                })
            };
            // Body, neck, head; two arms from the chest; two legs.
            add(DVec3::new(0., 0., 0.), 0.32, None)?;
            add(DVec3::new(0., 0., -0.55), 0.3, None)?;
            add(DVec3::new(0., 0., -1.0), 0.14, None)?;
            add(DVec3::new(0., 0., -1.35), 0.26, None)?;
            add(DVec3::new(-0.6, 0., -0.6), 0.12, Some(1))?;
            add(DVec3::new(-1.1, 0., -0.3), 0.1, None)?;
            add(DVec3::new(0.6, 0., -0.6), 0.12, Some(1))?;
            add(DVec3::new(1.1, 0., -0.3), 0.1, None)?;
            add(DVec3::new(-0.3, 0., 0.6), 0.14, Some(0))?;
            add(DVec3::new(-0.35, 0., 1.2), 0.12, None)?;
            add(DVec3::new(0.3, 0., 0.6), 0.14, Some(0))?;
            add(DVec3::new(0.35, 0., 1.2), 0.12, None)?;
            app.editor.dispatch(Command::SolidSketch {
                edit: SketchEdit::Select(Some(3)),
                continuing: false,
            })?;
            if mode == "drop" {
                app.editor.dispatch(Command::Drop)?;
            }
            app.clear_fields();
            app.tool_properties_open = true;
        }
        // Maximise on a lone view: the quad (viewport_buttons.rs).
        if frame == 5 && std::env::var_os("OPERANDI_QA_QUAD").is_some() {
            app.toggle_maximized(app.active_view);
        }
        // Viewport Properties open for the view, on a chosen tab.
        if frame == 5
            && let Ok(tabs) = std::env::var("OPERANDI_QA_VIEW_PROPERTIES")
        {
            app.toggle_view_properties(app.active_view, None);
            // "Active" or "Inactive" shows that tab; anything else the first.
            app.corner.properties.tab = match tabs.as_str() {
                "Active" => crate::viewport_properties::Tab::Active,
                "Inactive" => crate::viewport_properties::Tab::Inactive,
                _ => crate::viewport_properties::Tab::Drawing,
            };
        }
        if frame == 5 && std::env::var("OPERANDI_QA_TOOL").as_deref() == Ok("None") {
            // Inspect committed selection separately from face rollover.
            raw.events = vec![Event::PointerMoved(pos2(1., 1.))];
        }
        if frame == 6 && std::env::var_os("OPERANDI_QA_SCROLL").is_some() {
            raw.events = vec![
                Event::PointerMoved(pos2(120., 280.)),
                Event::MouseWheel {
                    unit: MouseWheelUnit::Point,
                    delta: vec2(0., -400.),
                    modifiers: Modifiers::NONE,
                },
            ];
        }
        if let Ok(stage) = std::env::var("OPERANDI_QA_CUBE_STAGE") {
            if frame == 5 {
                let plane = app.actual_plane();
                let normal = match plane {
                    1 => 0,
                    2 => 2,
                    _ => 1,
                };
                let mut a = DVec3::splat(-1.);
                let mut b = DVec3::splat(1.);
                a[normal] = 0.;
                b[normal] = 0.;
                let a = app
                    .camera
                    .project(a, app.viewport_rect)
                    .ok_or("QA Cube start projection")?
                    .0;
                let b = app
                    .camera
                    .project(b, app.viewport_rect)
                    .ok_or("QA Cube end projection")?
                    .0;
                raw.events = vec![
                    Event::PointerButton {
                        pos: a,
                        button: PointerButton::Primary,
                        pressed: true,
                        modifiers: Modifiers::NONE,
                    },
                    Event::PointerMoved(b),
                    Event::PointerButton {
                        pos: b,
                        button: PointerButton::Primary,
                        pressed: false,
                        modifiers: Modifiers::NONE,
                    },
                ];
            }
            if frame == 6 && stage == "solid" {
                let live = app.editor.live.as_ref().ok_or("QA Cube did not activate")?;
                let normal = (0..3)
                    .find(|&n| live.params.size[n] == 0.)
                    .ok_or("QA Cube is not flat")?;
                let pos = live.params.position;
                let h = app
                    .handles(app.viewport_rect)
                    .into_iter()
                    .find(|h| h.0 == 4 + normal * 2)
                    .ok_or("QA Cube depth square missing")?;
                let mut unit = pos;
                unit[normal] += 1.;
                let screen_unit = app
                    .camera
                    .project(unit, app.viewport_rect)
                    .ok_or("QA Cube unit projection")?
                    .0
                    - app
                        .camera
                        .project(pos, app.viewport_rect)
                        .ok_or("QA Cube center projection")?
                        .0;
                let end = h.2 + screen_unit;
                raw.events = vec![
                    Event::PointerButton {
                        pos: h.2,
                        button: PointerButton::Primary,
                        pressed: true,
                        modifiers: Modifiers::NONE,
                    },
                    Event::PointerMoved(end),
                    Event::PointerButton {
                        pos: end,
                        button: PointerButton::Primary,
                        pressed: false,
                        modifiers: Modifiers::NONE,
                    },
                ];
            }
        }
        if primitive_drag.is_some() && (5..=7).contains(&frame) {
            let (u, v) = match app.actual_plane() {
                1 => (DVec3::Y, DVec3::Z),
                2 => (DVec3::X, DVec3::Y),
                _ => (DVec3::X, DVec3::Z),
            };
            let center = app.editor.scene.workplane.center;
            let start = app
                .camera
                .project(center - u - v * 0.5, app.viewport_rect)
                .ok_or("QA primitive start projection")?
                .0;
            let end = app
                .camera
                .project(center + u + v * 0.25, app.viewport_rect)
                .ok_or("QA primitive end projection")?
                .0;
            raw.modifiers = if frame == 7 {
                Modifiers::NONE
            } else {
                draw_modifiers
            };
            raw.events = match frame {
                5 => vec![
                    Event::PointerButton {
                        pos: start,
                        button: PointerButton::Primary,
                        pressed: true,
                        modifiers: draw_modifiers,
                    },
                    Event::PointerMoved(start.lerp(end, 0.3)),
                ],
                6 => vec![Event::PointerMoved(start.lerp(end, 0.6))],
                _ => vec![
                    Event::PointerMoved(end),
                    Event::PointerButton {
                        pos: end,
                        button: PointerButton::Primary,
                        pressed: false,
                        modifiers: draw_modifiers,
                    },
                ],
            };
        }
        let output = ctx.run(raw, |ctx| {
            app.begin_input(ctx);
            app.compose(ctx);
            app.keyboard(ctx);
        });
        let jobs = ctx.tessellate(output.shapes, output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size[0], size[1]],
            pixels_per_point: output.pixels_per_point,
        };
        let mut renderer = state.renderer.write();
        for (id, delta) in &output.textures_delta.set {
            renderer.update_texture(&state.device, &state.queue, *id, delta);
        }
        let mut encoder = state.device.create_command_encoder(&Default::default());
        let mut buffers =
            renderer.update_buffers(&state.device, &state.queue, &mut encoder, &jobs, &screen);
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Operandi full UI simulation"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            renderer.render(&mut pass.forget_lifetime(), &jobs, &screen);
        }
        buffers.push(encoder.finish());
        state.queue.submit(buffers);
        for id in &output.textures_delta.free {
            renderer.free_texture(id);
        }
    }
    let padded = (size[0] * 4).div_ceil(256) * 256;
    let buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("QA pixels"),
        size: u64::from(padded) * u64::from(size[1]),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = state.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: None,
            },
        },
        extent,
    );
    state.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
    state
        .device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| e.to_string())?;
    rx.recv()
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let mapped = buffer.slice(..).get_mapped_range();
    let bgra = matches!(
        state.target_format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    );
    let mut bytes = format!("P6\n{} {}\n255\n", size[0], size[1]).into_bytes();
    for row in mapped.chunks(padded as usize) {
        for c in row[..(size[0] * 4) as usize].chunks(4) {
            if bgra {
                bytes.extend_from_slice(&[c[2], c[1], c[0]]);
            } else {
                bytes.extend_from_slice(&c[..3]);
            }
        }
    }
    std::fs::write(format!("{prefix}.ppm"), bytes).map_err(|e| e.to_string())?;
    let r = app.viewport_rect;
    let selected_edges: Vec<_> = app
        .editor
        .scene
        .active()
        .into_iter()
        .flat_map(|item| {
            app.editor
                .scene
                .selection
                .faces
                .iter()
                .flat_map(|&f| {
                    let vertices = &item.mesh.faces[f as usize].vertices;
                    (0..vertices.len())
                        .filter_map(|i| {
                            let a = app
                                .camera
                                .project(
                                    item.transform
                                        .point(item.mesh.vertices[vertices[i] as usize]),
                                    r,
                                )?
                                .0;
                            let b = app
                                .camera
                                .project(
                                    item.transform.point(
                                        item.mesh.vertices
                                            [vertices[(i + 1) % vertices.len()] as usize],
                                    ),
                                    r,
                                )?
                                .0;
                            Some([[a.x, a.y], [b.x, b.y]])
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let toolbox: BTreeMap<_, _> = app
        .toolbox_rects
        .iter()
        .map(|(k, r)| (k.clone(), [r.left(), r.top(), r.width(), r.height()]))
        .collect();
    let record = serde_json::json!({"smooth_shift_parameters":app.editor.live.as_ref().filter(|l|l.tool.is_smooth_shift()).map(|l|{let p=l.params.smooth_shift;serde_json::json!({"offset":p.offset,"scale":p.scale,"angle":p.angle,"thicken":p.thicken,"sharp":p.sharp,"angle_weight":p.angle_weight,"both_sides":p.both_sides,"interactive":l.applied})}),"spike_strength":app.editor.spike.as_ref().map(|s|s.requested),"ellipsoid":app.editor.ellipsoid.as_ref().map(|s|serde_json::json!({"position":s.requested.position,"radius":s.requested.radius,"top":s.requested.bulge_top,"side":s.requested.bulge_side,"preview_counts":s.preview.as_ref().map(|m|[m.vertices.len(),m.edges().len(),m.faces.len()]),"error":s.error})),"action_context":{"center":app.editor.action_context.center.label(),"axis":app.editor.action_context.axis.label()},"action_frame":app.editor.live.as_ref().and_then(|l|l.action_frame).map(|f|serde_json::json!({"center":f.center,"basis":f.basis})),"polygon_parameters":app.editor.live.as_ref().map(|l|serde_json::json!({"shift":l.params.shift,"inset":l.params.inset,"extent":l.params.extent,"sides":l.params.extrude_sides,"grouped":l.params.group_polygons})),"ground_axes":app.ground_axes,"ground_grid":app.display.settings.drawing.grid,"workplane_display":app.plane_display,"workplane_opacity":app.viewport_preferences.workplane_opacity,"mesh_counts":app.editor.scene.active().map(|i|[i.mesh.vertices.len(),i.mesh.edges().len(),i.mesh.faces.len()]),"primitive_parameters":app.editor.live.as_ref().map(|l|serde_json::json!({"position":l.params.position,"size":l.params.size,"radius":l.params.radial.map(|r|r.radius)})),"gesture_captured":app.navigation.is_some(),"status":app.status,"camera":{"yaw":app.camera.yaw,"pitch":app.camera.pitch,"distance":app.camera.distance,"target":app.camera.target},"actual_plane":app.actual_plane(),"grid_step":grid::spacing(&app.camera,r,app.actual_plane(),app.editor.scene.workplane.center),"handles":app.handles(r).iter().map(|h|serde_json::json!({"id":h.0,"origin":[h.1.x,h.1.y],"end":[h.2.x,h.2.y],"label":h.3})).collect::<Vec<_>>(),"typography":typography::diagnostics(&ctx),"selected_edges":selected_edges,"msaa_samples":app.viewport_gpu.as_ref().map(|g|g.sample_count()),"toolbox":toolbox,"capture":size,"logical_content":[ctx.content_rect().width(),ctx.content_rect().height()],"viewport":[r.left(),r.top(),r.width(),r.height()],"scale":ctx.pixels_per_point(),"native_scale":null,"gpu":app.gpu,"selection":app.editor.scene.selection.faces,"history":app.editor.undo.len(),"live":app.editor.live.as_ref().map(|l|l.tool.label()),"method":"offscreen egui/wgpu; synthetic pointer events; no physical display claim"});
    std::fs::write(format!("{prefix}.json"), record.to_string()).map_err(|e| e.to_string())?;
    Ok(())
}
