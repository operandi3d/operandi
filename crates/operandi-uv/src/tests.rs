// SPDX-License-Identifier: GPL-2.0-or-later
//! Tests for unwrapping. Fixtures are built here; nothing is read from disk.

use super::*;

pub(crate) fn corner(p: u32) -> Corner {
    Corner::new(p, DVec2::ZERO)
}

/// An `n` x `n` grid of quads in the XY plane, size 1, optionally bent.
pub(crate) fn grid(n: u32, bend: f64) -> UvMesh {
    let mut points = Vec::new();
    for j in 0..=n {
        for i in 0..=n {
            let x = i as f64 / n as f64;
            let y = j as f64 / n as f64;
            points.push(DVec3::new(x, y, bend * (x * 3.0).sin()));
        }
    }
    let id = |i: u32, j: u32| j * (n + 1) + i;
    let mut faces = Vec::new();
    for j in 0..n {
        for i in 0..n {
            faces.push(vec![
                corner(id(i, j)),
                corner(id(i + 1, j)),
                corner(id(i + 1, j + 1)),
                corner(id(i, j + 1)),
            ]);
        }
    }
    UvMesh {
        points,
        faces,
        ..Default::default()
    }
}

/// A unit cube, faces outward.
pub(crate) fn cube() -> UvMesh {
    let points = (0..8)
        .map(|i| DVec3::new((i & 1) as f64, ((i >> 1) & 1) as f64, ((i >> 2) & 1) as f64))
        .collect();
    let quads = [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ];
    UvMesh {
        points,
        faces: quads
            .iter()
            .map(|q| q.iter().map(|&p| corner(p)).collect())
            .collect(),
        ..Default::default()
    }
}

/// An open cylinder (tube) of `n` sides and `rings` rows, radius 1, height 2.
pub(crate) fn tube(n: u32, rings: u32) -> UvMesh {
    let mut points = Vec::new();
    for r in 0..=rings {
        for i in 0..n {
            let a = i as f64 / n as f64 * std::f64::consts::TAU;
            points.push(DVec3::new(a.cos(), a.sin(), 2.0 * r as f64 / rings as f64));
        }
    }
    let id = |i: u32, r: u32| r * n + (i % n);
    let mut faces = Vec::new();
    for r in 0..rings {
        for i in 0..n {
            faces.push(vec![
                corner(id(i, r)),
                corner(id(i + 1, r)),
                corner(id(i + 1, r + 1)),
                corner(id(i, r + 1)),
            ]);
        }
    }
    let seams = (0..rings).map(|r| [id(0, r), id(0, r + 1)]).collect();
    UvMesh {
        points,
        faces,
        seams,
        ..Default::default()
    }
}

pub(crate) fn signed_area(uv: &[DVec2]) -> f64 {
    let mut a = 0.0;
    for i in 0..uv.len() {
        a += uv[i].perp_dot(uv[(i + 1) % uv.len()]);
    }
    0.5 * a
}

fn all_same_winding(uvs: &[Vec<DVec2>]) -> bool {
    let signs: Vec<f64> = uvs.iter().map(|f| signed_area(f).signum()).collect();
    signs.iter().all(|&s| s == signs[0] && s != 0.0)
}

fn both_methods() -> [UnwrapOptions; 2] {
    [
        UnwrapOptions {
            method: UnwrapMethod::Conformal,
            ..Default::default()
        },
        UnwrapOptions::default(),
    ]
}

#[test]
fn flat_grid_unwraps_to_its_own_shape() {
    let mesh = grid(4, 0.0);
    for opts in both_methods() {
        let out = unwrap(&mesh, &opts).unwrap();
        assert_eq!(out.island_count, 1);
        assert_eq!(out.failed, 0);
        assert!(all_same_winding(&out.uvs));
        // A flat piece maps by a similarity: every quad stays a square of
        // equal size.
        let areas: Vec<f64> = out.uvs.iter().map(|f| signed_area(f).abs()).collect();
        for a in &areas {
            assert!((a / areas[0] - 1.0).abs() < 1e-6, "{areas:?}");
        }
        for f in &out.uvs {
            let e0 = (f[1] - f[0]).length();
            let e1 = (f[2] - f[1]).length();
            assert!((e0 / e1 - 1.0).abs() < 1e-6);
            assert!((f[1] - f[0]).dot(f[2] - f[1]).abs() < 1e-6 * e0 * e1);
        }
    }
}

