//! Vertex tool options (specifications/vertex-tool-options-2026-11-03):
//! Use Material on Vertex Bevel and Vertex Extrude (VTO-T02/T03), and
//! Vertex Slide's Interpolation and Merge Vertices (VTO-T07).
use glam::DVec3;
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
/// 3×3 quads in z = 0 facing +z; point `r * 4 + c` sits at (c, r, 0).
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
    m.validate().unwrap();
    m
}
fn editor(points: Vec<u32>) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = grid().into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = points;
    e
}
fn mesh(e: &Editor) -> Mesh {
    (*e.scene.active().unwrap().mesh).clone()
}
fn near(a: DVec3, b: DVec3) -> bool {
    a.distance(b) < 1e-9
}
/// Activate `tool` on grid point 5, preview `edit`, drop. Returns the
/// editor (history has the drop as its last step).
fn run_tool(tool: Tool, points: Vec<u32>, edit: impl Fn(&mut Parameters)) -> Editor {
    let mut e = editor(points);
    e.dispatch(Command::Activate(tool)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    edit(&mut p);
    e.dispatch(Command::Preview(p))
        .unwrap_or_else(|err| panic!("{tool:?}: {err}"));
    e.dispatch(Command::Drop).unwrap();
    e
}

#[test]
fn defaults_follow_the_recorded_values() {
    let p = Parameters::default();
    // VTO-T03 Use Material off, Material Name empty; Extrude/Width 0.
    assert!(!p.vertex_use_material);
    assert_eq!(p.vertex_material, "");
    assert_eq!((p.shift, p.inset), (0., 0.));
    // VTO-T07 Interpolation Percent, Percent 0, Merge Vertices on.
    assert!(!p.edge_slide_distance);
    assert!(p.vertex_slide_merge);
}

#[test]
fn vertex_extrude_use_material_tags_only_the_new_triangles() {
    // UVN-T-05: Width 0.2, Extrude 0.2 on interior point 5.
    let e = run_tool(Tool::VertexExtrude, vec![5], |p| {
        p.shift = 0.2;
        p.inset = 0.2;
        p.vertex_use_material = true;
        p.vertex_material = "Red".into();
    });
    let m = mesh(&e);
    assert_eq!((m.vertices.len(), m.faces.len()), (20, 13));
    let tagged: Vec<usize> = (0..m.faces.len())
        .filter(|&f| m.faces[f].material == "Red")
        .collect();
    assert_eq!(tagged.len(), 4);
    for &f in &tagged {
        assert_eq!(m.faces[f].vertices.len(), 3);
        assert!(
            m.faces[f].vertices.contains(&5),
            "triangles meet at the apex"
        );
    }
    // The cut quads keep their own tag.
    assert_eq!(
        m.faces.iter().filter(|f| f.material == "Default").count(),
        9
    );
}

#[test]
fn vertex_extrude_without_use_material_keeps_neighbour_tags() {
    let e = run_tool(Tool::VertexExtrude, vec![5], |p| {
        p.shift = 0.2;
        p.inset = 0.2;
        p.vertex_material = "Red".into();
    });
    assert!(mesh(&e).faces.iter().all(|f| f.material != "Red"));
}

#[test]
fn vertex_bevel_use_material_tags_the_cap_and_undo_restores() {
    // UVN-T-04: Inset 0.2 on interior point 5.
    let mut e = run_tool(Tool::VertexBevel, vec![5], |p| {
        p.inset = 0.2;
        p.vertex_use_material = true;
        p.vertex_material = "Cap".into();
    });
    let m = mesh(&e);
    assert_eq!((m.vertices.len(), m.faces.len()), (19, 10));
    let caps: Vec<&Face> = m.faces.iter().filter(|f| f.material == "Cap").collect();
    assert_eq!(caps.len(), 1);
    assert_eq!(caps[0].vertices.len(), 4);
    assert_eq!(caps[0].material_label.as_deref(), Some("Cap"));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e), grid());
}

#[test]
fn vertex_bevel_use_material_assigns_an_empty_name_as_typed() {
    let e = run_tool(Tool::VertexBevel, vec![5], |p| {
        p.inset = 0.2;
        p.vertex_use_material = true;
    });
    assert_eq!(
        mesh(&e)
            .faces
            .iter()
            .filter(|f| f.material.is_empty())
            .count(),
        1
    );
}

fn slide(amount: f64, by_distance: bool, merge: bool) -> Result<(Mesh, Vec<u32>)> {
    vertex_commands::vertex_slide_with(
        &grid(),
        &[5],
        DVec3::X,
        amount,
        vertex_commands::SlideOptions { by_distance, merge },
    )
}

#[test]
fn slide_percent_runs_from_minus_to_plus_one_hundred() {
    let (m, _) = slide(0.25, false, true).unwrap();
    assert!(near(m.vertices[5], DVec3::new(1.25, 1., 0.)));
    let (m, _) = slide(-0.5, false, true).unwrap();
    assert!(near(m.vertices[5], DVec3::new(0.5, 1., 0.)));
    assert!(slide(1.5, false, true).is_err());
    assert!(slide(f64::NAN, false, true).is_err());
}

#[test]
fn slide_distance_is_a_length_along_the_edge() {
    let (m, sel) = slide(0.4, true, true).unwrap();
    assert!(near(m.vertices[5], DVec3::new(1.4, 1., 0.)));
    assert_eq!(sel, vec![5]);
    // Provisional: a Distance may not pass the neighbouring point.
    assert!(slide(1.5, true, true).is_err());
}

#[test]
fn merge_vertices_welds_a_point_that_reaches_its_neighbour() {
    let (m, sel) = slide(1., false, true).unwrap();
    assert_eq!(m.vertices.len(), 15);
    assert_eq!(sel.len(), 1);
    assert!(near(m.vertices[sel[0] as usize], DVec3::new(2., 1., 0.)));
    m.validate().unwrap();
    // Off: the point stops on its neighbour without merging.
    let (m, sel) = slide(1., false, false).unwrap();
    assert_eq!(m.vertices.len(), 16);
    assert_eq!(sel, vec![5]);
    assert!(near(m.vertices[5], m.vertices[6]));
    // Distance mode merges at the full edge length too.
    let (m, _) = slide(1., true, true).unwrap();
    assert_eq!(m.vertices.len(), 15);
}

#[test]
fn editor_slide_merge_renumbers_the_selection_and_undoes() {
    let mut e = run_tool(Tool::VertexSlide, vec![5], |p| {
        p.delta = DVec3::X;
        p.shift = 1.;
    });
    let m = mesh(&e);
    assert_eq!(m.vertices.len(), 15);
    let sel = &e.scene.selection.vertices;
    assert_eq!(sel.len(), 1);
    assert!(near(m.vertices[sel[0] as usize], DVec3::new(2., 1., 0.)));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(mesh(&e), grid());
    assert_eq!(e.scene.selection.vertices, vec![5]);
    // Distance interpolation through the editor.
    let e = run_tool(Tool::VertexSlide, vec![5], |p| {
        p.delta = DVec3::X;
        p.edge_slide_distance = true;
        p.shift = 0.3;
    });
    assert!(near(mesh(&e).vertices[5], DVec3::new(1.3, 1., 0.)));
}
