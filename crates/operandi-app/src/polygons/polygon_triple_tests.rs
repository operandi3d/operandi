//! MP-C04 TRI-001/004/005 app routes for basic Triple. The Polygon panel row
//! is clicked with synthetic egui pointer events at its AccessKit bounds;
//! Shift+T and a user remap are synthetic key events over the viewport; the
//! command route is the production `execute` entry. Each route runs on its own
//! equal original fixture and the complete resulting scenes are compared.
//! These are worker-local synthetic-event checks: no native input, focus,
//! display-scale or native GUI claim is made.
//!
//! Fixture: the admitted original adjacent quads (face 0 selected) with
//! independently authored corner UVs/tags, plus an unrelated tagged box.
use super::tests::{draw, pointer};
use super::toolbox_form_tests::button_by_line;
use super::*;
use egui::accesskit::{self, Role};
use glam::DVec2;
use operandi_core::Mesh;

const SIZE: Vec2 = vec2(1440., 2000.);

fn context() -> Context {
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.enable_accesskit();
    ctx
}

fn fixture() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let target = e.scene.selection.item.unwrap();
    e.scene.new_mesh().unwrap();
    let quads = [[0u32, 3, 2, 1], [1, 2, 5, 4]];
    let mesh = Mesh {
        vertices: [
            (0., 0., 0.),
            (1., 0., 0.),
            (1., 0., 1.),
            (0., 0., 1.),
            (2., 0., 0.),
            (2., 0., 1.),
        ]
        .map(|(x, y, z)| DVec3::new(x, y, z))
        .to_vec(),
        faces: quads
            .iter()
            .enumerate()
            .map(|(i, v)| Face {
                vertices: v.to_vec(),
                uv: Some(
                    (0..4)
                        .map(|c| DVec2::new(i as f64, c as f64 * 0.25))
                        .collect(),
                ),
                material: ["Asphalt", "Verge"][i].into(),
                smooth: i == 0,
                part: Some(["Road", "Neighbor"][i].into()),
                material_label: None,
            })
            .collect(),
        ..Default::default()
    };
    let mut unrelated = Mesh::box_mesh(DVec3::new(5., 0., 0.), DVec3::ONE, 1).unwrap();
    for (i, f) in unrelated.faces.iter_mut().enumerate() {
        f.part = (i % 2 == 0).then(|| "Unrelated".into());
    }
    for item in &mut e.scene.items {
        item.mesh = if item.id == target {
            mesh.clone()
        } else {
            unrelated.clone()
        }
        .into();
    }
    e.scene.select_item(Some(target));
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    e.scene.validate().unwrap();
    e
}

fn app(fixture: &Editor) -> Operandi {
    let mut a = Operandi::empty();
    a.editor = fixture.clone();
    a.saved = a.editor.scene.clone();
    a.category = toolbox::Category::Polygon;
    a.camera.view = View::Perspective;
    a.frame(false);
    a
}

fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) -> Vec<accesskit::Node> {
    draw(a, ctx, SIZE, events)
        .platform_output
        .accesskit_update
        .expect("AccessKit enabled")
        .nodes
        .into_iter()
        .map(|(_, n)| n)
        .collect()
}

fn rect(n: &accesskit::Node) -> Rect {
    let b = n.bounds().expect("widget bounds");
    Rect::from_min_max(
        pos2(b.x0 as f32, b.y0 as f32),
        pos2(b.x1 as f32, b.y1 as f32),
    )
}

fn triple_button(a: &mut Operandi, ctx: &Context) -> Rect {
    let nodes = frame(a, ctx, vec![]);
    let node = nodes
        .iter()
        .find(|n| n.role() == Role::Button && n.label() == Some("Triple"))
        .expect("Triple row");
    rect(node)
}

fn click(a: &mut Operandi, ctx: &Context, at: Pos2) {
    frame(a, ctx, vec![Event::PointerMoved(at)]);
    frame(a, ctx, vec![pointer(at, true, Modifiers::NONE)]);
    frame(a, ctx, vec![pointer(at, false, Modifiers::NONE)]);
}

fn chord(a: &mut Operandi, ctx: &Context, k: Key, modifiers: Modifiers) {
    frame(a, ctx, vec![]);
    let pos = a.viewport_rect.center();
    frame(a, ctx, vec![Event::PointerMoved(pos)]);
    frame(
        a,
        ctx,
        vec![Event::Key {
            key: k,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }],
    );
}

