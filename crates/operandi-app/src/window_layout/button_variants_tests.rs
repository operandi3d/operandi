//! MB-01..15: modifier-key button variants, driven through real egui frames.
use super::toolbox_form_tests::button_by_line;
use super::*;
use button_variants::{ButtonId, Mods};

fn frame(a: &mut Operandi, ctx: &Context, mods: Modifiers, events: Vec<Event>) -> FullOutput {
    frame_focus(a, ctx, mods, true, events)
}
fn frame_focus(
    a: &mut Operandi,
    ctx: &Context,
    mods: Modifiers,
    focused: bool,
    events: Vec<Event>,
) -> FullOutput {
    ctx.options_mut(|o| o.max_passes = std::num::NonZeroUsize::MIN);
    ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1440., 900.))),
            modifiers: mods,
            focused,
            events,
            ..Default::default()
        },
        |ctx| {
            a.begin_input(ctx);
            a.compose(ctx);
            a.keyboard(ctx);
        },
    )
}
/// Hover first: egui hit-tests a press against the known pointer position.
fn hover(a: &mut Operandi, ctx: &Context, p: Pos2, mods: Modifiers) {
    frame(a, ctx, mods, vec![Event::PointerMoved(p)]);
}
fn pointer(p: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos: p,
        pressed,
        modifiers,
        button: PointerButton::Primary,
    }
}
/// Every drawn text that starts with `needle`.
fn texts(output: &FullOutput, needle: &str) -> usize {
    fn count(s: &egui::Shape, needle: &str) -> usize {
        match s {
            egui::Shape::Text(t) => usize::from(t.galley.text().starts_with(needle)),
            egui::Shape::Vec(shapes) => shapes.iter().map(|s| count(s, needle)).sum(),
            _ => 0,
        }
    }
    output.shapes.iter().map(|s| count(&s.shape, needle)).sum()
}
/// A box with its first polygon selected, in Polygon mode.
fn boxed() -> (Operandi, Context) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.activate(Tool::Box);
    a.run(Command::Preview(Parameters {
        position: DVec3::new(-4., 0., 0.),
        size: DVec3::new(1., 2., 3.),
        ..Default::default()
    }));
    a.run(Command::Drop);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.item = a.editor.scene.active().map(|i| i.id);
    a.editor.scene.selection.faces = vec![0];
    assert_eq!(
        a.editor.scene.active().unwrap().mesh.faces[0]
            .vertices
            .len(),
        4
    );
    a.category = toolbox::Category::Polygon;
    let ctx = Context::default();
    theme::install(&ctx);
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    (a, ctx)
}
fn shift_alt() -> Modifiers {
    Modifiers {
        shift: true,
        alt: true,
        ..Modifiers::NONE
    }
}
fn cmd() -> Modifiers {
    Modifiers {
        mac_cmd: true,
        command: true,
        ..Modifiers::NONE
    }
}

#[test]
fn mb_01_to_05_labels_swap_app_wide_and_come_back() {
    let (mut a, ctx) = boxed();
    let base = frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    assert_eq!(texts(&base, "Convert"), 0);
    let edges = texts(&base, "Edges");
    assert!(edges >= 1);
    // MB-01: the mouse is nowhere near the buttons; all three still swap.
    let alt = frame(
        &mut a,
        &ctx,
        Modifiers::ALT,
        vec![Event::PointerMoved(pos2(700., 450.))],
    );
    assert_eq!(texts(&alt, "Convert"), 3);
    assert_eq!(
        texts(&alt, "Set Part..."),
        2,
        "Set Material row and the Set Part row"
    );
    // MB-02: back at once.
    let up = frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    assert_eq!(texts(&up, "Convert"), 0);
    assert_eq!(texts(&up, "Edges"), edges);
    assert_eq!(texts(&up, "Set Material..."), 1);
    // MB-03 / MB-04: only Edges changes.
    let shift = frame(&mut a, &ctx, Modifiers::SHIFT, vec![]);
    assert_eq!(texts(&shift, "Bounds"), 1);
    assert_eq!(texts(&shift, "Vertices"), texts(&base, "Vertices"));
    let ctrl = frame(&mut a, &ctx, Modifiers::CTRL, vec![]);
    assert_eq!(texts(&ctrl, "Boundary"), 1);
    assert_eq!(texts(&ctrl, "Polygons"), texts(&base, "Polygons"));
    // MB-05: Cmd changes nothing.
    let cmd = frame(&mut a, &ctx, cmd(), vec![]);
    for label in [
        "Vertices",
        "Edges",
        "Polygons",
        "Set Material...",
        "Convert",
        "Bounds",
    ] {
        assert_eq!(texts(&cmd, label), texts(&base, label), "{label}");
    }
}

