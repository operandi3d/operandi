//! Vertex Bevel batch 1 (spec `vertex-tools-2026-11-14`, v1.1): Round
//! Level (VX-B02), border and corner points (VX-B03, VX-Q02), the Inset
//! clamp (VX-B05, VX-E05), selection after (VX-B06) and cap materials
//! (VX-B07). Oracle values are the spec's, Inset 0.2.
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

/// The spec's grid: 3×3 one-unit quads on XZ, normals +Y; point `r * 4 + c`
/// sits at (-1.5 + c, 0, -1.5 + r).
fn grid() -> Mesh {
    let mut m = Mesh {
        vertices: (0..16)
            .map(|i| DVec3::new(-1.5 + (i % 4) as f64, 0., -1.5 + (i / 4) as f64))
            .collect(),
        ..Default::default()
    };
    for r in 0..3 {
        for c in 0..3 {
            let i = r * 4 + c;
            m.faces.push(face(vec![i, i + 4, i + 5, i + 1]));
        }
    }
    m.validate().unwrap();
    assert!(m.normal(0).distance(DVec3::Y) < 1e-12);
    m
}

fn cube() -> (Mesh, u32) {
    let m = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap();
    let corner = m
        .vertices
        .iter()
        .position(|v| v.distance(DVec3::splat(0.5)) < 1e-9)
        .unwrap() as u32;
    (m, corner)
}

fn xz(x: f64, z: f64) -> DVec3 {
    DVec3::new(x, 0., z)
}

/// Some point of `m` lies within `tol` of `at`.
fn has_point(m: &Mesh, at: DVec3, tol: f64) -> bool {
    m.vertices.iter().any(|v| v.distance(at) <= tol)
}

fn assert_points(m: &Mesh, points: &[DVec3], tol: f64, case: &str) {
    for &p in points {
        let nearest = m
            .vertices
            .iter()
            .map(|v| v.distance(p))
            .fold(f64::INFINITY, f64::min);
        assert!(
            nearest <= tol,
            "{case}: no point near {p} (nearest {nearest:.4})"
        );
    }
}

fn bevel(m: &Mesh, points: &[u32], inset: f64, level: u32) -> (Mesh, Vec<u32>) {
    let (out, sel) = vertex_bevel_round(m, points, inset, level, None).unwrap();
    out.validate().unwrap();
    (out, sel)
}

fn counts(m: &Mesh) -> (usize, usize) {
    (m.vertices.len(), m.faces.len())
}

#[test]
fn round_level_0_cuts_an_interior_point_into_one_cap() {
    // VX-B1.
    let (out, sel) = bevel(&grid(), &[5], 0.2, 0);
    assert_eq!(counts(&out), (19, 10));
    let cuts = [
        xz(-0.7, -0.5),
        xz(-0.5, -0.3),
        xz(-0.3, -0.5),
        xz(-0.5, -0.7),
    ];
    assert_points(&out, &cuts, 1e-9, "VX-B1");
    // VX-B06: the cut points are selected.
    assert_eq!(sel.len(), 4);
    for v in sel {
        assert!(
            cuts.iter()
                .any(|c| c.distance(out.vertices[v as usize]) < 1e-9)
        );
    }
}

#[test]
fn round_levels_on_a_flat_point_match_the_oracle() {
    // VX-B2 (L1), VX-B3 (L2), VX-B4 (L3).
    let (l1, _) = bevel(&grid(), &[5], 0.2, 1);
    assert_eq!(counts(&l1), (24, 13), "VX-B2");
    assert_points(&l1, &[xz(-0.5586, -0.4414)], 1e-4, "VX-B2 arc midpoint");
    assert_points(&l1, &[xz(-0.5, -0.5)], 1e-9, "VX-B2 centre");

    let (l2, _) = bevel(&grid(), &[5], 0.2, 2);
    assert_eq!(counts(&l2), (40, 25), "VX-B3");
    let arc = [
        xz(-0.6235, -0.4848),
        xz(-0.5586, -0.4414),
        xz(-0.5152, -0.3765),
    ];
    assert_points(&l2, &arc, 1e-4, "VX-B3 arc");
    assert_points(&l2, &[xz(-0.5, -0.5)], 1e-9, "VX-B3 centre");
    assert_points(
        &l2,
        &[xz(-0.5, -0.4155), xz(-0.5388, -0.4612)],
        0.01,
        "VX-B3 inside",
    );
    // The flat cap stays flat.
    assert!(l2.vertices.iter().all(|v| v.y.abs() < 1e-12));

    let (l3, _) = bevel(&grid(), &[5], 0.2, 3);
    assert_eq!(counts(&l3), (64, 45), "VX-B4");
    // 6×6 quads in the cap (four 3×3 sectors).
    let quads = l3.faces.len() - 9;
    assert_eq!(quads, 36);
    assert!(l3.faces[9..].iter().all(|f| f.vertices.len() == 4));
}

