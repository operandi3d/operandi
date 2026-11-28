//! Edit tab wave 1 (docs/specs/edit-wave1/SUBDIVIDE.md): the Slide button,
//! Center row, Subdivide group and Freeze..., driven through real frames.
use super::*;
use operandi_core::{SubdivideMethod, SubdivideOptions};

fn frame(a: &mut Operandi, ctx: &Context, mods: Modifiers, events: Vec<Event>) {
    ctx.options_mut(|o| o.max_passes = std::num::NonZeroUsize::MIN);
    let _ = ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1440., 2000.))),
            modifiers: mods,
            events,
            ..Default::default()
        },
        |ctx| {
            a.begin_input(ctx);
            a.compose(ctx);
            a.keyboard(ctx);
        },
    );
}
fn pointer(p: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos: p,
        pressed,
        modifiers,
        button: PointerButton::Primary,
    }
}
fn click(a: &mut Operandi, ctx: &Context, at: Pos2, mods: Modifiers) {
    frame(a, ctx, mods, vec![Event::PointerMoved(at)]);
    frame(a, ctx, mods, vec![pointer(at, true, mods)]);
    frame(a, ctx, mods, vec![pointer(at, false, mods)]);
    frame(a, ctx, Modifiers::NONE, vec![]);
}
fn press(a: &mut Operandi, ctx: &Context, key: Key, modifiers: Modifiers) {
    frame(
        a,
        ctx,
        Modifiers::NONE,
        vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }],
    );
    frame(
        a,
        ctx,
        Modifiers::NONE,
        vec![Event::Key {
            key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers,
        }],
    );
}
/// A unit cube in Polygon mode, nothing selected, Edit tab showing.
fn cube_app() -> (Operandi, Context) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0)
            .unwrap()
            .into();
    a.editor.synchronize_identities().unwrap();
    a.run(Command::Mode(Mode::Polygon));
    a.category = toolbox::Category::Edit;
    let ctx = Context::default();
    theme::install(&ctx);
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    (a, ctx)
}
fn faces(a: &Operandi) -> usize {
    a.editor.scene.active().unwrap().mesh.faces.len()
}
/// A form button by the line it runs, else a heading by its key.
fn rect(a: &Operandi, key: &str) -> Rect {
    *a.form_rects
        .get(key)
        .or_else(|| a.toolbox_rects.get(key))
        .unwrap_or_else(|| panic!("no toolbox rect {key}"))
}

/// What the Edit tab's button running `line` shows now: label, tooltip.
fn shown(a: &Operandi, line: &str) -> (String, String) {
    a.form_button_for_test(toolbox_form::EDIT_FORM, line)
}

#[test]
fn sub_01_sub_51_tab_order() {
    let (a, _) = cube_app();
    let order = [
        "Edit Slide",
        "Polygon Slice",
        "Loop Slice",
        "Edit Center",
        "Center X",
        "Edit Subdivide",
        "SDS Subdivide",
        "Faceted Subdivide",
        "Smooth Subdivide",
        "Subdivide Options",
        "UnSubdivide",
        "poly.freeze",
    ];
    for pair in order.windows(2) {
        assert!(
            rect(&a, pair[0]).top() <= rect(&a, pair[1]).top(),
            "{} above {}",
            pair[0],
            pair[1]
        );
    }
    // X, Y, Z, All in one row.
    let row: Vec<Rect> = ["Center X", "Center Y", "Center Z", "Center All"]
        .iter()
        .map(|k| rect(&a, k))
        .collect();
    assert!(
        row.windows(2)
            .all(|w| w[0].top() == w[1].top() && w[0].left() < w[1].left())
    );
}

#[test]
fn sub_05_buttons_set_only_the_method() {
    let (mut a, ctx) = cube_app();
    *a.editor.presets.get_mut::<operandi_core::SubdividePreset>() = SubdivideOptions {
        max_angle_degrees: 30.,
        linear_uvs: true,
        ..Default::default()
    };
    let r = rect(&a, "Faceted Subdivide").center();
    click(&mut a, &ctx, r, Modifiers::NONE);
    assert_eq!(faces(&a), 24, "{}", a.status);
    assert_eq!(
        a.editor
            .presets
            .get::<operandi_core::SubdividePreset>()
            .method,
        SubdivideMethod::Faceted
    );
    assert_eq!(
        a.editor
            .presets
            .get::<operandi_core::SubdividePreset>()
            .max_angle_degrees,
        30.
    );
    assert!(
        a.editor
            .presets
            .get::<operandi_core::SubdividePreset>()
            .linear_uvs
    );
    // Faceted: the cube's corners didn't move.
    let m = &a.editor.scene.active().unwrap().mesh;
    assert!(m.vertices[..8].iter().all(|p| p.abs() == DVec3::splat(0.5)));
    assert_eq!(
        a.editor.history.undo.last().unwrap().label,
        "Subdivide Polygons"
    );
}

