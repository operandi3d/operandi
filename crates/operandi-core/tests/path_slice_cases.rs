//! Wave 2 oracle cases (`docs/specs/edit-wave2/cases-wave2.json`, 21
//! cases): Curve Slice and Primitive
//! Slice on a 4 x 4 grid and a 2 m cube. Points compare by position to
//! 1e-5, order-free; polygons as point-position cycles.
//!
//! Where Operandi differs on purpose, the expectation is adjusted here, not
//! loosened:
//!
//! - A coincident duplicate point is one point (PS-10: Operandi reuses the
//!   existing point).
//! - Slits (PS-12) and keyhole seams (PS-13) need a polygon to visit a
//!   point twice, which Operandi's polygons can't. A slit is left out (the
//!   polygon keeps the crossing point on its edge, not the slit's points);
//!   a keyhole polygon is two pieces in Operandi, which must cover exactly
//!   its area with its points.
use glam::{DMat4, DVec3};
use operandi_core::*;
use serde_json::Value;

const TOLERANCE: f64 = 1e-5;

fn cases() -> Vec<Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/specs/edit-wave2/cases-wave2.json");
    let data: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    data["cases"].as_array().unwrap().clone()
}

fn vec3(v: &Value) -> DVec3 {
    let a: Vec<f64> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    DVec3::new(a[0], a[1], a[2])
}

fn points(v: &Value) -> Vec<DVec3> {
    v.as_array().unwrap().iter().map(vec3).collect()
}

fn polygons(v: &Value) -> Vec<Vec<usize>> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|p| {
            p.as_array()
                .unwrap()
                .iter()
                .map(|i| i.as_u64().unwrap() as usize)
                .collect()
        })
        .collect()
}

fn mesh_of(case: &Value) -> Mesh {
    let mut mesh = Mesh {
        vertices: points(&case["input"]["points"]),
        ..Default::default()
    };
    for p in polygons(&case["input"]["polygons"]) {
        mesh.faces.push(Face {
            vertices: p.into_iter().map(|i| i as u32).collect(),
            uv: None,
            material: "Base".into(),
            smooth: false,
            part: None,
            material_label: None,
        });
    }
    mesh
}

fn axis(name: &str) -> SliceAxis {
    match name {
        "X" => SliceAxis::X,
        "Z" => SliceAxis::Z,
        _ => SliceAxis::Y,
    }
}

fn operation(name: &str) -> SliceOperation {
    match name {
        "Core" => SliceOperation::Core,
        "Tunnel" => SliceOperation::Tunnel,
        "Stencil" => SliceOperation::Stencil,
        _ => SliceOperation::Slice,
    }
}

