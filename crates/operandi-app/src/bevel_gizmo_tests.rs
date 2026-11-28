//! The Bevels' gizmo follows the selection's normal (2026-11-23): a
//! polygon's normal, an edge's diagonal, a point's normal, and the average
//! over a group of them. Expected directions are worked out here from the
//! mesh itself, not through the code under test.
use super::tests::{box_app, draw};
use super::*;

const SIZE: Vec2 = vec2(1440., 900.);

/// The box with `tool` live on `select`ion in `mode`, drawn once.
fn live(mode: Mode, tool: Tool, select: impl FnOnce(&mut Operandi)) -> Operandi {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    a.camera.view = crate::camera::View::Perspective;
    a.run(Command::Mode(mode));
    select(&mut a);
    a.activate(tool);
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.editor.session.live.is_some(), "{tool:?}: {}", a.status);
    a
}

fn mesh(a: &Operandi) -> &operandi_core::Mesh {
    &a.editor.scene.active().unwrap().mesh
}

/// The polygons holding every point in `points`.
fn faces_with(a: &Operandi, points: &[u32]) -> Vec<usize> {
    let m = mesh(a);
    (0..m.faces.len())
        .filter(|&f| points.iter().all(|p| m.faces[f].vertices.contains(p)))
        .collect()
}

fn normal_of(a: &Operandi, faces: &[usize]) -> DVec3 {
    faces
        .iter()
        .map(|&f| mesh(a).normal(f))
        .sum::<DVec3>()
        .normalize()
}

fn assert_along(got: DVec3, want: DVec3, what: &str) {
    assert!(
        got.normalize().dot(want.normalize()) > 0.9999,
        "{what}: gizmo {got:?}, selection normal {want:?}"
    );
}

/// Two box edges that don't share a polygon's normal: on the top polygon,
/// one each side of its first corner.
fn two_edges(a: &Operandi) -> [(u32, u32); 2] {
    let m = mesh(a);
    let top = (0..m.faces.len())
        .max_by(|&x, &y| m.normal(x).y.total_cmp(&m.normal(y).y))
        .unwrap();
    let v = &m.faces[top].vertices;
    // Edges are stored smaller point first.
    let key = |a: u32, b: u32| (a.min(b), a.max(b));
    [key(v[0], v[1]), key(v[1], v[2])]
}

#[test]
fn polygon_bevel_follows_one_polygon_and_a_group() {
    let a = live(Mode::Polygon, Tool::Bevel, |a| {
        a.editor.scene.selection.faces = [0].into_iter().collect();
    });
    assert_along(a.handle_axis(0), normal_of(&a, &[0]), "one polygon");
    // The Inset arrow stays square to it.
    assert!(a.handle_axis(1).dot(a.handle_axis(0)).abs() < 1e-6);
    // Two neighbouring polygons: their average normal, not the first's.
    let m = mesh(&a);
    let next = (1..m.faces.len())
        .find(|&f| {
            m.faces[f]
                .vertices
                .iter()
                .filter(|v| m.faces[0].vertices.contains(v))
                .count()
                == 2
        })
        .unwrap();
    let a = live(Mode::Polygon, Tool::Bevel, |a| {
        a.editor.scene.selection.faces = [0, next as u32].into_iter().collect();
    });
    assert_along(a.handle_axis(0), normal_of(&a, &[0, next]), "two polygons");
    assert!(a.handle_axis(1).dot(a.handle_axis(0)).abs() < 1e-6);
}

#[test]
fn edge_bevel_follows_an_edges_diagonal_and_a_group() {
    let probe = box_app();
    let [e1, e2] = two_edges(&probe);
    let a = live(Mode::Edge, Tool::EdgeBevel, |a| {
        a.editor.scene.selection.edges = vec![e1];
    });
    let one = normal_of(&a, &faces_with(&a, &[e1.0, e1.1]));
    assert_along(a.handle_axis(0), one, "one edge (its diagonal)");
    let a = live(Mode::Edge, Tool::EdgeBevel, |a| {
        a.editor.scene.selection.edges = vec![e1, e2];
    });
    let two = normal_of(&a, &faces_with(&a, &[e1.0, e1.1]))
        + normal_of(&a, &faces_with(&a, &[e2.0, e2.1]));
    assert_along(a.handle_axis(0), two, "two edges (their average)");
}