#[test]
fn sub_06_alt_click_sds_subdivide_runs_twice_as_one_step() {
    let (mut a, ctx) = cube_app();
    frame(&mut a, &ctx, Modifiers::ALT, vec![]);
    assert_eq!(shown(&a, "SDS Subdivide").0, "SDS Subdivide 2X");
    let steps = a.editor.history.undo.len();
    let r = rect(&a, "SDS Subdivide").center();
    click(&mut a, &ctx, r, Modifiers::ALT);
    assert_eq!(faces(&a), 96, "{}", a.status);
    assert_eq!(a.editor.history.undo.len(), steps + 1);
    a.run(Command::Undo);
    assert_eq!(faces(&a), 6);
    // A plain click: one Catmull-Clark step.
    click(&mut a, &ctx, r, Modifiers::NONE);
    assert_eq!(faces(&a), 24);
    assert_eq!(
        a.editor
            .presets
            .get::<operandi_core::SubdividePreset>()
            .method,
        SubdivideMethod::CatmullClark
    );
}

#[test]
fn sub_03_d_and_options() {
    let (mut a, ctx) = cube_app();
    // D in Polygon mode: SDS Subdivide (Catmull-Clark).
    press(&mut a, &ctx, Key::D, Modifiers::NONE);
    assert_eq!(faces(&a), 24, "{}", a.status);
    // Shift+D in Polygon mode: the Options dialog; OK runs the method.
    press(&mut a, &ctx, Key::D, Modifiers::SHIFT);
    assert!(a.subdivide_ui.options.is_some(), "{}", a.status);
    assert_eq!(faces(&a), 24);
    a.subdivide_ui
        .options
        .as_mut()
        .unwrap()
        .set_method(SubdivideMethod::Faceted);
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    press(&mut a, &ctx, Key::Enter, Modifiers::NONE);
    assert!(a.subdivide_ui.options.is_none());
    assert_eq!(faces(&a), 96, "{}", a.status);
    assert_eq!(
        a.editor
            .presets
            .get::<operandi_core::SubdividePreset>()
            .method,
        SubdivideMethod::Faceted
    );
    // D in Edge mode stays Edge Subdivide.
    a.run(Command::Mode(Mode::Edge));
    let edge = a.editor.scene.active().unwrap().mesh.edges()[0];
    a.editor.scene.selection.edges = vec![edge];
    let before = faces(&a);
    press(&mut a, &ctx, Key::D, Modifiers::NONE);
    assert_eq!(faces(&a), before);
    assert_eq!(
        a.editor.history.undo.last().unwrap().label,
        "Edge Subdivide"
    );
}

#[test]
fn sub_09_vertex_mode_reports_no_polygons() {
    let (mut a, ctx) = cube_app();
    a.run(Command::Mode(Mode::Vertex));
    a.editor.scene.selection.vertices = vec![0];
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    let r = rect(&a, "SDS Subdivide").center();
    click(&mut a, &ctx, r, Modifiers::NONE);
    assert_eq!(faces(&a), 6);
    assert!(a.status.contains("No polygons selected"), "{}", a.status);
}

