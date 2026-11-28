//! UV stage 4 and the rest of stage 3 (BEHAVIOR-UV-TOOLS §8, §10, §13,
//! §14; UVS-1 §1.6, §2): seam maps and Cut Map, Seam Marker, UV Symmetry
//! and Symmetrize, the UDIM tile buttons and the remaining Pack regions.
use glam::{DVec2, DVec3};
use operandi_core::uv_edit::UvAxis;
use operandi_core::uv_seams::{SeamEdit, seam_edges, seam_maps};
use operandi_core::uv_symmetry::{SymmetrizeOptions, UvSymmetry};
use operandi_core::uv_tools::{PackOptions, PackRegion, PackTag, SeamSource};
use operandi_core::*;

fn face(v: &[u32]) -> Face {
    Face {
        vertices: v.to_vec(),
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

fn editor_with(mesh: Mesh, mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(mode)).unwrap();
    e
}

fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

fn cube() -> Mesh {
    let mut m = primitive::segmented_cube(DVec3::ZERO, DVec3::splat(2.), [1; 3]).unwrap();
    m.uv_maps.clear();
    m
}

/// A row of `n` unit quads along X centred on X = 0, with UVs (x+n/2)/n.
fn row(n: u32) -> Mesh {
    let mut m = Mesh::default();
    let h = n as f64 / 2.;
    for i in 0..=n {
        m.vertices.push(DVec3::new(i as f64 - h, 0., 0.));
        m.vertices.push(DVec3::new(i as f64 - h, 0., 1.));
    }
    let mut rows = Vec::new();
    for i in 0..n {
        let (a, b, c, d) = (2 * i, 2 * i + 2, 2 * i + 3, 2 * i + 1);
        m.faces.push(face(&[a, b, c, d]));
        let u = |k: u32| (k as f64) / n as f64;
        rows.push(vec![
            Some(DVec2::new(u(i), 0.)),
            Some(DVec2::new(u(i + 1), 0.)),
            Some(DVec2::new(u(i + 1), 0.5)),
            Some(DVec2::new(u(i), 0.5)),
        ]);
    }
    m.uv_maps.insert("Texture".into(), rows);
    m.attributes.current_uv_map = Some("Texture".into());
    m
}

#[test]
fn seam_maps_add_select_clear_delete_and_stay_out_of_selection_sets() {
    let mut e = editor_with(cube(), Mode::Edge);
    let edge = mesh(&e).edges()[0];
    e.scene.selection.edges = vec![edge];
    e.dispatch(Command::UvSeam(SeamEdit::Add("Body".into())))
        .unwrap();
    assert_eq!(seam_maps(mesh(&e)), vec!["Body".to_string()]);
    assert_eq!(seam_edges(mesh(&e), "Body").len(), 1);
    assert!(
        selection_sets::set_names(mesh(&e), SetKind::Edge).is_empty(),
        "a seam map isn't an edge selection set"
    );
    // With no edges selected, Add Seam adds every edge (UVS-1 §1.6).
    e.scene.selection.edges.clear();
    e.dispatch(Command::UvSeam(SeamEdit::Add("All".into())))
        .unwrap();
    assert_eq!(seam_edges(mesh(&e), "All").len(), 12);
    e.dispatch(Command::UvSeam(SeamEdit::Select("Body".into())))
        .unwrap();
    assert_eq!(e.scene.selection.edges, vec![edge]);
    e.dispatch(Command::UvSeam(SeamEdit::Clear("Body".into())))
        .unwrap();
    assert!(seam_edges(mesh(&e), "Body").is_empty());
    e.dispatch(Command::UvSeam(SeamEdit::Delete("All".into())))
        .unwrap();
    assert!(!seam_maps(mesh(&e)).contains(&"All".to_string()));
    e.dispatch(Command::Undo).unwrap();
    assert!(
        seam_maps(mesh(&e)).contains(&"All".to_string()),
        "one undo step"
    );
}

#[test]
fn unwrap_cuts_at_a_seam_map() {
    // A cube's seams as a map: the top ring and one vertical, so the unwrap
    // makes islands without any edge selected.
    let mut e = editor_with(cube(), Mode::Edge);
    let m = mesh(&e).clone();
    let top: Vec<(u32, u32)> = m
        .edges()
        .iter()
        .copied()
        .filter(|&(a, b)| m.vertices[a as usize].y > 0.5 && m.vertices[b as usize].y > 0.5)
        .collect();
    e.scene.selection.edges = top;
    e.dispatch(Command::UvSeam(SeamEdit::Add("UV Seam".into())))
        .unwrap();
    e.scene.selection.edges.clear();
    let mut o = uv_tools::UnwrapOptions {
        seams: SeamSource::SeamMap("UV Seam".into()),
        ..Default::default()
    };
    o.iterations = 10;
    e.dispatch(Command::UvUnwrap(o.clone())).unwrap();
    let with_map = mesh(&e).uv_maps["Texture"].clone();
    // Without the map (and no edges selected) a closed cube becomes six
    // islands: a different layout.
    let mut plain = editor_with(cube(), Mode::Edge);
    plain
        .dispatch(Command::UvUnwrap(uv_tools::UnwrapOptions {
            iterations: 10,
            ..Default::default()
        }))
        .unwrap();
    assert_ne!(with_map, mesh(&plain).uv_maps["Texture"]);
}

#[test]
fn seam_marker_marks_the_loop_from_the_selected_edge() {
    // A tube (8 sides, 3 rings): a ring edge's loop goes all round.
    let mut m = Mesh::default();
    for ring in 0..3 {
        for i in 0..8 {
            let a = std::f64::consts::TAU * i as f64 / 8.;
            m.vertices.push(DVec3::new(a.cos(), ring as f64, a.sin()));
        }
    }
    for ring in 0..2u32 {
        for i in 0..8u32 {
            let j = (i + 1) % 8;
            m.faces.push(face(&[
                ring * 8 + i,
                ring * 8 + j,
                ring * 8 + 8 + j,
                ring * 8 + 8 + i,
            ]));
        }
    }
    let mut e = editor_with(m, Mode::Edge);
    e.scene.selection.edges = vec![(8, 9)];
    e.dispatch(Command::UvSeam(SeamEdit::Marker("UV Seam".into())))
        .unwrap();
    assert_eq!(seam_edges(mesh(&e), "UV Seam").len(), 8, "the middle ring");
    assert_eq!(e.scene.selection.edges, vec![(8, 9)], "the selection stays");
}

#[test]
fn uv_symmetry_mirrors_a_uv_edit_to_its_partner() {
    // Four quads; UVs symmetric about U = 0.5.
    let mut e = editor_with(row(4), Mode::Vertex);
    e.dispatch(Command::UvSymmetry(UvSymmetry {
        axis: Some(UvAxis::U),
        center: 0.5,
    }))
    .unwrap();
    // Point 2 sits at U 0.25; its partner (point 6) at U 0.75.
    e.scene.selection.vertices = vec![2];
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::UvToolSpace(Some(None))).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = DVec3::new(-0.1, 0.05, 0.);
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let r = &mesh(&e).uv_maps["Texture"];
    assert!(
        (r[0][1].unwrap() - DVec2::new(0.15, 0.05)).length() < 1e-9,
        "{r:?}"
    );
    assert!(
        (r[3][0].unwrap() - DVec2::new(0.85, 0.05)).length() < 1e-9,
        "mirrored: {r:?}"
    );
}

