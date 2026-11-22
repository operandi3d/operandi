//! Light properties Tier 1 (specifications/light-properties-2026-11-06):
//! LP-10 Render, LP-11 Dissolve, LP-19..21 Visible rows, LM-12/13 Affect
//! switches, LM-16 Set Temperature, the Portal kind (LP-PO), the panel's
//! undo names, and the shared Display/Assembly settings of every item
//! (CP-D01, CP-D06, CP-A01, CP-A02, IC-1..3).
use glam::DVec3;
use operandi_core::camera_light::*;
use operandi_core::item_common::*;
use operandi_core::*;

fn new_scene() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.add_default_camera_and_light().unwrap();
    e.next_item_id = e.scene.next_id;
    e
}

fn item(e: &Editor, id: u64) -> &Item {
    e.scene.items.iter().find(|i| i.id == id).unwrap()
}

fn light(e: &Editor, id: u64) -> &LightSettings {
    item(e, id).light.as_ref().unwrap()
}

fn new_light(e: &mut Editor, kind: LightKind) -> u64 {
    e.dispatch(Command::NewLight(kind)).unwrap();
    e.scene.selection.item.unwrap()
}

fn last_label(e: &Editor) -> &str {
    &e.undo.last().unwrap().label
}

fn round_trip(scene: &Scene) -> Scene {
    persistence::decode(&persistence::encode(scene).unwrap()).unwrap()
}

#[test]
fn render_rule_default_yes_no() {
    // LP-10: Default follows the eye, Yes always, No never.
    let mut l = LightSettings::default();
    assert_eq!(l.render, LightRender::Default);
    assert!(l.renders(true) && !l.renders(false));
    l.apply(LightEdit::Render(LightRender::On)).unwrap();
    assert!(l.renders(true) && l.renders(false));
    l.apply(LightEdit::Render(LightRender::Off)).unwrap();
    assert!(!l.renders(true) && !l.renders(false));
    let labels: Vec<_> = LightRender::ALL.iter().map(|r| r.label()).collect();
    assert_eq!(labels, ["Default", "Yes", "No"]);
}

#[test]
fn each_light_edit_is_one_undo_step_named_after_its_row() {
    let mut e = new_scene();
    let id = new_light(&mut e, LightKind::Spot);
    let steps = e.undo.len();
    for (edit, name) in [
        (LightEdit::Render(LightRender::Off), "Render"),
        (LightEdit::Dissolve(0.25), "Dissolve"),
        (LightEdit::VisibleToCamera(true), "Visible to Camera"),
        (
            LightEdit::VisibleToReflection(true),
            "Visible to Reflection Rays",
        ),
        (
            LightEdit::VisibleToRefraction(true),
            "Visible to Refraction Rays",
        ),
        (LightEdit::AffectDiffuse(0.0), "Affect Diffuse"),
        (LightEdit::AffectSpecular(0.0), "Affect Specular"),
        (LightEdit::Temperature(3200.0), "Set Temperature"),
    ] {
        e.dispatch(Command::EditLight { item: id, edit }).unwrap();
        assert_eq!(last_label(&e), name);
    }
    assert_eq!(e.undo.len(), steps + 8);
    let l = light(&e, id);
    assert!(l.stored.visible_to_reflection && l.stored.visible_to_refraction);
    assert_eq!((l.affect_diffuse, l.affect_specular), (0.0, 0.0));
    for _ in 0..8 {
        e.dispatch(Command::Undo).unwrap();
    }
    assert_eq!(light(&e, id), &LightSettings::new(LightKind::Spot));
    // Invalid values change nothing and add no step.
    assert!(
        e.dispatch(Command::EditLight {
            item: id,
            edit: LightEdit::Dissolve(1.5),
        })
        .is_err()
    );
    assert_eq!(e.undo.len(), steps);
}

#[test]
fn light_edits_apply_to_every_selected_light() {
    // LP-51: edits apply to all selected lights, as one step.
    let mut e = new_scene();
    let a = new_light(&mut e, LightKind::Point);
    let b = new_light(&mut e, LightKind::Area);
    let mesh = e.scene.items[0].id;
    e.dispatch(Command::SelectItem(Some(a))).unwrap();
    e.dispatch(Command::ToggleItemSelection(b)).unwrap();
    e.dispatch(Command::ToggleItemSelection(mesh)).unwrap();
    let steps = e.undo.len();
    e.dispatch(Command::EditLight {
        item: a,
        edit: LightEdit::Dissolve(0.5),
    })
    .unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    assert_eq!(light(&e, a).dissolve, 0.5);
    assert_eq!(light(&e, b).dissolve, 0.5);
    // The other default light wasn't selected.
    let other = e.scene.items[2].id;
    assert_eq!(light(&e, other).dissolve, 0.0);
}

