//! Painting stage 2 in the app (painting_brush.rs): the paint tools'
//! commands and toolbox, a stroke through the viewport's press, drag and
//! release, sizing, Auto Scale and the status line.
use super::*;
use crate::form::command_form::{Control, FormRef};
use operandi_core::paint_stroke::{PaintBlend, PaintTool};
use std::sync::Arc;

/// A sphere with UVs, selected, with a black Color texture as the paint
/// target; the camera looks at it.
fn app() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    a.execute("operandi.paint.addTexture kind:color resolution:256 color:0,0,0");
    assert!(a.paint_target().is_some(), "{}", a.status);
    a
}

fn rect() -> Rect {
    Rect::from_min_size(pos2(0., 0.), vec2(800., 600.))
}

/// Every command line a form's buttons run, sub-forms included.
fn lines(controls: &[Control], out: &mut Vec<String>) {
    for c in controls {
        if let Some(line) = &c.command {
            out.push(line.clone());
        }
        if let Some(FormRef::Inline(inner)) = &c.form {
            lines(&inner.controls, out);
        }
    }
}

#[test]
fn the_paint_tab_runs_commands_and_greys_later_tools() {
    let a = app();
    let form = a
        .forms
        .get(crate::toolbox_form::PAINT_FORM)
        .expect("the Paint tab's form");
    let mut all = Vec::new();
    lines(&form.controls, &mut all);
    for line in &all {
        let state = a
            .line_state(line)
            .unwrap_or_else(|| panic!("{line} isn't a command"));
        if line.starts_with("operandi.paint.notYet") {
            assert_eq!(
                state.unavailable.as_deref(),
                Some(crate::painting_brush::LATER),
                "{line}"
            );
        } else if line == "operandi.paint.ink kind:projection" {
            // No camera in a new scene (PNT-190).
            assert_eq!(state.unavailable.as_deref(), Some("Needs a camera"));
        } else {
            assert!(
                state.unavailable.is_none(),
                "{line}: {:?}",
                state.unavailable
            );
        }
    }
    for live in [
        "operandi.paint.airbrush",
        "operandi.paint.paintbrush",
        "operandi.paint.eraser",
    ] {
        assert!(all.iter().any(|l| l == live), "{live} on the tab");
    }
    let later: Vec<&String> = all.iter().filter(|l| l.contains("notYet")).collect();
    assert!(
        !later.is_empty(),
        "the later stages keep their greyed buttons"
    );
    let name = "Duplicate Tools";
    assert!(crate::painting_brush::LATER_TOOLS.contains(&name));
    assert!(all.iter().any(|l| l.contains(name)), "{name} on the tab");
    // Stage 5: one button per ink (no flyouts).
    for kind in ["image", "parametric", "random", "projection"] {
        let line = format!("operandi.paint.ink kind:{kind}");
        assert!(all.contains(&line), "{line} on the tab");
    }
    assert!(crate::toolbox::Category::ALL.contains(&crate::toolbox::Category::Paint));
}

#[test]
fn a_paint_tool_and_a_sculpt_brush_turn_each_other_off() {
    let mut a = app();
    a.execute("operandi.paint.airbrush");
    assert_eq!(a.painting.brush.tool, Some(PaintTool::Airbrush));
    assert!(a.line_state("operandi.paint.airbrush").unwrap().on);
    assert_eq!(a.tool_properties_title(), "Airbrush");
    // Picking a tool picks a tip: Smooth to start (PNT-41).
    assert!(a.line_state("operandi.paint.tip kind:smooth").unwrap().on);
    a.execute("Sculpt Push");
    assert!(a.sculpt.brush.is_some());
    assert_eq!(a.painting.brush.tool, None);
    a.execute("operandi.paint.eraser");
    assert!(a.sculpt.brush.is_none());
    a.execute("operandi.paint.eraser");
    assert_eq!(a.painting.brush.tool, None, "its button turns it off again");
}

