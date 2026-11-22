//! Vertex Normal Tools (G9 Part A). The spec's tests: VN-1 cube oracles;
//! By Angle hardens a cube's 12 edges and none of a UV sphere's; the map
//! keeps HardEdges through save and reopen; Select Hard picks exactly the
//! hardened edge; Spin Polygon Vertex Indices turns a quad's point order
//! with its UVs. Plus Toggle, Convert to Hard Edges, Rename and Transfer.
use glam::DVec3;
use operandi_core::vertex_normals::*;
use operandi_core::*;

const T: f64 = 1e-3;

fn cube() -> Mesh {
    Mesh::box_mesh(DVec3::ZERO, DVec3::splat(0.5), 0).unwrap()
}

fn editor(mesh: Mesh) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.undo.clear();
    e
}

fn run(e: &mut Editor, request: NormalRequest) -> Result<()> {
    e.dispatch(Command::VertexNormals(Box::new(NormalCommand {
        request,
        options: NormalOptions::default(),
    })))
}

fn mesh(e: &Editor) -> Mesh {
    e.scene.active().unwrap().mesh.to_owned_value()
}

/// The polygon whose normal is `n`.
fn face_facing(m: &Mesh, n: DVec3) -> usize {
    (0..m.faces.len())
        .find(|&f| m.normal(f).abs_diff_eq(n, 1e-9))
        .unwrap()
}

fn normal_at(m: &Mesh, f: usize, v: u32) -> DVec3 {
    let c = m.faces[f].vertices.iter().position(|&x| x == v).unwrap();
    custom_corner_normal(m, f, c).unwrap()
}

#[test]
fn vn1_cube_oracles() {
    let all: Vec<(u32, u32)> = cube().edges();
    assert_eq!(all.len(), 12);
    // All 12 edges hard: the −Y face's corners read (0, −1, 0).
    let mut e = editor(cube());
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = all.clone();
    run(
        &mut e,
        NormalRequest::Harden {
            mode: HardenMode::Edges,
            soften_others: false,
            connected: false,
        },
    )
    .unwrap();
    let m = mesh(&e);
    assert_eq!(hard_edges(&m).len(), 12);
    let ny = face_facing(&m, -DVec3::Y);
    let nz = face_facing(&m, -DVec3::Z);
    for &v in &m.faces[ny].vertices {
        assert!(normal_at(&m, ny, v).abs_diff_eq(-DVec3::Y, T));
    }
    // Soften the edge the −Y and −Z faces share.
    let shared: Vec<u32> = m.faces[ny]
        .vertices
        .iter()
        .copied()
        .filter(|v| m.faces[nz].vertices.contains(v))
        .collect();
    assert_eq!(shared.len(), 2);
    e.scene.selection.edges = vec![(shared[0].min(shared[1]), shared[0].max(shared[1]))];
    run(
        &mut e,
        NormalRequest::Soften {
            mode: Some(HardenMode::Edges),
            connected: false,
        },
    )
    .unwrap();
    let m = mesh(&e);
    let half = DVec3::new(0., -1., -1.).normalize();
    for &v in &m.faces[ny].vertices {
        let want = if shared.contains(&v) { half } else { -DVec3::Y };
        assert!(
            normal_at(&m, ny, v).abs_diff_eq(want, T),
            "{}",
            normal_at(&m, ny, v)
        );
    }
    for &v in &m.faces[nz].vertices {
        let want = if shared.contains(&v) { half } else { -DVec3::Z };
        assert!(normal_at(&m, nz, v).abs_diff_eq(want, T));
    }
    // No hard edges: every corner ±0.577 on every axis.
    run(
        &mut e,
        NormalRequest::Soften {
            mode: None,
            connected: false,
        },
    )
    .unwrap();
    let m = mesh(&e);
    assert!(hard_edges(&m).is_empty());
    for f in 0..m.faces.len() {
        for &v in &m.faces[f].vertices {
            for a in normal_at(&m, f, v).to_array() {
                assert!((a.abs() - 0.57735).abs() < T);
            }
        }
    }
}

#[test]
fn by_angle_hardens_a_cube_but_not_a_sphere() {
    let mut e = editor(cube());
    run(
        &mut e,
        NormalRequest::Harden {
            mode: HardenMode::ByAngle,
            soften_others: false,
            connected: false,
        },
    )
    .unwrap();
    assert_eq!(hard_edges(&mesh(&e)).len(), 12);
    let sphere = operandi_core::radial::globe_geometry(
        DVec3::ZERO,
        operandi_core::radial::RadialParameters {
            radius: DVec3::ONE,
            sides: 24,
            segments: 12,
        },
    )
    .unwrap();
    let mut e = editor(sphere);
    let r = run(
        &mut e,
        NormalRequest::Harden {
            mode: HardenMode::ByAngle,
            soften_others: false,
            connected: false,
        },
    );
    assert!(r.is_err(), "nothing sharper than 40° to harden");
    assert!(hard_edges(&mesh(&e)).is_empty());
}

