//! Light properties Tier 2 (specifications/light-properties-2026-11-06):
//! Targeting (LP-04), Cylinder Light (LP-CY), Mesh Light (LP-MS),
//! Photometric Light (LP-PH) and Physical Sun (LP-SUN).
use glam::DVec3;
use operandi_core::camera_light::*;
use operandi_core::item_common::*;
use operandi_core::item_target::aim_rotation;
use operandi_core::photometry::{LUMINOUS_EFFICACY, Photometry};
use operandi_core::physical_sun::*;
use operandi_core::*;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/photometry/operandi-downlight.ies"
);

fn new_scene() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.add_default_camera_and_light().unwrap();
    e.next_item_id = e.scene.next_id;
    e
}

fn new_light(e: &mut Editor, kind: LightKind) -> u64 {
    e.dispatch(Command::NewLight(kind)).unwrap();
    e.scene.selection.item.unwrap()
}

fn light(e: &Editor, id: u64) -> &LightSettings {
    e.scene
        .items
        .iter()
        .find(|i| i.id == id)
        .unwrap()
        .light
        .as_ref()
        .unwrap()
}

fn edit_light(e: &mut Editor, id: u64, edit: LightEdit) {
    e.dispatch(Command::EditLight { item: id, edit }).unwrap();
}

fn common(e: &mut Editor, id: u64, edit: ItemCommonEdit) -> Result<()> {
    e.dispatch(Command::EditItemCommon {
        items: vec![id],
        edit,
    })
}

fn place(e: &mut Editor, id: u64, at: DVec3) {
    e.scene
        .items
        .iter_mut()
        .find(|i| i.id == id)
        .unwrap()
        .transform
        .translation = at;
}

fn forward(e: &Editor, id: u64) -> DVec3 {
    item_frame(&e.scene, id).unwrap().forward
}

fn last_label(e: &Editor) -> &str {
    &e.undo.last().unwrap().label
}

fn round_trip(scene: &Scene) -> Scene {
    let bytes = persistence::encode(scene).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("\"version\": 33"));
    persistence::decode(&bytes).unwrap()
}

// --- Targeting (LP-04) -----------------------------------------------------

#[test]
fn aim_rotation_points_minus_z_with_y_up() {
    for f in [
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(0.3, -0.8, 0.5),
        DVec3::new(-2.0, 1.0, -1.0),
    ] {
        let q = aim_rotation(f, 0.0).unwrap();
        assert!((q * -DVec3::Z - f.normalize()).length() < 1e-12);
        // No roll: right stays level, up points up.
        assert!((q * DVec3::X).y.abs() < 1e-12);
        assert!((q * DVec3::Y).y > 0.0);
        // Roll turns up about the aim.
        let r = aim_rotation(f, std::f64::consts::FRAC_PI_2).unwrap();
        assert!((r * -DVec3::Z - f.normalize()).length() < 1e-12);
        assert!((r * DVec3::Y - q * -DVec3::X).length() < 1e-9);
    }
    assert!(aim_rotation(DVec3::ZERO, 0.0).is_none());
}

