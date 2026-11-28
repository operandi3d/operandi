//! UV editing stage 1 in the app (uv-editing-2026-11-16): the transform
//! tools in a UV view (handles, Ctrl steps, undo), paint and lasso
//! selection, the E7 buttons and their dialogs, the UV Maps panel and the
//! Checker overlay.
use super::tests::{draw, draw_held, pointer};
use super::*;
use glam::DVec2;
use operandi_core::uv_maps::UvMapEdit;

const SIZE: Vec2 = vec2(1440., 1000.);

/// A 2 x 2 grid of unit quads in XZ with "Texture" UVs (x / 2, z / 2), in
/// Polygon mode, the active view switched to UV.
fn uv_app(ctx: &Context) -> Operandi {
    theme::install(ctx);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=2u32 {
        for i in 0..=2u32 {
            mesh.vertices.push(DVec3::new(i as f64, 0., j as f64));
        }
    }
    let mut rows = Vec::new();
    for j in 0..2u32 {
        for i in 0..2u32 {
            let v = j * 3 + i;
            let corners = vec![v, v + 3, v + 4, v + 1];
            rows.push(
                corners
                    .iter()
                    .map(|&c| {
                        let p = mesh.vertices[c as usize];
                        Some(DVec2::new(p.x / 2., p.z / 2.))
                    })
                    .collect(),
            );
            mesh.faces.push(super::tests::face(corners));
        }
    }
    mesh.uv_maps.insert("Texture".into(), rows);
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.run(Command::Mode(Mode::Polygon));
    a.toggle_uv_view();
    draw(&mut a, ctx, SIZE, vec![]);
    a
}

fn view(a: &Operandi) -> uv_view::UvView {
    a.uv_views[&a.active_view].clone()
}

fn uvs(a: &Operandi) -> Vec<Vec<DVec2>> {
    a.editor.scene.active().unwrap().mesh.uv_maps["Texture"]
        .iter()
        .map(|r| r.iter().map(|p| p.unwrap()).collect())
        .collect()
}

/// A primary drag from `from` to `to` in steps, with `mods` held.
fn drag(a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2, mods: Modifiers) {
    draw_held(a, ctx, SIZE, vec![Event::PointerMoved(from)], mods);
    draw_held(a, ctx, SIZE, vec![pointer(from, true, mods)], mods);
    for k in 1..=8 {
        let p = from + (to - from) * (k as f32 / 8.);
        draw_held(a, ctx, SIZE, vec![Event::PointerMoved(p)], mods);
    }
    draw_held(a, ctx, SIZE, vec![pointer(to, false, mods)], mods);
    draw(a, ctx, SIZE, vec![]);
}

fn right_drag(a: &mut Operandi, ctx: &Context, path: &[Pos2]) {
    let press = |pos: Pos2, pressed: bool| Event::PointerButton {
        pos,
        button: PointerButton::Secondary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    draw(a, ctx, SIZE, vec![Event::PointerMoved(path[0])]);
    draw(a, ctx, SIZE, vec![press(path[0], true)]);
    for &p in &path[1..] {
        draw(a, ctx, SIZE, vec![Event::PointerMoved(p)]);
    }
    draw(a, ctx, SIZE, vec![press(*path.last().unwrap(), false)]);
    draw(a, ctx, SIZE, vec![]);
}

#[test]
fn move_in_a_uv_view_drags_uvs_on_the_u_arrow_and_leaves_3d() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    let points = a.editor.scene.active().unwrap().mesh.vertices.clone();
    a.run(Command::Activate(Tool::Move));
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.uv_tool_live(), "Move started in a UV view edits UVs");
    let v = view(&a);
    // Everything (nothing selected): the pivot is the tile's centre.
    let pivot = a.uv_tool_pivot().unwrap();
    assert!((pivot - DVec2::splat(0.5)).length() < 1e-9);
    let c = v.to_screen(v.rect, pivot);
    // Along the U arrow: only U changes, even when the pointer wanders.
    drag(
        &mut a,
        &ctx,
        c + vec2(40., 0.),
        c + vec2(100., 30.),
        Modifiers::NONE,
    );
    let du = 60. / v.zoom;
    let u = uvs(&a);
    assert!((u[0][0].x - du).abs() < 1e-3, "{:?} {}", u[0][0], a.status);
    assert!(u[0][0].y.abs() < 1e-9);
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices, points);
    let steps = a.editor.history.undo.len();
    a.run(Command::Drop);
    assert_eq!(a.editor.history.undo.len(), steps + 1);
    a.run(Command::Undo);
    assert!(uvs(&a)[0][0].x.abs() < 1e-9);
}

