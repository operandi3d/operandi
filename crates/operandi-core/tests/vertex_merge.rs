//! Vertex batch 2: Merge (tool and command) and Join options, spec
//! `vertex-tools-2026-11-14` v1.3 (VX-M01..M06, VX-C01, G04, G06, G13).
use glam::{DVec2, DVec3};
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

fn xz(x: f64, z: f64) -> DVec3 {
    DVec3::new(x, 0., z)
}

fn mesh(points: Vec<DVec3>, faces: Vec<Vec<u32>>) -> Mesh {
    let m = Mesh {
        vertices: points,
        faces: faces.into_iter().map(face).collect::<Vec<_>>().into(),
        ..Default::default()
    };
    m.validate().unwrap();
    m
}

/// Three triangles, one corner each near (-0.5, -0.5): points 0, 3 and 6.
fn three_near() -> Mesh {
    mesh(
        vec![
            xz(-0.5, -0.5),
            xz(-2., -0.5),
            xz(-2., -2.),
            xz(-0.45, -0.5),
            xz(1., -2.),
            xz(1., -0.5),
            xz(-0.5, -0.42),
            xz(-2., 1.),
            xz(-0.5, 1.),
        ],
        vec![vec![0, 1, 2], vec![3, 4, 5], vec![6, 7, 8]],
    )
}

fn merge(m: &Mesh, sel: &[u32], distance: f64, options: MergeOptions) -> (Mesh, Vec<u32>) {
    let (out, after) =
        vertex_command_geometry(m, sel, VertexCommand::Merge { distance, options }).unwrap();
    out.validate().unwrap();
    (out, after)
}

fn has(m: &Mesh, at: DVec3, tol: f64) -> bool {
    m.vertices.iter().any(|v| v.distance(at) <= tol)
}

#[test]
fn merged_points_land_at_their_average_and_nothing_stays_selected() {
    // VX-M01, VX-M06: command and tool alike.
    let want = xz(-0.48333, -0.47333);
    let (out, after) = merge(&three_near(), &[0, 3, 6], 0.1, MergeOptions::default());
    assert_eq!(out.vertices.len(), 7);
    assert!(has(&out, want, 1e-4));
    assert!(after.is_empty());
    let (out, after) = vertex_merge_geometry(&three_near(), &[6, 0, 3], 0.1).unwrap();
    assert!(has(&out, want, 1e-4));
    assert!(after.is_empty());
    // Nothing selected: every point is considered, also at the average.
    let (out, _) = merge(&three_near(), &[], 0.1, MergeOptions::default());
    assert!(has(&out, want, 1e-4));
}

#[test]
fn groups_chain_and_distance_is_at_most() {
    // VX-M02: points 1 apart chain through the middle one.
    let m = mesh(
        vec![
            xz(0., 0.),
            xz(-1., 0.),
            xz(-1., -1.),
            xz(1., 0.),
            xz(2., 0.),
            xz(2., -1.),
            xz(1., 1.),
            xz(2., 2.),
            xz(1., 2.),
        ],
        vec![vec![0, 1, 2], vec![3, 4, 5], vec![6, 7, 8]],
    );
    let (out, _) = merge(&m, &[0, 3, 6], 1.0, MergeOptions::default());
    assert_eq!(out.vertices.len(), 7);
    assert!(has(&out, xz(2. / 3., 1. / 3.), 1e-12));
    let (out, _) = merge(&m, &[0, 3, 6], 0.999, MergeOptions::default());
    assert_eq!(out.vertices.len(), 9);
}

#[test]
fn only_selected_points_merge() {
    // VX-M03: an unselected point 0.05 from a selected one stays.
    let (out, _) = merge(&three_near(), &[0, 6], 0.1, MergeOptions::default());
    assert_eq!(out.vertices.len(), 8);
    assert!(has(&out, xz(-0.45, -0.5), 1e-12));
}

#[test]
fn keep_one_vertex_polygons_keeps_squashed_polygons() {
    // VX-M05: merging a triangle's corners squashes it.
    let tri = || {
        mesh(
            vec![
                xz(0., 0.),
                xz(0.01, 0.),
                xz(0., 1.),
                xz(5., 5.),
                xz(6., 5.),
                xz(5., 6.),
            ],
            vec![vec![0, 1, 2], vec![3, 4, 5]],
        )
    };
    let keep = MergeOptions {
        keep_small: true,
        ..Default::default()
    };
    // Two corners merged: removed by default, a two-point line when kept.
    let (off, _) = merge(&tri(), &[0, 1], 0.1, MergeOptions::default());
    assert_eq!(off.faces.len(), 1);
    let (on, _) = merge(&tri(), &[0, 1], 0.1, keep);
    assert_eq!(on.faces.len(), 2);
    let line = on.faces.iter().position(|f| f.vertices.len() == 2).unwrap();
    assert!(on.is_polyline(line));
    // All three merged: Operandi has no one-point polygons yet, so the
    // remnant goes either way (provisional, VXM-Q2).
    let (on, _) = merge(&tri(), &[0, 1, 2], 1.5, keep);
    assert_eq!(on.faces.len(), 1);
    // Join keeps them the same way (VX-C01).
    let (on, _) = vertex_command_geometry(
        &tri(),
        &[0, 1],
        VertexCommand::Join {
            average: false,
            options: keep,
        },
    )
    .unwrap();
    assert_eq!(on.faces.len(), 2);
}

