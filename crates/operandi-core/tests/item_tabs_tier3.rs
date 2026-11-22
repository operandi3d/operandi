//! Shared item tabs, Tier 3 rows (BEHAVIOR-ITEM-COMMON-TABS): Pivot,
//! Center, Draw Options, Dimensions, Notes (CP-D02..09), Redirect, Pick
//! Walk, Tooltip, User Tag, per-item Drop Action (CP-A03..09), User
//! Channels (CP-U) and Tags (CP-T), with IC-1..4.
use operandi_core::drop_action::DropAction;
use operandi_core::item_common::*;
use operandi_core::item_extras::*;
use operandi_core::*;

fn new_scene() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.add_default_camera_and_light().unwrap();
    e.next_item_id = e.scene.next_id;
    e
}

fn ids(e: &Editor) -> Vec<u64> {
    e.scene.items.iter().map(|i| i.id).collect()
}

fn extra(e: &mut Editor, items: &[u64], edit: ExtraEdit) -> Result<()> {
    e.dispatch(Command::EditItemCommon {
        items: items.to_vec(),
        edit: ItemCommonEdit::Extra(edit),
    })
}

fn extras(e: &Editor, id: u64) -> &ItemExtras {
    &e.scene
        .items
        .iter()
        .find(|i| i.id == id)
        .unwrap()
        .common
        .extras
}

fn last_label(e: &Editor) -> &str {
    &e.undo.last().unwrap().label
}

fn round_trip(scene: &Scene) -> Scene {
    let bytes = persistence::encode(scene).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("\"version\": 33"));
    persistence::decode(&bytes).unwrap()
}

#[test]
fn defaults_write_nothing() {
    let e = new_scene();
    for id in ids(&e) {
        assert_eq!(extras(&e, id), &ItemExtras::default());
    }
    let x = ItemExtras::default();
    assert_eq!((x.pivot, x.center), (Choice::Default, Choice::Default));
    assert!(x.draw_options.is_none() && x.dimensions.is_none() && x.notes.is_none());
}

#[test]
fn add_and_remove_gadgets_keep_ic4() {
    let mut e = new_scene();
    let mesh = ids(&e)[0];
    extra(&mut e, &[mesh], ExtraEdit::DrawOptions(true)).unwrap();
    assert_eq!(last_label(&e), "Add Draw Options");
    assert_eq!(extras(&e, mesh).draw_options, Some(DrawOptions::default()));
    extra(&mut e, &[mesh], ExtraEdit::Wireframe(WireColor::User)).unwrap();
    extra(&mut e, &[mesh], ExtraEdit::WireColor([1.0, 0.0, 0.0])).unwrap();
    assert_eq!(last_label(&e), "Wireframe Color");
    // Remove drops the settings; undo brings them back.
    extra(&mut e, &[mesh], ExtraEdit::DrawOptions(false)).unwrap();
    assert_eq!(last_label(&e), "Remove Draw Options");
    assert!(extras(&e, mesh).draw_options.is_none());
    e.dispatch(Command::Undo).unwrap();
    let d = extras(&e, mesh).draw_options.clone().unwrap();
    assert_eq!(
        (d.wireframe, d.wire_color),
        (WireColor::User, [1.0, 0.0, 0.0])
    );
    // Rows of a gadget that isn't there are refused.
    assert!(extra(&mut e, &[mesh], ExtraEdit::DimensionsOpacity(0.5)).is_err());
    extra(&mut e, &[mesh], ExtraEdit::Dimensions(true)).unwrap();
    let dims = extras(&e, mesh).dimensions.clone().unwrap();
    assert_eq!(
        (dims.visibility, dims.opacity, dims.show),
        (DimensionsShow::On, 1.0, [true; 3])
    );
    extra(&mut e, &[mesh], ExtraEdit::DimensionsAxis(1, false)).unwrap();
    extra(
        &mut e,
        &[mesh],
        ExtraEdit::DimensionsShow(DimensionsShow::Selected),
    )
    .unwrap();
    extra(&mut e, &[mesh], ExtraEdit::Notes(true)).unwrap();
    extra(&mut e, &[mesh], ExtraEdit::NotesTitle("Todo".into())).unwrap();
    extra(&mut e, &[mesh], ExtraEdit::NoteLine(2, "third".into())).unwrap();
    assert_eq!(
        extras(&e, mesh).notes.as_ref().unwrap().lines,
        ["", "", "third"]
    );
    assert!(extra(&mut e, &[mesh], ExtraEdit::NoteLine(9, "tenth".into())).is_err());
    assert_eq!(round_trip(&e.scene), e.scene);
}

