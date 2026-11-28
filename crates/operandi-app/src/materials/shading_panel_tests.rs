//! Shading tab (ST-1): rows, selection, eye, opacity drag, drag to parent,
//! Add Layer, delete, undo and save.
use super::tests::{box_app, draw, key, pointer};
use super::*;
use operandi_core::shader_tree::{NewLayer, NodeKind};

const SIZE: Vec2 = vec2(1440., 900.);

fn shading_app() -> (Operandi, Context) {
    let mut a = box_app();
    let ctx = Context::default();
    theme::install(&ctx);
    a.upper_frame.show_page(1);
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    (a, ctx)
}
fn click(a: &mut Operandi, ctx: &Context, at: Pos2) {
    draw(
        a,
        ctx,
        SIZE,
        vec![
            Event::PointerMoved(at),
            pointer(at, true, Modifiers::NONE),
            pointer(at, false, Modifiers::NONE),
        ],
    );
    draw(a, ctx, SIZE, vec![]);
}
/// Press at `from`, move in steps to `to`, release.
fn drag(a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(a, ctx, SIZE, vec![pointer(from, true, Modifiers::NONE)]);
    for k in 1..=6 {
        let p = from + (to - from) * (k as f32 / 6.);
        draw(a, ctx, SIZE, vec![Event::PointerMoved(p)]);
    }
    draw(a, ctx, SIZE, vec![pointer(to, false, Modifiers::NONE)]);
    draw(a, ctx, SIZE, vec![]);
}
fn row_order(a: &Operandi) -> Vec<String> {
    let mut rows: Vec<_> = a.shading.row_rects.iter().collect();
    rows.sort_by(|x, y| x.1[0].top().total_cmp(&y.1[0].top()));
    rows.into_iter()
        .map(|(id, _)| a.editor.scene.shader_tree.get(*id).unwrap().name.clone())
        .collect()
}

#[test]
fn st1_shading_tab_opens_with_a_click_and_keeps_its_menu() {
    let mut a = box_app();
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    assert!(!a.shading_tab());
    // The second tab sits 86 px in from the Items panel's left edge.
    let tab = a.scene_rect.min + vec2(86. + 43., 11.);
    // Before the fix the header strip's right-click area sat on top of the
    // tabs and swallowed this click.
    click(&mut a, &ctx, tab);
    assert!(a.shading_tab(), "the Shading tab didn't take the click");
    assert!(!a.shading.row_rects.is_empty());
    let scenes = a.scene_rect.min + vec2(43., 11.);
    click(&mut a, &ctx, scenes);
    assert!(!a.shading_tab());
}

#[test]
fn st1_default_rows_select_and_delete_with_undo() {
    let (mut a, ctx) = shading_app();
    assert_eq!(
        row_order(&a),
        [
            "Alpha Output",
            "Final Color Output",
            "Base Shader",
            "Base Material"
        ]
    );
    let before = a.editor.scene.clone();
    let name = a.shading.row_rects[&3][1].center();
    click(&mut a, &ctx, name);
    assert_eq!(a.shading.selected, vec![3]);
    draw(&mut a, &ctx, SIZE, vec![key(Key::Backspace)]);
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.editor.scene.shader_tree.get(3).is_none());
    // Deleting a layer never deletes the mesh.
    assert_eq!(a.editor.scene.items.len(), before.items.len());
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, before);
}

#[test]
fn st1_eye_click_and_opacity_drag_are_one_step_each() {
    let (mut a, ctx) = shading_app();
    let steps = a.editor.history.undo.len();
    let eye = a.shading.row_rects[&4][0].center();
    click(&mut a, &ctx, eye);
    assert!(!a.editor.scene.shader_tree.get(4).unwrap().layer.enable);
    let cell = a.shading.row_rects[&3][3];
    drag(&mut a, &ctx, cell.center(), cell.center() - vec2(60., 0.));
    let opacity = a.editor.scene.shader_tree.get(3).unwrap().layer.opacity;
    assert!(opacity < 0.9, "opacity {opacity}");
    assert_eq!(a.editor.history.undo.len(), steps + 2);
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.shader_tree.get(3).unwrap().layer.opacity, 1.);
}