#[test]
fn rounding_grows_each_neighbouring_polygon_by_its_fillet() {
    // VX-B02: each side gains 2L-1 points, inserted in the polygon there.
    for level in 1..=3u32 {
        let (out, _) = bevel(&grid(), &[5], 0.2, level);
        let grown: Vec<usize> = out.faces[..9]
            .iter()
            .filter(|f| f.vertices.len() > 4)
            .map(|f| f.vertices.len())
            .collect();
        assert_eq!(grown, vec![5 + 2 * level as usize - 1; 4], "L{level}");
        // The surface stays one closed-up piece: every edge is used at
        // most twice and the cap shares the fillet edges.
        for count in out.edge_counts().values() {
            assert!(*count <= 2);
        }
    }
}

#[test]
fn cube_corner_rounds_into_a_dome() {
    let (cube, corner) = cube();
    // VX-B7.
    let (l0, _) = bevel(&cube, &[corner], 0.2, 0);
    assert_eq!(counts(&l0), (10, 7), "VX-B7");
    assert_points(
        &l0,
        &[
            DVec3::new(0.5, 0.3, 0.5),
            DVec3::new(0.5, 0.5, 0.3),
            DVec3::new(0.3, 0.5, 0.5),
        ],
        1e-9,
        "VX-B7",
    );
    // VX-B7c.
    let (l1, _) = bevel(&cube, &[corner], 0.2, 1);
    assert_eq!(counts(&l1), (14, 9), "VX-B7c");
    assert_points(&l1, &[DVec3::new(0.5, 0.4414, 0.4414)], 1e-4, "VX-B7c arc");
    assert_points(&l1, &[DVec3::splat(0.4679)], 0.01, "VX-B7c centre");
    assert!(l1.is_closed());
    // VX-B7b.
    let (l2, _) = bevel(&cube, &[corner], 0.2, 2);
    assert_eq!(counts(&l2), (26, 18), "VX-B7b");
    assert_points(
        &l2,
        &[
            DVec3::splat(0.4679),
            DVec3::new(0.4875, 0.4875, 0.4149),
            DVec3::new(0.492, 0.4532, 0.4532),
        ],
        0.01,
        "VX-B7b",
    );
    assert!(l2.is_closed());
    // The dome sits inside the original corner.
    assert!(l2.vertices.iter().all(|v| v.max_element() <= 0.5 + 1e-12));
}

#[test]
fn border_and_corner_points_are_cut() {
    // VX-B5: a corner point of one polygon is cut off, no cap.
    let (corner, sel) = bevel(&grid(), &[0], 0.2, 0);
    assert_eq!(counts(&corner), (17, 9), "VX-B5");
    assert_eq!(sel.len(), 2);
    assert_eq!(corner.faces[0].vertices.len(), 5);
    // VX-B6: a border point of two polygons gets a triangle cap.
    let (border, sel) = bevel(&grid(), &[1], 0.2, 0);
    assert_eq!(counts(&border), (18, 10), "VX-B6");
    assert_eq!(sel.len(), 3);
    assert_eq!(border.faces[9].vertices.len(), 3);
    assert_points(
        &border,
        &[xz(-0.7, -1.5), xz(-0.3, -1.5), xz(-0.5, -1.3)],
        1e-9,
        "VX-B6",
    );
}

