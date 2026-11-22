//! Painting stage 6 in the app: the Paint layout, the Sculpt/Paint toolbox
//! set, the brush preset strip and the Utilities tab (PNT-210..212), driven
//! through real frames and pointer events.
use super::*;
use crate::tests::{draw, pointer};
use crate::toolbox::Category;

const SIZE: egui::Vec2 = egui::vec2(1400., 900.);

fn app() -> Operandi {
    crate::layouts::ship_paint_layout_in_tests();
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    a.execute("operandi.paint.addTexture kind:color resolution:256 color:0,0,0");
    a
}

fn frames(a: &mut Operandi, ctx: &Context, n: usize) {
    for _ in 0..n {
        let _ = draw(a, ctx, SIZE, vec![]);
    }
}

fn click(a: &mut Operandi, ctx: &Context, p: Pos2) {
    for events in [
        vec![Event::PointerMoved(p)],
        vec![pointer(p, true, Modifiers::NONE)],
        vec![pointer(p, false, Modifiers::NONE)],
    ] {
        let _ = draw(a, ctx, SIZE, events);
    }
    frames(a, ctx, 1);
}

#[test]
fn the_paint_layout_is_listed_at_90_and_not_starred() {
    let a = app();
    let paint = a
        .layouts
        .list
        .iter()
        .find(|e| e.doc().name == "Paint")
        .expect("painting builds ship the Paint layout");
    assert_eq!(paint.doc().order, 90);
    assert!(!paint.doc().starred);
    assert_eq!(
        paint.doc().open_command.as_deref(),
        Some("operandi.paint.layout")
    );
}

#[test]
fn acceptance_29_the_paint_layout_opens_the_set_the_strip_and_textured_view() {
    let mut a = app();
    let ctx = Context::default();
    frames(&mut a, &ctx, 2);
    a.execute(&crate::Operandi::restore_line("Paint"));
    frames(&mut a, &ctx, 3);
    assert_eq!(a.layouts.current, "Paint", "{}", a.status);
    assert_eq!(
        a.paint_set_tabs(),
        Some(vec![
            Category::Sculpt,
            Category::Paint,
            Category::VertexMaps,
            Category::PaintUtilities
        ])
    );
    assert_eq!(a.category, Category::Paint);
    assert_eq!(
        a.display.settings.active.style,
        crate::view_display::Style::Texture
    );
    // The strip of the set's tab names was drawn, and nothing else.
    for name in [
        "Sculpt Tools",
        "Paint Tools",
        "Vertex Map Tools",
        "Utilities",
    ] {
        assert!(
            a.toolbox_rects.contains_key(&format!("Category {name}")),
            "{name} tab"
        );
    }
    assert!(!a.toolbox_rects.contains_key("Category Create"));
    // The brush preset strip runs along the view's bottom.
    let strip = a.strip_rect().expect("the strip shows");
    assert!(a.viewport_rect.contains_rect(strip));
    // Save a preset with a Hard tip, change the tip, then left-click the
    // preset's thumbnail: it comes back and nothing is painted. The
    // layout already turned Paintbrush on.
    assert_eq!(
        a.painting.brush.tool,
        Some(operandi_core::paint_stroke::PaintTool::Paintbrush)
    );
    a.execute("operandi.paint.tip kind:hard");
    a.execute("operandi.paint.savePreset name:StripHard");
    a.execute("operandi.paint.tip kind:smooth");
    a.painting.strip = None;
    frames(&mut a, &ctx, 2);
    let items = a.painting.strip.as_ref().unwrap().1.clone();
    let k = items
        .iter()
        .position(|i| i.name == "StripHard")
        .expect("saved preset in the strip");
    let size = crate::painting_layout::STRIP_HEIGHT - 8.;
    let thumb = strip.min + vec2(4. + k as f32 * (size + 6.) + size / 2., 4. + size / 2.);
    let image = a.current_image().unwrap();
    let before = a.editor.scene.paint.buffers[&image].clone();
    let steps = a.editor.undo.len();
    click(&mut a, &ctx, thumb);
    assert_eq!(
        a.painting.brush.tips.choice,
        operandi_core::paint_tips::TipChoice::Hard,
        "{}",
        a.status
    );
    assert_eq!(a.editor.undo.len(), steps, "the click didn't paint");
    assert!(std::sync::Arc::ptr_eq(
        &before,
        &a.editor.scene.paint.buffers[&image]
    ));
    // Back to Model: the usual tabs, no strip.
    a.execute(&crate::Operandi::restore_line("Model"));
    frames(&mut a, &ctx, 2);
    assert!(a.paint_set_tabs().is_none());
    assert!(a.strip_rect().is_none());
    assert!(a.toolbox_rects.contains_key("Category Create"));
    let _ = std::fs::remove_file(crate::painting_brush::brush_folder(&a).join("StripHard.opbrush"));
}