#[test]
fn st1_add_layer_drag_into_group_and_save() {
    let (mut a, ctx) = shading_app();
    a.shading_add(NewLayer::Group);
    let group = a.shading.selected[0];
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(
        row_order(&a),
        [
            "Alpha Output",
            "Final Color Output",
            "Base Shader",
            "Group",
            "Base Material"
        ]
    );
    // Drag Base Material onto the middle of the group row: it goes inside.
    let from = a.shading.row_rects[&4][1].center();
    let to = a.shading.row_rects[&group][1].center();
    drag(&mut a, &ctx, from, to);
    assert_eq!(
        a.editor.scene.shader_tree.get(4).unwrap().parent,
        Some(group)
    );
    // Add Layer with the group selected: inside it, at its top.
    a.shading.selected = vec![group];
    a.shading_add(NewLayer::Constant);
    let constant = a.shading.selected[0];
    let tree = &a.editor.scene.shader_tree;
    assert!(matches!(
        tree.get(constant).unwrap().kind,
        NodeKind::Texture(_)
    ));
    assert_eq!(
        tree.children(Some(group)).map(|n| n.id).collect::<Vec<_>>(),
        vec![constant, 4]
    );
    let p = std::env::temp_dir().join("operandi-st1-shader-tree.opscene");
    assert!(a.save(p.clone()));
    assert_eq!(persistence::load(&p).unwrap(), a.editor.scene);
    let _ = std::fs::remove_file(p);
}

/// Right edges of the given text labels in one drawn frame.
fn label_left_edges(out: &egui::FullOutput, labels: &[&str]) -> Vec<f32> {
    labels
        .iter()
        .map(|label| {
            out.shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.job.text == *label => Some(t.pos.x),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("no {label} label"))
        })
        .collect()
}

fn label_right_edges(out: &egui::FullOutput, labels: &[&str]) -> Vec<f32> {
    labels
        .iter()
        .map(|label| {
            out.shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.job.text == *label => {
                        Some(t.pos.x + t.galley.size().x)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("no {label} label"))
        })
        .collect()
}

#[test]
fn properties_labels_right_align_and_side_tabs_follow_the_selection() {
    let (mut a, ctx) = shading_app();
    // A shader tree layer: one tab named for its kind.
    a.shading.selected = vec![1];
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(a.property_tabs(), vec!["Render Output"]);
    a.shading.selected = vec![3];
    draw(&mut a, &ctx, SIZE, vec![]);
    let out = draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(a.property_tabs(), vec!["Shader"]);
    // Labels sit flush right against one control column.
    let edges = label_right_edges(&out, &["Blend Mode", "Opacity", "Indirect Illum Type"]);
    for e in &edges {
        assert!((e - edges[0]).abs() < 1.5, "label edges {edges:?}");
    }
    // Checkbox names sit after their box, in the control column, all
    // starting at the same place.
    let names = label_left_edges(&out, &["Invert", "GL Display", "Compute Shading"]);
    for n in &names {
        assert!((n - names[0]).abs() < 1.5, "checkbox names {names:?}");
        assert!(
            *n > edges[0] + 12.,
            "checkbox name {n} not past the label column {}",
            edges[0]
        );
    }
    // The Opacity value reads from the field column's left (UIC-90), and
    // the field's arrows end just short of the side tabs (UIC-120).
    let tabs_left = a.properties_tab_rects[0].left();
    let (opacity_y, opacity_left) = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "Opacity" => {
                let r = t.visual_bounding_rect();
                Some((r.center().y, r.left()))
            }
            _ => None,
        })
        .expect("Opacity label");
    // Only text in the Opacity row of this panel: a toolbox label on the
    // same line further left isn't the row's value.
    let value_right = out
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t)
                if t.galley.job.text != "Opacity"
                    && t.visual_bounding_rect().left() > opacity_left
                    && (t.visual_bounding_rect().center().y - opacity_y).abs() < 3. =>
            {
                Some(t.visual_bounding_rect().left())
            }
            _ => None,
        })
        .fold(f32::MAX, f32::min);
    assert!(
        value_right > edges[0] + 12. && value_right < names[0],
        "opacity value starts at {value_right}, labels end at {}, tabs at {tabs_left}",
        edges[0]
    );
    // A mesh on the Scenes tab: Mesh and Surface; Surface holds smoothing.
    a.upper_frame.show_page(0);
    draw(&mut a, &ctx, SIZE, vec![]);
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(
        a.property_tabs(),
        vec![
            "Mesh",
            "Surface",
            "Display",
            "Assembly",
            "User Channels",
            "Tags"
        ]
    );
    assert_eq!(a.properties_tab_rects.len(), 6);
    let surface = a.properties_tab_rects[1].center();
    click(&mut a, &ctx, surface);
    assert_eq!(a.properties_tab, "Surface");
    let out = draw(&mut a, &ctx, SIZE, vec![]);
    assert!(out.shapes.iter().any(|s| matches!(&s.shape,
        egui::Shape::Text(t) if t.galley.job.text == "Smoothing")));
    assert!(!out.shapes.iter().any(|s| matches!(&s.shape,
        egui::Shape::Text(t) if t.galley.job.text == "Item Transform")));
}

