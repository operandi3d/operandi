//! CL-1 camera and directional light items (spec v0, provisional): new
//! scene contents, defaults, the Angle of View formula, naming, item-mode
//! only editing, undo, save/reopen, and that exports and LXO round trips
//! don't invent or keep them.
use glam::DVec3;
use operandi_core::camera_light::*;
use operandi_core::camera_light_defaults as defaults;
use operandi_core::*;

/// What `Operandi::initialize_new_document` builds: a mesh, then the
/// default camera and light, with the mesh selected.
fn new_scene() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.add_default_camera_and_light().unwrap();
    e.next_item_id = e.scene.next_id;
    e
}

fn camera_id(e: &Editor) -> u64 {
    e.scene
        .items
        .iter()
        .find(|i| i.camera.is_some())
        .unwrap()
        .id
}
fn light_id(e: &Editor) -> u64 {
    e.scene.items.iter().find(|i| i.light.is_some()).unwrap().id
}

#[test]
fn new_scene_has_one_camera_and_one_directional_light() {
    let e = new_scene();
    let names: Vec<_> = e.scene.items.iter().map(|i| i.name.as_str()).collect();
    // ENV §1: and the default Environment.
    assert_eq!(
        names,
        ["Mesh", "Camera", "Directional Light", "Environment"]
    );
    assert_eq!(e.scene.selection.item, Some(1), "the mesh stays selected");
    let camera = &e.scene.items[1];
    assert_eq!(camera.camera.as_ref(), Some(&new_scene_camera_settings()));
    assert_eq!(e.scene.render_camera(), Some(camera.id));
    // Observed (spec v1): at (0, 0.75, 4), pitched down 5°, Film Fit Fill.
    let f = item_frame(&e.scene, camera.id).unwrap();
    assert!((f.position - DVec3::new(0.0, 0.75, 4.0)).length() < 1e-12);
    let pitch = (-5f64).to_radians();
    assert!((f.forward - DVec3::new(0.0, pitch.sin(), -pitch.cos())).length() < 1e-12);
    assert_eq!(camera.camera.as_ref().unwrap().film_fit, FilmFit::Fill);
    // The light: at (−2, 2, 2), X −30°, Y −45°, ZXY order.
    let light = &e.scene.items[2];
    assert!(light.transform.zxy);
    let f = item_frame(&e.scene, light.id).unwrap();
    assert!((f.position - DVec3::new(-2.0, 2.0, 2.0)).length() < 1e-12);
    let (s30, c30, s45) = (0.5, 0.75f64.sqrt(), 0.5f64.sqrt());
    assert!(
        (f.forward - DVec3::new(c30 * s45, -s30, -c30 * s45)).length() < 1e-12,
        "{}",
        f.forward
    );
    let light = &e.scene.items[2];
    assert_eq!(light.light.as_ref(), Some(&LightSettings::default()));
    assert!(
        item_frame(&e.scene, light.id).unwrap().forward.y < 0.0,
        "shines downward"
    );
    e.scene.validate().unwrap();
}

#[test]
fn defaults_match_the_spec_table() {
    let c = CameraSettings::default();
    assert_eq!(c.projection, Projection::Perspective);
    assert_eq!((c.film_width, c.film_height), (0.036, 0.024));
    assert_eq!(c.film_offset, [0.0, 0.0]);
    assert_eq!(c.target_distance, 4.0);
    assert!(!c.clipping);
    assert_eq!(c.focal_length, 0.050);
    assert_eq!(c.film_fit, FilmFit::Fill);
    assert_eq!(c.clip_near, 4.0, "Clipping Distance");
    assert_eq!(
        (
            c.stored.lens_squeeze,
            c.stored.film_roll,
            c.stored.display_size,
            c.stored.overscan
        ),
        (1.0, 0.0, 0.0, 1.0)
    );
    let l = LightSettings::default();
    assert_eq!(l.render, LightRender::Default);
    assert_eq!(l.dissolve, 0.0);
    assert_eq!(l.shadow, ShadowType::RayTraced);
    assert!(!l.visible_to_camera);
    assert_eq!(l.spread_angle, 0.0);
    assert_eq!(l.intensity, 3.0);
    let st = &l.stored;
    assert!(!st.visible_to_reflection && !st.visible_to_refraction);
    assert_eq!((st.samples, st.importance, st.display_size), (64, 1.0, 1.0));
    assert_eq!(st.shadow_map_size, 2.0);
    assert_eq!(st.shadow_color, [0.0; 3]);
    assert_eq!(
        (st.affect_caustics, st.affect_subsurface, st.opacity),
        (1.0, 1.0, 1.0)
    );
    assert!(st.material_enabled);
    assert_eq!(l.color, [1.0; 3]);
    assert_eq!((l.affect_diffuse, l.affect_specular), (1.0, 1.0));
}