#[test]
fn clicking_the_utilities_tab_shows_its_commands_through_the_toolbox() {
    let mut a = app();
    let ctx = Context::default();
    frames(&mut a, &ctx, 2);
    a.execute(&crate::Operandi::restore_line("Paint"));
    frames(&mut a, &ctx, 3);
    let tab = a.toolbox_rects["Category Utilities"];
    click(&mut a, &ctx, tab.center());
    assert_eq!(a.category, Category::PaintUtilities);
    let form = a.forms.get("toolbox.paintutil").unwrap();
    let lines: Vec<String> = form
        .controls
        .iter()
        .filter_map(|c| c.command.clone())
        .collect();
    for line in [
        "operandi.paint.addTexture kind:color",
        "operandi.paint.addTexture kind:displacement",
        "operandi.paint.addTexture kind:mask",
        "operandi.paint.addFurTexture",
        "sculpt.relaxMesh",
        "operandi.image.invert",
        "operandi.image.clear",
        "operandi.image.expandSeams",
        "operandi.image.toVectorDisplacement",
        "operandi.image.setResolution",
        "operandi.image.flatten",
    ] {
        assert!(lines.iter().any(|l| l == line), "{line}");
    }
    for greyed in [
        "operandi.paint.addFurTexture",
        "operandi.image.toVectorDisplacement",
        "operandi.paint.unavailable name:{Copy to Morph}",
    ] {
        assert!(
            a.line_state(greyed).unwrap().unavailable.is_some(),
            "{greyed} greyed"
        );
    }
    assert!(a.line_state("sculpt.relaxMesh").is_some());
    // Vertex Map Tools holds the vertex-map paint brushes.
    let vm = a.forms.get("toolbox.vertexmaps").unwrap();
    assert!(
        vm.controls
            .iter()
            .any(|c| c.command.as_deref() == Some("Vertex Map Paint"))
    );
}

#[test]
fn texture_menu_has_image_processing_and_file_has_the_image_entries() {
    let a = app();
    let ctx = egui::Context::default();
    let menus = a.menu_model(&ctx);
    let file = menus.iter().find(|m| m.id == "menu.file").unwrap();
    let texture = menus.iter().find(|m| m.id == "menu.texture").unwrap();
    let titles = |m: &crate::app_menu::Menu| -> Vec<String> {
        m.entries
            .iter()
            .filter_map(|e| match e {
                crate::app_menu::Entry::Submenu(s) => Some(s.title.clone()),
                crate::app_menu::Entry::Item(i) => Some(i.title.clone()),
                crate::app_menu::Entry::Separator => None,
            })
            .collect()
    };
    let f = titles(file);
    for t in [
        "New Image…",
        "New Image Sequence…",
        "Save Image",
        "Save Image As…",
        "Save All Images",
    ] {
        assert!(f.iter().any(|x| x == t), "File > {t}: {f:?}");
    }
    let t = titles(texture);
    assert!(t.iter().any(|x| x == "Paint Tools"), "{t:?}");
    assert!(t.iter().any(|x| x == "Image Processing"), "{t:?}");
}

/// The Model layout's strip has no Paint tab (Jason, 2026-11-18); the
/// Paint layout's set does, and Texture > Paint Tools still opens it from
/// the Model layout.
#[test]
fn only_the_paint_layout_has_a_paint_tab() {
    let mut a = app();
    let ctx = Context::default();
    frames(&mut a, &ctx, 2);
    assert_eq!(a.layouts.current, "Model");
    let paint_tab = |a: &Operandi| {
        a.toolbox_rects
            .keys()
            .any(|k| k == "Category Paint" || k == "Category Paint Tools")
    };
    assert!(a.toolbox_rects.contains_key("Category Sculpt"));
    assert!(!paint_tab(&a), "no Paint tab in the Model layout");
    a.execute(&crate::Operandi::restore_line("Paint"));
    frames(&mut a, &ctx, 3);
    assert!(paint_tab(&a), "the Paint layout's set has it");
    a.execute(&crate::Operandi::restore_line("Model"));
    a.category = Category::Sculpt;
    frames(&mut a, &ctx, 3);
    assert_eq!(a.layouts.current, "Model", "{}", a.status);
    assert!(!paint_tab(&a), "gone again in the Model layout");
    // Texture > Paint Tools > Paint Tab from the Model layout opens it.
    a.execute("operandi.paint.tab");
    frames(&mut a, &ctx, 2);
    assert_eq!(a.category, Category::Paint, "{}", a.status);
    assert!(paint_tab(&a), "shown while open");
}
