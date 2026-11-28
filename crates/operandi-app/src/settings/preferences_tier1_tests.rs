//! Preferences v2 Tier 1 (specifications/preferences-2026-11-06): the
//! Display, Tool Handles, Application, Work Plane and reset rows, through
//! the app's real routes.
use crate::camera::{self, Camera, View};
use crate::defaults_preferences::{Axis, Center};
use crate::handle_preferences::HandleDetail;
use crate::preferences::Page;
use crate::prefs_file::PrefsFile;
use crate::tests::draw;
use crate::{Command, Operandi};
use eframe::egui::{Context, Pos2, Rect, vec2};
use glam::DVec3;
use operandi_core::user_defaults::IndexStyle;
use operandi_core::{CenterMode, Mode, Tool};

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "operandi-pref-t1-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn grid_visibility_sets_new_views_grid() {
    let mut a = Operandi::empty();
    assert!(a.new_view_display().settings.drawing.grid, "measured: on");
    a.viewport_preferences.grid_visibility = false;
    assert!(!a.new_view_display().settings.drawing.grid);
    // Views that already have settings keep theirs.
    a.displays.insert(7, a.display.clone());
    a.activate_view(7);
    assert!(a.display.settings.drawing.grid);
}

#[test]
fn flatness_changes_perspective_but_keeps_the_target_size() {
    let r = Rect::from_min_size(Pos2::ZERO, vec2(800., 600.));
    let mut c = Camera::default();
    let edge = c.target + c.basis().1;
    let size = |c: &Camera| {
        c.project(edge, r)
            .unwrap()
            .0
            .distance(c.project(c.target, r).unwrap().0)
    };
    let before = size(&c);
    let near = c.target + c.basis().3 * -2.;
    let depth_ratio = |c: &Camera| {
        // How much bigger something nearer the eye draws: the strength of
        // the perspective.
        let far =
            c.project(c.target + c.basis().1, r).unwrap().0.x - c.project(c.target, r).unwrap().0.x;
        let close = c.project(near + c.basis().1, r).unwrap().0.x - c.project(near, r).unwrap().0.x;
        close / far
    };
    let strong = depth_ratio(&c);
    c.reflatten(camera::flatness_scale(100) / camera::flatness_scale(60));
    assert!((size(&c) - before).abs() < 0.5, "target keeps its size");
    assert!(depth_ratio(&c) < strong, "flatter");
    // Orthographic views look the same at any flatness.
    let mut o = Camera {
        view: View::Front,
        ..Camera::default()
    };
    let at = o.project(edge, r).unwrap().0;
    o.reflatten(3.);
    assert!(o.project(edge, r).unwrap().0.distance(at) < 0.01);
    // New views start at the preferred flatness; the page applies it live.
    let mut a = Operandi::empty();
    camera::set_flatness(60);
    let lens = a.camera.lens;
    a.viewport_preferences.flatness = 100;
    a.reflatten_views(60);
    assert!((a.camera.lens / lens - 2.).abs() < 1e-9);
    assert!((Camera::default().lens / camera::DEFAULT_LENS - 2.).abs() < 1e-9);
    camera::set_flatness(60);
}

#[test]
fn item_drawing_size_reaches_new_scenes_and_saves_with_the_scene() {
    let mut a = Operandi::empty();
    a.viewport_preferences.item_draw_size = 2.5;
    a.initialize_new_document();
    assert_eq!(a.editor.scene.item_draw_size, 2.5);
    assert!(!a.dirty());
    let path = temp("draw-size.opscene");
    operandi_core::persistence::save(&path, &a.editor.scene).unwrap();
    let back = operandi_core::persistence::load(&path).unwrap();
    assert_eq!(back.item_draw_size, 2.5);
    let mut bad = back;
    bad.item_draw_size = 0.;
    assert!(bad.validate().is_err());
}