#[test]
fn texture_layer_reveals_its_locator_and_its_sub_tab() {
    let (mut a, ctx) = shading_app();
    a.shading_add(NewLayer::ImageMap);
    let layer = a.shading.selected[0];
    let NodeKind::Texture(t) = &a.editor.scene.shader_tree.get(layer).unwrap().kind else {
        panic!()
    };
    let locator = t.locator.expect("a new texture layer gets its locator");
    let name = |a: &Operandi, id: u64| a.editor.scene.item(id).unwrap().name.clone();
    assert_eq!(name(&a, locator), "Image Map (Texture)");
    assert!(a.editor.scene.texture_group().is_some());
    draw(&mut a, &ctx, SIZE, vec![]);
    // The small expander before the name (top-level row: one indent).
    let cell = a.shading.row_rects[&layer][1];
    click(&mut a, &ctx, pos2(cell.min.x + 54., cell.center().y));
    assert!(a.shading.locator_open.contains(&layer));
    // Clicking the revealed locator row opens the Texture Locator sub-tab.
    a.shading.selected.clear();
    click(&mut a, &ctx, pos2(cell.min.x + 90., cell.center().y + 20.));
    assert_eq!(a.shading.selected, vec![layer]);
    assert_eq!(
        a.shading.layer_tab,
        crate::shading_panel::LayerTab::TextureLocator
    );
    let out = draw(&mut a, &ctx, SIZE, vec![]);
    let texts: Vec<String> = out
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t.galley.job.text.clone()),
            _ => None,
        })
        .collect();
    // UIC-114 (issue #6): vectors are one row per component, so Y and Z
    // can't run off the panel's edge.
    for label in [
        "Position X",
        "Rotation X",
        "Y",
        "Z",
        "Projection Type",
        "Projection Axis",
        "Size X",
        "Auto Size",
        "UV Map",
    ] {
        assert!(
            texts.iter().any(|t| t == label),
            "{label} missing: {texts:?}"
        );
    }
    // Deleting the layer deletes its unshared locator, in one undo step.
    a.shading.selected = vec![layer];
    assert!(a.shading_delete_selected());
    assert!(a.editor.scene.texture_locator(locator).is_none());
    a.run(Command::Undo);
    assert!(a.editor.scene.texture_locator(locator).is_some());
}

/// Gradient family §1: a Gradient layer shows its Input Parameter, the
/// Color strip and key list (Diffuse Color is a colour effect).
#[test]
fn gradient_layer_shows_input_strip_and_keys() {
    let (mut a, ctx) = shading_app();
    assert!(a.run_ok(Command::ShaderTree(
        operandi_core::shader_tree::ShaderTreeEdit::Add {
            layer: operandi_core::shader_tree::NewLayer::Gradient,
            selected: Some(4),
        }
    )));
    let id = a.editor.scene.shader_tree.next_id - 1;
    a.shading.selected = vec![id];
    draw(&mut a, &ctx, SIZE, vec![]);
    let out = draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(a.property_tabs()[0], "Texture");
    let texts: Vec<String> = out
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t.galley.job.text.clone()),
            _ => None,
        })
        .collect();
    for want in ["Input Parameter", "Incidence", "Color", "Add Key"] {
        assert!(texts.iter().any(|t| t == want), "no {want}: {texts:?}");
    }
}

/// Where `want` was drawn this frame (its centre), searching nested shapes.
fn text_at(out: &egui::FullOutput, want: &str) -> Option<Pos2> {
    fn walk(shape: &egui::Shape, want: &str, found: &mut Option<Pos2>) {
        match shape {
            egui::Shape::Text(t) if t.galley.text() == want => {
                *found = Some(t.pos + t.galley.rect.center().to_vec2())
            }
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, want, found)),
            _ => {}
        }
    }
    let mut found = None;
    for c in &out.shapes {
        walk(&c.shape, want, &mut found);
    }
    found
}

