//! Master plan phase 0 "doors" (MASTER-PLAN.md §4, ARCHITECTURE-RULES):
//! kit items that load without their kit (KIT-1, AR-09), scene time in
//! the file (AR-02), how a channel holds its value (AR-08), one evaluated
//! scene (AR-01), a time on command records, one selection-click path
//! (AR-23).
use glam::DVec3;
use operandi_core::channels::{self, Held, Target};
use operandi_core::click_select::{self, Click, ClickHow, Outcome};
use operandi_core::command_record::{CommandRecord, Source};
use operandi_core::evaluate::Purpose;
use operandi_core::kit_item::KitItem;
use operandi_core::scene_time::SceneTime;
use operandi_core::time_tick::{FrameRate, Tick};
use operandi_core::*;

fn splat() -> KitItem {
    KitItem {
        kit: "splat".into(),
        kind: "splat.cloud".into(),
        version: 3,
        values: serde_json::json!({
            "count": 120000,
            "shDegree": 3,
            "source": {"file": "project:splats/garden.ply", "scale": 0.5},
            "tags": ["outdoor", "scan"],
        }),
        bounds: Some([DVec3::new(-1., 0., -2.), DVec3::new(1., 1.5, 2.)]),
    }
}

/// A scene with one plain mesh and one kit item (id 2).
fn with_kit_item() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.new_mesh().unwrap();
    let item = &mut e.scene.items[1];
    item.name = "Garden splat".into();
    item.kit = Some(Box::new(splat()));
    item.transform.translation = DVec3::new(3., 0., 0.);
    e.scene.select_item(None);
    e
}

fn line(e: &mut Editor, text: &str) -> Result<Option<String>> {
    command_line::run_line(e, text).expect("a command line")
}

// ------------------------------------------------------------ door 1

#[test]
fn a_kit_item_without_its_kit_saves_and_reopens_unchanged() {
    let e = with_kit_item();
    let bytes = persistence::encode(&e.scene).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("\"version\": 49"), "kit items mark v49");
    let back = persistence::decode(&bytes).unwrap();
    let item = back.item(2).unwrap();
    assert_eq!(item.kit.as_deref(), Some(&splat()));
    assert!(!item.is_mesh());
    assert!(!item.kit.as_ref().unwrap().installed());
    // Saved again, byte for byte the same file.
    assert_eq!(persistence::encode(&back).unwrap(), bytes);
}

#[test]
fn a_file_written_by_a_build_with_the_kit_opens() {
    // As another build (with the splat kit) would write it: values this
    // build has never heard of, in its own order.
    let text = r#"{
      "format": "Operandi",
      "version": 49,
      "scene": {
        "items": [{
          "id": 1, "name": "Cloud", "visible": true,
          "transform": {"translation": [0,0,0], "rotation": [0,0,0], "scale": [1,1,1]},
          "mesh": {"vertices": [], "faces": []},
          "kit": {"kit": "splat", "kind": "splat.cloud", "version": 7,
                  "values": {"zeta": 1, "alpha": {"nested": [1, 2, {"deep": true}]}}}
        }],
        "next_id": 2,
        "selection": {"item": null, "mode": "Item", "vertices": [], "edges": [], "faces": []},
        "workplane": {"fixed": false, "plane": 0, "center": [0,0,0]}
      }
    }"#;
    let scene = persistence::decode(text.as_bytes()).unwrap();
    let kit = scene.items[0].kit.as_ref().unwrap();
    assert_eq!(kit.version, 7);
    assert_eq!(kit.values["alpha"]["nested"][2]["deep"], true);
    assert!(kit.bounds.is_none());
    // Its data survives a save and reopen exactly.
    let again = persistence::decode(&persistence::encode(&scene).unwrap()).unwrap();
    assert_eq!(again.items[0].kit, scene.items[0].kit);
}

#[test]
fn an_older_version_with_kit_data_is_refused_cleanly() {
    let e = with_kit_item();
    let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    let old = text.replace("\"version\": 49", "\"version\": 48");
    let err = persistence::decode(old.as_bytes()).unwrap_err();
    assert!(err.contains("version 49"), "{err}");
    // A scene with no kit items and default time keeps its old number.
    let mut plain = Editor::default();
    plain.scene.new_mesh().unwrap();
    let text = String::from_utf8(persistence::encode(&plain.scene).unwrap()).unwrap();
    assert!(!text.contains("\"version\": 49"));
}

