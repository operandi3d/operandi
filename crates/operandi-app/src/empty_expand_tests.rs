//! Shift+double-click on empty viewport space grows the component
//! selection (empty_expand.rs; Jason, 2026-11-23).
use super::tests::{box_app, draw, face, pointer};
use super::*;
use operandi_core::Mesh;

/// Two separate 2x2 quad grids in one mesh, on the ground plane seen from
/// the top: piece A at x -3..-1 (points 0..9, faces 0..4), piece B at
/// x 1..3 (points 9..18, faces 4..8). The gap between them is empty.
fn two_pieces_app() -> Operandi {
    let mut a = box_app();
    let mesh = &mut a.editor.scene.active_mut().unwrap().mesh;
    **mesh = Mesh::default();
    for left in [-3., 1.] {
        for row in 0..3 {
            for column in 0..3 {
                mesh.vertices
                    .push(DVec3::new(left + column as f64, 0., row as f64 - 1.));
            }
        }
    }
    for base in [0u32, 9] {
        for row in 0..2u32 {
            for column in 0..2u32 {
                let a = base + row * 3 + column;
                mesh.faces.push(face(vec![a, a + 3, a + 4, a + 1]));
            }
        }
    }
    a
}

fn ready(a: &mut Operandi, ctx: &Context, mode: Mode) {
    for _ in 0..2 {
        draw(a, ctx, vec2(1440., 900.), vec![]);
    }
    a.editor.scene.selection.mode = mode;
}

fn click(a: &mut Operandi, ctx: &Context, at: Pos2, mods: Modifiers) {
    draw(
        a,
        ctx,
        vec2(1440., 900.),
        vec![pointer(at, true, mods), pointer(at, false, mods)],
    );
}

fn double_click(a: &mut Operandi, ctx: &Context, at: Pos2, mods: Modifiers) {
    click(a, ctx, at, mods);
    click(a, ctx, at, mods);
}

fn pause(a: &mut Operandi, ctx: &Context) {
    // Synthetic frames advance 1/60 s: 30 of them is half a second.
    for _ in 0..30 {
        draw(a, ctx, vec2(1440., 900.), vec![]);
    }
}

/// The screen point between the two pieces, checked to hit nothing.
fn empty(a: &Operandi) -> Pos2 {
    let r = a.viewport_rect;
    let at = a.camera.project(DVec3::ZERO, r).unwrap().0;
    assert!(a.pick_component(at, r).is_none(), "the gap is empty");
    at
}

fn on(a: &Operandi, world: DVec3) -> Pos2 {
    a.camera.project(world, a.viewport_rect).unwrap().0
}

fn sorted<T: Ord + Clone>(v: &[T]) -> Vec<T> {
    let mut v = v.to_vec();
    v.sort();
    v
}

#[test]
fn polygons_add_the_connected_piece_in_one_undo_step() {
    let mut a = two_pieces_app();
    let ctx = Context::default();
    ready(&mut a, &ctx, Mode::Polygon);
    // A click on piece A's first polygon (centre -2.5, -0.5).
    let first = on(&a, DVec3::new(-2.5, 0., -0.5));
    click(&mut a, &ctx, first, Modifiers::NONE);
    assert_eq!(a.editor.scene.selection.faces, vec![0], "{}", a.status);
    let history = a.editor.undo.len();
    let gap = empty(&a);
    double_click(&mut a, &ctx, gap, Modifiers::SHIFT);
    let faces = &a.editor.scene.selection.faces;
    assert_eq!(sorted(faces), vec![0, 1, 2, 3], "piece A only");
    assert_eq!(a.status, "Expand Selection: 4 selected Polygons");
    assert_eq!(a.editor.undo.len(), history + 1, "one undo step");
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.selection.faces, vec![0]);
    a.run(Command::Redo);
    assert_eq!(a.editor.scene.selection.faces.len(), 4);

    // A third press is an ordinary Shift+click on nothing.
    click(&mut a, &ctx, gap, Modifiers::SHIFT);
    assert_eq!(a.editor.scene.selection.faces.len(), 4);
    assert_eq!(a.editor.undo.len(), history + 1);
}

#[test]
fn vertices_add_every_connected_point_of_each_selected_piece() {
    let mut a = two_pieces_app();
    let ctx = Context::default();
    ready(&mut a, &ctx, Mode::Vertex);
    a.editor.scene.selection.vertices = vec![4];
    let history = a.editor.undo.len();
    let gap = empty(&a);
    double_click(&mut a, &ctx, gap, Modifiers::SHIFT);
    let s = &a.editor.scene.selection;
    assert_eq!(sorted(&s.vertices), (0..9).collect::<Vec<u32>>());
    assert_eq!(a.status, "Expand Selection: 9 selected Vertices");
    assert_eq!(a.editor.undo.len(), history + 1);

    // With a point on each piece, both pieces grow.
    a.editor.scene.selection.vertices = vec![0, 17];
    pause(&mut a, &ctx);
    double_click(&mut a, &ctx, gap, Modifiers::SHIFT);
    assert_eq!(a.editor.scene.selection.vertices.len(), 18, "{}", a.status);
}

