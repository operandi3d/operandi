//! UV Unwrap and Pack UVs against spec `uv-unwrap-pack-relax` v1.1 cases.
use glam::{DVec2, DVec3};
use operandi_core::uv_tools::*;
use operandi_core::{Command, Editor, Face, Mesh};

/// The spec's 1 m cube (UVW-10..13): polygon 5 is the top cap.
fn cube() -> Mesh {
    let pts = [
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, -0.5, 0.5],
        [-0.5, -0.5, 0.5],
        [-0.5, 0.5, -0.5],
        [0.5, 0.5, -0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
    ];
    let polys = [
        [0, 1, 2, 3],
        [4, 5, 1, 0],
        [5, 6, 2, 1],
        [6, 7, 3, 2],
        [7, 4, 0, 3],
        [4, 7, 6, 5],
    ];
    Mesh {
        vertices: pts.iter().map(|p| DVec3::from_array(*p)).collect(),
        faces: polys
            .iter()
            .map(|p| Face {
                vertices: p.to_vec(),
                uv: None,
                material: "Default".into(),
                smooth: false,
                part: None,
                material_label: None,
            })
            .collect(),
        ..Default::default()
    }
}

const CAP_SEAMS: [(u32, u32); 8] = [
    (4, 5),
    (5, 6),
    (6, 7),
    (4, 7),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

fn with_texture(mut m: Mesh) -> Mesh {
    let rows = m
        .faces
        .iter()
        .map(|f| vec![Some(DVec2::ZERO); f.vertices.len()])
        .collect();
    m.uv_maps.insert("Texture".into(), rows);
    m
}

fn uvs(m: &Mesh) -> &Vec<Vec<Option<DVec2>>> {
    &m.uv_maps["Texture"]
}

/// UV islands: faces joined across edges with the same UVs at both ends.
fn island_count(m: &Mesh) -> usize {
    let rows = uvs(m);
    let faces: Vec<Vec<(u32, DVec2)>> = m
        .faces
        .iter()
        .zip(rows)
        .map(|(f, r)| {
            f.vertices
                .iter()
                .zip(r)
                .map(|(&p, q)| (p, q.unwrap()))
                .collect()
        })
        .collect();
    operandi_uv::pack::uv_islands(&faces).1
}

fn bounds(m: &Mesh, faces: impl Iterator<Item = usize>) -> (DVec2, DVec2) {
    let (mut lo, mut hi) = (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN));
    for f in faces {
        for p in uvs(m)[f].iter().flatten() {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
    }
    (lo, hi)
}

fn area(r: &[Option<DVec2>]) -> f64 {
    let p: Vec<DVec2> = r.iter().map(|q| q.unwrap()).collect();
    let mut a = 0.0;
    for i in 0..p.len() {
        a += p[i].perp_dot(p[(i + 1) % p.len()]);
    }
    0.5 * a
}

fn add_box(e: &mut Editor) {
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(operandi_core::Tool::Box))
        .unwrap();
    e.dispatch(Command::Preview(operandi_core::Parameters {
        size: DVec3::splat(1.0),
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Apply).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 6);
}

fn no_hidden(m: &Mesh) -> Vec<bool> {
    vec![false; m.faces.len()]
}

#[test]
fn uvw01_defaults() {
    let o = UnwrapOptions::default();
    assert_eq!(o.iterations, 100);
    assert_eq!(o.relative_gap, 0.2);
    assert!(!o.segment_by_material && !o.segment_by_part);
    assert_eq!(o.initial_projection, InitialProjection::Planar);
    assert_eq!(o.pinning_axis, UvAxis::U);
    assert_eq!(o.projection_axis, Axis3::Y);
    assert_eq!(o.offset_angle, 0.0);
    assert!(o.seal_holes && o.update_layout);
    assert_eq!(o.method, UnwrapSolver::Conformal);
    assert!(!o.symmetrize);
    assert_eq!((o.symmetry_axis, o.mirror_axis), (Axis3::X, UvAxis::U));
    assert_eq!(o.seams, SeamSource::SelectedEdges);
    assert!(!InitialProjection::UseCurrentMap.uses_axis());
    assert!(!InitialProjection::GroupNormal.uses_axis());
}

#[test]
fn uvw02_iterations_zero_is_the_planar_projection_fitted() {
    let m = with_texture(cube());
    let seams = [(0, 4), (1, 5), (2, 6), (3, 7)];
    let o = UnwrapOptions {
        iterations: 0,
        relative_gap: 0.0,
        ..Default::default()
    };
    let out = unwrap_mesh(&m, "Texture", &no_hidden(&m), &seams, &o).unwrap();
    // Planar along Y: U from X, V from -Z, then fitted into 0..1.
    let (lo, hi) = bounds(&out, 0..6);
    assert!(lo.min_element() >= -1e-9 && hi.max_element() <= 1.0 + 1e-9);
    for (f, face) in m.faces.iter().enumerate() {
        for (c, &v) in face.vertices.iter().enumerate() {
            let p = m.vertices[v as usize];
            let uv = uvs(&out)[f][c].unwrap();
            // Same point, same place: no unwrapping happened.
            let want = DVec2::new(p.x + 0.5, -p.z + 0.5);
            assert!((uv - want).length() < 1e-5, "{uv} vs {want}");
        }
    }
}

#[test]
fn uvw03_seams_split_uvs_and_layout_is_inside_the_tile() {
    let m = with_texture(cube());
    let seams = [
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
        (4, 5),
        (5, 6),
        (6, 7),
        (4, 7),
    ];
    let out = unwrap_mesh(
        &m,
        "Texture",
        &no_hidden(&m),
        &seams,
        &UnwrapOptions::default(),
    )
    .unwrap();
    assert_eq!(island_count(&out), 2);
    let (lo, hi) = bounds(&out, 0..6);
    assert!(
        lo.min_element() > 0.0 && hi.max_element() < 1.0,
        "{lo} {hi}"
    );
    // A seam edge's end has two UV positions.
    let at = |f: usize, p: u32| {
        let i = m.faces[f].vertices.iter().position(|&v| v == p).unwrap();
        uvs(&out)[f][i].unwrap()
    };
    // Point 5 is in the cap (5) and side 1; edge 4-5 is a seam.
    assert!((at(5, 5) - at(1, 5)).length() > 1e-4);
}

#[test]
fn uvw04_hidden_polygons_keep_their_uvs() {
    let mut m = with_texture(cube());
    let before: Vec<Option<DVec2>> = (0..4).map(|i| Some(DVec2::new(i as f64, 7.0))).collect();
    m.uv_maps.get_mut("Texture").unwrap()[5] = before.clone();
    let mut hidden = no_hidden(&m);
    hidden[5] = true;
    let out = unwrap_mesh(
        &m,
        "Texture",
        &hidden,
        &CAP_SEAMS,
        &UnwrapOptions::default(),
    )
    .unwrap();
    assert_eq!(uvs(&out)[5], before);
    assert!(uvs(&out)[0].iter().all(|p| p.unwrap().max_element() <= 1.0));
}

#[test]
fn uvw06_offset_angle_rotates_the_islands() {
    let m = with_texture(cube());
    let a = UnwrapOptions {
        relative_gap: 0.0,
        ..Default::default()
    };
    let b = UnwrapOptions {
        offset_angle: 90.0,
        ..a.clone()
    };
    let r0 = unwrap_mesh(&m, "Texture", &no_hidden(&m), &CAP_SEAMS, &a).unwrap();
    let r1 = unwrap_mesh(&m, "Texture", &no_hidden(&m), &CAP_SEAMS, &b).unwrap();
    // Edge 0-1 of the bottom turns a quarter.
    let d = |m: &Mesh| uvs(m)[0][1].unwrap() - uvs(m)[0][0].unwrap();
    let (d0, d1) = (d(&r0), d(&r1));
    assert!(d0.dot(d1).abs() < 1e-6 * d0.length() * d1.length());
}

#[test]
fn uvw07_one_undo_step() {
    let mut e = Editor::default();
    add_box(&mut e);
    let before = e.scene.active().unwrap().mesh.uv_maps.clone();
    let steps = e.history.undo.len();
    e.dispatch(Command::UvUnwrap(UnwrapOptions::default()))
        .unwrap();
    assert_eq!(e.history.undo.len(), steps + 1);
    assert_ne!(e.scene.active().unwrap().mesh.uv_maps, before);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.uv_maps, before);
}

