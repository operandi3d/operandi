//! Multiple active meshes, stage 3 (multi_tools.rs): mesh-editing commands
//! and tools act on every active mesh with its own component selection, as
//! one undo step.
use glam::DVec3;
use operandi_core::*;

/// Adds a 2 m cube as a new mesh centred at `x`; returns its id.
fn add_cube(e: &mut Editor, x: f64) -> u64 {
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        position: DVec3::new(x, 0., 0.),
        size: DVec3::splat(2.),
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.scene.active().unwrap().id
}
/// Cubes at x = 0 (`a`, primary) and x = 6 (`b`), both selected, in
/// `mode` with nothing selected.
fn two_in(mode: Mode) -> (Editor, u64, u64) {
    let mut e = Editor::default();
    let a = add_cube(&mut e, 0.);
    let b = add_cube(&mut e, 6.);
    e.dispatch(Command::Mode(mode)).unwrap();
    e.dispatch(Command::SelectItems(vec![a, b])).unwrap();
    if mode != Mode::Item {
        e.dispatch(Command::ClearSelection).unwrap();
    }
    (e, a, b)
}
fn two() -> (Editor, u64, u64) {
    two_in(Mode::Polygon)
}
fn pick(e: &mut Editor, item: u64, hit: Component) {
    e.dispatch(Command::Select {
        item,
        hit,
        replace: false,
        remove: false,
    })
    .unwrap();
}
fn mesh(e: &Editor, id: u64) -> Mesh {
    (*e.scene.items.iter().find(|i| i.id == id).unwrap().mesh).clone()
}
/// The polygon of `id` facing +Y (the top).
fn top(e: &Editor, id: u64) -> u32 {
    let m = mesh(e, id);
    (0..m.faces.len()).find(|&f| m.normal(f).y > 0.9).unwrap() as u32
}
/// Both tops selected.
fn tops() -> (Editor, u64, u64) {
    let (mut e, a, b) = two();
    let (ta, tb) = (top(&e, a), top(&e, b));
    pick(&mut e, a, Component::Polygon(ta));
    pick(&mut e, b, Component::Polygon(tb));
    (e, a, b)
}
fn faces_of(e: &Editor, id: u64) -> Vec<u32> {
    e.scene.selection.faces_of(id).to_vec()
}
fn highest(m: &Mesh) -> f64 {
    m.vertices.iter().map(|v| v.y).fold(f64::MIN, f64::max)
}

// ---- Batch A: one-shot commands ----

#[test]
fn flip_flips_every_mesh_selection_in_one_step() {
    let (mut e, a, b) = tops();
    let (a0, b0) = (mesh(&e, a), mesh(&e, b));
    let (ta, tb) = (top(&e, a), top(&e, b));
    let steps = e.history.undo.len();
    e.dispatch(Command::FlipPolygons).unwrap();
    assert_eq!(e.history.undo.len(), steps + 1, "one undo step");
    assert!(mesh(&e, a).normal(ta as usize).y < -0.9);
    assert!(mesh(&e, b).normal(tb as usize).y < -0.9);
    // Each mesh keeps its own selection.
    assert_eq!(faces_of(&e, a), vec![ta]);
    assert_eq!(faces_of(&e, b), vec![tb]);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e, a), a0);
    assert_eq!(mesh(&e, b), b0);
    assert_eq!(faces_of(&e, b), vec![tb]);
    e.dispatch(Command::Redo).unwrap();
    assert!(mesh(&e, b).normal(tb as usize).y < -0.9);
}

#[test]
fn detriangulate_runs_on_every_mesh_in_one_step() {
    // Duplicate tab's one-shot command joins Batch A (MAM-Q17 note).
    let (mut e, a, b) = tops();
    e.dispatch(Command::Triple).unwrap();
    assert_eq!(mesh(&e, a).faces.len(), 7);
    assert_eq!(mesh(&e, b).faces.len(), 7);
    assert_eq!(faces_of(&e, b).len(), 2, "b's triangles stay selected");
    let steps = e.history.undo.len();
    e.dispatch(Command::Detriangulate(DetriangulateOptions::default()))
        .unwrap();
    assert_eq!(e.history.undo.len(), steps + 1, "one undo step");
    assert_eq!(mesh(&e, a).faces.len(), 6);
    assert_eq!(mesh(&e, b).faces.len(), 6);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e, a).faces.len(), 7);
    assert_eq!(mesh(&e, b).faces.len(), 7);
}