#[test]
fn kelvin_to_rgb_follows_the_black_body() {
    // 6600 K is white; warm below, blue above; brightest channel 1.
    let white = kelvin_to_rgb(6600.0);
    assert!(white.iter().all(|c| (c - 1.0).abs() < 0.03), "{white:?}");
    let candle = kelvin_to_rgb(1500.0);
    assert_eq!(candle[0], 1.0);
    assert!(candle[0] > candle[1] && candle[1] > candle[2], "{candle:?}");
    let tungsten = kelvin_to_rgb(3200.0);
    assert!(tungsten[1] > candle[1] && tungsten[2] > candle[2]);
    let sky = kelvin_to_rgb(10000.0);
    assert_eq!(sky[2], 1.0);
    assert!(sky[0] < sky[1] && sky[1] < 1.0, "{sky:?}");
    for k in [
        1000.0, 2000.0, 4000.0, 5500.0, 6500.0, 8000.0, 20000.0, 40000.0,
    ] {
        let c = kelvin_to_rgb(k);
        assert!(c.iter().all(|v| (0.0..=1.0).contains(v)));
        assert!((c.iter().cloned().fold(0.0, f64::max) - 1.0).abs() < 1e-12);
    }
    // Warmer means redder relative to blue, all the way along.
    let mut last = f64::INFINITY;
    for k in (10..=120).map(|k| k as f64 * 100.0) {
        let c = kelvin_to_rgb(k);
        let ratio = c[0] / c[2].max(1e-9);
        assert!(ratio <= last + 1e-9, "{k}");
        last = ratio;
    }
}

#[test]
fn set_temperature_sets_the_colour_once() {
    // LM-16 / LPQ-06: one-shot, stores RGB only.
    let mut l = LightSettings::default();
    l.apply(LightEdit::Temperature(2700.0)).unwrap();
    assert_eq!(l.color, kelvin_to_rgb(2700.0));
    assert!(l.apply(LightEdit::Temperature(500.0)).is_err());
    assert!(l.apply(LightEdit::Temperature(f64::NAN)).is_err());
    assert_eq!(l.color, kelvin_to_rgb(2700.0));
}

#[test]
fn portal_defaults_rows_and_name() {
    // LP-PO-02..04: Rectangle 1 × 1 m, Environment Only off.
    let mut e = new_scene();
    let id = new_light(&mut e, LightKind::Portal);
    assert_eq!(item(&e, id).name, "Portal");
    assert_eq!(last_label(&e), "New Light");
    let p = light(&e, id);
    assert_eq!(p.shape, AreaShape::Rectangular);
    assert_eq!((p.width, p.height), (1.0, 1.0));
    assert!(!p.environment_only);
    assert!(!LightKind::Portal.emits() && LightKind::Portal.has_area());
    assert_eq!(LightKind::Portal.tab_name(), "Portal");
    assert_eq!(LightKind::Directional.tab_name(), "Directional Light");
    e.dispatch(Command::EditLight {
        item: id,
        edit: LightEdit::EnvironmentOnly(true),
    })
    .unwrap();
    assert_eq!(last_label(&e), "Environment Only");
    e.dispatch(Command::EditLight {
        item: id,
        edit: LightEdit::Shape(AreaShape::Elliptical),
    })
    .unwrap();
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(light(&e, id), &LightSettings::new(LightKind::Portal));
    // Undoing the creation removes it.
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.items.iter().all(|i| i.id != id));
}

#[test]
fn portal_and_item_settings_save_and_reopen() {
    // IC-3 and the portal: everything comes back; the file is version 16.
    let mut e = new_scene();
    let id = new_light(&mut e, LightKind::Portal);
    e.dispatch(Command::EditLight {
        item: id,
        edit: LightEdit::Width(2.5),
    })
    .unwrap();
    e.dispatch(Command::EditLight {
        item: id,
        edit: LightEdit::EnvironmentOnly(true),
    })
    .unwrap();
    let mesh = e.scene.items[0].id;
    for edit in [
        ItemCommonEdit::Visible(DisplayVisible::NoHideChildren),
        ItemCommonEdit::Label("Hero".into()),
        ItemCommonEdit::ShowLabel(false),
        ItemCommonEdit::LabelOffsetX(12.0),
        ItemCommonEdit::LabelOffsetY(-4.0),
        ItemCommonEdit::Select(Choice::No),
        ItemCommonEdit::Lock(Choice::Yes),
    ] {
        e.dispatch(Command::EditItemCommon {
            items: vec![mesh],
            edit,
        })
        .unwrap();
    }
    let bytes = persistence::encode(&e.scene).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("\"version\": 33"), "{}", &text[..80]);
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back, e.scene);
    // An older version can't hold them.
    let old = text.replacen("\"version\": 33", "\"version\": 15", 1);
    assert!(persistence::decode(old.as_bytes()).is_err());
    // A scene without them writes no "common" (it is version 33 for its
    // Environment item, environments-2026-11-09).
    let plain = new_scene();
    let text = String::from_utf8(persistence::encode(&plain.scene).unwrap()).unwrap();
    assert!(text.contains("\"version\": 33"));
    assert!(!text.contains("\"common\""));
}

