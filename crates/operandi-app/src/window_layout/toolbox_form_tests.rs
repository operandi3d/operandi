//! Toolbox forms, layouts stage 3 batch 1 (form/command_form.rs,
//! toolbox_form.rs): the file format, shipped and user layering with
//! Revert, unknown lines, head/tail slots, show-only-when, proficiency,
//! and the UV tab drawn from its form.
use super::tests::{draw, draw_held, pointer};
use super::*;
use crate::form::Proficiency;
use crate::form::command_form::{self as cf, Form, FormStore, Item};
use glam::DVec2;

const SIZE: Vec2 = vec2(1440., 1400.);

/// Where the toolbox drew the form button that runs `line` last frame.
/// Tests find buttons by their command line, never by position, so a tab
/// can move onto a form (or its buttons move) without touching them.
pub(super) fn button_by_line(a: &Operandi, line: &str) -> Rect {
    *a.form_rects.get(line).unwrap_or_else(|| {
        panic!(
            "no form button runs {line:?}; drawn: {:?}",
            a.form_rects.keys()
        )
    })
}

/// Clicks the form button that runs `line`, with `mods` held.
pub(super) fn click_line(a: &mut Operandi, ctx: &Context, line: &str, mods: Modifiers) {
    let at = button_by_line(a, line).center();
    draw_held(a, ctx, SIZE, vec![Event::PointerMoved(at)], mods);
    draw_held(
        a,
        ctx,
        SIZE,
        vec![pointer(at, true, mods), pointer(at, false, mods)],
        mods,
    );
}

/// A 2 x 2 grid of quads with "Texture" UVs, in Polygon mode, the toolbox
/// on its UV tab.
fn uv_app(ctx: &Context) -> Operandi {
    theme::install(ctx);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=2u32 {
        for i in 0..=2u32 {
            mesh.vertices.push(DVec3::new(i as f64, 0., j as f64));
        }
    }
    let mut rows = Vec::new();
    for j in 0..2u32 {
        for i in 0..2u32 {
            let v = j * 3 + i;
            let corners = vec![v, v + 3, v + 4, v + 1];
            rows.push(
                corners
                    .iter()
                    .map(|&c| {
                        let p = mesh.vertices[c as usize];
                        Some(DVec2::new(p.x / 2., p.z / 2.))
                    })
                    .collect(),
            );
            mesh.faces.push(super::tests::face(corners));
        }
    }
    mesh.uv_maps.insert("Texture".into(), rows);
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.run(Command::Mode(Mode::Polygon));
    a.category = toolbox::Category::Uv;
    draw(&mut a, ctx, SIZE, vec![]);
    a
}

fn store(files: &[(&str, &str)]) -> FormStore {
    FormStore::read(files, None)
}

const UV: &str = "operandi.uv.preset uv.flipU";

#[test]
fn shipped_forms_read_and_every_line_is_a_command() {
    let a = Operandi::empty();
    let forms = toolbox_form::shipped_store(None);
    assert!(forms.problems.is_empty(), "{:?}", forms.problems);
    assert!(forms.get(toolbox_form::UV_FORM).is_some());
    let widgets: Vec<&str> = cf::WIDGETS.iter().map(|(id, _)| *id).collect();
    for id in forms.ids() {
        let items = cf::plan(&forms, id, Proficiency::Advanced, &|_| true);
        for item in items {
            let controls = match item {
                Item::Button(c, _) => vec![*c],
                Item::Row(cs, _, _) => cs,
                Item::Menu { controls, .. } | Item::Hold { controls, .. } => controls,
                Item::Widget(w) => {
                    assert!(
                        widgets.contains(&w.as_str()),
                        "{id}: widget {w} isn't listed"
                    );
                    continue;
                }
                Item::Missing(m) => panic!("{id}: no form {m}"),
                _ => continue,
            };
            for c in controls {
                if let Some(w) = &c.widget {
                    assert!(
                        widgets.contains(&w.as_str()),
                        "{id}: widget {w} isn't listed"
                    );
                    continue;
                }
                // A hold cell in a row: its lines (batch 7).
                if let Some(cf::FormRef::Inline(hold)) = &c.form {
                    for line in hold.controls.iter().filter_map(|h| h.command.as_deref()) {
                        assert!(a.line_state(line).is_some(), "{id}: {line} isn't a command");
                    }
                    continue;
                }
                let line = c.command.unwrap();
                assert!(
                    a.line_state(&line).is_some(),
                    "{id}: {line} isn't a command"
                );
                for v in c.variants.all().map(cf::Variant::line) {
                    assert!(a.line_state(v).is_some(), "{id}: {v} isn't a command");
                }
            }
        }
    }
}