#[test]
fn sub_40_unsubdivide_opens_its_dialog_first() {
    let (mut a, ctx) = cube_app();
    a.execute("poly.subdivide ccsds");
    assert_eq!(faces(&a), 24, "{}", a.status);
    // UIC-03: the label ends in "..." because it opens a dialog.
    assert_eq!(shown(&a, "UnSubdivide").0, "UnSubdivide...");
    let r = rect(&a, "UnSubdivide").center();
    click(&mut a, &ctx, r, Modifiers::NONE);
    assert_eq!(faces(&a), 24, "a click alone changes nothing");
    // Starting values, in the spec's order.
    let d = a.subdivide_ui.unsubdivide.as_ref().expect("dialog open");
    let o = &d.options();
    assert_eq!(o.iterations, 1);
    assert!(!o.flat && o.uv_hint && o.material_hint);
    assert!(!o.part_hint && !o.smoothing_group_hint && !o.selection_set_hint);
    // Cancel (Escape): nothing changes.
    press(&mut a, &ctx, Key::Escape, Modifiers::NONE);
    assert!(a.subdivide_ui.unsubdivide.is_none());
    assert_eq!(faces(&a), 24);
    // OK runs it.
    click(&mut a, &ctx, r, Modifiers::NONE);
    press(&mut a, &ctx, Key::Enter, Modifiers::NONE);
    assert!(a.subdivide_ui.unsubdivide.is_none());
    assert_eq!(faces(&a), 6, "{}", a.status);
    assert_eq!(a.editor.history.undo.last().unwrap().label, "UnSubdivide");
    // Iteration 0 does nothing, and is remembered.
    a.execute("poly.subdivide ccsds");
    click(&mut a, &ctx, r, Modifiers::NONE);
    a.subdivide_ui
        .unsubdivide
        .as_mut()
        .unwrap()
        .set_iterations(0);
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    press(&mut a, &ctx, Key::Enter, Modifiers::NONE);
    assert_eq!(faces(&a), 24);
    assert_eq!(
        a.editor
            .presets
            .get::<operandi_core::UnsubdividePreset>()
            .iterations,
        0
    );
    click(&mut a, &ctx, r, Modifiers::NONE);
    assert_eq!(
        a.subdivide_ui
            .unsubdivide
            .as_ref()
            .unwrap()
            .options()
            .iterations,
        0
    );
    // Shift+click is no different from a click (no variant any more).
    let form = a.forms.get(toolbox_form::EDIT_FORM).unwrap();
    let unsub = form
        .controls
        .iter()
        .find(|c| c.command.as_deref() == Some("UnSubdivide"))
        .unwrap();
    assert_eq!(unsub.variants, Default::default());
    // Bare poly.unsubdiv opens the dialog too.
    a.subdivide_ui.unsubdivide = None;
    a.execute("poly.unsubdiv");
    assert!(a.subdivide_ui.unsubdivide.is_some());
}

#[test]
fn cen_center_row_and_alt_pairs() {
    let (mut a, ctx) = cube_app();
    // Move the cube off-centre, then centre it on X only.
    {
        let item = a.editor.scene.active_mut().unwrap();
        let mut mesh = operandi_core::Mesh::clone(&item.mesh);
        for p in &mut mesh.vertices {
            *p += DVec3::new(3., 4., 5.);
        }
        item.mesh = mesh.into();
    }
    a.editor.synchronize_identities().unwrap();
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    let x = rect(&a, "Center X").center();
    click(&mut a, &ctx, x, Modifiers::NONE);
    let lo = |a: &Operandi| {
        a.editor
            .scene
            .active()
            .unwrap()
            .mesh
            .vertices
            .iter()
            .fold(DVec3::INFINITY, |m, p| m.min(*p))
    };
    assert_eq!(lo(&a), DVec3::new(-0.5, 3.5, 4.5), "{}", a.status);
    // Alt+click Y = YZ.
    frame(&mut a, &ctx, Modifiers::ALT, vec![]);
    assert_eq!(shown(&a, "Center Y").0, "YZ");
    let y = rect(&a, "Center Y").center();
    click(&mut a, &ctx, y, Modifiers::ALT);
    assert_eq!(lo(&a), DVec3::splat(-0.5));
    // Item mode: nothing moves, a status message says why.
    a.run(Command::Mode(Mode::Item));
    a.execute("vert.center all");
    assert!(a.status.contains("component mode"), "{}", a.status);
}

