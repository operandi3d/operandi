//! Twist Bevel geometry against specifications/twist-bevel-2026-11-15
//! (v1.1), section 9's tests.
use glam::DVec3;
use operandi_core::*;

const EPS: f64 = 1e-9;

/// A unit cube and its top (+Y) polygon.
fn cube_top() -> (Mesh, u32) {
    let mesh = cube_geometry(DVec3::ZERO, DVec3::ONE).unwrap();
    let top = (0..mesh.faces.len())
        .find(|&f| mesh.normal(f).dot(DVec3::Y) > 0.99)
        .unwrap() as u32;
    (mesh, top)
}

/// A row of `n` unit quads in the XY plane (normal +Z), starting at x0.
fn row(n: u32, x0: f64) -> Mesh {
    let mut mesh = Mesh::default();
    for j in 0..=1 {
        for i in 0..=n {
            mesh.vertices.push(DVec3::new(x0 + i as f64, j as f64, 0.));
        }
    }
    for i in 0..n {
        mesh.faces.push(Face {
            vertices: vec![i, i + 1, i + n + 2, i + n + 1],
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
    }
    mesh
}

fn options(shift: f64) -> TwistBevelOptions {
    TwistBevelOptions {
        shift,
        ..Default::default()
    }
}

fn run(mesh: &Mesh, selected: &[u32], o: TwistBevelOptions) -> TwistBevelResult {
    twist_bevel_geometry(mesh, selected, &o, &TwistBevelPipe::default()).unwrap()
}

fn centre(mesh: &Mesh, face: u32) -> DVec3 {
    let f = &mesh.faces[face as usize];
    f.vertices
        .iter()
        .map(|&v| mesh.vertices[v as usize])
        .sum::<DVec3>()
        / f.vertices.len() as f64
}

/// The new points of the result, grouped by height above the top face.
fn rings_by_height(base: &Mesh, out: &Mesh) -> Vec<(f64, Vec<DVec3>)> {
    let mut rings: Vec<(f64, Vec<DVec3>)> = Vec::new();
    for &p in &out.vertices[base.vertices.len()..] {
        match rings.iter_mut().find(|r| (r.0 - p.y).abs() < 1e-6) {
            Some(r) => r.1.push(p),
            None => rings.push((p.y, vec![p])),
        }
    }
    rings.sort_by(|a, b| a.0.total_cmp(&b.0));
    rings
}

fn side_of(points: &[DVec3]) -> f64 {
    let xs = points.iter().map(|p| p.x);
    xs.clone().fold(f64::MIN, f64::max) - xs.fold(f64::MAX, f64::min)
}

/// Spec test 1: Shift 1, Segments 4: four rings 0.25 apart straight up, the face's
/// size; 16 new quads; the top face stays selected.
#[test]
fn straight_tube() {
    let (mesh, top) = cube_top();
    let out = run(&mesh, &[top], options(1.));
    assert_eq!(out.mesh.faces.len(), mesh.faces.len() + 16);
    assert!(
        out.mesh.faces[mesh.faces.len()..]
            .iter()
            .all(|f| f.vertices.len() == 4)
    );
    assert_eq!(out.selected, vec![top]);
    let rings = rings_by_height(&mesh, &out.mesh);
    assert_eq!(rings.len(), 4);
    for (k, (y, points)) in rings.iter().enumerate() {
        assert!((y - (0.5 + 0.25 * (k + 1) as f64)).abs() < EPS);
        assert_eq!(points.len(), 4);
        assert!((side_of(points) - 1.).abs() < EPS);
    }
    assert!((centre(&out.mesh, top) - DVec3::new(0., 1.5, 0.)).length() < EPS);
}

/// Spec test 2: Inset 0.25: cap side 0.5, ring k side 1 − 0.5·k/4.
#[test]
fn straight_taper() {
    let (mesh, top) = cube_top();
    let o = TwistBevelOptions {
        inset: 0.25,
        ..options(1.)
    };
    let out = run(&mesh, &[top], o);
    for (k, (_, points)) in rings_by_height(&mesh, &out.mesh).iter().enumerate() {
        let k = (k + 1) as f64;
        assert!(
            (side_of(points) - (1. - 0.5 * k / 4.)).abs() < 1e-9,
            "ring {k}"
        );
    }
}

/// Spec test 3: Bend 90°, Bend Direction 0: cap centre at C + N·R + B·R with
/// R = 1/(π/2); the cap faces B (+X, world X flattened onto the face).
#[test]
fn bend_quarter_turn() {
    let (mesh, top) = cube_top();
    let o = TwistBevelOptions {
        bend: 90.,
        ..options(1.)
    };
    let out = run(&mesh, &[top], o);
    let r = 1. / std::f64::consts::FRAC_PI_2;
    let expected = DVec3::new(0., 0.5, 0.) + DVec3::Y * r + DVec3::X * r;
    assert!((centre(&out.mesh, top) - expected).length() < 1e-9);
    assert!(out.mesh.normal(top as usize).distance(DVec3::X) < 1e-9);
    // Every ring keeps the face's size and sits on the arc.
    let (frames, _) = twist_bevel_frames(&mesh, &[top], &o, &TwistBevelPipe::default()).unwrap();
    let f = frames[0];
    for t in [0.25, 0.5, 0.75] {
        let p = f.spine(t);
        assert!((p.distance(DVec3::new(r, 0.5, 0.)) - r).abs() < 1e-9);
    }
}

/// Bend Direction turns the curl about the normal.
#[test]
fn bend_direction_turns_the_curl() {
    let (mesh, top) = cube_top();
    let o = TwistBevelOptions {
        bend: 90.,
        bend_direction: 90.,
        ..options(1.)
    };
    let out = run(&mesh, &[top], o);
    // About +Y, +X turned 90° is −Z.
    assert!(out.mesh.normal(top as usize).distance(-DVec3::Z) < 1e-9);
}

/// Spec test 4: Rotation 90°: the cap is the top face turned 90° about N; ring k
/// turned 22.5°·k.
#[test]
fn twist() {
    let (mesh, top) = cube_top();
    let o = TwistBevelOptions {
        rotation: 90.,
        ..options(1.)
    };
    let out = run(&mesh, &[top], o);
    let corner = mesh.vertices[mesh.faces[top as usize].vertices[0] as usize];
    let base_angle = corner.z.atan2(corner.x);
    for (k, (y, points)) in rings_by_height(&mesh, &out.mesh).iter().enumerate() {
        let k = (k + 1) as f64;
        // Right-hand turn about +Y by 22.5°·k (x, z) → angle decreases.
        let want = base_angle - (22.5 * k).to_radians();
        let hit = points.iter().any(|p| {
            let d = (p.z.atan2(p.x) - want).rem_euclid(std::f64::consts::TAU);
            d.min(std::f64::consts::TAU - d) < 1e-9
        });
        assert!(hit, "ring {k} at {y}");
    }
    // Turned a quarter, the square covers the same footprint.
    let cap_points: Vec<DVec3> = out.mesh.faces[top as usize]
        .vertices
        .iter()
        .map(|&v| out.mesh.vertices[v as usize])
        .collect();
    assert!((side_of(&cap_points) - 1.).abs() < 1e-9);
}

/// Spec test 5: Profile −0.5 pinches the middle ring, +0.5 bulges it; base and cap
/// keep the straight taper.
#[test]
fn profile_pinch_and_bulge() {
    let (mesh, top) = cube_top();
    let size = |profile: f64| {
        let o = TwistBevelOptions {
            inset: 0.25,
            profile,
            ..options(1.)
        };
        rings_by_height(&mesh, &run(&mesh, &[top], o).mesh)
            .iter()
            .map(|r| side_of(&r.1))
            .collect::<Vec<_>>()
    };
    let (straight, pinch, bulge) = (size(0.), size(-0.5), size(0.5));
    assert!(pinch[1] < straight[1], "{pinch:?} vs {straight:?}");
    assert!(bulge[1] > straight[1]);
    assert!((pinch[3] - straight[3]).abs() < 1e-9);
    assert!((bulge[3] - straight[3]).abs() < 1e-9);
    // W is the mean distance from the centre to the corners (√2/2); the
    // middle ring is pulled in by an extra W/2 on every side.
    let w = std::f64::consts::SQRT_2 / 2.;
    assert!((straight[1] - pinch[1] - w).abs() < 1e-9);
    assert!((bulge[1] - straight[1] - w).abs() < 1e-9);
}

/// Spec test 6: Group off on two adjoining faces: two separate tubes.
#[test]
fn group_off_makes_two_tubes() {
    let mesh = row(2, 0.);
    let o = TwistBevelOptions {
        group_polygons: false,
        ..options(1.)
    };
    let out = run(&mesh, &[0, 1], o);
    assert_eq!(out.mesh.vertices.len(), mesh.vertices.len() + 32);
    assert_eq!(out.mesh.faces.len(), 2 + 32);
    let grouped = run(&mesh, &[0, 1], options(1.));
    // One tube around the pair: 6 boundary points per ring, the middle two
    // points move with the cap.
    assert_eq!(grouped.mesh.faces.len(), 2 + 6 * 4);
}

/// Spec test 7: Symmetry X: the mirror face gets a mirror-image tube (bend mirrored,
/// twist reversed).
#[test]
fn symmetry_mirrors_the_tube() {
    let mesh = row(4, -2.);
    let o = TwistBevelOptions {
        bend: 50.,
        bend_direction: 30.,
        rotation: 40.,
        inset: 0.1,
        ..options(1.5)
    };
    let pipe = TwistBevelPipe {
        symmetry: Some((Symmetry::x(), symmetry::Side::Positive)),
        ..Default::default()
    };
    let out = twist_bevel_geometry(&mesh, &[3, 0], &o, &pipe).unwrap();
    for &p in &out.mesh.vertices {
        let m = DVec3::new(-p.x, p.y, p.z);
        assert!(
            out.mesh.vertices.iter().any(|q| q.distance(m) < 1e-9),
            "{p} has no mirror"
        );
    }
    // Without symmetry the two tubes curl the same way, not mirrored.
    let plain = run(&mesh, &[3, 0], o);
    assert!(plain.mesh.vertices.iter().any(|&p| {
        let m = DVec3::new(-p.x, p.y, p.z);
        !plain.mesh.vertices.iter().any(|q| q.distance(m) < 1e-6)
    }));
}

/// A group across the plane keeps its frame with Bend Direction locked
/// into the plane, and says so.
#[test]
fn symmetry_locks_a_crossing_group() {
    let mesh = row(2, -1.);
    let o = TwistBevelOptions {
        bend: 60.,
        bend_direction: 30.,
        ..options(1.)
    };
    let pipe = TwistBevelPipe {
        symmetry: Some((Symmetry::x(), symmetry::Side::Positive)),
        ..Default::default()
    };
    let out = twist_bevel_geometry(&mesh, &[0, 1], &o, &pipe).unwrap();
    assert!(out.note.is_some());
    for &p in &out.mesh.vertices {
        let m = DVec3::new(-p.x, p.y, p.z);
        assert!(out.mesh.vertices.iter().any(|q| q.distance(m) < 1e-9));
    }
}

/// Spec test 8: A linear falloff across a row of 5 faces scales each tube's Shift by
/// its face's weight.
#[test]
fn falloff_weights_each_group() {
    let mesh = row(5, 0.);
    let weight = |c: DVec3, _: &[u32]| c.x / 5.;
    let pipe = TwistBevelPipe {
        weight: Some(&weight),
        ..Default::default()
    };
    let o = TwistBevelOptions {
        group_polygons: false,
        ..options(2.)
    };
    let out = twist_bevel_geometry(&mesh, &[0, 1, 2, 3, 4], &o, &pipe).unwrap();
    for f in 0..5u32 {
        let c = centre(&out.mesh, f);
        let w = (f as f64 + 0.5) / 5.;
        assert!((c.z - 2. * w).abs() < 1e-9, "face {f}: {c}");
    }
}

/// Spec test 9: SubD polygons give SubD walls.
#[test]
fn subd_walls_stay_subd() {
    let (mut mesh, top) = cube_top();
    mesh.attributes.representations = vec![PolygonRepresentation::CatmullClark; mesh.faces.len()];
    let out = run(&mesh, &[top], options(1.));
    assert!(
        out.mesh
            .attributes
            .representations
            .iter()
            .all(|&r| r == PolygonRepresentation::CatmullClark)
    );
    assert_eq!(
        out.mesh.attributes.representations.len(),
        out.mesh.faces.len()
    );
}

/// Spec test 10: Shift 0, Inset 0: the mesh is unchanged.
#[test]
fn zero_changes_nothing() {
    let (mesh, top) = cube_top();
    let o = TwistBevelOptions {
        rotation: 45.,
        bend: 45.,
        ..options(0.)
    };
    assert_eq!(run(&mesh, &[top], o).mesh, mesh);
}

/// Spec test 11: Profile 0.5 with Profile Bias +1: the widest ring sits toward the
/// cap (t ≈ 0.95); base and cap unchanged.
#[test]
fn profile_bias_moves_the_bulge() {
    let (mesh, top) = cube_top();
    let o = TwistBevelOptions {
        profile: 0.5,
        profile_bias: 1.,
        segments: 20,
        ..options(1.)
    };
    let out = run(&mesh, &[top], o);
    let rings = rings_by_height(&mesh, &out.mesh);
    let (widest, _) = rings
        .iter()
        .enumerate()
        .max_by(|a, b| side_of(&a.1.1).total_cmp(&side_of(&b.1.1)))
        .unwrap();
    assert_eq!(widest + 1, 19, "ring 19 of 20 is t = 0.95");
    assert!((side_of(&rings[19].1) - 1.).abs() < 1e-9);
}

/// Spec test 12: Segment Spacing +1: rings at 1 − (1 − k/4)², i.e. 0.4375, 0.75,
/// 0.9375, 1.
#[test]
fn segment_spacing_bunches_toward_the_cap() {
    let (mesh, top) = cube_top();
    let o = TwistBevelOptions {
        segment_spacing: 1.,
        ..options(1.)
    };
    let rings = rings_by_height(&mesh, &run(&mesh, &[top], o).mesh);
    let heights: Vec<f64> = rings.iter().map(|r| r.0 - 0.5).collect();
    for (h, want) in heights.iter().zip([0.4375, 0.75, 0.9375, 1.]) {
        assert!((h - want).abs() < 1e-9, "{heights:?}");
    }
}

/// Spec test 13: Center Bias −1: rings bunched toward both ends, symmetric about the
/// middle.
#[test]
fn center_bias_bunches_toward_the_ends() {
    let o = TwistBevelOptions {
        center_bias: -1.,
        segments: 6,
        ..options(1.)
    };
    let t = twist_bevel_rings(&o);
    let even: Vec<f64> = (1..=6).map(|k| k as f64 / 6.).collect();
    assert!(t[0] < even[0], "{t:?}");
    assert!(t[4] > even[4], "{t:?}");
    // Symmetric about the middle: ring k and ring 6 − k add up to 1.
    for k in 1..6 {
        assert!((t[k - 1] + t[6 - k - 1] - 1.).abs() < 1e-9, "{t:?}");
    }
    assert!((t[2] - 0.5).abs() < 1e-12);
}

/// Odd input: Bend with Shift 0 does nothing more than the inset.
#[test]
fn bend_without_length_only_insets() {
    let (mesh, top) = cube_top();
    let o = TwistBevelOptions {
        inset: 0.1,
        bend: 90.,
        ..options(0.)
    };
    let out = run(&mesh, &[top], o);
    assert!(
        out.mesh.vertices[mesh.vertices.len()..]
            .iter()
            .all(|p| (p.y - 0.5).abs() < 1e-12)
    );
}

/// An inset past the centre is refused: Twist Bevel keeps its fold guard
/// (plain Bevel builds the inverted cap, PB-C-01).
#[test]
fn collapsing_inset_is_refused() {
    let (mesh, top) = cube_top();
    let o = TwistBevelOptions {
        inset: 0.8,
        ..options(1.)
    };
    assert!(twist_bevel_geometry(&mesh, &[top], &o, &TwistBevelPipe::default()).is_err());
}
