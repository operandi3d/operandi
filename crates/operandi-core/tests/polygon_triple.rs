//! MP-C04 TRI-001..005 core acceptance for basic Polygon Triple.
//!
//! Shapes, selections and expected counts/areas come from the admitted
//! original `ORIGINAL_FIXTURES.json`. Rich payload, refusal and island scenes
//! are independently authored here with the current Rust API. Geometric
//! results are judged by `assert_exact_triangulation`, an oracle written
//! independently of the operation: Newell plane, shoelace area, directed-edge
//! pairing and even-odd sampling coverage. It never reads the operation's
//! chart, ear order or tolerances.
use glam::{DVec2, DVec3};
use operandi_core::*;
use std::collections::BTreeMap;

fn face(v: &[u32]) -> Face {
    Face {
        vertices: v.to_vec(),
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

fn plain(positions: &[DVec3], faces: &[Vec<u32>]) -> Mesh {
    let m = Mesh {
        vertices: positions.to_vec(),
        faces: faces.iter().map(|f| face(f)).collect(),
        ..Default::default()
    };
    m.validate().unwrap();
    m
}

/// `mesh` active in Polygon mode with `selected`, plus an unrelated tagged,
/// moved box item that must never change.
fn editor(mesh: Mesh, selected: &[u32]) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let target = e.scene.selection.item.unwrap();
    e.scene.new_mesh().unwrap();
    let mut unrelated = Mesh::box_mesh(DVec3::new(5., 0., 0.), DVec3::ONE, 1).unwrap();
    for (i, f) in unrelated.faces.iter_mut().enumerate() {
        f.part = (i % 2 == 0).then(|| "Unrelated".into());
    }
    for item in &mut e.scene.items {
        if item.id == target {
            item.mesh = mesh.clone().into();
            item.name = "Target".into();
        } else {
            item.mesh = unrelated.clone().into();
            item.transform.translation = DVec3::new(0., 1., 0.);
        }
    }
    e.scene.select_item(Some(target));
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = selected.to_vec();
    e.scene.validate().unwrap();
    e.undo.clear();
    e
}

fn mesh(e: &Editor) -> &Mesh {
    &e.scene.active().unwrap().mesh
}

fn newell(points: &[DVec3]) -> DVec3 {
    let mut n = DVec3::ZERO;
    for i in 0..points.len() {
        n += points[i].cross(points[(i + 1) % points.len()]);
    }
    n
}

fn basis(n: DVec3) -> (DVec3, DVec3) {
    let seed = if n.x.abs() < 0.9 { DVec3::X } else { DVec3::Y };
    let u = n.cross(seed).normalize();
    (u, n.cross(u))
}

fn segment_distance(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let d = b - a;
    let t = ((p - a).dot(d) / d.length_squared()).clamp(0., 1.);
    p.distance(a + d * t)
}

fn even_odd(ring: &[DVec2], p: DVec2) -> bool {
    let mut inside = false;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) {
            inside = !inside;
        }
    }
    inside
}

fn in_triangle(t: &[DVec2; 3], p: DVec2) -> bool {
    let s = |a: DVec2, b: DVec2| (b - a).perp_dot(p - a);
    let (x, y, z) = (s(t[0], t[1]), s(t[1], t[2]), s(t[2], t[0]));
    (x > 0. && y > 0. && z > 0.) || (x < 0. && y < 0. && z < 0.)
}