#[test]
fn slide_button_follows_the_mode_and_freeze_dialog() {
    let (mut a, ctx) = cube_app();
    // Greyed in Polygon mode: a click does nothing.
    let r = rect(&a, "Edit Slide").center();
    click(&mut a, &ctx, r, Modifiers::NONE);
    assert!(a.editor.session.live.is_none());
    a.run(Command::Mode(Mode::Edge));
    let edge = a.editor.scene.active().unwrap().mesh.edges()[0];
    a.editor.scene.selection.edges = vec![edge];
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    click(&mut a, &ctx, r, Modifiers::NONE);
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::EdgeSlide),
        "{}",
        a.status
    );
    a.run(Command::Drop);
    a.run(Command::Mode(Mode::Polygon));
    // Freeze... opens a dialog; OK freezes a subdivision mesh.
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    let f = rect(&a, "poly.freeze").center();
    click(&mut a, &ctx, f, Modifiers::NONE);
    assert!(a.subdivide_ui.freeze.is_some());
    press(&mut a, &ctx, Key::Enter, Modifiers::NONE);
    assert!(a.subdivide_ui.freeze.is_none());
    assert_eq!(
        a.editor.history.undo.last().unwrap().label,
        "Freeze",
        "{}",
        a.status
    );
    assert!(faces(&a) > 6);
}

#[test]
fn script_arguments_use_fractions_and_radians() {
    let base = SubdivideOptions::default();
    let o = subdivide_ui::parse_subdivide(
        &base,
        &[
            "smooth",
            "fractal:0.25",
            "maxangle:0.5",
            "linear:true",
            "bound:crease",
        ],
    )
    .unwrap();
    assert_eq!(o.method, SubdivideMethod::Smooth);
    assert!((o.fractal_percent - 25.).abs() < 1e-9);
    assert!((o.max_angle_degrees - 0.5f64.to_degrees()).abs() < 1e-9);
    assert!(o.linear_uvs);
    assert_eq!(o.boundary, operandi_core::SubdivideBoundary::CreaseAll);
    assert!(subdivide_ui::parse_subdivide(&base, &["bogus"]).is_err());
    let u = subdivide_ui::parse_unsubdivide(&Default::default(), &["iteration:2", "flat:true"])
        .unwrap();
    assert_eq!((u.iterations, u.flat), (2, true));
    // Bare poly.subdivide opens the dialog (the map's Options... row).
    let (mut a, _) = cube_app();
    a.execute("poly.subdivide");
    assert!(a.subdivide_ui.options.is_some());
    a.execute("help poly.unsubdiv");
    assert!(a.status.starts_with("Help: poly.unsubdiv"));
}

#[test]
fn settings_survive_a_restart() {
    let (mut a, _) = cube_app();
    a.load_subdivide_settings();
    a.editor
        .presets
        .get_mut::<operandi_core::SubdividePreset>()
        .max_angle_degrees = 42.;
    a.editor
        .presets
        .get_mut::<operandi_core::UnsubdividePreset>()
        .flat = true;
    a.save_subdivide_settings();
    let saved = subdivide_ui::SavedSettings::load(&a.environment.paths().subdivide_settings());
    assert_eq!(saved.subdivide.max_angle_degrees, 42.);
    assert!(saved.unsubdivide.flat);
}

#[test]
fn uic_04_tooltips_name_the_shortcut_and_alternates() {
    let (a, _) = cube_app();
    let tip = shown(&a, "SDS Subdivide").1;
    assert!(tip.starts_with("SDS Subdivide"), "{tip}");
    assert!(
        tip.contains("[D]") || tip.contains("(D)") || tip.contains(" D"),
        "{tip}"
    );
    assert!(tip.contains("SDS Subdivide 2X"), "{tip}");
    let tip = shown(&a, "UnSubdivide").1;
    assert!(tip.starts_with("UnSubdivide..."), "{tip}");
    let tip = shown(&a, "Subdivide Options").1;
    assert!(tip.starts_with("Options..."), "{tip}");
    assert!(tip.contains("Shift+D"), "{tip}");
}