#[test]
fn pivot_center_are_locked_with_the_item() {
    let mut e = new_scene();
    let mesh = ids(&e)[0];
    extra(&mut e, &[mesh], ExtraEdit::Pivot(Choice::Yes)).unwrap();
    assert_eq!(last_label(&e), "Pivot");
    e.dispatch(Command::EditItemCommon {
        items: vec![mesh],
        edit: ItemCommonEdit::Lock(Choice::Yes),
    })
    .unwrap();
    assert!(extra(&mut e, &[mesh], ExtraEdit::Center(Choice::Yes)).is_err());
    // Other rows stay editable.
    extra(&mut e, &[mesh], ExtraEdit::Tooltip("hi".into())).unwrap();
}

#[test]
fn redirect_and_pick_walk() {
    let mut e = new_scene();
    let [mesh, camera, light, ..] = ids(&e)[..] else {
        panic!()
    };
    assert_eq!(e.scene.redirected(mesh), mesh);
    extra(&mut e, &[mesh], ExtraEdit::Redirect(Some(light))).unwrap();
    assert_eq!(last_label(&e), "Redirect");
    assert_eq!(e.scene.redirected(mesh), light);
    assert!(extra(&mut e, &[mesh], ExtraEdit::Redirect(Some(mesh))).is_err());
    assert!(extra(&mut e, &[mesh], ExtraEdit::Redirect(Some(999))).is_err());
    // Defaults: Up = parent, Down = first child, Left/Right = siblings.
    e.scene.set_parent(camera, Some(mesh), false).unwrap();
    assert_eq!(e.scene.pick_walk(camera, Walk::Up), Some(mesh));
    assert_eq!(e.scene.pick_walk(mesh, Walk::Down), Some(camera));
    assert_eq!(e.scene.pick_walk(mesh, Walk::Right), Some(light));
    assert_eq!(e.scene.pick_walk(light, Walk::Left), Some(mesh));
    assert_eq!(e.scene.pick_walk(mesh, Walk::Left), None);
    // An override wins.
    extra(&mut e, &[mesh], ExtraEdit::PickWalk(Walk::Up, Some(light))).unwrap();
    assert_eq!(last_label(&e), "Pick Walk Up");
    assert_eq!(e.scene.pick_walk(mesh, Walk::Up), Some(light));
    // A deleted target is ignored.
    e.scene.items.retain(|i| i.id != light);
    assert_eq!(e.scene.redirected(mesh), mesh);
    assert_eq!(e.scene.pick_walk(mesh, Walk::Up), None);
}

#[test]
fn per_item_drop_actions_override_the_top_bar() {
    let mut e = new_scene();
    let [mesh, camera, _, ..] = ids(&e)[..] else {
        panic!()
    };
    assert_eq!(
        e.scene.drop_action_for(mesh, camera, DropAction::Parent),
        DropAction::Parent
    );
    extra(
        &mut e,
        &[mesh],
        ExtraEdit::DropAction(1, Some(DropAction::Match)),
    )
    .unwrap();
    assert_eq!(last_label(&e), "Drop Action");
    assert_eq!(
        e.scene.drop_action_for(mesh, camera, DropAction::Parent),
        DropAction::Match
    );
    extra(
        &mut e,
        &[camera],
        ExtraEdit::DropAction(2, Some(DropAction::Place)),
    )
    .unwrap();
    assert_eq!(
        e.scene.drop_action_for(mesh, camera, DropAction::Parent),
        DropAction::Place
    );
    assert!(extra(&mut e, &[mesh], ExtraEdit::DropAction(3, None)).is_err());
}

#[test]
fn text_rows_on_several_items() {
    let mut e = new_scene();
    let all = ids(&e);
    extra(&mut e, &all, ExtraEdit::Tooltip("Hero".into())).unwrap();
    extra(&mut e, &all, ExtraEdit::UserTag("asset:hero".into())).unwrap();
    assert_eq!(last_label(&e), "User Tag");
    for id in &all {
        assert_eq!(extras(&e, *id).tooltip, "Hero");
    }
    e.dispatch(Command::Undo).unwrap();
    assert!(all.iter().all(|id| extras(&e, *id).user_tag.is_empty()));
}