#[test]
fn center_welds_the_uvs_on_the_line_and_symmetrize_copies_across() {
    let mut m = row(2);
    // Pull the middle column's two sides slightly apart.
    let mut rows = m.uv_maps["Texture"].clone();
    rows[0][1] = Some(DVec2::new(0.498, 0.));
    rows[0][2] = Some(DVec2::new(0.498, 0.5));
    rows[1][0] = Some(DVec2::new(0.502, 0.));
    rows[1][3] = Some(DVec2::new(0.502, 0.5));
    m.uv_maps.insert("Texture".into(), rows);
    let mut e = editor_with(m, Mode::Polygon);
    e.dispatch(Command::UvSymmetry(UvSymmetry {
        axis: Some(UvAxis::U),
        center: 0.5,
    }))
    .unwrap();
    e.dispatch(Command::UvSymmetryCenter).unwrap();
    let r = mesh(&e).uv_maps["Texture"].clone();
    assert_eq!(r[0][1], r[1][0], "one UV position on the line");
    assert!((r[0][1].unwrap().x - 0.5).abs() < 1e-12);
    // Symmetrize: wreck the -X side's UVs, then copy them back from +X.
    let mut rows = r.clone();
    rows[0] = vec![Some(DVec2::splat(0.9)); 4];
    let mut m = (*mesh(&e)).clone();
    m.uv_maps.insert("Texture".into(), rows);
    e.scene.active_mut().unwrap().mesh = m.into();
    e.dispatch(Command::UvSymmetrize(SymmetrizeOptions::default()))
        .unwrap();
    let after = &mesh(&e).uv_maps["Texture"];
    assert_eq!(after[0], r[0], "{after:?}");
}

