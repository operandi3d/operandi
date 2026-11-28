//! Sculpt tab, brush strokes in the viewport and the brush panel
//! (spec sculpt-foundation), driven with synthetic egui events.
use super::toolbox_form_tests::button_by_line;
use super::*;
use operandi_core::sculpt::{NormalMode, OffsetMode, TipKind};
use operandi_core::{SculptBrush, SculptEdit};

const SIZE: Vec2 = Vec2::new(1200., 800.);
/// Grid cells per side.
const N: u32 = 40;

/// `N` × `N` quads on the XY plane, 2 m wide, facing +Z (the Front view).
fn grid() -> operandi_core::Mesh {
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=N {
        for i in 0..=N {
            mesh.vertices.push(DVec3::new(
                2. * i as f64 / N as f64 - 1.,
                2. * j as f64 / N as f64 - 1.,
                0.,
            ));
        }
    }
    for j in 0..N {
        for i in 0..N {
            let a = j * (N + 1) + i;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + N + 2, a + N + 1],
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

/// The grid's middle point.
const MID: usize = ((N / 2) * (N + 1) + N / 2) as usize;

fn app() -> (Operandi, Context) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = grid().into();
    a.editor.history.undo.clear();
    a.camera.view = View::Front;
    a.camera.target = DVec3::ZERO;
    a.camera.distance = 3.;
    a.category = toolbox::Category::Sculpt;
    let ctx = Context::default();
    theme::install(&ctx);
    frame(&mut a, &ctx, vec![]);
    frame(&mut a, &ctx, vec![]);
    (a, ctx)
}

fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
    super::tests::draw(a, ctx, SIZE, events);
    a.keep_sculpt_exclusive();
    a.save_sculpt_settings();
}

fn press(p: Pos2, pressed: bool, button: PointerButton, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos: p,
        button,
        pressed,
        modifiers,
    }
}

/// A primary drag from `from` by `by` in `steps` frames, with `mods`.
fn stroke(a: &mut Operandi, ctx: &Context, from: Pos2, by: Vec2, steps: usize, mods: Modifiers) {
    frame(a, ctx, vec![Event::PointerMoved(from)]);
    frame(
        a,
        ctx,
        vec![press(from, true, PointerButton::Primary, mods)],
    );
    for k in 1..=steps {
        let p = from + by * (k as f32 / steps as f32);
        frame(a, ctx, vec![Event::PointerMoved(p)]);
    }
    frame(
        a,
        ctx,
        vec![press(from + by, false, PointerButton::Primary, mods)],
    );
    frame(a, ctx, vec![]);
}

fn mid(a: &Operandi) -> DVec3 {
    a.editor.scene.active().unwrap().mesh.vertices[MID]
}

fn brush(a: &mut Operandi, b: SculptBrush, size: f64) {
    a.activate_sculpt(b);
    a.sculpt.settings.brush_mut(b).tip.size = size;
}

fn centre(a: &Operandi) -> Pos2 {
    a.camera
        .project(DVec3::ZERO, a.viewport_rect)
        .map(|(p, _)| p)
        .unwrap()
}

#[test]
fn the_sculpt_tab_buttons_turn_brushes_on_and_off() {
    let (mut a, ctx) = app();
    // The Sculpt tab is in the Brush set (Jason, 2026-11-23).
    assert_eq!(a.layouts.tool_set, toolbox::ToolSet::Brush);
    for b in SculptBrush::ALL {
        if b.unavailable().is_some() {
            continue;
        }
        if b == SculptBrush::CenterPinch {
            // The pinch slot (SCT-02): Center Pinch comes from its list;
            // picking it by command puts it on the slot's face.
            a.execute("sculpt.centerPinch");
            assert_eq!(a.sculpt.brush, Some(b));
            assert_eq!(a.sculpt.settings.pinch_face, b);
            a.execute("sculpt.centerPinch");
            frame(&mut a, &ctx, vec![]);
        }
        let key = crate::sculpt_ui::action(b);
        let p = a.toolbox_rects[key].center();
        frame(&mut a, &ctx, vec![Event::PointerMoved(p)]);
        frame(
            &mut a,
            &ctx,
            vec![
                press(p, true, PointerButton::Primary, Modifiers::NONE),
                press(p, false, PointerButton::Primary, Modifiers::NONE),
            ],
        );
        assert_eq!(a.sculpt.brush, Some(b));
        assert_eq!(a.tool_properties_title(), crate::sculpt_ui::title(b));
    }
    // The same button again drops it (SC-93).
    let p = button_by_line(&a, "Sculpt Move").center();
    frame(
        &mut a,
        &ctx,
        vec![
            press(p, true, PointerButton::Primary, Modifiers::NONE),
            press(p, false, PointerButton::Primary, Modifiers::NONE),
        ],
    );
    assert_eq!(a.sculpt.brush, None);
    // The Tips and lock rows are drawn.
    for line in [
        "brush.smooth",
        "brush.hard",
        "sculpt.borderLock",
        "sculpt.cornerLock",
    ] {
        button_by_line(&a, line);
    }
}

#[test]
fn a_push_stroke_raises_the_surface_as_one_undo_step() {
    let (mut a, ctx) = app();
    brush(&mut a, SculptBrush::Push, 60.);
    let c = centre(&a);
    stroke(
        &mut a,
        &ctx,
        c - vec2(30., 0.),
        vec2(60., 0.),
        6,
        Modifiers::NONE,
    );
    let z = mid(&a).z;
    assert!(z > 0.005, "z {z} ({})", a.status);
    assert_eq!(a.editor.history.undo.len(), 1);
    assert_eq!(a.editor.history.undo[0].label, "Push Stroke");
    // The brush stays on for the next stroke (SC-93).
    assert_eq!(a.sculpt.brush, Some(SculptBrush::Push));
    a.run(Command::Undo);
    assert_eq!(mid(&a).z, 0.);
    a.run(Command::Redo);
    assert_eq!(mid(&a).z, z);
}

