//! Chamfer Edit against `specifications/chamfer-edit-2026-11-05`.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "chamfer-edit-2026-11-05";

/// Not built yet, or open.
const SKIP: &[(&str, &str)] = &[
    (
        "CE12",
        "Q-CE-05 (answered: keep Operandi's): points stay on the caps; own test",
    ),
    ("CE13", "decided Q-CE-02: a mismatch is refused (own test)"),
    ("CE15", "decided Q-CE-02: not copied"),
];

/// The round profile follows the decided arc rule (Q-CE-01), not the
/// reference's points: compared within this distance.
const PROFILE_TOLERANCE: f64 = 0.01;

fn run(c: &Value) -> std::result::Result<(), String> {
    let source = mesh(&c["input"]);
    let s = &c["settings"];
    let p = ChamferParameters {
        offset: s["offset"].as_f64().unwrap(),
        segments: s["segments"].as_str().unwrap().parse().unwrap(),
        source_segments: s["sourceSegs"].as_str().unwrap().parse().unwrap(),
        depth: s["depth"].as_f64().unwrap() * 100.,
        width_mode: s["mode"] == "width",
        absolute: s["absSize"] == 1,
        swap_side: s["swapSide"] == 1,
        reverse_x: s["reversex"] == 1,
        reverse_y: s["reversey"] == 1,
    };
    let got = chamfer_geometry(&source, &indices(&c["selection"]["sel_polygons"]), p)
        .map_err(|e| format!("refused: {e}"))?;
    let e = &c["expected"];
    // Slices of a run (CE16, CE17) are straight: exact.
    let run = c["selection"]["sel_polygons"]
        .as_array()
        .is_none_or(|l| l.is_empty() || l.len() == 3);
    let tol = if p.segments > 1 && !run {
        PROFILE_TOLERANCE
    } else {
        1e-5
    };
    same_mesh(&got, e, tol)
}