#[test]
fn a_file_is_one_form_of_exactly_one_kind_per_control() {
    // Two kinds in one control: refused.
    let both = r#"{"id":"x","controls":[{"command":"Move","divider":"A"}]}"#;
    assert!(Form::parse(both).is_err());
    // No id, another version, unknown fields: refused.
    assert!(Form::parse(r#"{"controls":[]}"#).is_err());
    assert!(Form::parse(r#"{"version":2,"id":"x"}"#).is_err());
    assert!(Form::parse(r#"{"id":"x","colour":"red"}"#).is_err());
    // A round trip keeps it.
    let text = toolbox_form::SHIPPED_FORMS
        .iter()
        .find(|(n, _)| *n == "toolbox.uv.json")
        .unwrap()
        .1;
    let form = Form::parse(text).unwrap();
    assert_eq!(Form::parse(&form.to_json()).unwrap(), form);
    // An unknown line loads (it shows red when drawn, UIC-221).
    assert!(Form::parse(r#"{"id":"x","controls":[{"command":"no.such.command"}]}"#).is_ok());
}

#[test]
fn user_copy_wins_and_revert_drops_it() {
    // The UV operations, shared by the UV and Edit tabs (batch 6).
    const OPS: &str = "toolbox.uv.ops";
    let root = crate::layouts_tests::data_root();
    let ctx = Context::default();
    let mut a = crate::layouts_tests::launch(&root, &ctx);
    let file = root.join("Layouts/Forms/toolbox.uv.ops.json");
    assert!(!a.forms.is_changed(OPS));
    // The user's copy: Pack UVs... renamed.
    let mut mine = a.forms.get(OPS).unwrap().clone();
    let pack = mine
        .controls
        .iter_mut()
        .find(|c| c.command.as_deref() == Some("Pack UVs"))
        .unwrap();
    pack.label = "Pack Islands...".into();
    a.forms.save(mine.clone()).unwrap();
    assert!(file.exists());
    // Read back at the next start: the user's copy shows.
    let mut b = crate::layouts_tests::launch(&root, &ctx);
    assert_eq!(b.forms.get(OPS), Some(&mine));
    assert_ne!(b.forms.shipped(OPS), Some(&mine));
    b.category = toolbox::Category::Uv;
    draw(&mut b, &ctx, SIZE, vec![]);
    assert!(b.toolbox_rects.contains_key("Pack Islands..."));
    // The Edit tab shows the same copy.
    b.category = toolbox::Category::Edit;
    draw(&mut b, &ctx, SIZE, vec![]);
    assert!(b.toolbox_rects.contains_key("Pack Islands..."));
    b.category = toolbox::Category::Uv;
    // Revert, as a command line: the file goes, the shipped form shows.
    b.execute("operandi.form.revert toolbox.uv.ops");
    assert!(!file.exists(), "{}", b.status);
    assert!(!b.forms.is_changed(OPS));
    draw(&mut b, &ctx, SIZE, vec![]);
    assert!(b.toolbox_rects.contains_key("Pack UVs..."));
    b.execute("operandi.form.revert toolbox.uv.ops");
    assert!(b.status.contains("no changes"), "{}", b.status);
    b.execute("operandi.form.revert no.such.form");
    assert!(b.status.contains("No form"), "{}", b.status);
    // Saving a copy equal to the shipped form is no change: no file.
    let shipped = b.forms.shipped(OPS).unwrap().clone();
    b.forms.save(shipped).unwrap();
    assert!(!file.exists());
    // A user file of another version is ignored; the shipped form shows.
    std::fs::write(&file, r#"{"version":9,"id":"toolbox.uv.ops"}"#).unwrap();
    let c = crate::layouts_tests::launch(&root, &ctx);
    assert!(!c.forms.is_changed(OPS));
    assert_eq!(c.forms.problems.len(), 1, "{:?}", c.forms.problems);
    // The Forms folder is no layout.
    assert!(c.layouts.find("Forms").is_none());
}

#[test]
fn unknown_line_shows_red_and_does_nothing() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    let mut form = a.forms.get(toolbox_form::UV_FORM).unwrap().clone();
    form.controls.push(cf::Control {
        command: Some("no.such.command now".into()),
        label: "Broken".into(),
        ..Default::default()
    });
    a.forms.save(form).unwrap();
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.line_state("no.such.command now").is_none());
    let steps = a.editor.history.undo.len();
    let status = a.status.clone();
    let out = {
        click_line(&mut a, &ctx, "no.such.command now", Modifiers::NONE);
        draw(&mut a, &ctx, SIZE, vec![])
    };
    assert_eq!(a.editor.history.undo.len(), steps);
    assert_eq!(a.status, status, "a click runs nothing");
    // Drawn in the error colour.
    let rect = button_by_line(&a, "no.such.command now");
    let red = out.shapes.iter().any(|s| match &s.shape {
        egui::Shape::Rect(r) => r.rect == rect && r.stroke.color == theme::error(),
        _ => false,
    });
    assert!(red, "the unknown button is outlined red");
}

#[test]
fn slots_put_forms_at_the_head_and_tail_without_editing_the_target() {
    let target = r#"{"id":"t","controls":[{"command":"Move"}]}"#;
    let late = r#"{"id":"late","slot":{"form":"t","at":"tail","sort":20},"controls":[{"command":"Rotate"}]}"#;
    let early = r#"{"id":"early","slot":{"form":"t","at":"tail","sort":10},"controls":[{"command":"Scale"}]}"#;
    let head =
        r#"{"id":"head","slot":{"form":"t","at":"head"},"controls":[{"command":"Transform"}]}"#;
    let s = store(&[("t", target), ("l", late), ("e", early), ("h", head)]);
    let lines: Vec<String> = cf::plan(&s, "t", Proficiency::Advanced, &|_| true)
        .into_iter()
        .filter_map(|i| match i {
            Item::Button(c, _) => c.command,
            _ => None,
        })
        .collect();
    assert_eq!(lines, ["Transform", "Move", "Scale", "Rotate"]);
    assert_eq!(
        s.get("t").unwrap().controls.len(),
        1,
        "the target is untouched"
    );
}

#[test]
fn show_when_hides_a_form_or_section_while_its_command_is_off() {
    let form = r#"{"id":"f","controls":[
        {"command":"Move"},
        {"divider":"Islands","showWhen":"Fit"},
        {"command":"Rotate"},
        {"divider":"Always"},
        {"command":"Scale"}]}"#;
    let gated = r#"{"id":"g","showWhen":"Fit","slot":{"form":"f","at":"tail"},"controls":[{"command":"Transform"}]}"#;
    let s = store(&[("f", form), ("g", gated)]);
    let lines = |on: bool| -> Vec<String> {
        cf::plan(&s, "f", Proficiency::Advanced, &|l| l != "Fit" || on)
            .into_iter()
            .filter_map(|i| match i {
                Item::Button(c, _) => c.command,
                Item::Section { label, .. } => Some(format!("[{label}]")),
                _ => None,
            })
            .collect()
    };
    assert_eq!(
        lines(true),
        [
            "Move",
            "[Islands]",
            "Rotate",
            "[Always]",
            "Scale",
            "Transform"
        ]
    );
    assert_eq!(lines(false), ["Move", "[Always]", "Scale"]);
    // In the app the answer is the command's own enable state (UIC-223):
    // the island buttons are off while a tool is live.
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    assert!(a.line_enabled(UV));
    a.activate(Tool::Move);
    assert!(!a.line_enabled(UV));
}

#[test]
fn proficiency_shows_controls_up_to_the_panel_level() {
    let form = r#"{"id":"f","controls":[
        {"command":"Move"},
        {"command":"Rotate","level":"standard"},
        {"command":"Scale","level":"advanced"}]}"#;
    let s = store(&[("f", form)]);
    let count = |level| {
        cf::plan(&s, "f", level, &|_| true)
            .iter()
            .filter(|i| matches!(i, Item::Button(..)))
            .count()
    };
    assert_eq!(count(Proficiency::Basic), 1);
    assert_eq!(count(Proficiency::Standard), 2);
    assert_eq!(count(Proficiency::Advanced), 3);
}

#[test]
fn variants_pick_ctrl_then_shift_then_alt() {
    let v = cf::Variants {
        shift: Some("s".into()),
        alt: Some("a".into()),
        ctrl: Some("c".into()),
    };
    assert_eq!(v.pick("p", false, false, false), "p");
    assert_eq!(v.pick("p", true, true, true), "c");
    assert_eq!(v.pick("p", false, true, true), "s");
    assert_eq!(v.pick("p", false, false, true), "a");
    assert_eq!(cf::Variants::default().pick("p", true, true, true), "p");
}

/// The "looks identical" check against an earlier build (opt-in): write
/// every shape the UV tab paints at rest, then with Move live, one per
/// line, to the file `OPERANDI_PAINT_DUMP` names. Run it on both builds
/// and compare. (Batch 1 compared the form with the old hard-coded tab in
/// one process, at rest, with Move live and folded: identical, before the
/// old code was deleted.)
#[test]
#[ignore]
fn paint_dump_uv_tab() {
    let Ok(path) = std::env::var("OPERANDI_PAINT_DUMP") else {
        return;
    };
    let mut text = String::new();
    for live in [false, true] {
        let ctx = Context::default();
        let mut a = uv_app(&ctx);
        if live {
            a.activate(Tool::Move);
        }
        let mut out = None;
        for _ in 0..3 {
            out = Some(draw(&mut a, &ctx, SIZE, vec![]));
        }
        for s in &out.unwrap().shapes {
            text.push_str(&format!("{:?} {:?}\n", s.clip_rect, s.shape));
        }
    }
    std::fs::write(path, text).unwrap();
}

#[test]
fn uv_tab_buttons_run_their_lines() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    // Unwrap... and Pack UVs... open their dialogs.
    click_line(&mut a, &ctx, "UV Unwrap", Modifiers::NONE);
    assert!(
        matches!(a.uv_dialog, Some(uv_dialogs::UvDialog::Unwrap(_))),
        "{}",
        a.status
    );
    a.uv_dialog = None;
    click_line(&mut a, &ctx, "Pack UVs", Modifiers::NONE);
    assert!(matches!(a.uv_dialog, Some(uv_dialogs::UvDialog::Pack(_))));
    a.uv_dialog = None;
    // Flip U runs as one undo step.
    let before = a.editor.scene.active().unwrap().mesh.uv_maps["Texture"][0][0].unwrap();
    click_line(&mut a, &ctx, UV, Modifiers::NONE);
    assert_eq!(
        a.editor.history.undo.last().unwrap().label,
        "Flip U",
        "{}",
        a.status
    );
    let after = a.editor.scene.active().unwrap().mesh.uv_maps["Texture"][0][0].unwrap();
    assert!(
        (after.x - (1. - before.x)).abs() < 1e-9,
        "{before} -> {after}"
    );
    // Ctrl/Cmd+click: the rule's dialog (its variant line).
    click_line(
        &mut a,
        &ctx,
        "operandi.uv.preset uv.fitUVs",
        Modifiers::COMMAND,
    );
    assert!(
        matches!(
            a.uv_dialog,
            Some(uv_dialogs::UvDialog::Island(
                _,
                operandi_core::uv_edit::RuleKind::Fit
            ))
        ),
        "{}",
        a.status
    );
    a.uv_dialog = None;
    // The shared top row's Rotate (the form no longer repeats it, Jason
    // 2026-11-24) starts its tool; the started one is lit (its command's
    // answer).
    a.execute("Rotate");
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Rotate)
    );
    assert!(a.line_state("Rotate").unwrap().on);
    assert!(!a.line_state("Move").unwrap().on);
    // While a tool is live the island buttons are greyed with the reason,
    // and a click does nothing.
    let why = a.line_state(UV).unwrap().unavailable;
    assert_eq!(why.as_deref(), Some(crate::unavailable::TOOL_ACTIVE));
    let steps = a.editor.history.undo.len();
    draw(&mut a, &ctx, SIZE, vec![]);
    click_line(&mut a, &ctx, UV, Modifiers::NONE);
    assert_eq!(a.editor.history.undo.len(), steps);
}