#[test]
fn ctrl_digs_and_shift_smooths() {
    let (mut a, ctx) = app();
    brush(&mut a, SculptBrush::Push, 60.);
    let c = centre(&a);
    stroke(
        &mut a,
        &ctx,
        c - vec2(30., 0.),
        vec2(60., 0.),
        6,
        Modifiers::CTRL,
    );
    let dug = mid(&a).z;
    assert!(dug < -0.005, "{dug}");
    stroke(
        &mut a,
        &ctx,
        c - vec2(30., 0.),
        vec2(60., 0.),
        6,
        Modifiers::SHIFT,
    );
    assert!(mid(&a).z > dug, "Shift smoothed toward flat");
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Smooth Stroke");
    assert_eq!(a.editor.history.undo.len(), 2);
}

#[test]
fn a_press_off_the_mesh_does_nothing() {
    let (mut a, ctx) = app();
    brush(&mut a, SculptBrush::Push, 60.);
    let corner = a.viewport_rect.left_top() + vec2(20., 60.);
    stroke(&mut a, &ctx, corner, vec2(10., 0.), 2, Modifiers::NONE);
    assert!(a.editor.history.undo.is_empty());
    assert_eq!(mid(&a).z, 0.);
}

#[test]
fn move_drags_the_grabbed_points_in_the_screen_plane() {
    let (mut a, ctx) = app();
    brush(&mut a, SculptBrush::Move, 60.);
    let c = centre(&a);
    stroke(&mut a, &ctx, c, vec2(0., -40.), 4, Modifiers::NONE);
    let p = mid(&a);
    // Screen up is world +Y in the Front view; nothing moves off the plane.
    assert!(p.y > 0.05, "{p:?} ({})", a.status);
    assert!(p.z.abs() < 1e-9);
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Move Stroke");
}

#[test]
fn right_drag_sizes_and_middle_drag_sets_strength() {
    let (mut a, ctx) = app();
    brush(&mut a, SculptBrush::Push, 40.);
    let c = centre(&a);
    frame(
        &mut a,
        &ctx,
        vec![press(c, true, PointerButton::Secondary, Modifiers::NONE)],
    );
    frame(&mut a, &ctx, vec![Event::PointerMoved(c + vec2(25., 5.))]);
    assert_eq!(a.sculpt.settings.push.tip.size, 65.);
    frame(
        &mut a,
        &ctx,
        vec![press(
            c + vec2(25., 5.),
            false,
            PointerButton::Secondary,
            Modifiers::NONE,
        )],
    );
    assert_eq!(a.sculpt.settings.push.tip.size, 65.);
    // Middle-drag: 0.5 % per pixel.
    frame(
        &mut a,
        &ctx,
        vec![press(c, true, PointerButton::Middle, Modifiers::NONE)],
    );
    frame(&mut a, &ctx, vec![Event::PointerMoved(c + vec2(-20., 0.))]);
    frame(
        &mut a,
        &ctx,
        vec![press(
            c + vec2(-20., 0.),
            false,
            PointerButton::Middle,
            Modifiers::NONE,
        )],
    );
    assert!((a.sculpt.settings.push.offset_amount - 0.2).abs() < 1e-9);
    // Neither touched the mesh or the history (SC-91).
    assert!(a.editor.history.undo.is_empty());
}

#[test]
fn panel_edits_change_the_next_stroke_only() {
    let (mut a, _ctx) = app();
    brush(&mut a, SculptBrush::Push, 60.);
    a.form_change::<crate::sculpt_ui::SculptFields>(|p| {
        p.offset_mode = OffsetMode::Absolute;
        p.normal_mode = NormalMode::Z;
    });
    assert_eq!(a.sculpt.settings.push.offset_mode, OffsetMode::Absolute);
    assert_eq!(a.sculpt.settings.push.normal_mode, NormalMode::Z);
    assert!(a.editor.history.undo.is_empty());
    // Each brush keeps its own settings (SC-95).
    a.activate_sculpt(SculptBrush::Inflate);
    assert_eq!(a.sculpt.settings.inflate.offset_mode, OffsetMode::Adaptive);
    a.activate_sculpt(SculptBrush::Push);
    assert_eq!(a.sculpt.settings.push.offset_mode, OffsetMode::Absolute);
}

#[test]
fn brush_settings_survive_a_restart() {
    let (mut a, ctx) = app();
    a.load_sculpt_settings();
    brush(&mut a, SculptBrush::Inflate, 77.);
    a.set_sculpt_lock(false, true);
    frame(&mut a, &ctx, vec![]);
    let mut b = Operandi::empty();
    b.environment = a.environment.clone();
    b.load_sculpt_settings();
    assert_eq!(b.sculpt.settings.inflate.tip.size, 77.);
    assert!(b.sculpt.settings.lock_borders);
}

#[test]
fn commands_drive_the_brushes_tips_and_locks() {
    let (mut a, _ctx) = app();
    a.execute("tool.set sculpt.inflate on");
    assert_eq!(a.sculpt.brush, Some(SculptBrush::Inflate));
    a.execute("tool.set sculpt.inflate ?");
    assert_eq!(a.status, "1");
    a.execute("brush.hard");
    assert_eq!(a.sculpt.settings.inflate.tip.kind, TipKind::Hard);
    a.execute("brush.smooth");
    assert_eq!(a.sculpt.settings.inflate.tip.kind, TipKind::Smooth);
    let corners = a.sculpt.settings.lock_corners;
    a.execute("sculpt.cornerLock");
    assert_eq!(a.sculpt.settings.lock_corners, !corners);
    a.execute("sculpt.borderLock on");
    assert!(a.sculpt.settings.lock_borders);
    a.execute("sculpt.move");
    assert_eq!(a.sculpt.brush, Some(SculptBrush::Move));
    a.execute("Sculpt Smooth");
    assert_eq!(a.sculpt.brush, Some(SculptBrush::Smooth));
    a.execute("tool.set sculpt.smooth off");
    assert_eq!(a.sculpt.brush, None);
}