#[test]
fn a_targeted_light_keeps_aiming_at_its_target() {
    let mut e = new_scene();
    let mesh = e.scene.items[0].id;
    let spot = new_light(&mut e, LightKind::Spot);
    place(&mut e, spot, DVec3::new(3.0, 4.0, 0.0));
    let steps = e.undo.len();
    common(&mut e, spot, ItemCommonEdit::Target(Some(mesh))).unwrap();
    assert_eq!(last_label(&e), "Set Target");
    assert_eq!(e.undo.len(), steps + 1);
    let want = (DVec3::ZERO - DVec3::new(3.0, 4.0, 0.0)).normalize();
    assert!((forward(&e, spot) - want).length() < 1e-9);
    assert!(e.scene.rotation_driven(spot));
    // Moving the target re-aims; the light's own rotation stays stored.
    place(&mut e, mesh, DVec3::new(3.0, 0.0, -4.0));
    assert!((forward(&e, spot) - DVec3::new(0.0, -1.0, -1.0).normalize()).length() < 1e-9);
    // Roll, Enable off (own rotation back), Remove Target.
    common(&mut e, spot, ItemCommonEdit::TargetRoll(0.5)).unwrap();
    assert_eq!(last_label(&e), "Target Roll");
    assert!((forward(&e, spot) - DVec3::new(0.0, -1.0, -1.0).normalize()).length() < 1e-9);
    common(&mut e, spot, ItemCommonEdit::TargetEnable(false)).unwrap();
    assert_eq!(last_label(&e), "Target Enable");
    assert!((forward(&e, spot) + DVec3::Z).length() < 1e-12);
    assert!(!e.scene.rotation_driven(spot));
    common(&mut e, spot, ItemCommonEdit::Target(None)).unwrap();
    assert_eq!(last_label(&e), "Remove Target");
    // Undo back to the aimed state.
    for _ in 0..3 {
        e.dispatch(Command::Undo).unwrap();
    }
    let t = e.scene.items.iter().find(|i| i.id == spot).unwrap();
    assert_eq!(t.common.target.as_ref().unwrap().item, mesh);
    assert!(t.common.target.as_ref().unwrap().enable);
}

#[test]
fn set_target_refuses_itself_children_and_locked_items() {
    let mut e = new_scene();
    let mesh = e.scene.items[0].id;
    let camera = e.scene.items[1].id;
    let area = new_light(&mut e, LightKind::Area);
    e.scene.set_parent(camera, Some(area), false).unwrap();
    assert!(common(&mut e, area, ItemCommonEdit::Target(Some(area))).is_err());
    assert!(common(&mut e, area, ItemCommonEdit::Target(Some(camera))).is_err());
    assert!(!e.scene.target_candidates(area).contains(&camera));
    assert!(e.scene.target_candidates(area).contains(&mesh));
    assert!(
        common(&mut e, area, ItemCommonEdit::TargetRoll(1.0)).is_err(),
        "no target yet"
    );
    common(&mut e, area, ItemCommonEdit::Lock(Choice::Yes)).unwrap();
    assert!(common(&mut e, area, ItemCommonEdit::Target(Some(mesh))).is_err());
}

#[test]
fn children_follow_and_mutual_targets_end() {
    let mut e = new_scene();
    let mesh = e.scene.items[0].id;
    let camera = e.scene.items[1].id;
    let a = new_light(&mut e, LightKind::Directional);
    let b = new_light(&mut e, LightKind::Spot);
    place(&mut e, a, DVec3::new(0.0, 5.0, 0.0));
    place(&mut e, b, DVec3::new(5.0, 0.0, 0.0));
    // A child 1 m down the aimed light's −Z follows the aim.
    e.scene.set_parent(camera, Some(a), false).unwrap();
    e.scene.items[1].transform = Transform {
        translation: DVec3::new(0.0, 0.0, -1.0),
        ..Default::default()
    };
    common(&mut e, a, ItemCommonEdit::Target(Some(mesh))).unwrap();
    let child = item_frame(&e.scene, camera).unwrap().position;
    assert!((child - DVec3::new(0.0, 4.0, 0.0)).length() < 1e-9);
    // Two lights aiming at each other: each aims at the other's place.
    common(&mut e, a, ItemCommonEdit::Target(Some(b))).unwrap();
    common(&mut e, b, ItemCommonEdit::Target(Some(a))).unwrap();
    let ab = (DVec3::new(5.0, 0.0, 0.0) - DVec3::new(0.0, 5.0, 0.0)).normalize();
    assert!((forward(&e, a) - ab).length() < 1e-9);
    assert!((forward(&e, b) + ab).length() < 1e-9);
    e.scene.validate().unwrap();
}