#[test]
fn ctrl_constrains_move_and_steps_rotate() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.run(Command::Activate(Tool::Move));
    draw(&mut a, &ctx, SIZE, vec![]);
    let v = view(&a);
    let c = v.to_screen(v.rect, a.uv_tool_pivot().unwrap());
    // Off the handles with Ctrl: the larger direction wins.
    drag(
        &mut a,
        &ctx,
        c + vec2(-60., 60.),
        c + vec2(-20., 70.),
        Modifiers::CTRL,
    );
    // Each drag is a step of the session (as in 3D); its values are the
    // step's.
    let step = |a: &Operandi| {
        a.editor
            .session
            .last_transform_step
            .as_ref()
            .unwrap()
            .params
            .clone()
    };
    let d = step(&a).delta;
    assert!(d.x.abs() > 0. && d.y == 0., "{d}");
    a.run(Command::Drop);

    a.run(Command::Activate(Tool::Rotate));
    draw(&mut a, &ctx, SIZE, vec![]);
    let v = view(&a);
    let c = v.to_screen(v.rect, a.uv_tool_pivot().unwrap());
    drag(
        &mut a,
        &ctx,
        c + vec2(100., 0.),
        c + vec2(80., -45.),
        Modifiers::CTRL,
    );
    let angle = step(&a).delta.z;
    assert!(angle > 0., "counter-clockwise on screen is positive (Q6)");
    assert!((angle / 15.).fract().abs() < 1e-9, "{angle}");
}

#[test]
fn paint_and_lasso_select_in_the_uv_view() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    let v = view(&a);
    // Paint across the two lower polygons.
    let from = v.to_screen(v.rect, DVec2::new(0.25, 0.25));
    let to = v.to_screen(v.rect, DVec2::new(0.75, 0.25));
    drag(&mut a, &ctx, from, to, Modifiers::NONE);
    let mut faces = a.editor.scene.selection.faces.clone();
    faces.sort();
    assert_eq!(faces, vec![0, 1], "{}", a.status);
    // Lasso around the upper-right polygon alone: it replaces.
    let corner = |u: f64, w: f64| v.to_screen(v.rect, DVec2::new(u, w));
    right_drag(
        &mut a,
        &ctx,
        &[
            corner(0.45, 0.45),
            corner(1.05, 0.45),
            corner(1.05, 1.05),
            corner(0.45, 1.05),
            corner(0.45, 0.46),
        ],
    );
    assert_eq!(a.editor.scene.selection.faces, vec![3], "{}", a.status);
}

#[test]
fn edit_tab_island_buttons_run_presets_and_ctrl_opens_the_dialog() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.category = toolbox::Category::Edit;
    let tall = vec2(1440., 2600.);
    draw(&mut a, &ctx, tall, vec![]);
    let flip = a.toolbox_rects["Flip U"].center();
    draw(&mut a, &ctx, tall, vec![Event::PointerMoved(flip)]);
    draw(
        &mut a,
        &ctx,
        tall,
        vec![
            pointer(flip, true, Modifiers::NONE),
            pointer(flip, false, Modifiers::NONE),
        ],
    );
    assert_eq!(
        a.editor.history.undo.last().unwrap().label,
        "Flip U",
        "{}",
        a.status
    );
    assert!((uvs(&a)[0][0].x - 1.).abs() < 1e-9);
    // Ctrl/Cmd+click Fit UVs: the dialog, with the remembered values.
    let fit = a.toolbox_rects["Fit UVs"].center();
    a.island_click("uv.fitUVs", Modifiers::COMMAND);
    let Some(uv_dialogs::UvDialog::Island(form, kind)) = &a.uv_dialog else {
        panic!("Fit UVs dialog didn't open: {}", a.status);
    };
    assert_eq!(*kind, operandi_core::uv_edit::RuleKind::Fit);
    assert!(form.values.bool("keep_proportion"));
    let values = form.values.clone();
    a.uv_dialog = None;
    a.island_dialog_ok(operandi_core::uv_edit::RuleKind::Fit, &values)
        .unwrap();
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Fit UVs");
    let _ = fit;
}

