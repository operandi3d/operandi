//! Control icons and Pick (form_icons.rs, UIC-314, UIC-212's Pick):
//! picking a button's line and icon, Esc / right-click stopping it, built-in
//! icons by name, custom files copied into the data folder (PNG and SVG),
//! missing files drawn as a placeholder, and icons travelling inside
//! exported pies.
use super::*;
use crate::form::command_form::Style;
use crate::pie_menu::{self, SlotIcon};
use crate::tests::draw;
use std::path::Path;

const SIZE: Vec2 = vec2(1200., 800.);

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "operandi-icons-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn png(path: &Path, shade: u8) {
    image::RgbaImage::from_pixel(8, 8, image::Rgba([shade, 40, 40, 255]))
        .save(path)
        .unwrap();
}

fn pie_app() -> Operandi {
    let mut a = Operandi::empty();
    a.execute("operandi.form.new id:my.pie label:Mine style:pie");
    assert!(a.forms.get("my.pie").is_some(), "{}", a.status);
    a
}

fn slot(a: &Operandi, i: usize) -> crate::form::command_form::Control {
    a.forms.get("my.pie").unwrap().controls[i].clone()
}

/// Pick: the next button's line and icon (and label) fill the slot, and
/// the button doesn't run; Pick is then off.
#[test]
fn pick_fills_a_slot_from_the_next_button_clicked() {
    let mut a = pie_app();
    a.execute("operandi.form.pick index:2 form:my.pie");
    assert!(a.form_picking(), "{}", a.status);
    // A click on a toolbox button runs its line through `execute`.
    a.workbench.click_now = true;
    a.execute("Move");
    assert!(a.editor.live.is_none(), "the picked button didn't run");
    assert!(!a.form_picking(), "one pick, then off");
    let c = slot(&a, 2);
    assert_eq!(c.command.as_deref(), Some("Move"));
    assert!(!c.label.is_empty(), "an empty label takes the button's");
    assert!(c.icon.is_some(), "and its icon");
    // A slot with a label keeps it.
    a.execute("operandi.form.set attr:label value:Go index:2 form:my.pie");
    a.execute("operandi.form.pick index:2 form:my.pie");
    a.workbench.click_now = true;
    a.execute("Rotate");
    let c = slot(&a, 2);
    assert_eq!(c.command.as_deref(), Some("Rotate"));
    assert_eq!(c.label, "Go");
    // One Undo Edit takes the pick back.
    a.execute("operandi.workbench.undo form:my.pie");
    assert_eq!(slot(&a, 2).command.as_deref(), Some("Move"));
}

/// While picking, a key's line (no click) still runs, and the Form
/// Editor's own lines run too; Esc and a right-click stop picking.
#[test]
fn pick_takes_only_clicks_and_stops_on_esc_or_right_click() {
    let mut a = pie_app();
    let ctx = Context::default();
    a.execute("operandi.form.pick index:0 form:my.pie");
    a.workbench.click_now = true;
    a.execute("operandi.form.slot index:5 line:{View Top} form:my.pie");
    assert_eq!(slot(&a, 5).command.as_deref(), Some("View Top"));
    assert!(a.form_picking(), "the editor's own line ran, still picking");
    // The pointer is the system's pointing hand while picking.
    let out = draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(out.platform_output.cursor_icon, CursorIcon::PointingHand);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(!a.form_picking(), "Esc stops");
    a.execute("operandi.form.pick index:0 form:my.pie");
    let at = pos2(600., 400.);
    for pressed in [true, false] {
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::PointerButton {
                pos: at,
                button: PointerButton::Secondary,
                pressed,
                modifiers: Modifiers::NONE,
            }],
        );
    }
    assert!(!a.form_picking(), "a right-click stops");
    assert!(slot(&a, 0).command.is_none(), "nothing was picked");
}