#[test]
fn mb_02_mb_07_shortcut_hint_belongs_to_the_base() {
    let (mut a, ctx) = boxed();
    a.bindings.insert("Edges".into(), "2".into());
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    let base = frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    assert_eq!(texts(&base, "Edges  [2]"), 1);
    let shift = frame(&mut a, &ctx, Modifiers::SHIFT, vec![]);
    assert_eq!(texts(&shift, "Edges  [2]"), 0);
    assert_eq!(texts(&shift, "Bounds  [2]"), 0);
    // Width is the base's, whatever is shown.
    let width = a.button_variants.rects[&ButtonId::Edges].width();
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    assert_eq!(a.button_variants.rects[&ButtonId::Edges].width(), width);
}

#[test]
fn mb_06_lit_state_follows_the_variant() {
    let (mut a, ctx) = boxed();
    assert!(a.action_active(a.shown(ButtonId::Polygons).action));
    frame(&mut a, &ctx, Modifiers::ALT, vec![]);
    assert!(!a.action_active(a.shown(ButtonId::Polygons).action));
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    assert!(a.action_active(a.shown(ButtonId::Polygons).action));
}

#[test]
fn mb_09_keys_released_in_another_app_leave_no_stuck_variant() {
    let (mut a, ctx) = boxed();
    frame(&mut a, &ctx, Modifiers::ALT, vec![]);
    assert_eq!(a.shown(ButtonId::Vertices).label, "Convert");
    // Focus goes away with Alt down; Alt is let go elsewhere, but the
    // window never hears about it and still reports Alt on return.
    frame_focus(
        &mut a,
        &ctx,
        Modifiers::ALT,
        false,
        vec![Event::WindowFocused(false)],
    );
    assert_eq!(a.shown(ButtonId::Vertices).label, "Vertices");
    frame(
        &mut a,
        &ctx,
        Modifiers::ALT,
        vec![Event::WindowFocused(true)],
    );
    assert_eq!(a.shown(ButtonId::Vertices).label, "Vertices");
    // Once the stale key reads released, a real press works again.
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    frame(&mut a, &ctx, Modifiers::ALT, vec![]);
    assert_eq!(a.shown(ButtonId::Vertices).label, "Convert");
}

#[test]
fn mb_10_mb_12_the_command_is_fixed_at_mouse_press() {
    // MB-10: Alt at press, released before mouse-up: Convert runs.
    let (mut a, ctx) = boxed();
    let p = a.button_variants.rects[&ButtonId::Vertices].center();
    hover(&mut a, &ctx, p, Modifiers::NONE);
    frame(
        &mut a,
        &ctx,
        Modifiers::ALT,
        vec![pointer(p, true, Modifiers::ALT)],
    );
    frame(
        &mut a,
        &ctx,
        Modifiers::NONE,
        vec![pointer(p, false, Modifiers::NONE)],
    );
    assert_eq!(a.editor.scene.selection.mode, Mode::Vertex, "{}", a.status);
    assert_eq!(a.editor.scene.selection.vertices.len(), 4, "converted");

    // MB-12: no key at press, Alt before mouse-up: the base runs.
    let (mut a, ctx) = boxed();
    let p = a.button_variants.rects[&ButtonId::Vertices].center();
    hover(&mut a, &ctx, p, Modifiers::NONE);
    frame(
        &mut a,
        &ctx,
        Modifiers::NONE,
        vec![pointer(p, true, Modifiers::NONE)],
    );
    frame(
        &mut a,
        &ctx,
        Modifiers::ALT,
        vec![pointer(p, false, Modifiers::ALT)],
    );
    assert_eq!(a.editor.scene.selection.mode, Mode::Vertex);
    assert!(
        a.editor.scene.selection.vertices.is_empty(),
        "plain mode switch"
    );

    // Press and release in one frame use the press event's own keys.
    let (mut a, ctx) = boxed();
    let p = a.button_variants.rects[&ButtonId::Vertices].center();
    hover(&mut a, &ctx, p, Modifiers::NONE);
    frame(
        &mut a,
        &ctx,
        Modifiers::NONE,
        vec![
            pointer(p, true, Modifiers::ALT),
            pointer(p, false, Modifiers::NONE),
        ],
    );
    assert_eq!(a.editor.scene.selection.vertices.len(), 4);
}