#[test]
fn island_dialog_values_make_the_rule() {
    use operandi_core::uv_edit::{AlignTo, Grouping, IslandMemory, IslandRule, RuleKind, UvAxis};
    let memory = IslandMemory::default();
    let rows = uv_edit_ui::island_rows(RuleKind::Align, UvAxis::U, Some(AlignTo::High), &memory);
    let values = form::Values::defaults(&rows);
    let op = uv_edit_ui::island_op(RuleKind::Align, &values);
    assert_eq!(
        op.rule,
        IslandRule::Align {
            axis: UvAxis::U,
            to: AlignTo::High
        }
    );
    assert_eq!(op.grouping, Grouping::Entire);
    let rows = uv_edit_ui::island_rows(RuleKind::Mirror, UvAxis::V, None, &memory);
    let op = uv_edit_ui::island_op(RuleKind::Mirror, &form::Values::defaults(&rows));
    assert_eq!(
        op.rule,
        IslandRule::Mirror {
            axis: UvAxis::V,
            center: 0.5
        }
    );
}

#[test]
fn uv_maps_panel_lists_selects_and_the_view_follows() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.run(Command::UvMap(UvMapEdit::New(Some("Iris".into()))));
    a.upper_frame.add(panel_registry::PanelKind::UvMaps);
    for _ in 0..3 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    let rects = a.uv_maps_tree.rects.clone();
    assert_eq!(rects.len(), 2, "two rows");
    // Rows are sorted: Iris (0), Texture (1). Click Texture.
    let at = rects.iter().find(|(k, _)| *k == 1).unwrap().1.center();
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![
            pointer(at, true, Modifiers::NONE),
            pointer(at, false, Modifiers::NONE),
        ],
    );
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(
        operandi_core::uv_maps::current_map(mesh).as_deref(),
        Some("Texture"),
        "{}",
        a.status
    );
}

#[test]
fn checker_is_off_by_default_and_rides_in_the_look_uniform() {
    let d = view_display::Drawing::default();
    assert!(!d.checker, "UVT-114: off by default");
    let look = gpu::Look {
        checker: true,
        ..gpu::Look::of(view_display::Group::default())
    };
    assert!(look.checker);
    let shader = gpu::world_shader_for_tests("");
    assert!(shader.contains("fn op_checker"));
    assert!(shader.contains("look.glass.y > 0.5"));
    // The UV view's toggle, off by default too.
    assert!(!uv_view::UvView::default().show_checker);
}

#[test]
fn plain_polygons_carry_the_selected_maps_uvs_for_the_checker() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.run(Command::UvMap(UvMapEdit::New(Some("Other".into()))));
    // "Other" is empty: every corner has no UV in the selected map.
    let item = a.editor.scene.active().unwrap();
    let none = gpu_scene::checker_uvs_for_tests(item);
    assert!(none.iter().all(|uv| *uv == uv_checker::NO_UV));
    a.run(Command::UvMap(UvMapEdit::Select("Texture".into())));
    let item = a.editor.scene.active().unwrap();
    let some = gpu_scene::checker_uvs_for_tests(item);
    assert!(some.contains(&[0.5, 0.5]));
    assert!(some.iter().all(|uv| *uv != uv_checker::NO_UV));
}