#[test]
fn uv_tab_section_folds_under_its_old_key() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    let header = a.toolbox_rects["UV Tools"].center();
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(header)]);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![
            pointer(header, true, Modifiers::NONE),
            pointer(header, false, Modifiers::NONE),
        ],
    );
    assert!(a.create_folded.contains("uv-tools"));
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(
        !a.form_rects.contains_key("UV Unwrap"),
        "folded: no buttons"
    );
}

/// Batch 3's form controls: a heading is a plain title, a `flyout`
/// sub-form one item with its face, and `look: flat` / `noShortcut` read
/// back as written.
#[test]
fn headings_and_tool_flyouts_plan_as_their_own_items() {
    let file = r#"{"version":1,"id":"t","controls":[
        {"heading":"Clone","id":"Duplicate Clone"},
        {"command":"Clone","look":"flat","noShortcut":true},
        {"form":{"style":"flyout","controls":[{"command":"Shear"},{"command":"Soft Move"}]},
         "id":"Soft Drag","face":1}
    ]}"#;
    let s = store(&[("t.json", file)]);
    let items = cf::plan(&s, "t", Proficiency::Advanced, &|_| true);
    assert!(
        matches!(&items[0], Item::Heading { key, label, .. } if key == "Duplicate Clone" && label == "Clone")
    );
    let Item::Button(clone, _) = &items[1] else {
        panic!("{:?}", items[1]);
    };
    assert_eq!(clone.look, cf::Look::Flat);
    assert!(clone.no_shortcut);
    let Item::Flyout {
        key,
        face,
        controls,
    } = &items[2]
    else {
        panic!("{:?}", items[2]);
    };
    assert_eq!((key.as_str(), *face, controls.len()), ("Soft Drag", 1, 2));
    // A face past the end is clamped to the last command.
    let file = file.replace("\"face\":1", "\"face\":9");
    let s = store(&[("t.json", &file)]);
    let items = cf::plan(&s, "t", Proficiency::Advanced, &|_| true);
    assert!(matches!(&items[2], Item::Flyout { face: 1, .. }));
}