#[test]
fn a_modeling_tool_turns_the_brush_off_and_back() {
    let (mut a, ctx) = app();
    brush(&mut a, SculptBrush::Push, 60.);
    a.activate(Tool::Move);
    frame(&mut a, &ctx, vec![]);
    assert_eq!(a.sculpt.brush, None);
    a.activate_sculpt(SculptBrush::Push);
    assert!(a.editor.session.live.is_none(), "the brush dropped Move");
    assert_eq!(a.sculpt.brush, Some(SculptBrush::Push));
    // Q drops it (SC-93 said Space; CV-03 gives Space to Mini Properties).
    frame(
        &mut a,
        &ctx,
        vec![Event::Key {
            key: Key::Q,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert_eq!(a.sculpt.brush, None);
}

#[test]
fn symmetry_strokes_both_sides_in_one_step() {
    let (mut a, ctx) = app();
    a.run(Command::Symmetry(operandi_core::Symmetry::x()));
    brush(&mut a, SculptBrush::Push, 40.);
    let at = a
        .camera
        .project(DVec3::new(0.5, 0., 0.), a.viewport_rect)
        .unwrap()
        .0;
    stroke(&mut a, &ctx, at, vec2(1., 0.), 1, Modifiers::NONE);
    let mesh = &a.editor.scene.active().unwrap().mesh;
    let row = (N / 2) * (N + 1);
    let (right, left) = (
        mesh.vertices[(row + N / 2 + N / 4) as usize].z,
        mesh.vertices[(row + N / 2 - N / 4) as usize].z,
    );
    assert!(right > 0.);
    assert!((right - left).abs() < 1e-9, "{right} {left}");
    assert_eq!(a.editor.history.undo.len(), 1, "both sides are one step");
}

/// SC-110 probe (opt-in, release): stroke frames on a 100k-point grid with
/// a 100 px brush, through the whole viewport frame (GPU geometry patched
/// on the CPU side). Run with
/// `cargo test --release -p operandi-app sculpt_frame_time -- --ignored --nocapture`
/// (see `OPERANDI_SCULPT_PROBE_SELECTION` below for a selection).
#[test]
#[ignore]
fn sculpt_frame_time_100k_points() {
    let n = 316u32;
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            mesh.vertices.push(DVec3::new(
                2. * i as f64 / n as f64 - 1.,
                2. * j as f64 / n as f64 - 1.,
                0.,
            ));
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
    for b in [SculptBrush::Push, SculptBrush::Smooth, SculptBrush::Move] {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.editor.scene.active_mut().unwrap().mesh = mesh.clone().into();
        a.force_resident = true;
        a.camera.view = View::Front;
        a.camera.target = DVec3::ZERO;
        let ctx = Context::default();
        theme::install(&ctx);
        // The mesh fills the view's height, as when sculpting it.
        a.camera.distance = 1.8;
        // OPERANDI_SCULPT_PROBE_SELECTION=shown selects the middle half of
        // the rows in Polygon mode, drawn while sculpting; `hidden` selects
        // them with Show Selections off (the cost before the overlay).
        if let Ok(how) = std::env::var("OPERANDI_SCULPT_PROBE_SELECTION") {
            a.run(Command::Mode(Mode::Polygon));
            a.editor.scene.selection.faces = (n * n / 4..3 * n * n / 4).collect();
            a.display.settings.drawing.selections = how != "hidden";
            println!("selection {how}: {} polygons", n * n / 2);
        }
        for _ in 0..2 {
            super::tests::draw(&mut a, &ctx, SIZE, vec![]);
        }
        let span = |x: f64| {
            a.camera
                .project(DVec3::new(x, 0., 0.), a.viewport_rect)
                .unwrap()
                .0
                .x
        };
        println!(
            "{b:?}: mesh {:.0} px wide in a {:.0} px view",
            span(1.) - span(-1.),
            a.viewport_rect.width()
        );
        brush(&mut a, b, 100.);
        let c = centre(&a);
        let at = |k: f32| c + vec2(-200. + k * 8., (k * 0.2).sin() * 60.);
        let mut idle = Vec::new();
        for k in 0..20 {
            let t = std::time::Instant::now();
            super::tests::draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at(k as f32))]);
            idle.push(t.elapsed().as_secs_f64() * 1e3);
        }
        idle.sort_by(f64::total_cmp);
        println!(
            "{b:?} idle hover frame median {:.1} ms",
            idle[idle.len() / 2]
        );
        super::tests::draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at(0.))]);
        let t = std::time::Instant::now();
        super::tests::draw(
            &mut a,
            &ctx,
            SIZE,
            vec![press(at(0.), true, PointerButton::Primary, Modifiers::NONE)],
        );
        let press_ms = t.elapsed().as_secs_f64() * 1e3;
        let builds = |a: &Operandi| {
            [
                a.surface_cache.builds(),
                a.resident_cache.builds(),
                operandi_core::topology_cache::computed(),
            ]
        };
        let before = builds(&a);
        let mut times = Vec::new();
        // OPERANDI_SCULPT_PROBE_FRAMES lengthens the stroke for a profiler.
        let frames: usize = std::env::var("OPERANDI_SCULPT_PROBE_FRAMES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);
        for k in 1..=frames {
            let k = (k % 50).max(1);
            let t = std::time::Instant::now();
            super::tests::draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at(k as f32))]);
            times.push(t.elapsed().as_secs_f64() * 1e3);
        }
        println!(
            "{b:?} builds during the stroke (surface, GPU, topology): {:?} -> {:?}",
            before,
            builds(&a)
        );
        let t = std::time::Instant::now();
        super::tests::draw(
            &mut a,
            &ctx,
            SIZE,
            vec![press(
                at(50.),
                false,
                PointerButton::Primary,
                Modifiers::NONE,
            )],
        );
        let release_ms = t.elapsed().as_secs_f64() * 1e3;
        times.sort_by(f64::total_cmp);
        println!(
            "{:?} 100k points, 100 px: press {press_ms:.1} ms, frame median {:.1} ms, p90 {:.1} ms, max {:.1} ms, release {release_ms:.1} ms",
            b,
            times[times.len() / 2],
            times[times.len() * 9 / 10],
            times[times.len() - 1]
        );
        assert_eq!(
            a.editor.history.undo.last().map(|c| c.label.clone()),
            Some(format!("{} Stroke", b.label()))
        );
    }
}

