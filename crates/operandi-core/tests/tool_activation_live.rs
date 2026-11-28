//! Tool activation against `specifications/tool-activation/TOOL-ACTIVATION.md`:
//! the bevel tools are live from the moment they turn on (TA-01..03, TA-06);
//! Polygon Extrude still waits for a viewport click (TA-04).
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use glam::DVec3;
use operandi_core::*;

/// A 1 m cube at the origin in `mode`, nothing selected.
fn cube(mode: Mode) -> Editor {
    let all = cases("bevel-activation-2026-11-05");
    let c = case(&all, "BA06");
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh(&c["input"]).into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(mode)).unwrap();
    e
}

fn cube_top_selected() -> Editor {
    let mut e = cube(Mode::Polygon);
    e.scene.selection.faces = vec![1];
    e
}

fn the_mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

fn counts(e: &Editor) -> (usize, usize) {
    let m = the_mesh(e);
    (m.vertices.len(), m.faces.len())
}

fn live_params(e: &Editor) -> Parameters {
    e.session.live.as_ref().unwrap().params.clone()
}

/// TA-01/TA-02: Inset then Shift in the panel, before any viewport click,
/// is one bevel adjusted live: 12 points / 10 polygons, top raised.
#[test]
fn polygon_inset_then_shift_before_any_click_is_one_bevel() {
    let mut e = cube_top_selected();
    let before = e.scene.clone();
    let undo = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let mut p = live_params(&e);
    p.inset = 0.1;
    e.dispatch(Command::ConfigureBevel(p.clone())).unwrap();
    assert_eq!(counts(&e), (12, 10), "Inset alone bevels at once");
    let top = |e: &Editor| {
        the_mesh(e)
            .vertices
            .iter()
            .filter(|v| (v.x.abs() - 0.4).abs() < 1e-9 && (v.z.abs() - 0.4).abs() < 1e-9)
            .map(|v| v.y)
            .collect::<Vec<_>>()
    };
    assert_eq!(top(&e), vec![0.5; 4]);
    p.shift = 0.1;
    e.dispatch(Command::ConfigureBevel(p)).unwrap();
    assert_eq!(counts(&e), (12, 10), "Shift adjusts the same bevel");
    assert!(top(&e).iter().all(|y| (y - 0.6).abs() < 1e-9));
    assert_eq!(e.scene.selection.faces.len(), 1, "the cap stays selected");

    // Drop: one undo step; Undo restores the cube, Redo the bevel.
    e.dispatch(Command::Drop).unwrap();
    assert!(e.session.live.is_none());
    assert_eq!(e.history.undo.len(), undo + 1);
    let bevelled = e.scene.clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(
        e.scene.active().unwrap().mesh,
        before.active().unwrap().mesh
    );
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(
        e.scene.active().unwrap().mesh,
        bevelled.active().unwrap().mesh
    );

    // Save and reopen agree with what the preview showed.
    let dir = std::env::temp_dir().join(format!("operandi-ta-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("Bevel.opscene");
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(back.active().unwrap().mesh, bevelled.active().unwrap().mesh);
}

/// Cancel after a live panel edit leaves the cube as it was.
#[test]
fn cancel_after_a_live_panel_edit_restores_the_mesh() {
    let mut e = cube_top_selected();
    let before = e.scene.active().unwrap().mesh.clone();
    let undo = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let mut p = live_params(&e);
    p.inset = 0.1;
    p.shift = 0.2;
    e.dispatch(Command::ConfigureBevel(p)).unwrap();
    assert_eq!(counts(&e), (12, 10));
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh, before);
    assert_eq!(e.history.undo.len(), undo);
}

/// TA-03: switching Bevel on and dropping it with no edit changes nothing,
/// and the panel re-sending the current values is not an edit.
#[test]
fn activate_and_drop_without_an_edit_changes_nothing() {
    for resend in [false, true] {
        let mut e = cube_top_selected();
        let before = e.scene.clone();
        let undo = e.history.undo.len();
        e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
        assert_eq!(counts(&e), (8, 6));
        if resend {
            let p = live_params(&e);
            e.dispatch(Command::ConfigureBevel(p)).unwrap();
            assert_eq!(counts(&e), (8, 6));
        }
        e.dispatch(Command::Drop).unwrap();
        assert_eq!(e.scene, before);
        assert_eq!(e.history.undo.len(), undo);
    }
}

fn top_edge(e: &Editor) -> (u32, u32) {
    let m = the_mesh(e);
    m.edges()
        .into_iter()
        .find(|&(a, b)| m.vertices[a as usize].y > 0.4 && m.vertices[b as usize].y > 0.4)
        .unwrap()
}

/// TA-01: Edge Bevel's Value edit before any click bevels live.
#[test]
fn edge_value_before_any_click_bevels_live() {
    let mut e = cube(Mode::Edge);
    e.scene.selection.edges = vec![top_edge(&e)];
    let undo = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::EdgeBevel)).unwrap();
    assert_eq!(counts(&e), (8, 6), "switching on changes nothing");
    let mut p = live_params(&e);
    p.chamfer.offset = 0.1;
    e.dispatch(Command::Preview(p)).unwrap();
    let (points, polygons) = counts(&e);
    assert!(
        points > 8 && polygons > 6,
        "{points} points, {polygons} polygons"
    );
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), undo + 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(counts(&e), (8, 6));
}

