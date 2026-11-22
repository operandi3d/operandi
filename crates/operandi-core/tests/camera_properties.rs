//! Camera Properties panels (spec camera-properties-2026-11-06): every
//! edit is one undo step named after the property, applies to every
//! selected camera, refuses bad values, and saves and reopens; Reset,
//! Sync to View, Autofocus and the Background Image load are one step each.
use glam::DVec3;
use operandi_core::camera_light::*;
use operandi_core::camera_props::*;
use operandi_core::*;

fn new_scene() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.add_default_camera_and_light().unwrap();
    e.next_item_id = e.scene.next_id;
    e
}

fn cameras(e: &Editor) -> Vec<u64> {
    e.scene
        .items
        .iter()
        .filter(|i| i.camera.is_some())
        .map(|i| i.id)
        .collect()
}

fn camera(e: &Editor, id: u64) -> &CameraSettings {
    e.scene
        .items
        .iter()
        .find(|i| i.id == id)
        .unwrap()
        .camera
        .as_ref()
        .unwrap()
}

fn edit(e: &mut Editor, item: u64, edit: CameraEdit) -> Result<()> {
    e.dispatch(Command::EditCamera { item, edit })
}

#[test]
fn new_scene_camera_matches_the_spec_defaults() {
    let e = new_scene();
    let id = cameras(&e)[0];
    let c = camera(&e, id);
    let item = e.scene.items.iter().find(|i| i.id == id).unwrap();
    assert!(item.transform.zxy, "CP-V05 Order ZXY");
    assert_eq!(c.focus_distance, 4.0, "CP-E02 new-scene camera");
    assert_eq!(c.target_distance, 4.0);
    assert_eq!(c.focal_length, 0.05);
    assert_eq!((c.film_width, c.film_height), (0.036, 0.024));
    assert_eq!(c.film_fit, FilmFit::Fill);
    assert_eq!(c.stored.display_size, 0.0, "CP-D04 cameras 0.0");
    assert!(!c.dof.enable);
    assert_eq!(c.dof.f_stop, 4.0);
    assert_eq!(c.dof.iris_blades, 0);
    assert_eq!(c.dof.edge_weighting, 0.5);
    assert!(!c.resolution.enable);
    assert_eq!((c.resolution.width, c.resolution.height), (1920, 1080));
    assert!(c.background.enable && c.background.image.is_none());
    assert!(c.overlay.is_none());
    // A camera made later focuses at 1 m.
    assert_eq!(CameraSettings::default().focus_distance, 1.0);
}

#[test]
fn edits_are_one_undo_step_named_after_the_property() {
    let mut e = new_scene();
    let id = cameras(&e)[0];
    for (change, name) in [
        (CameraEdit::FocalLength(0.035), "Focal Length"),
        (CameraEdit::DofEnable(true), "Depth of Field"),
        (CameraEdit::FStop(2.8), "F-Stop"),
        (CameraEdit::IrisBlades(6), "Iris Blades"),
        (CameraEdit::IrisRotation(0.2), "Iris Rotation"),
        (
            CameraEdit::ResolutionEnable(true),
            "Override Render Resolution",
        ),
        (CameraEdit::ResolutionWidth(800), "Width"),
        (
            CameraEdit::Projection(Projection::Spherical),
            "Projection Type",
        ),
        (CameraEdit::LensDistortion(0.1), "Lens Distortion"),
        (CameraEdit::ShowOverlay(true), "Show Camera Overlay"),
        (
            CameraEdit::Overlay(OverlayEdit::Grid(OverlayGrid::RuleOfThirds)),
            "Grid",
        ),
    ] {
        let undo = e.undo.len();
        edit(&mut e, id, change).unwrap();
        assert_eq!(e.undo.len(), undo + 1);
        assert_eq!(e.undo.last().unwrap().label, name);
    }
    let c = camera(&e, id);
    assert_eq!(c.overlay.as_ref().unwrap().grid, OverlayGrid::RuleOfThirds);
    assert_eq!(
        c.render_size(operandi_core::camera_light_defaults::RENDER_FRAME),
        [800, 1080]
    );
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(
        camera(&e, id).overlay.as_ref().unwrap().grid,
        OverlayGrid::Off
    );
}

