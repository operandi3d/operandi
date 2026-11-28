//! SS-U03 app regression: after a live Smooth Shift/Thicken stage is replaced
//! (Offset 0.1 m → 0.2 m through the panel field), dropped, undone and
//! redone, the viewport picking path and ordinary click selection follow the
//! current cage. The probe point lies on a generated wall between 0.1 m and
//! 0.2 m from the source face, so neither the source cube nor the earlier
//! 0.1 m preview has a surface there. The expected face is derived from the
//! current mesh's planes, independently of the pick under test. Synthetic egui
//! events only; no physical-input, native-focus or GPU claim.
use super::tests::{draw, key, pointer};
use super::*;

const SIZE: Vec2 = vec2(1440., 900.);

#[test]
fn smooth_shift_generated_wall_picks_after_replace_drop_undo_redo() {
    run(Tool::SmoothShift)
}

#[test]
fn thicken_generated_wall_picks_after_replace_drop_undo_redo() {
    run(Tool::Thicken)
}

fn context() -> Context {
    let ctx = Context::default();
    theme::install(&ctx);
    ctx
}

fn click(a: &mut Operandi, ctx: &Context, at: Pos2) {
    draw(
        a,
        ctx,
        SIZE,
        vec![
            pointer(at, true, Modifiers::NONE),
            pointer(at, false, Modifiers::NONE),
        ],
    );
}

fn tap(a: &mut Operandi, ctx: &Context, k: Key, at: Pos2) {
    let release = Event::Key {
        key: k,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at), key(k), release]);
}

fn labels(a: &Operandi) -> (Vec<String>, Vec<String>) {
    let l = |h: &[std::sync::Arc<Checkpoint>]| h.iter().map(|c| c.label.clone()).collect();
    (l(&a.editor.history.undo), l(&a.editor.history.redo))
}

/// Source face, its outward unit normal and its world corner positions in
/// the source cube.
struct Source {
    normal: DVec3,
    corners: Vec<DVec3>,
}

/// World position `height` along the source normal above the midpoint of the
/// source edge whose outward side faces the camera most directly. With 100 %
/// Scale this lies on that edge's generated wall once Offset exceeds
/// `height`, and in empty space otherwise.
fn probe(a: &Operandi, source: &Source, height: f64) -> (DVec3, DVec3) {
    let (eye, ..) = a.camera.basis();
    let center = source.corners.iter().copied().sum::<DVec3>() / source.corners.len() as f64;
    let n = source.corners.len();
    let (mid, out) = (0..n)
        .map(|i| {
            let (p, q) = (source.corners[i], source.corners[(i + 1) % n]);
            let mid = (p + q) * 0.5;
            (mid, (mid - center).normalize())
        })
        .max_by(|x, y| {
            let fx = x.1.dot((eye - x.0).normalize());
            let fy = y.1.dot((eye - y.0).normalize());
            fx.total_cmp(&fy)
        })
        .unwrap();
    assert!(
        out.dot((eye - mid).normalize()) > 0.2,
        "camera must see the wall clearly, not edge-on"
    );
    (mid + source.normal * height, out)
}

/// Faces of the current active mesh whose plane contains `point` with
/// outward normal `out` and whose convex outline encloses it. Computed from
/// mesh data alone, not from picking.
fn containing_faces(scene: &Scene, point: DVec3, out: DVec3) -> Vec<u32> {
    let item = scene.active().unwrap();
    let mesh = &item.mesh;
    (0..mesh.faces.len())
        .filter(|&f| {
            let w: Vec<DVec3> = mesh.faces[f]
                .vertices
                .iter()
                .map(|&v| scene.world_point(item.id, mesh.vertices[v as usize]))
                .collect();
            let normal = (0..w.len())
                .map(|i| (w[i] - w[0]).cross(w[(i + 1) % w.len()] - w[0]))
                .sum::<DVec3>()
                .normalize_or_zero();
            normal.dot(out) > 1. - 1e-9
                && (point - w[0]).dot(normal).abs() < 1e-9
                && (0..w.len()).all(|i| {
                    let e = w[(i + 1) % w.len()] - w[i];
                    // Strictly inside by a margin: no edge/corner ambiguity.
                    e.cross(point - w[i]).dot(normal) > 1e-3 * e.length()
                })
        })
        .map(|f| f as u32)
        .collect()
}