#[test]
fn targets_save_and_a_deleted_target_is_ignored() {
    let mut e = new_scene();
    let mesh = e.scene.items[0].id;
    let spot = new_light(&mut e, LightKind::Spot);
    place(&mut e, spot, DVec3::new(0.0, 3.0, 0.0));
    common(&mut e, spot, ItemCommonEdit::Target(Some(mesh))).unwrap();
    common(&mut e, spot, ItemCommonEdit::TargetRoll(0.25)).unwrap();
    assert_eq!(round_trip(&e.scene), e.scene);
    e.scene.items.retain(|i| i.id != mesh);
    assert!((forward(&e, spot) + DVec3::Z).length() < 1e-12);
    assert!(!e.scene.rotation_driven(spot));
}

#[test]
fn targeting_rows_by_kind() {
    // LP-04: not on Point, Dome or Mesh Light.
    for kind in LightKind::ALL {
        assert_eq!(
            kind.has_targeting(),
            !matches!(kind, LightKind::Point | LightKind::Mesh | LightKind::Dome),
            "{kind:?}"
        );
    }
}

// --- Cylinder Light (LP-CY) ------------------------------------------------

#[test]
fn cylinder_defaults_and_rows() {
    let mut e = new_scene();
    let id = new_light(&mut e, LightKind::Cylinder);
    let item = e.scene.items.iter().find(|i| i.id == id).unwrap();
    assert_eq!(item.name, "Cylinder Light");
    let l = light(&e, id);
    assert_eq!((l.radius, l.length), (0.2, 1.0));
    assert_eq!(l.intensity, 3.0);
    assert_eq!(l.kind.intensity_label(), "Radiance");
    edit_light(&mut e, id, LightEdit::Length(2.5));
    assert_eq!(last_label(&e), "Length");
    assert!(
        e.dispatch(Command::EditLight {
            item: id,
            edit: LightEdit::Length(0.0),
        })
        .is_err()
    );
    let back = round_trip(&e.scene);
    assert_eq!(back, e.scene);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(light(&e, id).length, 1.0);
}

// --- Mesh Light (LP-MS) ----------------------------------------------------

#[test]
fn mesh_light_prototype_rows() {
    let mut e = new_scene();
    let mesh = e.scene.items[0].id;
    let camera = e.scene.items[1].id;
    let id = new_light(&mut e, LightKind::Mesh);
    let l = light(&e, id);
    assert_eq!(l.prototype, None, "emits nothing until one is picked");
    assert!(l.use_prototype_transform);
    assert!(
        e.dispatch(Command::EditLight {
            item: id,
            edit: LightEdit::Prototype(Some(camera)),
        })
        .is_err(),
        "only meshes"
    );
    edit_light(&mut e, id, LightEdit::Prototype(Some(mesh)));
    assert_eq!(last_label(&e), "Prototype");
    edit_light(&mut e, id, LightEdit::UsePrototypeTransform(false));
    assert_eq!(last_label(&e), "Use Prototype Transform");
    // Several mesh lights may share one prototype.
    let other = new_light(&mut e, LightKind::Mesh);
    edit_light(&mut e, other, LightEdit::Prototype(Some(mesh)));
    assert_eq!(round_trip(&e.scene), e.scene);
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert!(light(&e, id).use_prototype_transform);
}

// --- Photometric Light (LP-PH) ---------------------------------------------

#[test]
fn ies_fixture_parses() {
    let p = Photometry::load(std::path::Path::new(FIXTURE)).unwrap();
    assert_eq!(p.vertical, [0.0, 45.0, 90.0, 135.0, 180.0]);
    assert_eq!(p.horizontal, [0.0]);
    assert_eq!(p.peak_candela(), 1000.0);
    assert_eq!((p.width, p.length), (0.3, 0.2));
    assert!((p.radiant_intensity() - 1000.0 / LUMINOUS_EFFICACY).abs() < 1e-12);
    // Between angles it's linear; all round the same (one horizontal).
    assert!((p.sample(22.5, 0.0) - 900.0).abs() < 1e-9);
    assert!((p.sample(22.5, 123.0) - 900.0).abs() < 1e-9);
    assert_eq!(p.sample(180.0, 0.0), 0.0);
}