#[test]
fn a_mesh_with_nothing_selected_is_left_alone() {
    let (mut e, a, b) = two();
    // Only b (not the primary) has a polygon selected.
    let tb = top(&e, b);
    pick(&mut e, b, Component::Polygon(tb));
    let a0 = mesh(&e, a);
    e.dispatch(Command::RemovePolygons).unwrap();
    assert_eq!(mesh(&e, a), a0, "primary untouched");
    assert_eq!(mesh(&e, b).faces.len(), 5);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e, b).faces.len(), 6);
    assert_eq!(faces_of(&e, b), vec![tb]);
}

#[test]
fn nothing_selected_anywhere_edits_every_mesh_whole() {
    let (mut e, a, b) = two();
    let (a0, b0) = (mesh(&e, a), mesh(&e, b));
    // Flip with nothing selected flips every polygon (FL05).
    e.dispatch(Command::FlipPolygons).unwrap();
    for id in [a, b] {
        let m = mesh(&e, id);
        let t = top(&e, id) as usize;
        assert!(m.normal(t).y > 0.9, "the face now facing up was the bottom");
        assert_ne!(
            m.faces[t].vertices,
            if id == a { &a0 } else { &b0 }.faces[t].vertices
        );
    }
    e.dispatch(Command::Undo).unwrap();
    assert_eq!((mesh(&e, a), mesh(&e, b)), (a0, b0));
}

#[test]
fn item_mode_commands_run_on_every_selected_mesh() {
    let (mut e, a, b) = two_in(Mode::Item);
    e.dispatch(Command::SubdividePolygons {
        options: SubdivideOptions::default(),
        times: 1,
    })
    .unwrap();
    assert_eq!(mesh(&e, a).faces.len(), 24);
    assert_eq!(mesh(&e, b).faces.len(), 24);
}

#[test]
fn results_are_kept_per_mesh_through_topology_changes() {
    let (mut e, a, b) = tops();
    e.dispatch(Command::SubdividePolygons {
        options: SubdivideOptions::default(),
        times: 1,
    })
    .unwrap();
    // Each top became four polygons, and each mesh has its four selected.
    assert_eq!(faces_of(&e, a).len(), 4, "{:?}", faces_of(&e, a));
    assert_eq!(faces_of(&e, b).len(), 4, "{:?}", faces_of(&e, b));
    assert_eq!(mesh(&e, b).faces.len(), 9);
}

#[test]
fn edge_and_vertex_commands_run_on_every_mesh() {
    let (mut e, a, b) = two_in(Mode::Vertex);
    pick(&mut e, a, Component::Vertex(0));
    pick(&mut e, b, Component::Vertex(0));
    e.dispatch(Command::VertexCommand(VertexCommand::Remove))
        .unwrap();
    assert_eq!(mesh(&e, a).vertices.len(), 7);
    assert_eq!(mesh(&e, b).vertices.len(), 7);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e, a).vertices.len(), 8);
    assert_eq!(mesh(&e, b).vertices.len(), 8);
}

#[test]
fn a_failure_on_one_mesh_changes_nothing() {
    let (mut e, a, b) = two();
    // Merge needs two touching polygons: a has them, b only one.
    let m = mesh(&e, a);
    let ta = top(&e, a);
    let side = (0..m.faces.len() as u32)
        .find(|&f| {
            f != ta
                && m.faces[f as usize]
                    .vertices
                    .iter()
                    .filter(|v| m.faces[ta as usize].vertices.contains(v))
                    .count()
                    == 2
        })
        .unwrap();
    pick(&mut e, a, Component::Polygon(ta));
    pick(&mut e, a, Component::Polygon(side));
    let tb = top(&e, b);
    pick(&mut e, b, Component::Polygon(tb));
    let (a0, b0) = (mesh(&e, a), mesh(&e, b));
    let steps = e.history.undo.len();
    let error = e.dispatch(Command::MergePolygons).unwrap_err();
    assert!(error.contains(&item_name(&e, b)), "{error}");
    assert_eq!((mesh(&e, a), mesh(&e, b)), (a0, b0));
    assert_eq!(e.history.undo.len(), steps);
}
fn item_name(e: &Editor, id: u64) -> String {
    e.scene
        .items
        .iter()
        .find(|i| i.id == id)
        .unwrap()
        .name
        .clone()
}

