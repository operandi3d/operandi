//! Pie menus, stage 1 (pie_menu.rs): opening on a key, flicking, tapping,
//! clicking, Esc, toggles, choices, disabled and empty slots, and the
//! input map's conflict warning.
use super::*;
use crate::pie_menu::{self, Mark, Outcome, Pie};
use crate::tests::{box_app, draw, pointer};

const SIZE: Vec2 = vec2(1200., 800.);

/// A shipped pie, read from its form file.
fn built_in(id: &str) -> Pie {
    Operandi::empty()
        .pie_named(id)
        .unwrap_or_else(|| panic!("no pie {id}"))
}
const AT: Pos2 = pos2(600., 400.);

fn key_event(key: Key, pressed: bool, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    }
}

fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
    draw(a, ctx, SIZE, events);
}

/// Hold Ctrl+Space (the view pie) with the pointer at `AT`.
fn press_view_pie(a: &mut Operandi, ctx: &Context) {
    frame(a, ctx, vec![Event::PointerMoved(AT)]);
    frame(a, ctx, vec![key_event(Key::Space, true, Modifiers::CTRL)]);
    assert!(
        a.pie.is_some(),
        "Ctrl+Space opens the view pie: {}",
        a.status
    );
    assert_eq!(a.pie.as_ref().unwrap().pie.name, pie_menu::VIEW);
}

fn release_space(a: &mut Operandi, ctx: &Context) {
    frame(a, ctx, vec![key_event(Key::Space, false, Modifiers::CTRL)]);
}

/// Hold a pie's key, flick by `offset`, release.
fn flick(a: &mut Operandi, ctx: &Context, offset: Vec2) {
    press_view_pie(a, ctx);
    frame(a, ctx, vec![Event::PointerMoved(AT + offset)]);
    release_space(a, ctx);
}

fn perspective_app() -> Operandi {
    let mut a = box_app();
    a.camera.view = View::Perspective;
    a
}

/// T1: flick up and release picks Top; flick right picks Right.
#[test]
fn flicks_pick_the_slot_they_point_at() {
    let mut a = perspective_app();
    let ctx = Context::default();
    let undo = a.editor.history.undo.len();
    flick(&mut a, &ctx, vec2(0., -80.));
    assert!(a.pie.is_none());
    assert_eq!(a.camera.view, View::Top);

    flick(&mut a, &ctx, vec2(80., 0.));
    assert!(a.pie.is_none());
    assert_eq!(a.camera.view, View::Right);
    // A short flick just past the dead zone counts too.
    flick(&mut a, &ctx, vec2(0., -(pie_menu::DEAD_ZONE + 2.)));
    assert_eq!(a.camera.view, View::Top);
    // Views are never undo steps; the pie isn't one either.
    assert_eq!(a.editor.history.undo.len(), undo);
}

/// T2: a tap (release in the dead zone without leaving it) keeps the pie
/// open; clicking Front runs it and closes.
#[test]
fn tap_then_click_a_slot() {
    let mut a = perspective_app();
    let ctx = Context::default();
    press_view_pie(&mut a, &ctx);
    release_space(&mut a, &ctx);
    assert!(a.pie.is_some(), "a tap keeps the pie open");
    // Moving around while tapped only highlights.
    let front = a.pie.as_ref().unwrap().rect(3).center();
    frame(&mut a, &ctx, vec![Event::PointerMoved(front)]);
    assert_eq!(a.pie.as_ref().unwrap().highlighted(), Some(3));
    assert_eq!(a.camera.view, View::Perspective);
    frame(&mut a, &ctx, vec![pointer(front, true, Modifiers::NONE)]);
    assert!(a.pie.is_some(), "the press alone runs nothing");
    frame(&mut a, &ctx, vec![pointer(front, false, Modifiers::NONE)]);
    assert!(a.pie.is_none());
    assert_eq!(a.camera.view, View::Front);
}

/// A tapped pie closes, with no action, on a click away from every slot.
#[test]
fn tapped_pie_closes_on_a_click_elsewhere() {
    let mut a = perspective_app();
    let ctx = Context::default();
    press_view_pie(&mut a, &ctx);
    release_space(&mut a, &ctx);
    let far = pos2(20., 20.);
    frame(&mut a, &ctx, vec![pointer(far, true, Modifiers::NONE)]);
    frame(&mut a, &ctx, vec![pointer(far, false, Modifiers::NONE)]);
    assert!(a.pie.is_none());
    assert_eq!(a.camera.view, View::Perspective);
}

