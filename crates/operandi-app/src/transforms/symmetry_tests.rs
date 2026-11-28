//! Symmetry from the command line and the input map (SYM-ST-01, SYM-SEL-01,
//! SYM-SZ-01). Geometry rules are tested in operandi-core.
use super::tests::box_app;
use super::*;

#[test]
fn symmetry_words_select_symmetrical_and_symmetrize() {
    let mut a = box_app();
    a.execute("symmetry x");
    assert_eq!(a.editor.symmetry, Symmetry::x());
    a.run(Command::Mode(Mode::Vertex));
    // Box points 0 (-1,-1,-1) and 1 (1,-1,-1) mirror across x = 0.
    a.editor.scene.selection.vertices = vec![1];
    a.execute("select symmetrical");
    assert_eq!(
        a.editor.scene.selection.vertices,
        vec![1, 0],
        "{}",
        a.status
    );
    a.execute("undo");
    assert_eq!(a.editor.scene.selection.vertices, vec![1]);

    a.editor.scene.active_mut().unwrap().mesh.vertices[1].y = -1.001;
    a.execute("symmetrize positive");
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices[0].y, -1.001);
    a.execute("symmetry off");
    assert_eq!(a.editor.symmetry, Symmetry::OFF);
}

#[test]
fn symmetry_actions_are_remappable_and_start_unbound() {
    let defaults = input::default_bindings();
    for name in [
        "Select Symmetrical",
        "Symmetrize Positive",
        "Symmetrize Negative",
    ] {
        assert_eq!(defaults.get(name).map(String::as_str), Some(""), "{name}");
        assert!(input::available(name, Mode::Vertex, false));
        assert!(!input::available(name, Mode::Vertex, true));
    }
}

#[test]
fn falloff_words_switch_types_and_keep_settings() {
    let mut a = box_app();
    a.execute("falloff radial");
    assert!(matches!(a.editor.spatial_falloff, Some(Falloff::Radial(_))));
    let wide = LinearFalloff {
        end: DVec3::X * 4.,
        ..Default::default()
    };
    a.run(Command::SpatialFalloff(Some(wide.into())));
    a.execute("falloff none");
    assert_eq!(a.editor.spatial_falloff, None);
    a.execute("falloff linear");
    assert_eq!(a.editor.spatial_falloff, Some(Falloff::Linear(wide)));
}

#[test]
fn falloff_handles_hit_and_drag() {
    let mut a = box_app();
    let r =
        eframe::egui::Rect::from_min_size(eframe::egui::Pos2::ZERO, eframe::egui::vec2(800., 600.));
    let field = LinearFalloff {
        start: DVec3::ZERO,
        end: DVec3::X,
        ..Default::default()
    };
    a.run(Command::SpatialFalloff(Some(field.into())));
    let end = a.camera.project(DVec3::X, r).unwrap().0;
    assert_eq!(a.falloff_hit(end, r), Some(1));
    // Dragging the End handle to the right moves End along +X (top view).
    a.drag_falloff(1, field.into(), end, end + eframe::egui::vec2(50., 0.), r);
    let Some(Falloff::Linear(moved)) = a.editor.spatial_falloff else {
        panic!("still linear");
    };
    assert!(moved.end.x > 1.);
    assert_eq!(moved.start, field.start);
}

/// Add Loop: click an edge, keep the button held and slide; the loop moves
/// along the edge with a live percent, and the whole gesture is one undo
/// step (Jason-specified).
#[test]
fn add_loop_click_then_slide_is_one_step() {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Edge));
    let r =
        eframe::egui::Rect::from_min_size(eframe::egui::Pos2::ZERO, eframe::egui::vec2(800., 600.));
    let mesh = a.editor.scene.active().unwrap().mesh.to_owned_value();
    let (ea, eb) = mesh.edges()[0];
    let pa = a.camera.project(mesh.vertices[ea as usize], r).unwrap().0;
    let pb = a.camera.project(mesh.vertices[eb as usize], r).unwrap().0;
    a.edge_click = Some(edge_panel::EdgeClick::AddLoop);
    let undo = a.editor.history.undo.len();
    let (taken, slide) = a.edge_click_at(pa.lerp(pb, 0.5), r, false);
    assert!(taken);
    let Some(input::GestureKind::EdgeInsert {
        tool,
        a: x,
        b: y,
        ends,
    }) = slide
    else {
        panic!("no slide gesture: {}", a.status);
    };
    assert_eq!(a.editor.history.undo.len(), undo + 1);
    let faces = a.editor.scene.active().unwrap().mesh.faces.len();
    a.edge_insert_slide(tool, x, y, ends, ends[0].0.lerp(ends[1].0, 0.25));
    assert_eq!(a.editor.history.undo.len(), undo + 1, "still one step");
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), faces);
    assert!(a.status.contains("25%"), "{}", a.status);
}