#[test]
fn several_mesh_edits_save_and_reopen() {
    let (mut e, a, b) = tops();
    e.dispatch(Command::FlipPolygons).unwrap();
    let path = std::env::temp_dir().join(format!(
        "operandi-multi-tools-{}.opscene",
        std::process::id()
    ));
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    for id in [a, b] {
        let m = &back.items.iter().find(|i| i.id == id).unwrap().mesh;
        assert_eq!(**m, mesh(&e, id));
    }
    assert_eq!(back.selection.faces_of(b), e.scene.selection.faces_of(b));
}

// ---- Batch B: interactive polygon tools ----

/// Activate `tool`, preview with `edit` applied to its parameters, Drop.
fn run_tool(e: &mut Editor, tool: Tool, edit: impl FnOnce(&mut Parameters)) {
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    edit(&mut p);
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
}

#[test]
fn bevel_has_one_handle_and_bevels_every_mesh_in_one_step() {
    let (mut e, a, b) = tops();
    let (a0, b0) = (mesh(&e, a), mesh(&e, b));
    let steps = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let frame = e.session.live.as_ref().unwrap().action_frame.unwrap();
    assert!(
        frame.center.distance(DVec3::new(3., 1., 0.)) < 1e-9,
        "one handle at the combined centre: {:?}",
        frame.center
    );
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.shift = 0.5;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), steps + 1, "one undo step");
    for id in [a, b] {
        assert_eq!(mesh(&e, id).faces.len(), 10);
        assert!((highest(&mesh(&e, id)) - 1.5).abs() < 1e-9);
        assert_eq!(faces_of(&e, id).len(), 1, "the new top stays selected");
    }
    e.dispatch(Command::Undo).unwrap();
    assert_eq!((mesh(&e, a), mesh(&e, b)), (a0, b0));
}