#[test]
fn vertex_bevel_follows_a_points_normal_and_a_group() {
    let probe = box_app();
    let [(p, q), _] = two_edges(&probe);
    let a = live(Mode::Vertex, Tool::VertexBevel, |a| {
        a.editor.scene.selection.vertices = vec![p];
    });
    let corner = normal_of(&a, &faces_with(&a, &[p]));
    assert_along(a.handle_axis(0), corner, "one corner");
    let a = live(Mode::Vertex, Tool::VertexBevel, |a| {
        a.editor.scene.selection.vertices = vec![p, q];
    });
    let both = corner + normal_of(&a, &faces_with(&a, &[q]));
    assert_along(a.handle_axis(0), both, "two corners (their average)");
}

/// Edge Bevel has two handles (2026-11-23): the bevel arrow along the edges'
/// normal and the Support Edges handle square to it; Support Edges adds a
/// row of polygons beside the bevel.
#[test]
fn edge_bevel_has_a_support_edges_handle() {
    let probe = box_app();
    let [e1, _] = two_edges(&probe);
    let mut a = live(Mode::Edge, Tool::EdgeBevel, |a| {
        a.editor.scene.selection.edges = vec![e1];
    });
    let handles = a.handles(a.viewport_rect);
    assert!(handles.iter().any(|h| h.0 == 0), "{handles:?}");
    assert!(handles.iter().any(|h| h.0 == 1), "the second handle shows");
    // Both start on the edge, at its middle: set off from it, they floated
    // beside the edge and read as pointing the wrong way.
    let m = mesh(&a);
    let middle = (m.vertices[e1.0 as usize] + m.vertices[e1.1 as usize]) / 2.;
    let on_screen = a.camera.project(middle, a.viewport_rect).unwrap().0;
    for h in handles.iter().filter(|h| h.0 < 2) {
        assert!(
            h.1.distance(on_screen) < 1.,
            "handle {} starts off the edge",
            h.0
        );
    }
    assert!(a.handle_axis(1).dot(a.handle_axis(0)).abs() < 1e-6);
    let mut faces = |support: f64| {
        let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
        p.chamfer.offset = 0.2;
        p.edge_bevel.support = support;
        a.run(Command::Preview(p));
        assert!(a.editor.session.live.is_some(), "{}", a.status);
        a.editor.scene.active().unwrap().mesh.faces.len()
    };
    let plain = faces(0.);
    assert!(faces(0.1) > plain, "Mitering adds polygons");
}

/// The Mitering field (2026-11-23) is the second handle's value: typing it
/// or setting it from a script adds the edges, and the handle's drag shows
/// in it. Edge Chamfer keeps it greyed.
#[test]
fn edge_bevel_mitering_field_and_script_attribute() {
    let probe = box_app();
    let [e1, _] = two_edges(&probe);
    let mut a = live(Mode::Edge, Tool::EdgeBevel, |a| {
        a.editor.scene.selection.edges = vec![e1];
    });
    a.fields.insert("chamfer_offset".into(), "0.2 m".into());
    assert!(a.confirm_fields(), "{}", a.status);
    let plain = mesh(&a).faces.len();
    a.fields.insert("ebc_mitering".into(), "0.1 m".into());
    assert!(a.confirm_fields(), "{}", a.status);
    let live = a.editor.session.live.as_ref().unwrap();
    assert_eq!(live.params.edge_bevel.support, 0.1);
    assert!(mesh(&a).faces.len() > plain, "Mitering adds polygons");
    a.execute("tool.attr edge.bevel mitering 0.05");
    assert_eq!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .edge_bevel
            .support,
        0.05
    );
    assert_eq!(
        a.fields.get("ebc_mitering").map(String::as_str),
        Some("0.05 m")
    );
    a.execute("tool.attr edge.bevel mitering -1");
    assert!(a.status.contains("Mitering"), "{}", a.status);
    assert_eq!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .edge_bevel
            .support,
        0.05
    );
}