/// Cone and Ellipsoid can be drawn in the viewport: press sets the centre on
/// the construction plane, dragging sets the radius (Jason-specified).
#[test]
fn cone_and_ellipsoid_draw_by_dragging() {
    for cone in [false, true] {
        let mut a = box_app();
        let r = eframe::egui::Rect::from_min_size(
            eframe::egui::Pos2::ZERO,
            eframe::egui::vec2(800., 600.),
        );
        a.run(if cone {
            Command::ArmCone
        } else {
            Command::ArmEllipsoid
        });
        let plane = a.actual_plane();
        let center = a.camera.project(DVec3::new(2., 0., 0.), r).unwrap().0;
        let edge = a.camera.project(DVec3::new(3., 0., 0.), r).unwrap().0;
        let input::GestureKind::SolidDraw { anchor, .. } = a.begin_solid_draw(cone, center, r)
        else {
            panic!("no draw gesture: {}", a.status);
        };
        a.solid_draw(cone, plane, anchor, edge, r, true);
        if cone {
            let p = &a.editor.session.cone.as_ref().unwrap().requested;
            assert!((p.size.x - 1.).abs() < 1e-6, "{:?}", p.size);
            assert!(
                a.editor.session.cone.as_ref().unwrap().preview.is_some(),
                "{}",
                a.status
            );
        } else {
            let p = &a.editor.session.ellipsoid.as_ref().unwrap().requested;
            assert!((p.radius.x - 1.).abs() < 1e-6, "{:?}", p.radius);
            assert!((p.position.x - 2.).abs() < 1e-6);
            assert!(
                a.editor
                    .session
                    .ellipsoid
                    .as_ref()
                    .unwrap()
                    .preview
                    .is_some(),
                "{}",
                a.status
            );
        }
    }
}

/// KEY-01/02: maps saved with the earlier factory chords move to the measured
/// ones (B for Bevel and Edge Bevel, Y for Transform); a user's own choice
/// stays.
#[test]
fn saved_maps_take_the_measured_bevel_and_transform_keys() {
    let dir = std::env::temp_dir().join(format!("operandi-keys-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("input.json");
    let old = std::collections::BTreeMap::from([
        ("Bevel", "Shift+B"),
        ("Edge Chamfer", "B"),
        ("Edge Bevel", ""),
        ("Transform", ""),
    ]);
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    let b = input::load_bindings(&path);
    assert_eq!(b["Bevel"], "B");
    assert_eq!(b["Edge Bevel"], "B");
    assert_eq!(b["Edge Chamfer"], "");
    assert_eq!(b["Transform"], "Y");
    let mine = std::collections::BTreeMap::from([("Bevel", "Ctrl+B")]);
    std::fs::write(&path, serde_json::to_vec(&mine).unwrap()).unwrap();
    assert_eq!(input::load_bindings(&path)["Bevel"], "Ctrl+B");
    assert!(input::bindings_valid(&input::default_bindings()));
    std::fs::remove_dir_all(dir).unwrap();
}

/// Symmetry phase 3 (section 8): Toggle Symmetry from the command line,
/// unbound by default; the seam guide lies on the plane; the seam snap
/// distance is a snapping preference (8 px).
#[test]
fn toggle_seam_guide_and_snap_distance() {
    let mut a = box_app();
    assert_eq!(
        input::default_bindings()
            .get("Toggle Symmetry")
            .map(String::as_str),
        Some("")
    );
    assert!(a.symmetry_seam_guide().is_empty(), "off: no guide");
    a.execute("toggle symmetry");
    assert_eq!(a.editor.symmetry, Symmetry::x(), "{}", a.status);
    let guide = a.symmetry_seam_guide();
    assert_eq!(guide.len(), 1);
    for p in guide[0] {
        assert!(p.x.abs() < 1e-9, "on the plane x = 0: {p}");
    }
    let size = guide[0][0].distance(guide[0][2]);
    assert!(size > 2. * 3f64.sqrt(), "bigger than the box: {size}");
    a.execute("toggle symmetry");
    assert_eq!(a.editor.symmetry.axis, None);

    assert_eq!(snapping::SnappingPreferences::default().symmetry_seam, 8.);
}

/// Jason 2026-11-14: each view can hide the symmetry plane outline
/// (Viewport Properties > Visibility > Show Symmetry Plane, and the view's
/// Display menu) without turning Symmetry off. On by default; old saved
/// view settings load with it on.
#[test]
fn show_symmetry_plane_hides_only_the_guide() {
    let mut a = box_app();
    a.execute("symmetry x");
    assert!(a.display.settings.drawing.symmetry_plane);
    assert_eq!(a.symmetry_seam_guide().len(), 1);
    a.display.settings.drawing.symmetry_plane = false;
    assert!(a.symmetry_seam_guide().is_empty(), "hidden in this view");
    assert_eq!(a.editor.symmetry, Symmetry::x(), "Symmetry stays on");
    a.display.settings.drawing.symmetry_plane = true;
    assert_eq!(a.symmetry_seam_guide().len(), 1);

    let mut old = serde_json::to_value(crate::view_display::Drawing::default()).unwrap();
    old.as_object_mut().unwrap().remove("symmetry_plane");
    let loaded: crate::view_display::Drawing = serde_json::from_value(old).unwrap();
    assert!(loaded.symmetry_plane, "older settings: shown");
}

/// The Show Symmetry Plane row runs a registered command (UIC-164).
#[test]
fn show_symmetry_plane_command() {
    let mut a = box_app();
    a.execute("operandi.view3d.showSymmetryPlane false");
    assert!(!a.display.settings.drawing.symmetry_plane, "{}", a.status);
    a.execute("operandi.view3d.showSymmetryPlane true");
    assert!(a.display.settings.drawing.symmetry_plane, "{}", a.status);
}