/// Independent TRI-002 oracle for source `face` of `source` replaced by
/// output faces `pieces` of `out`.
fn assert_exact_triangulation(source: &Mesh, face: usize, out: &Mesh, pieces: &[u32]) {
    let poly = &source.faces[face].vertices;
    let n = poly.len();
    assert_eq!(pieces.len(), n - 2, "n - 2 triangles");
    assert_eq!(out.vertices, source.vertices, "positions/points exact");
    let ring3: Vec<DVec3> = poly.iter().map(|&v| source.vertices[v as usize]).collect();
    let normal = newell(&ring3);
    let area = normal.length() / 2.;
    let unit = normal / normal.length();
    let mut sum = 0.;
    let mut directed = BTreeMap::<(u32, u32), u32>::new();
    for &p in pieces {
        let t = &out.faces[p as usize].vertices;
        assert_eq!(t.len(), 3);
        assert!(
            t.iter().all(|v| poly.contains(v)),
            "existing boundary vertices only"
        );
        let [a, b, c] = [t[0], t[1], t[2]].map(|v| out.vertices[v as usize]);
        let signed = (b - a).cross(c - a).dot(unit) / 2.;
        assert!(
            signed > 1e-9 * area,
            "positive area in source winding: {signed}"
        );
        sum += signed;
        for k in 0..3 {
            *directed.entry((t[k], t[(k + 1) % 3])).or_default() += 1;
        }
    }
    assert!((sum - area).abs() <= 1e-9 * area, "area {sum} vs {area}");
    let boundary = |a: u32, b: u32| (0..n).any(|i| poly[i] == a && poly[(i + 1) % n] == b);
    for i in 0..n {
        assert_eq!(directed.get(&(poly[i], poly[(i + 1) % n])), Some(&1));
        assert!(!directed.contains_key(&(poly[(i + 1) % n], poly[i])));
    }
    for (&(a, b), &count) in &directed {
        assert_eq!(count, 1, "no duplicate surface");
        if !boundary(a, b) {
            assert_eq!(
                directed.get(&(b, a)),
                Some(&1),
                "diagonal used twice, opposite"
            );
        }
    }
    // Sampling coverage: every interior sample in exactly one triangle,
    // every exterior sample (including a concave notch) in none.
    let (u, w) = basis(unit);
    let origin = ring3[0];
    let chart = |p: DVec3| DVec2::new((p - origin).dot(u), (p - origin).dot(w));
    let ring: Vec<DVec2> = ring3.iter().map(|&p| chart(p)).collect();
    let triangles: Vec<[DVec2; 3]> = pieces
        .iter()
        .map(|&p| {
            let t = &out.faces[p as usize].vertices;
            [t[0], t[1], t[2]].map(|v| chart(out.vertices[v as usize]))
        })
        .collect();
    let mut segments: Vec<(DVec2, DVec2)> = (0..n).map(|i| (ring[i], ring[(i + 1) % n])).collect();
    for t in &triangles {
        segments.extend([(t[0], t[1]), (t[1], t[2]), (t[2], t[0])]);
    }
    let lo = ring.iter().copied().fold(DVec2::INFINITY, DVec2::min);
    let hi = ring.iter().copied().fold(DVec2::NEG_INFINITY, DVec2::max);
    let size = hi - lo;
    let (mut inside_samples, mut outside_samples) = (0, 0);
    for i in 0..48 {
        for j in 0..48 {
            let s = lo - size * 0.05
                + size * 1.1 * DVec2::new((i as f64 + 0.37) / 48., (j as f64 + 0.61) / 48.);
            if segments
                .iter()
                .any(|&(a, b)| segment_distance(s, a, b) < 1e-6 * size.length())
            {
                continue;
            }
            let inside = even_odd(&ring, s);
            let count = triangles.iter().filter(|t| in_triangle(t, s)).count();
            assert_eq!(count, usize::from(inside), "coverage at {s:?}");
            if inside {
                inside_samples += 1;
            } else {
                outside_samples += 1;
            }
        }
    }
    assert!(inside_samples > 100 && outside_samples > 10);
}

fn triple(e: &mut Editor) {
    e.dispatch(Command::Triple).unwrap();
}

/// Every refusal leaves the whole editor (scene, history, drafts) unchanged.
fn refuses(e: &mut Editor) -> String {
    let before = e.clone();
    let err = e.dispatch(Command::Triple).expect_err("refused");
    assert!(*e == before, "refusal must be atomic: {err}");
    err
}