#[test]
fn mb_13_unmatched_pair_shows_and_runs_the_base() {
    let (mut a, ctx) = boxed();
    let out = frame(&mut a, &ctx, shift_alt(), vec![]);
    assert_eq!(texts(&out, "Convert") + texts(&out, "Bounds"), 0);
    let p = a.button_variants.rects[&ButtonId::Edges].center();
    hover(&mut a, &ctx, p, Modifiers::NONE);
    frame(
        &mut a,
        &ctx,
        shift_alt(),
        vec![
            pointer(p, true, shift_alt()),
            pointer(p, false, shift_alt()),
        ],
    );
    assert_eq!(a.editor.scene.selection.mode, Mode::Edge);
    assert!(
        a.editor.scene.selection.edges.is_empty(),
        "no boundary or convert ran"
    );
}

#[test]
fn edges_variants_run_boundary_and_cmd_click_runs_the_base() {
    for mods in [Modifiers::SHIFT, Modifiers::CTRL] {
        let (mut a, ctx) = boxed();
        let p = a.button_variants.rects[&ButtonId::Edges].center();
        hover(&mut a, &ctx, p, Modifiers::NONE);
        frame(
            &mut a,
            &ctx,
            mods,
            vec![pointer(p, true, mods), pointer(p, false, mods)],
        );
        assert_eq!(a.editor.scene.selection.mode, Mode::Edge);
        assert_eq!(a.editor.scene.selection.edges.len(), 4, "{mods:?}");
    }
    let (mut a, ctx) = boxed();
    let p = a.button_variants.rects[&ButtonId::Polygons].center();
    hover(&mut a, &ctx, p, Modifiers::NONE);
    frame(
        &mut a,
        &ctx,
        cmd(),
        vec![pointer(p, true, cmd()), pointer(p, false, cmd())],
    );
    assert_eq!(
        a.editor.scene.selection.mode,
        Mode::Item,
        "active mode clicked again"
    );
}

#[test]
fn mb_14_tooltip_follows_the_keys_and_lists_the_variants() {
    let (mut a, ctx) = boxed();
    let plain = a.variant_tooltip(ButtonId::Edges);
    assert!(plain.starts_with("Edges"), "{plain}");
    assert!(plain.contains("Shift: Bounds · Ctrl: Boundary"), "{plain}");
    frame(&mut a, &ctx, Modifiers::SHIFT, vec![]);
    let shift = a.variant_tooltip(ButtonId::Edges);
    assert!(shift.starts_with("Bounds"), "{shift}");
    assert!(shift.contains("border edges"), "{shift}");
}

#[test]
fn mb_15_buttons_swap_while_a_text_field_has_focus() {
    let (mut a, ctx) = boxed();
    ctx.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    assert!(a.text_editor_focused(&ctx));
    let out = frame(&mut a, &ctx, Modifiers::ALT, vec![]);
    assert_eq!(texts(&out, "Convert"), 3);
}

