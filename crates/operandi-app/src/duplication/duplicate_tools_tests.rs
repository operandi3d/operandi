//! The Duplicate tools in the app (specifications/duplicate-tools): the
//! Duplicate tab's rows (DS-60), viewport clicks and drags (MR-30/31,
//! LT-30, CL-20, AR-20..22, RA-20/21), typed fields and the name-map ids.
use super::tests::{draw, pointer};
use super::toolbox_form_tests::button_by_line;
use super::*;

/// A 2 x 2 grid of unit quads in XZ (x, z from 0 to 2), seen from the top.
fn grid_app(ctx: &Context) -> Operandi {
    theme::install(ctx);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=2u32 {
        for i in 0..=2u32 {
            mesh.vertices.push(DVec3::new(i as f64, 0., j as f64));
        }
    }
    for j in 0..2u32 {
        for i in 0..2u32 {
            let v = j * 3 + i;
            mesh.faces
                .push(super::tests::face(vec![v, v + 3, v + 4, v + 1]));
        }
    }
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.camera.view = View::Top;
    a.frame(false);
    a
}

fn mesh(a: &Operandi) -> &operandi_core::Mesh {
    &a.editor.scene.active().unwrap().mesh
}

fn screen(a: &Operandi, p: DVec3) -> Pos2 {
    a.camera.project(p, a.viewport_rect).unwrap().0
}

fn click(a: &mut Operandi, ctx: &Context, size: Vec2, at: Pos2) {
    draw(a, ctx, size, vec![Event::PointerMoved(at)]);
    draw(a, ctx, size, vec![pointer(at, true, Modifiers::NONE)]);
    draw(a, ctx, size, vec![pointer(at, false, Modifiers::NONE)]);
}

fn drag(a: &mut Operandi, ctx: &Context, size: Vec2, from: Pos2, to: Pos2) {
    draw(a, ctx, size, vec![Event::PointerMoved(from)]);
    draw(a, ctx, size, vec![pointer(from, true, Modifiers::NONE)]);
    for k in 1..=4 {
        let at = from + (to - from) * (k as f32 / 4.);
        draw(a, ctx, size, vec![Event::PointerMoved(at)]);
    }
    draw(a, ctx, size, vec![pointer(to, false, Modifiers::NONE)]);
}

fn right_drag(a: &mut Operandi, ctx: &Context, size: Vec2, from: Pos2, to: Pos2) {
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Secondary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    draw(a, ctx, size, vec![Event::PointerMoved(from)]);
    draw(a, ctx, size, vec![button(from, true)]);
    draw(a, ctx, size, vec![Event::PointerMoved(to)]);
    draw(a, ctx, size, vec![button(to, false)]);
}

fn live_params(a: &Operandi) -> Parameters {
    a.editor.session.live.as_ref().unwrap().params.clone()
}

fn live_tool(a: &Operandi) -> Option<Tool> {
    a.editor.session.live.as_ref().map(|l| l.tool)
}

#[test]
fn duplicate_tab_rows_follow_the_spec_order() {
    let ctx = Context::default();
    let size = vec2(1440., 1400.);
    let mut a = grid_app(&ctx);
    a.category = toolbox::Category::Duplicate;
    draw(&mut a, &ctx, size, vec![]);
    // DS-60: Extend · Clone · Array · Duplicate.
    let order = [
        "Duplicate Extend",
        "Radial Sweep",
        "Duplicate Clone",
        "Clone",
        "Duplicate Array",
        "Array",
        "Radial Array",
        "Duplicate Duplicate",
        "Mirror",
    ];
    let tops: Vec<f32> = order
        .iter()
        .map(|k| {
            a.toolbox_rects
                .get(*k)
                .unwrap_or_else(|| panic!("{k}"))
                .top()
        })
        .collect();
    assert!(tops.windows(2).all(|w| w[0] < w[1]), "{tops:?}");
    for tool in [
        Tool::Lathe,
        Tool::Clone,
        Tool::Array,
        Tool::RadialArray,
        Tool::Mirror,
    ] {
        // The tab is a form: its rows are found by their command lines,
        // or a pop-up button's by its first line (N23: Clone heads the
        // Clone / Scatter Clone button).
        let row = a
            .form_rects
            .get(tool.label())
            .or_else(|| a.form_rects.get(&format!("flyout:{}", tool.label())))
            .unwrap_or_else(|| panic!("{}", tool.label()))
            .center();
        click(&mut a, &ctx, size, row);
        assert_eq!(live_tool(&a), Some(tool), "{}", a.status);
        // Arming changes nothing (DS-30).
        assert_eq!(mesh(&a).faces.len(), 4);
        draw(&mut a, &ctx, size, vec![]);
        click(&mut a, &ctx, size, row);
        assert_eq!(live_tool(&a), None, "a second click drops it");
    }
    // DS-61 (v1.4): the button is Radial Sweep; "lathe" is an alias.
    assert_eq!(Tool::Lathe.label(), "Radial Sweep");
    assert_eq!(tool_info::by_name("lathe"), Some(Tool::Lathe));
}