#[test]
fn rounded_border_points_get_fillets_and_a_straight_open_side() {
    // VX-Q02: the two inner sides are fillets as for an interior point; the
    // open side stays straight along the border (not VX-B04's spacing).
    let (out, sel) = bevel(&grid(), &[1], 0.2, 2);
    out.validate().unwrap();
    assert_eq!(sel.len(), 3);
    // Fillet points as on the interior case, mirrored to this corner.
    assert_points(
        &out,
        &[xz(-0.5586, -1.4414), xz(-0.4414, -1.4414)],
        1e-4,
        "fillets",
    );
    // The open side: 2L - 1 = 3 points evenly on z = -1.5 between the cuts.
    for x in [-0.6, -0.5, -0.4] {
        assert!(has_point(&out, xz(x, -1.5), 1e-9), "open side {x}");
    }
    assert!(!has_point(&out, xz(-0.3586, -1.5), 1e-3));
    // Three 2×2 sectors.
    assert_eq!(out.faces.len(), 9 + 12);
    assert!(out.vertices.iter().all(|v| v.y.abs() < 1e-12));
    // A rounded lone corner: its polygon gains the fillet, still no cap.
    let (corner, sel) = bevel(&grid(), &[0], 0.2, 2);
    assert_eq!(counts(&corner), (16 - 1 + 2 + 3, 9));
    assert_eq!(sel.len(), 2);
    assert_eq!(corner.faces[0].vertices.len(), 5 + 3);
}

#[test]
fn inset_and_width_clamp_at_each_edge() {
    // VX-B05 / VX-B10: Inset 1.5 on one-unit edges puts the cuts on the
    // neighbouring points, unmerged.
    let (out, sel) = bevel(&grid(), &[5], 1.5, 0);
    assert_eq!(counts(&out), (19, 10));
    // The cuts sit on points 1, 4, 6 and 9, which stay separate points.
    let source = grid();
    for n in [1usize, 4, 6, 9] {
        let on = sel
            .iter()
            .filter(|&&v| out.vertices[v as usize].distance(source.vertices[n]) < 1e-9)
            .count();
        assert_eq!(on, 1, "point {n}");
    }
    // VX-B9: two selected neighbours cut their shared edge from each end;
    // a big Inset lets the cuts pass each other.
    let (out, sel) = bevel(&grid(), &[5, 6], 0.7, 0);
    out.validate().unwrap();
    assert_eq!(sel.len(), 8);
    assert!(has_point(&out, xz(0.2, -0.5), 1e-9));
    assert!(has_point(&out, xz(-0.2, -0.5), 1e-9));
    // VX-E05: Vertex Extrude's Width clamps the same way.
    let (out, _) = vertex_extrude_geometry(&grid(), &[5], 0.3, 2.).unwrap();
    out.validate().unwrap();
    assert_eq!(counts(&out), (20, 13));
    assert!(has_point(&out, xz(-1.5, -0.5), 1e-9));
}

#[test]
fn cube_inset_past_the_edges_clamps_instead_of_refusing() {
    let (cube, corner) = cube();
    let (out, sel) = bevel(&cube, &[corner], 1.5, 1);
    assert_eq!(sel.len(), 3);
    for v in sel {
        assert!(
            cube.vertices
                .iter()
                .any(|c| c.distance(out.vertices[v as usize]) < 1e-9)
        );
    }
}