/// The core result for this fixture, computed by the shared operation.
fn expected(source: &Editor) -> Scene {
    let mut e = source.clone();
    e.dispatch(Command::Triple).unwrap();
    e.scene
}

#[test]
fn tri_001_panel_row_command_and_shift_t_give_equal_complete_results() {
    let source = fixture();
    let want = expected(&source);
    assert_ne!(want, source.scene);
    assert_eq!(want.selection.faces.len(), 2);
    let ctx = context();

    let mut panel = app(&source);
    let row = triple_button(&mut panel, &ctx);
    click(&mut panel, &ctx, row.center());
    assert_eq!(panel.editor.scene, want, "{}", panel.status);
    assert_eq!(panel.editor.history.undo.len(), 1);
    assert_eq!(panel.editor.history.undo[0].label, "Triple");
    assert!(panel.status.contains("Triple"), "{}", panel.status);

    let mut command = app(&source);
    command.execute("poly.triple");
    assert_eq!(command.editor.scene, want, "{}", command.status);
    assert_eq!(command.editor.history.undo.len(), 1);
    let mut upper = app(&source);
    upper.execute("POLY.TRIPLE");
    assert_eq!(upper.editor.scene, want);

    assert_eq!(input::default_bindings()["Triple"], "Shift+T");
    let mut shortcut = app(&source);
    chord(&mut shortcut, &ctx, Key::T, Modifiers::SHIFT);
    assert_eq!(shortcut.editor.scene, want, "{}", shortcut.status);
    assert_eq!(shortcut.editor.history.undo.len(), 1);
}

#[test]
fn tri_001_user_remap_moves_the_shortcut_and_its_labels() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    a.bindings.insert("Triple".into(), "Alt+Shift+Y".into());
    chord(&mut a, &ctx, Key::T, Modifiers::SHIFT);
    assert_eq!(a.editor.scene, source.scene, "old default no longer bound");
    assert!(a.editor.history.undo.is_empty());
    assert!(a.shortcut("Triple").contains("Alt+Shift+Y"));
    chord(
        &mut a,
        &ctx,
        Key::Y,
        Modifiers {
            alt: true,
            shift: true,
            ..Modifiers::NONE
        },
    );
    assert_eq!(a.editor.scene, expected(&source), "{}", a.status);
}

#[test]
fn tri_001_tooltip_names_triple_and_the_effective_shortcut() {
    let source = fixture();
    let ctx = context();
    ctx.style_mut(|s| {
        s.interaction.tooltip_delay = 0.;
        s.interaction.show_tooltips_only_when_still = false;
    });
    let mut a = app(&source);
    let row = triple_button(&mut a, &ctx);
    let mut text = Vec::new();
    for _ in 0..3 {
        let nodes = frame(&mut a, &ctx, vec![Event::PointerMoved(row.center())]);
        text = nodes
            .iter()
            .filter_map(|n| n.label().or(n.value()).map(str::to_owned))
            .collect();
    }
    assert!(
        text.iter()
            .any(|t| t.contains("Triple") && !t.contains("poly.triple") && t.contains("[Shift+T]")),
        "{text:?}"
    );
    assert_eq!(a.editor.scene, source.scene, "hover is history-neutral");
}

#[test]
fn tri_001_text_field_focus_suppresses_shift_t() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    frame(&mut a, &ctx, vec![]);
    ctx.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    frame(&mut a, &ctx, vec![]);
    frame(
        &mut a,
        &ctx,
        vec![
            Event::Key {
                key: Key::T,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::SHIFT,
            },
            Event::Text("T".into()),
        ],
    );
    assert_eq!(a.editor.scene, source.scene, "typing does not triple");
    assert!(a.editor.history.undo.is_empty());
    assert!(a.command.ends_with('T'), "{:?}", a.command);
}

#[test]
fn tri_001_help_and_invalid_arguments_refuse_without_changes() {
    let source = fixture();
    let mut a = app(&source);
    let before = a.editor.clone();
    a.execute("help poly.triple");
    assert!(a.status.contains("poly.triple"), "{}", a.status);
    assert!(a.status.contains("no arguments"), "{}", a.status);
    a.execute("help");
    assert!(a.status.contains("poly.triple"), "{}", a.status);
    for bad in [
        "poly.triple x",
        "poly.triple ?",
        "poly.triple a b",
        "poly.triple {x",
    ] {
        a.execute(bad);
        assert!(a.editor == before, "{bad}");
        assert_ne!(a.status, "Ready", "{bad}");
    }
    a.execute("poly.triple extra");
    assert!(a.status.contains("no arguments"), "{}", a.status);
}