/// GR-Q03 v2: Edit Gradient… opens the window; a preset replaces the
/// curve the effect reads in one undo step; Reverse mirrors it; closing
/// keeps the curve in Recent.
#[test]
fn edit_gradient_window_applies_presets_and_reverse() {
    let (mut a, ctx) = shading_app();
    assert!(a.run_ok(Command::ShaderTree(
        operandi_core::shader_tree::ShaderTreeEdit::Add {
            layer: operandi_core::shader_tree::NewLayer::Gradient,
            selected: Some(4),
        }
    )));
    let id = a.editor.scene.shader_tree.next_id - 1;
    let gradient = |a: &Operandi| match &a.editor.scene.shader_tree.get(id).unwrap().kind {
        NodeKind::Texture(t) => match &t.source {
            operandi_core::shader_tree::TextureSource::Gradient(g) => (**g).clone(),
            _ => panic!("not a gradient"),
        },
        _ => panic!("not a texture"),
    };
    a.shading.selected = vec![id];
    draw(&mut a, &ctx, SIZE, vec![]);
    let out = draw(&mut a, &ctx, SIZE, vec![]);
    let at = text_at(&out, "Edit Gradient…").expect("the button");
    click(&mut a, &ctx, at);
    assert_eq!(a.shading.gradient_window, Some(id));
    draw(&mut a, &ctx, SIZE, vec![]);
    let out = draw(&mut a, &ctx, SIZE, vec![]);
    assert!(text_at(&out, "Distribute Evenly").is_some());
    let at = text_at(&out, "Presets ▾").expect("the Presets menu");
    click(&mut a, &ctx, at);
    let out = draw(&mut a, &ctx, SIZE, vec![]);
    let at = text_at(&out, "Black to White").expect("a colour preset");
    let before = gradient(&a);
    click(&mut a, &ctx, at);
    let g = gradient(&a);
    assert_eq!(g.color.keys.len(), 2, "{:?}", g.color);
    assert_eq!(g.color.keys[0].out, [0., 0., 0., 1.]);
    // Incidence keys spanned nothing: the input's usual 0..90°.
    assert_eq!(g.color.span(), Some((0., 90.)));
    let out = draw(&mut a, &ctx, SIZE, vec![]);
    let at = text_at(&out, "Reverse").expect("Reverse");
    click(&mut a, &ctx, at);
    assert_eq!(gradient(&a).color.keys[0].out, [1., 1., 1., 1.]);
    a.run(Command::Undo);
    a.run(Command::Undo);
    assert_eq!(gradient(&a), before, "two edits, two undo steps");
    // The preset remembered the curve it replaced.
    assert_eq!(a.shading.gradient_recent.first(), Some(&before));
}

#[test]
fn textures_add_procedurals_with_their_zone_rows() {
    use operandi_core::procedural::ProceduralKind;
    let (mut a, ctx) = shading_app();
    a.shading_add(NewLayer::Procedural(ProceduralKind::Grid));
    let layer = a.shading.selected[0];
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(
        row_order(&a),
        [
            "Alpha Output",
            "Final Color Output",
            "Base Shader",
            "Grid",
            "Base Material"
        ]
    );
    a.shading.selected = vec![layer];
    let out = draw(&mut a, &ctx, SIZE, vec![]);
    let texts: Vec<String> = out
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t.galley.job.text.clone()),
            _ => None,
        })
        .collect();
    // The rows at the top of Properties (the rest scroll).
    for label in ["Type", "Locator", "Line Value", "Line Color"] {
        assert!(texts.iter().any(|t| t == label), "{label} missing");
    }
    a.run(Command::Undo);
    assert!(a.editor.scene.shader_tree.get(layer).is_none());
}