#[test]
fn cancel_restores_every_mesh() {
    let (mut e, a, b) = tops();
    let (a0, b0) = (mesh(&e, a), mesh(&e, b));
    e.dispatch(Command::Activate(Tool::Inset)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.inset = 0.2;
    e.dispatch(Command::Preview(p)).unwrap();
    assert_ne!(mesh(&e, b), b0);
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!((mesh(&e, a), mesh(&e, b)), (a0, b0));
}

#[test]
fn amounts_are_world_distances_on_scaled_items() {
    let (mut e, a, b) = tops();
    // b is twice as big in its item transform: its top is at y = 2.
    e.scene
        .items
        .iter_mut()
        .find(|i| i.id == b)
        .unwrap()
        .transform
        .scale = DVec3::splat(2.);
    run_tool(&mut e, Tool::Shift, |p| p.shift = 0.5);
    let world_top = |e: &Editor, id: u64| {
        let w = e.scene.world_matrix(id).unwrap();
        mesh(e, id)
            .vertices
            .iter()
            .map(|&v| w.transform_point3(v).y)
            .fold(f64::MIN, f64::max)
    };
    assert!(
        (world_top(&e, a) - 1.5).abs() < 1e-9,
        "{}",
        world_top(&e, a)
    );
    assert!(
        (world_top(&e, b) - 2.5).abs() < 1e-9,
        "{}",
        world_top(&e, b)
    );
}

#[test]
fn tool_with_nothing_selected_on_the_primary_still_runs_on_the_others() {
    let (mut e, a, b) = two();
    let tb = top(&e, b);
    pick(&mut e, b, Component::Polygon(tb));
    let a0 = mesh(&e, a);
    run_tool(&mut e, Tool::Bevel, |p| p.shift = 0.25);
    assert_eq!(mesh(&e, a), a0);
    assert!((highest(&mesh(&e, b)) - 1.25).abs() < 1e-9);
}

#[test]
fn extrude_runs_on_every_mesh_and_selects_each_mesh_caps() {
    let (mut e, a, b) = tops();
    let (a0, b0) = (mesh(&e, a), mesh(&e, b));
    let steps = e.history.undo.len();
    run_tool(&mut e, Tool::Extrude, |p| p.extent = DVec3::Y);
    assert_eq!(e.history.undo.len(), steps + 1, "one undo step");
    for id in [a, b] {
        assert!((highest(&mesh(&e, id)) - 2.).abs() < 1e-9);
        assert_eq!(mesh(&e, id).faces.len(), 10);
        assert!(!faces_of(&e, id).is_empty(), "each mesh keeps its caps");
    }
    e.dispatch(Command::Undo).unwrap();
    assert_eq!((mesh(&e, a), mesh(&e, b)), (a0, b0));
    e.scene.validate().unwrap();
}

#[test]
fn bevel_continue_works_on_every_mesh() {
    let (mut e, a, b) = tops();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.shift = 0.5;
    e.dispatch(Command::Preview(p.clone())).unwrap();
    e.dispatch(Command::Continue).unwrap();
    p.shift = 0.5;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    for id in [a, b] {
        assert!((highest(&mesh(&e, id)) - 2.).abs() < 1e-9, "{id}");
        assert_eq!(mesh(&e, id).faces.len(), 14);
    }
}

#[test]
fn edge_and_vertex_tools_run_on_every_mesh() {
    let (mut e, a, b) = two_in(Mode::Edge);
    let edge = |e: &Editor, id: u64| {
        let m = mesh(e, id);
        let f = &m.faces[top(e, id) as usize].vertices;
        (f[0].min(f[1]), f[0].max(f[1]))
    };
    let (ea, eb) = (edge(&e, a), edge(&e, b));
    pick(&mut e, a, Component::Edge(ea.0, ea.1));
    pick(&mut e, b, Component::Edge(eb.0, eb.1));
    run_tool(&mut e, Tool::EdgeBevel, |p| p.chamfer.offset = 0.2);
    assert_eq!(mesh(&e, a).faces.len(), mesh(&e, b).faces.len());
    assert!(mesh(&e, b).faces.len() > 6);
    assert!(!e.scene.selection.edges_of(b).is_empty());

    let (mut e, a, b) = two_in(Mode::Vertex);
    pick(&mut e, a, Component::Vertex(0));
    pick(&mut e, b, Component::Vertex(0));
    run_tool(&mut e, Tool::VertexBevel, |p| p.inset = 0.2);
    assert_eq!(mesh(&e, a).vertices.len(), 10);
    assert_eq!(mesh(&e, b).vertices.len(), 10);
    assert_eq!(e.scene.selection.vertices_of(b).len(), 3);
}

// ---- Batch D: the rest ----

fn elements(e: &Editor, id: u64) -> ElementState {
    e.scene
        .items
        .iter()
        .find(|i| i.id == id)
        .unwrap()
        .elements
        .clone()
}

#[test]
fn hide_and_lock_selected_work_on_every_mesh() {
    let (mut e, a, b) = tops();
    let (ta, tb) = (top(&e, a), top(&e, b));
    e.dispatch(Command::EditElements {
        kind: ElementKind::Lock,
        op: ElementOp::Selected,
    })
    .unwrap();
    assert!(elements(&e, a).locked.faces.contains(&ta));
    assert!(elements(&e, b).locked.faces.contains(&tb));
    assert_eq!(elements(&e, b).locked.faces.len(), 1);
    e.dispatch(Command::Undo).unwrap();
    assert!(elements(&e, b).locked.is_empty());
}

#[test]
fn selection_sets_are_assigned_and_used_per_mesh() {
    let (mut e, a, b) = tops();
    let (ta, tb) = (top(&e, a), top(&e, b));
    e.dispatch(Command::AssignSelectionSet("caps".into()))
        .unwrap();
    e.dispatch(Command::ClearSelection).unwrap();
    assert!(faces_of(&e, b).is_empty());
    e.dispatch(Command::UseSelectionSet {
        name: "caps".into(),
        mode: UseSetMode::Replace,
    })
    .unwrap();
    assert_eq!(faces_of(&e, a), vec![ta]);
    assert_eq!(faces_of(&e, b), vec![tb]);
}

#[test]
fn tweak_edits_whichever_active_mesh_it_grabs() {
    let (mut e, a, b) = two_in(Mode::Vertex);
    let (a0, b0) = (mesh(&e, a), mesh(&e, b));
    e.dispatch(Command::Activate(Tool::Tweak)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.tweak = Some(TweakGrab {
        item: b,
        element: TweakElement::Vertex(0),
    });
    p.delta = DVec3::Y;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(mesh(&e, a), a0);
    assert!((mesh(&e, b).vertices[0] - b0.vertices[0] - DVec3::Y).length() < 1e-9);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e, b), b0);
}

#[test]
fn axis_slice_cuts_every_mesh() {
    let (mut e, a, b) = two();
    run_tool(&mut e, Tool::AxisSlice, |p| {
        p.slice.axis_slice.number = [0, 1, 0];
    });
    assert_eq!(mesh(&e, a).faces.len(), 10);
    assert_eq!(mesh(&e, b).faces.len(), 10);
}