#[test]
fn the_stroke_command_is_scriptable_and_saves_like_any_edit() {
    let (mut a, _ctx) = app();
    let id = a.editor.scene.active().unwrap().id;
    let edit = SculptEdit {
        label: "Push Stroke".into(),
        items: vec![operandi_core::SculptItemEdit {
            item: id,
            points: vec![(MID as u32, DVec3::new(0., 0., 0.25))],
        }],
    };
    assert!(a.run_ok(Command::SculptStroke(Box::new(edit))));
    // SC-94: plain mesh positions, so they save and reopen.
    let dir = std::env::temp_dir().join(format!("operandi-sculpt-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sculpted.opscene");
    a.editor.save_committed(&path).unwrap();
    let scene = operandi_core::persistence::load(&path).unwrap();
    let v = scene.item(id).unwrap().mesh.vertices[MID];
    assert_eq!(v.z, 0.25);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Batch 12: the Duplicate tab tools and the Sculpt brushes draw their own
/// icons (UIC-22), not the Script Action stand-in, and no two share one.
#[test]
fn duplicate_tools_and_sculpt_brushes_have_their_own_icons() {
    let duplicate = [
        (Tool::Lathe, icons::Icon::RadialSweep),
        (Tool::Clone, icons::Icon::Clone),
        (Tool::Array, icons::Icon::Array),
        (Tool::RadialArray, icons::Icon::RadialArray),
        (Tool::Mirror, icons::Icon::Mirror),
    ];
    for (tool, icon) in duplicate {
        assert_eq!(tool_info::info(tool).icon, icon, "{tool:?}");
    }
    let brushes = [
        (SculptBrush::Smooth, icons::Icon::SculptSmooth),
        (SculptBrush::Push, icons::Icon::SculptPush),
        (SculptBrush::Inflate, icons::Icon::SculptInflate),
        (SculptBrush::Move, icons::Icon::SculptMove),
        (SculptBrush::Fold, icons::Icon::SculptFold),
        (SculptBrush::Clay, icons::Icon::SculptClay),
        (SculptBrush::Smudge, icons::Icon::SculptSmudge),
        (SculptBrush::Carve, icons::Icon::SculptCarve),
        (SculptBrush::Scar, icons::Icon::SculptScar),
        (SculptBrush::Flatten, icons::Icon::SculptFlatten),
        (SculptBrush::TangentPinch, icons::Icon::SculptTangentPinch),
        (SculptBrush::CenterPinch, icons::Icon::SculptCenterPinch),
        (SculptBrush::Emboss, icons::Icon::SculptEmboss),
        (SculptBrush::Attenuate, icons::Icon::SculptAttenuate),
        (SculptBrush::Spin, icons::Icon::SculptSpin),
        (SculptBrush::VmPaint, icons::Icon::VmapPaint),
        (SculptBrush::VmSmooth, icons::Icon::VmapSmooth),
        (SculptBrush::VmScaleDown, icons::Icon::VmapScaleDown),
        (SculptBrush::VmErase, icons::Icon::VmapErase),
        (SculptBrush::VmSetValue, icons::Icon::VmapSetValue),
    ];
    assert_eq!(brushes.len(), SculptBrush::every().count());
    for (brush, icon) in brushes {
        assert_eq!(sculpt_ui::brush_icon(brush), icon, "{brush:?}");
    }
    let mut all: Vec<_> = duplicate.iter().map(|&(_, i)| i).collect();
    all.extend(brushes.iter().map(|&(_, i)| i));
    all.push(icons::Icon::Detriangulate);
    assert!(!all.contains(&icons::Icon::ScriptAction));
    let unique: std::collections::BTreeSet<_> = all.iter().collect();
    assert_eq!(unique.len(), all.len());
}

// ---- pen tablet (pen tablet plan stage 2, SC-45, SC-62) ----

/// A primary drag like [`stroke`], with the pen sample AppKit would send
/// for each pointer event recorded first (`pressure` at every event).
fn pen_stroke(
    a: &mut Operandi,
    ctx: &Context,
    from: Pos2,
    by: Vec2,
    steps: usize,
    pressure: impl Fn(f32) -> f32,
    tool: crate::tablet::PenTool,
) {
    use crate::tablet::{Phase, Raw, record};
    let sample = |p: Pos2, t: f32, phase: Phase| {
        record(Raw::Point {
            pressure: pressure(t),
            tilt: [0.1, -0.2],
            rotation: 0.,
            phase,
            timestamp: f64::from(t),
            pos: Some([p.x, p.y]),
        })
    };
    record(Raw::Proximity {
        entering: true,
        tool,
    });
    frame(a, ctx, vec![Event::PointerMoved(from)]);
    sample(from, 0., Phase::Down);
    frame(
        a,
        ctx,
        vec![press(from, true, PointerButton::Primary, Modifiers::NONE)],
    );
    for k in 1..=steps {
        let t = k as f32 / steps as f32;
        let p = from + by * t;
        sample(p, t, Phase::Drag);
        frame(a, ctx, vec![Event::PointerMoved(p)]);
    }
    sample(from + by, 1., Phase::Up);
    frame(
        a,
        ctx,
        vec![press(
            from + by,
            false,
            PointerButton::Primary,
            Modifiers::NONE,
        )],
    );
    record(Raw::Proximity {
        entering: false,
        tool,
    });
    frame(a, ctx, vec![]);
}

fn push_z_with(pressure: impl Fn(f32) -> f32, tool: crate::tablet::PenTool) -> (f64, String) {
    let (mut a, ctx) = app();
    brush(&mut a, SculptBrush::Push, 60.);
    let c = centre(&a);
    pen_stroke(
        &mut a,
        &ctx,
        c - vec2(30., 0.),
        vec2(60., 0.),
        6,
        pressure,
        tool,
    );
    assert_eq!(a.editor.history.undo.len(), 1, "one stroke, one undo step");
    (mid(&a).z, a.editor.history.undo[0].label.clone())
}

#[test]
fn pen_pressure_scales_strength_and_full_pressure_matches_the_mouse() {
    use crate::tablet::PenTool;
    let (mut a, ctx) = app();
    brush(&mut a, SculptBrush::Push, 60.);
    let c = centre(&a);
    stroke(
        &mut a,
        &ctx,
        c - vec2(30., 0.),
        vec2(60., 0.),
        6,
        Modifiers::NONE,
    );
    let mouse = mid(&a).z;
    let (full, _) = push_z_with(|_| 1., PenTool::Tip);
    assert!(
        (full - mouse).abs() < 1e-12,
        "pen at full {full} vs mouse {mouse}"
    );
    let (light, label) = push_z_with(|_| 0.3, PenTool::Tip);
    assert_eq!(label, "Push Stroke");
    assert!(
        light > 0. && light < mouse * 0.6,
        "light {light} vs {mouse}"
    );
    // Pressure rising along the stroke lands between the two.
    let (ramp, _) = push_z_with(|t| 0.3 + 0.7 * t, PenTool::Tip);
    assert!(ramp > light && ramp < full, "{light} < {ramp} < {full}");
}

#[test]
fn the_eraser_end_inverts_or_smooths_as_the_preference_says() {
    use crate::tablet::{EraserAction, PenTool};
    let (z, label) = push_z_with(|_| 1., PenTool::Eraser);
    assert!(z < -0.005, "eraser digs like Ctrl: {z}");
    assert_eq!(label, "Push Stroke");

    let (mut a, ctx) = app();
    a.input_preferences.pen_eraser_action = EraserAction::Smooth;
    brush(&mut a, SculptBrush::Push, 60.);
    let c = centre(&a);
    pen_stroke(
        &mut a,
        &ctx,
        c - vec2(30., 0.),
        vec2(60., 0.),
        6,
        |_| 1.,
        PenTool::Eraser,
    );
    assert_eq!(a.editor.history.undo[0].label, "Smooth Stroke");
}

#[test]
fn low_pressure_greys_until_a_pen_is_seen_and_then_smooths_light_strokes() {
    use crate::sculpt_ui::SculptFields;
    use crate::tablet::PenTool;
    use crate::tool_form::ToolFields;
    let greyed = |a: &Operandi| {
        let p = SculptFields::params(a).unwrap();
        let form = SculptFields::form(a, &p);
        let field = form
            .fields()
            .into_iter()
            .find(|f| f.row.label == "Smooth Mode")
            .expect("Smooth Mode row");
        // SI-15: Shift Key, Low Pressure, Uniform.
        field.greyed_options.iter().any(|(i, _)| *i == 1)
    };
    let (mut a, ctx) = app();
    brush(&mut a, SculptBrush::Push, 60.);
    assert!(greyed(&a), "no pen yet");
    crate::tablet::record(crate::tablet::Raw::Proximity {
        entering: true,
        tool: PenTool::Tip,
    });
    assert!(!greyed(&a), "a pen came near the tablet");

    // A bump, then light and hard strokes over it with Low Pressure on.
    let run = |pressure: f32| {
        let (mut a, ctx) = app();
        a.editor.scene.active_mut().unwrap().mesh.vertices[MID].z = 0.05;
        brush(&mut a, SculptBrush::Push, 60.);
        a.sculpt.settings.push.smooth_mode = operandi_core::sculpt::SmoothMode::LowPressure;
        let c = centre(&a);
        pen_stroke(
            &mut a,
            &ctx,
            c - vec2(30., 0.),
            vec2(60., 0.),
            6,
            |_| pressure,
            PenTool::Tip,
        );
        mid(&a).z
    };
    assert!(run(0.1) < 0.05, "light strokes smooth");
    assert!(run(0.9) > 0.05, "hard strokes sculpt");
    drop((a, ctx));
}

#[test]
fn pressure_sets_size_only_when_turned_on() {
    use crate::tablet::PenTool;
    // Off (default): a light stroke reaches as far as a hard one.
    let reach = |size: bool| {
        let (mut a, ctx) = app();
        a.input_preferences.pen_pressure_size = size;
        brush(&mut a, SculptBrush::Push, 60.);
        let c = centre(&a);
        pen_stroke(
            &mut a,
            &ctx,
            c - vec2(30., 0.),
            vec2(60., 0.),
            6,
            |_| 0.3,
            PenTool::Tip,
        );
        a.editor
            .scene
            .active()
            .unwrap()
            .mesh
            .vertices
            .iter()
            .filter(|v| v.z.abs() > 1e-9)
            .count()
    };
    let off = reach(false);
    let on = reach(true);
    assert!(on > 0 && on < off, "size follows pressure: {on} < {off}");
}

/// SCT acceptance 2: each toolbox brush's form has exactly the spec's rows,
/// in order, with its starting values.
#[test]
fn toolbox_brush_forms_have_the_spec_rows() {
    use crate::sculpt_ui::SculptFields;
    use crate::tool_form::ToolFields;
    let labels = |a: &Operandi| -> Vec<String> {
        let p = SculptFields::params(a).unwrap();
        SculptFields::form(a, &p)
            .fields()
            .into_iter()
            .map(|f| f.row.label.clone())
            .collect()
    };
    let tip = ["Size", "Auto Scale", "Bias", "Shape Preset"];
    let rows = |brush: &[&str], stroke: &[&str]| -> Vec<String> {
        brush
            .iter()
            .chain(stroke)
            .chain(tip.iter())
            .map(|s| s.to_string())
            .collect()
    };
    let stroke = [
        "Interpolation Step",
        "Continuous Stroke",
        "Curve Constraint",
    ];
    let offset = ["Offset Mode", "Offset Distance", "Offset Amount"];
    let cases: Vec<(SculptBrush, Vec<String>)> = vec![
        (
            SculptBrush::Fold,
            rows(
                &[
                    &["Sculpt Mode"][..],
                    &offset,
                    &["Smooth Amount", "Smooth Mode"],
                ]
                .concat(),
                &stroke,
            ),
        ),
        (
            SculptBrush::Clay,
            rows(
                &[
                    "Offset Amount",
                    "Level",
                    "Smooth Amount",
                    "Smooth Mode",
                    "Normal Mode",
                    "Density Mode",
                ],
                &stroke,
            ),
        ),
        (
            SculptBrush::Smudge,
            rows(
                &[
                    "Sculpt Mode",
                    "Offset Amount",
                    "Smooth Amount",
                    "Smooth Mode",
                ],
                &[],
            ),
        ),
        (
            SculptBrush::Scar,
            rows(&["Offset Amount", "Smooth Amount", "Smooth Mode"], &[]),
        ),
        (
            SculptBrush::TangentPinch,
            rows(
                &[
                    &["Sculpt Mode"][..],
                    &offset,
                    &["Smooth Amount", "Curve Constraint"],
                ]
                .concat(),
                &[],
            ),
        ),
        (
            SculptBrush::Emboss,
            rows(&["Sculpt Mode", "Offset Amount"], &[]),
        ),
        (
            SculptBrush::Spin,
            rows(
                &[
                    &["Sculpt Mode"][..],
                    &offset,
                    &["Smooth Amount", "Normal Mode"],
                ]
                .concat(),
                &[],
            ),
        ),
    ];
    let (mut a, _ctx) = app();
    for (brush, want) in cases {
        a.activate_sculpt(brush);
        assert_eq!(a.sculpt.brush, Some(brush));
        assert_eq!(labels(&a), want, "{brush:?}");
        a.drop_sculpt();
    }
    // Flatten opens with Inverted Flatten off and Brush Center.
    a.activate_sculpt(SculptBrush::Flatten);
    let l = labels(&a);
    assert_eq!(l[1], "Inverted Flatten");
    let p = SculptFields::params(&a).unwrap();
    assert!(!p.inverted_flatten);
    assert_eq!(p.offset_amount, 0.5);
    a.drop_sculpt();
    // Move gains Move Mode after Sculpt Mode (SCT-49).
    a.activate_sculpt(SculptBrush::Move);
    assert_eq!(labels(&a)[..2], ["Sculpt Mode", "Move Mode"]);
}

/// SCT-45: on a plain mesh Attenuate is greyed with its reason and its command refuses.
#[test]
fn attenuate_waits_for_a_multiresolution_level() {
    if !SculptBrush::Attenuate.unavailable().is_some() {
        return;
    }
    let (mut a, ctx) = app();
    let p = button_by_line(&a, "Sculpt Attenuate").center();
    frame(
        &mut a,
        &ctx,
        vec![
            press(p, true, PointerButton::Primary, Modifiers::NONE),
            press(p, false, PointerButton::Primary, Modifiers::NONE),
        ],
    );
    assert_eq!(a.sculpt.brush, None);
    a.execute("sculpt.shrink");
    assert_eq!(a.sculpt.brush, None);
    assert!(a.status.contains("multiresolution"), "{}", a.status);
}

/// VMT-01..19 through the app: the section's buttons, a Paint stroke with
/// no map selected makes "Weight" and paints it as one undo step (VMT-18),
/// and the falloff then masks a sculpt stroke.
#[test]
fn vertex_map_tools_paint_through_the_viewport() {
    let (mut a, ctx) = app();
    for b in SculptBrush::VERTEX_MAP {
        assert!(
            a.toolbox_rects.contains_key(crate::sculpt_ui::action(b)),
            "{b:?} button"
        );
    }
    for key in ["Live Deformers", "New Weight Map"] {
        assert!(a.toolbox_rects.contains_key(key), "{key}");
    }
    brush(&mut a, SculptBrush::VmPaint, 60.);
    assert_eq!(a.tool_properties_title(), "Vertex Map: Paint");
    let c = centre(&a);
    stroke(
        &mut a,
        &ctx,
        c - vec2(10., 0.),
        vec2(20., 0.),
        4,
        Modifiers::NONE,
    );
    assert_eq!(a.editor.selected_weight_map.as_deref(), Some("Weight"));
    let map = a
        .editor
        .scene
        .active()
        .unwrap()
        .mesh
        .attributes
        .point_weights["Weight"]
        .clone();
    assert!(map.get(&(MID as u32)).is_some_and(|w| *w > 0.4), "{map:?}");
    assert_eq!(
        a.editor.history.undo.last().unwrap().label,
        "Vertex Map Paint Stroke"
    );
    assert_eq!(mid(&a).z, 0., "painting moves nothing");
    // Command-line forms.
    a.execute("vertMap.new Second weight true value:0.25");
    assert_eq!(a.editor.selected_weight_map.as_deref(), Some("Second"));
    assert_eq!(
        a.editor
            .scene
            .active()
            .unwrap()
            .mesh
            .attributes
            .point_weights["Second"]
            .values()
            .next(),
        Some(&0.25)
    );
    a.execute("select.vertexMap Weight weight set");
    assert_eq!(a.editor.selected_weight_map.as_deref(), Some("Weight"));
    a.execute("sculpt.liveWeightEdit live:false");
    assert!(!a.sculpt.settings.live_deformers);
    a.execute("tool.set sculpt.vmapErase on");
    assert_eq!(a.sculpt.brush, Some(SculptBrush::VmErase));
    // The Add Falloff route: Vertex Map, then Invert on top (VM-40).
    a.execute("tool.set falloff.vertexMap on");
    a.execute("falloff.invert");
    assert!(matches!(
        a.editor.spatial_falloff,
        Some(operandi_core::Falloff::VertexMap(_))
    ));
    assert_eq!(a.editor.added_falloffs.len(), 1);
    a.execute("tool.set falloff.invert off");
    assert!(a.editor.added_falloffs.is_empty());
    a.execute("Vertex Map Set Value");
    assert_eq!(a.sculpt.brush, Some(SculptBrush::VmSetValue));
}

#[test]
fn n32_weight_paint_does_not_draw_weight_tinted_vertex_dots() {
    let (mut a, ctx) = app();
    a.run(Command::NewWeightMap {
        name: "Weight".into(),
        value: Some(1.),
    });
    brush(&mut a, SculptBrush::VmPaint, 60.);
    let output = super::tests::draw(&mut a, &ctx, SIZE, vec![]);
    let p = centre(&a);
    assert_eq!(a.sculpt.brush, Some(SculptBrush::VmPaint));
    assert!(!output.shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::Shape::Circle(circle)
            if circle.center.distance(p) < 1.
                && circle.fill == crate::viewport::weight_colour(1.))
    }));
}