/// The viewport's own picking route (`pick_component`, used by click
/// selection and the per-frame hover surface) at `at`.
fn picked_polygon(a: &Operandi, at: Pos2) -> Option<u32> {
    match a.pick_component(at, a.viewport_rect) {
        Some((item, Component::Polygon(f))) => {
            assert_eq!(Some(item), a.editor.scene.selection.item);
            Some(f)
        }
        Some((_, other)) => panic!("unexpected component {other:?}"),
        None => None,
    }
}

/// The viewport pick at `at` must report the independently derived wall.
fn assert_picks(a: &Operandi, at: Pos2, expected: u32, tool: Tool, label: &str) {
    assert_eq!(picked_polygon(a, at), Some(expected), "{tool:?} {label}");
}

fn run(tool: Tool) {
    let ctx = context();
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.activate(Tool::Box);
    a.run(Command::Preview(Parameters {
        size: DVec3::splat(2.),
        ..Default::default()
    }));
    a.run(Command::Apply);
    a.run(Command::Drop);
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![0];
    a.category = toolbox::Category::Polygon;
    a.camera.view = View::Perspective;
    a.frame(false);
    let cube = a.editor.scene.clone();
    let source = {
        let item = cube.active().unwrap();
        Source {
            normal: item.mesh.normal(0).normalize(),
            corners: item.mesh.faces[0]
                .vertices
                .iter()
                .map(|&v| cube.world_point(item.id, item.mesh.vertices[v as usize]))
                .collect(),
        }
    };
    let cube_faces = cube.active().unwrap().mesh.faces.len();

    a.activate(tool);
    for (name, value) in [
        ("offset", "0.1m"),
        ("scale", "100%"),
        ("angle", "89.5 deg"),
        ("thicken", "off"),
        ("sharp", "off"),
        ("angle_weight", "off"),
        ("both_sides", "off"),
    ] {
        a.execute(&format!("{} {name} {value}", tool.label()));
        assert_eq!(a.status, "Ready", "{tool:?} {name}");
    }
    draw(&mut a, &ctx, SIZE, vec![]);
    let empty = a.viewport_rect.left_top() + vec2(20., 60.);

    // The probe: 0.15 m along the source normal on the camera-facing wall.
    let (world, out) = probe(&a, &source, 0.15);
    let (at, _) = a.camera.project(world, a.viewport_rect).unwrap();
    assert!(a.viewport_rect.shrink(4.).contains(at), "{tool:?}");
    assert!(containing_faces(&cube, world, out).is_empty());
    let cube_pick = picked_polygon(&a, at);

    // First ordinary viewport click establishes the live 0.1 m stage.
    click(&mut a, &ctx, empty);
    assert!(a.editor.session.live.as_ref().is_some_and(|l| l.applied));
    let stage_01 = a.editor.scene.clone();
    assert!(
        containing_faces(&stage_01, world, out).is_empty(),
        "{tool:?}"
    );
    let pick_01 = picked_polygon(&a, at);

    // Live field replacement 0.1 m → 0.2 m (Return commits; SS-U02).
    let field = a.field_rects["sst_offset"].center();
    click(&mut a, &ctx, field);
    a.fields.insert("sst_offset".into(), "0.2m".into());
    draw(&mut a, &ctx, SIZE, vec![key(Key::Enter)]);
    draw(&mut a, &ctx, SIZE, vec![]);
    let live = a.editor.session.live.as_ref().unwrap();
    assert!(live.applied && live.tool == tool, "{tool:?}");
    assert!((live.params.smooth_shift.offset - 0.2).abs() < 1e-12);
    assert!(!a.text_editor_focused(&ctx), "{tool:?}: Return kept focus");
    let replaced = a.editor.scene.clone();
    assert_eq!(
        replaced.active().unwrap().mesh.faces.len(),
        stage_01.active().unwrap().mesh.faces.len(),
        "{tool:?}: replacement must not stack a second stage"
    );
    let walls = containing_faces(&replaced, world, out);
    assert_eq!(
        walls.len(),
        1,
        "{tool:?}: one generated wall holds the probe"
    );
    let wall = walls[0];
    assert!(
        wall as usize >= cube_faces,
        "{tool:?}: wall is new topology"
    );

    // Live: the render/pick snapshot is the editor scene (no cone/ellipsoid
    // overlay), and it reports the replaced wall. Selection clicks belong
    // to the tool here, so no selection assertion is made while live.
    assert!(a.editor.cone_preview_scene().ok().flatten().is_none());
    assert!(a.editor.ellipsoid_preview_scene().ok().flatten().is_none());
    assert_picks(&a, at, wall, tool, "live after replacement");
    // Oracle sensitivity: stale snapshots give a different answer.
    for (stale, label) in [(&cube, "source cube"), (&stage_01, "0.1 m preview")] {
        let hit = picking::Surface::new(stale).pick(&a.camera, at, a.viewport_rect);
        assert_ne!(hit.map(|h| h.1), Some(wall), "{tool:?} {label}");
    }
    assert_ne!(cube_pick, Some(wall), "{tool:?}");
    assert_ne!(pick_01, Some(wall), "{tool:?}");

    // Viewport Q drops the valid 0.2 m preview.
    tap(&mut a, &ctx, Key::Q, empty);
    assert!(a.editor.session.live.is_none(), "{tool:?}: Q did not drop");
    assert_eq!(
        a.editor.scene.active().unwrap().mesh,
        replaced.active().unwrap().mesh
    );
    let (undo, redo) = labels(&a);
    assert_eq!(undo.last().map(String::as_str), Some(tool.label()));
    assert!(redo.is_empty());
    let committed = a.editor.scene.clone();
    assert_picks(&a, at, wall, tool, "after Drop");

    // An ordinary viewport click selects the generated wall.
    click(&mut a, &ctx, at);
    let s = &a.editor.scene.selection;
    assert_eq!(
        (s.mode, s.faces.clone()),
        (Mode::Polygon, vec![wall]),
        "{tool:?}"
    );
    assert_eq!(labels(&a).0.last().map(String::as_str), Some("Selection"));
    let selected = a.editor.scene.clone();
    assert_eq!(
        selected.active().unwrap().mesh,
        committed.active().unwrap().mesh
    );
    assert_eq!(containing_faces(&selected, world, out), vec![wall]);

    // Undo unwinds the selection checkpoint first, then the modeling stage.
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, committed, "{tool:?}: selection undo");
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, cube, "{tool:?}: modeling undo");
    assert_eq!(
        labels(&a).1,
        vec!["Selection".to_string(), tool.label().into()]
    );
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    let undone = picked_polygon(&a, at);
    assert_eq!(undone, cube_pick, "{tool:?}: stale hit after Undo");
    assert!(undone.is_none_or(|f| (f as usize) < cube_faces));
    // Redo restores the stage, then the wall selection.
    a.run(Command::Redo);
    assert_eq!(a.editor.scene, committed, "{tool:?}: modeling redo");
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_picks(&a, at, wall, tool, "after Redo");
    a.run(Command::Redo);
    assert_eq!(a.editor.scene, selected, "{tool:?}: selection redo");
    let s = &a.editor.scene.selection;
    let faces = a.editor.scene.active().unwrap().mesh.faces.len();
    assert!(s.faces.iter().all(|&f| (f as usize) < faces));
    assert_eq!(containing_faces(&a.editor.scene, world, out), vec![wall]);
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_picks(&a, at, wall, tool, "after selection Redo");

    // Back to the cube: an ordinary click at the probe selects only what
    // the restored cube shows there, never the undone wall index.
    a.run(Command::Undo);
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, cube, "{tool:?}");
    click(&mut a, &ctx, at);
    let s = &a.editor.scene.selection;
    assert_eq!(s.faces.first().copied(), cube_pick, "{tool:?}");
    assert!(
        s.faces.iter().all(|&f| (f as usize) < cube_faces),
        "{tool:?}"
    );
    assert_eq!(
        a.editor.scene.active().unwrap().mesh,
        cube.active().unwrap().mesh
    );
}