/// A 2 x 2 x 2 box in `mode`, the toolbox on `category`.
fn box_app(ctx: &Context, category: toolbox::Category, mode: Mode) -> Operandi {
    theme::install(ctx);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
            .unwrap()
            .into();
    a.category = category;
    a.run(Command::Mode(mode));
    draw(&mut a, ctx, SIZE, vec![]);
    a
}

/// Batch 4: the Vertex tab from its form. Weld lights from its command
/// and Drop Weld shows only while Weld is on (LAY-85, `Drop Weld`'s
/// answer); Show Vertices is a checkbox ticked from its command; New
/// Vertex and Reorder Index grey outside Vertex mode with the reason.
#[test]
fn vertex_tab_weld_show_vertices_and_greyed_rows() {
    let ctx = Context::default();
    let mut a = box_app(&ctx, toolbox::Category::Vertex, Mode::Vertex);
    assert!(
        !a.form_rects.contains_key("Drop"),
        "no Drop Weld while Weld is off"
    );
    assert!(a.line_state("Drop Weld").unwrap().unavailable.is_some());
    click_line(&mut a, &ctx, "Weld", Modifiers::NONE);
    assert!(a.vertex_weld, "{}", a.status);
    assert!(a.line_state("Weld").unwrap().on);
    draw(&mut a, &ctx, SIZE, vec![]);
    click_line(&mut a, &ctx, "Drop", Modifiers::NONE);
    assert!(!a.vertex_weld, "{}", a.status);
    assert_eq!(a.status, "Weld off");
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(!a.form_rects.contains_key("Drop"));
    // Typed, Drop Weld refuses while Weld is off and ends it while on.
    a.execute("Drop Weld");
    assert_eq!(a.status, "Weld is off");
    a.execute("Weld");
    a.execute("Drop Weld");
    assert!(!a.vertex_weld, "{}", a.status);
    // Show Vertices: the checkbox follows the setting and flips it.
    let before = a.display.settings.drawing.show_verts;
    assert_eq!(a.line_state("Show Vertices").unwrap().on, before);
    click_line(&mut a, &ctx, "Show Vertices", Modifiers::NONE);
    assert_eq!(a.display.settings.drawing.show_verts, !before);
    assert_eq!(a.line_state("Show Vertices").unwrap().on, !before);
    // New Vertex and Reorder Index: greyed in Polygon mode, a click does
    // nothing; Split stays available.
    a.run(Command::Mode(Mode::Polygon));
    draw(&mut a, &ctx, SIZE, vec![]);
    for line in ["New Vertex", "Reorder Index"] {
        let why = a.line_state(line).unwrap().unavailable;
        assert_eq!(
            why.as_deref(),
            Some("Switch to Vertices mode first"),
            "{line}"
        );
        click_line(&mut a, &ctx, line, Modifiers::NONE);
        assert!(!a.has_modal(), "{line} opened its dialog");
    }
    assert!(a.line_state("Vertex Split").unwrap().unavailable.is_none());
    // The section folds under its old key.
    a.create_folded.insert("vertex-tools".into());
    draw(&mut a, &ctx, SIZE, vec![]);
    a.form_rects.clear();
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(
        !a.form_rects.contains_key("Vertex Split"),
        "folded: no rows"
    );
}