#[test]
fn sub_03_item_mode_d_subdivides_selected_meshes_and_shift_d_is_instance() {
    let (mut a, ctx) = cube_app();
    let first = a.editor.scene.active().unwrap().id;
    // A second cube item, both selected.
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::new(3., 0., 0.), DVec3::ONE, 0)
            .unwrap()
            .into();
    a.editor.synchronize_identities().unwrap();
    let second = a.editor.scene.active().unwrap().id;
    a.run(Command::Mode(Mode::Item));
    a.editor.scene.selection.item = Some(first);
    a.editor.scene.selection.also = vec![second];
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    let steps = a.editor.history.undo.len();
    press(&mut a, &ctx, Key::D, Modifiers::NONE);
    let count = |a: &Operandi, id: u64| a.editor.scene.item(id).unwrap().mesh.faces.len();
    assert_eq!(
        (count(&a, first), count(&a, second)),
        (24, 24),
        "{}",
        a.status
    );
    assert_eq!(a.editor.history.undo.len(), steps + 1, "one undo step");
    // Shift+D in Item mode: Instance (DX-38), one instance per selected
    // mesh, one undo step.
    let items = a.editor.scene.items.len();
    let steps = a.editor.history.undo.len();
    press(&mut a, &ctx, Key::D, Modifiers::SHIFT);
    assert!(a.subdivide_ui.options.is_none());
    assert_eq!(a.editor.scene.items.len(), items + 2, "{}", a.status);
    assert_eq!(a.editor.history.undo.len(), steps + 1, "one undo step");
    let made = a.editor.scene.selected_items();
    assert_eq!(made.len(), 2);
    for id in made {
        assert!(a.editor.scene.item(id).unwrap().instance_of.is_some());
    }
}

#[test]
fn sub_50_freeze_dialog_defaults_and_tessellation() {
    let (mut a, ctx) = cube_app();
    a.run(Command::ToggleSubdivision(
        operandi_core::subdivision::Toggle::CatmullClark,
    ));
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    let f = rect(&a, "poly.freeze").center();
    click(&mut a, &ctx, f, Modifiers::NONE);
    let d = &a
        .subdivide_ui
        .freeze
        .as_ref()
        .expect("dialog open")
        .options();
    // SUB-50 (corrected) starting values.
    assert_eq!(d.curves_as, subdivide_ui::FreezeCurves::Faces);
    assert!(!d.make_holes && !d.save_morph);
    assert!(d.displacement && d.deformation && d.tessellation && d.fixed_spans);
    assert_eq!(d.refinement_angle, "5.0 °");
    assert!(d.morph_name.is_empty());
    // FRZ v1: Curves as, Tessellation and Fixed Spans act; Curve
    // Refinement Angle is greyed while Fixed Spans is on; the rest stay
    // greyed with "Not available yet".
    {
        let form = a.subdivide_ui.freeze.as_mut().unwrap().form();
        let enabled = |form: &form::Dialog, key: &str| {
            let row = form.rows.iter().find(|r| r.key == key).unwrap();
            form.values.enabled(row)
        };
        for key in ["curves_as", "tessellation", "fixed_spans"] {
            assert!(enabled(form, key), "{key}");
        }
        for key in [
            "make_holes",
            "drill_axis",
            "displacement",
            "deformation",
            "save_morph",
            "morph_name",
        ] {
            assert!(!enabled(form, key), "{key}");
        }
        assert!(!enabled(form, "refinement_angle"));
        form.values.set("fixed_spans", form::Value::Bool(false));
        assert!(enabled(form, "refinement_angle"));
        form.values.set("fixed_spans", form::Value::Bool(true));
    }
    // Tessellation off: OK keeps the cage; nothing changes (FRZ-09), and it's remembered.
    a.subdivide_ui
        .freeze
        .as_mut()
        .unwrap()
        .set_tessellation(false);
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    let steps = a.editor.history.undo.len();
    press(&mut a, &ctx, Key::Enter, Modifiers::NONE);
    assert!(a.subdivide_ui.freeze.is_none());
    assert_eq!(a.editor.history.undo.len(), steps);
    assert_eq!(faces(&a), 6);
    assert!(a.status.contains("Nothing to freeze"), "{}", a.status);
    // Reopened: still off. On + OK: frozen in one step.
    click(&mut a, &ctx, f, Modifiers::NONE);
    assert!(
        !a.subdivide_ui
            .freeze
            .as_ref()
            .unwrap()
            .options()
            .tessellation
    );
    a.subdivide_ui
        .freeze
        .as_mut()
        .unwrap()
        .set_tessellation(true);
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    press(&mut a, &ctx, Key::Enter, Modifiers::NONE);
    assert_eq!(
        a.editor.history.undo.last().unwrap().label,
        "Freeze",
        "{}",
        a.status
    );
    assert!(faces(&a) > 6);
    // Cancel changes nothing.
    click(&mut a, &ctx, f, Modifiers::NONE);
    let before = a.editor.scene.clone();
    press(&mut a, &ctx, Key::Escape, Modifiers::NONE);
    assert!(a.subdivide_ui.freeze.is_none());
    assert_eq!(a.editor.scene, before);
}