#[test]
fn uvw10_closed_mesh_without_seams_gives_one_island_per_polygon() {
    let m = with_texture(cube());
    let out = unwrap_mesh(
        &m,
        "Texture",
        &no_hidden(&m),
        &[],
        &UnwrapOptions::default(),
    )
    .unwrap();
    assert_eq!(island_count(&out), 6);
    let (lo, hi) = bounds(&out, 0..6);
    assert!(lo.min_element() >= 0.0 && hi.max_element() <= 1.0);
}

#[test]
fn uvw11_cap_and_cross_layout_with_no_gap() {
    let m = with_texture(cube());
    let o = UnwrapOptions {
        relative_gap: 0.0,
        ..Default::default()
    };
    let out = unwrap_mesh(&m, "Texture", &no_hidden(&m), &CAP_SEAMS, &o).unwrap();
    assert_eq!(island_count(&out), 2);
    let (lo, hi) = bounds(&out, 0..6);
    let tol = 1e-4;
    assert!(lo.length() < tol, "{lo}");
    assert!(
        (hi.x - 1.0).abs() < tol && (hi.y - 0.75).abs() < tol,
        "{hi}"
    );
    for f in 0..6 {
        assert!((area(&uvs(&out)[f]).abs() - 0.0625).abs() < tol);
    }
}