#[test]
fn light_rows_round_trip() {
    let mut e = new_scene();
    let id = new_light(&mut e, LightKind::Spot);
    for edit in [
        LightEdit::Render(LightRender::On),
        LightEdit::Dissolve(0.3),
        LightEdit::VisibleToCamera(true),
        LightEdit::VisibleToReflection(true),
        LightEdit::VisibleToRefraction(true),
        LightEdit::AffectSpecular(0.0),
        LightEdit::Temperature(4500.0),
    ] {
        e.dispatch(Command::EditLight { item: id, edit }).unwrap();
    }
    assert_eq!(round_trip(&e.scene), e.scene);
}

#[test]
fn common_edits_are_named_steps_over_every_selected_item() {
    // IC-1, IC-2: one step named after the property; all selected items.
    let mut e = new_scene();
    let ids: Vec<u64> = e.scene.items.iter().map(|i| i.id).collect();
    let steps = e.undo.len();
    e.dispatch(Command::EditItemCommon {
        items: ids.clone(),
        edit: ItemCommonEdit::Visible(DisplayVisible::Yes),
    })
    .unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    assert_eq!(last_label(&e), "Visible");
    assert!(
        e.scene
            .items
            .iter()
            .all(|i| i.common.display.visible == DisplayVisible::Yes)
    );
    for (edit, name) in [
        (ItemCommonEdit::Label("A".into()), "Label"),
        (ItemCommonEdit::ShowLabel(false), "Show Label"),
        (ItemCommonEdit::LabelOffsetX(3.0), "Label Offset X"),
        (ItemCommonEdit::LabelOffsetY(3.0), "Label Offset Y"),
        (ItemCommonEdit::Select(Choice::No), "Select"),
        (ItemCommonEdit::Lock(Choice::Yes), "Lock"),
    ] {
        e.dispatch(Command::EditItemCommon {
            items: ids.clone(),
            edit,
        })
        .unwrap();
        assert_eq!(last_label(&e), name);
    }
    for _ in 0..7 {
        e.dispatch(Command::Undo).unwrap();
    }
    assert!(e.scene.items.iter().all(|i| i.common.is_default()));
    // Invalid values: nothing changes, no step.
    let steps = e.undo.len();
    assert!(
        e.dispatch(Command::EditItemCommon {
            items: ids.clone(),
            edit: ItemCommonEdit::LabelOffsetX(f64::INFINITY),
        })
        .is_err()
    );
    assert!(
        e.dispatch(Command::EditItemCommon {
            items: vec![],
            edit: ItemCommonEdit::Lock(Choice::Yes),
        })
        .is_err()
    );
    assert_eq!(e.undo.len(), steps);
}

#[test]
fn display_visible_rules() {
    // CP-D01 Default follows the kind toggle; Yes always; No never; No
    // (Hide Children) hides the children too; the eye always hides.
    let mut e = new_scene();
    let mesh = e.scene.items[0].id;
    let light = e.scene.items[2].id;
    e.scene.set_parent(light, Some(mesh), false).unwrap();
    let set = |e: &mut Editor, id: u64, v: DisplayVisible| {
        e.dispatch(Command::EditItemCommon {
            items: vec![id],
            edit: ItemCommonEdit::Visible(v),
        })
        .unwrap();
    };
    assert!(e.scene.drawn_in_viewport(light, true));
    assert!(!e.scene.drawn_in_viewport(light, false));
    assert!(e.scene.display_hidden().is_empty());
    set(&mut e, light, DisplayVisible::Yes);
    assert!(e.scene.drawn_in_viewport(light, false));
    set(&mut e, light, DisplayVisible::No);
    assert!(!e.scene.drawn_in_viewport(light, true));
    assert_eq!(e.scene.display_hidden(), [light].into());
    set(&mut e, light, DisplayVisible::Yes);
    set(&mut e, mesh, DisplayVisible::NoHideChildren);
    assert!(!e.scene.drawn_in_viewport(mesh, true));
    assert!(!e.scene.drawn_in_viewport(light, true), "child hidden too");
    assert_eq!(e.scene.display_hidden(), [mesh, light].into());
    set(&mut e, mesh, DisplayVisible::No);
    assert!(e.scene.drawn_in_viewport(light, true), "No keeps children");
    // The eye hides whatever Visible says.
    e.scene.items[2].visible = false;
    assert!(!e.scene.drawn_in_viewport(light, true));
}