/// UIC-86/87: labels never overlap their fields; labels share one right
/// edge; fields and checkboxes share one left edge.
fn assert_dialog_layout(a: &Operandi, which: &str) {
    let rows = &a.subdivide_ui.last_rows;
    assert!(!rows.is_empty(), "{which}: no rows drawn");
    let left = rows[0].field.left();
    let mut right_edge = None;
    for (i, row) in rows.iter().enumerate() {
        assert!(
            (row.field.left() - left).abs() < 1.,
            "{which} row {i}: field starts at {} not {left}",
            row.field.left()
        );
        if let Some(label) = row.label {
            assert!(
                !label.intersects(row.field) && label.right() <= row.field.left(),
                "{which} row {i}: label {label:?} overlaps field {:?}",
                row.field
            );
            let r = *right_edge.get_or_insert(label.right());
            assert!(
                (label.right() - r).abs() < 1.,
                "{which} row {i}: label edge"
            );
        }
        if i > 0 {
            assert!(
                rows[i - 1].field.bottom() <= row.field.top() + 0.5,
                "{which}: rows overlap"
            );
        }
    }
}

#[test]
fn dialogs_lay_out_without_overlaps() {
    let (mut a, ctx) = cube_app();
    for (which, command) in [
        ("Subdivide", "poly.subdivide"),
        ("UnSubdivide", "poly.unsubdiv"),
        ("Freeze", "poly.freeze"),
    ] {
        a.execute(command);
        for _ in 0..3 {
            frame(&mut a, &ctx, Modifiers::NONE, vec![]);
        }
        assert_dialog_layout(&a, which);
        press(&mut a, &ctx, Key::Escape, Modifiers::NONE);
        assert!(!a.subdivide_ui.open(), "{which} closed");
    }
}

/// A Bezier curve (FRZ-C01) as the only element of the active mesh.
fn bezier_app() -> (Operandi, Context) {
    use operandi_core::{Face, Mesh, PolygonRepresentation, curves::CurveKind};
    let (mut a, ctx) = cube_app();
    let mut m = Mesh {
        vertices: vec![
            DVec3::ZERO,
            DVec3::new(1., 1., 0.),
            DVec3::new(2., 0., 0.),
            DVec3::new(3., 1., 0.),
        ],
        ..Default::default()
    };
    m.faces.push(Face {
        vertices: vec![0, 1, 2, 3],
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    });
    m.attributes
        .representations
        .push(PolygonRepresentation::Polyline {
            closed: false,
            curve: CurveKind::Bezier,
        });
    a.editor.scene.active_mut().unwrap().mesh = m.into();
    a.editor.synchronize_identities().unwrap();
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    (a, ctx)
}

#[test]
fn frz_curves_freeze_from_the_dialog_and_the_command() {
    // Dialog: 2 Point Polygons, OK: 36 segments, one undo step, remembered.
    let (mut a, ctx) = bezier_app();
    let f = rect(&a, "poly.freeze").center();
    click(&mut a, &ctx, f, Modifiers::NONE);
    a.subdivide_ui
        .freeze
        .as_mut()
        .unwrap()
        .form()
        .values
        .set("curves_as", form::Value::Choice(0));
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    let steps = a.editor.history.undo.len();
    press(&mut a, &ctx, Key::Enter, Modifiers::NONE);
    assert_eq!(faces(&a), 36, "{}", a.status);
    assert_eq!(a.editor.history.undo.len(), steps + 1);
    assert_eq!(
        a.subdivide_ui.freeze_options.curves_as,
        subdivide_ui::FreezeCurves::TwoPointPolygons
    );
    // The plain Freeze action reuses the remembered values without a dialog.
    let (mut b, _ctx) = bezier_app();
    b.subdivide_ui.freeze_options = a.subdivide_ui.freeze_options.clone();
    b.execute("freeze");
    assert_eq!(faces(&b), 36, "{}", b.status);
    assert!(b.subdivide_ui.freeze.is_none());

    // Command form: readable or numeric face, radians or degrees cangle.
    let (mut a, _ctx) = bezier_app();
    a.execute("poly.freeze face:polylines");
    assert!(a.subdivide_ui.freeze.is_none());
    let m = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(
        (m.faces.len(), m.faces[0].vertices.len()),
        (1, 37),
        "{}",
        a.status
    );
    a.run(Command::Undo);
    a.execute("poly.freeze face:2 fixed:false cangle:20deg disp:false");
    let coarse = faces(&a);
    assert!((2..36).contains(&coarse), "{coarse} {}", a.status);
    assert!(
        a.status.contains("disp accepted and ignored"),
        "{}",
        a.status
    );
    assert_eq!(a.subdivide_ui.freeze_options.refinement_angle, "20.0 °");
    a.run(Command::Undo);
    a.execute("poly.freeze cangle:0.0349066");
    assert!(faces(&a) > coarse, "{}", a.status);
    assert_eq!(a.subdivide_ui.freeze_options.refinement_angle, "2.0 °");
    // Bad values change nothing.
    let before = a.editor.scene.clone();
    a.execute("poly.freeze face:3");
    assert!(a.status.starts_with("face takes"), "{}", a.status);
    a.execute("poly.freeze cangle:0");
    assert!(a.status.contains("above 0"), "{}", a.status);
    assert_eq!(a.editor.scene, before);
    a.execute("help poly.freeze");
    assert!(a.status.contains("accepted and ignored"), "{}", a.status);
}

