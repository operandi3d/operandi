//! Tweak TWK-2 against the observed oracles in
//! `specifications/tweak-2026-11-03/ORACLES.json`.
//!
//! The oracles' drag distances are unreliable (README caveat), so each case
//! checks which points move and their weight relative to the grabbed
//! element (`relative_weights`, tolerance 1e-5), as the spec asks.
use glam::{DMat4, DVec3};
use operandi_core::{
    Face, FalloffShape, Mesh, Symmetry, TweakConnect, TweakElement, TweakGrab, TweakMode,
    TweakOptions, tweak_apply, tweak_plan,
};
use serde_json::Value;

const ORACLES: &str = include_str!("../../../specifications/tweak-2026-11-03/ORACLES.json");

fn oracle(id: &str) -> Value {
    let all: Value = serde_json::from_str(ORACLES).unwrap();
    all["oracles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == id)
        .unwrap_or_else(|| panic!("no oracle {id}"))
        .clone()
}

fn input_mesh(o: &Value) -> Mesh {
    let input = &o["input"];
    Mesh {
        vertices: input["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                DVec3::new(
                    p[0].as_f64().unwrap(),
                    p[1].as_f64().unwrap(),
                    p[2].as_f64().unwrap(),
                )
            })
            .collect(),
        faces: input["polygons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| Face {
                vertices: f
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect(),
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            })
            .collect(),
        ..Default::default()
    }
}

fn options(o: &Value) -> TweakOptions {
    let s = &o["settings"];
    let mode = TweakMode::ALL
        .into_iter()
        .find(|m| m.label() == s["Mode"].as_str().unwrap())
        .unwrap();
    let connect = match s["Connected Elements"].as_str().unwrap() {
        "Ignore" => TweakConnect::Ignore,
        "Use connectivity" => TweakConnect::Connectivity,
        "Rigid Connections" => TweakConnect::Rigid,
        "Edge Loop" => TweakConnect::EdgeLoops,
        other => panic!("unknown setting {other}"),
    };
    let shape = match s["Shape Preset"].as_str().unwrap() {
        "Linear" => FalloffShape::Linear,
        "Smooth" => FalloffShape::Smooth,
        other => panic!("unknown shape {other}"),
    };
    TweakOptions {
        mode,
        range: s["Range_m"].as_f64().unwrap(),
        connect,
        shape,
    }
}

/// Plans and applies a unit tweak of `element` with the oracle's settings,
/// then compares each point's move with the oracle's relative weights. The
/// mesh's polygons must come out untouched (TWK-22).
fn check(id: &str, element: TweakElement) {
    let o = oracle(id);
    let mesh = input_mesh(&o);
    let options = options(&o);
    let grab = TweakGrab { item: 1, element };
    let plan = tweak_plan(&mesh, DMat4::IDENTITY, grab, options, Symmetry::default()).unwrap();
    let offset = DVec3::new(1., 0.5, 0.);
    let mut after = mesh.clone();
    tweak_apply(&mut after, &mesh, &plan, offset).unwrap();
    let expected = o["expected"]["relative_weights"].as_array().unwrap();
    assert_eq!(expected.len(), mesh.vertices.len(), "{id}");
    for (v, w) in expected.iter().enumerate() {
        let w = w.as_f64().unwrap();
        let moved = after.vertices[v] - mesh.vertices[v];
        let got = moved.x / offset.x;
        assert!(
            (got - w).abs() < 1e-5 && (moved - offset * got).length() < 1e-9,
            "{id} point {v}: weight {got}, expected {w}"
        );
    }
    assert_eq!(after.faces, mesh.faces, "{id}");
}

/// G4 point index for grid column x, row y (both −2..2).
fn g4(x: i32, y: i32) -> u32 {
    ((y + 2) * 5 + x + 2) as u32
}

#[test]
fn range_zero_moves_only_the_element() {
    check("TWK-O-T4-04-vertex-initial", TweakElement::Vertex(18));
    check("TWK-O-T4-04-edge", TweakElement::Edge(17, 18));
    check("TWK-O-T4-04-polygon", TweakElement::Polygon(10));
}

#[test]
fn linear_and_smooth_weights_follow_distance_to_the_vertex() {
    check("TWK-O-T4-08-linear", TweakElement::Vertex(g4(0, 0)));
    check("TWK-O-T4-08-smooth", TweakElement::Vertex(g4(0, 0)));
}

#[test]
fn polygon_falloff_measures_from_the_nearest_corner() {
    check("TWK-O-T4-08-autocenter-polygon", TweakElement::Polygon(10));
}

// The C2 trials picked the left cube's corner at (0.5, 0.5) in the Front
// view, where points 5 (z −0.5) and 6 (z 0.5) overlap; both got weight 1.
// Spec reading (a) (TWK-13): both corners were grabbed and d is 3D. The
// edge 5–6 is exactly that pair of points.
#[test]
fn connected_elements_on_two_cubes() {
    let pair = TweakElement::Edge(5, 6);
    check("TWK-O-T4-09-ignore-vertex", pair);
    check("TWK-O-T4-09-connectivity", pair);
    check("TWK-O-T4-09-rigid", pair);
    // Edge Loops + Range behaves as Ignore here (the cube's corner stops
    // the loop at once), as TWK-16 says to treat it for now.
    check("TWK-O-T4-09-edge-loop", pair);
}