/// Batch 4: the Edge tab's Add Loop / Add Point rows run their commands,
/// light while armed and disarm on a second click; Bridge lights while
/// it is armed.
#[test]
fn edge_tab_click_tools_and_bridge_light_from_their_commands() {
    let ctx = Context::default();
    let mut a = box_app(&ctx, toolbox::Category::Edge, Mode::Edge);
    for (line, tool) in [
        ("Add Loop", edge_panel::EdgeClick::AddLoop),
        ("Add Point", edge_panel::EdgeClick::AddPoint),
    ] {
        click_line(&mut a, &ctx, line, Modifiers::NONE);
        assert_eq!(a.edge_click, Some(tool), "{}", a.status);
        assert!(a.line_state(line).unwrap().on);
        click_line(&mut a, &ctx, line, Modifiers::NONE);
        assert_eq!(a.edge_click, None, "{}", a.status);
        assert!(!a.line_state(line).unwrap().on);
    }
    assert!(!a.line_state("Bridge").unwrap().on);
    click_line(&mut a, &ctx, "Bridge", Modifiers::NONE);
    assert!(a.editor.session.bridge.is_some(), "{}", a.status);
    assert!(a.line_state("Bridge").unwrap().on);
    // Set Position... opens Set Vertex Position.
    a.run(Command::Cancel);
    let e = a.editor.scene.active().unwrap().mesh.edges()[0];
    a.editor.scene.selection.edges = vec![e];
    draw(&mut a, &ctx, SIZE, vec![]);
    click_line(&mut a, &ctx, "Set Position", Modifiers::NONE);
    assert!(a.set_position.is_some(), "{}", a.status);
}

/// Batch 4's form pieces: `look: check` and `look: button` read back,
/// and `{binding}` is the line's binding as a button draws it.
#[test]
fn check_and_button_looks_and_binding_placeholder() {
    let file = r#"{"version":1,"id":"t","controls":[
        {"command":"Show Vertices","look":"check"},
        {"command":"Drop","label":"Drop Weld","look":"button"}
    ]}"#;
    let s = store(&[("t.json", file)]);
    let items = cf::plan(&s, "t", Proficiency::Advanced, &|_| true);
    let looks: Vec<cf::Look> = items
        .iter()
        .filter_map(|i| match i {
            Item::Button(c, _) => Some(c.look),
            _ => None,
        })
        .collect();
    assert_eq!(looks, [cf::Look::Check, cf::Look::Button]);
    let a = Operandi::empty();
    let bound = a.shortcut("Vertex Collapse");
    assert!(!bound.is_empty(), "Vertex Collapse has a default binding");
    assert_eq!(
        a.form_tooltip_for_test("Collapse · vert.collapse{binding}", "Vertex Collapse"),
        format!("Collapse · vert.collapse{bound}")
    );
    assert_eq!(
        a.form_tooltip_for_test("New · vert.new{binding}", "New Vertex"),
        "New · vert.new"
    );
}

/// The Polygon tab's last rows sit below the fold at `SIZE`.
const TALL: Vec2 = vec2(1440., 2000.);

fn click_tall(a: &mut Operandi, ctx: &Context, line: &str, mods: Modifiers) {
    let at = button_by_line(a, line).center();
    draw_held(a, ctx, TALL, vec![Event::PointerMoved(at)], mods);
    draw_held(
        a,
        ctx,
        TALL,
        vec![pointer(at, true, mods), pointer(at, false, mods)],
        mods,
    );
}

/// Batch 5: the Polygon tab from its form. Each session and dialog row
/// runs its command and lights from the command's answer while open;
/// Scale Offset (Detach)... runs its script line.
#[test]
fn polygon_tab_rows_run_their_commands_and_light_while_open() {
    let ctx = Context::default();
    let mut a = box_app(&ctx, toolbox::Category::Polygon, Mode::Polygon);
    a.editor.scene.selection.faces = vec![0];
    draw(&mut a, &ctx, TALL, vec![]);
    type Open = fn(&Operandi) -> bool;
    type Close = fn(&mut Operandi);
    let rows: [(&str, Open, Close); 9] = [
        (
            "Polygon Pen",
            |a| a.editor.session.pen_draft.is_some(),
            |a| a.run(Command::Cancel),
        ),
        (
            "Spike",
            |a| a.editor.session.spike.is_some(),
            |a| a.run(Command::Cancel),
        ),
        (
            "Bridge",
            |a| a.editor.session.bridge.is_some(),
            |a| a.run(Command::Cancel),
        ),
        (
            "Polygon Reduction",
            |a| a.editor.session.reduction.is_some(),
            |a| a.run(Command::Cancel),
        ),
        (
            "Set Position",
            |a| a.set_position.is_some(),
            |a| a.set_position = None,
        ),
        (
            "Scale Offset",
            |a| a.scale_offset.is_some(),
            |a| a.scale_offset = None,
        ),
        ("Flip It", |a| a.flip_it.is_some(), |a| a.flip_it = None),
        ("Set Part", |a| a.set_part.is_some(), |a| a.set_part = None),
        (
            "Set Material",
            |a| a.set_material.is_some(),
            |a| a.set_material = None,
        ),
    ];
    for (line, open, close) in rows {
        assert!(!a.line_state(line).unwrap().on, "{line} lit before");
        click_tall(&mut a, &ctx, line, Modifiers::NONE);
        assert!(open(&a), "{line}: {}", a.status);
        assert!(a.line_state(line).unwrap().on, "{line} not lit");
        close(&mut a);
        draw(&mut a, &ctx, TALL, vec![]);
    }
    // Flip It Origin lights only its own row.
    click_tall(&mut a, &ctx, "Flip It Origin", Modifiers::NONE);
    assert!(a.line_state("Flip It Origin").unwrap().on);
    assert!(!a.line_state("Flip It").unwrap().on);
    a.flip_it = None;
    draw(&mut a, &ctx, TALL, vec![]);
    click_tall(
        &mut a,
        &ctx,
        "@ppi_PolyScale_Offset_Detach.py",
        Modifiers::NONE,
    );
    assert!(a.scale_offset.is_some(), "{}", a.status);
    a.scale_offset = None;
    // The section folds under its old key.
    a.create_folded.insert("polygon-tools".into());
    a.form_rects.clear();
    draw(&mut a, &ctx, TALL, vec![]);
    assert!(!a.form_rects.contains_key("Triple"), "folded: no rows");
}