#[test]
fn a_kit_item_refuses_geometry_and_bad_bounds() {
    let mut e = with_kit_item();
    e.scene.items[1].mesh = cube_geometry(DVec3::ZERO, DVec3::ONE).unwrap().into();
    assert!(e.scene.validate().is_err());
    let mut e = with_kit_item();
    e.scene.items[1].kit.as_mut().unwrap().bounds = Some([DVec3::ONE, DVec3::ZERO]);
    assert!(e.scene.validate().is_err());
}

#[test]
fn scripts_read_a_kit_items_values_but_cannot_change_them() {
    let mut e = with_kit_item();
    let t = Target::Item(2);
    assert_eq!(channels::get(&e.scene, t, "count").unwrap(), "120000");
    assert_eq!(
        channels::get(&e.scene, t, "source.file").unwrap(),
        "project:splats/garden.ply"
    );
    let refused = line(&mut e, "item.channel name:count value:5 item:2").unwrap_err();
    assert!(refused.contains("Needs kit \"splat\""), "{refused}");
    // Unknown names list the values it has.
    let unknown = channels::get(&e.scene, t, "nope").unwrap_err();
    assert!(unknown.contains("source.scale"), "{unknown}");
    // Item channels every item has still work (one undo step).
    line(&mut e, "item.channel name:label value:Garden item:2").unwrap();
    assert_eq!(channels::get(&e.scene, t, "label").unwrap(), "Garden");
    assert_eq!(e.scene.items[1].kit.as_deref(), Some(&splat()));
}

#[test]
fn kit_items_survive_copy_delete_and_undo() {
    let mut e = with_kit_item();
    e.dispatch(Command::SelectItem(Some(2))).unwrap();
    e.dispatch(Command::DeleteItem).unwrap();
    assert!(e.scene.item(2).is_none());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.item(2).unwrap().kit.as_deref(), Some(&splat()));
}

// ------------------------------------------------------------ door 2

#[test]
fn scene_time_is_set_by_command_saved_and_undone() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    assert_eq!(e.scene.time, SceneTime::default());
    line(&mut e, "item.channel name:fps value:30 scene:true").unwrap();
    line(&mut e, "item.channel name:end value:10 scene:true").unwrap();
    line(&mut e, "item.channel name:time value:2.5 scene:true").unwrap();
    assert_eq!(
        e.scene.time,
        SceneTime {
            rate: FrameRate::FPS_30,
            start: Tick::ZERO,
            end: Tick::from_seconds(10.),
            current: Tick::from_seconds(2.5),
        }
    );
    // The query reads it back.
    let answer = command_exec::answer(
        &e,
        "item.channel",
        serde_json::json!({"name": "fps", "value": "?", "scene": true})
            .as_object()
            .unwrap(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(answer, "30");
    // Saved (v49) and reopened.
    let bytes = persistence::encode(&e.scene).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("\"version\": 49"));
    assert_eq!(persistence::decode(&bytes).unwrap().time, e.scene.time);
    // Rate and range are one undo step each; the current time is not an
    // undo step at all (TK-05, animation phase 1), so Undo takes back the
    // end and leaves the time.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.time.current, Tick::from_seconds(2.5));
    assert_eq!(e.scene.time.end, Tick::from_seconds(5.));
}

#[test]
fn bad_scene_times_are_refused_and_change_nothing() {
    let mut e = Editor::default();
    for bad in [
        "item.channel name:fps value:0 scene:true",
        "item.channel name:start value:9 scene:true",
        "item.channel name:frames value:1 scene:true",
        "item.channel name:fps value:24 scene:true item:1",
    ] {
        assert!(line(&mut e, bad).is_err(), "{bad}");
    }
    assert_eq!(e.scene.time, SceneTime::default());
    // A hand-edited file with a bad range is refused.
    let mut scene = Scene::default();
    scene.time.rate = FrameRate::FPS_30;
    let text = String::from_utf8(persistence::encode(&scene).unwrap()).unwrap();
    let bad = text.replace("\"num\": 30", "\"num\": -30");
    assert!(bad != text);
    assert!(persistence::decode(bad.as_bytes()).is_err());
}

// ------------------------------------------------------------ door 3

#[test]
fn every_channel_reads_through_how_it_is_held() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let id = e.scene.items[0].id;
    let held = channels::held(&e.scene, Target::Item(id), "rotOrder").unwrap();
    assert_eq!(held.label(), "constant");
    assert!(matches!(&held, Held::Constant(_)));
    // A constant is the same at every time.
    assert_eq!(
        held.value_at(Tick::ZERO),
        held.value_at(Tick::from_seconds(123.))
    );
    // The stored form is plain data, ready for the file.
    let json = serde_json::to_string(&held).unwrap();
    let back: Held = serde_json::from_str(&json).unwrap();
    assert_eq!(back, held);
    let fps = channels::held(&e.scene, Target::Scene, "fps").unwrap();
    assert_eq!(fps, Held::Constant(channels::ChannelValue::Float(24.)));
}