/// Leaving the dead zone and coming back, then releasing, closes with
/// nothing (not a tap).
#[test]
fn release_back_in_the_dead_zone_closes() {
    let mut a = perspective_app();
    let ctx = Context::default();
    press_view_pie(&mut a, &ctx);
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT + vec2(0., -80.))]);
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT)]);
    release_space(&mut a, &ctx);
    assert!(a.pie.is_none());
    assert_eq!(a.camera.view, View::Perspective);
}

/// T3: Esc closes; nothing changes and the undo stack is untouched.
#[test]
fn escape_closes_without_a_change() {
    let mut a = perspective_app();
    let ctx = Context::default();
    let scene = a.editor.scene.clone();
    let undo = a.editor.history.undo.len();
    press_view_pie(&mut a, &ctx);
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT + vec2(0., -80.))]);
    frame(
        &mut a,
        &ctx,
        vec![key_event(Key::Escape, true, Modifiers::NONE)],
    );
    assert!(a.pie.is_none());
    // The key's release afterwards does nothing either.
    release_space(&mut a, &ctx);
    assert_eq!(a.camera.view, View::Perspective);
    assert_eq!(a.editor.scene, scene);
    assert_eq!(a.editor.history.undo.len(), undo);
}

/// T4: the toggles pie shows Vertices on when they are shown; picking it
/// hides them. View display is never an undo step.
#[test]
fn toggle_entries_show_and_flip_their_state() {
    let mut a = perspective_app();
    let ctx = Context::default();
    a.display.settings.drawing.show_verts = true;
    let pie = built_in(pie_menu::TOGGLES);
    let vertices = pie.slot(3).unwrap();
    assert_eq!(vertices.label, "Vertices");
    assert_eq!(a.pie_status(&vertices.command).mark, Mark::Toggle(true));
    let undo = a.editor.history.undo.len();
    a.bindings
        .insert(pie_menu::TOGGLES_LINE.into(), "Ctrl+Alt+T".into());
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT)]);
    frame(
        &mut a,
        &ctx,
        vec![key_event(Key::T, true, Modifiers::CTRL | Modifiers::ALT)],
    );
    assert!(a.pie.is_some(), "{}", a.status);
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT + vec2(60., 60.))]);
    frame(
        &mut a,
        &ctx,
        vec![key_event(Key::T, false, Modifiers::CTRL | Modifiers::ALT)],
    );
    assert!(a.pie.is_none());
    assert!(!a.display.settings.drawing.show_verts);
    assert_eq!(a.pie_status(&vertices.command).mark, Mark::Toggle(false));
    assert_eq!(a.editor.history.undo.len(), undo);
    // Every built-in toggle flips and flips back.
    for i in 0..pie_menu::SLOTS {
        let Some(entry) = pie.slot(i) else { continue };
        let Mark::Toggle(before) = a.pie_status(&entry.command).mark else {
            panic!("{} is a toggle", entry.label);
        };
        a.execute(&entry.command);
        assert_eq!(a.pie_status(&entry.command).mark, Mark::Toggle(!before));
        a.execute(&entry.command);
        assert_eq!(a.pie_status(&entry.command).mark, Mark::Toggle(before));
    }
}

/// Choice entries show the active one: view, shading, selection mode.
#[test]
fn choice_entries_mark_the_active_one() {
    let mut a = perspective_app();
    let marks = |a: &Operandi, name| {
        let pie = built_in(name);
        (0..pie_menu::SLOTS)
            .map(|i| pie.slot(i).map(|e| a.pie_status(&e.command).mark))
            .collect::<Vec<_>>()
    };
    let on = |marks: Vec<Option<Mark>>| {
        marks
            .iter()
            .enumerate()
            .filter(|(_, m)| **m == Some(Mark::Choice(true)))
            .map(|(i, _)| i)
            .collect::<Vec<_>>()
    };
    assert_eq!(on(marks(&a, pie_menu::VIEW)), vec![1], "Perspective");
    a.display.set_style(view_display::Style::Gooch, false);
    assert_eq!(on(marks(&a, pie_menu::SHADING)), vec![5], "Gooch");
    a.run(Command::Mode(Mode::Edge));
    assert_eq!(on(marks(&a, pie_menu::SELECTION)), vec![0], "Edges");
    // Running a shading entry sets the active meshes' style.
    a.execute("Shading Wireframe");
    assert_eq!(
        a.display.settings.active.style,
        view_display::Style::Wireframe
    );
    a.execute("Action Center Origin");
    assert_eq!(on(marks(&a, pie_menu::ACTION_CENTERS)), vec![5], "Origin");
}