#[test]
fn angle_of_view_is_derived_from_film_width_and_focal_length() {
    let c = CameraSettings::default();
    let expected = 2.0 * (0.036f64 / (2.0 * 0.050)).atan();
    assert!((c.angle_of_view() - expected).abs() < 1e-15);
    // About 39.6° for a 50 mm lens on 36 mm film.
    assert!((c.angle_of_view().to_degrees() - 39.597_752_5).abs() < 1e-6);
    assert!((angle_of_view(0.018, 0.036).to_degrees() - 90.0).abs() < 1e-12);
    // Observed values (spec v1), 36 mm film.
    for (focal_mm, degrees) in [(35.0, 54.432223), (50.0, 39.597753), (85.0, 23.913168)] {
        let aov = angle_of_view(focal_mm / 1000.0, 0.036).to_degrees();
        assert!((aov - degrees).abs() < 5e-7, "{focal_mm} mm: {aov}");
    }
}

#[test]
fn create_duplicate_and_names_follow_the_kind() {
    let mut e = new_scene();
    e.dispatch(Command::NewCamera).unwrap();
    let id = e.scene.selection.item.unwrap();
    assert_eq!(e.scene.active().unwrap().name, "Camera (2)");
    e.dispatch(Command::DuplicateItem).unwrap();
    // The existing Duplicate rule appends " (2)" to the source name.
    assert_eq!(e.scene.active().unwrap().name, "Camera (2) (2)");
    assert!(e.scene.active().unwrap().camera.is_some());
    e.dispatch(Command::NewLight(LightKind::Directional))
        .unwrap();
    assert_eq!(e.scene.active().unwrap().name, "Directional Light (2)");
    // One undo step each.
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.selection.item, Some(id));
    // Still one render camera: the first until another is chosen.
    let first = camera_id(&e);
    assert_eq!(e.scene.render_camera(), Some(first));
    e.dispatch(Command::SetRenderCamera(id)).unwrap();
    assert_eq!(e.scene.render_camera(), Some(id));
    assert!(e.dispatch(Command::SetRenderCamera(light_id(&e))).is_err());
    // Deleting it falls back to the remaining camera.
    e.dispatch(Command::SelectItem(Some(id))).unwrap();
    e.dispatch(Command::DeleteItem).unwrap();
    assert_eq!(e.scene.render_camera(), Some(first));
}

#[test]
fn cameras_and_lights_edit_in_item_mode_only() {
    let mut e = new_scene();
    let id = camera_id(&e);
    e.dispatch(Command::SelectItem(Some(id))).unwrap();
    assert_eq!(e.scene.selection.mode, Mode::Polygon);
    for tool in [Tool::Bevel, Tool::Extrude, Tool::Box, Tool::Tweak] {
        assert!(e.dispatch(Command::Activate(tool)).is_err(), "{tool:?}");
    }
    // Move switches to Item mode and moves the item itself.
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    assert_eq!(e.scene.selection.mode, Mode::Item);
    e.dispatch(Command::Cancel).unwrap();
}

#[test]
fn panel_edits_are_single_undo_steps_and_validated() {
    let mut e = new_scene();
    let (c, l) = (camera_id(&e), light_id(&e));
    let undo = e.history.undo.len();
    e.dispatch(Command::EditCamera {
        item: c,
        edit: CameraEdit::FocalLength(0.035),
    })
    .unwrap();
    e.dispatch(Command::EditLight {
        item: l,
        edit: LightEdit::Intensity(5.0),
    })
    .unwrap();
    assert_eq!(e.history.undo.len(), undo + 2);
    assert!(
        e.dispatch(Command::EditCamera {
            item: c,
            edit: CameraEdit::FilmWidth(-1.0),
        })
        .is_err()
    );
    assert!(
        e.dispatch(Command::EditLight {
            item: c,
            edit: LightEdit::Intensity(1.0),
        })
        .is_err(),
        "a camera has no light settings"
    );
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(
        e.scene
            .items
            .iter()
            .find(|i| i.id == l)
            .unwrap()
            .light
            .as_ref()
            .unwrap()
            .intensity,
        defaults::LIGHT_INTENSITY
    );
}