#[test]
fn bad_round_levels_are_refused() {
    assert!(
        vertex_bevel_round(&grid(), &[5], 0.2, VERTEX_BEVEL_MAX_ROUND_LEVEL + 1, None).is_err()
    );
    assert!(vertex_bevel_round(&grid(), &[5], -0.2, 1, None).is_err());
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
fn the_tool_rounds_tags_every_cap_polygon_selects_the_cuts_and_undoes() {
    let mut e = editor(grid(), vec![5]);
    let before = e.scene.clone();
    e.dispatch(Command::Activate(Tool::VertexBevel)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    // Starting values: Inset 0, Use Material off, empty name, Round Level 0.
    assert_eq!(
        (
            p.inset,
            p.vertex_use_material,
            p.vertex_material.as_str(),
            p.vertex_round_level
        ),
        (0., false, "", 0)
    );
    p.inset = 0.2;
    p.vertex_round_level = 2;
    p.vertex_use_material = true;
    p.vertex_material = "Cap".into();
    e.dispatch(Command::Preview(p.clone())).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let m = (*e.scene.active().unwrap().mesh).clone();
    assert_eq!(counts(&m), (40, 25));
    // VX-B07: all sixteen cap quads take the material, nothing else does.
    let caps = m.faces.iter().filter(|f| f.material == "Cap").count();
    assert_eq!(caps, 16);
    assert!(m.faces[..9].iter().all(|f| f.material == "Default"));
    // VX-B06: only the four cut points are selected; no polygon.
    let sel = &e.scene.selection;
    assert_eq!(sel.vertices.len(), 4);
    assert!(sel.faces.is_empty());
    let cuts = [
        xz(-0.7, -0.5),
        xz(-0.5, -0.3),
        xz(-0.3, -0.5),
        xz(-0.5, -0.7),
    ];
    for &v in &sel.vertices {
        assert!(
            cuts.iter()
                .any(|c| c.distance(m.vertices[v as usize]) < 1e-9)
        );
    }
    // Save and reopen keep the rounded cap.
    let path =
        std::env::temp_dir().join(format!("vertex-bevel-round-{}.opscene", std::process::id()));
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    std::fs::remove_file(&path).ok();
    assert_eq!(*back.active().unwrap().mesh, m);
    // One undo step back to the grid.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(
        e.scene.active().unwrap().mesh,
        before.active().unwrap().mesh
    );
}

#[test]
fn cancel_leaves_the_mesh_as_it_was() {
    let mut e = editor(grid(), vec![1]);
    let before = e.scene.clone();
    let history = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::VertexBevel)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.inset = 0.3;
    p.vertex_round_level = 1;
    e.dispatch(Command::Preview(p)).unwrap();
    assert_ne!(
        e.scene.active().unwrap().mesh,
        before.active().unwrap().mesh
    );
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(
        e.scene.active().unwrap().mesh,
        before.active().unwrap().mesh
    );
    assert_eq!(e.history.undo.len(), history);
}

/// The new points (past the 15 kept grid points) as (x, z), sorted.
fn new_points(m: &Mesh) -> Vec<(f64, f64)> {
    let mut v: Vec<(f64, f64)> = m.vertices[15..].iter().map(|p| (p.x, p.z)).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v
}

#[test]
fn clamped_cuts_keep_the_regular_fillets_and_kink() {
    // VX-B08: point 5's +X neighbour 0.15 away, Inset 0.2.
    let mut g = grid();
    g.vertices[6] = xz(-0.35, -0.5);
    for level in [1, 2] {
        let (out, _) = bevel(&g, &[5], 0.2, level);
        let (regular, _) = bevel(&grid(), &[5], 0.2, level);
        // The clamped cut sits on the neighbour, unmerged.
        assert_points(&out, &[xz(-0.35, -0.5)], 1e-9, "clamped cut");
        assert_eq!(
            out.vertices
                .iter()
                .filter(|v| v.distance(xz(-0.35, -0.5)) < 1e-9)
                .count(),
            2
        );
        // Every fillet point is exactly where it is on the regular grid.
        let fillets: Vec<DVec3> = regular.faces[..9]
            .iter()
            .flat_map(|f| f.vertices.iter())
            .map(|&v| regular.vertices[v as usize])
            .filter(|p| p.distance(xz(-0.5, -0.5)) < 0.199)
            .collect();
        assert_eq!(fillets.len(), 4 * (2 * level as usize - 1));
        assert_points(&out, &fillets, 1e-12, "fillets unchanged");
        // The centre is the mean of the cut points.
        assert_points(&out, &[xz(-0.5125, -0.5)], 1e-12, "centre");
    }
    // Different edge lengths, all longer than Inset: as the regular grid.
    let mut g = grid();
    g.vertices[6] = xz(-0.25, -0.5);
    g.vertices[9] = xz(-0.5, 0.);
    let (out, _) = bevel(&g, &[5], 0.2, 2);
    let (regular, _) = bevel(&grid(), &[5], 0.2, 2);
    for (a, b) in new_points(&out).iter().zip(new_points(&regular)) {
        assert!((a.0 - b.0).abs() < 1e-12 && (a.1 - b.1).abs() < 1e-12);
    }
}