/// T5: only the first eight entries are used.
#[test]
fn only_eight_entries_show() {
    let entries: Vec<_> = (0..10)
        .map(|i| {
            Some((
                ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"][i],
                "Top View",
            ))
        })
        .collect();
    let pie = Pie::new("ten", "Ten", &entries);
    let shown: Vec<_> = (0..pie_menu::SLOTS)
        .filter_map(|i| pie.slot(i).map(|e| e.label.clone()))
        .collect();
    assert_eq!(shown, ["A", "B", "C", "D", "E", "F", "G", "H"]);
    assert!(pie.slot(8).is_none() && pie.slot(9).is_none());
}

/// T6: empty entries keep their slot: A at N, B at E, C at S, D at W.
#[test]
fn empty_entries_keep_their_slot() {
    let e = |l| Some((l, "Top View"));
    let pie = Pie::new(
        "gaps",
        "Gaps",
        &[e("A"), None, e("B"), None, e("C"), None, e("D"), None],
    );
    let at = |offset: Vec2| {
        pie_menu::slot_at(offset)
            .and_then(|i| pie.slot(i))
            .map(|e| e.label.as_str())
    };
    assert_eq!(at(vec2(0., -60.)), Some("A"));
    assert_eq!(at(vec2(60., 0.)), Some("B"));
    assert_eq!(at(vec2(0., 60.)), Some("C"));
    assert_eq!(at(vec2(-60., 0.)), Some("D"));
    assert_eq!(at(vec2(60., -60.)), None, "NE is empty");
    // Trailing missing entries are empty slots too.
    let short = Pie::new("short", "Short", &[e("A")]);
    assert!((1..pie_menu::SLOTS).all(|i| short.slot(i).is_none()));
    // An empty slot highlights nothing, and releasing over it closes.
    let mut open = pie_menu::Open::new(
        pie,
        AT,
        Rect::from_min_size(Pos2::ZERO, SIZE),
        Some((Key::Space, None)),
    );
    open.hover(AT + vec2(60., -60.));
    assert_eq!(open.highlighted(), None);
    assert_eq!(open.release(|_| true), Outcome::Close);
}

/// Wedges are 45° around each compass direction, outside the 20 px dead
/// zone, clockwise from the top.
#[test]
fn wedges_and_dead_zone() {
    assert_eq!(pie_menu::slot_at(vec2(0., -19.)), None);
    assert_eq!(pie_menu::slot_at(vec2(0., -21.)), Some(0));
    for i in 0..pie_menu::SLOTS {
        let d = pie_menu::direction(i);
        assert_eq!(pie_menu::slot_at(d * 50.), Some(i));
        // ±22° stays in the wedge.
        let turn = |deg: f32| {
            let (s, c) = deg.to_radians().sin_cos();
            vec2(d.x * c - d.y * s, d.x * s + d.y * c) * 50.
        };
        assert_eq!(pie_menu::slot_at(turn(22.)), Some(i));
        assert_eq!(pie_menu::slot_at(turn(-22.)), Some(i));
        assert_eq!(pie_menu::slot_at(turn(23.)), Some((i + 1) % 8));
    }
}

/// Near a window edge the pie moves in so it fits; the pointer isn't
/// moved (a release then picks by direction from the new centre).
#[test]
fn pie_stays_on_screen_near_edges() {
    let screen = Rect::from_min_size(Pos2::ZERO, SIZE);
    for p in [pos2(2., 2.), pos2(1198., 798.), pos2(600., 3.)] {
        let center = pie_menu::place(p, screen);
        for i in 0..pie_menu::SLOTS {
            assert!(
                screen.contains_rect(pie_menu::label_rect(center, i, 120.)),
                "{p:?} slot {i}"
            );
        }
    }
    assert_eq!(pie_menu::place(AT, screen), AT);
}