/// VMT-30 / VM-CASE-16: the weight shading colours.
#[test]
fn weight_colours_match_the_measured_shading() {
    let want = [
        (-1., (0, 0, 255)),
        (-0.5, (64, 70, 191)),
        (0., (128, 140, 128)),
        (0.25, (159, 105, 96)),
        (0.5, (191, 70, 64)),
        (1., (255, 0, 0)),
        (2., (255, 0, 0)),
    ];
    for (w, (r, g, b)) in want {
        assert_eq!(
            crate::viewport::weight_colour(w),
            egui::Color32::from_rgb(r, g, b),
            "{w}"
        );
    }
}

/// Astra P5-001: a dialog taking the input mid-stroke ends the stroke as
/// its own undo step; the next stroke is another, and two Undos give back
/// the flat grid. P5-002: while the stroke is held, no timed backup is
/// taken of its tentative points.
#[test]
fn a_dialog_mid_stroke_ends_it_as_its_own_step() {
    let (mut a, ctx) = app();
    brush(&mut a, SculptBrush::Push, 60.);
    let c = centre(&a);
    let from = c - vec2(30., 0.);
    frame(&mut a, &ctx, vec![Event::PointerMoved(from)]);
    frame(
        &mut a,
        &ctx,
        vec![press(from, true, PointerButton::Primary, Modifiers::NONE)],
    );
    for k in 1..=3 {
        frame(
            &mut a,
            &ctx,
            vec![Event::PointerMoved(from + vec2(10. * k as f32, 0.))],
        );
    }
    assert!(a.app_stroke_held());
    assert!(
        !a.editor.backup_ready() || a.app_stroke_held(),
        "the backup waits"
    );
    // A dialog opens before the release.
    a.show_about = true;
    frame(&mut a, &ctx, vec![]);
    assert!(!a.app_stroke_held(), "the stroke ended");
    assert_eq!(a.editor.history.undo.len(), 1, "as its own step");
    a.show_about = false;
    frame(&mut a, &ctx, vec![]);
    // The release lands late, then a second stroke.
    frame(
        &mut a,
        &ctx,
        vec![press(
            from + vec2(30., 0.),
            false,
            PointerButton::Primary,
            Modifiers::NONE,
        )],
    );
    stroke(
        &mut a,
        &ctx,
        c - vec2(0., 30.),
        vec2(0., 60.),
        6,
        Modifiers::NONE,
    );
    assert_eq!(a.editor.history.undo.len(), 2, "two strokes, two steps");
    a.run(Command::Undo);
    a.run(Command::Undo);
    assert_eq!(mid(&a).z, 0., "back to the flat grid");
}

