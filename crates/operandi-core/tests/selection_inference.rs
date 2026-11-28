//! Selection inference (specifications/selection-inference-2026-11-15,
//! v1.1): point and edge tools run from another selection mode on the
//! elements worked out from the current selection.
//!
//! Acceptance criteria:
//! 1. M01/M02: Vertex Bevel from polygons or edges gives the same mesh as
//!    on their points in Vertex mode.
//! 2. M03/SI-05/06/09: the mode stays, the selection you made stays, the
//!    tool's result points are the point selection; one undo step that
//!    restores the scene.
//! 3. SI-12: nothing selected behaves as the tool's own mode with nothing
//!    selected (Vertex Bevel refuses; Edge Bevel takes every edge).
//! 4. M06: an edge tool from points takes the edges between them.
//! 5. SI-02/SI-15: an edge tool from polygons takes their outline; a whole
//!    closed mesh (no outline) takes every edge and says so.
//! 6. SI-03: polygon tools refuse Vertex and Edge mode with a reason; tools
//!    marked "No" (Tear, Create) still refuse as before.
//! 7. SI-10: every active mesh converts its own selection.
//! 8. M05: Set Position in Edge mode with nothing selected moves every point.
//! 9. SI §7: every card that takes points or edges starts in every
//!    component mode.
use glam::DVec3;
use operandi_core::tool_card::{Takes, cards};
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}
/// 3×3 quads in z = 0; point `r * 4 + c` at (c, r, 0); polygon 4 is the
/// middle one, with corners 5, 6, 10, 9.
fn grid() -> Mesh {
    let mut m = Mesh {
        vertices: (0..16)
            .map(|i| DVec3::new((i % 4) as f64, (i / 4) as f64, 0.))
            .collect(),
        ..Default::default()
    };
    for r in 0..3 {
        for c in 0..3 {
            let i = r * 4 + c;
            m.faces.push(face(vec![i, i + 1, i + 5, i + 4]));
        }
    }
    m
}
fn cube() -> Mesh {
    Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap()
}
fn editor(m: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = m.into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = mode;
    e
}
fn mesh(e: &Editor) -> Mesh {
    Mesh::clone(&e.scene.active().unwrap().mesh)
}
/// Starts `tool`, drags it by `amount` (inset or offset) and drops it.
fn run(e: &mut Editor, tool: Tool, amount: f64) {
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.inset = amount;
    p.chamfer.offset = amount;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
}
fn vertex_bevel_on(points: Vec<u32>) -> Mesh {
    let mut e = editor(grid(), Mode::Vertex);
    e.scene.selection.vertices = points;
    run(&mut e, Tool::VertexBevel, 0.2);
    mesh(&e)
}
fn edge_bevel_on(m: Mesh, edges: Vec<(u32, u32)>) -> Mesh {
    let mut e = editor(m, Mode::Edge);
    e.scene.selection.edges = edges;
    run(&mut e, Tool::EdgeBevel, 0.1);
    mesh(&e)
}

#[test]
fn m01_vertex_bevel_from_polygons_matches_their_points() {
    let mut e = editor(grid(), Mode::Polygon);
    e.scene.selection.faces = vec![4];
    let before = e.scene.clone();
    e.dispatch(Command::Activate(Tool::VertexBevel)).unwrap();
    assert_eq!(e.scene.selection.mode, Mode::Polygon, "the mode stays");
    assert_eq!(
        e.inference_note(),
        Some("Vertex Bevel on 4 points from 1 polygon")
    );
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.inset = 0.2;
    e.dispatch(Command::Preview(p)).unwrap();
    assert_eq!(e.scene.selection.mode, Mode::Polygon);
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(mesh(&e), vertex_bevel_on(vec![5, 6, 10, 9]));
    // M03: still Polygon mode with polygon 4 selected; the cut points are
    // the point selection.
    assert_eq!(e.scene.selection.mode, Mode::Polygon);
    assert_eq!(e.scene.selection.faces, [4]);
    let centre = mesh(&e).center(4);
    assert!(
        centre.distance(DVec3::new(1.5, 1.5, 0.)) < 1e-9,
        "the same polygon"
    );
    let mut native = editor(grid(), Mode::Vertex);
    native.scene.selection.vertices = vec![5, 6, 10, 9];
    run(&mut native, Tool::VertexBevel, 0.2);
    assert_eq!(e.scene.selection.vertices, native.scene.selection.vertices);
    assert!(
        e.inference_note().is_none(),
        "nothing once the tool is gone"
    );
    // One undo step back to the scene as it was.
    assert_eq!(e.history.undo.last().unwrap().label, "Vertex Bevel");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e), mesh_of(&before));
    assert_eq!(e.scene.selection.mode, Mode::Polygon);
    assert_eq!(e.scene.selection.faces, [4]);
}
fn mesh_of(s: &Scene) -> Mesh {
    Mesh::clone(&s.active().unwrap().mesh)
}

