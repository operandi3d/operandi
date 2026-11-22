//! Painting stage 7, Part B (PNT-245), acceptance A4: every starter brush
//! applied to Paintbrush and to the Clay sculpt brush keeps the tool and
//! changes the result, through real drags; the Paint layout's strip shows
//! the starters and applies one with a click.
use super::*;
use crate::tests::{draw, pointer};
use operandi_core::SculptBrush;
use operandi_core::paint_stroke::PaintTool;
use std::sync::Arc;

const SIZE: egui::Vec2 = egui::vec2(1400., 900.);

fn app() -> (Operandi, Context) {
    crate::layouts::ship_paint_layout_in_tests();
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    a.execute("operandi.paint.addTexture kind:color resolution:256 color:0,0,0");
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..2 {
        let _ = draw(&mut a, &ctx, SIZE, vec![]);
    }
    (a, ctx)
}

fn drag(a: &mut Operandi, ctx: &Context) {
    let c = a.viewport_rect.center();
    let mut events = vec![
        vec![Event::PointerMoved(c)],
        vec![pointer(c, true, Modifiers::NONE)],
    ];
    for k in 1..10 {
        events.push(vec![Event::PointerMoved(c + vec2(k as f32 * 5., k as f32))]);
    }
    events.push(vec![pointer(c + vec2(45., 9.), false, Modifiers::NONE)]);
    for e in events {
        let _ = draw(a, ctx, SIZE, e);
    }
    let _ = draw(a, ctx, SIZE, vec![]);
}

/// The paint target's pixels after one Paintbrush drag with `starter`
/// (or the default brush).
fn painted(starter: Option<&str>) -> (Arc<operandi_core::paint::pixels::PaintImage>, Operandi) {
    let (mut a, ctx) = app();
    a.execute("operandi.paint.paintbrush");
    if let Some(name) = starter {
        a.execute(&format!("operandi.paint.applyStarter name:\"{name}\""));
    }
    drag(&mut a, &ctx);
    let image = a.current_image().unwrap();
    (a.editor.scene.paint.buffers[&image].clone(), a)
}

/// The sphere's points after one Clay drag.
fn sculpted(starter: Option<&str>) -> (Vec<glam::DVec3>, Operandi) {
    let (mut a, ctx) = app();
    a.activate_sculpt(SculptBrush::Clay);
    a.sculpt.settings.brush_mut(SculptBrush::Clay).tip.size = 120.;
    if let Some(name) = starter {
        a.execute(&format!("operandi.paint.applyStarter name:\"{name}\""));
    }
    drag(&mut a, &ctx);
    (a.editor.scene.active().unwrap().mesh.vertices.clone(), a)
}

#[test]
fn a4_every_starter_keeps_the_tool_and_changes_the_paint_and_the_clay() {
    let (plain_px, _) = painted(None);
    let (plain_pts, _) = sculpted(None);
    let starters = operandi_core::paint_starters::starter_brushes();
    assert_eq!(starters.len(), 7);
    for s in starters {
        let (px, a) = painted(Some(s.name));
        assert_eq!(
            a.painting.brush.tool,
            Some(PaintTool::Paintbrush),
            "{}",
            s.name
        );
        assert_eq!(a.painting.brush.tips.choice, s.brush.choice, "{}", s.name);
        let differs = (0..px.pixel_count()).any(|i| px.get(i) != plain_px.get(i));
        assert!(differs, "{}: the paint looks different", s.name);
        let (pts, a) = sculpted(Some(s.name));
        assert_eq!(a.sculpt.brush, Some(SculptBrush::Clay), "{}", s.name);
        assert_eq!(
            a.painting.brush.sculpt_tips.choice, s.brush.choice,
            "{}",
            s.name
        );
        // Dynamic Smooth only reacts to pen pressure; a mouse press is
        // always full pressure, so on Clay it sculpts as the plain brush.
        if s.name != "Dynamic Smooth" {
            let moved = pts
                .iter()
                .zip(&plain_pts)
                .any(|(p, q)| p.distance(*q) > 1e-9);
            assert!(moved, "{}: the clay looks different", s.name);
        }
        if s.picture.is_some() {
            assert!(
                a.editor.scene.images.iter().any(|i| i.name == s.name),
                "{}: its own picture",
                s.name
            );
        }
    }
}

#[test]
fn the_strip_shows_the_starters_and_a_click_applies_one() {
    let (mut a, ctx) = app();
    a.execute(&crate::Operandi::restore_line("Paint"));
    for _ in 0..3 {
        let _ = draw(&mut a, &ctx, SIZE, vec![]);
    }
    let items = a.painting.strip.as_ref().unwrap().1.clone();
    let k = items
        .iter()
        .position(|i| i.name == "Scattered Dots")
        .expect("starters in the strip");
    assert!(items[k].starter);
    let strip = a.strip_rect().unwrap();
    let size = crate::painting_layout::STRIP_HEIGHT - 8.;
    let at = strip.min + vec2(4. + k as f32 * (size + 6.) + size / 2., 4. + size / 2.);
    for e in [
        vec![Event::PointerMoved(at)],
        vec![
            pointer(at, true, Modifiers::NONE),
            pointer(at, false, Modifiers::NONE),
        ],
    ] {
        let _ = draw(&mut a, &ctx, SIZE, e);
    }
    assert_eq!(
        a.painting.brush.tips.choice,
        operandi_core::paint_tips::TipChoice::Hard,
        "{}",
        a.status
    );
    assert!(a.painting.brush.tips.nozzle.jitter_position > 1.);
}