/// Batch 5: plain rows take a modifier-click as a click (LAY-Q31's rule;
/// the old rows refused it), and the tool rows start their tools.
#[test]
fn polygon_tab_modifier_clicks_run_the_plain_line() {
    let ctx = Context::default();
    let mut a = box_app(&ctx, toolbox::Category::Polygon, Mode::Polygon);
    a.editor.scene.selection.faces = vec![0];
    draw(&mut a, &ctx, TALL, vec![]);
    let before = a.editor.scene.active().unwrap().mesh.polygon_count();
    click_tall(&mut a, &ctx, "Triple", Modifiers::SHIFT);
    let after = a.editor.scene.active().unwrap().mesh.polygon_count();
    assert_eq!(after, before + 1, "{}", a.status);
    a.editor.scene.selection.faces = vec![0];
    for line in ["Bevel", "Thicken", "Poly Haul", "Sketch Extrude"] {
        click_tall(&mut a, &ctx, line, Modifiers::NONE);
        assert!(a.line_state(line).unwrap().on, "{line}: {}", a.status);
        a.run(Command::Cancel);
        draw(&mut a, &ctx, TALL, vec![]);
    }
}

/// Batch 5: Set Material... wears its Alt variant while Alt is held
/// (label, lit state and tooltip from Set Part), and a click runs what
/// was shown at the press.
#[test]
fn polygon_tab_set_material_shows_its_alt_variant() {
    let ctx = Context::default();
    let mut a = box_app(&ctx, toolbox::Category::Polygon, Mode::Polygon);
    let count = |out: &egui::FullOutput, text: &str| {
        let needle = format!("text: \"{text}\"");
        out.shapes
            .iter()
            .filter(|s| format!("{:?}", s.shape).contains(&needle))
            .count()
    };
    let plain = draw(&mut a, &ctx, TALL, vec![]);
    assert_eq!(count(&plain, "Set Material..."), 1);
    assert_eq!(count(&plain, "Set Part..."), 1);
    let alt = draw_held(&mut a, &ctx, TALL, vec![], Modifiers::ALT);
    assert_eq!(count(&alt, "Set Material..."), 0);
    assert_eq!(
        count(&alt, "Set Part..."),
        2,
        "the Set Material row and the Set Part row"
    );
    // Pressed with Alt, released without: Set Part opens (MB-10).
    let at = button_by_line(&a, "Set Material").center();
    draw_held(
        &mut a,
        &ctx,
        TALL,
        vec![Event::PointerMoved(at)],
        Modifiers::ALT,
    );
    draw_held(
        &mut a,
        &ctx,
        TALL,
        vec![pointer(at, true, Modifiers::ALT)],
        Modifiers::ALT,
    );
    draw(
        &mut a,
        &ctx,
        TALL,
        vec![pointer(at, false, Modifiers::NONE)],
    );
    assert!(
        a.set_part.is_some() && a.set_material.is_none(),
        "{}",
        a.status
    );
    a.set_part = None;
    draw(&mut a, &ctx, TALL, vec![]);
    click_tall(&mut a, &ctx, "Set Material", Modifiers::NONE);
    assert!(a.set_material.is_some(), "{}", a.status);
}

/// Batch 5's form pieces: `look: natural`, labelled variants (they read
/// back and write out as they came) and the `{variants}` placeholder.
#[test]
fn natural_look_and_labelled_variants_read_back() {
    let file = r#"{"version":1,"id":"t","controls":[
        {"command":"Flip","look":"natural"},
        {"command":"Set Material","variants":{
            "alt":{"line":"Set Part","label":"Set Part...","tooltip":"T"},
            "shift":"Remove"}}
    ]}"#;
    let form = Form::parse(file).unwrap();
    assert_eq!(form.controls[0].look, cf::Look::Natural);
    let v = &form.controls[1].variants;
    assert!(v.shown());
    assert_eq!(v.pick("Set Material", false, false, true), "Set Part");
    assert_eq!(v.pick("Set Material", false, true, false), "Remove");
    assert!(matches!(&v.alt, Some(cf::Variant::Shown { label, .. }) if label == "Set Part..."));
    assert_eq!(Form::parse(&form.to_json()).unwrap(), form);
    assert!(!cf::Variants::default().shown());
}