#[test]
fn a_uv_edit_rewrites_only_the_polygons_whose_uvs_changed() {
    let ctx = Context::default();
    let a = uv_app(&ctx);
    let old = a.editor.scene.active().unwrap().mesh.to_owned_value();
    let mut new = old.clone();
    new.uv_maps.get_mut("Texture").unwrap()[2][1] = Some(DVec2::new(0.3, 0.3));
    assert_eq!(gpu_scene::uv_dirty_faces(&old, &new), Some(vec![2]));
    assert_eq!(gpu_scene::uv_dirty_faces(&old, &old.clone()), Some(vec![]));
    // Another selected map, or a new map: a full rebuild.
    let mut other = old.clone();
    other.attributes.current_uv_map = Some("Texture".into());
    assert_eq!(gpu_scene::uv_dirty_faces(&old, &other), None);
    let mut more = old.clone();
    more.uv_maps.insert("B".into(), Vec::new());
    assert_eq!(gpu_scene::uv_dirty_faces(&old, &more), None);
}

// ---------------------------------------------------------------- UVT-200..203

#[test]
fn uvt201_the_uv_layout_has_a_uv_and_a_perspective_view_and_the_uv_tab() {
    use operandi_layout::Pane;
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.toggle_uv_view(); // back to 3D: the Model layout's one view
    let model_views = a.views.views();
    let model_tab = a.category;
    a.execute("layout.restore UV");
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    let panes = a.views.panes();
    assert_eq!(panes.len(), 2, "{panes:?}");
    let uv = panes.iter().find_map(|p| match p {
        Pane::Uv(k) => Some(*k),
        _ => None,
    });
    let persp = panes.iter().find_map(|p| match p {
        Pane::Viewport(k) => Some(*k),
        _ => None,
    });
    let (uv, persp) = (uv.expect("a UV view"), persp.expect("a 3D view"));
    assert_eq!(a.cameras[&persp].view, camera::View::Perspective);
    // 50/50, UV on the left.
    let (ru, rp) = (a.view_rects[&uv], a.view_rects[&persp]);
    assert!(ru.center().x < rp.center().x);
    assert!((ru.width() - rp.width()).abs() < 4., "{ru:?} {rp:?}");
    assert_eq!(a.category, toolbox::Category::Uv);
    assert!(a.upper_frame.contains(panel_registry::PanelKind::UvMaps));
    // The UV tab shows the Map field and the island buttons.
    assert!(a.toolbox_rects.contains_key("Fit UVs"));
    // Back to Model: its views and tab come back; the added list goes.
    a.execute("layout.restore Model");
    assert_eq!(a.views.views(), model_views);
    assert_eq!(a.category, model_tab);
    assert!(!a.upper_frame.contains(panel_registry::PanelKind::UvMaps));
    // And UV again: the same arrangement as before.
    a.execute("layout.restore UV");
    assert!(a.views.views().contains(&uv));
}

#[test]
fn uic15_reset_layout_menu_asks_then_resets_the_uv_layout() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.toggle_uv_view();
    a.execute("layout.restore UV");
    draw(&mut a, &ctx, SIZE, vec![]);
    let before = a.views.views();
    a.split_view();
    assert_eq!(a.views.views().len(), 3);
    let menus = a.menu_model(&ctx);
    let layout = menus.iter().find(|m| m.title == "Layout").unwrap();
    let leaves = layout.leaves();
    let reset = leaves
        .iter()
        .find(|item| item.title == "Reset Layout…")
        .unwrap();
    a.menu_action(&ctx, reset.action);
    assert_eq!(a.layouts.confirm_reset.as_deref(), Some("UV"));
    assert_eq!(a.views.views().len(), 3, "nothing changes before the yes");
    a.answer_layout_reset(true);
    assert_eq!(a.views.panes().len(), 2);
    assert_eq!(a.views.views(), before, "the shipped arrangement again");
}