#[test]
fn bent_grid_has_no_flips() {
    let mesh = grid(8, 0.4);
    for opts in both_methods() {
        let out = unwrap(&mesh, &opts).unwrap();
        assert_eq!(out.failed, 0);
        assert!(all_same_winding(&out.uvs));
    }
}

#[test]
fn tube_cut_along_a_seam_opens_flat() {
    let mesh = tube(12, 3);
    for opts in both_methods() {
        let out = unwrap(&mesh, &opts).unwrap();
        assert_eq!(out.island_count, 1);
        assert_eq!(out.failed, 0);
        assert!(all_same_winding(&out.uvs));
        // The seam's two sides land in different places.
        let f_first = &out.uvs[0];
        let f_last = &out.uvs[11];
        // Face 0 corner 0 and face 11 corner 1 are the same point (id 0).
        assert!((f_first[0] - f_last[1]).length() > 1e-3);
        // Rectangle about 2π by 2 (before scaling, in proportion).
        let mut lo = DVec2::splat(f64::MAX);
        let mut hi = DVec2::splat(f64::MIN);
        for p in out.uvs.iter().flatten() {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        let size = hi - lo;
        let ratio = size.max_element() / size.min_element();
        let want = (12.0 * 2.0 * (std::f64::consts::PI / 12.0).sin()) / 2.0;
        assert!((ratio - want).abs() < 0.05 * want, "{ratio} vs {want}");
    }
}

#[test]
fn closed_cube_without_seams_fails_but_with_seams_unwraps() {
    let mut mesh = cube();
    let out = unwrap(&mesh, &UnwrapOptions::default()).unwrap();
    assert_eq!(out.island_count, 0);
    assert_eq!(out.failed, 1);
    // Cut every edge: six islands.
    let mut seams = Vec::new();
    for f in &mesh.faces {
        for i in 0..4 {
            seams.push([f[i].point, f[(i + 1) % 4].point]);
        }
    }
    mesh.seams = seams;
    let out = unwrap(&mesh, &UnwrapOptions::default()).unwrap();
    assert_eq!(out.island_count, 6);
    assert_eq!(out.failed, 0);
    let isl: std::collections::HashSet<_> = out.islands.iter().copied().collect();
    assert_eq!(isl.len(), 6);
}

#[test]
fn two_pins_are_kept() {
    let mut mesh = grid(3, 0.3);
    let a = DVec2::new(0.2, 0.3);
    let b = DVec2::new(0.9, 0.4);
    // Pin every corner of point 0 and point 15 (the grid's far corner).
    for f in &mut mesh.faces {
        for c in f.iter_mut() {
            if c.point == 0 {
                c.pin = true;
                c.uv = a;
            }
            if c.point == 15 {
                c.pin = true;
                c.uv = b;
            }
        }
    }
    let out = unwrap(&mesh, &UnwrapOptions::default()).unwrap();
    for (fi, f) in mesh.faces.iter().enumerate() {
        for (ci, c) in f.iter().enumerate() {
            if c.point == 0 {
                assert!((out.uvs[fi][ci] - a).length() < 1e-9);
            }
            if c.point == 15 {
                assert!((out.uvs[fi][ci] - b).length() < 1e-9);
            }
        }
    }
}

#[test]
fn ngon_and_triangle_mix() {
    // A hexagon fan around a centre point plus a pentagon n-gon beside it.
    let mut points = vec![DVec3::ZERO];
    for i in 0..6 {
        let a = i as f64 / 6.0 * std::f64::consts::TAU;
        points.push(DVec3::new(a.cos(), a.sin(), 0.1 * (i % 2) as f64));
    }
    points.push(DVec3::new(2.0, 0.5, 0.0));
    points.push(DVec3::new(2.2, -0.3, 0.0));
    let mut faces: Vec<Vec<Corner>> = (0..6)
        .map(|i| vec![corner(0), corner(1 + i), corner(1 + (i + 1) % 6)])
        .collect();
    // Pentagon using edge 1-6... points 6, 1 are neighbours (angles 300°, 0°).
    faces.push(vec![corner(6), corner(8), corner(7), corner(2), corner(1)]);
    let mesh = UvMesh {
        points,
        faces,
        ..Default::default()
    };
    let out = unwrap(&mesh, &UnwrapOptions::default()).unwrap();
    assert_eq!(out.failed, 0);
    assert_eq!(out.island_count, 1);
    assert!(out.uvs.iter().flatten().all(|p| p.is_finite()));
}

#[test]
fn empty_mesh_is_fine_and_bad_input_is_refused() {
    let out = unwrap(&UvMesh::default(), &UnwrapOptions::default()).unwrap();
    assert_eq!(out.island_count, 0);
    assert!(out.uvs.is_empty());

    let mut mesh = grid(1, 0.0);
    mesh.faces[0].truncate(2);
    assert_eq!(
        unwrap(&mesh, &UnwrapOptions::default()).unwrap_err(),
        UvError::FaceTooSmall { face: 0 }
    );
    let mut mesh = grid(1, 0.0);
    mesh.faces[0][0].point = 99;
    assert_eq!(
        unwrap(&mesh, &UnwrapOptions::default()).unwrap_err(),
        UvError::BadPoint { point: 99 }
    );
}

#[test]
fn validation_rejects_mismatched_weights() {
    let mut mesh = grid(1, 0.0);
    mesh.weights = Some(vec![1.0; mesh.points.len() - 1]);
    assert_eq!(mesh.validate(), Err(UvError::BadWeights));
}

#[test]
fn validation_reports_the_face_with_a_nonfinite_uv() {
    for invalid in [f64::NAN, f64::INFINITY] {
        let mut mesh = grid(2, 0.0);
        mesh.faces[2][1].uv.y = invalid;
        assert_eq!(mesh.validate(), Err(UvError::BadUv { face: 2 }));
    }
}

#[test]
fn validation_rejects_a_seam_with_an_unknown_point() {
    let mut mesh = grid(1, 0.0);
    mesh.seams = vec![[0, 1], [1, 99]];
    assert_eq!(mesh.validate(), Err(UvError::BadPoint { point: 99 }));
}

#[test]
fn hole_filling_keeps_a_ring_flat() {
    // A grid with its middle quad removed: two borders.
    let mut mesh = grid(3, 0.0);
    mesh.faces.remove(4);
    for fill in [false, true] {
        let opts = UnwrapOptions {
            construct: ConstructOptions {
                fill_holes: fill,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = unwrap(&mesh, &opts).unwrap();
        assert_eq!(out.failed, 0);
        assert_eq!(out.uvs.len(), 8);
        assert!(all_same_winding(&out.uvs));
    }
}

#[test]
fn topology_from_uvs_cuts_at_uv_breaks() {
    // Two quads sharing an edge, with different UVs on each side.
    let mut mesh = grid(2, 0.0);
    mesh.faces.truncate(2);
    for (fi, f) in mesh.faces.iter_mut().enumerate() {
        for (ci, c) in f.iter_mut().enumerate() {
            c.uv = DVec2::new(ci as f64 + fi as f64 * 10.0, 0.0);
        }
    }
    let opts = UnwrapOptions {
        construct: ConstructOptions {
            topology_from_uvs: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let out = unwrap(&mesh, &opts).unwrap();
    assert_eq!(out.island_count, 2);
}

#[test]
fn stretch_minimise_keeps_borders_and_lowers_nothing_badly() {
    let mesh = grid(4, 0.5);
    let mut p = Param::new(&mesh, ConstructOptions::default()).unwrap();
    p.lscm_begin_all(false, false, false);
    p.lscm_solve();
    p.lscm_end();
    p.flush();
    let before = p.corner_uvs();
    p.stretch_begin();
    for _ in 0..10 {
        p.stretch_iter();
    }
    p.flush();
    let after = p.corner_uvs();
    // Border corners don't move.
    for (fi, f) in mesh.faces.iter().enumerate() {
        for (ci, c) in f.iter().enumerate() {
            let (i, j) = (c.point % 5, c.point / 5);
            if i == 0 || j == 0 || i == 4 || j == 4 {
                assert!((before[fi][ci] - after[fi][ci]).length() < 1e-12);
            }
        }
    }
    assert!(all_same_winding(&after));
}

#[test]
fn stretch_blends_and_restores_the_starting_uvs() {
    let mut mesh = grid(2, 0.0);
    for corner in mesh.faces.iter_mut().flatten() {
        corner.uv = mesh.points[corner.point as usize].truncate();
        if corner.point == 4 {
            corner.uv.x += 0.15;
        }
    }
    let mut param = Param::new(&mesh, ConstructOptions::default()).unwrap();
    let before = param.corner_uvs();
    param.stretch_begin();
    for _ in 0..10 {
        param.stretch_iter();
    }
    param.flush();
    let relaxed = param.corner_uvs();
    // The top-right corner of the first quad is the displaced center.
    assert!((relaxed[0][2] - DVec2::splat(0.5)).length() < 0.15 - 1e-6);

    param.set_blend(0.25);
    param.flush();
    for ((start, end), blended) in before
        .iter()
        .flatten()
        .zip(relaxed.iter().flatten())
        .zip(param.corner_uvs().iter().flatten())
    {
        assert!((*blended - end.lerp(*start, 0.25)).length() < 1e-12);
    }
    param.set_blend(2.0);
    param.flush();
    assert_eq!(param.corner_uvs(), before);
    param.set_blend(-1.0);
    param.flush();
    assert_eq!(param.corner_uvs(), relaxed);
    param.flush_restore();
    assert_eq!(param.corner_uvs(), before);
}

fn slim_opts() -> UnwrapOptions {
    UnwrapOptions {
        method: UnwrapMethod::MinimumStretch,
        ..Default::default()
    }
}

/// Area-weighted spread of the UV/3D area ratio (0 = no stretch).
fn area_distortion(mesh: &UvMesh, uvs: &[Vec<DVec2>]) -> f64 {
    let mut ratios = Vec::new();
    for (f, uv) in mesh.faces.iter().zip(uvs) {
        let p: Vec<DVec3> = f.iter().map(|c| mesh.points[c.point as usize]).collect();
        let a3 = 0.5 * (p[1] - p[0]).cross(p[2] - p[0]).length()
            + 0.5 * (p[2] - p[0]).cross(p[3] - p[0]).length();
        ratios.push(signed_area(uv).abs() / a3);
    }
    let mean = ratios.iter().sum::<f64>() / ratios.len() as f64;
    (ratios.iter().map(|r| (r / mean - 1.0).powi(2)).sum::<f64>() / ratios.len() as f64).sqrt()
}

#[test]
fn minimum_stretch_unwraps_without_flips_and_less_area_stretch() {
    // A dome-like patch: conformal maps blow up area toward the rim.
    let mut mesh = grid(8, 0.0);
    for p in &mut mesh.points {
        let d = (p.x - 0.5).powi(2) + (p.y - 0.5).powi(2);
        p.z = (0.6 - d).max(0.0).sqrt();
    }
    let conformal = unwrap(&mesh, &both_methods()[0]).unwrap();
    let slim = unwrap(&mesh, &slim_opts()).unwrap();
    assert_eq!(slim.failed, 0);
    assert!(all_same_winding(&slim.uvs));
    let (dc, ds) = (
        area_distortion(&mesh, &conformal.uvs),
        area_distortion(&mesh, &slim.uvs),
    );
    assert!(ds < dc, "slim {ds} vs conformal {dc}");
}

#[test]
fn minimum_stretch_on_a_tube_and_a_holed_grid() {
    let out = unwrap(&tube(12, 3), &slim_opts()).unwrap();
    assert_eq!(out.failed, 0);
    assert!(all_same_winding(&out.uvs));
    let mut mesh = grid(3, 0.2);
    mesh.faces.remove(4);
    let out = unwrap(&mesh, &slim_opts()).unwrap();
    assert_eq!(out.failed, 0);
    assert!(all_same_winding(&out.uvs));
}

#[test]
fn minimum_stretch_keeps_pins() {
    let mut mesh = grid(4, 0.3);
    let pins = [
        (0u32, DVec2::new(0.0, 0.0)),
        (4, DVec2::new(1.0, 0.0)),
        (24, DVec2::new(1.0, 1.0)),
    ];
    for f in &mut mesh.faces {
        for c in f.iter_mut() {
            if let Some(&(_, uv)) = pins.iter().find(|p| p.0 == c.point) {
                c.pin = true;
                c.uv = uv;
            }
        }
    }
    let out = unwrap(&mesh, &slim_opts()).unwrap();
    assert_eq!(out.failed, 0);
    for (fi, f) in mesh.faces.iter().enumerate() {
        for (ci, c) in f.iter().enumerate() {
            if let Some(&(_, uv)) = pins.iter().find(|p| p.0 == c.point) {
                assert!((out.uvs[fi][ci] - uv).length() < 1e-9);
            }
        }
    }
    assert!(all_same_winding(&out.uvs));
}