#[test]
fn uvw12_polygon_selection_does_not_limit_unwrap() {
    let mut e = Editor::default();
    add_box(&mut e);
    e.dispatch(Command::Mode(operandi_core::Mode::Polygon))
        .unwrap();
    e.scene.selection.faces = vec![0];
    let mesh = &e.scene.active().unwrap().mesh;
    let before = mesh.uv_maps.get("Texture").cloned();
    let n = mesh.faces.len();
    e.dispatch(Command::UvUnwrap(UnwrapOptions::default()))
        .unwrap();
    let after = &e.scene.active().unwrap().mesh.uv_maps["Texture"];
    let changed = (0..n)
        .filter(|&f| before.as_ref().is_none_or(|b| b[f] != after[f]))
        .count();
    assert!(changed > 1, "only {changed} polygons changed");
}

#[test]
fn uvw13_no_uv_map_creates_texture() {
    let m = cube();
    assert_eq!(target_uv_map(&m, None), "Texture");
    let out = unwrap_mesh(
        &m,
        "Texture",
        &no_hidden(&m),
        &CAP_SEAMS,
        &UnwrapOptions::default(),
    )
    .unwrap();
    assert!(out.uv_maps.contains_key("Texture"));
    assert_eq!(island_count(&out), 2);
}

#[test]
fn segment_by_material_cuts_between_materials() {
    let mut m = with_texture(cube());
    m.faces[0].material = "Red".into();
    let o = UnwrapOptions {
        segment_by_material: true,
        ..Default::default()
    };
    let out = unwrap_mesh(&m, "Texture", &no_hidden(&m), &CAP_SEAMS, &o).unwrap();
    // The red bottom comes away, and with it the sides' only link.
    assert_eq!(island_count(&out), 6);
}