#[test]
fn uvt200_texture_menu_opens_one_uv_editor_window() {
    use operandi_layout::Pane;
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    let menus = a.menu_model(&ctx);
    assert!(menus.iter().any(|m| m.title == "Texture"));
    a.menu_action(&ctx, app_menu::line_action("Open UV Editor"));
    assert_eq!(a.views.windows.len(), 1);
    let w = &a.views.windows[0];
    // LAY-51: the UV Editor palette opens at 800 x 600.
    assert_eq!(w.size, [800., 600.]);
    let key = a.palettes.views[crate::palettes::UV_EDITOR];
    assert_eq!(a.views.view_pane(key), Some(Pane::Uv(key)));
    assert!(a.is_uv_view(key));
    // Again from the list: the same window, not a second.
    a.open_uv_editor();
    assert_eq!(a.views.windows.len(), 1);
    // The menu again closes it (LAY-50: one toggle).
    a.menu_action(&ctx, app_menu::line_action("Open UV Editor"));
    assert!(a.views.windows.is_empty());
}

#[test]
fn uvt200_double_clicking_a_map_opens_the_uv_editor_on_it() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.run(Command::UvMap(UvMapEdit::New(Some("Iris".into()))));
    a.run(Command::UvMap(UvMapEdit::Select("Texture".into())));
    a.upper_frame.add(panel_registry::PanelKind::UvMaps);
    for _ in 0..3 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    let at = a
        .uv_maps_tree
        .rects
        .iter()
        .find(|(k, _)| *k == 0)
        .unwrap()
        .1
        .center();
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![
            pointer(at, true, Modifiers::NONE),
            pointer(at, false, Modifiers::NONE),
            pointer(at, true, Modifiers::NONE),
            pointer(at, false, Modifiers::NONE),
        ],
    );
    assert!(a.palette_open(crate::palettes::UV_EDITOR), "{}", a.status);
    let mesh = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(
        operandi_core::uv_maps::current_map(mesh).as_deref(),
        Some("Iris")
    );
    assert!(
        a.uv_maps_tree.rename.is_none(),
        "double-click never renames"
    );
}

#[test]
fn uvt131_element_move_drags_the_uv_point_under_the_pointer() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.run(Command::Mode(Mode::Vertex));
    let points = a.editor.scene.active().unwrap().mesh.vertices.clone();
    a.run(Command::Activate(Tool::Tweak));
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(
        a.uv_tool_is(Tool::Tweak),
        "Element Move in a UV view edits UVs"
    );
    let v = view(&a);
    let middle = v.to_screen(v.rect, DVec2::splat(0.5));
    // Hovering outlines the UV point.
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(middle)]);
    assert!(
        matches!(
            view(&a).tweak_hover,
            Some(operandi_core::TweakElement::Vertex(_))
        ),
        "{:?}",
        view(&a).tweak_hover
    );
    drag(
        &mut a,
        &ctx,
        middle,
        middle + vec2(30., 0.),
        Modifiers::NONE,
    );
    let du = 30. / v.zoom;
    let u = uvs(&a);
    let mut moved = 0;
    for row in &u {
        for p in row {
            if (*p - DVec2::new(0.5 + du, 0.5)).length() < 1e-3 {
                moved += 1;
            } else {
                let on_grid = |x: f64| ((x * 2.).round() - x * 2.).abs() < 1e-9;
                assert!(on_grid(p.x) && on_grid(p.y), "only the middle moved: {p}");
            }
        }
    }
    assert_eq!(moved, 4, "the middle UV position's four corners: {u:?}");
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices, points);
    // Right-drag grows Range in UV units (UVT-Q11).
    right_drag(
        &mut a,
        &ctx,
        &[middle, middle + vec2(20., 0.), middle + vec2(40., 0.)],
    );
    let range = a
        .editor
        .session
        .live
        .as_ref()
        .unwrap()
        .params
        .tweak_options
        .range;
    let want = 40. * crate::viewport::tweak_view::TWEAK_RANGE_RATE / v.zoom;
    assert!((range - want).abs() < 1e-9, "{range} vs {want}");
}

#[test]
fn smooth_started_in_a_uv_view_relaxes_uvs() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.run(Command::Activate(Tool::Smooth));
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.uv_tool_is(Tool::Smooth), "{}", a.status);
}