/// T7: a disabled entry is greyed, keeps its slot and can't be picked:
/// releasing over it closes with nothing.
#[test]
fn disabled_entries_cannot_be_picked() {
    let mut a = box_app();
    let ctx = Context::default();
    a.run(Command::Mode(Mode::Edge));
    let pie = built_in(pie_menu::MODELING);
    let bevel = pie.slot(1).unwrap();
    assert_eq!(bevel.label, "Bevel");
    assert!(
        !a.pie_status(&bevel.command).enabled,
        "Bevel is Polygon-only"
    );
    assert!(
        a.pie_status(&pie.slot(2).unwrap().command).enabled,
        "Edge Extend"
    );
    let scene = a.editor.scene.clone();
    let undo = a.editor.history.undo.len();
    a.bindings
        .insert(pie_menu::MODELING_LINE.into(), "Ctrl+Alt+M".into());
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT)]);
    let mods = Modifiers::CTRL | Modifiers::ALT;
    frame(&mut a, &ctx, vec![key_event(Key::M, true, mods)]);
    assert!(a.pie.is_some(), "{}", a.status);
    frame(
        &mut a,
        &ctx,
        vec![Event::PointerMoved(AT + vec2(60., -60.))],
    );
    // Still highlighted (greyed), so the user sees why nothing runs.
    assert_eq!(a.pie.as_ref().unwrap().highlighted(), Some(1));
    frame(&mut a, &ctx, vec![key_event(Key::M, false, mods)]);
    assert!(a.pie.is_none());
    assert!(a.editor.session.live.is_none());
    assert_eq!(a.editor.scene, scene);
    assert_eq!(a.editor.history.undo.len(), undo);
}

/// T9: binding a pie to a key another action uses shows the input map's
/// conflict warning, and the live key refuses to guess.
#[test]
fn pie_on_a_used_key_conflicts() {
    let mut a = perspective_app();
    let ctx = Context::default();
    let mut bindings = input::default_bindings();
    assert!(
        input_map_editor::problems(&bindings).is_empty(),
        "factory pie keys are free: {:?}",
        input_map_editor::problems(&bindings)
    );
    bindings.insert(pie_menu::VIEW_LINE.into(), "W".into());
    assert_eq!(
        input_map_editor::clashes(&bindings, pie_menu::VIEW_LINE, "W"),
        vec!["Move".to_owned()]
    );
    assert!(input_map_editor::problems(&bindings).contains_key(pie_menu::VIEW_LINE));
    assert!(!input::bindings_valid(&bindings));
    // Pies show in the editor's list as "Any mode".
    assert_eq!(input_map_editor::context(pie_menu::VIEW_LINE), "Any mode");
    a.bindings = bindings;
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT)]);
    frame(&mut a, &ctx, vec![key_event(Key::W, true, Modifiers::NONE)]);
    assert!(a.pie.is_none());
    assert!(a.editor.session.live.is_none(), "Move doesn't run either");
    assert!(a.status.contains("conflict"), "{}", a.status);
}

/// Ctrl+Space binds and records; plain Space stays reserved.
#[test]
fn ctrl_space_is_a_chord_plain_space_is_not() {
    assert!(input::matches_chord(
        "Ctrl+Space",
        Key::Space,
        Modifiers::CTRL
    ));
    assert!(input::chord("Space").is_none());
    assert_eq!(
        input_map_editor::capture(&[key_event(Key::Space, true, Modifiers::CTRL)], &[]),
        Some(input_map_editor::Captured::Chord("Ctrl+Space".into()))
    );
    assert_eq!(
        input_map_editor::capture(&[key_event(Key::Space, true, Modifiers::NONE)], &[]),
        Some(input_map_editor::Captured::Reserved)
    );
    // The factory keys.
    let d = input::default_bindings();
    assert_eq!(d[pie_menu::VIEW_LINE], "Ctrl+Space");
    assert_eq!(d[pie_menu::SELECTION_LINE], "Alt+Q");
    assert_eq!(d[pie_menu::SHADING_LINE], "Ctrl+2");
    assert_eq!(d[pie_menu::TOGGLES_LINE], "Ctrl+1");
    for name in [
        pie_menu::FALLOFFS_LINE,
        pie_menu::ACTION_CENTERS_LINE,
        pie_menu::MODELING_LINE,
    ] {
        assert_eq!(d[name], "", "{name}");
    }
}