#[test]
fn select_tile_and_nudge_move_islands_a_tile_at_a_time() {
    let mut e = editor_with(row(2), Mode::Polygon);
    e.scene.selection.faces = vec![0, 1];
    e.dispatch(Command::UvOp(uv_ops::UvOp::NudgeTile { du: 1, dv: 0 }))
        .unwrap();
    assert!((mesh(&e).uv_maps["Texture"][0][0].unwrap() - DVec2::new(1., 0.)).length() < 1e-12);
    e.scene.selection.faces.clear();
    e.dispatch(Command::UvOp(uv_ops::UvOp::SelectTile { udim: 1002 }))
        .unwrap();
    assert_eq!(e.scene.selection.faces, vec![0, 1]);
}

fn islands_tiles(m: &Mesh) -> Vec<(i64, i64)> {
    m.uv_maps["Texture"]
        .iter()
        .map(|r| {
            let c = r.iter().flatten().sum::<DVec2>() / r.len() as f64;
            (c.x.floor() as i64, c.y.floor() as i64)
        })
        .collect()
}

#[test]
fn pack_nearest_keeps_each_island_in_its_own_tile() {
    let mut m = row(2);
    // Split the two quads into separate islands in tiles 1001 and 1003.
    let mut rows = m.uv_maps["Texture"].clone();
    for p in rows[1].iter_mut().flatten() {
        p.x += 2.;
    }
    m.uv_maps.insert("Texture".into(), rows);
    let mut e = editor_with(m, Mode::Polygon);
    e.dispatch(Command::PackUvs(PackOptions {
        region: PackRegion::Nearest,
        ..Default::default()
    }))
    .unwrap();
    assert_eq!(islands_tiles(mesh(&e)), vec![(0, 0), (2, 0)]);
}

#[test]
fn pack_polygon_tag_gives_each_material_a_tile_and_new_map_keeps_the_old() {
    let mut m = row(2);
    m.faces[1].material = "Metal".into();
    let before = m.uv_maps["Texture"].clone();
    let mut e = editor_with(m, Mode::Polygon);
    e.dispatch(Command::PackUvs(PackOptions {
        region: PackRegion::PolygonTag,
        polygon_tag: PackTag::Material,
        write_new: true,
        new_name: "Packed".into(),
        ..Default::default()
    }))
    .unwrap();
    let m = mesh(&e);
    assert_eq!(m.uv_maps["Texture"], before, "the original stays");
    let tiles: Vec<(i64, i64)> = m.uv_maps["Packed"]
        .iter()
        .map(|r| {
            let c = r.iter().flatten().sum::<DVec2>() / 4.;
            (c.x.floor() as i64, c.y.floor() as i64)
        })
        .collect();
    assert_eq!(tiles, vec![(0, 0), (1, 0)], "Default then Metal");
    assert_eq!(m.attributes.current_uv_map.as_deref(), Some("Packed"));
}

#[test]
fn pack_background_keeps_clear_of_another_meshs_islands() {
    let mut e = editor_with(row(1), Mode::Polygon);
    // A second mesh whose island fills the tile's left half.
    let mut other = row(1);
    other.uv_maps.insert(
        "Texture".into(),
        vec![vec![
            Some(DVec2::new(0., 0.)),
            Some(DVec2::new(0.5, 0.)),
            Some(DVec2::new(0.5, 1.)),
            Some(DVec2::new(0., 1.)),
        ]],
    );
    let first = e.scene.active().unwrap().id;
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = other.into();
    e.dispatch(Command::SelectItem(Some(first))).unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.dispatch(Command::PackUvs(PackOptions {
        background: true,
        ..Default::default()
    }))
    .unwrap();
    let r = &mesh(&e).uv_maps["Texture"][0];
    let lo = r.iter().flatten().fold(f64::MAX, |m, p| m.min(p.x));
    assert!(
        lo >= 0.5 - 1e-6,
        "packed to the right of the other island: {r:?}"
    );
}
