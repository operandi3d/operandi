//! The environment image layer's Texture Locator (env-texture-locator):
//! rotation orders, the sky's frame and its axis, Light Probe, save/reopen
//! and the version 43 and 44 migrations.
use glam::{DMat3, DQuat, DVec3};
use operandi_core::environment::{environment_image_uv, environment_probe_uv};
use operandi_core::scene::{RotationOrder, Transform};
use operandi_core::shader_tree::{Projection, ShaderTreeEdit};
use operandi_core::texture_locator::TextureLocator;
use operandi_core::*;

fn close_quat(a: DQuat, b: DQuat) -> bool {
    a.dot(b).abs() > 1.0 - 1e-12
}

/// Every order turns its first letter first (ETL-Q01): XYZ is the measured
/// IT-O-H3 matrix (Rz·Ry·Rx on column vectors), ZXY is Ry·Rx·Rz as always,
/// and ZYX turns as XYZ did before native v44 (glam's XYZ, Rx·Ry·Rz).
#[test]
fn every_order_turns_its_first_letter_first() {
    let angles = DVec3::new(30f64, 45., 60.).map(f64::to_radians);
    let old_xyz = DQuat::from_euler(glam::EulerRot::XYZ, angles.x, angles.y, angles.z);
    let x_first = DQuat::from_rotation_z(angles.z)
        * DQuat::from_rotation_y(angles.y)
        * DQuat::from_rotation_x(angles.x);
    assert!(close_quat(RotationOrder::Xyz.quat(angles), x_first));
    assert!(close_quat(RotationOrder::Zyx.quat(angles), old_xyz));
    let zxy = DQuat::from_rotation_y(angles.y)
        * DQuat::from_rotation_x(angles.x)
        * DQuat::from_rotation_z(angles.z);
    assert!(close_quat(RotationOrder::Zxy.quat(angles), zxy));
    let yzx = DQuat::from_rotation_x(angles.x)
        * DQuat::from_rotation_z(angles.z)
        * DQuat::from_rotation_y(angles.y);
    assert!(close_quat(RotationOrder::Yzx.quat(angles), yzx));
    // IT-O-H3 input, XYZ: rows are the item's axes; X turns first. The
    // numbers are the oracle's, to its six decimals.
    #[allow(clippy::approx_constant)]
    let measured = DMat3::from_cols_array(&[
        0.353553, 0.612372, -0.707107, -0.573223, 0.739199, 0.353553, 0.739199, 0.28033, 0.612372,
    ]);
    let t = Transform {
        rotation: angles,
        ..Default::default()
    };
    assert_eq!(t.rotation_order(), RotationOrder::Xyz);
    assert!(DMat3::from_mat4(t.matrix()).abs_diff_eq(measured, 1e-5));
    assert!(!DMat3::from_quat(old_xyz).abs_diff_eq(measured, 1e-3));
}

/// Each order's angles come back from its own rotation, and a transform
/// in that order rebuilds the same matrix.
#[test]
fn every_order_round_trips_its_angles() {
    let angles = DVec3::new(20f64, -35., 50.).map(f64::to_radians);
    for order in RotationOrder::ALL {
        let mut t = Transform {
            rotation: angles,
            ..Default::default()
        };
        t.set_rotation_order(order);
        assert_eq!(t.rotation_order(), order);
        let q = DQuat::from_mat4(&t.matrix());
        let back = t.angles_for(q);
        assert!(
            (back - angles).abs().max_element() < 1e-9,
            "{order:?} {back:?}"
        );
    }
    // XYZ and ZXY still live in `zxy` only, so old files don't change.
    let mut t = Transform::default();
    t.set_rotation_order(RotationOrder::Zxy);
    assert!(t.zxy && t.order.is_none());
    t.set_rotation_order(RotationOrder::Yzx);
    assert!(!t.zxy && t.order == Some(RotationOrder::Yzx));
    t.set_rotation_order(RotationOrder::Xyz);
    assert!(!t.zxy && t.order.is_none());
}

fn scene_with_hdri() -> (Editor, u64, u64) {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.add_default_camera_and_light().unwrap();
    e.next_item_id = e.scene.next_id;
    e.dispatch(Command::SetEnvironmentImage {
        path: "/hdri/sky.hdr".into(),
        width: 64,
        height: 32,
        item: None,
    })
    .unwrap();
    let env = e
        .scene
        .items
        .iter()
        .rev()
        .find(|i| i.environment.is_some())
        .unwrap()
        .id;
    let (locator, heading) = e.scene.environment_image_heading(env).unwrap();
    assert_eq!(heading, 0.0);
    (e, env, locator)
}