#[test]
fn rounded_cap_uvs_are_the_old_surface_uvs_at_each_point() {
    // VX-B09: planar map u = x/3 + 0.5, v = z/3 + 0.5; L2 on point 5.
    let uv = |p: DVec3| glam::DVec2::new(p.x / 3. + 0.5, p.z / 3. + 0.5);
    let mut g = grid();
    let rows: Vec<Vec<Option<glam::DVec2>>> = g
        .faces
        .iter()
        .map(|f| {
            f.vertices
                .iter()
                .map(|&v| Some(uv(g.vertices[v as usize])))
                .collect()
        })
        .collect();
    let mut maps = (*g.uv_maps).clone();
    maps.insert("Texture".into(), rows);
    g.uv_maps = maps.into();
    g.validate().unwrap();
    for (points, inset) in [(vec![5], 0.2), (vec![1], 0.2), (vec![5, 6], 0.3)] {
        let (out, _) = bevel(&g, &points, inset, 2);
        let rows = &out.uv_maps["Texture"];
        let mut checked = 0;
        for (f, face) in out.faces.iter().enumerate() {
            for (c, &v) in face.vertices.iter().enumerate() {
                let got = rows[f][c].expect("UV");
                let want = uv(out.vertices[v as usize]);
                assert!(
                    got.distance(want) < 1e-9,
                    "{points:?} face {f}: {got} vs {want}"
                );
                checked += 1;
            }
        }
        assert!(checked > 0);
    }
}

/// Grid point 5's +X neighbour (point 6) pulled in to `gap` from it.
fn short_x(gap: f64) -> Mesh {
    let mut g = grid();
    g.vertices[6] = xz(-0.5 + gap, -0.5);
    g
}

/// Per inside point (past the border), how far the clamp moved it.
fn inside_moves(clamped: &Mesh, regular: &Mesh, level: u32) -> Vec<(DVec3, DVec3)> {
    let start = 15 + 4 * 2 * level as usize;
    (start..regular.vertices.len())
        .map(|i| {
            (
                regular.vertices[i],
                clamped.vertices[i] - regular.vertices[i],
            )
        })
        .collect()
}

#[test]
fn inside_points_next_to_a_clamped_cut_match_the_oracle() {
    // VX-B08 / VX-B11 clamped case, L2.
    let (out, _) = bevel(&short_x(0.15), &[5], 0.2, 2);
    assert_points(
        &out,
        &[
            xz(-0.5419, -0.4599),
            xz(-0.507, -0.4081),
            xz(-0.4831, -0.4535),
            xz(-0.4514, -0.5),
        ],
        0.01,
        "VX-B11 L2",
    );
    assert_points(&out, &[xz(-0.5125, -0.5)], 1e-12, "centre");
}

#[test]
fn a_clamp_moves_inside_points_by_the_measured_table() {
    // VX-B11: shift = s × (a along + c across), exactly proportional to s.
    for level in [1u32, 2, 3] {
        let (regular, _) = bevel(&grid(), &[5], 0.2, level);
        for s in [0.01, 0.05, 0.1, 0.15] {
            let (clamped, _) = bevel(&short_x(0.2 - s), &[5], 0.2, level);
            let moves = inside_moves(&clamped, &regular, level);
            // The centre moves a quarter of s toward the original point.
            let centre = moves
                .iter()
                .find(|(at, _)| at.distance(xz(-0.5, -0.5)) < 1e-12)
                .unwrap();
            assert!(centre.1.distance(xz(-0.25 * s, 0.)) < 1e-12);
            // The point nearest the clamped cut moves along only.
            let (want, row) = match level {
                1 => continue,
                2 => (0.718, 0.0845),
                _ => (0.858, 0.1144),
            };
            let near = moves
                .iter()
                .min_by(|a, b| {
                    let d = |p: DVec3| p.distance(xz(-0.5 + row, -0.5));
                    d(a.0).total_cmp(&d(b.0))
                })
                .unwrap();
            assert!(near.1.distance(xz(-want * s, 0.)) < 1e-12, "L{level} s {s}");
        }
        // Arc points and the other cuts never move.
        let (clamped, _) = bevel(&short_x(0.1), &[5], 0.2, level);
        let border = 15..15 + 4 * 2 * level as usize;
        let moved: Vec<usize> = border
            .filter(|&i| clamped.vertices[i].distance(regular.vertices[i]) > 1e-12)
            .collect();
        assert_eq!(moved.len(), 1, "only the clamped cut, L{level}");
    }
    // L2's (+0.0388, ±0.0388) points: a 0.438, c ±0.155, across pointing
    // away from the clamped cut's line.
    let (regular, _) = bevel(&grid(), &[5], 0.2, 2);
    let (clamped, _) = bevel(&short_x(0.1), &[5], 0.2, 2);
    for (at, m) in inside_moves(&clamped, &regular, 2) {
        let rel = at - xz(-0.5, -0.5);
        if rel.x > 0.02 && rel.z.abs() > 0.02 {
            let want = xz(-0.438 * 0.1, 0.155 * 0.1 * rel.z.signum());
            assert!(m.distance(want) < 1e-12, "{rel}: {m}");
        }
    }
}