#[test]
fn test_morph_map_positions_also_compares_morphs() {
    // VX-M05: points 0 and 3 coincide, but a morph pulls point 3 away.
    let mut m = mesh(
        vec![
            xz(0., 0.),
            xz(-1., 0.),
            xz(-1., -1.),
            xz(0., 0.),
            xz(1., 0.),
            xz(1., -1.),
        ],
        vec![vec![0, 1, 2], vec![3, 4, 5]],
    );
    m.attributes
        .relative_morphs
        .insert("Smile".into(), [(3u32, DVec3::new(0., 0.5, 0.))].into());
    m.validate().unwrap();
    let (plain, _) = merge(&m, &[0, 3], 0.01, MergeOptions::default());
    assert_eq!(plain.vertices.len(), 5);
    let test = MergeOptions {
        test_morphs: true,
        ..Default::default()
    };
    let (tested, _) = merge(&m, &[0, 3], 0.01, test);
    assert_eq!(tested.vertices.len(), 6);
    // Within Distance in the morph too: they merge.
    let (far, _) = merge(&m, &[0, 3], 0.6, test);
    assert_eq!(far.vertices.len(), 5);
}

/// Two quads side by side with separate points on their shared side (1 = 4,
/// 2 = 7), a UV map whose values match there, and one UV record per corner.
fn uv_seam() -> Mesh {
    let mut m = mesh(
        vec![
            xz(0., 0.),
            xz(1., 0.),
            xz(1., 1.),
            xz(0., 1.),
            xz(1., 0.),
            xz(2., 0.),
            xz(2., 1.),
            xz(1., 1.),
        ],
        vec![vec![0, 3, 2, 1], vec![4, 7, 6, 5]],
    );
    let uv = |p: DVec3| Some(DVec2::new(p.x / 2., p.z));
    let rows: Vec<Vec<Option<DVec2>>> = m
        .faces
        .iter()
        .map(|f| {
            f.vertices
                .iter()
                .map(|&v| uv(m.vertices[v as usize]))
                .collect()
        })
        .collect();
    let records = vec![
        vec![Some(0), Some(1), Some(2), Some(3)],
        vec![Some(4), Some(5), Some(6), Some(7)],
    ];
    let mut maps = (*m.uv_maps).clone();
    maps.insert("Texture".into(), rows);
    m.uv_maps = maps.into();
    m.attributes.uv_records.insert("Texture".into(), records);
    m.validate().unwrap();
    m
}

/// The UV records the corners at point `v` use.
fn records_at(m: &Mesh, v: u32) -> Vec<u32> {
    let mut out = Vec::new();
    for (f, face) in m.faces.iter().enumerate() {
        for (c, &w) in face.vertices.iter().enumerate() {
            if w == v {
                out.push(m.attributes.uv_records["Texture"][f][c].unwrap());
            }
        }
    }
    out
}

#[test]
fn merge_discontinuous_uv_values_shares_matching_uvs() {
    // VX-M05 (provisional VXM-Q1): matching UVs at a merged point share one
    // UV; off, each corner keeps its own.
    let on = MergeOptions::default();
    assert!(on.merge_uvs);
    let (out, _) = merge(&uv_seam(), &[], 0., on);
    assert_eq!(out.vertices.len(), 6);
    let joined = out
        .vertices
        .iter()
        .position(|v| v.distance(xz(1., 0.)) < 1e-12)
        .unwrap() as u32;
    let r = records_at(&out, joined);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0], r[1]);
    let off = MergeOptions {
        merge_uvs: false,
        ..Default::default()
    };
    let (out, _) = merge(&uv_seam(), &[], 0., off);
    let r = records_at(&out, joined);
    assert_ne!(r[0], r[1]);
    // Different UVs stay apart even with the option on.
    let mut split = uv_seam();
    let mut maps = (*split.uv_maps).clone();
    maps.get_mut("Texture").unwrap()[1][0] = Some(DVec2::new(0.9, 0.));
    split.uv_maps = maps.into();
    let (out, _) = merge(&split, &[], 0., on);
    let r = records_at(&out, joined);
    assert_ne!(r[0], r[1]);
}

fn editor(mesh: Mesh, points: Vec<u32>) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.synchronize_identities().unwrap();
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = points;
    e
}

#[test]
fn the_merge_tool_starts_at_its_values_merges_at_the_average_and_undoes() {
    let mut e = editor(three_near(), vec![0, 3, 6]);
    let before = e.scene.clone();
    e.dispatch(Command::Activate(Tool::VertexMerge)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    // G13: Distance 0.001, both options off.
    assert_eq!(
        (
            p.shift,
            p.vertex_merge_keep_small,
            p.vertex_merge_test_morphs
        ),
        (0.001, false, false)
    );
    p.shift = 0.1;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let m = (*e.scene.active().unwrap().mesh).clone();
    assert!(has(&m, xz(-0.48333, -0.47333), 1e-4));
    assert!(e.scene.selection.vertices.is_empty());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(
        e.scene.active().unwrap().mesh,
        before.active().unwrap().mesh
    );
}

#[test]
fn the_merge_tool_keeps_squashed_polygons_when_asked() {
    let tri = mesh(
        vec![
            xz(0., 0.),
            xz(0.01, 0.),
            xz(0., 1.),
            xz(5., 5.),
            xz(6., 5.),
            xz(5., 6.),
        ],
        vec![vec![0, 1, 2], vec![3, 4, 5]],
    );
    let mut e = editor(tri, vec![0, 1]);
    e.dispatch(Command::Activate(Tool::VertexMerge)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.shift = 0.1;
    p.vertex_merge_keep_small = true;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 2);
}