#[test]
fn m02_vertex_bevel_from_edges_matches_their_points() {
    let mut e = editor(grid(), Mode::Edge);
    e.scene.selection.edges = vec![(5, 6), (9, 10)];
    run(&mut e, Tool::VertexBevel, 0.2);
    assert_eq!(mesh(&e), vertex_bevel_on(vec![5, 6, 9, 10]));
    assert_eq!(e.scene.selection.mode, Mode::Edge);
}

#[test]
fn nothing_selected_behaves_as_the_tools_own_mode() {
    // Vertex Bevel refuses an empty point selection in Vertex mode, so it
    // refuses an empty polygon selection too, changing nothing.
    let mut e = editor(grid(), Mode::Polygon);
    let before = e.scene.clone();
    assert!(e.dispatch(Command::Activate(Tool::VertexBevel)).is_err());
    assert!(e.session.live.is_none());
    assert_eq!(e.scene, before);
    // Edge Bevel with nothing selected bevels every edge (TA-06), from
    // Polygon and Vertex mode alike.
    let all = edge_bevel_on(cube(), Vec::new());
    for mode in [Mode::Polygon, Mode::Vertex] {
        let mut e = editor(cube(), mode);
        run(&mut e, Tool::EdgeBevel, 0.1);
        assert_eq!(mesh(&e), all, "{mode:?}");
        assert_eq!(e.scene.selection.mode, mode);
    }
}

#[test]
fn m06_edge_bevel_from_points_takes_the_edge_between_them() {
    let c = cube();
    let edge = c.edges()[0];
    let mut e = editor(c.clone(), Mode::Vertex);
    e.scene.selection.vertices = vec![edge.0, edge.1];
    run(&mut e, Tool::EdgeBevel, 0.1);
    assert_eq!(mesh(&e), edge_bevel_on(c, vec![edge]));
    assert_eq!(e.scene.selection.mode, Mode::Vertex);
    assert_eq!(e.scene.selection.vertices, [edge.0, edge.1]);
}

#[test]
fn edge_tools_from_polygons_take_the_outline() {
    // One cube face: its four edges.
    let c = cube();
    let top = (0..c.faces.len()).find(|&f| c.normal(f).y > 0.9).unwrap() as u32;
    let outline = selection_inference::outline_edges(&c, &[top]);
    assert_eq!(outline.len(), 4);
    let mut e = editor(c.clone(), Mode::Polygon);
    e.scene.selection.faces = vec![top];
    e.dispatch(Command::Activate(Tool::EdgeBevel)).unwrap();
    assert_eq!(e.scene.selection.edges, outline);
    e.dispatch(Command::Cancel).unwrap();
    run(&mut e, Tool::EdgeBevel, 0.1);
    assert_eq!(mesh(&e), edge_bevel_on(c.clone(), outline));
    assert_eq!(
        e.scene.selection.faces.len(),
        1,
        "the polygon stays selected"
    );
    // Two neighbouring polygons on the grid: the shared edge is left out.
    let g = grid();
    let two = selection_inference::outline_edges(&g, &[3, 4]);
    assert_eq!(two.len(), 6);
    assert!(!two.contains(&(5, 9)));
}

#[test]
fn a_whole_closed_mesh_has_no_outline_and_takes_every_edge() {
    let c = cube();
    let mut e = editor(c.clone(), Mode::Polygon);
    e.scene.selection.faces = (0..6).collect();
    e.dispatch(Command::Activate(Tool::EdgeBevel)).unwrap();
    assert_eq!(e.scene.selection.edges.len(), 12);
    assert_eq!(
        e.inference_note(),
        Some("Edge Bevel on all 12 edges of 6 polygons (no outline)")
    );
    e.dispatch(Command::Cancel).unwrap();
    run(&mut e, Tool::EdgeBevel, 0.1);
    assert_eq!(mesh(&e), edge_bevel_on(c.clone(), c.edges()));
}