#[test]
fn a_stroke_paints_live_and_is_one_undo_step() {
    let mut a = app();
    a.execute("operandi.paint.airbrush");
    a.execute("operandi.paint.set size:30");
    let image = a.current_image().unwrap();
    let before = a.editor.scene.paint.buffers[&image].clone();
    let r = rect();
    let steps = a.editor.undo.len();
    let kind = a.paint_press(r.center(), r, Modifiers::NONE);
    assert!(matches!(kind, input::GestureKind::Paint), "{}", a.status);
    for k in 1..20 {
        a.paint_drag(r.center() + vec2(k as f32 * 3., 0.), r);
    }
    // Live: the scene already shows paint before the release (PNT-51).
    let live = a.editor.scene.paint.buffers[&image].clone();
    assert!(!Arc::ptr_eq(&live, &before));
    let white = (0..live.pixel_count()).any(|i| live.get(i)[0] > 0.99);
    assert!(white, "white paint during the stroke");
    a.finish_paint_stroke();
    assert_eq!(a.editor.undo.len(), steps + 1);
    assert_eq!(a.editor.undo.last().unwrap().label, "Airbrush Stroke");
    assert!(a.editor.image_changed(image));
    a.execute("app.undo");
    assert!(
        Arc::ptr_eq(&a.editor.scene.paint.buffers[&image], &before),
        "undone exactly"
    );
}

