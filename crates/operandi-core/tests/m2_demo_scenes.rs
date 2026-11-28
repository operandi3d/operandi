//! Milestone 2 demonstration scenes: one original before/after pair per Edge
//! panel row, produced through the Editor (same route as the panel and the
//! command line) and saved as native scenes in `docs/m2-demo/scenes/`.
//!
//! The test fails when a saved scene no longer matches what the current code
//! produces. Regenerate with `OPERANDI_WRITE_M2_DEMO=1 cargo test --test
//! m2_demo_scenes`.
use glam::DVec3;
use operandi_core::*;
use std::path::PathBuf;

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
fn mesh(points: &[[f64; 3]], faces: &[&[u32]]) -> Mesh {
    Mesh {
        vertices: points.iter().map(|&p| DVec3::from_array(p)).collect(),
        faces: faces.iter().map(|f| face(f.to_vec())).collect(),
        ..Default::default()
    }
}
fn cube() -> Mesh {
    Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap()
}
fn cube_edge(m: &Mesh) -> (u32, u32) {
    m.edges()
        .into_iter()
        .find(|&(a, b)| {
            let (p, q) = (m.vertices[a as usize], m.vertices[b as usize]);
            p.y > 0.4 && q.y > 0.4 && p.z > 0.4 && q.z > 0.4
        })
        .unwrap()
}
/// Two quads side by side sharing edge 1–4.
fn strip() -> Mesh {
    mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [2., 0., 0.],
            [0., 1., 0.],
            [1., 1., 0.],
            [2., 1., 0.],
        ],
        &[&[0, 1, 4, 3], &[1, 2, 5, 4]],
    )
}
/// Two separate quads facing up with a gap between x = 1 and x = 2.
fn pair() -> Mesh {
    mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [2., 0., 0.],
            [3., 0., 0.],
            [3., 1., 0.],
            [2., 1., 0.],
        ],
        &[&[0, 1, 2, 3], &[4, 5, 6, 7]],
    )
}
fn open_box() -> (Mesh, Vec<(u32, u32)>) {
    let mut c = cube();
    let top = (0..6).find(|&f| c.normal(f).y > 0.9).unwrap();
    let rim = c.faces.remove(top).vertices;
    let edges = (0..4).map(|i| (rim[i], rim[(i + 1) % 4])).collect();
    (c, edges)
}
fn beveled_top() -> (Mesh, Vec<(u32, u32)>) {
    let c = cube();
    let top = (0..6).find(|&f| c.normal(f).y > 0.9).unwrap() as u32;
    let r = bevel_geometry(&c, &[top], 0.2, 0.2, false, &BevelOptions::default(), None).unwrap();
    let m = r.mesh;
    let cap = m.faces[r.selected[0] as usize].vertices.clone();
    let mut loops: Vec<(u32, u32)> = (0..4).map(|i| (cap[i], cap[(i + 1) % 4])).collect();
    for f in 0..m.faces.len() {
        if m.normal(f).y.abs() < 1e-9 {
            let vs = &m.faces[f].vertices;
            for i in 0..vs.len() {
                let (a, b) = (vs[i], vs[(i + 1) % vs.len()]);
                if m.vertices[a as usize].y > 0.4 && m.vertices[b as usize].y > 0.4 {
                    loops.push((a, b));
                }
            }
        }
    }
    (m, loops)
}

/// What a row does to its "before" editor.
enum Act {
    Tool(Tool, Box<dyn Fn(&mut Parameters)>),
    Command(Box<Command>),
    Bridge,
}

/// One demonstration: name, source mesh, selected edges, action.
type Row = (&'static str, Mesh, Vec<(u32, u32)>, Act);

fn editor(m: Mesh, edges: Vec<(u32, u32)>) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = m.into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = Mode::Edge;
    // Selected edges are stored low point first, as the app stores them.
    e.scene.selection.edges = edges
        .into_iter()
        .map(|(a, b)| (a.min(b), a.max(b)))
        .collect();
    e
}