#[test]
fn recalculate_keeps_hard_edges_through_save_and_reopen() {
    let mut e = editor(cube());
    let edge = cube().edges()[0];
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![edge];
    run(
        &mut e,
        NormalRequest::Harden {
            mode: HardenMode::Edges,
            soften_others: false,
            connected: false,
        },
    )
    .unwrap();
    run(&mut e, NormalRequest::Recalculate).unwrap();
    let m = mesh(&e);
    assert_eq!(hard_edges(&m), [edge].into_iter().collect());
    assert!(m.attributes.corner_normals.contains_key(DEFAULT_MAP));
    let bytes = persistence::encode(&e.scene).unwrap();
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back, e.scene);
    let shown_before = shading::mesh_corner_normals(&m, &e.scene.smoothing);
    let shown_after = shading::mesh_corner_normals(&back.items[0].mesh, &back.smoothing);
    assert_eq!(shown_before, shown_after);
    // Harden is one undo step; Recalculate right after changes nothing,
    // so it records none.
    assert_eq!(e.undo.len(), 1);
}

#[test]
fn select_hard_after_hardening_one_edge_gives_that_edge() {
    let mut e = editor(cube());
    let edge = cube().edges()[3];
    e.scene.selection.mode = Mode::Edge;
    e.scene.selection.edges = vec![edge];
    run(
        &mut e,
        NormalRequest::Harden {
            mode: HardenMode::Edges,
            soften_others: false,
            connected: false,
        },
    )
    .unwrap();
    e.scene.selection.edges.clear();
    run(
        &mut e,
        NormalRequest::Select {
            kind: EdgeKind::Hard,
            mode: SelectMode::Set,
        },
    )
    .unwrap();
    assert_eq!(e.scene.selection.edges, vec![edge]);
    run(
        &mut e,
        NormalRequest::Select {
            kind: EdgeKind::Soft,
            mode: SelectMode::Add,
        },
    )
    .unwrap();
    assert_eq!(e.scene.selection.edges.len(), 12);
}

#[test]
fn spin_polygon_vertex_indices_turns_a_quad_with_its_uvs() {
    let mut m = Mesh {
        vertices: vec![DVec3::ZERO, DVec3::X, DVec3::new(1., 0., 1.), DVec3::Z],
        ..Default::default()
    };
    m.faces.push(Face {
        vertices: vec![0, 1, 2, 3],
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    });
    let uv = |x: f64, y: f64| Some(glam::DVec2::new(x, y));
    m.uv_maps.insert(
        "Texture".into(),
        vec![vec![uv(0., 0.), uv(1., 0.), uv(1., 1.), uv(0., 1.)]],
    );
    m.validate().unwrap();
    let mut e = editor(m.clone());
    run(&mut e, NormalRequest::Spin { offset: 1 }).unwrap();
    let out = mesh(&e);
    assert_eq!(out.faces[0].vertices, vec![1, 2, 3, 0]);
    assert_eq!(
        out.uv_maps["Texture"][0],
        vec![uv(1., 0.), uv(1., 1.), uv(0., 1.), uv(0., 0.)]
    );
    // Corner data moved with its point: each point keeps its UV.
    for (c, &v) in out.faces[0].vertices.iter().enumerate() {
        let was = m.faces[0].vertices.iter().position(|&x| x == v).unwrap();
        assert_eq!(out.uv_maps["Texture"][0][c], m.uv_maps["Texture"][0][was]);
    }
    assert!(
        out.normal(0).abs_diff_eq(m.normal(0), 1e-12),
        "shape unchanged"
    );
    run(&mut e, NormalRequest::Spin { offset: -1 }).unwrap();
    assert_eq!(mesh(&e).faces[0].vertices, vec![0, 1, 2, 3]);
}

#[test]
fn toggle_convert_and_rename() {
    let mut e = editor(cube());
    run(&mut e, NormalRequest::ToggleMaps(ToggleMaps::Auto)).unwrap();
    assert!(shown_map_name(&mesh(&e)).is_some(), "Auto recreates");
    run(&mut e, NormalRequest::ToggleMaps(ToggleMaps::Auto)).unwrap();
    assert!(
        mesh(&e).attributes.corner_normals.is_empty(),
        "Auto removes"
    );
    // A plain cube shades split at 90° (Smoothing angle): all 12 hard.
    run(
        &mut e,
        NormalRequest::ConvertToHard {
            remove_groups: false,
        },
    )
    .unwrap();
    assert_eq!(hard_edges(&mesh(&e)).len(), 12);
    run(&mut e, NormalRequest::ToggleMaps(ToggleMaps::On)).unwrap();
    e.dispatch(Command::VertexNormals(Box::new(NormalCommand {
        request: NormalRequest::Rename,
        options: NormalOptions {
            map_name: "Custom".into(),
            ..Default::default()
        },
    })))
    .unwrap();
    assert_eq!(shown_map_name(&mesh(&e)), Some("Custom"));
}

#[test]
fn transfer_normals_from_the_background_mesh() {
    // VN-8: a flat plane under a smooth cube takes the cube's normals.
    let mut e = Editor::default();
    e.dispatch(Command::InsertUnit {
        tool: Tool::Box,
        destination: UnitDestination::NewMesh,
    })
    .unwrap();
    e.dispatch(Command::InsertShapeUnit(
        UnitShape::Plane,
        UnitDestination::NewMesh,
    ))
    .unwrap();
    run(&mut e, NormalRequest::Transfer { world: true }).unwrap();
    let m = mesh(&e);
    let name = shown_map_name(&m).expect("a normal map");
    assert_eq!(name, DEFAULT_MAP);
    let set = (0..m.faces.len())
        .flat_map(|f| (0..m.faces[f].vertices.len()).map(move |c| (f, c)))
        .filter(|&(f, c)| custom_corner_normal(&m, f, c).is_some())
        .count();
    assert!(set > 0);
}