/// The brush form as (section title, its field labels), top to bottom.
fn form_sections(a: &Operandi) -> Vec<(String, Vec<String>)> {
    use crate::sculpt_ui::SculptFields;
    use crate::tool_form::{Item, ToolFields};
    let p = SculptFields::params(a).unwrap();
    SculptFields::form(a, &p)
        .items
        .into_iter()
        .filter_map(|item| match item {
            Item::Section { name, items, .. } => Some((
                name.to_string(),
                items
                    .iter()
                    .filter_map(|i| match i {
                        Item::Field(f) => Some(f.row.label.clone()),
                        // The locks and Brush Settings draw themselves.
                        Item::Custom(_) => Some("(own)".into()),
                        Item::Separator => Some("—".into()),
                        _ => None,
                    })
                    .collect(),
            )),
            _ => None,
        })
        .collect()
}

/// SI §2: Inflate's form, exact rows and order, in one block plus the
/// "Smooth Brush" section; no Normal Mode row (SI-04), no Stroke section.
#[test]
fn inflate_form_has_the_spec_rows() {
    let (mut a, _ctx) = app();
    a.activate_sculpt(SculptBrush::Inflate);
    let s = form_sections(&a);
    let names: Vec<&str> = s.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["Sculpt: Inflate", "Smooth Brush"]);
    assert_eq!(
        s[0].1,
        [
            "Sculpt Mode",
            "Offset Mode",
            "Offset Distance",
            "Offset Amount",
            "Smooth Amount",
            "Smooth Mode",
            "Density Mode",
            "—",
            "Interpolation Step",
            "Continuous Stroke",
            "Curve Constraint",
            "—",
            "Paint Mode",
            "—",
            "Set Color",
            "Color",
            "Alpha",
            "—",
            "(own)",
        ]
    );
    assert_eq!(
        s[1].1,
        ["Size", "Auto Scale", "Bias", "Shape Preset", "(own)"]
    );
    a.execute("brush.hard");
    let s = form_sections(&a);
    assert_eq!(s[1].0, "Hard Brush");
    assert_eq!(s[1].1, ["Size", "Auto Scale", "Soft Border", "Bias"]);
}