#[test]
fn mirror_click_mirrors_and_the_centre_grip_slides_the_plane() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    let mut a = grid_app(&ctx);
    draw(&mut a, &ctx, size, vec![]);
    a.activate(Tool::Mirror);
    draw(&mut a, &ctx, size, vec![]);
    // MR-30: a click away from the handle shows the result; the plane
    // stays at the origin.
    let away = screen(&a, DVec3::new(1.5, 0., 1.5));
    click(&mut a, &ctx, size, away);
    assert!(live_params(&a).duplicate.placed, "{}", a.status);
    assert_eq!(live_params(&a).duplicate.mirror.center, DVec3::ZERO);
    // The seam on x = 0 welds: 8 polygons, 9 + 6 points.
    assert_eq!((mesh(&a).faces.len(), mesh(&a).vertices.len()), (8, 15));
    // MR-31: the centre grip slides the plane along X.
    draw(&mut a, &ctx, size, vec![]);
    let from = screen(&a, DVec3::ZERO);
    let to = screen(&a, DVec3::new(-1., 0., 0.));
    drag(&mut a, &ctx, size, from, to);
    let centre = live_params(&a).duplicate.mirror.center;
    assert!(
        (centre.x + 1.).abs() < 0.05 && centre.y == 0. && centre.z == 0.,
        "{centre}"
    );
    a.run(Command::Drop);
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Mirror");
}

#[test]
fn clone_press_drag_sets_the_step_and_right_drag_the_count() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    let mut a = grid_app(&ctx);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    draw(&mut a, &ctx, size, vec![]);
    a.activate(Tool::Clone);
    draw(&mut a, &ctx, size, vec![]);
    // CL-20: Offset = pointer − press on the work plane.
    let from = screen(&a, DVec3::new(0.5, 0., 0.5));
    let to = screen(&a, DVec3::new(0.5, 0., 3.5));
    drag(&mut a, &ctx, size, from, to);
    let offset = live_params(&a).duplicate.clone.offset;
    assert!(
        (offset - DVec3::new(0., 0., 3.)).length() < 0.05,
        "{offset}"
    );
    assert_eq!(mesh(&a).faces.len(), 5);
    // CL-22: right-drag 40 px right adds two clones.
    let at = screen(&a, DVec3::new(3., 0., 1.));
    right_drag(&mut a, &ctx, size, at, at + vec2(40., 0.));
    assert_eq!(live_params(&a).duplicate.clone.count, 3, "{}", a.status);
    assert_eq!(mesh(&a).faces.len(), 7);
}

#[test]
fn array_first_click_spaces_by_size() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    let mut a = grid_app(&ctx);
    draw(&mut a, &ctx, size, vec![]);
    a.activate(Tool::Array);
    draw(&mut a, &ctx, size, vec![]);
    // AR-20: Offset = the source's size (flat in Y), 2 × 1 × 2 pieces.
    let at = screen(&a, DVec3::new(3., 0., 3.));
    click(&mut a, &ctx, size, at);
    let p = live_params(&a).duplicate.array;
    assert_eq!(p.offset, DVec3::new(2., 0., 2.), "{}", a.status);
    assert_eq!(mesh(&a).faces.len(), 16);
}

#[test]
fn radial_array_click_sets_the_centre() {
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    let mut a = grid_app(&ctx);
    draw(&mut a, &ctx, size, vec![]);
    a.activate(Tool::RadialArray);
    draw(&mut a, &ctx, size, vec![]);
    // LT-30 / RA-20: the first click sets Center on the work plane.
    let at = screen(&a, DVec3::new(-0.5, 0., 1.));
    click(&mut a, &ctx, size, at);
    let c = live_params(&a).duplicate.radial.circle.center;
    assert!(
        (c - DVec3::new(-0.5, 0., 1.)).length() < 0.05,
        "{c} {}",
        a.status
    );
    assert_eq!(mesh(&a).faces.len(), 4 * 8);
    // A second click away moves the axis; it doesn't copy the copies.
    let steps = a.editor.history.undo.len();
    let at = screen(&a, DVec3::new(-0.5, 0., 2.));
    click(&mut a, &ctx, size, at);
    let c = live_params(&a).duplicate.radial.circle.center;
    assert!((c - DVec3::new(-0.5, 0., 2.)).length() < 0.05, "{c}");
    assert_eq!(mesh(&a).faces.len(), 4 * 8);
    assert_eq!(a.editor.history.undo.len(), steps);
}