#[test]
fn backdrop_preferences_fit_and_size_new_backdrops() {
    let mut a = Operandi::empty();
    a.viewport_preferences.backdrop_uniform_size = true;
    a.viewport_preferences.backdrop_pixel_size = 0.02;
    a.apply_user_defaults();
    a.create_backdrop();
    let id = a.editor.scene.selection.item.unwrap();
    let b = a.editor.scene.item(id).unwrap();
    let b = b.backdrop.as_ref().unwrap();
    assert!(b.uniform_size);
    assert_eq!(b.pixel_size, 0.02);
    // Fitting includes the Backdrop only while the preference is on.
    a.camera.view = View::Front;
    a.camera.distance = 123.;
    a.viewport_preferences.backdrop_fitting = false;
    a.frame(false);
    assert_eq!(a.camera.distance, 123., "nothing else to frame");
    a.viewport_preferences.backdrop_fitting = true;
    a.frame(false);
    assert_ne!(a.camera.distance, 123.);
    operandi_core::user_defaults::set_current(Default::default());
}

fn moving_cube() -> (Operandi, Context) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::cube_geometry(DVec3::ZERO, DVec3::ONE)
            .unwrap()
            .into();
    a.run(Command::Mode(Mode::Polygon));
    let ctx = Context::default();
    for _ in 0..2 {
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    (a, ctx)
}

#[test]
fn handle_point_sizes_and_secondary_handles() {
    let h = crate::handle_preferences::HandlePreferences::default();
    assert_eq!(h.large_point_size(), 9., "today's handle squares");
    assert_eq!(h.small_point_size(), 12., "today's falloff dots");
    let mut h = h;
    h.scale = 2.;
    h.small_scale = 50;
    h.plane_ratio = 1.5;
    assert_eq!(h.large_point_size(), 18.);
    assert_eq!(h.plane_point_size(), 27.);
    assert_eq!(h.small_point_size(), 12.);
    // Falloff handles are the unselected handles: Invisible leaves none to
    // grab.
    let (mut a, _) = moving_cube();
    a.editor.spatial_falloff = Some(operandi_core::falloff::Falloff::Linear(Default::default()));
    let r = a.viewport_rect;
    let p = a.camera.project(a.falloff_points()[0], r).unwrap().0;
    assert_eq!(a.falloff_hit(p, r), Some(0));
    a.handle_preferences.unselected = HandleDetail::Invisible;
    assert_eq!(a.falloff_hit(p, r), None);
    // Small Points Hit Size sets the reach.
    a.handle_preferences.unselected = HandleDetail::Basic;
    let off = p + vec2(7., 0.);
    assert_eq!(a.falloff_hit(off, r), Some(0), "7 px within 16/2");
    a.handle_preferences.small_point_hit = 8.;
    assert_eq!(a.falloff_hit(off, r), None, "7 px outside 8/2");
}

#[test]
fn rotation_snap_angle_steps_a_ctrl_rotate_drag() {
    let (mut a, _) = moving_cube();
    a.run(Command::Activate(Tool::Rotate));
    let r = a.viewport_rect;
    let base = a.editor.session.live.as_ref().unwrap().params.clone();
    let center = a.camera.project(DVec3::ZERO, r).unwrap().0;
    let start = center + vec2(80., 0.);
    let end = center + vec2(80., 27.);
    a.handle_constrain = true;
    a.update_handle(1, base.clone(), start, end, Tool::Rotate, r);
    let snapped = a.editor.session.live.as_ref().unwrap().params.delta[1];
    assert!(
        snapped != 0. && (snapped / 15.).fract().abs() < 1e-9,
        "{snapped}"
    );
    a.handle_preferences.rotation_snap = 5.;
    a.update_handle(1, base.clone(), start, end, Tool::Rotate, r);
    let fine = a.editor.session.live.as_ref().unwrap().params.delta[1];
    assert!(
        (fine / 5.).fract().abs() < 1e-9 && fine != snapped,
        "{fine}"
    );
    a.handle_constrain = false;
    a.update_handle(1, base, start, end, Tool::Rotate, r);
    let free = a.editor.session.live.as_ref().unwrap().params.delta[1];
    assert!((free / 5.).fract().abs() > 1e-6, "{free}");
}