#[test]
fn labels_show_only_when_typed_and_switched_on() {
    let mut e = new_scene();
    let id = e.scene.items[1].id;
    assert_eq!(item(&e, id).shown_label(), None);
    let edit = |e: &mut Editor, edit| {
        e.dispatch(Command::EditItemCommon {
            items: vec![id],
            edit,
        })
        .unwrap()
    };
    edit(&mut e, ItemCommonEdit::Label("Key cam".into()));
    assert_eq!(item(&e, id).shown_label(), Some("Key cam"));
    edit(&mut e, ItemCommonEdit::ShowLabel(false));
    assert_eq!(item(&e, id).shown_label(), None);
    edit(&mut e, ItemCommonEdit::ShowLabel(true));
    edit(&mut e, ItemCommonEdit::Label("   ".into()));
    assert_eq!(item(&e, id).shown_label(), None);
}

fn lock(e: &mut Editor, id: u64, choice: Choice) {
    e.dispatch(Command::EditItemCommon {
        items: vec![id],
        edit: ItemCommonEdit::Lock(choice),
    })
    .unwrap();
}

#[test]
fn lock_refuses_item_transforms() {
    // CP-A02: Lock Yes refuses typed transforms, tool drags and Camera view
    // moves; Default and No don't.
    let mut e = new_scene();
    let mesh = e.scene.items[0].id;
    let camera = e.scene.items[1].id;
    e.scene.items[0].mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    lock(&mut e, mesh, Choice::Yes);
    let before = e.scene.clone();
    let moved = Transform {
        translation: DVec3::new(1.0, 0.0, 0.0),
        ..Default::default()
    };
    let err = e
        .dispatch(Command::ItemTransform {
            item: mesh,
            transform: moved.clone(),
        })
        .unwrap_err();
    assert!(err.contains("locked"), "{err}");
    assert_eq!(e.scene, before);
    // A Move drag in Item mode does nothing.
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.dispatch(Command::SelectItem(Some(mesh))).unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    assert!(
        e.dispatch(Command::Preview(Parameters {
            delta: DVec3::X,
            ..Default::default()
        }))
        .is_err()
    );
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(item(&e, mesh).transform, Transform::default());
    // Visible is locked too; Label and Lock itself stay editable.
    assert!(
        e.dispatch(Command::EditItemCommon {
            items: vec![mesh],
            edit: ItemCommonEdit::Visible(DisplayVisible::No),
        })
        .is_err()
    );
    e.dispatch(Command::EditItemCommon {
        items: vec![mesh],
        edit: ItemCommonEdit::Label("Locked".into()),
    })
    .unwrap();
    // Camera view can't move a locked camera.
    lock(&mut e, camera, Choice::Yes);
    let t = item(&e, camera).transform.clone();
    assert!(
        e.dispatch(Command::CameraView {
            item: camera,
            transform: moved.clone(),
            target_distance: 4.0,
        })
        .is_err()
    );
    assert_eq!(item(&e, camera).transform, t);
    // Unlocked again (No), transforms work.
    lock(&mut e, mesh, Choice::No);
    e.dispatch(Command::ItemTransform {
        item: mesh,
        transform: moved.clone(),
    })
    .unwrap();
    assert_eq!(item(&e, mesh).transform, moved);
}

#[test]
fn lock_refuses_the_whole_multi_selection_edit() {
    // A locked item among several refuses a Visible edit for all (all or
    // nothing), and refuse_locked names it.
    let mut e = new_scene();
    let ids: Vec<u64> = e.scene.items.iter().map(|i| i.id).collect();
    lock(&mut e, ids[2], Choice::Yes);
    assert!(
        e.dispatch(Command::EditItemCommon {
            items: ids.clone(),
            edit: ItemCommonEdit::Visible(DisplayVisible::No),
        })
        .is_err()
    );
    assert!(
        e.scene
            .items
            .iter()
            .all(|i| i.common.display.visible == DisplayVisible::Default)
    );
    let err = e.scene.refuse_locked(&ids).unwrap_err();
    assert!(err.starts_with("Directional Light"), "{err}");
}

#[test]
fn select_no_keeps_item_list_selection() {
    // CP-A01: Select No only affects 3D-view picks (app side); the item
    // list and commands still select it.
    let mut e = new_scene();
    let mesh = e.scene.items[0].id;
    e.dispatch(Command::EditItemCommon {
        items: vec![mesh],
        edit: ItemCommonEdit::Select(Choice::No),
    })
    .unwrap();
    assert!(!item(&e, mesh).is_viewport_selectable());
    e.dispatch(Command::SelectItem(None)).unwrap();
    e.dispatch(Command::SelectItem(Some(mesh))).unwrap();
    assert_eq!(e.scene.selection.item, Some(mesh));
}