/// Edge Bevel's Loop Slide checkbox (2026-11-24): on by default, the
/// measured result (EBC-B3-03; 2026-11-26), in the panel and set by
/// script; a bad value changes nothing.
#[test]
fn edge_bevel_loop_slide_checkbox_and_script_attribute() {
    let probe = box_app();
    let [e1, e2] = two_edges(&probe);
    let mut a = live(Mode::Edge, Tool::EdgeBevel, |a| {
        a.editor.scene.selection.edges = vec![e1, e2];
    });
    let on = |a: &Operandi| {
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .edge_bevel
            .loop_slide
    };
    assert!(on(&a), "on by default");
    a.execute("tool.attr edge.bevel loopSlide ?");
    assert_eq!(a.status, "1");
    a.execute("tool.attr edge.bevel loopSlide off");
    assert!(!on(&a));
    a.fields.insert("chamfer_offset".into(), "0.2 m".into());
    assert!(a.confirm_fields(), "{}", a.status);
    let even = mesh(&a).vertices.clone();
    a.execute("tool.attr edge.bevel loopSlide on");
    assert!(on(&a));
    // On a box the third edge is square to both bevelled edges, so sliding
    // along it and an even width land on the same point: nothing changes.
    // (core tests/edge_bevel_loop_slide.rs covers where they differ.)
    assert_eq!(mesh(&a).vertices, even);
    a.execute("tool.attr edge.bevel loopSlide maybe");
    assert!(a.status.contains("true"), "{}", a.status);
    assert!(on(&a), "a bad value changes nothing");
}

/// Edge Bevel's Mitering Style dropdown (2026-11-24; Corner Pattern until
/// 2026-11-25): Default first and the default (2026-11-26), set by script,
/// and Uniform and Pinch fill the corner where the box's three edges meet
/// their own ways. Patch and Radial are left out for now.
#[test]
fn edge_bevel_corner_pattern_field_and_script_attribute() {
    let probe = box_app();
    let [e1, e2] = two_edges(&probe);
    let shared = if e1.0 == e2.0 || e1.0 == e2.1 {
        e1.0
    } else {
        e1.1
    };
    let third = mesh(&probe)
        .edges()
        .into_iter()
        .find(|&(a, b)| (a == shared || b == shared) && (a, b) != e1 && (a, b) != e2)
        .unwrap();
    let mut a = live(Mode::Edge, Tool::EdgeBevel, |a| {
        a.editor.scene.selection.edges = vec![e1, e2, third];
    });
    let corner = |a: &Operandi| {
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .edge_bevel
            .corner
    };
    assert_eq!(corner(&a), operandi_core::CornerPattern::Balanced);
    a.fields.insert("chamfer_offset".into(), "0.2 m".into());
    a.fields.insert("chamfer_segments".into(), "3".into());
    assert!(a.confirm_fields(), "{}", a.status);
    a.execute("tool.attr edge.bevel segments 3");
    // Each fill by its polygons' sides and its points' places.
    let shapes = |a: &Operandi| {
        let m = mesh(a);
        let mut s: Vec<usize> = m.faces.iter().map(|f| f.vertices.len()).collect();
        s.sort_unstable();
        let mut at: Vec<[i64; 3]> = m
            .vertices
            .iter()
            .map(|p| (*p * 1e6).round().as_i64vec3().to_array())
            .collect();
        at.sort_unstable();
        (s, at)
    };
    let default = shapes(&a);
    for (word, want) in [
        ("neutral", operandi_core::CornerPattern::NoMitering),
        ("uniform", operandi_core::CornerPattern::Uniform),
        ("pinch", operandi_core::CornerPattern::Pinch),
        ("default", operandi_core::CornerPattern::Balanced),
    ] {
        a.execute(&format!("tool.attr edge.bevel miteringStyle {word}"));
        assert_eq!(corner(&a), want, "{}", a.status);
        a.execute("tool.attr edge.bevel miteringStyle ?");
        assert_eq!(a.status, word);
        // A box corner has no split lines to round, so Neutral and Default
        // fill it alike; Uniform's and Pinch's arches differ from them.
        if matches!(word, "uniform" | "pinch") {
            assert_ne!(shapes(&a), default, "{word} fills the corner its own way");
        }
    }
    // Patch and Radial are refused for now; nothing changes.
    for word in ["patch", "radial"] {
        a.execute(&format!("tool.attr edge.bevel miteringStyle {word}"));
        assert!(a.status.contains("isn't available yet"), "{}", a.status);
        assert_eq!(corner(&a), operandi_core::CornerPattern::Balanced);
    }
    a.execute("tool.attr edge.bevel cornerPattern spiral");
    assert!(a.status.contains("Mitering Style"), "{}", a.status);
    assert_eq!(corner(&a), operandi_core::CornerPattern::Balanced);
    // The old names still work.
    a.execute("tool.attr edge.bevel cornerPattern noMitering");
    a.execute("tool.attr edge.bevel miteringStyle ?");
    assert_eq!(a.status, "neutral");
    a.execute("tool.attr edge.bevel cornerPattern balanced");
    assert_eq!(corner(&a), operandi_core::CornerPattern::Balanced);
}