/// Bug (Jason, 2026-11-16): after Element Move moved UVs, hovering picked
/// them where they used to be. A UV polygon moved by one drag is picked at
/// its new place and not at its old one, and a second drag starts from the
/// first one's result.
#[test]
fn uvt131_element_move_picks_uvs_where_they_moved_to() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.run(Command::Activate(Tool::Tweak));
    draw(&mut a, &ctx, SIZE, vec![]);
    a.editor
        .session
        .live
        .as_mut()
        .unwrap()
        .params
        .tweak_options
        .mode = operandi_core::TweakMode::Polygon;
    let v = view(&a);
    let hover = |a: &mut Operandi, p: Pos2| {
        draw(a, &ctx, SIZE, vec![Event::PointerMoved(p)]);
        view(a).tweak_hover
    };
    // The lower-left quad covers UV (0..0.5, 0..0.5); drag it off the tile
    // to the left, where nothing else is.
    let old = v.to_screen(v.rect, DVec2::splat(0.25));
    let new = old + vec2(-(0.75 * v.zoom) as f32, 0.);
    let moved = Some(operandi_core::TweakElement::Polygon(0));
    assert_eq!(hover(&mut a, old), moved);
    assert_eq!(hover(&mut a, new), None, "nothing there yet");
    drag(&mut a, &ctx, old, new, Modifiers::NONE);
    let du = -0.75;
    assert!(
        (uvs(&a)[0][0].x - du).abs() < 1e-3,
        "{:?} {}",
        uvs(&a)[0],
        a.status
    );
    // The moved quad picks at its new place; its old place is empty.
    assert_eq!(
        hover(&mut a, new),
        moved,
        "the moved polygon is under the pointer at its new place"
    );
    // Its old place is now inside the stretched neighbour, not itself.
    assert_ne!(
        hover(&mut a, old),
        moved,
        "its old place no longer picks it"
    );
    // A second drag starts from the first one's result.
    let further = new + vec2(0., -(0.75 * v.zoom) as f32);
    drag(&mut a, &ctx, new, further, Modifiers::NONE);
    let p = uvs(&a)[0][0];
    assert!(
        (p - DVec2::new(du, 0.75)).length() < 1e-3,
        "both drags kept: {p}"
    );
    assert_eq!(hover(&mut a, further), moved);
    assert_ne!(hover(&mut a, new), moved);
    // Undo walks back one drag with the tool on, and picks follow.
    a.run(Command::Undo);
    assert!((uvs(&a)[0][0] - DVec2::new(du, 0.)).length() < 1e-3);
    assert_eq!(hover(&mut a, new), moved);
    assert_ne!(hover(&mut a, further), moved);
    a.run(Command::Redo);
    assert_eq!(hover(&mut a, further), moved);
}

/// A primary click (press and release without moving) at `at`.
fn click(a: &mut Operandi, ctx: &Context, at: Pos2) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(a, ctx, SIZE, vec![pointer(at, true, Modifiers::NONE)]);
    draw(a, ctx, SIZE, vec![pointer(at, false, Modifiers::NONE)]);
    draw(a, ctx, SIZE, vec![]);
}

#[test]
fn uvt_q13_a_click_puts_handles_on_the_uv_element_and_its_arrow_drags_along_u() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.run(Command::Mode(Mode::Vertex));
    a.run(Command::Activate(Tool::Tweak));
    draw(&mut a, &ctx, SIZE, vec![]);
    let v = view(&a);
    let middle = v.to_screen(v.rect, DVec2::splat(0.5));
    click(&mut a, &ctx, middle);
    assert!(
        matches!(
            view(&a).tweak_clicked.map(|g| g.element),
            Some(operandi_core::TweakElement::Vertex(_))
        ),
        "{}",
        a.status
    );
    // Drag the U arrow (right of the point) up and right: only U changes.
    let on_arrow = middle + vec2(50., 0.);
    drag(
        &mut a,
        &ctx,
        on_arrow,
        on_arrow + vec2(30., -40.),
        Modifiers::NONE,
    );
    let du = 30. / v.zoom;
    let moved: Vec<DVec2> = uvs(&a)
        .into_iter()
        .flatten()
        .filter(|p| (p.x - (0.5 + du)).abs() < 1e-3)
        .collect();
    assert_eq!(moved.len(), 4, "{:?}", uvs(&a));
    assert!(moved.iter().all(|p| (p.y - 0.5).abs() < 1e-9), "V stays");
}

