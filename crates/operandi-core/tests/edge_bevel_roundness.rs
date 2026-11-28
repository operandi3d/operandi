//! Round Edge Bevel profiles are round (the forum's oval-corner report,
//! 2026-11-27). Where the two side points of a profile sit at different
//! places along the edge (Loop Slide sliding them along slanted edges, as
//! at a hexagon's corners), the profile used to turn about a point level
//! with their midpoint, nearer one side than the other, and came out as a
//! lopsided oval. Each profile must now be a circular arc: every row on one
//! circle through both side points, in one plane, at equal angles.
use glam::DVec3;
use operandi_core::*;

fn face(v: Vec<u32>) -> Face {
    Face {
        vertices: v,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// A capped prism from y = 0 to y = 1 over the polygon `ring` (x, z), its
/// top scaled by `top` (1 for a straight prism).
fn frustum(ring: &[(f64, f64)], top: f64) -> Mesh {
    let n = ring.len() as u32;
    let mut m = Mesh::default();
    for (y, k) in [(0., 1.), (1., top)] {
        for &(x, z) in ring {
            m.vertices.push(DVec3::new(x * k, y, z * k));
        }
    }
    let mut faces = vec![face((0..n).rev().collect()), face((n..2 * n).collect())];
    for k in 0..n {
        let j = (k + 1) % n;
        faces.push(face(vec![k, j, n + j, n + k]));
    }
    m.faces = faces.into();
    if m.normal(1).y < 0. {
        for f in m.faces.iter_mut() {
            f.vertices.reverse();
        }
    }
    m.validate().unwrap();
    m
}

fn regular(n: usize, r: f64) -> Vec<(f64, f64)> {
    (0..n)
        .map(|k| {
            let a = std::f64::consts::TAU * k as f64 / n as f64;
            (r * a.cos(), r * a.sin())
        })
        .collect()
}

fn key(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

/// The top rim of a `frustum` over `n` points.
fn rim(n: u32) -> Vec<(u32, u32)> {
    (0..n).map(|k| key(n + k, n + (k + 1) % n)).collect()
}

fn bevel(m: &Mesh, edges: &[(u32, u32)], w: f64, rows: u32, loop_slide: bool) -> EdgeBevelResult {
    let settings = EdgeBevelSettings {
        width: w,
        rows,
        depth: 100.,
        chamfer: false,
        options: EdgeBevelOptions {
            loop_slide,
            ..Default::default()
        },
    };
    let r = edge_bevel_with(m, edges, &settings, None).unwrap();
    r.mesh.validate().unwrap();
    r
}

/// Circle through three points: centre, radius, plane normal.
fn circle(a: DVec3, b: DVec3, c: DVec3) -> (DVec3, f64, DVec3) {
    let (ab, ac) = (b - a, c - a);
    let n = ab.cross(ac);
    let centre = a
        + (n.cross(ab) * ac.length_squared() + ac.cross(n) * ab.length_squared())
            / (2. * n.length_squared());
    (centre, centre.distance(a), n.normalize())
}

/// How far the profile `points` (both side points and every row) are from
/// being a circular arc with equal steps, as a share of its chord: the
/// worst of the distance off the circle through the ends and the middle,
/// the distance off that circle's plane, and the unevenness of the steps.
fn out_of_round(points: &[DVec3]) -> f64 {
    let n = points.len();
    let (a, b) = (points[0], points[n - 1]);
    let chord = a.distance(b);
    let (centre, r, normal) = circle(a, points[n / 2], b);
    let off = points
        .iter()
        .map(|&p| {
            ((p - centre).length() - r)
                .abs()
                .max((p - centre).dot(normal).abs())
        })
        .fold(0., f64::max);
    let steps: Vec<f64> = points.windows(2).map(|w| w[0].distance(w[1])).collect();
    let mean = steps.iter().sum::<f64>() / steps.len() as f64;
    let uneven = steps.iter().map(|s| (s - mean).abs()).fold(0., f64::max);
    off.max(uneven) / chord
}

/// The profiles of a bevel: around each original point the strip ends on,
/// the strip's points nearest it, ordered from one side to the other.
fn profiles(m: &Mesh, r: &EdgeBevelResult, rows: u32) -> Vec<Vec<DVec3>> {
    let strip: std::collections::BTreeSet<u32> =
        r.strip_edges.iter().flat_map(|&(a, b)| [a, b]).collect();
    let ends: std::collections::BTreeSet<u32> =
        r.selected_after.iter().flat_map(|&(a, b)| [a, b]).collect();
    assert!(ends.is_subset(&strip));
    let mut groups: std::collections::BTreeMap<usize, Vec<DVec3>> = Default::default();
    for &id in &strip {
        let p = r.mesh.vertices[id as usize];
        let near = (0..m.vertices.len())
            .min_by(|&i, &j| {
                m.vertices[i]
                    .distance(p)
                    .total_cmp(&m.vertices[j].distance(p))
            })
            .unwrap();
        groups.entry(near).or_default().push(p);
    }
    groups
        .into_values()
        .map(|mut g| {
            assert_eq!(g.len(), rows as usize + 1, "one profile per end: {g:?}");
            // Order along the arc: start at one of the two points farthest
            // apart, then nearest first.
            let (mut i, mut far) = (0, 0.);
            for (k, &p) in g.iter().enumerate() {
                for &q in &g {
                    if p.distance(q) > far {
                        (i, far) = (k, p.distance(q));
                    }
                }
            }
            let mut ordered = vec![g.swap_remove(i)];
            while !g.is_empty() {
                let last = *ordered.last().unwrap();
                let k = (0..g.len())
                    .min_by(|&x, &y| g[x].distance(last).total_cmp(&g[y].distance(last)))
                    .unwrap();
                ordered.push(g.swap_remove(k));
            }
            ordered
        })
        .collect()
}

fn worst(m: &Mesh, edges: &[(u32, u32)], w: f64, rows: u32, loop_slide: bool) -> f64 {
    let r = bevel(m, edges, w, rows, loop_slide);
    profiles(m, &r, rows)
        .iter()
        .map(|p| out_of_round(p))
        .fold(0., f64::max)
}

#[test]
fn profiles_at_slanted_side_edges_are_circular() {
    // A hexagonal prism and a hexagonal frustum (slanted sides, so the
    // top meets them at more than 90°): Loop Slide slides the side points
    // along the 120° top edges, not straight in from the edge.
    let hex = frustum(&regular(6, 0.5), 1.);
    let cone = frustum(&regular(6, 0.6), 0.6);
    for rows in [3, 4, 8] {
        for (name, m, edges, w) in [
            ("hex prism top edge", &hex, vec![key(6, 7)], 0.1),
            ("hex prism cap rim", &hex, rim(6), 0.08),
            ("hex frustum top edge", &cone, vec![key(6, 7)], 0.08),
            ("hex frustum cap rim", &cone, rim(6), 0.08),
        ] {
            for slide in [true, false] {
                let e = worst(m, &edges, w, rows, slide);
                assert!(e < 1e-6, "{name}, {rows} rows, Loop Slide {slide}: {e}");
            }
        }
    }
}

#[test]
fn profiles_that_were_already_round_stay_round() {
    let cube = frustum(&regular(4, std::f64::consts::FRAC_1_SQRT_2), 1.);
    let wedge = frustum(&[(0., 0.), (1., 0.), (0.2, 0.6)], 1.);
    let cylinder = frustum(&regular(24, 0.5), 1.);
    for rows in [3, 4, 8] {
        for (name, m, edges, w, slide) in [
            ("cube top edge", &cube, vec![key(4, 5)], 0.1, true),
            ("cube vertical edge", &cube, vec![key(0, 4)], 0.1, true),
            ("cube top rim, Loop Slide off", &cube, rim(4), 0.1, false),
            (
                "cube L, Loop Slide off",
                &cube,
                vec![key(4, 5), key(5, 6)],
                0.1,
                false,
            ),
            (
                "wedge vertical edges (60°-ish)",
                &wedge,
                vec![key(0, 3), key(1, 4), key(2, 5)],
                0.08,
                true,
            ),
            (
                "cylinder cap rim, Loop Slide off",
                &cylinder,
                rim(24),
                0.05,
                false,
            ),
        ] {
            let e = worst(m, &edges, w, rows, slide);
            assert!(e < 1e-6, "{name}, {rows} rows: {e}");
        }
    }
}

#[test]
fn profile_corners_lean_only_a_little() {
    // The arc's plane leans along the edge only as far as it takes to be
    // round: on the hex prism's top edge the profile at each end turns about
    // the old corner, so its rows stay within the end's own width of it.
    let hex = frustum(&regular(6, 0.5), 1.);
    let r = bevel(&hex, &[key(6, 7)], 0.1, 4, true);
    for p in profiles(&hex, &r, 4) {
        let old = [6usize, 7]
            .map(|i| hex.vertices[i])
            .into_iter()
            .min_by(|a, b| a.distance(p[2]).total_cmp(&b.distance(p[2])))
            .unwrap();
        for q in &p {
            assert!(q.distance(old) <= 0.1 + 1e-9, "{q} from {old}");
        }
    }
}

#[test]
fn many_sided_cap_rims_are_round_with_loop_slide() {
    // EBC-Q-RND1 (answered 2026-11-28): with Loop Slide on, the cap's side
    // moves Inset × max(1, 2·cos(φ/2)) along the corner's bisector, so a
    // rim corner wider than 120° moves exactly the Inset, as the wall does,
    // and the profile is a circle. Before, a 24-sided cap moved only
    // 0.26 Inset and its profile was a 3.9 : 1 oval.
    for n in [7u32, 8, 12, 24] {
        let m = frustum(&regular(n as usize, 0.5), 1.);
        for rows in [3, 4, 8] {
            let e = worst(&m, &rim(n), 0.04, rows, true);
            assert!(e < 1e-6, "{n}-sided rim, {rows} rows: {e}");
        }
    }
}

#[test]
fn a_square_cap_rim_is_the_stretched_quarter_circle() {
    // EBC-Q-RND1: at a 4-sided cap's 90° corner the cap side moves
    // 2·cos(45°) = 1.414 Insets along the bisector and the wall side one
    // Inset down. The profile is the quarter circle stretched to those two
    // distances about the old corner, at equal angle steps.
    let w = 0.1;
    let rows = 6;
    let cube = frustum(&regular(4, std::f64::consts::FRAC_1_SQRT_2), 1.);
    let r = bevel(&cube, &rim(4), w, rows, true);
    for p in profiles(&cube, &r, rows) {
        // The old corner nearest this profile.
        let v = (4..8)
            .map(|i| cube.vertices[i])
            .min_by(|a, b| a.distance(p[0]).total_cmp(&b.distance(p[0])))
            .unwrap();
        // Wall side first: straight down from the corner.
        let p: Vec<DVec3> = if p[0].y < p[p.len() - 1].y {
            p
        } else {
            p.into_iter().rev().collect()
        };
        let down = p[0] - v;
        let cap = p[p.len() - 1] - v;
        assert!(
            (down.length() - w).abs() < 1e-9,
            "wall side {}",
            down.length()
        );
        assert!(
            (cap.length() - w * std::f64::consts::SQRT_2).abs() < 1e-9,
            "cap side {}",
            cap.length()
        );
        // In the frame of the two sides, rows lie on the unit circle about
        // (1, 1), at equal angles.
        let (d, c) = (down / down.length_squared(), cap / cap.length_squared());
        let angles: Vec<f64> = p
            .iter()
            .map(|&q| {
                let (l1, l2) = ((q - v).dot(d), (q - v).dot(c));
                let e = (l1 - 1.).powi(2) + (l2 - 1.).powi(2);
                assert!((e - 1.).abs() < 1e-9, "{l1} {l2}");
                (1. - l1).atan2(1. - l2)
            })
            .collect();
        let step = std::f64::consts::FRAC_PI_2 / rows as f64;
        for w in angles.windows(2) {
            assert!(((w[1] - w[0]).abs() - step).abs() < 1e-9, "{angles:?}");
        }
    }
}
