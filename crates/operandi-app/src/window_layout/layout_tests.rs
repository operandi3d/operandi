//! Panel layout, display scale, fonts, colors and the toolbox category strip.
use super::tests::{box_app, draw, pointer};
use super::*;

#[test]
fn t11_011_014_simulated_layout_picks_handles_across_scale_matrix() {
    for (w, h) in [
        (1920., 1080.),
        (2560., 1440.),
        (3840., 2160.),
        (5120., 2880.),
    ] {
        for scale in [1., 1.5, 2.] {
            let size = vec2(w / scale, h / scale);
            let ctx = Context::default();
            theme::install(&ctx);
            ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), scale));
            let mut a = box_app();
            let _ = draw(&mut a, &ctx, size, vec![]);
            let _ = draw(&mut a, &ctx, size, vec![]);
            let r = a.viewport_rect;
            assert!(r.width() > 100. && r.height() > 100.);
            if size.x >= 1100. && size.y >= 720. {
                assert!(r.width() >= 499., "{size:?}: {r:?}");
                assert!(r.height() >= 499.);
            }
            let p = a.camera.project(DVec3::new(0.3, 1., 0.3), r).unwrap().0;
            let _ = draw(
                &mut a,
                &ctx,
                size,
                vec![
                    pointer(p, true, Modifiers::NONE),
                    pointer(p, false, Modifiers::NONE),
                ],
            );
            assert_eq!(a.editor.scene.selection.faces, vec![5], "{size:?}/{scale}");
            a.activate(Tool::Move);
            // The free center square and the plane circles (PH-01) have no
            // length.
            for (_, o, end, _) in a.handles(r).into_iter().filter(|h| {
                h.0 != viewport::FREE_HANDLE && !viewport::plane_handles::is_plane_handle(h.0)
            }) {
                assert!((o.distance(end) - 72.).abs() < 1.);
            }
            let (axis, o, end, _) = a.handles(r)[0].clone();
            let normal = (end - o).rot90().normalized();
            assert_eq!(a.handle_hit(o.lerp(end, 0.7) + normal * 4.9, r), Some(axis));
            assert_ne!(a.handle_hit(o.lerp(end, 0.7) + normal * 5.1, r), Some(axis));
        }
    }
}
#[test]
fn t11_016_normal_text_contrast() {
    fn luminance(c: Color32) -> f64 {
        let c = c.to_array();
        let f = |v: u8| {
            let s = f64::from(v) / 255.;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(c[0]) + 0.7152 * f(c[1]) + 0.0722 * f(c[2])
    }
    let contrast = |fg: Color32, bg: Color32| (luminance(fg) + 0.05) / (luminance(bg) + 0.05);
    for bg in [
        theme::control(),
        theme::hover(),
        theme::panel(),
        theme::field(),
    ] {
        let c = contrast(theme::text(), bg);
        assert!(c >= 4.5, "{c}");
    }
    for bg in [theme::chrome::control(), theme::chrome::hover()] {
        let c = contrast(theme::chrome::text(), bg);
        assert!(c >= 4.5, "{c}");
    }
    assert!(contrast(theme::orange(), theme::on_orange()) >= 4.5);
}
#[test]
fn t11_014_coalesced_panel_and_scene_dividers_own_input_and_clamp() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    a.activate(Tool::Box);
    for _ in 0..3 {
        let _ = draw(&mut a, &ctx, vec2(1100., 720.), vec![]);
    }
    let before = a.editor.scene.clone();
    let state = egui::containers::panel::PanelState::load(&ctx, Id::new("tools")).unwrap();
    let start = pos2(state.rect.right(), state.rect.center().y);
    let end = start + vec2(100., 0.);
    let _ = draw(
        &mut a,
        &ctx,
        vec2(1100., 720.),
        vec![
            pointer(start, true, Modifiers::NONE),
            Event::PointerMoved(end),
            pointer(end, false, Modifiers::NONE),
        ],
    );
    assert_eq!(a.editor.scene, before);
    assert_eq!(a.viewport_rect.width(), 500.);
    // 244 + 100; the right panel gives way so the viewport keeps 500.
    assert_eq!(a.viewport_rect.left(), 344.);
    let start = a.scene_divider_rect.center();
    let end = start + vec2(0., 100.);
    let _ = draw(
        &mut a,
        &ctx,
        vec2(1100., 720.),
        vec![
            pointer(start, true, Modifiers::NONE),
            Event::PointerMoved(end),
            pointer(end, false, Modifiers::NONE),
        ],
    );
    assert!(a.scene_fraction > 0.4 && a.scene_fraction <= 0.65);
    assert_eq!(a.editor.scene, before);
    assert!(a.layout_capture.is_none());
}
#[test]
fn divider_drag_keeps_going_when_the_pointer_leaves_the_window() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    a.activate(Tool::Box);
    let size = vec2(1100., 720.);
    for _ in 0..3 {
        let _ = draw(&mut a, &ctx, size, vec![]);
    }
    let left = a.viewport_rect.left();
    let state = egui::containers::panel::PanelState::load(&ctx, Id::new("tools")).unwrap();
    let start = pos2(state.rect.right(), state.rect.center().y);
    let _ = draw(
        &mut a,
        &ctx,
        size,
        vec![
            pointer(start, true, Modifiers::NONE),
            Event::PointerMoved(start + vec2(20., 0.)),
        ],
    );
    assert!(a.layout_capture.is_some(), "the press grabs the divider");
    // The pointer runs off the bottom of the window, still held.
    let outside = pos2(start.x + 60., size.y + 80.);
    let _ = draw(
        &mut a,
        &ctx,
        size,
        vec![Event::PointerGone, Event::PointerMoved(outside)],
    );
    assert!(
        a.layout_capture.is_some(),
        "leaving the window keeps the drag"
    );
    let _ = draw(&mut a, &ctx, size, vec![]);
    assert_eq!(
        a.viewport_rect.left(),
        left + 60.,
        "moves outside still resize"
    );
    let _ = draw(
        &mut a,
        &ctx,
        size,
        vec![pointer(outside, false, Modifiers::NONE)],
    );
    assert!(a.layout_capture.is_none(), "the release outside ends it");
    // Focus loss mid-drag still ends it.
    let start = pos2(start.x + 60., start.y);
    let _ = draw(
        &mut a,
        &ctx,
        size,
        vec![
            Event::PointerMoved(start),
            pointer(start, true, Modifiers::NONE),
            Event::PointerMoved(start + vec2(10., 0.)),
        ],
    );
    assert!(a.layout_capture.is_some());
    let _ = draw(&mut a, &ctx, size, vec![Event::WindowFocused(false)]);
    assert!(a.layout_capture.is_none(), "losing focus ends the drag");
}
#[test]
fn m11_amendment_category_switch_preserves_task_grid_and_focus_at_both_scales() {
    for scale in [1.0f32, 2.] {
        let ctx = Context::default();
        theme::install(&ctx);
        ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), scale));
        let mut a = box_app();
        a.editor.scene.selection.faces = vec![5];
        a.activate(Tool::Bevel);
        a.run(Command::Preview(Parameters {
            inset: 0.25,
            ..Default::default()
        }));
        for _ in 0..2 {
            let _ = draw(&mut a, &ctx, vec2(1100., 720.), vec![]);
        }
        let before = a.editor.clone();
        let transform = a.toolbox_rects["Move"];
        let rotate = a.toolbox_rects["Rotate"];
        assert_eq!(transform.top(), rotate.top());
        assert!(transform.right() < rotate.left());
        let tab = a.toolbox_rects["Category Polygon"];
        assert_eq!(tab.width(), 22.);
        // Jason 2026-11-01: tabs start just below the shared rows (Mesh
        // Cleanup / New Mesh), on the right (Jason, 2026-11-02), spanning only what they switch.
        let shared = a.toolbox_rects["Mesh Cleanup"];
        let create = a.toolbox_rects["Category Create"];
        assert!(create.top() > shared.bottom() && create.top() - shared.bottom() < 8.);
        assert!(a.toolbox_rects["Category Deform"].top() < create.bottom() + 66.);
        let cube = a.toolbox_rects["Cube"];
        assert_eq!(cube.height(), 34.);
        let header = a.toolbox_rects["Primitives"];
        assert_eq!(header.height(), 20.);
        assert!((cube.width() * 4. + 6. - header.width()).abs() < 0.1);
        assert!(cube.right() <= tab.left(), "tabs on the right of the tools");
        let p = tab.center();
        let _ = draw(
            &mut a,
            &ctx,
            vec2(1100., 720.),
            vec![
                pointer(p, true, Modifiers::NONE),
                pointer(p, false, Modifiers::NONE),
            ],
        );
        assert_eq!(a.category, toolbox::Category::Polygon);
        assert!(!a.toolbox_rects.contains_key("Cube"));
        assert!(a.toolbox_rects.contains_key("Bevel"));
        assert_eq!(a.editor.scene, before.scene);
        assert_eq!(
            a.editor.session.live.as_ref().unwrap().params,
            before.session.live.as_ref().unwrap().params
        );
        assert_eq!(a.editor.history.undo.len(), before.history.undo.len());
        // Tool drop does not choose a category for the user.
        a.run(Command::Drop);
        assert_eq!(a.category, toolbox::Category::Polygon);
        let _ = draw(&mut a, &ctx, vec2(1100., 720.), vec![]);
        let p = a.toolbox_rects["Polygon Tools"].center();
        let _ = draw(
            &mut a,
            &ctx,
            vec2(1100., 720.),
            vec![
                pointer(p, true, Modifiers::NONE),
                pointer(p, false, Modifiers::NONE),
            ],
        );
        assert!(!a.toolbox_rects.contains_key("Bevel"));
        assert!(a.toolbox_rects.contains_key("Move"));
    }
}
#[test]
fn m11_amendment_palette_semantics_keep_chrome_and_geometry_distinct() {
    use theme::{chrome as c, viewport as v};
    theme::set_overrides(&Default::default());
    assert_eq!(c::background().to_array(), [0x2E, 0x33, 0x3B, 255]);
    assert_eq!(c::control().to_array(), [0x3D, 0x44, 0x4F, 255]);
    assert_eq!(c::header().to_array(), [0x24, 0x29, 0x31, 255]);
    assert_eq!(c::active().to_array(), [0x7F, 0xB6, 0xE6, 255]);
    assert_eq!(v::background().to_array(), [0x2C, 0x31, 0x38, 255]);
    // Jason, 2026-11-19: component selection follows Accent by default
    // (overridable), so it no longer differs from the chrome's accent.
    assert_eq!(v::selection(), c::active());
    assert_ne!(v::handles(), v::grid_axes());

    // The previous default look is still available as Classic Grey.
    let classic = crate::color_theme::Scheme::BuiltIn(theme::presets::Preset::ClassicGrey);
    theme::set_overrides(&classic.colors(std::path::Path::new("")).unwrap());
    assert_eq!(c::background().to_array(), [0x3E, 0x3E, 0x3E, 255]);
    assert_eq!(c::control().to_array(), [0x4C, 0x4C, 0x4C, 255]);
    assert_eq!(c::header().to_array(), [0x34, 0x34, 0x34, 255]);
    assert_eq!(c::active().to_array(), [0xF8, 0x9A, 0x2B, 255]);
    assert_eq!(v::background().to_array(), [0x38, 0x38, 0x38, 255]);
    assert_eq!(v::selection().to_array(), [255, 212, 112, 255]);
    assert_ne!(c::active(), v::selection());
    assert_ne!(v::handles(), v::grid_axes());
    theme::set_overrides(&Default::default());
}
/// Jason, 2026-11-19: selected polygons showed only an outline, the fill
/// having gone to a colour that matched the faces. In every built-in
/// scheme the fill is opaque, matches the Selection colour, and differs
/// from the plain face colour; a pinned Selection colour is followed.
#[test]
fn selection_fill_is_visible_in_every_built_in_scheme() {
    use theme::viewport as v;
    for preset in theme::presets::Preset::ALL {
        let scheme = crate::color_theme::Scheme::BuiltIn(preset);
        theme::set_overrides(&scheme.colors(std::path::Path::new("")).unwrap());
        let fill = v::selection_fill();
        assert_eq!(fill.a(), 255, "{preset:?}: opaque fill");
        assert_eq!(fill, v::selection(), "{preset:?}: fill matches Selection");
        assert_ne!(fill, v::solid(), "{preset:?}: fill differs from faces");
    }
    theme::set_overrides(&Default::default());
    theme::set_override(
        theme::ColorId::Selection,
        Some(egui::Color32::from_rgb(0xFF, 0xC2, 0x4A)),
    );
    assert_eq!(
        v::selection_fill(),
        egui::Color32::from_rgb(0xFF, 0xC2, 0x4A)
    );
    theme::set_overrides(&Default::default());
}
#[test]
fn m16_panel_resize_scale_and_activation_routes() {
    for tool in [Tool::Sphere, Tool::Cylinder] {
        for (physical, scale) in [
            (vec2(1100., 720.), 1.),
            (vec2(1920., 1080.), 1.),
            (vec2(2560., 1440.), 1.5),
            (vec2(3840., 2160.), 2.),
            (vec2(5120., 2880.), 2.),
        ] {
            let size = physical / scale;
            let mut a = Operandi::empty();
            a.run(Command::NewMesh);
            let ctx = Context::default();
            theme::install(&ctx);
            ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), scale));
            for _ in 0..2 {
                draw(&mut a, &ctx, size, vec![]);
            }
            let button = a.toolbox_rects[tool.label()].center();
            draw(
                &mut a,
                &ctx,
                size,
                vec![
                    pointer(button, true, Modifiers::NONE),
                    pointer(button, false, Modifiers::NONE),
                ],
            );
            assert_eq!(a.editor.session.live.as_ref().unwrap().tool, tool);
            draw(&mut a, &ctx, size, vec![]);
            assert!(a.viewport_rect.width() > 100.);
            let keys = [
                "position0",
                "position1",
                "position2",
                "radius0",
                "radius1",
                "radius2",
                "radial_sides",
                "radial_segments",
            ];
            for key in keys {
                assert!(a.field_rects.contains_key(key), "{key}: {size:?}");
            }
            for pair in keys.windows(2) {
                assert!(a.field_rects[pair[0]].top() < a.field_rects[pair[1]].top());
            }
            a.run(Command::Drop);
            a.execute(tool.label());
            assert_eq!(a.editor.session.live.as_ref().unwrap().tool, tool);
            assert_eq!(a.bindings[tool.label()], "");
        }
    }
}
#[test]
fn par_t15_t16_category_strip_anchor_grid_hit_regions_and_live_state() {
    for scale in [1., 1.25, 1.5, 2.] {
        let ctx = Context::default();
        theme::install(&ctx);
        ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), scale));
        let mut a = box_app();
        a.activate(Tool::Sphere);
        let editor = a.editor.clone();
        for size in [vec2(1100., 720.), vec2(1440., 900.), vec2(1920., 1080.)] {
            // Three frames: the panel width settles a frame later at
            // fractional scales now that the shared rows are laid out first.
            for _ in 0..3 {
                draw(&mut a, &ctx, size, vec![]);
            }
            let tabs = [
                a.toolbox_rects["Category Create"],
                a.toolbox_rects["Category Polygon"],
            ];
            let transform = a.toolbox_rects["Transform"];
            let shared = a.toolbox_rects["Mesh Cleanup"];
            // Tabs start just below the shared rows and sit right of the tools.
            assert!(tabs[0].top() > shared.bottom() && tabs[0].top() - shared.bottom() < 8.);
            assert!(a.toolbox_rects["Cube"].right() <= tabs[0].left());
            // Tabs are separate, with a small gap between them, in the order
            // The Model set (Jason, 2026-11-23): Create, Select, Deform,
            // Duplicate, Edit, Vertex, Edge, Polygon, Topology, Curve.
            let order = [
                "Create",
                "Select",
                "Deform",
                "Duplicate",
                "Edit",
                "Vertex",
                "Edge",
                "Polygon",
                "Topology",
                "Curve",
            ];
            for pair in order.windows(2) {
                assert_eq!(
                    a.toolbox_rects[&format!("Category {}", pair[0])].bottom() + 2.,
                    a.toolbox_rects[&format!("Category {}", pair[1])].top(),
                    "{pair:?} at {scale} {size:?}"
                );
            }
            // The shared top row spans the panel, wider than the tab-side cells.
            assert_eq!(transform.height(), 34.);
            assert!(transform.width() > a.toolbox_rects["Cube"].width());
            assert!((a.toolbox_rects["Scale"].right() - tabs[0].right()).abs() <= 1.);
            // The strip stops at the bottom of the tool drawer.
            let drawer_bottom = a.tool_divider_rect.center().y - tool_split::DIVIDER_HEIGHT / 2.;
            assert!(
                a.toolbox_rects["Category Polygon"].bottom() <= drawer_bottom + 0.5,
                "tabs past the drawer at {scale} {size:?}"
            );
            // With eleven tabs (TB-01) and a tool open, even the largest
            // window may pack the tabs to their labels; they still fit.
            let compact = a.toolbox_rects["Category Create surface"].height() < 46.;
            for label in ["Create", "Polygon"] {
                let hit = a.toolbox_rects[&format!("Category {label}")];
                let surface = a.toolbox_rects[&format!("Category {label} surface")];
                let ink = a.toolbox_rects[&format!("Category {label} ink")];
                // The active tab reaches the content edge; others are set back.
                let active = a.category
                    == if label == "Create" {
                        toolbox::Category::Create
                    } else {
                        toolbox::Category::Polygon
                    };
                assert_eq!(surface.width(), if active { 22. } else { 19. });
                assert_eq!(hit.width(), 22.);
                // Tabs are at least 46 long while the tool drawer has room
                // for them; a short drawer (a tool open in a small window)
                // packs them to their labels. Either way they fit beside it.
                let floor = if compact { ink.height() } else { 46. };
                assert!(
                    (floor..60.).contains(&surface.height()),
                    "{label} at {scale} {size:?}: {surface:?}"
                );
                assert!(
                    surface.shrink(1.).contains_rect(ink),
                    "{label} clips at {scale}: {ink:?} in {surface:?}"
                );
                assert!(ink.center().distance(surface.center()) <= 1. / scale);
            }
            let viewport = a.viewport_rect;
            // Hit the outer two-unit gutter: the practical target exceeds the art.
            let point = pos2(tabs[1].left() + 0.5, tabs[1].center().y);
            draw(
                &mut a,
                &ctx,
                size,
                vec![
                    pointer(point, true, Modifiers::NONE),
                    pointer(point, false, Modifiers::NONE),
                ],
            );
            assert_eq!(a.category, toolbox::Category::Polygon);
            assert_eq!(a.editor.scene, editor.scene);
            assert_eq!(
                a.editor.session.live.as_ref().unwrap().params,
                editor.session.live.as_ref().unwrap().params
            );
            assert_eq!(a.editor.history.undo.len(), editor.history.undo.len());
            assert_eq!(a.viewport_rect, viewport, "scale {scale} size {size:?}");
            assert_eq!(a.toolbox_rects["Category Create"], tabs[0]);
            let header = a.toolbox_rects["Polygon Tools"].center();
            draw(
                &mut a,
                &ctx,
                size,
                vec![
                    pointer(header, true, Modifiers::NONE),
                    pointer(header, false, Modifiers::NONE),
                ],
            );
            assert_eq!(a.toolbox_rects["Category Create"], tabs[0]);
            draw(
                &mut a,
                &ctx,
                size,
                vec![
                    Event::PointerMoved(pos2(120., 400.)),
                    Event::MouseWheel {
                        unit: MouseWheelUnit::Point,
                        delta: vec2(0., -400.),
                        modifiers: Modifiers::NONE,
                    },
                ],
            );
            assert_eq!(a.toolbox_rects["Category Create"], tabs[0]);
            let point = tabs[0].center();
            draw(
                &mut a,
                &ctx,
                size,
                vec![
                    pointer(point, true, Modifiers::NONE),
                    pointer(point, false, Modifiers::NONE),
                ],
            );
            assert_eq!(a.category, toolbox::Category::Create);
            // Resize the actual sidebar, including a fractional logical boundary.
            let state = egui::containers::panel::PanelState::load(&ctx, Id::new("tools")).unwrap();
            let from = pos2(state.rect.right(), state.rect.center().y);
            draw(
                &mut a,
                &ctx,
                size,
                vec![
                    pointer(from, true, Modifiers::NONE),
                    Event::PointerMoved(from + vec2(-15.3, 0.)),
                    pointer(from + vec2(-15.3, 0.), false, Modifiers::NONE),
                ],
            );
            draw(&mut a, &ctx, size, vec![]);
            assert!(
                a.toolbox_rects["Category Create"].top() > a.toolbox_rects["Mesh Cleanup"].bottom()
            );
            assert_eq!(
                a.toolbox_rects["Transform"].height(),
                a.toolbox_rects["Cube"].height()
            );
            assert!(a.toolbox_rects["Cube"].right() <= a.toolbox_rects["Category Create"].left());
            // Restore the default width without modifying document/tool state.
            ctx.data_mut(|d| {
                d.insert_persisted(
                    Id::new("tools"),
                    egui::containers::panel::PanelState {
                        rect: Rect::from_min_size(state.rect.min, vec2(244., state.rect.height())),
                    },
                )
            });
            assert_eq!(a.editor.scene, editor.scene);
        }
    }
    // Separate right-side tabs: every tab rounds its two outer (right) corners.
    for (index, count) in [(0, 2), (1, 2), (0, 1), (1, 3)] {
        let c = toolbox::category_corners(index, count);
        assert!(c.ne > 0 && c.se > 0 && c.nw == 0 && c.sw == 0);
    }
}
#[test]
fn par_t17_rotated_text_native_scale_ink_center_and_physical_origin() {
    for scale in [1., 1.25, 1.5, 2.] {
        let ctx = Context::default();
        theme::install(&ctx);
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(500., 500.))),
            ..Default::default()
        };
        ctx.set_pixels_per_point(scale);
        let _ = ctx.run(raw, |ctx| {
            CentralPanel::default().show(ctx, |ui| {
                for label in ["Create", "Polygon"] {
                    for color in [theme::chrome::active_text(), theme::chrome::text()] {
                        let rect = Rect::from_min_size(pos2(23.37, 81.23), vec2(18., 52.));
                        let text = toolbox::category_text(ui, label, rect, color);
                        assert_eq!(text.galley.pixels_per_point, ctx.pixels_per_point());
                        assert!(
                            text.galley
                                .job
                                .sections
                                .iter()
                                .all(|s| s.format.font_id.size == typography::STANDARD_SIZE)
                        );
                        assert_eq!(text.angle, std::f32::consts::FRAC_PI_2);
                        assert!((text.pos.x * scale - (text.pos.x * scale).round()).abs() < 1e-4);
                        assert!((text.pos.y * scale - (text.pos.y * scale).round()).abs() < 1e-4);
                        assert!(rect.shrink(1.).contains_rect(text.visual_bounding_rect()));
                        assert!(
                            text.visual_bounding_rect().center().distance(rect.center())
                                <= 1. / scale
                        );
                    }
                }
            });
        });
    }
}
#[test]
#[cfg(target_os = "macos")]
fn font_t01_t02_verified_installed_face_and_shared_metrics() {
    let ctx = Context::default();
    theme::install(&ctx);
    let font = typography::report(&ctx);
    assert_eq!((&*font.family, &*font.face), ("Lucida Grande", "Regular"));
    assert_eq!(font.postscript_name, "LucidaGrande");
    assert!(font.fallback.is_none());
    assert_eq!(font.standard_size, 10.);
    let _ = ctx.run(RawInput::default(), |ctx| {
        let metrics = typography::diagnostics(ctx);
        println!("FONT-T02 {metrics}");
        for (row, expected) in metrics["advances"]
            .as_array()
            .unwrap()
            .iter()
            .zip([30.5625, 38.21875, 47.09375])
        {
            // Layout advances use 1/64-unit precision; permit one quantum.
            assert!((row["layout_width"].as_f64().unwrap() - expected).abs() <= 1. / 64.);
            assert_eq!(row["height"], 12.);
        }
        assert_eq!(ctx.style().text_styles[&TextStyle::Body].size, 10.);
        assert_eq!(ctx.style().text_styles[&TextStyle::Button].size, 10.);
        assert_eq!(ctx.style().text_styles[&TextStyle::Monospace].size, 11.);
        assert_eq!(ctx.style().text_styles[&TextStyle::Heading].size, 11.);
    });
}
#[test]
fn font_t05_unavailable_face_is_explicit_and_preserves_layout_and_task() {
    for scale in [1., 1.25, 1.5, 2.] {
        let ctx = Context::default();
        theme::install(&ctx);
        typography::install_from(
            &ctx,
            &std::env::temp_dir().join("operandi-nonexistent-font-fixture.ttc"),
            "Lucida Grande",
        );
        assert!(typography::report(&ctx).fallback.is_some());
        assert_eq!(typography::report(&ctx).postscript_name, "Ubuntu-Light");
        ctx.data_mut(|d| d.insert_temp(Id::new("simulation-scale"), scale));
        let mut a = box_app();
        a.activate(Tool::Sphere);
        let before = a.editor.clone();
        for category in [toolbox::Category::Create, toolbox::Category::Polygon] {
            a.category = category;
            draw(&mut a, &ctx, vec2(1100., 720.), vec![]);
            draw(&mut a, &ctx, vec2(1100., 720.), vec![]);
            // Shared top rows span the panel, wider than the tab-side cells.
            assert_eq!(a.toolbox_rects["Transform"].height(), 34.);
            assert!(a.toolbox_rects["Transform"].width() > 51.);
            for label in ["Create", "Polygon"] {
                let surface = a.toolbox_rects[&format!("Category {label} surface")];
                let ink = a.toolbox_rects[&format!("Category {label} ink")];
                assert!(surface.shrink(1.).contains_rect(ink));
            }
            assert_eq!(a.editor.scene, before.scene);
            assert_eq!(a.editor.history.undo.len(), before.history.undo.len());
            assert_eq!(
                a.editor.session.live.as_ref().unwrap().params,
                before.session.live.as_ref().unwrap().params
            );
        }
    }
}
#[test]
fn font_t03_horizontal_chrome_ink_fits_fixed_controls_at_all_scales() {
    for scale in [1., 1.25, 1.5, 2.] {
        let ctx = Context::default();
        theme::install(&ctx);
        ctx.set_pixels_per_point(scale);
        let _ = ctx.run(RawInput::default(), |ctx| {
            CentralPanel::default().show(ctx, |ui| {
                for label in [
                    "Primitives",
                    "Polygon Tools",
                    "Tool Properties",
                    "Properties",
                    "Vertex",
                    "Edge",
                    "Polygon",
                    "Position Y",
                    "Radius Y",
                    "Apply",
                    "Cancel",
                ] {
                    let galley = ui.painter().layout_no_wrap(
                        label.into(),
                        typography::standard_font(),
                        theme::chrome::text(),
                    );
                    let rect = Rect::from_min_size(pos2(10., 10.), vec2(210., 20.));
                    let shape = egui::epaint::TextShape::new(
                        rect.center() - galley.size() / 2.,
                        galley,
                        theme::chrome::text(),
                    );
                    assert!(
                        rect.contains_rect(shape.visual_bounding_rect()),
                        "{label} at {scale}"
                    );
                }
            });
        });
    }
}
#[test]
#[cfg(target_os = "macos")]
fn font_t06_family_then_size_are_independent_layout_variables() {
    for (family, path) in [
        ("System Font", "/System/Library/Fonts/SFNS.ttf"),
        ("Lucida Grande", typography::MACOS_FONT),
    ] {
        let ctx = Context::default();
        theme::install(&ctx);
        typography::install_from(&ctx, std::path::Path::new(path), family);
        assert!(typography::report(&ctx).fallback.is_none());
        let _ = ctx.run(RawInput::default(), |ctx| {
            for size in [10., 11.] {
                let widths: Vec<_> = ["Create", "Polygon", "Primitives"]
                    .into_iter()
                    .map(|text| {
                        ctx.fonts_mut(|f| {
                            f.layout_no_wrap(
                                text.into(),
                                FontId::proportional(size),
                                theme::chrome::text(),
                            )
                            .size()
                            .x
                        })
                    })
                    .collect();
                println!("FONT-T06 {family} size {size}: {widths:?}");
                if family == "System Font" && size == 11. {
                    assert_eq!(widths, vec![30.625, 36.78125, 44.125]);
                }
            }
        });
    }
}