#[test]
fn jitter_started_in_a_uv_view_edits_uvs_and_greys_range_z() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    let points = a.editor.scene.active().unwrap().mesh.vertices.clone();
    a.run(Command::Activate(Tool::Jitter));
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.uv_tool_is(Tool::Jitter), "{}", a.status);
    let before = uvs(&a);
    let v = view(&a);
    let c = v.to_screen(v.rect, DVec2::splat(0.5));
    // Drag right widens Range in UV units.
    drag(&mut a, &ctx, c, c + vec2(15., 0.), Modifiers::NONE);
    let r = a.editor.session.live.as_ref().unwrap().params.jitter.range;
    assert!(r.x > 0. && r.z == 0., "{r}");
    assert_ne!(uvs(&a), before);
    assert_eq!(a.editor.scene.active().unwrap().mesh.vertices, points);
}

#[test]
fn soft_drag_in_a_uv_view_moves_what_is_inside_its_circle() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.run(Command::Mode(Mode::Vertex));
    a.run(Command::ActivatePreset("xfrm.softDrag".into()));
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.uv_tool_is(Tool::Move), "{}", a.status);
    let v = view(&a);
    // Press on the tile's lower-left corner and drag: the corner moves,
    // the far corner (outside the 20 px circle) doesn't.
    let corner = v.to_screen(v.rect, DVec2::ZERO);
    drag(
        &mut a,
        &ctx,
        corner,
        corner + vec2(-20., 0.),
        Modifiers::NONE,
    );
    let u = uvs(&a);
    assert!(u[0][0].x < -1e-3, "the corner under the press moved: {u:?}");
    assert!(
        (u[3][2] - DVec2::ONE).length() < 1e-9,
        "the far corner stayed"
    );
}

#[test]
fn uvt19_a_click_in_a_3d_view_projects_from_it() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.toggle_uv_view(); // a 3D view again
    draw(&mut a, &ctx, SIZE, vec![]);
    a.camera.view = camera::View::Top;
    a.run(Command::Activate(Tool::UvViewProject));
    draw(&mut a, &ctx, SIZE, vec![]);
    let before = uvs(&a);
    let centre = a.viewport_rect.center();
    click(&mut a, &ctx, centre);
    let after = uvs(&a);
    assert_ne!(after, before, "{}", a.status);
    // Auto Scale: the grid fills the tile, aspect kept.
    let all: Vec<DVec2> = after.into_iter().flatten().collect();
    let hi = all.iter().fold(DVec2::MIN, |m, &p| m.max(p));
    assert!((hi.x.max(hi.y) - 1.).abs() < 1e-9, "{hi}");
}

#[test]
fn unwrap_tool_drag_in_a_uv_view_changes_iterations() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.run(Command::Activate(Tool::UvUnwrap));
    draw(&mut a, &ctx, SIZE, vec![]);
    let start = a
        .editor
        .session
        .live
        .as_ref()
        .unwrap()
        .params
        .uv_unwrap
        .iterations;
    let v = view(&a);
    let c = v.to_screen(v.rect, DVec2::splat(0.5));
    drag(&mut a, &ctx, c, c + vec2(-40., 0.), Modifiers::NONE);
    let now = a
        .editor
        .session
        .live
        .as_ref()
        .unwrap()
        .params
        .uv_unwrap
        .iterations;
    assert_eq!(
        now as f64,
        start as f64 - 40. / crate::uv_tool_forms::PIXELS_PER_ITERATION
    );
}