/// A saved map that already gives a pie's key to another action keeps it;
/// the pie starts unassigned there.
#[test]
fn saved_maps_keep_their_keys() {
    let dir = std::env::temp_dir().join(format!("operandi-pie-map-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("input_map.json");
    std::fs::write(&path, r#"{"Edges":"Ctrl+2"}"#).unwrap();
    let loaded = input::load_bindings(&path);
    assert_eq!(loaded["Edges"], "Ctrl+2");
    assert_eq!(loaded[pie_menu::SHADING_LINE], "");
    assert_eq!(loaded[pie_menu::TOGGLES_LINE], "Ctrl+1");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Opening a pie closes other popups, and the pie opens over them.
#[test]
fn opening_closes_other_popups() {
    let mut a = perspective_app();
    let ctx = Context::default();
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT)]);
    egui::Popup::open_id(&ctx, Id::new("some-menu"));
    assert!(egui::Popup::is_any_open(&ctx));
    frame(
        &mut a,
        &ctx,
        vec![key_event(Key::Space, true, Modifiers::CTRL)],
    );
    assert!(a.pie.is_some(), "{}", a.status);
    assert!(!egui::Popup::is_any_open(&ctx));
}

/// Every built-in entry names a command Operandi runs (no "Unknown
/// command"), and Selection pie entries run like their shortcuts.
#[test]
fn built_in_entries_are_real_commands() {
    for name in pie_menu::ALL {
        let pie = built_in(name);
        assert_eq!(pie.name, name);
        assert!(
            input::ACTIONS
                .iter()
                .any(|(a, _)| *a == pie_menu::line(name)),
            "{name}"
        );
        for i in 0..pie_menu::SLOTS {
            let Some(entry) = pie.slot(i) else { continue };
            let mut a = box_app();
            a.status.clear();
            if a.pie_status(&entry.command).enabled {
                a.execute(&entry.command);
            }
            assert!(
                !a.status.starts_with("Unknown command"),
                "{name}: {} -> {}",
                entry.label,
                a.status
            );
        }
    }
    let mut a = box_app();
    a.execute("Polygons");
    assert_eq!(a.editor.scene.selection.mode, Mode::Polygon);
    a.execute("Move");
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Move)
    );
}

/// A fast hold, point and release batched into the frame that opens the
/// pie still runs the slot it pointed at (no click, no stuck pie).
#[test]
fn hold_point_release_in_one_frame_runs_the_slot() {
    let mut a = perspective_app();
    let ctx = Context::default();
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT)]);
    frame(
        &mut a,
        &ctx,
        vec![
            key_event(Key::Space, true, Modifiers::CTRL),
            Event::PointerMoved(AT + vec2(0., -80.)),
            key_event(Key::Space, false, Modifiers::CTRL),
        ],
    );
    assert!(a.pie.is_none(), "the release closed the pie");
    assert_eq!(a.camera.view, View::Top);
}

/// Every built-in pie runs a slot on release, through the same gesture:
/// the Selection pie's Move (east) turns the Move tool on, as W does.
#[test]
fn every_pie_runs_its_slot_on_release() {
    let mut a = perspective_app();
    let ctx = Context::default();
    frame(&mut a, &ctx, vec![Event::PointerMoved(AT)]);
    frame(&mut a, &ctx, vec![key_event(Key::Q, true, Modifiers::ALT)]);
    assert_eq!(
        a.pie.as_ref().map(|p| p.pie.name.as_str()),
        Some(pie_menu::SELECTION)
    );
    let mv = a.pie.as_ref().unwrap().rect(3).center();
    frame(&mut a, &ctx, vec![Event::PointerMoved(mv)]);
    assert_eq!(a.pie.as_ref().unwrap().highlighted(), Some(3));
    frame(&mut a, &ctx, vec![key_event(Key::Q, false, Modifiers::ALT)]);
    assert!(a.pie.is_none());
    assert!(
        a.status.contains("Move") || a.editor_tool_active(),
        "{}",
        a.status
    );
}