#[test]
fn tri_005e_existing_triangle_kept_and_all_triangle_selection_is_neutral() {
    let positions = [
        (0., 0., 0.),
        (1., 0., 0.),
        (1., 0., 1.),
        (0., 0., 1.),
        (2., 0., 0.),
    ]
    .map(|(x, y, z)| DVec3::new(x, y, z));
    let faces = vec![vec![0, 3, 2, 1], vec![1, 2, 4]];
    let source = plain(&positions, &faces);
    let mut e = editor(source.clone(), &[1, 0]);
    triple(&mut e);
    let out = mesh(&e).clone();
    assert_eq!(out.faces[1], source.faces[1], "existing triangle unchanged");
    let selection = e.scene.selection.faces.clone();
    assert_eq!(selection[0], 1, "triangle stays selected in order");
    assert_eq!(selection.len(), 3);
    assert_exact_triangulation(&source, 0, &out, &selection[1..]);
    assert_eq!(e.undo.len(), 1);

    // Triangles only: exact no-op with no history, redo or scene change.
    let mut neutral = editor(source.clone(), &[1]);
    let before = neutral.clone();
    triple(&mut neutral);
    assert_eq!(neutral.scene, before.scene);
    assert!(neutral.undo == before.undo && neutral.redo == before.redo);
    // A redo branch also survives the neutral case.
    let mut with_redo = editor(source, &[0]);
    triple(&mut with_redo);
    with_redo.dispatch(Command::Undo).unwrap();
    with_redo.scene.selection.faces = vec![1];
    let before = with_redo.clone();
    triple(&mut with_redo);
    assert_eq!(with_redo.scene, before.scene);
    assert_eq!(with_redo.redo.len(), 1);
    assert!(with_redo.undo == before.undo && with_redo.redo == before.redo);
}

/// Arrowhead quad whose only valid diagonal is 1-3 (vertex 1 is reflex).
fn arrowhead() -> Vec<DVec3> {
    [(0., 0., 0.), (2., 0., 1.), (4., 0., 0.), (2., 0., 3.)]
        .map(|(x, y, z)| DVec3::new(x, y, z))
        .to_vec()
}

#[test]
fn tri_002_concave_dart_and_existing_edge_refusal() {
    let p = arrowhead();
    let source = plain(&p, &[vec![0, 1, 2, 3]]);
    let mut e = editor(source.clone(), &[0]);
    triple(&mut e);
    assert_exact_triangulation(&source, 0, mesh(&e), &e.scene.selection.faces);
    // The required diagonal already exists as another face's edge: using it
    // would make a non-manifold edge, so the edit refuses atomically.
    let mut q = p.clone();
    q.push(DVec3::new(2., 3., 2.));
    let blocked = plain(&q, &[vec![0, 1, 2, 3], vec![1, 3, 4]]);
    let mut e = editor(blocked, &[0]);
    assert!(refuses(&mut e).contains("existing mesh edge"));
}

#[test]
fn tri_002_collinear_corners_large_and_comb_polygons() {
    // Square with a collinear midpoint: the midpoint stays a corner.
    let square = [
        (0., 0., 0.),
        (1., 0., 0.),
        (2., 0., 0.),
        (2., 0., 2.),
        (0., 0., 2.),
    ]
    .map(|(x, y, z)| DVec3::new(x, y, z));
    let src = plain(&square, &[vec![0, 4, 3, 2, 1]]);
    let mut e = editor(src.clone(), &[0]);
    triple(&mut e);
    assert_exact_triangulation(&src, 0, mesh(&e), &e.scene.selection.faces);
    // 64-gon in a tilted plane.
    let tilt = DVec3::new(0.3, 1., 0.2).normalize();
    let (u, w) = basis(tilt);
    let ring: Vec<DVec3> = (0..64)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / 64.;
            DVec3::new(1., 2., 3.) + u * a.cos() * 2. + w * a.sin()
        })
        .collect();
    let src = plain(&ring, &[(0..64).collect()]);
    let mut e = editor(src.clone(), &[0]);
    triple(&mut e);
    assert_exact_triangulation(&src, 0, mesh(&e), &e.scene.selection.faces);
    // Comb with many reflex corners.
    let mut comb = vec![DVec3::new(0., 0., 0.), DVec3::new(10., 0., 0.)];
    for k in (0..5).rev() {
        let x = 2. * k as f64;
        comb.extend([
            DVec3::new(x + 2., 0., 4.),
            DVec3::new(x + 1.5, 0., 4.),
            DVec3::new(x + 1.5, 0., 1.),
            DVec3::new(x + 0.5, 0., 1.),
            DVec3::new(x + 0.5, 0., 4.),
        ]);
    }
    comb.push(DVec3::new(0., 0., 4.));
    let mut seen = Vec::new();
    comb.retain(|p| {
        let fresh = !seen.contains(p);
        seen.push(*p);
        fresh
    });
    let n = comb.len() as u32;
    let src = plain(&comb, &[(0..n).rev().collect()]);
    let mut e = editor(src.clone(), &[0]);
    triple(&mut e);
    assert_exact_triangulation(&src, 0, mesh(&e), &e.scene.selection.faces);
}