#[test]
fn user_channels() {
    let mut e = new_scene();
    let mesh = ids(&e)[0];
    let door = UserChannel::new(
        "Door Open",
        "Door %",
        ChannelType::Percent,
        ChannelValue::Number(0.25),
        Some(0.0),
        Some(1.0),
    )
    .unwrap();
    assert_eq!(door.label(), "Door %");
    extra(&mut e, &[mesh], ExtraEdit::AddChannel(door.clone())).unwrap();
    assert_eq!(last_label(&e), "Add User Channel");
    // Unique names; wrong kinds and bad names refused.
    assert!(extra(&mut e, &[mesh], ExtraEdit::AddChannel(door)).is_err());
    assert!(
        UserChannel::new(
            "x",
            "",
            ChannelType::Boolean,
            ChannelValue::Number(1.0),
            None,
            None
        )
        .is_err()
    );
    assert!(
        UserChannel::new(
            "",
            "",
            ChannelType::Float,
            ChannelValue::Number(1.0),
            None,
            None
        )
        .is_err()
    );
    assert!(
        UserChannel::new(
            "a",
            "",
            ChannelType::Float,
            ChannelValue::Number(0.0),
            Some(2.0),
            Some(1.0)
        )
        .is_err()
    );
    // Values clamp to the limits; integers round.
    extra(
        &mut e,
        &[mesh],
        ExtraEdit::ChannelValue("Door Open".into(), ChannelValue::Number(3.0)),
    )
    .unwrap();
    assert_eq!(
        extras(&e, mesh).channels[0].value,
        ChannelValue::Number(1.0)
    );
    let count = UserChannel::new(
        "Count",
        "",
        ChannelType::Integer,
        ChannelValue::Number(2.6),
        None,
        None,
    )
    .unwrap();
    assert_eq!(count.value, ChannelValue::Number(3.0));
    let divider = UserChannel::new(
        "Rig",
        "",
        ChannelType::Divider,
        ChannelValue::None,
        None,
        None,
    )
    .unwrap();
    for ch in [
        count,
        divider,
        UserChannel::new(
            "On",
            "",
            ChannelType::Boolean,
            ChannelValue::Bool(true),
            None,
            None,
        )
        .unwrap(),
        UserChannel::new(
            "Note",
            "",
            ChannelType::String,
            ChannelValue::Text("hi".into()),
            None,
            None,
        )
        .unwrap(),
    ] {
        extra(&mut e, &[mesh], ExtraEdit::AddChannel(ch)).unwrap();
    }
    assert!(
        extra(
            &mut e,
            &[mesh],
            ExtraEdit::ChannelValue("On".into(), ChannelValue::Text("x".into()))
        )
        .is_err()
    );
    assert_eq!(round_trip(&e.scene), e.scene);
    extra(&mut e, &[mesh], ExtraEdit::RemoveChannel("Count".into())).unwrap();
    assert_eq!(last_label(&e), "Remove User Channel");
    assert_eq!(extras(&e, mesh).channels.len(), 4);
    assert!(extra(&mut e, &[mesh], ExtraEdit::RemoveChannel("Count".into())).is_err());
}

#[test]
fn tags_one_per_type() {
    let mut e = new_scene();
    let mesh = ids(&e)[0];
    extra(
        &mut e,
        &[mesh],
        ExtraEdit::SetTag(COMMENT_TAG.into(), "client wants blue".into()),
    )
    .unwrap();
    assert_eq!(last_label(&e), "Tag");
    extra(
        &mut e,
        &[mesh],
        ExtraEdit::SetTag("Pipeline".into(), "hero".into()),
    )
    .unwrap();
    extra(
        &mut e,
        &[mesh],
        ExtraEdit::SetTag(COMMENT_TAG.into(), "now red".into()),
    )
    .unwrap();
    let tags = &extras(&e, mesh).tags;
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0].value, "now red");
    assert!(extra(&mut e, &[mesh], ExtraEdit::SetTag("  ".into(), "x".into())).is_err());
    assert_eq!(round_trip(&e.scene), e.scene);
    extra(&mut e, &[mesh], ExtraEdit::RemoveTag("Pipeline".into())).unwrap();
    assert_eq!(last_label(&e), "Remove Tag");
    assert!(extra(&mut e, &[mesh], ExtraEdit::RemoveTag("Pipeline".into())).is_err());
}