#[test]
fn tri_004_routes_preserve_another_tools_field_draft_on_refusal() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    a.execute("spike");
    assert!(a.editor.session.spike.is_some(), "{}", a.status);
    a.fields
        .insert("spi_strength".into(), "not a number".into());
    let editor = a.editor.clone();
    let fields = a.fields.clone();
    a.execute("help poly.triple");
    a.execute("poly.triple x");
    a.execute("poly.triple");
    assert!(a.status.contains("Drop or Cancel"), "{}", a.status);
    chord(&mut a, &ctx, Key::T, Modifiers::SHIFT);
    assert!(a.editor == editor);
    assert_eq!(a.fields, fields);
}

#[test]
fn tri_002_all_triangle_selection_is_neutral_from_the_panel() {
    let source = fixture();
    let mut done = app(&source);
    done.execute("poly.triple");
    let triangles = done.editor.clone();
    let ctx = context();
    let mut a = app(&triangles);
    let row = triple_button(&mut a, &ctx);
    click(&mut a, &ctx, row.center());
    assert_eq!(a.editor.scene, triangles.scene);
    assert!(
        a.editor.history.undo == triangles.history.undo
            && a.editor.history.redo == triangles.history.redo
    );
    assert!(a.status.contains("already triangles"), "{}", a.status);
}

#[test]
fn tri_004_menu_undo_redo_and_native_save_open() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    let row = triple_button(&mut a, &ctx);
    click(&mut a, &ctx, row.center());
    let result = a.editor.scene.clone();
    a.menu_action(&ctx, app_menu::line_action("app.undo"));
    assert_eq!(a.editor.scene, source.scene);
    a.menu_action(&ctx, app_menu::line_action("app.redo"));
    assert_eq!(a.editor.scene, result);
    let path = a.environment.paths().data_root().join("triple.opscene");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(a.save(path.clone()), "{}", a.status);
    let mut reopened = Operandi::empty();
    reopened.open(path);
    assert_eq!(reopened.editor.scene, result);
    let mut direct = app(&source);
    direct.execute("poly.triple");
    assert_eq!(direct.editor.scene, reopened.editor.scene);
}

#[test]
fn polygon_commands_draw_in_specified_group_and_order() {
    let ctx = context();
    let mut a = app(&fixture());
    let nodes = frame(&mut a, &ctx, vec![]);
    let bounds = |label: &str| {
        rect(
            nodes
                .iter()
                .find(|n| n.label() == Some(label))
                .unwrap_or_else(|| panic!("missing drawn {label}")),
        )
    };
    let heading = a.toolbox_rects["Commands"];
    let bridge = button_by_line(&a, "Bridge");
    let triple = bounds("Triple");
    let flip = button_by_line(&a, "Flip");
    let part = bounds("Set Part...");
    assert!(nodes.iter().any(|n| n.role() == Role::Button
        && n.label().is_some_and(|s| s.starts_with("Flip"))
        && rect(n) == flip));
    assert_eq!(triple, button_by_line(&a, "Triple"));
    assert_eq!(part, button_by_line(&a, "Set Part"));
    assert!(bridge.bottom() <= heading.top());
    assert!(heading.bottom() <= triple.top());
    assert!(triple.bottom() <= flip.top());
    assert!(flip.bottom() <= part.top());
}

/// Modelling parity batch 6: Triple Fan, Collapse Selection and Axis
/// Rotate through their command lines (the Polygon tab rows and the Tools
/// menu run the same lines).
#[test]
fn triple_fan_collapse_selection_and_axis_rotate_run_from_their_lines() {
    let mut a = super::tests::box_app();
    a.run(Command::Mode(Mode::Polygon));
    let faces = a.editor.scene.active().unwrap().mesh.faces.len();
    let corner = a.editor.scene.active().unwrap().mesh.faces[0].vertices[0];
    a.editor.scene.selection.faces = vec![0];
    a.editor.scene.selection.vertices = vec![corner];
    a.execute("triple fan");
    assert_eq!(
        a.editor.scene.active().unwrap().mesh.faces.len(),
        faces + 1,
        "{}",
        a.status
    );
    let last = a.editor.scene.active().unwrap().mesh.faces.len() as u32 - 1;
    a.editor.scene.selection.faces = vec![last];
    a.execute("collapse selection");
    assert!(a.editor.scene.selection.faces.is_empty(), "{}", a.status);
    a.execute("axis rotate");
    assert_eq!(
        a.editor.live_tool_label(),
        Some("Axis Rotate"),
        "{}",
        a.status
    );
    assert!(a.axis_rotate_axis().is_some());
}