#[test]
fn camera_view_navigation_is_one_undo_step() {
    let mut e = new_scene();
    let id = camera_id(&e);
    let before = e.scene.clone();
    let undo = e.history.undo.len();
    let transform = scene::Transform {
        translation: DVec3::new(1.0, 2.0, 3.0),
        rotation: look_at_rotation(DVec3::new(1.0, 2.0, 3.0), DVec3::ZERO),
        ..Default::default()
    };
    e.dispatch(Command::CameraView {
        item: id,
        transform: transform.clone(),
        target_distance: 14f64.sqrt(),
    })
    .unwrap();
    assert_eq!(e.history.undo.len(), undo + 1);
    let item = e.scene.items.iter().find(|i| i.id == id).unwrap();
    assert_eq!(item.transform, transform);
    assert!((item.camera.as_ref().unwrap().target_distance - 14f64.sqrt()).abs() < 1e-12);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
}

#[test]
fn save_and_reopen_keeps_cameras_and_lights() {
    let mut e = new_scene();
    let id = camera_id(&e);
    e.dispatch(Command::EditCamera {
        item: id,
        edit: CameraEdit::FilmFit(FilmFit::Vertical),
    })
    .unwrap();
    e.dispatch(Command::SetRenderCamera(id)).unwrap();
    let bytes = persistence::encode(&e.scene).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("\"version\": 33"));
    assert_eq!(persistence::decode(&bytes).unwrap(), e.scene);
    // An older version can't carry them.
    let text = String::from_utf8_lossy(&bytes).replace("\"version\": 33", "\"version\": 12");
    assert!(persistence::decode(text.as_bytes()).is_err());
    // A scene without them is unchanged and opens without them.
    let mut plain = Scene::default();
    plain.new_mesh().unwrap();
    let bytes = persistence::encode(&plain).unwrap();
    let opened = persistence::decode(&bytes).unwrap();
    assert!(opened.items.iter().all(|i| i.is_mesh()));
    assert_eq!(opened.render_camera(), None);
}