#[test]
fn alt_click_set_material_opens_set_part() {
    let (mut a, ctx) = boxed();
    // Set Material... sits at the end of the Commands group, below the fold
    // at this window size: scroll the tool list (above the Tool Properties
    // divider) down first.
    for _ in 0..40 {
        let r = button_by_line(&a, "Set Material");
        if r.bottom() <= a.tool_divider_rect.top() {
            break;
        }
        hover(&mut a, &ctx, pos2(r.center().x, 450.), Modifiers::NONE);
        frame(
            &mut a,
            &ctx,
            Modifiers::NONE,
            vec![Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0., -60.),
                modifiers: Modifiers::NONE,
            }],
        );
    }
    for _ in 0..10 {
        frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    }
    let p = button_by_line(&a, "Set Material").center();
    assert!(
        p.y < a.tool_divider_rect.top(),
        "Set Material still below the fold: {p:?}"
    );
    hover(&mut a, &ctx, p, Modifiers::NONE);
    frame(
        &mut a,
        &ctx,
        Modifiers::ALT,
        vec![
            pointer(p, true, Modifiers::ALT),
            pointer(p, false, Modifiers::ALT),
        ],
    );
    assert!(a.set_part.is_some());
    assert!(a.set_material.is_none());
    // Mods is the only key type the table knows about.
    assert_eq!(Mods::from_egui(Modifiers::ALT), Mods::ALT);
}

fn click(a: &mut Operandi, ctx: &Context, button: ButtonId, mods: Modifiers) {
    let p = a.button_variants.rects[&button].center();
    hover(a, ctx, p, Modifiers::NONE);
    frame(
        a,
        ctx,
        mods,
        vec![pointer(p, true, mods), pointer(p, false, mods)],
    );
}

/// `boxed()` with two polygons that share an edge selected instead of one.
fn two_neighbours() -> (Operandi, Context, (u32, u32)) {
    let (mut a, ctx) = boxed();
    let mesh = &a.editor.scene.active().unwrap().mesh;
    let edges = |f: usize| {
        let v = &mesh.faces[f].vertices;
        (0..v.len())
            .map(|i| {
                let (x, y) = (v[i], v[(i + 1) % v.len()]);
                (x.min(y), x.max(y))
            })
            .collect::<Vec<_>>()
    };
    let first = edges(0);
    let (other, shared) = (1..mesh.faces.len())
        .find_map(|f| {
            edges(f)
                .into_iter()
                .find(|e| first.contains(e))
                .map(|e| (f, e))
        })
        .expect("a neighbour of face 0");
    a.editor.scene.selection.faces = vec![0, other as u32];
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    (a, ctx, shared)
}

#[test]
fn mb_q9_shift_bounds_replaces_with_the_outline_of_two_neighbours() {
    let (mut a, ctx, shared) = two_neighbours();
    // An edge stored from earlier is replaced, not kept.
    a.editor.scene.selection.edges = vec![shared];
    click(&mut a, &ctx, ButtonId::Edges, Modifiers::SHIFT);
    let selection = &a.editor.scene.selection;
    assert_eq!(selection.mode, Mode::Edge, "{}", a.status);
    assert_eq!(selection.edges.len(), 6, "the outline only");
    assert!(!selection.edges.contains(&shared), "not the shared edge");
    // One undo step back to Polygon mode with the polygons selected.
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.selection.mode, Mode::Polygon);
    assert_eq!(a.editor.scene.selection.faces.len(), 2);
}

#[test]
fn mb_q9_ctrl_boundary_adds_to_the_edges_already_selected() {
    let (mut a, ctx, shared) = two_neighbours();
    // The shared edge is not on the outline, so it can only survive by
    // being kept.
    a.editor.scene.selection.edges = vec![shared];
    click(&mut a, &ctx, ButtonId::Edges, Modifiers::CTRL);
    let selection = &a.editor.scene.selection;
    assert_eq!(selection.mode, Mode::Edge, "{}", a.status);
    assert_eq!(selection.edges.len(), 7, "outline plus the kept edge");
    assert_eq!(selection.edges[0], shared, "kept edge stays first");
    // Running it again adds nothing twice.
    a.run(Command::Mode(Mode::Polygon));
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    click(&mut a, &ctx, ButtonId::Edges, Modifiers::CTRL);
    assert_eq!(a.editor.scene.selection.edges.len(), 7);
}

/// The Create tab's seven primitives (layouts stage 3 batch 7: their
/// primitive-quick variants live in toolbox.create.json).
const PRIMITIVES: [&str; 7] = [
    "Cube",
    "Sphere",
    "Cylinder",
    "Ellipsoid",
    "Cone",
    "Torus",
    "Capsule",
];