// Jason, 2026-11-01: larger toolbox icons made the Polygon tab taller than a
// 1440x900 window; the toolbox body scrolls so every row stays reachable.
#[test]
fn toolbox_scrolls_to_its_last_row() {
    let mut a = box_app();
    a.category = toolbox::Category::Polygon;
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    for _ in 0..3 {
        draw(&mut a, &ctx, size, vec![]);
    }
    let last = |a: &Operandi| {
        a.toolbox_rects
            .iter()
            .filter(|(k, _)| !k.starts_with("Category"))
            .map(|(_, r)| *r)
            .max_by(|x, y| x.bottom().total_cmp(&y.bottom()))
            .unwrap()
    };
    assert!(
        last(&a).bottom() > size.y,
        "the Polygon tab overflows at 900"
    );
    let over = a.toolbox_rects["Transform"].center() + vec2(0., 300.);
    for _ in 0..20 {
        draw(
            &mut a,
            &ctx,
            size,
            vec![
                Event::PointerMoved(over),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: vec2(0., -200.),
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
    assert!(
        last(&a).bottom() <= size.y,
        "scrolled into view: {:?}",
        last(&a)
    );
}

fn panel_rect(ctx: &Context, id: &str) -> Rect {
    egui::containers::panel::PanelState::load(ctx, Id::new(id))
        .unwrap()
        .rect
}
/// A point on the inner edge of docked side panel `id`.
fn panel_edge(ctx: &Context, id: &str) -> Pos2 {
    let rect = panel_rect(ctx, id);
    let x = if id == "tools" {
        rect.right()
    } else {
        rect.left()
    };
    pos2(x, rect.center().y)
}
/// Drags side panel `id`'s inner edge `dx` points sideways, grabbing it
/// `offset` points off the edge, then lets the panels redraw.
fn drag_panel_edge(a: &mut Operandi, ctx: &Context, size: Vec2, id: &str, offset: f32, dx: f32) {
    let start = panel_edge(ctx, id) + vec2(offset, 0.);
    let end = start + vec2(dx, 0.);
    let _ = draw(
        a,
        ctx,
        size,
        vec![
            pointer(start, true, Modifiers::NONE),
            Event::PointerMoved(end),
            pointer(end, false, Modifiers::NONE),
        ],
    );
    let _ = draw(a, ctx, size, vec![]);
}
fn settled_app(ctx: &Context, size: Vec2) -> Operandi {
    theme::install(ctx);
    let mut a = box_app();
    for _ in 0..3 {
        let _ = draw(&mut a, ctx, size, vec![]);
    }
    a
}
#[test]
fn right_panel_drags_wide_and_narrow_within_limits() {
    let size = vec2(1920., 1080.);
    let ctx = Context::default();
    let mut a = settled_app(&ctx, size);
    let before = a.editor.scene.clone();
    assert_eq!(panel_rect(&ctx, "scene").width(), layout::RIGHT_DEFAULT);
    // Grabbed a few points inside the viewport: the grab is wider than the line.
    drag_panel_edge(&mut a, &ctx, size, "scene", 3., -300.);
    assert_eq!(panel_rect(&ctx, "scene").width(), 600.);
    assert_eq!(a.viewport_rect.right(), 1920. - 600.);
    drag_panel_edge(&mut a, &ctx, size, "scene", 0., -500.);
    assert_eq!(panel_rect(&ctx, "scene").width(), layout::RIGHT_MAX);
    drag_panel_edge(&mut a, &ctx, size, "scene", -3., 2000.);
    assert_eq!(panel_rect(&ctx, "scene").width(), layout::RIGHT_MIN);
    assert_eq!(a.editor.scene, before, "panel drags never reach the scene");
    assert!(a.layout_capture.is_none());
}
#[test]
fn panel_edges_show_resize_cursor_on_hover_unless_locked() {
    let size = vec2(1920., 1080.);
    let ctx = Context::default();
    let mut a = settled_app(&ctx, size);
    let hover = |a: &mut Operandi, pos: Pos2| {
        draw(a, &ctx, size, vec![Event::PointerMoved(pos)])
            .platform_output
            .cursor_icon
    };
    for id in ["tools", "scene"] {
        let edge = panel_edge(&ctx, id);
        for dx in [-3., 0., 3.] {
            assert_eq!(
                hover(&mut a, edge + vec2(dx, 0.)),
                CursorIcon::ResizeHorizontal,
                "{id} {dx}"
            );
        }
        let off = if id == "tools" { 12. } else { -12. };
        assert_ne!(
            hover(&mut a, edge + vec2(off, 0.)),
            CursorIcon::ResizeHorizontal,
            "{id}: into the viewport, away from the edge"
        );
    }
    // View > Layout > Lock Divider Dragging: no cursor, no drag.
    a.dots.locks.dividers = true;
    let edge = panel_edge(&ctx, "scene");
    assert_ne!(hover(&mut a, edge), CursorIcon::ResizeHorizontal);
    drag_panel_edge(&mut a, &ctx, size, "scene", 0., -200.);
    assert_eq!(panel_rect(&ctx, "scene").width(), layout::RIGHT_DEFAULT);
}
#[test]
fn toolbox_keeps_tab_strip_and_grid_at_min_and_max_width() {
    let size = vec2(1920., 1080.);
    let ctx = Context::default();
    let mut a = settled_app(&ctx, size);
    let mut cubes = Vec::new();
    for (dx, width) in [(1000., layout::LEFT_MAX), (-1000., layout::LEFT_MIN)] {
        drag_panel_edge(&mut a, &ctx, size, "tools", 0., dx);
        let panel = panel_rect(&ctx, "tools");
        assert_eq!(panel.width(), width);
        assert_eq!(a.viewport_rect.left(), panel.right());
        let tab = a.toolbox_rects["Category Polygon"];
        assert_eq!(tab.width(), 22., "the tab strip keeps its width");
        assert!(tab.right() <= panel.right());
        let cube = a.toolbox_rects["Cube"];
        assert!(cube.right() <= tab.left(), "tools stay left of the tabs");
        for name in [
            "Category Create",
            "Category Polygon",
            "Move",
            "Scale",
            "Mesh Cleanup",
            "Primitives",
            "Cube",
        ] {
            let rect = a.toolbox_rects[name];
            assert!(
                rect.left() >= panel.left() - 0.5 && rect.right() <= panel.right() + 0.5,
                "{width}: {name} {rect:?} outside {panel:?}"
            );
        }
        // The grid keeps four columns that share the width.
        let move_tool = a.toolbox_rects["Move"];
        let rotate = a.toolbox_rects["Rotate"];
        assert_eq!(move_tool.top(), rotate.top());
        cubes.push(cube.width());
    }
    assert!(cubes[0] > cubes[1] && cubes[1] > 24., "{cubes:?}");
}
#[test]
fn panel_widths_survive_a_restart_and_clamp_when_out_of_range() {
    let size = vec2(1920., 1080.);
    let ctx = Context::default();
    let mut a = settled_app(&ctx, size);
    drag_panel_edge(&mut a, &ctx, size, "scene", 0., -300.);
    drag_panel_edge(&mut a, &ctx, size, "tools", 0., 100.);
    let saved = a.layout_json(&ctx);
    let dir = std::env::temp_dir().join(format!("operandi-layout-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (text, left, right) in [
        (saved, 344., 600.),
        (
            r#"{"version":1,"split":0.5,"left":50,"right":900}"#.to_string(),
            layout::LEFT_MIN,
            layout::RIGHT_MAX,
        ),
    ] {
        let path = dir.join("layout.json");
        std::fs::write(&path, text).unwrap();
        let ctx = Context::default();
        theme::install(&ctx);
        let mut b = box_app();
        b.load_layout_from(&ctx, &path);
        for _ in 0..3 {
            let _ = draw(&mut b, &ctx, size, vec![]);
        }
        assert_eq!(panel_rect(&ctx, "tools").width(), left);
        assert_eq!(panel_rect(&ctx, "scene").width(), right);
    }
    let _ = std::fs::remove_dir_all(dir);
}
/// The tool list / Tool Properties divider: take-over on activation, the
/// same split on every tab, drag clamps, and the split saved with the layout.
#[test]
fn tool_split_takes_over_drags_matches_every_tab_and_round_trips() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    let size = vec2(1100., 720.);
    a.run(Command::Drop);
    for _ in 0..3 {
        let _ = draw(&mut a, &ctx, size, vec![]);
    }
    // Take-over: below half grows to half.
    a.tool_split = 0.2;
    a.activate(Tool::Move);
    let _ = draw(&mut a, &ctx, size, vec![]);
    assert_eq!(a.tool_split, tool_split::TAKE_OVER_FRACTION);
    // More than half stays when switching tools.
    a.tool_split = 0.7;
    a.activate(Tool::Rotate);
    let _ = draw(&mut a, &ctx, size, vec![]);
    assert_eq!(a.tool_split, 0.7);
    // Dropping the tool leaves the divider where it is.
    a.run(Command::Drop);
    let _ = draw(&mut a, &ctx, size, vec![]);
    assert_eq!(a.tool_split, 0.7);
    // The same divider place on every tab, Select included.
    let mut tops = Vec::new();
    for category in [
        toolbox::Category::Create,
        toolbox::Category::Select,
        toolbox::Category::Deform,
        toolbox::Category::Edit,
        toolbox::Category::Vertex,
        toolbox::Category::Edge,
        toolbox::Category::Polygon,
    ] {
        a.category = category;
        let _ = draw(&mut a, &ctx, size, vec![]);
        let _ = draw(&mut a, &ctx, size, vec![]);
        tops.push(a.toolbox_rects["Tool Properties"].top());
        assert!(a.tool_divider_rect.center().y < tops[tops.len() - 1]);
    }
    assert!(tops.iter().all(|t| (t - tops[0]).abs() < 0.5), "{tops:?}");
    // Dragging the divider up gives Tool Properties more room.
    // Start low and drag only 40 points, so the drag stays clear of the
    // drawer's minimum (the length the tab strip needs). That length
    // depends on the UI font: Linux falls back to egui's own font, whose
    // taller tabs leave only about 56 points of room at this window size.
    const DRAG: f32 = 40.;
    let low = 0.15;
    a.tool_split = low;
    let _ = draw(&mut a, &ctx, size, vec![]);
    let header = a.toolbox_rects["Tool Properties"].top();
    let start = a.tool_divider_rect.center();
    let end = start - vec2(0., DRAG);
    let _ = draw(
        &mut a,
        &ctx,
        size,
        vec![
            pointer(start, true, Modifiers::NONE),
            Event::PointerMoved(end),
            pointer(end, false, Modifiers::NONE),
        ],
    );
    let _ = draw(&mut a, &ctx, size, vec![]);
    assert!(a.tool_split > low);
    let top = a.toolbox_rects["Tool Properties"].top();
    assert!(
        (top - (header - DRAG)).abs() < 1.,
        "header {header} moved to {top}, not {DRAG} points up"
    );
    assert!(a.layout_capture.is_none());
    // Saved with the layout and restored.
    let saved = a.tool_split;
    let path =
        std::env::temp_dir().join(format!("operandi-tool-split-{}.json", std::process::id()));
    std::fs::write(&path, a.layout_json(&ctx)).unwrap();
    let mut b = box_app();
    assert_eq!(b.tool_split, tool_split::DEFAULT_FRACTION);
    b.load_layout_from(&ctx, &path);
    let _ = std::fs::remove_file(&path);
    assert_eq!(b.tool_split, saved);
}

/// Contributor report, 2026-11-19: with macOS in Light appearance the top
/// bar, mode bar and Properties drew egui's unthemed light style (faint
/// labels). Our scheme applies whatever the OS appearance.
#[test]
fn os_light_appearance_still_draws_our_colour_scheme() {
    let ctx = Context::default();
    theme::set_overrides(&Default::default());
    for _ in 0..2 {
        let input = egui::RawInput {
            system_theme: Some(egui::Theme::Light),
            ..Default::default()
        };
        let _ = ctx.run(input, theme::refresh);
    }
    assert_eq!(ctx.theme(), egui::Theme::Dark);
    let visuals = ctx.style().visuals.clone();
    assert!(visuals.dark_mode);
    assert_eq!(visuals.panel_fill, theme::panel());
    assert_eq!(visuals.override_text_color, Some(theme::text()));
}
