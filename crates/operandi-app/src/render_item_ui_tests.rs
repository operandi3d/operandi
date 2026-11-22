//! The Render item in the Shader Tree (render item spec v1): clicking it
//! selects it and shows its Frame and Settings tabs; a field drag is one
//! undo step; a layer click leaves it; Final Color's Denoising is the
//! render's denoise switch.
use super::tests::{box_app, draw, pointer};
use super::*;
use crate::render_item_ui::{FRAME_TAB, SETTINGS_TAB, TABS};
use crate::shading_panel::ShaderKey;

const SIZE: Vec2 = vec2(1440., 900.);

fn app() -> (Operandi, Context) {
    let mut a = box_app();
    let ctx = Context::default();
    theme::install(&ctx);
    a.upper_frame.show_page(1);
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    (a, ctx)
}

fn frames(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
    draw(a, ctx, SIZE, events);
    draw(a, ctx, SIZE, vec![]);
}

fn click(a: &mut Operandi, ctx: &Context, at: Pos2) {
    frames(
        a,
        ctx,
        vec![
            Event::PointerMoved(at),
            pointer(at, true, Modifiers::NONE),
            pointer(at, false, Modifiers::NONE),
        ],
    );
}

fn render_row(a: &Operandi) -> Rect {
    a.shading
        .tree
        .rects
        .iter()
        .find(|(k, _)| *k == ShaderKey::Section("Render"))
        .map(|(_, r)| *r)
        .expect("the Render row is drawn")
}

/// The field rects of the rows drawn now, by label.
fn field_rects(a: &mut Operandi, ctx: &Context) -> std::collections::BTreeMap<String, Rect> {
    draw(a, ctx, SIZE, vec![]);
    crate::ui_inventory::take_rects();
    draw(a, ctx, SIZE, vec![]);
    crate::ui_inventory::take_rects()
}

#[test]
fn clicking_render_selects_it_and_shows_its_tabs() {
    let (mut a, ctx) = app();
    a.shading.selected = vec![3];
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(!a.render_item_selected());
    let row = render_row(&a).center();
    click(&mut a, &ctx, row);
    assert!(a.render_item_selected(), "RI-02: the click selects Render");
    assert!(a.shading.selected.is_empty(), "and nothing else");
    assert_eq!(a.property_tabs(), TABS.to_vec());
    // RI-03: the item selection and mode don't change.
    let item = a.editor.scene.selection.item;
    let rects = field_rects(&mut a, &ctx);
    assert_eq!(a.editor.scene.selection.item, item);
    for label in ["Name", "Render Camera", "Frame Width", "Frame Height"] {
        assert!(
            rects.contains_key(label),
            "Frame tab misses {label}: {rects:?}"
        );
    }
    // The Settings tab.
    a.properties_tab = SETTINGS_TAB.into();
    a.shading.render_selected = true;
    let rects = field_rects(&mut a, &ctx);
    for label in [
        "Maximum Quality",
        "Noise Threshold",
        "Diffuse Depth",
        "Reflection Depth",
        "Refraction Depth",
    ] {
        assert!(
            rects.contains_key(label),
            "Settings tab misses {label}: {rects:?}"
        );
    }
    a.properties_tab = crate::render_item_ui::PREVIEW_TAB.into();
    let rects = field_rects(&mut a, &ctx);
    for label in ["Convergence Target", "Max Render Time"] {
        assert!(
            rects.contains_key(label),
            "Preview tab misses {label}: {rects:?}"
        );
    }
    // RI-P01/P02 reach the Preview's request, not F9.
    a.run(Command::SetRenderSettings(
        operandi_core::render_settings::RenderSettings {
            preview_convergence: 0.5,
            preview_time_limit: 0.1,
            ..a.editor.scene.render.clone()
        },
    ));
    a.preview.saved.samples = 200;
    a.editor.scene.add_default_camera_and_light().unwrap();
    let request = a.preview_request().expect("a preview request");
    assert_eq!(request.samples, 100);
    assert!((request.time_limit - 6.0).abs() < 1e-4);
    a.properties_tab = SETTINGS_TAB.into();
    // A layer click leaves the Render item; the tab chosen is kept for
    // when Render is picked again (UIC-21).
    let layer = *a.shading.row_rects.keys().next().unwrap();
    let at = a.shading.row_rects[&layer][1].center();
    click(&mut a, &ctx, at);
    assert!(!a.render_item_selected());
    assert_eq!(a.shading.selected, vec![layer]);
    let row = render_row(&a).center();
    click(&mut a, &ctx, row);
    assert!(a.render_item_selected());
    assert_eq!(a.properties_tab, SETTINGS_TAB);
    assert_ne!(a.properties_tab, FRAME_TAB);
}