/// SI-10, SI-15, SI-16, SI-20..23: choice lists, their order, and the
/// image-only rows greyed with a note (Jason's "match", SI-Q3).
#[test]
fn inflate_choices_and_image_rows_are_greyed_with_a_note() {
    use crate::form::{Kind, When};
    use crate::sculpt_ui::SculptFields;
    use crate::tool_form::ToolFields;
    let (mut a, _ctx) = app();
    a.activate_sculpt(SculptBrush::Inflate);
    let p = SculptFields::params(&a).unwrap();
    let form = SculptFields::form(&a, &p);
    let field = |label: &str| {
        *form
            .fields()
            .iter()
            .find(|f| f.row.label == label)
            .unwrap_or_else(|| panic!("{label} row"))
    };
    let names = |label: &str| match &field(label).row.kind {
        Kind::Choice(n) => n.clone(),
        k => panic!("{label} is {k:?}"),
    };
    assert_eq!(
        names("Sculpt Mode"),
        ["Automatic", "Mesh", "Image Map (Details)"]
    );
    assert_eq!(field("Sculpt Mode").greyed_options.len(), 1);
    assert_eq!(
        names("Smooth Mode"),
        ["Shift Key", "Low Pressure", "Uniform"]
    );
    assert_eq!(
        names("Density Mode"),
        ["Additive", "Attenuated", "Maximum Stroke", "Maximum Map"]
    );
    assert!(
        field("Density Mode")
            .greyed_options
            .iter()
            .any(|(i, _)| *i == 3)
    );
    // SI-16 default: Attenuated.
    assert_eq!(
        p.density_mode,
        operandi_core::sculpt::DensityMode::Attenuate
    );
    for label in ["Paint Mode", "Set Color", "Color", "Alpha"] {
        let f = field(label);
        assert_eq!(f.row.when, When::Never, "{label} greyed");
        assert!(f.disabled_tip.is_some(), "{label} says why");
    }
    // The live rows stay live.
    for label in ["Sculpt Mode", "Offset Mode", "Smooth Mode", "Density Mode"] {
        assert_ne!(field(label).row.when, When::Never, "{label} live");
    }
}

