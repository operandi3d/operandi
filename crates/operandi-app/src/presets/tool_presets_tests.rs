//! Tool presets stage 1 in the app (specifications/tool-presets-2026-11-15):
//! script ids start the same row as the buttons, Flare's button is built,
//! Flip It's buttons open their axis dialog, and the Weld row's snap
//! radius is Weld's.
use super::tests::{draw, pointer};
use super::toolbox_form_tests::button_by_line;
use super::*;
use egui::accesskit::{self, Role};

const SIZE: Vec2 = vec2(1440., 1600.);

fn context() -> Context {
    let c = Context::default();
    theme::install(&c);
    c.enable_accesskit();
    c
}
fn frame(a: &mut Operandi, c: &Context, events: Vec<Event>) -> Vec<accesskit::Node> {
    draw(a, c, SIZE, events)
        .platform_output
        .accesskit_update
        .unwrap()
        .nodes
        .into_iter()
        .map(|(_, n)| n)
        .collect()
}
fn click(a: &mut Operandi, c: &Context, p: Pos2) {
    frame(a, c, vec![Event::PointerMoved(p)]);
    frame(a, c, vec![pointer(p, true, Modifiers::NONE)]);
    frame(a, c, vec![pointer(p, false, Modifiers::NONE)]);
}
fn choose(a: &mut Operandi, c: &Context, role: Role, name: &str) {
    let nodes = frame(a, c, vec![]);
    let node = nodes
        .iter()
        .find(|n| n.role() == role && n.label().or(n.value()) == Some(name))
        .unwrap_or_else(|| panic!("{name} on screen"));
    let b = node.bounds().unwrap();
    click(
        a,
        c,
        pos2((b.x0 + b.x1) as f32 / 2., (b.y0 + b.y1) as f32 / 2.),
    );
}

/// A 2 m box in Vertex mode.
fn box_in(mode: Mode) -> Operandi {
    let mut a = super::tests::box_app();
    a.run(Command::Mode(mode));
    a.editor.history.undo.clear();
    a
}

/// Spec section 8: `tool.set xfrm.magnet on` from a script gives the same
/// result as the button, and so do the old phrase and the label.
#[test]
fn script_ids_and_phrases_start_the_same_row_as_the_button() {
    let mut button = box_in(Mode::Vertex);
    button.activate_preset("xfrm.magnet");
    for line in ["tool.set xfrm.magnet on", "soft move", "Soft Move"] {
        let mut a = box_in(Mode::Vertex);
        a.execute(line);
        assert_eq!(
            a.editor.session.preset, button.editor.session.preset,
            "{line}: {}",
            a.status
        );
        assert_eq!(
            a.editor.spatial_falloff, button.editor.spatial_falloff,
            "{line}"
        );
        assert_eq!(a.editor.live_tool_label(), Some("Soft Move"), "{line}");
    }
    let mut a = box_in(Mode::Vertex);
    a.execute("tool.set Flare on");
    assert_eq!(a.editor.live_tool_label(), Some("Flare"), "{}", a.status);
    a.execute("tool.set Flare off");
    assert!(a.editor.session.live.is_none() && a.editor.session.preset.is_none());
}

/// Flare is built: the Bulge flyout's Flare entry starts it, and a second
/// pick drops it.
#[test]
fn flare_entry_in_the_bulge_flyout_starts_flare() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_in(Mode::Vertex);
    a.camera.view = View::Front;
    a.category = toolbox::Category::Deform;
    for _ in 0..3 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    // The tab is a form: the flyout is `flyout:Bulge`, its entries their lines.
    let point = a.form_rects["flyout:Bulge"].center();
    let button = |pressed| Event::PointerButton {
        pos: point,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Secondary,
    };
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(point)]);
    draw(&mut a, &ctx, SIZE, vec![button(true), button(false)]);
    draw(&mut a, &ctx, SIZE, vec![]);
    let flare = a.form_rects["Flare"].center();
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![
            pointer(flare, true, Modifiers::NONE),
            pointer(flare, false, Modifiers::NONE),
        ],
    );
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(a.editor.live_tool_label(), Some("Flare"), "{}", a.status);
    assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Push);
}

/// Flip It's toolbox button opens its dialog on X (TP-Q3); Cancel changes
/// nothing; OK runs it once, one undo step; the command line with an axis
/// runs at once.
#[test]
fn flip_it_button_dialog_and_command_line_agree() {
    let mut expected = box_in(Mode::Polygon);
    expected.run(Command::FlipIt {
        preset: "FlipIt".into(),
        axis: 0,
    });
    let want = expected.editor.scene.clone();

    let c = context();
    let mut a = box_in(Mode::Polygon);
    a.category = toolbox::Category::Polygon;
    let source = a.editor.scene.clone();
    frame(&mut a, &c, vec![]);
    let p = button_by_line(&a, "Flip It").center();
    click(&mut a, &c, p);
    assert!(a.flip_it.is_some(), "{}", a.status);
    choose(&mut a, &c, Role::Button, "Cancel");
    assert!(a.flip_it.is_none());
    assert_eq!(a.editor.scene, source);
    assert!(a.editor.history.undo.is_empty());
    click(&mut a, &c, p);
    choose(&mut a, &c, Role::Button, "OK");
    assert_eq!(a.editor.scene, want, "{}", a.status);
    assert_eq!(a.editor.history.undo.len(), 1);
    assert_eq!(a.editor.history.undo[0].label, "Flip It");

    let mut b = box_in(Mode::Polygon);
    b.execute("flip it x");
    assert_eq!(b.editor.scene, want, "{}", b.status);
    let mut origin = box_in(Mode::Polygon);
    origin.execute("flip it origin y");
    assert_eq!(
        origin.editor.history.undo.last().unwrap().label,
        "Flip It Origin"
    );
    let mut bare = box_in(Mode::Polygon);
    bare.execute("flip it origin");
    assert!(
        bare.flip_it
            .as_ref()
            .is_some_and(|d| d.preset == "FlipItOrigin")
    );
}

/// The Weld row's snapping layer is the radius Weld really uses.
#[test]
fn weld_row_matches_welds_snap_radius() {
    let row = operandi_core::tool_preset::row("DragWeld").unwrap();
    let radius = row.layers.iter().find_map(|l| match *l {
        operandi_core::tool_preset::Layer::Snapping { radius_px, .. } => Some(radius_px),
        _ => None,
    });
    assert_eq!(radius, crate::snap_service::Role::Weld.fixed_radius());
    // And the row's app phrase starts Weld.
    let mut a = box_in(Mode::Vertex);
    a.activate_preset("DragWeld");
    assert!(a.vertex_weld, "{}", a.status);
}

/// Every factory row is reachable: editor rows start their tool, app rows
/// name a command the registry knows, every icon slug exists.
#[test]
fn every_factory_row_is_reachable() {
    let registry = command_handlers::registry();
    for row in operandi_core::tool_preset::factory() {
        if !row.icon.is_empty() {
            assert!(
                icons::Icon::from_slug(&row.icon).is_some(),
                "{}: icon",
                row.id
            );
        }
        if let Some(phrase) = &row.actor.app {
            assert!(
                registry
                    .iter()
                    .any(|c| c.entry.name.eq_ignore_ascii_case(phrase)),
                "{}: {phrase}",
                row.id
            );
        }
        if row.tool().is_some() && !row.layers.is_empty() {
            let mut a = box_in(Mode::Vertex);
            a.activate_preset(&row.id);
            assert_eq!(
                a.editor.live_tool_label(),
                Some(row.label.as_str()),
                "{}",
                row.id
            );
        }
    }
}