fn rows() -> Vec<Row> {
    let c = cube();
    let ce = vec![cube_edge(&c)];
    let flat_bevel = edge_bevel_geometry(&c, &ce, 0.2, 3, 0., false).unwrap().0;
    let rows_of_flat: Vec<(u32, u32)> = flat_bevel
        .edges()
        .into_iter()
        .filter(|&(a, b)| {
            let (p, q) = (
                flat_bevel.vertices[a as usize],
                flat_bevel.vertices[b as usize],
            );
            (p.x - q.x).abs() > 0.9 && p.y > 0.29 && p.z > 0.29
        })
        .collect();
    let (open, rim) = open_box();
    let (ring, loops) = beveled_top();
    let mut normals = mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 0., -1.],
            [0., 1., 0.],
            [1., 1., 0.],
            [1., 1., -1.],
        ],
        &[&[0, 1, 4, 3], &[1, 2, 5, 4]],
    );
    normals.attributes.corner_normals.insert(
        "vert_normals".into(),
        vec![vec![Some(DVec3::new(1., 0., 1.).normalize()); 4]; 2],
    );
    let tri = mesh(
        &[[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        &[&[0, 1, 2], &[0, 2, 3]],
    );
    let one = mesh(
        &[[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        &[&[0, 1, 2, 3]],
    );
    let ec = |c: EdgeCommand| Act::Command(Box::new(Command::EdgeCommand(c)));
    vec![
        (
            "ME-T01-edge-bevel",
            c.clone(),
            ce.clone(),
            Act::Tool(
                Tool::EdgeBevel,
                Box::new(|p| {
                    p.chamfer.offset = 0.15;
                    p.chamfer.segments = 3;
                }),
            ),
        ),
        (
            "ME-T02-edge-extrude",
            c.clone(),
            ce.clone(),
            Act::Tool(
                Tool::EdgeExtrude,
                Box::new(|p| {
                    p.shift = 0.2;
                    p.inset = 0.1;
                }),
            ),
        ),
        (
            "ME-T03-extend",
            one.clone(),
            vec![(1, 2)],
            Act::Tool(Tool::EdgeExtend, Box::new(|p| p.delta = DVec3::X)),
        ),
        ("ME-T04-bridge", pair(), vec![(1, 2), (4, 7)], Act::Bridge),
        (
            "ME-T05-slide",
            strip(),
            vec![(1, 4)],
            Act::Tool(Tool::EdgeSlide, Box::new(|p| p.shift = 0.5)),
        ),
        (
            "ME-T06-edge-relax",
            flat_bevel,
            rows_of_flat,
            Act::Tool(Tool::EdgeRelax, Box::new(|p| p.shift = 0.05)),
        ),
        (
            "ME-T07-edge-chamfer",
            c.clone(),
            ce.clone(),
            Act::Tool(
                Tool::EdgeChamfer,
                Box::new(|p| {
                    p.chamfer.offset = 0.15;
                    p.chamfer.segments = 3;
                }),
            ),
        ),
        (
            "ME-T08-add-loop",
            c.clone(),
            ce.clone(),
            Act::Command(Box::new(Command::AddEdgeLoop {
                a: ce[0].0,
                b: ce[0].1,
                t: 0.5,
            })),
        ),
        (
            "ME-T09-add-point",
            c.clone(),
            ce.clone(),
            Act::Command(Box::new(Command::AddEdgePoint {
                a: ce[0].0,
                b: ce[0].1,
                t: 0.25,
            })),
        ),
        (
            "ME-T10-edge-crease",
            c.clone(),
            ce.clone(),
            Act::Tool(Tool::EdgeCrease, Box::new(|p| p.shift = 0.6)),
        ),
        (
            "ME-C01-grow-quads",
            one.clone(),
            vec![(1, 2)],
            ec(EdgeCommand::GrowQuads),
        ),
        ("ME-C02-fill-quads", open, rim, ec(EdgeCommand::FillQuads)),
        (
            "ME-C03-set-position",
            strip(),
            vec![(1, 4)],
            Act::Command(Box::new(Command::SetPosition(PositionAssignment {
                space: PositionSpace::Local,
                axis: PositionAxis::Z,
                value: 0.5,
                expected: None,
            }))),
        ),
        (
            "ME-C04-split",
            strip(),
            vec![(1, 4)],
            ec(EdgeCommand::Split),
        ),
        (
            "ME-C05-split-normals",
            normals,
            vec![(1, 4)],
            ec(EdgeCommand::SplitNormals),
        ),
        ("ME-C06-spin", tri, vec![(0, 2)], ec(EdgeCommand::Spin)),
        (
            "ME-C07-join",
            pair(),
            vec![(1, 2), (4, 7)],
            ec(EdgeCommand::Join),
        ),
        (
            "ME-C08-join-averaged",
            pair(),
            vec![(1, 2), (4, 7)],
            ec(EdgeCommand::JoinAveraged),
        ),
        (
            "ME-C09-collapse",
            c.clone(),
            ce.clone(),
            ec(EdgeCommand::Collapse),
        ),
        (
            "ME-C10-remove",
            strip(),
            vec![(1, 4)],
            ec(EdgeCommand::Remove),
        ),
        (
            "ME-C11-edges-to-curves",
            strip(),
            vec![(0, 1), (1, 2)],
            ec(EdgeCommand::ToCurves),
        ),
        ("ME-C12-unbevel", ring, loops, ec(EdgeCommand::Unbevel)),
        ("ME-C13-edge-subdivide", c, ce, ec(EdgeCommand::Subdivide)),
    ]
}

fn run(e: &mut Editor, act: &Act) -> Result<()> {
    match act {
        Act::Tool(tool, set) => {
            e.dispatch(Command::Activate(*tool))?;
            let mut p = e
                .session
                .live
                .as_ref()
                .ok_or("no live tool")?
                .params
                .clone();
            set(&mut p);
            e.dispatch(Command::Preview(p))?;
            e.dispatch(Command::Drop)
        }
        Act::Command(c) => e.dispatch((**c).clone()),
        Act::Bridge => {
            e.dispatch(Command::ArmBridge)?;
            e.dispatch(Command::Apply)?;
            e.dispatch(Command::Drop)
        }
    }
}

#[test]
fn m2_demo_scenes_cover_every_row_and_are_current() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/m2-demo/scenes");
    let write = std::env::var_os("OPERANDI_WRITE_M2_DEMO").is_some();
    if write {
        std::fs::create_dir_all(&dir).unwrap();
    }
    let rows = rows();
    assert_eq!(rows.len(), 23, "one scene pair per M2 row");
    for (name, m, edges, act) in rows {
        let mut e = editor(m, edges);
        let before = e.scene.clone();
        run(&mut e, &act).unwrap_or_else(|err| panic!("{name}: {err}"));
        assert_ne!(e.scene, before, "{name} changed nothing");
        e.scene.validate().unwrap();
        // One undo restores the before scene.
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(
            e.scene.active().unwrap().mesh,
            before.active().unwrap().mesh,
            "{name} undo"
        );
        e.dispatch(Command::Redo).unwrap();
        let after = e.scene.clone();
        for (suffix, scene) in [("before", &before), ("after", &after)] {
            let path = dir.join(format!("{name}.{suffix}.opscene"));
            if write {
                persistence::save(&path, scene).unwrap();
            }
            let saved =
                persistence::load(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
            assert_eq!(
                saved.active().unwrap().mesh,
                scene.active().unwrap().mesh,
                "{} is stale; regenerate (see module docs)",
                path.display()
            );
        }
    }
}