/// Pick works on any form's button control, not only pies; a control
/// that isn't a button is refused.
#[test]
fn pick_fills_a_toolbox_forms_button_too() {
    let mut a = Operandi::empty();
    a.execute("operandi.form.new id:my.tools");
    a.execute("operandi.form.add line:{View Top} form:my.tools");
    a.execute("operandi.form.addDivider form:my.tools");
    a.execute("operandi.form.pick index:0 form:my.tools");
    a.workbench.click_now = true;
    a.execute("Scale");
    let c = a.forms.get("my.tools").unwrap().controls[0].clone();
    assert_eq!(c.command.as_deref(), Some("Scale"));
    a.execute("operandi.form.pick index:1 form:my.tools");
    a.workbench.click_now = true;
    a.execute("Scale");
    assert!(a.status.contains("isn't a button"), "{}", a.status);
}

/// UIC-314: a built-in icon by name, a bad name refused, Clear empties.
#[test]
fn built_in_icons_by_name_and_clear() {
    let mut a = pie_app();
    a.execute("operandi.form.slot index:0 line:Move form:my.pie");
    a.execute("operandi.form.setIcon index:0 name:action-center form:my.pie");
    assert_eq!(slot(&a, 0).icon.as_deref(), Some("action-center"));
    a.execute("operandi.form.setIcon index:0 name:no-such-icon form:my.pie");
    assert_eq!(a.status, "No built-in icon called no-such-icon");
    a.execute("operandi.form.setIcon index:0 form:my.pie");
    assert!(slot(&a, 0).icon.is_none() && slot(&a, 0).icon_file.is_none());
    // Every icon Browse Icons offers is a built-in name.
    assert!(
        crate::rendered_icons::RENDERED
            .iter()
            .all(|(icon, _)| icons::Icon::from_slug(icon.slug()) == Some(*icon))
    );
}

/// UIC-314: a custom file is copied into Layouts/Icons; the same file
/// again reuses the copy, another file of the same name gets its own; an
/// SVG loads too; a file that isn't a picture is refused.
#[test]
fn custom_files_are_copied_into_the_data_folder() {
    let mut a = pie_app();
    let dir = temp_dir("copy");
    let first = dir.join("star.png");
    png(&first, 200);
    a.execute("operandi.form.slot index:0 line:Move form:my.pie");
    a.execute(&format!(
        "operandi.form.setIcon index:0 file:{{{}}} form:my.pie",
        first.display()
    ));
    let kept = slot(&a, 0).icon_file.expect("a custom icon");
    assert!(
        Path::new(&kept).starts_with(a.icons_folder()),
        "{kept} is in the icons folder"
    );
    assert!(
        slot(&a, 0).icon.is_none(),
        "the file replaces a built-in icon"
    );
    std::fs::remove_file(&first).unwrap();
    assert!(
        Path::new(&kept).exists(),
        "the copy stays when the original goes"
    );
    // The same picture again: the same copy. Another picture, same name:
    // a copy of its own.
    png(&first, 200);
    a.execute(&format!(
        "operandi.form.setIcon index:1 file:{{{}}} form:my.pie",
        first.display()
    ));
    assert_eq!(slot(&a, 1).icon_file.as_deref(), Some(kept.as_str()));
    png(&first, 90);
    a.execute(&format!(
        "operandi.form.setIcon index:2 file:{{{}}} form:my.pie",
        first.display()
    ));
    let other = slot(&a, 2).icon_file.unwrap();
    assert_ne!(other, kept);
    assert!(other.ends_with("star-2.png"), "{other}");
    // SVG.
    let svg = dir.join("dot.svg");
    std::fs::write(
        &svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><circle cx="8" cy="8" r="6" fill="red"/></svg>"#,
    )
    .unwrap();
    assert!(crate::workbench::custom_icon_picture(&svg).is_some());
    a.execute(&format!(
        "operandi.form.setIcon index:3 file:{{{}}} form:my.pie",
        svg.display()
    ));
    assert!(slot(&a, 3).icon_file.unwrap().ends_with("dot.svg"));
    // Not a picture.
    let text = dir.join("notes.png");
    std::fs::write(&text, "hello").unwrap();
    a.execute(&format!(
        "operandi.form.setIcon index:4 file:{{{}}} form:my.pie",
        text.display()
    ));
    assert!(a.status.contains("isn't a PNG"), "{}", a.status);
    let _ = std::fs::remove_dir_all(&dir);
}