#[test]
fn every_built_case_matches() {
    let mut failures = Vec::new();
    for c in cases(SPEC) {
        let id = c["id"].as_str().unwrap();
        if SKIP.iter().any(|(s, _)| *s == id) {
            continue;
        }
        if let Err(e) = run(&c) {
            failures.push(format!("{id}: {e}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Decided (Q-CE-01): profile points are evenly spaced in angle on the arc
/// tangent to both faces.
#[test]
fn round_profile_is_an_even_arc() {
    let all = cases(SPEC);
    let c = case(&all, "CE04");
    let got = chamfer_geometry(
        &mesh(&c["input"]),
        &[1],
        ChamferParameters {
            segments: 3,
            ..Default::default()
        },
    )
    .unwrap();
    // The 90° corner chamfer from (0.8, 1) to (1, 0.8): centre (0.8, 0.8),
    // radius 0.2, points at 30° and 60°.
    let center = glam::DVec2::new(0.8, 0.8);
    let mut new: Vec<glam::DVec2> = got.vertices[8..]
        .iter()
        .filter(|v| v.z == 0.)
        .map(|v| v.truncate())
        .collect();
    new.sort_by(|a, b| a.x.total_cmp(&b.x));
    assert_eq!(new.len(), 2);
    for (q, degrees) in new.iter().zip([30f64, 60.]) {
        let want =
            center + glam::DVec2::new(degrees.to_radians().sin(), degrees.to_radians().cos()) * 0.2;
        assert!((*q - want).length() < 1e-9, "{q} vs {want}");
    }
}

/// CE12 (rule 6): a 2-segment chamfer with Source Segments 2 rebuilt in 4.
/// Counts and polygons match the reference; the new points stay on the
/// end caps (z 0 or 1) and on the decided arc (Q-CE-01), within 0.04 of the reference's
/// x, y (Q-CE-05).
#[test]
fn ce12_two_source_segments_rebuild_in_four() {
    let all = cases(SPEC);
    let c = case(&all, "CE12");
    let got = chamfer_geometry(
        &mesh(&c["input"]),
        &[1, 2],
        ChamferParameters {
            segments: 4,
            source_segments: 2,
            ..Default::default()
        },
    )
    .unwrap();
    let want = mesh(&c["expected"]);
    assert_eq!(got.vertices.len(), want.vertices.len());
    assert_eq!(got.faces.len(), want.faces.len());
    for v in &got.vertices[8..] {
        assert!(v.z == 0. || v.z == 1., "{v}");
        let near = want.vertices[8..]
            .iter()
            .any(|w| v.truncate().distance(w.truncate()) < 0.04);
        assert!(near, "{v} far from the reference's points");
        // On the arc tangent to both faces: centre (0.8, 0.8), radius 0.2.
        let r = v.truncate().distance(glam::DVec2::new(0.8, 0.8));
        assert!((r - 0.2).abs() < 1e-9, "{v}: {r}");
    }
}

/// CE13 (decided Q-CE-02): Source Segments that don't match the selected
/// chamfer are refused and nothing changes.
#[test]
fn ce13_a_source_segments_mismatch_is_refused() {
    let all = cases(SPEC);
    let c = case(&all, "CE13");
    let source = mesh(&c["input"]);
    for (selected, count) in [(vec![1, 2], 1), (vec![1], 2), (vec![1, 2], 3)] {
        let r = chamfer_geometry(
            &source,
            &selected,
            ChamferParameters {
                segments: 4,
                source_segments: count,
                ..Default::default()
            },
        );
        assert!(r.is_err(), "{selected:?} with {count}");
    }
}

/// Rule 11 through the editor: CE02 and CE10 keep the strip selected (no
/// polygons added or removed); CE04 rebuilds it and clears the selection.
#[test]
fn the_selection_stays_unless_the_strip_is_rebuilt() {
    let all = cases(SPEC);
    for (id, kept) in [("CE02", true), ("CE10", true), ("CE04", false)] {
        let c = case(&all, id);
        let mut e = Editor::default();
        e.scene.new_mesh().unwrap();
        e.scene.active_mut().unwrap().mesh = mesh(&c["input"]).into();
        e.synchronize_identities().unwrap();
        e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
        e.scene.selection.faces = indices(&c["selection"]["sel_polygons"]);
        let s = &c["settings"];
        e.dispatch(Command::ChamferOnce(ChamferParameters {
            offset: s["offset"].as_f64().unwrap(),
            segments: s["segments"].as_str().unwrap().parse().unwrap(),
            source_segments: 1,
            depth: s["depth"].as_f64().unwrap() * 100.,
            ..Default::default()
        }))
        .unwrap();
        let want = indices(&c["expected"]["selection_after"]["sel_polygons"]);
        assert_eq!(e.scene.selection.faces, want, "{id}");
        assert_eq!(!want.is_empty(), kept, "{id}");
    }
}

/// Q-CE-03 (answered 2026-11-20): with nothing selected the run is the
/// whole mesh; Offset pushes its open ends out along it and Segments slices
/// it straight across, Depth unused; with two of three selected only those
/// two are sliced; a closed cube with nothing selected doesn't change.
#[test]
fn a_run_without_outer_polygons_is_sliced_across() {
    let all = cases(SPEC);
    let c = case(&all, "CE17");
    let source = mesh(&c["input"]);
    // Offset 0.1 on the whole run: z 0 and 1 go to -0.1 and 1.1.
    let pushed = chamfer_geometry(
        &source,
        &[],
        ChamferParameters {
            offset: 0.1,
            ..Default::default()
        },
    )
    .unwrap();
    for v in &pushed.vertices {
        assert!(
            (v.z + 0.1).abs() < 1e-12 || (v.z - 1.1).abs() < 1e-12,
            "{v}"
        );
    }
    // Depth has no effect.
    let deep = chamfer_geometry(
        &source,
        &[],
        ChamferParameters {
            segments: 3,
            depth: -100.,
            ..Default::default()
        },
    )
    .unwrap();
    same_mesh(&deep, &c["expected"], 1e-5).unwrap();
    // Two of the three selected: only those two are sliced.
    let two = chamfer_geometry(
        &source,
        &[1, 2],
        ChamferParameters {
            segments: 3,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(two.faces.len(), 1 + 2 * 3);
    assert!(
        two.faces
            .iter()
            .any(|f| f.vertices == source.faces[0].vertices),
        "top untouched"
    );
    // A closed cube with nothing selected: nothing changes.
    let cube = Mesh::box_mesh(glam::DVec3::ZERO, glam::DVec3::ONE, 0).unwrap();
    let same = chamfer_geometry(
        &cube,
        &[],
        ChamferParameters {
            segments: 3,
            offset: 0.1,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(same, cube);
}

/// Operandi's rule for unequal insets (Q-CE-06): the decided arc for equal
/// insets of 1, stretched along each face to that border's inset. CE04's
/// corner (at x = y = 1) with the side border moved to an inset of 0.4 while
/// the top stays at 0.2; Segments 3, Depth 100 %.
#[test]
fn unequal_insets_stretch_the_decided_arc() {
    let mut c = cases(SPEC).into_iter().find(|c| c["id"] == "CE04").unwrap();
    for i in [2, 6] {
        c["input"]["points"][i][1] = serde_json::json!(0.6);
    }
    let source = mesh(&c["input"]);
    let p = ChamferParameters {
        segments: 3,
        ..Default::default()
    };
    let got = chamfer_geometry(&source, &[1], p).unwrap();
    // Unit arc points at 30 and 60 degrees, written along the faces from the
    // corner, then scaled by 0.2 (along -x) and 0.4 (along -y).
    let s = 3f64.sqrt() / 2.;
    let want = [
        glam::DVec3::new(1. - 0.5 * 0.2, 1. - (1. - s) * 0.4, 0.),
        glam::DVec3::new(1. - (1. - s) * 0.2, 1. - 0.5 * 0.4, 0.),
    ];
    for w in want {
        assert!(
            got.vertices.iter().any(|v| v.distance(w) < 1e-6),
            "missing profile point {w:?} in {:?}",
            got.vertices
        );
    }
}