#[test]
fn last_selected_action_center_comes_back() {
    let mut a = Operandi::empty();
    a.editor.scene.selection.mode = Mode::Polygon;
    a.defaults_preferences.component_center = Center::LastSelected;
    a.defaults_preferences.component_axis = Axis::LastSelected;
    // What the user picks is remembered...
    a.editor.action_context.center = CenterMode::Origin;
    a.editor.action_context.axis = operandi_core::AxisOrientation::Workplane(2);
    a.remember_action_context();
    assert_eq!(a.defaults_preferences.last_component_center, Center::Origin);
    // ...and a new session starts with it.
    a.editor.action_context = Default::default();
    a.apply_tool_defaults();
    assert_eq!(a.editor.action_context.center, CenterMode::Origin);
    assert_eq!(
        a.editor.action_context.axis,
        operandi_core::AxisOrientation::Workplane(2)
    );
    // Item mode keeps its own.
    a.editor.scene.selection.mode = Mode::Item;
    a.apply_tool_defaults();
    assert_eq!(a.editor.action_context.center, CenterMode::None);
}

#[test]
fn names_follow_index_style_default_material_and_texture_map() {
    let mut a = Operandi::empty();
    a.defaults_preferences.index_style = IndexStyle::Underscore;
    a.defaults_preferences.default_material = "Metal".into();
    a.defaults_preferences.texture_map = "UVs".into();
    a.apply_user_defaults();
    a.run(Command::NewMesh);
    a.run(Command::NewMesh);
    let names: Vec<_> = a
        .editor
        .scene
        .items
        .iter()
        .map(|i| i.name.clone())
        .collect();
    assert!(names.contains(&"Mesh_2".to_owned()), "{names:?}");
    // A primitive's polygons get the material, its UVs the map name.
    a.run(Command::Mode(Mode::Polygon));
    a.activate(Tool::Box);
    a.run(Command::Preview(operandi_core::Parameters {
        size: DVec3::ONE,
        make_uvs: true,
        ..Default::default()
    }));
    a.run(Command::Apply);
    a.run(Command::Drop);
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert!(
        !mesh.faces.is_empty(),
        "{} {:?}",
        a.status,
        a.editor
            .scene
            .items
            .iter()
            .map(|i| (i.name.clone(), i.mesh.faces.len()))
            .collect::<Vec<_>>()
    );
    assert!(mesh.faces.iter().all(|f| f.material == "Metal"));
    assert!(mesh.uv_maps.contains_key("UVs") && !mesh.uv_maps.contains_key("Texture"));
    // Set Material starts with the Default Material on untagged polygons.
    let mut b = Operandi::empty();
    b.run(Command::NewMesh);
    b.editor.scene.active_mut().unwrap().mesh =
        operandi_core::cube_geometry(DVec3::ZERO, DVec3::ONE)
            .unwrap()
            .into();
    b.run(Command::Mode(Mode::Polygon));
    b.defaults_preferences.default_material = "Metal".into();
    b.open_set_material();
    assert_eq!(b.set_material.as_ref().unwrap().name(), "Metal");
    operandi_core::user_defaults::set_current(Default::default());
}

#[test]
fn default_scene_starts_new_scenes_untitled() {
    let path = temp("template.opscene");
    let mut t = Operandi::empty();
    t.initialize_new_document();
    t.run(Command::NewMesh);
    t.editor.scene.active_mut().unwrap().name = "Template Mesh".into();
    operandi_core::persistence::save(&path, &t.editor.scene).unwrap();
    let template = operandi_core::persistence::load(&path).unwrap();

    let mut a = Operandi::empty();
    a.defaults_preferences.default_scene = path.display().to_string();
    a.initialize_new_document();
    assert_eq!(a.editor.scene, template);
    assert!(a.path.is_none(), "untitled");
    assert!(!a.dirty());
    // File > New does the same.
    a.new_scene_tab();
    assert_eq!(a.editor.scene.items.len(), template.items.len());
    // A file that won't load is skipped with a note.
    a.defaults_preferences.default_scene = temp("missing.opscene").display().to_string();
    a.initialize_new_document();
    assert!(a.status.starts_with("Default Scene"), "{}", a.status);
    assert!(a.editor.scene.active().is_some(), "the ordinary new scene");
}