#[test]
fn a_press_off_the_mesh_or_without_a_texture_paints_nothing() {
    let mut a = app();
    a.execute("operandi.paint.paintbrush");
    let r = rect();
    assert!(matches!(
        a.paint_press(pos2(5., 5.), r, Modifiers::NONE),
        input::GestureKind::Blocked
    ));
    assert!(
        a.painting.brush.notice.is_some(),
        "off the mesh says so on screen"
    );
    // With the texture's layer not selected, the top image layer is still
    // painted (PNT-Q40).
    a.shading.selected.clear();
    assert!(matches!(
        a.paint_press(r.center(), r, Modifiers::NONE),
        input::GestureKind::Paint
    ));
    a.finish_paint_stroke();
    // No texture at all: Add a new texture?
    let mut bare = Operandi::empty();
    bare.run(Command::NewMesh);
    bare.run(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    bare.execute("operandi.paint.paintbrush");
    assert!(matches!(
        bare.paint_press(r.center(), r, Modifiers::NONE),
        input::GestureKind::Blocked
    ));
    assert!(bare.painting.ask_add, "no texture: Add a new texture?");
}

#[test]
fn right_drag_sizes_the_tip_up_to_the_maximum() {
    let mut a = app();
    a.execute("operandi.paint.airbrush");
    let start = a.painting.brush.tip().size;
    let p = pos2(100., 100.);
    assert!(matches!(
        a.begin_paint_size(p),
        input::GestureKind::PaintSize { .. }
    ));
    a.paint_size_drag(start, p, p + vec2(40., 0.));
    assert_eq!(a.painting.brush.tip().size, start + 20.);
    a.paint_size_drag(start, p, p + vec2(100_000., 0.));
    assert_eq!(
        a.painting.brush.tip().size,
        crate::painting_brush::MAX_BRUSH_SIZE
    );
    a.paint_size_release();
    assert!(
        a.status.contains("Airbrush") && a.status.contains("500 px"),
        "{}",
        a.status
    );
}

#[test]
fn settings_rows_run_commands_and_custom_profile_edits_make_custom() {
    let mut a = app();
    a.execute("operandi.paint.paintbrush");
    a.execute("operandi.paint.set opacity:50% blend:{Soft Light} attenuation:20 fg:1,0,0");
    let s = a.painting.brush.settings(PaintTool::Paintbrush);
    assert_eq!(
        (s.opacity, s.blend, s.attenuation, s.fg.color),
        (0.5, PaintBlend::SoftLight, 20, [1., 0., 0.])
    );
    // Other tools keep their own rows; the tip is shared.
    a.execute("operandi.paint.set size:42");
    assert_eq!(a.painting.brush.settings(PaintTool::Airbrush).opacity, 1.);
    assert_eq!(a.painting.brush.settings(PaintTool::Airbrush).tip.size, 42.);
    a.execute("operandi.paint.set inWeight:0.8");
    assert_eq!(
        a.painting.brush.tip().shape,
        operandi_core::brush_engine::TipShape::Custom
    );
    a.execute("operandi.paint.set opacity:2");
    assert!(a.status.contains("0 % to 100 %"), "{}", a.status);
    a.execute("operandi.paint.tip kind:hard");
    assert_eq!(
        a.painting.brush.tip().kind,
        operandi_core::brush_engine::TipKind::Hard
    );
}

#[test]
fn auto_scale_keeps_the_tip_on_the_model_when_zooming() {
    let mut a = app();
    a.execute("operandi.paint.airbrush");
    a.execute("operandi.paint.set size:30");
    let r = rect();
    let at = DVec3::ZERO;
    assert_eq!(
        a.paint_radius_for_test(at, r),
        30.,
        "Size is constant on screen"
    );
    a.execute("operandi.paint.set autoScale:true");
    let near = a.paint_radius_for_test(at, r);
    a.camera.distance *= 2.;
    let far = a.paint_radius_for_test(at, r);
    assert!((far - near / 2.).abs() < 0.5, "{near} -> {far}");
}

#[test]
fn texture_menu_has_paint_tools() {
    let a = app();
    let ctx = egui::Context::default();
    let menus = a.menu_model(&ctx);
    let texture = menus.iter().find(|m| m.id == "menu.texture").unwrap();
    let titles: Vec<String> = texture.leaves().iter().map(|i| i.title.clone()).collect();
    for t in ["Airbrush", "Paintbrush", "Eraser", "Smooth Tip", "Hard Tip"] {
        assert!(titles.iter().any(|x| x == t), "{t} in {titles:?}");
    }
}

#[test]
fn the_paint_tab_joins_the_strip_once_opened() {
    let mut a = Operandi::empty();
    assert!(!a.painting.brush.tab_shown);
    a.execute("operandi.paint.tab");
    assert_eq!(a.category, crate::toolbox::Category::Paint, "{}", a.status);
    let mut b = app();
    b.execute("operandi.paint.airbrush");
    assert!(b.painting.brush.tab_shown);
    assert_eq!(b.category, crate::toolbox::Category::Paint);
}

#[test]
fn every_paint_tool_has_a_live_button_and_command() {
    let a = app();
    let form = a.forms.get(crate::toolbox_form::PAINT_FORM).unwrap();
    let mut all = Vec::new();
    lines(&form.controls, &mut all);
    for tool in PaintTool::ALL {
        assert!(
            all.iter().any(|l| l == tool.command()),
            "{} on the Paint tab",
            tool.label()
        );
        assert!(a.line_state(tool.command()).unwrap().unavailable.is_none());
    }
    assert!(all.iter().any(|l| l == "operandi.paint.lasso"));
}

#[test]
fn clone_needs_a_source_then_copies_with_a_kept_offset() {
    let mut a = app();
    a.execute("operandi.paint.clone");
    let r = rect();
    let c = r.center();
    assert!(matches!(
        a.paint_press(c, r, Modifiers::NONE),
        input::GestureKind::Blocked
    ));
    assert_eq!(a.status, "Ctrl+click to set the clone source");
    let ctrl = Modifiers {
        ctrl: true,
        command: true,
        ..Modifiers::NONE
    };
    a.paint_press(c - vec2(40., 0.), r, ctrl);
    assert!(a.painting.brush.clone_source.is_some());
    assert!(matches!(
        a.paint_press(c + vec2(20., 0.), r, Modifiers::NONE),
        input::GestureKind::Paint
    ));
    a.finish_paint_stroke();
    let offset = a
        .painting
        .brush
        .clone_offset
        .expect("the first dab fixes the offset");
    assert!((offset.x + 60.).abs() < 1e-3);
    // The next stroke keeps it.
    a.paint_press(c + vec2(30., 10.), r, Modifiers::NONE);
    a.finish_paint_stroke();
    assert_eq!(a.painting.brush.clone_offset, Some(offset));
}

#[test]
fn line_previews_while_dragging_and_paints_on_release() {
    let mut a = app();
    a.execute("operandi.paint.line");
    a.execute("operandi.paint.set size:6");
    let image = a.current_image().unwrap();
    let before = a.editor.scene.paint.buffers[&image].clone();
    let r = rect();
    let steps = a.editor.undo.len();
    assert!(matches!(
        a.paint_press(r.center() - vec2(60., 0.), r, Modifiers::NONE),
        input::GestureKind::Paint
    ));
    a.paint_drag(r.center() + vec2(60., 0.), r);
    assert!(
        Arc::ptr_eq(&a.editor.scene.paint.buffers[&image], &before),
        "nothing written while dragging"
    );
    a.finish_paint_stroke();
    assert_eq!(a.editor.undo.len(), steps + 1, "{}", a.status);
    assert_eq!(a.editor.undo.last().unwrap().label, "Line Stroke");
    a.execute("operandi.paint.set lineShape:circle");
    assert_eq!(
        a.painting.brush.settings(PaintTool::Line).shape,
        operandi_core::paint_stroke::LineShape::Circle
    );
}

#[test]
fn fill_is_one_click_and_gradients_go_down_on_release() {
    let mut a = app();
    let r = rect();
    a.execute("operandi.paint.fill");
    a.execute("operandi.paint.set fg:1,0,0");
    let steps = a.editor.undo.len();
    assert!(matches!(
        a.paint_press(r.center(), r, Modifiers::NONE),
        input::GestureKind::Blocked
    ));
    assert_eq!(a.editor.undo.len(), steps + 1, "{}", a.status);
    assert_eq!(a.editor.undo.last().unwrap().label, "Fill");
    let image = a.current_image().unwrap();
    // Black everywhere, so the whole image is now red.
    assert_eq!(a.editor.scene.paint.buffers[&image].get(0)[0], 1.);
    a.execute("operandi.paint.linearGradient");
    assert!(matches!(
        a.paint_press(r.center() - vec2(30., 0.), r, Modifiers::NONE),
        input::GestureKind::Paint
    ));
    a.paint_drag(r.center() + vec2(30., 0.), r);
    a.finish_paint_stroke();
    assert_eq!(
        a.editor.undo.last().unwrap().label,
        "Linear Gradient",
        "{}",
        a.status
    );
}

#[test]
fn a_drawn_lasso_stays_when_a_paint_tool_is_picked() {
    let mut a = app();
    let r = rect();
    a.execute("operandi.paint.lasso");
    a.execute("operandi.paint.lassoSet style:rectangle softBorder:8");
    assert!(a.painting.brush.lasso_mode);
    a.paint_press(r.center() - vec2(30., 30.), r, Modifiers::NONE);
    a.paint_drag(r.center() + vec2(30., 30.), r);
    a.finish_paint_stroke();
    let lasso = a.painting.brush.lasso.clone().expect("drawn");
    assert_eq!(
        lasso.style,
        operandi_core::paint_lasso::LassoStyle::Rectangle
    );
    assert_eq!(lasso.soft_border, 8.);
    a.execute("operandi.paint.lasso");
    a.execute("operandi.paint.airbrush");
    assert!(a.painting.brush.lasso.is_some());
    a.execute("operandi.paint.lassoClear");
    assert!(a.painting.brush.lasso.is_none());
}

#[test]
fn every_tip_button_is_live_and_lit_when_picked() {
    let mut a = app();
    let form = a.forms.get(crate::toolbox_form::PAINT_FORM).unwrap();
    let mut all = Vec::new();
    lines(&form.controls, &mut all);
    for choice in operandi_core::paint_tips::TipChoice::ALL {
        let line = format!("operandi.paint.tip kind:{}", choice.word());
        assert!(all.contains(&line), "{line} on the Paint tab");
        a.execute(&line);
        assert_eq!(a.painting.brush.tips.choice, choice);
        assert!(a.line_state(&line).unwrap().on);
    }
    for live in ["operandi.paint.savePreset", "operandi.paint.nozzle"] {
        assert!(all.iter().any(|l| l == live));
    }
}

#[test]
fn the_sculpt_tab_gains_the_painting_buttons_only_in_painting_builds() {
    let a = app();
    // The slot form fills the Sculpt tab's tail.
    let form = a
        .forms
        .get("toolbox.sculpt.painting")
        .expect("painting builds add it");
    let mut all = Vec::new();
    lines(&form.controls, &mut all);
    // Stage 7 (PNT-151): live, not greyed.
    for line in [
        "operandi.paint.sculptTip kind:procedural",
        "operandi.paint.sculptTip kind:sphere",
        "operandi.paint.sculptTip kind:image",
        "operandi.paint.sculptTip kind:stamp",
        "operandi.paint.sculptTip kind:text",
        "operandi.paint.savePreset",
        "operandi.paint.nozzle",
        "operandi.paint.ink kind:image",
        "operandi.paint.maskBrush kind:paint",
        "operandi.paint.maskClear",
    ] {
        assert!(all.iter().any(|l| l == line), "{line}");
    }
    assert!(
        !all.iter().any(|l| l.contains("notYet")),
        "nothing left greyed"
    );
    assert_eq!(form.slot.as_ref().unwrap().form, "toolbox.sculpt");
}

#[test]
fn stamp_and_text_tips_lay_one_copy_on_release() {
    let mut a = app();
    let r = rect();
    let image = a.current_image().unwrap();
    a.execute("operandi.paint.paintbrush");
    a.execute("operandi.paint.tip kind:text");
    let before = a.editor.scene.paint.buffers[&image].clone();
    let steps = a.editor.undo.len();
    assert!(matches!(
        a.paint_press(r.center(), r, Modifiers::NONE),
        input::GestureKind::Paint
    ));
    a.paint_drag(r.center() + vec2(80., 0.), r);
    assert!(
        Arc::ptr_eq(&a.editor.scene.paint.buffers[&image], &before),
        "nothing until release"
    );
    a.finish_paint_stroke();
    assert_eq!(a.editor.undo.len(), steps + 1, "{}", a.status);
    let px = &a.editor.scene.paint.buffers[&image];
    assert!(
        (0..px.pixel_count()).any(|i| px.get(i)[0] > 0.5),
        "the text is painted"
    );
    // The Texture tip needs its image first.
    a.execute("operandi.paint.tip kind:stamp");
    assert!(matches!(
        a.paint_press(r.center(), r, Modifiers::NONE),
        input::GestureKind::Blocked
    ));
    assert!(a.status.contains("Pick an image"), "{}", a.status);
    a.execute(&format!("operandi.paint.tipSet image:{image}"));
    assert!(matches!(
        a.paint_press(r.center(), r, Modifiers::NONE),
        input::GestureKind::Paint
    ));
    a.paint_drag(r.center() + vec2(0., 50.), r);
    a.finish_paint_stroke();
    assert_eq!(a.editor.undo.len(), steps + 2, "{}", a.status);
}

#[test]
fn nozzle_rows_and_jitter_through_commands() {
    let mut a = app();
    a.execute("operandi.paint.airbrush");
    a.execute("operandi.paint.nozzle");
    assert!(a.painting.brush.tips.nozzle_on);
    a.execute(
        "operandi.paint.tipSet jitter:true jitterPosition:50% sensitivity:high strengthBy:both",
    );
    let n = a.painting.brush.tips.nozzle;
    assert!(n.jitter && n.jitter_position == 0.5);
    assert_eq!(n.sensitivity, operandi_core::paint_tips::Sensitivity::High);
    a.execute("operandi.paint.tipSet jitterSize:6");
    assert!(a.status.contains("500 %"), "{}", a.status);
    // A jittered stroke still paints and is one step.
    let r = rect();
    let steps = a.editor.undo.len();
    a.paint_press(r.center(), r, Modifiers::NONE);
    for k in 1..10 {
        a.paint_drag(r.center() + vec2(k as f32 * 4., 0.), r);
    }
    a.finish_paint_stroke();
    assert_eq!(a.editor.undo.len(), steps + 1);
}

#[test]
fn saving_and_applying_a_brush_preset_brings_every_value_back() {
    let mut a = app();
    let dir = std::env::temp_dir().join(format!("operandi-brush-ui-{}", std::process::id()));
    a.execute("operandi.paint.airbrush");
    a.execute("operandi.paint.tip kind:procedural");
    a.execute("operandi.paint.tipSet pattern:dots scaleU:2 jitter:true");
    a.execute("operandi.paint.nozzle");
    a.execute("operandi.paint.set size:44");
    let saved = a.painting.brush.tips.clone();
    let file = dir.join("Dots.opbrush");
    a.execute(&format!(
        "operandi.paint.savePreset filename:{}",
        file.display()
    ));
    assert!(file.is_file(), "{}", a.status);
    // Change everything, and the tool.
    a.execute("operandi.paint.tip kind:hard");
    a.execute("operandi.paint.set size:5");
    a.execute("operandi.paint.nozzle");
    a.execute("operandi.paint.eraser");
    a.execute(&format!(
        "operandi.paint.applyPreset filename:{}",
        file.display()
    ));
    assert_eq!(a.painting.brush.tips, saved, "{}", a.status);
    assert_eq!(
        a.painting.brush.tool,
        Some(PaintTool::Eraser),
        "the tool stays"
    );
}

/// The way a user paints: real pointer events through the viewport's
/// input handling, frame by frame (not calls into the paint code).
#[test]
fn a_left_drag_in_the_viewport_paints_through_the_real_input_path() {
    use crate::tests::{draw, pointer};
    let mut a = app();
    let ctx = Context::default();
    let size = vec2(1400., 900.);
    a.execute("operandi.paint.airbrush");
    a.execute("operandi.paint.set size:30");
    let image = a.current_image().unwrap();
    let before = a.editor.scene.paint.buffers[&image].clone();
    let steps = a.editor.undo.len();
    let _ = draw(&mut a, &ctx, size, vec![]);
    let _ = draw(&mut a, &ctx, size, vec![]);
    let c = a.viewport_rect.center();
    assert!(a.viewport_rect.width() > 100., "the viewport is laid out");
    let mut frames = vec![
        vec![Event::PointerMoved(c)],
        vec![pointer(c, true, Modifiers::NONE)],
    ];
    for k in 1..15 {
        frames.push(vec![Event::PointerMoved(c + vec2(k as f32 * 4., 0.))]);
    }
    frames.push(vec![pointer(c + vec2(56., 0.), false, Modifiers::NONE)]);
    for events in frames {
        let _ = draw(&mut a, &ctx, size, events);
    }
    let _ = draw(&mut a, &ctx, size, vec![]);
    assert_eq!(
        a.editor.undo.len(),
        steps + 1,
        "one stroke step; status: {}",
        a.status
    );
    assert_eq!(a.editor.undo.last().unwrap().label, "Airbrush Stroke");
    let after = &a.editor.scene.paint.buffers[&image];
    assert!(!Arc::ptr_eq(after, &before));
    assert!(
        (0..after.pixel_count()).any(|i| after.get(i)[0] > 0.9),
        "white paint landed"
    );
    // And the viewport's GPU materials carry the painted pixels.
    // (Headless frames don't draw on a GPU, so drive the viewport's
    // material fold the way the viewport does.)
    let scene = &a.editor.scene;
    let mut resident = crate::gpu_scene::Resident::default();
    resident.sync_materials(scene, false);
    let item = scene.active().unwrap();
    let g = resident
        .item(
            item,
            scene.world_matrix(item.id).unwrap(),
            None,
            None,
            &scene.smoothing,
        )
        .0;
    assert!(
        !g.fill.is_empty() && g.fill.iter().all(|v| v.program != 0),
        "the sphere draws its texture per pixel"
    );
    let m = resident
        .gpu_materials()
        .expect("the viewport reads the image");
    let white = m
        .images
        .iter()
        .any(|img| img.rgba.chunks(4).any(|p| p[0] > 230 && p[3] > 230));
    assert!(white, "the viewport's texture shows the paint");
}