#[test]
fn exports_leave_cameras_and_lights_out() {
    let mut e = new_scene();
    e.scene.items[0].mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    let dir = std::env::temp_dir().join(format!("operandi-cl1-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let obj = dir.join("scene.obj");
    let report = interchange::export(&obj, &e.scene, interchange::ExportFormat::Obj).unwrap();
    assert_eq!(report.items, 1);
    assert!(report.notes.iter().all(|n| !n.contains("hidden")));
    // LXO export still works and an LXO import doesn't add cameras or
    // lights (CL-3 adds them to the format).
    let lxo_path = dir.join("scene.lxo");
    interchange::export(&lxo_path, &e.scene, interchange::ExportFormat::Lxo).unwrap();
    let bytes = std::fs::read(&lxo_path).unwrap();
    let mut progress = |_: &str| true;
    let imported = lxo::parse(&bytes, &mut lxo::Control::new(&mut progress)).unwrap();
    assert_eq!(imported.scene.items.len(), 1);
    assert!(imported.scene.items.iter().all(|i| i.is_mesh()));
    let _ = std::fs::remove_dir_all(dir);
}

// --- CL-2: point, spot and area lights ------------------------------------

#[test]
fn create_light_makes_each_kind_at_the_origin_alone_selected() {
    for (kind, name) in [
        (LightKind::Directional, "Directional Light (2)"),
        (LightKind::Point, "Point Light"),
        (LightKind::Spot, "Spot Light"),
        (LightKind::Area, "Area Light"),
    ] {
        let mut e = new_scene();
        let undo = e.history.undo.len();
        e.dispatch(Command::NewLight(kind)).unwrap();
        assert_eq!(e.history.undo.len(), undo + 1, "{kind:?}: one undo step");
        let item = e.scene.active().unwrap();
        assert_eq!(item.name, name);
        assert_eq!(item.transform, scene::Transform::default());
        assert_eq!(item.light.as_ref().unwrap().kind, kind);
        assert!(e.scene.selection.vertices.is_empty() && e.scene.selection.faces.is_empty());
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene.items.len(), 4);
    }
    let mut e = new_scene();
    e.dispatch(Command::NewCamera).unwrap();
    assert_eq!(
        e.scene.active().unwrap().transform,
        scene::Transform::default()
    );
}

#[test]
fn point_spot_and_area_defaults() {
    let p = LightSettings::new(LightKind::Point);
    assert_eq!(p.falloff, LightFalloff::InverseDistanceSquared);
    assert_eq!(p.radius, defaults::LIGHT_RADIUS);
    assert_eq!(p.intensity, defaults::POINT_LIGHT_INTENSITY);
    assert_eq!(LightKind::Point.intensity_label(), "Radiant Intensity");
    let s = LightSettings::new(LightKind::Spot);
    // Observed (spec v1): 45° cone, 5° soft edge.
    assert!((s.cone_angle.to_degrees() - 45.0).abs() < 1e-9);
    assert!((s.soft_edge.to_degrees() - 5.0).abs() < 1e-9);
    // Full strength within 17.5° of the axis (a 35° inner cone), fading
    // over the outer 5° on each side.
    assert!((s.spot_full_angle().to_degrees() - 17.5).abs() < 1e-9);
    // Every kind defaults to Intensity 3.
    for kind in LightKind::ALL {
        assert_eq!(LightSettings::new(kind).intensity, 3.0, "{kind:?}");
    }
    assert_eq!(s.falloff, LightFalloff::InverseDistanceSquared);
    let a = LightSettings::new(LightKind::Area);
    assert_eq!(
        (a.shape, a.width, a.height),
        (AreaShape::Rectangular, 1.0, 1.0)
    );
    assert_eq!(a.falloff, LightFalloff::InverseDistanceSquared);
    assert_eq!(LightKind::Area.intensity_label(), "Radiance");
    // The shared channels are as for Directional.
    for l in [&p, &s, &a] {
        assert_eq!(l.render, LightRender::Default);
        assert_eq!(l.color, [1.0; 3]);
        assert_eq!((l.affect_diffuse, l.affect_specular), (1.0, 1.0));
        assert_eq!(l.shadow, ShadowType::RayTraced);
    }
}

#[test]
fn light_kind_channels_edit_validate_and_round_trip() {
    let mut e = new_scene();
    e.dispatch(Command::NewLight(LightKind::Spot)).unwrap();
    let spot = e.scene.selection.item.unwrap();
    e.dispatch(Command::NewLight(LightKind::Area)).unwrap();
    let area = e.scene.selection.item.unwrap();
    let edit = |e: &mut Editor, item, edit| e.dispatch(Command::EditLight { item, edit });
    edit(&mut e, spot, LightEdit::ConeAngle(60f64.to_radians())).unwrap();
    edit(&mut e, spot, LightEdit::Radius(0.1)).unwrap();
    edit(&mut e, spot, LightEdit::Falloff(LightFalloff::None)).unwrap();
    assert!(edit(&mut e, spot, LightEdit::ConeAngle(0.0)).is_err());
    assert!(edit(&mut e, spot, LightEdit::Radius(-1.0)).is_err());
    edit(&mut e, area, LightEdit::Shape(AreaShape::Elliptical)).unwrap();
    edit(&mut e, area, LightEdit::Width(2.0)).unwrap();
    assert!(edit(&mut e, area, LightEdit::Height(0.0)).is_err());
    let bytes = persistence::encode(&e.scene).unwrap();
    assert_eq!(persistence::decode(&bytes).unwrap(), e.scene);
    // Each edit is one step.
    e.dispatch(Command::Undo).unwrap();
    let a = e.scene.items.iter().find(|i| i.id == area).unwrap();
    assert_eq!(a.light.as_ref().unwrap().width, 1.0);
}

// --- Spec v2 (observed) ----------------------------------------------------

#[test]
fn spot_soft_edge_follows_the_observed_falloff() {
    // 40° cone, 10° edge: full to 10°, 0.60 at 15°, 0.14 at 18°, 0 at 20°.
    // Cycles' smoothstep-in-cos gives 0.62 and 0.16 there; the closest any
    // blend gets while staying full to 10° and zero at 20°.
    let mut s = LightSettings::new(LightKind::Spot);
    s.cone_angle = 40f64.to_radians();
    s.soft_edge = 10f64.to_radians();
    let at = |deg: f64| s.spot_attenuation(deg.to_radians());
    assert!((at(5.0) - 1.0).abs() < 1e-9);
    assert!((at(10.0) - 1.0).abs() < 1e-9);
    assert!((at(15.0) - 0.60).abs() < 0.025, "{}", at(15.0));
    assert!((at(18.0) - 0.14).abs() < 0.025, "{}", at(18.0));
    assert!(at(20.0).abs() < 1e-9 && at(25.0) == 0.0);
    // Edge 0: a hard cut.
    s.soft_edge = 0.0;
    assert_eq!(s.spot_blend(), 0.0);
    assert_eq!(
        (s.spot_attenuation(0.34), s.spot_attenuation(0.35)),
        (1.0, 0.0)
    );
}

#[test]
fn film_fit_covers_the_observed_film_area() {
    // What a 1920×1080 and a 1080×1920 frame cover of a 36×24 mm film, in
    // film millimetres (spec v2).
    let covered = |fit: FilmFit, w: f64, h: f64| {
        let c = CameraSettings {
            film_fit: fit,
            ..Default::default()
        };
        let (tw, th) = c.half_tangents(w / h);
        (
            2.0 * tw * c.focal_length * 1000.0,
            2.0 * th * c.focal_length * 1000.0,
        )
    };
    for (fit, wide, tall) in [
        (FilmFit::Fill, (36.0, 20.25), (13.5, 24.0)),
        (FilmFit::Horizontal, (36.0, 20.25), (36.0, 64.0)),
        (FilmFit::Vertical, (42.667, 24.0), (13.5, 24.0)),
        (FilmFit::Overscan, (42.667, 24.0), (36.0, 64.0)),
    ] {
        for ((w, h), want) in [((1920.0, 1080.0), wide), ((1080.0, 1920.0), tall)] {
            let got = covered(fit, w, h);
            assert!(
                (got.0 - want.0).abs() < 0.01 && (got.1 - want.1).abs() < 0.01,
                "{fit:?} {w}×{h}: {got:?}, want {want:?}"
            );
        }
    }
}

// --- Spec v3: tool channels (CL-14) ----------------------------------------

/// An editor with a selected area light and Move active in Item mode.
fn area_with_move() -> (Editor, u64) {
    let mut e = new_scene();
    e.dispatch(Command::NewLight(LightKind::Area)).unwrap();
    let id = e.scene.selection.item.unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    assert_eq!(e.scene.selection.mode, Mode::Item);
    (e, id)
}
fn width(e: &Editor, id: u64) -> f64 {
    e.scene
        .items
        .iter()
        .find(|i| i.id == id)
        .unwrap()
        .light
        .as_ref()
        .unwrap()
        .width
}
fn set_width(e: &mut Editor, id: u64, w: f64) -> Result<()> {
    e.dispatch(Command::ToolItemChannel {
        item: id,
        edit: ItemChannelEdit::Light(LightEdit::Width(w)),
    })
}

#[test]
fn tool_channel_edits_survive_moves_and_undo_inside_the_tool() {
    let (mut e, id) = area_with_move();
    set_width(&mut e, id, 2.0).unwrap();
    assert_eq!(width(&e, id), 2.0);
    // A move afterwards keeps the new width.
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = DVec3::new(1.0, 0.0, 0.0);
    e.dispatch(Command::Preview(p)).unwrap();
    assert_eq!(width(&e, id), 2.0);
    // Undo takes back the move, then the width; the tool stays.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(width(&e, id), 2.0);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(width(&e, id), 1.0);
    assert!(e.session.live.is_some());
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(width(&e, id), 2.0);
    // Drop keeps it as one document step.
    let undo = e.history.undo.len();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), undo + 1);
    assert_eq!(width(&e, id), 2.0);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(width(&e, id), 1.0);
}