/// Batch 6: the Edit tab's Slide wears the mode's slide icon and key
/// (show-when on `operandi.mode.is`), greys outside Vertex and Edge mode
/// from `Edit Slide`'s answer, and its rows wait for the active tool.
#[test]
fn edit_tab_slide_follows_the_mode_and_rows_wait_for_the_tool() {
    let ctx = Context::default();
    let mut a = box_app(&ctx, toolbox::Category::Edit, Mode::Polygon);
    assert!(a.line_enabled("operandi.mode.is item edge polygon"));
    assert!(!a.line_enabled("operandi.mode.is vertex"));
    let why = a.line_state("Edit Slide").unwrap().unavailable;
    assert_eq!(
        why.as_deref(),
        Some("Switch to Vertices or Edges mode (Polygon Slide isn't built yet)")
    );
    a.run(Command::Mode(Mode::Vertex));
    a.editor.scene.selection.vertices = vec![0];
    draw(&mut a, &ctx, TALL, vec![]);
    assert!(a.line_enabled("operandi.mode.is vertex"));
    click_tall(&mut a, &ctx, "Edit Slide", Modifiers::NONE);
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::VertexSlide)
    );
    assert!(a.line_state("Edit Slide").unwrap().on);
    // A live tool greys Subdivide, Center, Freeze, Boolean and Remesh.
    for line in [
        "SDS Subdivide",
        "Center X",
        "poly.freeze",
        "Boolean",
        "Remesh",
    ] {
        let why = a.line_state(line).unwrap().unavailable;
        assert_eq!(
            why.as_deref(),
            Some(crate::unavailable::TOOL_ACTIVE),
            "{line}"
        );
    }
    a.run(Command::Drop);
    // Typed, the new SDS Subdivide 2X subdivides twice in one step.
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces.clear();
    let steps = a.editor.history.undo.len();
    a.execute("SDS Subdivide 2X");
    assert_eq!(
        a.editor.scene.active().unwrap().mesh.faces.len(),
        96,
        "{}",
        a.status
    );
    assert_eq!(a.editor.history.undo.len(), steps + 1);
    // The Dicer placeholder still explains why the action is unavailable.
    let why = a
        .line_state("operandi.notBuilt \"Dicer\"")
        .unwrap()
        .unavailable;
    assert_eq!(why.as_deref(), Some("Dicer · not in Operandi yet"));
}

/// Batch 6: the Sculpt tab's rows run their commands and light from
/// them: a brush, a tip, the locks and Live Deformers.
#[test]
fn sculpt_tab_rows_run_and_light_from_their_commands() {
    let ctx = Context::default();
    let mut a = box_app(&ctx, toolbox::Category::Sculpt, Mode::Polygon);
    // Tips wait for a brush.
    let why = a.line_state("brush.hard").unwrap().unavailable;
    assert_eq!(why.as_deref(), Some("Pick a brush first"));
    click_tall(&mut a, &ctx, "Sculpt Clay", Modifiers::NONE);
    assert_eq!(a.sculpt.brush, Some(operandi_core::SculptBrush::Clay));
    assert!(a.line_state("Sculpt Clay").unwrap().on);
    draw(&mut a, &ctx, TALL, vec![]);
    click_tall(&mut a, &ctx, "brush.hard", Modifiers::NONE);
    assert!(a.line_state("brush.hard").unwrap().on, "{}", a.status);
    assert!(!a.line_state("brush.smooth").unwrap().on);
    for (line, get) in [
        (
            "sculpt.borderLock",
            (|a: &Operandi| a.sculpt.settings.lock_borders) as fn(&Operandi) -> bool,
        ),
        ("sculpt.cornerLock", |a: &Operandi| {
            a.sculpt.settings.lock_corners
        }),
        ("sculpt.liveWeightEdit", |a: &Operandi| {
            a.sculpt.settings.live_deformers
        }),
    ] {
        let before = get(&a);
        click_tall(&mut a, &ctx, line, Modifiers::NONE);
        assert_eq!(get(&a), !before, "{line}: {}", a.status);
        assert_eq!(a.line_state(line).unwrap().on, !before, "{line}");
    }
    // The same brush row again drops it.
    click_tall(&mut a, &ctx, "Sculpt Clay", Modifiers::NONE);
    assert_eq!(a.sculpt.brush, None);
    // Remesh... opens Voxel Remesh.
    click_tall(&mut a, &ctx, "sculpt.remesh", Modifiers::NONE);
    assert!(a.remesh.is_some(), "{}", a.status);
    assert!(a.line_state("sculpt.remesh").unwrap().on);
}

/// Batch 7: the Create tab's unit-primitive lines add 1 m primitives as one
/// undo step each, into the mesh (Unit) or as a new item (New Unit).
#[test]
fn unit_primitive_lines_add_one_undo_step_each() {
    for name in [
        "Cube",
        "Sphere",
        "Cylinder",
        "Ellipsoid",
        "Cone",
        "Torus",
        "Capsule",
    ] {
        let ctx = Context::default();
        let mut a = box_app(&ctx, toolbox::Category::Create, Mode::Item);
        let (items, faces) = (
            a.editor.scene.items.len(),
            a.editor.scene.active().unwrap().mesh.faces.len(),
        );
        a.execute(&format!("{name} Unit"));
        assert_eq!(a.editor.scene.items.len(), items, "{name}: {}", a.status);
        assert!(
            a.editor.scene.active().unwrap().mesh.faces.len() > faces,
            "{name}"
        );
        a.execute(&format!("{name} New Unit"));
        assert_eq!(
            a.editor.scene.items.len(),
            items + 1,
            "{name}: {}",
            a.status
        );
        a.run(Command::Undo);
        a.run(Command::Undo);
        assert_eq!(a.editor.scene.items.len(), items, "{name}");
        assert_eq!(
            a.editor.scene.active().unwrap().mesh.faces.len(),
            faces,
            "{name}"
        );
        a.run(Command::Redo);
        assert!(
            a.editor.scene.active().unwrap().mesh.faces.len() > faces,
            "{name}"
        );
    }
}