fn locator_settings(e: &Editor, locator: u64) -> TextureLocator {
    e.scene.texture_locator(locator).unwrap().clone()
}

fn set_locator(
    e: &mut Editor,
    locator: u64,
    edit: impl FnOnce(&mut TextureLocator, &mut Transform),
) {
    let item = e.scene.items.iter().find(|i| i.id == locator).unwrap();
    let mut settings = item.texture_locator().unwrap().clone();
    let mut transform = item.transform.clone();
    edit(&mut settings, &mut transform);
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::SetLocator {
        locator,
        settings: Box::new(settings),
        transform,
        continuing: false,
    }))
    .unwrap();
}

fn sky_to_image(e: &Editor) -> DMat3 {
    e.scene
        .environment_sky(operandi_core::environment::RayKind::Camera)
        .unwrap()
        .to_image
}

fn uv_close(a: [f64; 2], b: [f64; 2]) -> bool {
    let du = (a[0] - b[0]).rem_euclid(1.0);
    du.min(1.0 - du) < 1e-9 && (a[1] - b[1]).abs() < 1e-9
}

/// A new HDRI layer's locator: Spherical about Y, ZXY, at identity
/// (ETL-Q02).
#[test]
fn new_image_layer_gets_a_spherical_locator() {
    let (e, _, locator) = scene_with_hdri();
    let s = locator_settings(&e, locator);
    assert_eq!(s.projection, Projection::Spherical);
    assert_eq!(s.axis, 1);
    let item = e.scene.items.iter().find(|i| i.id == locator).unwrap();
    assert_eq!(item.transform.rotation_order(), RotationOrder::Zxy);
    assert_eq!(item.transform.rotation, DVec3::ZERO);
    assert_eq!(item.transform.scale, DVec3::ONE);
    assert_eq!(sky_to_image(&e), DMat3::IDENTITY);
}

/// Rotation Y +90° turns the picture from +Z toward +X: what was straight
/// ahead is seen along +X. Position and Size don't move it. One undo step
/// puts it back.
#[test]
fn rotation_y_turns_the_sky_and_undoes() {
    let (mut e, _, locator) = scene_with_hdri();
    let ahead = environment_image_uv(DMat3::IDENTITY, DVec3::Z);
    set_locator(&mut e, locator, |_, t| {
        t.rotation.y = 90f64.to_radians();
        t.translation = DVec3::new(3.0, -2.0, 5.0);
        t.scale = DVec3::new(2.0, 0.5, 4.0);
    });
    let to_image = sky_to_image(&e);
    assert!(uv_close(environment_image_uv(to_image, DVec3::X), ahead));
    e.dispatch(Command::Undo).unwrap();
    assert!(sky_to_image(&e).abs_diff_eq(DMat3::IDENTITY, 1e-12));
}

/// Rotation X and Z tilt the picture, in the locator's rotation order.
#[test]
fn rotation_order_changes_the_tilt() {
    let (mut e, _, locator) = scene_with_hdri();
    let angles = DVec3::new(30f64, 90., 0.).map(f64::to_radians);
    set_locator(&mut e, locator, |_, t| t.rotation = angles);
    let zxy = sky_to_image(&e);
    // The picture's frame is the locator's rotation undone.
    let want = DMat3::from_quat(RotationOrder::Zxy.quat(angles)).inverse();
    assert!(zxy.abs_diff_eq(want, 1e-9));
    set_locator(&mut e, locator, |_, t| {
        t.set_rotation_order(RotationOrder::Yxz)
    });
    let yxz = sky_to_image(&e);
    let want = DMat3::from_quat(RotationOrder::Yxz.quat(angles)).inverse();
    assert!(yxz.abs_diff_eq(want, 1e-9));
    assert!(!yxz.abs_diff_eq(zxy, 1e-3));
}