/// Topology batch (TC-22, TC-45, TC-52): Tack Tool, Topology Pen and
/// Retopo Sketch start from their lines; a finished Retopo stroke becomes
/// one open Curve in the active mesh.
#[test]
fn tack_topology_pen_and_retopo_sketch_start_from_their_lines() {
    let mut a = super::tests::box_app();
    a.run(Command::Mode(Mode::Polygon));
    a.execute("tack tool");
    assert!(
        matches!(a.utility, Some(crate::create_utility::Utility::Tack(_))),
        "{}",
        a.status
    );
    a.execute("tack tool");
    assert!(a.utility.is_none(), "a second click turns it off");
    a.execute("topology pen");
    assert_eq!(
        a.editor.live_tool_label(),
        Some("Topology Pen"),
        "{}",
        a.status
    );
    assert!(a.topology_pen_open());
    a.run(Command::Drop);
    a.execute("retopo sketch");
    assert!(matches!(
        a.utility,
        Some(crate::create_utility::Utility::RetopoSketch)
    ));
    let faces = a.editor.scene.active().unwrap().mesh.faces.len();
    a.retopo_stroke = (0..5)
        .map(|i| DVec3::new(i as f64 * 0.03, 2., 0.))
        .collect();
    a.retopo_sketch_release();
    let m = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(m.faces.len(), faces + 1, "{}", a.status);
    assert!(m.is_polyline(faces), "an open curve");
    assert!(a.retopo_stroke.is_empty());
}

/// TC-53: Retopo Edge Lock and Edge Flow add the selected polygons to
/// their sets, and a polygon can be in both.
#[test]
fn retopo_edge_lock_and_flow_add_the_selection_to_their_sets() {
    let mut a = super::tests::box_app();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0, 1];
    a.execute("retopo edge lock");
    a.execute("retopo edge flow");
    let sets = &a
        .editor
        .scene
        .active()
        .unwrap()
        .mesh
        .attributes
        .polygon_sets;
    for name in ["RetopoEdgeLock", "RetopoEdgeFlow"] {
        let set = sets
            .get(name)
            .unwrap_or_else(|| panic!("{name}: {}", a.status));
        assert_eq!(set.iter().copied().collect::<Vec<_>>(), vec![0, 1]);
    }
    // Nothing selected: a message, no set change.
    a.editor.scene.selection.faces.clear();
    a.execute("retopo edge lock");
    assert!(a.status.contains("select"), "{}", a.status);
}

/// The Edge Flow set was "RetopoFlow" before 2026-11-23: in an older scene
/// Retopo Edge Flow takes that set's members into RetopoEdgeFlow, in one
/// undo step, so the mesh keeps one set.
#[test]
fn retopo_edge_flow_takes_over_the_old_retopoflow_set() {
    let mut a = super::tests::box_app();
    a.run(Command::Mode(Mode::Polygon));
    let sets = |a: &Operandi| {
        a.editor
            .scene
            .active()
            .unwrap()
            .mesh
            .attributes
            .polygon_sets
            .clone()
    };
    a.editor
        .scene
        .active_mut()
        .unwrap()
        .mesh
        .attributes
        .polygon_sets
        .insert("RetopoFlow".into(), [2].into_iter().collect());
    a.editor.scene.selection.faces = vec![0, 1];
    a.execute("retopo edge flow");
    let now = sets(&a);
    assert!(!now.contains_key("RetopoFlow"), "{now:?}");
    assert_eq!(
        now["RetopoEdgeFlow"].iter().copied().collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    a.run(Command::Undo);
    let back = sets(&a);
    assert!(!back.contains_key("RetopoEdgeFlow"), "{back:?}");
    assert_eq!(
        back["RetopoFlow"].iter().copied().collect::<Vec<_>>(),
        vec![2]
    );
}
