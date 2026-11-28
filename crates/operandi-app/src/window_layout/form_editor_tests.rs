//! Layouts stage 7: the Form Editor and menus as forms
//! (BEHAVIOR-LAYOUTS-STAGE7.md, form_editor.rs, app_menu.rs).
use super::*;
use crate::app_menu::{Entry, Menu, line_action};
use crate::form::command_form::{self as cf, End, Form};
use crate::layouts_tests::{data_root, frames, launch};
use crate::toolbox_form_tests::button_by_line;
use std::path::Path;

fn app(root: &Path) -> (Operandi, Context) {
    let ctx = Context::default();
    let mut a = launch(root, &ctx);
    a.load_palettes();
    a.load_form_groups();
    frames(&mut a, &ctx, 2);
    (a, ctx)
}

fn menu(a: &Operandi, ctx: &Context, title: &str) -> Menu {
    a.menu_model(ctx)
        .into_iter()
        .find(|m| m.title == title || m.id == format!("menu.{}", title.to_lowercase()))
        .unwrap_or_else(|| panic!("no {title} menu"))
}

/// A menu's items as (title, enabled, why, checked).
fn rows(m: &Menu) -> Vec<(String, bool, Option<String>, bool)> {
    m.entries
        .iter()
        .filter_map(|e| match e {
            Entry::Item(i) => Some((i.title.clone(), i.enabled, i.why.clone(), i.checked)),
            _ => None,
        })
        .collect()
}

fn form(a: &Operandi, id: &str) -> Form {
    a.forms.get(id).cloned().unwrap()
}

// ------------------------------------------------- menus as forms

/// UIC-10 / UIC-210: Texture, Render, Help (and the new System) are drawn
/// from forms. Each paints and behaves as the hand-written menu did: the
/// expectations below are what the old code built (its titles, greying
/// reasons, ticks and shortcuts), checked in both states that change them.
#[test]
fn uic210_converted_menus_paint_as_the_old_ones() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    // Default-build menus; painting's entries: painting_tests.rs.
    #[cfg(feature = "painting")]
    {
        a.painting.hide_menus = true;
    }
    let cycles = operandi_cycles::available();
    let no_cycles = (!cycles).then(|| "This build has no Cycles renderer".to_owned());
    let key = |a: &Operandi, name: &str| a.shortcut(name);
    let expect_render = |a: &Operandi, preview: bool| {
        vec![
            (
                format!("Render{}", key(a, "Render")),
                cycles,
                no_cycles.clone(),
                false,
            ),
            (
                format!("Preview{}", key(a, "Preview")),
                cycles,
                no_cycles.clone(),
                preview,
            ),
            (
                format!(
                    "{}{}",
                    crate::image_viewer_ui::OPEN,
                    key(a, crate::image_viewer_ui::OPEN)
                ),
                true,
                None,
                false,
            ),
            // Render outputs group D (RO-Q27).
            ("Add Per-Light Render Outputs".to_owned(), true, None, false),
        ]
    };
    assert_eq!(rows(&menu(&a, &ctx, "Render")), expect_render(&a, false));
    a.preview.open = true;
    assert_eq!(
        rows(&menu(&a, &ctx, "Render")),
        expect_render(&a, a.preview.open)
    );
    a.preview.open = false;
    // TextureCraft pieces (2026-11-17) added the last four entries; the UV
    // tools finish (2026-11-18) the UV export rows.
    let texture = |a: &Operandi, open: bool| {
        let mut rows = vec![(
            format!("Open UV Editor{}", key(a, "Open UV Editor")),
            true,
            None,
            open,
        )];
        for label in [
            "Atlas Unwrap...",
            "Export UVs to SVG...",
            "Export UVs to EPS...",
            "Convert UVs to Mesh",
            "Transfer UVs",
            "Bake Curvature...",
            "Bake Thickness...",
            "Pad Texture to UVs...",
        ] {
            rows.push((label.to_owned(), true, None, false));
        }
        rows
    };
    assert_eq!(rows(&menu(&a, &ctx, "Texture")), texture(&a, false));
    a.execute("layout.window hash:uvEditor open:true");
    assert_eq!(rows(&menu(&a, &ctx, "Texture")), texture(&a, true));
    assert_eq!(
        rows(&menu(&a, &ctx, "Help")),
        vec![
            ("User Guide".to_owned(), true, None, false),
            ("What's This?  [F1] ".to_owned(), true, None, false),
            ("Report Bug or Idea…".to_owned(), true, None, false),
            ("Check for Updates".to_owned(), true, None, false),
        ]
    );
    // No native key equivalents, as before.
    for title in ["Render", "Texture", "Help", "System"] {
        assert!(
            menu(&a, &ctx, title)
                .leaves()
                .iter()
                .all(|i| i.command_key.is_empty())
        );
    }
}

