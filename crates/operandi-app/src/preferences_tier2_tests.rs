//! Preferences v2 Tier 2 (specifications/preferences-2026-11-06): Mesh
//! Items, polygon cap and boxes, rounding, units display, Up Axis, Input
//! Device, File I/O, Rendering, overlays and UI, through the app's routes.
use crate::camera::{self, Camera};
use crate::prefs_file::PrefsFile;
use crate::tests::draw;
use crate::{Command, Operandi};
use eframe::egui::{Context, Pos2, Rect, vec2};
use glam::DVec3;
use operandi_core::{Mode, PolygonRepresentation, Tool};

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "operandi-pref-t2-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn cube_app() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::cube_geometry(DVec3::ZERO, DVec3::ONE)
            .unwrap()
            .into();
    a
}

#[test]
fn new_meshes_take_the_mesh_items_defaults() {
    let mut a = Operandi::empty();
    let m = &mut a.mesh_items_preferences;
    m.subdivision_level = 3;
    m.catmull_clark_level = 1;
    m.auto_uv_map = true;
    m.auto_item_masks = true;
    a.defaults_preferences.texture_map = "UVs".into();
    a.apply_user_defaults();
    a.run(Command::NewMesh);
    let item = a.editor.scene.active().unwrap();
    let s = item.subdivision.clone().unwrap();
    assert_eq!((s.subdivision_level, s.catmull_clark_level), (3, 1));
    assert!(item.mesh.uv_maps.contains_key("UVs"));
    let id = item.id;
    let masked = a.editor.scene.shader_tree.nodes.iter().any(|n| {
        matches!(&n.kind, operandi_core::shader_tree::NodeKind::Group(g)
            if g.item == operandi_core::shader_tree::ItemMask::Item(id))
    });
    assert!(masked, "an item mask group");
    // Cone and Ellipsoid start with the Default Primitive Polygon.
    a.mesh_items_preferences.primitive_polygon = PolygonRepresentation::CatmullClark;
    a.apply_user_defaults();
    assert_eq!(
        operandi_core::ConeParameters::default().polygon,
        PolygonRepresentation::CatmullClark
    );
    operandi_core::user_defaults::set_current(Default::default());
}

#[test]
fn symmetric_primitive_creation_can_be_turned_off() {
    let make = |symmetric: bool| {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.mesh_items_preferences.symmetric_primitives = symmetric;
        a.apply_user_defaults();
        a.editor.symmetry.axis = Some(0);
        a.activate(Tool::Box);
        a.run(Command::Preview(operandi_core::Parameters {
            position: DVec3::new(2., 0., 0.),
            size: DVec3::ONE,
            ..Default::default()
        }));
        a.run(Command::Apply);
        a.run(Command::Drop);
        let n = a.editor.scene.active().unwrap().mesh.faces.len();
        operandi_core::user_defaults::set_current(Default::default());
        n
    };
    assert_eq!(make(true), 12, "mirrored box too");
    assert_eq!(make(false), 6);
}

#[test]
fn paste_follows_the_copy_and_paste_rules() {
    let paste = |select: bool, deselect_before: bool, deselect_after_copy: bool| {
        let mut a = cube_app();
        let m = &mut a.mesh_items_preferences;
        m.select_pasted = select;
        m.deselect_before_paste = deselect_before;
        m.deselect_after_copy = deselect_after_copy;
        a.apply_user_defaults();
        a.run(Command::Mode(Mode::Polygon));
        a.editor.scene.selection.faces = vec![0];
        a.copy_selection();
        let after_copy = a.editor.scene.selection.faces.clone();
        a.editor.scene.selection.faces = vec![1];
        a.run(Command::PasteSelection);
        let faces = a.editor.scene.selection.faces.clone();
        operandi_core::user_defaults::set_current(Default::default());
        (after_copy, faces)
    };
    // As before: old selection cleared, the pasted polygon selected.
    assert_eq!(paste(true, true, false), (vec![0], vec![6]));
    // The reference's reading: both kept as they were.
    assert_eq!(paste(false, false, false).1, vec![1]);
    assert_eq!(paste(true, false, false).1, vec![1, 6]);
    assert!(
        paste(true, true, true).0.is_empty(),
        "deselected after copy"
    );
}