#[test]
fn polygon_tools_and_no_tools_still_refuse_with_a_reason() {
    for mode in [Mode::Vertex, Mode::Edge] {
        let mut e = editor(cube(), mode);
        e.scene.selection.vertices = vec![0, 1, 2, 3];
        e.scene.selection.edges = cube().edges()[..2].to_vec();
        let before = e.scene.clone();
        for tool in [Tool::Bevel, Tool::Inset, Tool::Chamfer] {
            let err = e.dispatch(Command::Activate(tool)).unwrap_err();
            assert_eq!(
                err.to_string(),
                format!(
                    "{} needs polygons: switch to Polygon mode, or Convert the selection",
                    tool.label()
                )
            );
            assert_eq!(e.scene, before);
        }
    }
    // Tear and Create don't infer: they refuse Polygon mode as before.
    let mut e = editor(grid(), Mode::Polygon);
    e.scene.selection.faces = vec![4];
    assert_eq!(
        e.dispatch(Command::Activate(Tool::VertexTear))
            .unwrap_err()
            .to_string(),
        "Tear needs selected points in Vertex mode"
    );
    assert!(e.dispatch(Command::Activate(Tool::VertexCreate)).is_err());
}

#[test]
fn every_active_mesh_converts_its_own_selection() {
    let mut e = Editor::default();
    let mut ids = Vec::new();
    for _ in 0..2 {
        e.scene.new_mesh().unwrap();
        e.scene.active_mut().unwrap().mesh = grid().into();
        ids.push(e.scene.active().unwrap().id);
    }
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.dispatch(Command::SelectItems(ids.clone())).unwrap();
    for &id in &ids {
        e.dispatch(Command::Select {
            item: id,
            hit: Component::Polygon(4),
            replace: false,
            remove: false,
        })
        .unwrap();
    }
    let steps = e.history.undo.len();
    run(&mut e, Tool::VertexBevel, 0.2);
    let want = vertex_bevel_on(vec![5, 6, 10, 9]);
    for id in ids {
        let item = e.scene.items.iter().find(|i| i.id == id).unwrap();
        assert_eq!(Mesh::clone(&item.mesh), want);
    }
    assert_eq!(e.scene.selection.mode, Mode::Polygon);
    assert_eq!(e.history.undo.len(), steps + 1, "one undo step");
}

#[test]
fn m05_set_position_in_edge_mode_with_nothing_selected_moves_every_point() {
    let mut e = editor(grid(), Mode::Edge);
    e.dispatch(Command::SetPosition(PositionAssignment {
        space: PositionSpace::Local,
        axis: PositionAxis::Z,
        value: 0.5,
        expected: None,
    }))
    .unwrap();
    assert!(mesh(&e).vertices.iter().all(|v| v.z == 0.5));
    // One edge: its two points only.
    let mut e = editor(grid(), Mode::Edge);
    e.scene.selection.edges = vec![(5, 6)];
    e.dispatch(Command::SetPosition(PositionAssignment {
        space: PositionSpace::Local,
        axis: PositionAxis::Z,
        value: 0.5,
        expected: None,
    }))
    .unwrap();
    let moved: Vec<usize> = (0..16).filter(|&v| mesh(&e).vertices[v].z == 0.5).collect();
    assert_eq!(moved, [5, 6]);
}

#[test]
fn every_inferring_card_starts_in_every_component_mode() {
    for card in cards() {
        match card.takes {
            Takes::Points | Takes::Edges => {
                for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon] {
                    assert!(card.modes.contains(mode), "{:?} {mode:?}", card.tool);
                }
            }
            Takes::Polygons => assert!(!card.modes.contains(Mode::Vertex), "{:?}", card.tool),
            Takes::Native => {}
        }
    }
    use Tool::*;
    let points = [VertexBevel, VertexExtrude, VertexMerge, VertexSlide];
    let edges = [
        EdgeBevel,
        EdgeChamfer,
        EdgeExtrude,
        EdgeExtend,
        EdgeSlide,
        EdgeCrease,
        EdgeRelax,
    ];
    for t in points {
        assert_eq!(t.card().takes, Takes::Points, "{t:?}");
    }
    for t in edges {
        assert_eq!(t.card().takes, Takes::Edges, "{t:?}");
    }
    for t in [VertexTear, VertexCreate] {
        assert_eq!(t.card().takes, Takes::Native, "{t:?}");
    }
}

