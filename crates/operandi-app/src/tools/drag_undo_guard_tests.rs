//! One drag, one undo step (Jason, 2026-11-04 and 2026-11-14): press, drag
//! and release in the viewport is a single step, however many frames the
//! drag sends. This drags every tool with real pointer events (on empty
//! space, on the mesh and on each of its handles) and fails if any drag
//! records more than one step, in the document history or as tweaks of
//! the live tool.
use super::tests::draw;
use super::*;

const SIZE: Vec2 = vec2(1440., 900.);

/// 6 × 6 quads in z = 0, centred on the origin, 0.5 m apart.
fn grid() -> operandi_core::Mesh {
    let n = 6u32;
    let mut m = operandi_core::Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            m.vertices
                .push(DVec3::new((i as f64 - 3.) * 0.5, (j as f64 - 3.) * 0.5, 0.));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            m.faces.push(operandi_core::Face {
                vertices: vec![a, a + 1, a + n + 2, a + n + 1],
                uv: None,
                material: "Default".into(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    m
}

/// The mode a tool is used in; the speed guard's choice where it has one.
fn mode_for(tool: Tool) -> Mode {
    match tool {
        Tool::EdgeExtend
        | Tool::EdgeExtrude
        | Tool::EdgeBevel
        | Tool::EdgeChamfer
        | Tool::EdgeSlide
        | Tool::EdgeCrease
        | Tool::EdgeRelax
        | Tool::LoopSlice => Mode::Edge,
        Tool::VertexBevel
        | Tool::VertexExtrude
        | Tool::VertexMerge
        | Tool::VertexTear
        | Tool::VertexSlide
        | Tool::VertexCreate
        | Tool::Smooth => Mode::Vertex,
        _ => Mode::Polygon,
    }
}

fn app_with(tool: Tool) -> (Operandi, Context) {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh = grid().into();
    a.camera.view = View::Front;
    let mode = mode_for(tool);
    a.run(Command::Mode(mode));
    // The middle polygon (row 2, column 2 .. 3) and points/edges on it.
    let selection = &mut a.editor.scene.selection;
    match mode {
        Mode::Edge if tool == Tool::EdgeExtend => selection.edges = vec![(2, 3)],
        Mode::Edge => selection.edges = vec![(17, 18)],
        Mode::Vertex => selection.vertices = vec![17, 24],
        _ => selection.faces = vec![14, 15, 20, 21],
    }
    draw(&mut a, &ctx, SIZE, vec![]);
    a.activate(tool);
    draw(&mut a, &ctx, SIZE, vec![]);
    (a, ctx)
}

/// Steps the document history and the live tool's tweaks hold now.
fn steps(a: &Operandi) -> usize {
    a.editor.history.undo.len() + a.editor.session.tweaks.undo_steps()
}

/// Press at `from`, move ten times, release; the steps it recorded.
fn drag(a: &mut Operandi, ctx: &Context, from: Pos2, by: Vec2, button: PointerButton) -> usize {
    let before = steps(a);
    let press = |pos, pressed| Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    };
    draw(a, ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(a, ctx, SIZE, vec![press(from, true)]);
    for k in 1..=10 {
        draw(
            a,
            ctx,
            SIZE,
            vec![Event::PointerMoved(from + by * (k as f32 / 10.))],
        );
    }
    draw(a, ctx, SIZE, vec![press(from + by, false)]);
    steps(a).saturating_sub(before)
}

fn world_to_screen(a: &Operandi, p: DVec3) -> Pos2 {
    a.camera.project(p, a.viewport_rect).unwrap().0
}

#[test]
fn every_tool_drag_records_at_most_one_undo_step() {
    tool_drags_record_at_most_one_undo_step(0);
}
// The tools are split four ways so the parts run side by side (test
// speed, 2026-11-16): one part alone took ~27 s.
#[test]
fn every_tool_drag_records_at_most_one_undo_step_part_2() {
    tool_drags_record_at_most_one_undo_step(1);
}
#[test]
fn every_tool_drag_records_at_most_one_undo_step_part_3() {
    tool_drags_record_at_most_one_undo_step(2);
}
#[test]
fn every_tool_drag_records_at_most_one_undo_step_part_4() {
    tool_drags_record_at_most_one_undo_step(3);
}

/// Every fourth tool, starting at `part`.
fn tool_drags_record_at_most_one_undo_step(part: usize) {
    let mut failures = Vec::new();
    for tool in Tool::ALL.iter().copied().skip(part).step_by(4) {
        let (mut a, ctx) = app_with(tool);
        if a.editor.session.live.as_ref().map(|l| l.tool) != Some(tool) {
            // Tools that need a setup this grid doesn't give (a bevel's
            // rows, a sketch) are covered by their own tests.
            continue;
        }
        let r = a.viewport_rect;
        let mut presses = vec![
            ("empty space", r.left_top() + vec2(120., 120.)),
            ("the mesh", world_to_screen(&a, DVec3::new(0.25, 0.25, 0.))),
        ];
        for (id, _, tip, _) in a.handles(r) {
            presses.push((if id == 0 { "handle 0" } else { "a handle" }, tip));
        }
        for (what, from) in presses {
            for button in [PointerButton::Primary, PointerButton::Secondary] {
                let n = drag(&mut a, &ctx, from, vec2(90., -60.), button);
                if n > 1 {
                    failures.push(format!("{tool:?}: {button:?} drag on {what}: {n} steps"));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "drags that record more than one undo step:\n{}",
        failures.join("\n")
    );
}

/// The case Jason hit: drawing a Box (and its side handles after) is one
/// step per drag, and Undo takes the whole drag back at once.
#[test]
fn drawing_a_box_is_one_step_and_one_undo_takes_it_back() {
    let (mut a, ctx) = app_with(Tool::Box);
    let from = a.viewport_rect.left_top() + vec2(120., 120.);
    let before = a.editor.scene.clone();
    assert_eq!(
        drag(&mut a, &ctx, from, vec2(200., 150.), PointerButton::Primary),
        1
    );
    assert_ne!(a.editor.scene, before, "the box was drawn ({})", a.status);
    a.run(Command::Undo);
    assert!(
        a.editor.scene == before,
        "one Undo removes the drawn box ({})",
        a.status
    );
}