#[test]
fn uic210_converted_menu_items_run_their_lines() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.menu_action(&ctx, line_action("Open UV Editor"));
    assert!(a.palette_open(crate::palettes::UV_EDITOR));
    a.menu_action(&ctx, line_action("Open UV Editor"));
    assert!(!a.palette_open(crate::palettes::UV_EDITOR));
    a.menu_action(&ctx, line_action("Open Render Window"));
    assert!(a.render.open);
    // Recorded as from a menu (UIC-224).
    let last = a.history.log.records.back().unwrap();
    assert_eq!(last.source, operandi_core::Source::Menu);
}

#[test]
fn lay83_system_form_editor_opens_the_floating_form_editor() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    let system = menu(&a, &ctx, "System");
    assert_eq!(
        rows(&system),
        vec![
            ("Form Editor".to_owned(), true, None, false),
            // Python scripting (preview), shown while its switch is on.
            ("Script Console".to_owned(), true, None, false),
            ("Run Script…".to_owned(), true, None, false),
        ]
    );
    let action = system.leaves()[0].action;
    a.menu_action(&ctx, action);
    assert!(a.palette_open(crate::form_editor::PALETTE));
    // UIC-183: a floating window, listed with the palettes, ticked.
    assert!(a.palettes.find("formEditor").is_some());
    assert!(menu(&a, &ctx, "System").leaves()[0].checked);
    frames(&mut a, &ctx, 2);
    a.menu_action(&ctx, action);
    assert!(!a.palette_open(crate::form_editor::PALETTE));
}

// ------------------------------------------------- the tree

#[test]
fn lay83_tree_groups_forms_and_search_filters_them() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    let tree = a.form_tree();
    let group = |name: &str| {
        tree.iter()
            .find(|(g, _)| g == name)
            .map(|(_, ids)| ids.clone())
    };
    assert!(group("Toolbox").unwrap().contains(&"toolbox.uv".to_owned()));
    assert!(
        group("Toolbars")
            .unwrap()
            .contains(&"toolbar.workbench".to_owned())
    );
    assert!(group("Menus").unwrap().contains(&"menu.help".to_owned()));
    // Search: by id, label, control label or command line.
    a.form_editor.search = "report".into();
    let found: Vec<String> = a.form_tree().into_iter().flat_map(|(_, ids)| ids).collect();
    assert_eq!(found, ["menu.help"]);
    a.form_editor.search = "twist bevel".into();
    let found: Vec<String> = a.form_tree().into_iter().flat_map(|(_, ids)| ids).collect();
    assert!(found.contains(&"toolbox.polygon".to_owned()), "{found:?}");
    assert!(!found.contains(&"menu.help".to_owned()));
    // Lines inside a form's own rows count too (Create's Cube row).
    a.form_editor.search = "unit cube".into();
    let found: Vec<String> = a.form_tree().into_iter().flat_map(|(_, ids)| ids).collect();
    assert!(found.contains(&"toolbox.create".to_owned()), "{found:?}");
    a.form_editor.search.clear();
    // Drawn in its window: a row for the form and each of its controls.
    a.execute("operandi.form.select form:menu.help");
    frames(&mut a, &ctx, 3);
    assert!(
        a.form_editor.rects.contains_key("menu.help"),
        "{:?}",
        a.form_editor.rects.keys()
    );
    assert!(a.form_editor.rects.contains_key("menu.help#1"));
}