#[test]
fn properties_rows_keep_clear_of_the_scroll_bar() {
    // Jason (2026-11-09): when the Properties list scrolled, its bar
    // covered the fields and buttons at the right edge. egui's floating
    // bar takes no width, so the rows now stop short of it.
    let (mut a, ctx) = shading_app();
    a.shading.selected = vec![3];
    draw(&mut a, &ctx, SIZE, vec![]);
    crate::ui_inventory::take_rects();
    draw(&mut a, &ctx, SIZE, vec![]);
    let rects = crate::ui_inventory::take_rects();
    assert!(!rects.is_empty());
    // The rows end where the side tabs begin (workspace.rs), less the
    // bar's room.
    let content_right = a.properties_tab_rects[0].left() - 2.;
    let room = {
        let s = &ctx.style().spacing.scroll;
        s.bar_inner_margin + s.bar_width + s.bar_outer_margin
    };
    assert!(room >= 8., "scroll bar room {room}");
    for (label, rect) in rects {
        assert!(
            rect.right() <= content_right - room + 0.5,
            "{label} ends at {}, under the scroll bar ({} to {content_right})",
            rect.right(),
            content_right - room
        );
    }
}

/// Material editing item 4: a Material's rows sit in side tabs and
/// folding sections (UIC-100..107), Name above them all (UIC-104).
#[test]
fn material_rows_sit_in_tabs_and_sections() {
    use crate::material_layout::{MATERIAL_REF_TAB, TABS};
    let (mut a, ctx) = shading_app();
    a.shading.selected = vec![4];
    a.properties_tab = "Surface".into();
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(a.property_tabs(), TABS.to_vec());
    let texts = |a: &mut Operandi| -> Vec<String> {
        draw(a, &ctx, SIZE, vec![])
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t.galley.job.text.clone()),
                _ => None,
            })
            .collect()
    };
    let has = |t: &[String], s: &str| t.iter().any(|x| x == s);
    // A tab the Material lacks shows the first (UIC-101).
    let t = texts(&mut a);
    for want in [
        "Name",
        "Diffuse and Specular",
        "Surface Normal",
        "Refractive Index",
        "Shading Model",
        "Reflection Type",
        "Opacity",
    ] {
        assert!(has(&t, want), "{MATERIAL_REF_TAB} lacks {want}");
    }
    assert!(!has(&t, "Absorption Distance"));
    a.properties_tab = "Transparency".into();
    let t = texts(&mut a);
    assert!(has(&t, "Name"));
    assert!(has(&t, "Absorption Distance"));
    assert!(has(&t, "Dispersion"));
    assert!(!has(&t, "Refractive Index"));
    assert!(!has(&t, "Opacity"));
    a.properties_tab = "Tags".into();
    assert!(has(&texts(&mut a), "None yet"));
    // Editing a row in a section still edits the Material.
    a.properties_tab = "Rays".into();
    assert!(has(&texts(&mut a), "Reflection Rays"));
}

/// #56 (UIC-116): an Image Map's Image field has a Browse… button on the
/// row under it, which opens the file panel for a picture straight away
/// (before, the only way was the last entry of the Image list).
#[test]
fn image_map_has_a_browse_button_under_its_image_field() {
    let (mut a, ctx) = shading_app();
    a.shading_add(NewLayer::ImageMap);
    let out = draw(&mut a, &ctx, SIZE, vec![]);
    let out = if text_at(&out, "Browse…").is_some() {
        out
    } else {
        draw(&mut a, &ctx, SIZE, vec![])
    };
    let image = text_at(&out, "Image").expect("the Image row");
    let browse = text_at(&out, "Browse…").expect("a Browse… button");
    assert!(browse.y > image.y, "under the Image field");
    let answer: crate::file_chooser::HeldAnswer = Default::default();
    a.held_panel = Some(answer);
    click(&mut a, &ctx, browse);
    assert!(
        a.choosing.is_some(),
        "the picture panel is open: {}",
        a.status
    );
}

/// #56 (Jason, 2026-11-27): adding an Image Map asks for its picture at
/// once; cancelling keeps the empty layer.
#[test]
fn adding_an_image_map_asks_for_its_picture() {
    let (mut a, ctx) = shading_app();
    let answer: crate::file_chooser::HeldAnswer = Default::default();
    a.held_panel = Some(answer.clone());
    a.shading_add(NewLayer::ImageMap);
    let layer = a.shading.selected[0];
    assert!(
        a.choosing.is_some(),
        "the picture panel opened: {}",
        a.status
    );
    *answer.borrow_mut() = Some(None);
    a.poll_choice(&ctx);
    assert!(a.choosing.is_none());
    assert!(
        a.editor.scene.shader_tree.get(layer).is_some(),
        "cancel keeps the layer"
    );
    // Other layers don't ask.
    a.held_panel = Some(Default::default());
    a.shading_add(NewLayer::Gradient);
    assert!(a.choosing.is_none());
}