#[test]
fn bad_values_change_nothing() {
    let mut e = new_scene();
    let id = cameras(&e)[0];
    let before = e.scene.clone();
    let undo = e.undo.len();
    for bad in [
        CameraEdit::FStop(0.9),
        CameraEdit::IrisBlades(2),
        CameraEdit::IrisBlades(17),
        CameraEdit::ResolutionWidth(0),
        CameraEdit::EdgeWeighting(1.5),
        CameraEdit::FilmBack {
            width: 0.0,
            height: 0.024,
        },
        // No overlay yet.
        CameraEdit::Overlay(OverlayEdit::Opacity(0.2)),
        // An image that was never loaded.
        CameraEdit::BackgroundImage(Some(99)),
    ] {
        assert!(edit(&mut e, id, bad.clone()).is_err(), "{bad:?}");
    }
    assert_eq!(e.scene, before);
    assert_eq!(e.undo.len(), undo);
}

#[test]
fn every_selected_camera_follows() {
    let mut e = new_scene();
    e.dispatch(Command::NewCamera).unwrap();
    let ids = cameras(&e);
    assert_eq!(ids.len(), 2);
    e.scene.selection.mode = Mode::Item;
    e.scene.selection.item = Some(ids[0]);
    e.scene.selection.also = vec![ids[1]];
    let undo = e.undo.len();
    edit(&mut e, ids[0], CameraEdit::FStop(2.0)).unwrap();
    assert_eq!(e.undo.len(), undo + 1);
    assert!(ids.iter().all(|id| camera(&e, *id).dof.f_stop == 2.0));
    // All or nothing: the lights in the selection are skipped, a refusal
    // on any camera changes none.
    edit(&mut e, ids[0], CameraEdit::ShowOverlay(true)).unwrap();
    assert!(ids.iter().all(|id| camera(&e, *id).overlay.is_some()));
}

#[test]
fn film_back_preset_sets_width_and_height_in_one_step() {
    let mut e = new_scene();
    let id = cameras(&e)[0];
    let presets = built_in_film_backs();
    assert_eq!(
        matching_film_back(camera(&e, id), &presets).map(|p| p.name.as_str()),
        Some("Default (36 × 24 mm)")
    );
    let apsc = presets
        .iter()
        .find(|p| p.name.starts_with("APS-C"))
        .unwrap();
    let undo = e.undo.len();
    edit(
        &mut e,
        id,
        CameraEdit::FilmBack {
            width: apsc.width,
            height: apsc.height,
        },
    )
    .unwrap();
    assert_eq!(e.undo.len(), undo + 1);
    assert_eq!(matching_film_back(camera(&e, id), &presets), Some(apsc));
}