/// Projection Axis picks the picture's pole: with Z, the top row of the
/// picture is seen straight along +Z; with X, along +X.
#[test]
fn projection_axis_is_the_pole() {
    let (mut e, _, locator) = scene_with_hdri();
    for (axis, pole) in [(0u8, DVec3::X), (1, DVec3::Y), (2, DVec3::Z)] {
        set_locator(&mut e, locator, |s, _| s.axis = axis);
        let to_image = sky_to_image(&e);
        assert!(
            environment_image_uv(to_image, pole)[1].abs() < 1e-9,
            "axis {axis}"
        );
        assert!((environment_image_uv(to_image, -pole)[1] - 1.0).abs() < 1e-9);
        // A proper rotation: the picture isn't mirrored.
        assert!((to_image.determinant() - 1.0).abs() < 1e-9);
    }
}

/// The locator's transform, order and projection save and reopen; the
/// file is version 43 (no XYZ item the v44 reading turns differently).
#[test]
fn locator_saves_and_reopens() {
    let (mut e, env, locator) = scene_with_hdri();
    set_locator(&mut e, locator, |s, t| {
        t.rotation = DVec3::new(10f64, 90., -20.).map(f64::to_radians);
        t.set_rotation_order(RotationOrder::Yzx);
        s.axis = 2;
    });
    for item in &mut e.scene.items {
        if item.transform.xyz_meaning_changed() {
            item.transform.rotation = DVec3::ZERO;
        }
    }
    let bytes = persistence::encode(&e.scene).unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(doc["version"], 43);
    let opened = persistence::decode(&bytes).unwrap();
    assert_eq!(opened, e.scene);
    let (l, heading) = opened.environment_image_heading(env).unwrap();
    assert_eq!(l, locator);
    assert!((heading - 90f64.to_radians()).abs() < 1e-12);
}

/// Older files: a v42 image layer's Spherical locator stays Spherical; a
/// Light Probe saved before v44 (it drew as latitude-longitude, and was
/// this branch's unreleased default) becomes Spherical, so the picture
/// stays where it was; a layer saved without a locator gets a Spherical
/// one at identity.
#[test]
fn older_files_keep_the_sky_latitude_longitude() {
    let (mut e, env, locator) = scene_with_hdri();
    set_locator(&mut e, locator, |_, t| t.rotation.y = 45f64.to_radians());
    let before = sky_to_image(&e);
    let mut doc: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&e.scene).unwrap()).unwrap();
    for (version, saved) in [(42, "Spherical"), (42, "LightProbe"), (43, "LightProbe")] {
        doc["version"] = version.into();
        for item in doc["scene"]["items"].as_array_mut().unwrap() {
            if item["id"] == locator {
                item["texture"]["Locator"]["projection"] = saved.into();
            }
        }
        let opened = persistence::decode(&serde_json::to_vec(&doc).unwrap()).unwrap();
        assert_eq!(
            opened.texture_locator(locator).unwrap().projection,
            Projection::Spherical,
            "v{version} {saved}"
        );
        let sky = opened
            .environment_sky(operandi_core::environment::RayKind::Camera)
            .unwrap();
        assert!(!sky.light_probe);
        assert!(sky.to_image.abs_diff_eq(before, 1e-12));
    }

    // The same layer written without a locator.
    for item in doc["scene"]["items"].as_array_mut().unwrap() {
        if item["id"] == env {
            for layer in item["environment"]["layers"].as_array_mut().unwrap() {
                layer["kind"]["Texture"]["locator"] = serde_json::Value::Null;
            }
        }
    }
    let opened = persistence::decode(&serde_json::to_vec(&doc).unwrap()).unwrap();
    let (fresh, heading) = opened.environment_image_heading(env).unwrap();
    assert_eq!(heading, 0.0);
    let s = opened.texture_locator(fresh).unwrap();
    assert_eq!((s.projection, s.axis), (Projection::Spherical, 1));
    assert_eq!(
        opened
            .environment_sky(operandi_core::environment::RayKind::Camera)
            .unwrap()
            .to_image,
        DMat3::IDENTITY
    );
}

/// Light Probe draws the picture as a mirror ball and saves as v44, which
/// keeps it on reopening. Rotation turns it like Spherical.
#[test]
fn light_probe_is_a_mirror_ball_and_reopens() {
    let (mut e, _, locator) = scene_with_hdri();
    set_locator(&mut e, locator, |s, t| {
        s.projection = Projection::LightProbe;
        t.rotation.y = 90f64.to_radians();
    });
    let sky = e
        .scene
        .environment_sky(operandi_core::environment::RayKind::Camera)
        .unwrap();
    assert!(sky.light_probe);
    // What the ball's centre showed (+Z) is now seen along +X.
    let seen = environment_probe_uv(sky.to_image, DVec3::X);
    assert!(uv_close(seen, [0.5, 0.5]), "{seen:?}");
    let bytes = persistence::encode(&e.scene).unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(doc["version"], 44);
    let opened = persistence::decode(&bytes).unwrap();
    assert_eq!(
        opened.texture_locator(locator).unwrap().projection,
        Projection::LightProbe
    );
}