/// UIC-314: a slot whose icon file is gone draws a placeholder; opening
/// the pie and drawing it doesn't fail.
#[test]
fn a_missing_icon_file_draws_a_placeholder() {
    let mut a = pie_app();
    let ctx = Context::default();
    a.execute("operandi.form.slot index:0 line:Move label:Move form:my.pie");
    let mut form = a.forms.get("my.pie").unwrap().clone();
    form.controls[0].icon_file = Some("/no/such/icon.png".into());
    a.forms.save(form).unwrap();
    let pie = a.pie_named("my.pie").unwrap();
    assert_eq!(
        pie.shows(0).0,
        Some(SlotIcon::File("/no/such/icon.png".into()))
    );
    assert_eq!(a.slot_pictures(&ctx, &pie)[0], None, "placeholder");
    a.execute("operandi.pie.open my.pie");
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(pos2(600., 400.))],
    );
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.pie.is_some());
}

/// UIC-312/314: Export Pie carries the custom icon files; Import Pie puts
/// them in the importer's icons folder and the slots wear them.
#[test]
fn exported_pies_carry_their_custom_icons() {
    let dir = temp_dir("export");
    let picture = dir.join("badge.png");
    png(&picture, 180);
    let mut a = pie_app();
    a.execute("operandi.form.slot index:0 line:Move label:Move form:my.pie");
    a.execute(&format!(
        "operandi.form.setIcon index:0 file:{{{}}} form:my.pie",
        picture.display()
    ));
    let file = dir.join("mine.oppie");
    a.execute(&format!(
        "operandi.pie.export my.pie filename:{{{}}}",
        file.display()
    ));
    let read = crate::pie_forms::read_pie_file(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(read.icons.keys().collect::<Vec<_>>(), ["badge.png"]);
    assert_eq!(
        read.forms[0].controls[0].icon_file.as_deref(),
        Some("embedded:badge.png")
    );
    let mut b = Operandi::empty();
    b.execute(&format!("operandi.pie.import {{{}}}", file.display()));
    assert_eq!(b.status, "Imported pie my.pie");
    let worn = b.forms.get("my.pie").unwrap().controls[0]
        .icon_file
        .clone()
        .unwrap();
    assert!(Path::new(&worn).starts_with(b.icons_folder()), "{worn}");
    assert_eq!(
        std::fs::read(&worn).unwrap(),
        std::fs::read(&picture).unwrap()
    );
    assert_eq!(b.forms.get("my.pie").unwrap().style, Style::Pie);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn base64_round_trips() {
    for bytes in [&b""[..], b"a", b"ab", b"abc", b"abcd", &[0, 255, 128, 7, 9]] {
        let text = crate::form_icons::base64_encode(bytes);
        assert_eq!(crate::form_icons::base64_decode(&text).unwrap(), bytes);
    }
    assert_eq!(crate::form_icons::base64_encode(b"Man"), "TWFu");
    assert_eq!(crate::form_icons::base64_encode(b"Ma"), "TWE=");
    assert!(crate::form_icons::base64_decode("**").is_none());
}

#[test]
fn pick_is_lit_while_on_for_its_control() {
    let mut a = pie_app();
    a.execute("operandi.form.pick index:3 form:my.pie");
    assert!(
        a.line_state("operandi.form.pick index:3 form:my.pie")
            .unwrap()
            .on
    );
    assert!(
        !a.line_state("operandi.form.pick index:4 form:my.pie")
            .unwrap()
            .on
    );
    a.execute("operandi.form.pick index:3 form:my.pie");
    assert!(!a.form_picking(), "the same line again turns it off");
    let _ = pie_menu::SLOTS;
}