#[test]
fn edge_loops_move_the_loop_of_an_edge_and_only_the_element_otherwise() {
    check("TWK-O-T4-10-edge", TweakElement::Edge(17, 18));
    check("TWK-O-T4-10-vertex", TweakElement::Vertex(18));
    check("TWK-O-T4-10-polygon", TweakElement::Polygon(10));
}

#[test]
fn rigid_ignores_range() {
    let o = oracle("TWK-O-T4-09-rigid");
    let mesh = input_mesh(&o);
    let grab = TweakGrab {
        item: 1,
        element: TweakElement::Vertex(5),
    };
    for range in [0., 0.5, 50.] {
        let options = TweakOptions {
            range,
            connect: TweakConnect::Rigid,
            ..Default::default()
        };
        let plan = tweak_plan(&mesh, DMat4::IDENTITY, grab, options, Symmetry::default()).unwrap();
        assert_eq!(*plan.moves, (0..8).map(|v| (v, 1.)).collect::<Vec<_>>());
    }
}

#[test]
fn range_is_a_world_distance() {
    // Scaling the item ×2 doubles the distances, so a 1.5 m Range reaches
    // no neighbour 1 m (2 m in the world) away.
    let o = oracle("TWK-O-T4-08-linear");
    let mesh = input_mesh(&o);
    let grab = TweakGrab {
        item: 1,
        element: TweakElement::Vertex(g4(0, 0)),
    };
    let options = TweakOptions {
        range: 1.5,
        ..Default::default()
    };
    let plan = tweak_plan(
        &mesh,
        DMat4::from_scale(DVec3::splat(2.)),
        grab,
        options,
        Symmetry::default(),
    )
    .unwrap();
    assert_eq!(*plan.moves, vec![(g4(0, 0), 1.)]);
}

#[test]
fn bad_range_and_missing_elements_are_refused() {
    let mesh = input_mesh(&oracle("TWK-O-T4-08-linear"));
    let grab = |element| TweakGrab { item: 1, element };
    for range in [-1., f64::NAN, f64::INFINITY] {
        let options = TweakOptions {
            range,
            ..Default::default()
        };
        assert!(
            tweak_plan(
                &mesh,
                DMat4::IDENTITY,
                grab(TweakElement::Vertex(0)),
                options,
                Symmetry::default()
            )
            .is_err()
        );
    }
    assert!(
        tweak_plan(
            &mesh,
            DMat4::IDENTITY,
            grab(TweakElement::Vertex(99)),
            TweakOptions::default(),
            Symmetry::default()
        )
        .is_err()
    );
}

#[test]
fn edge_loop_crosses_a_grid_and_stops_at_its_borders() {
    let mesh = input_mesh(&oracle("TWK-O-T4-10-edge"));
    // A vertical interior edge runs the whole column.
    assert_eq!(
        operandi_core::edge_loop_points(&mesh, g4(0, 0), g4(0, 1)),
        (-2..=2).map(|y| g4(0, y)).collect::<Vec<_>>()
    );
    // A border edge follows the border to the corners.
    assert_eq!(
        operandi_core::edge_loop_points(&mesh, g4(-1, -2), g4(0, -2)),
        (-2..=2).map(|x| g4(x, -2)).collect::<Vec<_>>()
    );
}

#[test]
fn symmetry_spreads_the_falloff_around_both_grabs() {
    // TWK-18 with Range (provisional): Symmetry X on G4, grab (1, 0),
    // Range 1.5. (-1, 0) mirrors the grab at full weight; (0, 0), 1 m from
    // both, takes 1/3 and stays on the plane.
    let mesh = input_mesh(&oracle("TWK-O-T4-08-linear"));
    let symmetry = Symmetry {
        axis: Some(0),
        ..Default::default()
    };
    let grab = TweakGrab {
        item: 1,
        element: TweakElement::Vertex(g4(1, 0)),
    };
    let options = TweakOptions {
        range: 1.5,
        ..Default::default()
    };
    let plan = tweak_plan(&mesh, DMat4::IDENTITY, grab, options, symmetry).unwrap();
    let mut after = mesh.clone();
    let offset = DVec3::new(0.3, 0.6, 0.);
    tweak_apply(&mut after, &mesh, &plan, offset).unwrap();
    let moved = |x, y| after.vertices[g4(x, y) as usize] - mesh.vertices[g4(x, y) as usize];
    assert!((moved(1, 0) - offset).length() < 1e-12);
    assert!((moved(-1, 0) - DVec3::new(-0.3, 0.6, 0.)).length() < 1e-12);
    assert!(
        (moved(0, 0) - DVec3::new(0., 0.2, 0.)).length() < 1e-12,
        "{:?}",
        moved(0, 0)
    );
    assert!((moved(2, 0) - offset / 3.).length() < 1e-12);
    assert!((moved(-2, 0) - DVec3::new(-0.1, 0.2, 0.)).length() < 1e-12);
}