/// Version 44 (ETL-Q01): an item saved in the old Z-first XYZ opens in
/// ZYX, which turns the same way, so every world matrix (children too) is
/// unchanged. XYZ items the two readings agree on (one angle or none)
/// stay XYZ; a scene with an XYZ item turned about two axes saves as v44
/// and reopens in XYZ.
#[test]
fn version_44_moves_old_xyz_items_to_zyx() {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    scene.new_mesh().unwrap();
    scene.new_mesh().unwrap();
    let ids: Vec<u64> = scene.items.iter().map(|i| i.id).collect();
    scene.set_parent(ids[1], Some(ids[0]), false).unwrap();
    // Held as ZYX in memory: that is how the old XYZ turned.
    scene.items[0].transform.rotation = DVec3::new(0.3, -0.5, 0.7);
    scene.items[0]
        .transform
        .set_rotation_order(RotationOrder::Zyx);
    scene.items[1].transform.rotation = DVec3::new(-0.2, 0.0, 1.1);
    scene.items[1].transform.translation = DVec3::new(1.0, 2.0, 3.0);
    scene.items[1]
        .transform
        .set_rotation_order(RotationOrder::Zyx);
    scene.items[2].transform.rotation = DVec3::new(0.0, 0.9, 0.0);
    let before: Vec<_> = ids
        .iter()
        .map(|&id| scene.world_matrix(id).unwrap())
        .collect();
    let mut doc: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&scene).unwrap()).unwrap();
    // Write it the way a v43 file holds the old XYZ: no `order`.
    doc["version"] = 43.into();
    for item in doc["scene"]["items"].as_array_mut().unwrap() {
        item["transform"].as_object_mut().unwrap().remove("order");
    }
    let opened = persistence::decode(&serde_json::to_vec(&doc).unwrap()).unwrap();
    for (k, &id) in ids.iter().enumerate() {
        let after = opened.world_matrix(id).unwrap();
        assert!(after.abs_diff_eq(before[k], 1e-12), "item {k}");
    }
    let order = |s: &Scene, k: usize| s.items[k].transform.rotation_order();
    assert_eq!(order(&opened, 0), RotationOrder::Zyx);
    assert_eq!(order(&opened, 1), RotationOrder::Zyx);
    assert_eq!(order(&opened, 2), RotationOrder::Xyz);

    // A new XYZ turn about two axes is X-first and survives a v44 save.
    let mut fresh = opened.clone();
    fresh.items[2].transform.rotation = DVec3::new(0.4, 0.9, 0.0);
    let want = fresh.world_matrix(ids[2]).unwrap();
    let bytes = persistence::encode(&fresh).unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(doc["version"], 44);
    let reopened = persistence::decode(&bytes).unwrap();
    assert_eq!(order(&reopened, 2), RotationOrder::Xyz);
    assert!(
        reopened
            .world_matrix(ids[2])
            .unwrap()
            .abs_diff_eq(want, 1e-12)
    );
}

/// The rotOrder channel offers all six orders, XYZ and ZXY keeping their
/// old indices.
#[test]
fn rot_order_channel_takes_six_orders() {
    let (mut e, _, locator) = scene_with_hdri();
    use channels::{Target, get, set};
    let target = Target::Item(locator);
    for (index, order) in RotationOrder::ALL.into_iter().enumerate() {
        set(&mut e, target, "rotOrder", order.label())
            .unwrap_or_else(|err| panic!("{order:?}: {err}"));
        let t = &e
            .scene
            .items
            .iter()
            .find(|i| i.id == locator)
            .unwrap()
            .transform;
        assert_eq!(t.rotation_order(), order, "index {index}");
        assert_eq!(get(&e.scene, target, "rotOrder").unwrap(), order.label());
        set(&mut e, target, "rotOrder", &index.to_string()).unwrap();
        assert_eq!(get(&e.scene, target, "rotOrder").unwrap(), order.label());
    }
}