#[test]
fn reset_sync_and_autofocus_are_one_step_each() {
    let mut e = new_scene();
    let id = cameras(&e)[0];
    let before = e.scene.clone();
    let undo = e.undo.len();
    e.dispatch(Command::ResetTransform {
        items: vec![id],
        part: TransformReset::Rotation,
    })
    .unwrap();
    assert_eq!(e.undo.len(), undo + 1);
    assert_eq!(e.undo.last().unwrap().label, "Reset Rotation");
    let item = e.scene.items.iter().find(|i| i.id == id).unwrap();
    assert_eq!(item.transform.rotation, DVec3::ZERO);
    assert_eq!(item.transform.translation, DVec3::new(0.0, 0.75, 4.0));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);

    // Sync to View.
    let transform = scene::Transform {
        translation: DVec3::new(3.0, 1.0, 3.0),
        rotation: look_at_rotation(DVec3::new(3.0, 1.0, 3.0), DVec3::ZERO),
        ..Default::default()
    };
    e.dispatch(Command::SyncCameraToView {
        item: id,
        transform: transform.clone(),
        target_distance: 19f64.sqrt(),
    })
    .unwrap();
    assert_eq!(e.undo.last().unwrap().label, "Sync to View");
    e.dispatch(Command::Undo).unwrap();

    // Autofocus: nothing in view yet, then a 1 m box at the origin.
    assert!(e.dispatch(Command::CameraAutofocus(id)).is_err());
    e.scene.items[0].mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    e.dispatch(Command::CameraAutofocus(id)).unwrap();
    assert_eq!(e.undo.last().unwrap().label, "Autofocus");
    let d = camera(&e, id).focus_distance;
    assert!(d > 3.0 && d < 4.0, "{d}");

    // A locked camera refuses Reset and Sync.
    e.dispatch(Command::EditItemCommon {
        items: vec![id],
        edit: item_common::ItemCommonEdit::Lock(item_common::Choice::Yes),
    })
    .unwrap();
    assert!(
        e.dispatch(Command::ResetTransform {
            items: vec![id],
            part: TransformReset::All,
        })
        .is_err()
    );
    assert!(
        e.dispatch(Command::SyncCameraToView {
            item: id,
            transform,
            target_distance: 1.0,
        })
        .is_err()
    );
}

#[test]
fn background_image_loads_as_one_step() {
    let mut e = new_scene();
    let id = cameras(&e)[0];
    let undo = e.undo.len();
    e.dispatch(Command::CameraBackgroundLoad {
        item: id,
        path: "/tmp/plate.png".into(),
        width: 640,
        height: 480,
    })
    .unwrap();
    assert_eq!(e.undo.len(), undo + 1);
    let image = camera(&e, id).background.image.unwrap();
    assert_eq!(e.scene.image(image).unwrap().width, 640);
    edit(
        &mut e,
        id,
        CameraEdit::ResolutionFromClip {
            width: 640,
            height: 480,
        },
    )
    .unwrap();
    assert_eq!(camera(&e, id).render_size(e.scene.render.frame), [640, 480]);
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert!(camera(&e, id).background.image.is_none());
}

#[test]
fn save_and_reopen_keeps_the_new_channels() {
    let mut e = new_scene();
    let id = cameras(&e)[0];
    for change in [
        CameraEdit::DofEnable(true),
        CameraEdit::FStop(1.4),
        CameraEdit::IrisBlades(5),
        CameraEdit::Projection(Projection::CylindricalVr),
        CameraEdit::ResolutionEnable(true),
        CameraEdit::ShowOverlay(true),
        CameraEdit::Overlay(OverlayEdit::Horizon(true)),
        CameraEdit::FocusFromTarget(true),
        CameraEdit::LensSqueeze(1.33),
    ] {
        edit(&mut e, id, change).unwrap();
    }
    let bytes = persistence::encode(&e.scene).unwrap();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    assert!(text.contains("\"version\": 33"));
    assert_eq!(persistence::decode(&bytes).unwrap(), e.scene);
    // Version 19 can't carry them.
    let old = text.replace("\"version\": 33", "\"version\": 19");
    assert!(persistence::decode(old.as_bytes()).is_err());
    // An older file without the new fields opens with their defaults.
    let plain = new_scene();
    let bytes = persistence::encode(&plain.scene).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    // Version 33 for the Environment item; no camera channels written.
    assert!(text.contains("\"version\": 33"));
    let opened = persistence::decode(&bytes).unwrap();
    assert_eq!(opened, plain.scene);
}

#[test]
fn camera_settings_from_before_version_19_read_with_defaults() {
    let mut json = serde_json::to_value(CameraSettings::default()).unwrap();
    let map = json.as_object_mut().unwrap();
    for key in [
        "lens_distortion",
        "dof",
        "resolution",
        "background",
        "focus_from_target",
    ] {
        assert!(map.remove(key).is_some(), "{key}");
    }
    let read: CameraSettings = serde_json::from_value(json).unwrap();
    assert_eq!(read, CameraSettings::default());
}