/// Batch 7: the Tube hold cell runs Tube on a click, lists Tube and Solid
/// Sketch on a right-click, lights while either is on and wears Solid
/// Sketch's icon while it is.
#[test]
fn tube_hold_cell_runs_tube_and_lists_solid_sketch() {
    let ctx = Context::default();
    let mut a = box_app(&ctx, toolbox::Category::Create, Mode::Item);
    click_line(&mut a, &ctx, "Tube", Modifiers::NONE);
    assert!(
        a.editor
            .session
            .shape
            .as_ref()
            .is_some_and(|s| s.kind() == ShapeKind::Tube),
        "{}",
        a.status
    );
    a.run(Command::Drop);
    draw(&mut a, &ctx, SIZE, vec![]);
    let at = button_by_line(&a, "Tube").center();
    let right = |pressed| Event::PointerButton {
        pos: at,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Secondary,
    };
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(&mut a, &ctx, SIZE, vec![right(true), right(false)]);
    let out = draw(&mut a, &ctx, SIZE, vec![]);
    let listed = format!("{:?}", out.shapes);
    assert!(
        listed.contains("Solid Sketch"),
        "the menu lists Solid Sketch"
    );
    assert!(
        a.editor.session.shape.is_none(),
        "a right-click runs nothing"
    );
    // Pick Solid Sketch from the open menu by running its line, as the menu
    // does; the cell then lights.
    a.execute("Solid Sketch");
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.editor.session.solid_sketch.is_some(), "{}", a.status);
    assert!(a.line_state("Solid Sketch").unwrap().on);
}

/// Batch 7: `columns` keeps a short row's cells the grid's size, and the
/// `{binding:<line>}` and `{andBinding}` placeholders name bindings.
#[test]
fn columns_and_binding_placeholders() {
    let ctx = Context::default();
    let mut a = box_app(&ctx, toolbox::Category::Create, Mode::Item);
    let cube = button_by_line(&a, "Cube");
    let arc = button_by_line(&a, "Arc");
    assert!(
        (cube.width() - arc.width()).abs() < 0.01,
        "{cube:?} {arc:?}"
    );
    a.bindings.insert("Solid Sketch".into(), "Alt+K".into());
    let tip = a.form_tooltip_for_test("Tube{binding:Solid Sketch}", "Tube");
    assert_eq!(tip, format!("Tube{}", a.shortcut("Solid Sketch")));
    assert_eq!(
        a.form_tooltip_for_test("Ruler{andBinding}", "Ruler"),
        "Ruler"
    );
    a.bindings.insert("Ruler".into(), "Alt+R".into());
    assert_eq!(
        a.form_tooltip_for_test("Ruler{andBinding}", "Ruler"),
        format!("Ruler · {}", a.shortcut("Ruler"))
    );
}

/// Batch 7: the Create tab's cells light from their commands, and Merge
/// with one mesh selected says what it needs instead of running.
#[test]
fn create_tab_cells_light_and_merge_explains() {
    let ctx = Context::default();
    let mut a = box_app(&ctx, toolbox::Category::Create, Mode::Item);
    for line in ["Ruler", "Curve", "Ellipsoid", "Arc", "Paste Tool"] {
        if line == "Paste Tool" {
            a.run(Command::Mode(Mode::Polygon));
            a.editor.scene.selection.faces = vec![0];
            a.copy_selection();
        }
        click_line(&mut a, &ctx, line, Modifiers::NONE);
        assert!(a.line_state(line).unwrap().on, "{line}: {}", a.status);
        a.run(Command::Drop);
        a.drop_utility();
        a.drop_draw();
    }
    let undo = a.editor.history.undo.len();
    click_line(&mut a, &ctx, "Item Merge", Modifiers::NONE);
    assert_eq!(a.editor.history.undo.len(), undo);
    assert!(
        a.status.starts_with("Merge: select two or more meshes"),
        "{}",
        a.status
    );
}

/// N24 (UIC-25): holding the left button on a pop-up cell opens its
/// alternates, as the right-click does; the release runs nothing.
#[test]
fn n24_holding_a_pop_up_cell_opens_its_list() {
    let ctx = Context::default();
    let mut a = box_app(&ctx, toolbox::Category::Create, Mode::Item);
    let at = button_by_line(&a, "Tube").center();
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(&mut a, &ctx, SIZE, vec![pointer(at, true, Modifiers::NONE)]);
    // Half a second of frames with the button held (the hold is 0.35 s).
    let mut listed = String::new();
    for _ in 0..30 {
        let out = draw(&mut a, &ctx, SIZE, vec![]);
        listed = format!("{:?}", out.shapes);
    }
    assert!(listed.contains("Solid Sketch"), "the hold opens the list");
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(at, false, Modifiers::NONE)],
    );
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(
        a.editor.session.shape.is_none(),
        "the release ending the hold runs nothing"
    );
}