#[test]
fn typed_duplicate_fields_reach_the_tool() {
    let ctx = Context::default();
    let mut a = grid_app(&ctx);
    a.activate(Tool::Mirror);
    a.fields.insert("mirror_angle".into(), "90".into());
    assert!(a.confirm_fields(), "{}", a.status);
    assert_eq!(live_params(&a).duplicate.mirror.angle, 90.);
    a.fields.insert("mirror_angle".into(), "400".into());
    assert!(!a.confirm_fields());
    a.clear_fields();
    a.run(Command::Drop);
    a.activate(Tool::Lathe);
    a.fields.insert("circle_count".into(), "8".into());
    assert!(a.confirm_fields(), "{}", a.status);
    assert_eq!(live_params(&a).duplicate.lathe.circle.count, 8);
    a.run(Command::Drop);
    a.activate(Tool::Array);
    a.fields.insert("array_count1".into(), "2".into());
    assert!(a.confirm_fields(), "{}", a.status);
    assert_eq!(live_params(&a).duplicate.array.count, [2, 2, 2]);
}

#[test]
fn name_map_tool_ids_start_the_tools() {
    let ctx = Context::default();
    let mut a = grid_app(&ctx);
    for (id, tool) in [
        ("\"*.mirror\"", Tool::Mirror),
        ("\"Radial Sweep\"", Tool::Lathe),
        ("\"*.clone\"", Tool::Clone),
        ("\"*.array\"", Tool::Array),
        ("\"*.Radial Array\"", Tool::RadialArray),
    ] {
        a.execute(&format!("tool.set {id} on"));
        assert_eq!(live_tool(&a), Some(tool), "{id}: {}", a.status);
        a.run(Command::Drop);
    }
}

#[test]
fn detriangulate_opens_its_dialog_and_ok_is_one_step() {
    let ctx = Context::default();
    // Tall enough to show the whole Polygon tab without scrolling.
    let size = vec2(1440., 2200.);
    let mut a = grid_app(&ctx);
    // Split each quad on its diagonal.
    let m = mesh(&a).clone();
    let mut tri = m.clone();
    tri.faces = m
        .faces
        .iter()
        .flat_map(|f| {
            let v = &f.vertices;
            [
                super::tests::face(vec![v[0], v[1], v[2]]),
                super::tests::face(vec![v[0], v[2], v[3]]),
            ]
        })
        .collect();
    a.editor.scene.active_mut().unwrap().mesh = tri.into();
    a.category = toolbox::Category::Polygon;
    draw(&mut a, &ctx, size, vec![]);
    let row = button_by_line(&a, "Detriangulate").center();
    click(&mut a, &ctx, size, row);
    assert!(a.detriangulate.is_some(), "{}", a.status);
    // The dialog's values are the remembered options.
    let o = a.detriangulate.as_ref().unwrap().options();
    assert_eq!(
        o,
        *a.editor.presets.get::<operandi_core::DetriangulatePreset>()
    );
    // While it is open, the viewport shows the edges OK would remove:
    // the four diagonals, as the edit itself pairs them.
    let preview = a.detriangulate.as_ref().unwrap().preview.clone();
    assert_eq!(preview.len(), 4);
    assert_eq!(preview, a.editor.detriangulate_preview(&o).unwrap());
    // Options that join nothing preview nothing, at once.
    a.detriangulate
        .as_mut()
        .unwrap()
        .form
        .values
        .set("max_angle", form::Value::Number(10.));
    draw(&mut a, &ctx, size, vec![]);
    assert!(a.detriangulate.as_ref().unwrap().preview.is_empty());
    a.detriangulate = None;
    let steps = a.editor.history.undo.len();
    // Closed: no preview is drawn (it lives in the dialog).
    assert!(a.detriangulate.is_none());
    a.run_detriangulate();
    assert_eq!(mesh(&a).faces.len(), 4, "{}", a.status);
    assert_eq!(a.editor.history.undo.len(), steps + 1);
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Detriangulate");
    // Nothing left to join: nothing changes, and the status says so.
    a.run_detriangulate();
    assert_eq!(a.editor.history.undo.len(), steps + 1);
    assert!(a.status.contains("No triangle pairs"), "{}", a.status);
}