/// TA-06: Edge Bevel with no edges selected bevels every edge, exactly like
/// selecting them all.
#[test]
fn edge_bevel_with_nothing_selected_bevels_every_edge() {
    let run = |all_selected: bool| {
        let mut e = cube(Mode::Edge);
        if all_selected {
            e.scene.selection.edges = the_mesh(&e).edges();
        }
        e.dispatch(Command::Activate(Tool::EdgeBevel)).unwrap();
        let mut p = live_params(&e);
        p.chamfer.offset = 0.1;
        e.dispatch(Command::Preview(p)).unwrap();
        e.dispatch(Command::Drop).unwrap();
        e.scene.active().unwrap().mesh.clone()
    };
    let none = run(false);
    assert!(none.vertices.len() >= 24, "{} points", none.vertices.len());
    assert_eq!(none, run(true));
    // Edge Chamfer still needs a selection.
    let mut e = cube(Mode::Edge);
    assert!(e.dispatch(Command::Activate(Tool::EdgeChamfer)).is_err());
}

/// TA-01: Vertex Bevel Inset 0.2 before any click cuts the corner: one
/// point becomes three.
#[test]
fn vertex_inset_before_any_click_cuts_the_corner() {
    let mut e = cube(Mode::Vertex);
    let corner = the_mesh(&e)
        .vertices
        .iter()
        .position(|v| *v == DVec3::splat(0.5))
        .unwrap() as u32;
    e.scene.selection.vertices = vec![corner];
    e.dispatch(Command::Activate(Tool::VertexBevel)).unwrap();
    assert_eq!(counts(&e), (8, 6));
    let mut p = live_params(&e);
    p.inset = 0.2;
    e.dispatch(Command::Preview(p)).unwrap();
    assert_eq!(counts(&e), (10, 7));
    assert!(
        !the_mesh(&e).vertices.contains(&DVec3::splat(0.5)),
        "the corner point is gone"
    );
}

/// TA-04: Polygon Extrude still waits for a viewport click; a panel edit
/// only stores the value.
#[test]
fn extrude_panel_edit_before_a_click_changes_nothing() {
    let mut e = cube_top_selected();
    let before = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    let mut p = live_params(&e);
    p.extent = DVec3::new(0., 0.3, 0.);
    e.dispatch(Command::ConfigureExtrude(p)).unwrap();
    assert_eq!(
        e.scene.active().unwrap().mesh,
        before.active().unwrap().mesh
    );
    assert_eq!(live_params(&e).extent.y, 0.3, "the value is stored");
}

/// Undo is per edit: Cmd+Z takes back the last panel edit only, the tool
/// stays on and its values go back to what they were before that edit.
#[test]
fn undo_takes_back_one_panel_edit_at_a_time() {
    let mut e = cube_top_selected();
    let cube = e.scene.active().unwrap().mesh.clone();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let mut p = live_params(&e);
    p.inset = 0.1;
    e.dispatch(Command::ConfigureBevel(p.clone())).unwrap();
    let inset_only = e.scene.active().unwrap().mesh.clone();
    p.shift = 0.1;
    e.dispatch(Command::ConfigureBevel(p)).unwrap();

    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.session.live.as_ref().map(|l| l.tool), Some(Tool::Bevel));
    assert_eq!(live_params(&e).shift, 0.);
    assert_eq!(live_params(&e).inset, 0.1);
    assert_eq!(e.scene.active().unwrap().mesh, inset_only);

    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.session.live.as_ref().map(|l| l.tool), Some(Tool::Bevel));
    assert_eq!(live_params(&e).inset, 0.);
    assert_eq!(e.scene.active().unwrap().mesh, cube);

    // And Redo puts the edits back.
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh, inset_only);
}

/// Continue (the app's Shift+click / Shift+drag) keeps the bevel and starts
/// a fresh one; Undo walks back one edit at a time with the tool on; Drop
/// records one step per bevel (TA-07); Cancel keeps the finished bevel.
#[test]
fn stacked_bevels_undo_one_edit_at_a_time() {
    let mut e = cube_top_selected();
    let cube = e.scene.active().unwrap().mesh.clone();
    let undo = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let mut p = live_params(&e);
    p.inset = 0.1;
    e.dispatch(Command::ConfigureBevel(p)).unwrap();
    let one = e.scene.active().unwrap().mesh.clone();
    e.dispatch(Command::Continue).unwrap();
    assert_eq!((live_params(&e).inset, live_params(&e).shift), (0., 0.));
    assert_eq!(
        e.history.undo.len(),
        undo,
        "no document step while the tool is on"
    );
    let mut p = live_params(&e);
    p.inset = 0.1;
    e.dispatch(Command::ConfigureBevel(p)).unwrap();
    assert_eq!(counts(&e), (16, 14), "stacked on the first");

    e.dispatch(Command::Undo).unwrap(); // the second Inset
    assert_eq!(counts(&e), (12, 10));
    e.dispatch(Command::Undo).unwrap(); // the Continue
    assert_eq!(e.scene.active().unwrap().mesh, one);
    assert_eq!(live_params(&e).inset, 0.1);
    e.dispatch(Command::Undo).unwrap(); // the first Inset
    assert_eq!(e.scene.active().unwrap().mesh, cube);
    assert_eq!(e.session.live.as_ref().map(|l| l.tool), Some(Tool::Bevel));

    // Redo all three, Drop: TA-07, one undo step per bevel.
    for _ in 0..3 {
        e.dispatch(Command::Redo).unwrap();
    }
    assert_eq!(counts(&e), (16, 14));
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), undo + 2);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh, one);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh, cube);
    e.dispatch(Command::Redo).unwrap();
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(counts(&e), (16, 14));

    // Cancel during the second bevel keeps the first, as one step.
    let mut e = cube_top_selected();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let mut p = live_params(&e);
    p.inset = 0.1;
    e.dispatch(Command::ConfigureBevel(p.clone())).unwrap();
    e.dispatch(Command::Continue).unwrap();
    e.dispatch(Command::ConfigureBevel(p)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh, one);
    assert_eq!(e.history.undo.len(), undo + 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh, cube);
}
