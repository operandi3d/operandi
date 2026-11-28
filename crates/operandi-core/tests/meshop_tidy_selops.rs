//! Mesh Stack tidy batch (MESHOPS-TIDY-SPEC 2026-11-27): Select by Falloff
//! (TB-01..05) and Select Conditional Loop (TB-10..20), on the spec's
//! worked examples.
use glam::DVec3;
use operandi_core::meshop_conditional_loop::{LoopMethod, LoopRules};
use operandi_core::meshop_selection::{SelKind, SelOp, resolve};
use operandi_core::meshop_types::SelElement;
use operandi_core::{Face, Falloff, LinearFalloff, Mesh, ShapeWeights};

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
/// `cols` x `rows` unit quads in XY; point id = row·(cols+1) + column.
pub(crate) fn grid(cols: u32, rows: u32) -> Mesh {
    let mut m = Mesh::default();
    for r in 0..=rows {
        for c in 0..=cols {
            m.vertices.push(DVec3::new(c as f64, r as f64, 0.));
        }
    }
    let w = cols + 1;
    for r in 0..rows {
        for c in 0..cols {
            let a = r * w + c;
            m.faces.push(face(vec![a, a + 1, a + w + 1, a + w]));
        }
    }
    m.validate().unwrap();
    m
}
fn pick(ops: &[SelOp], mesh: &Mesh, element: SelElement) -> Vec<u32> {
    resolve(None, ops, mesh, element).unwrap().unwrap()
}

// ---- Select by Falloff

/// Weight 1 for x ≤ 0, falling to 0 at x = 1 (TB worked example).
fn linear_x() -> ShapeWeights {
    ShapeWeights::new(
        Falloff::Linear(LinearFalloff {
            start: DVec3::ZERO,
            end: DVec3::X,
            ..Default::default()
        }),
        Vec::new(),
    )
    .unwrap()
}
/// Four quads in a row, x from −2 to 2.
fn row_of_four() -> Mesh {
    let mut m = grid(4, 1);
    m.vertices.iter_mut().for_each(|p| p.x -= 2.);
    m
}
fn by_falloff(threshold: f64, falloff: Option<ShapeWeights>) -> SelOp {
    SelOp::new(SelKind::Falloff { threshold, falloff })
}

#[test]
fn select_by_falloff_picks_by_polygon_centre_weight() {
    let m = row_of_four();
    let f = || Some(linear_x());
    for (threshold, want) in [
        (0., vec![0, 1, 2, 3]),
        (0.25, vec![0, 1, 2]),
        (0.5, vec![0, 1, 2]),
        (0.75, vec![0, 1]),
        (1., vec![0, 1]),
    ] {
        assert_eq!(
            pick(&[by_falloff(threshold, f())], &m, SelElement::Polygon),
            want,
            "threshold {threshold}"
        );
    }
    let mut inverted = by_falloff(0.5, f());
    inverted.invert = true;
    assert_eq!(pick(&[inverted], &m, SelElement::Polygon), vec![3]);
}

#[test]
fn select_by_falloff_without_a_falloff_picks_nothing_but_off_drops_out() {
    let m = row_of_four();
    assert!(pick(&[by_falloff(0.5, None)], &m, SelElement::Polygon).is_empty());
    let mut off = by_falloff(0.5, Some(linear_x()));
    off.enable = false;
    assert_eq!(
        resolve(None, &[off], &m, SelElement::Polygon).unwrap(),
        None,
        "a turned-off selection op leaves everything"
    );
}

#[test]
fn select_by_falloff_uses_the_centre_not_the_corner_weights() {
    // TB-01: corner weights 1, 1, 0; centre weight 2/3.
    let mut m = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(0., 1., 0.),
            DVec3::new(1., 0., 0.),
        ],
        ..Default::default()
    };
    m.faces.push(face(vec![0, 1, 2]));
    let at = |t| pick(&[by_falloff(t, Some(linear_x()))], &m, SelElement::Polygon);
    assert_eq!(at(0.6), vec![0]);
    assert!(at(0.7).is_empty());
    // Points by their own places.
    let points = pick(&[by_falloff(0.5, Some(linear_x()))], &m, SelElement::Point);
    assert_eq!(points, vec![0, 1]);
}

#[test]
fn select_by_falloff_round_trips_and_is_in_the_menu() {
    let op = by_falloff(0.3, Some(linear_x()));
    let text = serde_json::to_string(&op).unwrap();
    assert!(text.contains("pmodel.falloffSelOp"), "{text}");
    assert_eq!(serde_json::from_str::<SelOp>(&text).unwrap(), op);
    assert!(
        SelKind::menu()
            .iter()
            .any(|(l, k)| *l == "Select by Falloff" && k.label() == *l)
    );
}

// ---- Select Conditional Loop

/// Edge number of the pair (a, b).
fn edge(m: &Mesh, a: u32, b: u32) -> u32 {
    m.edges()
        .iter()
        .position(|&e| e == (a.min(b), a.max(b)))
        .unwrap() as u32
}
fn edges(m: &Mesh, pairs: &[(u32, u32)]) -> Vec<u32> {
    let mut v: Vec<u32> = pairs.iter().map(|&(a, b)| edge(m, a, b)).collect();
    v.sort_unstable();
    v
}
fn seed(m: &Mesh, pairs: &[(u32, u32)]) -> SelOp {
    let list: Vec<String> = edges(m, pairs).iter().map(|e| e.to_string()).collect();
    SelOp::new(SelKind::Index {
        index: list.join(","),
    })
}
fn looped(rules: LoopRules) -> SelOp {
    SelOp::new(SelKind::ConditionalLoop(rules))
}
const COLUMN: [(u32, u32); 4] = [(2, 7), (7, 12), (12, 17), (17, 22)];