#[test]
fn every_inferring_tool_runs_or_refuses_cleanly_from_each_mode() {
    // The middle polygon of the grid, its outline, or its corner points:
    // each point and edge tool either starts (and leaves the mode as it
    // was, with the selection made) or refuses with the scene unchanged.
    let mut started = 0;
    for card in cards().filter(|c| matches!(c.takes, Takes::Points | Takes::Edges)) {
        for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon] {
            let mut e = editor(grid(), mode);
            e.scene.selection.vertices = vec![5, 6, 10, 9];
            e.scene.selection.edges = vec![(5, 6), (6, 10), (9, 10), (5, 9)];
            e.scene.selection.faces = vec![4];
            let before = e.scene.clone();
            match e.dispatch(Command::Activate(card.tool)) {
                Ok(()) => {
                    started += 1;
                    let mut p = e.session.live.as_ref().unwrap().params.clone();
                    p.inset = 0.1;
                    p.shift = 0.1;
                    p.chamfer.offset = 0.1;
                    p.delta = DVec3::new(0.1, 0., 0.);
                    let _ = e.dispatch(Command::Preview(p));
                    assert_eq!(e.scene.selection.mode, mode, "{:?}", card.tool);
                    e.dispatch(Command::Drop).unwrap();
                    assert_eq!(e.scene.selection.mode, mode, "{:?}", card.tool);
                    e.scene.validate().unwrap();
                }
                Err(_) => assert_eq!(e.scene, before, "{:?} from {mode:?}", card.tool),
            }
        }
    }
    assert!(started >= 20, "only {started} starts");
}

fn command_on(e: &mut Editor, cmd: Command) -> Mesh {
    e.dispatch(cmd).unwrap();
    mesh(e)
}
fn join(average: bool) -> Command {
    Command::VertexCommand(VertexCommand::Join {
        average,
        options: Default::default(),
    })
}

#[test]
fn m10_join_from_edges_and_polygons_matches_their_points() {
    for average in [true, false] {
        // Plain Join lands on the last point in selection order: the
        // second point of the last edge (SI-Q04).
        let mut native = editor(grid(), Mode::Vertex);
        native.scene.selection.vertices = vec![5, 6, 9, 10];
        let want = command_on(&mut native, join(average));
        let mut e = editor(grid(), Mode::Edge);
        e.scene.selection.edges = vec![(5, 6), (9, 10)];
        let before = e.scene.clone();
        assert_eq!(command_on(&mut e, join(average)), want, "average {average}");
        assert_eq!(e.scene.selection.mode, Mode::Edge);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, before, "undo restores the scene and selection");
    }
    let mut native = editor(grid(), Mode::Vertex);
    native.scene.selection.vertices = vec![5, 6, 10, 9];
    let want = command_on(&mut native, join(true));
    let mut e = editor(grid(), Mode::Polygon);
    e.scene.selection.faces = vec![4];
    assert_eq!(command_on(&mut e, join(true)), want);
    assert_eq!(e.scene.selection.mode, Mode::Polygon);
}

#[test]
fn align_and_edge_commands_infer_but_mode_aware_commands_dont() {
    let align = Command::VertexCommand(VertexCommand::Align {
        axes: [false, true, false],
        uniform: false,
    });
    let mut native = editor(grid(), Mode::Vertex);
    native.scene.selection.vertices = vec![5, 6, 10, 9];
    let want = command_on(&mut native, align.clone());
    let mut e = editor(grid(), Mode::Polygon);
    e.scene.selection.faces = vec![4];
    assert_eq!(command_on(&mut e, align), want);
    // Edge Split and Split Normals from a polygon: its outline.
    let c = cube();
    let top = (0..c.faces.len()).find(|&f| c.normal(f).y > 0.9).unwrap() as u32;
    for command in [EdgeCommand::Split, EdgeCommand::SplitNormals] {
        let mut native = editor(c.clone(), Mode::Edge);
        native.scene.selection.edges = selection_inference::outline_edges(&c, &[top]);
        let want = native
            .dispatch(Command::EdgeCommand(command))
            .map(|()| mesh(&native));
        let mut e = editor(c.clone(), Mode::Polygon);
        e.scene.selection.faces = vec![top];
        let got = e.dispatch(Command::EdgeCommand(command)).map(|()| mesh(&e));
        // Split Normals needs a normal map: both refuse alike.
        assert_eq!(got, want, "{command:?}");
        assert_eq!(e.scene.selection.mode, Mode::Polygon);
        assert_eq!(e.scene.selection.faces, [top]);
    }
    // Remove in Polygon mode still removes polygons, not their points.
    let mut e = editor(grid(), Mode::Polygon);
    e.scene.selection.faces = vec![4];
    e.dispatch(Command::VertexCommand(VertexCommand::Remove))
        .unwrap_err();
    assert_eq!(mesh(&e), grid());
}