#[test]
fn preferred_plane_bias_lock_offset_and_increments() {
    let mut u = crate::units_preferences::UnitsPreferences::default();
    // Looking mostly along Z: the XY plane, until the ground is preferred
    // strongly enough.
    let forward = DVec3::new(0., -0.45, -0.89).normalize();
    assert_eq!(u.view_plane(forward), 2);
    u.preferred_bias = 50;
    assert_eq!(u.view_plane(forward), 0);
    u.preferred_bias = 0;
    u.preferred_plane = 1;
    u.lock_preferred = true;
    assert_eq!(u.view_plane(forward), 1);
    // The locked plane's offset moves the work plane along its normal.
    let mut a = Operandi::empty();
    a.units_preferences = u;
    a.units_preferences.locked_offset = 0.5;
    assert_eq!(a.workplane_center(), DVec3::new(0.5, 0., 0.));
    assert_eq!(a.actual_plane(), 1);
    // A fixed work plane ignores the preference.
    a.editor.scene.workplane.fixed = true;
    assert_eq!(a.workplane_center(), DVec3::ZERO);
    use crate::grid::Increments;
    assert_eq!(Increments::OneTwoFive.step(0.3), 0.5);
    assert_eq!(Increments::OneFive.step(0.3), 0.5);
    assert_eq!(Increments::OneFive.step(0.6), 1.);
    assert_eq!(Increments::Tens.step(0.3), 1.);
    assert_eq!(Increments::Twos.step(0.3), 0.5);
    assert_eq!(Increments::Twos.step(3.), 4.);
}

#[test]
fn rollover_colours_by_polygon_kind() {
    use crate::theme::{self, ColorId, viewport};
    use operandi_core::PolygonRepresentation as Kind;
    theme::set_overrides(&Default::default());
    let mut cube = operandi_core::cube_geometry(DVec3::ZERO, DVec3::ONE).unwrap();
    cube.attributes.representations = vec![Kind::Face; 6];
    cube.attributes.representations[1] = Kind::Subdivs;
    cube.attributes.representations[2] = Kind::CatmullClark;
    let red = eframe::egui::Color32::from_rgb(200, 0, 0);
    theme::set_override(ColorId::RolloverCatmullClark, Some(red));
    assert_eq!(viewport::polygon_rollover(&cube, 2), red);
    assert_eq!(viewport::polygon_rollover(&cube, 0), viewport::ROLLOVER);
    assert_eq!(viewport::polygon_rollover(&cube, 1), viewport::ROLLOVER);
    // Falloff weights run from Cool to Warm.
    assert_eq!(viewport::falloff_weight(0.), viewport::FALLOFF_COOL);
    assert_eq!(viewport::falloff_weight(1.), viewport::FALLOFF_WARM);
    theme::set_overrides(&Default::default());
}

#[test]
fn reset_panel_and_reset_preferences_restore_factory_values() {
    let mut a = Operandi::empty();
    a.selection_preferences.point_size = 9;
    a.handle_preferences.large_points = 20.;
    a.snapping.inner_range = 5.;
    a.input_preferences.options.trackball = true;
    a.defaults_preferences.default_material = "Metal".into();
    a.defaults_preferences.author = "Jason".into();
    // One page resets alone.
    a.reset_page(Page::Selection);
    assert_eq!(a.selection_preferences.point_size, 3);
    assert_eq!(a.handle_preferences.large_points, 20.);
    a.reset_page(Page::ToolHandles);
    assert_eq!(a.handle_preferences.large_points, 9.);
    assert_eq!(a.snapping.inner_range, 24.);
    // Application keeps Scene Save/Export's author.
    a.reset_page(Page::Application);
    assert_eq!(a.defaults_preferences.default_material, "Default");
    assert_eq!(a.defaults_preferences.author, "Jason");
    // Reset Preferences… asks first, then resets every page.
    a.preferences_ui.confirm_reset = true;
    assert!(a.has_modal());
    a.reset_all_preferences();
    assert!(!a.input_preferences.options.trackball);
    assert!(a.defaults_preferences.author.is_empty());
    let saved =
        crate::preferences::InputPreferences::load(&a.environment.paths().input_preferences());
    assert!(!saved.options.trackball, "saved too");
}