#[test]
fn conditional_loop_grows_the_seed_border_to_border() {
    let m = grid(4, 4);
    let got = pick(
        &[seed(&m, &[(7, 12)]), looped(LoopRules::default())],
        &m,
        SelElement::Edge,
    );
    assert_eq!(got, edges(&m, &COLUMN));
    // TB-10: nothing coming in, nothing out.
    assert!(pick(&[looped(LoopRules::default())], &m, SelElement::Edge).is_empty());
}

#[test]
fn conditional_loop_one_direction_and_max_count() {
    let m = grid(4, 4);
    let run = |rules: LoopRules| pick(&[seed(&m, &[(7, 12)]), looped(rules)], &m, SelElement::Edge);
    let one = |ab| LoopRules {
        loop_both_dirs: false,
        direction_ab: ab,
        ..Default::default()
    };
    assert_eq!(run(one(true)), edges(&m, &[(2, 7), (7, 12)]));
    assert_eq!(run(one(false)), edges(&m, &[(7, 12), (12, 17), (17, 22)]));
    let cap = |n| LoopRules {
        cap_loop_count: n,
        ..Default::default()
    };
    assert_eq!(run(cap(1)), edges(&m, &[(7, 12)]));
    assert_eq!(run(cap(2)), edges(&m, &[(2, 7), (7, 12), (12, 17)]));
}

#[test]
fn conditional_loop_stops_where_selected_edges_meet() {
    let m = grid(4, 4);
    let seeds = [(7, 12), (16, 17), (17, 18)];
    let run = |rules: LoopRules| pick(&[seed(&m, &seeds), looped(rules)], &m, SelElement::Edge);
    let got = run(LoopRules::default());
    assert!(got.contains(&edge(&m, 12, 17)));
    assert!(!got.contains(&edge(&m, 17, 22)), "stops at 17");
    let count = |n| LoopRules {
        stop_at_selection_count: n,
        ..Default::default()
    };
    assert!(!run(count(1)).contains(&edge(&m, 17, 22)));
    assert!(run(count(3)).contains(&edge(&m, 17, 22)), "only 2 meet");
    let off = LoopRules {
        stop_at_selection: false,
        ..Default::default()
    };
    assert!(run(off).contains(&edge(&m, 17, 22)));
}

#[test]
fn conditional_loop_middle_stops_at_a_five_edge_point_angle_goes_on() {
    // Split quad (2,2)-(3,3) [points 12,13,18,17] with the diagonal 12-18:
    // the column walk from 7-12 reaches 12, which now has 5 edges.
    let mut m = grid(4, 4);
    let q = m
        .faces
        .iter()
        .position(|f| f.vertices == vec![12, 13, 18, 17])
        .unwrap();
    m.faces[q] = face(vec![12, 13, 18]);
    m.faces.push(face(vec![12, 18, 17]));
    m.validate().unwrap();
    let run = |method| {
        let rules = LoopRules {
            method,
            loop_both_dirs: false,
            direction_ab: false,
            ..Default::default()
        };
        pick(&[seed(&m, &[(7, 12)]), looped(rules)], &m, SelElement::Edge)
    };
    assert_eq!(run(LoopMethod::Middle), edges(&m, &[(7, 12)]));
    assert_eq!(
        run(LoopMethod::Angle),
        edges(&m, &[(7, 12), (12, 17), (17, 22)])
    );
    assert_eq!(run(LoopMethod::Dot), run(LoopMethod::Angle));
}

#[test]
fn conditional_loop_sharp_angles_stop_at_a_steep_fold() {
    // Fold the grid along row 2 (y = 2): rows above turn by `angle`.
    let fold = |deg: f64| {
        let mut m = grid(4, 4);
        let a = deg.to_radians();
        for p in m.vertices.iter_mut().filter(|p| p.y > 2.) {
            let d = p.y - 2.;
            *p = DVec3::new(p.x, 2. + d * a.cos(), d * a.sin());
        }
        m
    };
    let run = |m: &Mesh, sharp| {
        let rules = LoopRules {
            stop_at_sharp_angles: sharp,
            ..Default::default()
        };
        pick(&[seed(m, &[(7, 12)]), looped(rules)], m, SelElement::Edge)
    };
    let m = fold(60.);
    assert_eq!(run(&m, true), edges(&m, &COLUMN), "60° passes");
    let m = fold(100.);
    let got = run(&m, true);
    assert!(!got.contains(&edge(&m, 12, 17)), "100° stops at the fold");
    assert_eq!(run(&m, false), edges(&m, &COLUMN), "rule off passes");
}

#[test]
fn conditional_loop_invert_stop_set_and_round_trip() {
    let m = grid(4, 4);
    let mut inv = looped(LoopRules::default());
    inv.invert = true;
    let got = pick(&[seed(&m, &[(7, 12)]), inv], &m, SelElement::Edge);
    assert_eq!(got.len(), m.edges().len() - 4);
    // Stop At Selection Set: set edges meeting at 17 stop the walk.
    let mut m = grid(4, 4);
    m.attributes
        .edge_sets
        .insert("Stop".into(), [(16, 17), (17, 18)].into_iter().collect());
    let rules = LoopRules {
        stop_at_set_name: "Stop".into(),
        ..Default::default()
    };
    let got = pick(
        &[seed(&m, &[(7, 12)]), looped(rules.clone())],
        &m,
        SelElement::Edge,
    );
    assert!(!got.contains(&edge(&m, 17, 22)));
    let op = looped(rules);
    let text = serde_json::to_string(&op).unwrap();
    assert!(
        text.contains("selop.loopConditional.item") && text.contains("stopAtSetName"),
        "{text}"
    );
    assert_eq!(serde_json::from_str::<SelOp>(&text).unwrap(), op);
}