/// Run the case's tool on its input.
fn run(case: &Value) -> PathCutResult {
    let s = &case["settings"];
    let mesh = mesh_of(case);
    let cutter = PathCutter {
        axis: axis(s["axis"].as_str().unwrap()),
        operation: operation(s["operation"].as_str().unwrap()),
        front_face_only: s["front_face_only"].as_bool().unwrap_or(false),
        ..Default::default()
    };
    let (paths, action) = match s["tool"].as_str().unwrap() {
        "Curve Slice" => {
            let c = CurveSlice {
                points: points(&s["curve"]),
                steps: s["steps"].as_u64().unwrap() as u32,
                ..Default::default()
            };
            (c.paths(), c.plane_normal)
        }
        _ => {
            let half: Vec<f64> = s["half_size"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect();
            let shape = match s["type"].as_str().unwrap() {
                "Circle" => PrimitiveShape::Circle,
                _ => PrimitiveShape::Rectangle,
            };
            let normal = cutter.direction(DVec3::Y).unwrap();
            // [Half Size X, Half Size Y]: local X, local Y of the shape's
            // frame (world Z, world X on Axis Y; PS-40).
            let p = PrimitiveSlice {
                shape,
                place: Some(ShapePlace::on(vec3(&s["center"]), normal)),
                half_x: half[0],
                half_y: half[1],
                heading: s["heading_deg"].as_f64().unwrap_or(0.),
                pitch: s["pitch_deg"].as_f64().unwrap_or(0.),
                radius: s["radius"].as_f64().unwrap_or(0.5),
                sides: s["sides"].as_u64().unwrap_or(24) as u32,
                bank: s["bank_deg"].as_f64().unwrap_or(0.),
                ..Default::default()
            };
            (p.paths(), normal)
        }
    };
    let direction = cutter.direction(action).unwrap();
    path_cut(&mesh, DMat4::IDENTITY, &paths, direction, &cutter, &[]).unwrap()
}

fn same(a: DVec3, b: DVec3) -> bool {
    a.distance(b) < TOLERANCE
}

/// Positions without coincident duplicates.
fn unique(points: &[DVec3]) -> Vec<DVec3> {
    let mut out: Vec<DVec3> = Vec::new();
    for &p in points {
        if !out.iter().any(|&q| same(p, q)) {
            out.push(p);
        }
    }
    out
}

/// A slit runs in and back out along the same points (…a b c b a…): keep
/// one visit of its entry point.
fn collapse_slits(cycle: &[DVec3]) -> Vec<DVec3> {
    let mut c = cycle.to_vec();
    loop {
        let n = c.len();
        let turn = (0..n).find(|&i| n > 3 && same(c[(i + n - 1) % n], c[(i + 1) % n]));
        let Some(i) = turn else {
            return c;
        };
        // Grow the palindrome round the turning point.
        let mut k = 1;
        while k < n / 2 && same(c[(i + n - k - 1) % n], c[(i + k + 1) % n]) {
            k += 1;
        }
        let drop: Vec<usize> = (0..2 * k).map(|j| (i + n - k + 1 + j) % n).collect();
        c = (0..n).filter(|j| !drop.contains(j)).map(|j| c[j]).collect();
    }
}

/// A point repeated next to itself (the reference's duplicate where a path
/// passes through an existing point) is one corner.
fn drop_repeats_in_a_row(cycle: &[DVec3]) -> Vec<DVec3> {
    let n = cycle.len();
    (0..n)
        .filter(|&i| !same(cycle[i], cycle[(i + 1) % n]))
        .map(|i| cycle[i])
        .collect()
}

/// Our cycle without the points that sit on a straight side between their
/// neighbours and aren't in `want`: Operandi gives a polygon the points cut
/// into its edges by a neighbour (no T-junctions); the reference doesn't
/// (seen with Front Face Only on a cube's sides).
fn without_edge_points(cycle: &[DVec3], want: &[DVec3]) -> Vec<DVec3> {
    let mut c = cycle.to_vec();
    loop {
        let n = c.len();
        let extra = (0..n).find(|&i| {
            let (a, p, b) = (c[(i + n - 1) % n], c[i], c[(i + 1) % n]);
            !want.iter().any(|&q| same(p, q))
                && ((p - a).cross(b - p)).length() < 1e-9
                && (p - a).dot(b - p) > 0.
        });
        match extra {
            Some(i) if n > 3 => {
                c.remove(i);
            }
            _ => return c,
        }
    }
}

fn has_repeat(cycle: &[DVec3]) -> bool {
    (0..cycle.len()).any(|i| (i + 1..cycle.len()).any(|j| same(cycle[i], cycle[j])))
}

fn same_cycle(a: &[DVec3], b: &[DVec3]) -> bool {
    a.len() == b.len()
        && (0..b.len()).any(|shift| (0..a.len()).all(|i| same(a[i], b[(i + shift) % b.len()])))
}

/// Twice the signed area of a flat cycle, as a vector.
fn area(cycle: &[DVec3]) -> DVec3 {
    let n = cycle.len();
    (0..n).map(|i| cycle[i].cross(cycle[(i + 1) % n])).sum()
}

fn check(case: &Value) -> std::result::Result<(), String> {
    let id = case["id"].as_str().unwrap();
    let r = run(case);
    let got_points = r.mesh.vertices.clone();
    let got: Vec<Vec<DVec3>> = r
        .mesh
        .faces
        .iter()
        .map(|f| f.vertices.iter().map(|&v| got_points[v as usize]).collect())
        .collect();
    let expected_points = points(&case["expected"]["points"]);
    let mut expected: Vec<Vec<DVec3>> = polygons(&case["expected"]["polygons"])
        .into_iter()
        .map(|p| p.into_iter().map(|i| expected_points[i]).collect())
        .collect();
    expected = expected
        .iter()
        .map(|c| collapse_slits(&drop_repeats_in_a_row(c)))
        .collect();
    // PS-21 says to use Operandi's own curve evaluation, and the reference's
    // curve through three points bends a little differently, so a bent
    // curve's step points can't match by position: check the counts and
    // that the cut passes through its middle point.
    if case["settings"]["curve"]
        .as_array()
        .is_some_and(|c| c.len() > 2)
    {
        let middle = vec3(&case["settings"]["curve"][1]) * DVec3::new(1., 0., 1.);
        if got.len() != expected.len() || !got_points.iter().any(|&p| same(p, middle)) {
            return Err(format!(
                "{id}: {} polygons, want {}; middle point on the cut: {}",
                got.len(),
                expected.len(),
                got_points.iter().any(|&p| same(p, middle))
            ));
        }
        return Ok(());
    }
    // Expected points: no duplicates, only those a polygon still uses.
    let used = unique(&expected.concat());
    let want_points: Vec<DVec3> = unique(&expected_points)
        .into_iter()
        .filter(|p| used.iter().any(|&q| same(*p, q)))
        .collect();
    if got_points.len() != want_points.len()
        || !want_points
            .iter()
            .all(|&p| got_points.iter().any(|&q| same(p, q)))
    {
        let missing: Vec<_> = want_points
            .iter()
            .filter(|&&p| !got_points.iter().any(|&q| same(p, q)))
            .collect();
        let extra: Vec<_> = got_points
            .iter()
            .filter(|&&p| !want_points.iter().any(|&q| same(p, q)))
            .collect();
        return Err(format!(
            "{id}: {} points, want {}; missing {missing:?}; extra {extra:?}",
            got_points.len(),
            want_points.len()
        ));
    }
    let mut unmatched: Vec<&Vec<DVec3>> = got.iter().collect();
    let mut keyholes: Vec<&Vec<DVec3>> = Vec::new();
    for want in &expected {
        if has_repeat(want) {
            keyholes.push(want);
            continue;
        }
        match unmatched
            .iter()
            .position(|g| same_cycle(g, want) || same_cycle(&without_edge_points(g, want), want))
        {
            Some(i) => {
                unmatched.remove(i);
            }
            None => return Err(format!("{id}: no polygon {want:?}")),
        }
    }
    // Each keyhole: Operandi's two pieces cover its area with its points.
    for hole in keyholes {
        let pieces: Vec<&Vec<DVec3>> = unmatched
            .iter()
            .copied()
            .filter(|g| g.iter().all(|p| hole.iter().any(|q| same(*p, *q))))
            .collect();
        let total: DVec3 = pieces.iter().map(|g| area(g)).sum();
        if pieces.len() != 2 || total.distance(area(hole)) > 1e-6 {
            return Err(format!(
                "{id}: keyhole {hole:?} not covered by two pieces ({} found)",
                pieces.len()
            ));
        }
        unmatched.retain(|g| !pieces.contains(g));
    }
    if !unmatched.is_empty() {
        return Err(format!("{id}: extra polygons {unmatched:?}"));
    }
    // PS-17: nothing selected before, nothing after.
    if !r.selected.is_empty() {
        return Err(format!("{id}: selection {:?}", r.selected));
    }
    // Stencil: the inside polygons get the material.
    if case["settings"]["operation"] == "Stencil" {
        let tagged = r
            .mesh
            .faces
            .iter()
            .filter(|f| f.material == "Default")
            .count();
        if tagged != 4 {
            return Err(format!("{id}: {tagged} stencilled polygons, want 4"));
        }
    }
    Ok(())
}

#[test]
fn wave2_oracle_cases() {
    let cases = cases();
    assert_eq!(cases.len(), 21);
    let failures: Vec<String> = cases.iter().filter_map(|c| check(c).err()).collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