#[test]
fn edges_add_the_full_loop_through_each_selected_edge() {
    let mut a = two_pieces_app();
    let ctx = Context::default();
    ready(&mut a, &ctx, Mode::Edge);
    // Piece A's middle row and piece B's middle column.
    a.editor.scene.selection.edges = vec![(3, 4), (10, 13)];
    let history = a.editor.undo.len();
    let gap = empty(&a);
    double_click(&mut a, &ctx, gap, Modifiers::SHIFT);
    let edges = &a.editor.scene.selection.edges;
    assert_eq!(
        edges[..2],
        [(3, 4), (10, 13)],
        "the selection is kept, first"
    );
    assert_eq!(
        sorted(edges),
        vec![(3, 4), (4, 5), (10, 13), (13, 16)],
        "{}",
        a.status
    );
    assert_eq!(a.status, "Expand Selection: 4 selected Edges");
    assert_eq!(a.editor.undo.len(), history + 1);
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.selection.edges, vec![(3, 4), (10, 13)]);

    // A loop that can't grow says so and adds no step.
    a.run(Command::Redo);
    pause(&mut a, &ctx);
    double_click(&mut a, &ctx, gap, Modifiers::SHIFT);
    assert_eq!(a.editor.scene.selection.edges.len(), 4);
    assert_eq!(a.editor.undo.len(), history + 1, "{}", a.status);
}

#[test]
fn nothing_selected_or_item_mode_does_nothing() {
    let mut a = two_pieces_app();
    let ctx = Context::default();
    ready(&mut a, &ctx, Mode::Polygon);
    a.run(Command::ClearSelection);
    let history = a.editor.undo.len();
    let gap = empty(&a);
    double_click(&mut a, &ctx, gap, Modifiers::SHIFT);
    assert!(a.editor.scene.selection.faces.is_empty());
    assert_eq!(a.status, "Expand Selection: select components first");
    assert_eq!(a.editor.undo.len(), history);

    a.editor.scene.selection.mode = Mode::Item;
    let before = a.editor.scene.selection.clone();
    pause(&mut a, &ctx);
    double_click(&mut a, &ctx, gap, Modifiers::SHIFT);
    assert_eq!(a.editor.scene.selection, before);
    assert!(!a.status.starts_with("Expand Selection"), "{}", a.status);
}

#[test]
fn other_presses_keep_their_own_behaviour() {
    let mut a = two_pieces_app();
    let ctx = Context::default();
    ready(&mut a, &ctx, Mode::Polygon);
    let gap = empty(&a);
    let piece_b = on(&a, DVec3::new(1.5, 0., -0.5));
    let select_first = |a: &mut Operandi| {
        a.editor.scene.selection.faces = vec![0];
    };

    // A slow second Shift press is two Shift+clicks on nothing.
    select_first(&mut a);
    click(&mut a, &ctx, gap, Modifiers::SHIFT);
    pause(&mut a, &ctx);
    click(&mut a, &ctx, gap, Modifiers::SHIFT);
    assert_eq!(a.editor.scene.selection.faces, vec![0]);

    // Ctrl is no expand.
    pause(&mut a, &ctx);
    double_click(&mut a, &ctx, gap, Modifiers::SHIFT | Modifiers::CTRL);
    assert_eq!(a.editor.scene.selection.faces, vec![0]);

    // Shift on nothing, then Shift on a polygon: that press adds the
    // polygon, as any Shift+click does.
    pause(&mut a, &ctx);
    click(&mut a, &ctx, gap, Modifiers::SHIFT);
    click(&mut a, &ctx, piece_b, Modifiers::SHIFT);
    assert_eq!(a.editor.scene.selection.faces, vec![0, 4], "{}", a.status);

    // Shift+double-click on a polygon still adds that polygon's piece.
    pause(&mut a, &ctx);
    select_first(&mut a);
    double_click(&mut a, &ctx, piece_b, Modifiers::SHIFT);
    assert_eq!(
        sorted(&a.editor.scene.selection.faces),
        vec![0, 4, 5, 6, 7],
        "{}",
        a.status
    );

    // A Shift press that became a drag is no first press.
    pause(&mut a, &ctx);
    select_first(&mut a);
    let away = gap + vec2(0., 40.);
    draw(
        &mut a,
        &ctx,
        vec2(1440., 900.),
        vec![
            pointer(gap, true, Modifiers::SHIFT),
            Event::PointerMoved(away),
            Event::PointerMoved(gap),
            pointer(gap, false, Modifiers::SHIFT),
        ],
    );
    click(&mut a, &ctx, gap, Modifiers::SHIFT);
    assert_eq!(a.editor.scene.selection.faces, vec![0], "{}", a.status);

    // A plain double-click on nothing clears the selection, as before.
    pause(&mut a, &ctx);
    double_click(&mut a, &ctx, gap, Modifiers::NONE);
    assert!(a.editor.scene.selection.faces.is_empty(), "{}", a.status);
}