#[test]
fn stage3_buttons_run_and_their_dialogs_open() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    a.execute("operandi.uv.rotatePolygons");
    assert_eq!(
        a.editor.history.undo.last().unwrap().label,
        "Rotate Polygon UVs"
    );
    for kind in [
        "texel", "split", "sew", "moveSew", "errors", "setValue", "paste", "toOrigin",
    ] {
        a.execute(&format!("operandi.uv.dialog {kind}"));
        assert!(
            matches!(a.uv_dialog, Some(uv_dialogs::UvDialog::Op(..))),
            "{kind}: {}",
            a.status
        );
        a.uv_dialog = None;
    }
    // The toolbox holds the new buttons.
    a.execute("layout.restore UV");
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    for key in [
        "UV Project",
        "Unwrap Tool",
        "UV Relax",
        "operandi.uv.split",
        "operandi.uv.selectBorder",
    ] {
        assert!(
            a.form_rects.contains_key(key) || a.toolbox_rects.contains_key(key),
            "{key} not drawn"
        );
    }
}

#[test]
fn uvt43_box_transform_moves_and_scales_from_the_opposite_side() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    a.run(Command::Activate(Tool::UvBoxTransform));
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.uv_tool_is(Tool::UvBoxTransform), "{}", a.status);
    let v = view(&a);
    // Polygon 0 covers UV 0..0.5: drag inside it (off the pivot cross at
    // its centre) right by 0.1.
    let mid = v.to_screen(v.rect, DVec2::splat(0.1));
    let dx = (0.1 * v.zoom) as f32;
    drag(&mut a, &ctx, mid, mid + vec2(dx, 0.), Modifiers::NONE);
    let u = uvs(&a);
    assert!(
        (u[0][0] - DVec2::new(0.1, 0.)).length() < 1e-3,
        "{u:?} {} {:?}",
        a.status,
        view(&a).box_now
    );
    // Right-drag the box's right side from U 0.6 to 0.85: the left side
    // (U 0.1) stays.
    let side = v.to_screen(v.rect, DVec2::new(0.6, 0.25));
    let to = v.to_screen(v.rect, DVec2::new(0.85, 0.25));
    let press = |pos: Pos2, pressed: bool| Event::PointerButton {
        pos,
        button: PointerButton::Secondary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(side)]);
    draw(&mut a, &ctx, SIZE, vec![press(side, true)]);
    for k in 1..=6 {
        let p = side + (to - side) * (k as f32 / 6.);
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(p)]);
    }
    draw(&mut a, &ctx, SIZE, vec![press(to, false)]);
    draw(&mut a, &ctx, SIZE, vec![]);
    let u = uvs(&a);
    let xs: Vec<f64> = u[0].iter().map(|p| p.x).collect();
    let (lo, hi) = (
        xs.iter().cloned().fold(f64::MAX, f64::min),
        xs.iter().cloned().fold(f64::MIN, f64::max),
    );
    assert!((lo - 0.1).abs() < 1e-3 && (hi - 0.85).abs() < 2e-3, "{u:?}");
}

#[test]
fn uvt63_split_brush_splits_the_points_it_passes_and_sew_brush_joins_them() {
    let ctx = Context::default();
    let mut a = uv_app(&ctx);
    a.execute("operandi.uv.brush split");
    assert!(a.uv_brush.is_some());
    let v = view(&a);
    let mid = v.to_screen(v.rect, DVec2::splat(0.5));
    drag(&mut a, &ctx, mid, mid + vec2(15., 0.), Modifiers::NONE);
    assert_eq!(
        a.editor.history.undo.last().unwrap().label,
        "Split Brush",
        "{}",
        a.status
    );
    // The middle point's four corners came apart.
    let u = uvs(&a);
    let corners: Vec<DVec2> = u
        .iter()
        .flatten()
        .copied()
        .filter(|p| (*p - DVec2::splat(0.5)).length() < 0.05)
        .collect();
    assert_eq!(corners.len(), 4);
    assert!(
        corners
            .iter()
            .all(|p| (*p - DVec2::splat(0.5)).length() > 1e-4),
        "{corners:?}"
    );
    // Shift-stroke over them again sews.
    drag(&mut a, &ctx, mid, mid + vec2(15., 0.), Modifiers::SHIFT);
    assert_eq!(
        a.editor.history.undo.last().unwrap().label,
        "Sew Brush",
        "{}",
        a.status
    );
    a.execute("operandi.uv.brush split");
    assert!(a.uv_brush.is_none(), "a second click stops it");
}