/// Two unit-square islands at (2, 2) and (-3, 0) (UVP-01).
fn two_squares(size: f64) -> Mesh {
    let mut m = Mesh::default();
    let mut rows = Vec::new();
    for (k, o) in [DVec2::new(2.0, 2.0), DVec2::new(-3.0, 0.0)]
        .into_iter()
        .enumerate()
    {
        let base = 4 * k as u32;
        for (dx, dz) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
            m.vertices.push(DVec3::new(dx + 5.0 * k as f64, 0.0, dz));
        }
        m.faces.push(Face {
            vertices: vec![base, base + 1, base + 2, base + 3],
            uv: None,
            material: "Default".into(),
            smooth: false,
            part: None,
            material_label: None,
        });
        rows.push(
            [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
                .iter()
                .map(|&(x, y)| Some(o + DVec2::new(x, y) * size))
                .collect(),
        );
    }
    m.uv_maps.insert("Texture".into(), rows);
    m
}

fn quads_overlap(a: &[Option<DVec2>], b: &[Option<DVec2>]) -> bool {
    let bb = |r: &[Option<DVec2>]| {
        let (mut lo, mut hi) = (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN));
        for p in r.iter().flatten() {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        (lo, hi)
    };
    let ((al, ah), (bl, bh)) = (bb(a), bb(b));
    al.x + 1e-9 < bh.x && bl.x + 1e-9 < ah.x && al.y + 1e-9 < bh.y && bl.y + 1e-9 < ah.y
}

#[test]
fn pack_starting_values() {
    let o = PackOptions::default();
    assert!(o.pack && o.stretch && !o.orient && !o.keep_size);
    assert_eq!(o.direction, PackDirection::Auto);
    assert_eq!((o.gaps, o.pixel_size), (0.2, 10.24));
    assert!(!o.gaps_by_pixel && !o.bounding_box && !o.lock_stacked);
    assert_eq!((o.region, o.udim), (PackRegion::Normalized, 1001));
    assert!(o.find_best_box_rotation);
}

#[test]
fn uvp01_pack_into_the_tile() {
    let m = two_squares(1.0);
    let o = PackOptions {
        orient: true,
        ..Default::default()
    };
    let out = pack_mesh(&m, "Texture", &no_hidden(&m), &[], &o).unwrap();
    let (lo, hi) = bounds(&out, 0..2);
    assert!(
        lo.min_element() >= 0.0 && hi.max_element() <= 1.0,
        "{lo} {hi}"
    );
    assert!(!quads_overlap(&uvs(&out)[0], &uvs(&out)[1]));
    assert!((area(&uvs(&out)[0]).abs() - area(&uvs(&out)[1]).abs()).abs() < 1e-5);
}

#[test]
fn uvp02_keep_size() {
    let m = two_squares(0.25);
    let o = PackOptions {
        keep_size: true,
        ..Default::default()
    };
    let out = pack_mesh(&m, "Texture", &no_hidden(&m), &[], &o).unwrap();
    for f in 0..2 {
        assert!((area(&uvs(&out)[f]).abs() - 0.0625).abs() < 1e-5);
    }
    assert!(!quads_overlap(&uvs(&out)[0], &uvs(&out)[1]));
}

#[test]
fn uvp03_and_04_stacked_islands() {
    let mut m = two_squares(1.0);
    let first = uvs(&m)[0].clone();
    m.uv_maps.get_mut("Texture").unwrap()[1] = first;
    let out = pack_mesh(&m, "Texture", &no_hidden(&m), &[], &PackOptions::default()).unwrap();
    assert!(!quads_overlap(&uvs(&out)[0], &uvs(&out)[1]), "UVP-03");
    let o = PackOptions {
        lock_stacked: true,
        ..Default::default()
    };
    let out = pack_mesh(&m, "Texture", &no_hidden(&m), &[], &o).unwrap();
    for (a, b) in uvs(&out)[0].iter().zip(&uvs(&out)[1]) {
        assert!((a.unwrap() - b.unwrap()).length() < 1e-6, "UVP-04");
    }
}

#[test]
fn uvp05_manual_region() {
    let m = two_squares(1.0);
    let o = PackOptions {
        region: PackRegion::Manual,
        region_uv: DVec2::splat(-1.0),
        region_size: DVec2::splat(2.0),
        ..Default::default()
    };
    let out = pack_mesh(&m, "Texture", &no_hidden(&m), &[], &o).unwrap();
    let (lo, hi) = bounds(&out, 0..2);
    assert!(lo.min_element() >= -1.0 - 1e-5 && hi.max_element() <= 1.0 + 1e-5);
    assert!(hi.max_element() > 0.5, "uses the region: {hi}");
}