#[test]
fn ies_units_multipliers_and_bad_files() {
    let feet = "IESNA:LM-63-1995\nTILT=NONE\n1 -1 2 3 2 1 1 1 2 0\n0.5 1 10\n0 90 180\n0 90\n10 5 0\n20 5 0\n";
    let p = Photometry::parse(feet).unwrap();
    // Multiplier 2 × ballast 0.5; feet to metres.
    assert_eq!(p.candela, [vec![10.0, 5.0, 0.0], vec![20.0, 5.0, 0.0]]);
    assert!((p.width - 0.3048).abs() < 1e-12 && (p.length - 0.6096).abs() < 1e-12);
    // Two horizontal angles (0, 90): quadrant symmetry.
    assert!((p.sample(0.0, 45.0) - 15.0).abs() < 1e-9);
    assert!((p.sample(0.0, 135.0) - 15.0).abs() < 1e-9);
    for bad in [
        "no tilt here",
        "TILT=NONE\n1 1000 1 5",
        "TILT=lamp.tlt\n",
        "TILT=NONE\n1 1000 1 2 1 1 2 0 0 0\n1 1 1\n0 90\n0\n10 x\n",
        "TILT=NONE\n1 1000 1 2 1 1 2 0 0 0\n1 1 1\n90 0\n0\n10 5\n",
    ] {
        assert!(Photometry::parse(bad).is_err(), "{bad}");
    }
}

#[test]
fn photometric_defaults_and_loading_a_file() {
    let mut e = new_scene();
    let id = new_light(&mut e, LightKind::Photometric);
    let l = light(&e, id).clone();
    assert_eq!(l.intensity, 3.0);
    assert_eq!((l.width, l.height), (0.0, 0.0));
    assert!((l.cone_angle - std::f64::consts::TAU).abs() < 1e-12);
    assert_eq!(l.soft_edge, 0.0);
    assert_eq!(l.kind.intensity_label(), "Radiant Intensity");
    // LP-PH-02: loading sets Radiant Intensity and Width/Height.
    e.dispatch(Command::LoadPhotometry {
        item: id,
        path: FIXTURE.into(),
    })
    .unwrap();
    assert_eq!(last_label(&e), "Photometry File");
    let l = light(&e, id);
    assert_eq!(l.photometry_file, FIXTURE);
    assert!((l.intensity - 1000.0 / LUMINOUS_EFFICACY).abs() < 1e-12);
    assert_eq!((l.width, l.height), (0.3, 0.2));
    // Only the path is saved, not the file.
    let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(text.contains("operandi-downlight.ies") && !text.contains("LM-63"));
    assert_eq!(round_trip(&e.scene), e.scene);
    // A bad path changes nothing and adds no step.
    let steps = e.undo.len();
    assert!(
        e.dispatch(Command::LoadPhotometry {
            item: id,
            path: "/nonexistent/x.ies".into(),
        })
        .is_err()
    );
    assert_eq!(e.undo.len(), steps);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(light(&e, id), &LightSettings::new(LightKind::Photometric));
    // Only photometric lights take a file.
    let spot = new_light(&mut e, LightKind::Spot);
    assert!(
        e.dispatch(Command::LoadPhotometry {
            item: spot,
            path: FIXTURE.into(),
        })
        .is_err()
    );
}

// --- Physical Sun (LP-SUN) -------------------------------------------------

fn sun_at(month: u32, day: u32, time: f64) -> PhysicalSun {
    PhysicalSun {
        enabled: true,
        month,
        day,
        time,
        ..Default::default()
    }
}

/// The highest elevation over a day and when it happens (hours).
fn noon(sun: &PhysicalSun) -> (f64, f64) {
    (0..=24 * 60)
        .map(|m| {
            let s = PhysicalSun {
                time: (m as f64 / 60.0).min(23.999),
                ..sun.clone()
            };
            (s.position().elevation, s.time)
        })
        .fold((f64::MIN, 0.0), |a, b| if b.0 > a.0 { b } else { a })
}

