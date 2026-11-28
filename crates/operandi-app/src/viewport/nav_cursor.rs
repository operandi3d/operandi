//! The pointer shows what a view navigation drag does (Leo, 2026-11-28):
//! while the view is orbited, panned, zoomed, rolled or looked around, the
//! system pointer hides and a small drawn sign takes its place at the
//! pointer: a circling arrow for orbit, a four-way cross for pan, a
//! magnifier for zoom and so on. The signs are Operandi's own drawings,
//! light with a dark edge so they read over any background.
use eframe::egui::{Color32, Painter, Pos2, Shape, Stroke, Vec2, vec2};
use std::f32::consts::PI;

use super::input::NavigationKind;

/// Half the sign's size, in points.
const R: f32 = 9.;
const FILL: Color32 = Color32::from_rgb(245, 245, 245);
const EDGE: Color32 = Color32::from_rgb(20, 20, 20);

/// Draws `kind`'s sign centred on `at`.
pub(crate) fn draw(painter: &Painter, at: Pos2, kind: NavigationKind) {
    // Each line twice: a wide dark edge under a thin light stroke.
    for (width, colour) in [(4.5f32, EDGE), (2f32, FILL)] {
        let stroke = Stroke::new(width, colour);
        for shape in strokes(at, kind, stroke) {
            painter.add(shape);
        }
    }
}

/// The sign's lines and arrow heads.
pub(crate) fn strokes(at: Pos2, kind: NavigationKind, stroke: Stroke) -> Vec<Shape> {
    let mut out = Vec::new();
    match kind {
        // A circling arrow round the pointer.
        NavigationKind::Orbit => circling(&mut out, at, R, stroke, false),
        // The same round a dot: turning about the view's own axis.
        NavigationKind::Roll => {
            circling(&mut out, at, R, stroke, false);
            out.push(Shape::circle_filled(at, stroke.width * 0.9, stroke.color));
        }
        // Turning in place: a circling arrow the other way.
        NavigationKind::Look => circling(&mut out, at, R, stroke, true),
        // Arrows out in four directions.
        NavigationKind::Pan => {
            for d in [vec2(1., 0.), vec2(-1., 0.), vec2(0., 1.), vec2(0., -1.)] {
                arrow(&mut out, at, at + d * R * 1.2, stroke);
            }
        }
        // Up and down only.
        NavigationKind::PanVertical => {
            for d in [vec2(0., 1.), vec2(0., -1.)] {
                arrow(&mut out, at, at + d * R * 1.2, stroke);
            }
        }
        // A magnifier: the lens round the pointer, the handle down-right.
        NavigationKind::Zoom | NavigationKind::BoxZoom => {
            let lens = R * 0.7;
            out.push(Shape::circle_stroke(at, lens, stroke));
            let d = vec2(1., 1.).normalized();
            out.push(Shape::line_segment(
                [at + d * lens, at + d * (lens + R * 0.8)],
                stroke,
            ));
            if kind == NavigationKind::Zoom {
                // A plus in the lens.
                let s = lens * 0.5;
                out.push(Shape::line_segment(
                    [at - vec2(s, 0.), at + vec2(s, 0.)],
                    stroke,
                ));
                out.push(Shape::line_segment(
                    [at - vec2(0., s), at + vec2(0., s)],
                    stroke,
                ));
            }
        }
    }
    out
}

/// Three quarters of a circle of radius `r` round `at` with an arrow head
/// at its end; `reverse` runs it the other way round.
fn circling(out: &mut Vec<Shape>, at: Pos2, r: f32, stroke: Stroke, reverse: bool) {
    // Both end at the top, the arrow head pointing across it (Leo,
    // 2026-11-28: a quarter turn right of where they first ended).
    let (from, span) = if reverse {
        (PI, -PI * 1.5)
    } else {
        (0., PI * 1.5)
    };
    let steps = 18;
    let point = |a: f32| at + vec2(a.cos(), a.sin()) * r;
    let arc: Vec<Pos2> = (0..=steps)
        .map(|k| point(from + span * k as f32 / steps as f32))
        .collect();
    out.push(Shape::line(arc, stroke));
    let end = from + span;
    let tip = point(end);
    // Along the circle at the end, the way it runs.
    let along = vec2(-end.sin(), end.cos()) * span.signum();
    head(out, tip, along, stroke);
}

/// A line from `from` to `to` with a head at `to`.
fn arrow(out: &mut Vec<Shape>, from: Pos2, to: Pos2, stroke: Stroke) {
    out.push(Shape::line_segment([from, to], stroke));
    head(out, to, (to - from).normalized(), stroke);
}

/// A V-shaped arrow head at `tip`, pointing along `dir`.
fn head(out: &mut Vec<Shape>, tip: Pos2, dir: Vec2, stroke: Stroke) {
    let size = R * 0.45;
    let side = vec2(-dir.y, dir.x);
    for s in [1., -1.] {
        out.push(Shape::line_segment(
            [tip, tip - dir * size + side * (size * 0.8 * s)],
            stroke,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    #[test]
    fn the_circling_arrows_end_at_the_top() {
        let at = pos2(100., 100.);
        let s = Stroke::new(1_f32, Color32::WHITE);
        for kind in [NavigationKind::Orbit, NavigationKind::Look] {
            let Shape::Path(arc) = &strokes(at, kind, s)[0] else {
                panic!("{kind:?}: the arc comes first")
            };
            let end = *arc.points.last().unwrap();
            assert!(
                (end.x - at.x).abs() < 0.01 && end.y < at.y,
                "{kind:?} ends at {end:?}"
            );
        }
    }

    #[test]
    fn every_kind_has_a_sign_and_zoom_and_pan_differ() {
        let at = pos2(100., 100.);
        let s = Stroke::new(1_f32, Color32::WHITE);
        for kind in [
            NavigationKind::Orbit,
            NavigationKind::Pan,
            NavigationKind::Zoom,
            NavigationKind::Roll,
            NavigationKind::Look,
            NavigationKind::PanVertical,
            NavigationKind::BoxZoom,
        ] {
            let shapes = strokes(at, kind, s);
            assert!(!shapes.is_empty(), "{kind:?}");
            // Every sign stays near the pointer.
            for shape in &shapes {
                let r = shape.visual_bounding_rect();
                assert!(
                    r.min.distance(at) < 3. * R && r.max.distance(at) < 3. * R,
                    "{kind:?}: {r:?}"
                );
            }
        }
        let n = |k| strokes(at, k, s).len();
        assert_ne!(n(NavigationKind::Pan), n(NavigationKind::Zoom));
    }
}