#[test]
fn lay83_new_group_new_form_and_its_group_persist() {
    let root = data_root();
    {
        let (mut a, _ctx) = app(&root);
        a.execute("operandi.form.newGroup {My Forms}");
        assert!(
            a.form_tree()
                .iter()
                .any(|(g, ids)| g == "My Forms" && ids.is_empty())
        );
        a.execute("operandi.form.new id:my.tools label:{My Tools} group:{My Forms}");
        assert_eq!(a.form_group("my.tools"), "My Forms");
        assert_eq!(a.form_editor.selected, Some(("my.tools".into(), None)));
        a.execute("operandi.form.add line:{Cube}");
        a.execute("operandi.form.addDivider label:More");
        a.execute("operandi.form.addForm sub:menu.help");
        let f = form(&a, "my.tools");
        assert_eq!(f.label, "My Tools");
        assert_eq!(f.controls.len(), 3);
        assert_eq!(f.controls[0].command.as_deref(), Some("Cube"));
        assert_eq!(f.controls[1].divider.as_deref(), Some("More"));
        assert_eq!(
            f.controls[2].form,
            Some(cf::FormRef::Id("menu.help".into()))
        );
        // A duplicate id or a form holding itself is refused.
        a.execute("operandi.form.new id:my.tools");
        assert!(a.status.contains("already"), "{}", a.status);
        a.execute("operandi.form.addForm sub:my.tools");
        assert!(a.status.contains("itself"), "{}", a.status);
    }
    // Relaunched: the form and the group are still there.
    let (a, _ctx) = app(&root);
    assert_eq!(form(&a, "my.tools").controls.len(), 3);
    assert_eq!(a.form_group("my.tools"), "My Forms");
}

// ------------------------------------------------- properties

#[test]
fn lay83_control_and_form_properties_edit_the_users_copy_and_revert() {
    let root = data_root();
    let (mut a, _ctx) = app(&root);
    let shipped = form(&a, "menu.help");
    a.execute("operandi.form.select form:menu.help index:2");
    a.execute("operandi.form.set attr:label value:{Tell Us} index:2");
    a.execute("operandi.form.set attr:tooltip value:{Opens an issue} index:2");
    a.execute("operandi.form.set attr:width value:wide index:2");
    a.execute("operandi.form.set attr:level value:advanced index:2");
    a.execute("operandi.form.set attr:label value:{Help!}");
    let f = form(&a, "menu.help");
    assert_eq!(f.controls[2].label, "Tell Us");
    assert_eq!(f.controls[2].tooltip, "Opens an issue");
    assert_eq!(f.controls[2].width, cf::Width::Wide);
    assert_eq!(f.label, "Help!");
    assert!(a.forms.is_changed("menu.help"));
    // LAY-87: the user's copy sits over the shipped file.
    assert!(
        root.join("Layouts")
            .join("Forms")
            .join("menu.help.json")
            .is_file()
    );
    assert_eq!(a.forms.shipped("menu.help"), Some(&shipped));
    // The converted menu shows the edit.
    let ctx = Context::default();
    assert!(
        menu(&a, &ctx, "Help")
            .leaves()
            .iter()
            .any(|i| i.title == "Tell Us")
    );
    // Undo takes one edit back; Revert all of them.
    a.execute("operandi.workbench.undo form:menu.help");
    assert_eq!(form(&a, "menu.help").label, shipped.label);
    a.execute("operandi.form.revert menu.help");
    assert_eq!(form(&a, "menu.help"), shipped);
    assert!(!a.forms.is_changed("menu.help"));
    // A bad value only reports.
    a.execute("operandi.form.set attr:width value:huge index:1 form:menu.help");
    assert!(a.status.contains("isn't one of"), "{}", a.status);
}

