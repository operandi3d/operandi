//! Vertex tool defaults against `specifications/vertex-tool-options-2026-11-03`
//! (v2): the recorded settings after Reset (VTO-D0-01, 03, 04, 05).
use operandi_core::*;

fn editor() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(glam::DVec3::ZERO, glam::DVec3::ONE, 1)
        .unwrap()
        .into();
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vec![0];
    e.synchronize_identities().unwrap();
    e
}

fn params(e: &Editor) -> &Parameters {
    &e.session.live.as_ref().unwrap().params
}

#[test]
fn vertex_bevel_defaults() {
    // VTO-D0-03: Inset 0, Use Material off, Material Name empty.
    let mut e = editor();
    e.dispatch(Command::Activate(Tool::VertexBevel)).unwrap();
    let p = params(&e);
    assert_eq!(p.inset, 0.);
    assert!(!p.vertex_use_material);
    assert_eq!(p.vertex_material, "");
}

#[test]
fn vertex_extrude_defaults() {
    // VTO-D0-04: Extrude (shift) 0, Use Material off, Material Name empty.
    // Width starts above 0 so the Extrude arrow alone extrudes
    // (provisional EXW-Q01; the recorded default is 0, VTO-Q03 open).
    let mut e = editor();
    e.dispatch(Command::Activate(Tool::VertexExtrude)).unwrap();
    let p = params(&e);
    assert_eq!(p.shift, 0.);
    assert!(!p.vertex_use_material);
    assert_eq!(p.vertex_material, "");
}

#[test]
fn merge_tool_distance_starts_at_a_millimetre_carries_over_and_resets() {
    // VTO-D0-05: Distance 0.001 after Reset; before Reset, the last value.
    let mut e = editor();
    e.dispatch(Command::Activate(Tool::VertexMerge)).unwrap();
    assert_eq!(params(&e).shift, MERGE_TOOL_DISTANCE);
    let mut p = params(&e).clone();
    p.shift = 0.25;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    e.dispatch(Command::Activate(Tool::VertexMerge)).unwrap();
    let carried = params(&e).shift;
    let mut p = params(&e).clone();
    p.shift = 0.25;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Activate(Tool::VertexMerge)).unwrap();
    assert_eq!(params(&e).shift, 0.25, "the last use's value carries over");
    e.dispatch(Command::ResetVertexMerge).unwrap();
    assert_eq!(params(&e).shift, MERGE_TOOL_DISTANCE);
    // A canceled session keeps whatever was there before it.
    assert_eq!(carried, MERGE_TOOL_DISTANCE);
}

#[test]
fn vertex_slide_defaults() {
    // VTO-D0-01: Percent interpolation at 0 %, Merge Vertices on, Face
    // Handle on.
    let mut e = editor();
    e.dispatch(Command::Activate(Tool::VertexSlide)).unwrap();
    let p = params(&e);
    assert!(!p.edge_slide_distance, "Interpolation Percent");
    assert_eq!(p.shift, 0.);
    assert!(p.vertex_slide_merge && p.vertex_slide_face_handle);
}