#[test]
fn sun_defaults_match_the_spec() {
    let s = PhysicalSun::default();
    assert!(!s.enabled);
    assert_eq!((s.time, s.month, s.day), (12.0, 6, 21));
    assert!(!s.daylight_saving);
    assert_eq!((s.haze, s.clamp, s.gamma), (3.0, SunClamp::Clamp, 2.2));
    assert_eq!(s.north_offset, 0.0);
    assert_eq!(
        (s.latitude, s.longitude, s.time_zone),
        (37.53, -122.30, -8.0)
    );
    assert_eq!(s.location(), Some("San Francisco"));
    assert_eq!(s.day_of_year(), 172);
}

#[test]
fn sun_position_matches_known_values() {
    // San Francisco, June solstice: highest at 90° − 37.53° + 23.44° ≈
    // 75.9°, at local solar noon: 12:00 + 4 min/° × 2.3° west of the zone's
    // meridian + 1.5 min equation of time ≈ 12:11 PST.
    let (elev, at) = noon(&sun_at(6, 21, 12.0));
    assert!((elev - 75.9).abs() < 0.3, "{elev}");
    assert!((at - 12.18).abs() < 0.05, "{at}");
    // December solstice: 90 − 37.53 − 23.44 ≈ 29.0°, near 12:01 PST (the
    // equation of time is +2 min then).
    let (elev, at) = noon(&sun_at(12, 21, 12.0));
    assert!((elev - 29.0).abs() < 0.3, "{elev}");
    assert!((at - 12.13).abs() < 0.05, "{at}");
    // At solar noon the sun is due south (azimuth 180°) at this latitude.
    let s = PhysicalSun {
        time: noon(&sun_at(3, 20, 12.0)).1,
        ..sun_at(3, 20, 12.0)
    };
    let p = s.position();
    assert!((p.azimuth - 180.0).abs() < 1.0, "{}", p.azimuth);
    assert!(
        (p.elevation - (90.0 - 37.53)).abs() < 0.6,
        "{}",
        p.elevation
    );
    // Morning sun in the east, afternoon in the west; midnight below.
    assert!((60.0..120.0).contains(&sun_at(3, 20, 7.0).position().azimuth));
    assert!((240.0..300.0).contains(&sun_at(3, 20, 17.5).position().azimuth));
    assert!(sun_at(6, 21, 0.0).position().elevation < 0.0);
    // The equator at the March equinox: nearly overhead at solar noon.
    let quito = PhysicalSun {
        latitude: 0.0,
        longitude: 0.0,
        time_zone: 0.0,
        ..sun_at(3, 20, 12.0)
    };
    assert!(noon(&quito).0 > 89.0);
    // Southern hemisphere: a June noon sun is in the north.
    let sydney = PhysicalSun {
        latitude: -33.87,
        longitude: 151.21,
        time_zone: 10.0,
        ..sun_at(6, 21, 12.0)
    };
    let p = sydney.position();
    assert!(p.azimuth < 30.0 || p.azimuth > 330.0, "{}", p.azimuth);
    assert!((noon(&sydney).0 - (90.0 - 33.87 - 23.44)).abs() < 0.4);
}

#[test]
fn daylight_saving_shifts_the_clock_an_hour() {
    let standard = sun_at(6, 21, 12.0).position();
    let dst = PhysicalSun {
        daylight_saving: true,
        time: 13.0,
        ..sun_at(6, 21, 12.0)
    }
    .position();
    assert!((standard.azimuth - dst.azimuth).abs() < 1e-9);
    assert!((standard.elevation - dst.elevation).abs() < 1e-9);
}