/// LAY-Q53: Customizable Workbench is a Form Editor checkbox; a form with
/// it set can be edited in place like the shipped workbenches.
#[test]
fn lay_q53_customizable_workbench_is_a_form_property() {
    let root = data_root();
    let (mut a, _ctx) = app(&root);
    a.execute("operandi.form.new id:bench.two");
    a.execute("operandi.workbench.edit on:true form:bench.two");
    assert!(a.status.contains("isn't a workbench"), "{}", a.status);
    a.execute("operandi.form.set attr:workbench value:true form:bench.two");
    assert!(form(&a, "bench.two").workbench);
    a.execute("operandi.workbench.edit on:true form:bench.two");
    assert_eq!(a.workbench.editing.as_deref(), Some("bench.two"));
}

/// UIC-214 / UIC-215: head/tail slots and show-only-when are properties.
#[test]
fn uic214_slots_and_show_when_are_properties() {
    let root = data_root();
    let (mut a, _ctx) = app(&root);
    a.execute("operandi.form.new id:my.head");
    a.execute("operandi.form.add line:{Report Bug} form:my.head");
    a.execute("operandi.form.set attr:slot value:menu.help form:my.head");
    a.execute("operandi.form.set attr:slotAt value:head form:my.head");
    a.execute("operandi.form.set attr:slotSort value:3 form:my.head");
    let slot = form(&a, "my.head").slot.unwrap();
    assert_eq!(
        (slot.form.as_str(), slot.at, slot.sort),
        ("menu.help", End::Head, 3)
    );
    let slotted: Vec<&str> = a
        .forms
        .slotted("menu.help", End::Head)
        .iter()
        .map(|f| f.id.as_str())
        .collect();
    assert_eq!(slotted, ["my.head"]);
    a.execute("operandi.form.set attr:showWhen value:{Open UV Editor} form:my.head");
    assert_eq!(
        form(&a, "my.head").show_when.as_deref(),
        Some("Open UV Editor")
    );
    a.execute("operandi.form.set attr:slotAt value:middle form:my.head");
    assert!(a.status.contains("isn't one of"), "{}", a.status);
}

// ------------------------------------------------- reorder, preview, save

#[test]
fn lay83_reorder_moves_a_control_and_the_selection_follows() {
    let root = data_root();
    let (mut a, _ctx) = app(&root);
    a.execute("operandi.form.select form:menu.help index:0");
    a.execute("operandi.form.move from:0 to:1");
    let f = form(&a, "menu.help");
    assert_eq!(f.controls[0].label, "What's This?");
    assert_eq!(f.controls[1].label, "User Guide");
    assert_eq!(a.form_editor.selected, Some(("menu.help".into(), Some(1))));
    a.execute("operandi.form.move from:0 to:9");
    assert!(a.status.contains("controls"), "{}", a.status);
    a.execute("operandi.form.delete index:0");
    // User Guide, Report Bug and Check for Updates are left.
    assert_eq!(form(&a, "menu.help").controls.len(), 3);
}

#[test]
fn lay83_drag_in_the_tree_reorders() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.execute("operandi.form.select form:menu.help index:0");
    frames(&mut a, &ctx, 3);
    let from = a.form_editor.rects["menu.help#0"].center();
    let to = a.form_editor.rects["menu.help#1"].center();
    let size = crate::layouts_tests::SIZE;
    crate::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(from)]);
    crate::tests::draw(
        &mut a,
        &ctx,
        size,
        vec![crate::tests::pointer(from, true, Modifiers::NONE)],
    );
    for t in 1..=6 {
        let p = from + (to - from) * (t as f32 / 6.);
        crate::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(p)]);
    }
    crate::tests::draw(
        &mut a,
        &ctx,
        size,
        vec![crate::tests::pointer(to, false, Modifiers::NONE)],
    );
    frames(&mut a, &ctx, 1);
    assert_eq!(
        form(&a, "menu.help").controls[0].label,
        "What's This?",
        "{}",
        a.status
    );
}