/// Independent derivation of exceptional edge uses: every directed surface
/// edge use grouped by undirected key, in ascending output face order, kept
/// only where an edge has two same-direction uses or more than two uses.
fn exceptional_uses(m: &Mesh) -> BTreeMap<String, Vec<[u32; 3]>> {
    let mut uses: BTreeMap<(u32, u32), Vec<[u32; 3]>> = BTreeMap::new();
    for (f, face) in m.faces.iter().enumerate() {
        let v = &face.vertices;
        for i in 0..v.len() {
            let (a, b) = (v[i], v[(i + 1) % v.len()]);
            uses.entry((a.min(b), a.max(b)))
                .or_default()
                .push([f as u32, a, b]);
        }
    }
    uses.into_iter()
        .filter(|(_, u)| u.len() > 2 || (u.len() == 2 && u[0][1..] == u[1][1..]))
        .map(|((a, b), u)| (format!("{a}:{b}"), u))
        .collect()
}

/// Review follow-up: an exceptional incidence ledger (same-direction shared
/// edge) on a selected quad's boundary. When that boundary moves to an
/// appended triangle after an unsplit neighbour, the record must list uses in
/// output face order exactly as mesh validation derives them. Both source face
/// orders and every quad boundary edge are exercised.
#[test]
fn tri_003_exceptional_edge_uses_follow_output_face_order() {
    let quad = [0u32, 3, 2, 1];
    let mut reordered_case = false;
    for k in 0..4 {
        let (a, b) = (quad[k], quad[(k + 1) % 4]);
        let mut positions = [(0., 0., 0.), (1., 0., 0.), (1., 0., 1.), (0., 0., 1.)]
            .map(|(x, y, z)| DVec3::new(x, y, z))
            .to_vec();
        // Off-plane apex: the neighbour uses a->b in the same direction.
        let mid = (positions[a as usize] + positions[b as usize]) / 2.;
        positions.push(mid + DVec3::new(0., 1., 0.));
        for quad_first in [true, false] {
            let neighbour = vec![a, b, 4];
            let faces = if quad_first {
                vec![quad.to_vec(), neighbour]
            } else {
                vec![neighbour, quad.to_vec()]
            };
            let quad_index = if quad_first { 0 } else { 1 };
            let mut source = Mesh {
                vertices: positions.clone(),
                faces: faces.iter().map(|f| face(f)).collect(),
                ..Default::default()
            };
            source.attributes.extrude_edge_uses = exceptional_uses(&source);
            assert_eq!(source.attributes.extrude_edge_uses.len(), 1);
            source.validate().unwrap();
            let mut e = editor(source.clone(), &[quad_index]);
            triple(&mut e);
            let out = mesh(&e).clone();
            assert_exact_triangulation(
                &source,
                quad_index as usize,
                &out,
                &e.scene.selection.faces,
            );
            let expected = exceptional_uses(&out);
            assert_eq!(
                out.attributes.extrude_edge_uses, expected,
                "edge {k}, quad first {quad_first}"
            );
            let key = format!("{}:{}", a.min(b), a.max(b));
            let uses = &expected[&key];
            assert_eq!(uses.len(), 2);
            assert!(uses.iter().all(|u| u[1..] == [a, b]));
            let neighbour_index = 1 - quad_index;
            assert!(
                uses.iter().any(|u| u[0] == neighbour_index),
                "neighbour unchanged"
            );
            if uses.iter().any(|u| u[0] > neighbour_index && u[0] >= 2) && quad_first {
                reordered_case = true;
            }
            assert_eq!(
                out.faces[neighbour_index as usize],
                source.faces[neighbour_index as usize]
            );
            e.dispatch(Command::Undo).unwrap();
            assert_eq!(mesh(&e), &source);
        }
    }
    assert!(
        reordered_case,
        "an early face's edge reached an appended triangle"
    );
}