/// A 10 x 10 grid of quads, the active mesh, in Polygon mode.
fn grid_app() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let n = 10u32;
    let mut mesh = operandi_core::Mesh::default();
    for y in 0..=n {
        for x in 0..=n {
            mesh.vertices.push(DVec3::new(x as f64, y as f64, 0.));
        }
    }
    for y in 0..n {
        for x in 0..n {
            let k = y * (n + 1) + x;
            mesh.faces.push(operandi_core::Face {
                vertices: vec![k, k + 1, k + n + 2, k + n + 1],
                uv: None,
                material: "Default".into(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.run(Command::Mode(Mode::Polygon));
    a
}

/// Jason, 2026-11-12: a Subdivide whose result takes over half of the free
/// memory asks first; Return subdivides, Escape changes nothing.
#[test]
fn a_subdivide_needing_over_half_the_free_memory_asks_first() {
    let ctx = Context::default();
    let mut a = grid_app();
    // 400 polygons after one pass, 2 MB at 5 KB each; 3 MB "free".
    a.subdivide_ui.free_memory = Some(|| 3_000_000);
    a.subdivide_with(SubdivideMethod::Faceted, 1);
    let large = a.subdivide_ui.large.as_ref().expect("asks");
    assert_eq!(large.estimate.polygons, 400);
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 100);
    assert!(a.has_modal());
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    press(&mut a, &ctx, Key::Escape, Modifiers::NONE);
    assert!(a.subdivide_ui.large.is_none());
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 100);
    assert!(a.status.contains("nothing changed"), "{}", a.status);
    // Asked again, Return goes ahead as one undo step.
    let steps = a.editor.history.undo.len();
    a.subdivide_with(SubdivideMethod::Faceted, 1);
    frame(&mut a, &ctx, Modifiers::NONE, vec![]);
    press(&mut a, &ctx, Key::Enter, Modifiers::NONE);
    assert!(a.subdivide_ui.large.is_none());
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 400);
    assert_eq!(a.editor.history.undo.len(), steps + 1);
    // Plenty free: no question.
    a.subdivide_ui.free_memory = Some(|| 1 << 40);
    a.subdivide_with(SubdivideMethod::Faceted, 1);
    assert!(a.subdivide_ui.large.is_none());
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 1600);
}

/// More than all of it: no question, the editor refuses with the numbers.
#[test]
fn a_subdivide_needing_more_than_the_free_memory_is_refused() {
    let mut a = grid_app();
    a.subdivide_ui.free_memory = Some(|| 1_000);
    let before = a.editor.history.undo.len();
    a.subdivide_with(SubdivideMethod::Faceted, 1);
    assert!(
        a.subdivide_ui.large.is_none(),
        "no question above free memory"
    );
    assert!(
        a.status.contains("400 polygons") && a.status.contains("nothing changed"),
        "{}",
        a.status
    );
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 100);
    assert_eq!(a.editor.history.undo.len(), before);
}