#[test]
fn lay83_preview_and_save_form_to_its_own_file() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.execute("operandi.form.preview");
    assert!(a.status.contains("Select a form"), "{}", a.status);
    a.execute("operandi.form.preview form:toolbox.uv");
    assert!(a.form_editor.preview);
    a.execute("layout.window hash:formEditor open:true");
    frames(&mut a, &ctx, 3);
    // The preview draws the form's buttons.
    assert!(!a.form_rects.is_empty());
    a.execute("operandi.form.preview on:false");
    assert!(!a.form_editor.preview);
    // Save Form: to a named file, else the Saved Forms folder.
    let file = root.join("exported-uv.json");
    a.execute(&format!(
        "operandi.form.export form:toolbox.uv filename:{{{}}}",
        file.display()
    ));
    let text = std::fs::read_to_string(&file).unwrap();
    assert_eq!(Form::parse(&text).unwrap(), form(&a, "toolbox.uv"));
    a.execute("operandi.form.export form:menu.help");
    assert!(
        root.join("Saved Forms").join("menu.help.json").is_file(),
        "{}",
        a.status
    );
}

// ------------------------------------------------- Find Form, Show in Form Editor

#[test]
fn lay83_find_form_jumps_to_the_clicked_buttons_form_without_running_it() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.category = toolbox::Category::Create;
    frames(&mut a, &ctx, 2);
    a.execute("operandi.form.find on:true");
    assert!(a.form_editor.finding);
    let at = button_by_line(&a, "Cube").center();
    let size = crate::layouts_tests::SIZE;
    crate::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(at)]);
    crate::tests::draw(
        &mut a,
        &ctx,
        size,
        vec![
            crate::tests::pointer(at, true, Modifiers::NONE),
            crate::tests::pointer(at, false, Modifiers::NONE),
        ],
    );
    assert!(!a.form_editor.finding);
    let (id, index) = a.form_editor.selected.clone().unwrap();
    assert_eq!(id, "toolbox.create");
    let c = &form(&a, &id).controls[index.unwrap()];
    assert!(c.form.is_some(), "the row holding Cube");
    assert!(
        a.editor.session.live.is_none() && a.editor.session.shape.is_none(),
        "Cube didn't start"
    );
    assert!(a.palette_open(crate::form_editor::PALETTE));
    // Esc stops it.
    a.execute("operandi.form.find on:true");
    let esc = Event::Key {
        key: Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    crate::tests::draw(&mut a, &ctx, crate::layouts_tests::SIZE, vec![esc]);
    assert!(!a.form_editor.finding);
}

#[test]
fn lay83_show_in_form_editor_selects_the_workbench_item() {
    let root = data_root();
    let (mut a, _ctx) = app(&root);
    a.execute("operandi.workbench.add line:{Cube} form:toolbox.workbench");
    a.execute("operandi.form.select form:toolbox.workbench index:0");
    assert_eq!(
        a.form_editor.selected,
        Some(("toolbox.workbench".into(), Some(0)))
    );
    assert!(a.palette_open(crate::form_editor::PALETTE));
    a.execute("operandi.form.select form:nothing.here");
    assert!(a.status.contains("No form"), "{}", a.status);
}

#[test]
fn uic160_commands_can_be_taken_from_the_command_history() {
    let root = data_root();
    let (mut a, ctx) = app(&root);
    a.execute("Open Render Window");
    a.execute("operandi.form.new id:from.history");
    a.execute("layout.window hash:formEditor open:true");
    frames(&mut a, &ctx, 2);
    // The History menu lists recent lines; picking one fills the field.
    assert!(
        a.history
            .log
            .records
            .iter()
            .any(|r| r.line == "Open Render Window")
    );
    a.form_editor.add_line = "Open Render Window".into();
    let line = std::mem::take(&mut a.form_editor.add_line);
    a.execute(&format!("operandi.form.add line:{{{line}}}"));
    assert_eq!(
        form(&a, "from.history").controls[0].command.as_deref(),
        Some("Open Render Window")
    );
}