#[test]
fn physical_sun_aims_the_light_and_greys_rotation() {
    let mut e = new_scene();
    let id = e.scene.items[2].id;
    let before = forward(&e, id);
    edit_light(&mut e, id, LightEdit::Sun(SunEdit::Enabled(true)));
    assert_eq!(last_label(&e), "Physical Sun");
    assert!(e.scene.rotation_driven(id));
    let sun = light(&e, id).sun.sun_direction();
    assert!(
        (forward(&e, id) + sun).length() < 1e-9,
        "shines from the sun"
    );
    assert!((forward(&e, id) - before).length() > 1e-3);
    // North Offset turns it about +Y.
    edit_light(&mut e, id, LightEdit::Sun(SunEdit::NorthOffset(0.5)));
    let turned = glam::DQuat::from_rotation_y(0.5) * sun;
    assert!((forward(&e, id) + turned).length() < 1e-9);
    // Location fills latitude, longitude and time zone (one step).
    let tokyo = LOCATIONS.iter().position(|l| l.0 == "Tokyo").unwrap();
    edit_light(&mut e, id, LightEdit::Sun(SunEdit::Location(tokyo)));
    assert_eq!(last_label(&e), "Location");
    let s = &light(&e, id).sun;
    assert_eq!((s.latitude, s.longitude, s.time_zone), (35.68, 139.69, 9.0));
    assert_eq!(s.location(), Some("Tokyo"));
    // Invalid rows change nothing.
    for bad in [
        SunEdit::Time(24.0),
        SunEdit::Day(2, 30),
        SunEdit::Haze(0.5),
        SunEdit::Latitude(95.0),
        SunEdit::Location(99),
    ] {
        assert!(
            e.dispatch(Command::EditLight {
                item: id,
                edit: LightEdit::Sun(bad),
            })
            .is_err()
        );
    }
    assert_eq!(round_trip(&e.scene), e.scene);
    // Off again: its own rotation is back.
    edit_light(&mut e, id, LightEdit::Sun(SunEdit::Enabled(false)));
    assert!((forward(&e, id) - before).length() < 1e-12);
    // Only directional lights have a Physical Sun.
    let spot = new_light(&mut e, LightKind::Spot);
    edit_light(&mut e, spot, LightEdit::Sun(SunEdit::Enabled(true)));
    assert!(!e.scene.rotation_driven(spot));
}

#[test]
fn clamp_intensity_modes() {
    let s = sun_at(6, 21, 12.2);
    let clamp = s.strength(3.0);
    assert!(clamp > 0.0 && clamp <= 3.0);
    // Clamp follows the sun's height, at most Radiant Exitance.
    let low = sun_at(6, 21, 18.5).strength(3.0);
    assert!(low < clamp, "{low} vs {clamp}");
    let replace = PhysicalSun {
        clamp: SunClamp::Replace,
        ..s.clone()
    };
    assert_eq!(replace.strength(3.0), 3.0);
    let none = PhysicalSun {
        clamp: SunClamp::None,
        ..s.clone()
    };
    // Real sunlight near noon: several hundred W/m².
    assert!(
        (700.0..1200.0).contains(&none.strength(3.0)),
        "{}",
        none.strength(3.0)
    );
    // More haze, less sun.
    let hazy = PhysicalSun {
        haze: 8.0,
        clamp: SunClamp::None,
        ..s.clone()
    };
    assert!(hazy.strength(3.0) < none.strength(3.0));
    // Night: no sunlight in any mode.
    for clamp in SunClamp::ALL {
        let night = PhysicalSun {
            clamp,
            ..sun_at(6, 21, 0.5)
        };
        assert_eq!(night.strength(3.0), 0.0);
    }
}

#[test]
fn tier2_lights_need_version_16() {
    for kind in [LightKind::Cylinder, LightKind::Mesh, LightKind::Photometric] {
        let mut e = new_scene();
        new_light(&mut e, kind);
        let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
        assert!(text.contains("\"version\": 33"), "{kind:?}");
        let old = text.replacen("\"version\": 33", "\"version\": 15", 1);
        assert!(persistence::decode(old.as_bytes()).is_err(), "{kind:?}");
    }
}
