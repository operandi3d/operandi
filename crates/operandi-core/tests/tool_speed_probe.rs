//! Opt-in probe: core preview cost per frame for every interactive tool on a
//! 16k-quad mesh (128 x 128 grid, Catmull-Clark), with a small selection.
use glam::DVec3;
use operandi_core::*;

fn grid() -> Mesh {
    let n = 128u32;
    let mut mesh = Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            mesh.vertices.push(DVec3::new(
                i as f64 * 0.1,
                j as f64 * 0.1,
                ((i + j) % 3) as f64 * 0.01,
            ));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + n + 2, a + n + 1],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    mesh.attributes.representations = vec![PolygonRepresentation::CatmullClark; mesh.faces.len()];
    mesh
}

#[test]
#[ignore = "opt-in timing probe"]
fn tool_preview_costs() {
    let n = 128u32;
    let mid_face = (n / 2) * n + n / 2;
    let mid_point = (n / 2) * (n + 1) + n / 2;
    type Step = fn(&mut Parameters, f64);
    let tools: Vec<(Tool, Mode, Step)> = vec![
        (Tool::Move, Mode::Polygon, |p, k| p.delta.z = 0.01 * k),
        (Tool::Rotate, Mode::Polygon, |p, k| {
            p.rotation_delta.z = 0.01 * k
        }),
        (Tool::Scale, Mode::Polygon, |p, k| {
            p.scale_delta = DVec3::splat(1. + 0.01 * k)
        }),
        (Tool::Transform, Mode::Polygon, |p, k| p.delta.z = 0.01 * k),
        (Tool::Extrude, Mode::Polygon, |p, k| p.shift = 0.01 * k),
        (Tool::SmoothShift, Mode::Polygon, |p, k| {
            p.smooth_shift.offset = 0.01 * k
        }),
        (Tool::Thicken, Mode::Polygon, |p, k| {
            p.smooth_shift.offset = 0.01 * k
        }),
        (Tool::Bevel, Mode::Polygon, |p, k| {
            p.inset = 0.0005 * k;
            p.shift = 0.01 * k
        }),
        (Tool::PolyHaul, Mode::Polygon, |p, k| {
            p.poly_haul_offset = 0.01 * k
        }),
        (Tool::EdgeExtrude, Mode::Edge, |p, k| {
            p.shift = 0.01 * k;
            p.inset = 0.01
        }),
        (Tool::EdgeBevel, Mode::Edge, |p, k| {
            p.chamfer.offset = 0.001 * k
        }),
        (Tool::EdgeChamfer, Mode::Edge, |p, k| {
            p.chamfer.offset = 0.001 * k
        }),
        (Tool::EdgeSlide, Mode::Edge, |p, k| p.shift = 0.02 * k),
        (Tool::EdgeCrease, Mode::Edge, |p, k| p.shift = 0.05 * k),
        // Edge Relax isn't listed: it needs a bevel whose rows end on side
        // edges, which this grid can't set up simply.
        (Tool::EdgeExtend, Mode::Edge, |p, k| p.delta.z = 0.01 * k),
        (Tool::VertexBevel, Mode::Vertex, |p, k| p.inset = 0.002 * k),
        (Tool::VertexExtrude, Mode::Vertex, |p, k| {
            p.shift = 0.01 * k;
            p.inset = 0.01
        }),
        (Tool::VertexMerge, Mode::Vertex, |p, k| p.shift = 0.001 * k),
        (Tool::VertexTear, Mode::Vertex, |p, k| p.delta.z = 0.01 * k),
        (Tool::VertexSlide, Mode::Vertex, |p, k| {
            p.shift = 0.05 * k;
            p.delta = DVec3::X
        }),
        // Slice tools: the ring of the selected middle edge; two points on
        // the middle polygon's opposite edges; a line across the middle row.
        (Tool::LoopSlice, Mode::Edge, |p, k| {
            p.slice.loop_slice.positions = vec![0.3 + 0.01 * k]
        }),
        (Tool::EdgeSlice, Mode::Polygon, |p, k| {
            let t = 0.2 + 0.01 * k;
            p.slice.edge_slice.points = vec![
                SlicePoint::Edge {
                    a: 8320,
                    b: 8321,
                    t,
                },
                SlicePoint::Edge {
                    a: 8449,
                    b: 8450,
                    t,
                },
            ]
        }),
        (Tool::PolygonSlice, Mode::Polygon, |p, k| {
            let y = 6.41 + 0.001 * k;
            p.slice.polygon_slice.cuts = vec![PlaneCut {
                start: DVec3::new(6.3, y, 0.),
                end: DVec3::new(6.7, y, 0.),
                up: DVec3::Z,
            }]
        }),
        // Waves 2 and 3 across the selected middle polygons.
        (Tool::PenSlice, Mode::Polygon, |p, k| {
            let o = &mut p.slice.pen_slice;
            o.cutter.axis = SliceAxis::Z;
            o.closed = false;
            let y = 6.45 + 0.001 * k;
            o.points = vec![
                DVec3::new(6.3500000000000005, y, 1.),
                DVec3::new(6.65, y, 1.),
            ];
        }),
        (Tool::CurveSlice, Mode::Polygon, |p, k| {
            let o = &mut p.slice.curve_slice;
            o.cutter.axis = SliceAxis::Z;
            let y = 6.45 + 0.001 * k;
            o.points = vec![
                DVec3::new(6.3500000000000005, y, 1.),
                DVec3::new(6.5, y + 0.025, 1.),
                DVec3::new(6.65, y, 1.),
            ];
        }),
        (Tool::PrimitiveSlice, Mode::Polygon, |p, k| {
            let o = &mut p.slice.primitive_slice;
            o.place = Some(ShapePlace::on(
                DVec3::new(6.5 + 0.001 * k, 6.45, 0.),
                DVec3::Z,
            ));
            o.half_x = 0.1;
            o.half_y = 0.025;
        }),
        (Tool::AxisSlice, Mode::Polygon, |p, k| {
            p.slice.axis_slice.offset.x = 0.001 * k
        }),
    ];
    let only = std::env::var("PROBE_TOOL").ok();
    let frames: usize = std::env::var("PROBE_FRAMES")
        .ok()
        .and_then(|f| f.parse().ok())
        .unwrap_or(10);
    for (tool, mode, step) in tools {
        if only.as_ref().is_some_and(|o| *o != format!("{tool:?}")) {
            continue;
        }
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        let mesh = grid();
        e.scene.selection.mode = mode;
        match (tool, mode) {
            // Extend works on open borders: an edge along the grid's edge.
            (Tool::EdgeExtend, _) => e.scene.selection.edges = vec![(n / 2, n / 2 + 1)],
            // Tear needs a polygon and one of its points shared with others.
            (Tool::VertexTear, _) => {
                e.scene.selection.faces = vec![mid_face];
                e.scene.selection.vertices = vec![mid_point];
            }
            (_, Mode::Polygon) => {
                e.scene.selection.faces = vec![mid_face, mid_face + 1, mid_face + n]
            }
            (_, Mode::Edge) => e.scene.selection.edges = vec![(mid_point, mid_point + 1)],
            _ => e.scene.selection.vertices = vec![mid_point],
        }
        e.scene.active_mut().unwrap().mesh = mesh.into();
        if let Err(err) = e.dispatch(Command::Activate(tool)) {
            println!("TOOL {tool:?}: activate refused: {err}");
            continue;
        }
        let Some(base) = e.session.live.as_ref().map(|l| l.params.clone()) else {
            println!("TOOL {tool:?}: no live session");
            continue;
        };
        let t = std::time::Instant::now();
        let mut failed = None;
        for k in 1..=frames {
            let mut p = base.clone();
            step(&mut p, (k % 40 + 1) as f64);
            if let Err(err) = e.dispatch(Command::Preview(p)) {
                failed = Some(err);
                break;
            }
        }
        let ms = t.elapsed().as_secs_f64() * 1000. / frames as f64;
        let faces = e.scene.active().map_or(0, |i| i.mesh.faces.len());
        match failed {
            Some(err) => println!("TOOL {tool:?}: preview refused: {err}"),
            None => println!("TOOL {tool:?}: {ms:.1} ms/frame (faces {faces})"),
        }
    }
}