/// SI-24: the form's locks are the Sculpt tab's switches.
#[test]
fn the_form_locks_are_the_sculpt_tab_switches() {
    use crate::sculpt_ui::SculptFields;
    use crate::tool_form::ToolFields;
    let (mut a, ctx) = app();
    a.activate_sculpt(SculptBrush::Inflate);
    assert!(!a.sculpt.settings.lock_borders && a.sculpt.settings.lock_corners);
    // Every sculpt brush's form ends with them (SCT-01); the vertex map
    // tools' don't.
    for brush in [
        SculptBrush::Push,
        SculptBrush::Smooth,
        SculptBrush::Move,
        SculptBrush::Clay,
    ] {
        let (rows, _) = crate::sculpt_ui::toolbox_rows(brush);
        assert_eq!(
            rows.last(),
            Some(&crate::sculpt_ui::Row::Locks),
            "{brush:?}"
        );
    }
    let (rows, _) = crate::sculpt_ui::toolbox_rows(SculptBrush::VmPaint);
    assert!(!rows.contains(&crate::sculpt_ui::Row::Locks));
    // Ticking Lock Borders in the panel ticks the tab's switch.
    let p = SculptFields::params(&a).unwrap();
    assert!(SculptFields::form(&a, &p).items.iter().any(|i| matches!(
        i,
        crate::tool_form::Item::Section { items, .. }
            if items.iter().any(|i| matches!(i, crate::tool_form::Item::Custom(_)))
    )));
    a.execute("sculpt.borderLock on");
    frame(&mut a, &ctx, vec![]);
    assert!(a.sculpt.settings.lock_borders);
}

/// SPM-10..30: Push, Smooth and Move on the row-list form, with their
/// starting values (Normal Mode Average on Push, Offset Amount 25 % on
/// Smooth and 100 % on Move).
#[test]
fn push_smooth_and_move_use_the_row_list_form() {
    use crate::tool_form::ToolFields;
    let (mut a, _ctx) = app();
    for (brush, want) in [
        (
            SculptBrush::Push,
            &[
                "Sculpt Mode",
                "Offset Mode",
                "Offset Distance",
                "Offset Amount",
                "Smooth Amount",
                "Smooth Mode",
                "Normal Mode",
                "Density Mode",
                "—",
                "Interpolation Step",
                "Continuous Stroke",
                "Curve Constraint",
                "—",
                "Set Color",
                "Color",
                "Alpha",
                "—",
                "(own)",
            ][..],
        ),
        (
            SculptBrush::Smooth,
            &["Sculpt Mode", "Offset Amount", "Soft Filter", "—", "(own)"],
        ),
        (
            SculptBrush::Move,
            &[
                "Sculpt Mode",
                "Move Mode",
                "Move Back Faces",
                "Offset Amount",
                "Smooth Amount",
                "Rigid UVs",
                "—",
                "(own)",
            ],
        ),
    ] {
        a.activate_sculpt(brush);
        let p = crate::sculpt_ui::SculptFields::params(&a).unwrap();
        match brush {
            SculptBrush::Push => assert_eq!(p.normal_mode, NormalMode::Average),
            SculptBrush::Smooth => assert_eq!(p.offset_amount, 0.25),
            _ => assert_eq!(p.offset_amount, 1.),
        }
        let s = form_sections(&a);
        assert_eq!(s.len(), 2, "{brush:?}: brush block and tip, no Stroke");
        assert_eq!(s[0].1, want, "{brush:?}");
        assert_eq!(s[1].0, "Smooth Brush");
        a.drop_sculpt();
    }
}

/// SI-34: Brush Settings' values drive the tip's Custom falloff, and the
/// Shape Preset row then reads "Custom".
#[test]
fn custom_falloff_shows_as_custom_in_shape_preset() {
    use crate::sculpt_ui::SculptFields;
    use crate::tool_form::{Bind, ToolFields};
    use operandi_core::sculpt::TipShape;
    let (mut a, _ctx) = app();
    a.activate_sculpt(SculptBrush::Inflate);
    let tip = &mut a.sculpt.settings.inflate.tip;
    tip.custom.in_weight = 0.8;
    tip.shape = TipShape::Custom;
    let p = SculptFields::params(&a).unwrap();
    let form = SculptFields::form(&a, &p);
    let shape = form
        .fields()
        .into_iter()
        .find(|f| f.row.label == "Shape Preset")
        .unwrap();
    let Bind::Choice { get, .. } = &shape.bind else {
        panic!("choice");
    };
    assert_eq!(get(&p), usize::MAX, "no preset matches");
    assert_eq!(shape.otherwise.as_deref(), Some("Custom"));
    // Picking a preset again leaves Custom.
    a.form_change::<SculptFields>(|p| p.tip.shape = TipShape::Fat);
    assert_eq!(a.sculpt.settings.inflate.tip.shape, TipShape::Fat);
}