#[test]
fn a_field_drag_is_one_undo_step() {
    let (mut a, ctx) = app();
    let row = render_row(&a).center();
    click(&mut a, &ctx, row);
    a.properties_tab = SETTINGS_TAB.into();
    let rects = field_rects(&mut a, &ctx);
    let at = rects["Diffuse Depth"].center();
    let before = a.editor.scene.render.clone();
    let steps = a.editor.undo.len();
    let mut events = vec![Event::PointerMoved(at), pointer(at, true, Modifiers::NONE)];
    for k in 1..=6 {
        events.push(Event::PointerMoved(at + vec2(10. * k as f32, 0.)));
    }
    draw(&mut a, &ctx, SIZE, events);
    for k in 7..=10 {
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::PointerMoved(at + vec2(10. * k as f32, 0.))],
        );
    }
    let end = at + vec2(100., 0.);
    frames(&mut a, &ctx, vec![pointer(end, false, Modifiers::NONE)]);
    let after = a.editor.scene.render.clone();
    assert!(
        after.diffuse_depth > before.diffuse_depth,
        "the drag raised Diffuse Depth ({} -> {})",
        before.diffuse_depth,
        after.diffuse_depth
    );
    assert_eq!(a.editor.undo.len(), steps + 1, "one step for the drag");
    assert_eq!(a.editor.undo.last().unwrap().label, "Render Settings");
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.render, before);
}

#[test]
fn final_color_denoising_is_the_render_denoise_switch() {
    let (mut a, ctx) = app();
    let fc = a
        .editor
        .scene
        .shader_tree
        .nodes
        .iter()
        .find(|n| {
            matches!(
                n.kind,
                operandi_core::shader_tree::NodeKind::RenderOutput(
                    operandi_core::shader_tree::OutputEffect::FinalColor
                )
            )
        })
        .unwrap()
        .id;
    a.shading.selected = vec![fc];
    let rects = field_rects(&mut a, &ctx);
    let at = *rects.get("Denoising").expect("Final Color shows Denoising");
    assert!(a.editor.scene.render_denoise());
    // Open the dropdown, then pick None (the first entry).
    click(&mut a, &ctx, at.center());
    // The list opens under the field; None is its first entry.
    click(&mut a, &ctx, at.center() + vec2(0., at.height() + 6.));
    assert!(
        !a.editor.scene.render_denoise(),
        "picking None turns the render's denoising off"
    );
    a.run(Command::Undo);
    assert!(a.editor.scene.render_denoise());
}

#[test]
fn a_new_scene_uses_the_new_scene_render_settings() {
    let mut a = Operandi::empty();
    a.initialize_new_document();
    assert_eq!(
        a.editor.scene.render,
        operandi_core::render_settings::RenderSettings::new_scene()
    );
}

/// Direct Caustics spec v1: the Global Illumination tab's Caustics
/// section; ticking Direct Caustics is one undo step.
#[test]
fn global_illumination_tab_turns_direct_caustics_on() {
    let (mut a, ctx) = app();
    let row = render_row(&a).center();
    click(&mut a, &ctx, row);
    a.properties_tab = crate::render_item_ui::GI_TAB.into();
    let rects = field_rects(&mut a, &ctx);
    for label in [
        "Direct Caustics",
        "Total Photons",
        "Local Photons",
        "Walkthrough Mode",
        "Indirect Caustics",
    ] {
        assert!(
            rects.contains_key(label),
            "GI tab misses {label}: {rects:?}"
        );
    }
    assert!(!a.editor.scene.render.direct_caustics.enabled);
    let steps = a.editor.undo.len();
    let at = rects["Direct Caustics"].center();
    click(&mut a, &ctx, at);
    assert!(a.editor.scene.render.direct_caustics.enabled);
    assert_eq!(a.editor.undo.len(), steps + 1);
    a.run(Command::Undo);
    assert!(!a.editor.scene.render.direct_caustics.enabled);
}