// ------------------------------------------------------------ door 4

#[test]
fn one_evaluated_scene_for_every_reader() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    // Nothing procedural: every purpose reads the document scene itself.
    for purpose in [
        Purpose::Final,
        Purpose::Pick,
        Purpose::Display { base_edit: None },
    ] {
        assert!(matches!(
            e.evaluated(purpose),
            std::borrow::Cow::Borrowed(_)
        ));
    }
    assert!(e.evaluated_display().is_none());
    // A MeshOps Cube on an empty Base Mesh.
    e.dispatch(Command::MeshOpCreate("prim.cube.item".into()))
        .unwrap();
    let base_points = e.scene.items[0].mesh.vertices.len();
    let shown = e.evaluated(Purpose::Final);
    assert!(shown.items[0].mesh.vertices.len() > base_points);
    // Export reads the same scene renders do.
    let exported = e.evaluated(Purpose::Final).into_owned();
    assert_eq!(
        exported.items[0].mesh.vertices.len(),
        shown.items[0].mesh.vertices.len()
    );
    // Picking stays on the Base Mesh until it can map back (AR-04).
    assert_eq!(
        e.evaluated(Purpose::Pick).items[0].mesh.vertices.len(),
        base_points
    );
    // Edit Base Mesh shows the Base Mesh in the viewport.
    let id = e.scene.items[0].id;
    let display = e.evaluated(Purpose::Display {
        base_edit: Some(id),
    });
    assert_eq!(display.items[0].mesh.vertices.len(), base_points);
}

// ------------------------------------------------------------ door 5

#[test]
fn command_records_have_room_for_a_scene_time() {
    let record = CommandRecord::of_line("select.drop polygon", Source::Typed, 1.5);
    assert_eq!(record.scene_time, None);
    let at = FrameRate::FPS_24.tick_of(24);
    let keyed = record.at_scene_time(at);
    assert_eq!(keyed.scene_time, Some(Tick::from_seconds(1.)));
    assert_eq!(keyed.time, 1.5, "app time is separate");
}

// ------------------------------------------------------------ door 6

#[test]
fn a_click_on_an_item_selects_its_redirect() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.new_mesh().unwrap();
    let (a, b) = (e.scene.items[0].id, e.scene.items[1].id);
    line(
        &mut e,
        &format!("item.channel name:redirect value:{b} item:{a}"),
    )
    .unwrap();
    let outcome = click_select::resolve(
        &e.scene,
        Click {
            hit: Some((a, Component::Item)),
            how: ClickHow::Replace,
            over_skipped_mesh: false,
        },
    );
    let Outcome::Run(command) = outcome else {
        panic!("a hit selects");
    };
    e.dispatch(*command).unwrap();
    assert_eq!(e.scene.selection.item, Some(b));
}

#[test]
fn the_qa_fixture_opens_with_its_kit_item_and_time() {
    let scene = persistence::decode(include_bytes!("../../../fixtures/kit-item-splat.opscene"))
        .expect("the GUI QA fixture opens");
    assert!(scene.items[0].kit.is_some());
    assert_eq!(scene.time.rate, FrameRate::FPS_29_97);
    assert_eq!(scene.time.current_frame(), 24);
}

#[test]
fn typed_queries_answer_scene_time_and_kit_values() {
    let mut e = with_kit_item();
    assert_eq!(
        line(&mut e, "item.channel name:fps value:? scene:true").unwrap(),
        Some("24".to_string())
    );
    assert_eq!(
        line(&mut e, "item.channel name:count value:? item:2").unwrap(),
        Some("120000".to_string())
    );
}

#[test]
fn frames_are_exact_at_29_97() {
    let mut e = Editor::default();
    line(&mut e, "item.channel name:fps value:29.97 scene:true").unwrap();
    assert_eq!(e.scene.time.rate, FrameRate::FPS_29_97);
    line(&mut e, "item.channel name:frame value:1000 scene:true").unwrap();
    assert_eq!(e.scene.time.current, FrameRate::FPS_29_97.tick_of(1000));
    // 1000 frames at 30000/1001 fps is exactly 1001/30 s: whole ticks.
    assert_eq!(
        e.scene.time.current.0 * 30,
        1001 * operandi_core::time_tick::TICKS_PER_SECOND
    );
    assert_eq!(
        line(&mut e, "item.channel name:frame value:? scene:true").unwrap(),
        Some("1000".to_string())
    );
    // Saved as integers, so it reopens to the same tick.
    let back = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    assert_eq!(back.time, e.scene.time);
}