#[test]
fn auto_hide_children_and_defer_smoothing_flags() {
    let mut a = Operandi::empty();
    // GV-01: Auto Hide Child Items is on by default (2026-11-10).
    assert!(a.mesh_items_preferences.auto_hide_children);
    assert!(!a.mesh_items_preferences.defer_smoothing);
    let path = temp("mesh-items.json");
    let mut p = a.mesh_items_preferences.clone();
    p.auto_hide_children = false;
    p.defer_smoothing = true;
    p.save(&path).unwrap();
    assert_eq!(
        crate::mesh_items_preferences::MeshItemsPreferences::load(&path),
        p
    );
    a.mesh_items_preferences = p;
    a.reset_page(crate::preferences::Page::MeshItems);
    assert!(a.mesh_items_preferences.auto_hide_children);
}

/// Max GL Polygon Count is retired (Jason, 2026-11-20): a step that makes
/// the scene very heavy asks nothing; the graphics memory watch
/// (gpu_memory.rs) looks after the card instead. Old saved values still
/// load.
#[test]
fn a_heavy_step_asks_nothing() {
    let mut a = cube_app();
    let ctx = Context::default();
    draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    a.viewport_preferences.max_gl_polygons = 10;
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::cube_geometry(DVec3::splat(3.), DVec3::ONE)
            .unwrap()
            .into();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = (0..6).collect();
    a.execute("Polygon Subdivide");
    draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    assert!(a.drawn_triangles() > 10);
    assert!(!a.has_modal());
    let old: crate::viewport_preferences::ViewportPreferences =
        serde_json::from_str(r#"{"version": 1, "max_gl_polygons": 5000000}"#).unwrap();
    let saved = serde_json::to_string(&old).unwrap();
    assert!(!saved.contains("max_gl_polygons"), "{saved}");
}

#[test]
fn bounding_box_threshold_needs_navigation_and_the_cap() {
    use crate::viewport_preferences::BoxThreshold;
    let mut a = cube_app();
    a.viewport_preferences.box_threshold = BoxThreshold::OnlyInactiveSurfaceItems;
    let active = a.editor.scene.selection.item.unwrap();
    assert!(!a.draws_as_box(active + 1, true), "not navigating");
    a.navigation = Some(crate::input::Navigation {
        kind: crate::input::GestureKind::Navigate(crate::input::NavigationKind::Orbit),
        start: Pos2::ZERO,
        last: Pos2::ZERO,
        modifiers: Default::default(),
        button: eframe::egui::PointerButton::Primary,
    });
    assert!(a.draws_as_box(active + 1, true));
    assert!(
        !a.draws_as_box(active, true),
        "the active item keeps drawing"
    );
    assert!(!a.draws_as_box(active + 1, false), "under the cap");
    a.viewport_preferences.box_threshold = BoxThreshold::AllSurfaceItems;
    assert!(a.draws_as_box(active, true));
}

#[test]
fn coordinate_rounding_steps_work_plane_points() {
    use crate::units_preferences::Rounding;
    let mut a = Operandi::empty();
    let r = Rect::from_min_size(Pos2::ZERO, vec2(800., 600.));
    a.camera.view = crate::camera::View::Top;
    let p = r.center() + vec2(37.3, 11.7);
    let raw = a.work_point(p, r, 0, DVec3::ZERO).unwrap();
    assert_eq!(
        raw,
        a.camera.plane_point(p, r, 0, DVec3::ZERO).unwrap(),
        "None: raw"
    );
    a.units_preferences.rounding = Rounding::ForcedFixed;
    a.units_preferences.fixed_increment = 0.25;
    let q = a.work_point(p, r, 0, DVec3::ZERO).unwrap();
    for c in [q.x, q.z] {
        assert!(((c / 0.25).round() - c / 0.25).abs() < 1e-9, "{q}");
    }
    assert_eq!(q.y, 0.);
    a.units_preferences.rounding = Rounding::Normal;
    let mpp = a.camera.metres_per_pixel(raw, r).unwrap();
    let step = a.units_preferences.rounding_step(mpp).unwrap();
    assert!(step >= mpp * 3. && step <= mpp * 3. * 2.5 + 1e-12);
    a.units_preferences.rounding = Rounding::Fixed;
    a.units_preferences.fixed_increment = 1000.;
    assert_eq!(a.units_preferences.rounding_step(mpp), Some(1000.));
    // Fixed floors the grid, Forced Fixed holds it.
    crate::grid::set_floor(Some((5., true)));
    assert_eq!(crate::grid::spacing(&a.camera, r, 0, DVec3::ZERO), 5.);
    crate::grid::set_floor(None);
}

#[test]
fn up_axis_turns_the_views_and_files() {
    let r = Rect::from_min_size(Pos2::ZERO, vec2(800., 600.));
    camera::set_up_axis(2);
    let c = Camera::default();
    let (_, _, up, forward) = c.basis();
    assert!(up.z > 0.5, "Z is up on screen: {up}");
    assert!(forward.is_normalized());
    let top = Camera {
        view: crate::camera::View::Top,
        ..Camera::default()
    };
    assert!(
        (top.basis().3 - DVec3::NEG_Z).length() < 1e-9,
        "Top looks down Z"
    );
    assert_eq!(camera::ground_plane(), 2);
    // Camera view's yaw/pitch round-trip in the turned frame.
    let toward = -c.basis().3;
    let (yaw, pitch) = Camera::yaw_pitch_of(toward);
    assert!((yaw - c.yaw).abs() < 1e-9 && (pitch - c.pitch).abs() < 1e-9);
    let _ = r;
    // A Y-up OBJ comes in standing on Z, and goes back out Y-up.
    let path = temp("up.obj");
    std::fs::write(&path, "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
    let mut a = Operandi::empty();
    a.units_preferences.up_axis = 2;
    a.units_preferences.apply();
    a.file_io_preferences.apply();
    let imported = operandi_core::interchange::import(&path).unwrap();
    let v = &imported.scene.items[0].mesh.vertices;
    assert!(v.iter().any(|p| (*p - DVec3::Z).length() < 1e-9), "{v:?}");
    let out = temp("up-out.obj");
    operandi_core::interchange::export(
        &out,
        &imported.scene,
        operandi_core::interchange::ExportFormat::Obj,
    )
    .unwrap();
    assert!(std::fs::read_to_string(&out).unwrap().contains("v 0 1 0"));
    camera::set_up_axis(1);
    a.units_preferences.up_axis = 1;
    a.file_io_preferences.apply();
}

#[test]
fn input_device_picks_trackball_speed() {
    use crate::navigation_preferences::{InputDevice, NavigationOptions};
    let mut o = NavigationOptions {
        trackball_speed_mouse: 1.,
        trackball_speed_tablet: 3.,
        ..Default::default()
    };
    assert_eq!(o.trackball_speed(), 1.);
    assert!(!o.tablet_navigation());
    for d in [InputDevice::Tablet, InputDevice::Hybrid] {
        o.input_device = d;
        assert!(o.tablet_navigation());
        assert_eq!(o.trackball_speed(), 3.);
    }
}

#[test]
fn file_io_options_reach_obj_stl_and_gltf() {
    use operandi_core::interchange::{ExportFormat, FileUnit, GltfExportType, export, import};
    let mut a = cube_app();
    a.editor.scene.active_mut().unwrap().mesh = {
        let mut m = operandi_core::cube_geometry(DVec3::ZERO, DVec3::ONE).unwrap();
        operandi_core::add_cube_uvs(&mut m, DVec3::ZERO, DVec3::ONE).unwrap();
        for f in &mut m.faces {
            f.material = "Red".into();
        }
        m.into()
    };
    let f = &mut a.file_io_preferences;
    f.obj_export_units = FileUnit::Centimeters;
    f.obj_save_groups = true;
    a.file_io_preferences.apply();
    let obj = temp("cube.obj");
    export(&obj, &a.editor.scene, ExportFormat::Obj).unwrap();
    let text = std::fs::read_to_string(&obj).unwrap();
    assert!(text.contains("\ng ") && !text.contains("\no "), "groups");
    assert!(
        text.contains("v 50 50 50") || text.contains("v -50 -50 -50"),
        "centimetres"
    );
    assert!(text.contains("mtllib cube.mtl"));
    assert!(
        std::fs::read_to_string(obj.with_extension("mtl"))
            .unwrap()
            .contains("newmtl Red")
    );
    // Importing in centimetres brings it back to metres.
    a.file_io_preferences.obj_import_units = FileUnit::Centimeters;
    a.file_io_preferences.apply();
    let back = import(&obj).unwrap();
    let max = back.scene.items[0]
        .mesh
        .vertices
        .iter()
        .fold(f64::MIN, |m, p| m.max(p.max_element()));
    assert!((max - 0.5).abs() < 1e-9, "{max}");
    // STL units.
    a.file_io_preferences.stl_export_units = FileUnit::Millimeters;
    a.file_io_preferences.apply();
    let stl = temp("cube.stl");
    export(&stl, &a.editor.scene, ExportFormat::StlAscii).unwrap();
    assert!(std::fs::read_to_string(&stl).unwrap().contains("500"));
    // glTF: short indices, no materials, selected only.
    let f = &mut a.file_io_preferences;
    f.gltf_short_indices = true;
    f.gltf_materials = false;
    f.gltf_tangents = true;
    f.gltf_export = GltfExportType::Selected;
    a.file_io_preferences.apply();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = {
        let mut m = operandi_core::cube_geometry(DVec3::splat(5.), DVec3::ONE).unwrap();
        operandi_core::add_cube_uvs(&mut m, DVec3::splat(5.), DVec3::ONE).unwrap();
        m.into()
    };
    let gltf = temp("cube.gltf");
    export(&gltf, &a.editor.scene, ExportFormat::Gltf).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(&gltf).unwrap()).unwrap();
    assert_eq!(
        json["meshes"].as_array().unwrap().len(),
        1,
        "only the selected item"
    );
    assert!(json.get("materials").is_none());
    let prim = &json["meshes"][0]["primitives"][0];
    assert!(prim.get("material").is_none());
    let indices = prim["indices"].as_u64().unwrap() as usize;
    assert_eq!(json["accessors"][indices]["componentType"], 5123);
    assert!(prim["attributes"].get("TANGENT").is_some());
    // Reopens.
    assert!(import(&gltf).is_ok());
    operandi_core::interchange::set_io_options(Default::default());
}

#[test]
fn snapping_state_overlay_line() {
    let mut a = cube_app();
    a.defaults_preferences.overlays = crate::defaults_preferences::Overlays {
        current_selection: false,
        polygon_count: false,
        grid_size: false,
        snapping_state: true,
        ..Default::default()
    };
    let r = Rect::from_min_size(Pos2::ZERO, vec2(800., 600.));
    assert_eq!(a.overlay_lines(r), ["Snapping: Off"]);
    a.snapping.enabled = true;
    assert!(a.overlay_lines(r)[0].starts_with("Snapping: "));
    assert_ne!(a.overlay_lines(r)[0], "Snapping: Off");
}

#[test]
fn render_threads_reach_the_final_render_settings() {
    let mut a = Operandi::empty();
    a.render_preferences.automatic_threads = false;
    a.render_preferences.threads = 3;
    assert_eq!(a.render_preferences.render_threads(), 3);
    a.file_io_preferences.image_format = crate::file_io_preferences::ImageFormat::Jpeg;
    let p = crate::render_ui::numbered_path(Some(&temp("renders")), "Render", "jpg").unwrap();
    assert_eq!(p.extension().unwrap(), "jpg");
}