#[test]
fn uvp06_gaps_by_pixel() {
    let m = two_squares(1.0);
    let o = PackOptions {
        gaps_by_pixel: true,
        pixel_size: 20.0,
        image_size: 1024.0,
        ..Default::default()
    };
    let out = pack_mesh(&m, "Texture", &no_hidden(&m), &[], &o).unwrap();
    let (a0, a1) = (bounds(&out, 0..1), bounds(&out, 1..2));
    // Gap along whichever axis they sit side by side.
    let gx = (a1.0.x - a0.1.x).max(a0.0.x - a1.1.x);
    let gy = (a1.0.y - a0.1.y).max(a0.0.y - a1.1.y);
    let gap = gx.max(gy);
    assert!((gap - 20.0 / 1024.0).abs() < 0.002, "{gap}");
}

#[test]
fn uvp08_pack_cube_layout_with_starting_values() {
    let m = with_texture(cube());
    let o = UnwrapOptions {
        relative_gap: 0.0,
        ..Default::default()
    };
    let unwrapped = unwrap_mesh(&m, "Texture", &no_hidden(&m), &CAP_SEAMS, &o).unwrap();
    let out = pack_mesh(
        &unwrapped,
        "Texture",
        &no_hidden(&m),
        &[],
        &PackOptions::default(),
    )
    .unwrap();
    let tol = 0.002;
    // Every face the same size, about 0.3254 wide.
    for f in 0..6 {
        let w = area(&uvs(&out)[f]).abs().sqrt();
        assert!((w - 0.3254).abs() < tol, "face {f} is {w} wide");
    }
    let (lo, hi) = bounds(&out, 0..6);
    assert!(
        (lo.min_element() - 0.01).abs() < tol && (hi.max_element() - 0.99).abs() < tol,
        "{lo} {hi}"
    );
    // The cap tucks into the cross's empty corner.
    let (cl, ch) = bounds(&out, 5..6);
    assert!(
        cl.min_element() > 0.6 && ch.min_element() > 0.9,
        "cap at {cl} {ch}"
    );
}

#[test]
fn uvp09_pack_selected_island_only() {
    let m = with_texture(cube());
    let o = UnwrapOptions {
        relative_gap: 0.0,
        ..Default::default()
    };
    let unwrapped = unwrap_mesh(&m, "Texture", &no_hidden(&m), &CAP_SEAMS, &o).unwrap();
    let packed = pack_mesh(
        &unwrapped,
        "Texture",
        &no_hidden(&m),
        &[],
        &PackOptions::default(),
    )
    .unwrap();
    let out = pack_mesh(
        &packed,
        "Texture",
        &no_hidden(&m),
        &[5],
        &PackOptions::default(),
    )
    .unwrap();
    for f in 0..5 {
        assert_eq!(uvs(&out)[f], uvs(&packed)[f], "cross face {f} moved");
    }
    let cap = &uvs(&out)[5];
    for f in 0..5 {
        assert!(!quads_overlap(cap, &uvs(&out)[f]), "cap overlaps face {f}");
    }
    let (lo, hi) = bounds(&out, 5..6);
    assert!(lo.min_element() >= 0.0 && hi.max_element() <= 1.0);
}

#[test]
fn refusals() {
    let m = cube();
    assert!(pack_mesh(&m, "Texture", &no_hidden(&m), &[], &PackOptions::default()).is_err());
    // Every Pack Region is built since the UV tools finish (2026-11-18).
    let o = PackOptions {
        region: PackRegion::ByItem,
        ..Default::default()
    };
    assert!(o.validate().is_ok());
    let o = UnwrapOptions {
        relative_gap: -1.0,
        ..Default::default()
    };
    assert!(o.validate().is_err());
}