/// The pointer on a button highlights that button even where the corner of
/// a diagonal button pokes into a side slot's wedge.
#[test]
fn a_button_under_the_pointer_wins_over_its_wedge() {
    let mut a = perspective_app();
    let ctx = Context::default();
    press_view_pie(&mut a, &ctx);
    let open = a.pie.as_ref().unwrap();
    let ne = open.rect(1);
    let corner = pos2(ne.right() - 3., ne.bottom() - 3.);
    assert_eq!(
        pie_menu::slot_at(corner - open.center),
        Some(2),
        "in Right's wedge"
    );
    frame(&mut a, &ctx, vec![Event::PointerMoved(corner)]);
    assert_eq!(a.pie.as_ref().unwrap().highlighted(), Some(1));
    release_space(&mut a, &ctx);
    assert_eq!(a.camera.view, View::Perspective);
}

/// The layout is a tight cluster: buttons never overlap, every gap is small,
/// and the whole pie fits in about 420 x 230 px.
#[test]
fn pie_is_a_tight_cluster() {
    let c = Pos2::ZERO;
    let rects: Vec<Rect> = (0..pie_menu::SLOTS)
        .map(|i| pie_menu::label_rect(c, i, 120.))
        .collect();
    for (i, a) in rects.iter().enumerate() {
        for b in &rects[i + 1..] {
            assert!(!a.intersects(*b), "{a:?} overlaps {b:?}");
        }
    }
    let all = rects.iter().fold(Rect::NOTHING, |u, r| u.union(*r));
    assert!(all.width() <= 420. && all.height() <= 230., "{all:?}");
    // Neighbouring rows are 10 px apart.
    assert_eq!(rects[1].top() - rects[0].bottom(), 10.);
    assert_eq!(rects[2].top() - rects[1].bottom(), 10.);
    // Long labels widen outward and still fit the label.
    assert_eq!(pie_menu::button_width(40., false), 120.);
    assert!(pie_menu::button_width(130., true) > 130. + 18.);
    let wide = pie_menu::label_rect(c, 1, 170.);
    assert_eq!(wide.left(), rects[1].left());
}

/// Letting go of Ctrl first (Space still down) also counts as letting go
/// of Ctrl+Space: the highlighted slot runs.
#[test]
fn letting_go_of_the_modifier_first_runs_the_slot() {
    let mut a = perspective_app();
    let ctx = Context::default();
    let held = |a: &mut Operandi, events: Vec<Event>, m: Modifiers| {
        crate::tests::draw_held(a, &ctx, SIZE, events, m);
    };
    held(&mut a, vec![Event::PointerMoved(AT)], Modifiers::NONE);
    held(
        &mut a,
        vec![key_event(Key::Space, true, Modifiers::CTRL)],
        Modifiers::CTRL,
    );
    assert!(a.pie.is_some());
    held(
        &mut a,
        vec![Event::PointerMoved(AT + vec2(80., 0.))],
        Modifiers::CTRL,
    );
    assert!(a.pie.is_some(), "still held");
    // Ctrl up; Space is still down, so no key event arrives.
    held(&mut a, vec![], Modifiers::NONE);
    assert!(a.pie.is_none());
    assert_eq!(a.camera.view, View::Right);
}

/// GitHub #22: in Item mode the Wireframe toggle hides and shows the wire
/// there (Show Wireframe in Item Mode, VS-6); component modes keep their
/// overlay.
#[test]
fn wireframe_toggle_works_in_item_mode() {
    let mut a = perspective_app();
    a.editor.scene.selection.mode = Mode::Item;
    let toggle = "Toggle Wireframe Overlay";
    let shown = |a: &Operandi| {
        let g = a.display.settings.active;
        a.display.effective(g, Mode::Item).overlay != view_display::Overlay::None
    };
    assert!(shown(&a));
    assert_eq!(a.pie_status(toggle).mark, Mark::Toggle(true));
    a.execute(toggle);
    assert!(!shown(&a), "the wire is hidden in Item mode");
    assert_eq!(a.pie_status(toggle).mark, Mark::Toggle(false));
    // Component modes still show their own overlay.
    let g = a.display.settings.active;
    assert_ne!(
        a.display.effective(g, Mode::Polygon).overlay,
        view_display::Overlay::None
    );
    a.execute(toggle);
    assert!(shown(&a));
    assert_eq!(a.pie_status(toggle).mark, Mark::Toggle(true));
}