#[test]
fn several_clamped_cuts_add_up() {
    // VX-B11 step 3: +X and +Z neighbours both short.
    let (regular, _) = bevel(&grid(), &[5], 0.2, 3);
    let mut both = short_x(0.12);
    both.vertices[9] = xz(-0.5, -0.5 + 0.17);
    let mut z_only = grid();
    z_only.vertices[9] = both.vertices[9];
    let (b, _) = bevel(&both, &[5], 0.2, 3);
    let (x, _) = bevel(&short_x(0.12), &[5], 0.2, 3);
    let (z, _) = bevel(&z_only, &[5], 0.2, 3);
    let (mb, mx, mz) = (
        inside_moves(&b, &regular, 3),
        inside_moves(&x, &regular, 3),
        inside_moves(&z, &regular, 3),
    );
    for ((b, x), z) in mb.iter().zip(&mx).zip(&mz) {
        assert!(b.1.distance(x.1 + z.1) < 1e-12);
    }
}

#[test]
fn unmeasured_clamps_blend_close_enough() {
    // VX-B11 [OP]: Round Level 4 and a cube corner with a short edge.
    let (regular, _) = bevel(&grid(), &[5], 0.2, 4);
    let (clamped, _) = bevel(&short_x(0.1), &[5], 0.2, 4);
    let moves = inside_moves(&clamped, &regular, 4);
    let centre = moves
        .iter()
        .find(|(at, _)| at.distance(xz(-0.5, -0.5)) < 1e-12)
        .unwrap();
    assert!(centre.1.distance(xz(-0.025, 0.)) < 1e-12);
    // Points near the clamped cut move inward, far ones hardly at all.
    for (at, m) in &moves {
        assert!(m.length() < 0.1);
        let rel = *at - xz(-0.5, -0.5);
        if rel.x > 0.1 {
            assert!(m.x < -0.05, "{rel}: {m}");
        }
        if rel.x < -0.1 {
            assert!(m.length() < 0.01, "{rel}: {m}");
        }
    }
    let (mut cube, corner) = cube();
    let shortened = cube
        .vertices
        .iter()
        .position(|v| v.distance(DVec3::new(0.5, 0.5, -0.5)) < 1e-9)
        .unwrap();
    cube.vertices[shortened] = DVec3::new(0.5, 0.5, 0.35);
    let (out, _) = bevel(&cube, &[corner], 0.2, 2);
    assert!(out.vertices.iter().all(|v| v.is_finite()));
    let mean_cut =
        (DVec3::new(0.5, 0.3, 0.5) + DVec3::new(0.3, 0.5, 0.5) + DVec3::new(0.5, 0.5, 0.35)) / 3.;
    // Across the surface the centre is the cuts' mean (VX-B08).
    let n = DVec3::ONE.normalize();
    assert!(out.vertices.iter().any(|v| {
        let d = *v - mean_cut;
        (d - n * d.dot(n)).length() < 1e-9
    }));
}