#[test]
fn mb_q8_primitives_carry_item_and_unit_variants() {
    let (mut a, ctx) = create_panel();
    for name in PRIMITIVES {
        let (label, tip) = a.form_button_for_test(toolbox_form::CREATE_FORM, name);
        assert_eq!(label, name);
        assert!(
            tip.contains(&format!("Shift: {name} Item · Ctrl: Unit {name}")),
            "{tip}"
        );
    }
    // MB-13: Ctrl+Shift has no variant, so the button shows its own line.
    frame(&mut a, &ctx, Modifiers::SHIFT | Modifiers::CTRL, vec![]);
    let (label, _) = a.form_button_for_test(toolbox_form::CREATE_FORM, "Cone");
    assert_eq!(label, "Cone");
    frame(&mut a, &ctx, Modifiers::CTRL, vec![]);
    let (label, _) = a.form_button_for_test(toolbox_form::CREATE_FORM, "Cone");
    assert_eq!(label, "Unit Cone");
}

fn create_panel() -> (Operandi, Context) {
    let (mut a, ctx) = boxed();
    a.run(Command::Mode(Mode::Item));
    a.category = toolbox::Category::Create;
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    (a, ctx)
}

/// Clicks the Create tab's button running `line` with `mods` held.
fn click_cell(a: &mut Operandi, ctx: &Context, line: &str, mods: Modifiers) {
    let p = button_by_line(a, line).center();
    hover(a, ctx, p, Modifiers::NONE);
    frame(
        a,
        ctx,
        mods,
        vec![pointer(p, true, mods), pointer(p, false, mods)],
    );
}

#[test]
fn mb_q8_shift_adds_an_item_and_ctrl_a_unit_in_this_mesh() {
    for name in PRIMITIVES {
        // Shift: a new mesh item.
        let (mut a, ctx) = create_panel();
        let items = a.editor.scene.items.len();
        let undo = a.editor.history.undo.len();
        click_cell(&mut a, &ctx, name, Modifiers::SHIFT);
        assert_eq!(
            a.editor.scene.items.len(),
            items + 1,
            "{name}: {}",
            a.status
        );
        assert_eq!(a.editor.history.undo.len(), undo + 1, "{name}");
        assert!(a.editor.session.live.is_none(), "{name}: no drag");
        // Ctrl: into the current mesh.
        let (mut a, ctx) = create_panel();
        let faces = a.editor.scene.active().unwrap().mesh.faces.len();
        click_cell(&mut a, &ctx, name, Modifiers::CTRL);
        assert_eq!(a.editor.scene.items.len(), items, "{name}: {}", a.status);
        assert!(
            a.editor.scene.active().unwrap().mesh.faces.len() > faces,
            "{name}"
        );
        assert!(a.editor.session.live.is_none(), "{name}: no drag");
    }
}

#[test]
fn mb_q8_primitive_keys_are_read_at_press_and_labels_swap_while_typing() {
    let (mut a, ctx) = create_panel();
    let p = button_by_line(&a, "Cone").center();
    let items = a.editor.scene.items.len();
    hover(&mut a, &ctx, p, Modifiers::NONE);
    frame(
        &mut a,
        &ctx,
        Modifiers::SHIFT,
        vec![pointer(p, true, Modifiers::SHIFT)],
    );
    frame(
        &mut a,
        &ctx,
        Modifiers::NONE,
        vec![pointer(p, false, Modifiers::NONE)],
    );
    assert_eq!(a.editor.scene.items.len(), items + 1, "{}", a.status);
    assert!(
        a.editor.session.cone.is_none(),
        "Shift at press: no Cone session"
    );

    ctx.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    assert!(a.text_editor_focused(&ctx));
    // Every primitive cell gains an "Item" tag (other panels' "Item..."
    // texts are there either way).
    let plain = texts(&frame(&mut a, &ctx, Modifiers::NONE, vec![]), "Item");
    let out = frame(&mut a, &ctx, Modifiers::SHIFT, vec![]);
    // Eight since Text gained its Plane Item variant (missing-tools-g2).
    assert_eq!(texts(&out, "Item") - plain, 8);
    let (label, tip) = a.form_button_for_test(toolbox_form::CREATE_FORM, "Cone");
    assert_eq!(label, "Cone Item");
    assert!(tip.starts_with("Cone Item"), "{tip}");
    assert!(tip.contains("Shift: Cone Item · Ctrl: Unit Cone"), "{tip}");
}