#[test]
fn tool_channel_edits_cancel_and_need_the_right_tool() {
    let (mut e, id) = area_with_move();
    set_width(&mut e, id, 3.0).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(width(&e, id), 1.0, "Cancel reverts the channel edit");
    // No tool, or a tool that doesn't show channels: refused.
    assert!(set_width(&mut e, id, 2.0).is_err());
    e.dispatch(Command::Activate(Tool::Scale)).unwrap();
    assert!(set_width(&mut e, id, 2.0).is_err());
    e.dispatch(Command::Cancel).unwrap();
    // Invalid values are refused and change nothing.
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    assert!(set_width(&mut e, id, 0.0).is_err());
    assert_eq!(width(&e, id), 1.0);
    // Spot and camera channels go the same way.
    e.dispatch(Command::Cancel).unwrap();
    let cam = camera_id(&e);
    e.dispatch(Command::SelectItem(Some(cam))).unwrap();
    e.dispatch(Command::Activate(Tool::Transform)).unwrap();
    e.dispatch(Command::ToolItemChannel {
        item: cam,
        edit: ItemChannelEdit::Camera(CameraEdit::FocalLength(0.035)),
    })
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    let c = e.scene.items.iter().find(|i| i.id == cam).unwrap();
    assert_eq!(c.camera.as_ref().unwrap().focal_length, 0.035);
}