/// Like Polygon Bevel's extrude and inset, Edge Bevel's bevel and Mitering
/// handles combine (2026-11-23): dragging one keeps what the other did, in
/// either order, and the handles then ride on the new bevel instead of
/// staying where the edge was.
#[test]
fn edge_bevel_bevel_and_mitering_combine_and_the_handles_follow() {
    let probe = box_app();
    let [e1, _] = two_edges(&probe);
    let mut results = Vec::new();
    for order in [[0usize, 1], [1, 0]] {
        let mut a = live(Mode::Edge, Tool::EdgeBevel, |a| {
            a.editor.scene.selection.edges = vec![e1];
        });
        let r = a.viewport_rect;
        let start = a.handles(r)[0].1;
        for &axis in &order {
            let h = a.handles(r).into_iter().find(|h| h.0 == axis).unwrap();
            let params = a.editor.session.live.as_ref().unwrap().params.clone();
            let to = h.2 + (h.2 - h.1).normalized() * 30.;
            a.update_handle(h.0, params, h.2, to, Tool::EdgeBevel, r);
        }
        let p = &a.editor.session.live.as_ref().unwrap().params;
        assert!(
            p.chamfer.offset > 0. && p.edge_bevel.support > 0.,
            "{order:?}: {}",
            a.status
        );
        // The handles start at the middle of the bevel's edges now.
        let m = mesh(&a);
        let edges = &a.editor.scene.selection.edges;
        let middle = edges
            .iter()
            .map(|e| (m.vertices[e.0 as usize] + m.vertices[e.1 as usize]) / 2.)
            .sum::<DVec3>()
            / edges.len() as f64;
        let on_screen = a.camera.project(middle, r).unwrap().0;
        let origin = a.handles(r)[0].1;
        assert!(
            origin.distance(on_screen) < 1.,
            "{order:?}: the handles follow the bevel"
        );
        assert!(
            origin.distance(start) > 5.,
            "{order:?}: they moved off the old edge"
        );
        results.push((p.chamfer.offset, p.edge_bevel.support, m.faces.len()));
    }
    assert_eq!(
        results[0].2, results[1].2,
        "the order doesn't change the result"
    );
}

/// The tester's report (2026-11-23): three sides of a square, B, then the
/// Tool Properties. Inset then Mitering (or the other way round, typed or
/// with the field's arrows) refused the second one with "Invalid component
/// selection". Both must apply, in any order, any number of times.
mod panel_bevel_and_mitering {
    use super::super::tests::{arrows_rect, box_app, draw, key, pointer};
    use super::super::*;
    const SIZE: Vec2 = vec2(1440., 1000.);

    fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
        draw(a, ctx, SIZE, events);
    }
    fn type_in(a: &mut Operandi, ctx: &Context, field: &str, value: &str) {
        let at = a.field_rects[field].center();
        frame(a, ctx, vec![Event::PointerMoved(at)]);
        frame(a, ctx, vec![pointer(at, true, Modifiers::NONE)]);
        frame(a, ctx, vec![pointer(at, false, Modifiers::NONE)]);
        let mut events = vec![key(Key::End)];
        events.extend((0..12).map(|_| key(Key::Backspace)));
        events.push(Event::Text(value.into()));
        frame(a, ctx, events);
        frame(a, ctx, vec![key(Key::Enter)]);
        frame(a, ctx, vec![]);
    }
    fn arrows(a: &mut Operandi, ctx: &Context, field: &str) {
        let at = arrows_rect(a, field).center();
        frame(a, ctx, vec![Event::PointerMoved(at)]);
        frame(a, ctx, vec![pointer(at, true, Modifiers::NONE)]);
        for step in 1..=5 {
            frame(
                a,
                ctx,
                vec![Event::PointerMoved(at + vec2(8. * step as f32, 0.))],
            );
        }
        frame(
            a,
            ctx,
            vec![pointer(at + vec2(40., 0.), false, Modifiers::NONE)],
        );
        frame(a, ctx, vec![]);
    }
    /// The box's top square without its back side, then B.
    fn start(ctx: &Context) -> Operandi {
        theme::install(ctx);
        let mut a = box_app();
        a.camera.view = View::Perspective;
        a.run(Command::Mode(Mode::Edge));
        let m = &a.editor.scene.active().unwrap().mesh;
        let v = &m.vertices;
        a.editor.scene.selection.edges = m
            .edges()
            .into_iter()
            .filter(|&(p, q)| {
                let (p, q) = (v[p as usize], v[q as usize]);
                p.y > 0.9 && q.y > 0.9 && !(p.z < -0.9 && q.z < -0.9)
            })
            .collect();
        assert_eq!(a.editor.scene.selection.edges.len(), 3);
        for _ in 0..3 {
            frame(&mut a, ctx, vec![]);
        }
        let c = a.viewport_rect.center();
        frame(&mut a, ctx, vec![Event::PointerMoved(c), key(Key::B)]);
        frame(&mut a, ctx, vec![]);
        assert_eq!(
            a.editor.session.live.as_ref().map(|l| l.tool),
            Some(Tool::EdgeBevel)
        );
        a
    }
    fn values(a: &Operandi) -> (f64, f64) {
        assert!(
            a.field_errors.is_empty(),
            "{:?} {}",
            a.field_errors,
            a.status
        );
        let p = &a.editor.session.live.as_ref().expect("still live").params;
        (p.chamfer.offset, p.edge_bevel.support)
    }
    fn faces(a: &Operandi) -> usize {
        a.editor.scene.active().unwrap().mesh.faces.len()
    }

    #[test]
    fn typed_in_either_order() {
        let ctx = Context::default();
        let mut a = start(&ctx);
        type_in(&mut a, &ctx, "chamfer_offset", "0.2");
        assert_eq!(values(&a), (0.2, 0.));
        let bevelled = faces(&a);
        type_in(&mut a, &ctx, "ebc_mitering", "0.1");
        assert_eq!(values(&a), (0.2, 0.1));
        assert!(faces(&a) > bevelled);
        type_in(&mut a, &ctx, "chamfer_offset", "0.3");
        assert_eq!(values(&a), (0.3, 0.1));
        type_in(&mut a, &ctx, "ebc_mitering", "0.05");
        assert_eq!(values(&a), (0.3, 0.05));
        type_in(&mut a, &ctx, "chamfer_offset", "0.2");
        type_in(&mut a, &ctx, "ebc_mitering", "0.1");
        assert_eq!(values(&a), (0.2, 0.1));

        let ctx = Context::default();
        let mut b = start(&ctx);
        type_in(&mut b, &ctx, "ebc_mitering", "0.1");
        assert_eq!(values(&b), (0., 0.1));
        type_in(&mut b, &ctx, "chamfer_offset", "0.2");
        assert_eq!(values(&b), (0.2, 0.1));
        assert_eq!(faces(&a), faces(&b), "the order doesn't change the result");
    }

    #[test]
    fn with_the_arrows_in_either_order() {
        for order in [
            ["chamfer_offset", "ebc_mitering"],
            ["ebc_mitering", "chamfer_offset"],
        ] {
            let ctx = Context::default();
            let mut a = start(&ctx);
            for field in order.iter().chain(&order) {
                let before = values(&a);
                arrows(&mut a, &ctx, field);
                let after = values(&a);
                let moved = if *field == "chamfer_offset" {
                    after.0 > before.0
                } else {
                    after.1 > before.1
                };
                assert!(
                    moved,
                    "{order:?}: {field} didn't change: {before:?} → {after:?}"
                );
            }
        }
    }
}

/// Vertex Bevel's Inset handle is a blue scale handle, not an arrow (the
/// maintainer, 2026-11-26); the other Bevels keep arrows.
#[test]
fn vertex_bevel_shows_a_scale_handle() {
    use crate::viewport::scale_look_handle;
    assert!(scale_look_handle(Tool::VertexBevel, 0));
    assert!(!scale_look_handle(Tool::EdgeBevel, 0));
    assert!(!scale_look_handle(Tool::Bevel, 0));
}
